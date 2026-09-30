#!/usr/bin/env python3
"""Diff two normalized JSON object lists (gold vs silver).

Alignment is by (type, ordinal-within-type) because handles are reassigned on
rewrite. Handles compare by (handle code, resolved target type), not raw value:
gold emits handle tuples whose `absref` is the absolute handle (the `value`
slot is a relative counter for coded references) and whose `code` carries the
reference semantics (0 absolute/none, 4 soft owner, 8 hard owner, 12 hard
pointer). Silver stores only the resolved absolute handle, so its code is
None ("unknown") and imposes no constraint; when both sides carry a code the
codes must match exactly.
"""

import json
import math
import sys
from typing import Any, Dict, List, Optional, Tuple


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


def handle_value(value: Any) -> Any:
    """Extract the absolute handle value from a normalized handle dict.

    LibreDWG emits handle tuples as [code, size, value, absref]; for coded
    references (ownerhandle, reactors, ...) `value` is a relative counter and
    only `absref` is the absolute handle silver stores. Prefer `absref`.
    """
    if isinstance(value, dict):
        return value.get("absref", value.get("value"))
    return value


def build_handle_type_map(objects: List[Dict[str, Any]]) -> Dict[Any, str]:
    """Map absolute handle value -> object type for handle comparison."""
    m: Dict[Any, str] = {}
    for obj in objects:
        hv = handle_value(obj.get("fields", {}).get("handle"))
        if hv is not None:
            m[hv] = obj["type"]
    return m


def is_handle_dict(value: Any) -> bool:
    """A normalized handle tuple dict carries a code and an absolute ref."""
    return isinstance(value, dict) and "code" in value and (
        "absref" in value or "value" in value
    )


def resolve_handle(value: Any, handle_map: Dict[Any, str]) -> Any:
    """Replace handle values with a (code, resolved target type) token.

    The token keeps the handle code so `values_equal` can compare codes when
    both sides carry one (silver emits None: it stores no code). Non-handle
    dicts carry no comparable payload and collapse to None as before.
    """
    if isinstance(value, dict):
        if is_handle_dict(value):
            absref = handle_value(value)
            return {
                "__handle_code__": value.get("code"),
                "__handle_target__": handle_map.get(absref, absref),
            }
        return None
    if isinstance(value, list):
        return [resolve_handle(v, handle_map) for v in value]
    return value


def values_equal(a: Any, b: Any, rel_tol: float = 1e-6) -> bool:
    # Handle tokens compare by (code, resolved target). A token on only one
    # side means the handle resolved on one side but not the other (or one
    # side lacks the handle): a real mismatch, never equality. A side whose
    # code is None (silver stores no handle codes) constrains the target only.
    a_tok = isinstance(a, dict) and "__handle_target__" in a
    b_tok = isinstance(b, dict) and "__handle_target__" in b
    if a_tok or b_tok:
        if not (a_tok and b_tok):
            return False
        if not values_equal(a["__handle_target__"], b["__handle_target__"], rel_tol):
            return False
        code_a, code_b = a["__handle_code__"], b["__handle_code__"]
        if code_a is None or code_b is None:
            return True
        return code_a == code_b
    if type(a) is not type(b):
        # Allow int vs float near-equality.
        if isinstance(a, (int, float)) and isinstance(b, (int, float)):
            return math.isclose(float(a), float(b), rel_tol=rel_tol, abs_tol=1e-10)
        return False
    if isinstance(a, float):
        return math.isclose(a, b, rel_tol=rel_tol, abs_tol=1e-10)
    if isinstance(a, list):
        if len(a) != len(b):
            return False
        return all(values_equal(x, y, rel_tol) for x, y in zip(a, b))
    if isinstance(a, dict):
        if set(a.keys()) != set(b.keys()):
            return False
        return all(values_equal(a[k], b[k], rel_tol) for k in a)
    return a == b


def diff_fields(
    gold: List[Dict[str, Any]],
    silver: List[Dict[str, Any]],
    ignore: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    ignore_set: set = set()
    ignore_patterns: List[str] = []
    if ignore:
        ignore_set.update(ignore.get("global", {}).get("ignore", []))
        for pat in ignore.get("global", {}).get("ignore", []):
            if "*" in pat:
                ignore_patterns.append(pat)

    gold_handle_map = build_handle_type_map(gold)
    silver_handle_map = build_handle_type_map(silver)

    # Group by type, preserving order.
    def group(objs: List[Dict[str, Any]]) -> Dict[str, List[Dict[str, Any]]]:
        g: Dict[str, List[Dict[str, Any]]] = {}
        for o in objs:
            g.setdefault(o["type"], []).append(o)
        return g

    gold_by_type = group(gold)
    silver_by_type = group(silver)

    diffs: List[Dict[str, Any]] = []
    all_types = set(gold_by_type.keys()) | set(silver_by_type.keys())

    for typ in sorted(all_types):
        g_items = gold_by_type.get(typ, [])
        s_items = silver_by_type.get(typ, [])
        count = max(len(g_items), len(s_items))
        if len(g_items) != len(s_items):
            diffs.append(
                {
                    "type": typ,
                    "index": -1,
                    "kind": "count_mismatch",
                    "gold_count": len(g_items),
                    "silver_count": len(s_items),
                }
            )
        for i in range(count):
            g = g_items[i] if i < len(g_items) else None
            s = s_items[i] if i < len(s_items) else None
            if g is None or s is None:
                diffs.append(
                    {
                        "type": typ,
                        "index": i,
                        "kind": "missing",
                        "side": "silver" if g is None else "gold",
                    }
                )
                continue

            g_fields = g["fields"]
            s_fields = s["fields"]

            for k in g_fields:
                if is_ignored(k, ignore_set, ignore_patterns):
                    continue
                if k not in s_fields:
                    diffs.append(
                        {
                            "type": typ,
                            "index": i,
                            "kind": "missing_in_silver",
                            "field": k,
                            "gold_value": g_fields[k],
                        }
                    )
                    continue
                gv = resolve_handle(g_fields[k], gold_handle_map)
                sv = resolve_handle(s_fields[k], silver_handle_map)
                if not values_equal(gv, sv):
                    diffs.append(
                        {
                            "type": typ,
                            "index": i,
                            "kind": "wrong_value",
                            "field": k,
                            "gold_value": gv,
                            "silver_value": sv,
                        }
                    )

            for k in s_fields:
                if is_ignored(k, ignore_set, ignore_patterns):
                    continue
                if k not in g_fields:
                    diffs.append(
                        {
                            "type": typ,
                            "index": i,
                            "kind": "extra_in_silver",
                            "field": k,
                            "silver_value": s_fields[k],
                        }
                    )

    return {
        "total_diffs": len(diffs),
        "diffs": diffs,
    }


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: diff_fields.py <gold_norm.json> <silver_norm.json> [ignore_fields.toml]", file=sys.stderr)
        sys.exit(1)
    ignore: Optional[Dict[str, Any]] = None
    if len(sys.argv) >= 4:
        ignore = load_ignore_fields(sys.argv[3])
    with open(sys.argv[1], "r", encoding="utf-8") as f:
        gold = json.load(f)
    with open(sys.argv[2], "r", encoding="utf-8") as f:
        silver = json.load(f)
    result = diff_fields(gold, silver, ignore)
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
