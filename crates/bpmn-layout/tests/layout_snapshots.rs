//! Reviewable native snapshots (TEST-005 §Snapshots): committed canonical JSON
//! under `tests/snapshots/` compared exactly. Snapshot changes require PR
//! review with before/after visualization (TASK-018 §8.3 step 5).
//!
//! Regenerate deliberately with:
//! `cargo test -p bpmn-layout --test layout_snapshots -- --ignored regenerate_snapshots`

use std::path::PathBuf;

mod common;

fn snapshots_dir() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

fn snapshot_path(stem: &str) -> PathBuf {
    snapshots_dir().join(format!("{stem}.json"))
}

#[test]
fn snapshots_match_approved_baselines() {
    for (stem, process_id) in common::CORE_FIXTURES {
        let outcome = common::layout_core(stem, process_id);
        let actual = serde_json::to_string_pretty(&common::canonical(&outcome)).expect("serialize");
        let path = snapshot_path(stem);
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "read {}: {error} (regenerate with `cargo test -p bpmn-layout --test layout_snapshots -- --ignored regenerate_snapshots` and review the diff)",
                path.display()
            )
        });
        let expected_value: serde_json::Value =
            serde_json::from_str(&expected).expect("snapshot parses");
        let actual_value: serde_json::Value =
            serde_json::from_str(&actual).expect("canonical parses");
        assert_eq!(
            expected_value, actual_value,
            "{stem}: snapshot mismatch; regenerate with `--ignored regenerate_snapshots`, review the diff and attach before/after renders to the PR"
        );
    }
}

#[test]
#[ignore = "deliberate snapshot regeneration; requires review of the diff"]
fn regenerate_snapshots() {
    std::fs::create_dir_all(snapshots_dir()).expect("create snapshots dir");
    for (stem, process_id) in common::CORE_FIXTURES {
        let outcome = common::layout_core(stem, process_id);
        let json = serde_json::to_string_pretty(&common::canonical(&outcome)).expect("serialize");
        let path = snapshot_path(stem);
        std::fs::write(&path, format!("{json}\n")).expect("write snapshot");
        println!("wrote {}", path.display());
    }
}
