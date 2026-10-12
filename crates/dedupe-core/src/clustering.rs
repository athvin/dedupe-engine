//! Connected components at a threshold — Splink's SQL algorithm
//! (`connected_components.py`): min-representative propagation with a `stable`
//! flag that retires converged clusters from the working set. `cluster_id` is
//! the minimum node id in each component.

use crate::error::Result;
use crate::session::{Session, TableHandle};
use crate::settings::Settings;
use crate::sql::literals::pyrepr;
use crate::sql::pipeline::CtePipeline;
use crate::stages::uid_expr_for;

/// Threshold expressed on match probability (Splink's default clustering API).
pub struct ClusterOptions {
    pub threshold_match_probability: f64,
}

fn materialize_sql(
    session: &Session,
    inputs: &[&TableHandle],
    sql: String,
    name: &str,
) -> Result<TableHandle> {
    let mut p = CtePipeline::new();
    for h in inputs {
        p.add_input(h);
    }
    p.enqueue(sql, name);
    session.materialize(&p)
}

/// Cluster pairwise predictions at a probability threshold. Returns a handle
/// to `(node_id, cluster_id)`.
pub fn cluster_at_threshold(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
    predictions: &TableHandle,
    opts: ClusterOptions,
) -> Result<TableHandle> {
    let uid = uid_expr_for(settings);
    let threshold = pyrepr(opts.threshold_match_probability);

    // Nodes: one row per record.
    let nodes = materialize_sql(
        session,
        &[concat],
        format!(
            "select\n    {} as node_id\nfrom {}",
            uid.render_bare(),
            concat.templated
        ),
        "__splink__df_nodes_with_composite_ids",
    )?;

    // Edges from predictions above threshold.
    let edges = materialize_sql(
        session,
        &[predictions],
        format!(
            "select\n    {} as node_id_l,\n    {} as node_id_r,\n    match_probability\nfrom {}\nwhere match_probability >= {threshold}",
            uid.render_suffixed("l"),
            uid.render_suffixed("r"),
            predictions.templated
        ),
        "__splink__df_edges_from_predict",
    )?;

    // Symmetrized neighbours.
    let neighbours = materialize_sql(
        session,
        &[&edges],
        format!(
            "select node_id_l as node_rep, node_id_l as node_id, node_id_r as neighbour_rep, node_id_r as neighbour\nfrom {e}\nwhere match_probability >= {threshold}\nunion all\nselect node_id_r as node_rep, node_id_r as node_id, node_id_l as neighbour_rep, node_id_l as neighbour\nfrom {e}\nwhere match_probability >= {threshold}",
            e = edges.templated
        ),
        "__splink__df_neighbours",
    )?;

    // Initial representatives: each node is its own representative.
    let mut representatives = materialize_sql(
        session,
        &[&nodes],
        format!(
            "select distinct node_id as node_id, node_id as representative, 0 as stable\nfrom {}",
            nodes.templated
        ),
        "__splink__df_representatives",
    )?;

    let mut neighbours_src = neighbours.clone();
    let mut stable_tables: Vec<TableHandle> = Vec::new();
    let mut iteration = 0u32;

    loop {
        iteration += 1;
        // rep_updates: min representative and min stable per old_rep.
        let rep_updates = materialize_sql(
            session,
            &[&neighbours_src, &representatives],
            format!(
                "select old_rep, min(representative) as representative, min(stable) as stable\nfrom (\n  select node_rep as old_rep, neighbour_rep as representative, 0 as stable from {n}\n  union all\n  select representative as old_rep, representative, 1 as stable from {r}\n) as rep_union\ngroup by old_rep",
                n = neighbours_src.templated,
                r = representatives.templated
            ),
            "__splink__rep_updates",
        )?;

        // Push new reps down to nodes. Templated names carry the iteration so
        // the final union references each stable set as a distinct input.
        let new_reps = materialize_sql(
            session,
            &[&rep_updates, &representatives],
            format!(
                "select repr.node_id, r.representative, r.stable\nfrom {ru} as r\nleft join {prev} as repr\non r.old_rep = repr.representative",
                ru = rep_updates.templated,
                prev = representatives.templated
            ),
            &format!("__splink__representatives_{iteration}"),
        )?;

        let stable = materialize_sql(
            session,
            &[&new_reps],
            format!("select * from {} where stable = 1", new_reps.templated),
            &format!("__splink__representatives_stable_{iteration}"),
        )?;
        let unstable = materialize_sql(
            session,
            &[&new_reps],
            format!("select * from {} where stable = 0", new_reps.templated),
            &format!("__splink__representatives_unstable_{iteration}"),
        )?;

        // Re-map neighbour endpoints to new reps; keep only cross-cluster edges.
        let filtered = materialize_sql(
            session,
            &[&neighbours_src, &new_reps],
            format!(
                "select l.representative as node_rep, n.node_id, n.neighbour, r.representative as neighbour_rep\nfrom {n} as n\nleft join {reps} as l on l.node_id = n.node_id\nleft join {reps} as r on n.neighbour = r.node_id\nwhere l.representative <> r.representative",
                n = neighbours_src.templated,
                reps = new_reps.templated
            ),
            &format!("__splink__filtered_neighbours_{iteration}"),
        )?;

        let remaining = session.count(&filtered.physical)?;

        // Clean up the per-iteration intermediates we no longer need.
        session.drop_table(&rep_updates)?;
        session.drop_table(&representatives)?;
        session.drop_table(&new_reps)?;
        session.drop_table(&neighbours_src)?;

        stable_tables.push(stable);

        if remaining == 0 {
            session.drop_table(&filtered)?;
            // Final unstable set joins the retired stable sets.
            representatives = unstable;
            break;
        }
        representatives = unstable;
        neighbours_src = filtered;
    }

    // Final cluster assignment: union of retired stable sets + final unstable.
    let mut union_parts: Vec<String> = stable_tables
        .iter()
        .map(|t| {
            format!(
                "select node_id as node_id, representative as cluster_id\nfrom {}",
                t.templated
            )
        })
        .collect();
    union_parts.push(format!(
        "select node_id as node_id, representative as cluster_id\nfrom {}",
        representatives.templated
    ));

    let mut inputs: Vec<&TableHandle> = stable_tables.iter().collect();
    inputs.push(&representatives);
    let output = materialize_sql(
        session,
        &inputs,
        union_parts.join("\n\nUNION ALL\n\n"),
        "__splink__clustering_output_final",
    )?;

    for t in &stable_tables {
        session.drop_table(t)?;
    }
    session.drop_table(&representatives)?;
    session.drop_table(&neighbours)?;
    session.drop_table(&nodes)?;
    session.drop_table(&edges)?;

    Ok(output)
}

