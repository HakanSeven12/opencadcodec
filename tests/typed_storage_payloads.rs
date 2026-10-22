#![cfg(feature = "typed-storage")]
use acadrust::{
    compound_file::{
        BinaryRecord, CompoundEntry, CompoundFile, CompoundProperty, CompoundPropertySet,
        CompoundPropertySetStream, CompoundPropertyValue, CompoundStream, CompoundStreamContent,
        StructuredStoragePayload,
    },
    entities::{
        AcisData, CellContent, EntityType, SectionSymbol, SectionSymbolPoint, Solid3D, Table,
    },
    objects::{
        AssocVariable, AssociativeData, AssociativeObject, ClassObject, ClassObjectData,
        ObjectType, VbaProject,
    },
    storage::{StorageNamespace, TypedRecordContainer},
    types::Vector3,
    vba::VbaModuleStream,
    CadDocument,
};

fn capture(source: &CadDocument) -> TypedRecordContainer {
    let container = TypedRecordContainer::capture_document(
        source,
        StorageNamespace::new("payloads".into()).unwrap(),
    )
    .unwrap()
    .container;
    TypedRecordContainer::from_bytes(&container.to_bytes().unwrap()).unwrap()
}
fn stream(name: &str, content: CompoundStreamContent) -> CompoundEntry {
    CompoundEntry::Stream(CompoundStream {
        name: name.into(),
        class_id: [0; 16],
        state_bits: 9,
        creation_time: u64::MAX,
        modified_time: 17,
        content,
    })
}

#[test]
fn standalone_vba_cfb_encoding_and_performance_evidence_survives() {
    let prefix = b"ALLOWED_VBA_PERFORMANCE";
    let mut module_bytes = prefix.to_vec();
    module_bytes.extend_from_slice(&[1, 3, 0x30, b'C', b'o', b'd', b'e']);
    let mut module = VbaModuleStream::decode(&module_bytes, prefix.len() as u32, 1252).unwrap();
    module.source = "Authored text differs from original".into();
    let expected_module = serde_json::to_value(&module).unwrap();
    let ansi = CompoundPropertyValue::AnsiString {
        value: "Authored ANSI".into(),
        code_page: 1252,
        original_value: "Original ANSI".into(),
        original_code_page: 1251,
        encoded: BinaryRecord::split(b"ALLOWED_ANSI_ENCODING", 4096),
    };
    let mut file = CompoundFile::default();
    file.root.creation_time = u64::MAX;
    file.root
        .entries
        .push(stream("Module1", CompoundStreamContent::VbaModule(module)));
    file.root.entries.push(stream(
        "Properties",
        CompoundStreamContent::PropertySet(CompoundPropertySetStream {
            version: 1,
            system_identifier: 42,
            class_id: [3; 16],
            sets: vec![CompoundPropertySet {
                format_id: [4; 16],
                properties: vec![
                    CompoundProperty {
                        id: 2,
                        value: ansi.clone(),
                    },
                    CompoundProperty {
                        id: 3,
                        value: CompoundPropertyValue::F32(f32::from_bits(0x7fc0_0042)),
                    },
                    CompoundProperty {
                        id: 4,
                        value: CompoundPropertyValue::U64(u64::MAX),
                    },
                ],
            }],
        }),
    ));
    let mut source = CadDocument::new();
    let h = source.allocate_handle();
    source.objects.insert(
        h,
        ObjectType::ClassObject(ClassObject {
            handle: h,
            data: ClassObjectData::VbaProject(VbaProject {
                storage: StructuredStoragePayload {
                    leading_records: BinaryRecord::split(b"ALLOWED_LEADING", 4096),
                    compound_file: Some(file),
                    trailing_records: BinaryRecord::split(b"ALLOWED_TRAILING", 4096),
                },
            }),
            ..Default::default()
        }),
    );
    let container = capture(&source);
    let views = container.records();
    let ObjectType::ClassObject(object) =
        &views.objects().iter().find(|(key, _)| *key == h).unwrap().1
    else {
        panic!("class")
    };
    let ClassObjectData::VbaProject(project) = &object.data else {
        panic!("VBA")
    };
    let file = project.storage.compound_file.as_ref().unwrap();
    assert_eq!(file.root.creation_time, u64::MAX);
    let CompoundEntry::Stream(module) = &file.root.entries[0] else {
        panic!("stream")
    };
    let CompoundStreamContent::VbaModule(module) = &module.content else {
        panic!("module")
    };
    assert_eq!(serde_json::to_value(module).unwrap(), expected_module);
    let CompoundEntry::Stream(properties) = &file.root.entries[1] else {
        panic!("stream")
    };
    let CompoundStreamContent::PropertySet(properties) = &properties.content else {
        panic!("properties")
    };
    assert_eq!(properties.sets[0].properties[0].value, ansi);
    let CompoundPropertyValue::F32(value) = properties.sets[0].properties[1].value else {
        panic!("f32")
    };
    assert_eq!(value.to_bits(), 0x7fc0_0042);
    assert_eq!(
        properties.sets[0].properties[2].value,
        CompoundPropertyValue::U64(u64::MAX)
    );
    assert_eq!(
        BinaryRecord::join(&project.storage.leading_records),
        b"ALLOWED_LEADING"
    );
    assert_eq!(
        BinaryRecord::join(&project.storage.trailing_records),
        b"ALLOWED_TRAILING"
    );
}

