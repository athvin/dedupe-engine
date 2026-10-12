//! DuckLake snapshot / time-travel e2e: write a table twice (two snapshots),
//! list snapshots, and read the table as of each version. Gated behind
//! DEDUPE_E2E_DUCKLAKE=1 (installs the ducklake extension).

use dedupe_core::DuckLakeConfig;
use dedupe_core::session::{Session, SessionOptions, TableHandle};

fn enabled() -> bool {
    std::env::var("DEDUPE_E2E_DUCKLAKE").as_deref() == Ok("1")
}

#[test]
fn snapshots_and_time_travel() {
    if !enabled() {
        eprintln!("skipping DuckLake snapshot e2e (set DEDUPE_E2E_DUCKLAKE=1 to run)");
        return;
    }

    let tmp = std::env::temp_dir().join(format!("dedupe_snap_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("data")).unwrap();

    let mut session = Session::open_in_memory(SessionOptions::default()).unwrap();
    session
        .attach_ducklake(&DuckLakeConfig {
            metadata: tmp.join("c.ducklake").to_string_lossy().into_owned(),
            data_path: Some(tmp.join("data").to_string_lossy().into_owned()),
            ..Default::default()
        })
        .unwrap();

    // Two writes = two snapshots, with different row counts.
    session
        .execute_batch("CREATE TEMP TABLE stage_v1 AS SELECT * FROM (VALUES (1),(2)) v(x)")
        .unwrap();
    let h1 = TableHandle {
        templated: "stage_v1".into(),
        physical: "stage_v1".into(),
        columns: vec!["x".into()],
    };
    let v1 = session.write_to_lake(&h1, "t").unwrap();

    session
        .execute_batch("CREATE TEMP TABLE stage_v2 AS SELECT * FROM (VALUES (1),(2),(3),(4)) v(x)")
        .unwrap();
    let h2 = TableHandle {
        templated: "stage_v2".into(),
        physical: "stage_v2".into(),
        columns: vec!["x".into()],
    };
    let v2 = session.write_to_lake(&h2, "t").unwrap();

    assert!(
        v2 > v1,
        "second write must create a later snapshot ({v1} -> {v2})"
    );

    let snaps = session.list_snapshots().unwrap();
    assert!(snaps.len() >= 2, "expected at least two snapshots");
    assert_eq!(snaps.last().unwrap().version, v2);

    // Time-travel: the table at v1 has 2 rows, at v2 has 4.
    let at_v1 = session
        .register_lake_table_at_version("t", "t_v1", v1)
        .unwrap();
    let at_v2 = session
        .register_lake_table_at_version("t", "t_v2", v2)
        .unwrap();
    assert_eq!(session.count(&at_v1.physical).unwrap(), 2);
    assert_eq!(session.count(&at_v2.physical).unwrap(), 4);

    let _ = std::fs::remove_dir_all(&tmp);
}
