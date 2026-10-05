//! Sheet sets: the sheet set data file (`.dst`), a drawing's link to the
//! sheet it holds (`AcSheetSetData` in the named-object dictionary) and the
//! `\AcSm` field evaluator.
//!
//! A `.dst` file is UTF-8 XML put through a fixed one-to-one byte
//! substitution ([`encode`] / [`decode`]). The XML is a tree of components:
//! `AcSmDatabase` → `AcSmSheetSet` → `AcSmSubset`* → `AcSmSheet`, each with
//! `AcSmProp` values and object-valued properties (custom property bags,
//! layout and file references, …). Elements are kept as a generic tree so
//! everything the library does not model survives a read → write round trip.
//! Named children (those with a `propname`) stay sorted by name ahead of the
//! unnamed ones (subsets and sheets), as the reference writes them.

use crate::document::CadDocument;
use crate::fields::FieldContext;
use crate::objects::{Dictionary, ObjectType, XRecord, XRecordValue};

// ── cipher ───────────────────────────────────────────────────────────────────

const fn encode_byte(p: u8) -> u8 {
    let q = p.wrapping_add(1);
    let low = (q & 0x0F) ^ 0x0D;
    let h = q >> 4;
    let hc = if h & 1 == 1 { (h + 10) & 0x0F } else { (h + 8) & 0x0F };
    (hc << 4) | low
}

const ENCODE: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = encode_byte(i as u8);
        i += 1;
    }
    t
};

const DECODE: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        t[ENCODE[i] as usize] = i as u8;
        i += 1;
    }
    t
};

/// Plain XML bytes → `.dst` bytes.
pub fn encode(plain: &[u8]) -> Vec<u8> {
    plain.iter().map(|b| ENCODE[*b as usize]).collect()
}

/// `.dst` bytes → plain XML bytes.
pub fn decode(data: &[u8]) -> Vec<u8> {
    data.iter().map(|b| DECODE[*b as usize]).collect()
}

// ── XML tree ─────────────────────────────────────────────────────────────────

/// An XML element: attributes in file order, its text (leaf values) and its
/// child elements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Element>,
}

impl Element {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The component id (`ID` attribute).
    pub fn id(&self) -> &str {
        self.attr("ID").unwrap_or("")
    }

    pub fn propname(&self) -> Option<&str> {
        self.attr("propname")
    }

    /// The child holding property `name` (an `AcSmProp` or an object).
    pub fn named(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.propname() == Some(name))
    }

    pub fn named_mut(&mut self, name: &str) -> Option<&mut Element> {
        self.children.iter_mut().find(|c| c.propname() == Some(name))
    }

    /// A value property (`<AcSmProp propname="name">value</AcSmProp>`).
    pub fn prop(&self, name: &str) -> Option<&str> {
        self.named(name).filter(|c| c.name == "AcSmProp").map(|c| c.text.as_str())
    }

    /// Set a string property (vt 8), keeping the named children sorted.
    pub fn set_prop(&mut self, name: &str, value: &str) {
        self.set_prop_vt(name, 8, value);
    }

    /// Set a property of the given variant type (8 string, 3 int, 2 short).
    pub fn set_prop_vt(&mut self, name: &str, vt: i32, value: &str) {
        if let Some(c) = self.named_mut(name).filter(|c| c.name == "AcSmProp") {
            c.text = value.into();
            return;
        }
        self.put_named(prop(name, vt, value));
    }

    /// Insert or replace the named child `child`, keeping named children in
    /// name order ahead of the unnamed ones.
    pub fn put_named(&mut self, child: Element) {
        let name = child.propname().unwrap_or("").to_string();
        if let Some(i) = self.children.iter().position(|c| c.propname() == Some(&name)) {
            self.children[i] = child;
            return;
        }
        let key = name.to_ascii_lowercase();
        let at = self
            .children
            .iter()
            .position(|c| match c.propname() {
                Some(n) => n.to_ascii_lowercase() > key,
                None => true,
            })
            .unwrap_or(self.children.len());
        self.children.insert(at, child);
    }

    pub fn remove_named(&mut self, name: &str) {
        self.children.retain(|c| c.propname() != Some(name));
    }

    fn find(&self, id: &str) -> Option<&Element> {
        if self.id() == id {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(id))
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut Element> {
        if self.id() == id {
            return Some(self);
        }
        self.children.iter_mut().find_map(|c| c.find_mut(id))
    }

    fn parent_of(&self, id: &str) -> Option<&Element> {
        if self.children.iter().any(|c| c.id() == id) {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.parent_of(id))
    }

    fn write(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.name);
        for (k, v) in &self.attrs {
            out.push(' ');
            out.push_str(k);
            out.push_str("=\"");
            escape_into(v, true, out);
            out.push('"');
        }
        if self.text.is_empty() && self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        escape_into(&self.text, false, out);
        for c in &self.children {
            c.write(out);
        }
        out.push_str("</");
        out.push_str(&self.name);
        out.push('>');
    }
}

