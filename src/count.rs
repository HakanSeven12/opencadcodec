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
//!   the references of the block the target reference shows.
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
            let inside = insert_corners(doc, insert, 0)
                .is_some_and(|pts| pts.iter().all(|p| point_in_polygon([p.x, p.y], polygon)));
            if !inside {
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
/// each with its count, ordered by label.
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
    out.sort_by_key(|(k, _)| k.label().to_ascii_uppercase());
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
                        return Some(usize::from(doc.get_entity(*one).is_some()));
                    };
                    let instances = block_instances(doc, None);
                    Some(count_of(&instances, &insert.block_name, &CountKey::default()))
                }
                // ponytail: a multi-object target is one group; repeated
                // groups are not searched for.
                [] => Some(0),
                _ => Some(usize::from(targets.iter().all(|h| doc.get_entity(*h).is_some()))),
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

/// The WCS outline of a closed LWPOLYLINE used as a count area (bulges are
/// taken as straight segments).
pub fn boundary_polygon(doc: &CadDocument, handle: Handle) -> Option<Vec<[f64; 2]>> {
    match doc.get_entity(handle)? {
        EntityType::LwPolyline(pl) if pl.vertices.len() >= 3 => {
            Some(pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect())
        }
        _ => None,
    }
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

/// WCS corners of a reference's block extents (the box of the definition's
/// entities moved by the reference transform). `None` for an empty block.
pub fn insert_corners(doc: &CadDocument, insert: &Insert, depth: usize) -> Option<Vec<Vector3>> {
    let record = doc.block_records.get(&insert.block_name)?;
    let base = record.base_point;
    let mut lo = Vector3::new(f64::MAX, f64::MAX, f64::MAX);
    let mut hi = Vector3::new(f64::MIN, f64::MIN, f64::MIN);
    let mut grow = |p: Vector3| {
        lo = Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
        hi = Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
    };
    for &h in &record.entity_handles {
        match doc.get_entity(h) {
            Some(EntityType::Insert(nested)) if depth < 8 => {
                for p in insert_corners(doc, nested, depth + 1).unwrap_or_default() {
                    grow(p);
                }
            }
            Some(EntityType::AttributeDefinition(_)) | None => {}
            Some(e) => {
                let b = e.as_entity().bounding_box();
                grow(b.min);
                grow(b.max);
            }
        }
    }
    if lo.x > hi.x {
        return None;
    }
    let t = insert.get_transform();
    let mut out = Vec::with_capacity(8);
    for x in [lo.x, hi.x] {
        for y in [lo.y, hi.y] {
            for z in [lo.z, hi.z] {
                out.push(t.apply(Vector3::new(x - base.x, y - base.y, z - base.z)));
            }
        }
    }
    Some(out)
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
