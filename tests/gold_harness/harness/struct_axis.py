#!/usr/bin/env python3
"""The structure axis (IMPLEMENTATION.md §19, packet H0 — the skeleton).

A second, separately-gated comparison axis beside the OBJECTS axis: the
17 observed structure keys gold emits (§19.1's closed enumeration), with
a per-key leaf-census compare. This module is the measuring instrument —
the per-field typed projections land with the H2-H5 packets; H0 only
measures per-key presence and leaf-level gaps (name-canonical matches,
value diffs on matches, leaves missing on either side).

Design rules (§19.2's H0):
- The OBJECTS axis stays frozen: nothing here touches the record-list
  normalizers or the differ.
- `created_by` is EXCLUDED (gold's own PACKAGE_STRING stamp — not file
  content); VBAProject/Signature are DECLARED-ABSENT (corpus-absent);
  any UNDECLARED gold top-level key is surfaced (the no-leak assertion),
  never silently passed.

CLI: struct_axis.py <gold.json> <silver_or_gold2.json> [--write-target]
prints the per-file census JSON.
"""

import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

from normalize_gold import _sanitize_gold_acis  # the byte-level JSON fixer

# ── The key registry (§19.1's closed enumeration) ──

#: The 17 observed structure keys (the corpus-wide union; presence is
#: per-version, §19.1's matrix).
OBSERVED_KEYS: List[str] = [
    "HEADER",
    "FILEHEADER",
    "R2004_Header",
    "R2007_Header",
    "SecondHeader",
    "AuxHeader",
    "SummaryInfo",
    "AppInfo",
    "AppInfoHistory",
    "Template",
    "FileDepList",
    "RevHistory",
    "Security",
    "ObjFreeSpace",
    "THUMBNAILIMAGE",
    "AcDs",
    "CLASSES",
]

#: Keys with recorded exclusion reasons (never compared).
EXCLUDED_KEYS: Dict[str, str] = {
    "created_by": "gold's own PACKAGE_STRING stamp (out_json.c:2617) — oracle identity, not file content",
    "OBJECTS": "the body axis — already gated by the record-list comparison",
}

#: Declared in gold's drop-list but absent from the corpus files.
DECLARED_ABSENT: Dict[str, str] = {
    "VBAProject": "corpus-absent (the drop-list declares it)",
    "Signature": "corpus-absent AND gold's emitter is deliberately disabled (out_json.c:2673)",
}

_MAX_SAMPLES = 8
_MAX_SAMPLE_CHARS = 120
_REL_TOL = 1e-6


# ── Loaders (gold output needs the -nan / raw-ACIS shims) ──


def load_gold_json(path: Path) -> Dict[str, Any]:
    raw = _sanitize_gold_acis(open(str(path), "rb").read())
    s = raw.decode("utf-8", errors="replace")
    # Gold prints invalid doubles as -nan (bit-double error code '11');
    # normalize_gold maps them to NaN for its own strict=False load. Here
    # they map to a plain 0.0 for census counting (a census leaf with a
    # NaN on the gold side still counts as one leaf; exact NaN-vs-value
    # fidelity is record-axis work, already handled there).
    s = re.sub(r"(?<![\"\w])-?nan(?![\"\w])", "0.0", s)
    s = s.replace("-nan", "0.0").replace("NaN", "0.0")
    return json.loads(s, strict=False)


def load_silver_json(path: Path) -> Dict[str, Any]:
    return json.load(open(str(path), "r", encoding="utf-8"))


def load_auto(path: Path) -> Tuple[Dict[str, Any], str]:
    """Load a raw dump, robustly: try plain JSON first (silver), fall
    back to the gold shims. Returns (doc, side) with side 'gold'/'silver'."""
    try:
        return load_silver_json(path), "silver"
    except (json.JSONDecodeError, UnicodeDecodeError):
        return load_gold_json(path), "gold"


# ── Structure views ──