fn escape_into(s: &str, attr: bool, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let after = &rest[p..];
        let Some(end) = after.find(';') else {
            out.push_str(after);
            return out;
        };
        let entity = &after[1..end];
        let ch = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            e if e.starts_with("#x") || e.starts_with("#X") => {
                u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32)
            }
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => out.push(c),
            None => out.push_str(&after[..=end]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.s[self.i..].starts_with(|c: char| c.is_whitespace()) {
            self.i += 1;
        }
    }

    /// Skip the declaration, processing instructions and comments.
    fn skip_misc(&mut self) {
        loop {
            self.skip_ws();
            let rest = &self.s[self.i..];
            let close = if rest.starts_with("<?") {
                "?>"
            } else if rest.starts_with("<!--") {
                "-->"
            } else if rest.starts_with("<!") {
                ">"
            } else {
                return;
            };
            match rest.find(close) {
                Some(p) => self.i += p + close.len(),
                None => {
                    self.i = self.s.len();
                    return;
                }
            }
        }
    }

    fn element(&mut self) -> Result<Element, String> {
        self.skip_misc();
        if !self.s[self.i..].starts_with('<') {
            return Err(format!("element expected at byte {}", self.i));
        }
        self.i += 1;
        let name_end = self.s[self.i..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .ok_or("unterminated tag")?;
        let mut el = Element::new(&self.s[self.i..self.i + name_end]);
        self.i += name_end;
        loop {
            self.skip_ws();
            let rest = &self.s[self.i..];
            if rest.starts_with("/>") {
                self.i += 2;
                return Ok(el);
            }
            if rest.starts_with('>') {
                self.i += 1;
                break;
            }
            let eq = rest.find('=').ok_or("attribute without value")?;
            let key = rest[..eq].trim().to_string();
            let after = rest[eq + 1..].trim_start();
            let quote = after.chars().next().ok_or("attribute value expected")?;
            if quote != '"' && quote != '\'' {
                return Err("quoted attribute value expected".into());
            }
            let value_end = after[1..].find(quote).ok_or("unterminated attribute")?;
            el.attrs.push((key, unescape(&after[1..1 + value_end])));
            self.i = self.s.len() - after.len() + value_end + 2;
        }
        loop {
            let rest = &self.s[self.i..];
            let lt = rest.find('<').ok_or("unterminated element")?;
            el.text.push_str(&unescape(&rest[..lt]));
            self.i += lt;
            let rest = &self.s[self.i..];
            if rest.starts_with("</") {
                let gt = rest.find('>').ok_or("unterminated end tag")?;
                self.i += gt + 1;
                // Indentation between child elements is not a value.
                if !el.children.is_empty() && el.text.trim().is_empty() {
                    el.text.clear();
                }
                return Ok(el);
            }
            if rest.starts_with("<!--") {
                self.skip_misc();
                continue;
            }
            el.children.push(self.element()?);
        }
    }
}

/// Parse an XML document's root element.
pub fn parse_xml(text: &str) -> Result<Element, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    Parser { s: text, i: 0 }.element()
}

/// Serialize as the reference does: declaration, CRLF, the elements back to
/// back, CRLF.
pub fn write_xml(root: &Element) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n");
    root.write(&mut out);
    out.push_str("\r\n");
    out
}

// ── component classes ────────────────────────────────────────────────────────

pub const CLSID_DATABASE: &str = "g2162C6B6-0CE4-40E8-912B-46F59DFDF826";
pub const CLSID_SHEET_SET: &str = "gB20534F2-0978-418C-8D14-2E6928A077ED";
pub const CLSID_SUBSET: &str = "g076D548F-B0F5-4FE1-B35D-7F7B73B8D322";
pub const CLSID_SHEET: &str = "g16A07941-BC15-4D48-A880-9D5A211D5065";
pub const CLSID_CALLOUT_BLOCKS: &str = "g203EAB46-483B-4E6B-A10B-15E9A4B210FF";
pub const CLSID_CUSTOM_PROPERTY_BAG: &str = "g4D103908-8C86-4D95-BBF4-68B9A7B00731";
pub const CLSID_CUSTOM_PROPERTY_VALUE: &str = "g8D22A2A4-1777-4D78-84CC-69EF741FE954";
pub const CLSID_PROJECT_POINT_LOCATIONS: &str = "gE40EA246-BAB4-4907-81A5-511EA30C16FD";
pub const CLSID_PUBLISH_OPTIONS: &str = "gF57F96E7-0F16-4DC9-8F09-52F7BB389AB6";
pub const CLSID_RESOURCES: &str = "g3F0FAF10-09DE-4EBA-AED1-C4E4D6FECF5D";
pub const CLSID_SHEET_SEL_SETS: &str = "g444780B8-6527-43A8-8DC4-FAB41B7E48BB";
pub const CLSID_VIEW_CATEGORIES: &str = "g021730DF-5BEA-48E9-BC7A-35087A674FD0";
pub const CLSID_LAYOUT_REFERENCE: &str = "g94910E94-4FCA-427C-B6ED-2EC9E1C900C7";
pub const CLSID_SHEET_VIEWS: &str = "gF40F931B-64BC-4B90-9FC8-A11A77D6815B";
pub const CLSID_FILE_REFERENCE: &str = "g6BF87AE7-1BEC-4BDB-98BB-5B91F7772793";

