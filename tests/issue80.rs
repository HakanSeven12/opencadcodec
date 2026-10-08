//! Repro for issue #80: a document read from DWG, mutated in place, then
//! written back contains invalid layer references.
//!
//! The reporter renamed a layer through `layers.iter_mut()`. Tables key entries
//! by the normalized name captured at insertion, so assigning `layer.name`
//! directly leaves the entry reachable only under its *old* name. Every
//! name-based lookup then misses, and the writer emitted a NULL layer hard
//! pointer for each entity on that layer â€” a required reference.

use std::io::Cursor;

use opencadcodec::entities::{EntityType, Line};
use opencadcodec::tables::Layer;
use opencadcodec::types::{DxfVersion, Handle};
use opencadcodec::{CadDocument, DwgReader, DwgWriter, DxfWriter};

/// A drawing on disk: one layer besides "0", carrying one entity.
fn source_document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    doc.layers.add(Layer::new("OLD_NAME")).unwrap();
    let mut line = Line::from_coords(0.0, 0.0, 0.0, 1.0, 1.0, 0.0);
    line.common.layer = "OLD_NAME".to_string();
    doc.add_entity(EntityType::Line(line)).unwrap();

    let bytes = DwgWriter::write_to_vec(&doc).expect("write source");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read source")
}

/// The reporter's mutation: rename in place, bypassing `rename_layer`.
fn rename_in_place(doc: &mut CadDocument, old: &str, new: &str) {
    for layer in doc.layers.iter_mut() {
        if layer.name == old {
            layer.name = new.to_string();
        }
    }
}

fn dwg_roundtrip(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read")
}

/// Every entity's layer must name a layer the written table actually defines.
/// An unresolvable one is invalid file data.
fn assert_layers_resolve(doc: &CadDocument, label: &str) {
    for entity in doc.entities() {
        let layer = &entity.common().layer;
        assert!(
            doc.layers.contains(layer),
            "{label}: entity {:?} references undefined layer {layer:?} (defined: {:?})",
            entity.common().handle,
            doc.layers.names().collect::<Vec<_>>(),
        );
    }
}

#[test]
fn repro_issue80_in_place_layer_rename_survives_dwg_roundtrip() {
    // A rename with no new references happens to survive even unfixed: the
    // entities still carry the old name, and the stale key still resolves it.
    // Kept as a guard that the repair does not disturb this working case â€”
    // `repro_issue80_rename_plus_added_entity` is the one that corrupts.
    let mut doc = source_document(DxfVersion::AC1027);
    rename_in_place(&mut doc, "OLD_NAME", "NEW_NAME");

    let rt = dwg_roundtrip(&doc);

    assert_layers_resolve(&rt, "in-place rename");
    assert!(rt.layers.contains("NEW_NAME"), "renamed layer missing");
    assert!(!rt.layers.contains("OLD_NAME"), "old layer name survived");
    // The entity keeps its layer: the rename must not silently move drawing
    // content to layer "0".
    let layers: Vec<String> = rt
        .entities()
        .map(|entity| entity.common().layer.clone())
        .collect();
    assert_eq!(
        layers,
        vec!["NEW_NAME".to_string()],
        "entity lost its layer"
    );
}

#[test]
fn repro_issue80_rename_plus_added_entity() {
    // The full reported flow: read, rename a layer, add entities, write back.
    let mut doc = source_document(DxfVersion::AC1032);
    rename_in_place(&mut doc, "OLD_NAME", "NEW_NAME");
    let mut line = Line::from_coords(2.0, 2.0, 0.0, 3.0, 3.0, 0.0);
    line.common.layer = "NEW_NAME".to_string();
    doc.add_entity(EntityType::Line(line)).unwrap();

    let rt = dwg_roundtrip(&doc);

    assert_layers_resolve(&rt, "rename + added entity");
    assert_eq!(rt.entities().count(), 2, "entity count");
    for entity in rt.entities() {
        assert_eq!(
            entity.common().layer,
            "NEW_NAME",
            "entity {:?} not on the renamed layer",
            entity.common().handle,
        );
    }
}

