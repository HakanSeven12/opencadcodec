//! Block counting: the instances behind the `AcCount` / `AcCount2` fields and
//! the COUNT feature of a host application.
//!
//! A count looks at the block references (INSERTs) of model space. References
//! of the same block with the same position, rotation and scale overlap each
//! other: the first one counts, every further copy is a *duplicate* (an error
//! of the count that is reported, not counted).
//!
//! Count field codes carry their query as JSON:
//!
//! - `\AcCount {"key":{},"name":"CHAIR","type":"block"}` — the references of a
//!   block; `key` narrows them by `layer`, `scale` (`{"x":..,"y":..,"z":..}`,
//!   magnitudes) and `mirrorState`.
//! - `\AcCount2 {"boundaryObjectHandle":"8D6C","evaluatorId":"AcCount2",...}`
//!   — the same, only the references lying wholly inside a closed polyline.
//! - `\AcCount {"groupCondition":{...},"targets":["8D1A"],"type":"single"}` —
//!   the references of the block the target reference shows; several targets
//!   (or one that is not a reference) count the copies of the group.
//!
//! JSON that does not parse, or names no block, counts `0`; a code without
//! JSON shows `####`.

use crate::document::CadDocument;
use crate::entities::{EntityType, Insert};
use crate::types::{Handle, Vector3};
use std::collections::HashMap;

/// One block reference taking part in a count.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockInstance {
    pub handle: Handle,
    /// Block name as the block table spells it.
    pub name: String,
    pub layer: String,
    /// Scale magnitudes (a mirrored reference counts with its positive scale).
    pub scale: [f64; 3],
    /// The reference is mirrored (an odd number of negative scale factors).
    pub mirrored: bool,
    /// The earlier reference this one overlaps exactly; such a duplicate is
    /// not counted.
    pub duplicate_of: Option<Handle>,
}

/// What narrows a count beyond the block name. `None` matches any value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CountKey {
    pub layer: Option<String>,
    pub scale: Option<[f64; 3]>,
    pub mirror_state: Option<bool>,
}

impl CountKey {
    /// The key of `instance` restricted to the chosen properties.
    pub fn of(instance: &BlockInstance, layer: bool, scale: bool, mirror: bool) -> Self {
        Self {
            layer: layer.then(|| instance.layer.clone()),
            scale: scale.then_some(instance.scale),
            mirror_state: mirror.then_some(instance.mirrored),
        }
    }

    pub fn matches(&self, instance: &BlockInstance) -> bool {
        self.layer.as_ref().is_none_or(|l| l.eq_ignore_ascii_case(&instance.layer))
            && self.scale.is_none_or(|s| same_scale(s, instance.scale))
            && self.mirror_state.is_none_or(|m| m == instance.mirrored)
    }

    /// The key object as the reference writes it (`{}` when empty; members in
    /// name order).
    pub fn json(&self) -> String {
        let mut parts = Vec::new();
        if let Some(layer) = &self.layer {
            parts.push(format!("\"layer\":{}", json_string(layer)));
        }
        if let Some(m) = self.mirror_state {
            parts.push(format!("\"mirrorState\":{m}"));
        }
        if let Some([x, y, z]) = self.scale {
            parts.push(format!("\"scale\":{{\"x\":{x:?},\"y\":{y:?},\"z\":{z:?}}}"));
        }
        format!("{{{}}}", parts.join(","))
    }

    /// Row label for an expanded count: the chosen values joined by `_`
    /// (`0_1.0000_Default`).
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if let Some(layer) = &self.layer {
            parts.push(layer.clone());
        }
        if let Some([x, y, z]) = self.scale {
            if same_scale([x, x, x], [x, y, z]) {
                parts.push(format!("{x:.4}"));
            } else {
                parts.push(format!("{x:.4},{y:.4},{z:.4}"));
            }
        }
        if let Some(m) = self.mirror_state {
            parts.push(if m { "Mirrored" } else { "Default" }.to_string());
        }
        parts.join("_")
    }
}

fn same_scale(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0))
}

/// The counted block references of model space, in drawing order. `area`
/// (a closed WCS polygon) keeps only references whose extents lie wholly
/// inside it. Anonymous, layout and external-reference blocks take no part.
pub fn block_instances(doc: &CadDocument, area: Option<&[[f64; 2]]>) -> Vec<BlockInstance> {
    let mut seen: HashMap<(String, [i64; 7]), Handle> = HashMap::new();
    let mut out = Vec::new();
    for entity in doc.model_space_entities() {
        let EntityType::Insert(insert) = entity else {
            continue;
        };
        let Some(record) = doc.block_records.get(&insert.block_name) else {
            continue;
        };
        if record.name.starts_with('*')
            || record.flags.anonymous
            || record.flags.is_xref
            || record.flags.is_xref_overlay
            || record.flags.is_external
        {
            continue;
        }
        if let Some(polygon) = area {
            if !inside_polygon(&insert_outline(doc, insert, 0), polygon) {
                continue;
            }
        }
        let (sx, sy, sz) = (insert.x_scale(), insert.y_scale(), insert.z_scale());
        // ponytail: duplicates are exact overlaps (same transform), rounded to 1e-6.
        let r = |v: f64| (v * 1e6).round() as i64;
        let p = insert.insert_point;
        let key = (
            record.name.to_ascii_uppercase(),
            [r(p.x), r(p.y), r(p.z), r(insert.rotation), r(sx), r(sy), r(sz)],
        );
        let handle = insert.common.handle;
        let duplicate_of = match seen.get(&key) {
            Some(first) => Some(*first),
            None => {
                seen.insert(key, handle);
                None
            }
        };
        out.push(BlockInstance {
            handle,
            name: record.name.clone(),
            layer: insert.common.layer.clone(),
            scale: [sx.abs(), sy.abs(), sz.abs()],
            mirrored: sx * sy * sz < 0.0,
            duplicate_of,
        });
    }
    out
}