/// Custom property flags: owned by the sheet set, or a sheet property whose
/// set-level value is the default for new sheets.
pub const CUSTOM_SHEET_SET_PROP: i32 = 1;
pub const CUSTOM_SHEET_PROP: i32 = 2;

/// A new component id: `g` + an upper-case GUID without braces.
pub fn new_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let word = |salt: u64| {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(n);
        h.write_u64(salt);
        h.write_u128(web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0));
        h.finish()
    };
    let (a, b) = (word(0x5eed), word(0xfeed));
    // Version 4, RFC 4122 variant.
    let b = (b & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
    let a = (a & 0xFFFF_FFFF_FFFF_0FFF) | 0x0000_0000_0000_4000;
    format!(
        "g{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
        a >> 32,
        (a >> 16) & 0xFFFF,
        a & 0xFFFF,
        b >> 48,
        b & 0xFFFF_FFFF_FFFF
    )
}

/// `<AcSmProp propname="…" vt="…">value</AcSmProp>`.
pub fn prop(name: &str, vt: i32, value: &str) -> Element {
    Element {
        name: "AcSmProp".into(),
        attrs: vec![("propname".into(), name.into()), ("vt".into(), vt.to_string())],
        text: value.into(),
        children: Vec::new(),
    }
}

/// A component object with a fresh id; `propname` makes it an object-valued
/// property (vt 13) of its parent.
pub fn object(tag: &str, clsid: &str, propname: Option<&str>) -> Element {
    let mut el = Element::new(tag);
    el.attrs.push(("clsid".into(), clsid.into()));
    el.attrs.push(("ID".into(), new_id()));
    if let Some(p) = propname {
        el.attrs.push(("propname".into(), p.into()));
        el.attrs.push(("vt".into(), "13".into()));
    }
    el
}

/// What a tree component is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    SheetSet,
    Subset,
    Sheet,
}

impl ComponentKind {
    pub fn of(el: &Element) -> Option<Self> {
        match el.name.as_str() {
            "AcSmSheetSet" => Some(Self::SheetSet),
            "AcSmSubset" => Some(Self::Subset),
            "AcSmSheet" => Some(Self::Sheet),
            _ => None,
        }
    }
}

/// A layout reference (`AcSmAcDbLayoutReference`): a layout in a drawing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutReference {
    /// The drawing (absolute path as stored, or resolved from the relative one).
    pub file_name: String,
    /// The layout's name.
    pub name: String,
    /// The layout object's handle, upper-case hex (empty when unknown).
    pub handle: String,
}

/// The `.dst` sheet set database.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetSetDatabase {
    pub root: Element,
    /// Where the database was read from / last written to.
    pub path: Option<String>,
}

impl SheetSetDatabase {
    /// A new, empty sheet set named `name`.
    pub fn new(name: &str, description: &str) -> Self {
        let mut root = object("AcSmDatabase", CLSID_DATABASE, None);
        root.children.push(prop("DbFingerPrint", 8, &new_id()));
        root.children.push(prop("DbVersion", 8, "1.1"));
        root.children.push(prop("FileRevision", 3, "0"));
        let mut set = object("AcSmSheetSet", CLSID_SHEET_SET, Some("SheetSet"));
        for (tag, clsid, name) in [
            ("AcSmCalloutBlocks", CLSID_CALLOUT_BLOCKS, "CalloutBlocks"),
            ("AcSmCustomPropertyBag", CLSID_CUSTOM_PROPERTY_BAG, "CustomPropertyBag"),
            ("AcSmProjectPointLocations", CLSID_PROJECT_POINT_LOCATIONS, "ProjectPointLocations"),
            ("AcSmResources", CLSID_RESOURCES, "Resources"),
            ("AcSmSheetSelSets", CLSID_SHEET_SEL_SETS, "SheetSelSets"),
            ("AcSmViewCategories", CLSID_VIEW_CATEGORIES, "ViewCategories"),
        ] {
            set.put_named(object(tag, clsid, Some(name)));
        }
        let mut publish = object("AcSmPublishOptions", CLSID_PUBLISH_OPTIONS, Some("PublishOptions"));
        publish.set_prop_vt("DwfType", 2, "-1");
        publish.set_prop_vt("PromptForName", 2, "-1");
        set.put_named(publish);
        set.set_prop("Desc", description);
        set.set_prop("Name", name);
        root.children.push(set);
        Self { root, path: None }
    }

    /// Parse decoded XML.
    pub fn from_xml(text: &str) -> Result<Self, String> {
        let root = parse_xml(text)?;
        if root.name != "AcSmDatabase" {
            return Err(format!("not a sheet set database ({})", root.name));
        }
        Ok(Self { root, path: None })
    }

