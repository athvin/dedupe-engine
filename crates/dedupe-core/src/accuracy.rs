//! Accuracy evaluation from ground-truth labels (Splink's label-column path).
//!
//! Produces the "truth-space table": one row per match-weight threshold with
//! the confusion-matrix counts (TP/FP/TN/FN) and derived metrics (precision,
//! recall, F-scores, specificity, NPV, accuracy, φ, ROC rates). This is the
//! data behind Splink's accuracy / ROC / precision-recall charts; no charts
//! are produced here.
//!
//! The label-column path assigns truth from equality of a ground-truth column
//! (e.g. a known cluster id): candidate pairs are everything the model's
//! blocking rules generate, UNION same-label pairs the blocking missed (via an
//! appended label-equality blocking rule). Pairs never materialised are
//! implicit true negatives (the "ghost" adjustment against the full cartesian
//! count).

use crate::blocking::BlockingRule;
use crate::error::Result;
use crate::predict;
use crate::session::{Session, TableHandle};
use crate::settings::Settings;
use crate::sql::literals::pyrepr;
use crate::sql::pipeline::CtePipeline;
use crate::stages::score::{self, DF_MATCH_WEIGHT_PARTS, DF_PREDICT};
use crate::stages::uid_expr_for;
use crate::training::prior::total_possible_comparisons;

pub struct AccuracyOptions {
    pub label_column: String,
    /// A clerical/label score at or above this counts as a true match.
    pub threshold_actual: f64,
    /// Round match weights to this granularity before enumerating thresholds
    /// (`None` = every distinct weight). Splink's default is 0.1.
    pub match_weight_round_to_nearest: Option<f64>,
    /// Push clerical positives missed by blocking to a sentinel threshold so
    /// they count as false negatives at every real threshold.
    pub positives_not_captured_by_blocking_rules_scored_as_zero: bool,
}

impl Default for AccuracyOptions {
    fn default() -> Self {
        AccuracyOptions {
            label_column: "cluster".to_string(),
            threshold_actual: 0.5,
            match_weight_round_to_nearest: Some(0.1),
            positives_not_captured_by_blocking_rules_scored_as_zero: true,
        }
    }
}

/// Build the labelled, scored pair set: the model's blocking rules plus an
/// appended label-equality rule, scored, with `clerical_match_score` (binary
/// label equality) and `found_by_blocking_rules` (produced by a real model
/// rule rather than the appended label rule).
fn labelled_predictions(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
    tf_tables: &[TableHandle],
    label_column: &str,
) -> Result<TableHandle> {
    let uid = uid_expr_for(settings);
    let n_model_rules = settings.blocking_rules.len();

    // Augmented rules: model rules (indices 0..n) + label-equality rule (n).
    let mut rules = settings.blocking_rules.clone();
    rules.push(BlockingRule {
        index: n_model_rules,
        sql: format!("l.\"{label_column}\" = r.\"{label_column}\""),
    });

    let mut bp = CtePipeline::new().with_input(concat);
    bp.enqueue(
        crate::stages::block::blocked_id_pairs_sql(
            &rules,
            &uid,
            settings.link_type,
            &settings.source_dataset_column,
            concat.templated.as_str(),
        ),
        crate::stages::block::BLOCKED_ID_PAIRS,
    );
    let blocked = session.materialize(&bp)?;

    // Retain the label column through scoring so clerical truth can be derived.
    let mut acc_settings = settings.clone();
    if !acc_settings
        .additional_columns_to_retain
        .iter()
        .any(|c| c == label_column)
    {
        acc_settings
            .additional_columns_to_retain
            .push(label_column.to_string());
    }

    // comparison vectors -> match weight parts -> predict (no threshold).
    let mut p = predict::comparison_vectors_pipeline(&acc_settings, concat, tf_tables, &blocked);
    p.enqueue(
        score::match_weight_parts_sql(&acc_settings)?,
        DF_MATCH_WEIGHT_PARTS,
    );
    p.enqueue(
        score::predict_sql(&acc_settings, acc_settings.prior, false, None)?,
        DF_PREDICT,
    );
    let predictions = session.materialize(&p)?;
    session.drop_table(&blocked)?;

    // labels_with_predictions: add clerical_match_score + found_by_blocking_rules.
    let mut lp = CtePipeline::new().with_input(&predictions);
    lp.enqueue(
        format!(
            "select *,\n  case when \"{label_column}_l\" = \"{label_column}_r\" then cast(1.0 as float8) else cast(0.0 as float8) end as clerical_match_score,\n  not (cast(match_key as int) = {n_model_rules}) as found_by_blocking_rules\nfrom {}",
            predictions.templated
        ),
        "__splink__labels_with_predictions",
    );
    let out = session.materialize(&lp)?;
    session.drop_table(&predictions)?;
    Ok(out)
}

