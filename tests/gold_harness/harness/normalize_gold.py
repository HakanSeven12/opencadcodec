#!/usr/bin/env python3
"""Normalize LibreDWG `dwgread -O JSON` output to a canonical object list.

Keeps only the top-level OBJECTS array, drops all header/section keys, and
flattens values to a simple, diffable form:
  - handles become dicts with keys code/size/value/absref (or a plain int when
    the handle resolves to 0 / NULL)
  - 2D/3D points become [x, y] or [x, y, z]
  - colors become simple ints or dicts only if they carry RGB/name data
"""

import json
import re
import sys
from typing import Any, Dict, List, Optional


def _sanitize_gold_acis(raw: bytes) -> bytes:
    """Re-escape gold's raw first acis_data element when it carries binary.

    Gold pretty-prints `acis_data` as ["<raw SAB slice>", "hex…"]. The raw
    slice may contain ANY byte — control chars, non-UTF-8, quotes, backslashes
    — making the output invalid JSON. A scanner (not a regex: slice bytes can
    mimic any anchor) re-escapes the first element in place when it is a raw
    slice: the element must be followed by `,` (a second/hex element exists).
    `[""]`-style empty arrays are two adjacent quotes and pass through
    verbatim. If a scan finds no proper element end it leaves the whole span
    untouched (never corrupts).
    """
    need = b'"acis_data": ['
    out = bytearray()
    pos = 0
    n = len(raw)
    while True:
        j = raw.find(need, pos)
        if j < 0:
            out += raw[pos:]
            return bytes(out)
        out += raw[pos : j + len(need)]
        k = j + len(need)
        while k < n and raw[k] in b"\n\r \t":
            out.append(raw[k])
            k += 1
        if k >= n or raw[k] != 0x22:
            pos = k
            continue
        open_q = k
        if raw[open_q + 1 : open_q + 2] == b'"':
            # [""] — the only element is an empty string; emit verbatim.
            out += b'""'
            pos = open_q + 2
            continue
        k = open_q + 1
        esc = bytearray()
        closed = -1
        while k < n:
            b = raw[k]
            if b == 0x22 and raw[k + 1 : k + 2] == b",":
                closed = k  # position of the closing quote
                break
            if b == 0x22:
                esc += b'\\"'
            elif b == 0x5C:
                esc += b'\\\\'
            elif b == 0x0A:
                esc += b'\\n'
            elif b == 0x0D:
                esc += b'\\r'
            elif b == 0x09:
                esc += b'\\t'
            elif 0x20 <= b <= 0x7E:
                esc.append(b)
            else:
                esc += b"\\u%04x" % b
            k += 1
        if closed < 0:
            # No element end found; leave the opening quote verbatim and
            # continue scanning after it (never corrupt).
            out += b'"'
            pos = open_q + 1
            continue
        out += b'"' + bytes(esc) + b'"'
        pos = closed + 1  # the comma and the rest are copied verbatim next round


HEADER_KEYS = {
    "created_by",
    "FILEHEADER",
    "HEADER",
    "CLASSES",
    "THUMBNAILIMAGE",
    "ObjFreeSpace",
    "SecondHeader",
    "Template",
    "AuxHeader",
    "R2004_Header",
    "R2007_Header",
    "SummaryInfo",
    "VBAProject",
    "AppInfo",
    "AppInfoHistory",
    "FileDepList",
    "Security",
    "RevHistory",
    "AcDs",
}


def load_ignore_fields(path: str) -> Dict[str, Any]:
    try:
        import tomllib

        with open(path, "rb") as f:
            data = tomllib.load(f)
    except ImportError:
        import tomli as tomllib  # type: ignore

        with open(path, "rb") as f:
            data = tomllib.load(f)
    return data


def is_ignored(name: str, ignore_set: set, ignore_patterns: List[str]) -> bool:
    if name in ignore_set:
        return True
    for pat in ignore_patterns:
        if pat.endswith("*") and name.startswith(pat[:-1]):
            return True
        if pat.startswith("*") and name.endswith(pat[1:]):
            return True
    return False


def normalize_handle(value: Any) -> Any:
    """LibreDWG emits handles as [code, size, value, absref?]."""
    if isinstance(value, list) and len(value) >= 3:
        return {
            "code": value[0],
            "size": value[1],
            "value": value[2],
            "absref": value[3] if len(value) > 3 else value[2],
        }
    return value