/// One block's line of a count: the counted references and the duplicates.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockCount {
    pub name: String,
    pub counted: Vec<Handle>,
    pub duplicates: Vec<Handle>,
}

/// Every block of `instances` with its counted and duplicate references,
/// sorted by name (case-insensitively).
pub fn block_counts(instances: &[BlockInstance]) -> Vec<BlockCount> {
    let mut map: HashMap<String, BlockCount> = HashMap::new();
    for i in instances {
        let entry = map.entry(i.name.to_ascii_uppercase()).or_insert_with(|| BlockCount {
            name: i.name.clone(),
            counted: Vec::new(),
            duplicates: Vec::new(),
        });
        if i.duplicate_of.is_some() {
            entry.duplicates.push(i.handle);
        } else {
            entry.counted.push(i.handle);
        }
    }
    let mut out: Vec<BlockCount> = map.into_values().collect();
    out.sort_by_key(|c| c.name.to_ascii_uppercase());
    out
}

/// The counted (non-duplicate) references of block `name` matching `key`.
pub fn count_of(instances: &[BlockInstance], name: &str, key: &CountKey) -> usize {
    instances
        .iter()
        .filter(|i| i.duplicate_of.is_none() && i.name.eq_ignore_ascii_case(name) && key.matches(i))
        .count()
}

/// The distinct keys of block `name` when expanded by the chosen properties,
/// each with its count, in the order the drawing first shows them.
pub fn expanded_counts(
    instances: &[BlockInstance],
    name: &str,
    layer: bool,
    scale: bool,
    mirror: bool,
) -> Vec<(CountKey, usize)> {
    let mut out: Vec<(CountKey, usize)> = Vec::new();
    for i in instances.iter().filter(|i| i.duplicate_of.is_none() && i.name.eq_ignore_ascii_case(name)) {
        let key = CountKey::of(i, layer, scale, mirror);
        match out.iter_mut().find(|(k, _)| k.matches(i)) {
            Some((_, n)) => *n += 1,
            None => out.push((key, 1)),
        }
    }
    out
}

/// `{"key":{...},"name":"CHAIR","type":"block"}`.
pub fn block_json(name: &str, key: &CountKey) -> String {
    format!("{{\"key\":{},\"name\":{},\"type\":\"block\"}}", key.json(), json_string(name))
}

/// `{"boundaryObjectHandle":"8D6C","evaluatorId":"AcCount2","key":{...},...}`.
pub fn area_json(name: &str, key: &CountKey, boundary: Handle) -> String {
    format!(
        "{{\"boundaryObjectHandle\":\"{:X}\",\"evaluatorId\":\"AcCount2\",\"key\":{},\"name\":{},\"type\":\"block\"}}",
        boundary.value(),
        key.json(),
        json_string(name)
    )
}

/// `{"groupCondition":{...},"targets":["8D1A"],"type":"single"}`.
pub fn single_json(targets: &[Handle]) -> String {
    let targets: Vec<String> = targets.iter().map(|h| format!("\"{:X}\"", h.value())).collect();
    format!(
        "{{\"groupCondition\":{{\"attributes\":[],\"parameters\":[],\"properties\":[]}},\"targets\":[{}],\"type\":\"single\"}}",
        targets.join(",")
    )
}

/// The objects like one picked object that is not a block reference, in
/// model space and inside the counted block references (nested ones too).
/// Objects match at any position, rotation and size (see [`similar`]). Each match is listed as the top-level
/// object it is drawn by (a reference repeats once per match inside it).
pub fn similar_matches(doc: &CadDocument, target: &EntityType, area: Option<&[[f64; 2]]>) -> Vec<Vec<Handle>> {
    let duplicates: Vec<Handle> =
        block_instances(doc, None).into_iter().filter(|i| i.duplicate_of.is_some()).map(|i| i.handle).collect();
    let matches = |e: &EntityType| similar(target, e);
    let mut out = Vec::new();
    for e in doc.model_space_entities() {
        let top = e.common().handle;
        if duplicates.contains(&top) {
            continue;
        }
        let mut found = Vec::new();
        nested_matches(doc, e, &matches, &mut Vec::new(), 0, &mut found);
        for outline in found {
            if area.is_none_or(|polygon| inside_polygon(&outline, polygon)) {
                out.push(vec![top]);
            }
        }
    }
    out
}

/// Outlines (in WCS) of the objects under `e` that `matches` accepts; `chain`
/// holds the references `e` is drawn through, outermost first.
fn nested_matches(
    doc: &CadDocument,
    e: &EntityType,
    matches: &dyn Fn(&EntityType) -> bool,
    chain: &mut Vec<Insert>,
    depth: usize,
    found: &mut Vec<Vec<Piece>>,
) {
    // A table's drawing (its `*T` block) counts like a reference's; a
    // dimension's does not.
    let placed = match e {
        EntityType::Insert(ins) => Some(ins.clone()),
        EntityType::Table(t) => {
            // DWG tables may name their block only by its record.
            let name = t
                .block_record_handle
                .and_then(|h| doc.block_records.iter().find(|r| r.handle == h))
                .map_or_else(|| t.block_name.clone(), |r| r.name.clone());
            let mut ins = Insert::new(name, t.insertion_point);
            ins.rotation = t.horizontal_direction.y.atan2(t.horizontal_direction.x);
            ins.normal = t.normal;
            Some(ins)
        }
        _ => None,
    };
    if let Some(ins) = placed {
        let Some(record) = doc.block_records.get(&ins.block_name) else {
            return;
        };
        if depth >= 8 {
            return;
        }
        chain.push(ins);
        for &h in &record.entity_handles {
            if let Some(child) = doc.get_entity(h) {
                nested_matches(doc, child, matches, chain, depth + 1, found);
            }
        }
        chain.pop();
        return;
    }
    if e.common().invisible || !matches(e) {
        return;
    }
    let mut outline = entity_outline(doc, e);
    for ins in chain.iter().rev() {
        let base = doc.block_records.get(&ins.block_name).map(|r| r.base_point).unwrap_or_default();
        let t = ins.get_transform();
        outline = outline.into_iter().map(|p| p.placed(&t, base)).collect();
    }
    found.push(outline);
}

