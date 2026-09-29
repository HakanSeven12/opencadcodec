#!/usr/bin/env python3
"""Batch driver for the gold-vs-silver roundtrip harness.

Runs `run_roundtrip.py` over the in-scope corpus and aggregates a
`report.json`/`report.md` ranking divergent (entity_type, field) pairs.
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent

GOLD_TESTDATA = os.environ.get("GOLD_TESTDATA", str(Path.home() / "work/libredwg/test/test-data"))


def in_scope_files(testdata: Path) -> List[Path]:
    files: List[Path] = []
    for version in ["2000", "2004", "2007", "2010", "2013", "2018"]:
        d = testdata / version
        if d.exists():
            files.extend(
                sorted(
                    p
                    for p in d.iterdir()
                    if p.suffix.lower() == ".dwg"
                    # Pathological upstream-crash file (GitHub issue 44):
                    # gold's decode emits -nan tokens and thousands of
                    # garbage rows on an intentionally corrupt file. Frozen
                    # audit decision (IMPLEMENTATION.md §8.1.6a): excluded
                    # from the campaign's scope and rankings. It was
                    # previously excluded de facto by the normalize crash;
                    # normalize_gold's -nan shim (2026-09-20) made it parse,
                    # so keep the exclusion explicit.
                    and p.name != "gh44-error.dwg"
                )
            )
    for prefix in ["example_", "sample_"]:
        for p in testdata.iterdir():
            if p.name.startswith(prefix) and p.suffix.lower() == ".dwg":
                # Scope guard: only R2000-R2018 (AC1015..AC1032). Exclude the
                # R13/R14 examples, which use a different, out-of-scope format.
                lower = p.name.lower()
                if "r13" in lower or "r14" in lower:
                    continue
                files.append(p)
    # In-repo authored fixtures (IMPLEMENTATION.md §F2.1): one operation
    # per file, version-suffixed stems, qualified per the §F2.1 gates
    # before landing. Collected after the gold tree so a fixture stem
    # colliding with a gold stem stays visible in the report (stems
    # are required globally unique; a collision is a fixture bug).
    files.extend(sorted((SCRIPT_DIR / "tests").rglob("*.dwg")))
    return sorted(set(files))


def run_file(path: Path, workdir: Path) -> Dict[str, Any]:
    result = subprocess.run(
        [sys.executable, str(SCRIPT_DIR / "run_roundtrip.py"), str(path), str(workdir / path.stem)],
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    diff_orig_path = workdir / path.stem / f"{path.stem}_diff_orig.json"
    diff_rt_path = workdir / path.stem / f"{path.stem}_diff_rt.json"
    struct_orig_path = workdir / path.stem / f"{path.stem}_struct_orig.json"
    struct_rt_path = workdir / path.stem / f"{path.stem}_struct_rt.json"
    diff_orig: Dict[str, Any] = {"total_diffs": -1, "diffs": []}
    diff_rt: Dict[str, Any] = {"total_diffs": -1, "diffs": []}
    struct_orig: Dict[str, Any] = {"totals": {"key_gap_sum": -1}}
    struct_rt: Dict[str, Any] = {"totals": {"key_gap_sum": -1}}
    if diff_orig_path.exists():
        with open(diff_orig_path, "r", encoding="utf-8") as f:
            diff_orig = json.load(f)
    if diff_rt_path.exists():
        with open(diff_rt_path, "r", encoding="utf-8") as f:
            diff_rt = json.load(f)
    # The structure axis (§19 H0): per-key census — read (gold vs
    # silver projection) and write-target (gold_orig vs gold_rt).
    # Missing files mean the roundtrip predated the axis (or the census
    # failed defensively inside run_roundtrip) — the -1 placeholder
    # keeps the file counted and visible in the census column.
    for p, holder in ((struct_orig_path, "orig"), (struct_rt_path, "rt")):
        if p.exists():
            try:
                with open(p, "r", encoding="utf-8") as f:
                    if holder == "orig":
                        struct_orig = json.load(f)
                    else:
                        struct_rt = json.load(f)
            except json.JSONDecodeError:
                pass
    return {
        "file": str(path),
        "returncode": result.returncode,
        "stdout": result.stdout,
        "read_fidelity_diffs": diff_orig["total_diffs"],
        "write_fidelity_diffs": diff_rt["total_diffs"],
        "struct_read_key_gap": struct_orig.get("totals", {}).get("key_gap_sum", -1),
        "struct_write_key_gap": struct_rt.get("totals", {}).get("key_gap_sum", -1),
    }


def default_workdir() -> Path:
    """Return a repo-relative default corpus workdir under target/gold_harness_corpus/."""
    target = REPO_ROOT / "target"
    target.mkdir(parents=True, exist_ok=True)
    return target / "gold_harness_corpus"


def aggregate(results: List[Dict[str, Any]], workdir: Path) -> Tuple[Dict[Tuple[str, str], int], Dict[Tuple[str, str], int]]:
    read_counts: Dict[Tuple[str, str], int] = {}
    write_counts: Dict[Tuple[str, str], int] = {}
    for r in results:
        if r["read_fidelity_diffs"] == -1:
            continue
        # Re-read the diff files to aggregate by (type, field).
        stem = Path(r["file"]).stem
        file_workdir = workdir / stem
        for diff_path, counter in [
            (file_workdir / f"{stem}_diff_orig.json", read_counts),
            (file_workdir / f"{stem}_diff_rt.json", write_counts),
        ]:
            if not diff_path.exists():
                continue
            with open(diff_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            for d in data.get("diffs", []):
                if d.get("kind") == "count_mismatch":
                    key = (d.get("type", "?"), "_count")
                elif d.get("kind") == "missing":
                    key = (d.get("type", "?"), "_missing")
                else:
                    key = (d.get("type", "?"), d.get("field", "-"))
                counter[key] = counter.get(key, 0) + 1
    return read_counts, write_counts


def rebuild_rows_aggregate_only(workdir: Path, files: List[Path]) -> List[Dict[str, Any]]:
    """Rebuild the per-file rows from the on-disk per-file diff/struct
    JSONs without re-running the roundtrip pipeline — the aggregate
    logic can evolve without paying the ~10-minute corpus cycle again.
    `stdout`/`returncode` are not persisted per file; the rows carry
    markers instead (the report consumers read the counters)."""
    rows: List[Dict[str, Any]] = []
    for path in files:
        stem = path.stem
        fdir = workdir / stem
        row: Dict[str, Any] = {
            "file": str(path),
            "returncode": 0,
            "stdout": "(aggregate-only)",
        }
        diff_orig: Dict[str, Any] = {"total_diffs": -1}
        diff_rt: Dict[str, Any] = {"total_diffs": -1}
        struct_orig: Dict[str, Any] = {"totals": {"key_gap_sum": -1}}
        struct_rt: Dict[str, Any] = {"totals": {"key_gap_sum": -1}}
        for name, holder in (
            (f"{stem}_diff_orig.json", diff_orig),
            (f"{stem}_diff_rt.json", diff_rt),
        ):
            p = fdir / name
            if p.exists():
                try:
                    with open(p, "r", encoding="utf-8") as f:
                        holder.update(json.load(f))
                except json.JSONDecodeError:
                    pass
        for name, holder in (
            (f"{stem}_struct_orig.json", struct_orig),
            (f"{stem}_struct_rt.json", struct_rt),
        ):
            p = fdir / name
            if p.exists():
                try:
                    with open(p, "r", encoding="utf-8") as f:
                        holder.update(json.load(f))
                except json.JSONDecodeError:
                    pass
        row["read_fidelity_diffs"] = diff_orig.get("total_diffs", -1)
        row["write_fidelity_diffs"] = diff_rt.get("total_diffs", -1)
        row["struct_read_key_gap"] = struct_orig.get("totals", {}).get("key_gap_sum", -1)
        row["struct_write_key_gap"] = struct_rt.get("totals", {}).get("key_gap_sum", -1)
        rows.append(row)
    return rows


def main() -> int:
    testdata = Path(GOLD_TESTDATA)
    workdir = default_workdir()
    workdir.mkdir(parents=True, exist_ok=True)

    files = in_scope_files(testdata)
    print(f"[corpus] {len(files)} files in scope")

    aggregate_only = "--aggregate-only" in sys.argv
    if aggregate_only:
        print("[corpus] aggregate-only mode: reusing the on-disk per-file JSONs")
        results = rebuild_rows_aggregate_only(workdir, files)
        for i, path in enumerate(files, 1):
            if not (workdir / path.stem).exists():
                print(f"[corpus] WARNING: no per-file dir for {path.name}")
    else:
        results: List[Dict[str, Any]] = []
        for i, f in enumerate(files, 1):
            print(f"[{i}/{len(files)}] {f.name}")
            results.append(run_file(f, workdir))

    read_counts, write_counts = aggregate(results, workdir)

    # ── The structure census aggregate (§19 H0): per key, the projected
    # leaf gaps (read axis: gold vs silver projection) and the
    # write-target gaps (gold_orig vs gold_rt), summed over the corpus
    # with presence counts. This is the day-one attack-order table.
    struct_census: Dict[str, Dict[str, Any]] = {}
    struct_read_total = 0
    struct_write_total = 0
    undeclared_len: Dict[str, int] = {}
    for r in results:
        stem = Path(r["file"]).stem
        for side_name in ("orig", "rt"):
            struct_path = workdir / stem / f"{stem}_struct_{side_name}.json"
            if not struct_path.exists():
                continue
            try:
                with open(struct_path, "r", encoding="utf-8") as f:
                    st = json.load(f)
            except json.JSONDecodeError:
                continue
            if "error" in st:
                continue
            prefix = "read_" if side_name == "orig" else "write_"
            if side_name == "orig":
                struct_read_total += st.get("totals", {}).get("key_gap_sum", 0)
            else:
                struct_write_total += st.get("totals", {}).get("key_gap_sum", 0)
            for key, kres in (st.get("per_key") or {}).items():
                row = struct_census.setdefault(key, {
                    "read_files_gold": 0,
                    "read_files_other": 0,
                    "read_matched": 0,
                    "read_value_diffs": 0,
                    "read_missing_other": 0,
                    "read_gold_leaves": 0,
                    "write_matched": 0,
                    "write_value_diffs": 0,
                    "write_missing_other": 0,
                })
                status = kres.get("status", "?")
                if status == "absent_both":
                    continue
                if side_name == "orig":
                    # Presence counts come from the READ axis only (one
                    # per corpus file); the write-target columns carry
                    # their own sums below.
                    if status != "missing_gold":
                        row["read_files_gold"] += 1
                    if status == "present_both":
                        row["read_files_other"] += 1
                if status != "missing_gold" and status != "present_both":
                    # the whole key is the gap on this side (gold has
                    # it, the other side projects nothing)
                    row[f"{prefix}missing_other"] += kres.get("gold_leaves", 0)
                    if side_name == "orig":
                        row["read_gold_leaves"] += kres.get("gold_leaves", 0)
                elif status == "present_both":
                    row[f"{prefix}matched"] += kres.get("matched", 0)
                    row[f"{prefix}value_diffs"] += kres.get("value_diffs", 0)
                    row[f"{prefix}missing_other"] += kres.get(
                        "missing_silver" if side_name == "orig" else "missing_gold_rt", 0
                    )
                    if side_name == "orig":
                        row["read_gold_leaves"] += kres.get("gold_leaves", 0)
            if side_name == "orig":
                for u in st.get("undeclared_keys", []) or []:
                    undeclared_len[u] = undeclared_len.get(u, 0) + 1

    # ── The §20 genus gates (the constructed-content oracle): ADDITIONAL
    # output. The four fidelity axes, the differ, and the normalizers stay
    # untouched (§20.5); the sections land NONZERO on purpose — the
    # initial counts are the work queue (§20.3). Defensive: a genus
    # pipeline failure never fails the corpus run.
    genus_block: Dict[str, Any] = {}
    try:
        import genus_gates

        genus_report = genus_gates.run(workdir=workdir / "genus_gates")
        genus_block = {
            "counts": genus_report["genus_gates"]["counts"],
            "constructed_corpus": genus_report["genus_gates"]["constructed_corpus"],
            "per_file": genus_report["genus_gates"]["per_file"],
        }
        genus_sections = {
            "sab_form_diffs": genus_report["sab_form_diffs"],
            "sh_genus_diffs": genus_report["sh_genus_diffs"],
            "acds_genus_diffs": genus_report["acds_genus_diffs"],
        }
    except Exception as exc:  # noqa: BLE001 — the genus layer never fails the corpus
        genus_block = {"error": f"{type(exc).__name__}: {exc}"}
        genus_sections = {
            "sab_form_diffs": [],
            "sh_genus_diffs": [],
            "acds_genus_diffs": [],
        }

    report = {
        "files": len(results),
        "read_fidelity_total": sum(r["read_fidelity_diffs"] for r in results if r["read_fidelity_diffs"] != -1),
        "write_fidelity_total": sum(r["write_fidelity_diffs"] for r in results if r["write_fidelity_diffs"] != -1),
        "read_fidelity_by_type_field": {f"{k[0]}.{k[1]}": v for k, v in sorted(read_counts.items(), key=lambda x: -x[1])},
        "write_fidelity_by_type_field": {f"{k[0]}.{k[1]}": v for k, v in sorted(write_counts.items(), key=lambda x: -x[1])},
        "struct_read_key_gap_total": struct_read_total,
        "struct_write_key_gap_total": struct_write_total,
        "struct_census_per_key": struct_census,
        "struct_undeclared_keys": undeclared_len,
        "per_file": results,
        "genus_gates": genus_block,
        "sab_form_diffs": genus_sections["sab_form_diffs"],
        "sh_genus_diffs": genus_sections["sh_genus_diffs"],
        "acds_genus_diffs": genus_sections["acds_genus_diffs"],
    }

    report_json = workdir / "report.json"
    with open(report_json, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    print(f"[corpus] wrote {report_json}")

    md = ["# Gold-vs-Silver Corpus Report\n"]
    md.append(f"Files: {report['files']}\n")
    md.append(f"Read-fidelity diffs: {report['read_fidelity_total']}\n")
    md.append(f"Write-fidelity diffs: {report['write_fidelity_total']}\n")
    md.append(f"Structure read key-gap: {struct_read_total}\n")
    md.append(f"Structure write-target key-gap: {struct_write_total}\n\n")
    if undeclared_len:
        md.append("## Undeclared gold top-level keys (the H0 no-leak signal)\n")
        for u, c in sorted(undeclared_len.items(), key=lambda x: -x[1]):
            md.append(f"- {u}: {c} files\n")
        md.append("\n")
    md.append("## Structure census per key (§19 H0 day-one; the H2-H5 attack surface)\n")
    md.append("Presence = the READ axis (files where gold emits the key / silver projects it); leaf sums over both axes.\n")
    md.append("| key | read files gold/silver | read: matched+diffs+missing | write-target: matched+diffs+missing |\n")
    md.append("|---|---|---|---|\n")
    for key, row in sorted(
        struct_census.items(),
        key=lambda kv: -(kv[1].get("read_missing_other", 0) + kv[1].get("read_value_diffs", 0)),
    ):
        md.append(
            f"| {key} | {row.get('read_files_gold', 0)}/{row.get('read_files_other', 0)} "
            f"| {row.get('read_matched', 0)}+{row['read_value_diffs']}+{row['read_missing_other']} "
            f"| {row['write_matched']}+{row['write_value_diffs']}+{row['write_missing_other']} |\n"
        )
    md.append("\n## §20 genus gates (the constructed-content work queue)\n")
    md.append("The constructed-content oracle: silver-authored documents asserted "
              "against the authored-specimen genus. ADDITIONAL output — the four "
              "fidelity axes above are untouched. The counts land nonzero on "
              "purpose: they are the work queue, each row closing through the "
              "§8.1.2 packet workflow with a strict-loader verdict adjudicating. "
              "Adjudicated-TOLERATED rows keep their counts as the recorded "
              "state (§20.4); the PENDING rows are the remaining work.\n")
    if "error" in genus_block:
        md.append(f"Genus pipeline error: `{genus_block['error']}`\n")
    else:
        for section, title in (
            ("sab_form_diffs", "G-A — SAB form genus"),
            ("sh_genus_diffs", "G-B — SH tree genus"),
            ("acds_genus_diffs", "G-C — AcDs container genus"),
        ):
            rows = genus_sections[section]
            split = genus_block["counts"][section.replace("_diffs", "_pending")]
            md.append(f"\n### {title} ({len(rows)} rows — "
                      f"{split['pending_rows']} pending / "
                      f"{split['tolerated_rows']} adjudicated-TOLERATED)\n\n")
            if not rows:
                md.append("No divergences.\n")
                continue
            md.append("| type | field | count | status | detail |\n")
            md.append("|------|-------|-------|--------|--------|\n")
            for row in rows:
                detail = str(row["detail"]).replace("|", "\\|")
                md.append(f"| {row['type']} | {row['field']} | {row['count']} | "
                          f"{row.get('status', 'pending')} | {detail} |\n")
            tolerated = [row for row in rows if row.get("status")]
            if tolerated:
                md.append("\nAdjudicated rows (the recorded verdicts, §20.4):\n\n")
                for row in tolerated:
                    md.append(f"- **{row['field']}** ({row['status']}): "
                              f"{row['verdict']}\n")
    md.append("\n## Top read-fidelity divergences\n")
    for k, v in sorted(read_counts.items(), key=lambda x: -x[1])[:30]:
        md.append(f"- {k[0]}.{k[1]}: {v}\n")
    md.append("\n## Top write-fidelity divergences\n")
    for k, v in sorted(write_counts.items(), key=lambda x: -x[1])[:30]:
        md.append(f"- {k[0]}.{k[1]}: {v}\n")
    report_md = workdir / "report.md"
    with open(report_md, "w", encoding="utf-8") as f:
        f.writelines(md)
    print(f"[corpus] wrote {report_md}")
    if "error" in genus_block:
        print(f"[corpus] genus gates: pipeline error — {genus_block['error']}")
    else:
        counts = genus_block["counts"]
        pending = sum(counts[key]["pending_rows"] for key in
                      ("sab_form_pending", "sh_genus_pending", "acds_genus_pending"))
        print(f"[corpus] genus gates (the §20 work queue): "
              f"sab_form {counts['sab_form_diffs']} rows, "
              f"sh_genus {counts['sh_genus_diffs']} rows, "
              f"acds_genus {counts['acds_genus_diffs']} rows "
              f"({pending} rows pending; adjudicated-TOLERATED rows keep "
              f"their counts as the recorded state)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
