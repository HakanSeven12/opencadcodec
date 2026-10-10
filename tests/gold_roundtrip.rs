//! Integration test for the gold-vs-silver roundtrip harness.
//!
//! Gated behind the `gold-harness` feature. Uses the env vars when present:
//!   - GOLD_DWGREAD: path to a built libredwg `dwgread` binary.
//!   - GOLD_TESTDATA: path to libredwg `test/test-data` directory.
//!
//! The whole file is feature-gated (not just the test body), so a plain
//! `cargo test` compiles an empty 0-test target here instead of warning
//! about the oracle helpers as dead code.
//!
//! Oracle-optional: `cargo test` must work on machines without a LibreDWG
//! checkout. When the oracle is unavailable the test SKIPS PASS with a
//! written notice in `target/gold_harness_oracle_skipped.txt` (Rust
//! suppresses passing tests' output, so the marker file carries the
//! message). Set GOLD_HARNESS_REQUIRE=1 to turn absence into a hard
//! failure instead (for CI). To check out and build the oracle on demand:
//! `bash tests/gold_harness/harness/bootstrap_oracle.sh`.
//!
//! With the oracle present it shells out to `tests/gold_harness/harness/run_roundtrip.py`.
//! By default only checks that the harness runs without crashing. With
//! GOLD_HARNESS_STRICT=1 it asserts zero `missing_in_silver` diffs for the
//! representative subset and asserts that the storage-only `EntityCommon`
//! fields (`z_is_zero`, `ltype_flags`, `prev_entity`, `next_entity`,
//! `nolinks`) never appear in the read-fidelity or rewrite-fidelity diffs.

#![cfg(feature = "gold-harness")]

use std::path::{Path, PathBuf};
use std::process::Command;

const DRIVER: &str = "tests/gold_harness/harness/run_roundtrip.py";

fn cargo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The reason the gold oracle is unavailable, when it is.
fn oracle_available() -> Result<(), String> {
    let dwgread = match std::env::var_os("GOLD_DWGREAD") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => return Err("GOLD_DWGREAD is not set".to_string()),
    };
    // Spawning doubles as the executability check: a spawn error means the
    // path does not point at a runnable binary (missing checkout, wrong
    // architecture, ...). The exit status is irrelevant here.
    match Command::new(&dwgread).arg("--help").output() {
        Ok(_) => {}
        Err(e) => {
            return Err(format!(
                "GOLD_DWGREAD ('{}') is not executable: {}",
                dwgread.display(),
                e
            ))
        }
    }
    let testdata = match std::env::var_os("GOLD_TESTDATA") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => return Err("GOLD_TESTDATA is not set".to_string()),
    };
    if !testdata.join("2000").is_dir() {
        return Err(format!(
            "GOLD_TESTDATA ('{}') has no 2000/ corpus folder",
            testdata.display()
        ));
    }
    Ok(())
}

fn oracle_skip_marker_path() -> PathBuf {
    cargo_root().join("target").join("gold_harness_oracle_skipped.txt")
}

fn run_harness(input: &Path, workdir: &Path) -> std::process::Output {
    let script = cargo_root().join(DRIVER);
    let mut cmd = Command::new("python3");
    cmd.arg(&script)
        .arg(input)
        .arg(workdir)
        .current_dir(cargo_root());
    if let Some(v) = std::env::var_os("GOLD_DWGREAD") {
        cmd.env("GOLD_DWGREAD", v);
    }
    if let Some(v) = std::env::var_os("GOLD_TESTDATA") {
        cmd.env("GOLD_TESTDATA", v);
    }
    cmd.output().expect("failed to execute harness driver")
}

/// Fields that come from `EntityCommon` storage-only values. They must never
/// appear in a read-fidelity diff because they are round-trip metadata, not
/// application-visible entity state.
const PROHIBITED_COMMON_FIELDS: &[&str] = &[
    "z_is_zero",
    "ltype_flags",
    "prev_entity",
    "next_entity",
    "nolinks",
];

