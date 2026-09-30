#!/usr/bin/env python3
"""§20 genus_extract — the constructed-content oracle's expectation extractor.

The fifth validation layer's reference generator (IMPLEMENTATION.md §20.3,
mechanics item 1). Every existing gate compares silver against GOLD on the
same bytes; constructed content — documents silver authors from scratch —
has no gold counterpart. The authored specimens ARE the genus reference
(§20.1): this script decodes them SILVER-side (dwg2json), walks the SAB
modeler blobs and the SH history trees the specimens carry, and projects
their invariants into `genus_expectations.json`:

  G-A sab_form   per-class record-width SETS (authored variance is
                 legitimate — e.g. straight-curve 85/103), the
                 magic/version pair, the header triple
                 (num_records/num_bodies/has_history), the product
                 strings, the tolerance triples, the terminator names,
                 the record-class ordering (a pairwise always-after
                 partial order), and per-class specimen lineage.
  G-B sh_genus   the history-tree topology: payload-owner kind (the
                 ACAD_EVALUATION_GRAPH interposition — never the solid),
                 payload owner != ownerhandle, the 33/427 version pairs
                 (root + node), history_node_id resolution, node owner
                 == graph, graph owner == root, the solid's
                 history_handle linkage, the scalar root fields.
  G-C acds_genus the AcDs container: ds_version, the segidx-first
                 position, file_header_size, the num_segidx scale, the
                 populated-slot tail pattern, the named-pointer slot
                 types.

Specimen sources: the sh_history fixture family (always, decoded fresh —
the pinned copy is FIXTURES-ONLY so it regenerates identically in every
checkout) plus, on demand, the ACIS-bearing gold-tree files: --corpus-scan
reuses the corpus workdir's silver decodes
(target/gold_harness_corpus/<stem>/<stem>_silver_orig.json; skipped with
a notice when the workdir is absent), and any .dwg arguments are decoded
fresh alongside the fixtures. Expectations come from specimens, never
hand-pinned magic numbers (§20.5); values a specimen shows outside
its family's dominant genus are recorded under `anomalies` for review,
never silently absorbed.

The output is regenerable and deterministic; the pinned copy
(tests/gold_harness/genus_expectations.json) is diffed against a fresh
extraction in CI (the cargo mirror) so expectation drift is itself
reviewable. Where gold's interpretation is known-wrong the genus
reference stays the specimen bytes (§20.4) — this file never reads a
gold decode.

CLI: genus_extract.py [--out PATH] [--workdir PATH] [--corpus-scan]
                      [--fixtures DIR] [specimen.dwg ...]
"""

import argparse
import json
import os
import shutil
import struct
import subprocess
from collections import defaultdict
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO = SCRIPT_DIR.parents[1]
DEFAULT_FIXTURES = SCRIPT_DIR / "tests" / "sh_history"
DEFAULT_OUT = SCRIPT_DIR / "genus_expectations.json"
DEFAULT_WORKDIR = REPO / "target" / "genus_gates" / "extract"
CORPUS_WORKDIR = REPO / "target" / "gold_harness_corpus"

ACIS_MAGIC = b"ACIS BinaryFile"
ASM_MAGIC = b"ASM BinaryFile"

# ── The SAB walker (mirrors src/entities/acis/sab.rs framing) ──
#
# Header: magic ("ACIS BinaryFile" 15 B, or "ASM BinaryFile" 14 B + 1
# trailing byte — the version u32 begins at offset 15 either way),
# 4 × u32 LE (version, num_records, num_bodies, has_history), three
# tagged strings, then two or three tagged doubles (resfit is optional).
# Records: [SUBTYPE 0x0E + raw 1-byte len + bytes]* ENTITY_TYPE 0x0D +
# raw 1-byte len + bytes, attribute POINTER 0x0C + 4 B, subtype id
# INTEGER 0x04 + 4 B, tokens…, END_OF_RECORD 0x11 — except the
# End-of-ACIS/ASM-data terminator, which carries no attribute pointer
# and no EOR. Record width = first type tag through EOR inclusive (the
# terminator's width = its start to the blob end).


