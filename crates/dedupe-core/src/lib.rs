//! dedupe-core: a Rust reimplementation of Splink 5.0.0's Fellegi-Sunter
//! record-linkage engine, narrowly scoped to DuckDB/DuckLake execution.
//!
//! Parity contract: given the same settings JSON and input data, this engine
//! must reproduce Python Splink 5.0.0's outputs on DuckDB exactly (trained
//! parameters, match weights/probabilities, clusters), verified by golden
//! fixtures under `fixtures/golden/`.

pub mod blocking;
pub mod comparison;
pub mod error;
pub mod model;
pub mod predict;
pub mod scoring;
pub mod session;
pub mod settings;
pub mod sql;
pub mod stages;
pub mod training;

pub use error::{DedupeError, Result};
pub use model::TrainedModel;
pub use settings::Settings;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The Splink version whose behavior this engine replicates.
pub const SPLINK_PARITY_VERSION: &str = "5.0.0";

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        assert_eq!(super::SPLINK_PARITY_VERSION, "5.0.0");
    }
}