def gold_structure_views(data: Dict[str, Any]) -> Tuple[Dict[str, Any], List[str]]:
    """Extract the observed keys; return (views, undeclared-top-level-keys).

    The undeclared list drives the no-leak assertion: a gold top-level
    key outside OBSERVED/EXCLUDED/ABSENT must surface, never silently
    pass (§19.2's H0 corollary — `Signature` was the latent example).
    """
    views: Dict[str, Any] = {k: data.get(k) for k in OBSERVED_KEYS}
    known = set(OBSERVED_KEYS) | set(EXCLUDED_KEYS) | set(DECLARED_ABSENT)
    undeclared = sorted(k for k in data.keys() if k not in known)
    return views, undeclared



#: Gold's 256-entry RGB palette (libredwg dwg.c rgb_palette), hex-packed
#: 3 bytes per entry — the authority for the CMC derived-index prints
#: (§19 H3: dwg_find_color_index is a first-match linear scan over the
#: unsorted palette; silver's ACI table differs on 222 of 256 entries).
_PALETTE_HEX = (
    "000000ff0000ffff0000ff0000ffff0000ffff00ffffffff414141808080ff0000ffaaaa"
    "bd0000bd7e7e8100008156566800006845454f00004f3535ff3f00ffbfaabd2e00bd8d7e"
    "811f00816056681900684e454f13004f3b35ff7f00ffd4aabd5e00bd9d7e814000816b56"
    "6834006856454f27004f4235ffbf00ffeaaabd8d00bdad7e816000817656684e00685f45"
    "4f3b004f4935ffff00ffffaabdbd00bdbd7e8181008181566868006868454f4f004f4f35"
    "bfff00eaffaa8dbd00adbd7e6081007681564e68005f68453b4f00494f357fff00d4ffaa"
    "5ebd009dbd7e4081006b8156346800566845274f00424f353fff00bfffaa2ebd008dbd7e"
    "1f81006081561968004e6845134f003b4f3500ff00aaffaa00bd007ebd7e008100568156"
    "006800456845004f00354f3500ff3faaffbf00bd2e7ebd8d00811f56816000681945684e"
    "004f13354f3b00ff7faaffd400bd5e7ebd9d00814056816b006834456856004f27354f42"
    "00ffbfaaffea00bd8d7ebdad00816056817600684e45685f004f3b354f4900ffffaaffff"
    "00bdbd7ebdbd008181568181006868456868004f4f354f4f00bfffaaeaff008dbd7eadbd"
    "006081567681004e68455f68003b4f35494f007fffaad4ff005ebd7e9dbd004081566b81"
    "00346845566800274f35424f003fffaabfff002ebd7e8dbd001f81566081001968454e68"
    "00134f353b4f0000ffaaaaff0000bd7e7ebd00008156568100006845456800004f35354f"
    "3f00ffbfaaff2e00bd8d7ebd1f00816056811900684e456813004f3b354f7f00ffd4aaff"
    "5e00bd9d7ebd4000816b568134006856456827004f42354fbf00ffeaaaff8d00bdad7ebd"
    "6000817656814e00685f45683b004f49354fff00ffffaaffbd00bdbd7ebd810081815681"
    "6800686845684f004f4f354fff00bfffaaeabd008dbd7ead81006081567668004e68455f"
    "4f003b4f3549ff007fffaad4bd005ebd7e9d81004081566b6800346845564f00274f3542"
    "ff003fffaabfbd002ebd7e8d81001f81566068001968454e4f00134f353b333333505050"
    "696969828282bebebeffffff"
)

_GOLD_PALETTE = bytes.fromhex(_PALETTE_HEX)


def _gold_find_color_index(rgb: int) -> int:
    """dwg_find_color_index: the first palette match of rgb's low 24 bits."""
    rgb &= 0x00FFFFFF
    r = (rgb >> 16) & 0xFF
    g = (rgb >> 8) & 0xFF
    b = rgb & 0xFF
    for i in range(256):
        j = 3 * i
        if _GOLD_PALETTE[j] == r and _GOLD_PALETTE[j + 1] == g and _GOLD_PALETTE[j + 2] == b:
            return i
    return 256


