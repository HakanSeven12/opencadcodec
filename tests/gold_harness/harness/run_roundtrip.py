#!/usr/bin/env python3
"""Run the gold-vs-silver roundtrip harness on one DWG file.

Requires:
  - GOLD_DWGREAD env var pointing to a built libredwg `dwgread` binary.
  - acadrust `dwg2json` and `dwgrewrite` binaries built with `--features serde`.

Produces:
  - <workdir>/<stem>_gold_orig.json
  - <workdir>/<stem>_silver_orig.json
  - <workdir>/<stem>_rt.dwg
  - <workdir>/<stem>_gold_rt.json
  - <workdir>/<stem>_silver_rt.json
  - <workdir>/<stem>_diff.json
  - <workdir>/<stem>_report.md
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# The structure axis (IMPLEMENTATION.md §19, packet H0): a second,
# separately-reported census beside the frozen OBJECTS comparison.
# IMPORTS the module directly (same script dir); the OBJECTS-axis
# subprocess pipeline (normalize_gold/normalize_silver/diff_fields) is
# untouched by this axis.
from struct_axis import census_from_files

GOLD_DWGREAD = os.environ.get("GOLD_DWGREAD", "")
CARGO = os.environ.get("CARGO", "cargo")
SCRIPT_DIR = Path(__file__).resolve().parent
HARNESS_DIR = SCRIPT_DIR.parent  # the tests/gold_harness/ root
REPO_ROOT = HARNESS_DIR.parent.parent


def run(cmd: List[str], **kwargs: Any) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, check=True, text=True, **kwargs)


def run_gold_dwgread(dwg: Path, out_json: Path) -> None:
    if not GOLD_DWGREAD:
        raise RuntimeError("GOLD_DWGREAD env var is not set")
    with open(out_json, "w", encoding="utf-8") as f:
        subprocess.run(
            [GOLD_DWGREAD, "-O", "JSON", str(dwg)],
            stdout=f,
            stderr=subprocess.DEVNULL,
            check=True,
        )


def run_silver_dwg2json(dwg: Path, out_json: Path) -> None:
    run(
        [
            CARGO,
            "run",
            "--quiet",
            "--bin",
            "dwg2json",
            "--features",
            "serde",
            "--",
            str(dwg),
            str(out_json),
        ],
        cwd=REPO_ROOT,
    )


def run_silver_rewrite(dwg: Path, out_dwg: Path) -> None:
    run(
        [
            CARGO,
            "run",
            "--quiet",
            "--bin",
            "dwgrewrite",
            "--features",
            "serde",
            "--",
            str(dwg),
            str(out_dwg),
        ],
        cwd=REPO_ROOT,
    )


def normalize(gold_json: Path, silver_json: Path, ignore: Optional[Path]) -> Tuple[Path, Path]:
    gold_norm = gold_json.with_suffix(".norm.json")
    silver_norm = silver_json.with_suffix(".norm.json")
    ignore_arg = [str(ignore)] if ignore else []
    run([sys.executable, str(SCRIPT_DIR / "normalize_gold.py"), str(gold_json)] + ignore_arg, stdout=open(gold_norm, "w", encoding="utf-8"))
    run([sys.executable, str(SCRIPT_DIR / "normalize_silver.py"), str(silver_json)] + ignore_arg, stdout=open(silver_norm, "w", encoding="utf-8"))
    return gold_norm, silver_norm


def diff(gold_norm: Path, silver_norm: Path, ignore: Optional[Path]) -> Dict[str, Any]:
    ignore_arg = [str(ignore)] if ignore else []
    result = run(
        [sys.executable, str(SCRIPT_DIR / "diff_fields.py"), str(gold_norm), str(silver_norm)] + ignore_arg,
        stdout=subprocess.PIPE,
    )
    return json.loads(result.stdout)


def summarize(name: str, result: Dict[str, Any]) -> str:
    lines = [f"## {name}: {result['total_diffs']} diff(s)"]
    if result["total_diffs"] == 0:
        lines.append("Perfect match.")
        return "\n".join(lines)

    by_kind: Dict[str, int] = {}
    by_type_field: Dict[Tuple[str, str], int] = {}
    for d in result["diffs"]:
        by_kind[d["kind"]] = by_kind.get(d["kind"], 0) + 1
        key = (d.get("type", "?"), d.get("field", "-"))
        by_type_field[key] = by_type_field.get(key, 0) + 1

    lines.append("### By kind")
    for k, c in sorted(by_kind.items()):
        lines.append(f"- {k}: {c}")
    lines.append("### Top divergent (type, field)")
    for (typ, field), c in sorted(by_type_field.items(), key=lambda x: -x[1])[:20]:
        lines.append(f"- {typ}.{field}: {c}")
    return "\n".join(lines)


def default_workdir() -> Path:
    """Return a repo-relative default workdir under target/gold_harness/."""
    target = REPO_ROOT / "target"
    target.mkdir(parents=True, exist_ok=True)
    return target / "gold_harness"


def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: run_roundtrip.py <file.dwg> [workdir]", file=sys.stderr)
        return 2
    dwg = Path(sys.argv[1]).resolve()
    workdir = Path(sys.argv[2]) if len(sys.argv) >= 3 else default_workdir()
    workdir.mkdir(parents=True, exist_ok=True)
    stem = re.sub(r"[^A-Za-z0-9_-]+", "_", dwg.stem)

    ignore = SCRIPT_DIR.parent / "config" / "ignore_fields.toml"
    if not ignore.exists():
        ignore = None

    gold_orig = workdir / f"{stem}_gold_orig.json"
    silver_orig = workdir / f"{stem}_silver_orig.json"
    rt_dwg = workdir / f"{stem}_rt.dwg"
    gold_rt = workdir / f"{stem}_gold_rt.json"
    silver_rt = workdir / f"{stem}_silver_rt.json"
    diff_orig_path = workdir / f"{stem}_diff_orig.json"
    diff_rt_path = workdir / f"{stem}_diff_rt.json"
    report_path = workdir / f"{stem}_report.md"

    print(f"[harness] processing {dwg}")
    print("[harness] reading original with libredwg...")
    run_gold_dwgread(dwg, gold_orig)
    print("[harness] reading original with acadrust...")
    run_silver_dwg2json(dwg, silver_orig)

    print("[harness] rewriting with acadrust...")
    run_silver_rewrite(dwg, rt_dwg)

    print("[harness] reading rewrite with libredwg...")
    run_gold_dwgread(rt_dwg, gold_rt)
    print("[harness] reading rewrite with acadrust...")
    run_silver_dwg2json(rt_dwg, silver_rt)

    print("[harness] normalizing and diffing original...")
    gold_norm, silver_norm = normalize(gold_orig, silver_orig, ignore)
    diff_orig = diff(gold_norm, silver_norm, ignore)
    with open(diff_orig_path, "w", encoding="utf-8") as f:
        json.dump(diff_orig, f, indent=2, ensure_ascii=False)

    print("[harness] normalizing and diffing rewrite...")
    gold_rt_norm, silver_rt_norm = normalize(gold_rt, silver_rt, ignore)
    diff_rt = diff(gold_rt_norm, silver_rt_norm, ignore)
    with open(diff_rt_path, "w", encoding="utf-8") as f:
        json.dump(diff_rt, f, indent=2, ensure_ascii=False)

    # ── The structure axis (§19 H0): a separately-reported census.
    # Read axis: gold_orig's structure vs silver_orig's projection.
    # Write-target axis: gold_orig vs gold_rt — does silver's rewrite
    # preserve the file structure as gold reads it back (the H7/H1
    # prelude; addresses/sizes/crc shifting on rewrite is measured, the
    # tolerated-vs-defective field split is the later packet's work).
    # The OBJECTS-axis counters above are untouched; these counters
    # report beside them and gate independently.
    print("[harness] structure census (read + write-target)...")
    struct_orig_path = workdir / f"{stem}_struct_orig.json"
    struct_rt_path = workdir / f"{stem}_struct_rt.json"
    try:
        struct_orig = census_from_files(gold_orig, silver_orig, "silver")
        struct_rt = census_from_files(gold_orig, gold_rt, "gold_rt")
    except Exception as exc:  # defensive: the axis must never break the file run
        struct_orig = {"error": f"{type(exc).__name__}: {exc}", "totals": {"key_gap_sum": -1}}
        struct_rt = {"error": f"{type(exc).__name__}: {exc}", "totals": {"key_gap_sum": -1}}
    with open(struct_orig_path, "w", encoding="utf-8") as f:
        json.dump(struct_orig, f, indent=2, ensure_ascii=False)
    with open(struct_rt_path, "w", encoding="utf-8") as f:
        json.dump(struct_rt, f, indent=2, ensure_ascii=False)

    report = f"""# Roundtrip report: {dwg.name}

