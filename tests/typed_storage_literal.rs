#![cfg(feature = "typed-storage")]
#[path = "support/typed_storage_wire.rs"]
mod wire;
use acadrust::{entities::EntityType, storage::TypedRecordContainer, types::Vector3};

const V1: &[u8] = include_bytes!("fixtures/typed_storage/v1-minimal.occts");

#[test]
fn frozen_v1_is_readable_without_running_the_writer() {
    let stored = TypedRecordContainer::from_bytes(V1).unwrap();
    assert_eq!(stored.metadata().format_version, 1);
    assert_eq!(stored.metadata().namespace.as_str(), "v1-minimal");
    assert_eq!(stored.metadata().writer_codec_version, "0.6.4");
    let views = stored.records();
    assert_eq!(
        views.document_context().summary_info.title,
        "Frozen V1 typed fixture"
    );
    assert!(views.document_context().header.create_ellipse_as_polyline);
    let line = views
        .entities()
        .iter()
        .find_map(|e| match e {
            EntityType::Line(line) => Some(line),
            _ => None,
        })
        .unwrap();
    assert_eq!(line.start, Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(line.end, Vector3::new(7.0, 8.0, 9.0));
    assert_eq!(line.common.material_flags, 3);
    assert!(views
        .classes()
        .iter()
        .any(|c| c.dxf_name == "FROZEN_CLASS" && c.class_number == 777));
    assert_eq!(
        stored.capture_report().entries[0].code,
        "storage.typed_only"
    );
}

#[test]
fn v1_model_defaults_apply_only_to_declared_optional_serde_fields() {
    let (env, mut body) = wire::unpack(V1);
    let line = body["entities"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find_map(|e| e.get_mut("Line"))
        .unwrap();
    let common = line["common"].as_object_mut().unwrap();
    for name in [
        "layer_handle",
        "linetype_handle",
        "color_book_handle",
        "face_visual_style_handle",
        "edge_visual_style_handle",
        "material_flags",
        "material_handle",
        "shadow_flags",
        "plotstyle_flags",
        "plotstyle_handle",
        "entity_mode",
    ] {
        assert!(common.remove(name).is_some());
    }
    let stored = TypedRecordContainer::from_bytes(&wire::pack(env, &body)).unwrap();
    let views = stored.records();
    let common = views
        .entities()
        .iter()
        .find(|e| matches!(e, EntityType::Line(_)))
        .unwrap()
        .common();
    assert_eq!(common.material_flags, 0);
    assert_eq!(common.shadow_flags, 0);
    assert_eq!(common.layer_handle, None);
}