#: The HEADER keys whose DwgHeaderRaw value is a raw CMC (DwgRawCmc).
_HEADER_CMC_KEYS = frozenset(
    {"CECOLOR", "DIMCLRD", "DIMCLRE", "DIMCLRT", "DIMTFILLCLR", "INTERFERECOLOR"}
)


def _project_header_cmc(raw: Any, pre2004: bool) -> Any:
    """Project a raw CMC into gold's JSON shape (out_json.c field_cmc).

    The unique rule, re-derived from libredwg src (bits.c bit_read_CMC +
    out_json.c field_cmc): decode overwrites the wire BS index with
    dwg_find_color_index(rgb) and validates flag (>= 4 is zeroed, strings
    not read) and method (out of 0xC0..0xC8 forced to 0xC2) — silver's
    DwgRawCmc reader mirrors both validations, so this projection is a
    pure emitter:

    - pre-R2004: the bare (unsigned) index;
    - R2004+: index printed iff the palette lookup is non-zero —
      INCLUDING 256 (no palette match; the emitter's else-branch
      derivations are dead: a zero lookup implies rgb & 0xFFFFFF == 0,
      so the TRUECOLOR low-byte and BYLAYER/BYBLOCK re-lookup also
      yield 0); rgb as "%06x"; flag iff non-zero (the alpha 0x20 /
      handle 0x40 emitter branches are unreachable — the validated flag
      is 0..3); name / book_name behind flag bits 0/1.
    """
    if not isinstance(raw, dict):
        return raw
    if pre2004:
        return raw.get("index", 0)
    rgb = int(raw.get("rgb", 0))
    flag = int(raw.get("flag", 0))
    out: Dict[str, Any] = {}
    index = _gold_find_color_index(rgb)
    if index:
        out["index"] = index
    out["rgb"] = "%06x" % rgb
    if flag:
        out["flag"] = flag
    if flag & 1:
        out["name"] = raw.get("name", "")
    if flag & 2:
        out["book_name"] = raw.get("book_name", "")
    return out


