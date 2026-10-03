//! DWG -> JSON silver dump for round-trip comparison.
//!
//! Usage:
//!   cargo run --bin dwg2json --features serde -- INPUT_DWG [OUTPUT_JSON]
//!
//! This binary writes a JSON representation of a `CadDocument` *and* a
//! companion map `_common_dwg` keyed by entity handle. The map exposes the
//! `EntityCommon` fields that are intentionally skipped by the crate's serde
//! implementation because they are DWG storage / round-trip only. Including
//! them lets the gold-vs-silver diff see them without polluting the public
//! entity schema.

use std::collections::BTreeMap;
use std::path::PathBuf;

use acadrust::entities::EntityCommon;
use acadrust::types::Handle;
use acadrust::CadDocument;
use acadrust::DwgReader;
use serde::Serialize;

#[derive(Serialize)]
struct SilverDump {
    #[serde(flatten)]
    document: CadDocument,
    /// Storage-only entity common fields keyed by handle (hex string).
    #[serde(rename = "_common_dwg", skip_serializing_if = "BTreeMap::is_empty")]
    common_dwg: BTreeMap<String, SilverEntityCommon>,
}

/// Mirror of `EntityCommon` with the fields that the crate skips for serde.
///
/// Fields that gold only serializes from a certain DWG version onwards are
/// `Option` and set to `None` when the document version predates them, so the
/// silver dump does not emit fields that gold cannot have:
///   - `material_flags` / `shadow_flags`: gold `SINCE R_2007a`
///   - `has_ds_data`: gold `SINCE R_2013`
#[derive(Serialize)]
struct SilverEntityCommon {
    handle: Handle,
    #[serde(skip_serializing_if = "Option::is_none")]
    linetype_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graphic_data: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_book_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    face_visual_style_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_visual_style_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    material_flags: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    material_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_flags: Option<u8>,
    plotstyle_flags: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    plotstyle_handle: Option<Handle>,
    linetype_flags: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_ds_data: Option<bool>,
    /// Gold `is_xdic_missing` bit (SINCE R_2004a). Derived: gold sets it
    /// exactly when the entity carries no xdictionary handle.
    #[serde(skip_serializing_if = "Option::is_none")]
    is_xdic_missing: Option<bool>,
    /// Gold `has_full_visualstyle` bit (SINCE R_2010b). Derived from the
    /// presence of the full visual-style handle.
    #[serde(skip_serializing_if = "Option::is_none")]
    has_full_visualstyle: Option<bool>,
    /// Gold `has_face_visualstyle` bit (SINCE R_2010b).
    #[serde(skip_serializing_if = "Option::is_none")]
    has_face_visualstyle: Option<bool>,
    /// Gold `has_edge_visualstyle` bit (SINCE R_2010b).
    #[serde(skip_serializing_if = "Option::is_none")]
    has_edge_visualstyle: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prev_entity_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_entity_handle: Option<Handle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nolinks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    z_are_zero: Option<bool>,
}

impl SilverEntityCommon {
    fn from_common(common: &EntityCommon, version: acadrust::types::DxfVersion) -> Self {
        use acadrust::types::DxfVersion;
        let r2004 = version >= DxfVersion::AC1018;
        let r2007 = version >= DxfVersion::AC1021;
        let r2010 = version >= DxfVersion::AC1024;
        let r2013 = version >= DxfVersion::AC1027;
        Self {
            handle: common.handle,
            linetype_handle: common.linetype_handle,
            graphic_data: common.graphic_data.clone(),
            color_book_handle: common.color_book_handle,
            face_visual_style_handle: common.face_visual_style_handle,
            edge_visual_style_handle: common.edge_visual_style_handle,
            material_flags: r2007.then_some(common.material_flags),
            material_handle: common.material_handle,
            shadow_flags: r2007.then_some(common.shadow_flags),
            plotstyle_flags: common.plotstyle_flags,
            plotstyle_handle: common.plotstyle_handle,
            linetype_flags: common.linetype_flags,
            entity_mode: common.entity_mode,
            has_ds_data: r2013.then_some(common.has_ds_data),
            is_xdic_missing: r2004.then_some(common.xdictionary_handle.is_none()),
            has_full_visualstyle: r2010.then_some(common.full_visual_style_handle.is_some()),
            has_face_visualstyle: r2010.then_some(common.face_visual_style_handle.is_some()),
            has_edge_visualstyle: r2010.then_some(common.edge_visual_style_handle.is_some()),
            prev_entity_handle: common.prev_entity_handle,
            next_entity_handle: common.next_entity_handle,
            nolinks: common.nolinks,
            z_are_zero: common.z_are_zero,
        }
    }
}

/// Return `Block` and `BlockEnd` entities for every block record.
///
/// `CadDocument::entities()` deliberately excludes these delimiter entities so
/// callers see only drawable geometry. For the silver dump we need the
/// storage-only `EntityCommon` fields on every entity, including the
/// delimiters, so we collect them directly via `get_entity`.
fn block_entity_iter(doc: &CadDocument) -> impl Iterator<Item = &acadrust::entities::EntityType> {
    doc.block_records.iter().flat_map(|br| {
        [br.block_entity_handle, br.block_end_handle]
            .into_iter()
            .filter_map(|handle| doc.get_entity(handle))
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: dwg2json INPUT_DWG [OUTPUT_JSON]");
        std::process::exit(1);
    }
    let input = PathBuf::from(&args[1]);
    let output = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("json"));

    let mut reader = DwgReader::from_file(&input)?;
    let doc: CadDocument = reader.read()?;

    let mut common_dwg: BTreeMap<String, SilverEntityCommon> = BTreeMap::new();
    for entity in doc.entities().chain(block_entity_iter(&doc)) {
        let silver = SilverEntityCommon::from_common(entity.common(), doc.version);
        common_dwg.insert(format!("{}", silver.handle), silver);
    }
    // Polyface-mesh sub-entity records: the models retain the full wire
    // `EntityCommon` (the pre-R2004 entity chain — nolinks/prev/next — is
    // author data, not a convention: the golden authored files chain only
    // some records where the libredwg corpus files chain per-family
    // patterns), so their storage-only fields join the `_common_dwg` map
    // keyed by the sub-entity's own handle.
    for entity in doc.entities() {
        if let acadrust::entities::EntityType::PolyfaceMesh(pm) = entity {
            for v in &pm.vertices {
                let silver = SilverEntityCommon::from_common(&v.common, doc.version);
                common_dwg.insert(format!("{}", silver.handle), silver);
            }
            for f in &pm.faces {
                let silver = SilverEntityCommon::from_common(&f.common, doc.version);
                common_dwg.insert(format!("{}", silver.handle), silver);
            }
        }
    }

    let dump = SilverDump {
        document: doc,
        common_dwg,
    };

    let file = std::fs::File::create(&output)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &dump)?;
    println!("Wrote {}", output.display());
    Ok(())
}
