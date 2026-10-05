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
/// model space and inside the counted block references (nested ones too):
/// a line finds every line and polyline, a circle every circle, a polyline
/// the polylines of the same shape at any size, rotation or position, and
/// anything else objects of its kind. Each match is listed as the top-level
/// object it is drawn by (a reference repeats once per match inside it).
pub fn similar_matches(doc: &CadDocument, target: &EntityType, area: Option<&[[f64; 2]]>) -> Vec<Vec<Handle>> {
    let duplicates: Vec<Handle> =
        block_instances(doc, None).into_iter().filter(|i| i.duplicate_of.is_some()).map(|i| i.handle).collect();
    let shape = polyline_shape(target);
    let matches = |e: &EntityType| match target {
        EntityType::Line(_) => matches!(e, EntityType::Line(_) | EntityType::LwPolyline(_)),
        EntityType::LwPolyline(_) => {
            matches!(e, EntityType::LwPolyline(_)) && same_shape(shape.as_deref(), polyline_shape(e).as_deref())
        }
        _ => std::mem::discriminant(e) == std::mem::discriminant(target),
    };
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
fn nested_matches<'a>(
    doc: &'a CadDocument,
    e: &'a EntityType,
    matches: &dyn Fn(&EntityType) -> bool,
    chain: &mut Vec<&'a Insert>,
    depth: usize,
    found: &mut Vec<Vec<Vec<Vector3>>>,
) {
    if let EntityType::Insert(ins) = e {
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
    if !matches(e) {
        return;
    }
    let mut outline = entity_outline(doc, e);
    for ins in chain.iter().rev() {
        let base = doc.block_records.get(&ins.block_name).map(|r| r.base_point).unwrap_or_default();
        let t = ins.get_transform();
        for part in &mut outline {
            for p in part.iter_mut() {
                *p = t.apply(*p - base);
            }
        }
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
fn same_shape(a: Option<&[[f64; 3]]>, b: Option<&[[f64; 3]]>) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return false;
    };
    let close = |x: &[f64; 3], y: &[f64; 3]| x.iter().zip(y).all(|(p, q)| (p - q).abs() < 1e-6);
    a.len() == b.len() && (0..a.len()).any(|k| a.iter().enumerate().all(|(i, x)| close(x, &b[(i + k) % b.len()])))
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

/// Whether an outline lies wholly inside the polygon: all its points inside
/// and none of its segments crossing an edge.
pub fn inside_polygon(outline: &[Vec<Vector3>], polygon: &[[f64; 2]]) -> bool {
    let n = polygon.len();
    outline.iter().any(|part| !part.is_empty())
        && outline.iter().all(|part| {
            part.iter().all(|p| point_in_polygon([p.x, p.y], polygon))
                && part.windows(2).all(|w| {
                    (0..n).all(|k| {
                        !segments_cross([w[0].x, w[0].y], [w[1].x, w[1].y], polygon[k], polygon[(k + 1) % n])
                    })
                })
        })
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

/// WCS corners of a reference's extents: the box of its geometry as it lies
/// in the drawing. `None` for an empty block.
pub fn insert_corners(doc: &CadDocument, insert: &Insert, depth: usize) -> Option<Vec<Vector3>> {
    let outline = insert_outline(doc, insert, depth);
    let mut pts = outline.iter().flatten();
    let first = *pts.next()?;
    let (lo, hi) = pts.fold((first, first), |(lo, hi), p| {
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

/// The geometry of a reference in WCS as polylines (curves sampled).
pub fn insert_outline(doc: &CadDocument, insert: &Insert, depth: usize) -> Vec<Vec<Vector3>> {
    let Some(record) = doc.block_records.get(&insert.block_name) else {
        return Vec::new();
    };
    let base = record.base_point;
    let t = insert.get_transform();
    let mut out = Vec::new();
    for &h in &record.entity_handles {
        let parts = match doc.get_entity(h) {
            Some(EntityType::Insert(nested)) if depth < 8 => insert_outline(doc, nested, depth + 1),
            Some(EntityType::Insert(_)) | Some(EntityType::AttributeDefinition(_)) | None => continue,
            Some(e) => entity_outline(doc, e),
        };
        out.extend(parts.into_iter().map(|part| part.into_iter().map(|p| t.apply(p - base)).collect::<Vec<_>>()));
    }
    out
}

/// The geometry of an entity in WCS as polylines: lines, circles, arcs and
/// lightweight polylines exactly (curves sampled every 7.5°), references
/// through their block, anything else as its bounding box.
// ponytail: sampled curves; a boundary corner poking between two samples is missed.
pub fn entity_outline(doc: &CadDocument, entity: &EntityType) -> Vec<Vec<Vector3>> {
    use crate::types::Matrix3;
    let step = 7.5f64.to_radians();
    let arc = |center: Vector3, r: f64, a0: f64, a1: f64, normal: Vector3| -> Vec<Vector3> {
        let sweep = (a1 - a0).rem_euclid(std::f64::consts::TAU);
        let sweep = if sweep < 1e-12 { std::f64::consts::TAU } else { sweep };
        let n = ((sweep / step).ceil() as usize).max(2);
        let m = Matrix3::arbitrary_axis(normal);
        (0..=n)
            .map(|k| {
                let a = a0 + sweep * k as f64 / n as f64;
                m * (center + Vector3::new(r * a.cos(), r * a.sin(), 0.0))
            })
            .collect()
    };
    match entity {
        EntityType::Line(l) => vec![vec![l.start, l.end]],
        EntityType::Circle(c) => vec![arc(c.center, c.radius, 0.0, std::f64::consts::TAU, c.normal)],
        EntityType::Arc(a) => vec![arc(a.center, a.radius, a.start_angle, a.end_angle, a.normal)],
        EntityType::LwPolyline(pl) if !pl.vertices.is_empty() => {
            let m = Matrix3::arbitrary_axis(pl.normal);
            let n = pl.vertices.len();
            let segs = if pl.is_closed { n } else { n - 1 };
            let at = |x: f64, y: f64| Vector3::new(x, y, pl.elevation);
            let mut pts = vec![at(pl.vertices[0].location.x, pl.vertices[0].location.y)];
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
                    let k = ((sweep.abs() / step).ceil() as usize).max(2);
                    for j in 1..k {
                        let a = a0 + sweep * j as f64 / k as f64;
                        pts.push(at(cx + r * a.cos(), cy + r * a.sin()));
                    }
                }
                pts.push(at(p1.x, p1.y));
            }
            vec![pts.into_iter().map(|p| m * p).collect()]
        }
        EntityType::Insert(ins) => insert_outline(doc, ins, 0),
        e => {
            let b = e.as_entity().bounding_box();
            let z = b.min.z;
            vec![vec![
                Vector3::new(b.min.x, b.min.y, z),
                Vector3::new(b.max.x, b.min.y, z),
                Vector3::new(b.max.x, b.max.y, z),
                Vector3::new(b.min.x, b.max.y, z),
                Vector3::new(b.min.x, b.min.y, z),
            ]]
        }
    }
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