#: Silver's seed projections — replaced per-packet with real field-list
#: projections from the spec files (H0 established the baseline; H2
#: lands FILEHEADER).
def silver_structure_views(doc: Dict[str, Any]) -> Dict[str, Any]:
    views: Dict[str, Any] = {}
    fh_doc = doc.get("dwg_file_header")
    if isinstance(fh_doc, dict):
        # The H2 projection: the reader's DwgFileHeaderSummary carries
        # gold's FILEHEADER field names 1:1 (codepage included). The
        # R2004+ tail is dropped on pre-2004 files — gold emits only the
        # 8-field family there (version gates: gold's PRE(R_2004a)).
        fh = dict(fh_doc)
        version = str(fh.get("version", ""))
        if version in ("AC1012", "AC1014", "AC1015"):
            for k in (
                "unknown_0",
                "app_dwg_version",
                "app_maint_version",
                "security_type",
                "rl_1c_address",
                "summaryinfo_address",
                "vbaproj_address",
                "r2004_header_address",
            ):
                fh.pop(k, None)
        else:
            # The R2004+ family: gold emits no `sections` (the locator
            # count is PRE(R_2004a), and the reader leaves it 0 there).
            fh.pop("sections", None)
        views["FILEHEADER"] = fh
    else:
        # The H0 seed (documents predating the H2 retention): the two
        # document-level scalars.
        fh: Dict[str, Any] = {}
        if isinstance(doc.get("version"), str):
            fh["version"] = doc["version"]
        if isinstance(doc.get("maintenance_version"), int):
            # silver's `maintenance_version` is gold's `maint_rel_version`
            # (the release level: AC1015 files carry 15 there while gold's
            # `maint_version` stays 0 — the R2000 census finding that pinned
            # this mapping).
            fh["maint_rel_version"] = doc["maintenance_version"]
        if fh:
            views["FILEHEADER"] = fh
    # H2's R2004_Header sub-row: the document's r2004 summary is gold's
    # shape 1:1 (AC18-format files only; the separate R2007_Header
    # sub-row stays open).
    if isinstance(doc.get("dwg_r2004_header"), dict):
        views["R2004_Header"] = doc["dwg_r2004_header"]
    # H2's R2007_Header sub-row: the gold-named projection of the AC1021
    # container metadata (33 fields; sections_amount has no gold-emitted
    # counterpart and is dropped at the projection source).
    if isinstance(doc.get("dwg_r2007_header"), dict):
        views["R2007_Header"] = doc["dwg_r2007_header"]
    # H2's R2000 pair: the sentinel-located SecondHeader and the
    # locator-addressed AuxHeader — both gold-JSON-shaped summaries.
    if isinstance(doc.get("dwg_second_header"), dict):
        views["SecondHeader"] = doc["dwg_second_header"]
    if isinstance(doc.get("dwg_aux_header"), dict):
        views["AuxHeader"] = doc["dwg_aux_header"]
    # H4's metadata blocks — gold-JSON-shaped summaries (the canonical
    # name matcher aligns snake_case onto gold's names, e.g.
    # tdindwg → TDINDWG, measurement → MEASUREMENT).
    if isinstance(doc.get("summary_info"), dict):
        # Gold emits SummaryInfo only when the FILEHEADER's
        # summaryinfo_address is set (out_json.c:2663) — mirror the gate
        # (§19 H7 review, the gh209_1 presence coupling): a document read
        # from a file whose author carried no section stays absent on
        # both sides. This also covers the pre-R2004 family: no address
        # at all (the reader's field is 0 there), so no SummaryInfo view
        # is ever projected for them — gold never emits it pre-R2004.
        fh_raw = doc.get("dwg_file_header")
        addr = (
            fh_raw.get("summaryinfo_address", 0)
            if isinstance(fh_raw, dict)
            else 0
        )
        if isinstance(addr, int) and addr != 0:
            views["SummaryInfo"] = doc["summary_info"]
    if isinstance(doc.get("dwg_template"), dict):
        views["Template"] = doc["dwg_template"]
    if isinstance(doc.get("dwg_file_dep_list"), dict):
        views["FileDepList"] = doc["dwg_file_dep_list"]
    if isinstance(doc.get("dwg_rev_history"), dict):
        views["RevHistory"] = doc["dwg_rev_history"]
    if isinstance(doc.get("dwg_security"), dict):
        views["Security"] = doc["dwg_security"]
    if isinstance(doc.get("dwg_obj_free_space"), dict):
        views["ObjFreeSpace"] = doc["dwg_obj_free_space"]
    if isinstance(doc.get("dwg_app_info"), dict):
        views["AppInfo"] = doc["dwg_app_info"]
    if isinstance(doc.get("dwg_app_info_history"), dict):
        views["AppInfoHistory"] = doc["dwg_app_info_history"]
    if isinstance(doc.get("dwg_acds"), dict):
        # H5a: gold's AcDs shape (json_section_acds, acds.spec) — the
        # data-store section outline: header fields + segidx[] +
        # segments[] with the per-type sub-blocks, REPEAT counts
        # suppressed; absent entirely on the R2000 family.
        views["AcDs"] = doc["dwg_acds"]
    if isinstance(doc.get("dwg_header_raw"), dict):
        # H3: gold's HEADER shape (json_header_write →
        # header_variables.spec): the reader's DwgHeaderRaw mirror, one
        # field per gold key, named and version-gated exactly as gold
        # prints them. The CMC fields carry the raw wire parts and get
        # gold's emitter shape here (out_json.c field_cmc: the
        # palette-derived index, the "%06x" rgb word, the flag-gated
        # name/book_name), with pre-R2004 files printing the bare
        # index.
        raw = doc["dwg_header_raw"]
        pre2004 = str(raw.get("__version", "")) in (
            "AC1012",
            "AC1014",
            "AC1015",
        )
        views["HEADER"] = {
            k: (_project_header_cmc(v, pre2004) if k in _HEADER_CMC_KEYS else v)
            for k, v in raw.items()
            if k != "__version"
        }
    elif isinstance(doc.get("header"), dict):
        views["HEADER"] = doc["header"]
    if isinstance(doc.get("preview"), dict):
        # H5c: gold's THUMBNAILIMAGE shape (json_thumbnail_write,
        # out_json.c): {size, chain} — the container's data between the
        # 16-byte sentinel and the 2-byte CRC, hex-encoded (uppercase).
        # Uniform across every version (pinned: the chain starts exactly
        # 16 bytes past the thumbnail address on R2000/R2004/R2018;
        # size == the chain byte count on all three).
        raw = doc["preview"].get("raw") or []
        version = str(
            (doc.get("dwg_file_header") or {}).get("version")
            or doc.get("version")
            or ""
        )
        # The tail families are gold's PER-DECODER rules (H5c, pinned in
        # libredwg decode.c — not a simple version split):
        # - pre-R2004 (bracketed container read): chain = the mid-section
        #   between the two 16-byte sentinels (sample_2000: len 17039 =
        #   16 + 17007 + 16);
        # - AC1021 (decode_R2007 thumbnail read): size = sec − 32 — also
        #   cuts BOTH sentinels (Box_2007: len 36783 -> size 36751;
        #   decode.c "2x sentinel");
        # - the rest of the R2004 family (read_2004_section_preview):
        #   size = sec − 16, the whole tail KEPT — the 2-byte CRC inside
        #   the extent (sample_2018: len 2150 -> size 2134) or even a
        #   16-byte end sentinel where the author wrote one (2018/Arc:
        #   len 126 -> size 110, byte-pinned). The chain starts exactly
        #   16 past the thumbnail address on every layout; size == the
        #   chain byte count.
        both_sentinels = version in ("AC1012", "AC1014", "AC1015", "AC1021")
        if isinstance(raw, list) and len(raw) > (32 if both_sentinels else 16):
            if both_sentinels:
                views["THUMBNAILIMAGE"] = {
                    "size": len(raw) - 32,
                    "chain": bytes(raw[16:-16]).hex().upper(),
                }
            else:
                views["THUMBNAILIMAGE"] = {
                    "size": len(raw) - 16,
                    "chain": bytes(raw[16:]).hex().upper(),
                }
        else:
            views["THUMBNAILIMAGE"] = doc["preview"]
    if isinstance(doc.get("classes"), dict) and isinstance(
        doc["classes"].get("entries"), list
    ):
        # H5b: gold's CLASSES shape (json_classes_write, out_json.c:1988):
        # number/dxfname/cppname/appname/proxyflag/num_instances/
        # is_zombie/item_class_id, then dwg_version + maint_version
        # SINCE R_2004a. The numeric fields come from the gold-shadow
        # record (DxfClass::gold_shadow — gold's own walk values, which
        # carry the desync garbage on the 2027.1-authored tables) with
        # the sane parse as the None-fallback. Silver's extra fields
        # (is_an_entity/unknown1/unknown2) do not print in gold and are
        # dropped.
        version = str(
            (doc.get("dwg_file_header") or {}).get("version")
            or doc.get("version")
            or ""
        )
        pre_r2004 = version in ("AC1012", "AC1014", "AC1015")
        out = []
        for e in doc["classes"]["entries"]:
            sh = e.get("gold_shadow") or {}
            rec = {
                "number": sh.get("number", e.get("class_number")),
                "dxfname": e.get("dxf_name"),
                "cppname": e.get("cpp_class_name"),
                "appname": e.get("application_name"),
                "proxyflag": sh.get("proxyflag", e.get("proxy_flags")),
                "num_instances": sh.get("num_instances", e.get("instance_count")),
                "is_zombie": sh.get(
                    "is_zombie", 1 if e.get("was_zombie") else 0
                ),
                "item_class_id": sh.get("item_class_id", e.get("item_class_id")),
            }
            if not pre_r2004:
                rec["dwg_version"] = sh.get(
                    "dwg_version", e.get("dwg_version")
                )
                rec["maint_version"] = sh.get(
                    "maint_version", e.get("maintenance_version")
                )
            out.append(rec)
        views["CLASSES"] = out
    return views


