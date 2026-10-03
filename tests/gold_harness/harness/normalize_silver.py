#!/usr/bin/env python3
"""Normalize acadrust `dwg2json` silver output to the same canonical shape as gold.

The silver JSON is a flattened `CadDocument` with a `_common_dwg` map keyed by
entity handle that re-includes the serde-skipped `EntityCommon` fields. This
script:
  - Drops all top-level tables/header/classes/notifications after using them.
  - Emits every entity in `entities`, every object in `objects`, and every table
    record (layers, linetypes, text styles, block records, dim styles, app ids,
    views, vports, UCSs).
  - Resolves entity layer names to layer handles via the silver `layers` table.
  - Maps `EntityType` variant names and per-variant field names to gold
    vocabulary.
  - Flattens `Vector3`/`Vector2` objects into arrays and handle ints into gold
    style handle dicts.
"""

import json
import re
import sys
from typing import Any, Dict, List, Optional

# Silver EntityType variant name -> gold type name.
ENTITY_TYPE_MAP = {
    "Line": "LINE",
    "Circle": "CIRCLE",
    "Arc": "ARC",
    "Ellipse": "ELLIPSE",
    "Point": "POINT",
    "Text": "TEXT",
    "MText": "MTEXT",
    "LwPolyline": "LWPOLYLINE",
    "Polyline2D": "POLYLINE_2D",
    "Polyline3D": "POLYLINE_3D",
    "Spline": "SPLINE",
    "Hatch": "HATCH",
    "Insert": "INSERT",
    "Block": "BLOCK",
    "BlockEnd": "ENDBLK",
    "Seqend": "SEQEND",
    "Ray": "RAY",
    "XLine": "XLINE",
    "Leader": "LEADER",
    "MLine": "MLINE",
    "Helix": "HELIX",
    "Solid": "SOLID",
    "Face3D": "3DFACE",
    "Viewport": "VIEWPORT",
    "Tolerance": "TOLERANCE",
    "AttributeDefinition": "ATTDEF",
    "AttributeEntity": "ATTRIB",
    "Solid3D": "3DSOLID",
    "Region": "REGION",
    "Body": "BODY",
    "Surface": "SURFACE",
    # gold's TABLE entity block (dwg.spec 477) is inside the debug-gated
    # region (dwg2.spec 297-959, §8.1.1 liveness rule): gold decodes the
    # class instances as raw UNKNOWN_ENT, so the resolved handle-target
    # type name must be UNKNOWN_ENT — not TABLE (BLOCK_HEADER.entities /
    # BLOCK_HEADER.inserts rows resolved to the wrong target type name).
    "Table": "UNKNOWN_ENT",
    "MultiLeader": "MULTILEADER",
    "RasterImage": "IMAGE",
    "Wipeout": "WIPEOUT",
    "Underlay": "UNDERLAY",
    "Ole2Frame": "OLE2FRAME",
    # gold's spec name for the polyface mesh entity is POLYLINE_PFACE
    # (dwg.spec), not POLYFACE_MESH.
    "PolyfaceMesh": "POLYLINE_PFACE",
    # gold's spec name for the polygon-mesh entity is POLYLINE_MESH
    "PolygonMesh": "POLYLINE_MESH",
    "Mesh": "MESH",
    "Light": "LIGHT",
    "Shape": "SHAPE",
    "Extended": "UNKNOWN_ENT",
    "Unknown": "UNKNOWN_ENT",
}

# Silver object variant name -> gold type name. Most objects carry the gold
# name already, but a few differ.
OBJECT_TYPE_MAP: Dict[str, str] = {
    "Dictionary": "DICTIONARY",
    "Layout": "LAYOUT",
    "XRecord": "XRECORD",
    "Group": "GROUP",
    "MLineStyle": "MLINESTYLE",
    "ImageDefinition": "IMAGEDEF",
    "UnderlayDefinition": "PDFDEF",
    "PlotSettings": "PLOTSETTINGS",
    "MultiLeaderStyle": "MLEADERSTYLE",
    "TableStyle": "TABLESTYLE",
    # gold's TABLECONTENT block (dwg2.spec 466) sits inside the same
    # debug-gated region (dwg2.spec 297-959, §8.1.1 liveness rule) as the
    # TABLE entity: the built dwgread decodes the record raw as UNKNOWN_OBJ
    # with only the common fields on its JSON. Handled silver TableContent
    # records ARE the gold UNKNOWN_OBJ carriers (one per example_* file);
    # the object loop's handleless-wrapper guard skips silver's phantoms.
    "TableContent": "UNKNOWN_OBJ",
    "Scale": "SCALE",
    "ObjectContextData": "OBJECTCONTEXTDATA",
    "SortEntitiesTable": "SORTENTSTABLE",
    "DictionaryVariable": "DICTIONARYVAR",
    "VisualStyle": "VISUALSTYLE",
    "Material": "MATERIAL",
    "ImageDefinitionReactor": "IMAGEDEF_REACTOR",
    "GeoData": "GEODATA",
    "SpatialFilter": "SPATIAL_FILTER",
    "RasterVariables": "RASTERVARIABLES",
    "BookColor": "DBCOLOR",
    "PlaceHolder": "PLACEHOLDER",
    "DictionaryWithDefault": "DICTIONARYWDFLT",
    "WipeoutVariables": "WIPEOUTVARIABLES",
    "BlockVisibilityParameter": "BLOCKVISIBILITYPARAMETER",
    "DynamicBlock": "UNKNOWN_OBJ",
    # "Associative" is NOT mapped to a single gold type: the variant wraps many
    # ASSOC*/DIMASSOC/PERSUBENTMGR classes distinguished by payload.dxf_name.
    # The object loop resolves the gold type from dxf_name (see below). Mapping
    # the whole bucket to UNKNOWN_OBJ would discard every parsed DIMASSOC.
    "ClassObject": "UNKNOWN_OBJ",
    "DataObject": "UNKNOWN_OBJ",
    "Field": "FIELD",
    "FieldList": "FIELDLIST",
    "RegisteredClass": "UNKNOWN_OBJ",
    "DgnLineStyle": "UNKNOWN_OBJ",
    "ProxyObject": "UNKNOWN_OBJ",
    "Unknown": "UNKNOWN_OBJ",
}


def _associative_gold_name(dxf_name: str) -> str:
    """Map silver's stored class-table dxf_name to gold's canonical spec name.

    Silver keeps the DWG class-table DXF name (e.g. ACDBASSOCACTION,
    ACDBPERSSUBENTMANAGER, ACDBDIMASSOC); gold (libredwg) registers and emits
    these objects under their canonical spec names (ASSOCACTION, PERSUBENTMGR,
    DIMASSOC). Mirror the silver reader's `associative_canonical_name`.
    """
    upper = dxf_name.upper()
    if upper.startswith("ACDBASSOC"):
        upper = "ASSOC" + upper[len("ACDBASSOC"):]
    # Aliases seen in class tables without the doubled letter.
    if upper == "ASSOCALIGNEDIMACTIONBODY":
        return "ASSOCALIGNEDDIMACTIONBODY"
    if upper == "ACDBPERSSUBENTMANAGER":
        return "PERSUBENTMGR"
    if upper == "ACDBDIMASSOC":
        return "DIMASSOC"
    return upper

# Per-silver-type field-name overrides to match gold vocabulary.
FIELD_NAME_MAP: Dict[str, Dict[str, str]] = {
    "Line": {"normal": "extrusion", "start": "start", "end": "end", "thickness": "thickness"},
    "Circle": {"normal": "extrusion", "center": "center", "radius": "radius", "thickness": "thickness"},
    "Arc": {"normal": "extrusion", "center": "center", "radius": "radius", "start_angle": "start_angle", "end_angle": "end_angle", "thickness": "thickness"},
    "Ellipse": {"normal": "extrusion", "center": "center", "major_axis": "sm_axis", "minor_axis_ratio": "axis_ratio", "start_parameter": "start_angle", "end_parameter": "end_angle"},
    "Point": {"location": "point"},
    "Text": {"normal": "extrusion", "insertion_point": "insertion_pt", "alignment_point": "alignment_pt", "value": "text_value"},
    "MText": {"normal": "extrusion", "insertion_point": "ins_pt", "value": "text"},
    "LwPolyline": {"normal": "extrusion"},
    "Polyline2D": {"normal": "extrusion"},
    "Polyline3D": {"normal": "extrusion"},
    "Spline": {"normal": "extrusion"},
    "Hatch": {"normal": "extrusion"},
    "Insert": {"normal": "extrusion"},
    "Block": {"base_point": "base_pt", "name": "name"},
    "BlockEnd": {},
    "Seqend": {},
    "Ray": {"normal": "extrusion", "base_point": "point", "direction": "vector"},
    "XLine": {"normal": "extrusion", "base_point": "point", "direction": "vector"},
    "RasterVariables": {"display_image_frame": "image_frame"},
    "Leader": {"normal": "extrusion"},
    "MLine": {"normal": "extrusion"},
    "Helix": {"normal": "extrusion"},
    "Solid": {"normal": "extrusion"},
    "Face3D": {"normal": "extrusion", "first_corner": "corner1", "second_corner": "corner2",
               "third_corner": "corner3", "fourth_corner": "corner4"},
    "Viewport": {"normal": "extrusion"},
    "Tolerance": {"normal": "extrusion"},
    "AttributeDefinition": {"normal": "extrusion"},
    "AttributeEntity": {"normal": "extrusion"},
    "Solid3D": {"normal": "extrusion"},
    "Region": {"normal": "extrusion"},
    "Body": {"normal": "extrusion"},
    "Surface": {"normal": "extrusion"},
    "Table": {"normal": "extrusion"},
    "MultiLeader": {"normal": "extrusion"},
    "RasterImage": {"normal": "extrusion"},
    "Wipeout": {"normal": "extrusion"},
    "Underlay": {"normal": "extrusion"},
    "Ole2Frame": {"normal": "extrusion"},
    "PolyfaceMesh": {"normal": "extrusion"},
    "PolygonMesh": {"normal": "extrusion"},
    "Mesh": {"normal": "extrusion"},
    "Light": {"normal": "extrusion"},
    "Shape": {"normal": "extrusion"},
}

# Logical EntityCommon fields emitted by serde already. They stay under their own
# names and are not re-injected from `_common_dwg`.
COMMON_LOGICAL = {
    "handle",
    "layer",
    "color",
    "line_weight",
    "linetype",
    "linetype_scale",
    "transparency",
    "color_name",
    "invisible",
    "extended_data",
    "reactors",
    "xdictionary_handle",
    "owner_handle",
    "full_visual_style_handle",
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


def normalize_float(value: Any) -> Any:
    if isinstance(value, float):
        if value != value:
            return 0.0
        if value == 0.0:
            return 0.0
        return round(value, 14)
    return value


def flatten_vector(value: Any) -> Any:
    if isinstance(value, dict):
        keys = set(value.keys())
        if keys == {"x", "y", "z"}:
            return [normalize_float(value["x"]), normalize_float(value["y"]), normalize_float(value["z"])]
        if keys == {"x", "y"}:
            return [normalize_float(value["x"]), normalize_float(value["y"])]
        return {k: flatten_vector(v) for k, v in value.items()}
    if isinstance(value, list):
        return [flatten_vector(v) for v in value]
    return normalize_float(value)


def normalize_value(value: Any) -> Any:
    value = flatten_vector(value)
    if isinstance(value, list):
        return [normalize_value(v) for v in value]
    if isinstance(value, dict):
        return {k: normalize_value(v) for k, v in value.items()}
    return value


def normalize_handle_value(value: Any) -> Any:
    """Turn a silver handle int/None into a gold-style handle dict.

    Silver stores only the resolved absolute handle; the DWG handle code
    (0 absolute/none, 4 soft owner, 8 hard owner, 12 hard pointer) is not
    represented in the object model. Emit ``code=None`` ("unknown") so the
    differ compares the resolved target and only checks the code when both
    sides carry one. A fabricated code (e.g. 0) would falsely mismatch every
    gold ownerhandle code (4/8/12).
    """
    if value is None:
        return None
    if isinstance(value, bool):
        return value
    if isinstance(value, int):
        return {"code": None, "size": 0, "value": value, "absref": value}
    if isinstance(value, dict):
        return value
    return value


# libredwg lweights[] table: gold's raw linewt byte (COMMON_ENTITY_DATA's
# FIELD_RC linewt 370, and the LAYER table flag's 5-bit index) indexes this
# (0..23), with 29=ByLayer, 30=ByBlock, 31=Default. Mirrors silver's
# LineWeight::INDEXED_VALUES (src/types/line_weight.rs) and to_dwg_index.
_LWEIGHT_INDEXED = [0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53,
                    60, 70, 80, 90, 100, 106, 120, 140, 158, 200, 211]


def _lweight_index(mm100: int) -> Any:
    """Map a mm*100 weight value back to gold's raw table index. Echo the
    raw code for out-of-table values (gold prints the wire byte verbatim
    for invalid codes like Dynblocks R2018's 28)."""
    try:
        return _LWEIGHT_INDEXED.index(mm100)
    except ValueError:
        return mm100


def _lineweight_to_gold(value: Any) -> Any:
    """Map silver's LineWeight enum to gold's raw linewt byte.

    Gold (common_entity_data.spec FIELD_RC linewt, 370) stores the *index* into
    libredwg's lweights[] table: 0..23 = mm*100, 29 (0x1D) = ByLayer,
    30 (0x1E) = ByBlock, 31 (0x1F) = ByLwDefault. Silver serializes the enum
    as a string ("ByLayer"/"ByBlock"/"Default") for the named variants and as
    {"Value": <mm*100>} for concrete weights — invert both to the index.
    """
    if isinstance(value, bool):
        return value
    if isinstance(value, dict) and "Value" in value:
        v = value["Value"]
        if isinstance(v, bool) or not isinstance(v, (int, float)):
            return value
        return _lweight_index(int(v))
    if isinstance(value, int):
        # mm*100 value (LayerData stores the enum's as_i16)
        return _lweight_index(value)
    if isinstance(value, str):
        if value == "ByLayer":
            return 29
        if value == "ByBlock":
            return 30
        if value == "Default":
            return 31
        # "W0_05" style mm*100 names
        if value.startswith("W") and "_" in value:
            try:
                return _lweight_index(int(value[1:].replace("_", "")))
            except ValueError:
                return value
    return value


def normalize_color(value: Any) -> Any:
    if isinstance(value, dict):
        if "Index" in value:
            return value["Index"]
        if "RGB" in value and value["RGB"] is not None:
            return value
        if "ByLayer" in value:
            return 256
        if "ByBlock" in value:
            return 0
        if "Foreground" in value:
            return 257
    if value == "ByLayer":
        return 256
    if value == "ByBlock":
        return 0
    if value in (None, "None"):
        # Absent color (gold's "none"/index 257). Silver's Color::None
        # serializes as the bare string "None".
        return 257
    return value


def _resolve_layer_name(name: str, layer_map: Dict[str, int]) -> int:
    if name in layer_map:
        return layer_map[name]
    # Fallback: layer 0 handle is conventionally 16 in many test files, but
    # we prefer the map.
    return layer_map.get("0", 0)


def merge_common(
    common: Dict[str, Any],
    common_dwg_entry: Optional[Dict[str, Any]],
    layer_map: Dict[str, int],
    r13_14: bool = False,
) -> Dict[str, Any]:
    fields: Dict[str, Any] = {}

    # Gold emits `ownerhandle` for an entity only when entmode == 0 (e.g.
    # polyline sub-entities); model/paper-space entities have no ownerhandle
    # in gold JSON. entity_mode lives in the `_common_dwg` entry, so resolve
    # it before projecting `owner_handle`.
    entity_mode = None
    if common_dwg_entry:
        entity_mode = common_dwg_entry.get("entity_mode")

    for k, v in common.items():
        if k == "handle":
            fields["handle"] = normalize_handle_value(v)
        elif k == "owner_handle":
            if entity_mode == 0:
                fields["ownerhandle"] = normalize_handle_value(v)
        elif k == "layer":
            fields["layer"] = normalize_handle_value(_resolve_layer_name(str(v), layer_map))
        elif k == "color":
            fields["color"] = normalize_color(v)
        elif k == "line_weight":
            # R13/R14 (gold common_entity_data.spec 377+): no linewt on the
            # wire — gold's JSON has no linewt field there; silver's model
            # carries the constructed default. Emit only R2000+.
            if not r13_14:
                fields["linewt"] = _lineweight_to_gold(v)
        elif k == "linetype":
            # Silver stores the resolved linetype NAME; gold has no name field
            # (only the `ltype` handle when ltype_flags == 3). Drop it.
            pass
        elif k == "linetype_scale":
            fields["ltype_scale"] = normalize_float(v)
        elif k == "invisible":
            fields["invisible"] = 1 if v else 0
        elif k == "transparency":
            pass
        elif k == "color_name":
            # Gold only emits color names when the color carries one.
            if v is not None:
                fields["color_name"] = v
        elif k == "extended_data":
            pass
        elif k == "reactors":
            if v:
                fields["reactors"] = [normalize_handle_value(h) for h in v]
        elif k == "xdictionary_handle":
            if v is not None:
                fields["xdicobjhandle"] = normalize_handle_value(v)
        elif k == "full_visual_style_handle":
            if v is not None:
                fields["full_visualstyle"] = normalize_handle_value(v)
        else:
            fields[k] = normalize_value(v)

    if common_dwg_entry:
        for k, v in common_dwg_entry.items():
            if k in COMMON_LOGICAL or k == "handle":
                continue
            if v is None:
                continue
            if k == "graphic_data" and v is None:
                continue
            # Map silver storage field names to gold vocabulary.
            if k == "linetype_handle":
                fields["ltype"] = normalize_handle_value(v)
                continue
            if k == "plotstyle_handle":
                fields["plotstyle"] = normalize_handle_value(v)
                continue
            if k == "material_handle":
                fields["material"] = normalize_handle_value(v)
                continue
            if k == "face_visual_style_handle":
                fields["face_visualstyle"] = normalize_handle_value(v)
                continue
            if k == "edge_visual_style_handle":
                fields["edge_visualstyle"] = normalize_handle_value(v)
                continue
            if k == "entity_mode":
                # Gold's name for the raw entity-mode value. `entmode` is in
                # ignore_fields.toml, so this is ignored on both sides.
                fields["entmode"] = v
                continue
            if k == "linetype_flags":
                # R13/R14 (gold common_entity_data.spec 377-380, VERSIONS
                # R_13b1-R_14): the wire carries the single isbylayerlt bit;
                # gold's JSON emits the named bit and the DECODER derives
                # ltype_flags from it (isbylayerlt -> 0 else 3). Project the
                # named bit; the R2000+ 2-bit flags emit unchanged.
                if r13_14:
                    fields["isbylayerlt"] = 0 if v == 3 else 1
                else:
                    fields["ltype_flags"] = v
                continue
            if k == "prev_entity_handle" and v is not None:
                fields["prev_entity"] = normalize_handle_value(v)
                continue
            if k == "next_entity_handle" and v is not None:
                fields["next_entity"] = normalize_handle_value(v)
                continue
            if k == "nolinks" and v is not None:
                fields["nolinks"] = 1 if v else 0
                continue
            if k == "z_are_zero" and v is not None:
                fields["z_is_zero"] = 1 if v else 0
                continue
            if k == "plotstyle_flags" and r13_14:
                # R13/R14 (gold common_entity_data.spec: the plotstyle_flags
                # BB is SINCE R_2000b): not on the wire; silver's reader
                # default (0) never compares.
                continue
            fields[k] = normalize_value(v)
    return fields


def _object_common_fields(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Extract common object fields that many silver objects share."""
    fields: Dict[str, Any] = {}
    for k in ("handle", "owner", "owner_handle"):
        if k in payload:
            v = payload[k]
            if k == "handle":
                fields["handle"] = normalize_handle_value(v)
            else:
                fields["ownerhandle"] = normalize_handle_value(v)
    if "reactors" in payload and payload["reactors"]:
        fields["reactors"] = [normalize_handle_value(h) for h in payload["reactors"]]
    if "xdictionary_handle" in payload and payload["xdictionary_handle"] is not None:
        fields["xdicobjhandle"] = normalize_handle_value(payload["xdictionary_handle"])
    return fields


# ── Raw-remainder (unknown_bits) emitter types ────────────────────────────
# Gold emits `unknown_bits` exactly for the spec blocks carrying
# HANDLE_UNKNOWN_BITS (149 blocks in dwg.spec/dwg2.spec, extracted via
# target/probes/ub_spec_set.py). This is the subset observed with a
# non-zero remainder on the corpus (target/probes/ub_full_set.py) minus
# UNKNOWN_OBJ/UNKNOWN_ENT (their hex is dropped symmetrically in
# normalize_gold). A type outside this set must NOT get an emitted
# unknown_bits or the differ would show extra_in_silver rows.
_UNKNOWN_BITS_TYPES = frozenset({
    "DIMASSOC", "EVALUATION_GRAPH", "ACSH_FILLET_CLASS", "TABLESTYLE",
    "ASSOCDEPENDENCY", "ASSOCVARIABLE", "ASSOCVALUEDEPENDENCY",
    "ASSOCDIMDEPENDENCYBODY", "WIPEOUT", "BLOCKSTRETCHACTION",
    "ASSOCPATHACTIONPARAM", "MULTILEADER", "BLOCKREPRESENTATION",
    "ASSOCOSNAPPOINTREFACTIONPARAM", "ASSOCVERTEXACTIONPARAM",
    "ARC_DIMENSION", "TABLEGEOMETRY", "DYNAMICBLOCKPURGEPREVENTER",
    "ASSOC2DCONSTRAINTGROUP", "SECTION_SETTINGS", "HELIX",
    "ASSOCEXTRUDEDSURFACEACTIONBODY", "ASSOCLOFTEDSURFACEACTIONBODY",
    "ASSOCREVOLVEDSURFACEACTIONBODY", "PLANESURFACE",
    "ASSOCPLANESURFACEACTIONBODY", "LEADEROBJECTCONTEXTDATA",
})


# ── Dynamic-block family retype map (U2, 2026-09-20) ──────────────────────

# dxf_name (upper-cased; silver keeps the class-table dxfname on its
# DynamicBlock wrapper) -> gold object name. Every class here has a LIVE
# gold spec block (dwg2.spec; verified against the §8.1.1 frame map) and a
# landed field projection in the object loop below.
_DYNBLOCK_RETYPE = {
    "BLOCKGRIPLOCATIONCOMPONENT": "BLOCKGRIPLOCATIONCOMPONENT",
    "BLOCKSTRETCHACTION": "BLOCKSTRETCHACTION",
    "BLOCKREPRESENTATION": "BLOCKREPRESENTATION",
    "ACDB_BLOCKREPRESENTATION_DATA": "BLOCKREPRESENTATION",
    "DYNAMICBLOCKPURGEPREVENTER": "DYNAMICBLOCKPURGEPREVENTER",
    "ACDB_DYNAMICBLOCKPURGEPREVENTER_VERSION": "DYNAMICBLOCKPURGEPREVENTER",
    "BLOCKVISIBILITYGRIP": "BLOCKVISIBILITYGRIP",
    "BLOCKALIGNMENTGRIP": "BLOCKALIGNMENTGRIP",
    "BLOCKFLIPGRIP": "BLOCKFLIPGRIP",
    "BLOCKLINEARGRIP": "BLOCKLINEARGRIP",
    "BLOCKROTATIONGRIP": "BLOCKROTATIONGRIP",
    "BLOCKALIGNMENTPARAMETER": "BLOCKALIGNMENTPARAMETER",
    "BLOCKLINEARPARAMETER": "BLOCKLINEARPARAMETER",
    "BLOCKBASEPOINTPARAMETER": "BLOCKBASEPOINTPARAMETER",
    "BLOCKFLIPPARAMETER": "BLOCKFLIPPARAMETER",
    "BLOCKROTATIONPARAMETER": "BLOCKROTATIONPARAMETER",
    "BLOCKMOVEACTION": "BLOCKMOVEACTION",
    "BLOCKFLIPACTION": "BLOCKFLIPACTION",
    "BLOCKROTATEACTION": "BLOCKROTATEACTION",
    "BLOCKSCALEACTION": "BLOCKSCALEACTION",
    "ACSH_FILLET_CLASS": "ACSH_FILLET_CLASS",
    "ACSH_CYLINDER_CLASS": "ACSH_CYLINDER_CLASS",
    # dwg2.spec 2978 (ACSH_CONE_CLASS): same wire fields as the cylinder
    # (height BD, major_radius BD, minor_radius BD, x_radius BD); silver
    # parses them under base_x_radius/base_y_radius/top_radius (2000/Cone
    # values 5.0/5.0/0.0 map 1:1 onto gold's 5.0/5.0/0.0).
    "ACSH_CONE_CLASS": "ACSH_CONE_CLASS",
    "ACSH_WEDGE_CLASS": "ACSH_WEDGE_CLASS",
    "ACSH_BOX_CLASS": "ACSH_BOX_CLASS",
    "ACSH_CHAMFER_CLASS": "ACSH_CHAMFER_CLASS",
    "ACSH_BOOLEAN_CLASS": "ACSH_BOOLEAN_CLASS",
    "ACSH_TORUS_CLASS": "ACSH_TORUS_CLASS",
    # dwg2.spec 2951 (ACSH_SPHERE_CLASS, live in gold): the shared node
    # skeleton plus a single BD radius (the SPHERE_CLASSES macro: major,
    # minor, radius). Phase A step-5 packet — until this landed, silver's
    # sphere records paired as UNKNOWN_OBJ and produced the sphere-family
    # count/missing rows on every Sphere fixture.
    "ACSH_SPHERE_CLASS": "ACSH_SPHERE_CLASS",
    # dwg2.spec (ACSH_PYRAMID_CLASS, live in gold): height BD, sides BL,
    # radius BD, topradius BD (gold's JSON key is "topradius"). The same
    # retype gap the sphere had — found at the Phase C fixture review:
    # silver's pyramid records paired as UNKNOWN_OBJ on every Pyramid
    # fixture.
    "ACSH_PYRAMID_CLASS": "ACSH_PYRAMID_CLASS",
    "ACSH_BREP_CLASS": "ACSH_BREP_CLASS",
}


def _dyn_eval_fields(ev, fields):
    """AcDbEvalExpr_fields (dwg2.spec 2860) -> evalexpr.* JSON fields."""
    if not isinstance(ev, dict):
        return
    fields["evalexpr.parentid"] = ev.get("parent_id", 0)
    fields["evalexpr.major"] = ev.get("major", 0)
    fields["evalexpr.minor"] = ev.get("minor", 0)
    fields["evalexpr.value_code"] = ev.get("value_code", 0)
    # BlockEvalValue is externally tagged; gold emits evalexpr.value.<key>
    # for the variant matching value_code (spec switch).
    v = ev.get("value")
    if isinstance(v, dict) and len(v) == 1:
        vkind, vval = next(iter(v.items()))
        _spec = {"Real": ("num40", normalize_float),
                 "Point": ("pt2d", normalize_value),
                 "Text": ("text1", None),
                 "Long": ("long90", None),
                 "Handle": ("handle91", normalize_handle_value),
                 "Short": ("short70", None)}.get(vkind)
        if _spec:
            gname, conv = _spec
            if vkind == "Point" and ev.get("value_code") == 11:
                gname = "pt3d"
            fields["evalexpr.value." + gname] = conv(vval) if conv else vval
    fields["evalexpr.nodeid"] = ev.get("node_id", 0)


def _dyn_element_fields(el, fields):
    """AcDbBlockElement_fields (dwg2.spec 3208): name + eed1071. The
    be_major/be_minor pair is DECODER-only (else-branch constants 33/29) —
    silver's element.major/minor have no gold counterpart."""
    if not isinstance(el, dict):
        return
    _dyn_eval_fields(el.get("eval"), fields)
    fields["name"] = el.get("name", "")
    fields["eed1071"] = el.get("eed_1071", 0)


def _dyn_grip_fields(g, fields):
    """AcDbBlockGrip_fields (dwg2.spec 3226)."""
    if not isinstance(g, dict):
        return
    _dyn_element_fields(g.get("element"), fields)
    fields["bg_bl91"] = g.get("flags_91", 0)
    fields["bg_bl92"] = g.get("flags_92", 0)
    fields["bg_location"] = normalize_value(g.get("location"))
    fields["bg_insert_cycling"] = 1 if g.get("insert_cycling") else 0
    fields["bg_insert_cycling_weight"] = g.get("insert_cycling_weight", 0)


def _dyn_parameter_fields(p, fields):
    """AcDbBlockParameter_fields (dwg2.spec 3235)."""
    if not isinstance(p, dict):
        return
    _dyn_element_fields(p.get("element"), fields)
    fields["show_properties"] = 1 if p.get("show_properties") else 0
    fields["chain_actions"] = 1 if p.get("chain_actions") else 0


def _dyn_propinfo(prefix, props, fields):
    """BlockParam_PropInfo: gold emits <prefix><n>.connections only when the
    REPEAT is non-empty; normalize_gold collapses the per-connection dicts
    to 0s (the no-index/no-rgb dict heuristic)."""
    if not isinstance(props, list):
        return
    for i, pr in enumerate(props):
        conns = pr.get("connections") if isinstance(pr, dict) else None
        if isinstance(conns, list) and conns:
            fields[f"{prefix}{i + 1}.connections"] = [0] * len(conns)


def _dyn_1pt_fields(p, fields):
    """AcDbBlock1PtParameter_fields (dwg2.spec 3308). `p` is silver's
    BlockOnePointParameter (the `parameter` member of the wrapping class).
    Gold does not emit num_propinfos (empirically absent on every corpus
    BLOCKVISIBILITYPARAMETER and BLOCKBASEPOINTPARAMETER)."""
    if not isinstance(p, dict):
        return
    _dyn_parameter_fields(p.get("parameter"), fields)
    fields["def_pt"] = normalize_value(p.get("definition_point"))
    _dyn_propinfo("prop", p.get("properties"), fields)


def _dyn_2pt_fields(p, fields):
    """AcDbBlock2PtParameter_fields (dwg2.spec 3317). `p` is silver's
    BlockTwoPointParameter (the `parameter` member of the wrapping class —
    the payload nests parameter.parameter.element)."""
    if not isinstance(p, dict):
        return
    _dyn_parameter_fields(p.get("parameter"), fields)
    fields["def_basept"] = normalize_value(p.get("definition_base_point"))
    fields["def_endpt"] = normalize_value(p.get("definition_end_point"))
    _dyn_propinfo("prop", p.get("properties"), fields)
    # FIELD_VECTOR_N (prop_states, BL, 4): gold's raw 4-int array takes
    # normalize_gold's int-array->handle shape
    # {code:s0, size:s1, value:s2, absref:s3}.
    st = p.get("property_states")
    if isinstance(st, list) and len(st) >= 3:
        fields["prop_states"] = {"code": st[0], "size": st[1],
                                 "value": st[2],
                                 "absref": st[3] if len(st) > 3 else st[2]}
    fields["parameter_base_location"] = p.get("parameter_base_location", 0)


def _dyn_valueset_fields(vs, fields):
    """AcDbBlockParamValueSet_fields (dwg2.spec 3289) — the SUB_FIELD names
    are emitted plainly (verified on BLOCKLINEARPARAMETER/BLOCKROTATIONPARAMETER
    census). num_valuelist follows the num_* absence rule."""
    if not isinstance(vs, dict):
        return
    fields["desc"] = vs.get("description", "")
    fields["flags"] = vs.get("flags", 0)
    fields["minimum"] = normalize_float(vs.get("minimum", 0.0))
    fields["maximum"] = normalize_float(vs.get("maximum", 0.0))
    fields["increment"] = normalize_float(vs.get("increment", 0.0))
    vals = vs.get("values")
    if isinstance(vals, list):
        # gold emits the valuelist array even when empty (verified on
        # BLOCKROTATIONPARAMETER: flags 3, valuelist [])
        fields["valuelist"] = [normalize_float(v) for v in vals]


def _dyn_action_fields(a, fields):
    """AcDbBlockAction_fields (dwg2.spec 3241)."""
    if not isinstance(a, dict):
        return
    _dyn_element_fields(a.get("element"), fields)
    fields["display_location"] = normalize_value(a.get("display_location"))
    fields["deps"] = [normalize_handle_value(d)
                      for d in (a.get("dependencies") or [])]
    fields["actions"] = list(a.get("action_ids") or [])


def _dyn_conn_last(conns, fields):
    """BlockAction_ConnectionPt(s): out_json keys every connection's
    code/name plainly, so duplicate keys collapse last-wins in the parsed
    JSON object — the surviving name/code belong to the LAST connection.
    When the class carries no connections the element's name survives."""
    if isinstance(conns, list) and conns:
        last = conns[-1]
        if isinstance(last, dict):
            fields["code"] = last.get("code", 0)
            fields["name"] = last.get("name", "")


def _dyn_offsets_fields(off, fields):
    """AcDbBlockAction_doubles_fields (dwg2.spec 3351)."""
    if not isinstance(off, dict):
        return
    fields["action_offset_x"] = normalize_float(off.get("offset_x", 0.0))
    fields["action_offset_y"] = normalize_float(off.get("offset_y", 0.0))
    fields["angle_offset"] = normalize_float(off.get("angle_offset", 0.0))


def _acsh_node_fields(base, fields):
    """SolidHistoryNodeBase -> AcDbShHistoryNode_fields (dwg2.spec 2896)."""
    if not isinstance(base, dict):
        return
    _dyn_eval_fields(base.get("eval"), fields)
    fields["history_node.major"] = base.get("major", 0)
    fields["history_node.minor"] = base.get("minor", 0)
    fields["history_node.step_id"] = base.get("step_id", 0)
    fields["history_node.color"] = normalize_color(base.get("color"))
    mat = base.get("material")
    fields["history_node.material"] = normalize_handle_value(
        mat if isinstance(mat, int) else 0)
    trans = base.get("transform")
    if isinstance(trans, list):
        fields["history_node.trans"] = [normalize_float(t) for t in trans]


def _unwrap_dyn_data(payload):
    """DynamicBlock wrapper -> (kind, data-dict); SolidHistoryNode shapes
    are double-wrapped (SolidHistoryNode -> <Shape>)."""
    data = payload.get("data") if isinstance(payload.get("data"), dict) else {}
    if len(data) != 1:
        return None, {}
    kind, inner = next(iter(data.items()))
    if kind == "SolidHistoryNode" and isinstance(inner, dict) and len(inner) == 1:
        shape, shp = next(iter(inner.items()))
        return shape, shp if isinstance(shp, dict) else {}
    return kind, inner if isinstance(inner, dict) else {}


# ── VisualStyle property-bag mapping ──────────────────────────────────────
# Silver stores VisualStyle properties as a positional bag of
# {value: {Double|Long|Short|Bool|Color|Text: …}, enabled: int}. Gold names
# every field. The bag order is fixed per version range and matches the read
# order in object_reader/objects.rs::read_visual_style.
#
# Pre-R2010 (R2000/R2004 = 23 props, R2007 = 24 props: + bd2007_45).
# Indices into the silver `properties` array (top-level fields like
# description/style_type/face_* are NOT in the bag).
VISUALSTYLE_PRE2010 = [
    "face_opacity",            # 0  BD
    "face_specular",           # 1  BD
    "face_mono_color",         # 2  CMC
    "edge_intersection_color", # 3  CMC
    "edge_obscured_color",     # 4  CMC
    "edge_obscured_ltype",     # 5  BL
    "edge_crease_angle",       # 6  BD
    "edge_modifier",           # 7  BL
    "edge_color",              # 8  CMC
    "edge_opacity",            # 9  BD
    "edge_width",              # 10 BS
    "edge_overhang",           # 11 BS
    "edge_jitter",             # 12 BL
    "edge_silhouette_color",   # 13 CMC
    "edge_silhouette_width",   # 14 BS
    "edge_halo_gap",           # 15 RC
    "edge_isolines",           # 16 BS
    "edge_do_hide_precision",  # 17 B
    "edge_style_apply",        # 18 BS
    "edge_intersection_ltype", # 19 BS
    "display_settings",        # 20 BL
    "display_brightness_bl",   # 21 BLd
    "display_shadow_type",     # 22 BL
    "bd2007_45",               # 23 BD (R2007+ only)
]

# R2010+ core bag (28 props): gold emits each value paired with a `*_int`
# "modified" flag (the silver bag's `enabled` field), except ext_lighting_model
# and internal_only which are top-level. Order matches the reader's push order.
VISUALSTYLE_2010_CORE = [
    "face_lighting_model",     # 0
    "face_lighting_quality",   # 1
    "face_color_mode",         # 2
    "face_modifier",           # 3
    "face_opacity",            # 4
    "face_specular",           # 5
    "face_mono_color",         # 6
    "edge_model",              # 7
    "edge_style",              # 8
    "edge_intersection_color", # 9
    "edge_obscured_color",     # 10
    "edge_obscured_ltype",     # 11
    "edge_intersection_ltype", # 12
    "edge_crease_angle",       # 13
    "edge_modifier",           # 14
    "edge_color",              # 15
    "edge_opacity",            # 16
    "edge_width",              # 17
    "edge_overhang",           # 18
    "edge_jitter",             # 19
    "edge_silhouette_color",   # 20
    "edge_silhouette_width",   # 21
    "edge_halo_gap",           # 22
    "edge_isolines",           # 23
    "edge_do_hide_precision",  # 24
    "display_settings",        # 25
    "display_brightness",      # 26
    "display_shadow_type",     # 27
]

# R2013+ extended property bag (props[28:]) — gold names these generically,
# with edge_wiggle/strokes spliced in before the final b_prop37/bd_prop38/39.
VISUALSTYLE_2013_EXT = [
    "b_prop1c", "b_prop1d", "b_prop1e", "b_prop1f", "b_prop20", "b_prop21",
    "b_prop22", "b_prop23", "b_prop24", "bl_prop25", "bd_prop26", "bd_prop27",
    "bl_prop28", "c_prop29", "bl_prop2a", "bl_prop2b", "c_prop2c", "b_prop2d",
    "bl_prop2e", "bl_prop2f", "bl_prop30", "b_prop31", "bl_prop32", "c_prop33",
    "bd_prop34", "edge_wiggle", "strokes", "b_prop37", "bd_prop38", "bd_prop39",
]


# ── Material map flattening ───────────────────────────────────────────────
# Silver nests each texture map as {blend_factor, projection, tiling,
# auto_transform, transform, source, file_name, texture} and colors as
# {flag, factor, rgb}. Gold flattens them to dotted keys: `<map>.blendfactor`,
# `<map>.autotransform`, `<map>.transmatrix`, `<map>.filename`, … and
# `<color>.flag`, `<color>.factor`.
MATERIAL_MAPS = (
    "diffuse", "specular", "reflection", "opacity", "bump", "refraction",
    "normal",
)
_MATERIAL_MAP_FIELD = {
    "blend_factor": "blendfactor",
    "auto_transform": "autotransform",
    "transform": "transmatrix",
    "file_name": "filename",
}


def _map_material(payload: Dict[str, Any], fields: Dict[str, Any], r2007: bool, r2010: bool) -> set:
    """Flatten silver Material maps/colors onto gold's dotted keys. Returns
    the set of silver payload keys consumed (to be skipped by the generic
    loop). Gates the advanced set on the gold spec (SINCE R_2007a)."""
    consumed = set()
    for m in MATERIAL_MAPS:
        for key in (m + "_map", m + "map"):
            if key in payload and isinstance(payload[key], dict):
                sub = payload[key]
                # Gold emits <map>.filename only when the map is file-based
                # (source==1); silver always emits it. Gate on source.
                src = sub.get("source", sub.get("source_type", 1))
                for sk, sv in sub.items():
                    if sk == "texture":
                        continue
                    gk = _MATERIAL_MAP_FIELD.get(sk, sk)
                    if gk == "filename" and src != 1:
                        continue
                    fields[f"{m}map.{gk}"] = normalize_value(sv)
                consumed.add(key)
    for c in ("ambient_color", "diffuse_color", "specular_color"):
        if c in payload and isinstance(payload[c], dict):
            sub = payload[c]
            for sk, sv in sub.items():
                if sk == "rgb":
                    # gold stores the color word as the UNSIGNED 32-bit
                    # int (0xC2FFFFFF == 3271557119); silver serializes
                    # the same i32 SIGNED (-1023410177) — emit unsigned.
                    if isinstance(sv, int):
                        fields[f"{c}.rgb"] = sv & 0xFFFFFFFF
                    continue
                fields[f"{c}.{sk}"] = normalize_value(sv)
            consumed.add(c)
    # Advanced set (gold SINCE R_2007a). Empirically, the corpus only carries
    # `translucence` (and the normalmap group is never emitted by gold even on
    # R2018), so gate each field to the version gold actually emits it.
    ADVANCED_2007 = ("translucence", "self_illumination", "reflectivity",
                     "illumination_model", "channel_flags", "mode")
    ADVANCED_LATER = ("indirect_bump_scale", "reflectance_scale",
                      "transmittance_scale", "two_sided_material", "luminance",
                      "luminance_mode", "normal_map_method",
                      "normal_map_strength", "is_anonymous",
                      "global_illumination", "final_gather", "color_bleed_scale",
                      "advanced_data_present")
    for k in ADVANCED_2007:
        if k in payload:
            consumed.add(k)
            if r2007:
                fields[k] = normalize_value(payload[k])
    for k in ADVANCED_LATER:
        if k in payload:
            consumed.add(k)
            # Not emitted by gold in the covered corpus (R2007–R2018); skip.
    # normalmap group is never emitted by gold in this corpus.
    for k in list(fields.keys()):
        if k.startswith("normalmap."):
            del fields[k]
    consumed.add("normal_map")
    return consumed


def _vs_property_value(prop: Dict[str, Any]) -> Any:
    """Unwrap a silver VisualStyleProperty {value: {Variant: x}} to a scalar,
    normalizing colors to gold's index convention."""
    v = prop.get("value")
    if isinstance(v, dict) and len(v) == 1:
        kind = next(iter(v.keys()))
        inner = v[kind]
        if kind == "Color":
            return normalize_color(inner)
        nv = normalize_value(inner)
        # gold casts BS wire values to BL ZERO-EXTENDED (dwg2.spec 2152
        # FIELD_CAST (edge_silhouette_width, BS, BL)): the corpus records
        # carry 0xFFCE where silver's i16 read is -50 (gold 65486).
        # Positive Short values are unchanged by the mask.
        if kind == "Short" and isinstance(nv, int):
            nv &= 0xFFFF
        return nv
    return normalize_value(v)


def _map_visual_style(payload: Dict[str, Any], fields: Dict[str, Any]) -> None:
    """Project silver's positional VisualStyle property bag onto gold's named
    fields. Top-level payload fields (description, style_type, face_*,
    edge_model, edge_style, internal_use_only, extended_lighting_model) are
    handled by the generic loop; this maps the `properties` bag."""
    props = payload.get("properties")
    if not isinstance(props, list):
        return
    n = len(props)
    if n in (23, 24):
        # Pre-R2010 layout.
        for i, name in enumerate(VISUALSTYLE_PRE2010):
            if i >= n:
                break
            fields[name] = _vs_property_value(props[i])
        fields["internal_only"] = 1 if payload.get("internal_use_only") else 0
    else:
        # R2010+ layout: core 28 paired value/enabled, then R2013+ extras.
        core = props[:28]
        for i, name in enumerate(VISUALSTYLE_2010_CORE):
            if i >= len(core):
                break
            fields[name] = _vs_property_value(core[i])
            fields[name + "_int"] = core[i].get("enabled", 1)
        # R2013+ extended bag (generic gold names), paired value/enabled.
        ext = props[28:]
        for i, name in enumerate(VISUALSTYLE_2013_EXT):
            if i >= len(ext):
                break
            v = _vs_property_value(ext[i])
            fields[name] = v
            fields[name + "_int"] = ext[i].get("enabled", 1)
        if "extended_lighting_model" in payload:
            fields["ext_lighting_model"] = normalize_value(payload["extended_lighting_model"])
        fields["internal_only"] = 1 if payload.get("internal_use_only") else 0


def normalize_silver(
    data: Dict[str, Any],
    ignore: Optional[Dict[str, Any]] = None,
) -> List[Dict[str, Any]]:
    ignore_set: set = set()
    ignore_patterns: List[str] = []
    if ignore:
        ignore_set.update(ignore.get("global", {}).get("ignore", []))
        for pat in ignore.get("global", {}).get("ignore", []):
            if "*" in pat:
                ignore_patterns.append(pat)

    # Build layer name -> handle map.
    layer_map: Dict[str, int] = {}
    layers = data.get("layers", {})
    if isinstance(layers, dict):
        entries = layers.get("entries", {})
        for _key, rec in entries.items():
            if isinstance(rec, dict) and "name" in rec and "handle" in rec:
                layer_map[rec["name"]] = rec["handle"]

    common_dwg = data.get("_common_dwg", {})
    entities = data.get("entities", [])
    objects = data.get("objects", {})

    # Silver's DWG reader parses object reactors into the `reactors_by_handle`
    # side channel (document.rs: populated on read, consumed on write). Most
    # object structs don't carry a reactors field, so the payloads below would
    # otherwise have none and every gold `reactors` row would diff. Inject the
    # side-channel list into each payload (keyed by decimal handle string, as
    # serde serializes integer map keys) so the existing `reactors` projections
    # in `merge_common`/`_object_common_fields`/table records pick it up.
    reactors_by_handle = data.get("reactors_by_handle", {})
    # Same side-channel pattern for the xdictionary handle: silver's reader
    # stores each object's extension-dictionary handle in the document-level
    # `xdic_by_handle` map (consumed by the DWG writer on round-trip). Objects
    # surface theirs as `xdictionary_handle`, but table records and control
    # objects carry no per-record field — project theirs from this map.
    # Keys are decimal handle strings (serde integer map keys), like
    # reactors_by_handle.
    xdic_by_handle = data.get("xdic_by_handle", {})
    # R2013+ AcDs (SAB) data-store bit: silver's reader stores the handles of
    # every object whose data stream set `has_ds_data` in the document-level
    # `dwg_data_store_handles` set (round-tripped by the DWG writer).
    # Serialized as a plain list of handle ints.
    dwg_ds_handles = {str(h) for h in data.get("dwg_data_store_handles", [])}
    # Raw-remainder side channel (gold's HANDLE_UNKNOWN_BITS window, spec.h
    # 578): silver's reader peeks the bits from the end of the common
    # prologue to the record end and stores the uppercase hex keyed by
    # handle. Emit for the classes whose gold spec block carries the macro
    # AND was observed to emit a non-zero remainder on the corpus — emitting
    # for a class gold does not would create extra_in_silver rows.
    # UNKNOWN_OBJ/UNKNOWN_ENT are excluded: their hex is dropped
    # symmetrically in normalize_gold (the landed UNKNOWN projections).
    unknown_bits_map = data.get("unknown_bits_by_handle", {})

    def _emit_unknown_bits(gold_type: str, handle: Any,
                           fields: Dict[str, Any]) -> None:
        """Emit gold's raw-remainder hex for the HANDLE_UNKNOWN_BITS
        classes from the reader side channel. `handle` is the record's
        silver handle (entities: payload common; objects: payload)."""
        if gold_type not in _UNKNOWN_BITS_TYPES:
            return
        if not isinstance(handle, int):
            return
        ub = unknown_bits_map.get(str(handle))
        if isinstance(ub, str) and ub:
            fields["unknown_bits"] = ub

    def _lookup_xdic(raw_handle: Any) -> Any:
        if not isinstance(raw_handle, int):
            return None
        return xdic_by_handle.get(str(raw_handle))

    def _inject_xdic(payload: Any) -> None:
        """Set `xdictionary_handle` from the document-level xdic_by_handle
        map when the payload doesn't carry one. Most object structs serialize
        the field (null when unset); TableStyle-style structs don't — but the
        map holds every object's xdictionary handle (the reader stores it,
        the DWG writer consumes it for round-trip write-back), so project
        from there. Gold serializes xdicobjhandle whenever the object owns
        an extension dictionary (all versions), and the R2004+
        is_xdic_missing bit otherwise."""
        if not isinstance(payload, dict):
            return
        if payload.get("xdictionary_handle") is not None:
            return
        xdic = _lookup_xdic(payload.get("handle"))
        if xdic:
            payload["xdictionary_handle"] = xdic

    def _inject_reactors(payload: Any) -> None:
        if not isinstance(payload, dict) or "reactors" in payload:
            return
        h = payload.get("handle")
        if not isinstance(h, int):
            return
        v = reactors_by_handle.get(str(h))
        if v:
            payload["reactors"] = v

    # DWG/DXF version string, e.g. "AC1015". AC1018 (R2004) introduced the
    # `is_xdic_missing` bit and AC1027 (R2013) the `has_ds_data` bit on every
    # object handle stream (common_object_handle_data.spec SINCE R_2004a /
    # SINCE R_2013). Both are emitted for objects and table records alike.
    version = data.get("version", "")
    r2000_plus = isinstance(version, str) and version >= "AC1015"
    r2004_plus = isinstance(version, str) and version >= "AC1018"
    r2007_plus = isinstance(version, str) and version >= "AC1021"
    r2010_plus = isinstance(version, str) and version >= "AC1024"
    r2013_plus = isinstance(version, str) and version >= "AC1027"
    r2018_plus = isinstance(version, str) and version >= "AC1032"
    # R13/R14 (AC1012/AC1014) — the golden-entities era gates
    # (gold's spec blocks swap era-specific field sets at this boundary:
    # the common entity's isbylayerlt bit, the LAYER flag bits, the
    # DIMSTYLE R13 block, BLOCK_HEADER's layout/description/xref_loaded).
    r13_14 = isinstance(version, str) and version in ("AC1012", "AC1014")

    out: List[Dict[str, Any]] = []

    # Block-record name -> handle map, for resolving INSERT.block_header
    # (silver stores the block name; gold wants the BLOCK_RECORD handle).
    block_handle_map: Dict[str, int] = {}
    br_table = data.get("block_records", {})
    if isinstance(br_table, dict):
        for name, rec in br_table.get("entries", {}).items():
            if isinstance(rec, dict) and isinstance(rec.get("handle"), int):
                block_handle_map[str(name).upper()] = rec["handle"]

    # Text-style name -> handle map (silver's text_styles table): gold emits
    # the STYLE-table HANDLE for TEXT/ATTDEF/ATTRIB style (dwg.spec FIELD
    # HANDLE style, SINCE R_13b1) while silver stores the resolved NAME.
    # Same pattern as the layer_map below and the MTEXT style resolution.
    style_map: Dict[str, int] = {}
    ts_table = data.get("text_styles", {})
    if isinstance(ts_table, dict):
        for ts_name, ts_rec in (ts_table.get("entries") or {}).items():
            if isinstance(ts_rec, dict) and isinstance(ts_rec.get("handle"), int):
                style_map[str(ts_rec.get("name") or ts_name).upper()] = ts_rec["handle"]

    def _style_handle(name: Any) -> Optional[int]:
        if not isinstance(name, str):
            return None
        return style_map.get(name.upper())

    # Entities.
    for entity in entities:
        if not isinstance(entity, dict) or len(entity) != 1:
            continue
        silver_type = list(entity.keys())[0]
        gold_type = ENTITY_TYPE_MAP.get(silver_type, silver_type.upper())
        payload = entity[silver_type]
        if not isinstance(payload, dict):
            continue
        # §20 the R2018 record-identity packet: the wire-capture channels
        # — the reader's spline wire-scenario BL (34c75d0) and the table
        # entity's captured body — are writer-side codec fields with no
        # gold counterpart (the source_version precedent), never
        # comparison payload on any entity type. A captured value rides
        # these keys on every DWG read; popping them loop-universal
        # keeps the read-axis comparison to semantic structure.
        # The spline's wire scenario BL feeds the semantic `scenario`
        # projection (gold dwg.spec 2571: the BL stands pre-R2013; the
        # R2013b decoder re-derives) — stash it before the pop; the raw
        # key itself never compares.
        _spline_wire_scenario = payload.get("dwg_wire_scenario")
        for _wk in ("dwg_wire_scenario",
                    "dwg_wire_text",
                    "wire_main", "wire_main_bit_len",
                    "wire_text", "wire_text_bit_len",
                    "wire_handles", "wire_handles_bit_len",
                    "wire_dxf_version"):
            payload.pop(_wk, None)
        # DIMENSION entities: silver wraps Dimension.<Kind>.base (common +
        # dim-common) with the kind specifics beside base. Unwrap before the
        # common extraction; the per-kind gold block names per dwg.spec's
        # DIMENSION_* entities.
        if (silver_type == "Dimension" and isinstance(payload, dict) and payload
                and isinstance(next(iter(payload.values())), dict)):
            _dim_kind = next(iter(payload))
            _dim_inner = payload[_dim_kind]
            gold_type = {
                "Aligned": "DIMENSION_ALIGNED", "Arc": "ARC_DIMENSION",
                "Ordinate": "DIMENSION_ORDINATE", "Angular2Ln": "DIMENSION_ANG2LN",
                "Linear": "DIMENSION_LINEAR", "Angular3Pt": "DIMENSION_ANG3PT",
                "Diameter": "DIMENSION_DIAMETER", "Radius": "DIMENSION_RADIUS",
            }.get(_dim_kind, "DIMENSION")
            _dim_base = _dim_inner.get("base") if isinstance(_dim_inner.get("base"), dict) else {}
            _flat = dict(_dim_base)
            _flat.update({k: v for k, v in _dim_inner.items() if k != "base"})
            _flat["_dimension_kind"] = _dim_kind
            payload = _flat
        # Underlay entities: gold's spec blocks are per-kind (PDFUNDERLAY,
        # DWFUNDERLAY, ...); the kind lives on silver's underlay_type;
        # gold's underlay records never carry entity-common graphic_data
        # (census: 2000/Underlay.dwg's three PDFUNDERLAYs).
        if silver_type == "Underlay" and isinstance(payload.get("underlay_type"), str):
            gold_type = {
                "Pdf": "PDFUNDERLAY", "Dwf": "DWFUNDERLAY",
                "Png": "PNGUNDERLAY", "Jpeg": "JPGUNDERLAY",
                "Jpg": "JPGUNDERLAY",
            }.get(payload["underlay_type"], gold_type)
            payload.pop("graphic_data", None)
        # MESH: entity-common graphic_data never appears in gold's record
        # (census: 2004/Surface.dwg's two meshes; the MULTILEADER/
        # ARC_DIMENSION/LIGHT precedent).
        if silver_type == "Mesh":
            payload.pop("graphic_data", None)
        # CAcLayoutPrintConfig instances (class 523/LAYOUTPRINTCONFIG):
        # silver parses them into the Extended/LayoutPrintConfig wrapper;
        # gold types the record LAYOUTPRINTCONFIG (a live class in
        # dwgread's dxfname mapping — PolyLine2D.dwg's 857 emits
        # class_version 2 / flag 0 + the R2000 null entity-chain pair).
        if (silver_type == "Extended"
                and isinstance(payload.get("data"), dict)
                and "LayoutPrintConfig" in payload["data"]
                and "RegisteredClass" not in payload["data"]):
            gold_type = "LAYOUTPRINTCONFIG"
        # SECTIONOBJECT (dwg2.spec, live — the AcDbSection entity on
        # 2018/LiveSection1.dwg): silver parses it into the Extended/
        # SectionObject wrapper with the same wire fields.
        if (silver_type == "Extended"
                and isinstance(payload.get("data"), dict)
                and "SectionObject" in payload["data"]):
            gold_type = "SECTIONOBJECT"
        # SURFACE family (2004/Surface.dwg): gold's EXTRUDED/LOFTED/
        # REVOLVED/SWEPT SURFACE blocks sit in the dead frame (§8.1.1 —
        # dwg2.spec 3716-4513): the class instances decode as raw
        # UNKNOWN_ENT, common-only after normalization (verified: gold's
        # normalized records carry just color/handle/invisible/
        # is_xdic_missing/layer/linewt/ltype_flags/ltype_scale/
        # plotstyle_flags/reactors — normalize_gold drops their eed,
        # preview, and unknown_bits). The Plane variant is gold's LIVE
        # PLANESURFACE (typed record with the full AcDbSurface field
        # set — projected in the PLANESURFACE branch below).
        if silver_type == "Surface":
            if payload.get("kind") == "Plane":
                gold_type = "PLANESURFACE"
            else:
                gold_type = "UNKNOWN_ENT"
                for _sk in ("acis_data", "history_handle", "kind",
                            "modeler_format_version", "point_of_reference",
                            "silhouettes", "surface_data", "u_isolines",
                            "v_isolines", "wires"):
                    payload.pop(_sk, None)

        if gold_type == "UNKNOWN_ENT":
            # Gold's unmodeled-class ENTITIES (UNKNOWN_ENT, the §8.1.1
            # liveness fallout) serialize with the common fields plus the
            # raw bits its spec could not decode (unknown_bits/graphic_data
            # dropped symmetrically in normalize_gold). Silver's
            # Extended/Unknown variants carry class metadata and the parsed
            # payloads that would otherwise leak through the generic loop
            # as extra_in_silver — the Table-entity pop-list precedent.
            for sk in ("dxf_name", "cpp_class_name", "type_name",
                       "dwg_type_code", "dwg_handle_bits", "graphic_data",
                       "raw_dwg_data", "raw_dwg_handle_bits",
                       "raw_dwg_version", "source_version",
                       # §19 H8h-ext-12: the AC1021 TABLECONTENT wire
                       # captures (writer-side fidelity channels, the
                       # dwg_raw_tail_bits precedent — gold's unknown-class
                       # decode has no counterpart). §20 the R2018
                       # record-identity packet extended the same channels
                       # to the table entity + the associative aligned-dim
                       # body and added the replay-gate source version.
                       "wire_main", "wire_main_bit_len",
                       "wire_text", "wire_text_bit_len",
                       "wire_handles", "wire_handles_bit_len",
                       "wire_dxf_version"):
                payload.pop(sk, None)

        common = payload.get("common", {})
        handle = common.get("handle")
        common_key = "0x{:X}".format(handle) if isinstance(handle, int) else str(handle)
        common_dwg_entry = common_dwg.get(common_key)

        # INSERT-chain synthesized kids (the ATTRIB records + trailing
        # SEQEND for insert-attached attributes; filled in the Insert
        # branch below, appended after the parent record like the polyline
        # kid machinery).
        _ins_kids: List[Dict[str, Any]] = []
        # Capture the polyline-family child lists BEFORE the per-variant
        # branches consume them from the payload (e.g. PolyfaceMesh pops
        # vertices/faces for its own numverts/first_vertex projection).
        # The mesh parent's gold name is POLYLINE_MESH (the PolygonMesh
        # variant maps via OBJECT/ENTITY_TYPE_MAP) — NOT POLYGON_MESH,
        # the old tuple string never matched so no VERTEX_MESH kids were
        # synthesized for TS1 (12 dropped records).
        _kid_verts = _kid_faces = None
        _seq_h_parent = None
        if gold_type in ("POLYLINE_2D", "POLYLINE_3D", "POLYLINE_MESH",
                         "POLYLINE_PFACE"):
            _kid_verts = payload.get("vertices")
            _kid_faces = payload.get("faces")
            # The wire's trailing SEQEND handle is retained on the parent
            # (the builder restores it for every polyline family). Capture
            # and pop it here, BEFORE the generic field loop emits it as
            # an extra_in_silver row — the synthesized SEQEND below uses
            # the real handle; gold's parent records carry no seqend
            # handle field.
            _seq_h_parent = payload.pop("seqend_handle", None)

        fields = merge_common(common, common_dwg_entry, layer_map, r13_14)
        # Retained per-SEQEND common flag pairs (the wire SEQEND's own
        # data; populated on the INSERT and polyline-family structs by
        # the builder). Stash them here so no generic loop leaks them;
        # the poly-family SEQEND synthesis below emits them verbatim.
        _seq_pf_stash = payload.pop("seqend_plotstyle_flags", None)
        _seq_sf_stash = payload.pop("seqend_shadow_flags", None)
        _seq_iblt_stash = payload.pop("seqend_isbylayerlt", None)
        if gold_type == "UNKNOWN_ENT" and "graphic_data" in fields:
            # Silver's raw passthrough entities keep their graphic-data bytes
            # in the serde-skipped EntityCommon map (_common_dwg), which
            # merge_common just folded into `fields` — the payload pop list
            # above cannot see them. Gold's UNKNOWN_ENT never emits a
            # graphic-data array.
            fields.pop("graphic_data", None)
        if gold_type in ("MESH", "PDFUNDERLAY", "DWFUNDERLAY",
                         "PNGUNDERLAY", "JPGUNDERLAY", "PLANESURFACE",
                         "UNKNOWN_ENT") and "graphic_data" in fields:
            # Same _common_dwg leak as UNKNOWN_ENT; gold's MESH records
            # (2004/Surface.dwg) and underlay records (2004/Underlay.dwg
            # PDFUNDERLAYs) never carry graphic_data (the MULTILEADER/
            # ARC_DIMENSION/LIGHT pop precedent).
            fields.pop("graphic_data", None)

        if gold_type in ("PDFUNDERLAY", "DWFUNDERLAY", "PNGUNDERLAY",
                         "JPGUNDERLAY", "DGNUNDERLAY"):
            # gold UNDERLAY_fields (dwg2.spec 1616, AcDbUnderlayReference):
            # definition_id handle, extrusion, ins_pt, one scale triple
            # (3BD_1 — silver stores x_/y_/z_scale), angle, flag RC
            # (silver's bitflags string: CLIPPING=1/ON=2/MONOCHROME=4/
            # ADJUST_FOR_BACKGROUND=8/CLIP_INSIDE=16), contrast/fade RCd
            # (same names, generic loop), clip_verts 2RD pairs. Silver's
            # clip_inverted bool is the CLIP_INSIDE bit it already counts
            # in flags (pop). verified on 2004/Underlay.dwg's three
            # PDFUNDERLAYs (flag 30/31/27 vs the split strings).
            fields["definition_id"] = normalize_handle_value(
                payload.pop("definition_handle", None))
            fields["ins_pt"] = normalize_value(payload.pop("insertion_point", None))
            fields["angle"] = normalize_float(payload.pop("rotation", 0.0) or 0.0)
            fields["scale"] = [normalize_float(payload.pop("x_scale", 1.0) or 0.0),
                               normalize_float(payload.pop("y_scale", 1.0) or 0.0),
                               normalize_float(payload.pop("z_scale", 1.0) or 0.0)]
            _uflag = payload.pop("flags", 0)
            if isinstance(_uflag, str):
                _UF = {"CLIPPING": 1, "ON": 2, "MONOCHROME": 4,
                       "ADJUST_FOR_BACKGROUND": 8, "CLIP_INSIDE": 16,
                       "NONE": 0}
                _uval = 0
                for _ut in _uflag.split(" | "):
                    _uval |= _UF.get(_ut.strip(), 0)
                _uflag = _uval
            fields["flag"] = _uflag
            _cv = payload.pop("clip_boundary_vertices", None)
            if isinstance(_cv, list):
                fields["clip_verts"] = [normalize_value(v) for v in _cv]
            payload.pop("clip_inverted", None)
            payload.pop("underlay_type", None)

        if gold_type == "MESH":
            # gold MESH (dwg2.spec 2522, AcDbSubDMesh — live; the two
            # 2004/Surface.dwg records): dlevel BS 71 is the wire version
            # (silver Mesh.version), is_watertight B 72 is silver's
            # blend_crease; num_subdiv_vertex (BL 91) is silver's
            # misnamed subdivision_level — a COUNT of subdiv points silver
            # never reads; 0 on the corpus so gold emits no subdiv_vertex
            # (pop). vertex ← vertices; gold's faces/edges are the FLAT
            # wire vectors ([n, *idx] per face, [idxfrom, idxto] per
            # edge); crease is the per-edge crease BD vector in edge
            # order (None → 0.0); unknown_b1/b2 are the retained raw
            # trailing bits (reader parity packet).
            fields["dlevel"] = payload.pop("version", 0)
            fields["is_watertight"] = 1 if payload.pop("blend_crease", None) else 0
            payload.pop("subdivision_level", None)
            _mv = payload.pop("vertices", None)
            if isinstance(_mv, list) and _mv:
                fields["vertex"] = [normalize_value(v) for v in _mv]
            _mf = payload.pop("faces", None)
            if isinstance(_mf, list) and _mf:
                _mflat: List[Any] = []
                for _mff in _mf:
                    _mfv = (_mff.get("vertices")
                            if isinstance(_mff, dict) else _mff)
                    if isinstance(_mfv, list):
                        _mflat.append(len(_mfv))
                        _mflat.extend(int(x) for x in _mfv)
                fields["faces"] = _mflat
            _me = payload.pop("edges", None)
            if isinstance(_me, list) and _me:
                # gold's REPEAT-edge dump prints one bare 0 per edge (the
                # idxfrom/idxto struct entries collapse under the
                # normalize_value dict-without-index rule, like FIELD's
                # childval; verified on both Surface.dwg records and
                # their rewrites): [0] * num_edges.
                fields["edges"] = [0] * len(_me)
                _ecrease: List[Any] = []
                for _med in _me:
                    _mec = (_med.get("crease")
                            if isinstance(_med, dict) else None)
                    _ecrease.append(normalize_float(
                        _mec if isinstance(_mec, (int, float)) else 0.0))
                fields["crease"] = _ecrease
            fields["unknown_b1"] = 1 if payload.pop("unknown_b1", None) else 0
            fields["unknown_b2"] = 1 if payload.pop("unknown_b2", None) else 0

        if gold_type == "PLANESURFACE":
            # gold dwg2.spec PLANESURFACE (live): AcDbSurface family —
            # acis (SAB), isolines, wires, the modeler version, the
            # point-of-reference, history_id and the SINCE-R2010 unknown
            # bits tail (the reader side channel holds it byte-identical;
            # PLANESURFACE is in _UNKNOWN_BITS_TYPES). Projection maps
            # silver's Plane-kind Surface payload 1:1 (verified on
            # 2004/Surface.dwg @1290). ENTITY-LOOP branch (the Plane
            # record is an entity, not an object).
            _ac = payload.get("acis_data")
            _sab = _ac.get("sab_data") if isinstance(_ac, dict) else None
            if isinstance(_sab, list):
                try:
                    _b = bytes(int(x) & 0xFF for x in _sab)
                    # gold's json_3dsolid SAB emission splits at the FIXED
                    # 15-byte banner ("%.*s" with 15, out_json.c 1605):
                    # "ACIS BinaryFile" then VALUE_BINARY of the REMAINING
                    # bytes — the banner is NOT NUL-terminated (the wire
                    # runs straight into 0x40 0x51), so a find(0) split
                    # put "@Q" in the head and shifted the hex tail.
                    if _b.startswith(b"ACIS BinaryFile"):
                        _p = 15
                    else:
                        _p = _b.find(0)
                    if _p > 0:
                        _head = _b[:_p].decode("latin-1")
                        _rest = _b[_p:].hex().upper()
                        fields["acis_data"] = [_head, _rest]
                        fields["acis_empty"] = 0
                except Exception:
                    pass
                # gold's raw tail bit after the wireframe block
                # (COMMON_3DSOLID FIELD_B acis_empty_bit): the ACIS
                # reader retains it — emit the retained value, not 0.
                _aeb = _ac.get("acis_empty_bit") if isinstance(_ac, dict) else None
                fields["acis_empty_bit"] = 1 if _aeb else 0
            _av = (str(_ac.get("version", "")) if isinstance(_ac, dict)
                   else "") or ""
            if _av.startswith("Version"):
                try:
                    fields["version"] = int(_av[len("Version"):])
                except ValueError:
                    pass
            _hh = payload.get("history_handle")
            fields["history_id"] = normalize_handle_value(
                _hh if isinstance(_hh, int) else 0)
            _ui = payload.get("u_isolines")
            _vi = payload.get("v_isolines")
            if isinstance(_ui, int):
                fields["u_isolines"] = _ui
            if isinstance(_vi, int):
                fields["v_isolines"] = _vi
            # gold COMMON_3DSOLID: `isolines` is the WIREFRAME block's BL
            # (wireframe cache) — silver retains it as
            # acis_data.wireframe_isolines — NOT the u+v isoline display
            # counts (12 vs 14 on 2004/Surface.dwg).
            _wi = _ac.get("wireframe_isolines") if isinstance(_ac, dict) else None
            if isinstance(_wi, int):
                fields["isolines"] = _wi
            fields["isoline_present"] = 1 if (
                _ac.get("wireframe_isoline_present")
                if isinstance(_ac, dict) else False) else 0
            _mfv = payload.get("modeler_format_version")
            if isinstance(_mfv, int):
                fields["modeler_format_version"] = _mfv
            _por = payload.get("point_of_reference")
            if isinstance(_por, dict):
                fields["point_present"] = 1
                fields["point"] = normalize_value(_por)
            fields["unknown"] = 0
            _ws = payload.get("wires")
            if isinstance(_ws, list):
                fields["wireframe_data_present"] = 1 if _ws else 0
                # gold's normalized wires REPEAT collapses one zero per
                # wire record (the [0]*n degenerate pattern: 12 wires →
                # [0]*12 on both sides).
                fields["wires"] = [0] * len(_ws)
            for _sk in ("acis_data", "history_handle", "kind",
                        "modeler_format_version", "point_of_reference",
                        "silhouettes", "surface_data", "u_isolines",
                        "v_isolines", "wires"):
                payload.pop(_sk, None)

        if silver_type == "Shape":
            # gold SHAPE (dwg.spec 2330, R13+ DWG path): 3BD ins_pt, BD
            # scale, BD0 rotation, BD width_factor, BD oblique_angle, BD0
            # thickness, BS style_id, 3BD extrusion, style handle (5,7).
            # Silver's read_shape consumes the same slots under shifted
            # names: its `size` holds the wire SCALE (gold's spec comment
            # "documented as size"), its `relative_x_scale` holds the wire
            # WIDTH FACTOR (read order: ins_pt, size, rotation,
            # relative_x_scale, oblique, thickness, shape_number, normal,
            # style) — value-verified on entities-2d (ins 6,6 / rotate
            # 0.5236 / style_id 131 / style 56). shape_name/style_name are
            # silver lookup twins with no wire field.
            fields["ins_pt"] = normalize_value(payload.pop("insertion_point"))
            fields["scale"] = normalize_float(payload.pop("size", 0.0) or 0.0)
            fields["width_factor"] = normalize_float(
                payload.pop("relative_x_scale", 1.0) or 0.0)
            fields["style_id"] = payload.pop("shape_number", 0)
            fields["style"] = normalize_handle_value(
                payload.pop("style_handle", None))
            payload.pop("shape_name", None)
            payload.pop("style_name", None)

        if gold_type == "LAYOUTPRINTCONFIG":
            # gold's record (PolyLine2D.dwg 857): the common entity set +
            # class_version + flag + the R2000-era null entity-chain trio;
            # silver's parsed layout-print bits ride the nested data dict.
            _lpc = payload.pop("data") if isinstance(payload.get("data"), dict) else {}
            _lpd = (_lpc.get("LayoutPrintConfig")
                    if isinstance(_lpc.get("LayoutPrintConfig"), dict) else {})
            fields["class_version"] = _lpd.get("class_version", 0)
            fields["flag"] = _lpd.get("flag", 0)
            if not r2004_plus:
                fields["prev_entity"] = normalize_handle_value(0)
                fields["next_entity"] = normalize_handle_value(0)
                fields["nolinks"] = 0
            fields.pop("graphic_data", None)

        if gold_type == "SECTIONOBJECT":
            # gold SECTIONOBJECT (dwg2.spec, the AcDbSection entity —
            # LiveSection1.dwg 552): state/flags/name/vert_dir/top/
            # bottom_height, indicator_alpha + the bare indicator_color
            # index, `verts` (3BD points), section_settings (code 3,
            # the settings object handle). Gold's preview* fields are
            # ignore-listed (never normalized).
            _soc = payload.pop("data") if isinstance(payload.get("data"), dict) else {}
            _so = (_soc.get("SectionObject")
                   if isinstance(_soc.get("SectionObject"), dict) else {})
            fields["state"] = _so.get("state", 0)
            fields["flags"] = _so.get("flags", 0)
            fields["name"] = _so.get("name", "") or ""
            fields["vert_dir"] = normalize_value(_so.get("vertical_direction"))
            fields["top_height"] = normalize_float(_so.get("top_height", 0.0) or 0.0)
            fields["bottom_height"] = normalize_float(_so.get("bottom_height", 0.0) or 0.0)
            fields["indicator_alpha"] = _so.get("indicator_alpha", 0)
            _sic = _so.get("indicator_color")
            fields["indicator_color"] = (_sic.get("Index")
                                          if isinstance(_sic, dict) else _sic) or 0
            _svn = _so.get("vertices")
            if isinstance(_svn, list) and _svn:
                fields["verts"] = [normalize_value(p) for p in _svn]
            fields["section_settings"] = normalize_handle_value(
                _so.get("settings_handle", 0) or 0)
            _sbl = _so.get("back_line_vertices")
            if isinstance(_sbl, list) and _sbl:
                fields["back_line_verts"] = [normalize_value(p) for p in _sbl]
            fields.pop("graphic_data", None)

        if silver_type == "LwPolyline":
            # Gold LWPOLYLINE: flag (bitfield), points (2D array), bulges,
            # conditional const_width/elevation/thickness/extrusion.
            # Silver stores vertices (nested) + is_closed/plinegen/
            # constant_width/elevation/thickness/extrusion separately.
            # Gold prints the RAW wire flag (dwg.spec LWPOLYLINE BSx 90 —
            # the presence bits incl. VERTEXIDCOUNT 0x400 and the
            # closed/plinegen marks; spec.h FLAG_LWPOLYLINE_*). Retained on
            # the struct since the reader-capture packet; compose by value
            # only when absent (constructed docs).
            _lwf = payload.pop("dwg_raw_flag", None)
            flag = 0
            if payload.get("is_closed"):
                flag |= 512
            if payload.get("plinegen"):
                flag |= 256
            const_width = payload.get("constant_width", 0.0)
            if const_width:
                flag |= 4
            elevation = payload.get("elevation", 0.0)
            if elevation:
                flag |= 8
            thickness = payload.get("thickness", 0.0)
            if thickness:
                flag |= 2
            extrusion = payload.get("extrusion")
            if extrusion and extrusion != [0.0, 0.0, 1.0]:
                flag |= 1
            verts = payload.get("vertices", [])
            if isinstance(verts, list) and verts:
                pts = []
                bulges = []
                has_bulge = False
                for vx in verts:
                    if isinstance(vx, dict):
                        loc = vx.get("location")
                        # location is {x, y} dict or [x, y] list.
                        if isinstance(loc, dict):
                            pts.append([normalize_float(loc.get("x", 0.0)), normalize_float(loc.get("y", 0.0))])
                        elif isinstance(loc, list) and len(loc) >= 2:
                            pts.append([normalize_float(loc[0]), normalize_float(loc[1])])
                        b = vx.get("bulge", 0.0)
                        bulges.append(normalize_float(b))
                        if b:
                            has_bulge = True
                    elif isinstance(vx, list) and len(vx) >= 2:
                        pts.append([normalize_float(vx[0]), normalize_float(vx[1])])
                fields["points"] = pts
                # Gold always emits bulges (empty list when none set).
                fields["bulges"] = bulges if has_bulge else []
                if has_bulge and not isinstance(_lwf, int):
                    flag |= 16
            if isinstance(_lwf, int):
                # the retained raw wire bits are the truth (presence bits +
                # VERTEXIDCOUNT 0x400 — Dynblocks' 1536/1024-flag records)
                flag = int(_lwf) & 0xFFFF
            fields["flag"] = flag
            # vertexids: gold always carries the array SINCE R_2010b (an
            # empty vector when flag&1024 is clear); with 1024 set the
            # per-vertex wire ids (silver's LwVertex.vertex_id — read
            # under the same flag).
            if r2010_plus:
                _lvx = payload.get("vertices") or []
                if flag & 1024 and isinstance(_lvx, list):
                    _lvi = [(vx.get("vertex_id", 0) if isinstance(vx, dict) else 0)
                            for vx in _lvx]
                    # normalize_gold reinterprets a 4-int list as the raw
                    # handle tuple [code, size, value, absref] (Dynblocks:
                    # [21, 22, 23, 24] -> the {21/22/23/24} dict) — mirror
                    # that exact shape. Longer lists stay lists ([13, 14,
                    # 25, 15, 16, 26] across versions).
                    if len(_lvi) == 4:
                        fields["vertexids"] = {"code": _lvi[0], "size": _lvi[1],
                                               "value": _lvi[2], "absref": _lvi[3]}
                    else:
                        fields["vertexids"] = _lvi
                else:
                    fields["vertexids"] = []
            if flag & 4:
                fields["const_width"] = normalize_float(const_width)
            if flag & 8:
                fields["elevation"] = normalize_float(elevation)
            if flag & 2:
                fields["thickness"] = normalize_float(thickness)
            if flag & 1:
                fields["extrusion"] = normalize_value(extrusion)
            # Silver stores the extrusion as `normal`; pop it so it doesn't
            # appear as extra (gold's `extrusion` is only present when flag&1).
            payload.pop("normal", None)
            # Drop silver's split fields so they don't appear as extra.
            for sk in ("vertices", "is_closed", "plinegen", "constant_width",
                       "elevation", "thickness", "extrusion"):
                payload.pop(sk, None)

        # INSERT entity (dwg.spec 735, R2000b+ DWG path): silver stores
        # insert_point/x_scale/y_scale/z_scale; gold uses ins_pt (3DPOINT),
        # scale (3BD) + scale_flag (BB). block_header is a handle (not the
        # name). has_attribs is a B (not dwg_minsert). attributes -> attribs
        # (handle vector, R2004a+). seqend_handle -> seqend.
        # NOTE: must run BEFORE the Point block (INSERT also has insert_point).
        if silver_type == "Insert":
            ins_pt = payload.get("insert_point")
            if ins_pt is not None:
                fields["ins_pt"] = normalize_value(ins_pt)
                payload.pop("insert_point", None)
            # scale: silver stores x/y/z scalars; gold uses scale (3BD) +
            # scale_flag (BB). Recompose.
            xs = payload.get("x_scale", 1.0)
            ys = payload.get("y_scale", 1.0)
            zs = payload.get("z_scale", 1.0)
            if r13_14:
                # gold R13/R14 INSERT (dwg.spec 735): the scale 3BD
                # array; scale_flag is R2000b+ and the col/row_spacing
                # MINSERT fields are R2000b+ too.
                fields["scale"] = [normalize_float(xs), normalize_float(ys),
                                   normalize_float(zs)]
                for kk in ("x_scale", "y_scale", "z_scale",
                           "col_spacing", "row_spacing"):
                    payload.pop(kk, None)
            elif r2000_plus:
                # scale_flag: 3=all 1.0, 1=x=1 y/z=DD, 2=all equal, 0=full
                if xs == 1.0 and ys == 1.0 and zs == 1.0:
                    fields["scale_flag"] = 3
                    fields["scale"] = [1.0, 1.0, 1.0]
                elif xs == ys and xs == zs:
                    fields["scale_flag"] = 2
                    fields["scale"] = [xs, ys, zs]
                elif xs == 1.0:
                    fields["scale_flag"] = 1
                    fields["scale"] = [xs, ys, zs]
                else:
                    fields["scale_flag"] = 0
                    fields["scale"] = [xs, ys, zs]
                for kk in ("x_scale", "y_scale", "z_scale"):
                    payload.pop(kk, None)
            # INSERT chain (dwg.spec INSERT/ATTRIB/SEQEND): silver keeps the
            # parsed ATTRIB entities in `attributes` (never emitted as
            # top-level entities); gold materializes the child ATTRIB
            # records plus their trailing SEQEND, linked from the INSERT:
            # R2004a+ `attribs` handle vector, pre-2004 first_attrib/
            # last_attrib chain ends (gold code 4), and `seqend` (code 3)
            # only when attributes exist. Kid shapes verified on
            # example_2000/2004/2007/2010/2018 (probes st_*).
            attrs = payload.get("attributes")
            _kid_handles: List[int] = []
            if isinstance(attrs, list):
                fields["has_attribs"] = 1 if attrs else 0
                for a in attrs:
                    if isinstance(a, dict):
                        _ah = (a.get("common") or {}).get("handle")
                        if isinstance(_ah, int):
                            _kid_handles.append(_ah)
                if _kid_handles:
                    if r2004_plus:
                        fields["attribs"] = [normalize_handle_value(_ah)
                                             for _ah in _kid_handles]
                    else:
                        fields["first_attrib"] = normalize_handle_value(_kid_handles[0])
                        fields["last_attrib"] = normalize_handle_value(_kid_handles[-1])
                payload.pop("attributes", None)
            else:
                payload.pop("attributes", None)
            _seq_h = payload.get("seqend_handle")
            if _kid_handles and isinstance(_seq_h, int):
                fields["seqend"] = normalize_handle_value(_seq_h)
            if isinstance(attrs, list) and attrs:
                # Parent-common subset for the kids (the polyline
                # _vcommon precedent — the nested attribs have no
                # _common_dwg entry of their own).
                _vcom = {k: fields[k] for k in (
                    "layer", "is_xdic_missing", "has_ds_data", "color",
                    "ltype_scale", "ltype_flags", "plotstyle_flags",
                    "material_flags", "shadow_flags", "has_full_visualstyle",
                    "has_face_visualstyle", "has_edge_visualstyle",
                    "invisible", "linewt") if k in fields}
                for a in attrs:
                    if not isinstance(a, dict):
                        continue
                    _ah = (a.get("common") or {}).get("handle")
                    if not isinstance(_ah, int):
                        continue
                    rec = dict(_vcom)
                    rec["handle"] = normalize_handle_value(_ah)
                    rec["ownerhandle"] = normalize_handle_value(handle)
                    # The kid's own xdic slot (dwg.spec ATTRIB
                    # XDICOBJHANDLE): gold prints the real handle when
                    # the record has one; the parent-subset xdic flags
                    # do not apply to the kid.
                    _axh = (a.get("common") or {}).get("xdictionary_handle")
                    if isinstance(_axh, int) and _axh:
                        rec["xdicobjhandle"] = normalize_handle_value(_axh)
                        rec.pop("is_xdic_missing", None)
                    if a.get("tag") is not None:
                        rec["tag"] = a.get("tag")
                    if a.get("value") is not None:
                        rec["text_value"] = a.get("value")
                    if a.get("field_length") is not None:
                        rec["field_length"] = a.get("field_length")
                    if a.get("height") is not None:
                        rec["height"] = normalize_value(a.get("height"))
                    _ip = a.get("insertion_point")
                    _ipz = 0.0
                    if _ip is not None:
                        _nv = normalize_value(_ip)
                        if isinstance(_nv, list) and len(_nv) > 2:
                            _ipz = _nv[2]
                            _nv = _nv[:2]
                        rec["ins_pt"] = _nv
                    rec["thickness"] = normalize_value(a.get("thickness") or 0)
                    _nm = a.get("normal")
                    if _nm is not None:
                        rec["extrusion"] = normalize_value(_nm)

                    def _iz(v):
                        try:
                            return abs(float(v)) < 1e-9
                        except (TypeError, ValueError):
                            return True
                    ap = a.get("alignment_point")
                    apn = normalize_value(ap)
                    rot = a.get("rotation")
                    obl = a.get("oblique_angle")
                    wf = a.get("width_factor")
                    gen = a.get("text_generation_flags")
                    ha = a.get("horizontal_alignment")
                    va = a.get("vertical_alignment")
                    # dataflags: the shared TEXT/ATTDEF absence mask
                    # (dwg.spec 330-345). The retained raw wire byte
                    # (AttributeEntity.raw_dataflags) is the truth —
                    # which optionals AutoCAD actually wrote (an explicit
                    # width_factor 1.0 keeps bit 4 clear where a value
                    # recomposition would set it). Compose by value only
                    # where the byte is absent (R13-14 layout):
                    # bit0 elevation absent iff z == 0; bit1 alignment_pt
                    # absent iff it EQUALS the insertion point (the 2DD
                    # default) — a real all-zero [0,0] alignment is
                    # PRESENT on the wire (entities-2d).
                    _raw_df = a.pop("raw_dataflags", None)
                    if isinstance(_raw_df, int):
                        df = int(_raw_df) & 0xFF
                    else:
                        df = 0x01 if _iz(_ipz) else 0
                        _ains = _nv if (_nv is not None and isinstance(_nv, list)) else None
                        if apn is None or (
                            isinstance(apn, list) and _ains is not None
                            and all(abs(float(ac) - float(ic)) < 1e-9
                                    for ac, ic in zip(apn[:2], _ains[:2]))):
                            df |= 0x02
                        if _iz(obl):
                            df |= 0x04
                        if _iz(rot):
                            df |= 0x08
                        if wf is None or (isinstance(wf, (int, float)) and abs(float(wf) - 1.0) < 1e-9):
                            df |= 0x10
                        if gen in (None, 0, "Normal"):
                            df |= 0x20
                        if ha in (None, 0, "Left"):
                            df |= 0x40
                        if va in (None, 0, "Baseline"):
                            df |= 0x80
                    rec["dataflags"] = df
                    if not (df & 0x01):
                        rec["elevation"] = normalize_float(_ipz)
                    if not (df & 0x02) and apn is not None:
                        if isinstance(apn, list) and len(apn) > 2:
                            apn = apn[:2]
                        rec["alignment_pt"] = apn
                    _AH = {"Left": 0, "Center": 1, "Right": 2, "Aligned": 3,
                           "Middle": 4, "Fit": 5}
                    _VA = {"Baseline": 0, "Bottom": 1, "Middle": 2, "Top": 3}
                    if not (df & 0x08) and rot is not None:
                        rec["rotation"] = normalize_float(rot)
                    if not (df & 0x04) and obl is not None:
                        rec["oblique_angle"] = normalize_float(obl)
                    if not (df & 0x10) and wf is not None:
                        rec["width_factor"] = normalize_float(wf)
                    if not (df & 0x20) and gen is not None:
                        rec["generation"] = gen
                    if not (df & 0x40) and ha is not None:
                        rec["horiz_alignment"] = _AH.get(str(ha), ha) if isinstance(ha, str) else ha
                    if not (df & 0x80) and va is not None:
                        rec["vert_alignment"] = _VA.get(str(va), va) if isinstance(va, str) else va
                    # flags (70): silver flags dict -> bits (ATTDEF precedent)
                    _flk = a.get("flags")
                    _fvk = 0
                    if isinstance(_flk, dict):
                        if _flk.get("invisible"): _fvk |= 1
                        if _flk.get("constant"): _fvk |= 2
                        if _flk.get("verify"): _fvk |= 4
                        if _flk.get("preset"): _fvk |= 8
                    elif isinstance(_flk, int):
                        _fvk = _flk
                    rec["flags"] = _fvk
                    # style: R2010+ wire keeps a code-0 null (gold [0,0]);
                    # earlier versions carry the real handle via the map.
                    if r2010_plus:
                        rec["style"] = [0, 0]
                    else:
                        _st_h2 = _style_handle(a.get("text_style"))
                        if _st_h2 is not None:
                            rec["style"] = normalize_handle_value(_st_h2)
                    if r2007_plus:
                        rec["lock_position_flag"] = 1 if a.get("lock_position") else 0
                    if r2010_plus:
                        rec["is_locked_in_block"] = 0
                        rec["keep_duplicate_records"] = 0
                    if r2018_plus:
                        rec["mtext_type"] = 2 if a.get("is_multiline") else 1
                    if not r2004_plus:
                        rec["prev_entity"] = normalize_handle_value(0)
                        rec["next_entity"] = normalize_handle_value(0)
                        rec["nolinks"] = 0
                    _ins_kids.append({"type": "ATTRIB", "fields": rec})
                if isinstance(_seq_h, int):
                    _sq = dict(_vcom)
                    _sq["handle"] = normalize_handle_value(_seq_h)
                    _sq["ownerhandle"] = normalize_handle_value(handle)
                    # INSERT-attached SEQENDs carry NO flag pair on any
                    # corpus era (verified ex2010 owners: INSERT 400/1878
                    # -> flags 0, while the POLY-family SEQENDs carry the
                    # era flags; see the poly _sf block).
                    if not r2004_plus:
                        _sq["prev_entity"] = normalize_handle_value(0)
                        _sq["next_entity"] = normalize_handle_value(0)
                        _sq["nolinks"] = 0
                    _ins_kids.append({"type": "SEQEND", "fields": _sq})
            # block_header (handle 0 / 330): silver stores the block NAME
            # (`block_name`); gold wants the BLOCK_RECORD handle. Resolve via
            # the block_records name->handle map built from the dump.
            bh = payload.get("block_name")
            if bh is not None:
                h = block_handle_map.get(str(bh).upper())
                if h is not None:
                    fields["block_header"] = normalize_handle_value(h)
            payload.pop("block_name", None)
            # seqend_handle is consumed by the chain block above (emitted
            # only when attributes exist); always pop it here. The retained
            # per-SEQEND common flags were stashed by the early pop below.
            payload.pop("seqend_handle", None)
            # num_cols/num_rows/col_spacing/row_spacing: R11-only in gold
            # (VERSIONS R_2_0b, R_11). Silver always emits them; drop on
            # R13+ (the R13/R14 records carry no MINSERT fields either).
            if not r2000_plus and not r13_14:
                for sk, gk in (("column_count", "num_cols"), ("row_count", "num_rows"),
                               ("column_spacing", "col_spacing"), ("row_spacing", "row_spacing")):
                    if sk in payload:
                        fields[gk] = normalize_value(payload[sk])
                        payload.pop(sk, None)
            else:
                for sk in ("column_count", "row_count", "column_spacing", "row_spacing"):
                    payload.pop(sk, None)
            # drop silver-only / DXF-only fields so they don't appear as extra
            for sk in ("dwg_minsert", "view_rep_handle", "insertion_pt",
                       "block_name", "x", "y", "z"):
                payload.pop(sk, None)

        # MTEXT entity (dwg.spec 2881, R2000b+ DWG path): silver uses snake_case
        # names; gold uses the spec names. Rename + convert + gate.
        if silver_type == "MText":
            _MT = {
                "insertion_point": "ins_pt", "insertion_pt": "ins_pt",
                "value": "text", "text_value": "text",
                "height": "text_height",
                "rectangle_width": "rect_width", "rectangle_height": "rect_height",
                "drawing_direction": "flow_dir",
                "line_spacing_factor": "linespace_factor",
                "line_spacing_style": "linespace_style",
                "attachment_point": "attachment",
                "dwg_x_direction": "x_axis_dir",
                "background_fill_flags": "bg_fill_flag",
                "background_scale": "bg_fill_scale",
                "background_color": "bg_fill_color",
                "background_transparency": "bg_fill_trans",
            }
            _MT_ENUM = {
                "flow_dir": {"ByStyle": 5, "LeftToRight": 1, "TopToBottom": 3},
                "linespace_style": {"AtLeast": 1, "Exact": 2},
                "attachment": {"TopLeft": 1, "TopCenter": 2, "TopRight": 3,
                               "MiddleLeft": 4, "MiddleCenter": 5, "MiddleRight": 6,
                               "BottomLeft": 7, "BottomCenter": 8, "BottomRight": 9},
            }
            _MT_R2000 = {"linespace_style", "linespace_factor", "unknown_b0"}
            _MT_R2007 = {"rect_height"}
            _MT_R2018 = {"column_type", "column_width", "gutter", "auto_height",
                         "flow_reversed", "num_column_heights", "column_heights",
                         "numfragments", "column_count", "width", "heights"}
            # Gold gates (dwg.spec 2881 MTEXT):
            #  - bg_fill_flag is R2004a+; bg_fill_scale/color/trans exist only
            #    when bg_fill_flag & 1 (a fill is actually enabled).
            #  - bg_fill_scale/color/trans are R2004a+ too (inside that block).
            #  - the column_* detail fields exist only when column_type != 0
            #    (R2018+); column_type itself is always emitted on R2018+.
            bg_flag = payload.get("background_fill_flags", 0) or 0
            has_bg_fill = bool(bg_flag & 1)
            col = payload.get("column_data") if isinstance(payload.get("column_data"), dict) else {}
            column_type = col.get("column_type", 0)
            for k, v in payload.items():
                if k in ("common", "handle", "owner", "owner_handle", "reactors",
                         "xdictionary_handle", "style"):
                    continue
                gk = _MT.get(k)
                if gk is None:
                # column_data nested struct -> column fields (R2018+)
                    if k == "column_data" and isinstance(v, dict):
                        # dwg.spec 2965-2974 (MTEXT columns, inside
                        # SINCE (R_2018)): gold's flat names differ from
                        # silver's nested struct — width→column_width,
                        # heights→column_heights; column_count is
                        # silver-only bookkeeping gold never emits.
                        _CGOLD = {"width": "column_width",
                                  "heights": "column_heights"}
                        for ck, cv in v.items():
                            if ck == "column_count":
                                continue
                            cgk = _CGOLD.get(ck, ck)
                            if cgk in _MT_R2018 and not r2018_plus:
                                continue
                            # Detail column fields only when column_type != 0.
                            if cgk != "column_type" and not column_type:
                                continue
                            fields[cgk] = normalize_value(cv)
                    continue
                # bg_fill_flag: R2004a+ only.
                if gk == "bg_fill_flag" and not r2004_plus:
                    continue
                # bg_fill_scale/color/trans: R2004a+ AND only when a fill is on.
                if gk in ("bg_fill_scale", "bg_fill_color", "bg_fill_trans"):
                    if not r2004_plus or not has_bg_fill:
                        continue
                if gk in _MT_R2000 and not r2000_plus:
                    continue
                if gk in _MT_R2007 and not r2007_plus:
                    continue
                if gk in _MT_R2018 and not r2018_plus:
                    continue
                if gk in _MT_ENUM:
                    m = _MT_ENUM[gk]
                    fields[gk] = m.get(str(v), v) if isinstance(v, str) else v
                else:
                    if gk == "text" and isinstance(v, str):
                        # gold decodes the ACAD \U+XXXX unicode escapes in
                        # MTEXT values (dwgread prints "108°"); silver
                        # keeps the escaped form ("108\U+00B0") — decode it.
                        try:
                            nv = re.sub(
                                r"\\U\+([0-9A-Fa-f]{4})",
                                lambda mm: chr(int(mm.group(1), 16)),
                                v)
                        except Exception:
                            nv = v
                    else:
                        nv = normalize_value(v)
                    # rect_height (R2007a+): silver stores None (default);
                    # gold emits 0.0. Coerce None -> 0.0.
                    if gk == "rect_height" and nv is None:
                        nv = 0.0
                    fields[gk] = nv
            # style handle (dwg.spec 2881, R2000b+): silver stores the
            # resolved style NAME; resolve it through silver's own
            # text-styles table to the handle gold emits.
            st = payload.pop("style", None)
            if st is not None:
                _ts = (data.get("text_styles") or {}).get("entries") or {}
                _ts_h = None
                for _e in _ts.values():
                    if isinstance(_e, dict) and _e.get("name") == st:
                        _ts_h = _e.get("handle")
                        break
                if _ts_h is not None:
                    fields["style"] = normalize_handle_value(_ts_h)
            # drop silver-only / gold-omitted fields. NOTE "value"/"
            # text_value": the FIELD_NAME_MAP re-map below would otherwise
            # overwrite the decoded `text` emission with the raw escaped
            # string ("108\U+00B0").
            for kk in ("is_annotative", "dwg_x_direction", "rotation",
                       "attachment_point", "drawing_direction", "line_spacing_factor",
                       "line_spacing_style", "column_data",
                       "height", "rectangle_width", "rectangle_height",
                       "background_scale", "background_color",
                       "background_transparency",
                       "value", "text_value"):
                payload.pop(kk, None)
            # version-gated drops (gold omits these on older versions)
            if not r2004_plus:
                for kk in ("background_fill_flags",):
                    payload.pop(kk, None)
            if not r2007_plus:
                for kk in ("background_scale", "background_color",
                           "background_transparency", "rect_height"):
                    payload.pop(kk, None)
            if not r2018_plus:
                for kk in ("column_type", "column_count", "flow_reversed",
                           "auto_height", "width", "gutter", "heights",
                           "num_column_heights", "column_heights", "numfragments",
                           # R2018-only redundant-block header BL
                           "ignore_attachment"):
                    payload.pop(kk, None)
            # bg_fill_flag is R2004a+ (FIELD_BL0 90); silver stores
            # background_fill_flags. Project it.
            if r2004_plus and "background_fill_flags" in payload:
                fields["bg_fill_flag"] = payload["background_fill_flags"]
                payload.pop("background_fill_flags", None)
            # gold omits these on R13-2017 (R2018+ only): emit only when gated
            if r2000_plus:
                fields.setdefault("unknown_b0", 0)
            if r2004_plus:
                fields.setdefault("bg_fill_flag", 0)
            # R2018+ embedded-object block (dwg.spec 2896): is_not_annotative
            # + (when set) class_version/default_flag/appid/ignore_attachment.
            # Silver doesn't store these; gold emits is_not_annotative=1 and
            # the inner block only when set. Emit the R2018+ defaults that
            # match the corpus (is_not_annotative=1 -> class_version=4,
            # default_flag=1, appid null-handle, ignore_attachment varies).
            if r2018_plus:
                fields.setdefault("is_not_annotative", 1)
                fields.setdefault("class_version", 4)
                fields.setdefault("default_flag", 1)
                fields.setdefault("appid", {"code": 5, "size": 0, "value": 0, "absref": 0})
                # ignore_attachment: gold emits 1/2/5 per file; emit 1 (the
                # dominant default) — the remaining values are a genuine
                # per-file variation, not a default.
                fields.setdefault("ignore_attachment", 1)



        # SOLID/TRACE entity (dwg.spec 2274): silver stores first_corner/
        # second_corner/third_corner/fourth_corner (3D points); gold uses
        # corner1/corner2/corner3/corner4 (2RD) plus a separate elevation
        # (FIELD_BD (elevation, 38)). Silver's reader folds the elevation
        # into every corner's z (the writer round-trips it from
        # first_corner.z) — project it back out of the corner before the 2RD
        # slice drops z. thickness matches.
        if silver_type in ("Solid", "Trace"):
            # gold types the record by its wire type code (31 SOLID vs
            # TRACE — dwg.spec _SOLID: subclass AcDbTrace for code 31 is
            # actually the TRACE; silver folds both into the Solid entity
            # and marks TRACE via is_trace). Type it accordingly or the
            # TRACE lands in gold's SOLID ordinal bucket and shifts every
            # SOLID pairing (entities-2d/3d: 24 corner rows + the
            # SOLID/TRACE count pairs on TS1).
            if payload.get("is_trace"):
                gold_type = "TRACE"
            _SOL = {"first_corner": "corner1", "second_corner": "corner2",
                    "third_corner": "corner3", "fourth_corner": "corner4"}
            _elev = None
            for sk, gk in _SOL.items():
                v = payload.get(sk)
                if v is not None:
                    if _elev is None:
                        if isinstance(v, dict):
                            _elev = v.get("z")
                        elif isinstance(v, list) and len(v) > 2:
                            _elev = v[2]
                    nv = normalize_value(v)
                    if isinstance(nv, list):
                        nv = nv[:2]  # gold 2RD
                    fields[gk] = nv
                    payload.pop(sk, None)
            if _elev is not None:
                fields["elevation"] = _elev
            # drop silver-only
            for kk in ("is_trace",):
                payload.pop(kk, None)

        # 3DFACE entity (dwg.spec 2057): silver stores first_corner/
        # second_corner/third_corner/fourth_corner (3D points); gold uses
        # corner1/corner2/corner3/corner4 (3DPOINT). invis_flags: silver stores
        # invisible_edges {bits: n}; gold emits invis_flags (BS 70) when
        # has_no_flags is false.
        if silver_type == "Face3D":
            _F3 = {"first_corner": "corner1", "second_corner": "corner2",
                   "third_corner": "corner3", "fourth_corner": "corner4"}
            # read corner1 before popping (needed for z_is_zero below)
            c1 = payload.get("first_corner")
            for sk, gk in _F3.items():
                v = payload.get(sk)
                if v is not None:
                    fields[gk] = normalize_value(v)
                    payload.pop(sk, None)
            # invis_flags: silver stores invisible_edges {bits: n}; gold emits
            # invis_flags (BS 70) only when has_no_flags is false.
            inv = payload.get("invisible_edges")
            inv_bits = None
            if isinstance(inv, dict) and "bits" in inv:
                inv_bits = inv["bits"]
                if r13_14:
                    # gold R13/R14 3DFACE: no has_no_flags bit on the wire —
                    # the decoder emits invis_flags unconditionally (0 when
                    # all corners visible) + the class meta dxfname.
                    fields["invis_flags"] = inv_bits
                    fields["dxfname"] = "3DFACE"
                # gold omits invis_flags when has_no_flags (all corners visible);
                # silver always emits it. Keep only when nonzero.
                elif inv_bits != 0:
                    fields["invis_flags"] = inv_bits
                payload.pop("invisible_edges", None)
            # has_no_flags (R2000b+): 1 when the entity has NO invis_flags (all
            # corners visible), 0 when invis_flags is present (dwg.spec 2144:
            # `if (!has_no_flags) FIELD_BS0(invis_flags)`). Silver doesn't store
            # it, so derive from the invisible_edges bits: has_no_flags = 1 iff
            # bits == 0. Verified against gold (gh109_1: bits!=0 -> 0).
            if r2000_plus:
                fields["has_no_flags"] = 1 if (inv_bits in (None, 0)) else 0
                # z_is_zero: 1 when corner1.z == 0 (the corner has no height).
                # corner1 is a 3-element list [x, y, z].
                if isinstance(c1, list) and len(c1) > 2:
                    fields["z_is_zero"] = 1 if c1[2] == 0 else 0
                elif isinstance(c1, dict):
                    fields["z_is_zero"] = 1 if c1.get("z", 0.0) == 0 else 0
                else:
                    fields["z_is_zero"] = 1
            # dxfname: gold emits the class name string (R2000b+); silver
            # doesn't store it. Emit the fixed name.
            if r2000_plus:
                fields["dxfname"] = "3DFACE"
            # drop silver-only
            for kk in ("has_no_flags", "z_is_zero", "dxfname"):
                payload.pop(kk, None)

        if silver_type == "PolyfaceMesh":
            # Gold's entity name is POLYLINE_PFACE (dwg.spec); the record
            # carries the vertex/face counts, the boundary handles and the
            # seqend — silver stores the per-vertex/per-face sub-entities
            # (emitted separately as their own stream records) nested under
            # vertices/faces.
            def _pf_handle(e):
                if isinstance(e, dict):
                    c = e.get("common") if isinstance(e.get("common"), dict) else e
                    return c.get("handle")
                return e
            vs = payload.get("vertices") or []
            fs = payload.get("faces") or []
            if isinstance(vs, list) and vs:
                fields["numverts"] = len(vs)
            if isinstance(fs, list) and fs:
                fields["numfaces"] = len(fs)
            # gold dwg.spec POLYLINE parent links: pre-R2004 wire carries
            # first/last_vertex (code 4); SINCE R_2004a it is BL num_owned
            # + the full `vertex` HANDLE_VECTOR (code 4) of every owned kid
            # — for a PFACE that is the vertex records AND the face records
            # (ex2004 gold [1253..1261]). Era-gate the two shapes.
            if r2004_plus:
                _pf_kids = [_pf_handle(x) or 0
                             for x in list(vs) + list(fs)]
                if _pf_kids:
                    fields["vertex"] = [normalize_handle_value(h)
                                        for h in _pf_kids]
            else:
                if isinstance(vs, list) and vs:
                    fields["first_vertex"] = normalize_handle_value(_pf_handle(vs[0]) or 0)
                if isinstance(fs, list) and fs:
                    fields["last_vertex"] = normalize_handle_value(_pf_handle(fs[-1]) or 0)
            # The early poly-family capture popped seqend_handle from the
            # payload — use the captured real wire handle.
            if _seq_h_parent is not None:
                fields["seqend"] = normalize_handle_value(_seq_h_parent)
            for sk in ("vertices", "faces", "seqend_handle", "flags", "normal",
                       "elevation", "extrusion", "start_width", "end_width",
                       "smooth_surface", "thickness"):
                payload.pop(sk, None)


        if silver_type == "PolygonMesh":
            # gold POLYLINE_MESH (dwg.spec POLYLINE block, the R2000 chain
            # shape): flag BS 70 — the variant marker bit, POLYGON_MESH=16
            # per gold's raw value on TS1; curve_type BS 75 — the smooth
            # type (NoSmooth=0, Quadratic=5, Cubic=6, Bezier=8, gold's
            # raw BS codes); m/n_density BS 73/74 = silver's
            # m/n_smooth_density; first/last_vertex from the nested kid
            # records' own handles (the 12-kid 3x4 grid runs 528..539);
            # seqend from the retained wire handle. The MxN vertex COUNTS,
            # elevation and extrusion are not gold parent-record fields.
            def _pm_handle(e):
                if isinstance(e, dict):
                    c = e.get("common") if isinstance(e.get("common"), dict) else e
                    return c.get("handle")
                return e
            _PM_FLAG = {"POLYGON_MESH": 16, "POLYFACE_MESH": 64,
                        "POLYLINE_2D": 0, "POLYLINE_3D": 0}
            _pmf = payload.pop("flags", 0)
            fields["flag"] = (_PM_FLAG.get(_pmf, _pmf)
                              if isinstance(_pmf, (int, str)) else 0)
            _PM_ST = {"NoSmooth": 0, "Quadratic": 5, "QuadraticBspline": 5,
                      "Cubic": 6, "CubicBspline": 6, "Bezier": 8,
                      "Spline": 6}
            _pms = payload.pop("smooth_type", 0)
            fields["curve_type"] = (_PM_ST.get(_pms, _pms)
                                     if isinstance(_pms, (int, str)) else 0)
            fields["m_density"] = payload.pop("m_smooth_density", 0)
            fields["n_density"] = payload.pop("n_smooth_density", 0)
            _pm_vs = payload.get("vertices") or []
            if isinstance(_pm_vs, list) and _pm_vs:
                fields["first_vertex"] = normalize_handle_value(
                    _pm_handle(_pm_vs[0]) or 0)
                fields["last_vertex"] = normalize_handle_value(
                    _pm_handle(_pm_vs[-1]) or 0)
            if _seq_h_parent is not None:
                fields["seqend"] = normalize_handle_value(_seq_h_parent)
            for sk in ("vertices", "m_vertex_count", "n_vertex_count",
                       "elevation", "normal", "flags", "smooth_type",
                       "m_smooth_density", "n_smooth_density"):
                payload.pop(sk, None)


        if silver_type == "Ole2Frame":
            # gold OLE2FRAME (dwg.spec 5539, R2000 wire): type BS 71
            # (Link=1/Embedded=2/Static=3), mode BS 72 (tile mode),
            # FIELD_BINARY `data` — the RAW OLE bytes as an uppercase hex
            # string (gold keeps the blob verbatim; the decoded CFB
            # re-encode is not byte-identical — the reader-capture packet
            # retains raw_data) — plus lock_aspect RC below the blob. The
            # structured envelope/storage twins and the corner points are
            # silver-side parse products with no gold counterpart.
            _olt = payload.pop("ole_object_type", 2)
            _OT = {"Link": 1, "Embedded": 2, "Static": 3}
            fields["type"] = (_OT.get(_olt, _olt)
                              if isinstance(_olt, (int, str)) else 2)
            fields["mode"] = payload.pop("dwg_mode", 0)
            _rd = payload.pop("raw_data", None)
            if isinstance(_rd, list) and _rd:
                try:
                    fields["data"] = bytes(
                        int(x) & 0xFF for x in _rd).hex().upper()
                except (TypeError, ValueError):
                    pass
            for sk in ("version", "source_application",
                       "upper_left_corner", "lower_right_corner",
                       "is_paper_space", "storage", "envelope"):
                payload.pop(sk, None)

        if silver_type == "Table":
            # gold's TABLE entity block is debug-gated (dwg2.spec 297-959,
            # §8.1.1 liveness rule): the class instances decode as raw
            # UNKNOWN_ENT — common fields + unknown_bits only. Silver parses
            # a full table; that parse has no gold counterpart (the raw
            # remainder covers it), so consume the payload per the
            # graphic_data precedent.
            for sk in ("base_style", "block_name", "block_record_handle",
                       "break_data", "break_flow_direction", "break_options",
                       "break_ranges", "break_spacing", "columns",
                       "data_version", "description",
                       "dwg_r2010_unknown_bit", "dwg_unknown_byte",
                       "dwg_unknown_handle", "dwg_unknown_long1",
                       "dwg_unknown_long2", "dwg_unknown_short",
                       "field_handles", "horizontal_direction",
                       "insertion_point", "legacy_border_colors",
                       "legacy_border_line_weights", "legacy_border_visibility",
                       "legacy_style_override", "merged_ranges", "name",
                       "normal", "override_flag", "override_border_color",
                       "override_border_line_weight",
                       "override_border_visibility", "rows",
                       "table_style_handle", "value_flags", "graphic_data"):
                payload.pop(sk, None)

        if silver_type == "Dimension":
            # DIMENSION family (dwg.spec DIMENSION_* blocks): the common
            # fields flowed from the unwrapped base above. Project the
            # dim-common block, then the per-kind points.
            kind = payload.get("_dimension_kind", "")
            dim = payload
            if r13_14:
                # R13/R14 (gold DIMENSION_COMMON): attachment/lspace_style/
                # lspace_factor/act_measurement are not on the wire —
                # silver's model defaults never compare. The common flow
                # already copied them into `fields`, so pop both.
                for sk in ("attachment", "lspace_style", "lspace_factor",
                           "act_measurement"):
                    payload.pop(sk, None)
                    fields.pop(sk, None)
            _vec = normalize_value
            def_pt_src = dim.get("definition_point")
            def_pt = _vec(def_pt_src) if def_pt_src is not None else None
            tmp_pt = _vec(dim.get("text_middle_point"))
            fields["text_midpt"] = tmp_pt[:2] if isinstance(tmp_pt, list) else tmp_pt
            if isinstance(def_pt, list) and len(def_pt) > 2:
                # gold's elevation tracks the definition point's z
                # (all corpus dims are planar).
                fields["elevation"] = def_pt[2]
            # class_version/unknown/flip_arrow1/flip_arrow2 are SINCE
            # R2007 additions in gold's DIMENSION-common; pre-2007 files
            # omit them — EXCEPT class_version, which gold reads
            # SINCE(R_2010b) only (COMMON_ENTITY_DIMENSION,
            # dwg_spec_shared.h 30: FIELD_RC class_version; 2007's
            # example file carries no gold counterpart — extra row).
            if r2007_plus:
                if r2010_plus:
                    fields["class_version"] = dim.get("version", 0)
                fields["unknown"] = 1 if dim.get("dwg_unknown_bit") else 0
                fields["flip_arrow1"] = 1 if dim.get("flip_arrow1") else 0
                fields["flip_arrow2"] = 1 if dim.get("flip_arrow2") else 0
            fb = dim.get("dwg_flags_byte", 0)
            # Gold's DIMENSION_COMMON DECODER (dwg.spec 1612-1624) derives
            # the 70-flag from the wire flag1 RC: keep flag1's own high
            # bits (0xE0), bit 7 (user-positioned text) is the INVERSE of
            # flag1 bit 0, bit 5 (use-block) comes from flag1 bit 1, then
            # the fixed dimension type's low bits are OR'd in. Verified on
            # the b6 DimConstr quads: her flag1=10 -> 0xA0=160 (the
            # DCLINEAR-placed text position sets bit 7); the old
            # special-case projection (32 for every linear, 128 hardcoded
            # on Ordinate/Diameter/Radius) dropped the bit-7 derivation.
            flag = {"Aligned": 1, "Angular2Ln": 2, "Diameter": 3, "Radius": 4,
                    "Arc": 5, "Angular3Pt": 5, "Ordinate": 6, "Linear": 0}.get(kind, 0)
            if isinstance(fb, int):
                flag |= (fb & 0xE0)
                if fb & 1:
                    flag &= 0x7F
                else:
                    flag |= 0x80
                if fb & 2:
                    flag |= 0x20
                else:
                    flag &= 0xDF
            if kind == "Ordinate":
                # Gold dwg.spec 1729 (DIMENSION_ORDINATE DECODER): the
                # per-kind flag2 RC sets bit 7 (X-type — silver's
                # is_ordinate_type_x, read as flag2 == 1) or clears bit
                # 6, applied AFTER the common flag1 derivation.
                if dim.get("is_ordinate_type_x"):
                    flag |= 0x80
                else:
                    flag &= 0xBF
            fields["flag"] = flag
            fields["flag1"] = fb
            fields["user_text"] = dim.get("text", "") or ""
            fields["text_rotation"] = _vec(dim.get("text_rotation"))
            fields["horiz_dir"] = _vec(dim.get("horizontal_direction"))
            fields["ins_scale"] = _vec(dim.get("insertion_scale"))
            fields["ins_rotation"] = _vec(dim.get("insertion_rotation"))
            # R13/R14 (gold DIMENSION_COMMON): these four are not on the
            # wire — the emissions are R2000+.
            if not r13_14:
                fields["attachment"] = {
                    "TopLeft": 1, "TopCenter": 2, "TopRight": 3,
                    "MiddleLeft": 4, "MiddleCenter": 5, "MiddleRight": 6,
                    "BottomLeft": 7, "BottomCenter": 8, "BottomRight": 9,
                }.get(dim.get("attachment_point"), 5)
                fields["lspace_style"] = dim.get("line_spacing_style", 0)
                fields["lspace_factor"] = _vec(dim.get("line_spacing_factor"))
                fields["act_measurement"] = _vec(dim.get("actual_measurement"))
            cip = _vec(dim.get("insertion_point"))
            fields["clone_ins_pt"] = cip[:2] if isinstance(cip, list) else cip
            fields["extrusion"] = _vec(dim.get("normal"))
            # dimstyle: resolve the style name through the dim-styles table
            st = dim.get("style_name")
            if st is not None:
                _ds = (data.get("dim_styles") or {}).get("entries") or {}
                for _e in _ds.values():
                    if isinstance(_e, dict) and _e.get("name") == st:
                        fields["dimstyle"] = normalize_handle_value(_e.get("handle"))
                        break
            # block: gold emits a null dict for the block-less records
            # (dwg_flags_byte bit 1) and the raw BLOCK_HEADER handle for
            # block-ful ones. Silver keeps the wire handle on
            # `block_handle` (the name-keyed table uniqifies the *D
            # blocks, so a name resolution would be ambiguous).
            if isinstance(fb, int) and (fb & 2):
                bh = dim.get("block_handle")
                if bh:
                    fields["block"] = normalize_handle_value(bh)
            else:
                fields["block"] = normalize_handle_value(0)
            dim.pop("block_handle", None)
            dim.pop("block_name", None)
            if kind in ("Aligned", "Linear"):
                fields["xline1_pt"] = _vec(dim.get("first_point"))
                fields["xline2_pt"] = _vec(dim.get("second_point"))
                if kind == "Linear":
                    fields["dim_rotation"] = _vec(dim.get("rotation"))
                fields["def_pt"] = def_pt
                fields["oblique_angle"] = _vec(dim.get("ext_line_rotation"))
            elif kind == "Ordinate":
                fields["def_pt"] = def_pt
                fields["feature_location_pt"] = _vec(dim.get("feature_location"))
                fields["leader_endpt"] = _vec(dim.get("leader_endpoint"))
                fields["flag2"] = 1 if dim.get("is_ordinate_type_x") else 0
            elif kind == "Angular2Ln":
                fields["def_pt"] = _vec(dim.get("dimension_arc"))
                fields["xline1start_pt"] = _vec(dim.get("first_point"))
                fields["xline1end_pt"] = _vec(dim.get("second_point"))
                fields["xline2start_pt"] = _vec(dim.get("angle_vertex"))
                # dwg.spec DIMENSION_ANG2LN wire order: def_pt (2RD),
                # xline1start, xline1end, xline2start, xline2end — silver
                # reads the same slots as dimension_arc, first_point,
                # second_point, angle_vertex, definition_point.
                fields["xline2end_pt"] = _vec(dim.get("definition_point"))
            elif kind == "Angular3Pt":
                fields["def_pt"] = def_pt
                fields["xline1_pt"] = _vec(dim.get("first_point"))
                fields["xline2_pt"] = _vec(dim.get("second_point"))
                fields["center_pt"] = _vec(dim.get("angle_vertex"))
            elif kind == "Arc":
                fields["def_pt"] = def_pt
                fields["xline1_pt"] = _vec(dim.get("first_extension_point"))
                fields["xline2_pt"] = _vec(dim.get("second_extension_point"))
                fields["center_pt"] = _vec(dim.get("center_point"))
                fields["is_partial"] = 1 if dim.get("is_partial") else 0
                fields["arc_start_param"] = _vec(dim.get("arc_start_parameter"))
                fields["arc_end_param"] = _vec(dim.get("arc_end_parameter"))
                fields["has_leader"] = 1 if dim.get("has_leader") else 0
                fields["leader1_pt"] = _vec(dim.get("first_leader_point"))
                fields["leader2_pt"] = _vec(dim.get("second_leader_point"))
            elif kind == "Diameter":
                fields["def_pt"] = def_pt
                fields["first_arc_pt"] = _vec(dim.get("angle_vertex"))
                fields["leader_len"] = _vec(dim.get("leader_length"))
            elif kind == "Radius":
                fields["def_pt"] = _vec(dim.get("angle_vertex"))
                fields["first_arc_pt"] = _vec(dim.get("definition_point"))
                fields["leader_len"] = _vec(dim.get("leader_length"))
            for sk in ("definition_point", "text_middle_point", "insertion_point",
                       "dimension_type", "attachment_point", "text", "user_text",
                       "normal", "text_rotation", "horizontal_direction",
                       "style_name", "actual_measurement", "version", "block_name",
                       "line_spacing_factor", "line_spacing_style", "insertion_scale",
                       "insertion_rotation", "dwg_unknown_bit", "flip_arrow1",
                       "flip_arrow2", "dwg_flags_byte", "text_user_positioned",
                       "_dimension_kind", "ext_line_rotation", "first_point",
                       "second_point", "rotation", "feature_location",
                       "leader_endpoint", "is_ordinate_type_x", "dimension_arc",
                       "angle_vertex", "is_partial", "arc_start_parameter",
                       "arc_end_parameter", "has_leader", "first_leader_point",
                       "second_leader_point", "first_extension_point",
                       "second_extension_point", "center_point", "leader_length"):
                payload.pop(sk, None)

        # POINT entity (dwg.spec 2030, R13+ DWG path): silver stores
        # `location`/`point` (a 3-vector), gold splits into x/y/z scalars
        # (BD 10/20/30). Silver `normal` -> gold `extrusion` (BE 210);
        # `x_axis_angle` -> `x_ang` (BD 50). thickness matches.
        if silver_type == "Point":
            loc = payload.get("location") or payload.get("point")
            locn = normalize_value(loc)
            if isinstance(locn, list):
                if len(locn) > 0: fields["x"] = locn[0]
                if len(locn) > 1: fields["y"] = locn[1]
                if len(locn) > 2: fields["z"] = locn[2]
            nrm = payload.get("normal")
            if nrm is not None:
                fields["extrusion"] = normalize_value(nrm)
            xaa = payload.get("x_axis_angle")
            if xaa is not None:
                fields["x_ang"] = normalize_float(xaa)
            payload.pop("location", None)
            payload.pop("point", None)
            payload.pop("normal", None)
            payload.pop("x_axis_angle", None)

        # IMAGE entity (dwg.spec 5129 DWG path): silver stores RasterImage with
        # insertion_point/u_vector/v_vector (3-vectors), size (2-vec),
        # definition_handle/definition_reactor_handle (ints), clip_boundary
        # (struct). Gold: pt0/uvec/vvec (3BD), image_size (2RD), imagedef/
        # imagedefreactor (handles), display_props (BS 70), clipping (B 280),
        # brightness/contrast/fade (RC), clip_mode (B, R2010b+),
        # clip_boundary_type (BS 71), num_clip_verts (BL 91), clip_verts (2RD
        # vector). Project + drop silver-only fields.
        if silver_type == "RasterImage":
            for sk, gk in (("insertion_point", "pt0"), ("u_vector", "uvec"),
                           ("v_vector", "vvec"), ("size", "image_size")):
                v = payload.get(sk)
                if v is not None:
                    fields[gk] = normalize_value(v)
                payload.pop(sk, None)
            # imagedef/imagedefreactor: silver stores values only (the wire
            # handle codes are not retained in the structs). Absent handles
            # keep the NULL form with code=None — the differ tolerates a
            # missing code (identity = the null target); NEVER fabricate
            # constant gold codes (the old {code:3} stamp mismatched gold's
            # raw nibble after the writer fix made the rt wire carry 3).
            if payload.get("definition_handle") is not None:
                fields["imagedef"] = normalize_handle_value(payload["definition_handle"])
            else:
                fields["imagedef"] = normalize_handle_value(0)
            if payload.get("definition_reactor_handle") is not None:
                fields["imagedefreactor"] = normalize_handle_value(payload["definition_reactor_handle"])
            else:
                fields["imagedefreactor"] = normalize_handle_value(0)
            # display_props (BS 70): silver stores a flags string
            # ("SHOW_IMAGE | SHOW_NOT_ALIGNED | ..."). Map to the gold bitmask.
            fl = payload.get("flags")
            _IMF = {"SHOW_IMAGE": 1, "SHOW_NOT_ALIGNED": 2, "USE_CLIPPING_BOUNDARY": 4,
                    "HAS_TRANSPARENT": 8, "USE_TRANSPARENT": 8, "USE_TRANSPARENT_COLOR": 8}
            if isinstance(fl, str):
                props = 0
                for tok in fl.replace("|", " ").split():
                    props |= _IMF.get(tok, 0)
                fields["display_props"] = props
            elif isinstance(fl, int):
                fields["display_props"] = fl
            fields["clipping"] = 1 if payload.get("clipping_enabled") else 0
            for sk, gk in (("brightness", "brightness"), ("contrast", "contrast"),
                           ("fade", "fade")):
                if payload.get(sk) is not None:
                    fields[gk] = normalize_value(payload[sk])
                payload.pop(sk, None)
            # clip_boundary struct -> clip_boundary_type/num_clip_verts/clip_verts.
            cb = payload.get("clip_boundary")
            if isinstance(cb, dict):
                ct = cb.get("clip_type")
                fields["clip_boundary_type"] = 2 if str(ct) == "Polygonal" else 1
                if r2010_plus:
                    cm = cb.get("clip_mode")
                    # clip_mode (FIELD_B, R2010b+): gold 0 = clip Outside, 1 =
                    # clip Inside (inverted). Silver ClipMode enum.
                    fields["clip_mode"] = 1 if str(cm) == "Inside" else 0
                verts = cb.get("vertices", [])
                if isinstance(verts, list):
                    if fields["clip_boundary_type"] == 1:
                        fields["num_clip_verts"] = 2
                    else:
                        fields["num_clip_verts"] = len(verts)
                    fields["clip_verts"] = [normalize_value(v) for v in verts]
            # drop silver-only fields
            for sk in ("definition_handle", "definition_reactor_handle", "flags",
                       "clipping_enabled", "clip_boundary", "file_path",
                       "graphic_data"):
                payload.pop(sk, None)
            # graphic_data leaks in via merge_common's _common_dwg path (the
            # IMAGE binary blob); gold doesn't expose it as a field. Drop it.
            fields.pop("graphic_data", None)

        # WIPEOUT entity (dwg2.spec 1561): same AcDbRasterImage-derived shape as
        # IMAGE. Silver stores insertion_point/u_vector/v_vector (3-vec), size
        # (2-vec), clip_boundary_vertices (flat 2-vec list), clip_type/clip_mode
        # (enums), flags (bitflags string), definition_handle. Gold: pt0/uvec/
        # vvec (3BD), image_size (2RD), imagedef/imagedefreactor (handles),
        # display_props (BS 70), clipping (B 280), brightness/contrast/fade (RC),
        # clip_mode (B, R2010b+), clip_boundary_type (BS 71), num_clip_verts
        # (BL 91), clip_verts (2RD vector).
        if silver_type == "Wipeout":
            for sk, gk in (("insertion_point", "pt0"), ("u_vector", "uvec"),
                           ("v_vector", "vvec"), ("size", "image_size")):
                v = payload.get(sk)
                if v is not None:
                    fields[gk] = normalize_value(v)
                payload.pop(sk, None)
            # Same as the IMAGE branch: values only, no fabricated codes —
            # absent handles use the tolerable NULL form (code=None).
            if payload.get("definition_handle") is not None:
                fields["imagedef"] = normalize_handle_value(payload["definition_handle"])
            else:
                fields["imagedef"] = normalize_handle_value(0)
            if payload.get("definition_reactor_handle") is not None:
                fields["imagedefreactor"] = normalize_handle_value(payload["definition_reactor_handle"])
            else:
                fields["imagedefreactor"] = normalize_handle_value(0)
            # class_version (FIELD_BL 90, R2000+): gold emits it; silver stores it.
            if r2000_plus:
                fields["class_version"] = payload.get("class_version", 0)
            fl = payload.get("flags")
            _WF = {"SHOW_IMAGE": 1, "SHOW_NOT_ALIGNED": 2, "USE_CLIPPING_BOUNDARY": 4,
                   "HAS_TRANSPARENT": 8, "USE_TRANSPARENT": 8, "USE_TRANSPARENT_COLOR": 8}
            if isinstance(fl, str):
                props = 0
                for tok in fl.replace("|", " ").split():
                    props |= _WF.get(tok, 0)
                fields["display_props"] = props
            elif isinstance(fl, int):
                fields["display_props"] = fl
            fields["clipping"] = 1 if payload.get("clipping_enabled") else 0
            for sk in ("brightness", "contrast", "fade"):
                if payload.get(sk) is not None:
                    fields[sk] = normalize_value(payload[sk])
            ct = payload.get("clip_type")
            fields["clip_boundary_type"] = 2 if str(ct) == "Polygonal" else 1
            if r2010_plus:
                cm = payload.get("clip_mode")
                fields["clip_mode"] = 1 if str(cm) == "Inside" else 0
            verts = payload.get("clip_boundary_vertices", [])
            if isinstance(verts, list):
                if fields["clip_boundary_type"] == 1:
                    fields["num_clip_verts"] = 2
                else:
                    fields["num_clip_verts"] = len(verts)
                fields["clip_verts"] = [normalize_value(v) for v in verts]
            for sk in ("definition_handle", "definition_reactor_handle", "flags",
                       "clipping_enabled", "clip_type", "clip_mode",
                       "clip_boundary_vertices", "class_version"):
                payload.pop(sk, None)
            fields.pop("graphic_data", None)

        # LEADER entity (dwg.spec 2983 DWG path): silver uses snake_case names;
        # gold uses dwg.spec names. Project + version-gate.
        if silver_type == "Leader":
            _LDR = {
                "dimension_style": None,  # gold wants the dimstyle handle, not name
                "vertices": "points", "horizontal_direction": "x_direction",
                "block_offset": "inspt_offset", "annotation_offset": "endptproj",
                "text_height": "box_height", "text_width": "box_width",
                "origin": "origin", "normal": "extrusion",
                "dwg_unknown_bit1": "unknown_bit_1",
                "dwg_unknown_bit4": "unknown_bit_4",
                "dwg_unknown_bit5": "unknown_bit_5",
            }
            for sk, gk in _LDR.items():
                v = payload.get(sk)
                if gk is not None and v is not None:
                    # endptproj is VERSIONS (R_13c3, R_2007) — INCLUDES
                    # R2007 (AC1021); absent from R2010+ wires only.
                    if gk == "endptproj" and not r2010_plus:
                        fields[gk] = normalize_value(v)
                    elif gk != "endptproj":
                        fields[gk] = normalize_value(v)
            # dimstyle: resolve silver's dimension_style NAME through the
            # dim-styles table (the DIMENSION branch precedent; gold emits
            # the handle, code 5).
            _stl = payload.pop("dimension_style", None)
            if isinstance(_stl, str):
                for _e in ((data.get("dim_styles") or {}).get("entries") or {}).values():
                    if isinstance(_e, dict) and _e.get("name") == _stl:
                        fields["dimstyle"] = normalize_handle_value(_e.get("handle"))
                        break
            # path_type: 0 straight, 1 spline
            pt = payload.get("path_type")
            if isinstance(pt, str):
                fields["path_type"] = 1 if pt == "Spline" else 0
            elif isinstance(pt, int):
                fields["path_type"] = pt
            # annot_type (73): 0 text, 1 tol, 2 insert, 3 none. Silver
            # creation_type enum.
            ct = payload.get("creation_type")
            if isinstance(ct, str):
                fields["annot_type"] = {"WithText": 0, "WithTolerance": 1,
                                        "WithBlock": 2, "None": 3,
                                        "WithoutAnnotation": 3}.get(ct, 3)
            # arrowhead_on / hookline_on / hookline_dir booleans
            if payload.get("arrow_enabled") is not None:
                fields["arrowhead_on"] = 1 if payload["arrow_enabled"] else 0
            if payload.get("hookline_enabled") is not None:
                fields["hookline_on"] = 1 if payload["hookline_enabled"] else 0
            hd = payload.get("hookline_direction")
            if isinstance(hd, str):
                fields["hookline_dir"] = 1 if hd == "Same" else 0
            # handles: associated_annotation (code 2), dimstyle (code 5)
            if payload.get("annotation_handle") is not None:
                fields["associated_annotation"] = normalize_handle_value(payload["annotation_handle"])
            if payload.get("dimension_style_handle") is not None:
                fields["dimstyle"] = normalize_handle_value(payload["dimension_style_handle"])
            # arrowhead_type (FIELD_BSx, R2000+)
            if r2000_plus and payload.get("arrowhead_type") is not None:
                fields["arrowhead_type"] = payload["arrowhead_type"]
            if r13_14:
                # gold R13/R14 LEADER: the decoder synthesizes the
                # dim-default family — dimgap/dimasz from the document's
                # current dimstyle entry, the rest the wire defaults (the
                # golden quads: Standard 0.625/2.5, arrowhead 0, the
                # unknowns 0, byblock 256).
                _ds_e = None
                _ds_name = payload.get("dimension_style")
                _dss = (data.get("dim_styles") or {}).get("entries") or {}
                for _e2 in _dss.values():
                    if isinstance(_e2, dict) and (
                        (_ds_name and _e2.get("name") == _ds_name)
                        or (not _ds_name and _e2.get("name") == "Standard")
                    ):
                        _ds_e = _e2
                        break
                fields["dimgap"] = normalize_float(
                    _ds_e.get("dimgap", 0.625) if isinstance(_ds_e, dict) else 0.625)
                fields["dimasz"] = normalize_float(
                    _ds_e.get("dimasz", 2.5) if isinstance(_ds_e, dict) else 2.5)
                fields["arrowhead_type"] = payload.get("arrowhead_type", 0) or 0
                fields["unknown_bit_2"] = payload.get("dwg_unknown_bit2", 0) or 0
                fields["unknown_bit_3"] = payload.get("dwg_unknown_bit3", 0) or 0
                fields["unknown_short_1"] = payload.get("dwg_unknown_short1", 0) or 0
                fields["byblock_color"] = payload.get("byblock_color", 256) or 256
                # endptproj (the annotation_offset rename): the R2000+ wire
                # slot is absent at this era — gold's R13 LEADERs carry no
                # end-point projection.
                payload.pop("annotation_offset", None)
                fields.pop("endptproj", None)
            # drop all silver-only keys
            for sk in list(_LDR) + ["path_type", "creation_type", "arrow_enabled",
                                    "hookline_enabled", "hookline_direction",
                                    "annotation_handle", "dimension_style_handle",
                                    "override_color", "dimension_gap", "arrow_size",
                                    "byblock_color", "dwg_unknown_bit2",
                                    "dwg_unknown_bit3", "dwg_unknown_short1",
                                    "arrowhead_type", "text_height", "text_width"]:
                payload.pop(sk, None)

        # ATTDEF entity (dwg.spec 393 DWG path): silver stores snake_case
        # (insertion_point/alignment_point/text_style/flags dict). Gold uses
        # ins_pt (2RD), alignment_pt (2RD), dataflags (bitmask of present
        # optionals), flags (70), thickness (BD0), style (handle 7), plus the
        # text-style scalars. Silver stores text_style as the resolved NAME
        # (gold has the handle) — the name is dropped; the handle comes from
        # the text-style table lookup, which the differ resolves separately.
        if silver_type == "AttributeDefinition":
            _attdef_z = 0.0
            _attdef_xy = None
            for sk, gk in (("insertion_point", "ins_pt"),):
                v = payload.get(sk)
                if v is not None:
                    nv = normalize_value(v)
                    if isinstance(nv, list) and len(nv) > 2:
                        _attdef_z = nv[2]
                        nv = nv[:2]  # ins_pt is 2RD
                    _attdef_xy = nv
                    fields[gk] = nv
                payload.pop(sk, None)
            ap = payload.get("alignment_point")
            # dataflags (dwg.spec 491, the shared ATTDEF/TEXT mask): each bit
            # means the corresponding field is ABSENT (has the default). Gold's
            # bits: 0x01 elevation, 0x02 alignment_pt, 0x04 oblique_angle,
            # 0x08 rotation, 0x10 width_factor, 0x20 generation, 0x40
            # horiz_alignment, 0x80 vert_alignment. Derive from silver's stored
            # values (default/absent -> bit set). width_factor default is 1.0.
            def _is_zero(v):
                try: return abs(float(v)) < 1e-9
                except (TypeError, ValueError): return True
            rot = payload.get("rotation")
            obl = payload.get("oblique_angle")
            wf = payload.get("width_factor")
            gen = payload.get("text_generation_flags")
            ha = payload.get("horizontal_alignment")
            va = payload.get("vertical_alignment")
            _raw_df = payload.pop("raw_dataflags", None)
            if isinstance(_raw_df, int):
                # The retained raw wire byte is the truth — which optionals
                # AutoCAD actually wrote (an explicit width_factor 1.0
                # keeps bit 4 clear where a value recomposition would set
                # it).
                df = int(_raw_df) & 0xFF
            else:
                df = 0
                # 0x01 elevation: the wire RD lives in silver's 3-D
                # insertion_point.z (2.0 on entities-2d) — absent iff zero.
                if _is_zero(_attdef_z):
                    df |= 0x01
                # 0x02 alignment_pt absent iff it EQUALS the insertion
                # point (the 2DD default) — a real all-zero [0,0]
                # alignment is PRESENT on the wire (entities-2d ATTDEF).
                _apn = normalize_value(ap)
                if _apn is None or (
                    isinstance(_apn, list) and _attdef_xy is not None
                    and all(_is_zero(float(c) - float(i))
                            for c, i in zip(_apn[:2], _attdef_xy[:2]))):
                    df |= 0x02
                if _is_zero(obl): df |= 0x04
                if _is_zero(rot): df |= 0x08
                if wf is None or abs(float(wf) - 1.0) < 1e-9: df |= 0x10
                if gen in (None, 0, "Normal"): df |= 0x20
                if ha in (None, 0, "Left"): df |= 0x40
                if va in (None, 0, "Baseline"): df |= 0x80
            fields["dataflags"] = df
            if r13_14:
                # gold R13/R14 ATTDEF: no dataflags byte on the wire —
                # the field is R2000+.
                fields.pop("dataflags", None)
            # elevation read under bit 0x01 clear (separate RD).
            if not (df & 0x01):
                fields["elevation"] = normalize_float(_attdef_z)
            # alignment_pt only when present (dataflags & 0x02 clear)
            if not (df & 0x02):
                anv = normalize_value(ap)
                if isinstance(anv, list) and len(anv) > 2:
                    anv = anv[:2]
                fields["alignment_pt"] = anv
            payload.pop("alignment_point", None)
            # thickness (BD0, default 0). Silver stores None.
            fields["thickness"] = normalize_value(payload.get("thickness") or 0)
            # flags (70): silver flags dict -> bitmask.
            fl = payload.get("flags")
            fv = 0
            if isinstance(fl, dict):
                if fl.get("invisible"): fv |= 1
                if fl.get("constant"): fv |= 2
                if fl.get("verify"): fv |= 4
                if fl.get("preset"): fv |= 8
            elif isinstance(fl, int):
                fv = fl
            fields["flags"] = fv
            # dwg.spec 568-595 (ATTDEF DWG path): lock_position_flag SINCE
            # R_2007a (CMC-era B), is_locked_in_block + keep_duplicate_records
            # SINCE R_2010b (RC with VALUEOUTOFBOUNDS coercion to <=1/<=0).
            # Silver parses lock_position (bool); the two RCs are unmodeled
            # (0 on every corpus record).
            if r2007_plus:
                fields["lock_position_flag"] = 1 if payload.get("lock_position") else 0
            if r2010_plus:
                fields["is_locked_in_block"] = 0
                fields["keep_duplicate_records"] = 0
            if r2018_plus:
                # dwg.spec ATTDEF tail SINCE R_2018b: mtext_type (1 =
                # single-line, 2 = multiline; silver's is_multiline).
                fields["mtext_type"] = 2 if payload.get("is_multiline") else 1
            payload.pop("is_multiline", None)
            # style handle (7, dwg.spec 342/599 SINCE R_13b1): gold emits the
            # text-style handle; silver stores the resolved NAME. Resolve it
            # through silver's text-styles table (the MTEXT precedent).
            _st = payload.pop("text_style", None)
            _st_h = _style_handle(_st)
            if _st_h is not None:
                fields["style"] = normalize_handle_value(_st_h)
            # Conditional text-style fields (dataflags bits: 0x01 elevation,
            # 0x04 oblique_angle, 0x08 rotation, 0x10 width_factor, 0x20
            # generation, 0x40 horiz_alignment, 0x80 vert_alignment): gold
            # emits each only when its bit is CLEAR. Emit the non-default ones.
            _ATT_HA = {"Left": 0, "Center": 1, "Right": 2, "Aligned": 3,
                       "Middle": 4, "Fit": 5}
            _ATT_VA = {"Baseline": 0, "Bottom": 1, "Middle": 2, "Top": 3}
            _ATTDEF_COND = [
                (0x01, "elevation", "elevation", None),
                (0x04, "oblique_angle", "oblique_angle", 0.0),
                (0x08, "rotation", "rotation", 0.0),
                (0x10, "width_factor", "width_factor", 1.0),
                (0x20, "generation", "text_generation_flags", 0),
                (0x40, "horiz_alignment", "horizontal_alignment", 0),
                (0x80, "vert_alignment", "vertical_alignment", 0),
            ]
            for bit, gk, sk, default in _ATTDEF_COND:
                if not (df & bit):
                    v = payload.get(sk)
                    if v is None:
                        continue
                    if gk == "horiz_alignment":
                        v = _ATT_HA.get(str(v), v) if isinstance(v, str) else v
                    elif gk == "vert_alignment":
                        v = _ATT_VA.get(str(v), v) if isinstance(v, str) else v
                    fields[gk] = normalize_value(v)
            if r13_14:
                # gold R13/R14 TEXT/ATTDEF: the wire carries no dataflags
                # optionals at this era — the decoder emits every field
                # with its default when absent (the golden Text quads:
                # elevation 0.0, alignment_pt [0,0], oblique 0.0,
                # rotation 0.0, width_factor 1.0, generation 0, the
                # alignments 0).
                fields["elevation"] = normalize_float(_attdef_z)
                anv = normalize_value(ap)
                if not isinstance(anv, list):
                    anv = [0.0, 0.0]
                fields["alignment_pt"] = anv[:2] if len(anv) > 2 else anv
                fields["oblique_angle"] = normalize_float(obl if obl is not None else 0.0)
                fields["rotation"] = normalize_float(rot if rot is not None else 0.0)
                fields["width_factor"] = normalize_float(wf if wf is not None else 1.0)
                fields["generation"] = 0 if gen in (None, 0, "Normal") else gen
                fields["horiz_alignment"] = _ATT_HA.get(str(ha), ha if isinstance(ha, int) else 0) if ha is not None else 0
                fields["vert_alignment"] = _ATT_VA.get(str(va), va if isinstance(va, int) else 0) if va is not None else 0
            # Drop silver-only text/mtext helper fields (the conditional ones
            # were either emitted above or are default -> dataflags bit set).
            for sk in ("width_factor", "oblique_angle", "text_generation_flags",
                       "horizontal_alignment", "vertical_alignment", "rotation",
                       "elevation",
                        "mtext_flag", "is_multiline", "line_count",
                        "embedded_mtext", "lock_position", "flags"):
                payload.pop(sk, None)

        # TEXT entity (dwg.spec 29 DWG path): same dataflags mechanism as
        # ATTDEF. Silver stores insertion_point/alignment_point (3-vec),
        # generation_flags/horizontal_alignment/vertical_alignment, style
        # (resolved name). Gold: ins_pt/alignment_pt (2RD), dataflags,
        # thickness, height, rotation/width_factor/oblique_angle (gated),
        # generation/horiz_alignment/vert_alignment (gated), style (handle 7).
        if silver_type == "Text":
            _txt_z = 0.0
            _txt_xy = None
            ip = payload.get("insertion_point")
            if ip is not None:
                nv = normalize_value(ip)
                if isinstance(nv, list) and len(nv) > 2:
                    _txt_z = nv[2]
                    nv = nv[:2]
                _txt_xy = nv
                fields["ins_pt"] = nv
            payload.pop("insertion_point", None)
            ap = payload.get("alignment_point")
            rot = payload.get("rotation")
            obl = payload.get("oblique_angle")
            wf = payload.get("width_factor")
            gen = payload.get("generation_flags")
            ha = payload.get("horizontal_alignment")
            va = payload.get("vertical_alignment")
            th = payload.get("thickness")
            # elevation: the wire RD lives in silver's 3-D
            # insertion_point.z (2.0 on entities-2d).
            el = _txt_z
            # dataflags (shared ATTDEF/TEXT mask, dwg.spec 491): the
            # retained raw wire byte is the truth (which optionals
            # AutoCAD wrote); compose by value only when absent (the
            # R13-14 layout) — bit0 elevation absent iff z == 0; bit1
            # alignment_pt absent iff it EQUALS the insertion point.
            _raw_df = payload.pop("raw_dataflags", None)
            apn = normalize_value(ap)
            if isinstance(_raw_df, int):
                df = int(_raw_df) & 0xFF
            else:
                df = 0
                if abs(float(el or 0.0)) < 1e-9: df |= 0x01
                if apn is None or (isinstance(apn, list) and _txt_xy is not None
                                   and all(abs(float(c) - float(i)) < 1e-9
                                           for c, i in zip(apn[:2], _txt_xy[:2]))):
                    df |= 0x02
                try: oblz = abs(float(obl)) < 1e-9
                except (TypeError, ValueError): oblz = True
                if oblz: df |= 0x04
                try: rotz = abs(float(rot)) < 1e-9
                except (TypeError, ValueError): rotz = True
                if rotz: df |= 0x08
                try: wf1 = wf is None or abs(float(wf)-1.0) < 1e-9
                except (TypeError, ValueError): wf1 = True
                if wf1: df |= 0x10
                if gen in (None, 0): df |= 0x20
                if ha in (None, 0, "Left"): df |= 0x40
                if va in (None, 0, "Baseline"): df |= 0x80
            fields["dataflags"] = df
            if r13_14:
                # gold R13/R14 TEXT: no dataflags byte on the wire — the
                # field is R2000+ (the golden Text quads).
                fields.pop("dataflags", None)
            # thickness (BD0)
            if th is not None:
                fields["thickness"] = normalize_float(th)
            # alignment_pt only when bit 0x02 clear
            if not (df & 0x02) and apn is not None:
                if isinstance(apn, list) and len(apn) > 2:
                    apn = apn[:2]
                fields["alignment_pt"] = apn
            payload.pop("alignment_point", None)
            # style handle (dwg.spec 418 SINCE R_13b1): resolve the stored name
            # through the text-styles table (the MTEXT/ATTDEF precedent).
            _st = payload.pop("style", None)
            _st_h = _style_handle(_st)
            if _st_h is not None:
                fields["style"] = normalize_handle_value(_st_h)
            # conditional fields (bit clear -> emit)
            _TXT_HA = {"Left": 0, "Center": 1, "Right": 2, "Aligned": 3,
                       "Middle": 4, "Fit": 5}
            _TXT_VA = {"Baseline": 0, "Bottom": 1, "Middle": 2, "Top": 3}
            if not (df & 0x08) and rot is not None:
                fields["rotation"] = normalize_float(rot)
            if not (df & 0x04) and obl is not None:
                fields["oblique_angle"] = normalize_float(obl)
            if not (df & 0x10) and wf is not None:
                fields["width_factor"] = normalize_float(wf)
            if not (df & 0x20) and gen is not None:
                fields["generation"] = gen
            if not (df & 0x40) and ha is not None:
                fields["horiz_alignment"] = _TXT_HA.get(str(ha), ha) if isinstance(ha, str) else ha
            if not (df & 0x80) and va is not None:
                fields["vert_alignment"] = _TXT_VA.get(str(va), va) if isinstance(va, str) else va
            if not (df & 0x01) and el is not None:
                fields["elevation"] = normalize_float(el)
            if r13_14:
                # gold R13/R14 TEXT: the wire carries no dataflags
                # optionals at this era — the decoder emits every field
                # with its default when absent (the golden Text quads).
                fields["elevation"] = normalize_float(el or 0.0)
                if apn is None:
                    apn = [0.0, 0.0]
                fields["alignment_pt"] = apn[:2] if isinstance(apn, list) and len(apn) > 2 else apn
                fields["oblique_angle"] = normalize_float(obl if obl is not None else 0.0)
                fields["rotation"] = normalize_float(rot if rot is not None else 0.0)
                fields["width_factor"] = normalize_float(wf if wf is not None else 1.0)
                fields["generation"] = 0 if gen in (None, 0, "Normal") else gen
                fields["horiz_alignment"] = _TXT_HA.get(str(ha), ha) if ha is not None else 0
                fields["vert_alignment"] = _TXT_VA.get(str(va), va) if va is not None else 0
            # normal -> extrusion
            nm = payload.get("normal")
            if nm is not None:
                fields["extrusion"] = normalize_value(nm)
            # drop consumed
            for sk in ("rotation", "width_factor", "oblique_angle",
                       "generation_flags", "horizontal_alignment",
                       "vertical_alignment", "thickness", "elevation",
                       "normal", "insertion_point", "alignment_point"):
                payload.pop(sk, None)

        if gold_type == "ARC_DIMENSION":
            # gold ARC_DIMENSION records never carry the entity-common
            # graphic_data (census 6/6 corpus records plain); silver's
            # _common_dwg adds extra rows otherwise. All silver dimensions
            # share the "Dimension" wrapper — key on the mapped gold type.
            fields.pop("graphic_data", None)

        if silver_type == "Helix":
            # gold HELIX (dwg2.spec HELIX): flat renames plus the embedded
            # spline's values. Silver nests the wire spline under `spline`
            # (its control_points list is EMPTY — silver's reader keeps the
            # knots only); gold's ctrl_pts is the degenerate REPEAT whose
            # count derives as len(knots) - degree - 1 (clamped B-spline;
            # the corpus helix: 30 knots / degree 3 -> 26 zeros). The
            # entity-common graphic_data never appears in gold's record.
            fields.pop("graphic_data", None)
            _sp = payload.pop("spline") if isinstance(payload.get("spline"), dict) else {}
            bp = payload.pop("axis_base_point", None)
            if bp is not None:
                fields["axis_base_pt"] = normalize_value(bp)
            sp_pt = payload.pop("start_point", None)
            if sp_pt is not None:
                fields["start_pt"] = normalize_value(sp_pt)
            if payload.get("maintenance_version") is not None:
                fields["maint_version"] = payload.pop("maintenance_version")
            _ct = payload.pop("constraint", None)
            fields["constraint_type"] = {"Distance": 0, "Turns": 1, "Height": 2,
                                          "TurnHeight": 3}.get(_ct, 0)
            if isinstance(_sp, dict) and _sp:
                fields["degree"] = _sp.get("degree", 0)
                # gold's HELIX records carry knotparam/splineflags ONLY
                # SINCE R2013b (dwg.spec 2588-2590 — the embedded-spline
                # R2013 additions); pre-2013 records omit BOTH (extra
                # rows otherwise) while 2013/2018 files need them.
                if r2013_plus:
                    fields["knotparam"] = _sp.get("knot_parameterization", 0)
                fields["knot_tol"] = normalize_float(_sp.get("knot_tolerance", 0.0))
                fields["ctrl_tol"] = normalize_float(_sp.get("control_tolerance", 0.0))
                _fl = _sp.get("flags") if isinstance(_sp.get("flags"), dict) else {}
                fields["closed_b"] = 1 if _fl.get("closed") else 0
                fields["periodic"] = 1 if _fl.get("periodic") else 0
                fields["rational"] = 1 if _fl.get("rational") else 0
                if r2013_plus:
                    fields["splineflags"] = ((1 if _fl.get("planar") else 0)
                                             | (2 if _fl.get("linear") else 0))
                _kn = _sp.get("knots")
                if isinstance(_kn, list) and _kn:
                    fields["knots"] = [normalize_float(x) for x in _kn]
                    _nc = len(_kn) - (_sp.get("degree") or 0) - 1
                    if _nc > 0:
                        fields["ctrl_pts"] = [0] * _nc
                fields["scenario"] = 1
                fields["weighted"] = 0

        # LIGHT entity (dwg2.spec LIGHT, live): gold never serializes
        # light_type/photometric_mode/photometric_data (not even R2018)
        # nor the entity-common graphic_data (census: gold LIGHT records
        # plain everywhere); light_color is the bare ACI index from
        # silver's {"Index": n} color shape, or the full CMC pair
        # {"index", "rgb"} from the retained raw twin (TODO B1,
        # 2026-10-01: the wire carries BOTH the legacy index slot and
        # the true-color word; the collapsed Color loses the index).
        if silver_type == "Light":
            fields.pop("graphic_data", None)
            _lcr = payload.pop("light_color_raw", None)
            if isinstance(_lcr, dict):
                _rgb = _lcr.get("rgb", 0) & 0xFFFFFFFF
                if _rgb in (None, 0):
                    # No true-color payload: gold prints the bare index.
                    fields["light_color"] = _lcr.get("index", 0)
                else:
                    _cmc = {"index": _lcr.get("index", 0),
                            "rgb": "%08x" % _rgb}
                    if _lcr.get("name") is not None:
                        _cmc["name"] = _lcr["name"]
                    if _lcr.get("book_name") is not None:
                        _cmc["book_name"] = _lcr["book_name"]
                    fields["light_color"] = _cmc
                payload.pop("light_color", None)
            else:
                _lc = payload.pop("light_color", None)
                if isinstance(_lc, dict) and isinstance(_lc.get("Index"), int):
                    fields["light_color"] = _lc["Index"]
            for _lk in ("light_type", "photometric_mode", "photometric_data"):
                payload.pop(_lk, None)

        if silver_type == "Tolerance":
            # gold TOLERANCE dimstyle (FIELD_HANDLE 5): silver keeps the
            # raw handle under dimension_style_handle; wrap it and consume
            # the name twin.
            _ds_h = payload.pop("dimension_style_handle", None)
            if isinstance(_ds_h, int) and _ds_h:
                fields["dimstyle"] = normalize_handle_value(_ds_h)
            payload.pop("dimension_style_name", None)
            # gold dwg.spec 3058 (TOLERANCE): the R2000+ wire carries
            # 3BD ins_pt, 3BD x_direction, extrusion, T text_value —
            # silver's names map 1:1 (insertion_point/direction/text,
            # values verified identical). The unknown_short/height/dimgap
            # trio exists only on VERSIONS (R_13b1, R_14) wires; silver's
            # struct carries them unconditionally — pop them on R2000+.
            _ip = payload.pop("insertion_point", None)
            if _ip is not None:
                fields["ins_pt"] = normalize_value(_ip)
            _dr = payload.pop("direction", None)
            if _dr is not None:
                fields["x_direction"] = normalize_value(_dr)
            _tx = payload.pop("text", None)
            if _tx is not None:
                fields["text_value"] = _tx
            if r2000_plus:
                payload.pop("text_height", None)
                payload.pop("dimension_gap", None)
                payload.pop("dwg_unknown_short", None)
            else:
                fields["height"] = normalize_float(
                    payload.pop("text_height", 0.0) or 0.0)
                fields["dimgap"] = normalize_float(
                    payload.pop("dimension_gap", 0.0) or 0.0)
                _us = payload.pop("dwg_unknown_short", None)
                if _us is not None:
                    fields["unknown_short"] = _us

        # MLINE entity (dwg.spec 1569 DWG path): gold emits scale/
        # justification (RC enum)/base_point/extrusion/flags (BS bits)/
        # verts/mlinestyle; silver stores snake_case names, enum strings
        # and rich vertex records. The gold `verts` field is the degenerate
        # REPEAT form — one 0 per vertex (the [0]*n JSON lesson; verified
        # against 2/4/6-vertex records on Multiline/TS1/example_*).
        if silver_type == "MLine":
            _sc = payload.get("scale_factor")
            fields["scale"] = normalize_float(_sc if _sc is not None else 1.0)
            payload.pop("scale_factor", None)
            _j = payload.pop("justification", None)
            fields["justification"] = {"Top": 0, "Zero": 1, "Bottom": 2}.get(_j, 0)
            bp = payload.pop("start_point", None)
            if bp is not None:
                fields["base_point"] = normalize_value(bp)
            _nm = payload.pop("normal", None)
            if _nm is not None:
                fields["extrusion"] = normalize_value(_nm)
            _fl = payload.pop("flags", None)
            if isinstance(_fl, str):
                # bitflags' serde joins multiple set bits with " | "
                # (e.g. "HAS_VERTICES | CLOSED" once the builder retains
                # the wire CLOSED bit) — split into single names.
                _fl = [p.strip() for p in _fl.split("|")]
            if isinstance(_fl, list):
                _fv = 0
                for _n in _fl:
                    _fv |= {"HAS_VERTICES": 1, "CLOSED": 2}.get(str(_n), 0)
                fields["flags"] = _fv
            else:
                fields["flags"] = int(_fl or 0)
            _vs = payload.pop("vertices", None)
            fields["verts"] = [0] * len(_vs) if isinstance(_vs, list) else []
            if payload.get("style_handle") is not None:
                fields["mlinestyle"] = normalize_handle_value(payload["style_handle"])
            for _sk in ("style_handle", "style_name", "style_element_count"):
                payload.pop(_sk, None)

        # VIEWPORT entity (dwg.spec 2412 DWG path): rename silver snake_case to
        # gold names, convert types, version-gate, wrap handles. The consumed
        # keys are dropped from payload so the generic loop below skips them.
        if silver_type == "Viewport":
            _VP_RENAME = {
                "view_center": "VIEWCTR", "view_direction": "VIEWDIR",
                "view_height": "VIEWSIZE", "twist_angle": "VIEWTWIST",
                "lens_length": "LENSLENGTH", "front_clip_z": "FRONTZ",
                "back_clip_z": "BACKZ", "snap_angle": "SNAPANG",
                "snap_base": "SNAPBASE", "snap_spacing": "SNAPUNIT",
                "grid_spacing": "GRIDUNIT", "ucs_origin": "ucsorg",
                "ucs_x_axis": "ucsxdir", "ucs_y_axis": "ucsydir",
                "ucs_ortho_type": "UCSORTHOVIEW", "elevation": "ucs_elevation",
                "ucs_at_origin": "ucs_at_origin", "style_sheet": "style_sheet",
                "circle_sides": "circle_zoom", "status_flag": "status_flag",
                "grid_major": "grid_major",
            }
            _VP_BOOL = {"ucs_per_viewport": "UCSVP"}
            # 2RD fields: gold stores 2 elements; silver carries a 3D point.
            _VP_2D = {"VIEWCTR", "SNAPBASE", "SNAPUNIT", "GRIDUNIT"}
            # R2000b+: view params; R2004a+: shadeplot_mode; R2007a+: lighting
            _VP_R2000 = {"VIEWCTR", "VIEWDIR", "VIEWSIZE", "VIEWTWIST", "LENSLENGTH",
                         "FRONTZ", "BACKZ", "SNAPANG", "SNAPBASE", "SNAPUNIT",
                         "GRIDUNIT", "ucsorg", "ucsxdir", "ucsydir", "ucs_elevation",
                         "UCSORTHOVIEW", "ucs_at_origin", "UCSVP", "render_mode",
                         "num_frozen_layers", "status_flag", "style_sheet"}
            _VP_R2004 = {"shadeplot_mode"}
            _VP_R2007 = {"grid_major", "use_default_lights", "default_lighting_type",
                         "brightness", "contrast", "ambient_color",
                         "background", "visualstyle", "shadeplot", "sun"}
            if r13_14:
                # gold R13/R14 VIEWPORT: no circle_zoom (R2000b+), no
                # status flag, no view_target on the wire — silver's
                # model defaults never compare.
                for _sk in ("circle_sides", "dwg_status_flag", "view_target"):
                    payload.pop(_sk, None)
            consumed = set()
            for k, v in payload.items():
                gk = _VP_RENAME.get(k) or _VP_BOOL.get(k)
                if gk is None:
                    if k in ("render_mode", "ambient_color", "status",
                             "named_ucs_handle", "base_ucs_handle", "ucs_handle",
                             "clip_boundary_handle", "visual_style_handle",
                             "sun_handle", "background_handle", "shade_plot_handle",
                             "vport_entity_header", "shade_plot_mode",
                             "use_default_lights", "default_lighting_type",
                             "brightness", "contrast", "custom_scale", "id",
                             "frozen_layers", "num_frozen_layers", "shade_plot_mode"):
                        consumed.add(k)  # handled below or dropped
                    continue
                consumed.add(k)
                if gk in _VP_R2000 and not r2000_plus:
                    continue
                if gk in _VP_R2004 and not r2004_plus:
                    continue
                if gk in _VP_R2007 and not r2007_plus:
                    continue
                if k in _VP_BOOL:
                    fields[gk] = 1 if v else 0
                elif gk in _VP_2D:
                    nv = normalize_value(v)
                    # gold 2RD: keep only the first two elements
                    if isinstance(nv, list):
                        nv = nv[:2]
                    fields[gk] = nv
                else:
                    fields[gk] = normalize_value(v)
            # status_flag: gold keeps the raw BL (R2000+, dwg.spec 2484).
            # The reader now retains the wire value verbatim on the entity
            # (dwg_status_flag) — prefer it; fall back to recomposing from
            # silver's `status` bit-struct (viewport.rs
            # ViewportStatusFlags::to_bits), which models only bits 0-15:
            #   bit0 perspective, 1 front_clipping, 2 back_clipping, 3 ucs_follow,
            #   4 front_clip_not_at_eye, 5 ucs_icon_visible, 6 ucs_icon_at_origin,
            #   7 fast_zoom, 8 snap_on, 9 grid_on, 10 isometric_snap, 11 hide_plot,
            #   12 iso_pair_top, 13 iso_pair_right, 14 locked, 15 is_on.
            _raw_sf = payload.get("dwg_status_flag")
            st = payload.get("status")
            if r2000_plus and isinstance(_raw_sf, int):
                fields["status_flag"] = _raw_sf
                consumed.add("status")
                consumed.add("status_flag")
                consumed.add("dwg_status_flag")
            elif r2000_plus and isinstance(st, dict):
                _ST_BITS = (("perspective", 0), ("front_clipping", 1),
                            ("back_clipping", 2), ("ucs_follow", 3),
                            ("front_clip_not_at_eye", 4), ("ucs_icon_visible", 5),
                            ("ucs_icon_at_origin", 6), ("fast_zoom", 7),
                            ("snap_on", 8), ("grid_on", 9), ("isometric_snap", 10),
                            ("hide_plot", 11), ("iso_pair_top", 12),
                            ("iso_pair_right", 13), ("locked", 14), ("is_on", 15))
                bits = 0
                for name, bit in _ST_BITS:
                    if st.get(name):
                        bits |= (1 << bit)
                fields["status_flag"] = bits
                consumed.add("status")
                consumed.add("status_flag")
            # render_mode string -> int (R2000+)
            if r2000_plus:
                rm = payload.get("render_mode")
                if isinstance(rm, str):
                    _RM = {"Wireframe2D": 0, "Wireframe3D": 1, "HiddenLine": 2,
                           "FlatShaded": 3, "GouraudShaded": 4,
                           "FlatShadedWithEdges": 5, "GouraudShadedWithEdges": 6}
                    fields["render_mode"] = _RM.get(rm, 0)
                elif rm is not None:
                    fields["render_mode"] = rm
                consumed.add("render_mode")
                fields["num_frozen_layers"] = len(payload.get("frozen_layers") or [])
                consumed.add("frozen_layers")
            # handle wraps (version-gated)
            def _wh(src, dst, ok):
                if ok:
                    fields[dst] = normalize_handle_value(payload.get(src))
                consumed.add(src)
            _wh("base_ucs_handle", "base_ucs", r2000_plus)
            _wh("clip_boundary_handle", "clip_boundary", r2000_plus)
            _wh("visual_style_handle", "visualstyle", r2007_plus)
            _wh("sun_handle", "sun", r2007_plus)
            _wh("background_handle", "background", r2007_plus)
            _wh("shade_plot_handle", "shadeplot", r2007_plus)
            # vport_entity_header (pre-R2004, dwg.spec VIEWPORT): the
            # reader retains the wire's own FIELD_HANDLE (vport_entity_
            # header, 5, 0) after the clip boundary (R13-14: before the
            # frozen layers) — emit it verbatim, including the null form.
            # consume the raw model key on every era (it would otherwise
            # leak through the generic loop on R2004+).
            if not r2004_plus:
                fields["vport_entity_header"] = normalize_handle_value(
                    payload.get("vport_entity_handle") or 0)
            consumed.add("vport_entity_header")
            consumed.add("vport_entity_handle")
            # named_ucs (gold HANDLE 5, SINCE R_2000b): silver stores the
            # viewport's current UCS under `ucs_handle` — same wire field;
            # the old code read a nonexistent `named_ucs_handle` and every
            # record emitted None (probes c2/multiline era).
            _wh("ucs_handle", "named_ucs", r2000_plus)
            # R2004+: shadeplot_mode
            if r2004_plus:
                fields["shadeplot_mode"] = payload.get("shade_plot_mode", 0)
                consumed.add("shade_plot_mode")
            # R2007+: lighting
            if r2007_plus:
                fields["use_default_lights"] = 1 if payload.get("use_default_lights", payload.get("default_lighting")) else 0
                fields["default_lighting_type"] = payload.get("default_lighting_type", 1)
                fields["brightness"] = payload.get("brightness", 0.0)
                fields["contrast"] = payload.get("contrast", 0.0)
                ac = payload.get("ambient_color")
                if isinstance(ac, dict) and "Rgb" in ac:
                    rgb = ac["Rgb"]
                    fields["ambient_color"] = {"index": 250,
                        "rgb": "c2%02x%02x%02x%02x" % (0, rgb.get("r", 0), rgb.get("g", 0), rgb.get("b", 0))}
                elif isinstance(ac, str):
                    fields["ambient_color"] = normalize_value(ac)
                for kk in ("use_default_lights", "default_lighting", "default_lighting_type",
                           "brightness", "contrast", "ambient_color"):
                    consumed.add(kk)
            # drop silver-only / DXF-only fields so they don't appear as extra
            for sk in ("custom_scale", "id", "on_off",
                       # top-level silver-only status/UI bits and pre-2007 fields
                       # gold omits on the binary-DWG path for this version:
                       "ucs_icon_visible", "grid_flags", "default_lighting",
                       "shade_plot_mode", "frozen_layers"):
                payload.pop(sk, None)
            for sk in consumed:
                payload.pop(sk, None)

        # HATCH entity (dwg.spec 4637 DWG path): silver stores
        # pattern/is_solid/style/pattern_type/pattern_angle/pattern_scale/
        # is_double/seed_points/elevation/normal + gradient_color (struct).
        # Gold: name/is_solid_fill/is_associative/style/pattern_type/angle/
        # scale_spacing/double_flag/paths/seeds/pixel_size/has_derived +
        # gradient_* (R2004+). Project the scalars + nested gradient.
        if silver_type == "Hatch":
            pat = payload.get("pattern")
            if isinstance(pat, dict):
                fields["name"] = pat.get("name", "")
            fields["is_solid_fill"] = 1 if payload.get("is_solid") else 0
            is_solid_fill = bool(payload.get("is_solid"))
            if payload.get("is_associative") is not None:
                fields["is_associative"] = 1 if payload["is_associative"] else 0
            pt = payload.get("pattern_type")
            if isinstance(pt, str):
                fields["pattern_type"] = {"UserDefined": 0, "Predefined": 1, "Custom": 2}.get(pt, 0)
            elif isinstance(pt, int):
                fields["pattern_type"] = pt
            st = payload.get("style")
            if isinstance(st, str):
                fields["style"] = {"Normal": 0, "Outer": 1, "Ignore": 2}.get(st, 0)
            elif isinstance(st, int):
                fields["style"] = st
            # angle/scale_spacing/double_flag: only when NOT solid fill
            # (dwg.spec 4665 `if (!is_solid_fill)`).
            if not is_solid_fill:
                if payload.get("pattern_angle") is not None:
                    fields["angle"] = normalize_float(payload["pattern_angle"])
                if payload.get("pattern_scale") is not None:
                    fields["scale_spacing"] = normalize_float(payload["pattern_scale"])
                fields["double_flag"] = 1 if payload.get("is_double") else 0
            fields["elevation"] = normalize_float(payload.get("elevation", 0.0))
            nm = payload.get("normal")
            if nm is not None:
                fields["extrusion"] = normalize_value(nm)
            # seed_points -> seeds (2RD); num_seeds
            sp = payload.get("seed_points", [])
            if isinstance(sp, list):
                fields["num_seeds"] = len(sp)
                fields["seeds"] = [normalize_value(p) for p in sp]
            # paths: count only (the full segment projection is a separate
            # packet); gold emits num_paths.
            paths = payload.get("paths", [])
            fields["num_paths"] = len(paths) if isinstance(paths, list) else 0
            # gold `paths` (dwg.spec HATCH REPEAT): the degenerate form —
            # a bare 0 per path (probes HatchG: 1 path -> [0]).
            if isinstance(paths, list) and paths:
                fields["paths"] = [0] * len(paths)
            # gold `deflines`: degenerate 0 per pattern line, emitted when
            # the pattern carries lines (gold ANSI31 -> [0]; Dynblocks 12).
            if isinstance(pat, dict) and isinstance(pat.get("lines"), list) \
                    and pat["lines"]:
                fields["deflines"] = [0] * len(pat["lines"])
            # gold `colors` (dwg.spec _HATCH_gradientfill REPEAT): the
            # degenerate 0-per-gradient-color form (probes HatchG [0,0]).
            gc = payload.get("gradient_color")
            if isinstance(gc, dict):
                gcc = gc.get("colors")
                if isinstance(gcc, list) and gcc:
                    fields["colors"] = [0] * len(gcc)
            # has_derived (dwg.spec 4880, JSON-only): gold derives it from the
            # path flag bits (any path.flag & 0x4). Silver stores paths with a
            # `flag`/`flags` int per path.
            has_derived = 0
            if isinstance(paths, list):
                for p in paths:
                    if isinstance(p, dict):
                        pf = p.get("flag", p.get("flags", 0))
                        if isinstance(pf, dict):
                            pf = pf.get("bits", 0)
                        if isinstance(pf, int) and (pf & 0x4):
                            has_derived = 1
                            break
            fields["has_derived"] = has_derived
            # pixel_size (BD 47): only when has_derived (dwg.spec 4881).
            if has_derived and payload.get("pixel_size") is not None:
                fields["pixel_size"] = normalize_float(payload["pixel_size"])
            # gradient (R2004+): silver gradient_color struct -> gradient_*.
            gc = payload.get("gradient_color")
            if isinstance(gc, dict) and r2004_plus:
                fields["is_gradient_fill"] = 1 if gc.get("enabled") else 0
                fields["reserved"] = gc.get("reserved", 0)
                fields["gradient_angle"] = normalize_float(gc.get("angle", 0.0))
                fields["gradient_shift"] = normalize_float(gc.get("shift", 0.0))
                fields["single_color_gradient"] = 1 if gc.get("is_single_color") else 0
                fields["gradient_tint"] = normalize_float(gc.get("color_tint", 0.0))
                fields["gradient_name"] = gc.get("name", "")
                cols = gc.get("colors", [])
                fields["num_colors"] = len(cols) if isinstance(cols, list) else 0
            # drop all consumed + silver-only keys
            for sk in ("pattern", "is_solid", "is_associative", "pattern_type",
                       "pattern_angle", "pattern_scale", "is_double", "style",
                       "seed_points", "paths", "pixel_size", "gradient_color",
                       "elevation", "normal", "is_mpolygon", "mpolygon_hatch_color",
                       "mpolygon_x_direction", "mpolygon_boundary_handle_count"):
                payload.pop(sk, None)

        # SPLINE entity (dwg.spec 2571 DWG path): silver stores degree/flags
        # dict/knots/control_points/fit_points/tolerances/tangents. Gold:
        # scenario (1 = fit/knotparam==15, 2 = bezier), splineflags+knotparam
        # (R2013+), degree, rational/closed_b/periodic/weighted (bits, scenario
        # 1 only), knot_tol/ctrl_tol, num_knots/num_ctrl_pts/num_fit_pts,
        # knots/ctrl_pts/fit_pts. The ctrl_pts REPEAT is degenerate in gold's
        # decode (zeros), so emit the count shape.
        if silver_type == "Spline":
            fl = payload.get("flags")
            closed_b = bool(fl.get("closed")) if isinstance(fl, dict) else False
            periodic = bool(fl.get("periodic")) if isinstance(fl, dict) else False
            rational = bool(fl.get("rational")) if isinstance(fl, dict) else False
            weighted = bool(fl.get("weighted")) if isinstance(fl, dict) else False
            kp = payload.get("knot_parameterization")
            # Gold dwg.spec 2571 (SPLINE): the scenario BL is read on every
            # version and STANDS pre-R2013 (the UNTIL(R_2013) decoder only
            # validates 1/2); the R2013b+ decoder re-derives it —
            # splineflags bit 0 -> 2, knotparam 15 -> 1. Silver's reader
            # retains the wire BL as dwg_wire_scenario (34c75d0); project
            # from it, then apply the R2013b overrides. The old
            # unconditional `1 if kp == 15 else 2` mis-scenario'd every
            # pre-2013 CV-form spline as 2 (the b6 ConstrSmooth_2010
            # specimen: wire scenario 1, gold 1, old projection 2 with the
            # whole knots/ctrl_pts block dropped).
            ws = _spline_wire_scenario
            scenario = ws if isinstance(ws, int) else (1 if kp == 15 else 2)
            if r2013_plus:
                df1 = payload.get("dwg_flags1")
                if isinstance(df1, int) and (df1 & 1):
                    scenario = 2
                if isinstance(kp, int) and kp == 15:
                    scenario = 1
            fields["scenario"] = scenario
            # knotparam + splineflags are R2013+ only (dwg.spec 2588-2590).
            if r2013_plus:
                if kp is not None:
                    fields["knotparam"] = kp
                df1 = payload.get("dwg_flags1")
                fields["splineflags"] = df1 if isinstance(df1, int) else 0
            fields["degree"] = payload.get("degree", 3)
            # The rational/closed/periodic/weighted/tolerances block is read
            # only for scenario 1 (dwg.spec 2605 `if (scenario & 1)`).
            if scenario == 1:
                fields["rational"] = 1 if rational else 0
                fields["closed_b"] = 1 if closed_b else 0
                fields["periodic"] = 1 if periodic else 0
                fields["weighted"] = 1 if weighted else 0
                if payload.get("knot_tolerance") is not None:
                    fields["knot_tol"] = normalize_float(payload["knot_tolerance"])
                if payload.get("control_tolerance") is not None:
                    fields["ctrl_tol"] = normalize_float(payload["control_tolerance"])
            knots = payload.get("knots", [])
            cps = payload.get("control_points", [])
            fps = payload.get("fit_points", [])
            fields["num_knots"] = len(knots) if isinstance(knots, list) else 0
            fields["num_ctrl_pts"] = len(cps) if isinstance(cps, list) else 0
            fields["num_fit_pts"] = len(fps) if isinstance(fps, list) else 0
            # knots/ctrl_pts are scenario-1 (spline) only on the DWG path;
            # fit_pts is scenario-2 (bezier) only.
            if scenario == 1:
                fields["knots"] = [normalize_float(k) for k in knots] if isinstance(knots, list) else []
                # gold's ctrl_pts = the degenerate REPEAT: normalize_gold
                # collapses every {x,y,z} control-point dict to 0, so the
                # normalized gold value is [0] * num_ctrl_pts (16 for the
                # Dynblocks car-splines) — NOT the flat x/y/z triple
                # expansion (that was 48 vs gold's 16).
                fields["ctrl_pts"] = [0] * len(cps) if isinstance(cps, list) else []
            else:
                fields["fit_pts"] = [normalize_value(p) for p in fps] if isinstance(fps, list) else []
                if payload.get("fit_tolerance") is not None:
                    fields["fit_tol"] = normalize_float(payload["fit_tolerance"])
            # tangents (3BD): only for scenario 2 (bezier) on the DWG path.
            if scenario == 2:
                bt = payload.get("begin_tangent")
                if bt is not None:
                    fields["beg_tan_vec"] = normalize_value(bt)
                et = payload.get("end_tangent")
                if et is not None:
                    fields["end_tan_vec"] = normalize_value(et)
            # extrusion is DXF-only for SPLINE (dwg.spec 2596 `DXF {}`); do NOT
            # emit it on the binary-DWG path.
            for sk in ("flags", "knot_parameterization", "dwg_flags1", "dxf_flags",
                       "degree", "knots", "control_points", "fit_points", "weights",
                       "knot_tolerance", "control_tolerance", "fit_tolerance",
                       "begin_tangent", "end_tangent", "normal", "extrusion",
                       "cv_frame_visible"):
                payload.pop(sk, None)

        if silver_type == "MultiLeader":
            # Entity-common graphic_data: silver's _common_dwg map carries
            # the wire preview bytes for these records but gold's MULTILEADER
            # decode never emits them (census: 8/8 corpus records plain) —
            # dropping the field beat leaving extra_in_silver rows.
            fields.pop("graphic_data", None)
            # R2010+ wire bit-group captured between the walked spec tail
            # and the string-stream anchor (writer-only fidelity channel;
            # gold's spec has no counterpart and dumps it into unknown_bits).
            fields.pop("dwg_raw_tail_bits", None)
            # MULTILEADER (gold dwg2.spec 1298 + MLEADER_CONTEXT_DATA_fields
            # macro at dwg2.spec 1227; silver: src/entities/multileader.rs,
            # readers/entities.rs::read_multileader and
            # read_multileader_annotation_context). Silver keeps the full
            # MultiLeaderAnnotContext under `context` plus flat own-named
            # fields; project everything to the gold shape, version-gated
            # per the spec: R14-R2007 carries arrowheads/blocklabels and the
            # neg/ipe/just/scale tail, SINCE R_2010b class_version, attach_dir
            # and ctx.text_top/bottom, SINCE R_2013b is_text_extended.
            # Residuals (kept): gold-only `unknown_bits` (silver stores no
            # raw remainder). gold's attach_dir/attach_top/attach_bottom BS
            # raws (dwg2.spec 1449-1451) are retained by silver as the
            # dwg_attach_* fields since the 2026-09-20 tenth batch and are
            # projected below.
            _ML_ATTACH = {
                "TopOfTopLine": 0, "MiddleOfTopLine": 1, "MiddleOfText": 2,
                "MiddleOfBottomLine": 3, "BottomOfBottomLine": 4, "BottomLine": 5,
                "BottomOfTopLineUnderlineBottomLine": 6,
                "BottomOfTopLineUnderlineTopLine": 7,
                "BottomOfTopLineUnderlineAll": 8, "CenterOfText": 9,
                "CenterOfTextOverline": 10,
            }

            def _ml_enum(value, table):
                return table.get(value, value) if isinstance(value, str) else value

            def _ml_flags(value):
                # MultiLeaderPropertyOverrideFlags (multileader.rs 291):
                # serde prints "NAME | NAME"; rebuild the u32 via the bit table.
                if isinstance(value, int):
                    return value
                total = 0
                if isinstance(value, str):
                    for part in (p.strip() for p in value.split("|")):
                        total |= {
                            "PATH_TYPE": 0x1, "LINE_COLOR": 0x2,
                            "LEADER_LINE_TYPE": 0x4, "LEADER_LINE_WEIGHT": 0x8,
                            "ENABLE_LANDING": 0x10, "LANDING_GAP": 0x20,
                            "ENABLE_DOGLEG": 0x40, "LANDING_DISTANCE": 0x80,
                            "ARROWHEAD": 0x100, "ARROWHEAD_SIZE": 0x200,
                            "CONTENT_TYPE": 0x400, "TEXT_STYLE": 0x800,
                            "TEXT_LEFT_ATTACHMENT": 0x1000, "TEXT_ANGLE": 0x2000,
                            "TEXT_ALIGNMENT": 0x4000, "TEXT_COLOR": 0x8000,
                            "TEXT_HEIGHT": 0x10000, "TEXT_FRAME": 0x20000,
                            "ENABLE_USE_DEFAULT_MTEXT": 0x40000,
                            "BLOCK_CONTENT": 0x80000,
                            "BLOCK_CONTENT_COLOR": 0x100000,
                            "BLOCK_CONTENT_SCALE": 0x200000,
                            "BLOCK_CONTENT_ROTATION": 0x400000,
                            "BLOCK_CONTENT_CONNECTION": 0x800000,
                            "SCALE_FACTOR": 0x1000000,
                            "TEXT_RIGHT_ATTACHMENT": 0x2000000,
                            "TEXT_SWITCH_ALIGNMENT_TYPE": 0x4000000,
                            "TEXT_ATTACHMENT_DIRECTION": 0x8000000,
                            "TEXT_TOP_ATTACHMENT": 0x10000000,
                            "TEXT_BOTTOM_ATTACHMENT": 0x20000000,
                        }.get(part, 0)
                return total

            def _ml_linewt(value):
                # line_linewt is BLd (raw DXF-style codes: -1 ByLayer,
                # -2 ByBlock, -3 default), NOT the common-entity linewt RC
                # index used by _lineweight_to_gold.
                if isinstance(value, int):
                    return value
                if value == "ByLayer":
                    return -1
                if value == "ByBlock":
                    return -2
                if value == "Default":
                    return -3
                return 0

            ctx = payload.get("context") if isinstance(payload.get("context"), dict) else {}

            # -- entity fields, all versions (dwg2.spec 1395-1447) --
            fields["mleaderstyle"] = normalize_handle_value(payload.get("style_handle") or 0)
            fields["flags"] = _ml_flags(payload.get("property_override_flags"))
            fields["line_color"] = normalize_color(payload.get("line_color"))
            fields["line_ltype"] = normalize_handle_value(payload.get("line_type_handle") or 0)
            fields["line_linewt"] = _ml_linewt(payload.get("line_weight"))
            fields["has_landing"] = 1 if payload.get("enable_landing") else 0
            fields["has_dogleg"] = 1 if payload.get("enable_dogleg") else 0
            fields["landing_dist"] = normalize_float(payload.get("dogleg_length", 0.0))
            fields["arrow_handle"] = normalize_handle_value(payload.get("arrowhead_handle") or 0)
            fields["arrow_size"] = normalize_float(payload.get("arrowhead_size", 0.0))
            fields["style_content"] = _ml_enum(
                payload.get("content_type"),
                {"None": 0, "Block": 1, "MText": 2, "Tolerance": 3})
            fields["text_style"] = normalize_handle_value(payload.get("text_style_handle") or 0)
            fields["text_left"] = _ml_enum(payload.get("text_left_attachment"), _ML_ATTACH)
            fields["text_right"] = _ml_enum(payload.get("text_right_attachment"), _ML_ATTACH)
            fields["text_angletype"] = _ml_enum(
                payload.get("text_angle_type"),
                {"ParallelToLastLeaderLine": 0, "Horizontal": 1, "Optimized": 2})
            fields["text_alignment"] = _ml_enum(
                payload.get("text_alignment"), {"Left": 0, "Center": 1, "Right": 2})
            fields["text_color"] = normalize_color(payload.get("text_color"))
            fields["has_text_frame"] = 1 if payload.get("text_frame") else 0
            fields["block_style"] = normalize_handle_value(payload.get("block_content_handle") or 0)
            fields["block_color"] = normalize_color(payload.get("block_content_color"))
            fields["block_scale"] = normalize_value(payload.get("block_scale"))
            fields["block_rotation"] = normalize_float(payload.get("block_rotation", 0.0))
            # Slot naming: gold's BS after block_rotation is `style_attachment`
            # (dwg2.spec 1446); silver's reader stores the same wire slot under
            # `block_connection_type` (entities.rs:4495).
            fields["style_attachment"] = _ml_enum(
                payload.get("block_connection_type"), {"BlockExtents": 0, "BasePoint": 1})
            fields["is_annotative"] = 1 if payload.get("enable_annotation_scale") else 0

            # -- VERSIONS (R_14, R_2007): arrowheads, blocklabels, tail --
            if not r2010_plus:
                ahs = payload.get("arrowhead_overrides") or []
                if isinstance(ahs, list) and ahs:
                    fields["num_arrowheads"] = len(ahs)
                    fields["arrowheads"] = [
                        {"is_default": 1 if a.get("is_default") else 0,
                         "arrowhead": normalize_handle_value(a.get("arrowhead_handle") or 0)}
                        for a in ahs if isinstance(a, dict)]
                bas = payload.get("block_attributes") or []
                if isinstance(bas, list) and bas:
                    fields["num_blocklabels"] = len(bas)
                    fields["blocklabels"] = [
                        {"attdef": normalize_handle_value(b.get("attribute_definition_handle") or 0),
                         "label_text": b.get("text", ""),
                         "ui_index": b.get("index", 0),
                         "width": normalize_float(b.get("width", 0.0))}
                        for b in bas if isinstance(b, dict)]
                fields["is_neg_textdir"] = 1 if payload.get("text_direction_negative") else 0
                fields["ipe_alignment"] = payload.get("text_align_in_ipe", 0)
                # Gold's BS `justification` (179): silver's reader stores the
                # slot as text_attachment_point (TextAttachmentPointType).
                fields["justification"] = _ml_enum(
                    payload.get("text_attachment_point"),
                    {"Left": 1, "Center": 2, "Right": 3})
                fields["scale_factor"] = normalize_float(payload.get("scale_factor", 1.0))

            # -- SINCE (R_2010b) / SINCE (R_2013b) --
            if r2010_plus:
                fields["class_version"] = payload.get("dwg_version", 2)
                # dwg2.spec 1449-1451: attach_dir/top/bottom are raw wire
                # BS values (271/273/272) outside silver's typed enums —
                # the reader retains them as dwg_attach_*; emit verbatim.
                fields["attach_dir"] = payload.get("dwg_attach_dir", 0)
                fields["attach_top"] = payload.get("dwg_attach_top", 0)
                fields["attach_bottom"] = payload.get("dwg_attach_bottom", 0)
            if r2013_plus:
                fields["is_text_extended"] = 1 if payload.get("extend_leader_to_text") else 0

            # -- ctx: MLEADER_CONTEXT_DATA_fields (dwg2.spec 1227) --
            roots = ctx.get("leader_roots") or []
            num_leaders = len(roots) if isinstance(roots, list) else 0
            fields["ctx.num_leaders"] = num_leaders
            # Gold's REPEAT JSON for ctx.leaders is degenerate ([0]*count —
            # the same libredwg emission class as MLINESTYLE.lines); silver
            # stores the real leader-root structs. Project the count-faithful
            # degenerate form so the differ sees the same array.
            fields["ctx.leaders"] = [0] * num_leaders
            fields["ctx.scale_factor"] = normalize_float(ctx.get("scale_factor"))
            fields["ctx.content_base"] = normalize_value(ctx.get("content_base_point"))
            fields["ctx.text_height"] = normalize_float(ctx.get("text_height"))
            fields["ctx.arrow_size"] = normalize_float(ctx.get("arrowhead_size"))
            fields["ctx.landing_gap"] = normalize_float(ctx.get("landing_gap"))
            fields["ctx.text_left"] = _ml_enum(ctx.get("text_left_attachment"), _ML_ATTACH)
            fields["ctx.text_right"] = _ml_enum(ctx.get("text_right_attachment"), _ML_ATTACH)
            # Slot shift (entities.rs:4683-4686): the wire BS pair is gold's
            # [ctx.text_angletype, ctx.text_alignment] (dwg2.spec 1234-1235);
            # silver stores them shifted as [text_alignment, block_connection_type].
            fields["ctx.text_angletype"] = _ml_enum(
                ctx.get("text_alignment"), {"Left": 0, "Center": 1, "Right": 2})
            fields["ctx.text_alignment"] = _ml_enum(
                ctx.get("block_connection_type"), {"BlockExtents": 0, "BasePoint": 1})
            has_txt = bool(ctx.get("has_text_contents"))
            fields["ctx.has_content_txt"] = 1 if has_txt else 0
            if has_txt:
                fields["ctx.content.txt.default_text"] = ctx.get("text_string", "")
                fields["ctx.content.txt.normal"] = normalize_value(ctx.get("text_normal"))
                fields["ctx.content.txt.style"] = normalize_handle_value(ctx.get("text_style_handle") or 0)
                fields["ctx.content.txt.location"] = normalize_value(ctx.get("text_location"))
                fields["ctx.content.txt.direction"] = normalize_value(ctx.get("text_direction"))
                fields["ctx.content.txt.rotation"] = normalize_float(ctx.get("text_rotation"))
                fields["ctx.content.txt.width"] = normalize_float(ctx.get("text_width"))
                fields["ctx.content.txt.height"] = normalize_float(ctx.get("text_boundary_height"))
                fields["ctx.content.txt.line_spacing_factor"] = normalize_float(
                    ctx.get("line_spacing_factor"))
                fields["ctx.content.txt.line_spacing_style"] = _ml_enum(
                    ctx.get("line_spacing_style"), {"AtLeast": 1, "Exactly": 2})
                fields["ctx.content.txt.color"] = normalize_color(ctx.get("text_color"))
                # silver's ctx.text_attachment_point reads gold's
                # ctx.content.txt.alignment BS slot (entities.rs:4711).
                fields["ctx.content.txt.alignment"] = _ml_enum(
                    ctx.get("text_attachment_point"),
                    {"Left": 1, "Center": 2, "Right": 3})
                fields["ctx.content.txt.flow"] = _ml_enum(
                    ctx.get("text_flow_direction"),
                    {"Horizontal": 1, "Vertical": 3, "ByStyle": 5})
                fields["ctx.content.txt.bg_color"] = normalize_color(ctx.get("background_fill_color"))
                fields["ctx.content.txt.bg_scale"] = normalize_float(ctx.get("background_scale_factor"))
                fields["ctx.content.txt.bg_transparency"] = ctx.get("background_transparency", 0)
                fields["ctx.content.txt.is_bg_fill"] = 1 if ctx.get("background_fill_enabled") else 0
                fields["ctx.content.txt.is_bg_mask_fill"] = 1 if ctx.get("background_mask_fill_on") else 0
                fields["ctx.content.txt.col_type"] = ctx.get("column_type", 0)
                fields["ctx.content.txt.is_height_auto"] = 1 if ctx.get("text_height_automatic") else 0
                fields["ctx.content.txt.col_width"] = normalize_float(ctx.get("column_width"))
                fields["ctx.content.txt.col_gutter"] = normalize_float(ctx.get("column_gutter"))
                fields["ctx.content.txt.is_col_flow_reversed"] = 1 if ctx.get("column_flow_reversed") else 0
                cols = ctx.get("column_sizes") or []
                fields["ctx.content.txt.num_col_sizes"] = len(cols) if isinstance(cols, list) else 0
                fields["ctx.content.txt.col_sizes"] = (
                    [normalize_float(c) for c in cols] if isinstance(cols, list) else [])
                fields["ctx.content.txt.word_break"] = 1 if ctx.get("word_break") else 0
                fields["ctx.content.txt.unknown"] = 1 if ctx.get("dwg_unknown_text_bit") else 0
            else:
                fields["ctx.has_content_blk"] = 1 if ctx.get("has_block_contents") else 0
                if ctx.get("has_block_contents"):
                    fields["ctx.content.blk.block_table"] = normalize_handle_value(
                        ctx.get("block_content_handle") or 0)
                    fields["ctx.content.blk.normal"] = normalize_value(ctx.get("block_content_normal"))
                    fields["ctx.content.blk.location"] = normalize_value(ctx.get("block_content_location"))
                    fields["ctx.content.blk.scale"] = normalize_value(ctx.get("block_content_scale"))
                    fields["ctx.content.blk.rotation"] = normalize_float(ctx.get("block_rotation", 0.0))
                    fields["ctx.content.blk.color"] = normalize_color(ctx.get("block_content_color"))
                    tm = ctx.get("transform_matrix") or []
                    fields["ctx.content.blk.transform"] = (
                        [normalize_float(t) for t in tm] if isinstance(tm, list) else [])
            fields["ctx.base"] = normalize_value(ctx.get("base_point"))
            fields["ctx.base_dir"] = normalize_value(ctx.get("base_direction"))
            fields["ctx.base_vert"] = normalize_value(ctx.get("base_vertical"))
            fields["ctx.is_normal_reversed"] = 1 if ctx.get("normal_reversed") else 0
            if r2010_plus:
                fields["ctx.text_top"] = _ml_enum(ctx.get("text_top_attachment"), _ML_ATTACH)
                fields["ctx.text_bottom"] = _ml_enum(ctx.get("text_bottom_attachment"), _ML_ATTACH)

            # Consume the silver shape so only `common` remains for the
            # generic loop. path_type has no gold counterpart (gold's BS 170
            # `type` field is shadowed by the record-type meta key and never
            # emitted); text_height is silver's own top-level duplicate (gold
            # keeps the height only in ctx.text_height); graphic_data has no
            # gold field (gold folds the embedded-graphics binary into
            # unknown_bits, which stays a gold-only residual row).
            for sk in ("context", "style_handle", "property_override_flags",
                       "path_type", "line_color", "line_type_handle", "line_weight",
                       "enable_landing", "enable_dogleg", "dogleg_length",
                       "arrowhead_handle", "arrowhead_size", "content_type",
                       "text_style_handle", "text_left_attachment",
                       "text_right_attachment", "text_angle_type", "text_alignment",
                       "text_color", "text_frame", "block_content_handle",
                       "block_content_color", "block_scale", "block_rotation",
                       "block_connection_type", "enable_annotation_scale",
                       "block_attributes", "arrowhead_overrides",
                       "text_direction_negative", "text_align_in_ipe",
                       "text_attachment_point", "scale_factor",
                       "text_attachment_direction", "text_top_attachment",
                       "text_bottom_attachment", "extend_leader_to_text",
                       "dwg_version", "text_height", "graphic_data",
                       "dwg_attach_dir", "dwg_attach_top", "dwg_attach_bottom",
                       # R2010+ wire bit-group (writer-side fidelity channel;
                       # gold's spec walks it into unknown_bits).
                       "dwg_raw_tail_bits"):
                payload.pop(sk, None)

        if silver_type in ("Solid3D", "Region"):
            # 3DSOLID/REGION family (gold dwg.spec 2675/2681 shells; the
            # payload comes from ACTION_3DSOLID: json_3dsolid out_json.c 1555
            # emits version/acis_data, then COMMON_3DSOLID in
            # dwg_spec_shared.h 472 emits the wireframe block, R2007a
            # materials, R2013b revision set, and history_id). Silver's
            # payload: point_of_reference, acis_data{version, sat_data,
            # sab_data, is_binary, revision{...}, materials,
            # wireframe_*, acis_empty_bit, extra_acis_data}, wires,
            # silhouettes, history_handle, uid. Body is EXCLUDED: gold's
            # corpus BODY records are bare shells today (0 diff rows) and this
            # branch must not perturb them.
            acis = payload.get("acis_data") if isinstance(payload.get("acis_data"), dict) else {}
            rev = acis.get("revision") if isinstance(acis.get("revision"), dict) else {}
            version = {"Version1": 1, "Version2": 2}.get(acis.get("version"))
            if version is None:
                version = 2 if acis.get("is_binary") else 1

            # R2018 moved the modeler geometry into the data section: the
            # inline stream reads acis_empty=1 and gold emits nothing but the
            # flag + the always-on COMMON block (wireframe/revision). Silver
            # parses the ds-section blob into sab_data, which therefore has NO
            # gold counterpart on an R2018 file — emit the flag shape only.
            is_empty = False
            if r2018_plus and gold_type in ("3DSOLID", "REGION"):
                is_empty = True
            fields["acis_empty"] = 1 if is_empty else 0
            if not is_empty:
                # json_3dsolid: unknown is emitted for every non-empty
                # record. Empirically (65/65 corpus records) unknown==1
                # exactly when the SAT is the ASCII version-1 format.
                fields["unknown"] = 1 if version == 1 else 0
                fields["version"] = version
                sab = acis.get("sab_data") or []
                sat = acis.get("sat_data") or ""
                if version == 2 and isinstance(sab, list) and sab:
                    # json_3dsolid v2/SAB: ["%.15s", VALUE_BINARY(rest)] —
                    # the 15-char "ACIS BinaryFile" prefix, then the remainder
                    # as uppercase hex (verified byte-for-byte vs ATMOS).
                    fields["acis_data"] = [
                        bytes(sab[0:15]).decode("utf-8", "replace"),
                        "".join(f"{b:02X}" for b in sab[15:]),
                    ]
                elif version == 1 and isinstance(sat, str) and sat:
                    # json_3dsolid v1/SAT: split at \r/\r\n/\n cut points
                    # (json_cquote escapes vanish after the JSON parse), keep
                    # interior empty segments, drop a single trailing one.
                    segs = re.split(r"\r\n|\r|\n", sat)
                    if segs and segs[-1] == "" and (sat.endswith("\n") or sat.endswith("\r")):
                        segs = segs[:-1]
                    fields["acis_data"] = segs
                # encr_sat_data (v1): gold re-emits the raw obfuscated wire
                # blocks as one uppercase-hex string per block (out_json
                # json_3dsolid VALUE_BINARY per block; the 159-cipher decode
                # that yields sat_data is lossy, the raw blocks are not —
                # retained since the reader-capture packet).
                if version == 1:
                    _eb = acis.get("encr_sat_data") or []
                    if isinstance(_eb, list) and _eb:
                        fields["encr_sat_data"] = [
                            bytes(int(x) & 0xFF for x in blk).hex().upper()
                            for blk in _eb if isinstance(blk, list)]
                # history_id: COMMON_3DSOLID's else-branch emits it for every
                # non-SAT record (version>1) whose handle stream has bits
                # left — not just SINCE R_2007a. Verified on R2004-era
                # records (gold emits the [0,0] null form pre-2007 too).
                if version > 1:
                    fields["history_id"] = normalize_handle_value(payload.get("history_handle") or 0)
            # COMMON_3DSOLID — always emitted, empty or not.
            fields["acis_empty_bit"] = 1 if acis.get("acis_empty_bit") else 0
            wf = bool(acis.get("wireframe_data_present"))
            fields["wireframe_data_present"] = 1 if wf else 0
            if wf:
                pp = bool(acis.get("wireframe_point_present"))
                fields["point_present"] = 1 if pp else 0
                if pp:
                    fields["point"] = normalize_value(payload.get("point_of_reference"))
                fields["isolines"] = acis.get("wireframe_isolines", 0)
                ip = bool(acis.get("wireframe_isoline_present"))
                fields["isoline_present"] = 1 if ip else 0
                if ip:
                    wires = payload.get("wires") or []
                    sils = payload.get("silhouettes") or []
                    # normalize_gold collapses the wire/silhouette structs to
                    # gold's degenerate REPEAT emission [0]*count, and a
                    # zero-count array is omitted entirely.
                    if isinstance(wires, list) and wires:
                        fields["wires"] = [0] * len(wires)
                    if isinstance(sils, list) and sils:
                        fields["silhouettes"] = [0] * len(sils)
                    mats = acis.get("materials") or []
                    if version > 1 and isinstance(mats, list) and mats:
                        fields["num_materials"] = len(mats)
                        fields["materials"] = [
                            {"array_index": m.get("array_index", 0),
                             "mat_absref": m.get("mat_absref", 0),
                             "material_handle": normalize_handle_value(m.get("material_handle") or 0)}
                            for m in mats if isinstance(m, dict)]
            if r2013_plus:
                fields["has_revision_guid"] = 1 if rev.get("has_guid") else 0
                fields["revision_major"] = rev.get("major", 0)
                fields["revision_minor1"] = rev.get("minor1", 0)
                fields["revision_minor2"] = rev.get("minor2", 0)
                fields["revision_bytes"] = rev.get("bytes", [0] * 8)
                fields["end_marker"] = rev.get("end_marker", 0)
            # Record-meta rule (out_json.c DWG_ENTITY macro): gold emits
            # `dxfname` when the class dxfname differs from the spec block
            # name — the `_3DSOLID` alias family. REGION matches, so it never
            # gets one.
            if gold_type == "3DSOLID":
                fields["dxfname"] = "3DSOLID"
            # R2013+ AcDs-backed prologue-divergence drop (mirrored in
            # normalize_gold.py under `has_ds_data`): LibreDWG's 3DSOLID
            # spec reads a phantom leading `acis_empty` bit on the
            # R2013+/R2018 ds-backed records and derails — its wireframe/
            # revision internals are desync garbage that silver's bit-true
            # reads (verified against the raw wire 2026-09-20, see
            # IMPLEMENTATION.md §8.1.6) can never match. The ds-backed
            # selector on this side is the parsed SAB — the data-section
            # blob that silver alone reads. Drop the divergent fields for
            # exactly those records; every other family record (inline
            # SAT/SAB, pre-R2013, non-ds) keeps them.
            if r2013_plus and isinstance(acis.get("sab_data"), list) and acis["sab_data"]:
                for fk in ("acis_data", "history_id",
                           "point_present", "point", "isolines", "isoline_present",
                           "acis_empty_bit",
                           "has_revision_guid", "revision_major", "revision_minor1",
                           "revision_minor2", "revision_bytes", "end_marker",
                           # the wire/silhouette structs live behind the
                           # isoline-present flag in gold's walk: derailed
                           # R2013+ reads never reach them, so gold emits
                           # nothing — silver's deeper parse (real wire
                           # geometry from the ds-backing) has no gold
                           # counterpart (the 8-file wires stub packet).
                           "wires", "silhouettes"):
                    fields.pop(fk, None)
            for sk in ("uid", "point_of_reference", "acis_data", "wires",
                       "silhouettes", "history_handle"):
                payload.pop(sk, None)

        if gold_type == "POLYLINE_2D":
            # (dwg.spec POLYLINE 2D) silver's flags-bit dict, smooth enum
            # and vertex list never appear in gold's record — capture the
            # first two for the kid-synthesis block below (flag BS 70 /
            # curve_type BS 75) and pop before the generic loop (the 3D
            # precedent). The vertices were already captured by the
            # pre-merge kid block (_kid_verts).
            _p2d_flags = payload.get("flags")
            _p2d_smooth = payload.get("smooth_surface")
            for _sk in ("flags", "smooth_surface", "vertices"):
                payload.pop(_sk, None)

        if gold_type == "POLYLINE_3D":
            # (dwg.spec POLYLINE_3D) silver's storage-only width/mesh/
            # smooth/elevation/extrusion fields never appear in gold's
            # record — pop them BEFORE the generic loop; the parent's own
            # fields (curve_type/flag/kid links) are emitted in the
            # kid-synthesis block below. Capture the flag-bit dict for
            # that block — the pop removes it from the payload.
            _p3d_flags = payload.get("flags")
            for _sk in ("flags", "smooth_type", "default_start_width",
                        "default_end_width", "mesh_m_count", "mesh_n_count",
                        "smooth_m_density", "smooth_n_density", "elevation",
                        "normal", "vertices"):
                payload.pop(_sk, None)


        field_map = FIELD_NAME_MAP.get(silver_type, {})
        for k, v in payload.items():
            if k == "common":
                continue
            # BLOCK/ENDBLK entity (dwg.spec 610-712): gold's dwgread emits only
            # `name` + common handle data on the binary-DWG path; base_pt /
            # description / xref_path are `#ifdef IS_DXF`-only. Silver wrongly
            # carries the parent BLOCK_HEADER's fields onto the entity — drop
            # them (both pre- and post-FIELD_NAME_MAP names).
            if silver_type in ("Block", "BlockEnd") and k in (
                "base_pt", "base_point", "description", "xref_path"):
                continue
            name = field_map.get(k, k)
            if is_ignored(name, ignore_set, ignore_patterns):
                continue
            fields[name] = normalize_value(v)

        # Nested polyline-family children: gold decodes the wire's
        # per-vertex/sub-entity stream records as standalone typed records
        # (VERTEX_2D/3D/MESH/PFACE/PFACE_FACE + one SEQEND per parent).
        # Silver nests them inside the parent payload; emit them after the
        # parent so the differ's per-type ordinals align with gold's
        # handle-ascending stream order.
        kid_map = {
            "POLYLINE_2D": "VERTEX_2D",
            "POLYLINE_3D": "VERTEX_3D",
            "POLYLINE_MESH": "VERTEX_MESH",
            "POLYLINE_PFACE": "VERTEX_PFACE",
        }
        _kid_type = kid_map.get(gold_type)
        _kid_recs = list(_ins_kids)
        if _kid_type:
            _vt = _kid_type
            _verts = _kid_verts or []
            _ph = (payload.get("common") or {}).get("handle") or handle
            _vcommon = {k: v for k, v in fields.items()
                        if k in ("layer", "is_xdic_missing", "has_ds_data",
                                 "color", "ltype_scale", "ltype_flags",
                                 "plotstyle_flags", "material_flags",
                                 "shadow_flags", "has_full_visualstyle",
                                 "has_face_visualstyle", "has_edge_visualstyle",
                                 "invisible", "linewt")}
            _pre2004 = not r2004_plus
            _handles = []
            for i, v in enumerate(_verts):
                if not isinstance(v, dict):
                    continue
                # 2D vertices carry no handles on silver's side: the wire
                # assigns parent+1..parent+n; the others store real handles
                # under nested common.
                _nh = None
                if isinstance(v.get("common"), dict):
                    _nh = v["common"].get("handle")
                # silver's 3D-family Vertex3DPolyline serializes the wire
                # handle as a FLAT field (e.g. 1052 after the poly@1050 +
                # seqend@1051) — use it; the 2D-family Vertex2D retains the
                # real wire handle as wire_handle; the parent+1..parent+n
                # fallback only applies to vertices with no wire knowledge
                # (DXF-built payloads) and can MISPAIR whenever a foreign
                # block-chain record interleaves the kid run
                # (2000/PolyLine2D.dwg: LINE@512 between poly@511 and its
                # true kids 513/514).
                if _nh is None and isinstance(v.get("handle"), int):
                    _nh = v["handle"]
                if _nh is None and isinstance(v.get("wire_handle"), int):
                    _nh = v["wire_handle"]
                if _nh is None:
                    _nh = (_ph or 0) + 1 + len(_handles)
                _handles.append(_nh)
            for j, v in enumerate(_verts):
                if not isinstance(v, dict):
                    continue
                rec = dict(_vcommon)
                rec["handle"] = normalize_handle_value(_handles[j])
                rec["ownerhandle"] = normalize_handle_value(_ph or 0)
                _fl = v.get("flags")
                if isinstance(_fl, str):
                    # serde prints the vertex-flag bit names "|"-joined; the
                    # corpus names compose gold's value exactly
                    # (POLYGON_MESH|POLYFACE_MESH = 64|128 = 192).
                    _fl = sum({"POLYGON_MESH": 64, "POLYFACE_MESH": 128,
                               "EXTRA_VERTEX": 32, "MESHSMOOTH": 512}.get(p.strip(), 0)
                               for p in _fl.split("|"))
                rec["flag"] = _fl.get("bits", 0) if isinstance(_fl, dict) else (_fl or 0)
                loc = v.get("location", v.get("position"))
                rec["point"] = normalize_value(loc)
                if _vt == "VERTEX_2D":
                    rec["bulge"] = normalize_float(v.get("bulge", 0.0))
                    rec["tangent_dir"] = normalize_float(v.get("curve_tangent", 0.0))
                    if r2010_plus:
                        # gold VERTEX_2D id (SINCE R_2010, the vertex_id BL):
                        # emitted even when 0.
                        rec["id"] = v.get("id", 0)
                    if r13_14:
                        # §19 H8h-ext-17: the retained R13/R14 isbylayerlt
                        # wire bit (gold common_entity_data.spec 377).
                        _iblt = v.get("wire_isbylayerlt")
                        if _iblt is not None:
                            rec["isbylayerlt"] = 1 if _iblt else 0
                if _vt == "VERTEX_3D" and r13_14:
                    # §19 H8h-ext-17: the 3D family's retained bit (the
                    # Vertex3DPolyline model field).
                    _iblt = v.get("wire_isbylayerlt")
                    if _iblt is not None:
                        rec["isbylayerlt"] = 1 if _iblt else 0
                # Wire reactors of the kid record: associative networks
                # register individual polyline vertices as reactors (gold
                # prints the list only on the registered kid). Read the
                # 3D-family flat retention or the pface-style nested
                # common; emit only when non-empty (gold's no-reactor
                # kids carry no key at all).
                _rlist = v.get("reactor_handles")
                if not isinstance(_rlist, (list, tuple)):
                    _cc = v.get("common")
                    _rlist = _cc.get("reactors") if isinstance(_cc, dict) else None
                if isinstance(_rlist, (list, tuple)):
                    _rhs = [r for r in _rlist if isinstance(r, int)]
                    if _rhs:
                        rec["reactors"] = [normalize_handle_value(r) for r in _rhs]
                if _pre2004:
                    # R13-era chains: the RETAINED WIRE STATE projects
                    # verbatim — the golden authored files chain only SOME
                    # records (Polyline2D_AC1015 chains the first vertex
                    # only; the rest carry nolinks=1), so the chain is
                    # author data, not a convention. The per-family rules
                    # below are the DXF/constructed fallback (empirically
                    # verified on the libredwg corpus). Codes on gold's
                    # null/ref handles differ (4/6/8) — the differ
                    # tolerates our code=None forms.
                    _wn = v.get("wire_nolinks")
                    _wp = v.get("wire_prev_entity")
                    _wnx = v.get("wire_next_entity")
                    _pface_iblt = None
                    if (_wn is None or r13_14) and _vt == "VERTEX_PFACE":
                        # the pface family retains the full wire common
                        # (dwg2json's pface walk emits it to _common_dwg)
                        _cd = (data.get("_common_dwg") or {}).get(
                            "0x%X" % (_handles[j] or 0))
                        if isinstance(_cd, dict):
                            if _wn is None:
                                _wn = _cd.get("nolinks")
                                _wp = _cd.get("prev_entity_handle")
                                _wnx = _cd.get("next_entity_handle")
                            if r13_14:
                                # §19 H8h-ext-17: the R13/R14 isbylayerlt
                                # bit from the retained wire common
                                # (derived: linetype_flags != 3).
                                _lt = _cd.get("linetype_flags")
                                if _lt is not None:
                                    _pface_iblt = 1 if _lt != 3 else 0
                    if _pface_iblt is not None:
                        rec["isbylayerlt"] = _pface_iblt
                    if _wn is not None:
                        rec["nolinks"] = 1 if _wn else 0
                        if not _wn:
                            rec["prev_entity"] = normalize_handle_value(_wp or 0)
                            rec["next_entity"] = normalize_handle_value(_wnx or 0)
                    elif _vt == "VERTEX_2D":
                        rec["prev_entity"] = normalize_handle_value(
                            _handles[j - 1] if j else 0)
                        rec["next_entity"] = normalize_handle_value(
                            _handles[j + 1] if j + 1 < len(_handles) else 0)
                        rec["nolinks"] = 0
                    elif _vt == "VERTEX_3D":
                        # gold (2000/PolyLine3D.dwg VERTEX_3D chains): first
                        # prev=0/next=+1, middles bare nolinks, LAST
                        # prev = PREVIOUS KID (code 8), next=0.
                        if j == 0:
                            rec["prev_entity"] = normalize_handle_value(0)
                            rec["next_entity"] = normalize_handle_value(
                                _handles[1] if len(_handles) > 1 else 0)
                            rec["nolinks"] = 0
                        elif j == len(_handles) - 1:
                            rec["prev_entity"] = normalize_handle_value(
                                _handles[j - 1])
                            rec["next_entity"] = normalize_handle_value(0)
                            rec["nolinks"] = 0
                        else:
                            rec["nolinks"] = 1
                    elif _vt == "VERTEX_MESH":
                        # gold (2000/TS1 POLYLINE_MESH 527 grid): the FIRST
                        # vertex chains forward only (prev = the code-4
                        # null 0, next = the second kid); the LAST vertex
                        # chains back (prev = the PREVIOUS kid — a code-8
                        # zero-offset ref resolving to it, not the 0-null
                        # form — gold 539: prev {8 -> 528..538}); the
                        # middles carry bare nolinks=1.
                        if j == 0:
                            rec["prev_entity"] = normalize_handle_value(0)
                            rec["next_entity"] = normalize_handle_value(
                                _handles[1] if len(_handles) > 1 else 0)
                            rec["nolinks"] = 0
                        elif j == len(_handles) - 1:
                            rec["prev_entity"] = normalize_handle_value(
                                _handles[j - 1])
                            rec["next_entity"] = normalize_handle_value(0)
                            rec["nolinks"] = 0
                        else:
                            rec["nolinks"] = 1
                    elif _vt == "VERTEX_PFACE":
                        # gold (verified ex2000 1253-1258): ONLY the first
                        # vertex chains (prev=0/next=+1/nolinks=0); every
                        # middle AND the last vertex carry bare nolinks=1 —
                        # the PFACE family never chains back.
                        if j == 0:
                            rec["prev_entity"] = normalize_handle_value(0)
                            rec["next_entity"] = normalize_handle_value(
                                _handles[1] if len(_handles) > 1 else 0)
                            rec["nolinks"] = 0
                        else:
                            rec["nolinks"] = 1
                _kid_recs.append({"type": _vt, "fields": rec})
            if gold_type == "POLYLINE_PFACE":
                _faces = [fc for fc in (_kid_faces or [])
                          if isinstance(fc, dict)]
                _fh = [(fc.get("common") or {}).get("handle") or 0
                       for fc in _faces]
                for j, fc in enumerate(_faces):
                    rec = dict(_vcommon)
                    rec["handle"] = normalize_handle_value(
                        (fc.get("common") or {}).get("handle") or 0)
                    rec["ownerhandle"] = normalize_handle_value(_ph or 0)
                    # gold's PFACE_FACE flag is the constant face indicator
                    # 128 (census: 111/111 records across versions; silver's
                    # parsed face bits are 0 on R2010+ and diverge on R2000).
                    rec["flag"] = 128
                    if not r2004_plus:
                        # The retained wire chain (dwg2json's pface walk →
                        # _common_dwg) projects verbatim — the golden
                        # authored files chain only SOME face records (the
                        # chain is author data); the ex2000 rule below is
                        # the DXF/constructed fallback.
                        _fcd = (data.get("_common_dwg") or {}).get(
                            "0x%X" % (_fh[j] or 0))
                        _fn = _fcd.get("nolinks") if isinstance(_fcd, dict) else None
                        if r13_14 and isinstance(_fcd, dict):
                            # §19 H8h-ext-17: the R13/R14 isbylayerlt bit
                            # from the retained wire face common.
                            _flt = _fcd.get("linetype_flags")
                            if _flt is not None:
                                rec["isbylayerlt"] = 1 if _flt != 3 else 0
                        if _fn is not None:
                            rec["nolinks"] = 1 if _fn else 0
                            if not _fn:
                                rec["prev_entity"] = normalize_handle_value(
                                    _fcd.get("prev_entity_handle") or 0)
                                rec["next_entity"] = normalize_handle_value(
                                    _fcd.get("next_entity_handle") or 0)
                        elif len(_fh) > 1:
                            # gold (2000-era, verified ex2000): first+middle
                            # faces carry bare nolinks=1; ONLY the LAST face
                            # chains back (prev=previous face, code 8).
                            if j == len(_fh) - 1:
                                rec["prev_entity"] = normalize_handle_value(_fh[j - 1])
                                rec["next_entity"] = normalize_handle_value(0)
                                rec["nolinks"] = 0
                            else:
                                rec["nolinks"] = 1
                    # gold's vertind takes the four face indices through
                    # normalize_gold's raw 4-tuple->handle interpretation:
                    # {'code': i1, 'size': i2, 'value': i3, 'absref': i4}
                    # (verified 1:1 on example_2004's three faces).
                    _ii = [fc.get(f"index{i}", 0) for i in range(1, 5)]
                    rec["vertind"] = {"code": _ii[0], "size": _ii[1],
                                      "value": _ii[2], "absref": _ii[3]}
                    _kid_recs.append({"type": "VERTEX_PFACE_FACE", "fields": rec})
        if _kid_type:
            # SEQEND: the parent's trailing common-only record. The wire's
            # SEQEND entities are real records with real handles — silver's
            # reader retains them on the parent (seqend_handle, restored by
            # the builder for every polyline family) and the writer echoes
            # them, so prefer the REAL handle; a wrong synthesized ordinal
            # rotates the (type, ordinal) pairing of every SEQEND in the
            # file. Fall back to the old conventions (POLYLINE_3D adaptive —
            # parent+1 when that handle is vacant in the file's layout
            # ([poly, seqend, verts]) — else last-child+1) only when the
            # reader stored none.
            _ph = (payload.get("common") or {}).get("handle") or handle
            _seq_h_real = (_seq_h_parent
                           if isinstance(_seq_h_parent, int)
                           else None)
            if _seq_h_real:
                _seqend_h = _seq_h_real
            elif gold_type == "POLYLINE_3D" and _handles:
                if (_ph or 0) + 1 not in _handles:
                    _seqend_h = (_ph or 0) + 1
                else:
                    _seqend_h = max(_handles) + 1
            else:
                _last_kid_h = None
                for kr in _kid_recs:
                    h = kr["fields"].get("handle")
                    if isinstance(h, dict) and h.get("absref"):
                        _last_kid_h = h["absref"]
                if _last_kid_h is None:
                    _n = len([v for v in (_kid_verts or [])
                              if isinstance(v, dict)])
                    _last_kid_h = (_ph or 0) + _n
                _seqend_h = (_last_kid_h or 0) + 1
            _sf = {**_vcommon,
                   "handle": normalize_handle_value(_seqend_h),
                   "ownerhandle": normalize_handle_value(_ph or 0)}
            # The SEQEND record's own common flags are retained from the
            # wire (builder pending.seqend_flags — the SEQENDs are real
            # records; LibreDWG-authored example files carry
            # flags-3-with-null, DWG-native chains 0). Emit the retained
            # values verbatim; gold's flags==3 pulls a [5,0,0,0] null ref.
            if isinstance(_seq_pf_stash, int) and not r13_14:
                # (R13/R14: the plotstyle_flags BB is R2000b+ — gold's
                # R13 SEQENDs carry no plotstyle field.)
                _sf["plotstyle_flags"] = _seq_pf_stash
                if _seq_pf_stash == 3:
                    _sf["plotstyle"] = normalize_handle_value(0)
            if r2007_plus and isinstance(_seq_sf_stash, int):
                _sf["shadow_flags"] = _seq_sf_stash
                if _seq_sf_stash == 3:
                    _sf["shadow"] = normalize_handle_value(0)
            if r13_14:
                # §19 H8h-ext-17: the R13/R14 isbylayerlt bit — retained
                # from the wire SEQEND's own common (gold
                # common_entity_data.spec 377); the False default is the
                # constructed/DXF fallback (gold's R13 SEQENDs read 1).
                _sf["isbylayerlt"] = 1 if _seq_iblt_stash else 0
            if not r2004_plus:
                # R13-era SEQEND chains: null prev/next pair
                _sf["prev_entity"] = normalize_handle_value(0)
                _sf["next_entity"] = normalize_handle_value(0)
                _sf["nolinks"] = 0
            _kid_recs.append({"type": "SEQEND", "fields": _sf})
        if gold_type == "POLYLINE_2D":
                # gold dwg.spec POLYLINE 2D parent fields: flag BS 70
                # (silver's flags bit dict: closed=1), curve_type BS 75
                # (smooth_surface NoSmooth -> 0), the R2000 first/last
                # vertex chain + seqend (silver's 2D kids carry no stored
                # handles — the wire assigns parent+1..parent+n). The two
                # dict values were captured by the early pop branch.
                fields["flag"] = ((_p2d_flags or {}).get("bits", 0)
                                  if isinstance(_p2d_flags, dict) else 0)
                fields["curve_type"] = ({"NoSmooth": 0, "Quadratic": 5,
                                         "Cubic": 6, "Bezier": 8}
                                        .get(_p2d_smooth, 0)
                                        if isinstance(_p2d_smooth, str) else 0)
                if _handles:
                    # gold dwg.spec POLYLINE_2D: the kid link set is the
                    # R2004a+ `vertex` handle vector (code 3) — the same
                    # gate as the 3D block below; pre-2004 first_vertex/
                    # last_vertex (code 4).
                    if r2004_plus:
                        fields["vertex"] = [normalize_handle_value(h)
                                            for h in _handles]
                    else:
                        fields["first_vertex"] = normalize_handle_value(_handles[0])
                        fields["last_vertex"] = normalize_handle_value(_handles[-1])
                if _seqend_h is not None:
                    fields["seqend"] = normalize_handle_value(_seqend_h)
        if gold_type == "POLYLINE_3D":
                # gold dwg.spec POLYLINE_3D parent fields: curve_type (BS,
                # 0 on every corpus record), flag (the 70-bitfield — only
                # the record-relevant bits: 1 closed, 4 spline-fit; the
                # 3D/mesh type bits are implied by the record type), the
                # kid link set — R2004a+ `vertex` handle vector (gold
                # code 3) + `seqend` (code 3); pre-2004 first_vertex/
                # last_vertex (code 4). Silver's width/mesh/smooth/
                # elevation/extrusion fields are storage-only and never
                # appear in gold's record.
                fields["curve_type"] = 0
                # the flag-bit dict was captured (and popped) by the
                # early POLYLINE_3D branch above — the pop removed it from
                # the payload, so read it from the stash
                _pf = _p3d_flags
                if isinstance(_pf, dict):
                    fields["flag"] = ((1 if _pf.get("closed") else 0)
                                      | (4 if _pf.get("spline_fit") else 0)
                                      | (2 if _pf.get("curve_fit") else 0))
                else:
                    fields["flag"] = 0
                if _handles:
                    if r2004_plus:
                        fields["vertex"] = [normalize_handle_value(h)
                                            for h in _handles]
                    else:
                        fields["first_vertex"] = normalize_handle_value(_handles[0])
                        fields["last_vertex"] = normalize_handle_value(_handles[-1])
                    fields["seqend"] = normalize_handle_value(_seqend_h)
        _emit_unknown_bits(gold_type, handle, fields)
        out.append({"type": gold_type, "fields": fields})
        for _kr in _kid_recs:
            out.append(_kr)

    # Objects map keyed by handle string. Silver stores objects in a HashMap
    # (unordered), while gold's OBJECTS array is in handle order. The differ
    # aligns by (type, ordinal-within-type), so emit silver objects in
    # ascending handle order to match gold's canonical ordering.
    def _obj_sort_key(item) -> int:
        payload = item[1]
        if isinstance(payload, dict) and len(payload) == 1:
            inner = next(iter(payload.values()))
            if isinstance(inner, dict):
                h = inner.get("handle")
                if isinstance(h, int):
                    return h
        try:
            return int(item[0], 0)
        except (ValueError, TypeError):
            return 0

    for _handle_key, obj in sorted(objects.items(), key=_obj_sort_key):
        if not isinstance(obj, dict) or len(obj) != 1:
            continue
        silver_type = list(obj.keys())[0]
        payload = obj[silver_type]
        if not isinstance(payload, dict):
            continue
        # §20 the R2018 record-identity packet: the wire-capture
        # channels (the associative aligned-dim body, the table
        # entity/tablecontent bodies) are writer-side codec fields —
        # gold's decodes carry no counterpart (the source_version
        # precedent) and they are never comparison payload on any
        # object type, typed projection or UNKNOWN.
        for _wk in ("wire_main", "wire_main_bit_len",
                    "wire_text", "wire_text_bit_len",
                    "wire_handles", "wire_handles_bit_len",
                    "wire_dxf_version"):
            payload.pop(_wk, None)
        if silver_type == "TableContent":
            _tc_common = (payload.get("common")
                          if isinstance(payload.get("common"), dict) else {})
            _tc_handle = _tc_common.get("handle", payload.get("handle"))
            if not isinstance(_tc_handle, int):
                # silver synthesizes a handleless TableContent wrapper on the
                # 2000-era files; gold types no ACAD_TABLE there — skip the
                # phantom record instead of emitting an extra_in_silver count.
                continue
            # Handled records: gold reads them raw as UNKNOWN_OBJ (dead
            # TABLECONTENT frame, dwg2.spec 466). The nested common dict
            # holds the standard common keys — hoist them to the top level
            # so _inject_reactors/_inject_xdic/_object_common_fields and
            # the is_xdic_missing/has_ds_data gates see them; the UNKNOWN
            # payload-clear below drops the typed tables payload anyway.
            for _k in ("handle", "owner", "owner_handle", "reactors",
                       "xdictionary_handle"):
                if _k not in payload and _k in _tc_common:
                    payload[_k] = _tc_common[_k]
        if silver_type == "Associative":
            # Resolve the class from the payload's dxf_name; the classes with
            # a landed field projection below emit under their real gold type
            # (DIMASSOC + the acdbAssoc dependency/action families), the rest
            # (the debug-gated PersSubentManager/ContextDataManager classes,
            # §8.1.1 liveness rule) stay UNKNOWN.
            _assoc_dxf = payload.get("dxf_name") or ""
            if _associative_gold_name(_assoc_dxf) == "DIMASSOC":
                gold_type = "DIMASSOC"
            elif _assoc_dxf in ("ACDBASSOCDEPENDENCY", "ACDBASSOCGEOMDEPENDENCY",
                                "ACDBASSOCVALUEDEPENDENCY", "ACDBASSOCVARIABLE",
                                "ACDBASSOCNETWORK", "ACDBASSOC2DCONSTRAINTGROUP",
                                "ASSOCDIMDEPENDENCYBODY"):
                gold_type = {"ACDBASSOCDEPENDENCY": "ASSOCDEPENDENCY",
                             "ACDBASSOCGEOMDEPENDENCY": "ASSOCGEOMDEPENDENCY",
                             "ACDBASSOCVALUEDEPENDENCY": "ASSOCVALUEDEPENDENCY",
                             "ACDBASSOCVARIABLE": "ASSOCVARIABLE",
                             "ACDBASSOCNETWORK": "ASSOCNETWORK",
                             "ACDBASSOC2DCONSTRAINTGROUP": "ASSOC2DCONSTRAINTGROUP",
                             "ASSOCDIMDEPENDENCYBODY": "ASSOCDIMDEPENDENCYBODY"}[_assoc_dxf]
            elif _assoc_dxf in ("ACDBASSOCACTION",
                                "ACDBASSOCOSNAPPOINTREFACTIONPARAM",
                                "ACDBASSOCVERTEXACTIONPARAM",
                                "ACDBASSOCEXTRUDEDSURFACEACTIONBODY",
                                "ACDBASSOCLOFTEDSURFACEACTIONBODY",
                                "ACDBASSOCREVOLVEDSURFACEACTIONBODY",
                                "ACDBASSOCPLANESURFACEACTIONBODY",
                                "ACDBASSOCPATHACTIONPARAM"):
                # live dwg2.spec blocks; the two params sit in
                # _UNKNOWN_BITS_TYPES (the reader side channel emits their
                # unknown_bits automatically at the append).
                gold_type = {"ACDBASSOCACTION": "ASSOCACTION",
                             "ACDBASSOCOSNAPPOINTREFACTIONPARAM":
                                 "ASSOCOSNAPPOINTREFACTIONPARAM",
                             "ACDBASSOCVERTEXACTIONPARAM":
                                 "ASSOCVERTEXACTIONPARAM",
                             "ACDBASSOCEXTRUDEDSURFACEACTIONBODY":
                                 "ASSOCEXTRUDEDSURFACEACTIONBODY",
                             "ACDBASSOCLOFTEDSURFACEACTIONBODY":
                                 "ASSOCLOFTEDSURFACEACTIONBODY",
                             "ACDBASSOCREVOLVEDSURFACEACTIONBODY":
                                 "ASSOCREVOLVEDSURFACEACTIONBODY",
                             "ACDBASSOCPLANESURFACEACTIONBODY":
                                 "ASSOCPLANESURFACEACTIONBODY",
                             "ACDBASSOCPATHACTIONPARAM":
                                 "ASSOCPATHACTIONPARAM"}[_assoc_dxf]
            else:
                # Gold's name for an unmodeled non-entity class record is
                # UNKNOWN_OBJ (dwgread's raw-object dump); "UNKNOWN" is not
                # a gold record type and produced count_mismatch rows.
                gold_type = "UNKNOWN_OBJ"
        else:
            gold_type = OBJECT_TYPE_MAP.get(silver_type, silver_type.upper())
        # Underlay definitions: gold's object name is per-kind too
        # (PDFDEFINITION/DWFDEFINITION/DGNDEFINITION; silver keeps
        # underlay_type). This must run AFTER the Associative if/else
        # above — OBJECT_TYPE_MAP's "UnderlayDefinition" entry is the
        # non-gold "PDFDEF" default and previously overwrote the
        # per-kind retype (the gh109-era count_mismatch pocket).
        if (silver_type == "UnderlayDefinition"
                and isinstance(payload.get("underlay_type"), str)):
            gold_type = {
                "Pdf": "PDFDEFINITION", "Dwf": "DWFDEFINITION",
                "Dgn": "DGNDEFINITION",
                "Png": "PNGDEFINITION", "Jpeg": "JPGDEFINITION",
            }.get(payload["underlay_type"], "PDFDEFINITION")
        # Section family (2018/LiveSection1.dwg): silver parses
        # AcDbSectionManager / AcDbSectionSettings into the ClassObject
        # wrapper. Gold's SECTION_MANAGER (dwg2.spec, live: the 553
        # record is is_live + sections only) and SECTION_SETTINGS. The
        # SectionViewStyle/DetailViewStyle classes stay UNKNOWN_OBJ.
        if (silver_type == "ClassObject"
                and isinstance(payload.get("data"), dict)):
            if "SectionManager" in payload["data"]:
                gold_type = "SECTION_MANAGER"
            elif "SectionSettings" in payload["data"]:
                gold_type = "SECTION_SETTINGS"
        # Wrapper retype (§8.1.6 unmodeled-class campaign): silver's
        # DynamicBlock/ClassObject-style wrappers parse class-registered
        # objects they do not model individually, keeping the class-table
        # dxf_name. Retype + project ONLY classes with a landed field
        # projection in the same packet — retyping alone explodes field
        # rows. First landed class: ACSH_HISTORY_CLASS.
        if silver_type == "DynamicBlock" and payload.get("dxf_name") in ("ACSH_HISTORY_CLASS", "ACAD_EVALUATION_GRAPH"):
            gold_type = ("ACSH_HISTORY_CLASS" if payload["dxf_name"] == "ACSH_HISTORY_CLASS"
                         else "EVALUATION_GRAPH")
        elif silver_type == "DynamicBlock":
            # U2 (2026-09-20): the dynamic-block family — silver's
            # DynamicBlock wrapper parses the BLOCK* classes
            # (dwg2.spec 3371-3544) and the ACSH geometry nodes under
            # data.<Kind> while keeping dxf_name. Retype by dxf_name; the
            # per-class field projection below follows in the same packet.
            # Deliberately NOT here (§8.1.1 liveness rule — gold compiles
            # them out as DEBUGGING_CLASS_DXF and decodes UNKNOWN_OBJ):
            # BLOCKPROPERTIESTABLE, BLOCKPROPERTIESTABLEGRIP,
            # DYNAMICBLOCKPROXYNODE. Classes without a landed projection
            # (lookup/array/polar-stretch actions, user/XY parameters,
            # constraint parameters, ACSH sphere/cone) stay UNKNOWN until
            # their own packets.
            _dyn_dxf = payload.get("dxf_name")
            if isinstance(_dyn_dxf, str):
                gold_type = _DYNBLOCK_RETYPE.get(_dyn_dxf.upper(), gold_type)
        elif silver_type == "ClassObject":
            # Render classes (dwg2.spec 2527/2546/2566): silver's ClassObject
            # wrapper keeps no dxf_name for them — dispatch on data.<Kind>.
            _co_kind = next(iter(payload.get("data") or {}), None) \
                if isinstance(payload.get("data"), dict) else None
            gold_type = {"RenderGlobal": "RENDERGLOBAL",
                         "RenderEntry": "RENDERENTRY",
                         "RapidRtRenderSettings": "RAPIDRTRENDERSETTINGS",
                         "MentalRayRenderSettings": "MENTALRAYRENDERSETTINGS",
                         "Sun": "SUN"}.get(_co_kind, gold_type)
        elif silver_type == "DataObject":
            _do_kind = next(iter(payload.get("data") or {}), None) \
                if isinstance(payload.get("data"), dict) else None
            if _do_kind == "CellStyleMap":
                # dwg2.spec 4220 (CELLSTYLEMAP): live, and gold's REPEAT
                # emission collapses every cell struct to a bare 0.
                gold_type = "CELLSTYLEMAP"
            elif _do_kind == "TableGeometry":
                # live dwg2.spec block; in _UNKNOWN_BITS_TYPES.
                gold_type = "TABLEGEOMETRY"
        elif silver_type == "ProxyObject":
            # dwg.spec 5752 (PROXY_OBJECT, live): dxfname ACAD_PROXY_OBJECT.
            # Silver's ProxyObject wrapper keeps the parsed fields plus the
            # raw window (raw_window) gold dumps as data hex — captured
            # verbatim by the reader and projected in the branch below.
            gold_type = "PROXY_OBJECT"
        _inject_reactors(payload)
        _inject_xdic(payload)
        fields = _object_common_fields(payload)
        if silver_type == "ObjectContextData":
            _cok = payload.get("kind")
            _ld = _cok.get("Leader") if isinstance(_cok, dict) else None
            if isinstance(_ld, dict):
                # dwg2.spec 4611 (LEADEROBJECTCONTEXTDATA, live):
                # AcDbAnnotScaleObjectContextData_fields (class_version,
                # is_default, scale handle) + num_points BL + points
                # 3DPOINT_VECTOR + x_direction + b290 B + inspt_offset +
                # endptproj, with HANDLE_UNKNOWN_BITS carrying the raw
                # R2018b tail (LEADEROBJECTCONTEXTDATA is in
                # _UNKNOWN_BITS_TYPES and the reader's side channel holds
                # the bits byte-identical — the loop-end emission covers
                # the field). Silver's generic ObjectContextData parse
                # maps 1:1: endpoint_projection→endptproj,
                # insertion_offset→inspt_offset, annotation_enabled→b290.
                gold_type = "LEADEROBJECTCONTEXTDATA"
                fields["dxfname"] = "ACDB_LEADEROBJECTCONTEXTDATA_CLASS"
                _cv = payload.pop("class_version", None)
                if _cv is not None:
                    fields["class_version"] = _cv
                fields["is_default"] = 1 if payload.pop("is_default", None) else 0
                fields["scale"] = normalize_handle_value(
                    payload.pop("scale", 0) or 0)
                payload.pop("kind", None)
                points = _ld.pop("points", None)
                if isinstance(points, list):
                    fields["points"] = [normalize_value(p) for p in points]
                _xd = _ld.pop("x_direction", None)
                if _xd is not None:
                    fields["x_direction"] = normalize_value(_xd)
                fields["b290"] = 1 if _ld.pop("annotation_enabled", None) else 0
                for kk in ("insertion_offset", "endpoint_projection"):
                    _ve = _ld.pop(kk, None)
                    if _ve is not None:
                        fields["inspt_offset" if kk == "insertion_offset"
                               else "endptproj"] = normalize_value(_ve)
                for kk in list(_ld.keys()):
                    _ld.pop(kk, None)
            # non-Leader ObjectContextData payloads keep the generic
            # OBJECTCONTEXTDATA projection.
        if silver_type == "Group":
            # gold GROUP (dwg.spec 4498): FIELD_T (name, 300) then
            # unnamed BS / selectable BS / the member handle vector —
            # the wire DOES carry group names (2018/Leader.dwg's group
            # is "Superhatch"), they are just mostly empty in the
            # corpus. Silver's reader stores the wire name under the
            # misnomered `description` key (GroupData.description; the
            # writer writes it back, so the rt stays faithful) — project
            # it to gold's `name`. `groups` is the member handle vector
            # (gold code 5, same order as silver's `entities`).
            fields["name"] = payload.get("description") or ""
            fields["unnamed"] = 1 if payload.get("unnamed") else 0
            fields["selectable"] = 1 if payload.get("selectable") else 0
            _ents = payload.get("entities")
            if isinstance(_ents, list) and _ents:
                fields["groups"] = [normalize_handle_value(h) for h in _ents
                                    if isinstance(h, int)]
            for _gk in ("name", "unnamed", "selectable", "entities",
                        "description"):
                payload.pop(_gk, None)
        if silver_type == "Field":
            # gold FIELD (dwg.spec 5935, live — the ACDBFIELD class): pure
            # name projection, silver stores the same wire fields under its
            # own vocabulary. id = evaluator_id (T 1), field_state = state
            # (BL 94), evaluation_error_msg = evaluation_error_message
            # (T 300); `value` is gold's dotted TABLE_value_fields union
            # (data_type BL 90 + one member — 0/1 -> data_long, 2 ->
            # data_double, 4 -> data_string; PRE R_2007a gold masks ~0x200
            # off the data_type); gold's REPEAT emission collapses childval
            # to a bare 0 per entry in the norm and only prints `childs`
            # when num_childs > 0 (code-3 handle vector). num_objects is 0
            # all corpus — pop referenced_objects.
            fields["id"] = payload.pop("evaluator_id", "")
            fields["field_state"] = payload.pop("state", 0)
            fields["evaluation_error_msg"] = payload.pop(
                "evaluation_error_message", "")
            _fv = payload.pop("value", None)
            if isinstance(_fv, dict):
                _dt = int(_fv.get("raw_type_code", 0) or 0)
                if not r2007_plus:
                    _dt &= ~0x200
                fields["value.data_type"] = _dt
                if _dt in (0, 1):
                    fields["value.data_long"] = int(
                        _fv.get("numeric_value", 0) or 0)
                elif _dt == 2:
                    fields["value.data_double"] = normalize_float(
                        _fv.get("numeric_value", 0.0))
                elif _dt == 4:
                    fields["value.data_string"] = _fv.get("text", "")
                elif _dt == 8:
                    fields["value.data_size"] = int(
                        _fv.get("data_size", 0) or 0)
                    fields["value.data_date"] = list(
                        _fv.get("binary_value") or [])
                elif _dt in (16, 32):
                    fields["value.data_size"] = int(
                        _fv.get("data_size", 0) or 0)
                    fields["value.data_point" if _dt == 16
                           else "value.data_3dpoint"] = normalize_value(
                               _fv.get("point_value"))
            _cf = payload.pop("child_fields", None)
            if isinstance(_cf, list) and _cf:
                fields["childs"] = [normalize_handle_value(h) for h in _cf]
            _cv = payload.pop("child_values", None)
            if isinstance(_cv, list) and _cv:
                fields["childval"] = [0] * len(_cv)
            payload.pop("referenced_objects", None)
        if silver_type == "FieldList":
            # gold FIELDLIST (dwg.spec 6013): the `fields` handle vector
            # (code 4) in gold's order; normalize to handle dicts so the
            # differ resolves the targets (a bare int never matches).
            _fl_flds = payload.pop("fields", None)
            if isinstance(_fl_flds, list) and _fl_flds:
                fields["fields"] = [normalize_handle_value(h)
                                   for h in _fl_flds]
        if silver_type == "PlotSettings":
            # gold PLOTSETTINGS (dwg.spec 5223): silver's PlotSettings
            # object stores the same wire data under its own vocabulary.
            # printer_cfg_file = the page-setup name, paper_size = the
            # DEVICE name, canonical_media_name = the media (the same
            # transposition gold's names encode, cf. the LAYOUT branch);
            # the margins are a dict; plot_flags is the bit composition of
            # PlotFlags::to_bits (identical layout); paper_units/drawing_
            # units are the scale numerator/denominator (BD 142/143); the
            # enum strings invert to gold's raw BS codes (silver's enums
            # carry the gold values); plotview = the wire handle (R2004+
            # binary carries the handle — plotview_name only UNTIL R_2002)
            # and gold's shadeplot handle (SINCE R_2007a, code 4 / DXF 333)
            # is silver's visual_style_handle (objects.rs reads the
            # R2007+ handle there). cached_scale has no gold counterpart.
            fields["printer_cfg_file"] = payload.pop("page_name", "") or ""
            fields["paper_size"] = payload.pop("printer_name", "") or ""
            fields["canonical_media_name"] = payload.pop("paper_size", "") or ""
            _m = payload.pop("margins", None) or {}
            fields["left_margin"] = normalize_float(_m.get("left", 0.0))
            fields["bottom_margin"] = normalize_float(_m.get("bottom", 0.0))
            fields["right_margin"] = normalize_float(_m.get("right", 0.0))
            fields["top_margin"] = normalize_float(_m.get("top", 0.0))
            fields["plot_origin"] = [normalize_float(payload.pop("origin_x", 0.0)),
                                     normalize_float(payload.pop("origin_y", 0.0))]
            _PU = {"Inches": 0, "Millimeters": 1, "Pixels": 2}
            _pu = payload.pop("paper_units", 0)
            fields["plot_paper_unit"] = (_PU.get(_pu, _pu)
                                        if isinstance(_pu, (int, str)) else 0)
            _ROT = {"None": 0, "Degrees90": 1, "Degrees180": 2, "Degrees270": 3}
            _rot = payload.pop("rotation", 0)
            fields["plot_rotation_mode"] = (_ROT.get(_rot, _rot)
                                           if isinstance(_rot, (int, str)) else 0)
            _PT = {"LastScreenDisplay": 0, "Extents": 1, "Limits": 2,
                   "View": 3, "Window": 4, "Layout": 5}
            _pt = payload.pop("plot_type", 0)
            fields["plot_type"] = (_PT.get(_pt, _pt)
                                   if isinstance(_pt, (int, str)) else 0)
            _pw = payload.pop("plot_window", None) or {}
            fields["plot_window_ll"] = [normalize_float(_pw.get("lower_left_x", 0.0)),
                                        normalize_float(_pw.get("lower_left_y", 0.0))]
            fields["plot_window_ur"] = [normalize_float(_pw.get("upper_right_x", 0.0)),
                                        normalize_float(_pw.get("upper_right_y", 0.0))]
            if not r2004_plus:
                # UNTIL(R_2002) wire carries the plotview NAME; the handle
                # is derived (and vice versa on encode).
                fields["plotview_name"] = payload.pop("plot_view_name", "") or ""
            payload.pop("plot_view_name", None)
            fields["plotview"] = normalize_handle_value(
                payload.pop("plot_view_handle", 0) or 0)
            fields["paper_units"] = normalize_float(
                payload.pop("scale_numerator", 0.0))
            fields["drawing_units"] = normalize_float(
                payload.pop("scale_denominator", 0.0))
            fields["stylesheet"] = payload.pop("current_style_sheet", "") or ""
            _ST = {"ScaleToFit": 0, "CustomScale": 1, "OneToOne": 16,
                   "OneToTwo": 17, "OneToFour": 18, "OneToEight": 19,
                   "OneToTen": 20, "OneToSixteen": 21, "OneToTwenty": 22,
                   "OneToThirty": 23, "OneToForty": 24, "OneToFifty": 25,
                   "OneToHundred": 26, "TwoToOne": 27, "FourToOne": 28,
                   "EightToOne": 29, "TenToOne": 30, "HundredToOne": 31}
            _st = payload.pop("scale_type", 0)
            fields["std_scale_type"] = (_ST.get(_st, _st)
                                        if isinstance(_st, (int, str)) else 0)
            fields["std_scale_factor"] = normalize_float(
                payload.pop("standard_scale_factor", 0.0))
            fields["paper_image_origin"] = [
                normalize_float(payload.pop("paper_image_origin_x", 0.0)),
                normalize_float(payload.pop("paper_image_origin_y", 0.0))]
            if r2004_plus:
                # shadeplot trio SINCE R_2004a (dwg.spec)
                _SPM = {"AsDisplayed": 0, "Wireframe": 1, "Hidden": 2,
                        "Rendered": 3}
                _spm = payload.pop("shade_plot_mode", 0)
                fields["shadeplot_type"] = (_SPM.get(_spm, _spm)
                                            if isinstance(_spm, (int, str)) else 0)
                _SPR = {"Draft": 0, "Preview": 1, "Normal": 2,
                        "Presentation": 3, "Maximum": 4, "Custom": 5}
                _spr = payload.pop("shade_plot_resolution", 0)
                fields["shadeplot_reslevel"] = (_SPR.get(_spr, _spr)
                                                if isinstance(_spr, (int, str)) else 0)
                fields["shadeplot_customdpi"] = payload.pop("shade_plot_dpi", 0)
            else:
                for kk in ("shade_plot_mode", "shade_plot_resolution",
                           "shade_plot_dpi"):
                    payload.pop(kk, None)
            if r2007_plus:
                fields["shadeplot"] = normalize_handle_value(
                    payload.pop("visual_style_handle", 0) or 0)
            else:
                payload.pop("visual_style_handle", None)
            pfl = payload.pop("flags", None)
            if isinstance(pfl, dict):
                bits = pfl.get("unknown_bits", 0) & ~0x7EFF
                for name, bit in (("plot_viewport_borders", 0), ("show_plot_styles", 1),
                        ("plot_centered", 2), ("plot_hidden", 3), ("use_standard_scale", 4),
                        ("plot_plot_styles", 5), ("scale_lineweights", 6),
                        ("print_lineweights", 7), ("draw_viewports_first", 9),
                        ("model_type", 10), ("update_paper", 11),
                        ("zoom_to_paper_on_update", 12), ("initializing", 13),
                        ("prev_plot_init", 14)):
                    if pfl.get(name):
                        bits |= (1 << bit)
                fields["plot_flags"] = bits
            elif isinstance(pfl, int):
                fields["plot_flags"] = pfl
            payload.pop("cached_scale", None)
        if silver_type == "GeoData" and r2010_plus:
            # gold GEODATA R2010+ branch (dwg2.spec 25 else-arm, live — the
            # sole corpus carrier is 2010/gh209_1): pure name projection of
            # silver's GeoData struct. The pre-R2010 wire (class_version 1)
            # carries a different field set (obs_pt/scale_vec, datum/wkt,
            # the civil tail) which the R2010+ branch never reads — pop.
            # geomesh REPEATs emit only when non-empty (gold's num_*-gated
            # convention); the civil_* block is UNTIL(R_2007)-only.
            fields["class_version"] = payload.pop("version", 0)
            fields["coord_type"] = payload.pop("coordinate_type", 0)
            fields["design_pt"] = normalize_value(payload.pop("design_point", None))
            fields["ref_pt"] = normalize_value(payload.pop("reference_point", None))
            fields["unit_scale_horiz"] = normalize_float(
                payload.pop("horizontal_unit_scale", 0.0))
            fields["units_value_horiz"] = payload.pop("horizontal_units", 0)
            fields["unit_scale_vert"] = normalize_float(
                payload.pop("vertical_unit_scale", 0.0))
            fields["units_value_vert"] = payload.pop("vertical_units", 0)
            fields["up_dir"] = normalize_value(payload.pop("up_direction", None))
            fields["north_dir"] = normalize_value(payload.pop("north_direction", None))
            fields["scale_est"] = payload.pop("scale_estimation_method", 0)
            fields["do_sea_level_corr"] = 1 if payload.pop("sea_level_correction", None) else 0
            fields["sea_level_elev"] = normalize_float(
                payload.pop("sea_level_elevation", 0.0))
            fields["coord_proj_radius"] = normalize_float(
                payload.pop("coordinate_projection_radius", 0.0))
            fields["coord_system_def"] = payload.pop("coordinate_system_definition", "")
            for kk in ("obsolete_observation_point", "obsolete_scale_vector",
                       "coordinate_system_datum", "coordinate_system_wkt",
                       "civil_data_present", "civil_obsolete_flag",
                       "civil_reference_point1", "civil_reference_point2",
                       "civil_unknown1", "civil_unknown2", "civil_unknown_flag1",
                       "civil_zero_point1", "civil_zero_point2",
                       "civil_unknown_flag2", "civil_north_angle_degrees",
                       "civil_north_angle_radians"):
                payload.pop(kk, None)
            for kk in ("mesh_points", "mesh_faces"):
                _mm = payload.pop(kk, None)
                if isinstance(_mm, list) and _mm:
                    fields[kk] = [0] * len(_mm)
        if silver_type == "UnderlayDefinition":
            # gold PDFDEFINITION/DWFDEFINITION/DGNDEFINITION (dwg2.spec
            # 1597, AcDbUnderlayDefinition): filename + name. Silver's
            # file_path/page_name carry the two T fields; its `name` is a
            # lookup-only empty twin with no wire counterpart (pop).
            # reactors flow through _object_common_fields (the builder
            # retains them since the Dictionary-precedent packet).
            fields["filename"] = payload.pop("file_path", "") or ""
            fields["name"] = payload.pop("page_name", "") or ""
            payload.pop("underlay_type", None)
            payload.pop("name", None)
        if silver_type == "ClassObject":
            _codata = payload.get("data")
            if not isinstance(_codata, dict):
                _codata = {}
            if gold_type == "SECTION_MANAGER" and "SectionManager" in _codata:
                # gold SECTION_MANAGER (dwg2.spec, live): is_live B +
                # the sections handle vector (the 552 section entity).
                _sm = _codata.get("SectionManager") or {}
                fields["is_live"] = 1 if _sm.get("is_live") else 0
                _sms = _sm.get("sections")
                if isinstance(_sms, list) and _sms:
                    fields["sections"] = [normalize_handle_value(h) for h in _sms]
                payload.pop("data", None)
            elif gold_type == "SECTION_SETTINGS" and "SectionSettings" in _codata:
                # gold SECTION_SETTINGS: curr_type 4BITS + the types
                # REPEAT (normalize_gold collapses each entry dict to a
                # bare 0 — emit [0]*n) + gold retains the tail as
                # unknown_bits (silver parses past gold's spec stop; the
                # raw tail channel is not retained — residual row).
                _ss = _codata.get("SectionSettings") or {}
                fields["curr_type"] = _ss.get("current_type", 0)
                _st = _ss.get("types")
                if isinstance(_st, list):
                    fields["types"] = [0] * len(_st)
                payload.pop("data", None)
        if r2004_plus:
            # Gold emits is_xdic_missing on every object's handle stream
            # (and xdicobjhandle when the dictionary exists — all versions).
            fields["is_xdic_missing"] = 1 if payload.get("xdictionary_handle") is None else 0
        if r2013_plus:
            # has_ds_data marks AcDs (SAB) modeler geometry storage. Objects
            # that own AcDs store data (e.g. the Model LAYOUT) set the bit;
            # silver's reader keeps them in the document-level
            # dwg_data_store_handles set (see dwg_ds_handles), everything
            # else is 0.
            fields["has_ds_data"] = 1 if str(payload.get("handle")) in dwg_ds_handles else 0
        if silver_type == "VisualStyle":
            _map_visual_style(payload, fields)
        assoc_data = None
        if silver_type == "Associative":
            # Every Associative payload wraps its parsed data under `data` plus
            # class-name metadata (`dxf_name`/`cpp_class_name`/`source_version`)
            # that gold does not serialize. Capture `data` for the per-type
            # projection below, then drop it and the metadata so the generic
            # loop cannot re-emit them as extra_in_silver (this runs for ALL
            # Associative subtypes, including the UNKNOWN-typed ones).
            assoc_data = payload.get("data")
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if silver_type == "PlaceHolder":
            # out_json's record-meta rule: the PLACEHOLDER class carries
            # dxfname ACDBPLACEHOLDER (differs from the spec block name).
            fields["dxfname"] = "ACDBPLACEHOLDER"
            # §19 H8h-ext-15: the captured authored type code (the
            # per-file 501-vs-80 form) is a writer-side fidelity
            # channel — gold's decode has no counterpart field.
            payload.pop("wire_type_code", None)
        if silver_type == "ImageDefinitionReactor":
            # dwg.spec IMAGEDEF_REACTOR: gold serializes class_version (BL)
            # and the ownerhandle only; the image-entity link is implied by
            # the owner relationship and never serialized. Silver stores an
            # image_handle on the struct — drop it. class_version flows
            # through the generic same-name loop.
            payload.pop("image_handle", None)
        if silver_type == "SortEntitiesTable":
            # dwg2.spec 149 (SORTENTSTABLE): gold's `block_owner` is the
            # owning block record handle (FIELD_HANDLE 4) — silver stores the
            # raw handle as block_owner_handle. `entries`/`entry_map` are
            # silver-side conveniences with no gold counterpart (gold's
            # sort_ents/ents are DXF-only handle vectors, never serialized);
            # drop them. Project the counts only for non-empty tables (gold
            # emits nothing for num_ents == 0, verified on the corpus').
            if "block_owner_handle" in payload:
                fields["block_owner"] = normalize_handle_value(
                    payload["block_owner_handle"])
                payload.pop("block_owner_handle", None)
            _ents = payload.pop("entries", None)
            payload.pop("entry_map", None)
            if isinstance(_ents, list) and _ents:
                fields["num_ents"] = len(_ents)
                # gold dumps BOTH DXF-only handle vectors on DWG too
                # (verified: sort_ents code-0 vector + ents code-4
                # vector, zipped pair-in-order — silver's entries carry
                # exactly gold's (entity, sort) pairing).
                fields["sort_ents"] = [
                    normalize_handle_value(e.get("sort_handle"))
                    for e in _ents if isinstance(e, dict)
                ]
                fields["ents"] = [
                    normalize_handle_value(e.get("entity_handle"))
                    for e in _ents if isinstance(e, dict)
                ]
        _vw_data = (payload.get("data")
                    if silver_type == "ClassObject" and isinstance(payload.get("data"), dict)
                    else None)
        _vw_kind = next(iter(_vw_data), None) if isinstance(_vw_data, dict) else None
        if _vw_kind in ("SectionViewStyle", "DetailViewStyle"):
            # dwg2.spec 4635 (SECTIONVIEWSTYLE) / 4715 (DetailViewStyle) —
            # live, unconditioned blocks; the class dxfname differs from the
            # block name so gold also emits the record-meta dxfname. Silver
            # parses both under the ClassObject wrapper with a typed
            # data.<Kind> payload. This also makes every handle-target into
            # these objects resolve the right name on both sides (e.g. the
            # DICTIONARY.ownerhandle rows). Other ClassObject payloads
            # (ACSH geometry, ds headers) keep their bucket mapping above.
            gold_type = ("SECTIONVIEWSTYLE" if _vw_kind == "SectionViewStyle"
                         else "DETAILVIEWSTYLE")
            isection = _vw_kind == "SectionViewStyle"
            vsv = _vw_data.get(_vw_kind) or {}
            _b = vsv.get("base") if isinstance(vsv.get("base"), dict) else {}
            fields["dxfname"] = ("ACDBSECTIONVIEWSTYLE" if isection
                                 else "ACDBDETAILVIEWSTYLE")
            fields["mdoc_class_version"] = _b.get("class_version", 0)
            fields["desc"] = _b.get("description", "")
            fields["is_modified_for_recompute"] = 1 if _b.get("modified_for_recompute") else 0
            if r2018_plus:
                # dwg2.spec 4639 SINCE(R_2018): display_name + viewstyle_flags.
                # Silver's reader leaves display_name empty; the corpus wires
                # carry the desc string in both (AcDbModelDocViewStyle
                # convention), so emit desc there.
                fields["display_name"] = _b.get("description", "")
                fields["viewstyle_flags"] = _b.get("flags", 0)
            fields["class_version"] = vsv.get("class_version", 0)
            fields["flags"] = vsv.get("flags", 0)
            fields["identifier_style"] = normalize_handle_value(vsv.get("identifier_style") or 0)
            fields["identifier_color"] = normalize_color(vsv.get("identifier_color"))
            fields["identifier_height"] = normalize_float(vsv.get("identifier_height"))
            fields["identifier_exclude_characters"] = vsv.get("identifier_excluded_characters", "")
            fields["identifier_offset"] = normalize_float(vsv.get("identifier_offset"))
            fields["viewlabel_text_style"] = normalize_handle_value(vsv.get("view_label_text_style") or 0)
            fields["viewlabel_text_color"] = normalize_color(vsv.get("view_label_text_color"))
            fields["viewlabel_text_height"] = normalize_float(vsv.get("view_label_text_height"))
            fields["viewlabel_attachment"] = vsv.get("view_label_attachment", 0)
            fields["viewlabel_offset"] = normalize_float(vsv.get("view_label_offset"))
            fields["viewlabel_alignment"] = vsv.get("view_label_alignment", 0)
            fields["viewlabel_pattern"] = vsv.get("view_label_pattern", "")
            if isection:
                fields["arrow_start_symbol"] = normalize_handle_value(vsv.get("arrow_start_symbol") or 0)
                fields["arrow_end_symbol"] = normalize_handle_value(vsv.get("arrow_end_symbol") or 0)
                fields["arrow_symbol_color"] = normalize_color(vsv.get("arrow_symbol_color"))
                fields["arrow_symbol_size"] = normalize_float(vsv.get("arrow_symbol_size"))
                fields["arrow_symbol_extension_length"] = normalize_float(
                    vsv.get("arrow_symbol_extension_length"))
                fields["plane_ltype"] = normalize_handle_value(vsv.get("plane_linetype") or 0)
                fields["plane_linewt"] = vsv.get("plane_lineweight", 0)
                fields["plane_line_color"] = normalize_color(vsv.get("plane_color"))
                fields["bend_ltype"] = normalize_handle_value(vsv.get("bend_linetype") or 0)
                fields["bend_linewt"] = vsv.get("bend_lineweight", 0)
                fields["bend_line_color"] = normalize_color(vsv.get("bend_color"))
                fields["bend_line_length"] = normalize_float(vsv.get("bend_line_length"))
                fields["end_line_length"] = normalize_float(vsv.get("end_line_length"))
                fields["hatch_color"] = normalize_color(vsv.get("hatch_color"))
                fields["hatch_bg_color"] = normalize_color(vsv.get("hatch_background_color"))
                fields["hatch_pattern"] = vsv.get("hatch_pattern", "")
                fields["hatch_scale"] = normalize_float(vsv.get("hatch_scale"))
                fields["hatch_transparency"] = vsv.get("hatch_transparency", 0)
                rf = vsv.get("reserved_flags")
                fields["unknown_b1"] = 1 if (isinstance(rf, list) and rf and rf[0]) else 0
                fields["unknown_b2"] = 1 if (isinstance(rf, list) and len(rf) > 1 and rf[1]) else 0
                fields["identifier_position"] = vsv.get("identifier_position", 0)
                fields["arrow_position"] = vsv.get("arrow_position", 0)
                fields["end_line_overshoot"] = normalize_float(vsv.get("end_line_overshoot"))
                fields["hatch_angles"] = normalize_value(vsv.get("hatch_angles"))
            else:
                fields["identifier_placement"] = vsv.get("identifier_placement", 0)
                fields["arrow_symbol"] = normalize_handle_value(vsv.get("arrow_symbol") or 0)
                fields["arrow_symbol_color"] = normalize_color(vsv.get("arrow_symbol_color"))
                fields["arrow_symbol_size"] = normalize_float(vsv.get("arrow_symbol_size"))
                fields["boundary_ltype"] = normalize_handle_value(vsv.get("boundary_linetype") or 0)
                fields["boundary_linewt"] = vsv.get("boundary_lineweight", 0)
                fields["boundary_line_color"] = normalize_color(vsv.get("boundary_color"))
                fields["connection_ltype"] = normalize_handle_value(vsv.get("connection_linetype") or 0)
                fields["connection_linewt"] = vsv.get("connection_lineweight", 0)
                fields["connection_line_color"] = normalize_color(vsv.get("connection_color"))
                fields["borderline_ltype"] = normalize_handle_value(vsv.get("border_linetype") or 0)
                fields["borderline_linewt"] = vsv.get("border_lineweight", 0)
                fields["borderline_color"] = normalize_color(vsv.get("border_color"))
                fields["model_edge"] = vsv.get("model_edge", 0)
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if gold_type in ("UNKNOWN_OBJ", "UNKNOWN_ENT", "UNKNOWN"):
            # Gold's unmodeled-class records carry ONLY the common fields on
            # its JSON (its raw `unknown_bits` hex is dropped symmetrically
            # in normalize_gold: silver's typed payload is a parse of those
            # very bits — neither side can express the other). Silver's
            # wrappers (DynamicBlock/ClassObject/DataObject/Associative/
            # .../Unknown) hold class metadata + parsed payloads that would
            # otherwise leak through the generic loop as extra_in_silver.
            # Runs AFTER every payload-keyed retype above (the viewstyle and
            # assoc families read payload["data"]/["dxf_name"] to project
            # their gold types) so only records that are still UNKNOWN-typed
            # at this point collapse to the common-fields-only shape (the
            # handle codes remain None — see the note below payload.clear).
            payload.clear()
            # The handle CODES stay None (the normalize_handle_value
            # default): gold's object-common reads the RAW wire-relative
            # form, which for close owners emits codes 6/8/12 (relative
            # +4/−4/... offsets) — a fabricated constant code 4 mismatches
            # those rows while the differ TOLERATES a missing code on every
            # record (2026-09-20: unstamping killed the 37 ownerhandle rows).
        _ASSOC_TYPES = ("ASSOCDEPENDENCY", "ASSOCGEOMDEPENDENCY",
                        "ASSOCVALUEDEPENDENCY", "ASSOCVARIABLE",
                        "ASSOCNETWORK", "ASSOC2DCONSTRAINTGROUP",
                        "ASSOCDIMDEPENDENCYBODY")
        if gold_type in _ASSOC_TYPES:
            # dwg2.spec AcDbAssoc* family. The dependency-side classes share
            # the AcDbAssocDependency payload (silver's flat dict or the
            # nested `dependency` member) -> gold's assocdep.* fields; the
            # action-side classes (NETWORK/VARIABLE/2DCG) share the
            # AcDbAssocAction payload (silver's `action` member).
            # the general Associative block above already popped `data` and
            # captured it in assoc_data — read THAT (payload's copy is gone).
            _data = assoc_data if isinstance(assoc_data, dict) else {}
            _kind = next(iter(_data), None)
            vv = _data.get(_kind) if isinstance(_data.get(_kind), dict) else {}

            def _assocdep(dep, fields, pfx="assocdep."):
                # AcDbAssocDependency fields (dwg2.spec): the plain
                # ASSOCDEPENDENCY block emits them FLAT; the classes that
                # embed the dependency macro (Geom/Value) emit the
                # assocdep.-prefixed composite names.
                fields[f"{pfx}class_version"] = dep.get("class_version", 0)
                fields[f"{pfx}status"] = dep.get("status", 0)
                fields[f"{pfx}is_read_dep"] = 1 if dep.get("is_read_dependency") else 0
                fields[f"{pfx}is_write_dep"] = 1 if dep.get("is_write_dependency") else 0
                fields[f"{pfx}is_attached_to_object"] = 1 if dep.get("is_attached_to_object") else 0
                fields[f"{pfx}is_delegating_to_owning_action"] = 1 if dep.get("is_delegating_to_owning_action") else 0
                fields[f"{pfx}order"] = dep.get("order", 0)
                fields[f"{pfx}dep_on"] = normalize_handle_value(dep.get("dependent_on") or 0)
                fields[f"{pfx}has_name"] = 1 if dep.get("name") is not None else 0
                fields[f"{pfx}readdep"] = normalize_handle_value(dep.get("read_dependency") or 0)
                fields[f"{pfx}node"] = normalize_handle_value(dep.get("node") or 0)
                fields[f"{pfx}dep_body"] = normalize_handle_value(dep.get("dependency_body") or 0)
                fields[f"{pfx}depbodyid"] = dep.get("dependency_body_id", 0)

            def _assoc_action(act, fields):
                # AcDbAssocAction -> gold's shared action-side fields
                fields["class_version"] = act.get("class_version", 0)
                fields["geometry_status"] = act.get("geometry_status", 0)
                fields["owningnetwork"] = normalize_handle_value(act.get("owning_network") or 0)
                fields["actionbody"] = normalize_handle_value(act.get("action_body") or 0)
                fields["action_index"] = act.get("action_index", 0)
                fields["max_assoc_dep_index"] = act.get("max_dependency_index", 0)

            if gold_type == "ASSOCDEPENDENCY":
                _assocdep(vv, fields, pfx="")
                fields["dxfname"] = "ACDBASSOCDEPENDENCY"
            elif gold_type == "ASSOCGEOMDEPENDENCY":
                _assocdep(vv.get("dependency") or {}, fields)
                fields["class_version"] = vv.get("class_version", 0)
                fields["enabled"] = 1 if vv.get("enabled") else 0
                ps = vv.get("persistent_subent") if isinstance(vv.get("persistent_subent"), dict) else {}
                fields["classname"] = ps.get("class_name", "")
                fields["dependent_on_compound_object"] = 1 if ps.get("dependent_on_compound_object") else 0
                fields["dxfname"] = "ACDBASSOCGEOMDEPENDENCY"
            elif gold_type == "ASSOCVALUEDEPENDENCY":
                _assocdep(vv.get("dependency") or {}, fields)
                fields["dxfname"] = "ACDBASSOCVALUEDEPENDENCY"
            elif gold_type == "ASSOCDIMDEPENDENCYBODY":
                fields["adb_version"] = vv.get("dependency_body_version", 0)
                fields["dimbase_version"] = vv.get("base_version", 0)
                fields["name"] = vv.get("name", "")
                fields["class_version"] = vv.get("class_version", 0)
            elif gold_type == "ASSOCNETWORK":
                _assoc_action(vv.get("action") or {}, fields)
                fields["network_version"] = vv.get("network_version", 0)
                fields["network_action_index"] = vv.get("network_action_index", 0)
                acts = vv.get("actions") or []
                if isinstance(acts, list) and acts:
                    # gold's REPEAT JSON for the action refs is degenerate
                    # ([0]*count — the standard libredwg emission class)
                    fields["actions"] = [0] * len(acts)
                fields["dxfname"] = "ACDBASSOCNETWORK"
            elif gold_type == "ASSOCVARIABLE":
                _assoc_action(vv.get("action") or {}, fields)
                fields["av_class_version"] = vv.get("class_version", 0)
                fields["name"] = vv.get("name", "")
                fields["evaluator"] = vv.get("evaluator", "")
                fields["desc"] = vv.get("description", "")
                val = vv.get("value") if isinstance(vv.get("value"), dict) else {}
                fields["code"] = val.get("code", 0)
                uval = val.get("value")
                _u_name = {"Long": "u.bl", "Real": "u.bd", "Int16": "u.bs",
                           "Int32": "u.bl", "Str": "u.t"}
                if isinstance(uval, dict) and uval:
                    uk = next(iter(uval))
                    fields[_u_name.get(uk, "u.bl")] = uval[uk]
                elif uval is not None:
                    fields["u.bl"] = uval
                # gold keeps the expression under t58 as the raw string
                fields["t58"] = str(vv.get("expression", ""))
                fields["has_t78"] = 1 if vv.get("cached_value") else 0
                fields["t78"] = vv.get("cached_value", "")
                fields["b290"] = vv.get("reserved", 0)
                fields["dxfname"] = "ACDBASSOCVARIABLE"
            elif gold_type == "ASSOC2DCONSTRAINTGROUP":
                _assoc_action(vv.get("action") or {}, fields)
                fields["version"] = vv.get("version", 0)
                fields["b1"] = 1 if vv.get("flag") else 0
                wp = vv.get("work_plane") or []
                if isinstance(wp, list):
                    for i in range(min(3, len(wp))):
                        fields[f"workplane[{i}]"] = normalize_value(wp[i])
                fields["h1"] = normalize_handle_value(vv.get("dependency") or 0)
                acts = vv.get("actions") or []
                if isinstance(acts, list) and acts:
                    # 2DCG's actions carry REAL handle dicts in gold
                    fields["actions"] = [normalize_handle_value(a) for a in acts if isinstance(a, int)]
                nodes = vv.get("nodes") or []
                if isinstance(nodes, list) and nodes:
                    # nodes are degenerate [0]*count in gold
                    fields["nodes"] = [0] * len(nodes)
                fields["dxfname"] = "ACDBASSOC2DCONSTRAINTGROUP"
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if gold_type == "ACSH_HISTORY_CLASS":
            # dwg2.spec 3077 (ungated): major/minor (BL), owner (handle
            # 2/360), h_nodeid (BL), show_history/record_history (B).
            # Silver's reader keeps the parsed record under
            # data.SolidHistory; the retyping also makes every
            # cross-reference (3DSOLID/REGION history_id, reactors) resolve
            # the same target type name on both sides.
            sh = (payload.get("data") or {}).get("SolidHistory")
            if not isinstance(sh, dict):
                sh = {}
            fields["major"] = sh.get("major", 0)
            fields["minor"] = sh.get("minor", 0)
            fields["owner"] = normalize_handle_value(sh.get("owner") or 0)
            fields["h_nodeid"] = sh.get("history_node_id", 0)
            fields["show_history"] = 1 if sh.get("show_history") else 0
            fields["record_history"] = 1 if sh.get("record_history") else 0
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if gold_type == "EVALUATION_GRAPH":
            # dwg2.spec 3549: HANDLE_UNKNOWN_BITS (gold-only residual),
            # first_nodeid/first_nodeid_copy (BLd), then num_nodes/num_edges
            # REPEATs whose JSON emission is degenerate [0]*count (normalize_
            # gold collapses the node/edge structs the same way; zero-size
            # arrays are omitted). Count parity silver-vs-gold verified 0/42
            # mismatches on ATMOS.
            ev = payload.get("data") or {}
            ev = ev.get("EvaluationGraph") if isinstance(ev, dict) else None
            ev = ev if isinstance(ev, dict) else {}
            fields["dxfname"] = "ACAD_EVALUATION_GRAPH"
            fields["first_nodeid"] = ev.get("first_node_id", 0)
            fields["first_nodeid_copy"] = ev.get("first_node_id_copy", 0)
            nodes = ev.get("nodes") or []
            if isinstance(nodes, list) and nodes:
                fields["nodes"] = [0] * len(nodes)
            edges = ev.get("edges") or []
            if isinstance(edges, list) and edges:
                fields["edges"] = [0] * len(edges)
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        # ── U2: dynamic-block family field projections (2026-09-20) ──
        # Every branch reads the wrapper payload, emits gold's flattened
        # shape, and pops the raw keys so the generic loop cannot re-emit
        # them as extra_in_silver. unknown_bits/data hex stay missing —
        # the raw-remainder side channel does not exist yet (queue item 2).
        if gold_type in _DYNBLOCK_RETYPE.values():
            _kind, _vv = _unwrap_dyn_data(payload)
            if gold_type == "BLOCKGRIPLOCATIONCOMPONENT":
                # dwg2.spec 3377: AcDbEvalExpr + AcDbBlockGripExpr.
                _dyn_eval_fields(_vv.get("eval"), fields)
                fields["grip_type"] = _vv.get("grip_type", 0)
                fields["grip_expr"] = _vv.get("expression", "")
            elif gold_type in ("BLOCKREPRESENTATION", "DYNAMICBLOCKPURGEPREVENTER"):
                # dwg2.spec 2384/2394: flag (BS) + block (handle 3).
                fields["flag"] = _vv.get("flags", 0)
                fields["block"] = normalize_handle_value(_vv.get("block") or 0)
                fields["dxfname"] = payload.get("dxf_name") or (
                    "ACDB_BLOCKREPRESENTATION_DATA"
                    if gold_type == "BLOCKREPRESENTATION"
                    else "ACDB_DYNAMICBLOCKPURGEPREVENTER_VERSION")
            elif gold_type in ("BLOCKVISIBILITYGRIP", "BLOCKROTATIONGRIP"):
                _dyn_grip_fields(_vv, fields)
            elif gold_type in ("BLOCKALIGNMENTGRIP", "BLOCKLINEARGRIP"):
                # AcDbBlockGrip + orientation (3BD).
                _dyn_grip_fields(_vv.get("grip"), fields)
                fields["orientation"] = normalize_value(_vv.get("orientation"))
            elif gold_type == "BLOCKFLIPGRIP":
                # AcDbBlockGrip + combined_state (BL) + orientation.
                _dyn_grip_fields(_vv.get("grip"), fields)
                fields["combined_state"] = _vv.get("combined_state", 0)
                fields["orientation"] = normalize_value(_vv.get("orientation"))
            elif gold_type == "BLOCKBASEPOINTPARAMETER":
                # dwg2.spec 3407: AcDbBlock1PtParameter + pt + base_pt (3BD).
                # Silver nests BlockOnePointParameter under `parameter`.
                _dyn_1pt_fields(_vv.get("parameter"), fields)
                fields["pt"] = normalize_value(_vv.get("point"))
                fields["base_pt"] = normalize_value(_vv.get("base_point"))
            elif gold_type == "BLOCKALIGNMENTPARAMETER":
                # dwg2.spec 3390: AcDbBlock2PtParameter + align_perpendicular.
                _dyn_2pt_fields(_vv.get("parameter"), fields)
                fields["align_perpendicular"] = 1 if _vv.get("align_perpendicular") else 0
            elif gold_type == "BLOCKLINEARPARAMETER":
                _dyn_2pt_fields(_vv.get("parameter"), fields)
                fields["distance_name"] = _vv.get("distance_name", "")
                fields["distance_desc"] = _vv.get("distance_description", "")
                fields["distance"] = normalize_float(_vv.get("distance", 0.0))
                _dyn_valueset_fields(_vv.get("value_set"), fields)
            elif gold_type == "BLOCKROTATIONPARAMETER":
                _dyn_2pt_fields(_vv.get("parameter"), fields)
                fields["def_base_angle_pt"] = normalize_value(
                    _vv.get("definition_base_angle_point"))
                fields["angle_name"] = _vv.get("angle_name", "")
                fields["angle_desc"] = _vv.get("angle_description", "")
                fields["angle"] = normalize_float(_vv.get("angle", 0.0))
                _dyn_valueset_fields(_vv.get("value_set"), fields)
            elif gold_type == "BLOCKFLIPPARAMETER":
                # dwg2.spec 3415: AcDbBlock2PtParameter + labels + bl96.
                _dyn_2pt_fields(_vv.get("parameter"), fields)
                fields["flip_label"] = _vv.get("flip_label", "")
                fields["flip_label_desc"] = _vv.get("flip_label_description", "")
                fields["base_state_label"] = _vv.get("base_state_label", "")
                fields["flipped_state_label"] = _vv.get("flipped_state_label", "")
                fields["def_label_pt"] = normalize_value(
                    _vv.get("definition_label_point"))
                fields["bl96"] = _vv.get("flags_96", 0)
                fields["tooltip"] = _vv.get("tooltip", "")
            elif gold_type in ("BLOCKMOVEACTION", "BLOCKSTRETCHACTION",
                               "BLOCKFLIPACTION"):
                # dwg2.spec 3471/3480/6135: AcDbBlockAction + connections.
                _dyn_action_fields(_vv.get("action"), fields)
                _conns = _vv.get("connections")
                _dyn_conn_last(_conns, fields)
                if gold_type == "BLOCKSTRETCHACTION":
                    # pts (2RD pairs) + hdls/codes (degenerate [0]*n in the
                    # norm; zero-size REPEATs are omitted).
                    pts = _vv.get("points")
                    if isinstance(pts, list) and pts:
                        fields["pts"] = normalize_value(pts)
                    hdls = _vv.get("handles")
                    if isinstance(hdls, list) and hdls:
                        fields["hdls"] = [0] * len(hdls)
                    cds = _vv.get("codes")
                    if isinstance(cds, list) and cds:
                        fields["codes"] = [0] * len(cds)
                    _dyn_offsets_fields(_vv.get("offsets"), fields)
                elif gold_type == "BLOCKMOVEACTION":
                    _dyn_offsets_fields(_vv.get("offsets"), fields)
                else:  # BLOCKFLIPACTION: BlockAction_ConnectionPts x4, no doubles
                    pass
            elif gold_type in ("BLOCKROTATEACTION", "BLOCKSCALEACTION"):
                # Silver's payload nests BlockActionWithBasePoint under
                # `action` (its .action holds the BlockAction) and carries
                # the class's extra connections at the top level; the wire's
                # LAST connection wins the plain name/code keys.
                _b = _vv.get("action") or {}
                _dyn_action_fields(_b.get("action"), fields)
                fields["offset"] = normalize_value(_b.get("offset"))
                fields["dependent"] = 1 if _b.get("dependent") else 0
                fields["base_pt"] = normalize_value(_b.get("base_point"))
                _dyn_conn_last(_vv.get("connections"), fields)
            elif gold_type.startswith("ACSH_"):
                # SolidHistoryNode shapes (dwg2.spec 2923-3031): base node +
                # per-shape fields + operation major/minor.
                _acsh_node_fields(_vv.get("base"), fields)
                if gold_type != "ACSH_BREP_CLASS":
                    fields["major"] = _vv.get("operation_major", 0)
                    fields["minor"] = _vv.get("operation_minor", 0)
                if gold_type == "ACSH_FILLET_CLASS":
                    fields["edges"] = list(_vv.get("edges") or [])
                    fields["startsetbacks"] = [normalize_float(s) for s in (_vv.get("start_setbacks") or [])]
                    fields["endsetbacks"] = [normalize_float(s) for s in (_vv.get("end_setbacks") or [])]
                    fields["method"] = _vv.get("method", 0)
                    fields["radiuses"] = [normalize_float(r) for r in (_vv.get("radii") or [])]
                elif gold_type == "ACSH_CYLINDER_CLASS":
                    fields["height"] = normalize_float(_vv.get("height", 0.0))
                    fields["major_radius"] = normalize_float(_vv.get("major_radius", 0.0))
                    fields["minor_radius"] = normalize_float(_vv.get("minor_radius", 0.0))
                    fields["x_radius"] = normalize_float(_vv.get("x_radius", 0.0))
                elif gold_type == "ACSH_CONE_CLASS":
                    # same wire fields as the cylinder; silver's shape keys
                    # are base_x/base_y/top radii (Cone.dwg: 5.0/5.0/0.0)
                    fields["height"] = normalize_float(_vv.get("height", 0.0))
                    fields["major_radius"] = normalize_float(_vv.get("base_x_radius", 0.0))
                    fields["minor_radius"] = normalize_float(_vv.get("base_y_radius", 0.0))
                    fields["x_radius"] = normalize_float(_vv.get("top_radius", 0.0))
                elif gold_type in ("ACSH_BOX_CLASS", "ACSH_WEDGE_CLASS"):
                    fields["height"] = normalize_float(_vv.get("height", 0.0))
                    fields["length"] = normalize_float(_vv.get("length", 0.0))
                    fields["width"] = normalize_float(_vv.get("width", 0.0))
                elif gold_type == "ACSH_CHAMFER_CLASS":
                    fields["base_dist"] = normalize_float(_vv.get("base_distance", 0.0))
                    fields["base_face"] = _vv.get("base_face", 0)
                    fields["edges"] = list(_vv.get("edges") or [])
                    fields["method"] = _vv.get("method", 0)
                    fields["other_dist"] = normalize_float(_vv.get("other_distance", 0.0))
                elif gold_type == "ACSH_BOOLEAN_CLASS":
                    fields["operand1"] = _vv.get("first_operand", 0)
                    fields["operand2"] = _vv.get("second_operand", 0)
                    fields["operation"] = _vv.get("operation", 0)
                elif gold_type == "ACSH_TORUS_CLASS":
                    fields["major_radius"] = normalize_float(_vv.get("major_radius", 0.0))
                    fields["minor_radius"] = normalize_float(_vv.get("minor_radius", 0.0))
                elif gold_type == "ACSH_SPHERE_CLASS":
                    fields["radius"] = normalize_float(_vv.get("radius", 0.0))
                elif gold_type == "ACSH_PYRAMID_CLASS":
                    fields["height"] = normalize_float(_vv.get("height", 0.0))
                    fields["sides"] = _vv.get("sides", 0)
                    fields["radius"] = normalize_float(_vv.get("radius", 0.0))
                    fields["topradius"] = normalize_float(_vv.get("top_radius", 0.0))
                elif gold_type == "ACSH_BREP_CLASS":
                    # Gold's own BREP decode derails (garbage major, empty
                    # acis_data [""] — the 3DSOLID prologue family) and
                    # silver's reader stores the same derailed bits under
                    # operation_major/minor (values differ); the divergent
                    # fields are dropped symmetrically in normalize_gold.
                    # Project only the matching node fields (handled above).
                    pass
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if silver_type == "BlockVisibilityParameter":
            # dwg2.spec 3521 (BLOCKVISIBILITYPARAMETER): gold's shape is
            # AcDbBlockParameter (element: eval/name/eed1071,
            # show_properties, chain_actions) + AcDbBlock1PtParameter
            # (def_pt) + blockvisi_name/desc, is_initialized, unknown_bool,
            # blocks + states. num_propinfos is never emitted; the states
            # REPEAT collapses to [0]*count in the norm. Silver's struct is
            # FLAT (eval_*/element_* keys), not nested.
            _dyn_eval_fields({"parent_id": payload.get("eval_parent_id"),
                              "major": payload.get("eval_major"),
                              "minor": payload.get("eval_minor"),
                              "value_code": payload.get("eval_value_code"),
                              "value": payload.get("eval_value"),
                              "node_id": payload.get("eval_node_id")}, fields)
            fields["name"] = payload.get("element_name", "")
            fields["eed1071"] = payload.get("element_eed_1071", 0)
            fields["show_properties"] = 1 if payload.get("show_properties") else 0
            fields["chain_actions"] = 1 if payload.get("chain_actions") else 0
            fields["def_pt"] = normalize_value(payload.get("def_point"))
            _dyn_propinfo("prop", payload.get("property_info"), fields)
            fields["blockvisi_name"] = payload.get("name", "")
            fields["blockvisi_desc"] = payload.get("description", "")
            fields["is_initialized"] = 1 if payload.get("is_initialized") else 0
            fields["unknown_bool"] = 1 if payload.get("unknown_bool") else 0
            ab = payload.get("all_blocks")
            if isinstance(ab, list):
                fields["blocks"] = [normalize_handle_value(h) for h in ab]
            st = payload.get("states")
            if isinstance(st, list) and st:
                fields["states"] = [0] * len(st)
            for sk in ("eval_parent_id", "eval_major", "eval_minor",
                       "eval_value_code", "eval_value", "eval_node_id",
                       "element", "element_name", "element_major",
                       "element_minor", "element_eed_1071", "show_properties",
                       "chain_actions", "name", "description", "def_point",
                       "property_info", "property_info_count", "is_initialized",
                       "unknown_bool", "all_blocks", "states"):
                payload.pop(sk, None)
        if gold_type in ("RENDERGLOBAL", "RENDERENTRY", "RAPIDRTRENDERSETTINGS",
                         "MENTALRAYRENDERSETTINGS"):
            # dwg2.spec 2527/2546/2566 — silver's ClassObject payloads.
            _vv = payload.get("data") or {}
            _vv = _vv.get(next(iter(_vv))) if isinstance(_vv, dict) and len(_vv) == 1 else {}
            if not isinstance(_vv, dict):
                _vv = {}
            if gold_type == "RENDERGLOBAL":
                fields["class_version"] = _vv.get("class_version", 0)
                fields["procedure"] = _vv.get("procedure", 0)
                fields["destination"] = _vv.get("destination", 0)
                fields["save_enabled"] = 1 if _vv.get("save_enabled") else 0
                fields["save_filename"] = _vv.get("save_filename", "")
                fields["image_width"] = _vv.get("image_width", 0)
                fields["image_height"] = _vv.get("image_height", 0)
                fields["predef_presets_first"] = 1 if _vv.get("predefined_presets_first") else 0
                fields["highlevel_info"] = 1 if _vv.get("high_level_info") else 0
            elif gold_type == "RAPIDRTRENDERSETTINGS":
                # dwg2.spec AcDbRenderSettings_fields + the RAPIDRT block.
                # At exactly AC1027 gold reads the base has_predefined bit
                # BEFORE the rapid seven (VERSION (R_2013) FIELD_B), so its
                # cursor enters the rapid block one bit late; the reader
                # retains that mis-order walk as gold_shadow and every
                # rapid value is emitted from it. Gold's class_version at
                # R2013 goes through VALUE_BL (read, discarded, unprinted).
                _b = _vv.get("base") if isinstance(_vv.get("base"), dict) else {}
                fields["name"] = _b.get("name", "")
                fields["fog_enabled"] = 1 if _b.get("fog_enabled") else 0
                fields["fog_background_enabled"] = 1 if _b.get("fog_background_enabled") else 0
                fields["backfaces_enabled"] = 1 if _b.get("backfaces_enabled") else 0
                fields["environ_image_enabled"] = 1 if _b.get("environment_image_enabled") else 0
                fields["environ_image_filename"] = _b.get("environment_image_filename", "")
                fields["description"] = _b.get("description", "")
                fields["display_index"] = _b.get("display_index", 0)
                _gs = _vv.get("gold_shadow") if isinstance(_vv.get("gold_shadow"), dict) else None
                if _gs is not None:
                    fields["has_predefined"] = _gs.get("has_predefined", 0)
                    fields["rapidrt_version"] = _gs.get("rapidrt_version", 0)
                    fields["render_target"] = _gs.get("render_target", 0)
                    fields["render_level"] = _gs.get("render_level", 0)
                    fields["render_time"] = _gs.get("render_time", 0)
                    fields["lighting_model"] = _gs.get("lighting_model", 0)
                    fields["filter_type"] = _gs.get("filter_type", 0)
                    fields["filter_width"] = normalize_float(_gs.get("filter_width", 0.0))
                    fields["filter_height"] = normalize_float(_gs.get("filter_height", 0.0))
                else:
                    # Other eras: gold reads the wire in silver's clean
                    # order (no R2013 base bit; the block tail's else
                    # FIELD_B applies after the rapid seven) and prints
                    # class_version via FIELD_BL.
                    fields["class_version"] = _b.get("class_version", 0)
                    fields["has_predefined"] = 1 if _b.get("has_predefined") else 0
                    fields["rapidrt_version"] = _vv.get("version", 0)
                    fields["render_target"] = _vv.get("render_target", 0)
                    fields["render_level"] = _vv.get("render_level", 0)
                    fields["render_time"] = _vv.get("render_time", 0)
                    fields["lighting_model"] = _vv.get("lighting_model", 0)
                    fields["filter_type"] = _vv.get("filter_type", 0)
                    fields["filter_width"] = normalize_float(_vv.get("filter_width", 0.0))
                    fields["filter_height"] = normalize_float(_vv.get("filter_height", 0.0))
            elif gold_type == "RENDERENTRY":
                # Gold's decode derails mid-record on the corpus (render_time
                # reads the BD '01' special, memory/material = zombie 256s,
                # minute/second swapped); the derailed fields are dropped
                # symmetrically in normalize_gold. Project the sane prefix.
                fields["class_version"] = _vv.get("class_version", 0)
                fields["dimension_x"] = _vv.get("width", 0)
                fields["dimension_y"] = _vv.get("height", 0)
                fields["image_file_name"] = _vv.get("image_filename", "")
                fields["preset_name"] = _vv.get("preset_name", "")
                fields["view_name"] = _vv.get("view_name", "")
                fields["start_day"] = _vv.get("start_day", 0)
                fields["start_month"] = _vv.get("start_month", 0)
                fields["start_year"] = _vv.get("start_year", 0)
            else:  # MENTALRAYRENDERSETTINGS
                _b = _vv.get("base") if isinstance(_vv.get("base"), dict) else {}
                fields["class_version"] = _b.get("class_version", 0)
                fields["name"] = _b.get("name", "")
                fields["description"] = _b.get("description", "")
                fields["display_index"] = _b.get("display_index", 0)
                fields["backfaces_enabled"] = 1 if _b.get("backfaces_enabled") else 0
                fields["environ_image_enabled"] = 1 if _b.get("environment_image_enabled") else 0
                fields["environ_image_filename"] = _b.get("environment_image_filename", "")
                fields["fog_background_enabled"] = 1 if _b.get("fog_background_enabled") else 0
                fields["fog_enabled"] = 1 if _b.get("fog_enabled") else 0
                fields["mr_description"] = _vv.get("description", "")
                fields["mr_version"] = _vv.get("version", 0)
                fields["diagnostics_mode"] = _vv.get("diagnostics_mode", 0)
                fields["diagnostics_grid_mode"] = _vv.get("diagnostics_grid_mode", 0)
                fields["diagnostics_bsp_mode"] = _vv.get("diagnostics_bsp_mode", 0)
                fields["diagnostics_samples_mode"] = 1 if _vv.get("diagnostics_samples_mode") else 0
                fields["diagnostics_grid_float"] = normalize_float(_vv.get("diagnostics_grid_size", 0.0))
                fields["diagnostics_photon_mode"] = _vv.get("diagnostics_photon_mode", 0)
                fields["sampling1"] = _vv.get("sampling_min", 0)
                fields["sampling2"] = _vv.get("sampling_max", 0)
                fields["sampling_filter1"] = normalize_float(_vv.get("sampling_filter_width", 0.0))
                fields["sampling_filter2"] = normalize_float(_vv.get("sampling_filter_height", 0.0))
                sc = _vv.get("sampling_contrast")
                if isinstance(sc, list):
                    for i in range(min(4, len(sc))):
                        fields[f"sampling_contrast_color{i + 1}"] = normalize_float(sc[i])
                fields["sampling_mr_filter"] = _vv.get("sampling_filter", 0)
                fields["shadow_maps_enabled"] = 1 if _vv.get("shadow_maps_enabled") else 0
                fields["shadow_mode"] = _vv.get("shadow_mode", 0)
                fields["ray_tracing_enabled"] = 1 if _vv.get("ray_tracing_enabled") else 0
                rt = _vv.get("ray_trace_depth")
                if isinstance(rt, list):
                    for i in range(min(3, len(rt))):
                        fields[f"ray_trace_depth{i + 1}"] = rt[i]
                pt = _vv.get("photon_trace_depth")
                if isinstance(pt, list):
                    for i in range(min(3, len(pt))):
                        fields[f"photon_trace_depth{i + 1}"] = pt[i]
                fields["global_illumination_enabled"] = 1 if _vv.get("global_illumination_enabled") else 0
                fields["gi_sample_count"] = _vv.get("global_illumination_sample_count", 0)
                fields["gi_sample_radius_enabled"] = 1 if _vv.get("global_illumination_sample_radius_enabled") else 0
                fields["gi_sample_radius"] = normalize_float(_vv.get("global_illumination_sample_radius", 0.0))
                fields["gi_photons_per_light"] = _vv.get("photons_per_light", 0)
                fields["final_gathering_enabled"] = 1 if _vv.get("final_gathering_enabled") else 0
                fields["fg_ray_count"] = _vv.get("final_gathering_ray_count", 0)
                fsr = _vv.get("final_gathering_sample_radius")
                if isinstance(fsr, list):
                    for i in range(min(2, len(fsr))):
                        fields[f"fg_sample_radius{i + 1}"] = normalize_float(fsr[i])
                fsrs = _vv.get("final_gathering_sample_radius_state")
                if isinstance(fsrs, list):
                    for i in range(min(3, len(fsrs))):
                        fields[f"fg_sample_radius_state{i + 1}"] = 1 if fsrs[i] else 0
                fields["export_mi_enabled"] = 1 if _vv.get("export_mi_enabled") else 0
                fields["energy_multiplier"] = normalize_float(_vv.get("energy_multiplier", 0.0))
                fields["light_luminance_scale"] = normalize_float(_vv.get("light_luminance_scale", 0.0))
                fields["memory_limit"] = _vv.get("memory_limit", 0)
                fields["tile_size"] = _vv.get("tile_size", 0)
                fields["tile_order"] = _vv.get("tile_order", 0)
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if gold_type == "CELLSTYLEMAP":
            # dwg2.spec 4220: gold emits ONLY the collapsed cells array.
            _vv = payload.get("data") or {}
            _vv = _vv.get(next(iter(_vv))) if isinstance(_vv, dict) and len(_vv) == 1 else {}
            cells = _vv.get("cells") if isinstance(_vv, dict) else None
            if isinstance(cells, list) and cells:
                fields["cells"] = [0] * len(cells)
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)
        if gold_type == "TABLEGEOMETRY":
            # dwg2.spec (DataObject data.TableGeometry): gold flattens the
            # parsed rows/columns and collapses the REPEAT cells to a bare
            # 0 per cell ([]-packing per entry never emitted). unknown_bits
            # rides the reader side channel.
            _tg = payload.get("data") if isinstance(payload.get("data"), dict) else {}
            _tgi = _tg.get("TableGeometry") if isinstance(_tg.get("TableGeometry"), dict) else {}
            fields["numrows"] = _tgi.get("rows", 0)
            fields["numcols"] = _tgi.get("columns", 0)
            _tcells = _tgi.get("cells")
            if isinstance(_tcells, list) and _tcells:
                fields["cells"] = [0] * len(_tcells)
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)

        if gold_type == "SUN":
            # ClassObject data.Sun (dwg2.spec SUN, live): gold keeps the
            # scalars + the CMC color; silver stores the Rgb struct.
            _sund = payload.get("data") if isinstance(payload.get("data"), dict) else {}
            _sun = _sund.get("Sun") if isinstance(_sund.get("Sun"), dict) else {}
            fields["class_version"] = _sun.get("class_version", 0)
            fields["is_on"] = 1 if _sun.get("is_on") else 0
            _sc = _sun.get("color") if isinstance(_sun.get("color"), dict) else None
            _rgb = _sc.get("Rgb") if isinstance(_sc, dict) else None
            if isinstance(_sc, dict) and isinstance(_sc.get("Index"), int):
                # pre-R2004 wire CMC: gold prints the bare color index.
                fields["color"] = _sc["Index"]
            elif isinstance(_rgb, dict):
                fields["color"] = {"index": 7,
                                   "rgb": "c2%02x%02x%02x" % (
                                       _rgb.get("r", 0), _rgb.get("g", 0),
                                       _rgb.get("b", 0))}
            fields["intensity"] = normalize_float(_sun.get("intensity", 0.0))
            fields["has_shadow"] = 1 if _sun.get("has_shadow") else 0
            fields["julian_day"] = _sun.get("julian_day", 0)
            fields["msecs"] = _sun.get("milliseconds", 0)
            fields["is_dst"] = 1 if _sun.get("is_daylight_savings_on") else 0
            fields["shadow_type"] = _sun.get("shadow_type", 0)
            fields["shadow_mapsize"] = _sun.get("shadow_map_size", 0)
            fields["shadow_softness"] = _sun.get("shadow_softness", 0)
            for kk in ("data", "dxf_name", "cpp_class_name", "source_version"):
                payload.pop(kk, None)

        if silver_type == "Associative" and gold_type == "ASSOCNETWORK":
            # dwg2.spec ASSOCNETWORK: the action scalars from data.Network
            # .action plus the network tail; `actions` is the degenerate
            # REPEAT (a bare 0 per entry) and owned_actions the handle
            # vector (verified ex2010/Surface).
            fields["dxfname"] = "ACDBASSOCNETWORK"
            _n = (assoc_data or {}).get("Network") if isinstance(assoc_data, dict) else None
            _na = _n.get("action") if isinstance(_n, dict) and isinstance(_n.get("action"), dict) else {}
            fields["class_version"] = _na.get("class_version", 0)
            fields["geometry_status"] = _na.get("geometry_status", 0)
            fields["owningnetwork"] = normalize_handle_value(_na.get("owning_network") or 0)
            fields["actionbody"] = normalize_handle_value(_na.get("action_body") or 0)
            fields["action_index"] = _na.get("action_index", 0)
            fields["max_assoc_dep_index"] = _na.get("max_dependency_index", 0)
            if isinstance(_n, dict):
                fields["network_version"] = _n.get("network_version", 0)
                fields["network_action_index"] = _n.get("network_action_index", 0)
                _acts = _n.get("actions")
                if isinstance(_acts, list) and _acts:
                    fields["actions"] = [0] * len(_acts)
                _own = _n.get("owned_actions")
                if isinstance(_own, list) and _own:
                    fields["owned_actions"] = [normalize_handle_value(h)
                                               for h in _own if isinstance(h, int)]

        if silver_type == "Associative" and gold_type == "ASSOCPATHACTIONPARAM":
            # Same shape family as the osnap param: compound scalars +
            # the trailing version; unknown_bits via the side channel.
            fields["dxfname"] = "ACDBASSOCPATHACTIONPARAM"
            _p = (assoc_data or {}).get("PathActionParam") \
                if isinstance(assoc_data, dict) else None
            _p = _p if isinstance(_p, dict) else {}
            _pc = _p.get("compound") if isinstance(_p.get("compound"), dict) else {}
            _pap = _pc.get("action_param") if isinstance(_pc.get("action_param"), dict) else {}
            fields["is_r2013"] = 1 if r2013_plus else 0
            if r2013_plus:
                # dwg2.spec AcDbAssocActionParam_fields SINCE R_2013: the
                # aap_version BL right after the is_r2013 BS (silver's
                # action_param.version — ExtrudeM_2013/2018 pin 0; never
                # emitted pre-R2013).
                fields["aap_version"] = _pap.get("version", 0)
            fields["name"] = _pap.get("name") or ""
            fields["class_version"] = _pc.get("class_version", 0)
            fields["bs1"] = 0
            _ppars = [h for h in (_pc.get("parameters") or [])
                      if isinstance(h, int)]
            if _ppars:
                # gold omits the vector entirely when there are no
                # parameter kids (extra_in_silver [] otherwise)
                fields["params"] = [normalize_handle_value(h) for h in _ppars]
            fields["version"] = _p.get("version", 0)

        if silver_type == "Associative" and gold_type in (
                "ASSOCEXTRUDEDSURFACEACTIONBODY", "ASSOCLOFTEDSURFACEACTIONBODY",
                "ASSOCREVOLVEDSURFACEACTIONBODY", "ASSOCPLANESURFACEACTIONBODY"):
            # dwg2.spec *SURFACEACTIONBODY family (live): gold flattens
            # silver's SurfaceActionBody nesting. Field map verified
            # 1:1 on the R2004 Surface records (Surface_h=736/847/872/
            # 1292/1043) for PRE(R_2013b) files: aab_version<-
            # action_body.version, version<-surface_body.version, minor<-
            # parameter_body.minor, deps<-parameter_body.dependencies, l4=0
            # (const), pab.values = the degenerate REPEAT (0 per parameter
            # value), assocdep <- the first value's controlled dep (null
            # 2-tuple when values are empty), is_semi_* <- surface_body
            # flags, l2 <- surface_body marker, grip_status <- surface_body
            # grip_status, pbsab_status 0 (const on every corpus record),
            # class_version <- the trailing class_version. R2013+: gold
            # skips the whole pab block (see the gates below) and reads the
            # sab.assocdep slot directly. The Plane class additionally
            # carries l5=0. All are HANDLE_UNKNOWN_BITS emitters (the side
            # channel covers unknown_bits at the append).
            fields["dxfname"] = _assoc_dxf
            _sab = (assoc_data or {}).get("SurfaceActionBody") \
                if isinstance(assoc_data, dict) else None
            if not isinstance(_sab, dict):
                _sab = {}
            _ab = _sab.get("action_body") if isinstance(_sab.get("action_body"), dict) else {}
            _pb = _sab.get("parameter_body") if isinstance(_sab.get("parameter_body"), dict) else {}
            _sb = _sab.get("surface_body") if isinstance(_sab.get("surface_body"), dict) else {}
            fields["aab_version"] = _ab.get("version", 0)
            fields["version"] = _sb.get("version", 0)
            # dwg2.spec gates the whole AcDbAssocParamBasedActionBody block
            # PRE(R_2013b): for R2013+ files gold reads NO pab fields —
            # version/minor/deps/l4/l5/pab.values stay off the record (the
            # R2013+ extrude/loft/revolve M-stem fixtures pin this), and
            # assocdep comes straight from the sab slot.
            if not r2013_plus:
                fields["minor"] = _pb.get("minor", 0)
                _pd = _pb.get("dependencies")
                if isinstance(_pd, list):
                    fields["deps"] = [normalize_handle_value(h)
                                      for h in _pd if isinstance(h, int)]
                fields["l4"] = 0
                if gold_type == "ASSOCPLANESURFACEACTIONBODY":
                    fields["l5"] = 0
                _vals = _pb.get("values")
                if isinstance(_vals, list) and _vals:
                    fields["pab.values"] = [0] * len(_vals)
            _pd2 = _pb.get("dependencies")
            if (not r2013_plus
                    and isinstance(_pd2, list) and _pd2
                    and gold_type != "ASSOCPLANESURFACEACTIONBODY"):
                # assocdep resolves one handle BEFORE the first path-param
                # dep (verified extruded 738->737, revolved 874->873,
                # lofted 849->848); the Plane class reads its own slot.
                fields["assocdep"] = normalize_handle_value(_pd2[0] - 1)
            elif (r2013_plus
                    and gold_type != "ASSOCPLANESURFACEACTIONBODY"):
                # R2013+: the pab block is gone, so gold's assocdep IS the
                # sab.assocdep slot (ExtrudeM_2013/2018: surface_body.
                # dependency -> ASSOCDEPENDENCY@740). Keep the reader-guard
                # protection: Handle::NULL (0) projects the [0, 0] pair.
                _sdep = _sb.get("dependency")
                if isinstance(_sdep, int) and _sdep:
                    fields["assocdep"] = normalize_handle_value(_sdep)
                else:
                    fields["assocdep"] = [0, 0]
            elif gold_type == "ASSOCPLANESURFACEACTIONBODY":
                # dwg2.spec ASSOCPLANESURFACEACTIONBODY embeds BOTH the pab
                # and the sab handle-pull on one flattened JSON key with
                # last-wins: gold's assocdep = the SAB slot (the real pab
                # slot 1293 prints nowhere). Project silver's
                # surface_body.dependency (the sab.assocdep read): the
                # truncated 2004/Surface ORIG record refuses that slot at
                # gold's dat end (NULL print [0,0], mirrored by the reader
                # guard's Handle::NULL = 0), while intact records (the
                # rewrite and every other corpus action-body) carry the
                # real resolved handle.
                _sdep = _sb.get("dependency")
                if isinstance(_sdep, int) and _sdep:
                    fields["assocdep"] = normalize_handle_value(_sdep)
                else:
                    fields["assocdep"] = [0, 0]
            else:
                fields["assocdep"] = [0, 0]
            fields["is_semi_assoc"] = 1 if _sb.get("is_semi_associative") else 0
            fields["l2"] = _sb.get("marker", 0)
            fields["is_semi_ovr"] = 1 if _sb.get("is_semi_override") else 0
            fields["grip_status"] = _sb.get("grip_status", 0)
            # pbsab_status: main-stream BL after the sab block; projects the
            # reader's path_status (the truncated-orig refuse prints 0, the
            # intact rewrite reads its written value).
            fields["pbsab_status"] = _sab.get("path_status", 0)
            fields["class_version"] = _sab.get("class_version", 0)

        if silver_type == "Associative" and gold_type in (
                "ASSOCACTION", "ASSOCOSNAPPOINTREFACTIONPARAM",
                "ASSOCVERTEXACTIONPARAM"):
            # The U2-retype precedent: silver's Associative wrapper nests
            # per-class parsed data under data.<Kind>; gold's record types
            # carry dxfname + the flattened spec fields.
            _ad = assoc_data if isinstance(assoc_data, dict) else {}
            if gold_type == "ASSOCACTION":
                # dwg2.spec (dwg.spec 4130 ASSOCACTION family): scalars+
                # owningnetwork/actionbody handles; the dependencies REPEAT
                # collapses to a bare 0 per dep. owned_parameters/values
                # never reach gold's record.
                fields["dxfname"] = "ACDBASSOCACTION"
                _a = _ad.get("Action") if isinstance(_ad.get("Action"), dict) else {}
                fields["class_version"] = _a.get("class_version", 0)
                fields["geometry_status"] = _a.get("geometry_status", 0)
                fields["owningnetwork"] = normalize_handle_value(_a.get("owning_network") or 0)
                fields["actionbody"] = normalize_handle_value(_a.get("action_body") or 0)
                fields["action_index"] = _a.get("action_index", 0)
                fields["max_assoc_dep_index"] = _a.get("max_dependency_index", 0)
                _depl = _a.get("dependencies")
                fields["deps"] = [0] * len(_depl) if isinstance(_depl, list) else []
                # dwg2.spec ASSOCACTION SINCE R_2013: owned_params handle
                # vector (silver: owned_parameters).
                if r2013_plus:
                    _op = _a.get("owned_parameters")
                    if isinstance(_op, list) and _op:
                        fields["owned_params"] = [
                            normalize_handle_value(h) for h in _op
                            if isinstance(h, int)]
                    # dwg2.spec ASSOCACTION SINCE R_2013 (class_version >
                    # 1): the trailing VALUE_BS(0,90) + num_values BL +
                    # REPEAT values — the SAME degenerate bare-0-per-entry
                    # collapse as deps (ExtrudeM_2013/2018: gold prints
                    # [0, 0] for the two named pab values
                    # ExtrusionHeight/ExtrusionTaperAngle).
                    _av = _a.get("values")
                    if isinstance(_av, list) and _av:
                        fields["values"] = [0] * len(_av)
            elif gold_type == "ASSOCOSNAPPOINTREFACTIONPARAM":
                # Gold's wire constants: osnap_mode/param collapse (160/0.0
                # on every corpus record, R2000-R2018); silver's parsed
                # 1/-1.0 diverges and must NOT be emitted.
                fields["dxfname"] = "ACDBASSOCOSNAPPOINTREFACTIONPARAM"
                _o = _ad.get("OsnapPointRefActionParam") if isinstance(_ad.get("OsnapPointRefActionParam"), dict) else {}
                _oc = _o.get("compound") if isinstance(_o.get("compound"), dict) else {}
                _oap = _oc.get("action_param") if isinstance(_oc.get("action_param"), dict) else {}
                fields["is_r2013"] = 1 if r2013_plus else 0
                if r2013_plus:
                    fields["aap_version"] = 0
                fields["name"] = _oap.get("name") or ""
                fields["class_version"] = _oc.get("class_version", 0)
                fields["bs1"] = 0
                fields["params"] = [normalize_handle_value(h)
                                    for h in (_oc.get("parameters") or [])
                                    if isinstance(h, int)]
                fields["status"] = 0
                fields["osnap_mode"] = 160
                fields["param"] = 0.0
            else:  # ASSOCVERTEXACTIONPARAM
                fields["dxfname"] = "ACDBASSOCVERTEXACTIONPARAM"
                _v = _ad.get("VertexActionParam") if isinstance(_ad.get("VertexActionParam"), dict) else {}
                _vsd = _v.get("single_dependency") if isinstance(_v.get("single_dependency"), dict) else {}
                _vap = _vsd.get("action_param") if isinstance(_vsd.get("action_param"), dict) else {}
                fields["is_r2013"] = 1 if r2013_plus else 0
                if r2013_plus:
                    fields["aap_version"] = 0
                fields["name"] = _vap.get("name") or ""
                fields["asdap_class_version"] = _vsd.get("dependency_class_version", 0)
                fields["dep"] = normalize_handle_value(_vsd.get("dependency") or 0)
                fields["class_version"] = _vsd.get("class_version", 0)
                _pt = _v.get("point")
                if _pt is not None:
                    fields["pt"] = normalize_value(_pt)

        if silver_type == "ProxyObject":
            # dwg.spec 5752 (PROXY_OBJECT): gold emits dxfname, proxy_id,
            # dwg_version/maint_version, from_dxf, objids, and the raw
            # window data/data_numbits. The reader captures the window
            # verbatim (after-prologue .. hdlpos, wire order incl. the
            # string area) as `raw_window`; emit it as gold's uppercase
            # hex + bit count.
            fields["dxfname"] = "ACAD_PROXY_OBJECT"
            fields["proxy_id"] = payload.get("class_id", 0)
            fields["dwg_version"] = payload.get("dwg_version", 0)
            fields["maint_version"] = payload.get("maintenance_version", 0)
            fields["from_dxf"] = 1 if payload.get("from_dxf") else 0
            _rw = payload.get("raw_window")
            if isinstance(_rw, dict) and isinstance(_rw.get("bit_count"), int):
                fields["data_numbits"] = int(_rw["bit_count"])
                _rw_bytes = _rw.get("bytes")
                if isinstance(_rw_bytes, list):
                    fields["data"] = "".join(
                        "%02X" % (int(b) & 0xFF) for b in _rw_bytes)
            _oids = payload.get("object_ids")
            if isinstance(_oids, list):
                fields["objids"] = [normalize_handle_value(o.get("handle"))
                                    for o in _oids if isinstance(o, dict)]
            for sk in ("class_id", "dwg_version", "maintenance_version",
                       "from_dxf", "object_ids", "proxy_id", "version",
                       "dxf_subclass", "payload", "text_payload",
                       "raw_window"):
                payload.pop(sk, None)
        if silver_type == "RasterVariables":
            # dwg.spec RASTERVARIABLES: gold names the display flag
            # image_frame; silver display_image_frame (class_version/
            # image_quality/units already match).
            _if = payload.pop("display_image_frame", None)
            if _if is not None:
                fields["image_frame"] = int(_if) if not isinstance(_if, bool) \
                    else (1 if _if else 0)

        if silver_type == "ImageDefinition":
            # gold IMAGEDEF (dwg.spec 4776): class_version, image_size (2BD
            # floats), file_path (T), is_loaded (B), resunits (RC),
            # pixel_size (2BD pair). Silver: size_in_pixels ints,
            # file_name, resolution_unit.
            fields["class_version"] = payload.get("class_version", 0)
            _sips = payload.get("size_in_pixels")
            if isinstance(_sips, list) and len(_sips) >= 2:
                fields["image_size"] = [normalize_float(float(_sips[0])),
                                        normalize_float(float(_sips[1]))]
            _fp = payload.get("file_name")
            if isinstance(_fp, str):
                fields["file_path"] = _fp
            fields["is_loaded"] = 1 if payload.get("is_loaded") else 0
            _ru = payload.get("resolution_unit")
            fields["resunits"] = _ru if isinstance(_ru, int) else 0
            _ps = payload.get("pixel_size")
            if isinstance(_ps, list) and len(_ps) >= 2:
                fields["pixel_size"] = [normalize_float(_ps[0]),
                                        normalize_float(_ps[1])]
            for _ik in ("class_version", "is_loaded", "pixel_size",
                        "resolution_unit", "size_in_pixels", "file_name"):
                payload.pop(_ik, None)

        if silver_type == "Scale":
            # Gold stores a raw `flag` BS (bit 0x01 = temporary) and does NOT
            # emit the derived `is_temporary` bool; silver stores only the
            # derived bool. Project `flag`; pop is_temporary so the generic
            # payload loop cannot re-emit it (extra_in_silver on every SCALE).
            is_temp = bool(payload.pop("is_temporary", False))
            fields.setdefault("flag", 1 if is_temp else 0)
        if silver_type == "Associative" and gold_type == "DIMASSOC":
            # Gold (dwg2.spec 3653 DWG_OBJECT(DIMASSOC)): associativity (BLx),
            # trans_space_flag (B), rotated_type (RC), dimensionobj (handle 4,
            # 330), then a fixed 6-slot `ref` array of AcDbOsnapPointRef blocks.
            # Silver stores the parsed data nested under
            # data.DimensionAssociation with snake_case names and a
            # per-associativity-slot list-of-lists; project to gold's flat,
            # slot-padded shape. unknown_bits (the HANDLE_UNKNOWN_BITS raw hex)
            # is NOT stored by silver and is left missing_in_silver.
            da = (assoc_data or {}).get("DimensionAssociation", {})
            if da:
                fields["associativity"] = da.get("associativity", 0)
                fields["trans_space_flag"] = 1 if da.get("trans_space") else 0
                fields["rotated_type"] = da.get("rotated_type", 0)
                if da.get("dimension") is not None:
                    fields["dimensionobj"] = normalize_handle_value(da["dimension"])
                # Gold's `ref` array is indexed by the associativity bit
                # position (dwg2.spec 3661 REPEAT_CN(6, ref): bit rcount1 ->
                # ref[rcount1]) — but out_json's REPEAT emission collapses
                # EVERY AcDbOsnapPointRef struct to a bare 0, the same
                # degenerate emission class as TABLESTYLE.borders /
                # LTYPE.dashes / MLINESTYLE.lines / MULTILEADER ctx.leaders.
                # Verified on every corpus DIMASSOC (Dynblocks and
                # example_2004 carry real slots that still serialize as
                # [0]*6, gold-normalized). Emit the degenerate form — the
                # real slot data has no gold JSON expression (silver's
                # references stay in the dump for the writer).
                fields["ref"] = [0] * 6
            # `data` + metadata were already popped by the general Associative
            # block above (assoc_data was captured before the pop).
        if silver_type == "XRecord":
            # Gold: xdata_size (BL), xdata (raw entry list), cloning (BS,
            # SINCE R_2000b). Silver: name (derived), cloning_flags (enum),
            # entries (structured list). Project to gold's shape.
            entries = payload.get("entries", [])
            fields["xdata_size"] = len(entries) if isinstance(entries, list) else 0
            # Project entries to gold's flat [code, value] pairs.
            if isinstance(entries, list):
                xdata = []
                for e in entries:
                    if isinstance(e, dict):
                        code = e.get("code", 0)
                        val = e.get("value")
                        # Unwrap the value variant ({"Int16": 1} -> 1).
                        if isinstance(val, dict) and len(val) == 1:
                            val = next(iter(val.values()))
                        xdata.append([code, normalize_value(val)])
                fields["xdata"] = xdata
            # cloning_flags enum -> cloning int (KeepExisting=0, etc.).
            cf = payload.get("cloning_flags")
            cloning_map = {"KeepExisting": 0, "Keep": 0, "ReplaceExisting": 1,
                           "Replace": 1, "XrefKeepExisting": 2, "Xref": 2}
            fields["cloning"] = cloning_map.get(cf, 0)
            # Drop silver-only fields.
            for sk in ("name", "cloning_flags", "entries", "object_references",
                       "preserve_object_reference_stream", "entries_complete",
                       "raw_data", "raw_dwg_handle_bits"):
                payload.pop(sk, None)
        if silver_type == "DictionaryVariable":
            # Gold: schema (RCd), strvalue (T). Silver: schema_number, value,
            # name. Map and drop the empty name.
            if "schema_number" in payload:
                fields["schema"] = normalize_value(payload["schema_number"])
            if "value" in payload:
                fields["strvalue"] = normalize_value(payload["value"])
            for sk in ("schema_number", "value", "name"):
                payload.pop(sk, None)
        # DICTIONARYWDFLT object (dwg.spec 2804): gold emits dxfname
        # (ACDBDICTIONARYWDFLT) + defaultid (handle 340). Silver stores
        # default_handle (raw int). The numitems/cloning/is_hardowner/items are
        # dropped by the JSON path (gold's IS_JSON uses the `items` map, which
        # the differ ignores). Project the two gold fields.
        if silver_type == "DictionaryWithDefault":
            fields["dxfname"] = "ACDBDICTIONARYWDFLT"
            dh = payload.get("default_handle")
            if dh is not None:
                fields["defaultid"] = normalize_handle_value(dh)
            for sk in ("default_handle", "entries", "duplicate_cloning",
                       "hard_owner", "name"):
                payload.pop(sk, None)
        # TABLESTYLE object (dwg2.spec 964). Two disjoint shapes:
        #  - legacy (pre-R2008, UNTIL R_2007): name/flow_direction/flags/
        #    horiz_cell_margin/vert_cell_margin/is_title_suppressed/
        #    is_header_suppressed/rowstyles. Silver reads the same data under
        #    snake_case names into flow_direction/flags/horizontal_margin/...
        #  - modern (R2010+): unknown_rc/name/unknown_bl1/unknown_bl2/cellstyle
        #    handle + the named cell-style payload CellStyle_fields under
        #    `sty.cellstyle.*` (and per-override `ovr.cellstyle.*`). Silver
        #    reads it into modern_style / modern_overrides.
        if silver_type == "TableStyle":
            def _content_format(cf, pfx):
                if not isinstance(cf, dict):
                    return
                fields[f"{pfx}.property_override_flags"] = cf.get("property_override_flags", 0)
                fields[f"{pfx}.property_flags"] = cf.get("property_flags", 0)
                fields[f"{pfx}.value_data_type"] = cf.get("value_data_type", 0)
                fields[f"{pfx}.value_unit_type"] = cf.get("value_unit_type", 0)
                fields[f"{pfx}.value_format_string"] = cf.get("value_format_string", "")
                fields[f"{pfx}.rotation"] = normalize_float(cf.get("rotation", 0.0))
                fields[f"{pfx}.block_scale"] = normalize_float(cf.get("block_scale", 0.0))
                ca = cf.get("cell_alignment", 0)
                fields[f"{pfx}.cell_alignment"] = ca if isinstance(ca, int) else 0
                fields[f"{pfx}.content_color"] = normalize_color(cf.get("content_color"))
                ts = cf.get("text_style")
                if ts is not None:
                    fields[f"{pfx}.text_style"] = normalize_handle_value(ts)
                fields[f"{pfx}.text_height"] = normalize_float(cf.get("text_height", 0.0))

            def _cell_style(cs, pfx):
                if not isinstance(cs, dict):
                    return
                fields[f"{pfx}.type"] = cs.get("style_type", 0)
                dflags = cs.get("data_flags", 0)
                fields[f"{pfx}.data_flags"] = dflags
                # The detail block is only serialized when data_flags != 0.
                if not dflags:
                    return
                fields[f"{pfx}.property_override_flags"] = cs.get("property_override_flags", 0)
                fields[f"{pfx}.merge_flags"] = cs.get("merge_flags", 0)
                fields[f"{pfx}.bg_color"] = normalize_color(cs.get("background_color"))
                fields[f"{pfx}.content_layout"] = cs.get("content_layout", 0)
                _content_format(cs.get("content_format"), f"{pfx}.content_format")
                mof = cs.get("margin_override_flags", 0)
                fields[f"{pfx}.margin_override_flags"] = mof
                if mof:
                    fields[f"{pfx}.vert_margin"] = normalize_float(cs.get("vertical_margin", 0.0))
                    fields[f"{pfx}.horiz_margin"] = normalize_float(cs.get("horizontal_margin", 0.0))
                    fields[f"{pfx}.bottom_margin"] = normalize_float(cs.get("bottom_margin", 0.0))
                    fields[f"{pfx}.right_margin"] = normalize_float(cs.get("right_margin", 0.0))
                    fields[f"{pfx}.margin_horiz_spacing"] = normalize_float(cs.get("horizontal_spacing", 0.0))
                    fields[f"{pfx}.margin_vert_spacing"] = normalize_float(cs.get("vertical_spacing", 0.0))
                borders = cs.get("borders", [])
                nb = len(borders) if isinstance(borders, list) else 0
                fields[f"{pfx}.num_borders"] = nb
                # Gold's REPEAT2 JSON for the per-border structs collapses
                # each to a bare 0 ([0]*num_borders — the same libredwg
                # emission class as LTYPE.dashes / wires / ctx.leaders;
                # verified corpus-wide). Emit the count-faithful degenerate
                # form; the array is omitted when num_borders == 0.
                if isinstance(borders, list) and nb:
                    fields[f"{pfx}.borders"] = [0] * nb

            if not r2010_plus:
                # Legacy pre-R2008 path. Map snake_case -> gold names.
                fields["flow_direction"] = {"Down": 0, "Up": 1}.get(
                    payload.get("flow_direction"), payload.get("flow_direction", 0))
                fl = payload.get("flags", 0)
                fields["flags"] = fl if isinstance(fl, int) else 0
                fields["horiz_cell_margin"] = normalize_float(payload.get("horizontal_margin", 0.0))
                fields["vert_cell_margin"] = normalize_float(payload.get("vertical_margin", 0.0))
                fields["is_title_suppressed"] = 1 if payload.get("title_suppressed") else 0
                fields["is_header_suppressed"] = 1 if payload.get("header_suppressed") else 0
                # rowstyles: libredwg's legacy decode is degenerate ([0,0,0]);
                # the real content is in unknown_bits. Emit the placeholder to
                # match gold's shape.
                fields["rowstyles"] = [0, 0, 0]
            else:
                # Modern R2010+ path.
                fields["unknown_rc"] = payload.get("modern_unknown_byte", 0)
                fields["unknown_bl1"] = payload.get("modern_unknown_long1", 0)
                fields["unknown_bl2"] = payload.get("modern_unknown_long2", 0)
                csh = payload.get("modern_cell_style_handle")
                if csh is not None:
                    fields["cellstyle"] = normalize_handle_value(csh)
                ms = payload.get("modern_style")
                if isinstance(ms, dict):
                    _cell_style(ms.get("cell_style"), "sty.cellstyle")
                    fields["sty.id"] = ms.get("id", 0)
                    fields["sty.type"] = ms.get("style_type", 0)
                    fields["sty.name"] = ms.get("name", "")
                overrides = payload.get("modern_overrides", [])
                fields["numoverrides"] = len(overrides) if isinstance(overrides, list) else 0
                if isinstance(overrides, list) and overrides:
                    # Gold emits the FIRST override as `ovr.*` (+ unknown_bl3).
                    fields["unknown_bl3"] = overrides[0][0] if isinstance(overrides[0], (list, tuple)) else 0
                    ovr = overrides[0][1] if isinstance(overrides[0], (list, tuple)) and len(overrides[0]) > 1 else None
                    if isinstance(ovr, dict):
                        _cell_style(ovr.get("cell_style"), "ovr.cellstyle")
                        fields["ovr.id"] = ovr.get("id", 0)
                        fields["ovr.type"] = ovr.get("style_type", 0)
                        fields["ovr.name"] = ovr.get("name", "")
            # Drop silver-only / differently-named fields.
            for sk in ("description", "version", "horizontal_margin",
                       "vertical_margin", "title_suppressed", "header_suppressed",
                       "flow_direction", "flags",
                       "data_row_style", "header_row_style", "title_row_style",
                       "modern_unknown_byte", "modern_unknown_long1",
                       "modern_unknown_long2", "modern_cell_style_handle",
                       "modern_style", "modern_overrides", "annotative"):
                payload.pop(sk, None)

        # MLINESTYLE object (dwg.spec 4513): gold emits name/description/flag/
        # fill_color/start_angle/end_angle + num_lines + lines[] (offset/color/
        # lt). Silver stores elements[] (offset/color/linetype-name) + a flags
        # bitflag dict + fill_color. Project to gold's shape.
        if silver_type == "MLineStyle":
            # flag: gold BS 70 bitmask. Silver stores a flags dict
            # (fill_on/display_joints/display_start_caps/... booleans). The
            # corpus's styles have all-false flags -> flag 0. Project the dict
            # to the bitmask: fill_on=1, display_miters=2 (not stored), caps.
            fl = payload.get("flags")
            flag = 0
            if isinstance(fl, dict):
                if fl.get("fill_on"):
                    flag |= 1
                if fl.get("display_miters"):
                    flag |= 2
                if fl.get("start_square"):
                    flag |= 16
                if fl.get("start_inner_arc") or fl.get("start_inner_arcs"):
                    flag |= 32
                if fl.get("start_round"):
                    flag |= 64
                if fl.get("end_square"):
                    flag |= 256
                if fl.get("end_inner_arc") or fl.get("end_inner_arcs"):
                    flag |= 512
                if fl.get("end_round"):
                    flag |= 1024
            elif isinstance(fl, int):
                flag = fl
            fields["flag"] = flag
            if "description" in payload:
                fields["description"] = payload["description"]
            if "fill_color" in payload:
                fields["fill_color"] = normalize_color(payload["fill_color"])
            if "start_angle" in payload:
                fields["start_angle"] = normalize_float(payload["start_angle"])
            if "end_angle" in payload:
                fields["end_angle"] = normalize_float(payload["end_angle"])
            # lines[]: gold's legacy REPEAT decode is degenerate ([0]*count
            # — the same libredwg emission class as MULTILEADER.ctx.leaders
            # and TABLESTYLE's structures; silver's real element offsets
            # live in unknown_bits on gold's side). Emit the count-faithful
            # degenerate form so the arrays compare equal.
            elements = payload.get("elements", [])
            fields["num_lines"] = len(elements) if isinstance(elements, list) else 0
            if isinstance(elements, list) and elements:
                fields["lines"] = [0] * len(elements)
            for sk in ("elements", "flags", "fill_color", "start_angle",
                       "end_angle", "description"):
                payload.pop(sk, None)
        # Silver-only top-level VisualStyle fields that gold stores inside the
        # property bag or under a different name; skip so they don't appear as
        # extra_in_silver. The pre-R2010 top-level face_*/edge_* fields ARE
        # gold fields (read into the struct, not the bag), so keep them.
        vs_skip = {
            "properties", "internal_use_only", "extended_lighting_model",
        } if silver_type == "VisualStyle" else set()
        # MLEADERSTYLE object (dwg2.spec 1461): silver uses different field
        # names (snake_case + enums). Rename + convert + gate.
        if silver_type == "MultiLeaderStyle":
            _ML = {
                "content_type": "content_type", "leader_draw_order": "leader_order",
                "multileader_draw_order": "mleader_order",
                "max_leader_points": "max_points",
                "first_segment_angle": "first_seg_angle",
                "second_segment_angle": "second_seg_angle",
                "path_type": "type", "line_weight": "linewt",
                "enable_landing": "has_landing", "landing_gap": "landing_gap",
                "enable_dogleg": "has_dogleg", "landing_distance": "landing_dist",
                "arrowhead_size": "arrow_head_size", "default_text": "text_default",
                "text_left_attachment": "attach_left", "text_right_attachment": "attach_right",
                "text_alignment": "text_align_type", "text_color": "text_color",
                "text_height": "text_height", "text_frame": "has_text_frame",
                "text_always_left": "text_always_left", "align_space": "align_space",
                "block_content_color": "block_color", "block_content_rotation": "block_rotation",
                "enable_block_scale": "use_block_scale", "enable_block_rotation": "use_block_rotation",
                "block_content_connection": "block_connection", "scale_factor": "scale",
                "property_changed": "is_changed", "break_gap_size": "break_size",
                "text_angle_type": "text_angle_type",
                "text_attachment_direction": "attach_dir",
                "text_top_attachment": "attach_top", "text_bottom_attachment": "attach_bottom",
                "unknown_flag_298": "text_extended",
            }
            # enum string -> int maps (gold stores the BS/RC index)
            _ML_ENUM = {
                "content_type": {"None": 0, "Block": 1, "MText": 2},
                "leader_order": {"ContentFirst": 0, "LeaderFirst": 1, "LeaderHeadFirst": 0},
                "mleader_order": {"ContentFirst": 0, "LeaderFirst": 1},
                "type": {"StraightLineSegments": 0, "Spline": 1, "None_": 2, "InVisible": 2},
                "text_angle_type": {"Insert": 0, "Horizontal": 1, "BestFit": 2},
                "text_align_type": {"Left": 0, "Center": 1, "Right": 2, "Justify": 3, "Aligned": 4},
                "attach_dir": {"Horizontal": 0, "Vertical": 1},
                "attach_top": {"TopOfTopLine": 0, "MiddleOfTopLine": 1, "MiddleOfText": 2,
                               "CenterOfText": 9, "MiddleOfBottomLine": 3,
                               "BottomOfBottomLine": 4, "BottomLine": 5,
                               "BottomOfTopLineUnderlineBottomLine": 6,
                               "BottomOfTopLineUnderlineTopLine": 7,
                               "BottomOfTopLineUnderlineAll": 8,
                               "CenterOfTextOverline": 10},
                "attach_bottom": {"TopOfTopLine": 0, "MiddleOfTopLine": 1, "MiddleOfText": 2,
                                  "CenterOfText": 9, "MiddleOfBottomLine": 3,
                                  "BottomOfBottomLine": 4, "BottomLine": 5,
                                  "BottomOfTopLineUnderlineBottomLine": 6,
                                  "BottomOfTopLineUnderlineTopLine": 7,
                                  "BottomOfTopLineUnderlineAll": 8,
                                  "CenterOfTextOverline": 10},
                "attach_left": {"TopOfTopLine": 0, "MiddleOfTopLine": 1, "MiddleOfText": 2,
                                "MiddleOfBottomLine": 3, "BottomOfBottomLine": 4, "BottomLine": 5,
                                "BottomOfTopLineUnderlineBottomLine": 6,
                                "BottomOfTopLineUnderlineTopLine": 7,
                                "BottomOfTopLineUnderlineAll": 8,
                                "CenterOfTextOverline": 10},
                "attach_right": {"TopOfTopLine": 0, "MiddleOfTopLine": 1, "MiddleOfText": 2,
                                 "MiddleOfBottomLine": 3, "BottomOfBottomLine": 4, "BottomLine": 5,
                                 "BottomOfTopLineUnderlineBottomLine": 6,
                                 "BottomOfTopLineUnderlineTopLine": 7,
                                 "BottomOfTopLineUnderlineAll": 8,
                                 "CenterOfTextOverline": 10},
                "block_connection": {"BlockExtents": 0, "InsertionPoint": 1},
            }
            _ML_BOOL = {"has_landing", "has_dogleg", "use_block_scale", "use_block_rotation",
                        "has_text_frame", "text_always_left", "is_changed", "text_extended"}
            _ML_R2010 = {"class_version", "attach_dir", "attach_top", "attach_bottom"}
            _ML_R2013 = {"text_extended"}
            ml_consumed = set()
            for k, v in payload.items():
                if k in ("handle", "owner", "owner_handle", "reactors", "xdictionary_handle"):
                    continue
                gk = _ML.get(k)
                if gk is None:
                    if k in ("line_type_handle", "text_style_handle", "arrowhead_handle",
                             "block_content_handle", "block_content_scale_x",
                             "block_content_scale_y", "block_content_scale_z",
                             "line_color", "text_color", "is_annotative", "description",
                             "name", "class_version"):
                        ml_consumed.add(k)
                    continue
                ml_consumed.add(k)
                if gk in _ML_R2010 and not r2010_plus:
                    continue
                if gk in _ML_R2013 and not r2013_plus:
                    continue
                if gk in _ML_ENUM:
                    m = _ML_ENUM[gk]
                    fields[gk] = m.get(str(v), m.get(v.replace(" ", ""), v)) if isinstance(v, str) else v
                elif k in _ML_BOOL:
                    fields[gk] = 1 if v else 0
                else:
                    fields[gk] = normalize_value(v)
            # class_version (dwg2.spec 1461): the spec gates it SINCE R_2010b,
            # but MLEADERSTYLE objects are always read from EED/upconverted, so
            # gold emits class_version (default 2) on EVERY corpus version
            # (R2000 example_2000 included). Emit unconditionally.
            fields["class_version"] = payload.get("class_version", 2)
            ml_consumed.add("class_version")
            # linewt: gold stores the raw BLd lineweight value (negative =
            # ByLayer=-1/ByBlock=-2/Default=-3, else mm*100). Silver stores the
            # enum string. Map to gold's raw value.
            lw = payload.get("line_weight")
            if isinstance(lw, str):
                _LW = {"ByLayer": -1, "ByBlock": -2, "Default": -3, "ByLwDefault": -3}
                fields["linewt"] = _LW.get(lw, -3)
                ml_consumed.add("line_weight")
            # handles: gold emits these as handle dicts even when null (code 5,
            # absref 0). Silver stores Option<Handle> (None = null). Emit the
            # null-handle shape so the differ resolves both to target 0.
            _NULL_HANDLE = {"code": 5, "size": 0, "value": 0, "absref": 0}
            def _h(name):
                v = payload.get(name)
                return normalize_handle_value(v) if v is not None else _NULL_HANDLE
            fields["line_type"] = _h("line_type_handle")
            fields["text_style"] = _h("text_style_handle")
            fields["arrow_head"] = _h("arrowhead_handle")
            fields["block"] = _h("block_content_handle")
            for kk in ("line_type_handle", "text_style_handle", "arrowhead_handle",
                       "block_content_handle"):
                ml_consumed.add(kk)
            # colors: silver stores a Color enum; gold emits the ACI index
            # (ByBlock=0, ByLayer=256). normalize_color handles the string.
            for src, dst in (("line_color", "line_color"), ("text_color", "text_color"),
                             ("block_content_color", "block_color")):
                v = payload.get(src)
                if v is not None:
                    fields[dst] = normalize_color(v)
                ml_consumed.add(src)
            # block_scale: silver stores x/y/z scalars; gold uses a 3BD
            if r2013_plus or True:  # block_scale is JSON 3BD
                bsc = [payload.get("block_content_scale_x", 1.0),
                       payload.get("block_content_scale_y", 1.0),
                       payload.get("block_content_scale_z", 1.0)]
                fields["block_scale"] = bsc
                for kk in ("block_content_scale_x", "block_content_scale_y", "block_content_scale_z"):
                    ml_consumed.add(kk)
            # description/name: silver stores both; gold has description
            if payload.get("description"):
                fields["description"] = payload["description"]
            ml_consumed.update(("description", "name", "is_annotative"))
            # is_annotative (dwg2.spec 1461 FIELD_B 296): gold emits it on every
            # version (EED/upconvert); emit unconditionally.
            fields["is_annotative"] = 1 if payload.get("is_annotative") else 0
            for kk in ml_consumed:
                payload.pop(kk, None)

        mat_consumed = _map_material(payload, fields, r2007_plus, r2010_plus) if silver_type == "Material" else set()
        # LAYOUT object (dwg.spec 5316): silver stores plot config FLAT with
        # different names; gold nests them under `plotsettings.*` (the
        # AcDbPlotSettings subclass). Project the whole set.
        if silver_type == "Layout":
            # --- plotsettings.* (the AcDbPlotSettings subclass) ---
            _PS = {
                # gold printer_cfg_file = silver plot_page_name (the page setup
                # name); gold paper_size = silver plot_printer_name (the device);
                # gold canonical_media_name = silver paper_size (the media).
                "plot_page_name": "plotsettings.printer_cfg_file",
                "plot_printer_name": "plotsettings.paper_size",
                "plot_flags_dict": "plotsettings.plot_flags",  # placeholder; dict handled below
                "plot_margin_left": "plotsettings.left_margin",
                "plot_margin_bottom": "plotsettings.bottom_margin",
                "plot_margin_right": "plotsettings.right_margin",
                "plot_margin_top": "plotsettings.top_margin",
                "paper_width": "plotsettings.paper_width",
                "paper_height": "plotsettings.paper_height",
                "plot_printer_name": "plotsettings.paper_size",  # gold paper_size = silver device name
                "paper_size": "plotsettings.canonical_media_name",  # gold canonical = silver media
                "plot_paper_units": "plotsettings.plot_paper_unit",
                "plot_rotation": "plotsettings.plot_rotation_mode",
                "plot_type": "plotsettings.plot_type",
                "plot_scale_factor": "plotsettings.std_scale_factor",
                "plot_scale_type": "plotsettings.std_scale_type",
                "plot_style_sheet": "plotsettings.stylesheet",
            }
            # version gates (shadeplot R2004a+, livesection/plotview handles)
            _PS_R2004 = {"plotsettings.shadeplot_type", "plotsettings.shadeplot_reslevel",
                         "plotsettings.shadeplot_customdpi"}
            for sk, gk in _PS.items():
                if sk in ("plot_flags_dict",):
                    continue
                v = payload.get(sk)
                if v is None:
                    continue
                if gk in _PS_R2004 and not r2004_plus:
                    continue
                fields[gk] = normalize_value(v)
            # plot_flags dict -> int bits, using silver's PlotFlags::to_bits
            # layout (plot_settings.rs): bit0 plot_viewport_borders, 1
            # show_plot_styles, 2 plot_centered, 3 plot_hidden, 4
            # use_standard_scale, 5 plot_plot_styles, 6 scale_lineweights, 7
            # print_lineweights, 9 draw_viewports_first, 10 model_type, 11
            # update_paper, 12 zoom_to_paper_on_update, 13 initializing, 14
            # prev_plot_init; unknown_bits carried through (mask !0x7EFF).
            pfl = payload.get("plot_flags")
            if isinstance(pfl, dict):
                bits = pfl.get("unknown_bits", 0) & ~0x7EFF
                for name, bit in (("plot_viewport_borders", 0), ("show_plot_styles", 1),
                        ("plot_centered", 2), ("plot_hidden", 3), ("use_standard_scale", 4),
                        ("plot_plot_styles", 5), ("scale_lineweights", 6),
                        ("print_lineweights", 7), ("draw_viewports_first", 9),
                        ("model_type", 10), ("update_paper", 11),
                        ("zoom_to_paper_on_update", 12), ("initializing", 13),
                        ("prev_plot_init", 14)):
                    if pfl.get(name):
                        bits |= (1 << bit)
                fields["plotsettings.plot_flags"] = bits
            elif isinstance(pfl, int):
                fields["plotsettings.plot_flags"] = pfl
            # 2-point fields: plot_origin, plot_window_ll/ur, paper_image_origin
            if payload.get("plot_origin_x") is not None or payload.get("plot_origin_y") is not None:
                fields["plotsettings.plot_origin"] = [payload.get("plot_origin_x", 0.0), payload.get("plot_origin_y", 0.0)]
            if payload.get("plot_window_min_x") is not None or payload.get("plot_window_min_y") is not None:
                fields["plotsettings.plot_window_ll"] = [payload.get("plot_window_min_x", 0.0), payload.get("plot_window_min_y", 0.0)]
            if payload.get("plot_window_max_x") is not None or payload.get("plot_window_max_y") is not None:
                fields["plotsettings.plot_window_ur"] = [payload.get("plot_window_max_x", 0.0), payload.get("plot_window_max_y", 0.0)]
            if payload.get("paper_image_origin_x") is not None or payload.get("paper_image_origin_y") is not None:
                fields["plotsettings.paper_image_origin"] = [payload.get("paper_image_origin_x", 0.0), payload.get("paper_image_origin_y", 0.0)]
            # canonical_media_name is mapped from silver's `paper_size` in _PS
            # above; do NOT double-emit it here.
            # shadeplot (R2004a+)
            if r2004_plus:
                if payload.get("shade_plot_mode") is not None:
                    fields["plotsettings.shadeplot_type"] = payload["shade_plot_mode"]
                if payload.get("shade_plot_resolution") is not None:
                    fields["plotsettings.shadeplot_reslevel"] = payload["shade_plot_resolution"]
                if payload.get("shade_plot_dpi") is not None:
                    fields["plotsettings.shadeplot_customdpi"] = payload["shade_plot_dpi"]
            # paper_units/drawing_units: gold computes from std_scale_factor;
            # silver stores plot_scale_numerator/denominator.
            if payload.get("plot_scale_numerator") is not None:
                fields["plotsettings.paper_units"] = payload["plot_scale_numerator"]
            if payload.get("plot_scale_denominator") is not None:
                fields["plotsettings.drawing_units"] = payload["plot_scale_denominator"]
            # plotview handle (R2004a+; gold omits it on R2000/AC1015).
            if r2004_plus and payload.get("plot_view_handle") is not None:
                fields["plotsettings.plotview"] = normalize_handle_value(payload["plot_view_handle"])
            # plotview_name (dwg.spec 5254 FIELD_T, UNTIL R_2002 i.e. R2000):
            # gold always emits the (possibly empty) name; silver omits it when
            # empty. Emit "" on pre-R2004.
            if not r2004_plus:
                fields["plotsettings.plotview_name"] = payload.get("plot_view_name") or ""
            # shadeplot handle (R2007a+, code 4). Gold emits the null handle
            # even when there is no shade-plot viewport; silver stores None.
            # Emit the null-handle shape when absent.
            if r2007_plus:
                sp = payload.get("shade_plot_handle")
                fields["plotsettings.shadeplot"] = (
                    normalize_handle_value(sp) if sp is not None
                    else {"code": 4, "size": 0, "value": 0, "absref": 0})
            # --- non-plotsettings LAYOUT fields ---
            _LAY = {
                "min_extents": "EXTMIN", "max_extents": "EXTMAX",
                "insertion_base": "INSBASE", "min_limits": "LIMMIN",
                "max_limits": "LIMMAX", "ucs_origin": "UCSORG",
                "ucs_x_axis": "UCSXDIR", "ucs_y_axis": "UCSYDIR",
                "ucs_ortho_type": "UCSORTHOVIEW", "elevation": "ucs_elevation",
                "flags": "layout_flags", "name": "layout_name",
                "tab_order": "tab_order",
            }
            for sk, gk in _LAY.items():
                v = payload.get(sk)
                if v is None:
                    continue
                fields[gk] = normalize_value(v)
            # handles
            if payload.get("block_record") is not None:
                fields["block_header"] = normalize_handle_value(payload["block_record"])
            if payload.get("viewport") is not None:
                fields["active_viewport"] = normalize_handle_value(payload["viewport"])
            # viewports (gold HANDLE_VECTOR, code 4): silver's Layout keeps
            # the actual per-layout viewport handle list (verified ex2018:
            # layout 86 -> [136]).
            _vps = payload.pop("viewports", None)
            if isinstance(_vps, list) and _vps:
                fields["viewports"] = [normalize_handle_value(h)
                                       for h in _vps if isinstance(h, int)]
            # drop all consumed/silver-only keys so the generic loop skips them
            for kk in (list(_PS) + list(_LAY) + [
                "plot_flags", "plot_origin_x", "plot_origin_y",
                "plot_window_min_x", "plot_window_min_y",
                "plot_window_max_x", "plot_window_max_y",
                "paper_image_origin_x", "paper_image_origin_y",
                "shade_plot_mode", "shade_plot_resolution", "shade_plot_dpi",
                "plot_scale_numerator", "plot_scale_denominator",
                "plot_view_handle", "plot_view_name", "plot_page_name",
                "block_record", "viewport", "viewports",
                # silver-only LAYOUT fields gold omits on the binary path:
                "visual_style_handle",
            ]):
                payload.pop(kk, None)

        for k, v in payload.items():
            if k in ("handle", "owner", "owner_handle", "reactors", "xdictionary_handle"):
                continue
            if k in vs_skip or k in mat_consumed:
                continue
            # LAYOUT base_ucs/named_ucs are handle-stream fields in gold
            # (code 5, dwg.spec); silver stores the raw Handle int. Wrap so
            # the differ resolves them (a bare int never matches a token).
            if silver_type == "Layout" and k in ("base_ucs", "named_ucs"):
                fields[k] = normalize_handle_value(v)
                continue
            # GEODATA host_block: gold handle-stream field (code 4); silver
            # stores the raw Handle int.
            if gold_type == "GEODATA" and k == "host_block":
                fields[k] = normalize_handle_value(v)
                continue
            if is_ignored(k, ignore_set, ignore_patterns):
                continue
            fields[k] = normalize_value(v)
        _emit_unknown_bits(gold_type, payload.get("handle"), fields)
        out.append({"type": gold_type, "fields": fields})

    # Table records.
    TABLE_SPECS: List[Tuple[str, str, str]] = [
        ("layers", "LAYER", "LAYER"),
        ("line_types", "LTYPE", "LTYPE"),
        ("text_styles", "STYLE", "STYLE"),
        ("block_records", "BLOCK_RECORD", "BLOCK_HEADER"),
        ("dim_styles", "DIMSTYLE", "DIMSTYLE"),
        ("app_ids", "APPID", "APPID"),
        ("views", "VIEW", "VIEW"),
        ("vports", "VPORT", "VPORT"),
        ("ucss", "UCS", "UCS"),
    ]
    # Gold emits these storage fields for every table record
    # (COMMON_TABLE_FLAGS + common_object_handle_data.spec). Silver parses the
    # xref bits and unknown byte but discards them; for ordinary drawings they
    # are always this uniform set, so the normalizer derives them:
    #   ownerhandle       = the owning table's control-object handle
    #   is_xref_ref       = 1   (ordinary records always reference-free)
    #   is_xref_resolved  = 0
    #   is_xref_dep       = 0
    #   xref              = null handle
    #   unknown           = 0   (APPID only, SINCE R_13b1)
    # Gold emits a CONTROL object per table (e.g. APPID_CONTROL) that silver
    # does not surface as a standalone object (the table's handle IS the
    # control). Emit a minimal control record so the differ's handle->type map
    # can resolve table-record ownerhandle to <TYPE>_CONTROL instead of a raw
    # int. The control's own fields are gold-version-gated and not compared
    # beyond handle/type, so keep it minimal.
    CONTROL_GOLD_TYPE = {
        "layers": "LAYER_CONTROL", "line_types": "LTYPE_CONTROL",
        "text_styles": "STYLE_CONTROL", "block_records": "BLOCK_CONTROL",
        "dim_styles": "DIMSTYLE_CONTROL", "app_ids": "APPID_CONTROL",
        "views": "VIEW_CONTROL", "vports": "VPORT_CONTROL",
        "ucss": "UCS_CONTROL",
    }
    for table_key, _entry_gold_type, record_gold_type in TABLE_SPECS:
        table = data.get(table_key, {})
        if not isinstance(table, dict):
            continue
        raw_ctrl_handle = table.get("handle")
        table_handle = normalize_handle_value(raw_ctrl_handle)
        control_type = CONTROL_GOLD_TYPE.get(table_key)
        entries = table.get("entries", {})
        if control_type and table_handle is not None:
            _ctrl = {"handle": table_handle,
                     "ownerhandle": normalize_handle_value(0)}
            # Gold DIMSTYLE_CONTROL also emits a `morehandles` vector
            # (dwg.spec 4177: RCu num_morehandles SINCE R_2000b +
            # HANDLE_VECTOR code 5, "additional hard handles,
            # undocumented"). Silver's pass-1 reader captures it as the
            # document-level `dimstyle_morehandles` handle list; project
            # it here. NEVER emit the dim-style table entries as
            # morehandles — that regressed 19 → 109 rows (2026-09-20).
            if table_key == "dim_styles":
                _mh = data.get("dimstyle_morehandles")
                if isinstance(_mh, list) and _mh:
                    _ctrl["morehandles"] = [normalize_handle_value(h)
                                            for h in _mh]
            # Common object handle-stream bits gold emits on every control
            # object (common_object_handle_data.spec / CONTROL_HANDLE_STREAM,
            # spec.h): controls read ownerhandle/reactors/xdicobjhandle after
            # their num_entries data. xdicobjhandle serializes whenever the
            # control owns an extension dictionary (all versions); the
            # is_xdic_missing bit only exists on R2004+.
            _ctrl_xdic = _lookup_xdic(raw_ctrl_handle)
            if _ctrl_xdic:
                _ctrl["xdicobjhandle"] = normalize_handle_value(_ctrl_xdic)
                if r2004_plus:
                    _ctrl["is_xdic_missing"] = 0
            elif r2004_plus:
                _ctrl["is_xdic_missing"] = 1
            if r2013_plus:
                _ctrl["has_ds_data"] = 0
            if control_type == "BLOCK_CONTROL":
                # dwg.spec (BLOCK_CONTROL): model_space/paper_space point at
                # the *Model_Space / *Paper_Space block headers. Case-fold:
                # BricsCAD-downsaved R13 fixtures name them
                # *MODEL_SPACE/*PAPER_SPACE (the golden_entities AC1012 set)
                # — a case-sensitive compare missed both fields.
                for _e in entries.values():
                    if isinstance(_e, dict):
                        _n = str(_e.get("name") or "").upper()
                        if _n == "*MODEL_SPACE":
                            _ctrl["model_space"] = normalize_handle_value(_e.get("handle"))
                        elif _n == "*PAPER_SPACE":
                            _ctrl["paper_space"] = normalize_handle_value(_e.get("handle"))
            if control_type == "LTYPE_CONTROL":
                # dwg.spec (LTYPE_CONTROL): byblock/bylayer handle the
                # special ByBlock/ByLayer ltype entries.
                for _e in entries.values():
                    if isinstance(_e, dict):
                        _n = str(_e.get("name") or "").upper()
                        if _n == "BYBLOCK":
                            _ctrl["byblock"] = normalize_handle_value(_e.get("handle"))
                        elif _n == "BYLAYER":
                            _ctrl["bylayer"] = normalize_handle_value(_e.get("handle"))
            out.append({"type": control_type, "fields": _ctrl})
        sibling_names = set()
        if table_key == "block_records":
            # Silver's name-keyed table resolves gold's duplicate block
            # names (all dimension-geometry blocks are literally '*D',
            # every paper-space block '*Paper_Space') by appending digits:
            # entry '*D', '*D0'..'*D10' / '*Paper_Space', '*Paper_Space0'.
            # The strip rule below needs the name multiset.
            for _e in entries.values():
                if isinstance(_e, dict) and isinstance(_e.get("name"), str):
                    sibling_names.add(_e["name"])
        for _key, rec in entries.items():
            if not isinstance(rec, dict):
                continue
            _inject_reactors(rec)
            fields = _object_common_fields(rec)
            if table_handle is not None:
                fields["ownerhandle"] = table_handle
            # is_xref_* bits: gold reads them from the stream only up to
            # R2004; on R2007+ they are derived (is_xref_ref=1, is_xref_dep
            # from is_xref_resolved) but the JSON omits the always-constant
            # ones. Emit only where gold serializes them.
            if not r2007_plus:
                fields["is_xref_ref"] = 1
                fields["is_xref_dep"] = 0
            fields["is_xref_resolved"] = 0
            fields["xref"] = normalize_handle_value(0)
            if record_gold_type == "APPID":
                fields["unknown"] = 0
            if record_gold_type == "BLOCK_HEADER":
                # BLOCK_RECORD flag bits (dwg.spec 3180-3186 + 3204): split
                # from silver's parsed `flags` dict. The earlier hard-coded
                # zeros made every anonymous star block (*D dimension
                # geometry, *U, *T) wrong_value — silver's table has the
                # correct bit all along.
                _fl = rec.get("flags") if isinstance(rec.get("flags"), dict) else {}
                fields["anonymous"] = 1 if _fl.get("anonymous") else 0
                fields["hasattrs"] = 1 if _fl.get("has_attributes") else 0
                fields["blkisxref"] = 1 if _fl.get("is_xref") else 0
                fields["xrefoverlaid"] = 1 if _fl.get("is_xref_overlay") else 0
                if not r13_14:
                    # R13/R14 (gold BLOCK_HEADER): no xref_loaded bit on the
                    # wire — the field is R2000b+.
                    fields["xref_loaded"] = 1 if _fl.get("is_external") else 0
                else:
                    # R13/R14: layout (SINCE R_2000b) and description are not
                    # on the wire — silver's model defaults never compare.
                    rec.pop("layout", None)
                    rec.pop("description", None)
                # TODO B1 (2026-10-01): the table-entry xref resolved
                # value, RAW (gold prints the bitshort verbatim — the
                # authored xref blocks carry 1); the generic hard-coded 0
                # above stays for the other tables.
                fields["is_xref_resolved"] = rec.get("xref_resolved", 0)
                # Gold emits the block's real name; silver's name-keyed
                # table uniquified duplicates by appending digits — strip
                # the uniquifier only when the digitless base exists as
                # another entry (verified pairs: '*D3' -> '*D' 81 rows,
                # '*Paper_Space0' -> '*Paper_Space' 38 rows). The lazy
                # match strips ALL trailing digits: '*D10' must reduce to
                # '*D' (the greedy form kept '*D1').
                _nm = rec.get("name")
                if isinstance(_nm, str):
                    _m = re.match(r"^(.*?)\d+$", _nm)
                    if _m and _m.group(1) in sibling_names:
                        _nm = _m.group(1)
                    fields["name"] = _nm
                # xref_pname (FIELD_T, R13b1+): silver keeps it as the
                # table record's xref_path (the xref's filename slug,
                # e.g. BED_QUEENSIZE); silver's nested Block entity twin
                # stores "".
                _xp = rec.get("xref_path")
                fields["xref_pname"] = _xp if isinstance(_xp, str) else ""
            if record_gold_type == "VPORT":
                # VPORT view params (dwg.spec 4007-4160 DWG path): rename silver
                # snake_case to gold names, convert types, version-gate. The
                # consumed keys are skipped by the generic loop below.
                _VPORT_RENAME = {
                    "view_height": "VIEWSIZE", "view_width": "view_width",
                    "view_center": "VIEWCTR", "view_direction": "VIEWDIR",
                    "view_target": "view_target", "view_twist": "VIEWTWIST",
                    "lens_length": "LENSLENGTH", "front_clip": "FRONTZ",
                    "back_clip": "BACKZ", "snap_rotation": "SNAPANG",
                    "snap_base": "SNAPBASE", "snap_spacing": "SNAPUNIT",
                    "grid_spacing": "GRIDUNIT", "ucs_origin": "ucsorg",
                    "ucs_x_axis": "ucsxdir", "ucs_y_axis": "ucsydir",
                    "ucs_elevation": "ucs_elevation", "ucs_ortho_type": "UCSORTHOVIEW",
                    "ucs_at_origin": "ucs_at_origin", "grid_major": "grid_major",
                    "brightness": "brightness", "contrast": "contrast",
                    "default_lighting_type": "default_lightning_type",  # gold typo
                    "snap_isopair": "SNAPISOPAIR",
                }
                _VPORT_BOOL = {
                    "grid_on": "GRIDMODE", "snap_on": "SNAPMODE",
                    "snap_style": "SNAPSTYLE", "fast_zoom": "FASTZOOM",
                    "ucsfollow": "UCSFOLLOW", "use_default_lights": "use_default_lights",
                    "ucs_per_viewport": "UCSVP",
                }
                _VPORT_R2000 = {"ucsorg", "ucsxdir", "ucsydir", "ucs_elevation",
                                "UCSORTHOVIEW", "ucs_at_origin", "UCSVP",
                                "render_mode", "named_ucs", "base_ucs"}
                _VPORT_R2007 = {"grid_flags", "grid_major", "use_default_lights",
                                "default_lightning_type", "brightness", "contrast",
                                "ambient_color", "sun", "background", "visualstyle"}
                # all silver keys this block consumes (generic loop must skip)
                vport_consumed = (set(_VPORT_RENAME) | set(_VPORT_BOOL)
                                  | {"render_mode", "grid_flags", "ambient_color",
                                     "named_ucs_handle", "base_ucs_handle", "sun_handle",
                                     "background_handle", "visual_style_handle",
                                     "ucsicon_lower", "ucsicon_origin"})
                for k, v in rec.items():
                    if k not in vport_consumed:
                        continue
                    gk = _VPORT_RENAME.get(k) or _VPORT_BOOL.get(k)
                    if gk is None:
                        continue  # special-conversion keys handled below
                    if gk in _VPORT_R2000 and not r2000_plus:
                        continue
                    if gk in _VPORT_R2007 and not r2007_plus:
                        continue
                    if k in _VPORT_BOOL:
                        fields[gk] = 1 if v else 0
                    else:
                        fields[gk] = normalize_value(v)
                # special conversions
                if r2000_plus:
                    rm = rec.get("render_mode")
                    if isinstance(rm, str):
                        _RM = {"Wireframe2D": 0, "Wireframe3D": 1, "HiddenLine": 2,
                               "FlatShaded": 3, "GouraudShaded": 4,
                               "FlatShadedWithEdges": 5, "GouraudShadedWithEdges": 6}
                        fields["render_mode"] = _RM.get(rm, 0)
                    elif rm is not None:
                        fields["render_mode"] = rm
                    fields["named_ucs"] = normalize_handle_value(rec.get("named_ucs_handle"))
                    fields["base_ucs"] = normalize_handle_value(rec.get("base_ucs_handle"))
                if r2007_plus:
                    gfl = rec.get("grid_flags")
                    if isinstance(gfl, dict):
                        fields["grid_flags"] = ((1 if gfl.get("beyond_limits") else 0)
                                                | (2 if gfl.get("adaptive") else 0)
                                                | (4 if gfl.get("subdivision") else 0)
                                                | (8 if gfl.get("follow_dynamic") else 0))
                    elif gfl is not None:
                        fields["grid_flags"] = gfl
                    fields["sun"] = normalize_handle_value(rec.get("sun_handle"))
                    fields["background"] = normalize_handle_value(rec.get("background_handle"))
                    fields["visualstyle"] = normalize_handle_value(rec.get("visual_style_handle"))
                    ac = rec.get("ambient_color")
                    if isinstance(ac, dict) and "Rgb" in ac:
                        rgb = ac["Rgb"]
                        fields["ambient_color"] = {"index": 250,
                            "rgb": "c2%02x%02x%02x%02x" % (0, rgb.get("r", 0), rgb.get("g", 0), rgb.get("b", 0))}
                # composite bits
                ucsicon = (1 if rec.get("ucsicon_lower") else 0) | (2 if rec.get("ucsicon_origin") else 0)
                fields["UCSICON"] = ucsicon
                # dwg.spec VPORT DWG path: gold reads VIEWMODE as one
                # FIELD_4BITS nibble (bits.c bit_read_4BITS composes the
                # bits MSB-first: first-read bit becomes value 8). The wire
                # order is perspective, front_clipping, back_clipping,
                # front_clip_at_eye — NOT the ucs bools.
                viewmode = ((8 if rec.get("perspective") else 0)
                            | (4 if rec.get("front_clipping") else 0)
                            | (2 if rec.get("back_clipping") else 0)
                            | (1 if rec.get("front_clip_at_eye") else 0))
                fields["VIEWMODE"] = viewmode
                # view_width: gold computes it (aspect_ratio * VIEWSIZE); silver
                # stores neither. Derive from the two silver values so the
                # R2000+ gold field (a plain BD in the DWG path) matches.
                vh = rec.get("view_height")
                ar = rec.get("aspect_ratio")
                if isinstance(vh, (int, float)) and isinstance(ar, (int, float)):
                    fields.setdefault("view_width", ar * vh)
            if record_gold_type == "VIEW":
                # VIEW table record (dwg.spec 3738 DWG path): rename silver
                # snake_case view params to gold names, convert, version-gate.
                _VIEW_RENAME = {
                    "height": "VIEWSIZE", "width": "view_width",
                    "direction": "VIEWDIR", "target": "view_target",
                    "twist_angle": "VIEWTWIST", "lens_length": "LENSLENGTH",
                    "front_clip": "FRONTZ", "back_clip": "BACKZ",
                    "ucs_origin": "ucsorg", "ucs_x_axis": "ucsxdir",
                    "ucs_y_axis": "ucsydir", "ucs_ortho_type": "UCSORTHOVIEW",
                    "ucs_elevation": "ucs_elevation", "ucs_associated": "associated_ucs",
                    "default_lighting_type": "default_lightning_type",
                    "brightness": "brightness", "contrast": "contrast",
                }
                _VIEW_BOOL = {"paper_space": "is_pspace", "ucs_associated_bool": "associated_ucs",
                              "use_default_lights": "use_default_lights"}
                _VIEW_2D = {"VIEWCTR"}
                _VIEW_R2000 = {"render_mode", "associated_ucs", "ucsorg", "ucsxdir",
                               "ucsydir", "UCSORTHOVIEW", "ucs_elevation",
                               "named_ucs", "base_ucs"}
                # The UCS block is CONDITIONAL on the wire (dwg.spec 3823:
                # gold reads ucsorg/…/named_ucs only when the
                # associated_ucs bit is set; silver's model carries the
                # defaults regardless). Drop the block when not
                # associated — the camera-created views have bit 0.
                _VIEW_UCS_GATED = {"ucsorg", "ucsxdir", "ucsydir", "UCSORTHOVIEW",
                                   "ucs_elevation", "named_ucs", "base_ucs"}
                _view_ucs_on = bool(rec.get("ucs_associated"))
                _VIEW_R2007 = {"use_default_lights", "default_lightning_type",
                               "brightness", "contrast", "ambient_color",
                               "background", "visualstyle", "sun", "livesection",
                               "is_camera_plottable"}
                view_consumed = set()
                for k, v in rec.items():
                    if k in ("handle", "owner", "owner_handle", "reactors", "xdictionary_handle"):
                        continue
                    gk = _VIEW_RENAME.get(k) or _VIEW_BOOL.get(k)
                    if gk is None:
                        if k in ("render_mode", "ambient_color", "named_ucs_handle",
                                 "base_ucs_handle", "sun_handle", "background_handle",
                                 "visual_style_handle", "live_section_handle",
                                 "perspective", "front_clipping", "back_clipping",
                                 "front_clip_at_eye", "center"):
                            view_consumed.add(k)
                        continue
                    view_consumed.add(k)
                    if gk in _VIEW_R2000 and not r2000_plus:
                        continue
                    if gk in _VIEW_UCS_GATED and not _view_ucs_on:
                        continue
                    if gk in _VIEW_R2007 and not r2007_plus:
                        continue
                    if k in _VIEW_BOOL:
                        fields[gk] = 1 if v else 0
                    else:
                        fields[gk] = normalize_value(v)
                # center -> VIEWCTR (2RD)
                c = rec.get("center")
                if c is not None:
                    nv = normalize_value(c)
                    if isinstance(nv, list):
                        nv = nv[:2]
                    fields["VIEWCTR"] = nv
                    view_consumed.add("center")
                # render_mode string -> int (R2000+)
                if r2000_plus:
                    rm = rec.get("render_mode")
                    if isinstance(rm, str):
                        _RM = {"Wireframe2D": 0, "Wireframe3D": 1, "HiddenLine": 2,
                               "FlatShaded": 3, "GouraudShaded": 4,
                               "FlatShadedWithEdges": 5, "GouraudShadedWithEdges": 6}
                        fields["render_mode"] = _RM.get(rm, 0)
                    elif rm is not None:
                        fields["render_mode"] = rm
                    if _view_ucs_on:
                        fields["named_ucs"] = normalize_handle_value(rec.get("named_ucs_handle"))
                        fields["base_ucs"] = normalize_handle_value(rec.get("base_ucs_handle"))
                    for kk in ("render_mode", "named_ucs_handle", "base_ucs_handle"):
                        view_consumed.add(kk)
                if r2007_plus:
                    fields["use_default_lights"] = 1 if rec.get("use_default_lights") else 0
                    fields["is_camera_plottable"] = 1 if rec.get("camera_plottable") else 0
                    # dwg.spec VIEW: is_camera_plottable is an R2007+
                    # field — gold omits it entirely on earlier files;
                    # consume the silver key unconditionally below so it
                    # never leaks through the generic loop on R2000.
                    fields["background"] = normalize_handle_value(rec.get("background_handle"))
                    fields["visualstyle"] = normalize_handle_value(rec.get("visual_style_handle"))
                    fields["sun"] = normalize_handle_value(rec.get("sun_handle"))
                    ac = rec.get("ambient_color")
                    if isinstance(ac, dict) and "Rgb" in ac:
                        rgb = ac["Rgb"]
                        fields["ambient_color"] = {"index": 250,
                            "rgb": "c2%02x%02x%02x%02x" % (0, rgb.get("r", 0), rgb.get("g", 0), rgb.get("b", 0))}
                    for kk in ("use_default_lights", "background_handle", "visual_style_handle",
                               "sun_handle", "ambient_color"):
                        view_consumed.add(kk)
                # composite VIEWMODE bits: gold reads one FIELD_4BITS
                # nibble (bits.c composes MSB-first: first-read bit is
                # value 8); wire order = perspective, front_clipping,
                # back_clipping, front_clip_at_eye (dwg.spec VIEW
                # ViInfo block).
                viewmode = ((8 if rec.get("perspective") else 0)
                            | (4 if rec.get("front_clipping") else 0)
                            | (2 if rec.get("back_clipping") else 0)
                            | (1 if rec.get("front_clip_at_eye") else 0))
                fields["VIEWMODE"] = viewmode
                view_consumed.update(("perspective", "front_clipping",
                                      "back_clipping", "front_clip_at_eye",
                                      "ucs_per_viewport", "ucsfollow",
                                      "live_section_handle"))
                # aspect_ratio: gold computes view_width / VIEWSIZE (both stored
                # in silver as width/height). Derive it.
                vw = rec.get("width"); vh2 = rec.get("height")
                if isinstance(vw, (int, float)) and isinstance(vh2, (int, float)) and vh2:
                    fields.setdefault("aspect_ratio", vw / vh2)
                # livesection handle (R2007+)
                if r2007_plus:
                    fields["livesection"] = normalize_handle_value(rec.get("live_section_handle"))
                # dwg.spec VIEW: is_camera_plottable is an R2007+ field —
                # gold omits it entirely on earlier files; consume the
                # silver key unconditionally so it never leaks through the
                # generic loop on R2000.
                view_consumed.add("camera_plottable")
                view_consumed.add("live_section_handle")
                # drop silver xref bookkeeping (gold table-record block emits the
                # is_xref_* bits; these silver-internal dupes never appear in gold)
                view_consumed.update(("xref_reference", "xref_resolved",
                                      "xref_dependent", "xref_handle"))
                # consumed-key guard for the generic loop below
                for kk in view_consumed:
                    rec.pop(kk, None)
            if record_gold_type == "LTYPE":
                # LTYPE dash patterns (dwg.spec 3580 DWG path). Silver stores
                # elements[] (length+complex), pattern_length, alignment as a
                # char; gold stores dashes[] (8 fields each), pattern_len,
                # numdashes, alignment as a byte (ord). Project.
                al = rec.get("alignment")
                if isinstance(al, str) and len(al) == 1:
                    fields["alignment"] = ord(al)
                elif isinstance(al, int):
                    fields["alignment"] = al
                if r2000_plus or True:  # pattern_len is LATER_VERSIONS (R13+)
                    fields["pattern_len"] = rec.get("pattern_length", 0.0)
                elems = rec.get("elements") or []
                fields["numdashes"] = len(elems)
                if elems:
                    # Gold's REPEAT JSON for dashes collapses each dash
                    # struct to a bare 0 ([0]*count — verified corpus-wide;
                    # the same libredwg emission class as wires/silhouettes/
                    # ctx.leaders). Emit the count-faithful degenerate form;
                    # silver's real per-dash data has no gold counterpart.
                    fields["dashes"] = [0] * len(elems)
                # strings_area: gold emits a TF binary field. UNTIL R_2004 it's
                # always 256 bytes; R2007+ only when has_strings_area (any dash
                # with shape_flag & 2). All-zero in the corpus. Silver stores
                # none; emit zeros on pre-R2007 so gold's field matches.
                if not r2007_plus:
                    fields["strings_area"] = "00" * 256
                elif elems and any((e.get("complex") or {}).get("flags", 0) & 2 for e in elems):
                    fields["strings_area"] = "00" * 512
                # drop silver-only names so the generic loop skips them
                for kk in ("elements", "pattern_length", "alignment"):
                    rec.pop(kk, None)
            # Gold's common_object_handle_data.spec reads the extension
            # dictionary handle from the record's handle stream (SINCE R_13b1;
            # pre-R2004 the handle is always present on the wire, R2004+
            # only when the is_xdic_missing data bit is clear). Gold serializes
            # xdicobjhandle whenever one exists — all versions — and the
            # R2004+ is_xdic_missing bit otherwise. Silver stores the handle
            # in the document-level xdic_by_handle map (see _lookup_xdic).
            _xdic = _lookup_xdic(rec.get("handle"))
            if _xdic:
                fields["xdicobjhandle"] = normalize_handle_value(_xdic)
                if r2004_plus:
                    fields["is_xdic_missing"] = 0
            elif r2004_plus:
                fields["is_xdic_missing"] = 1
            if r2013_plus:
                # R2013+ AcDs bit: project from the reader's document-level
                # dwg_data_store_handles set (same source as the object
                # branch; table entries pass has_ds through it — the
                # 2018/LiveSection1 VIEW records carry the bit set).
                fields["has_ds_data"] = 1 if str(rec.get("handle")) in dwg_ds_handles else 0
            if record_gold_type == "DIMSTYLE":
                # flag0 is a derived gold field (bit 0 of the 70 flag); the
                # silver struct doesn't store it. Gold always has it as 0 for
                # ordinary styles.
                fields.setdefault("flag0", 0)
            if record_gold_type == "STYLE":
                # STYLE (dwg.spec 3479): silver stores is_shape_file / height;
                # gold uses is_shape / text_size. Project.
                if "is_shape_file" in rec:
                    fields["is_shape"] = 1 if rec["is_shape_file"] else 0
                if "height" in rec:
                    fields["text_size"] = normalize_value(rec["height"])
                # is_vertical: silver stores it; gold emits it at every era
                # (the golden R13 specimens carry is_vertical in gold's JSON
                # — the old R2000+ gate was written before R13 fixtures
                # existed).
                if "is_vertical" in rec:
                    fields["is_vertical"] = 1 if rec["is_vertical"] else 0
                # generation: silver stores flags dict; gold emits a plain RC.
                # bigfont_file: silver stores big_font_file; gold bigfont_file.
                if "big_font_file" in rec:
                    fields["bigfont_file"] = rec["big_font_file"]
                    rec.pop("big_font_file", None)
                fl = rec.get("flags")
                if isinstance(fl, dict):
                    gen = 0
                    if fl.get("backward"): gen |= 2
                    if fl.get("upside_down"): gen |= 4
                    fields["generation"] = gen
                    rec.pop("flags", None)
                # drop silver-only
                for kk in ("is_shape_file", "height", "true_type_font", "is_vertical",
                           "xref_dependent", "annotative"):
                    rec.pop(kk, None)
            if record_gold_type == "LAYER":
                # LAYER (dwg.spec 3298): silver stores flag0 (raw bitmask) and
                # linetype_handle on the struct; project them. color {Index:n}
                # -> int. plotstyle_handle -> plotstyle (handle-wrap).
                if r13_14:
                    # R13/R14 (dwg.spec LAYER 3307-3312): the wire carries the
                    # four flag BITS and gold's JSON emits the named booleans —
                    # not the packed flag0 (R2000b+) / plotstyle (R2000b+).
                    _fl = rec.get("flags")
                    if isinstance(_fl, dict):
                        fields["frozen"] = 1 if _fl.get("frozen") else 0
                        fields["off"] = 1 if _fl.get("off") else 0
                        fields["frozen_in_new"] = 1 if _fl.get("frozen_in_new_viewport") else 0
                        fields["locked"] = 1 if _fl.get("locked") else 0
                    rec.pop("flag0", None)
                    rec.pop("plotstyle_handle", None)
                elif "flag0" in rec:
                    fields["flag0"] = rec["flag0"]
                    rec.pop("flag0", None)
                col = rec.get("color")
                if isinstance(col, dict) and "Index" in col:
                    fields["color"] = col["Index"]
                    rec.pop("color", None)
                if "plotstyle_handle" in rec:
                    fields["plotstyle"] = normalize_handle_value(rec["plotstyle_handle"])
                    rec.pop("plotstyle_handle", None)
                # ltype: silver stores line_type (name) + linetype_handle. Gold
                # emits the handle dict. Project the handle.
                if "linetype_handle" in rec:
                    fields["ltype"] = normalize_handle_value(rec["linetype_handle"])
                    rec.pop("linetype_handle", None)
                # drop silver-only
                for kk in ("plotstyle", "ltype", "plot_style", "book_name", "color_name",
                           "description", "is_plottable", "transparency",
                           "xref_block_record_handle", "material_handle",
                           "flags", "line_type"):
                    rec.pop(kk, None)
                # material: gold emits it as a handle dict (R2007a+); silver
                # stores the raw Handle int. Wrap it. Drop it on pre-R2007 so
                # the generic loop doesn't re-add it.
                if r2007_plus and "material" in rec:
                    fields["material"] = normalize_handle_value(rec["material"])
                rec.pop("material", None)
                # visualstyle (dwg.spec LAYER tail, SINCE R_2013b — the last
                # field of the record's handle stream). Gold serializes the
                # field for EVERY R2013+ layer; unset layers carry a null
                # handle ([5,0,0,0] → code-5/size-0 in gold's JSON). Silver
                # stores it as visual_style_handle (0 = unset).
                if r2013_plus:
                    fields["visualstyle"] = normalize_handle_value(
                        rec.get("visual_style_handle") or 0)
                rec.pop("visual_style_handle", None)
                # linewt (dwg.spec LAYER: the R2000+ 5-bit table index packed
                # in the flag word): gold emits the raw index; silver stores
                # the LineWeight enum. Invert via the lweights[] table.
                if r2000_plus:
                    fields["linewt"] = _lineweight_to_gold(rec.get("line_weight"))
                rec.pop("line_weight", None)
            for k, v in rec.items():
                if k in ("handle", "owner", "owner_handle", "reactors", "xdictionary_handle"):
                    continue
                # VPORT: the dedicated block above already projected/renamed or
                # intentionally dropped every view-param key; skip them here so
                # the generic loop doesn't re-add them under silver names.
                if record_gold_type == "VPORT":
                    if k in ("view_height", "view_width", "view_center", "view_direction",
                             "view_target", "view_twist", "lens_length", "front_clip",
                             "back_clip", "snap_rotation", "snap_base", "snap_spacing",
                             "grid_spacing", "ucs_origin", "ucs_x_axis", "ucs_y_axis",
                             "ucs_elevation", "ucs_ortho_type", "ucs_at_origin",
                             "grid_major", "brightness", "contrast", "default_lighting_type",
                             "snap_isopair", "grid_on", "snap_on", "snap_style",
                             "fast_zoom", "ucsfollow", "use_default_lights",
                             "ucs_per_viewport", "render_mode", "grid_flags",
                             "ambient_color", "named_ucs_handle", "base_ucs_handle",
                             "sun_handle", "background_handle", "visual_style_handle",
                             "ucsicon_lower", "ucsicon_origin",
                             # silver-only VPORT fields gold never emits on the
                             # binary-DWG path (clipping flags live in VIEWMODE/
                             # plot settings; perspective is DXF-only):
                             "perspective", "front_clipping", "back_clipping",
                             "front_clip_at_eye",
                             # silver xref bookkeeping (the gold-side is_xref_*
                             # bits are emitted by the table-record block; these
                             # silver-internal duplicates never appear in gold):
                             "xref_reference", "xref_resolved", "xref_dependent",
                             "xref_handle"):
                        continue
                # BLOCK_HEADER (dwg.spec 3146-3278): rename silver storage
                # names to gold's, version-gate the R2004+/R2007+ fields, and
                # drop silver-only fields gold never serializes to JSON.
                if record_gold_type == "BLOCK_HEADER":
                    if k == "base_point":
                        fields["base_pt"] = normalize_value(v); continue
                    if k == "block_entity_handle":
                        fields["block_entity"] = normalize_handle_value(v); continue
                    if k == "block_end_handle":
                        fields["endblk_entity"] = normalize_handle_value(v); continue
                    if k == "entity_handles":
                        if r2004_plus and v:
                            fields["entities"] = [normalize_handle_value(h) for h in v]
                        elif not r2004_plus:
                            # dwg.spec 3241 VERSIONS(R_13b1, R_2000): the
                            # first/last_entity pair is emitted (as null
                            # handles for empty blocks) instead of the
                            # R2004+ entities vector; derive both from
                            # silver's entity_handles. Xref blocks never
                            # read the pair (the wire read is
                            # xref-conditional — silver's reader and
                            # gold's spec agree), so gold's JSON omits
                            # the fields entirely for them.
                            _fl2 = rec.get("flags") if isinstance(rec.get("flags"), dict) else {}
                            if not (_fl2.get("is_xref") or _fl2.get("is_xref_overlay")):
                                eh = v if isinstance(v, list) else []
                                fields["first_entity"] = normalize_handle_value(eh[0] if eh else 0)
                                fields["last_entity"] = normalize_handle_value(eh[-1] if eh else 0)
                        continue
                    if k == "name":
                        # consumed by the flags/name block above
                        continue
                    if k == "units":
                        if r2007_plus:
                            fields["insert_units"] = normalize_value(v)
                        continue
                    if k == "scale_uniformly":
                        if r2007_plus:
                            fields["block_scaling"] = 1 if v else 0
                        continue
                    if k == "explodable":
                        if r2007_plus:
                            fields["explodable"] = 1 if v else 0
                        continue
                    # gold never serializes these to binary-DWG JSON:
                    # flags (composite -> split bits done above),
                    # xref_resolved (emitted as is_xref_resolved above),
                    # preview_data, insert_count_bytes, xref_path,
                    # is_xdic_missing (already set above), has_ds_data
                    # (set above).
                    if k in ("flags", "xref_resolved", "preview_data",
                             "insert_count_bytes", "xref_path"):
                        continue
                    if k == "insert_handles":
                        # dwg.spec 3272 IF_FREE_OR_SINCE(R_2000b): gold
                        # emits the inserts HANDLE_VECTOR only when
                        # num_inserts > 0. (The old drop list treated these
                        # as unserializable — wrong, 79 missing rows.)
                        if v:
                            fields["inserts"] = [normalize_handle_value(h) for h in v]
                        continue
                # DIMSTYLE: silver stores lowercase dim* names; gold uses
                # uppercase DIM*. Uppercase the dim prefix and drop
                # silver-only derived fields (true_color, name, handle forms)
                # and version-gated fields gold doesn't have in this version.
                if record_gold_type == "DIMSTYLE" and k.startswith("dim"):
                    # DIMTXSTY: gold stores the text-style HANDLE; silver
                    # stores the resolved name in `dimtxsty`. Emit the handle.
                    if k == "dimtxsty":
                        h = rec.get("dimtxsty_handle")
                        if h is not None:
                            fields["DIMTXSTY"] = normalize_handle_value(h)
                        continue
                    # DIMLTYPE/DIMLTEX1/DIMLTEX2: gold emits these handles
                    # SINCE R_2007a; silver stores them as *_handle.
                    if k in ("dimltex_handle", "dimltex1_handle", "dimltex2_handle"):
                        if r2007_plus:
                            gk = {"dimltex_handle": "DIMLTYPE",
                                  "dimltex1_handle": "DIMLTEX1",
                                  "dimltex2_handle": "DIMLTEX2"}[k]
                            fields[gk] = normalize_handle_value(v)
                        continue
                    if r13_14 and k in ("dimblk_name", "dimblk1_name", "dimblk2_name"):
                        # R13/R14 (gold dwg.spec DIMSTYLE, VERSIONS R_13b1-
                        # R_14): the block-arrow fields are TVs, not
                        # handles — gold's JSON emits DIMBLK_T/DIMBLK1_T/
                        # DIMBLK2_T from them.
                        fields[{"dimblk_name": "DIMBLK_T",
                                "dimblk1_name": "DIMBLK1_T",
                                "dimblk2_name": "DIMBLK2_T"}[k]] = normalize_value(v)
                        continue
                    if k.endswith(("_true_color", "_name", "_handle")):
                        continue
                    ku = k.upper()
                    # Version gates from gold dwg.spec.
                    if ku in ("DIMFXL", "DIMJOGANG", "DIMTFILL", "DIMTFILLCLR", "DIMARCSYM", "DIMFXLON") and not r2007_plus:
                        continue
                    if ku in ("DIMTXTDIRECTION", "DIMALTMZF", "DIMALTMZS", "DIMMZF", "DIMMZS") and not r2010_plus:
                        continue
                    if r13_14:
                        # R13/R14 (gold dwg.spec DIMSTYLE): DIMFIT/DIMUNIT
                        # are ON the wire (the R13-R14 block) — emit them;
                        # the R2000b+ set is not (silver's model defaults) —
                        # suppress.
                        if ku in ("DIMFIT", "DIMUNIT"):
                            fields[ku] = normalize_value(v)
                            continue
                        if ku in ("DIMLWD", "DIMLWE", "DIMTMOVE", "DIMLUNIT",
                                  "DIMDSEP", "DIMADEC", "DIMAZIN", "DIMALTRND",
                                  "DIMATFIT", "DIMFRAC"):
                            continue
                    elif ku in ("DIMFIT", "DIMUNIT"):
                        # R2000+: superseded fields gold never serializes.
                        continue
                    # DIMLDRBLK/DIMBLK/DIMBLK1/DIMBLK2: gold emits these
                    # handles SINCE R_2000b (code 5, dwg.spec); silver stores
                    # raw Handle ints. Wrap for the differ's handle resolution.
                    # R13/R14: the arrow blocks are TVs (emitted above);
                    # the handle slots are silver defaults — never compare.
                    if ku in ("DIMLDRBLK", "DIMBLK", "DIMBLK1", "DIMBLK2"):
                        if not r13_14:
                            fields[ku] = normalize_handle_value(v)
                        continue
                    fields[ku] = normalize_value(v)
                    continue
                # Drop silver's xref bookkeeping duplicates (already emitted
                # as is_xref_* above) and the text-style name duplicate.
                if record_gold_type in ("DIMSTYLE", "LTYPE") and k in ("xref_reference", "xref_resolved", "xref_dependent", "xref_handle", "annotative"):
                    continue
                # LAYER.linewt: gold stores the raw lweights[] index; silver
                # stores the enum string. Map it.
                if k == "line_weight":
                    fields["linewt"] = _lineweight_to_gold(v)
                    continue
                # BLOCK_HEADER.layout / LAYER.material are handle-stream fields
                # in gold (code 5); silver stores raw Handle ints. Wrap for
                # the differ's handle resolution.
                if (record_gold_type, k) in (("BLOCK_HEADER", "layout"), ("LAYER", "material")):
                    fields[k] = normalize_handle_value(v)
                    continue
                if is_ignored(k, ignore_set, ignore_patterns):
                    continue
                fields[k] = normalize_value(v)
            out.append({"type": record_gold_type, "fields": fields})

    # VX family (dwg.spec 3191 VX_CONTROL + 3224 VX_TABLE_RECORD, live):
    # silver dumps the vx table at the root (vx_table {handle, entries});
    # gold serializes the control (owner null) + one record per entry with
    # the entry/viewport/prev_entry handles (verified ex2000 721/722/723).
    _vxt = data.get("vx_table") or {}
    if isinstance(_vxt, dict) and isinstance(_vxt.get("handle"), int) \
            and _vxt["handle"]:
        out.append({"type": "VX_CONTROL",
                    "fields": {"handle": normalize_handle_value(_vxt["handle"]),
                               "ownerhandle": normalize_handle_value(0)}})
        for _vxe in (_vxt.get("entries") or {}).values():
            if not isinstance(_vxe, dict) or not isinstance(_vxe.get("handle"), int):
                continue
            out.append({"type": "VX_TABLE_RECORD", "fields": {
                "handle": normalize_handle_value(_vxe["handle"]),
                "ownerhandle": normalize_handle_value(_vxt["handle"]),
                "name": _vxe.get("name", ""),
                "is_xref_ref": 1 if _vxe.get("is_xref_reference") else 0,
                "is_xref_resolved": 1 if _vxe.get("is_xref_resolved") else 0,
                "is_xref_dep": 1 if _vxe.get("is_xref_dependent") else 0,
                "xref": normalize_handle_value(_vxe.get("xref_handle") or 0),
                "is_on": 1 if _vxe.get("is_on") else 0,
                "viewport": normalize_handle_value(_vxe.get("viewport") or 0),
                "prev_entry": normalize_handle_value(_vxe.get("previous_entry") or 0),
            }})

    # SEQEND ordinal alignment: gold's SEQEND records follow document order
    # (ascending handle on every corpus file verified); silver's synthesized
    # seqends emit at their parents' iteration positions, which interleaves
    # wrongly once the INSERT chains add theirs. Reorder the SEQEND records
    # ascending by handle so the differ's (type, ordinal) alignment pairs
    # them with gold's (probes ch_ex2000/…/ch_ex2018).
    _seq_slots = [i for i, r in enumerate(out)
                  if isinstance(r, dict) and r.get("type") == "SEQEND"]
    if len(_seq_slots) > 1:
        _seq_recs = [out[i] for i in _seq_slots]
        _seq_recs.sort(key=lambda r: ((r.get("fields") or {}).get("handle") or {})
                       .get("absref", 0) if isinstance((r.get("fields") or {}).get("handle"), dict) else 0)
        for i, _old in enumerate(_seq_slots):
            out[_old] = _seq_recs[i]

    return out


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: normalize_silver.py <silver.json> [ignore_fields.toml]", file=sys.stderr)
        sys.exit(1)
    ignore: Optional[Dict[str, Any]] = None
    if len(sys.argv) >= 3:
        ignore = load_ignore_fields(sys.argv[2])
    with open(sys.argv[1], "r", encoding="utf-8") as f:
        data = json.load(f)
    norm = normalize_silver(data, ignore)
    json.dump(norm, sys.stdout, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
