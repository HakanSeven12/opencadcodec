#![cfg(feature = "typed-storage")]
use acadrust::{
    entities::{EntityType, Helix, Line},
    objects::{Field, ObjectType},
    storage::{CapturedXDataValue, StorageNamespace, TypedRecordContainer},
    types::{DxfVersion, Handle},
    xdata::{ExtendedDataRecord, XDataValue},
    CadDocument,
};

fn capture(source: &CadDocument) -> TypedRecordContainer {
    let captured = TypedRecordContainer::capture_document(
        source,
        StorageNamespace::new("eed".into()).unwrap(),
    )
    .unwrap();
    TypedRecordContainer::from_bytes(&captured.container.to_bytes().unwrap()).unwrap()
}

#[test]
fn missing_app_context_keeps_decoded_values_without_a_fictitious_application() {
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let h = source
        .add_entity(EntityType::Line(Line::default()))
        .unwrap();
    let missing = 0x20_0000_0000_0001;
    source
        .get_entity_mut(h)
        .unwrap()
        .common_mut()
        .extended_data
        .raw_dwg_eed
        .push((missing, vec![70, 17, 0]));
    let container = capture(&source);
    let views = container.records();
    let evidence = views.extended_data_evidence();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].source_handle, h);
    assert_eq!(evidence[0].application_handle, Handle::new(missing));
    assert_eq!(evidence[0].application_name, None);
    assert_eq!(
        evidence[0].values,
        vec![CapturedXDataValue::Value(XDataValue::Integer16(17))]
    );
    let line = views
        .entities()
        .iter()
        .find(|e| e.common().handle == h)
        .unwrap();
    assert!(line.common().extended_data.records().is_empty());
    assert!(line.common().extended_data.raw_dwg_eed.is_empty());
    assert_eq!(
        source
            .get_entity(h)
            .unwrap()
            .common()
            .extended_data
            .raw_dwg_eed
            .len(),
        1
    );
}

#[test]
fn nested_eed_with_missing_layer_keeps_the_source_reference_explicit() {
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let app = source.app_ids.iter().next().unwrap().handle;
    let missing = 0x20_0000_0000_0017_u64;
    let mut bytes = vec![3];
    bytes.extend_from_slice(&missing.to_be_bytes());
    bytes.extend_from_slice(&[70, 42, 0]);
    let mut helix = Helix::new();
    helix
        .spline
        .common
        .extended_data
        .raw_dwg_eed
        .push((app.value(), bytes));
    let h = source
        .add_entity(EntityType::Helix(Box::new(helix)))
        .unwrap();
    let container = capture(&source);
    let views = container.records();
    let evidence = views.extended_data_evidence();
    assert_eq!(evidence.len(), 1);
    assert!(evidence[0].location.contains("spline"));
    assert_eq!(
        evidence[0].values,
        vec![
            CapturedXDataValue::UnresolvedLayer(Handle::new(missing)),
            CapturedXDataValue::Value(XDataValue::Integer16(42))
        ]
    );
    let EntityType::Helix(helix) = views
        .entities()
        .iter()
        .find(|e| e.common().handle == h)
        .unwrap()
    else {
        panic!("HELIX")
    };
    assert!(helix.spline.common.extended_data.records().is_empty());
    assert!(helix.spline.common.extended_data.raw_dwg_eed.is_empty());
}

#[test]
fn typed_authored_values_are_not_replaced_by_decoded_eed() {
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let app = source.app_ids.iter().next().unwrap().clone();
    let h = source
        .add_entity(EntityType::Line(Line::default()))
        .unwrap();
    let xdata = &mut source.get_entity_mut(h).unwrap().common_mut().extended_data;
    xdata.add_record(ExtendedDataRecord {
        application_name: app.name.clone(),
        values: vec![XDataValue::Integer16(99)],
    });
    xdata
        .raw_dwg_eed
        .push((app.handle.value(), vec![70, 17, 0]));
    let container = capture(&source);
    let views = container.records();
    let line = views
        .entities()
        .iter()
        .find(|e| e.common().handle == h)
        .unwrap();
    assert_eq!(
        line.common().extended_data.records()[0].values,
        vec![XDataValue::Integer16(99)]
    );
    assert_eq!(
        views.extended_data_evidence()[0].values,
        vec![CapturedXDataValue::Value(XDataValue::Integer16(17))]
    );
}

#[test]
fn field_owned_xdata_is_decoded_without_losing_its_existing_records() {
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let app = source.app_ids.iter().next().unwrap().clone();
    let h = source.allocate_handle();
    let mut field = Field {
        handle: h,
        ..Default::default()
    };
    field
        .xdata
        .raw_dwg_eed
        .push((app.handle.value(), vec![70, 23, 0]));
    source.objects.insert(h, ObjectType::Field(field));
    let container = capture(&source);
    let views = container.records();
    let ObjectType::Field(field) = &views.objects().iter().find(|(key, _)| *key == h).unwrap().1
    else {
        panic!("FIELD")
    };
    assert_eq!(
        field.xdata.get_record(&app.name).unwrap().values,
        vec![XDataValue::Integer16(23)]
    );
    assert!(field.xdata.raw_dwg_eed.is_empty());
}

#[test]
fn undecodable_eed_is_reported_without_inventing_a_decoded_prefix() {
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let h = source
        .add_entity(EntityType::Line(Line::default()))
        .unwrap();
    source
        .get_entity_mut(h)
        .unwrap()
        .common_mut()
        .extended_data
        .raw_dwg_eed
        .push((123, vec![70, 17, 0, 255]));
    let container = capture(&source);
    assert!(container.records().extended_data_evidence().is_empty());
    assert!(container
        .capture_report()
        .entries
        .iter()
        .any(|e| e.code == "eed.undecodable"));
}