# ── The leaf census ──


def _canon(name: Any) -> str:
    """Canonical leaf-name for cross-side matching: silver's snake_case
    names canon onto gold's mixed-case names (required_versions →
    REQUIREDVERSIONS). H3's per-variable ledger replaces this with the
    authoritative name map."""
    return re.sub(r"[^A-Z0-9]", "", str(name).upper())


def flatten(value: Any, prefix: str = "") -> Dict[str, Any]:
    """Flatten a nested dict/list to a leaf map (path → scalar).

    Scalars (str/int/float/bool/None) are terminal; dicts recurse per
    key; lists recurse per index. Long hex strings stay one leaf (the
    thumbnail blob compares as one leaf — byte fidelity lands in H5's
    digest work)."""
    out: Dict[str, Any] = {}
    if isinstance(value, dict):
        for k, v in value.items():
            out.update(flatten(v, f"{prefix}.{k}" if prefix else str(k)))
    elif isinstance(value, list):
        for i, v in enumerate(value):
            out.update(flatten(v, f"{prefix}[{i}]"))
    else:
        out[prefix] = value
    return out


def _leaves_equal(a: Any, b: Any) -> bool:
    if isinstance(a, bool) or isinstance(b, bool):
        return a is b if isinstance(a, bool) and isinstance(b, bool) else a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        if a == b:
            return True
        try:
            return abs(a - b) <= _REL_TOL * max(abs(a), abs(b), 1.0)
        except (TypeError, OverflowError):
            return False
    if isinstance(a, str) and isinstance(b, str):
        return a == b
    return a == b