/// A polyline's shape independent of size, rotation and position: per
/// vertex the segment's share of the length, the turn to the next segment
/// and the bulge.
fn polyline_shape(e: &EntityType) -> Option<Vec<[f64; 3]>> {
    let EntityType::LwPolyline(pl) = e else {
        return None;
    };
    let n = pl.vertices.len();
    let segs = if pl.is_closed { n } else { n.saturating_sub(1) };
    let dir = |i: usize| {
        let (a, b) = (pl.vertices[i % n].location, pl.vertices[(i + 1) % n].location);
        (b.x - a.x, b.y - a.y)
    };
    let total: f64 = (0..segs).map(|i| dir(i).0.hypot(dir(i).1)).sum();
    if total <= 1e-12 {
        return None;
    }
    Some(
        (0..segs)
            .map(|i| {
                let (dx, dy) = dir(i);
                let turn = if pl.is_closed || i + 1 < segs {
                    let (ex, ey) = dir(i + 1);
                    (dx * ey - dy * ex).atan2(dx * ex + dy * ey)
                } else {
                    0.0
                };
                [dx.hypot(dy) / total, turn, pl.vertices[i].bulge]
            })
            .collect(),
    )
}

/// Two polyline shapes are alike when one is the other started at another
/// vertex (closed ones).
fn same_shape(a: Option<&[[f64; 3]]>, b: Option<&[[f64; 3]]>, closed: bool) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return false;
    };
    let close = |x: &[f64; 3], y: &[f64; 3]| x.iter().zip(y).all(|(p, q)| (p - q).abs() < 1e-6);
    let shifts = if closed { a.len() } else { 1 };
    // A mirrored copy turns the other way.
    let mirrored: Vec<[f64; 3]> = b.iter().map(|x| [x[0], -x[1], -x[2]]).collect();
    a.len() == b.len()
        && [b, mirrored.as_slice()]
            .iter()
            .any(|b| (0..shifts).any(|k| a.iter().enumerate().all(|(i, x)| close(x, &b[(i + k) % b.len()]))))
}

/// Whether `e` counts as a copy of the single picked `target` (as measured
/// on the reference): circles always; arcs, ellipses, polylines, splines and
/// hatches when their shape is the same at any position, rotation and size;
/// texts with the same string; lines always; anything else objects of its
/// kind.
fn similar(target: &EntityType, e: &EntityType) -> bool {
    let eq = |a: f64, b: f64| (a - b).abs() < 1e-6;
    let sweep = |a: f64, b: f64| (b - a).rem_euclid(std::f64::consts::TAU);
    match (target, e) {
        (EntityType::Line(_), EntityType::Line(_)) => true,
        (EntityType::Arc(a), EntityType::Arc(b)) => eq(sweep(a.start_angle, a.end_angle), sweep(b.start_angle, b.end_angle)),
        (EntityType::Ellipse(a), EntityType::Ellipse(b)) => {
            eq(a.minor_axis_ratio, b.minor_axis_ratio)
                && eq(sweep(a.start_parameter, a.end_parameter), sweep(b.start_parameter, b.end_parameter))
        }
        (EntityType::LwPolyline(a), EntityType::LwPolyline(b)) => {
            a.is_closed == b.is_closed && same_shape(polyline_shape(target).as_deref(), polyline_shape(e).as_deref(), a.is_closed)
        }
        (EntityType::Text(a), EntityType::Text(b)) => a.value == b.value,
        (EntityType::MText(a), EntityType::MText(b)) => a.value == b.value,
        (EntityType::Spline(a), EntityType::Spline(b)) => {
            // Splines kept by their fit points compare by those.
            let pts = |s: &crate::entities::Spline| {
                normalized(if s.control_points.is_empty() { &s.fit_points } else { &s.control_points })
            };
            let (pa, pb) = (pts(a), pts(b));
            a.degree == b.degree
                && pa.len() == pb.len()
                && !pa.is_empty()
                && pa.iter().zip(&pb).all(|(p, q)| eq(p[0], q[0]) && eq(p[1], q[1]))
        }
        (EntityType::Hatch(a), EntityType::Hatch(b)) => {
            let (pa, pb) = (hatch_points(a), hatch_points(b));
            let n = pb.len();
            a.is_solid == b.is_solid
                && a.pattern.name.eq_ignore_ascii_case(&b.pattern.name)
                && pa.len() == n
                && n > 0
                && (0..n).any(|k| {
                    let turned: Vec<Vector3> = (0..n).map(|i| pb[(i + k) % n]).collect();
                    let (x, y) = (normalized(&pa), normalized(&turned));
                    x.len() == y.len() && x.iter().zip(&y).all(|(p, q)| eq(p[0], q[0]) && eq(p[1], q[1]))
                })
        }
        _ => std::mem::discriminant(e) == std::mem::discriminant(target),
    }
}