/// Incremental clustering: fold a new batch's edges into an existing cluster
/// assignment without re-running connected components over the whole graph.
///
/// Correctness: the result is *identical* to fully re-clustering the combined
/// edge set, because connected components is a pure function of (nodes, edges)
/// and `cluster_id` is the minimum node id in a component. We exploit this by
/// contracting each existing cluster to a single super-node, running
/// connected components over just the new edges (mapped onto super-nodes),
/// then expanding — far cheaper than re-clustering every node and edge.
///
/// Inputs:
/// - `all_concat`: `__splink__df_concat`-shaped table of ALL records
///   (existing + new).
/// - `existing_clusters`: `(node_id, cluster_id)` for the already-clustered
///   (existing) records.
/// - `new_predictions`: scored pairs involving new records (its `unique_id_l`
///   / `unique_id_r` / `match_probability` columns); pairs below the threshold
///   are ignored.
///
/// Returns `(node_id, cluster_id)` for every record. This is dedupe-scoped
/// (single bare unique-id column).
pub fn incremental_cluster(
    session: &Session,
    settings: &Settings,
    all_concat: &TableHandle,
    existing_clusters: &TableHandle,
    new_predictions: &TableHandle,
    opts: ClusterOptions,
) -> Result<TableHandle> {
    let uid = uid_expr_for(settings);
    let uid_col = &settings.unique_id_column;

    // Every node's seed representative: its existing cluster id, or itself if
    // it is a brand-new record not present in the existing assignment.
    let all_reps = materialize_sql(
        session,
        &[all_concat, existing_clusters],
        format!(
            "select n.node_id, coalesce(e.cluster_id, n.node_id) as seed_rep\nfrom (select {node} as node_id from {concat}) as n\nleft join {existing} as e on n.node_id = e.node_id",
            node = uid.render_bare(),
            concat = all_concat.templated,
            existing = existing_clusters.templated,
        ),
        "__splink__df_incremental_reps",
    )?;

    // Super-node graph: distinct seed reps are the nodes, and each new edge
    // (above threshold) becomes an edge between its endpoints' seed reps.
    let seed_nodes = materialize_sql(
        session,
        &[&all_reps],
        format!(
            "select distinct seed_rep as \"{uid_col}\" from {}",
            all_reps.templated
        ),
        "__splink__df_incremental_seed_nodes",
    )?;
    let threshold = pyrepr(opts.threshold_match_probability);
    let seed_edges = materialize_sql(
        session,
        &[new_predictions, &all_reps],
        format!(
            "select al.seed_rep as \"{uid_col}_l\", ar.seed_rep as \"{uid_col}_r\", p.match_probability\nfrom {preds} as p\njoin {reps} as al on al.node_id = {uid_l}\njoin {reps} as ar on ar.node_id = {uid_r}\nwhere p.match_probability >= {threshold}",
            preds = new_predictions.templated,
            reps = all_reps.templated,
            uid_l = uid.render_suffixed("l"),
            uid_r = uid.render_suffixed("r"),
        ),
        "__splink__df_incremental_seed_edges",
    )?;

    // Connected components over the (small) super-node graph.
    let seed_clusters = cluster_at_threshold(session, settings, &seed_nodes, &seed_edges, opts)?;

    // Expand: map every node through its seed rep to the merged cluster id.
    let output = materialize_sql(
        session,
        &[&all_reps, &seed_clusters],
        format!(
            "select ar.node_id, sc.cluster_id\nfrom {reps} as ar\njoin {seed} as sc on ar.seed_rep = sc.node_id",
            reps = all_reps.templated,
            seed = seed_clusters.templated,
        ),
        "__splink__clustering_output_final",
    )?;

    session.drop_table(&all_reps)?;
    session.drop_table(&seed_nodes)?;
    session.drop_table(&seed_edges)?;
    session.drop_table(&seed_clusters)?;
    Ok(output)
}

/// Join the cluster assignment back to the input records.
pub fn clustered_with_input_data(
    session: &Session,
    settings: &Settings,
    concat: &TableHandle,
    clusters: &TableHandle,
) -> Result<TableHandle> {
    let uid = uid_expr_for(settings);
    let node_expr = uid.render("c");
    let sql = format!(
        "select cc.cluster_id, c.*\nfrom {clusters} as cc\nleft join {concat} as c\non cc.node_id = {node_expr}",
        clusters = clusters.templated,
        concat = concat.templated,
    );
    materialize_sql(
        session,
        &[clusters, concat],
        sql,
        "__splink__df_clustered_with_input_data",
    )
}
