//! `estimate_probability_two_random_records_match` via deterministic rules.
//!
//! Counts deduped pairs produced by the deterministic rules (same
//! "AND NOT preceding" dedup as blocking), divides by total possible
//! comparisons, and scales up by recall:
//!   prob = (num_observed / recall) / num_total_comparisons

use crate::blocking::BlockingRule;
use crate::error::{DedupeError, Result};
use crate::model::TrainedModel;
use crate::session::{Session, TableHandle};
use crate::settings::json::LinkType;
use crate::sql::pipeline::CtePipeline;
use crate::stages::block::blocked_id_pairs_sql;
use crate::stages::uid_expr_for;

/// Total possible comparisons for the dedupe path: C(N, 2).
fn total_possible_comparisons_dedupe(n: i64) -> f64 {
    (n as f64) * ((n - 1) as f64) / 2.0
}

/// Estimate the prior from deterministic rules and write it into the model.
pub fn estimate_prior(
    session: &Session,
    model: &mut TrainedModel,
    concat: &TableHandle,
    deterministic_rules: &[String],
    recall: f64,
) -> Result<f64> {
    if recall > 1.0 || recall <= 0.0 {
        return Err(DedupeError::settings(format!(
            "recall must be in (0, 1], got {recall}"
        )));
    }
    if model.settings.link_type != LinkType::DedupeOnly {
        return Err(DedupeError::Other(
            "prior estimation currently supports dedupe_only".to_string(),
        ));
    }

    let total_nodes = session.count(&concat.physical)?;
    let num_total_comparisons = total_possible_comparisons_dedupe(total_nodes);

    let rules: Vec<BlockingRule> = deterministic_rules
        .iter()
        .enumerate()
        .map(|(i, sql)| BlockingRule {
            index: i,
            sql: sql.clone(),
        })
        .collect();

    let uid = uid_expr_for(&model.settings);
    let mut p = CtePipeline::new().with_input(concat);
    p.enqueue(
        blocked_id_pairs_sql(
            &rules,
            &uid,
            model.settings.link_type,
            &model.settings.source_dataset_column,
            concat.templated.as_str(),
        ),
        "__splink__deterministic_blocked",
    );
    let blocked = session.materialize(&p)?;
    let num_observed = session.count(&blocked.physical)? as f64;
    session.drop_table(&blocked)?;

    if num_observed > num_total_comparisons * recall {
        return Err(DedupeError::Other(format!(
            "deterministic rules produced more matches ({num_observed}) than recall {recall} \
             allows; recall must be at least {:.2}",
            num_observed / num_total_comparisons
        )));
    }

    let prob = (num_observed / recall) / num_total_comparisons;
    model.set_prior(prob);
    Ok(prob)
}
