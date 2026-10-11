//! DuckLake attachment layer (completed in milestone M7).
//!
//! Architecture: inputs are read from the attached `lake` catalog via
//! cross-catalog SELECTs; all `__splink__*` intermediates live in the native
//! temp catalog (DuckLake cannot host temp tables); final outputs are written
//! back to the lake in dedicated single-catalog transactions.

#[derive(Debug, Clone)]
pub struct DuckLakeConfig {
    /// Everything after `ducklake:` in the ATTACH string, verbatim — e.g.
    /// `meta.ducklake`, `sqlite:meta.db`, `postgres:dbname=lake host=...`.
    pub metadata: String,
    /// `DATA_PATH` attach option (Parquet data location).
    pub data_path: Option<String>,
    /// Catalog alias; never `USE`d — all references are fully qualified.
    pub alias: String,
    /// Schema within the lake catalog.
    pub schema: String,
    pub read_only: bool,
}

impl Default for DuckLakeConfig {
    fn default() -> Self {
        DuckLakeConfig {
            metadata: String::new(),
            data_path: None,
            alias: "lake".to_string(),
            schema: "main".to_string(),
            read_only: false,
        }
    }
}
