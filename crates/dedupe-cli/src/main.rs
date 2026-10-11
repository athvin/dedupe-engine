mod args;

use std::path::Path;

use anyhow::{Context, Result, bail};
use args::{Cli, ClusterArgs, Command, InputSource, PredictArgs, TrainArgs};
use clap::Parser;
use dedupe_core::linker::{Linker, TrainConfig};
use dedupe_core::session::TableHandle;
use dedupe_core::{DuckLakeConfig, Session, SessionOptions, Settings};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Train(a) => train(a),
        Command::Predict(a) => predict(a),
        Command::Cluster(a) => cluster(a),
    }
}

/// Open a session, attaching a lake if the source references one.
fn open_session(source: &InputSource) -> Result<Session> {
    let mut session = Session::open_in_memory(SessionOptions {
        threads: source.threads,
    })?;
    if let Some(catalog) = &source.catalog {
        let cfg = DuckLakeConfig {
            metadata: catalog.clone(),
            data_path: source.data_path.clone(),
            schema: source.schema.clone(),
            ..Default::default()
        };
        session
            .attach_ducklake(&cfg)
            .context("attaching DuckLake catalog")?;
    }
    Ok(session)
}

/// Register the configured input as a templated table.
fn register_input(session: &Session, source: &InputSource, templated: &str) -> Result<TableHandle> {
    if let Some(csv) = &source.input_csv {
        return Ok(session.register_csv(Path::new(csv), templated, &[])?);
    }
    if let Some(parquet) = &source.input_parquet {
        let p = parquet.replace('\'', "''");
        return Ok(session.register_table(&format!("read_parquet('{p}')"), templated)?);
    }
    if let Some(table) = &source.input {
        return Ok(session.register_lake_table(table, templated)?);
    }
    bail!("no input specified: use --input (lake), --input-csv, or --input-parquet");
}

fn train(a: TrainArgs) -> Result<()> {
    if a.deterministic_rules.is_empty() {
        bail!("train requires at least one --deterministic-rule");
    }
    let settings =
        Settings::from_json_file(Path::new(&a.settings)).context("loading settings JSON")?;
    let session = open_session(&a.source)?;
    let input = register_input(&session, &a.source, "input_table")?;

    let mut linker = Linker::new(&session, settings, &input)?;
    let outcome = linker.train(&TrainConfig {
        deterministic_rules: a.deterministic_rules,
        recall: a.recall,
        u_max_pairs: a.max_pairs,
        em_rules: a.em_rules,
    })?;

    let model = linker.into_model();
    model.save_to_json_file(Path::new(&a.output_model))?;

    eprintln!(
        "trained: prior={:.6}, {} EM session(s)",
        outcome.prior,
        outcome.em_sessions.len()
    );
    for s in &outcome.em_sessions {
        eprintln!(
            "  EM blocked on `{}`: {} iterations{}, deactivated [{}]",
            s.blocking_rule,
            s.iterations.len() - 1,
            if s.converged { " (converged)" } else { "" },
            s.deactivated.join(", ")
        );
    }
    println!("{}", a.output_model);
    Ok(())
}

fn predict(a: PredictArgs) -> Result<()> {
    let settings = Settings::from_json_file(Path::new(&a.model)).context("loading model JSON")?;
    let session = open_session(&a.source)?;
    let input = register_input(&session, &a.source, "input_table")?;

    let linker = Linker::new(&session, settings, &input)?;
    let predictions = linker.predict(a.threshold_match_probability)?;
    let n = session.count(&predictions.physical)?;

    if let Some(path) = &a.output_parquet {
        write_parquet(&session, &predictions, path)?;
    }
    if a.source.catalog.is_some() {
        session.write_to_lake(&predictions, &a.output)?;
        eprintln!("wrote {n} predictions to lake table `{}`", a.output);
    } else if a.output_parquet.is_none() {
        bail!("no output target: provide --catalog (lake) or --output-parquet");
    }
    println!("{n}");
    Ok(())
}

fn cluster(a: ClusterArgs) -> Result<()> {
    let settings = Settings::from_json_file(Path::new(&a.model)).context("loading model JSON")?;
    let session = open_session(&a.source)?;
    let input = register_input(&session, &a.source, "input_table")?;

    let predictions = if let Some(parquet) = &a.predictions_parquet {
        let p = parquet.replace('\'', "''");
        session.register_table(&format!("read_parquet('{p}')"), "__splink__df_predict")?
    } else if let Some(table) = &a.predictions {
        session.register_lake_table(table, "__splink__df_predict")?
    } else {
        bail!("no predictions specified: use --predictions (lake) or --predictions-parquet");
    };

    let linker = Linker::new(&session, settings, &input)?;
    let clusters = linker.cluster(&predictions, a.threshold)?;
    let n = session.count(&clusters.physical)?;

    if let Some(path) = &a.output_parquet {
        write_parquet(&session, &clusters, path)?;
    }
    if a.source.catalog.is_some() {
        session.write_to_lake(&clusters, &a.output)?;
        eprintln!("wrote {n} clustered rows to lake table `{}`", a.output);
    } else if a.output_parquet.is_none() {
        bail!("no output target: provide --catalog (lake) or --output-parquet");
    }
    println!("{n}");
    Ok(())
}

fn write_parquet(session: &Session, table: &TableHandle, path: &str) -> Result<()> {
    let p = path.replace('\'', "''");
    session.execute_batch(&format!(
        "COPY {} TO '{p}' (FORMAT PARQUET)",
        table.physical
    ))?;
    Ok(())
}
