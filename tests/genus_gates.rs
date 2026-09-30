//! The §20 genus-gates cargo mirror (feature-gated like gold_roundtrip.rs).
//!
//! Runs the constructed-content oracle end to end:
//!   1. a fresh expectation extraction (genus_extract.py) over the sh_history
//!      specimen family, compared against the pinned
//!      tests/gold_harness/config/genus_expectations.json — expectation drift is
//!      itself reviewable (§20.3), so a mismatch fails here until the pin
//!      is consciously regenerated;
//!   2. the gate run (genus_gates.py) over the constructed corpus, with the
//!      report asserted well-formed: the three ranked sections exist and
//!      every row is (type, field, count ≥ 1) shaped.
//!
//! The counts are NOT asserted zero — the gates land nonzero on purpose;
//! the initial counts are the work queue (§20.3).
//!
//! Presence-gated (the corpus fixtures may be absent in some
//! environments): without the specimen family, the pinned expectations,
//! or python3, the test SKIPS PASS with a written notice in
//! target/genus_gates_skipped.txt (Rust suppresses passing tests' output,
//! so the marker file carries the message). Set GOLD_HARNESS_REQUIRE=1
//! to turn absence into a hard failure instead (for CI).

#![cfg(feature = "gold-harness")]

use std::path::{Path, PathBuf};
use std::process::Command;

fn cargo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn skip_marker_path() -> PathBuf {
    cargo_root().join("target").join("genus_gates_skipped.txt")
}

fn python3_available() -> bool {
    Command::new("python3").arg("--version").output().is_ok()
}

/// The presence gate: everything the genus pipeline needs. Returns the
/// reason it is unavailable, when it is.
fn genus_available() -> Result<(), String> {
    let fixtures = cargo_root().join("tests/gold_harness/fixtures/sh_history");
    let has_specimen = std::fs::read_dir(&fixtures)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.path().extension().map(|ext| ext == "dwg").unwrap_or(false))
        })
        .unwrap_or(false);
    if !has_specimen {
        return Err(format!(
            "the sh_history specimen family is absent at {}",
            fixtures.display()
        ));
    }
    let pin = cargo_root().join("tests/gold_harness/config/genus_expectations.json");
    if !pin.is_file() {
        return Err(format!(
            "the pinned genus expectations are absent at {} — run genus_extract.py",
            pin.display()
        ));
    }
    if !python3_available() {
        return Err("python3 is not runnable".to_string());
    }
    Ok(())
}

fn run_python(script: &Path, args: &[&str], workdir: &Path) -> std::process::Output {
    let mut cmd = Command::new("python3");
    cmd.arg(script).args(args).current_dir(workdir);
    cmd.output().expect("failed to spawn the genus pipeline")
}

#[cfg(feature = "gold-harness")]
#[test]
fn genus_gates_pipeline_runs_and_expectations_match_pin() {
    if let Err(reason) = genus_available() {
        let marker = skip_marker_path();
        let _ = std::fs::write(
            &marker,
            format!(
                "genus pipeline unavailable: {}\n\
                 the §20 constructed-content oracle did NOT run.\n\
                 restore the specimen family + the pinned expectations\n\
                 (tests/gold_harness/genus/genus_extract.py regenerates the pin).\n",
                reason
            ),
        );
        if std::env::var_os("GOLD_HARNESS_REQUIRE").is_some() {
            panic!(
                "genus pipeline required (GOLD_HARNESS_REQUIRE=1) but unavailable: {}",
                reason
            );
        }
        return;
    }
    let _ = std::fs::remove_file(skip_marker_path());

    let root = cargo_root();
    let workdir = root.join("target").join("genus_gates_test");
    std::fs::create_dir_all(&workdir).expect("create the genus test workdir");

    // 1. Fresh extraction; must equal the pinned copy (parsed-equal — the
    //    pin regenerates identically in every checkout because it is
    //    fixtures-only).
    let fresh = workdir.join("genus_expectations_fresh.json");
    let fresh_str = fresh.to_string_lossy().to_string();
    let extract = run_python(
        &root.join("tests/gold_harness/genus/genus_extract.py"),
        &["--out", &fresh_str],
        &root,
    );
    assert!(
        extract.status.success(),
        "genus_extract.py failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&extract.stdout),
        String::from_utf8_lossy(&extract.stderr)
    );
    let fresh_json: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&fresh).unwrap()).unwrap();
    let pin: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(root.join("tests/gold_harness/config/genus_expectations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fresh_json, pin,
         "the pinned genus expectations drifted from a fresh extraction — \
          regenerate tests/gold_harness/config/genus_expectations.json with \
          genus_extract.py and review the diff (§20.3: expectation drift \
          is reviewable)"
    );

    // 2. The gate run over the constructed corpus.
    let report_path = workdir.join("genus_report.json");
    let workdir_str = workdir.to_string_lossy().to_string();
    let gates = run_python(
        &root.join("tests/gold_harness/genus/genus_gates.py"),
        &["--workdir", &workdir_str],
        &root,
    );
    assert!(
        gates.status.success(),
        "genus_gates.py failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&gates.stdout),
        String::from_utf8_lossy(&gates.stderr)
    );
    assert!(
        report_path.exists(),
        "the genus report is missing after a successful gate run"
    );

    // 3. The report shape: three ranked sections, every row
    //    (type, field, count ≥ 1). The counts themselves are the work
    //    queue — deliberately not asserted zero (§20.3).
    let report: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&report_path).unwrap()).unwrap();
    for section in ["sab_form_diffs", "sh_genus_diffs", "acds_genus_diffs"] {
        let rows = report
            .get(section)
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("the genus report lacks the {section} section"));
        for row in rows {
            let count = row
                .get("count")
                .and_then(|c| c.as_u64())
                .unwrap_or_else(|| panic!("a {section} row lacks a numeric count: {row}"));
            assert!(count >= 1, "a {section} row carries count 0: {row}");
            assert!(
                row.get("type").is_some() && row.get("field").is_some(),
                "a {section} row lacks the (type, field) shape: {row}"
            );
        }
    }
}