#[cfg(feature = "gold-harness")]
#[test]
fn gold_harness_runs_on_representative_files() {
    // Oracle-optional: without LibreDWG the harness cannot run; skip pass
    // with a written notice so `cargo test` stays green on oracle-free
    // machines, unless GOLD_HARNESS_REQUIRE demands the real run (CI).
    if let Err(reason) = oracle_available() {
        let marker = oracle_skip_marker_path();
        let _ = std::fs::write(
            &marker,
            format!(
                "gold oracle unavailable: {}\n\
                 the gold-vs-silver fidelity check did NOT run.\n\
                 bootstrap the oracle with:  bash tests/gold_harness/harness/bootstrap_oracle.sh\n\
                 then re-run with GOLD_DWGREAD/GOLD_TESTDATA exported.\n",
                reason
            ),
        );
        if std::env::var_os("GOLD_HARNESS_REQUIRE").is_some() {
            panic!(
                "gold oracle required (GOLD_HARNESS_REQUIRE=1) but unavailable: {} \
                 — bootstrap with: bash tests/gold_harness/harness/bootstrap_oracle.sh",
                reason
            );
        }
        return;
    }
    let _ = std::fs::remove_file(oracle_skip_marker_path());

    let testdata = std::env::var_os("GOLD_TESTDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env!("CARGO_MANIFEST_DIR")));

    let representative = [
        testdata.join("2000/Line.dwg"),
        testdata.join("2000/circle.dwg"),
    ];
    let out_dir = std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| cargo_root().join("target").join("gold_harness_test"));
    let workdir = out_dir.join("gold_harness_test");

    for file in representative {
        if !file.exists() {
            eprintln!("skipping missing gold file: {}", file.display());
            continue;
        }
        let out = run_harness(&file, &workdir.join(file.file_stem().unwrap()));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "harness failed for {}\nstdout:\n{}\nstderr:\n{}",
            file.display(),
            stdout,
            stderr
        );

        let diff_paths = [
            workdir.join(file.file_stem().unwrap()).join(format!(
                "{}_diff_orig.json",
                file.file_stem().unwrap().to_string_lossy()
            )),
            workdir.join(file.file_stem().unwrap()).join(format!(
                "{}_diff_rt.json",
                file.file_stem().unwrap().to_string_lossy()
            )),
        ];

        for diff_path in &diff_paths {
            assert!(
                diff_path.exists(),
                "diff file missing for {}: {}",
                file.display(),
                diff_path.display()
            );
            let diff: serde_json::Value =
                serde_json::from_reader(std::fs::File::open(diff_path).unwrap()).unwrap();
            let diffs = diff.get("diffs").and_then(|v| v.as_array()).unwrap();

            let prohibited: Vec<&serde_json::Value> = diffs
                .iter()
                .filter(|d| {
                    d.get("field")
                        .and_then(|f| f.as_str())
                        .map(|f| PROHIBITED_COMMON_FIELDS.contains(&f))
                        .unwrap_or(false)
                })
                .collect();
            assert!(
                prohibited.is_empty(),
                "{} contains prohibited EntityCommon fields for {}: {:?}",
                diff_path.file_stem().unwrap().to_string_lossy(),
                file.display(),
                prohibited
            );
        }

        if std::env::var_os("GOLD_HARNESS_STRICT").is_some() {
            let diff: serde_json::Value =
                serde_json::from_reader(std::fs::File::open(&diff_paths[0]).unwrap()).unwrap();
            let diffs = diff.get("diffs").and_then(|v| v.as_array()).unwrap();
            let missing: Vec<&serde_json::Value> = diffs
                .iter()
                .filter(|d| d.get("kind").and_then(|k| k.as_str()) == Some("missing_in_silver"))
                .collect();
            assert!(
                missing.is_empty(),
                "missing_in_silver diffs for {}: {:?}",
                file.display(),
                missing
            );
        }
    }
}
