//! Session: owns one DuckDB connection, materializes pipelines as temp
//! tables with run-scoped physical names, and registers input data. The
//! DuckLake attachment layer lives in `ducklake` (all `__splink__*`
//! intermediates stay in the native temp catalog regardless — DuckLake cannot
//! host temp tables).

pub mod ducklake;

use std::cell::Cell;
use std::path::Path;

use duckdb::Connection;

use crate::error::{DedupeError, Result};
use crate::sql::pipeline::CtePipeline;

#[derive(Debug, Clone)]
pub struct TableHandle {
    /// Splink-style templated name, e.g. `__splink__df_concat`.
    pub templated: String,
    /// Actual table/view name in the database.
    pub physical: String,
    /// Column names in table order.
    pub columns: Vec<String>,
}

#[derive(Default)]
pub struct SessionOptions {
    /// None = DuckDB default. Parity tests use Some(1) so float aggregation
    /// order matches the golden-generation runs.
    pub threads: Option<u32>,
}

pub struct Session {
    conn: Connection,
    run_id: String,
    seq: Cell<u64>,
    lake: Option<LakeAttachment>,
}

#[derive(Debug, Clone)]
struct LakeAttachment {
    alias: String,
    schema: String,
}

impl Session {
    pub fn open_in_memory(opts: SessionOptions) -> Result<Session> {
        let conn = Connection::open_in_memory()?;
        if let Some(threads) = opts.threads {
            conn.execute_batch(&format!("SET threads = {threads};"))?;
        }
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        Ok(Session {
            conn,
            run_id: format!("{nanos:08x}"),
            seq: Cell::new(0),
            lake: None,
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Attach a DuckLake catalog. Intermediates still live in the native temp
    /// catalog; the lake is used only to read inputs and write outputs. The
    /// catalog is never `USE`d — all lake references are fully qualified.
    pub fn attach_ducklake(&mut self, cfg: &ducklake::DuckLakeConfig) -> Result<()> {
        self.conn
            .execute_batch("INSTALL ducklake; LOAD ducklake;")?;
        let mut options = Vec::new();
        if let Some(dp) = &cfg.data_path {
            options.push(format!("DATA_PATH '{}'", dp.replace('\'', "''")));
        }
        if cfg.read_only {
            options.push("READ_ONLY".to_string());
        }
        // Pin catalog format: never auto-migrate behind our back.
        options.push("AUTOMATIC_MIGRATION false".to_string());
        let opts = if options.is_empty() {
            String::new()
        } else {
            format!(" ({})", options.join(", "))
        };
        self.conn.execute_batch(&format!(
            "ATTACH 'ducklake:{}' AS {}{opts};",
            cfg.metadata.replace('\'', "''"),
            cfg.alias
        ))?;
        if !cfg.read_only {
            // Deterministic Parquet output (no catalog-inlined small writes).
            self.conn.execute_batch(&format!(
                "CALL {}.set_option('data_inlining_row_limit', 0);",
                cfg.alias
            ))?;
        }
        self.lake = Some(LakeAttachment {
            alias: cfg.alias.clone(),
            schema: cfg.schema.clone(),
        });
        Ok(())
    }

    fn lake(&self) -> Result<&LakeAttachment> {
        self.lake
            .as_ref()
            .ok_or_else(|| DedupeError::Other("no DuckLake catalog attached".to_string()))
    }

    /// Register a table from the attached lake catalog under a templated name
    /// (via a temp view, so downstream handling is uniform).
    pub fn register_lake_table(&self, table: &str, templated: &str) -> Result<TableHandle> {
        let lake = self.lake()?;
        let qualified = format!("{}.{}.{}", lake.alias, lake.schema, table);
        self.register_table(&qualified, templated)
    }

    /// Write a table into the lake in its own transaction (one writable
    /// catalog per transaction).
    pub fn write_to_lake(&self, src: &TableHandle, target: &str) -> Result<()> {
        let lake = self.lake()?;
        self.conn.execute_batch(&format!(
            "BEGIN; CREATE OR REPLACE TABLE {}.{}.{target} AS SELECT * FROM {}; COMMIT;",
            lake.alias, lake.schema, src.physical
        ))?;
        Ok(())
    }

    fn next_physical(&self, templated: &str) -> String {
        let seq = self.seq.get();
        self.seq.set(seq + 1);
        format!("{templated}_{}_{seq}", self.run_id)
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.conn.execute_batch(sql).map_err(DedupeError::from)
    }

    fn table_columns(&self, table: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(&format!("DESCRIBE SELECT * FROM {table}"))?;
        let cols = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(cols)
    }

    /// Register a CSV file as a temp table. `column_types` (name -> DuckDB
    /// type) pins the schema; pass the pandas-equivalent types for parity
    /// with Splink's tutorial ingestion.
    pub fn register_csv(
        &self,
        path: &Path,
        templated: &str,
        column_types: &[(&str, &str)],
    ) -> Result<TableHandle> {
        let physical = self.next_physical(templated);
        let path_str = path.to_string_lossy().replace('\'', "''");
        let sql = if column_types.is_empty() {
            format!("CREATE TEMP TABLE {physical} AS SELECT * FROM read_csv('{path_str}')")
        } else {
            let cols = column_types
                .iter()
                .map(|(name, ty)| format!("'{name}': '{ty}'"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "CREATE TEMP TABLE {physical} AS SELECT * FROM read_csv('{path_str}', columns = {{{cols}}})"
            )
        };
        self.conn.execute_batch(&sql)?;
        let columns = self.table_columns(&physical)?;
        Ok(TableHandle {
            templated: templated.to_string(),
            physical,
            columns,
        })
    }

    /// Register an existing table or view (by qualified name) under a
    /// templated name via a temp view, so downstream handling is uniform.
    pub fn register_table(&self, qualified_name: &str, templated: &str) -> Result<TableHandle> {
        let physical = self.next_physical(templated);
        self.conn.execute_batch(&format!(
            "CREATE TEMP VIEW {physical} AS SELECT * FROM {qualified_name}"
        ))?;
        let columns = self.table_columns(&physical)?;
        Ok(TableHandle {
            templated: templated.to_string(),
            physical,
            columns,
        })
    }

    /// Render and execute a pipeline, materializing the result as a temp
    /// table named after the pipeline's final output.
    pub fn materialize(&self, pipeline: &CtePipeline) -> Result<TableHandle> {
        let templated = pipeline.output_name().to_string();
        let physical = self.next_physical(&templated);
        let sql = format!("CREATE TEMP TABLE {physical} AS\n{}", pipeline.render());
        self.conn.execute_batch(&sql).map_err(|e| {
            DedupeError::Other(format!("materializing {templated}: {e}\nSQL:\n{sql}"))
        })?;
        let columns = self.table_columns(&physical)?;
        Ok(TableHandle {
            templated,
            physical,
            columns,
        })
    }

    pub fn drop_table(&self, handle: &TableHandle) -> Result<()> {
        self.conn
            .execute_batch(&format!("DROP TABLE IF EXISTS {}", handle.physical))?;
        Ok(())
    }

    pub fn count(&self, table: &str) -> Result<i64> {
        let n = self
            .conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get::<_, i64>(0)
            })?;
        Ok(n)
    }

    pub fn query_scalar_f64(&self, sql: &str) -> Result<f64> {
        Ok(self.conn.query_row(sql, [], |r| r.get::<_, f64>(0))?)
    }

    /// Run a query and return all rows rendered as strings (for diagnostics
    /// and small result sets). NULL renders as "NULL".
    pub fn query_strings(&self, sql: &str) -> Result<Vec<Vec<String>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let ncols = {
            let rows = stmt.query([])?;
            let _ = rows;
            stmt.column_count()
        };
        let mut out = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let mut rendered = Vec::with_capacity(ncols);
            for i in 0..ncols {
                let v: duckdb::types::Value = row.get(i)?;
                rendered.push(render_value(&v));
            }
            out.push(rendered);
        }
        Ok(out)
    }
}

fn render_value(v: &duckdb::types::Value) -> String {
    use duckdb::types::Value;
    match v {
        Value::Null => "NULL".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::TinyInt(i) => i.to_string(),
        Value::SmallInt(i) => i.to_string(),
        Value::Int(i) => i.to_string(),
        Value::BigInt(i) => i.to_string(),
        Value::HugeInt(i) => i.to_string(),
        Value::UTinyInt(i) => i.to_string(),
        Value::USmallInt(i) => i.to_string(),
        Value::UInt(i) => i.to_string(),
        Value::UBigInt(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(f) => format!("{f:?}"),
        Value::Text(s) => s.clone(),
        other => format!("{other:?}"),
    }
}
