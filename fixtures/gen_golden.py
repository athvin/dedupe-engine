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