class SabWalkError(ValueError):
    pass


def _read_tagged_string(buf, off):
    tag = buf[off]
    if tag == 0x07:
        n = buf[off + 1]
        return buf[off + 2:off + 2 + n].decode("latin-1"), off + 2 + n
    if tag == 0x08:
        n = int.from_bytes(buf[off + 1:off + 3], "little")
        return buf[off + 3:off + 3 + n].decode("latin-1"), off + 3 + n
    if tag in (0x09, 0x12):
        n = int.from_bytes(buf[off + 1:off + 5], "little")
        return buf[off + 5:off + 5 + n].decode("latin-1"), off + 5 + n
    raise SabWalkError(f"not a string tag 0x{tag:02x} at {off}")


def walk_sab(sab):
    """Decode one SAB blob → (header dict, records list).

    Every record dict carries name/start/width; the terminator record is
    flagged. Raises SabWalkError on any framing surprise — the extractor
    must be complete over the specimen corpus, never silently truncate.
    """
    if sab.startswith(ACIS_MAGIC):
        magic, off = "ACIS", len(ACIS_MAGIC)
    elif sab.startswith(ASM_MAGIC):
        magic, off = "ASM", len(ASM_MAGIC) + 1
    else:
        raise SabWalkError("bad SAB magic")
    version, num_records, num_bodies, has_history = struct.unpack_from("<IIII", sab, off)
    off += 16
    product_id, off = _read_tagged_string(sab, off)
    product_version, off = _read_tagged_string(sab, off)
    date, off = _read_tagged_string(sab, off)
    tolerances = []
    while off < len(sab) and sab[off] == 0x06:
        tolerances.append(struct.unpack_from("<d", sab, off + 1)[0])
        off += 9
    header = {
        "magic": magic,
        "version": version,
        "num_records": num_records,
        "num_bodies": num_bodies,
        "has_history": has_history,
        "product_id": product_id,
        "product_version": product_version,
        "date": date,
        "tolerances": tolerances,
    }
    records = []
    while off < len(sab):
        start = off
        names = []
        while sab[off] in (0x0E, 0x0D):
            tag = sab[off]
            n = sab[off + 1]
            if off + 2 + n > len(sab):
                raise SabWalkError(f"truncated type name at {off}")
            names.append(sab[off + 2:off + 2 + n].decode("latin-1"))
            off += 2 + n
            if tag == 0x0D:
                break
        name = "-".join(names)
        if name.startswith("End-of-"):
            records.append({"name": name, "start": start,
                            "width": len(sab) - start, "terminator": True})
            break
        if off + 5 > len(sab) or sab[off] != 0x0C:
            raise SabWalkError(f"expected attribute pointer at {off} (record {name})")
        off += 5
        if off + 5 > len(sab) or sab[off] != 0x04:
            raise SabWalkError(f"expected subtype id at {off} (record {name})")
        off += 5
        while True:
            if off >= len(sab):
                raise SabWalkError(f"record {name} ran past the blob end (no EOR)")
            t = sab[off]
            if t == 0x11:
                off += 1
                break
            if t == 0x02:
                off += 2
            elif t == 0x03:
                off += 3
            elif t in (0x04, 0x05, 0x0C, 0x15):
                off += 5
            elif t in (0x06, 0x17):
                off += 9
            elif t == 0x07:
                off += 2 + sab[off + 1]
            elif t == 0x08:
                off += 3 + int.from_bytes(sab[off + 1:off + 3], "little")
            elif t in (0x09, 0x12):
                off += 5 + int.from_bytes(sab[off + 1:off + 5], "little")
            elif t in (0x0A, 0x0B, 0x0F, 0x10):
                off += 1
            elif t in (0x13, 0x14):
                off += 25
            elif t == 0x16:
                off += 17
            else:
                raise SabWalkError(f"unknown SAB tag 0x{t:02x} at {off} (record {name})")
        records.append({"name": name, "start": start, "width": off - start})
    return header, records


