#!/usr/bin/env python3
"""Generate golden parity fixtures for dedupe-core.

Runs real Splink (pinned in pyproject.toml/uv.lock) on the vendored fake_1000
dataset and captures everything dedupe-core's parity tests assert against:

- model snapshots after each training step (Splink saved-model JSON)
- per-iteration EM histories (lambda + m/u per level per iteration)
- per-stage intermediate tables (parquet) + the exact SQL Splink executed
- final predictions and clusters (parquet, deterministically sorted)
- settings introspection (gamma values, referenced columns, exact-match flags
  as derived by Splink's own sqlglot-based logic)
- MANIFEST.json (tool versions + sha256 of every golden file)

This script is deliberately linear and boring: it IS the parity spec.
Regenerate with:  uv run gen_golden.py
"""

from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

import duckdb
import pandas as pd
import splink
import splink.comparison_library as cl
from splink import DuckDBAPI, Linker, SettingsCreator, block_on

HERE = Path(__file__).resolve().parent
DATA = HERE / "datasets" / "fake_1000.csv"
OUT = HERE / "golden" / "fake_1000"

CLUSTER_THRESHOLD = 0.95
RECALL = 0.7
# > C(1000,2) = 499_500, so the canonical flow uses the full data (no sampling filter).
MAX_PAIRS_TUTORIAL = 1e6
# < C(1000,2): exercises the v5 hash-based deterministic sampling filter.
MAX_PAIRS_SAMPLED = 2e5

DETERMINISTIC_RULES = [
    block_on("first_name", "dob"),
    "l.first_name = r.first_name and levenshtein(r.surname, l.surname) <= 2",
    block_on("email"),
]
EM_RULE_1 = block_on("first_name", "surname")
EM_RULE_2 = block_on("dob")

# Incremental linkage: records with unique_id < this are the "existing" set,
# the rest are a "new" batch to be matched against (and within).
INCREMENTAL_SPLIT = 900


def new_db_api(
    sql_log: list | None = None, capture: dict | None = None
) -> tuple[duckdb.DuckDBPyConnection, DuckDBAPI]:
    # threads=1 keeps floating-point aggregation order stable across runs.
    con = duckdb.connect(":memory:", config={"threads": 1})
    db_api = DuckDBAPI(connection=con)
    orig = db_api._log_and_run_sql_execution

    def wrapped(final_sql: str, templated_name: str, physical_name: str):
        if sql_log is not None:
            sql_log.append(
                {
                    "templated_name": templated_name,
                    "physical_name": physical_name,
                    "sql": final_sql,
                }
            )
        result = orig(final_sql, templated_name, physical_name)
        # Some intermediates (blocked pairs, the u-estimation sample) are
        # dropped by Splink immediately after use, so export them the moment
        # they are created.
        if capture is not None and templated_name in capture:
            dest, order_by = capture[templated_name]
            con.execute(
                f"COPY (SELECT * FROM {physical_name} ORDER BY {order_by}) "
                f"TO '{dest}' (FORMAT PARQUET)"
            )
        return result

    db_api._log_and_run_sql_execution = wrapped
    return con, db_api


def build_settings() -> SettingsCreator:
    return SettingsCreator(
        link_type="dedupe_only",
        comparisons=[
            cl.NameComparison("first_name"),
            cl.NameComparison("surname"),
            cl.LevenshteinAtThresholds("dob", 1),
            cl.ExactMatch("city").configure(term_frequency_adjustments=True),
            cl.EmailComparison("email"),
        ],
        blocking_rules_to_generate_predictions=[
            block_on("first_name", "city"),
            block_on("surname"),
        ],
        retain_intermediate_calculation_columns=True,
    )


def make_linker(db_api: DuckDBAPI) -> Linker:
    df = pd.read_csv(DATA)
    sdf = db_api.register(df, dataset_display_name="fake_1000")
    return Linker(sdf, build_settings())


def input_column_names(cols) -> list[str]:
    out = []
    for ic in cols:
        name = None
        for attr in ("unquote",):
            try:
                name = getattr(ic, attr)().name
                break
            except Exception:
                pass
        if name is None:
            name = getattr(ic, "name", None) or str(ic)
        out.append(name)
    return out


