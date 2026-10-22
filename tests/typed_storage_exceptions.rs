#![cfg(feature = "typed-storage")]
#[path = "support/typed_storage_wire.rs"]
mod wire;
use acadrust::{
    entities::{
        EntityCommon, EntityType, ExtendedEntity, ExtendedEntityData, FormatData,
        LayoutPrintConfigData, UnknownEntity,
    },
    objects::{
        ClassObject, ClassObjectData, CsacDocumentOptions, ObjectType, ProxyPayload,
        RegisteredClassObject,
    },
    storage::{ContentCoverage, StorageErrorKind, StorageNamespace, TypedRecordContainer},
    types::{DxfVersion, Handle},
    CadDocument,
};
use serde_json::json;

#[test]
fn exception_records_exclude_framing_but_keep_their_typed_content() {
    let mut source = CadDocument::new();
    let raw = b"EXCLUDED_EXCEPTION_ORIGINAL_RECORD".to_vec();
    let mut unknown = UnknownEntity::new("VENDOR_ENTITY");
    unknown.raw_dwg_data = Some(raw.clone());
    unknown.dwg_handle_bits = 91;
    unknown.raw_dxf_codes = Some(vec![(1, "EXCLUDED_DXF_RECORD".into())]);
    source.add_entity(EntityType::Unknown(unknown)).unwrap();
    for data in [
        ExtendedEntityData::Format(FormatData {
            raw_dwg_data: Some(raw.clone()),
            raw_dwg_handle_bits: 92,
            raw_dwg_version: Some(DxfVersion::AC1032),
            raw_dxf_codes: None,
        }),
        ExtendedEntityData::LayoutPrintConfig(LayoutPrintConfigData {
            class_version: 7,
            flag: 13,
            raw_dwg_data: Some(raw.clone()),
            raw_dwg_handle_bits: 93,
            raw_dwg_version: Some(DxfVersion::AC1032),
        }),
    ] {
        source
            .add_entity(EntityType::Extended(Box::new(ExtendedEntity {
                common: EntityCommon::default(),
                data,
            })))
            .unwrap();
    }
    let unknown_handle = source.allocate_handle();
    source.objects.insert(
        unknown_handle,
        ObjectType::Unknown {
            type_name: "VENDOR_OBJECT".into(),
            handle: unknown_handle,
            owner: Handle::NULL,
            raw_dwg_data: Some(raw.clone()),
            raw_dxf_codes: None,
            raw_dwg_handle_bits: 94,
            raw_dwg_version: Some(DxfVersion::AC1032),
        },
    );
    let registered_handle = source.allocate_handle();
    let payload = ProxyPayload::from_bytes(b"ALLOWED_STANDALONE_PROXY_BYTES");
    source.objects.insert(
        registered_handle,
        ObjectType::RegisteredClass(RegisteredClassObject {
            handle: registered_handle,
            dxf_name: "REGISTERED_VENDOR".into(),
            payload: payload.clone(),
            raw_dwg_data: Some(raw.clone()),
            raw_dwg_handle_bits: 95,
            ..Default::default()
        }),
    );
    let class_handle = source.allocate_handle();
    source.objects.insert(
        class_handle,
        ObjectType::ClassObject(ClassObject {
            handle: class_handle,
            data: ClassObjectData::CsacDocumentOptions(CsacDocumentOptions {
                class_version: 17,
                raw_dwg_data: Some(raw.clone()),
                raw_dwg_handle_bits: 96,
                raw_dwg_version: Some(DxfVersion::AC1032),
            }),
            ..Default::default()
        }),
    );
    let captured = TypedRecordContainer::capture_document(
        &source,
        StorageNamespace::new("exceptions".into()).unwrap(),
    )
    .unwrap();
    assert!(captured
        .report
        .entries
        .iter()
        .any(|e| e.coverage == ContentCoverage::ShellOnly));
    let bytes = captured.container.to_bytes().unwrap();
    for sentinel in [raw.as_slice(), b"EXCLUDED_DXF_RECORD"] {
        assert!(!bytes.windows(sentinel.len()).any(|b| b == sentinel));
    }
    let read = TypedRecordContainer::from_bytes(&bytes).unwrap();
    let views = read.records();
    let ObjectType::RegisteredClass(registered) = &views
        .objects()
        .iter()
        .find(|(h, _)| *h == registered_handle)
        .unwrap()
        .1
    else {
        panic!("registered")
    };
    assert_eq!(registered.payload, payload);
    assert_eq!(registered.raw_dwg_handle_bits, 0);
    let ObjectType::ClassObject(class) = &views
        .objects()
        .iter()
        .find(|(h, _)| *h == class_handle)
        .unwrap()
        .1
    else {
        panic!("class")
    };
    let ClassObjectData::CsacDocumentOptions(options) = &class.data else {
        panic!("options")
    };
    assert_eq!(options.class_version, 17);
    assert_eq!(options.raw_dwg_handle_bits, 0);
    for entity in views.entities() {
        match entity {
            EntityType::Unknown(value) => assert_eq!(value.dwg_handle_bits, 0),
            EntityType::Extended(value) => match &value.data {
                ExtendedEntityData::Format(value) => assert_eq!(value.raw_dwg_handle_bits, 0),
                ExtendedEntityData::LayoutPrintConfig(value) => {
                    assert_eq!(value.flag, 13);
                    assert_eq!(value.raw_dwg_handle_bits, 0);
                }
                _ => {}
            },
            _ => {}
        }
    }
    // Even framing fields accepted by ordinary model Serde are not V1 input.
    let (env, body) = wire::unpack(&bytes);
    for (variant, handle) in [
        ("Unknown", unknown_handle),
        ("RegisteredClass", registered_handle),
    ] {
        let mut altered = body.clone();
        let pair = altered["objects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|pair| pair[0] == json!(handle.value()))
            .unwrap();
        pair[1][variant]["raw_dwg_handle_bits"] = json!(100);
        assert_eq!(
            TypedRecordContainer::from_bytes(&wire::pack(env.clone(), &altered))
                .map(|_| ())
                .expect_err("excluded framing")
                .kind,
            StorageErrorKind::MalformedInput
        );
    }
}
