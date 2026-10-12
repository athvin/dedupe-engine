//! High-level orchestration façade over a `Session`, mirroring the stages a
//! Splink user runs: train (prior -> u -> EM sessions), predict, cluster.

use crate::clustering::{ClusterOptions, cluster_at_threshold, clustered_with_input_data};
use crate::error::Result;
use crate::model::TrainedModel;
use crate::predict;
use crate::scoring::match_weight_to_prob;
use crate::session::{Session, TableHandle};
use crate::settings::Settings;
use crate::training::em::{EmOptions, EmSessionResult, run_em_session};
use crate::training::estimate_u::{UOptions, estimate_u};
use crate::training::prior::estimate_prior;

pub struct TrainConfig {
    pub deterministic_rules: Vec<String>,
    pub recall: f64,
    pub u_max_pairs: f64,
    pub em_rules: Vec<String>,
}

pub struct TrainOutcome {
    pub prior: f64,
    pub em_sessions: Vec<EmSessionResult>,
}

/// Orchestrates the full Splink workflow against one registered input table.
pub struct Linker<'a> {
    session: &'a Session,
    model: TrainedModel,
    concat: TableHandle,
    tf_tables: Vec<TableHandle>,
}

impl<'a> Linker<'a> {
    /// Build a linker over an already-registered input table (e.g. from a CSV
    /// or a lake table). Materializes `__splink__df_concat` and the TF tables.
    pub fn new(session: &'a Session, settings: Settings, input: &TableHandle) -> Result<Self> {
        Self::new_multi(session, settings, &[(input, "")])
    }

    /// Build a linker over one or more registered inputs. For link types, each
    /// `(table, display_name)` becomes a `source_dataset`; for dedupe a single
    /// input is used.
    pub fn new_multi(
        session: &'a Session,
        settings: Settings,
        inputs: &[(&TableHandle, &str)],
    ) -> Result<Self> {
        let model = TrainedModel::new(settings);
        let concat = predict::materialize_concat_multi(session, inputs, &model.settings)?;
        let tf_tables = predict::materialize_tf_tables(session, &model.settings, &concat)?;
        Ok(Linker {
            session,
            model,
            concat,
            tf_tables,
        })
    }

    pub fn model(&self) -> &TrainedModel {
        &self.model
    }

    pub fn into_model(self) -> TrainedModel {
        self.model
    }

    /// Run the full training sequence in Splink's canonical order.
    pub fn train(&mut self, cfg: &TrainConfig) -> Result<TrainOutcome> {
        let prior = estimate_prior(
            self.session,
            &mut self.model,
            &self.concat,
            &cfg.deterministic_rules,
            cfg.recall,
        )?;
        estimate_u(
            self.session,
            &mut self.model,
            &self.concat,
            UOptions {
                max_pairs: cfg.u_max_pairs,
                ..Default::default()
            },
        )?;
        let mut em_sessions = Vec::new();
        for rule in &cfg.em_rules {
            let result = run_em_session(
                self.session,
                &mut self.model,
                &self.concat,
                &self.tf_tables,
                rule,
                EmOptions::default(),
            )?;
            em_sessions.push(result);
        }
        Ok(TrainOutcome { prior, em_sessions })
    }

    /// Predict, optionally filtering to a match-probability threshold.
    pub fn predict(&self, threshold_match_probability: Option<f64>) -> Result<TableHandle> {
        let threshold_mw = threshold_match_probability.and_then(prob_threshold_to_mw);
        let blocked =
            predict::materialize_blocked_pairs(self.session, &self.model.settings, &self.concat)?;
        let pipeline = predict::predict_pipeline(
            &self.model.settings,
            &self.concat,
            &self.tf_tables,
            &blocked,
            threshold_mw,
        )?;
        self.session.materialize(&pipeline)
    }

    /// Cluster predictions at a probability threshold and join back to inputs.
    pub fn cluster(
        &self,
        predictions: &TableHandle,
        threshold_match_probability: f64,
    ) -> Result<TableHandle> {
        let clusters = cluster_at_threshold(
            self.session,
            &self.model.settings,
            &self.concat,
            predictions,
            ClusterOptions {
                threshold_match_probability,
            },
        )?;
        clustered_with_input_data(self.session, &self.model.settings, &self.concat, &clusters)
    }
}

/// Splink converts a probability threshold to a match-weight threshold for the
/// predict filter; a threshold of 0 means "no filter".
fn prob_threshold_to_mw(p: f64) -> Option<f64> {
    if p <= 0.0 {
        None
    } else {
        // Inverse of match_weight_to_prob: mw = log2(p / (1 - p)).
        let _ = match_weight_to_prob; // documents the inverse relationship
        Some((p / (1.0 - p)).log2())
    }
}