def dump_settings_introspection(linker: Linker, path: Path) -> None:
    so = linker._settings_obj
    comparisons = []
    for c in so.comparisons:
        levels = []
        for lv in c.comparison_levels:
            try:
                exact_cols = lv._exact_match_colnames
            except Exception:
                exact_cols = None
            levels.append(
                {
                    "sql_condition": lv._sql_condition,
                    "comparison_vector_value": lv._comparison_vector_value,
                    "is_null_level": lv._is_null_level,
                    "is_else_level": lv._is_else_level,
                    "is_exact_match": lv._is_exact_match,
                    "exact_match_colnames": exact_cols,
                    "input_columns_used_by_sql_condition": input_column_names(
                        lv._input_columns_used_by_sql_condition
                    ),
                    "has_tf_adjustments": lv._has_tf_adjustments,
                    "tf_adjustment_column": (
                        lv._tf_adjustment_input_column_name
                        if lv._has_tf_adjustments
                        else None
                    ),
                }
            )
        comparisons.append(
            {
                "output_column_name": c.output_column_name,
                "gamma_column_name": c._gamma_column_name,
                "mw_column_name": c._mw_column_name,
                "input_columns_used_by_case_statement": input_column_names(
                    c._input_columns_used_by_case_statement
                ),
                "levels": levels,
            }
        )
    blocking = [
        br.blocking_rule_sql for br in so._blocking_rules_to_generate_predictions
    ]
    path.write_text(
        json.dumps(
            {"comparisons": comparisons, "blocking_rules": blocking},
            indent=2,
            default=str,
        )
    )


