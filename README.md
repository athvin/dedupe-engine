# dedupe-engine

A Rust reimplementation of [Splink](https://github.com/moj-analytical-services/splink)'s
Fellegi-Sunter probabilistic record-linkage engine, narrowly scoped to a single
execution target: **DuckDB / DuckLake**. There is no multi-engine abstraction —
Spark, Athena, SQLite and Postgres backends are out of scope.

**Parity contract:** given the same settings JSON and input data, this engine
reproduces Python **Splink 5.0.0**'s DuckDB outputs exactly — trained
parameters, match weights/probabilities, and clusters — verified by committed
golden fixtures. Every milestone has a parity test that diffs against Splink.

## What it does (v1)

The full core linkage pipeline, matching Splink's canonical workflow:

1. **Prior estimation** — `probability_two_random_records_match` from
   deterministic rules and an estimated recall.
2. **u estimation** — deterministic hash-sampled random pairs (Splink 5's
   sampling), per-comparison block-and-count with RHS chunking + early stopping.
3. **EM training of m** — one session per training blocking rule, with
   blocking-adjusted priors and median combination across sessions.
4. **`predict()`** — pairwise match weights/probabilities with term-frequency
   adjustments, scored additively in log2 space (`mw_` columns).
5. **Clustering** — connected components (min-representative propagation).
6. **Incremental linkage** — match a new batch of records against an existing
   dataset without re-running the whole pipeline. Mirrors Splink 5's
   `predict_between` (new × existing, cross pairs only) and `predict_within`
   (within the new batch), scoring with the trained model and term
   frequencies from the established data.
7. **Incremental clustering** — fold a new batch's edges into an existing
   cluster assignment without re-running connected components over the whole
   graph (contract existing clusters to super-nodes, run CC over just the new
   edges, expand). Splink has no such API, but the result is provably
   identical to a full re-cluster of the combined edge set.
8. **Link types** — `link_only` and `link_and_dedupe` across multiple
   datasets (composite `source_dataset`+`unique_id` keys), through the full
   train/predict/cluster pipeline.
9. **Accuracy evaluation** — a truth-space table from a ground-truth label
   column: per-threshold confusion matrix and metrics (precision, recall,
   F-scores, specificity, NPV, accuracy, φ, ROC rates). Data, not charts.
10. **Blocking analysis** — per-rule comparison counts (marginal + cumulative)
    vs the total possible, a pre-predict tractability check.
11. **DuckLake snapshots / time-travel** — every lake write commits a snapshot;
    list snapshots and read inputs as of any version (`--as-of-version`).
    This is DuckLake-native and has no Splink equivalent.
12. **Serving & explainability** — `deterministic_link` (rule-only linkage),
    `score_pairs` (cartesian scoring of record sets, no blocking),
    `TrainedModel::detailed_records` (trained m/u/Bayes-factor/match-weight per
    level as data), and `n_largest_blocks` (which blocking-key values dominate).
13. **Clustering QA** — graph metrics (node degree/centrality, cluster
    size/density/centralisation), `cluster_using_single_best_links` (constrained
    one-to-one), and multi-threshold cluster summary stats.
14. **Evaluation depth** — `prediction_errors_from_labels_{column,table}`
    (FP/FN inspection), accuracy truth-space from a labels table, and
    `unlinkables_data`.
15. **Profiling & label training** — column completeness and value-frequency
    distributions; `estimate_m_from_label_column` / `estimate_m_from_pairwise_labels`.

16. **Exploding / array blocking rules** — `arrays_to_explode` blocking rules
    (UNNEST an array column, block on shared elements) with the full Splink
    mechanism: per-rule marginal exploded id-pairs tables, the preceding-rule
    `false` exclusion, and the `min(match_key)` two-stage dedup.
17. **Edge `is_bridge` metric** — `clustering::edge_bridges` flags edges whose
    removal disconnects a cluster, implemented Rust-native via `petgraph` (no
    `igraph` dependency) and golden-tested against Splink-with-igraph.

Everything is emitted as **data** (tables/records) — no charts or dashboards
(this is a backend engine; a separate frontend consumes the data). Splink's
`*_chart` / `*_dashboard` / labelling-tool methods are intentionally not ported.

## Layout

```
crates/dedupe-core   # the engine library
crates/dedupe-cli    # `dedupe` binary: train / predict / cluster
fixtures/            # pinned Python project that generates the golden spec
```

The engine is a **SQL generator**: it builds the same `__splink__*` CTE
pipeline Splink does and executes it on one DuckDB connection. All
intermediates are native temp tables; DuckLake is used only to read inputs and
write final outputs (it cannot host temp tables), with writes in dedicated
single-catalog transactions.

## Performance & scaling

Because DuckDB does the compute, scaling across CPUs and bounding memory are
configuration, not code: `SessionOptions` exposes `threads`, `memory_limit`,
`temp_directory` (disk spilling for larger-than-RAM workloads), and
`preserve_insertion_order`, with `parity()` / `performance()` presets (CLI:
`--threads`, `--memory-limit`, `--temp-dir`). On an 8-core box the scoring-heavy
predict path scales ~3.2×; output-bound blocking does not. When a candidate set
is too large to score at once, `dedupe predict --num-chunks N` scores it in N
bounded-memory hash passes with an identical result (add `--stream` to write
each pass straight to the lake for constant memory end to end). DuckLake has no secondary
indexes, but `write_to_lake_with` supports `partition_by` and `order_by` (CLI
`--partition-by` / `--order-by`) for read pruning and clustering-key-style
zone-map tightening, and `dedupe maintenance` runs catalog compaction / snapshot
expiry / delete-file rewrite. Benchmarks (`cargo bench`), the profiling harness
(`examples/profile_predict.rs`), and the full write-up are in
[PERFORMANCE.md](PERFORMANCE.md).

## Usage

```sh
# Train against a DuckLake catalog
dedupe train \
  --settings settings.json \
  --catalog catalog.ducklake --data-path ./data --input people \
  --deterministic-rule 'l."first_name" = r."first_name" AND l."dob" = r."dob"' \
  --recall 0.7 \
  --em-rule 'l."first_name" = r."first_name" AND l."surname" = r."surname"' \
  --em-rule 'l."dob" = r."dob"' \
  --output-model model.json

dedupe predict --model model.json --catalog catalog.ducklake --data-path ./data \
  --input people --output predictions

dedupe cluster --model model.json --catalog catalog.ducklake --data-path ./data \
  --input people --predictions predictions --threshold 0.95 --output clusters

# Incremental: match a new batch against the existing dataset (and within
# itself), using the trained model and the established data's term frequencies.
dedupe link-new --model model.json \
  --catalog catalog.ducklake --data-path ./data \
  --input people --new-input new_arrivals \
  --scope both --output incremental_predictions

# Fold those new predictions into the existing clusters (no full re-cluster).
dedupe cluster-new --model model.json \
  --catalog catalog.ducklake --data-path ./data \
  --input all_people --existing-clusters clusters \
  --predictions incremental_predictions --threshold 0.95 --output clusters

# Link two datasets (link_only / link_and_dedupe).
dedupe train --settings settings.json --catalog catalog.ducklake --data-path ./data \
  --dataset people_a=table_a --dataset people_b=table_b \
  --deterministic-rule '...' --em-rule '...' --output-model model.json

# Evaluate accuracy against a ground-truth label column.
dedupe accuracy --model model.json --input-csv people.csv \
  --label-column true_entity --output-parquet truth_space.parquet

# Check blocking-rule cardinality before predicting.
dedupe count-comparisons --settings settings.json --input people \
  --blocking-rule 'l."first_name" = r."first_name"' --catalog catalog.ducklake

# DuckLake snapshots / time travel.
dedupe snapshots --catalog catalog.ducklake --data-path ./data
dedupe predict --model model.json --catalog catalog.ducklake --data-path ./data \
  --input people --as-of-version 3 --output predictions
```

`--input-csv` / `--input-parquet` and `--output-parquet` bypass the lake for
quick local runs. A trained model is written as Splink-compatible
`save_model_to_json` output, so it round-trips with Python Splink.

## Testing & parity

```sh
cargo test                          # all parity gates, no Python needed
DEDUPE_E2E_DUCKLAKE=1 cargo test    # also the DuckLake round-trip e2e
DEDUPE_SCALE_TESTS=1 cargo test \
  -p dedupe-core --test parity_historical_50k   # full pipeline on 50k records (~2 min)
```

Parity is checked on two datasets: `fake_1000` (fast, every feature) and
`historical_50k` (50,578 records, richer schema and high null rates — train +
predict + cluster at scale, gated behind `DEDUPE_SCALE_TESTS=1`).

Golden fixtures live in `fixtures/golden/` and are regenerated by running real
Splink:

```sh
cd fixtures && uv sync --frozen && uv run gen_golden.py
```

`fixtures/pyproject.toml` pins `splink==5.0.0` and `duckdb==1.5.5` (the version
in Splink 5's own lockfile). `dedupe-core` bundles DuckDB 1.5.5 via the
`duckdb` crate so the engine and the golden generator execute on identical
DuckDB builds. A `MANIFEST.json` records sha256 hashes of every golden; a test
fails if any golden drifts.

## Intentional divergences from Splink

- No implicit caching (Splink 5 removed it too) — explicit table handles with
  run-scoped physical names.
- Idiomatic Rust errors (`thiserror`) instead of Python exceptions; Arrow/SQL
  rather than a pandas-style dataframe layer.
- `sqlparser` (DuckDB dialect) is used only to extract column names and detect
  exact-match levels from raw `sql_condition` strings — never to transpile.