def normalize_value(value: Any) -> Any:
    if isinstance(value, list):
        # Heuristic: 2/3 numeric elements are a point; 3/4-element int-only
        # arrays are handles (LibreDWG emits both 3- and 4-element forms).
        if len(value) in (2, 3) and all(isinstance(v, (int, float)) for v in value):
            # Distinguish point from handle: points contain floats.
            if any(isinstance(v, float) for v in value):
                return [round(v, 14) if isinstance(v, float) else v for v in value]
            if len(value) == 3 and all(isinstance(v, int) for v in value):
                return normalize_handle(value)
        if len(value) in (3, 4) and all(isinstance(v, int) for v in value):
            return normalize_handle(value)
        return [normalize_value(v) for v in value]
    if isinstance(value, dict):
        # CMC color hash? Collapse to the index when the rgb payload is absent
        # or zero (ByLayer/ByBlock/indexed colors). R2004+ ENC output includes
        # derived keys (`rgb`, `flag`) alongside `index`; the index is the
        # semantic value silver stores. True-color entities keep their dict.
        if set(value.keys()) == {"index"}:
            return value["index"]
        if "index" in value and value.get("rgb") in (None, "000000", 0):
            return value["index"]
        # ByBlock-with-transparency: gold's field_cmc omits `index` when it
        # resolves to 0 (the `index > 0 && index < 256` check in out_json.c),
        # leaving {"rgb":"000000", flag:32, alpha_type:1, ...}. alpha_type 1 is
        # ByBlock transparency; rgb 000000 + no index = ByBlock color. Silver
        # stores this as Color::ByBlock -> 0. Collapse so the dict doesn't diff
        # against silver's scalar. (Only when there is no `index` key at all —
        # a present index is the semantic value and is handled above.)
        if "index" not in value and value.get("rgb") in (None, "000000", 0):
            return 0
        # CMC `rgb` high byte is the color METHOD (bits.c bit_downconvert_CMC
        # 4061 / include/dwg.h DWG_COLOR_METHOD): c0 = ByLayer, c1 = ByBlock,
        # c2 = ACI/entity color (low 24 bits hold the rgb), c3 = truecolor
        # (indexed variant holds the real index in the low byte), c8 = "none".
        # Collapse to silver's scalar convention.
        rgb = value.get("rgb")
        if isinstance(rgb, str) and len(rgb) == 8:
            flag = rgb[:2]
            if flag == "c3":
                try:
                    return int(rgb[6:8], 16)
                except ValueError:
                    pass
            if flag == "c0":
                return 256      # ByLayer
            if flag == "c1":
                return 0        # ByBlock
            if flag == "c8":
                return 257      # none
        return {k: normalize_value(v) for k, v in value.items()}
    if isinstance(value, float):
        # Normalize -0.0 and NaN the same way gold release build does.
        if value != value:
            return 0.0
        if value == 0.0:
            return 0.0
        return round(value, 14)
    return value


def object_type(obj: Dict[str, Any]) -> str:
    return obj.get("entity") or obj.get("object") or "UNKNOWN"