def to_jsonable(obj):
    import dataclasses

    if dataclasses.is_dataclass(obj) and not isinstance(obj, type):
        return {k: to_jsonable(v) for k, v in dataclasses.asdict(obj).items()}
    if isinstance(obj, dict):
        return {k: to_jsonable(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [to_jsonable(v) for v in obj]
    if isinstance(obj, (str, int, float, bool)) or obj is None:
        return obj
    return str(obj)


def dump_em_session(session, name: str) -> None:
    payload = {
        "blocking_rule": session._blocking_rule_for_training.blocking_rule_sql,
        "comparisons_that_cannot_be_estimated": [
            c.output_column_name
            for c in getattr(session, "_comparisons_that_cannot_be_estimated", [])
        ],
        "blocking_adjusted_probability_two_random_records_match": (
            session._blocking_adjusted_probability_two_random_records_match
        ),
        "lambda_history_records": to_jsonable(session._lambda_history_records),
        "iteration_history_records": to_jsonable(session._iteration_history_records),
    }
    (OUT / f"{name}.json").write_text(json.dumps(payload, indent=2, default=str))


def export_table(con: duckdb.DuckDBPyConnection, table: str, dest: Path, order_by: str) -> None:
    con.execute(
        f"COPY (SELECT * FROM {table} ORDER BY {order_by}) TO '{dest}' (FORMAT PARQUET)"
    )


def write_sql_log(sql_log: list, name: str) -> None:
    sql_dir = OUT / "sql"
    sql_dir.mkdir(exist_ok=True)
    for i, entry in enumerate(sql_log):
        fname = f"{name}_{i:02d}_{entry['templated_name']}.sql"
        (sql_dir / fname).write_text(
            f"-- templated: {entry['templated_name']}\n"
            f"-- physical: {entry['physical_name']}\n"
            + entry["sql"]
            + "\n"
        )


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def link_settings(link_type: str) -> SettingsCreator:
    return SettingsCreator(
        link_type=link_type,
        comparisons=[
            cl.ExactMatch("first_name"),
            cl.ExactMatch("surname"),
            cl.ExactMatch("dob"),
            cl.ExactMatch("city").configure(term_frequency_adjustments=True),
            cl.ExactMatch("email").configure(term_frequency_adjustments=True),
        ],
        blocking_rules_to_generate_predictions=[
            block_on("first_name"),
            block_on("surname"),
        ],
        retain_intermediate_calculation_columns=True,
    )


def gen_link_goldens(link_type: str) -> None:
    """Train + predict + cluster for a given link type on two datasets derived
    from fake_1000 (even unique_id -> df_a, odd -> df_b)."""
    con, db_api = new_db_api()
    df = pd.read_csv(DATA)
    a = df[df.unique_id % 2 == 0].copy()
    b = df[df.unique_id % 2 == 1].copy()
    sa = db_api.register(a, dataset_display_name="df_a")
    sb = db_api.register(b, dataset_display_name="df_b")
    con.execute(
        f"COPY (SELECT * FROM {sa.physical_name} ORDER BY unique_id) "
        f"TO '{OUT / 'link_df_a.parquet'}' (FORMAT PARQUET)"
    )
    con.execute(
        f"COPY (SELECT * FROM {sb.physical_name} ORDER BY unique_id) "
        f"TO '{OUT / 'link_df_b.parquet'}' (FORMAT PARQUET)"
    )

    linker = Linker([sa, sb], link_settings(link_type))
    linker.misc.save_model_to_json(
        OUT / f"link_{link_type}_model_initial.json", overwrite=True
    )
    linker.training.estimate_probability_two_random_records_match(
        [block_on("first_name", "dob"), block_on("email")], recall=RECALL
    )
    # max_pairs high enough that no sampling filter fires (deterministic u).
    linker.training.estimate_u_using_random_sampling(max_pairs=MAX_PAIRS_TUTORIAL)
    linker.training.estimate_parameters_using_expectation_maximisation(
        block_on("first_name"), estimate_without_term_frequencies=True
    )
    linker.training.estimate_parameters_using_expectation_maximisation(
        block_on("dob"), estimate_without_term_frequencies=True
    )
    linker.misc.save_model_to_json(OUT / f"link_{link_type}_model.json", overwrite=True)

    predictions = linker.inference.predict()
    export_table(
        con,
        predictions.physical_name,
        OUT / f"link_{link_type}_predictions.parquet",
        "source_dataset_l, unique_id_l, source_dataset_r, unique_id_r, match_key",
    )
    clusters = linker.clustering.cluster_pairwise_predictions_at_threshold(
        predictions, threshold_match_probability=CLUSTER_THRESHOLD
    )
    export_table(
        con,
        clusters.physical_name,
        OUT / f"link_{link_type}_clusters.parquet",
        "source_dataset, unique_id",
    )


def gen_batch1_goldens() -> None:
    """deterministic_link, score_pairs, model detailed records, n_largest_blocks."""
    from splink.blocking_analysis import n_largest_blocks

    con, db_api = new_db_api()
    df = pd.read_csv(DATA)
    sdf = db_api.register(df, dataset_display_name="fake_1000")
    settings = json.loads((OUT / "model_final.json").read_text())
    linker = Linker(sdf, settings)

    # deterministic_link: blocked pairs from the model's blocking rules, no scoring.
    det = linker.inference.deterministic_link()
    export_table(
        con,
        det.physical_name,
        OUT / "deterministic_link.parquet",
        "unique_id_l, unique_id_r, match_key",
    )

    # model weights as data: per comparison level m/u/bayes_factor/log2_bayes_factor.
    recs = linker._settings_obj._parameters_as_detailed_records
    import dataclasses

    out = []
    for r in recs:
        d = dataclasses.asdict(r) if dataclasses.is_dataclass(r) else dict(r)
        out.append(d)
    (OUT / "model_detailed_records.json").write_text(json.dumps(out, indent=2, default=str))

    # score_pairs: cartesian scoring of two fixed record slices (no blocking).
    for col in ["first_name", "surname", "city", "email"]:
        linker.table_management.compute_tf_table(col)
    a = df.head(5).copy()
    b = df.iloc[5:10].copy()
    sa = db_api.register(a, dataset_display_name="score_a")
    sb = db_api.register(b, dataset_display_name="score_b")
    con.execute(
        f"COPY (SELECT * FROM {sa.physical_name} ORDER BY unique_id) TO '{OUT / 'score_pairs_a.parquet'}' (FORMAT PARQUET)"
    )
    con.execute(
        f"COPY (SELECT * FROM {sb.physical_name} ORDER BY unique_id) TO '{OUT / 'score_pairs_b.parquet'}' (FORMAT PARQUET)"
    )
    sp = linker.inference.score_pairs(sa, sb)
    export_table(
        con,
        sp.physical_name,
        OUT / "score_pairs.parquet",
        "unique_id_l, unique_id_r",
    )

    # n_largest_blocks: biggest blocking-key blocks for a rule.
    con2, db2 = new_db_api()
    sdf2 = db2.register(df, dataset_display_name="fake_1000")
    nlb = n_largest_blocks(
        sdf2,
        blocking_rule=block_on("first_name"),
        link_type="dedupe_only",
        n_largest=5,
    )
    nlb_df = nlb.as_pandas_dataframe()
    con2.execute("CREATE TABLE _nlb AS SELECT * FROM nlb_df")
    export_table(con2, "_nlb", OUT / "n_largest_blocks.parquet", "block_count desc")


def gen_batch2_goldens() -> None:
    """graph metrics (nodes+clusters), single-best-links, multi-threshold clustering."""
    from splink.internals.clustering import (
        cluster_pairwise_predictions_at_multiple_thresholds,
    )

    # Graph metrics + multi-threshold on the dedupe model.
    con, db_api = new_db_api()
    df = pd.read_csv(DATA)
    sdf = db_api.register(df, dataset_display_name="fake_1000")
    settings = json.loads((OUT / "model_final.json").read_text())
    linker = Linker(sdf, settings)
    preds = linker.inference.predict()
    clusters = linker.clustering.cluster_pairwise_predictions_at_threshold(
        preds, threshold_match_probability=CLUSTER_THRESHOLD
    )
    gm = linker.clustering.compute_graph_metrics(preds, clusters)
    export_table(con, gm.nodes.physical_name, OUT / "graph_metrics_nodes.parquet", "composite_unique_id")
    export_table(con, gm.clusters.physical_name, OUT / "graph_metrics_clusters.parquet", "cluster_id")

    # Multi-threshold clustering summary stats.
    nodes_tbl = db_api.register(df[["unique_id"]].copy(), dataset_display_name="nodes_only")
    mt = cluster_pairwise_predictions_at_multiple_thresholds(
        nodes=sdf,
        edges=preds,
        db_api=db_api,
        node_id_column_name="unique_id",
        match_probability_thresholds=[0.5, 0.9, 0.95, 0.99],
        output_cluster_summary_stats=True,
    )
    export_table(
        con, mt.physical_name, OUT / "multi_threshold_summary.parquet", "threshold_match_probability"
    )

    # Single-best-links on the link_only datasets (each source is duplicate-free).
    con2, db2 = new_db_api()
    a = df[df.unique_id % 2 == 0].copy()
    b = df[df.unique_id % 2 == 1].copy()
    sa = db2.register(a, dataset_display_name="df_a")
    sb = db2.register(b, dataset_display_name="df_b")
    link_model = json.loads((OUT / "link_link_only_model.json").read_text())
    linker2 = Linker([sa, sb], link_model)
    preds2 = linker2.inference.predict()
    sbl = linker2.clustering.cluster_using_single_best_links(
        preds2,
        duplicate_free_datasets=["df_a", "df_b"],
        threshold_match_probability=CLUSTER_THRESHOLD,
    )
    export_table(
        con2, sbl.physical_name, OUT / "single_best_links_clusters.parquet", "source_dataset, unique_id"
    )


def gen_blocking_analysis_golden() -> None:
    """Per-rule comparison counts for the model's blocking rules (exact, no
    sampling), via Splink's count_comparisons_from_blocking_rules."""
    from splink.blocking_analysis import count_comparisons_from_blocking_rules

    con, db_api = new_db_api()
    df = pd.read_csv(DATA)
    sdf = db_api.register(df, dataset_display_name="fake_1000")
    records = count_comparisons_from_blocking_rules(
        sdf,
        blocking_rules=[block_on("first_name", "city"), block_on("surname")],
        link_type="dedupe_only",
        record_sample_proportion=1.0,
    )
    keep = [
        "blocking_rule",
        "match_key",
        "marginal_comparison_count",
        "cumulative_comparison_count",
        "total_possible_comparison_count",
    ]
    out = [{k: r[k] for k in keep} for r in records]
    (OUT / "blocking_analysis.json").write_text(json.dumps(out, indent=2, default=str))


def gen_accuracy_golden() -> None:
    """Truth-space table from the fake_1000 ground-truth `cluster` column,
    using the trained model (label-column accuracy path)."""
    con, db_api = new_db_api()
    df = pd.read_csv(DATA)
    sdf = db_api.register(df, dataset_display_name="fake_1000")
    settings = json.loads((OUT / "model_final.json").read_text())
    linker = Linker(sdf, settings)
    truth_space = linker.evaluation.accuracy_analysis_from_labels_column(
        "cluster",
        output_type="table",
        match_weight_round_to_nearest=0.1,
        add_metrics=["specificity", "npv", "accuracy", "f1", "f2", "f0_5", "p4", "phi"],
    )
    export_table(
        con,
        truth_space.physical_name,
        OUT / "accuracy_truth_space.parquet",
        "truth_threshold",
    )


def gen_incremental_goldens() -> None:
    con, db_api = new_db_api()
    df = pd.read_csv(DATA)
    existing = df[df.unique_id < INCREMENTAL_SPLIT].copy()
    new = df[df.unique_id >= INCREMENTAL_SPLIT].copy()

    # Export the exact input splits so the Rust parity test reads identical data.
    e_sdf = db_api.register(existing, dataset_display_name="existing")
    n_sdf = db_api.register(new, dataset_display_name="new")
    con.execute(
        f"COPY (SELECT * FROM {e_sdf.physical_name} ORDER BY unique_id) "
        f"TO '{OUT / 'incremental_existing.parquet'}' (FORMAT PARQUET)"
    )
    con.execute(
        f"COPY (SELECT * FROM {n_sdf.physical_name} ORDER BY unique_id) "
        f"TO '{OUT / 'incremental_new.parquet'}' (FORMAT PARQUET)"
    )

    # Trained model, with term frequencies computed from the existing data.
    settings = json.loads((OUT / "model_final.json").read_text())
    linker = Linker(e_sdf, settings)
    for col in ["first_name", "surname", "city", "email"]:
        linker.table_management.compute_tf_table(col)

    between = linker.inference.predict_between(e_sdf, n_sdf, warning_mode="never")
    export_table(
        con,
        between.physical_name,
        OUT / "incremental_between.parquet",
        "unique_id_l, unique_id_r, match_key",
    )

    within = linker.inference.predict_within(n_sdf, warning_mode="never")
    export_table(
        con,
        within.physical_name,
        OUT / "incremental_within.parquet",
        "unique_id_l, unique_id_r, match_key",
    )


def main() -> None:
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)

    # ------------------------------------------------------------------
    # Canonical run: the Splink tutorial flow end to end.
    # ------------------------------------------------------------------
    sql_log: list = []
    capture: dict = {}
    con, db_api = new_db_api(sql_log, capture)
    linker = make_linker(db_api)

    dump_settings_introspection(linker, OUT / "settings_introspection.json")
    linker.misc.save_model_to_json(OUT / "model_initial.json", overwrite=True)

    linker.training.estimate_probability_two_random_records_match(
        DETERMINISTIC_RULES, recall=RECALL
    )
    linker.misc.save_model_to_json(OUT / "model_after_prior.json", overwrite=True)

    linker.training.estimate_u_using_random_sampling(max_pairs=MAX_PAIRS_TUTORIAL)
    linker.misc.save_model_to_json(OUT / "model_after_u.json", overwrite=True)

    em1 = linker.training.estimate_parameters_using_expectation_maximisation(
        EM_RULE_1, estimate_without_term_frequencies=True
    )
    dump_em_session(em1, "em_session_1_first_name_surname")
    linker.misc.save_model_to_json(OUT / "model_after_em1.json", overwrite=True)

    em2 = linker.training.estimate_parameters_using_expectation_maximisation(
        EM_RULE_2, estimate_without_term_frequencies=True
    )
    dump_em_session(em2, "em_session_2_dob")
    linker.misc.save_model_to_json(OUT / "model_final.json", overwrite=True)

    write_sql_log(sql_log, "training")
    sql_log.clear()

    # Predict with debug_mode so every pipeline stage materialises as its own
    # physical table; debug_keep_temp_views leaves each stage's templated-name
    # temp view bound so we can export it afterwards. Blocked pairs are dropped
    # by predict itself, so they are captured at creation time instead.
    stage_dir = OUT / "stages"
    stage_dir.mkdir()
    capture["__splink__blocked_id_pairs"] = (
        stage_dir / "__splink__blocked_id_pairs.parquet",
        "join_key_l, join_key_r, match_key",
    )
    db_api.debug_mode = True
    db_api.debug_keep_temp_views = True
    predictions = linker.inference.predict()
    db_api.debug_mode = False
    db_api.debug_keep_temp_views = False
    capture.clear()

    stage_order = {
        "__splink__df_concat": "unique_id",
        "__splink__df_concat_with_tf": "unique_id",
        "blocked_with_cols": "unique_id_l, unique_id_r, match_key",
        "__splink__df_comparison_vectors": "unique_id_l, unique_id_r, match_key",
        "__splink__df_match_weight_parts": "unique_id_l, unique_id_r, match_key",
        "__splink__df_predict": "unique_id_l, unique_id_r, match_key",
    }
    for table, order_by in stage_order.items():
        try:
            export_table(con, f'"{table}"', stage_dir / f"{table}.parquet", order_by)
        except Exception as exc:
            print(f"WARNING: could not export stage {table}: {exc}")

    export_table(
        con,
        predictions.physical_name,
        OUT / "predictions.parquet",
        "unique_id_l, unique_id_r, match_key",
    )
    write_sql_log(sql_log, "predict")
    sql_log.clear()

    clusters = linker.clustering.cluster_pairwise_predictions_at_threshold(
        predictions, threshold_match_probability=CLUSTER_THRESHOLD
    )
    export_table(con, clusters.physical_name, OUT / "clusters.parquet", "unique_id")
    write_sql_log(sql_log, "cluster")
    sql_log.clear()

    # ------------------------------------------------------------------
    # Variant A: u estimation, simple path (single chunk, no early stop).
    # ------------------------------------------------------------------
    con_a, db_api_a = new_db_api()
    linker_a = make_linker(db_api_a)
    linker_a.training.estimate_probability_two_random_records_match(
        DETERMINISTIC_RULES, recall=RECALL
    )
    linker_a.training.estimate_u_using_random_sampling(
        max_pairs=MAX_PAIRS_TUTORIAL, num_chunks=1, min_count_per_level=None
    )
    linker_a.misc.save_model_to_json(OUT / "model_after_u_simple.json", overwrite=True)

    # ------------------------------------------------------------------
    # Variant B: u estimation with max_pairs below the pair count, so the
    # hash-based deterministic sampling filter actually fires. debug_mode
    # keeps __splink__df_concat_sample around so we can capture exactly
    # which records Splink sampled.
    # ------------------------------------------------------------------
    sql_log_b: list = []
    capture_b: dict = {
        "__splink__df_concat_sample": (OUT / "u_sample_ids.parquet", "unique_id"),
    }
    con_b, db_api_b = new_db_api(sql_log_b, capture_b)
    linker_b = make_linker(db_api_b)
    linker_b.training.estimate_probability_two_random_records_match(
        DETERMINISTIC_RULES, recall=RECALL
    )
    linker_b.training.estimate_u_using_random_sampling(max_pairs=MAX_PAIRS_SAMPLED)
    linker_b.misc.save_model_to_json(OUT / "model_after_u_sampled.json", overwrite=True)
    if not (OUT / "u_sample_ids.parquet").exists():
        print("WARNING: u_sample_ids.parquet was not captured")
    write_sql_log(sql_log_b, "u_sampled")

    # ------------------------------------------------------------------
    # Incremental linkage: match a new batch against the existing records
    # (predict_between) and within the new batch (predict_within), using a
    # trained model and term frequencies computed from the existing data.
    # ------------------------------------------------------------------
    gen_incremental_goldens()

    # ------------------------------------------------------------------
    # Link types: link_only and link_and_dedupe over two datasets.
    # ------------------------------------------------------------------
    gen_link_goldens("link_only")
    gen_link_goldens("link_and_dedupe")

    # ------------------------------------------------------------------
    # Accuracy evaluation from the ground-truth label column.
    # ------------------------------------------------------------------
    gen_accuracy_golden()
    gen_blocking_analysis_golden()
    gen_batch1_goldens()
    gen_batch2_goldens()

    # ------------------------------------------------------------------
    # Manifest.
    # ------------------------------------------------------------------
    files = {
        str(p.relative_to(OUT)): sha256(p)
        for p in sorted(OUT.rglob("*"))
        if p.is_file()
    }
    manifest = {
        "dataset": "fake_1000",
        "generated_with": {
            "python": sys.version.split()[0],
            "splink": splink.__version__,
            "duckdb": duckdb.__version__,
            "pandas": pd.__version__,
        },
        "parameters": {
            "recall": RECALL,
            "max_pairs_tutorial": MAX_PAIRS_TUTORIAL,
            "max_pairs_sampled": MAX_PAIRS_SAMPLED,
            "cluster_threshold": CLUSTER_THRESHOLD,
            "em_estimate_without_term_frequencies": True,
            "threads": 1,
        },
        "files": files,
    }
    (OUT / "MANIFEST.json").write_text(json.dumps(manifest, indent=2))
    print(f"OK: {len(files)} golden files written to {OUT}")


if __name__ == "__main__":
    main()