/// A hatch's boundary corners (polyline vertices and edge start points),
/// path after path.
// ponytail: curved edges count by their start point only.
fn hatch_points(h: &crate::entities::Hatch) -> Vec<Vector3> {
    use crate::entities::hatch::BoundaryEdge;
    let mut out = Vec::new();
    for path in &h.paths {
        for edge in &path.edges {
            match edge {
                BoundaryEdge::Polyline(pl) => out.extend(pl.vertices.iter().map(|v| Vector3::new(v.x, v.y, 0.0))),
                BoundaryEdge::Line(l) => out.push(Vector3::new(l.start.x, l.start.y, 0.0)),
                BoundaryEdge::CircularArc(a) => out.push(Vector3::new(a.center.x, a.center.y, a.radius)),
                BoundaryEdge::EllipticArc(e) => out.push(Vector3::new(e.center.x, e.center.y, 0.0)),
                BoundaryEdge::Spline(sp) => out.extend(sp.control_points.iter().map(|v| Vector3::new(v.x, v.y, 0.0))),
            }
        }
    }
    out
}

/// Points moved, turned and scaled so the first lies at the origin and the
/// first distinct one at (1, 0).
fn normalized(points: &[Vector3]) -> Vec<[f64; 2]> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let Some(d) = points.iter().map(|p| *p - *first).find(|d| d.x.hypot(d.y) > 1e-12) else {
        return Vec::new();
    };
    let (len, ang) = (d.x.hypot(d.y), d.y.atan2(d.x));
    let (c, s) = (ang.cos(), ang.sin());
    points
        .iter()
        .map(|p| {
            let (x, y) = (p.x - first.x, p.y - first.y);
            [(x * c + y * s) / len, (y * c - x * s) / len]
        })
        .collect()
}

/// The copies of a group of picked objects (one object that is not a block
/// reference: [`similar_matches`]): every arrangement of model-space
/// objects that repeats the targets moved by one translation (same kind and
/// shape; references of the same block with the same rotation and scale).
/// Duplicate references take no part, and with `area` every member must lie
/// inside it. Each arrangement lists its members in the targets' order.
pub fn group_matches(doc: &CadDocument, targets: &[Handle], area: Option<&[[f64; 2]]>) -> Vec<Vec<Handle>> {
    if let [one] = targets {
        match doc.get_entity(*one) {
            Some(EntityType::Insert(_)) => {}
            Some(e) => return similar_matches(doc, e, area),
            None => return Vec::new(),
        }
    }
    let r = |v: f64| (v * 1e6).round() as i64;
    let key = |sig: &str, p: Vector3| (sig.to_string(), [r(p.x), r(p.y), r(p.z)]);
    let duplicates: Vec<Handle> = block_instances(doc, None).into_iter().filter(|i| i.duplicate_of.is_some()).map(|i| i.handle).collect();
    let mut index: HashMap<(String, [i64; 3]), Vec<Handle>> = HashMap::new();
    let mut order: Vec<(Handle, String, Vector3)> = Vec::new();
    for e in doc.model_space_entities() {
        let h = e.common().handle;
        if duplicates.contains(&h) {
            continue;
        }
        if let Some((sig, anchor)) = shape_signature(e) {
            index.entry(key(&sig, anchor)).or_default().push(h);
            order.push((h, sig, anchor));
        }
    }
    let mut wanted = Vec::new();
    for t in targets {
        match doc.get_entity(*t).and_then(shape_signature) {
            Some(s) => wanted.push(s),
            // ponytail: an object without a comparable shape only matches itself.
            None if targets.iter().all(|t| doc.get_entity(*t).is_some()) => return vec![targets.to_vec()],
            None => return Vec::new(),
        }
    }
    let Some((sig0, anchor0)) = wanted.first().cloned() else {
        return Vec::new();
    };
    let mut out: Vec<Vec<Handle>> = Vec::new();
    for (h, sig, anchor) in &order {
        if *sig != sig0 {
            continue;
        }
        let d = *anchor - anchor0;
        let mut members = vec![*h];
        for (s, a) in &wanted[1..] {
            let Some(m) = index.get(&key(s, *a + d)).and_then(|hs| hs.iter().find(|m| !members.contains(m))) else {
                break;
            };
            members.push(*m);
        }
        if members.len() != wanted.len() {
            continue;
        }
        if let Some(polygon) = area {
            let inside = members
                .iter()
                .all(|m| doc.get_entity(*m).is_some_and(|e| inside_polygon(&entity_outline(doc, e), polygon)));
            if !inside {
                continue;
            }
        }
        let mut sorted = members.clone();
        sorted.sort_by_key(|h| h.value());
        if !out.iter().any(|o| {
            let mut s = o.clone();
            s.sort_by_key(|h| h.value());
            s == sorted
        }) {
            out.push(members);
        }
    }
    out
}

/// What an object must repeat to match in a group, and the point it is
/// placed by. `None` for objects without a comparable shape.
fn shape_signature(entity: &EntityType) -> Option<(String, Vector3)> {
    let f = |v: f64| format!("{:.6}", v + 0.0);
    let v = |p: Vector3| format!("{},{},{}", f(p.x), f(p.y), f(p.z));
    Some(match entity {
        EntityType::Insert(i) => (
            format!(
                "INSERT|{}|{}|{}|{}|{}|{}",
                i.block_name.to_ascii_uppercase(),
                f(i.rotation),
                f(i.x_scale()),
                f(i.y_scale()),
                f(i.z_scale()),
                v(i.normal)
            ),
            i.insert_point,
        ),
        EntityType::Line(l) => (format!("LINE|{}", v(l.end - l.start)), l.start),
        EntityType::Circle(c) => (format!("CIRCLE|{}|{}", f(c.radius), v(c.normal)), c.center),
        EntityType::Arc(a) => (
            format!("ARC|{}|{}|{}|{}", f(a.radius), f(a.start_angle), f(a.end_angle), v(a.normal)),
            a.center,
        ),
        EntityType::LwPolyline(pl) => {
            let first = pl.vertices.first()?.location;
            let shape: Vec<String> = pl
                .vertices
                .iter()
                .map(|p| format!("{},{},{}", f(p.location.x - first.x), f(p.location.y - first.y), f(p.bulge)))
                .collect();
            (
                format!("LWPOLYLINE|{}|{}|{}", pl.is_closed, v(pl.normal), shape.join(";")),
                Vector3::new(first.x, first.y, pl.elevation),
            )
        }
        _ => return None,
    })
}