# ── Silver-side decode (dwg2json) ──


def resolve_cargo():
    """The cargo driver: $CARGO, then PATH, then the default rustup location."""
    candidates = [os.environ.get("CARGO"), shutil.which("cargo"),
                  str(Path.home() / ".cargo" / "bin" / "cargo")]
    for candidate in candidates:
        if candidate and Path(candidate).exists():
            return candidate
    raise SystemExit("cargo not found (set CARGO or export PATH)")


def build_dwg2json(cargo):
    """Build the harness decoder once; returns the binary path."""
    cmd = [cargo or "cargo", "build", "--quiet", "--bin", "dwg2json", "--features", "serde"]
    subprocess.run(cmd, cwd=REPO, check=True)
    binary = REPO / "target" / "debug" / "dwg2json"
    if not binary.exists():
        raise SystemExit("dwg2json binary missing after build")
    return binary


def decode_json(binary, dwg_path, json_path):
    subprocess.run([str(binary), str(dwg_path), str(json_path)],
                   check=True, stdout=subprocess.DEVNULL)


def acis_entities(doc):
    """Yield (entity_type, payload) for every ACIS-bearing entity."""
    for entry in doc.get("entities", []):
        (etype, payload), = entry.items()
        if etype in ("Solid3D", "Region", "Body"):
            yield etype, payload


def dynamic_blocks(doc):
    """Yield (handle, payload) for every DynamicBlock object record."""
    for key, value in doc.get("objects", {}).items():
        (otype, payload), = value.items()
        if otype == "DynamicBlock":
            yield int(key), payload


def unwrap_node(data):
    """SolidHistoryNode payloads are {SolidHistoryNode: {Variant: {...}}}."""
    (wrapper, inner), = data.items()
    if wrapper != "SolidHistoryNode":
        return None
    if isinstance(inner, dict) and "base" not in inner:
        (_, inner), = inner.items()
    return inner


# ── The three projections ──


def project_sab_form(stem, etype, payload, sink):
    acis = payload.get("acis_data", {})
    sab_bytes = acis.get("sab_data")
    if not sab_bytes:
        sat = acis.get("sat_data", "")
        if sat:
            sink["sat_only"] += 1
        return
    try:
        header, records = walk_sab(bytes(sab_bytes))
    except SabWalkError as exc:
        sink["anomalies"].append({"specimen": stem, "kind": "sab_walk", "detail": str(exc)})
        return
    sink["sab_carriers"] += 1
    sink["specimens"].add(stem)
    pair = [header["magic"], header["version"]]
    sink["magic_version_pairs"].add(tuple(pair))
    sink["header_triples"][tuple(pair)].add(
        (header["num_records"], header["num_bodies"], header["has_history"]))
    sink["product_strings"].add((header["product_id"], header["product_version"]))
    sink["tolerance_triples"].add(tuple(header["tolerances"]))
    order = []
    for record in records:
        name = record["name"]
        if record.get("terminator"):
            sink["terminator_names"].add(name)
            continue
        sink["class_widths"][name].add(record["width"])
        sink["class_specimens"][name].add(stem)
        if name not in order:
            order.append(name)
    sink["orders"].append(order)


