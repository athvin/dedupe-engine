//! Assembles the predict-pipeline stages. Mirrors Splink 5's `_predict_chunk`
//! (single-chunk path): blocked pairs are materialized from
//! `__splink__df_concat`; comparison vectors (and later scoring) run as one
//! CTE pipeline over the blocked pairs with `__splink__df_concat_with_tf`
//! re-derived as a CTE.

use crate::error::Result;
use crate::session::{Session, TableHandle};
use crate::settings::Settings;
use crate::sql::pipeline::CtePipeline;
use crate::stages::block::{self, BLOCKED_ID_PAIRS};
use crate::stages::concat::{self, DF_CONCAT};
use crate::stages::score;
use crate::stages::tf::{self, DF_CONCAT_WITH_TF};
use crate::stages::uid_expr_for;
use crate::stages::vectors::{self, BLOCKED_WITH_COLS, DF_COMPARISON_VECTORS};

/// Materialize `__splink__df_concat` from a single registered input table.
pub fn materialize_concat(session: &Session, input: &TableHandle) -> Result<TableHandle> {
    let mut p = CtePipeline::new().with_input(input);
    p.enqueue(concat::concat_sql_single(input), DF_CONCAT);
    session.materialize(&p)
}

/// Materialize `__splink__df_concat` from multiple registered inputs (link
/// types), prepending each dataset's display name as the `source_dataset`
/// literal. Column order comes from the first input.
pub fn materialize_concat_multi(
    session: &Session,
    inputs: &[(&TableHandle, &str)],
    settings: &Settings,
) -> Result<TableHandle> {
    if inputs.len() == 1 && !settings.source_dataset_required() {
        return materialize_concat(session, inputs[0].0);
    }
    // Inputs are referenced by physical name inside the union SQL, so they are
    // NOT registered as pipeline inputs (that would rewrite the source_dataset
    // string literals).
    let mut p = CtePipeline::new();
    p.enqueue(
        concat::concat_sql_union(inputs, &settings.source_dataset_column),
        DF_CONCAT,
    );
    session.materialize(&p)
}

/// Compute and materialize one TF table per TF column (the cached artifacts).
pub fn materialize_tf_tables(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
) -> Result<Vec<TableHandle>> {
    settings
        .tf_columns()
        .iter()
        .map(|col| {
            let mut p = CtePipeline::new().with_input(concat);
            p.enqueue(
                tf::tf_table_sql(col, &settings.tf_prefix),
                tf::tf_table_name(col),
            );
            session.materialize(&p)
        })
        .collect()
}

/// Materialize `__splink__blocked_id_pairs` (blocking reads the concat, not
/// concat_with_tf). Materialized rather than inlined, matching Splink's
/// default and sidestepping the DuckDB UNION-ALL pushdown regression.
pub fn materialize_blocked_pairs(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
) -> Result<TableHandle> {
    let uid = uid_expr_for(settings);
    let mut p = CtePipeline::new().with_input(concat);
    p.enqueue(
        block::blocked_id_pairs_sql(
            &settings.blocking_rules,
            &uid,
            settings.link_type,
            &settings.source_dataset_column,
            DF_CONCAT,
        ),
        BLOCKED_ID_PAIRS,
    );
    session.materialize(&p)
}

/// Pipeline from blocked id pairs through `__splink__df_comparison_vectors`.
pub fn comparison_vectors_pipeline(
    settings: &Settings,
    concat: &TableHandle,
    tf_tables: &[TableHandle],
    blocked_pairs: &TableHandle,
) -> CtePipeline {
    let uid = uid_expr_for(settings);
    let mut p = CtePipeline::new()
        .with_input(concat)
        .with_input(blocked_pairs);
    for t in tf_tables {
        p.add_input(t);
    }
    p.enqueue(
        tf::concat_with_tf_sql(&settings.tf_columns(), &settings.tf_prefix),
        DF_CONCAT_WITH_TF,
    );
    p.enqueue(
        vectors::blocked_with_cols_sql(settings, &uid),
        BLOCKED_WITH_COLS,
    );
    p.enqueue(
        vectors::comparison_vectors_sql(settings),
        DF_COMPARISON_VECTORS,
    );
    p
}

/// Deterministic link: candidate pairs from the blocking rules joined back to
/// their record columns, with NO probabilistic scoring. The output is the
/// `blocked_with_cols` table (uid + comparison/tf columns as `_l`/`_r`, plus
/// `match_key`). Use for strict rule-based linkage.
pub fn deterministic_link(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
    tf_tables: &[TableHandle],
) -> Result<TableHandle> {
    let uid = uid_expr_for(settings);
    let blocked = materialize_blocked_pairs(session, settings, concat)?;
    let mut p = CtePipeline::new().with_input(concat).with_input(&blocked);
    for t in tf_tables {
        p.add_input(t);
    }
    p.enqueue(
        tf::concat_with_tf_sql(&settings.tf_columns(), &settings.tf_prefix),
        DF_CONCAT_WITH_TF,
    );
    p.enqueue(
        vectors::blocked_with_cols_sql(settings, &uid),
        BLOCKED_WITH_COLS,
    );
    let out = session.materialize(&p)?;
    session.drop_table(&blocked)?;
    Ok(out)
}

/// Full predict pipeline: comparison vectors -> match weight parts ->
/// `__splink__df_predict` scored with the settings' current parameters.
pub fn predict_pipeline(
    settings: &Settings,
    concat: &TableHandle,
    tf_tables: &[TableHandle],
    blocked_pairs: &TableHandle,
    threshold_match_weight: Option<f64>,
) -> Result<CtePipeline> {
    let mut p = comparison_vectors_pipeline(settings, concat, tf_tables, blocked_pairs);
    p.enqueue(
        score::match_weight_parts_sql(settings)?,
        score::DF_MATCH_WEIGHT_PARTS,
    );
    p.enqueue(
        score::predict_sql(settings, settings.prior, false, threshold_match_weight)?,
        score::DF_PREDICT,
    );
    Ok(p)
}

/// Convenience: run the whole predict flow from a registered input table.
pub fn run_predict(
    session: &Session,
    settings: &Settings,
    input: &TableHandle,
    threshold_match_weight: Option<f64>,
) -> Result<TableHandle> {
    let concat = materialize_concat(session, input)?;
    let tf_tables = materialize_tf_tables(session, settings, &concat)?;
    let blocked = materialize_blocked_pairs(session, settings, &concat)?;
    let pipeline = predict_pipeline(
        settings,
        &concat,
        &tf_tables,
        &blocked,
        threshold_match_weight,
    )?;
    session.materialize(&pipeline)
}