    /// Parse a `.dst` file's bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let plain = decode(data);
        let text = String::from_utf8(plain).map_err(|e| e.to_string())?;
        Self::from_xml(&text)
    }

    pub fn to_xml(&self) -> String {
        write_xml(&self.root)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        encode(self.to_xml().as_bytes())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn read(path: &str) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut db = Self::from_bytes(&data)?;
        db.path = Some(path.to_string());
        Ok(db)
    }

    /// Write to `path`, counting a new file revision.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write(&mut self, path: &str) -> Result<(), String> {
        let revision = self.file_revision() + 1;
        self.root.set_prop_vt("FileRevision", 3, &revision.to_string());
        std::fs::write(path, self.to_bytes()).map_err(|e| e.to_string())?;
        self.path = Some(path.to_string());
        Ok(())
    }

    /// `FileRevision` — counted up on every save; drawings record it as
    /// `ShSetVersion`.
    pub fn file_revision(&self) -> i32 {
        self.root.prop("FileRevision").and_then(|v| v.trim().parse().ok()).unwrap_or(0)
    }

    pub fn sheet_set(&self) -> &Element {
        self.root
            .children
            .iter()
            .find(|c| c.name == "AcSmSheetSet")
            .unwrap_or(&self.root)
    }

    pub fn sheet_set_mut(&mut self) -> &mut Element {
        let i = self.root.children.iter().position(|c| c.name == "AcSmSheetSet");
        match i {
            Some(i) => &mut self.root.children[i],
            None => &mut self.root,
        }
    }

    pub fn name(&self) -> &str {
        self.sheet_set().prop("Name").unwrap_or("")
    }

    pub fn find(&self, id: &str) -> Option<&Element> {
        self.root.find(id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Element> {
        self.root.find_mut(id)
    }

    /// The subset or sheet set that holds component `id`.
    pub fn parent_of(&self, id: &str) -> Option<&Element> {
        self.root.parent_of(id)
    }

    /// The folder the `.dst` lives in.
    fn folder(&self) -> Option<std::path::PathBuf> {
        let path = std::path::Path::new(self.path.as_deref()?);
        Some(path.parent()?.to_path_buf())
    }

    /// Add a subset under `parent` (the sheet set or a subset); returns its id.
    /// It inherits the parent's sheet storage location and template.
    pub fn add_subset(&mut self, parent: &str, name: &str, description: &str) -> Option<String> {
        let inherited: Vec<Element> = {
            let p = self.find(parent)?;
            ["DefDwtLayout", "NewSheetLocation"]
                .iter()
                .filter_map(|n| p.named(n).cloned())
                .map(|mut e| {
                    renew_ids(&mut e);
                    e
                })
                .collect()
        };
        let mut sub = object("AcSmSubset", CLSID_SUBSET, None);
        for e in inherited {
            sub.put_named(e);
        }
        sub.set_prop("Desc", description);
        sub.set_prop("Name", name);
        let id = sub.id().to_string();
        self.find_mut(parent)?.children.push(sub);
        Some(id)
    }

    /// Add a sheet under `parent`; returns its id.
    pub fn add_sheet(&mut self, parent: &str, number: &str, title: &str, description: &str) -> Option<String> {
        let mut sheet = object("AcSmSheet", CLSID_SHEET, None);
        sheet.put_named(object("AcSmCustomPropertyBag", CLSID_CUSTOM_PROPERTY_BAG, Some("CustomPropertyBag")));
        sheet.set_prop("Desc", description);
        sheet.set_prop("Number", number);
        sheet.put_named(object("AcSmSheetViews", CLSID_SHEET_VIEWS, Some("SheetViews")));
        sheet.set_prop("Title", title);
        // New sheets get the set's sheet properties with their defaults.
        for (name, value, flags) in custom_properties(self.sheet_set()) {
            if flags & CUSTOM_SHEET_PROP != 0 {
                set_custom_property(&mut sheet, &name, &value, flags);
            }
        }
        let id = sheet.id().to_string();
        let p = self.find_mut(parent)?;
        p.children.push(sheet);
        Some(id)
    }

    /// Remove a subset (with everything in it) or a sheet.
    pub fn remove(&mut self, id: &str) -> bool {
        fn go(el: &mut Element, id: &str) -> bool {
            if let Some(i) = el.children.iter().position(|c| c.id() == id) {
                el.children.remove(i);
                return true;
            }
            el.children.iter_mut().any(|c| go(c, id))
        }
        go(&mut self.root, id)
    }

    /// Point `component`'s file reference property (`NewSheetLocation`,
    /// `AltPageSetups`, …) at `file`.
    pub fn set_file_reference(&mut self, component: &str, propname: &str, file: &str) -> bool {
        let folder = self.folder();
        let Some(el) = self.find_mut(component) else {
            return false;
        };
        if file.is_empty() {
            el.remove_named(propname);
            return true;
        }
        let mut r = object("AcSmFileReference", CLSID_FILE_REFERENCE, Some(propname));
        set_file_props(&mut r, file, folder.as_deref());
        el.put_named(r);
        true
    }

    /// A file reference property resolved against the `.dst` folder.
    pub fn file_reference(&self, component: &str, propname: &str) -> Option<String> {
        let r = self.find(component)?.named(propname)?;
        Some(self.resolve_file(r))
    }

    /// Point a sheet's `Layout` (or a set's / subset's `DefDwtLayout`) at
    /// `layout` of `file`.
    pub fn set_layout_reference(&mut self, component: &str, propname: &str, reference: &LayoutReference) -> bool {
        let folder = self.folder();
        let Some(el) = self.find_mut(component) else {
            return false;
        };
        let mut r = match el.named(propname) {
            Some(old) if old.name == "AcSmAcDbLayoutReference" => {
                let mut r = old.clone();
                r.children.clear();
                r
            }
            _ => object("AcSmAcDbLayoutReference", CLSID_LAYOUT_REFERENCE, Some(propname)),
        };
        if !reference.handle.is_empty() {
            r.set_prop("AcDbHandle", &reference.handle);
        }
        set_file_props(&mut r, &reference.file_name, folder.as_deref());
        r.set_prop("Name", &reference.name);
        el.put_named(r);
        true
    }

    /// A component's layout reference property (`Layout` of a sheet,
    /// `DefDwtLayout` of a set / subset).
    pub fn layout_reference(&self, component: &str, propname: &str) -> Option<LayoutReference> {
        let r = self.find(component)?.named(propname)?;
        Some(LayoutReference {
            file_name: self.resolve_file(r),
            name: r.prop("Name").unwrap_or("").to_string(),
            handle: r.prop("AcDbHandle").unwrap_or("").to_string(),
        })
    }

    /// The file a reference names: the stored absolute path when it exists,
    /// else the relative path resolved against the `.dst` folder, else the
    /// stored path as it is.
    pub fn resolve_file(&self, reference: &Element) -> String {
        let absolute = reference.prop("FileName").unwrap_or("").to_string();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !absolute.is_empty() && std::path::Path::new(&absolute).exists() {
                return absolute;
            }
            if let (Some(rel), Some(folder)) = (reference.prop("Relative_FileName"), self.folder()) {
                let joined = folder.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                if joined.exists() || absolute.is_empty() {
                    return normalize_path(&joined);
                }
            }
        }
        absolute
    }

    /// Every sheet in tree order with the subset path above it.
    pub fn sheets(&self) -> Vec<&Element> {
        fn go<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
            for c in &el.children {
                match ComponentKind::of(c) {
                    Some(ComponentKind::Sheet) => out.push(c),
                    Some(ComponentKind::Subset) => go(c, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        go(self.sheet_set(), &mut out);
        out
    }

    /// The sheet for layout `layout` of drawing `drawing` (any of the
    /// drawing's sheets when `layout` is `None` or matches none).
    pub fn sheet_for(&self, drawing: &str, layout: Option<&str>) -> Option<&Element> {
        let want = path_key(drawing);
        let of_drawing: Vec<&Element> = self
            .sheets()
            .into_iter()
            .filter(|s| {
                s.named("Layout").is_some_and(|r| {
                    path_key(&self.resolve_file(r)) == want
                        || r.prop("FileName").is_some_and(|f| path_key(f) == want)
                })
            })
            .collect();
        layout
            .and_then(|name| {
                of_drawing.iter().copied().find(|s| {
                    s.named("Layout")
                        .and_then(|r| r.prop("Name"))
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
                })
            })
            .or_else(|| of_drawing.first().copied())
    }

    /// The value a `\AcSm` field shows for `component.property` of `sheet`.
    pub fn sheet_value(&self, sheet: &Element, component: &str, property: &str) -> Option<String> {
        let set = self.sheet_set();
        let custom = |el: &Element, name: &str| {
            custom_properties(el).into_iter().find(|(n, _, _)| n == name).map(|(_, v, _)| v)
        };
        match component {
            "Sheet" => Some(match property {
                "Number" | "Title" | "RevisionNumber" | "RevisionDate" | "IssuePurpose" | "Category" => {
                    sheet.prop(property).unwrap_or("").to_string()
                }
                "Description" => sheet.prop("Desc").unwrap_or("").to_string(),
                "NumberAndTitle" => number_and_title(sheet),
                // A sheet property the sheet lacks shows the set's default.
                other => custom(sheet, other).or_else(|| {
                    custom_properties(set)
                        .into_iter()
                        .find(|(n, _, f)| n == other && f & CUSTOM_SHEET_PROP != 0)
                        .map(|(_, v, _)| v)
                })?,
            }),
            "SheetSet" => Some(match property {
                "Name" | "ProjectName" | "ProjectNumber" | "ProjectPhase" | "ProjectMilestone" => {
                    set.prop(property).unwrap_or("").to_string()
                }
                "Description" => set.prop("Desc").unwrap_or("").to_string(),
                other => custom(set, other)?,
            }),
            "Subset" => {
                let parent = self.parent_of(sheet.id())?;
                if ComponentKind::of(parent) != Some(ComponentKind::Subset) {
                    return None;
                }
                Some(match property {
                    "Description" => parent.prop("Desc").unwrap_or("").to_string(),
                    other => parent.prop(other).unwrap_or("").to_string(),
                })
            }
            _ => None,
        }
    }
}

/// How the sheet list shows a sheet: `Number - Title` (the title alone
/// without a number).
pub fn number_and_title(sheet: &Element) -> String {
    let number = sheet.prop("Number").unwrap_or("");
    let title = sheet.prop("Title").unwrap_or("");
    if number.is_empty() {
        title.to_string()
    } else {
        format!("{number} - {title}")
    }
}

fn renew_ids(el: &mut Element) {
    if let Some(a) = el.attrs.iter_mut().find(|(k, _)| k == "ID") {
        a.1 = new_id();
    }
    for c in &mut el.children {
        renew_ids(c);
    }
}

/// The custom properties of a set or sheet: name, value and flags.
pub fn custom_properties(el: &Element) -> Vec<(String, String, i32)> {
    el.named("CustomPropertyBag")
        .map(|bag| {
            bag.children
                .iter()
                .filter(|c| c.name == "AcSmCustomPropertyValue")
                .map(|c| {
                    (
                        c.propname().unwrap_or("").to_string(),
                        c.prop("Value").unwrap_or("").to_string(),
                        c.prop("Flags").and_then(|f| f.trim().parse().ok()).unwrap_or(0),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Add or change a custom property of a set or sheet.
pub fn set_custom_property(el: &mut Element, name: &str, value: &str, flags: i32) {
    if el.named("CustomPropertyBag").is_none() {
        el.put_named(object("AcSmCustomPropertyBag", CLSID_CUSTOM_PROPERTY_BAG, Some("CustomPropertyBag")));
    }
    let bag = el.named_mut("CustomPropertyBag").expect("bag");
    if let Some(v) = bag.named_mut(name).filter(|c| c.name == "AcSmCustomPropertyValue") {
        v.set_prop_vt("Flags", 3, &flags.to_string());
        v.set_prop("Value", value);
        return;
    }
    let mut v = object("AcSmCustomPropertyValue", CLSID_CUSTOM_PROPERTY_VALUE, Some(name));
    v.set_prop_vt("Flags", 3, &flags.to_string());
    v.set_prop("Value", value);
    // The bag keeps its values in insertion order.
    bag.children.push(v);
}

pub fn remove_custom_property(el: &mut Element, name: &str) {
    if let Some(bag) = el.named_mut("CustomPropertyBag") {
        bag.children.retain(|c| c.propname() != Some(name));
    }
}

fn set_file_props(r: &mut Element, file: &str, folder: Option<&std::path::Path>) {
    for p in ["Environ_FileName", "FileName", "Relative_FileName", "SpecialFolder_FileName"] {
        r.remove_named(p);
    }
    let file = &native_path(file);
    r.set_prop("FileName", file);
    if let Some(rel) = folder.and_then(|f| relative_path(f, std::path::Path::new(file))) {
        r.set_prop("Relative_FileName", &rel);
    }
}

/// `file` relative to `folder` in the reference's form (`.\a.dwg`,
/// `..\x\a.dwg`, `.` for the folder itself); `None` across drives.
pub fn relative_path(folder: &std::path::Path, file: &std::path::Path) -> Option<String> {
    use std::path::Component;
    let parts = |p: &std::path::Path| -> Vec<String> {
        p.components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    };
    let (a, b) = (parts(folder), parts(file));
    if a.is_empty() || b.is_empty() || !a[0].eq_ignore_ascii_case(&b[0]) {
        return None;
    }
    let common = a.iter().zip(&b).take_while(|(x, y)| x.eq_ignore_ascii_case(y)).count();
    let mut out: Vec<String> = Vec::new();
    if common == a.len() {
        out.push(".".into());
    }
    out.extend(std::iter::repeat("..".to_string()).take(a.len() - common));
    out.extend(b[common..].iter().cloned());
    Some(out.join("\\"))
}

fn normalize_path(p: &std::path::Path) -> String {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().to_string()
}

/// A path as the reference stores it: Windows separators on Windows.
pub fn native_path(p: &str) -> String {
    if cfg!(windows) {
        p.replace('/', "\\")
    } else {
        p.to_string()
    }
}

/// Case- and separator-insensitive key for comparing drawing paths.
pub fn path_key(p: &str) -> String {
    p.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

// ── the drawing's sheet link (AcSheetSetData) ────────────────────────────────

/// The NOD dictionary a drawing saved as a sheet keeps.
pub const SHEET_SET_DATA: &str = "AcSheetSetData";

/// What a sheet drawing records about its sheet set (each value an XRECORD
/// in the `AcSheetSetData` dictionary).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SheetSetData {
    /// Layout object handle, lower-case hex (`6f`).
    pub layout_handle: String,
    pub layout_name: String,
    /// The drawing's own full path.
    pub sheet_dwg_name: String,
    /// The `.dst` full path.
    pub sheet_set_file_name: String,
    /// The `.dst` file revision when the drawing was saved.
    pub sheet_set_version: i32,
    /// Saves of the drawing as a sheet.
    pub update_count: i32,
    /// UTC time of the last such save, `yyyy/MM/dd HH:mm:ss.fff`.
    pub update_time: String,
}

impl CadDocument {
    /// The drawing's sheet set link, when it was saved as a sheet.
    pub fn sheet_set_data(&self) -> Option<SheetSetData> {
        let ObjectType::Dictionary(nod) = self.objects.get(&self.header.named_objects_dict_handle)? else {
            return None;
        };
        let ObjectType::Dictionary(dict) = self.objects.get(&nod.get(SHEET_SET_DATA)?)? else {
            return None;
        };
        let value = |key: &str| -> Option<&XRecordValue> {
            let ObjectType::XRecord(x) = self.objects.get(&dict.get(key)?)? else {
                return None;
            };
            x.entries.iter().find(|e| e.code != 280).map(|e| &e.value)
        };
        let text = |key: &str| value(key).and_then(|v| v.as_string()).unwrap_or("").to_string();
        let int = |key: &str| value(key).and_then(|v| v.as_i32()).unwrap_or(0);
        Some(SheetSetData {
            layout_handle: text("LayoutHandle"),
            layout_name: text("LayoutName"),
            sheet_dwg_name: text("SheetDwgName"),
            sheet_set_file_name: text("ShSetFileName"),
            sheet_set_version: int("ShSetVersion"),
            update_count: int("UpdateCount"),
            update_time: text("UpdateTime"),
        })
    }

    /// Write (or replace) the drawing's sheet set link.
    pub fn set_sheet_set_data(&mut self, data: &SheetSetData) {
        let nod = self.header.named_objects_dict_handle;
        let existing = match self.objects.get(&nod) {
            Some(ObjectType::Dictionary(d)) => d.get(SHEET_SET_DATA),
            _ => None,
        };
        let dict_handle = match existing {
            Some(h) if matches!(self.objects.get(&h), Some(ObjectType::Dictionary(_))) => h,
            _ => {
                let h = self.allocate_handle();
                let mut d = Dictionary::new();
                d.handle = h;
                d.owner = nod;
                d.hard_owner = true;
                d.duplicate_cloning = 1;
                d.reactors = vec![nod];
                self.objects.insert(h, ObjectType::Dictionary(d));
                if let Some(ObjectType::Dictionary(n)) = self.objects.get_mut(&nod) {
                    n.add_entry(SHEET_SET_DATA, h);
                }
                h
            }
        };
        let records: [(&str, XRecordValue, i32); 7] = [
            ("LayoutHandle", XRecordValue::String(data.layout_handle.clone()), 1),
            ("LayoutName", XRecordValue::String(data.layout_name.clone()), 1),
            ("SheetDwgName", XRecordValue::String(data.sheet_dwg_name.clone()), 1),
            ("ShSetFileName", XRecordValue::String(data.sheet_set_file_name.clone()), 1),
            ("ShSetVersion", XRecordValue::Int32(data.sheet_set_version), 90),
            ("UpdateCount", XRecordValue::Int32(data.update_count), 90),
            ("UpdateTime", XRecordValue::String(data.update_time.clone()), 1),
        ];
        for (key, value, code) in records {
            let old = match self.objects.get(&dict_handle) {
                Some(ObjectType::Dictionary(d)) => d.get(key),
                _ => None,
            };
            let handle = old
                .filter(|h| matches!(self.objects.get(h), Some(ObjectType::XRecord(_))))
                .unwrap_or_else(|| self.allocate_handle());
            let mut x = XRecord::new();
            x.handle = handle;
            x.owner = dict_handle;
            x.reactors = vec![dict_handle];
            x.cloning_flags = crate::objects::DictionaryCloningFlags::KeepExisting;
            x.entries.push(crate::objects::XRecordEntry::new(code, value));
            self.objects.insert(handle, ObjectType::XRecord(x));
            if old != Some(handle) {
                if let Some(ObjectType::Dictionary(d)) = self.objects.get_mut(&dict_handle) {
                    d.entries.retain(|(k, _)| k != key);
                    d.add_entry(key, handle);
                }
            }
        }
    }
}

// ── \AcSm fields ─────────────────────────────────────────────────────────────

/// The temporary value a sheet set placeholder field shows: its type name.
pub fn placeholder_type_name(component: &str, property: &str) -> String {
    match (component, property) {
        ("Sheet", "NumberAndTitle" | "Title" | "Number" | "Description") => format!("Sheet{property}"),
        ("Sheet", "RevisionNumber" | "RevisionDate" | "IssuePurpose" | "Category") => property.to_string(),
        ("Sheet", _) => "Custom".into(),
        ("View", "ViewportScale") => "ViewportScale".into(),
        ("View", p) => format!("View{p}"),
        _ => property.to_string(),
    }
}

/// Split a `\AcSm` field code into its component, property and format:
/// `\AcSm.16.2 ?Sheet.Drawn By \f "%tc4"` → (`?Sheet`, `Drawn By`, `%tc4`).
/// `None` for the bare `\AcSm` code.
pub fn parse_code(code: &str) -> Option<(String, String, String)> {
    let body = code.trim().trim_start_matches('\\');
    let body = body.split_once(char::is_whitespace)?.1.trim();
    let (target, fmt) = match body.find("\\f ") {
        Some(p) => {
            let f = body[p + 3..].trim();
            let f = f.strip_prefix('"').unwrap_or(f);
            let f = f.strip_suffix('"').unwrap_or(f);
            (body[..p].trim(), f.replace("\\\"", "\""))
        }
        None => (body, String::new()),
    };
    let (component, property) = target.split_once('.')?;
    Some((component.to_string(), property.to_string(), fmt))
}

/// Evaluate a `\AcSm` field. A drawing that is no sheet of an open sheet set
/// shows `####`; a placeholder shows its type name.
pub(crate) fn eval_acsm(
    doc: &CadDocument,
    code: &str,
    ctx: &dyn FieldContext,
    layout: Option<&str>,
    text_case: impl Fn(String, &str) -> String,
) -> Option<String> {
    const NOT_A_SHEET: &str = "####";
    let Some((component, property, fmt)) = parse_code(code) else {
        return Some(NOT_A_SHEET.into());
    };
    if let Some(component) = component.strip_prefix('?') {
        return Some(placeholder_type_name(component, &property));
    }
    let Some(drawing) = doc.source_path.as_deref() else {
        return Some(NOT_A_SHEET.into());
    };
    // A model-space host takes the layout the drawing was saved as a sheet for.
    let link = doc.sheet_set_data();
    let layout = layout
        .filter(|l| !l.eq_ignore_ascii_case("Model"))
        .map(str::to_string)
        .or_else(|| link.as_ref().map(|d| d.layout_name.clone()));
    let value = ctx.sheet_sets(&mut |db: &SheetSetDatabase| {
        let sheet = db.sheet_for(drawing, layout.as_deref())?;
        db.sheet_value(sheet, &component, &property)
    });
    Some(match value {
        Some(v) => text_case(v, &fmt),
        None => NOT_A_SHEET.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipher_and_round_trip() {
        assert_eq!(encode(b"<?xml"), vec![0xD0, 0xCD, 0x14, 0xE3, 0xE0]);
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&all)), all);

        let mut db = SheetSetDatabase::new("Demo Set", "A & B <c>");
        let set = db.sheet_set().id().to_string();
        let sub = db.add_subset(&set, "Architectural", "Arch subset").unwrap();
        let sheet = db.add_sheet(&sub, "A-101", "Ground Floor Plan", "Sheet description").unwrap();
        db.set_layout_reference(
            &sheet,
            "Layout",
            &LayoutReference { file_name: "C:\\p\\sheet1.dwg".into(), name: "Layout1".into(), handle: "6F".into() },
        );
        set_custom_property(db.sheet_set_mut(), "Drawn By", "RS", CUSTOM_SHEET_PROP);
        let back = SheetSetDatabase::from_bytes(&db.to_bytes()).unwrap();
        assert_eq!(back.root, db.root);
        let s = back.sheet_for("c:/p/SHEET1.dwg", Some("Layout1")).unwrap();
        assert_eq!(back.sheet_value(s, "Sheet", "NumberAndTitle").unwrap(), "A-101 - Ground Floor Plan");
        assert_eq!(back.sheet_value(s, "Subset", "Name").unwrap(), "Architectural");
        assert_eq!(back.sheet_value(s, "Sheet", "Drawn By").unwrap(), "RS");
        assert_eq!(back.sheet_value(s, "SheetSet", "Description").unwrap(), "A & B <c>");
        // Named children sorted, Layout between IssuePurpose and Number.
        let names: Vec<_> = s.children.iter().filter_map(|c| c.propname()).collect();
        assert_eq!(names, ["CustomPropertyBag", "Desc", "Layout", "Number", "SheetViews", "Title"]);
        #[cfg(windows)]
        assert_eq!(
            relative_path(std::path::Path::new("C:\\p"), std::path::Path::new("C:\\p\\sheet1.dwg")).unwrap(),
            ".\\sheet1.dwg"
        );
        assert_eq!(
            parse_code("\\AcSm.16.2 ?Sheet.Drawn By \\f \"%tc4\""),
            Some(("?Sheet".into(), "Drawn By".into(), "%tc4".into()))
        );
    }
}
