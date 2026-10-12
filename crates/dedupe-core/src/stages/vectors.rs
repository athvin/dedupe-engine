//! `blocked_with_cols` (join record columns back onto blocked id pairs) and
//! `__splink__df_comparison_vectors` (gamma CASE columns).
//! Matches goldens `predict_04_blocked_with_cols.sql` and
//! `predict_05___splink__df_comparison_vectors.sql`.

use super::UidExpr;
use super::block::BLOCKED_ID_PAIRS;
use super::tf::DF_CONCAT_WITH_TF;
use crate::comparison::Comparison;
use crate::settings::Settings;

pub const BLOCKED_WITH_COLS: &str = "blocked_with_cols";
pub const DF_COMPARISON_VECTORS: &str = "__splink__df_comparison_vectors";

fn l_r_select(col: &str) -> [String; 2] {
    [
        format!("\"l\".\"{col}\" AS \"{col}_l\""),
        format!("\"r\".\"{col}\" AS \"{col}_r\""),
    ]
}

/// Unique-id (and source_dataset) columns selected as `x_l`/`x_r`.
fn uid_select_columns(settings: &Settings) -> Vec<String> {
    let mut out = Vec::new();
    if settings.source_dataset_required() {
        out.extend(l_r_select(&settings.source_dataset_column));
    }
    out.extend(l_r_select(&settings.unique_id_column));
    out
}

/// Per-comparison `x_l`/`x_r` selections for blocking: input columns then TF
/// columns.
fn comparison_blocking_columns(c: &Comparison, tf_prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    for col in &c.input_columns {
        out.extend(l_r_select(col));
    }
    for col in c.tf_input_columns() {
        out.extend(l_r_select(&format!("{tf_prefix}{col}")));
    }
    out
}

/// uid/source-dataset columns plus each comparison's blocking columns (not
/// yet deduped; no additional-retain columns).
fn select_columns_for_blocking(settings: &Settings, comparisons: &[&Comparison]) -> Vec<String> {
    let mut out = uid_select_columns(settings);
    for c in comparisons {
        out.extend(comparison_blocking_columns(c, &settings.tf_prefix));
    }
    out
}

fn dedup_preserving_order(items: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for item in items {
        if !deduped.contains(&item) {
            deduped.push(item);
        }
    }
    deduped
}

/// The full select list for `blocked_with_cols`, deduped preserving order.
fn columns_to_select_for_blocking(settings: &Settings) -> Vec<String> {
    let all: Vec<&Comparison> = settings.comparisons.iter().collect();
    let mut out = select_columns_for_blocking(settings, &all);
    for col in &settings.additional_columns_to_retain {
        out.extend(l_r_select(col));
    }
    dedup_preserving_order(out)
}

fn blocked_with_cols_join_sql(
    cols: &str,
    uid: &UidExpr,
    blocked: &str,
    left_table: &str,
    right_table: &str,
) -> String {
    format!(
        "select\n    {cols},\n    b.match_key\nfrom {blocked} as b\ninner join {left_table} as l\non {uid_l} = b.join_key_l\ninner join {right_table} as r\non {uid_r} = b.join_key_r",
        uid_l = uid.render("l"),
        uid_r = uid.render("r"),
    )
}

pub fn blocked_with_cols_sql(settings: &Settings, uid: &UidExpr) -> String {
    blocked_with_cols_sql_tables(
        settings,
        uid,
        BLOCKED_ID_PAIRS,
        DF_CONCAT_WITH_TF,
        DF_CONCAT_WITH_TF,
    )
}

/// As `blocked_with_cols_sql`, but with explicit blocked-pairs and left/right
/// source tables (incremental linkage uses separate left/right concats).
pub fn blocked_with_cols_sql_tables(
    settings: &Settings,
    uid: &UidExpr,
    blocked: &str,
    left_table: &str,
    right_table: &str,
) -> String {
    let cols = columns_to_select_for_blocking(settings).join(",\n    ");
    blocked_with_cols_join_sql(&cols, uid, blocked, left_table, right_table)
}

/// As `blocked_with_cols_sql`, but restricted to a subset of comparisons and
/// without the additional-retain columns — the shape EM training scores.
pub fn blocked_with_cols_sql_comparisons(
    settings: &Settings,
    comparisons: &[&Comparison],
    uid: &UidExpr,
) -> String {
    let cols =
        dedup_preserving_order(select_columns_for_blocking(settings, comparisons)).join(",\n    ");
    blocked_with_cols_join_sql(
        &cols,
        uid,
        BLOCKED_ID_PAIRS,
        DF_CONCAT_WITH_TF,
        DF_CONCAT_WITH_TF,
    )
}

/// The gamma CASE statement for one comparison. Level order is declaration
/// order (first matching WHEN wins); the ELSE level emits `ELSE <gamma>`.
pub fn gamma_case_sql(c: &Comparison) -> String {
    let mut arms = Vec::new();
    for level in &c.levels {
        if level.is_else {
            arms.push(format!("    ELSE {}", level.gamma));
        } else {
            arms.push(format!(
                "    WHEN {} THEN {}",
                level.sql_condition, level.gamma
            ));
        }
    }
    format!(
        "CASE\n{}\nEND as {}",
        arms.join("\n"),
        c.gamma_column_name()
    )
}

/// Select list for `__splink__df_comparison_vectors`: uid columns, then per
/// comparison [input `x_l`/`x_r` if retain_matching_columns, gamma CASE, TF
/// columns], then additional-retain columns, then match_key.
pub fn comparison_vectors_sql(settings: &Settings) -> String {
    let mut cols: Vec<String> = Vec::new();
    if settings.source_dataset_required() {
        cols.push(format!("\"{}_l\"", settings.source_dataset_column));
        cols.push(format!("\"{}_r\"", settings.source_dataset_column));
    }
    cols.push(format!("\"{}_l\"", settings.unique_id_column));
    cols.push(format!("\"{}_r\"", settings.unique_id_column));

    for c in &settings.comparisons {
        if settings.retain_matching_columns {
            for col in &c.input_columns {
                cols.push(format!("\"{col}_l\""));
                cols.push(format!("\"{col}_r\""));
            }
        }
        cols.push(gamma_case_sql(c));
        for col in c.tf_input_columns() {
            cols.push(format!("\"{}{col}_l\"", settings.tf_prefix));
            cols.push(format!("\"{}{col}_r\"", settings.tf_prefix));
        }
    }
    for col in &settings.additional_columns_to_retain {
        cols.push(format!("\"{col}_l\""));
        cols.push(format!("\"{col}_r\""));
    }
    cols.push("match_key".to_string());

    let mut deduped: Vec<String> = Vec::new();
    for c in cols {
        if !deduped.contains(&c) {
            deduped.push(c);
        }
    }
    format!(
        "select\n    {}\nfrom {BLOCKED_WITH_COLS}",
        deduped.join(",\n    ")
    )
}
