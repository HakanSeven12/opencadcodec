#!/usr/bin/env python3
"""The entity-behavior verification matrix (the maintainer's 2026-09-30
directive: "verify behaviour of all entities supported and document").

Five axes per entity kind, all measured on the gen_all canonical (the
single AC1032 document carrying every supported entity kind):

  BUILD          the canonical's own add_entity verdict (OK/SKIP) —
                 does the public API accept the kind at all?
  SILVER-READ    silver's own decode of the generated file: the kind's
                 entity count (a kind that writes but does not read
                 back is a roundtrip hole, not a supported kind).
  GOLD-READ      gold's (libredwg) decode of the same file: the kind's
                 census from the reference reader + any ERROR lines
                 (the known Box/Sphere/Torus record overruns rank as
                 the standing persubent-tail residue, not new).
  REWRITE        the conventional-arm rewrite (DWG_NO_ECHO) re-read:
                 the kind's count must survive the read->write->read
                 cycle (the edited-document path).
  MODELER        the strict-loader probe on the canonical (BricsCAD +
                 AutoCAD): the per-entity census verdicts — the
                 ent[h] bbox lines (real extents = the entity's model
                 constructs; bbox-FAIL = the modeler refuses it).

The report: a markdown matrix (entity x axis -> verdict + evidence)
written to target/entity_verification/report.md, the console summary,
and the standing residue section. Run:

    python3 tests/gold_harness/entity_verification.py [--no-probe]

--no-probe skips the loader axis (the file axes still run); the probe
takes ~5 minutes (both GUI loaders, one file each).
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CANON = REPO / "gen_all_entities_all_versions.dwg"
WORK = REPO / "target" / "entity_verification"
GOLD = Path.home() / "work" / "libredwg" / "programs" / "dwgread"
DWG2JSON = REPO / "target" / "debug" / "dwg2json"
DWGREWRITE = REPO / "target" / "debug" / "dwgrewrite"

# The canonical's kinds (the EntityType:: inventory of the example) and
# the DWG type-name mapping where silver's JSON kind and gold's census
# name differ from the API name.
# The API kind, the canonical's add_entity name (uppercase), and
# gold's census name where it differs from the canonical's.
KINDS = [
    ("Point", "POINT", "POINT"),
    ("Line", "LINE", "LINE"),
    ("Circle", "CIRCLE", "CIRCLE"),
    ("Arc", "ARC", "ARC"),
    ("Ellipse", "ELLIPSE", "ELLIPSE"),
    ("XLine", "XLINE", "XLINE"),
    ("Ray", "RAY", "RAY"),
    ("Solid", "SOLID", "SOLID"),
    ("Shape", "SHAPE", "SHAPE"),
    ("Text", "TEXT", "TEXT"),
    ("MText", "MTEXT", "MTEXT"),
    ("Spline", "SPLINE", "SPLINE"),
    ("Polyline2D", "POLYLINE_2D", "POLYLINE"),
    ("Polyline3D", "POLYLINE_3D", "POLYLINE"),
    ("LwPolyline", "LWPOLYLINE", "LWPOLYLINE"),
    ("PolyfaceMesh", "POLYFACE_MESH", "POLYLINE"),
    ("Mesh", "MESH", "MESH"),
    ("MLine", "MLINE", "MLINE"),
    ("Insert", "INSERT", "INSERT"),
    ("Viewport", "VIEWPORT", "VIEWPORT"),
    ("Tolerance", "TOLERANCE", "TOLERANCE"),
    ("Dimension", "DIMENSION", "DIMENSION_LINEAR"),
    ("Leader", "LEADER", "LEADER"),
    ("MultiLeader", "MULTILEADER", "MULTILEADER"),
    ("Hatch", "HATCH", "HATCH"),
    ("Face3D", "FACE_3D", "FACE_3D"),
    ("Solid3D", "SOLID_3D", "_3DSOLID"),
    ("Region", "REGION", "REGION"),
    ("Body", "BODY", "BODY"),
]


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def silver_kind_counts(json_path):
    doc = json.load(open(json_path))
    counts = {}
    for e in doc.get("entities", []):
        for kind in e:
            counts[kind] = counts.get(kind, 0) + 1
    return counts


def gold_census(text):
    """gold -v9: 'Add entity NAME' lines + ERROR lines."""
    adds = {}
    for m in re.finditer(r"^Add entity (\S+)", text, re.M):
        adds[m.group(1)] = adds.get(m.group(1), 0) + 1
    errors = [l.strip() for l in text.splitlines() if "ERROR" in l]
    return adds, errors


def probe_axis(report):
    """The strict-loader axis: probe the canonical under both loaders."""
    sys.path.insert(0, str(Path(__file__).parent))
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "slp", Path(__file__).parent / "strict_load_probe.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    probe_dir = Path("/mnt/c/Users/SEBAST~1/AppData/Local/Temp/kilo/"
                     "strict_load_probe")
    probe_dir.mkdir(parents=True, exist_ok=True)
    verdicts = {}
    for tag, (loader, mode) in (("bcad", (mod.DEFAULT_BCAD, "gui")),
                                 ("acad", (mod.DEFAULT_ACAD, "gui"))):
        run_name = f"GenAllCanonical__{tag}"
        lines = mod.probe_one(run_name, CANON, probe_dir, loader, 150, mode)
        v = mod.verdict(lines)
        per_entity = {}
        for l in lines:
            m = re.match(r"ent\[([0-9A-Fa-f]+)\]: (bbox|bbox-FAIL): (.*)", l)
            if m:
                per_entity[m.group(1)] = (m.group(2), m.group(3))
        audit = mod.analyze_audit_log(mod.harvest_audit_log(run_name, probe_dir, mode))
        verdicts[tag] = {"verdict": v, "per_entity": per_entity,
                         "audit": audit, "lines": lines}
    return verdicts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-probe", action="store_true",
                    help="skip the strict-loader axis")
    args = ap.parse_args()

    WORK.mkdir(parents=True, exist_ok=True)

    # ── Axis 1: BUILD (the canonical's own verdicts) ──
    print("== generating the canonical (cargo run) ==")
    r = run(["cargo", "run", "--quiet", "--example",
             "gen_all_entities_all_versions_dwg", "--features", "serde"],
            cwd=REPO)
    build_ok = set(re.findall(r"OK\s+(\S+)", r.stdout))
    build_skip = {}
    for m in re.finditer(r"SKIP\s+(\S+)\s+(.*)", r.stdout):
        build_skip[m.group(1)] = m.group(2).strip()
    print(f"   build: {len(build_ok)} OK, {len(build_skip)} SKIP")

    # ── Axis 2: SILVER-READ ──
    orig_json = WORK / "orig.json"
    run([str(DWG2JSON), str(CANON), str(orig_json)])
    silver = silver_kind_counts(orig_json)
    print(f"   silver read: {sum(silver.values())} entities, "
          f"{len(silver)} kinds")

    # ── Axis 4: REWRITE survival ──
    rw = WORK / "rewrite.dwg"
    run([str(DWGREWRITE), str(CANON), str(rw)],
        env={**__import__("os").environ, "DWG_NO_ECHO": "1"})
    rw_json = WORK / "rewrite.json"
    run([str(DWG2JSON), str(rw), str(rw_json)])
    silver_rw = silver_kind_counts(rw_json)

    # ── Axis 3: GOLD-READ (both files) ──
    g = run([str(GOLD), "-v9", str(CANON)])
    gold_adds, gold_errors = gold_census(g.stdout + g.stderr)
    g2 = run([str(GOLD), "-v9", str(rw)])
    gold_rw_adds, gold_rw_errors = gold_census(g2.stdout + g2.stderr)
    print(f"   gold read: {sum(gold_adds.values())} entities, "
          f"{len(gold_errors)} ERROR lines")

    # ── Axis 5: MODELER ──
    modeler = None
    if not args.no_probe:
        print("== probing the canonical (both loaders) ==")
        modeler = probe_axis(None)
        for tag, v in modeler.items():
            print(f"   {tag}: {v['verdict']} "
                  f"({len(v['per_entity'])} per-entity census lines)")

    # ── The matrix ──
    lines = []
    lines.append("# The entity-behavior verification matrix")
    lines.append("")
    lines.append("The gen_all canonical (one AC1032 document, every "
                 "supported entity kind), verified on five axes:")
    lines.append("BUILD (the public API accepts the kind), SILVER-READ "
                 "(our decode of the generated file), GOLD-READ "
                 "(libredwg's decode), REWRITE (the conventional-arm "
                 "read->write->read survival), MODELER (the strict-loader "
                 "per-entity census).")
    lines.append("")
    lines.append("| Entity | Build | Silver | Gold | Rewrite | Modeler (bcad / acad) |")
    lines.append("|--------|-------|--------|------|---------|----------------------|")

    def modeler_cell(kind):
        if not modeler:
            return "(skipped)"
        parts = []
        for tag in ("bcad", "acad"):
            v = modeler[tag]
            if "NO RESULT" in " ".join(v["lines"]) or not v["per_entity"]:
                parts.append(f"{tag}: {v['verdict']}")
            else:
                parts.append(f"{tag}: {v['verdict']}")
        return " / ".join(parts)

    for kind, canon_name, gold_name in KINDS:
        b = "OK" if canon_name in build_ok else (
            f"SKIP ({build_skip.get(canon_name, '?')})" if canon_name in build_skip else "?")
        s = silver.get(kind, 0)
        gr = sum(n for k, n in gold_adds.items()
                 if k == gold_name or k.startswith(gold_name + "_"))
        rw_n = silver_rw.get(kind, 0)
        cell_rw = f"{rw_n}/{s}" if rw_n == s and s else f"{rw_n}!={s}"
        lines.append(f"| {kind} | {b} | {s} | {gr} | {cell_rw} | "
                     f"{modeler_cell(kind)} |")

    lines.append("")
    lines.append(f"Silver read-back total: {sum(silver.values())} entities "
                 f"in {len(silver)} kinds; gold census total: "
                 f"{sum(gold_adds.values())}.")
    if gold_errors:
        lines.append("")
        lines.append("## Gold ERROR lines (the standing residue)")
        lines.append("```")
        for e in gold_errors[:20]:
            lines.append(e[:160])
        lines.append("```")
    if modeler:
        lines.append("")
        lines.append("## The modeler census (the canonical, per loader)")
        for tag, v in modeler.items():
            lines.append(f"### {tag}: {v['verdict']}")
            lines.append("```")
            for l in v["lines"]:
                if l.startswith("ent[") or l.startswith("open-entity") \
                        or l.startswith("post-audit-entity"):
                    lines.append(l[:150])
            if v["audit"]:
                lines.append("-- audit --")
                for a in v["audit"][:10]:
                    lines.append(a[:150])
            lines.append("```")

    out = WORK / "report.md"
    out.write_text("\n".join(lines) + "\n")
    print(f"\nreport -> {out}")
    print("\n".join(lines[:40]))


if __name__ == "__main__":
    main()