/// Evaluate an `AcCount` / `AcCount2` field code (`\AcCount {json}`, with
/// or without the `%<…>%` wrapper).
pub fn evaluate(doc: &CadDocument, code: &str) -> String {
    let mut code = code.trim();
    if let Some(inner) = code.strip_prefix("%<").and_then(|c| c.strip_suffix(">%")) {
        code = inner.trim();
    }
    let body = code.trim_start_matches('\\');
    let json = body.split_once(char::is_whitespace).map(|(_, j)| j.trim()).unwrap_or("");
    if json.is_empty() {
        return "####".into();
    }
    evaluate_json(doc, json).unwrap_or(0).to_string()
}

fn evaluate_json(doc: &CadDocument, json: &str) -> Option<usize> {
    let value = Json::parse(json)?;
    let obj = value.object()?;
    match get(obj, "type")?.str()? {
        "single" => {
            let targets: Vec<Handle> = get(obj, "targets")?
                .array()?
                .iter()
                .filter_map(|t| u64::from_str_radix(t.str()?, 16).ok().map(Handle::new))
                .collect();
            match targets.as_slice() {
                [one] => {
                    let Some(EntityType::Insert(insert)) = doc.get_entity(*one) else {
                        return Some(group_matches(doc, &targets, None).len());
                    };
                    let instances = block_instances(doc, None);
                    Some(count_of(&instances, &insert.block_name, &CountKey::default()))
                }
                [] => Some(0),
                _ => Some(group_matches(doc, &targets, None).len()),
            }
        }
        "block" => {
            let name = get(obj, "name")?.str()?;
            let key = parse_key(get(obj, "key"))?;
            let polygon = get(obj, "boundaryObjectHandle")
                .and_then(|h| h.str())
                .and_then(|h| u64::from_str_radix(h, 16).ok())
                .and_then(|h| boundary_polygon(doc, Handle::new(h)));
            let instances = block_instances(doc, polygon.as_deref());
            Some(count_of(&instances, name, &key))
        }
        _ => Some(0),
    }
}

/// What a count field's JSON asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum CountQuery {
    /// A block's references (`type` `block`), in a boundary polyline for
    /// `AcCount2`.
    Block { name: String, key: CountKey, boundary: Option<Handle> },
    /// The picked objects (`type` `single`).
    Single(Vec<Handle>),
}

/// Read a count field's JSON; `None` when it does not parse.
pub fn parse_query(json: &str) -> Option<CountQuery> {
    let value = Json::parse(json.trim())?;
    let obj = value.object()?;
    match get(obj, "type")?.str()? {
        "single" => Some(CountQuery::Single(
            get(obj, "targets")?
                .array()?
                .iter()
                .filter_map(|t| u64::from_str_radix(t.str()?, 16).ok().map(Handle::new))
                .collect(),
        )),
        "block" => Some(CountQuery::Block {
            name: get(obj, "name")?.str()?.to_string(),
            key: parse_key(get(obj, "key"))?,
            boundary: get(obj, "boundaryObjectHandle")
                .and_then(|h| h.str())
                .and_then(|h| u64::from_str_radix(h, 16).ok())
                .map(Handle::new),
        }),
        _ => None,
    }
}

fn parse_key(key: Option<&Json>) -> Option<CountKey> {
    let Some(key) = key else {
        return Some(CountKey::default());
    };
    let obj = key.object()?;
    let scale = match get(obj, "scale") {
        Some(s) => {
            let s = s.object()?;
            Some([get(s, "x")?.num()?, get(s, "y")?.num()?, get(s, "z")?.num()?])
        }
        None => None,
    };
    Some(CountKey {
        layer: get(obj, "layer").and_then(|l| l.str()).map(str::to_string),
        scale,
        mirror_state: get(obj, "mirrorState").and_then(|m| match m {
            Json::Bool(b) => Some(*b),
            _ => None,
        }),
    })
}

/// The WCS outline of a closed LWPOLYLINE used as a count area; `None` when
/// the object cannot be one (see [`valid_boundary`]).
pub fn boundary_polygon(doc: &CadDocument, handle: Handle) -> Option<Vec<[f64; 2]>> {
    match doc.get_entity(handle)? {
        EntityType::LwPolyline(pl) if valid_boundary(pl) => {
            Some(pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect())
        }
        _ => None,
    }
}

/// A count area boundary: a closed polyline of at least three vertices made
/// of line segments only (no bulge) that does not intersect itself.
pub fn valid_boundary(pl: &crate::entities::LwPolyline) -> bool {
    let ring: Vec<[f64; 2]> = pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect();
    pl.is_closed && ring.len() >= 3 && pl.vertices.iter().all(|v| v.bulge.abs() < 1e-12) && !self_intersects(&ring)
}

/// Whether a closed ring crosses itself.
pub fn self_intersects(ring: &[[f64; 2]]) -> bool {
    let n = ring.len();
    (0..n).any(|i| {
        ((i + 2)..n).any(|j| {
            // Neighbouring edges share a vertex; the last edge meets the first.
            !(i == 0 && j == n - 1) && segments_cross(ring[i], ring[(i + 1) % n], ring[j], ring[(j + 1) % n])
        })
    })
}

