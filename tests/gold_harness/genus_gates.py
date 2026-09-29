#!/usr/bin/env python3
"""§20 genus_gates — the constructed-content oracle (the fifth layer).

The validation layer §20 designs and 2646f05 scopes: every existing gate
compares silver against GOLD on the same bytes, so constructed content —
documents silver authors from scratch — shipped real defects the corpus
could not see (the 2026-09-28 cylinder audit's factory
ACSH_HISTORY_CLASS and the SAB conic/quadric short-width desync both
passed 280/0/0 the same morning). This gate decodes the CONSTRUCTED
corpus silver-side, asserts it against the authored-specimen genus
(genus_expectations.json, extracted by genus_extract.py), and emits
ranked report sections:

  sab_form_diffs   G-A — the SAB form genus: per-class record widths
                   (width SETS — authored variance is legitimate), the
                   magic/version pair, the header triple, the product
                   strings, the tolerances, the terminator, the
                   record-class ordering, authored-uniform classes gone
                   missing, classes with no specimen coverage.
  sh_genus_diffs   G-B — the SH tree genus: for every history record in
                   a constructed decode, the topology invariants
                   (payload owner resolves to the graph, never the
                   solid; payload owner != ownerhandle; the 33/427
                   version pairs; history_node_id resolves to a written
                   node; the node's owner is the graph) and the scalar
                   root fields; plus the constructed-tree fixture's
                   observed state (the 2646f05 elide contract ranks as
                   the tree's current genus divergence).
  acds_genus_diffs G-C — the AcDs container genus: ds_version, the
                   segidx-first position, file_header_size, the
                   num_segidx scale, the populated-slot tail pattern,
                   the named-pointer slot types.

The gates land NONZERO on purpose (§20.3): the initial counts ARE the
work queue — G-C's container divergence ranks day one — and each row
closes through the §8.1.2 packet workflow with a strict-loader verdict
adjudicating (§20.4: the gates rank divergence; they do not decide
fatality). A row carrying a recorded verdict in ADJUDICATIONS closes
as TOLERATED: it KEEPS its count (the row is the recorded state) but
is annotated in the report and excluded from the pending_* counts —
the pending queue is the remaining work. The four fidelity axes and
the differ are untouched (§20.5).

The constructed corpus: the gen_all canonical (regenerated into the
workdir via the generator example; its md5 recorded as a fact, never
asserted — the generation identity is the separate canary) plus the
constructed-fixture family the codec emits (genus_constructed: one
solid per SAB surface family, one create_solid_history tree, one
region, one body).

CLI: genus_gates.py [--workdir DIR] [--expectations PATH] [--strict]
`--strict` asserts zero PENDING rows (adjudicated-TOLERATED rows stay
as the recorded state; for the day the queue closes). Exit 0 on
completion regardless of counts; nonzero only on pipeline failure.
"""

import argparse
import hashlib
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO = SCRIPT_DIR.parents[1]
DEFAULT_EXPECTATIONS = SCRIPT_DIR / "genus_expectations.json"
DEFAULT_WORKDIR = REPO / "target" / "genus_gates"

sys.path.insert(0, str(SCRIPT_DIR))
from genus_extract import (  # noqa: E402  (the shared SAB walker + helpers)
    SabWalkError, acis_entities, build_dwg2json, dynamic_blocks, resolve_cargo,
    unwrap_node, walk_sab,
)


class RowCollector:
    """The ranked (type, field, count) rows of one gate family.

    A row whose (gate, field) carries an ADJUDICATIONS entry keeps its
    count (§20.3: the row stays as the recorded state) but is annotated
    with the recorded strict-loader verdict, so the queue's pending
    work separates from the adjudicated divergence.
    """

    def __init__(self, gate):
        self.gate = gate
        self.rows = {}

    def add(self, field, detail, count=1):
        key = (field, detail)
        self.rows[key] = self.rows.get(key, 0) + count

    def section(self):
        ranked = sorted(
            ({"type": self.gate, "field": field, "count": count, "detail": detail}
             for (field, detail), count in self.rows.items()),
            key=lambda row: (-row["count"], row["field"], row["detail"]),
        )
        for row in ranked:
            verdict = ADJUDICATIONS.get((self.gate, row["field"]))
            if verdict:
                row["status"] = verdict["verdict"]
                row["verdict"] = verdict["note"]
        return ranked