def project_sh_genus(stem, doc, sink):
    objects = doc.get("objects", {})
    entities_by_handle = {}
    for entry in doc.get("entities", []):
        (etype, payload), = entry.items()
        # EntityCommon is a nested `common` map on the ACIS entities (the
        # handle lives there); some entity types flatten it to the top.
        handle = payload.get("handle") or payload.get("common", {}).get("handle")
        if handle is not None:
            entities_by_handle[handle] = etype

    for handle, payload in dynamic_blocks(doc):
        if payload.get("dxf_name") != "ACSH_HISTORY_CLASS":
            continue
        sink["roots"] += 1
        sink["specimens"].add(stem)
        data = payload.get("data", {})
        (wrapper, root), = data.items()
        if wrapper != "SolidHistory":
            sink["anomalies"].append({"specimen": stem, "kind": "sh_root_shape",
                                      "detail": f"unexpected wrapper {wrapper}"})
            continue
        owner_handle = payload.get("owner")
        sink["root_owner_entity_kinds"].add(entities_by_handle.get(owner_handle, "unresolved"))
        sink["owner_differs"].add(root["owner"] != owner_handle)
        graph = objects.get(str(root["owner"]))
        if graph is None:
            sink["anomalies"].append({"specimen": stem, "kind": "sh_graph_unresolved",
                                      "detail": f"payload owner {root['owner']} resolves to nothing"})
            continue
        (graph_type, graph_payload), = graph.items()
        if graph_type != "DynamicBlock":
            sink["anomalies"].append({"specimen": stem, "kind": "sh_graph_kind",
                                      "detail": f"payload owner resolves to {graph_type}"})
        sink["payload_owner_kinds"].add(graph_payload.get("dxf_name"))
        sink["graph_owner_is_root"].add(graph_payload.get("owner") == handle)
        sink["root_version_pairs"].add((root["major"], root["minor"]))
        sink["show_history"].add(root["show_history"])
        sink["record_history"].add(root["record_history"])
        # history_node_id must resolve to a written node owned by the graph.
        node_id = root["history_node_id"]
        sink["history_node_ids"].add(node_id)
        matches = []
        for node_handle, node_payload in dynamic_blocks(doc):
            dxf_name = node_payload.get("dxf_name", "")
            if not dxf_name.startswith("ACSH_") or dxf_name == "ACSH_HISTORY_CLASS":
                continue
            inner = unwrap_node(node_payload.get("data", {}))
            if inner is None:
                continue
            base = inner.get("base", {})
            step = base.get("step_id")
            eval_block = base.get("eval", {})
            if step == node_id or eval_block.get("node_id") == node_id:
                matches.append((node_handle, node_payload, base, eval_block))
        if not matches:
            sink["anomalies"].append({"specimen": stem, "kind": "sh_node_unresolved",
                                      "detail": f"history_node_id {node_id} resolves to no written node"})
            sink["node_ids_resolve"].add(False)
            continue
        sink["node_ids_resolve"].add(True)
        for node_handle, node_payload, base, eval_block in matches:
            sink["node_owner_is_graph"].add(node_payload.get("owner") == root["owner"])
            sink["node_version_pairs"].add((base.get("major"), base.get("minor")))
            sink["node_eval_version_pairs"].add((eval_block.get("major"), eval_block.get("minor")))
        # The owning solid's history soft-pointer must name this root.
        solid_history = None
        for _etype, entity in acis_entities(doc):
            entity_handle = entity.get("handle") or entity.get("common", {}).get("handle")
            if entity_handle == owner_handle:
                solid_history = entity.get("history_handle")
        if solid_history is not None:
            sink["solid_history_links_root"].add(solid_history == handle)