def compare_key_views(
    gold_value: Any, other_value: Any, other_side: str = "silver"
) -> Dict[str, Any]:
    """Leaf-census one structure key between gold and the other side.

    Counts (never field-level projections — that is the H2-H5 work):
    - `matched` leaves (name-canonical match on both sides)
    - `value_diffs` among the matched
    - `missing_<other_side>`: gold leaves the other side lacks
    - `missing_gold`: other-side leaves gold lacks (projection noise)
    - `samples`: the first few diffing leaf names, for eyeballing
    """
    if gold_value is None and other_value is None:
        return {"status": "absent_both"}
    if gold_value is None:
        return {
            "status": "missing_gold",
            "other_leaves": len(flatten(other_value)),
        }
    if other_value is None:
        return {
            "status": f"missing_{other_side}",
            "gold_leaves": len(flatten(gold_value)),
        }
    g = flatten(gold_value)
    o = flatten(other_value)
    o_by_canon: Dict[str, Tuple[str, Any]] = {}
    for k, v in o.items():
        o_by_canon.setdefault(_canon(k), (k, v))
    g_by_canon: Dict[str, Tuple[str, Any]] = {}
    for k, v in g.items():
        g_by_canon.setdefault(_canon(k), (k, v))

    matched = 0
    value_diffs = 0
    missing_other = 0
    missing_gold = 0
    samples: List[str] = []
    for k, v in g.items():
        hit = o_by_canon.get(_canon(k))
        if hit is None:
            missing_other += 1
            continue
        matched += 1
        if not _leaves_equal(v, hit[1]):
            value_diffs += 1
            if len(samples) < _MAX_SAMPLES:
                # Trim each repr: sample leaves can be multi-KB hex blobs
                # (thumbnails, AppInfo unknown_bits) that would bloat every
                # per-file census artifact.
                entry = f"{k}: gold={v!r} vs {other_side}={hit[1]!r}"
                if len(entry) > _MAX_SAMPLE_CHARS:
                    entry = entry[:_MAX_SAMPLE_CHARS] + "…"
                samples.append(entry)
    for k in o:
        if _canon(k) not in g_by_canon:
            missing_gold += 1

    return {
        "status": "present_both",
        "gold_leaves": len(g),
        "other_leaves": len(o),
        "matched": matched,
        "value_diffs": value_diffs,
        f"missing_{other_side}": missing_other,
        "missing_gold": missing_gold,
        "samples": samples,
    }