/// Segments ab and cd meet (touching counts).
fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let on = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        r[0] >= p[0].min(q[0]) && r[0] <= p[0].max(q[0]) && r[1] >= p[1].min(q[1]) && r[1] <= p[1].max(q[1])
    };
    let (d1, d2, d3, d4) = (orient(c, d, a), orient(c, d, b), orient(a, b, c), orient(a, b, d));
    (d1 * d2 < 0.0 && d3 * d4 < 0.0)
        || (d1 == 0.0 && on(c, d, a))
        || (d2 == 0.0 && on(c, d, b))
        || (d3 == 0.0 && on(a, b, c))
        || (d4 == 0.0 && on(a, b, d))
}

/// Even-odd point-in-polygon test.
pub fn point_in_polygon(p: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[(i + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

/// A piece of drawn geometry in WCS: a straight segment, or an elliptic arc
/// `c + u·cos t + v·sin t` for `t` in `t0..=t1` (circles, arcs, ellipses
/// and bulges, also as references stretch them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Piece {
    Segment(Vector3, Vector3),
    Conic { c: Vector3, u: Vector3, v: Vector3, t0: f64, t1: f64 },
}

impl Piece {
    fn start(&self) -> Vector3 {
        match *self {
            Piece::Segment(a, _) => a,
            Piece::Conic { c, u, v, t0, .. } => c + u * t0.cos() + v * t0.sin(),
        }
    }

    /// The piece moved by a reference (`p ↦ t(p − base)`); conics stay exact.
    fn placed(self, t: &crate::types::Transform, base: Vector3) -> Piece {
        let at = |p: Vector3| t.apply(p - base);
        match self {
            Piece::Segment(a, b) => Piece::Segment(at(a), at(b)),
            Piece::Conic { c, u, v, t0, t1 } => {
                let c2 = at(c);
                Piece::Conic { c: c2, u: at(c + u) - c2, v: at(c + v) - c2, t0, t1 }
            }
        }
    }

    /// Points along the piece (for extents and zooming).
    pub fn points(&self) -> Vec<Vector3> {
        match *self {
            Piece::Segment(a, b) => vec![a, b],
            Piece::Conic { c, u, v, t0, t1 } => {
                let n = (((t1 - t0) / 5f64.to_radians()).ceil() as usize).max(2);
                (0..=n)
                    .map(|k| {
                        let t = t0 + (t1 - t0) * k as f64 / n as f64;
                        c + u * t.cos() + v * t.sin()
                    })
                    .collect()
            }
        }
    }

    /// Whether the piece meets the segment ab (in plan).
    fn meets(&self, a: [f64; 2], b: [f64; 2]) -> bool {
        match *self {
            Piece::Segment(p, q) => segments_cross([p.x, p.y], [q.x, q.y], a, b),
            Piece::Conic { c, u, v, t0, t1 } => {
                // n·(p(t) − a) = k + α cos t + β sin t = 0 along the edge's line.
                let d = [b[0] - a[0], b[1] - a[1]];
                let n = [-d[1], d[0]];
                let dot = |x: f64, y: f64| n[0] * x + n[1] * y;
                let (k, al, be) = (dot(c.x - a[0], c.y - a[1]), dot(u.x, u.y), dot(v.x, v.y));
                let r = al.hypot(be);
                if r < 1e-15 || k.abs() > r {
                    return false;
                }
                let phi = be.atan2(al);
                let w = (-k / r).clamp(-1.0, 1.0).acos();
                [phi + w, phi - w].iter().any(|&t| {
                    let t = t0 + (t - t0).rem_euclid(std::f64::consts::TAU);
                    if t > t1 + 1e-12 {
                        return false;
                    }
                    let p = c + u * t.cos() + v * t.sin();
                    let s = ((p.x - a[0]) * d[0] + (p.y - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]);
                    (-1e-12..=1.0 + 1e-12).contains(&s)
                })
            }
        }
    }
}

/// Whether drawn geometry lies wholly inside the polygon: each piece starts
/// inside it and meets none of its edges.
pub fn inside_polygon(pieces: &[Piece], polygon: &[[f64; 2]]) -> bool {
    let n = polygon.len();
    !pieces.is_empty()
        && pieces.iter().all(|piece| {
            let s = piece.start();
            point_in_polygon([s.x, s.y], polygon) && (0..n).all(|k| !piece.meets(polygon[k], polygon[(k + 1) % n]))
        })
}

/// WCS corners of a reference's extents: the box of its geometry as it lies
/// in the drawing. `None` for an empty block.
pub fn insert_corners(doc: &CadDocument, insert: &Insert, depth: usize) -> Option<Vec<Vector3>> {
    let pts: Vec<Vector3> = insert_outline(doc, insert, depth).iter().flat_map(Piece::points).collect();
    let first = *pts.first()?;
    let (lo, hi) = pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
            Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
        )
    });
    let mut out = Vec::with_capacity(8);
    for x in [lo.x, hi.x] {
        for y in [lo.y, hi.y] {
            for z in [lo.z, hi.z] {
                out.push(Vector3::new(x, y, z));
            }
        }
    }
    Some(out)
}

/// The geometry of a reference in WCS.
pub fn insert_outline(doc: &CadDocument, insert: &Insert, depth: usize) -> Vec<Piece> {
    let Some(record) = doc.block_records.get(&insert.block_name) else {
        return Vec::new();
    };
    let base = record.base_point;
    let t = insert.get_transform();
    let mut out = Vec::new();
    for &h in &record.entity_handles {
        let pieces = match doc.get_entity(h) {
            Some(EntityType::Insert(nested)) if depth < 8 => insert_outline(doc, nested, depth + 1),
            Some(EntityType::Insert(_)) | Some(EntityType::AttributeDefinition(_)) | None => continue,
            Some(e) => entity_outline(doc, e),
        };
        out.extend(pieces.into_iter().map(|p| p.placed(&t, base)));
    }
    out
}