/// The 7-step truth-space SQL over a `__splink__labels_with_predictions`-shaped
/// table (needs `match_weight`, `clerical_match_score`, `found_by_blocking_rules`).
fn truth_space_sql(
    labels_with_predictions: &str,
    threshold_actual: f64,
    round_to_nearest: Option<f64>,
    total_labels: f64,
    positives_as_zero: bool,
) -> String {
    let truth_thres_expr = match round_to_nearest {
        Some(r) => format!(
            "cast({r} as float) * (round(match_weight/{r}))",
            r = pyrepr(r)
        ),
        None => "match_weight".to_string(),
    };
    let ta = pyrepr(threshold_actual);
    let tt_adj = if positives_as_zero {
        "CASE WHEN found_by_blocking_rules then truth_threshold ELSE cast(-999 as float8) END"
    } else {
        "truth_threshold"
    };
    let total_labels_str = format!("cast({} as float8)", pyrepr(total_labels));
    let extra_neg =
        format!("({total_labels_str} - total_clerical_positives - total_clerical_negatives)");

    format!(
        r#"WITH
__splink__labels_with_pos_neg as (
    select *,
        {truth_thres_expr} as truth_threshold,
        case when clerical_match_score >= {ta} then 1 else 0 end as clerical_positive,
        case when clerical_match_score >= {ta} then 0 else 1 end as clerical_negative
    from {labels_with_predictions}
),
__splink__labels_with_pos_neg_tt_adj as (
    select *, {tt_adj} as truth_threshold_adj
    from __splink__labels_with_pos_neg
),
__splink__labels_with_pos_neg_grouped as (
    select
        truth_threshold_adj as truth_threshold,
        count(*) as num_records_in_row,
        sum(clerical_positive) as clerical_positive,
        sum(clerical_negative) as clerical_negative
    from __splink__labels_with_pos_neg_tt_adj
    group by truth_threshold_adj
),
__splink__labels_with_pos_neg_grouped_with_stats as (
    select
        truth_threshold,
        (sum(clerical_positive) over (order by truth_threshold desc))
            as cumulative_clerical_positives_at_or_above_threshold,
        (sum(clerical_negative) over (order by truth_threshold)) - clerical_negative
            as cumulative_clerical_negatives_below_threshold,
        (select sum(clerical_positive) from __splink__labels_with_pos_neg_grouped) as total_clerical_positives,
        (select sum(clerical_negative) from __splink__labels_with_pos_neg_grouped) as total_clerical_negatives,
        (select sum(num_records_in_row) from __splink__labels_with_pos_neg_grouped) as total_clerical_labels,
        -num_records_in_row + sum(num_records_in_row) over (order by truth_threshold)
            as num_labels_scored_below_threshold,
        sum(num_records_in_row) over (order by truth_threshold desc)
            as num_labels_scored_at_or_above_threshold
    from __splink__labels_with_pos_neg_grouped
),
__splink__labels_with_pos_neg_grouped_with_stats_adj as (
    select
        truth_threshold,
        cumulative_clerical_positives_at_or_above_threshold,
        cumulative_clerical_negatives_below_threshold + {extra_neg}
            as cumulative_clerical_negatives_below_threshold,
        total_clerical_positives,
        {total_labels_str} - total_clerical_positives as total_clerical_negatives,
        {total_labels_str} as total_clerical_labels,
        num_labels_scored_below_threshold + {extra_neg} as num_labels_scored_below_threshold,
        num_labels_scored_at_or_above_threshold
    from __splink__labels_with_pos_neg_grouped_with_stats
),
__splink__labels_with_pos_neg_grouped_with_truth_stats as (
    select
        truth_threshold,
        total_clerical_labels,
        total_clerical_positives as P,
        total_clerical_negatives as N,
        num_labels_scored_at_or_above_threshold - cumulative_clerical_positives_at_or_above_threshold as FP,
        cumulative_clerical_positives_at_or_above_threshold as TP,
        num_labels_scored_below_threshold - cumulative_clerical_negatives_below_threshold as FN,
        cumulative_clerical_negatives_below_threshold as TN
    from __splink__labels_with_pos_neg_grouped_with_stats_adj
)
select
    truth_threshold,
    power(2, truth_threshold) / (1 + power(2, truth_threshold)) as match_probability,
    cast(total_clerical_labels as float8) as total_clerical_labels,
    cast(P as float8) as p,
    cast(N as float8) as n,
    cast(TP as float8) as tp,
    cast(TN as float8) as tn,
    cast(FP as float8) as fp,
    cast(FN as float8) as fn,
    cast(P/total_clerical_labels as float8) as P_rate,
    cast(N as float)/total_clerical_labels as N_rate,
    cast(TP as float8)/P as tp_rate,
    cast(TN as float8)/N as tn_rate,
    cast(FP as float8)/N as fp_rate,
    cast(FN as float8)/P as fn_rate,
    case when TP+FP=0 then 1 else cast(TP as float8)/(TP+FP) end as precision,
    cast(TP as float8)/P as recall,
    cast(TN as float8)/N as specificity,
    case when TN+FN=0 then 1 else cast(TN as float8)/(TN+FN) end as npv,
    cast(TP+TN as float8)/(P+N) as accuracy,
    cast(2.0*TP/(2*TP + FN + FP) as float8) as f1,
    cast(5.0*TP/(5*TP + 4*FN + FP) as float8) as f2,
    cast(1.25*TP/(1.25*TP + 0.25*FN + FP) as float8) as f0_5,
    cast(4.0*TP*TN/((4.0*TP*TN) + ((TP + TN)*(FP + FN))) as float8) as p4,
    case when TN+FN=0 or TP+FP=0 or P=0 or N=0 then 0
        else cast((TP*TN)-(FP*FN) as float8)/sqrt((TP+FP)*P*N*(TN+FN)) end as phi
from __splink__labels_with_pos_neg_grouped_with_truth_stats
where truth_threshold >= cast(-998 as float8)
order by truth_threshold"#
    )
}

/// Compute the truth-space table from a ground-truth label column.
pub fn truth_space_from_labels_column(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
    tf_tables: &[TableHandle],
    opts: &AccuracyOptions,
) -> Result<TableHandle> {
    let total_labels = total_possible_comparisons(session, concat, settings)?;
    let labelled = labelled_predictions(session, settings, concat, tf_tables, &opts.label_column)?;

    let mut p = CtePipeline::new().with_input(&labelled);
    p.enqueue(
        truth_space_sql(
            &labelled.templated,
            opts.threshold_actual,
            opts.match_weight_round_to_nearest,
            total_labels,
            opts.positives_not_captured_by_blocking_rules_scored_as_zero,
        ),
        "__splink__truth_space_table",
    );
    let out = session.materialize(&p)?;
    session.drop_table(&labelled)?;
    Ok(out)
}