# The recorded strict-loader verdicts (§20.4): a row listed here is the
# ADJUDICATED state — kept in the counts as the recorded divergence,
# annotated in the report, and excluded from the pending work queue.
# Every entry cites its provenance; nothing is tolerated without a
# recorded verdict.
ADJUDICATIONS = {
    ("sab_form", "header-magic-version"): {
        "verdict": "TOLERATED",
        "note": "the ACIS|700 binary SAB flavor is silver's native output; "
                "BricsCAD-ACCEPTED (the 2026-09-21 strict-load zero; the "
                "2026-09-22 region probe re-confirmed the restorer audits "
                "the stream clean). The authored 21200/21500/21800/ASM|22300 "
                "pairs are the author's era stamps — a flavor change is a "
                "writer-generation change, not a genus defect.",
    },
    ("sab_form", "header-triple"): {
        "verdict": "TOLERATED",
        "note": "the (0, 1, 0) constructed header triple rides the same "
                "BricsCAD-ACCEPTED stance (the 2026-09-21 strict-load zero); "
                "the authored (0, 2, 4/12/24/26) semantics are unexplained "
                "constants — reproducing them would forge unmeasured fields.",
    },
    ("sab_form", "product-strings"): {
        "verdict": "TOLERATED",
        "note": "the author's identity rule: silver writes its own product "
                "strings; forging Autodesk identity stamps is forbidden "
                "(the campaign's standing rule). BricsCAD-ACCEPTED (the "
                "2026-09-21 strict-load zero).",
    },
    ("sh_genus", "constructed-tree"): {
        "verdict": "TOLERATED",
        "note": "the 2646f05 cylinder verdict: the constructed-tree elide "
                "contract (no ACSH records; the solid's history soft-pointer "
                "NULL). The topology invariants stay armed for any "
                "constructed tree that survives save.",
    },
}


# ── The constructed corpus ──