/// The geometry of an entity in WCS: lines, circles, arcs, ellipses and
/// lightweight polylines (bulges as true arcs) exactly, splines along their
/// curve, references through their block, anything else as its box.
pub fn entity_outline(doc: &CadDocument, entity: &EntityType) -> Vec<Piece> {
    use crate::types::Matrix3;
    let tau = std::f64::consts::TAU;
    let circle = |center: Vector3, r: f64, normal: Vector3, t0: f64, t1: f64| {
        let m = Matrix3::arbitrary_axis(normal);
        Piece::Conic { c: m * center, u: m * Vector3::new(r, 0.0, 0.0), v: m * Vector3::new(0.0, r, 0.0), t0, t1 }
    };
    let span = |a0: f64, a1: f64| {
        let s = (a1 - a0).rem_euclid(tau);
        if s < 1e-12 { tau } else { s }
    };
    match entity {
        EntityType::Line(l) => vec![Piece::Segment(l.start, l.end)],
        EntityType::Circle(c) => vec![circle(c.center, c.radius, c.normal, 0.0, tau)],
        EntityType::Arc(a) => vec![circle(a.center, a.radius, a.normal, a.start_angle, a.start_angle + span(a.start_angle, a.end_angle))],
        EntityType::Ellipse(e) => {
            let n = e.normal;
            let minor = Vector3::new(n.y * e.major_axis.z - n.z * e.major_axis.y, n.z * e.major_axis.x - n.x * e.major_axis.z, n.x * e.major_axis.y - n.y * e.major_axis.x) * e.minor_axis_ratio;
            vec![Piece::Conic {
                c: e.center,
                u: e.major_axis,
                v: minor,
                t0: e.start_parameter,
                t1: e.start_parameter + span(e.start_parameter, e.end_parameter),
            }]
        }
        EntityType::LwPolyline(pl) if !pl.vertices.is_empty() => {
            let m = Matrix3::arbitrary_axis(pl.normal);
            let n = pl.vertices.len();
            let segs = if pl.is_closed { n } else { n - 1 };
            let at = |x: f64, y: f64| m * Vector3::new(x, y, pl.elevation);
            let mut out = Vec::new();
            for i in 0..segs {
                let (v0, v1) = (&pl.vertices[i], &pl.vertices[(i + 1) % n]);
                let (p0, p1) = (v0.location, v1.location);
                let (dx, dy) = (p1.x - p0.x, p1.y - p0.y);
                let chord = dx.hypot(dy);
                if v0.bulge.abs() > 1e-12 && chord > 1e-12 {
                    // bulge = tan(sweep / 4); the centre lies on the chord's bisector.
                    let sweep = 4.0 * v0.bulge.atan();
                    let r = chord / (2.0 * (sweep / 2.0).sin().abs());
                    let h = r * (sweep / 2.0).cos() * sweep.signum();
                    let (cx, cy) = ((p0.x + p1.x) / 2.0 - dy / chord * h, (p0.y + p1.y) / 2.0 + dx / chord * h);
                    let a0 = (p0.y - cy).atan2(p0.x - cx);
                    let (t0, t1) = if sweep > 0.0 { (a0, a0 + sweep) } else { (a0 + sweep, a0) };
                    out.push(Piece::Conic {
                        c: at(cx, cy),
                        u: m * Vector3::new(r, 0.0, 0.0),
                        v: m * Vector3::new(0.0, r, 0.0),
                        t0,
                        t1,
                    });
                } else {
                    out.push(Piece::Segment(at(p0.x, p0.y), at(p1.x, p1.y)));
                }
            }
            out
        }
        EntityType::Spline(s) => spline_points(s).windows(2).map(|w| Piece::Segment(w[0], w[1])).collect(),
        EntityType::Insert(ins) => insert_outline(doc, ins, 0),
        e => {
            let b = e.as_entity().bounding_box();
            let z = b.min.z;
            let c = [
                Vector3::new(b.min.x, b.min.y, z),
                Vector3::new(b.max.x, b.min.y, z),
                Vector3::new(b.max.x, b.max.y, z),
                Vector3::new(b.min.x, b.max.y, z),
            ];
            (0..4).map(|i| Piece::Segment(c[i], c[(i + 1) % 4])).collect()
        }
    }
}

/// Points on a spline (de Boor, weights honoured), dense enough that the
/// chords stay within a millionth of its size of the curve.
// ponytail: a spline is followed by chords, not intersected analytically.
fn spline_points(s: &crate::entities::Spline) -> Vec<Vector3> {
    let (cp, k) = (&s.control_points, &s.knots);
    let p = s.degree.max(1) as usize;
    if cp.len() <= p || k.len() != cp.len() + p + 1 {
        return s.fit_points.clone();
    }
    let w = |i: usize| s.weights.get(i).copied().filter(|w| *w > 0.0).unwrap_or(1.0);
    let eval = |t: f64| {
        let mut span = p;
        while span + 1 < cp.len() && k[span + 1] <= t {
            span += 1;
        }
        let mut d: Vec<[f64; 4]> = (0..=p)
            .map(|j| {
                let (q, wj) = (cp[span - p + j], w(span - p + j));
                [q.x * wj, q.y * wj, q.z * wj, wj]
            })
            .collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = span - p + j;
                let den = k[i + p + 1 - r] - k[i];
                let a = if den.abs() < 1e-15 { 0.0 } else { (t - k[i]) / den };
                for c in 0..4 {
                    d[j][c] = (1.0 - a) * d[j - 1][c] + a * d[j][c];
                }
            }
        }
        let h = d[p];
        Vector3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3])
    };
    let (ta, tb) = (k[p], k[cp.len()]);
    let size = cp.iter().fold(0.0f64, |m, q| m.max((*q - cp[0]).x.hypot((*q - cp[0]).y)));
    let n = (cp.len() * 64).max(256);
    let mut out: Vec<Vector3> = (0..=n).map(|i| eval(ta + (tb - ta) * i as f64 / n as f64)).collect();
    // Halve the steps where a chord's midpoint strays from the curve.
    for _ in 0..6 {
        let mut refined = vec![out[0]];
        let mut changed = false;
        let m = out.len() - 1;
        for i in 0..m {
            let (t0, t1) = (ta + (tb - ta) * i as f64 / m as f64, ta + (tb - ta) * (i + 1) as f64 / m as f64);
            let mid = eval((t0 + t1) / 2.0);
            let chord = (out[i] + out[i + 1]) * 0.5;
            if (mid - chord).x.hypot((mid - chord).y) > size * 1e-6 {
                changed = true;
            }
            refined.push(mid);
            refined.push(out[i + 1]);
        }
        if !changed {
            break;
        }
        out = refined;
    }
    out
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ── a small JSON reader (count codes only) ──────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

