#![cfg(feature = "typed-storage")]
#[path = "support/typed_storage_wire.rs"]
mod wire;

use acadrust::{
    entities::{EntityType, Line},
    objects::{ObjectType, PlotSettings, SortEntitiesTable, XRecord},
    storage::{StorageErrorKind, StorageNamespace, TypedRecordContainer},
    types::{Handle, Vector3},
    CadDocument,
};
use serde_json::{json, Value};

fn capture(source: &CadDocument) -> TypedRecordContainer {
    TypedRecordContainer::capture_document(
        source,
        StorageNamespace::new("boundary-test".into()).unwrap(),
    )
    .unwrap()
    .container
}

#[test]
fn operational_cache_and_raw_xrecord_fields_are_excluded() {
    let mut source = CadDocument::new();
    let xhandle = source.allocate_handle();
    let xrecord = XRecord {
        handle: xhandle,
        raw_data: b"EXCLUDED_XRECORD_RAW_DATA".to_vec(),
        entries_complete: false,
        raw_dwg_handle_bits: 73,
        ..XRecord::new()
    };
    source.objects.insert(xhandle, ObjectType::XRecord(xrecord));
    let phandle = source.allocate_handle();
    let mut plot = PlotSettings::new("Authored plot");
    plot.handle = phandle;
    plot.set_custom_scale(2.0, 4.0);
    let mut json = serde_json::to_value(&plot).unwrap();
    json["cached_scale"] = json!(999.0);
    let plot: PlotSettings = serde_json::from_value(json).unwrap();
    assert_eq!(plot.scale_factor(), 999.0);
    source
        .objects
        .insert(phandle, ObjectType::PlotSettings(plot));
    let shandle = source.allocate_handle();
    let mut sort = SortEntitiesTable::new();
    sort.handle = shandle;
    sort.add_entry(Handle::new(123), Handle::new(u64::MAX));
    source
        .objects
        .insert(shandle, ObjectType::SortEntitiesTable(sort));

    let container = capture(&source);
    for (_, value) in container.records().objects() {
        match value {
            ObjectType::XRecord(value) if value.handle == xhandle => {
                assert!(value.raw_data.is_empty());
                assert_eq!(value.raw_dwg_handle_bits, 0);
                assert!(!value.entries_complete);
            }
            ObjectType::PlotSettings(value) if value.handle == phandle => {
                assert_eq!(value.scale_factor(), 0.5)
            }
            _ => {}
        }
    }
    let bytes = container.to_bytes().unwrap();
    let (_, body) = wire::unpack(&bytes);
    let object = |h: Handle| {
        body["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pair| pair[0] == json!(h.value()))
            .unwrap()[1]
            .clone()
    };
    let x = object(xhandle);
    assert!(x["XRecord"].get("raw_data").is_none());
    assert!(x["XRecord"].get("raw_dwg_handle_bits").is_none());
    assert_eq!(x["XRecord"]["entries_complete"], json!(false));
    assert!(object(phandle)["PlotSettings"]
        .get("cached_scale")
        .is_none());
    let sort = object(shandle);
    assert!(sort["SortEntitiesTable"].get("entry_map").is_none());
    assert_eq!(
        sort["SortEntitiesTable"]["entries"][0]["sort_value"].as_u64(),
        Some(u64::MAX)
    );
    assert!(!bytes
        .windows(b"EXCLUDED_XRECORD_RAW_DATA".len())
        .any(|b| b == b"EXCLUDED_XRECORD_RAW_DATA"));
}

#[test]
fn excluded_cache_keys_are_rejected_on_input() {
    let mut source = CadDocument::new();
    let h = source.allocate_handle();
    let mut plot = PlotSettings::new("plot");
    plot.handle = h;
    source.objects.insert(h, ObjectType::PlotSettings(plot));
    let (env, mut body) = wire::unpack(&capture(&source).to_bytes().unwrap());
    let objects = body["objects"].as_array_mut().unwrap();
    let object = objects
        .iter_mut()
        .find(|pair| pair[0] == json!(h.value()))
        .unwrap();
    object[1]["PlotSettings"]["cached_scale"] = Value::Null;
    assert_eq!(
        TypedRecordContainer::from_bytes(&wire::pack(env, &body))
            .map(|_| ())
            .expect_err("expected rejection")
            .kind,
        StorageErrorKind::MalformedInput
    );
}

#[test]
fn every_float_bit_pattern_and_large_reference_survives() {
    for bits in [
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x7ff0_0000_0000_0000,
        0x7ff8_0000_0000_0042,
    ] {
        let mut source = CadDocument::new();
        let h = source
            .add_entity(EntityType::Line(Line::from_points(
                Vector3::new(f64::from_bits(bits), 0.0, 0.0),
                Vector3::ZERO,
            )))
            .unwrap();
        source
            .get_entity_mut(h)
            .unwrap()
            .common_mut()
            .material_handle = Some(Handle::new(u64::MAX));
        let bytes = capture(&source).to_bytes().unwrap();
        let container = TypedRecordContainer::from_bytes(&bytes).unwrap();
        let views = container.records();
        let EntityType::Line(line) = views
            .entities()
            .iter()
            .find(|e| e.common().handle == h)
            .unwrap()
        else {
            panic!("LINE")
        };
        assert_eq!(line.start.x.to_bits(), bits);
        assert_eq!(line.common.material_handle, Some(Handle::new(u64::MAX)));
    }
}