def generate_constructed_corpus(workdir, cargo):
    """Regenerate gen_all + the constructed family into the workdir.

    The gen_all canonical is rebuilt through the generator example with
    the workdir as CWD (the example writes its output file relative to
    CWD); its md5 is recorded in the report as a fact — the generation
    identity canary is a separate gate, never asserted here.
    """
    constructed = workdir / "constructed"
    constructed.mkdir(parents=True, exist_ok=True)

    manifest = REPO / "Cargo.toml"
    cmd = [cargo, "run", "--quiet", "--manifest-path", str(manifest),
           "--example", "gen_all_entities_all_versions_dwg", "--features", "serde"]
    subprocess.run(cmd, cwd=constructed, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    canonical = constructed / "gen_all_entities_all_versions.dwg"
    if not canonical.exists():
        raise SystemExit("the generator example produced no canonical file")

    subprocess.run([cargo, "build", "--quiet", "--bin", "genus_constructed",
                    "--features", "serde"], cwd=REPO, check=True)
    binary = REPO / "target" / "debug" / "genus_constructed"
    subprocess.run([str(binary), str(constructed)], check=True)

    files = sorted(constructed.glob("*.dwg"))
    return canonical, files


# ── G-A: the SAB form genus ──


def gate_sab_form(stem, etype, payload, expectations, rows):
    acis = payload.get("acis_data", {})
    sab_bytes = acis.get("sab_data")
    if not sab_bytes:
        return
    try:
        header, records = walk_sab(bytes(sab_bytes))
    except SabWalkError as exc:
        rows.add(f"{etype}-sab-walk", f"decode failure: {exc}")
        return
    genus = expectations["sab_form"]

    pair = (header["magic"], header["version"])
    if pair not in {tuple(p) for p in genus["magic_version_pairs"]}:
        rows.add("header-magic-version",
                 f"{pair[0]}|{pair[1]} not in genus "
                 f"{[f'{m}|{v}' for m, v in genus['magic_version_pairs']]}")
    genus_triples = {tuple(t) for triples in genus["header_triples"].values()
                     for t in triples}
    triple = (header["num_records"], header["num_bodies"], header["has_history"])
    if triple not in genus_triples:
        rows.add("header-triple", f"{triple} not in genus {sorted(genus_triples)}")
    if (header["product_id"], header["product_version"]) not in {
            tuple(p) for p in genus["product_strings"]}:
        rows.add("product-strings",
                 f"{header['product_id']!r}/{header['product_version']!r} "
                 f"not in genus")
    if tuple(header["tolerances"]) not in {tuple(t) for t in genus["tolerance_triples"]}:
        rows.add("tolerances", f"{header['tolerances']} not in genus")

    order = []
    class_counts = Counter()
    for record in records:
        name = record["name"]
        if record.get("terminator"):
            if name not in genus["terminator_names"]:
                rows.add("terminator", f"{name} not in genus {genus['terminator_names']}")
            continue
        class_counts[name] += 1
        widths = genus["class_widths"].get(name)
        if widths is None:
            rows.add(name, "no specimen coverage (unattested class)")
        elif record["width"] not in widths:
            rows.add(name, f"width {record['width']} not in genus {widths}")
        if name not in order:
            order.append(name)

    # Authored-uniform classes missing from this stream: the classes every
    # specimen SAB carries (asmheader, the persubent attribs, …).
    carriers = expectations["provenance"]["sab_carriers"]
    for name, specimens in genus["class_specimens"].items():
        if len(specimens) == carriers and name not in class_counts:
            rows.add(name, "authored-uniform class missing from the constructed stream")

    # Record-class ordering: `order_constraints[a]` lists the classes
    # that ALWAYS appear after `a` in the authored genus. The violation:
    # `earlier` appears before `later` in the constructed stream while
    # the genus puts `earlier` after `later`.
    constraints = genus["order_constraints"]
    for later_index, later in enumerate(order):
        for earlier in order[:later_index]:
            if earlier in constraints.get(later, []):
                rows.add("ordering", f"{earlier} before {later} "
                                     f"(genus: {earlier} always after {later})")


# ── G-B: the SH tree genus ──


def gate_sh_genus(stem, doc, expectations, rows, tree_fixture_stems):
    genus = expectations["sh_genus"]
    objects = doc.get("objects", {})
    roots = []
    for handle, payload in dynamic_blocks(doc):
        if payload.get("dxf_name") != "ACSH_HISTORY_CLASS":
            continue
        roots.append((handle, payload))
        data = payload.get("data", {})
        (wrapper, root), = data.items()
        if wrapper != "SolidHistory":
            rows.add("root-shape", f"unexpected wrapper {wrapper}")
            continue
        owner_handle = payload.get("owner")
        if root["owner"] == owner_handle:
            rows.add("payload-owner-equals-ownerhandle",
                    "the payload owner duplicates the record's own ownerhandle")
        graph = objects.get(str(root["owner"]))
        if graph is None:
            rows.add("payload-owner-resolves",
                     f"payload owner {root['owner']} resolves to nothing")
        else:
            (graph_type, graph_payload), = graph.items()
            if graph_payload.get("dxf_name") not in genus["payload_owner_kinds"]:
                rows.add("payload-owner-kind",
                         f"payload owner resolves to "
                         f"{graph_payload.get('dxf_name')}, genus "
                         f"{genus['payload_owner_kinds']}")
            if graph_payload.get("owner") != handle:
                rows.add("graph-owner-is-root", "the graph's owner is not the history root")
        if (root["major"], root["minor"]) not in {tuple(p) for p in genus["root_version_pairs"]}:
            rows.add("root-version-pair",
                     f"({root['major']}, {root['minor']}) not in genus "
                     f"{genus['root_version_pairs']}")
        if root["show_history"] not in genus["show_history"]:
            rows.add("show-history", f"{root['show_history']} not in genus")
        if root["record_history"] not in genus["record_history"]:
            rows.add("record-history", f"{root['record_history']} not in genus")
        # The owning solid's history soft-pointer must name this root
        # (the pin's solid_history_links_root genus — the cylinder-audit
        # dangling-link invariant's sibling: every authored specimen
        # links the solid back to its root).
        if genus["solid_history_links_root"] == [True]:
            owner_entity = None
            for _etype, entity in acis_entities(doc):
                entity_handle = (entity.get("handle")
                                 or entity.get("common", {}).get("handle"))
                if entity_handle == owner_handle:
                    owner_entity = entity
                    break
            if owner_entity is None:
                rows.add("solid-history-links-root",
                         f"the root's ownerhandle {owner_handle} resolves to no "
                         f"ACIS entity")
            elif owner_entity.get("history_handle") != handle:
                rows.add("solid-history-links-root",
                         f"the owning solid's history soft-pointer is "
                         f"{owner_entity.get('history_handle')}, not the root "
                         f"{handle}")
        node_id = root["history_node_id"]
        matches = []
        for node_handle, node_payload in dynamic_blocks(doc):
            dxf_name = node_payload.get("dxf_name", "")
            if not dxf_name.startswith("ACSH_") or dxf_name == "ACSH_HISTORY_CLASS":
                continue
            inner = unwrap_node(node_payload.get("data", {}))
            if inner is None:
                continue
            base = inner.get("base", {})
            if base.get("step_id") == node_id or base.get("eval", {}).get("node_id") == node_id:
                matches.append((node_handle, node_payload, base))
        if not matches:
            rows.add("history-node-id-resolves",
                     f"history_node_id {node_id} resolves to no written node")
        else:
            for node_handle, node_payload, base in matches:
                if node_payload.get("owner") != root["owner"]:
                    rows.add("node-owner-is-graph",
                             "the node's owner is not the graph")
                if (base.get("major"), base.get("minor")) not in {
                        tuple(p) for p in genus["node_version_pairs"]}:
                    rows.add("node-version-pair",
                             f"({base.get('major')}, {base.get('minor')}) not in genus "
                             f"{genus['node_version_pairs']}")
                eval_block = base.get("eval", {})
                if (eval_block.get("major"), eval_block.get("minor")) not in {
                        tuple(p) for p in genus["node_eval_version_pairs"]}:
                    rows.add("node-eval-version-pair",
                             f"({eval_block.get('major')}, {eval_block.get('minor')}) "
                             f"not in genus {genus['node_eval_version_pairs']}")

    # The constructed-tree fixture's observed state: the 2646f05 elide
    # contract (constructed ACSH_ records elide at save; the solid's
    # history soft-pointer writes NULL). The authored genus interposes
    # the graph and links the solid — the elide ranks as the tree's
    # current genus divergence, adjudicated TOLERATED by the cylinder
    # verdict until a constructed probe passes a strict loader.
    if stem in tree_fixture_stems:
        if roots:
            rows.add("constructed-tree",
                     f"tree survived save ({len(roots)} ACSH root records) — "
                     f"the invariants above engaged")
        else:
            rows.add("constructed-tree",
                     "elided at save (the 2646f05 contract): no ACSH records, "
                     "the solid's history soft-pointer NULL — the authored genus "
                     "interposes the ACAD_EVALUATION_GRAPH and its node id "
                     "resolves; ranked for strict-loader adjudication")


# ── G-C: the AcDs container genus ──


def gate_acds_genus(stem, doc, expectations, rows):
    acds = doc.get("dwg_acds")
    if not acds:
        return
    genus = expectations["acds_genus"]
    if acds.get("unknown_1") not in genus["unknown_1s"]:
        rows.add("unknown_1",
                 f"{acds.get('unknown_1')} not in genus {genus['unknown_1s']}")
    if acds.get("version") not in genus["container_versions"]:
        rows.add("container-version",
                 f"{acds.get('version')} not in genus {genus['container_versions']}")
    if acds.get("ds_version") not in genus["ds_versions"]:
        rows.add("ds_version",
                 f"{acds.get('ds_version')} not in genus {genus['ds_versions']}")
    if acds.get("segidx_offset") not in genus["segidx_offsets"]:
        rows.add("segidx_offset",
                 f"{acds.get('segidx_offset')} not in genus {genus['segidx_offsets']} "
                 f"(segidx-last vs the segidx-first container)")
    if acds.get("file_header_size") not in genus["file_header_sizes"]:
        rows.add("file_header_size",
                 f"{acds.get('file_header_size')} not in genus {genus['file_header_sizes']}")
    segidx_len = len(acds.get("segidx", []))
    if segidx_len not in genus["num_segidx"]:
        rows.add("num_segidx",
                 f"{segidx_len} not in genus {genus['num_segidx']} (the scale)")
    types = [segment.get("type") for segment in acds.get("segments", [])]
    n = len(types)
    tail = [list(item) for item in sorted((n - index, kind)
                                          for index, kind in enumerate(types)
                                          if kind is not None)]
    if tail not in genus["tail_patterns"]:
        rows.add("tail-pattern", f"populated-slot pattern {tail} not in genus")
    for pointer_name, genus_types in genus["named_pointer_types"].items():
        slot = acds.get(pointer_name)
        if slot is None or not 0 <= slot < n or types[slot] is None:
            rows.add(pointer_name, "the named pointer's slot is unpopulated")
        elif types[slot] not in genus_types:
            rows.add(pointer_name,
                     f"slot type {types[slot]} not in genus {genus_types}")


# ── The run ──

TREE_FIXTURE_STEMS = {"HistoryTree"}


def _section_counts(section_rows):
    """The (rows, occurrences, pending, tolerated) split of one section.

    The primary counts keep §20.3's contract (every row counts — the
    adjudicated rows stay as the recorded state); the pending_* counts
    are the remaining work queue.
    """
    rows = len(section_rows)
    occurrences = sum(row["count"] for row in section_rows)
    tolerated_rows = sum(1 for row in section_rows if row.get("status"))
    tolerated_occ = sum(row["count"] for row in section_rows if row.get("status"))
    return {
        "rows": rows,
        "occurrences": occurrences,
        "pending_rows": rows - tolerated_rows,
        "pending_occurrences": occurrences - tolerated_occ,
        "tolerated_rows": tolerated_rows,
        "tolerated_occurrences": tolerated_occ,
    }


def run(workdir=None, expectations_path=None, cargo=None):
    """Run the genus gates; returns the report dict.

    Importable from run_corpus.py (the sections attach there as
    ADDITIONAL output; the fidelity totals stay untouched).
    """
    workdir = Path(workdir) if workdir else DEFAULT_WORKDIR
    expectations_path = Path(expectations_path) if expectations_path else DEFAULT_EXPECTATIONS
    cargo = cargo or resolve_cargo()
    workdir.mkdir(parents=True, exist_ok=True)

    with open(expectations_path) as handle:
        expectations = json.load(handle)

    canonical, constructed_files = generate_constructed_corpus(workdir, cargo)

    digest = hashlib.md5(canonical.read_bytes()).hexdigest()
    size = canonical.stat().st_size

    binary = build_dwg2json(cargo)
    decoded_dir = workdir / "decoded"
    decoded_dir.mkdir(parents=True, exist_ok=True)

    sab_rows = RowCollector("sab_form")
    sh_rows = RowCollector("sh_genus")
    acds_rows = RowCollector("acds_genus")
    files_section = []
    for dwg_path in constructed_files:
        stem = dwg_path.stem
        json_path = decoded_dir / f"{stem}.json"
        subprocess.run([str(binary), str(dwg_path), str(json_path)],
                       check=True, stdout=subprocess.DEVNULL)
        with open(json_path) as handle:
            doc = json.load(handle)

        def total(collector):
            return sum(collector.rows.values())

        sab_before, sh_before, acds_before = total(sab_rows), total(sh_rows), total(acds_rows)
        for etype, payload in acis_entities(doc):
            gate_sab_form(stem, etype, payload, expectations, sab_rows)
        gate_sh_genus(stem, doc, expectations, sh_rows, TREE_FIXTURE_STEMS)
        gate_acds_genus(stem, doc, expectations, acds_rows)
        files_section.append({
            "stem": stem,
            "sab_form_diffs": total(sab_rows) - sab_before,
            "sh_genus_diffs": total(sh_rows) - sh_before,
            "acds_genus_diffs": total(acds_rows) - acds_before,
        })

    sections = {
        "sab_form_diffs": sab_rows.section(),
        "sh_genus_diffs": sh_rows.section(),
        "acds_genus_diffs": acds_rows.section(),
    }
    report = {
        "genus_gates": {
            "expectations": str(expectations_path),
            "constructed_corpus": {
                "dir": str(workdir / "constructed"),
                "files": [f.name for f in constructed_files],
                "gen_all_md5": digest,
                "gen_all_size": size,
            },
            "per_file": files_section,
            "counts": {
                "sab_form_diffs": len(sections["sab_form_diffs"]),
                "sh_genus_diffs": len(sections["sh_genus_diffs"]),
                "acds_genus_diffs": len(sections["acds_genus_diffs"]),
                "sab_form_pending": _section_counts(sections["sab_form_diffs"]),
                "sh_genus_pending": _section_counts(sections["sh_genus_diffs"]),
                "acds_genus_pending": _section_counts(sections["acds_genus_diffs"]),
            },
        },
        "sab_form_diffs": sections["sab_form_diffs"],
        "sh_genus_diffs": sections["sh_genus_diffs"],
        "acds_genus_diffs": sections["acds_genus_diffs"],
    }
    return report


def write_report(report, workdir):
    workdir = Path(workdir)
    json_path = workdir / "genus_report.json"
    with open(json_path, "w") as handle:
        json.dump(report, handle, indent=1, sort_keys=True)
        handle.write("\n")
    md_path = workdir / "genus_report.md"
    with open(md_path, "w") as handle:
        corpus = report["genus_gates"]["constructed_corpus"]
        handle.write("# §20 genus gates — the constructed-content oracle\n\n")
        handle.write("The ranked work queue (the gates land nonzero on purpose; "
                     "each row closes through the §8.1.2 packet workflow with a "
                     "strict-loader verdict adjudicating — §20.4).\n\n")
        handle.write(f"- constructed corpus: `{corpus['dir']}` "
                     f"({len(corpus['files'])} files)\n")
        handle.write(f"- gen_all canonical: md5 `{corpus['gen_all_md5']}`, "
                     f"{corpus['gen_all_size']} bytes (recorded, not asserted)\n")
        handle.write(f"- expectations: `{report['genus_gates']['expectations']}`\n\n")
        for section, title in (("sab_form_diffs", "G-A — SAB form genus"),
                               ("sh_genus_diffs", "G-B — SH tree genus"),
                               ("acds_genus_diffs", "G-C — AcDs container genus")):
            rows = report[section]
            split = report["genus_gates"]["counts"][section.replace("_diffs", "_pending")]
            handle.write(f"## {title} ({len(rows)} rows — "
                         f"{split['pending_rows']} pending / "
                         f"{split['tolerated_rows']} adjudicated-TOLERATED)\n\n")
            if not rows:
                handle.write("No divergences.\n\n")
                continue
            handle.write("| type | field | count | status | detail |\n")
            handle.write("|------|-------|-------|--------|--------|\n")
            for row in rows:
                detail = str(row["detail"]).replace("|", "\\|")
                status = row.get("status", "pending")
                handle.write(f"| {row['type']} | {row['field']} | "
                             f"{row['count']} | {status} | {detail} |\n")
            tolerated = [row for row in rows if row.get("status")]
            if tolerated:
                handle.write("\nAdjudicated rows (the recorded verdicts, §20.4):\n\n")
                for row in tolerated:
                    handle.write(f"- **{row['field']}** ({row['status']}): "
                                 f"{row['verdict']}\n")
                handle.write("\n")
            handle.write("\n")
    return json_path, md_path


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--workdir", type=Path, default=DEFAULT_WORKDIR)
    parser.add_argument("--expectations", type=Path, default=DEFAULT_EXPECTATIONS)
    parser.add_argument("--strict", action="store_true",
                        help="assert zero rows (for the day the queue closes)")
    args = parser.parse_args()

    if not args.expectations.exists():
        raise SystemExit(f"expectations absent: {args.expectations} — "
                         f"run genus_extract.py first (§20.3 mechanics)")

    report = run(workdir=args.workdir, expectations_path=args.expectations)
    json_path, md_path = write_report(report, args.workdir)

    counts = report["genus_gates"]["counts"]
    print(f"genus report -> {json_path}")
    print(f"  (markdown: {md_path})")
    for section, label in (("sab_form_diffs", "sab_form_diffs"),
                           ("sh_genus_diffs", "sh_genus_diffs"),
                           ("acds_genus_diffs", "acds_genus_diffs")):
        split = counts[section.replace("_diffs", "_pending")]
        print(f"  {label}:   {counts[section]} distinct rows, "
              f"{split['occurrences']} total occurrences "
              f"({split['pending_rows']} rows / {split['pending_occurrences']} "
              f"occurrences pending; {split['tolerated_rows']} rows / "
              f"{split['tolerated_occurrences']} occurrences adjudicated-TOLERATED)")
    print("  the pending counts are the work queue (§20.3/§20.4) — "
          "adjudicated rows stay as the recorded state")

    if args.strict:
        pending = sum(counts[key]["pending_rows"] for key in
                      ("sab_form_pending", "sh_genus_pending", "acds_genus_pending"))
        if pending:
            print(f"--strict: {pending} pending rows remain "
                  f"(adjudicated-TOLERATED rows do not close the queue)")
            sys.exit(1)


if __name__ == "__main__":
    main()