def project_acds_genus(stem, doc, sink):
    acds = doc.get("dwg_acds")
    if not acds:
        return
    # Pre-R2013 sections parse as the reader's empty placeholder (every
    # field zero, no segidx, no segments) — the jard-container genus is
    # carried by the R2013+ sections only; skip the placeholders.
    if not acds.get("segidx") and not acds.get("segments") and not acds.get("file_signature"):
        sink["empty_containers"] += 1
        return
    sink["containers"] += 1
    sink["specimens"].add(stem)
    sink["unknown_1s"].add(acds.get("unknown_1"))
    sink["container_versions"].add(acds.get("version"))
    sink["ds_versions"].add(acds.get("ds_version"))
    sink["segidx_offsets"].add(acds.get("segidx_offset"))
    sink["file_header_sizes"].add(acds.get("file_header_size"))
    segidx = acds.get("segidx", [])
    sink["num_segidx"].add(len(segidx))
    types = [segment.get("type") for segment in acds.get("segments", [])]
    n = len(types)
    tail = tuple(sorted((n - index, kind) for index, kind in enumerate(types)
                        if kind is not None))
    sink["tail_patterns"].add(tail)
    for pointer_name in ("schidx_segidx", "datidx_segidx", "search_segidx", "prvsav_segidx"):
        slot = acds.get(pointer_name)
        if slot is not None and 0 <= slot < n and types[slot] is not None:
            sink["named_pointer_types"][pointer_name].add(types[slot])
    # The 2026-09-30 wrapper-arm round-2 fields — the PER-SEGMENT
    # header values, keyed by segment NAME so the two schdat slots
    # (A at the early slot carrying ds 1, B at the schema pair
    # carrying 16) fold into one set honestly. `_data_` is excluded
    # on purpose: its payload is content-sized (one blob per SAB
    # record) while every OTHER segment's total size is the
    # authored container's fixed allocation. These fields exist
    # because the G-C hybrid slipped through the closed gate set:
    # the layout matched while the per-segment ds_version and the
    # schema-pair sizes carried a different era's values
    # (ds_version=1 against a 16/17 file header; schdat/schidx at
    # the Form-B 448 sizes against the Form-A 384/512/256).
    for segment in acds.get("segments", []):
        if not segment:
            continue
        name = segment.get("name")
        if not name or name == "_data_":
            continue
        sink["seg_slot_ds"].add(f"{name}={segment.get('ds_version')}")
        sink["seg_slot_sizes"].add(f"{name}={segment.get('segsize')}")
        sink["seg_slot_aligns"].add(
            f"{name}={segment.get('data_algn_offset')}/"
            f"{segment.get('objdata_algn_offset')}")


# ── Specimen sources ──


def fixture_specimens(fixtures_dir):
    return sorted(fixtures_dir.glob("*.dwg"))


def corpus_scan_specimens():
    """ACIS-bearing gold-tree files, discovered through the corpus workdir's
    silver decodes (no fresh decode — the JSONs are the same artifact this
    extractor would produce)."""
    found = []
    for json_path in sorted(CORPUS_WORKDIR.glob("*/*_silver_orig.json")):
        stem = json_path.name.removesuffix("_silver_orig.json")
        if (SCRIPT_DIR / "tests" / "sh_history" / f"{stem}.dwg").exists():
            continue  # fixtures are decoded fresh, never scanned
        try:
            with open(json_path) as handle:
                doc = json.load(handle)
        except (OSError, json.JSONDecodeError):
            continue
        for _etype, payload in acis_entities(doc):
            acis = payload.get("acis_data", {})
            if acis.get("sab_data") or acis.get("sat_data"):
                found.append((stem, json_path, doc))
                break
    return found


