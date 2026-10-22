//! Source-independent typed capture and inspection, without CAD writeback claims.
#![cfg(feature = "typed-storage")]

use acadrust::{
    entities::{EntityType, Line, RawRecord},
    storage::{StorageErrorKind, StorageNamespace, TypedRecordContainer},
    types::{DxfVersion, Handle},
    CadDocument,
};
use std::sync::Arc;

#[test]
fn capture_preserves_backing_markers_duplicate_names_and_stale_lookup_values() {
    use acadrust::{
        classes::DxfClass,
        entities::{Block, BlockEnd},
        tables::VPort,
        types::Vector3,
    };
    let mut source = CadDocument::new();
    source
        .add_entity(EntityType::Block(Block::new("Local", Vector3::ZERO)))
        .unwrap();
    source
        .add_entity(EntityType::BlockEnd(BlockEnd::new()))
        .unwrap();
    for number in [519, 503] {
        let mut class = DxfClass::new("SAME_NAME", "VendorClass");
        class.class_number = number;
        source.classes.push_preserving(class);
    }
    source.vports = acadrust::tables::Table::new();
    for width in [19.0, 23.0] {
        let mut vport = VPort::new("*Active");
        vport.handle = source.allocate_handle();
        vport.name = "*Active".into();
        vport.view_height = width;
        source.vports.add_allow_duplicate(vport);
    }
    source.layers.get_mut("0").unwrap().name = "Edited behind lookup".into();
    assert!(source.layers.get("Edited behind lookup").is_none());
    let captured = TypedRecordContainer::capture_document(
        &source,
        StorageNamespace::new("order".into()).unwrap(),
    )
    .unwrap();
    drop(source);
    let views = captured.container.records();
    assert!(views
        .entities()
        .iter()
        .any(|e| matches!(e,EntityType::Block(b) if b.name=="Local")));
    assert!(views
        .entities()
        .iter()
        .any(|e| matches!(e, EntityType::BlockEnd(_))));
    assert_eq!(
        views
            .classes()
            .iter()
            .filter(|c| c.dxf_name == "SAME_NAME")
            .map(|c| c.class_number)
            .collect::<Vec<_>>(),
        vec![519, 503]
    );
    assert_eq!(
        views
            .vports()
            .iter()
            .map(|v| v.view_height)
            .collect::<Vec<_>>(),
        vec![19.0, 23.0]
    );
    assert!(views
        .layers()
        .iter()
        .any(|l| l.name == "Edited behind lookup"));
}

#[test]
fn namespace_is_explicit_and_nonempty() {
    assert_eq!(
        StorageNamespace::new(String::new())
            .map(|_| ())
            .expect_err("expected rejection")
            .kind,
        StorageErrorKind::InvalidNamespace
    );
    assert_eq!(
        StorageNamespace::new("source-A".into()).unwrap().as_str(),
        "source-A"
    );
}

#[test]
fn owned_capture_keeps_typed_records_and_document_context() {
    let mut source = CadDocument::new();
    source.header.create_ellipse_as_polyline = true;
    source.source_path = Some("drawing-source.dwg".into());
    let handle = source
        .add_entity(EntityType::Line(Line::default()))
        .unwrap();
    let common = source.get_entity_mut(handle).unwrap().common_mut();
    common.layer = "Preserved".into();
    common.layer_handle = Some(Handle::new(0x20_0000_0000_0001));
    common.material_flags = 3;
    common.shadow_flags = 2;
    common.graphic_data = Some(vec![0xde, 0xad, 0xbe, 0xef]);
    common.raw_record = Some(Arc::new(RawRecord {
        data: vec![0xfa, 0xce],
        handle_bits: 8,
        version: DxfVersion::AC1032,
    }));
    common.has_ds_data = true;

    let captured = TypedRecordContainer::capture_document(
        &source,
        StorageNamespace::new("source-A".into()).unwrap(),
    )
    .unwrap();
    assert_eq!(captured.report, *captured.container.capture_report());
    assert_eq!(captured.container.metadata().namespace.as_str(), "source-A");
    assert!(source
        .get_entity(handle)
        .unwrap()
        .common()
        .raw_record
        .is_some());
    source.get_entity_mut(handle).unwrap().common_mut().layer = "Edited later".into();
    drop(source);

    let bytes = captured.container.to_bytes().unwrap();
    assert_eq!(&bytes[..12], b"OCCTSNAP\0\0\0\x01");
    let restored = TypedRecordContainer::from_bytes(&bytes).unwrap();
    let views = restored.records();
    let line = views
        .entities()
        .iter()
        .find(|entity| entity.common().handle == handle)
        .unwrap();
    assert_eq!(line.common().layer, "Preserved");
    assert_eq!(
        line.common().layer_handle,
        Some(Handle::new(0x20_0000_0000_0001))
    );
    assert_eq!(line.common().material_flags, 3);
    assert_eq!(line.common().shadow_flags, 2);
    assert_eq!(line.common().graphic_data, None);
    assert_eq!(line.common().raw_record, None);
    assert!(!line.common().has_ds_data);
    assert!(views.document_context().header.create_ellipse_as_polyline);
    assert_eq!(
        views.document_context().source_path.as_deref(),
        Some("drawing-source.dwg")
    );
    assert!(!views.layers().is_empty());
    assert!(!views.block_records().is_empty());
}

#[test]
fn captured_side_views_remain_inspectable_and_explicitly_unqualified() {
    let mut source = CadDocument::new();
    let h = source.allocate_handle();
    source.fields.insert(
        h,
        acadrust::document::FieldDef {
            handle: h,
            owner: Handle::NULL,
            evaluator: "AcExpr".into(),
            code: "Authored expression".into(),
            objects: vec![Handle::new(0x20_0000_0000_0001)],
        },
    );
    let captured = TypedRecordContainer::capture_document(
        &source,
        StorageNamespace::new("side-views".into()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        captured.container.records().document_context().fields[0]
            .1
            .code,
        "Authored expression"
    );
    assert!(captured
        .report
        .entries
        .iter()
        .any(|e| e.location == format!("context/fields/{}", h.value())
            && e.code == "context.side_view_unqualified"));
}