def compare_structure(
    gold_data: Dict[str, Any],
    other_data: Dict[str, Any],
    other_side: str = "silver",
    other_is_gold_dump: bool = False,
) -> Dict[str, Any]:
    """The per-file census: every observed key compared (or accounted
    absent/missing), exclusions recorded, undeclared keys surfaced."""
    gold_views, undeclared = gold_structure_views(gold_data)
    if other_is_gold_dump:
        # The write-preservation axis: the other side is gold's own read
        # of the rewritten file — same view extraction, no silver mapping.
        other_views, _ = gold_structure_views(other_data)
    else:
        other_views = silver_structure_views(other_data)

    per_key: Dict[str, Any] = {}
    leaf_sum_read_axis = 0
    for key in OBSERVED_KEYS:
        r = compare_key_views(gold_views.get(key), other_views.get(key), other_side)
        per_key[key] = r
        if r.get("status") == "present_both":
            leaf_sum_read_axis += r["value_diffs"] + r[f"missing_{other_side}"]
        elif r.get("status") == f"missing_{other_side}":
            leaf_sum_read_axis += r["gold_leaves"]

    absent_notes: Dict[str, str] = {}
    for key, reason in DECLARED_ABSENT.items():
        if gold_data.get(key) is not None:
            absent_notes[key] = f"PRESENT on gold (expected absent: {reason})"
        else:
            absent_notes[key] = "absent as declared"

    return {
        "axis": "read" if other_side == "silver" else "write-target",
        "per_key": per_key,
        "undeclared_keys": undeclared,
        "declared_absent": absent_notes,
        "excluded": dict(EXCLUDED_KEYS),
        "totals": {
            # The attack-order number: projection work needed per file
            "key_gap_sum": leaf_sum_read_axis,
            "keys_missing_other": sum(
                1 for r in per_key.values() if r.get("status") == f"missing_{other_side}"
            ),
            "keys_present_both": sum(
                1 for r in per_key.values() if r.get("status") == "present_both"
            ),
        },
    }


def census_from_files(
    gold_path: Path, other_path: Path, other_side: str = "auto"
) -> Dict[str, Any]:
    """Load both raw dumps and run the census.

    `other_side` must be `"silver"` (the read axis: silver's dwg2json
    projection) or `"gold_rt"` (the write-preservation axis: gold's own
    read of the rewritten file, same view extraction as gold_orig).
    The sides differ in VIEW EXTRACTION, not just parsing: a gold dump
    that happens to parse as plain JSON (no -nan tokens) must still be
    extracted gold-style — so callers that know the side pass it, and
    `"auto"` (the CLI fallback) guesses by parse behavior only.
    """
    gold_doc = load_gold_json(gold_path)
    if other_side == "auto":
        _, detected = load_auto(other_path)
        other_side = "gold_rt" if detected == "gold" else "silver"
    if other_side == "silver":
        other_doc = load_silver_json(other_path)
        other_is_gold = False
    else:
        other_doc = load_gold_json(other_path)
        other_is_gold = True
    comparison = compare_structure(
        gold_doc,
        other_doc,
        other_side="gold_rt" if other_is_gold else "silver",
        other_is_gold_dump=other_is_gold,
    )
    comparison["gold_file"] = str(gold_path)
    comparison["other_file"] = str(other_path)
    return comparison


def main() -> int:
    if len(sys.argv) < 3:
        print(
            "Usage: struct_axis.py <gold.json> <silver_or_gold2.json> [silver|gold_rt|auto]\n"
            "  The other side: 'silver' (read axis), 'gold_rt' (the write-\n"
            "  preservation axis: gold's read of the rewrite), or 'auto'\n"
            "  (guess by parse behavior).",
            file=sys.stderr,
        )
        return 2
    side = sys.argv[3] if len(sys.argv) >= 4 else "auto"
    result = census_from_files(Path(sys.argv[1]), Path(sys.argv[2]), side)
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)
    return 0


if __name__ == "__main__":
    sys.exit(main())