def extract(specimens, workdir, binary, corpus_scan):
    workdir.mkdir(parents=True, exist_ok=True)
    sink = {
        "sab": {"sab_carriers": 0, "sat_only": 0, "anomalies": [],
                "specimens": set(), "magic_version_pairs": set(),
                "header_triples": defaultdict(set), "product_strings": set(),
                "tolerance_triples": set(), "class_widths": defaultdict(set),
                "class_specimens": defaultdict(set), "terminator_names": set(),
                "orders": []},
        "sh": {"roots": 0, "anomalies": [], "specimens": set(),
               "root_owner_entity_kinds": set(), "owner_differs": set(),
               "payload_owner_kinds": set(), "graph_owner_is_root": set(),
               "root_version_pairs": set(), "show_history": set(),
               "record_history": set(), "history_node_ids": set(),
               "node_ids_resolve": set(), "node_owner_is_graph": set(),
               "node_version_pairs": set(), "node_eval_version_pairs": set(),
               "solid_history_links_root": set()},
        "acds": {"containers": 0, "empty_containers": 0, "specimens": set(),
                 "unknown_1s": set(), "container_versions": set(),
                 "ds_versions": set(),
                 "segidx_offsets": set(), "file_header_sizes": set(),
                 "num_segidx": set(), "tail_patterns": set(),
                 "named_pointer_types": defaultdict(set),
                 "seg_slot_ds": set(), "seg_slot_sizes": set(),
                 "seg_slot_aligns": set()},
        "fixture_files": [], "corpus_files": [],
    }

    for dwg_path in specimens:
        stem = dwg_path.stem
        json_path = workdir / f"{stem}.json"
        decode_json(binary, dwg_path, json_path)
        with open(json_path) as handle:
            doc = json.load(handle)
        sink["fixture_files"].append(stem)
        for etype, payload in acis_entities(doc):
            project_sab_form(stem, etype, payload, sink["sab"])
        project_sh_genus(stem, doc, sink["sh"])
        project_acds_genus(stem, doc, sink["acds"])

    if corpus_scan:
        scanned = corpus_scan_specimens()
        for stem, json_path, doc in scanned:
            sink["corpus_files"].append(stem)
            for etype, payload in acis_entities(doc):
                project_sab_form(stem, etype, payload, sink["sab"])
            project_sh_genus(stem, doc, sink["sh"])
            project_acds_genus(stem, doc, sink["acds"])
        if not CORPUS_WORKDIR.is_dir():
            print(f"note: corpus workdir absent ({CORPUS_WORKDIR}); "
                  "extraction ran fixtures-only")
    return sink


def pairwise_order(orders):
    """A -> classes that ALWAYS appear after A (across every order where
    both appear): appeared after A in at least one order and NEVER
    before it in any order — after[A] minus before[A]. Pairs the
    authored corpus orders both ways (e.g. cone-surface/plane-surface
    across specimen families) are variable, not genus, and must pin NO
    constraint; the raw union would false-positive both directions.
    The record-class ordering genus: a class may never precede a class
    that always precedes it."""
    after = defaultdict(set)
    before = defaultdict(set)
    for order in orders:
        for i, a in enumerate(order):
            for b in order[i + 1:]:
                after[a].add(b)
                before[b].add(a)
    return {a: sorted(after[a] - before[a]) for a in after}


