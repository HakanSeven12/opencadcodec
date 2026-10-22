#![cfg(feature = "typed-storage")]
use acadrust::{
    compound_file::{BinaryRecord, StructuredStoragePayload},
    entities::{
        AttributeDefinition, AttributeEntity, EntityCommon, EntityType, ExtendedEntity,
        ExtendedEntityData, GeoPositionMarkerData, Insert, MText, Ole2Frame,
    },
    objects::{ObjectType, SortEntitiesTable, XRecord, XRecordEntry, XRecordValue},
    storage::{StorageNamespace, TypedRecordContainer},
    types::{DxfVersion, Handle, Vector3},
    xdata::XDataValue,
    CadDocument,
};
fn stored(source: &CadDocument) -> TypedRecordContainer {
    let captured = TypedRecordContainer::capture_document(
        source,
        StorageNamespace::new("nested".into()).unwrap(),
    )
    .unwrap();
    TypedRecordContainer::from_bytes(&captured.container.to_bytes().unwrap()).unwrap()
}
fn mtext(app: u64, handle: u64) -> Box<MText> {
    let mut text = MText::new();
    text.value = "Authored nested text".into();
    text.common.handle = Handle::new(handle);
    text.common
        .extended_data
        .raw_dwg_eed
        .push((app, vec![70, 39, 0]));
    Box::new(text)
}
#[test]
fn embedded_mtext_decodes_eed_in_attributes_insert_children_and_geo_markers() {
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let app = source.app_ids.iter().next().unwrap().handle;
    let mut definition = AttributeDefinition::new("TAG".into(), "Prompt".into(), "Value".into());
    definition.embedded_mtext = Some(mtext(app.value(), 1001));
    source
        .add_entity(EntityType::AttributeDefinition(definition))
        .unwrap();
    let mut attribute = AttributeEntity::new("TAG".into(), "Value".into());
    attribute.embedded_mtext = Some(mtext(app.value(), 1002));
    source
        .add_entity(EntityType::AttributeEntity(attribute))
        .unwrap();
    let mut insert = Insert::new("Example", Vector3::ZERO);
    let mut child = AttributeEntity::new("CHILD".into(), "Value".into());
    child.embedded_mtext = Some(mtext(app.value(), 1003));
    insert.attributes.push(child);
    source.add_entity(EntityType::Insert(insert)).unwrap();
    source
        .add_entity(EntityType::Extended(Box::new(ExtendedEntity {
            common: EntityCommon::default(),
            data: ExtendedEntityData::GeoPositionMarker(GeoPositionMarkerData {
                class_version: 1,
                position: Vector3::ZERO,
                radius: 1.0,
                notes: "Marker".into(),
                landing_gap: 0.5,
                mtext_visible: true,
                text_alignment: 0,
                enable_frame_text: false,
                embedded_mtext: Some(*mtext(app.value(), 1004)),
            }),
        })))
        .unwrap();
    let container = stored(&source);
    let views = container.records();
    let mut texts = Vec::new();
    for entity in views.entities() {
        match entity {
            EntityType::AttributeDefinition(v) => texts.push(v.embedded_mtext.as_deref().unwrap()),
            EntityType::AttributeEntity(v) => texts.push(v.embedded_mtext.as_deref().unwrap()),
            EntityType::Insert(v) => texts.push(v.attributes[0].embedded_mtext.as_deref().unwrap()),
            EntityType::Extended(v) => {
                if let ExtendedEntityData::GeoPositionMarker(v) = &v.data {
                    texts.push(v.embedded_mtext.as_ref().unwrap());
                }
            }
            _ => {}
        }
    }
    assert_eq!(texts.len(), 4);
    for text in texts {
        assert_eq!(text.value, "Authored nested text");
        assert_eq!(
            text.common
                .extended_data
                .get_record("ACAD")
                .expect("nested EED retained")
                .values,
            vec![XDataValue::Integer16(39)]
        );
        assert!(text.common.extended_data.raw_dwg_eed.is_empty());
    }
    assert_eq!(
        container
            .capture_report()
            .entries
            .iter()
            .filter(|e| e.code == "eed.decoded" && e.location.contains("embedded_mtext/common"))
            .count(),
        4
    );
}

#[test]
fn raw_xrecord_and_sort_sources_have_local_exclusion_evidence() {
    let mut source = CadDocument::new();
    let x = source.allocate_handle();
    let s = source.allocate_handle();
    source.objects.insert(
        x,
        ObjectType::XRecord(XRecord {
            handle: x,
            raw_data: vec![0xf1, 0xf2],
            ..XRecord::new()
        }),
    );
    let mut sort = SortEntitiesTable::new();
    sort.handle = s;
    sort.raw_dxf_codes = Some(vec![(1, "Excluded source code".into())]);
    source
        .objects
        .insert(s, ObjectType::SortEntitiesTable(sort));
    let container = stored(&source);
    let views = container.records();
    for (handle, code, field) in [
        (x, "xrecord.raw_data_excluded", "raw_data"),
        (s, "sort_entities.raw_codes_excluded", "raw_dxf_codes"),
    ] {
        let index = views
            .objects()
            .iter()
            .position(|(h, _)| *h == handle)
            .unwrap();
        assert!(container
            .capture_report()
            .entries
            .iter()
            .any(|e| e.code == code && e.location == format!("objects/{index}/{field}")));
    }
}

