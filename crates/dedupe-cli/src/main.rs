mod args;

use std::path::Path;

use anyhow::{Context, Result, bail};
use args::{
    Cli, ClusterArgs, ClusterNewArgs, Command, InputSource, LinkNewArgs, LinkNewScope, PredictArgs,
    TrainArgs,
};
use clap::Parser;
use dedupe_core::clustering::{ClusterOptions, clustered_with_input_data, incremental_cluster};
use dedupe_core::incremental::{predict_between, predict_within};
use dedupe_core::linker::{Linker, TrainConfig};
use dedupe_core::predict;
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
        Command::LinkNew(a) => link_new(a),
        Command::ClusterNew(a) => cluster_new(a),
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

fn link_new(a: LinkNewArgs) -> Result<()> {
    let settings = Settings::from_json_file(Path::new(&a.model)).context("loading model JSON")?;
    let session = open_session(&a.source)?;

    // Existing records come from the standard --input/--input-csv/--input-parquet.
    let existing_input = register_input(&session, &a.source, "existing_input")?;
    // New batch comes from --new-input / --new-input-csv / --new-input-parquet.
    let new_input = register_new_input(&session, &a)?;

    let existing_concat = predict::materialize_concat(&session, &existing_input)?;
    let new_concat = predict::materialize_concat(&session, &new_input)?;
    // Term frequencies reflect the established (existing) data.
    let tf_tables = predict::materialize_tf_tables(&session, &settings, &existing_concat)?;

    let threshold_mw = a.threshold_match_probability.and_then(|p| {
        if p <= 0.0 {
            None
        } else {
            Some((p / (1.0 - p)).log2())
        }
    });

    let mut parts: Vec<TableHandle> = Vec::new();
    if matches!(a.scope, LinkNewScope::Between | LinkNewScope::Both) {
        parts.push(predict_between(
            &session,
            &settings,
            &existing_concat,
            &new_concat,
            &tf_tables,
            threshold_mw,
        )?);
    }
    if matches!(a.scope, LinkNewScope::Within | LinkNewScope::Both) {
        parts.push(predict_within(
            &session,
            &settings,
            &new_concat,
            &tf_tables,
            threshold_mw,
        )?);
    }

    let result = union_tables(&session, &parts)?;
    let n = session.count(&result.physical)?;

    if let Some(path) = &a.output_parquet {
        write_parquet(&session, &result, path)?;
    }
    if a.source.catalog.is_some() {
        session.write_to_lake(&result, &a.output)?;
        eprintln!(
            "wrote {n} incremental predictions to lake table `{}`",
            a.output
        );
    } else if a.output_parquet.is_none() {
        bail!("no output target: provide --catalog (lake) or --output-parquet");
    }
    println!("{n}");
    Ok(())
}

fn cluster_new(a: ClusterNewArgs) -> Result<()> {
    let settings = Settings::from_json_file(Path::new(&a.model)).context("loading model JSON")?;
    let session = open_session(&a.source)?;
    let input = register_input(&session, &a.source, "input_table")?;
    let all_concat = predict::materialize_concat(&session, &input)?;

    // Existing clusters, normalised to (node_id, cluster_id).
    let existing_raw = if let Some(parquet) = &a.existing_clusters_parquet {
        let p = parquet.replace('\'', "''");
        session.register_table(&format!("read_parquet('{p}')"), "existing_clusters_raw")?
    } else if let Some(table) = &a.existing_clusters {
        session.register_lake_table(table, "existing_clusters_raw")?
    } else {
        bail!("no existing clusters: use --existing-clusters or --existing-clusters-parquet");
    };
    let existing_clusters = session.register_table(
        &format!(
            "(select \"{uid}\" as node_id, cluster_id from {tbl})",
            uid = settings.unique_id_column,
            tbl = existing_raw.physical
        ),
        "existing_clusters",
    )?;

    let new_predictions = if let Some(parquet) = &a.predictions_parquet {
        let p = parquet.replace('\'', "''");
        session.register_table(&format!("read_parquet('{p}')"), "__splink__df_predict")?
    } else if let Some(table) = &a.predictions {
        session.register_lake_table(table, "__splink__df_predict")?
    } else {
        bail!("no new predictions: use --predictions or --predictions-parquet");
    };

    let clusters = incremental_cluster(
        &session,
        &settings,
        &all_concat,
        &existing_clusters,
        &new_predictions,
        ClusterOptions {
            threshold_match_probability: a.threshold,
        },
    )?;
    let joined = clustered_with_input_data(&session, &settings, &all_concat, &clusters)?;
    let n = session.count(&joined.physical)?;

    if let Some(path) = &a.output_parquet {
        write_parquet(&session, &joined, path)?;
    }
    if a.source.catalog.is_some() {
        session.write_to_lake(&joined, &a.output)?;
        eprintln!("wrote {n} clustered rows to lake table `{}`", a.output);
    } else if a.output_parquet.is_none() {
        bail!("no output target: provide --catalog (lake) or --output-parquet");
    }
    println!("{n}");
    Ok(())
}

/// Register the new-batch input for `link-new`.
fn register_new_input(session: &Session, a: &LinkNewArgs) -> Result<TableHandle> {
    if let Some(csv) = &a.new_input_csv {
        return Ok(session.register_csv(Path::new(csv), "new_input", &[])?);
    }
    if let Some(parquet) = &a.new_input_parquet {
        let p = parquet.replace('\'', "''");
        return Ok(session.register_table(&format!("read_parquet('{p}')"), "new_input")?);
    }
    if let Some(table) = &a.new_input {
        return Ok(session.register_lake_table(table, "new_input")?);
    }
    bail!(
        "no new batch specified: use --new-input (lake), --new-input-csv, or --new-input-parquet"
    );
}

/// UNION ALL a set of prediction tables into one temp table.
fn union_tables(session: &Session, parts: &[TableHandle]) -> Result<TableHandle> {
    if parts.len() == 1 {
        return Ok(parts[0].clone());
    }
    let physical = format!("__splink__df_incremental_{}", std::process::id());
    let union = parts
        .iter()
        .map(|t| format!("SELECT * FROM {}", t.physical))
        .collect::<Vec<_>>()
        .join("\nUNION ALL\n");
    session.execute_batch(&format!("CREATE TEMP TABLE {physical} AS {union}"))?;
    Ok(TableHandle {
        templated: "__splink__df_incremental".to_string(),
        physical,
        columns: parts[0].columns.clone(),
    })
}

fn write_parquet(session: &Session, table: &TableHandle, path: &str) -> Result<()> {
    let p = path.replace('\'', "''");
    session.execute_batch(&format!(
        "COPY {} TO '{p}' (FORMAT PARQUET)",
        table.physical
    ))?;
    Ok(())
}