#[test]
fn entity_owned_acis_payloads_are_not_reparsed_or_normalized() {
    let mut solid = Solid3D::new();
    solid.acis_data.sat_data = "Independent authored SAT bytes #\n".into();
    solid.acis_data.sab_data = b"ALLOWED_SAB_DATA".to_vec();
    solid.acis_data.is_binary = true;
    solid.acis_data.extra_acis_data =
        Some(Box::new(AcisData::from_sat("Independent extra body #\n")));
    let expected = solid.acis_data.clone();
    let mut source = CadDocument::new();
    let h = source.add_entity(EntityType::Solid3D(solid)).unwrap();
    let container = capture(&source);
    let views = container.records();
    let EntityType::Solid3D(solid) = views
        .entities()
        .iter()
        .find(|e| e.common().handle == h)
        .unwrap()
    else {
        panic!("solid")
    };
    assert_eq!(solid.acis_data, expected);
}

#[test]
fn meaningful_cached_values_raw_scalars_and_compatibility_views_survive() {
    let mut source = CadDocument::new();
    let h = source.allocate_handle();
    source.objects.insert(
        h,
        ObjectType::Associative(AssociativeObject {
            handle: h,
            data: AssociativeData::Variable(AssocVariable {
                has_cached_value: true,
                cached_value: "Authored cached result".into(),
                expression: "A different expression".into(),
                ..Default::default()
            }),
            ..Default::default()
        }),
    );
    let mut section = SectionSymbol::new();
    section.raw_flags_90 = 0x3141;
    section.raw_point_count_90 = 77;
    section.points.push(SectionSymbolPoint {
        raw_flag_280: 231,
        ..Default::default()
    });
    let sh = source
        .add_entity(EntityType::SectionSymbol(section))
        .unwrap();
    let mut table = Table::new(Vector3::ZERO, 1, 1);
    let mut content = CellContent::new();
    content.value.raw_type_code = 12345;
    content.value.raw_unit_type_code = 67890;
    content.value.binary_value = b"ALLOWED_CELL_BINARY".to_vec();
    table.rows[0].cells[0].contents.push(content);
    let th = source
        .add_entity(EntityType::Table(Box::new(table)))
        .unwrap();
    let container = capture(&source);
    let views = container.records();
    let ObjectType::Associative(associative) =
        &views.objects().iter().find(|(key, _)| *key == h).unwrap().1
    else {
        panic!("assoc")
    };
    let AssociativeData::Variable(variable) = &associative.data else {
        panic!("variable")
    };
    assert!(variable.has_cached_value);
    assert_eq!(variable.cached_value, "Authored cached result");
    let EntityType::SectionSymbol(section) = views
        .entities()
        .iter()
        .find(|e| e.common().handle == sh)
        .unwrap()
    else {
        panic!("section")
    };
    assert_eq!(section.raw_flags_90, 0x3141);
    assert_eq!(section.raw_point_count_90, 77);
    assert_eq!(section.points[0].raw_flag_280, 231);
    let EntityType::Table(table) = views
        .entities()
        .iter()
        .find(|e| e.common().handle == th)
        .unwrap()
    else {
        panic!("table")
    };
    let value = &table.rows[0].cells[0].contents.last().unwrap().value;
    assert_eq!(value.raw_type_code, 12345);
    assert_eq!(value.raw_unit_type_code, 67890);
    assert_eq!(value.binary_value, b"ALLOWED_CELL_BINARY");
}