#[test]
fn construction_geometry_keeps_available_nested_common_values_and_eed() {
    use acadrust::{
        entities::{EmbeddedEntity, Line, Surface, SurfaceData, SurfaceKind},
        objects::{
            DynamicBlockData, DynamicBlockObject, SolidHistoryLoft, SolidHistoryLoftParameters,
            SolidHistoryOperation,
        },
    };
    let mut source = CadDocument::new();
    source.dwg_source_version = Some(DxfVersion::AC1032);
    let app = source.app_ids.iter().next().unwrap().handle;
    let geometry = |handle| {
        let mut line = Line::default();
        line.common.handle = Handle::new(handle);
        line.common
            .extended_data
            .raw_dwg_eed
            .push((app.value(), vec![70, 51, 0]));
        EmbeddedEntity::Line(line)
    };
    let mut surface = Surface::new(SurfaceKind::Extruded);
    if let SurfaceData::Extruded { sweep_entity, .. } = &mut surface.surface_data {
        *sweep_entity = Some(geometry(1101));
    }
    let surface_handle = source
        .add_entity(EntityType::Surface(Box::new(surface)))
        .unwrap();
    let h = source.allocate_handle();
    let mut object = DynamicBlockObject::new("ACSH_LOFT_CLASS", "AcDbShLoft");
    object.handle = h;
    object.data =
        DynamicBlockData::SolidHistoryNode(SolidHistoryOperation::Loft(SolidHistoryLoft {
            cross_sections: vec![geometry(1102)],
            parameters: Some(SolidHistoryLoftParameters {
                path_entity: Some(geometry(1103)),
                ..Default::default()
            }),
            ..Default::default()
        }));
    source.objects.insert(h, ObjectType::DynamicBlock(object));
    let container = stored(&source);
    let views = container.records();
    let EntityType::Surface(surface) = views
        .entities()
        .iter()
        .find(|e| e.common().handle == surface_handle)
        .unwrap()
    else {
        panic!("surface")
    };
    let SurfaceData::Extruded {
        sweep_entity: Some(EmbeddedEntity::Line(profile)),
        ..
    } = &surface.surface_data
    else {
        panic!("profile")
    };
    let ObjectType::DynamicBlock(object) =
        &views.objects().iter().find(|(key, _)| *key == h).unwrap().1
    else {
        panic!("history")
    };
    let DynamicBlockData::SolidHistoryNode(SolidHistoryOperation::Loft(loft)) = &object.data else {
        panic!("loft")
    };
    let EmbeddedEntity::Line(section) = &loft.cross_sections[0] else {
        panic!("section")
    };
    let Some(EmbeddedEntity::Line(path)) = &loft.parameters.as_ref().unwrap().path_entity else {
        panic!("path")
    };
    for line in [profile, section, path] {
        assert_eq!(
            line.common
                .extended_data
                .get_record("ACAD")
                .expect("construction EED retained")
                .values,
            vec![XDataValue::Integer16(51)]
        );
        assert!(line.common.extended_data.raw_dwg_eed.is_empty());
    }
}

#[test]
fn standalone_ole_wrappers_and_binary_xrecord_values_are_content() {
    let mut source = CadDocument::new();
    let mut ole = Ole2Frame::new();
    ole.source_application = "Example.OLE".into();
    ole.storage = StructuredStoragePayload {
        leading_records: BinaryRecord::split(b"OLE_CONTENT_PREFIX", 4096),
        compound_file: None,
        trailing_records: BinaryRecord::split(b"OLE_CONTENT_SUFFIX", 4096),
    };
    let h = source
        .add_entity(EntityType::Ole2Frame(Box::new(ole)))
        .unwrap();
    let x = source.allocate_handle();
    source.objects.insert(
        x,
        ObjectType::XRecord(XRecord {
            handle: x,
            entries: vec![XRecordEntry::new(
                310,
                XRecordValue::Chunk(vec![0, 255, 1, 128]),
            )],
            ..XRecord::new()
        }),
    );
    let container = stored(&source);
    let views = container.records();
    let EntityType::Ole2Frame(ole) = views
        .entities()
        .iter()
        .find(|e| e.common().handle == h)
        .unwrap()
    else {
        panic!("OLE")
    };
    assert_eq!(ole.source_application, "Example.OLE");
    assert_eq!(
        BinaryRecord::join(&ole.storage.leading_records),
        b"OLE_CONTENT_PREFIX"
    );
    assert_eq!(
        BinaryRecord::join(&ole.storage.trailing_records),
        b"OLE_CONTENT_SUFFIX"
    );
    let ObjectType::XRecord(record) = &views.objects().iter().find(|(h, _)| *h == x).unwrap().1
    else {
        panic!("XRecord")
    };
    assert_eq!(
        record.entries[0].value,
        XRecordValue::Chunk(vec![0, 255, 1, 128])
    );
    assert!(record.raw_data.is_empty());
}