def normalize_gold(data: Dict[str, Any], ignore: Optional[Dict[str, Any]] = None) -> List[Dict[str, Any]]:
    ignore_set: set = set()
    ignore_patterns: List[str] = []
    if ignore:
        ignore_set.update(ignore.get("global", {}).get("ignore", []))
        for pat in ignore.get("global", {}).get("ignore", []):
            if "*" in pat:
                ignore_patterns.append(pat)

    objects = data.get("OBJECTS", [])
    out: List[Dict[str, Any]] = []

    # R2013+ (AC1027/AC1032) file version gate for the 3DSOLID-family
    # prologue-divergence drop below. LibreDWG's 3DSOLID spec reads a legacy
    # leading `acis_empty` bit that AutoCAD's R2013+ AcDs-backed records
    # never carry (`DECODE_3DSOLID`, dwg_spec_shared.h 175), so gold's decode
    # derails on those records and emits desync garbage for COMMON_3DSOLID's
    # wireframe/revision internals (isolines=205, revision_major=big, hex
    # revision_bytes, absent points; "Invalid REGION.wires" aborts) while
    # silver's reader is bit-true on the very same bytes (verified against
    # the raw object records with dump_section_bytes, 2026-09-20 — see
    # IMPLEMENTATION.md §8.1.6). The garbage cannot be derived from silver's
    # model, so both normalizers drop the divergent fields symmetrically for
    # these records only; non-ds and pre-R2013 family records keep every
    # field (gold parses those sanely).
    _ver = ((data.get("FILEHEADER") or {}).get("version")) or ""
    _r2013_plus = _ver in ("AC1027", "AC1032")
    _PROLOGUE_DIVERGENT_FIELDS = frozenset({
        "acis_data", "history_id",  # garbage inline emissions (R2013 files)
        "point_present", "point", "isolines", "isoline_present",
        "acis_empty_bit",
        "has_revision_guid", "revision_major", "revision_minor1",
        "revision_minor2", "revision_bytes", "end_marker",
    })

    for obj in objects:
        typ = object_type(obj)
        # has_ds_data marks the AcDs-backed records where gold's decode
        # derailed (the entity-common bit R2013+).
        _drop_prologue = (
            _r2013_plus
            and typ in ("3DSOLID", "REGION")
            and bool(obj.get("has_ds_data"))
        )
        # Gold's unmodeled-class records (UNKNOWN_OBJ/UNKNOWN_ENT) carry a
        # raw `unknown_bits` hex dump of the bits its spec cannot decode.
        # Silver parses those same records into typed payloads, so the hex
        # is not derivable from silver's model (and silver's typed data has
        # no gold expression). normalize_silver projects those records to the
        # common-fields-only shape; drop the hex here symmetrically — the
        # passthrough write itself stays verified by the handle/common rows.
        _drop_unknown_bits = typ in ("UNKNOWN_OBJ", "UNKNOWN_ENT")
        # U2 (2026-09-20): gold's decode for two more record families
        # derails mid-record while silver's reader stays consistent — the
        # same irreducible-garbage class as the 3DSOLID prologue above:
        #  - RENDERENTRY (dwg2.spec 2566, ATMOS-DC22S): after the sane
        #    prefix (class_version/dimensions/filename/preset/view/date)
        #    gold reads zombie 256s for material/memory, the BD '01' special
        #    for render_time (1.0), swaps minute/second, misreads msec/
        #    display_index/light_count/triangle_count.
        #  - ACSH_BREP_CLASS (dwg2.spec 3031, ATMOS-DC22S): major reads
        #    3528495168 (float-bit garbage) and acis_data [""] while the
        #    history-node base stays consistent.
        _TYPE_DIVERGENT_FIELDS = {
            "RENDERENTRY": frozenset({
                "display_index", "light_count", "material_count",
                "memory_amount", "render_time", "start_minute",
                "start_msec", "start_second", "triangle_count",
            }),
            "ACSH_BREP_CLASS": frozenset({
                "major", "minor", "acis_data", "acis_empty",
                "acis_empty_bit", "unknown", "version",
                "wireframe_data_present",
                # rt side: silver's rewrite re-parses through the
                # COMMON_3DSOLID shape; the divergent tail stays
                "history_id", "point_present", "point", "isolines",
                "isoline_present",
            }),
        }
        fields: Dict[str, Any] = {}
        for k, v in obj.items():
            if k in ("entity", "object"):
                continue
            if is_ignored(k, ignore_set, ignore_patterns):
                continue
            if _drop_prologue and k in _PROLOGUE_DIVERGENT_FIELDS:
                continue
            if k in _TYPE_DIVERGENT_FIELDS.get(typ, ()):
                continue
            if _drop_unknown_bits and k == "unknown_bits":
                continue
            # LibreDWG prints a code-0 null handle as the bare 2-tuple
            # [0, 0] (absolute, no offset counter) while code-3 nulls print
            # as the full 4-tuple. In the corpus only `history_id`
            # (3DSOLID/REGION, dwg_spec_shared.h COMMON_3DSOLID) hits the
            # 2-tuple form as a handle; 2-tuple points elsewhere must stay
            # lists, so gate strictly on the field name. Canonicalize the
            # null to gold's absref-0 dict so it compares equal against
            # silver's null dict (its code is None — tolerated by the
            # differ). ATTRIB.style's matching [0, 0] form is untouched.
            # DICTIONARYWDFLT.defaultid hits the same 2-tuple when silver's
            # rewriter nulls the default (the whole class is list-vs-dict).
            if k in ("history_id", "defaultid") and v == [0, 0]:
                fields[k] = {"code": 0, "size": 0, "value": 0, "absref": 0}
                continue
            fields[k] = normalize_value(v)
        out.append({"type": typ, "fields": fields})
    return out


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: normalize_gold.py <gold.json> [ignore_fields.toml]", file=sys.stderr)
        sys.exit(1)
    ignore: Optional[Dict[str, Any]] = None
    if len(sys.argv) >= 3:
        ignore = load_ignore_fields(sys.argv[2])
    # Gold's dwgread prints raw modeler SAB slices as JSON strings that can
    # contain arbitrary binary bytes: control characters, non-UTF-8 bytes,
    # and even double quotes and backslashes, producing INVALID JSON. This
    # only surfaces once silver rewrites class-indirected 3DSOLID-family
    # records faithfully (classes-verbatim fix, 2026-09-20) — gold's own
    # decode of those wires derails (its 3DSOLID spec reads a phantom
    # leading acis_empty bit) and it dumps raw wire bytes. Sanitize the
    # first acis_data element by scanning the byte stream (a regex anchor
    # is ambiguous when the slice itself contains quote-comma-newline
    # sequences); `[""]`-style empty arrays are two adjacent quotes and are
    # left verbatim. Everything else is tolerated via strict=False on load.
    raw_bytes = _sanitize_gold_acis(open(sys.argv[1], "rb").read())
    raw = raw_bytes.decode("utf-8", errors="replace")
    # Gold prints invalid doubles as -nan (bit-double error code '11'),
    # which is not a JSON/python literal; make it parseable. Its rows then
    # compare as ordinary wrong_value diffs.
    raw = raw.replace("-nan", "NaN")
    data = json.loads(raw, strict=False)
    norm = normalize_gold(data, ignore)
    json.dump(norm, sys.stdout, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