fn get<'a>(obj: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    obj.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

impl Json {
    fn parse(s: &str) -> Option<Json> {
        let b = s.as_bytes();
        let mut i = 0;
        let v = Self::value(b, &mut i)?;
        Self::ws(b, &mut i);
        (i == b.len()).then_some(v)
    }
    fn object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(o) => Some(o),
            _ => None,
        }
    }
    fn array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
    }
    fn value(b: &[u8], i: &mut usize) -> Option<Json> {
        Self::ws(b, i);
        match *b.get(*i)? {
            b'{' => {
                *i += 1;
                let mut out = Vec::new();
                Self::ws(b, i);
                if b.get(*i) == Some(&b'}') {
                    *i += 1;
                    return Some(Json::Obj(out));
                }
                loop {
                    Self::ws(b, i);
                    let Json::Str(k) = Self::value(b, i)? else { return None };
                    Self::ws(b, i);
                    (b.get(*i) == Some(&b':')).then_some(())?;
                    *i += 1;
                    out.push((k, Self::value(b, i)?));
                    Self::ws(b, i);
                    match b.get(*i)? {
                        b',' => *i += 1,
                        b'}' => {
                            *i += 1;
                            return Some(Json::Obj(out));
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                *i += 1;
                let mut out = Vec::new();
                Self::ws(b, i);
                if b.get(*i) == Some(&b']') {
                    *i += 1;
                    return Some(Json::Arr(out));
                }
                loop {
                    out.push(Self::value(b, i)?);
                    Self::ws(b, i);
                    match b.get(*i)? {
                        b',' => *i += 1,
                        b']' => {
                            *i += 1;
                            return Some(Json::Arr(out));
                        }
                        _ => return None,
                    }
                }
            }
            b'"' => {
                *i += 1;
                let mut s = String::new();
                loop {
                    let c = *b.get(*i)?;
                    *i += 1;
                    match c {
                        b'"' => return Some(Json::Str(s)),
                        b'\\' => {
                            let e = *b.get(*i)?;
                            *i += 1;
                            match e {
                                b'n' => s.push('\n'),
                                b't' => s.push('\t'),
                                b'r' => s.push('\r'),
                                b'b' => s.push('\u{8}'),
                                b'f' => s.push('\u{c}'),
                                b'u' => {
                                    let hex = std::str::from_utf8(b.get(*i..*i + 4)?).ok()?;
                                    *i += 4;
                                    s.push(char::from_u32(u32::from_str_radix(hex, 16).ok()?)?);
                                }
                                other => s.push(other as char),
                            }
                        }
                        _ => {
                            // Copy a whole UTF-8 sequence.
                            let start = *i - 1;
                            let len = match c {
                                0..=0x7F => 1,
                                0xC0..=0xDF => 2,
                                0xE0..=0xEF => 3,
                                _ => 4,
                            };
                            *i = start + len;
                            s.push_str(std::str::from_utf8(b.get(start..*i)?).ok()?);
                        }
                    }
                }
            }
            b't' if b[*i..].starts_with(b"true") => {
                *i += 4;
                Some(Json::Bool(true))
            }
            b'f' if b[*i..].starts_with(b"false") => {
                *i += 5;
                Some(Json::Bool(false))
            }
            b'n' if b[*i..].starts_with(b"null") => {
                *i += 4;
                Some(Json::Null)
            }
            _ => {
                let start = *i;
                while *i < b.len() && matches!(b[*i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    *i += 1;
                }
                std::str::from_utf8(&b[start..*i]).ok()?.parse().ok().map(Json::Num)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_and_keys() {
        assert!(Json::parse("{\"a\":[1,2.5,true],\"b\":{}}").is_some());
        assert!(Json::parse("{bad}").is_none());
        let key = CountKey { layer: Some("0".into()), scale: Some([2.0; 3]), mirror_state: Some(false) };
        assert_eq!(
            block_json("CHAIR", &key),
            r#"{"key":{"layer":"0","mirrorState":false,"scale":{"x":2.0,"y":2.0,"z":2.0}},"name":"CHAIR","type":"block"}"#
        );
        assert_eq!(key.label(), "0_2.0000_Default");
        assert_eq!(parse_key(Json::parse(&key.json()).as_ref()), Some(key));
        assert_eq!(
            single_json(&[Handle::new(0x8D1A)]),
            r#"{"groupCondition":{"attributes":[],"parameters":[],"properties":[]},"targets":["8D1A"],"type":"single"}"#
        );
        assert!(point_in_polygon([1.0, 1.0], &[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]));
        assert!(!point_in_polygon([3.0, 1.0], &[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]));
    }
}