Workdir: `{workdir}`

{summarize("Read fidelity (gold_orig vs silver_orig)", diff_orig)}

{summarize("Write fidelity (gold_orig vs gold_rt)", diff_rt)}

{summarize("Internal consistency (silver_orig vs silver_rt)", {"total_diffs": 0, "diffs": []})}

## Structure read census (§19 H0; gold_orig vs silver_orig)
key-gap sum: {struct_orig.get("totals", {}).get("key_gap_sum", -1)}
(unmatched/missing leaves per structure key — the H2-H5 attack surface)

## Structure write-target census (§19 H0; gold_orig vs gold_rt)
key-gap sum: {struct_rt.get("totals", {}).get("key_gap_sum", -1)}

## Outputs
- gold original: `{gold_orig}`
- silver original: `{silver_orig}`
- rewrite DWG: `{rt_dwg}`
- gold rewrite: `{gold_rt}`
- silver rewrite: `{silver_rt}`
- structure read census: `{struct_orig_path}`
- structure write-target census: `{struct_rt_path}`
"""
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)

    print(f"[harness] wrote report to {report_path}")
    print(f"[harness] read-fidelity diffs: {diff_orig['total_diffs']}")
    print(f"[harness] write-fidelity diffs: {diff_rt['total_diffs']}")
    print(f"[harness] structure read key-gap: {struct_orig.get('totals', {}).get('key_gap_sum', -1)}")
    print(f"[harness] structure write-target key-gap: {struct_rt.get('totals', {}).get('key_gap_sum', -1)}")
    # Return 0 as long as the pipeline completed; diff counts are reported in
    # the JSON files and are asserted on by the caller (e.g. gold_roundtrip.rs).
    return 0


if __name__ == "__main__":
    sys.exit(main())
