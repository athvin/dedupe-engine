use clap::{Args, Parser, Subcommand};

/// dedupe: a Rust record-linkage engine (Splink 5.0.0 parity) on DuckLake.
#[derive(Parser, Debug)]
#[command(name = "dedupe", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Train a model (prior -> u -> EM) and write it to JSON.
    Train(TrainArgs),
    /// Predict pairwise match probabilities with a trained model.
    Predict(PredictArgs),
    /// Cluster predictions into entities via connected components.
    Cluster(ClusterArgs),
}

/// How to locate the input records.
#[derive(Args, Debug)]
pub struct InputSource {
    /// DuckLake ATTACH payload (everything after `ducklake:`), e.g.
    /// `catalog.ducklake`, `sqlite:meta.db`, `postgres:dbname=lake host=...`.
    #[arg(long)]
    pub catalog: Option<String>,
    /// DATA_PATH for the lake (where Parquet data lives).
    #[arg(long)]
    pub data_path: Option<String>,
    /// Schema within the lake catalog.
    #[arg(long, default_value = "main")]
    pub schema: String,
    /// Input table name within the lake catalog.
    #[arg(long)]
    pub input: Option<String>,
    /// Read input directly from a CSV file instead of the lake.
    #[arg(long)]
    pub input_csv: Option<String>,
    /// Read input directly from a Parquet file instead of the lake.
    #[arg(long)]
    pub input_parquet: Option<String>,
    /// Pin DuckDB thread count (use 1 for bit-reproducible float aggregation).
    #[arg(long)]
    pub threads: Option<u32>,
}

#[derive(Args, Debug)]
pub struct TrainArgs {
    /// Settings JSON (Splink-compatible).
    #[arg(long)]
    pub settings: String,
    #[command(flatten)]
    pub source: InputSource,
    /// Deterministic matching rule (repeatable) for prior estimation.
    #[arg(long = "deterministic-rule")]
    pub deterministic_rules: Vec<String>,
    /// Estimated recall of the deterministic rules.
    #[arg(long, default_value_t = 0.7)]
    pub recall: f64,
    /// Target pair count for u estimation sampling.
    #[arg(long, default_value_t = 1e6)]
    pub max_pairs: f64,
    /// EM training blocking rule (repeatable).
    #[arg(long = "em-rule")]
    pub em_rules: Vec<String>,
    /// Path to write the trained model JSON.
    #[arg(long)]
    pub output_model: String,
}

#[derive(Args, Debug)]
pub struct PredictArgs {
    /// Trained model JSON.
    #[arg(long)]
    pub model: String,
    #[command(flatten)]
    pub source: InputSource,
    /// Output table name written to the lake.
    #[arg(long, default_value = "predictions")]
    pub output: String,
    /// Optional match-probability threshold filter.
    #[arg(long)]
    pub threshold_match_probability: Option<f64>,
    /// Also write output to this Parquet file (when not using a lake).
    #[arg(long)]
    pub output_parquet: Option<String>,
}

#[derive(Args, Debug)]
pub struct ClusterArgs {
    /// Trained model JSON (for id/settings context).
    #[arg(long)]
    pub model: String,
    #[command(flatten)]
    pub source: InputSource,
    /// Predictions table name in the lake (or use --predictions-parquet).
    #[arg(long)]
    pub predictions: Option<String>,
    /// Predictions Parquet file.
    #[arg(long)]
    pub predictions_parquet: Option<String>,
    /// Cluster at this match-probability threshold.
    #[arg(long, default_value_t = 0.95)]
    pub threshold: f64,
    /// Output table name written to the lake.
    #[arg(long, default_value = "clusters")]
    pub output: String,
    /// Also write output to this Parquet file (when not using a lake).
    #[arg(long)]
    pub output_parquet: Option<String>,
}