#[test]
fn repro_issue80_entity_on_undefined_layer_is_not_a_null_pointer() {
    // A layer name that was never added must not become a NULL hard pointer:
    // fall back to "0", which every drawing defines.
    //
    // This pins the behaviour rather than reproducing the failure: acadrust's
    // own reader maps a NULL layer pointer back to "0", so a round-trip through
    // it cannot distinguish the two. The guarantee therefore lives in the writer.
    let mut doc = source_document(DxfVersion::AC1027);
    let mut line = Line::from_coords(4.0, 4.0, 0.0, 5.0, 5.0, 0.0);
    line.common.layer = "GHOST".to_string();
    let handle = doc.add_entity(EntityType::Line(line)).unwrap();

    let rt = dwg_roundtrip(&doc);

    assert_layers_resolve(&rt, "undefined layer");
    let ghost = rt
        .entities()
        .find(|entity| entity.common().handle == handle)
        .expect("added entity missing after roundtrip");
    assert_eq!(ghost.common().layer, "0", "undefined layer not defaulted");
}

#[test]
fn repro_issue80_in_place_rename_survives_dxf_write() {
    // DXF resolves layers by name through the same tables, so it had the same
    // desync: entities referencing a layer the LAYER table no longer defines.
    let mut doc = source_document(DxfVersion::AC1027);
    rename_in_place(&mut doc, "OLD_NAME", "NEW_NAME");

    let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf"))
        .expect("utf8 dxf");

    assert!(text.contains("NEW_NAME"), "renamed layer missing from DXF");
    assert!(
        !text.contains("OLD_NAME"),
        "stale layer name written to DXF"
    );
}

#[test]
fn resync_table_keys_is_a_noop_after_the_documented_rename_api() {
    // `rename_layer` / `Table::rename` keep keys in sync themselves, so the
    // writer's repair pass must find nothing to do and leave them alone.
    let mut doc = source_document(DxfVersion::AC1027);
    doc.rename_layer("OLD_NAME", "NEW_NAME").unwrap();

    assert!(
        !doc.has_stale_table_keys(),
        "documented rename left a stale key"
    );
    assert_eq!(doc.resync_table_keys(), 0, "resync changed a synced table");
    assert_eq!(
        doc.layers
            .get("NEW_NAME")
            .map(|l| l.name.clone())
            .as_deref(),
        Some("NEW_NAME")
    );
}

#[test]
fn resync_table_keys_reports_and_repairs_in_place_renames() {
    let mut doc = source_document(DxfVersion::AC1027);
    rename_in_place(&mut doc, "OLD_NAME", "NEW_NAME");

    assert!(doc.has_stale_table_keys(), "in-place rename not detected");
    // Before the repair the entry is unreachable under its own name.
    assert!(doc.layers.get("NEW_NAME").is_none());

    assert_eq!(doc.resync_table_keys(), 1, "expected one re-keyed entry");

    assert!(!doc.has_stale_table_keys());
    let layer = doc.layers.get("NEW_NAME").expect("layer not re-keyed");
    assert_eq!(layer.name, "NEW_NAME");
    assert_ne!(layer.handle, Handle::NULL, "re-key lost the handle");
    assert!(
        doc.layers.get("OLD_NAME").is_none(),
        "old key still resolves"
    );
}

#[test]
fn resync_keeps_a_rename_that_collides_with_a_live_layer() {
    // Renaming onto a name another layer already owns must not drop either
    // entry, and must not steal the existing layer's entities.
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    doc.layers.add(Layer::new("KEEP")).unwrap();
    doc.layers.add(Layer::new("MOVE")).unwrap();
    let mut line = Line::from_coords(0.0, 0.0, 0.0, 1.0, 1.0, 0.0);
    line.common.layer = "KEEP".to_string();
    doc.add_entity(EntityType::Line(line)).unwrap();
    let keep_handle = doc.layers.get("KEEP").unwrap().handle;

    rename_in_place(&mut doc, "MOVE", "KEEP");
    doc.resync_table_keys();

    assert_eq!(doc.layers.iter().count(), 3, "an entry was dropped");
    // The pre-existing "KEEP" still answers lookups, so its entity is untouched.
    assert_eq!(doc.layers.get("KEEP").map(|l| l.handle), Some(keep_handle));
    let rt = dwg_roundtrip(&doc);
    assert_layers_resolve(&rt, "colliding rename");
}