def render_expectations(sink, corpus_scan_enabled):
    sab = sink["sab"]
    sh = sink["sh"]
    acds = sink["acds"]
    anomalies = [{"family": "sab_form", **a} for a in sab["anomalies"]]
    anomalies += [{"family": "sh_genus", **a} for a in sh["anomalies"]]

    def sorted_values(mapping):
        return {key: sorted(mapping[key]) for key in sorted(mapping)}

    expectations = {
        "format_version": 1,
        "generated_by": "genus_extract.py (IMPLEMENTATION.md §20.3)",
        "provenance": {
            "fixture_files": sorted(sink["fixture_files"]),
            "corpus_scan": {
                "enabled": corpus_scan_enabled,
                "files": sorted(sink["corpus_files"]),
            },
            "sab_carriers": sab["sab_carriers"],
            "sat_only_specimens": sab["sat_only"],
            "sh_roots": sh["roots"],
            "acds_containers": acds["containers"],
            "acds_empty_containers": acds["empty_containers"],
        },
        "sab_form": {
            "magic_version_pairs": sorted(sab["magic_version_pairs"]),
            "header_triples": {f"{m}|{v}": sorted(triples)
                               for (m, v), triples in sorted(sab["header_triples"].items())},
            "product_strings": sorted(sab["product_strings"]),
            "tolerance_triples": sorted(sab["tolerance_triples"]),
            "terminator_names": sorted(sab["terminator_names"]),
            "class_widths": sorted_values(sab["class_widths"]),
            "class_specimens": {name: sorted(stems)
                                for name, stems in sorted(sab["class_specimens"].items())},
            "order_constraints": pairwise_order(sab["orders"]),
        },
        "sh_genus": {
            "root_owner_entity_kinds": sorted(sh["root_owner_entity_kinds"]),
            "payload_owner_kinds": sorted(sh["payload_owner_kinds"]),
            "owner_differs_from_ownerhandle": sorted(sh["owner_differs"]),
            "graph_owner_is_root": sorted(sh["graph_owner_is_root"]),
            "root_version_pairs": sorted(sh["root_version_pairs"]),
            "node_version_pairs": sorted(sh["node_version_pairs"]),
            "node_eval_version_pairs": sorted(sh["node_eval_version_pairs"]),
            "history_node_ids": sorted(sh["history_node_ids"]),
            "node_ids_resolve": sorted(sh["node_ids_resolve"]),
            "node_owner_is_graph": sorted(sh["node_owner_is_graph"]),
            "solid_history_links_root": sorted(sh["solid_history_links_root"]),
            "show_history": sorted(sh["show_history"]),
            "record_history": sorted(sh["record_history"]),
        },
        "acds_genus": {
            "unknown_1s": sorted(acds["unknown_1s"]),
            "container_versions": sorted(acds["container_versions"]),
            "ds_versions": sorted(acds["ds_versions"]),
            "segidx_offsets": sorted(acds["segidx_offsets"]),
            "file_header_sizes": sorted(acds["file_header_sizes"]),
            "num_segidx": sorted(acds["num_segidx"]),
            "tail_patterns": [list(pattern) for pattern in sorted(acds["tail_patterns"])],
            "named_pointer_types": sorted_values(acds["named_pointer_types"]),
            "seg_slot_ds": sorted(acds["seg_slot_ds"]),
            "seg_slot_sizes": sorted(acds["seg_slot_sizes"]),
            "seg_slot_aligns": sorted(acds["seg_slot_aligns"]),
        },
        "anomalies": anomalies,
    }
    return expectations


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT,
                        help=f"expectations output (default {DEFAULT_OUT})")
    parser.add_argument("--workdir", type=Path, default=DEFAULT_WORKDIR,
                        help="scratch dir for the fresh specimen decodes")
    parser.add_argument("--fixtures", type=Path, default=DEFAULT_FIXTURES,
                        help="the sh_history specimen family directory")
    parser.add_argument("--corpus-scan", action="store_true",
                        help="fold in the ACIS-bearing gold-tree files "
                             "(reuses the corpus workdir's silver decodes)")
    parser.add_argument("specimens", nargs="*", type=Path,
                        help="extra specimen .dwg files (decoded fresh)")
    args = parser.parse_args()

    specimens = fixture_specimens(args.fixtures) + args.specimens
    if not specimens:
        raise SystemExit(f"no specimens under {args.fixtures}")

    binary = build_dwg2json(resolve_cargo())
    corpus_scan = args.corpus_scan
    sink = extract(specimens, args.workdir, binary, corpus_scan)
    expectations = render_expectations(sink, corpus_scan)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "w") as handle:
        json.dump(expectations, handle, indent=1, sort_keys=True)
        handle.write("\n")

    sab = sink["sab"]
    sh = sink["sh"]
    acds = sink["acds"]
    print(f"genus expectations -> {args.out}")
    print(f"  specimens: {len(sink['fixture_files'])} fixtures "
          f"+ {len(sink['corpus_files'])} corpus-scan files")
    print(f"  sab_form: {sab['sab_carriers']} SAB carriers, "
          f"{len(sab['class_widths'])} classes, "
          f"pairs {sorted(sab['magic_version_pairs'])}")
    print(f"  sh_genus: {sh['roots']} roots, "
          f"payload owners {sorted(sh['payload_owner_kinds'])}, "
          f"versions {sorted(sh['root_version_pairs'])}")
    print(f"  acds_genus: {acds['containers']} containers, "
          f"ds_version {sorted(acds['ds_versions'])}, "
          f"num_segidx {sorted(acds['num_segidx'])}")
    if expectations["anomalies"]:
        print(f"  ANOMALIES: {len(expectations['anomalies'])} (review before pinning)")
        for anomaly in expectations["anomalies"][:10]:
            print(f"    {anomaly}")


if __name__ == "__main__":
    main()
