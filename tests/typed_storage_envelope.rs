#![cfg(feature = "typed-storage")]
#[path = "support/typed_storage_wire.rs"]
mod wire;
use acadrust::{
    entities::{EntityType, Helix},
    storage::{StorageErrorKind, StorageNamespace, TypedRecordContainer},
    CadDocument,
};
use serde_json::json;

fn fixture() -> Vec<u8> {
    let mut source = CadDocument::new();
    source
        .add_entity(EntityType::Helix(Box::new(Helix::new())))
        .unwrap();
    TypedRecordContainer::capture_document(&source, StorageNamespace::new("wire".into()).unwrap())
        .unwrap()
        .container
        .to_bytes()
        .unwrap()
}
fn malformed(bytes: &[u8]) {
    assert_eq!(
        TypedRecordContainer::from_bytes(bytes)
            .map(|_| ())
            .expect_err("expected rejection")
            .kind,
        StorageErrorKind::MalformedInput
    );
}

#[test]
fn rejects_bad_framing_and_checks_kind_before_record_body() {
    let original = fixture();
    for len in [0, 7, 8, 11, 12, original.len() - 1] {
        malformed(&original[..len]);
    }
    let mut bytes = original.clone();
    bytes[0] = b'X';
    malformed(&bytes);
    let mut bytes = original.clone();
    bytes[11] = 2;
    assert_eq!(
        TypedRecordContainer::from_bytes(&bytes)
            .map(|_| ())
            .expect_err("expected rejection")
            .kind,
        StorageErrorKind::UnsupportedVersion
    );
    let (mut env, _) = wire::unpack(&original);
    env.kind = "future-selection".into();
    env.body.0 = vec![0xc1];
    assert_eq!(
        TypedRecordContainer::from_bytes(&wire::frame(&env))
            .map(|_| ())
            .expect_err("expected rejection")
            .kind,
        StorageErrorKind::UnsupportedKind
    );
}

#[test]
fn rejects_trailing_data_in_both_messagepack_layers() {
    let mut bytes = fixture();
    bytes.push(0xc0);
    malformed(&bytes);
    let (mut env, _) = wire::unpack(&fixture());
    env.body.0.push(0xc0);
    malformed(&wire::frame(&env));
}

#[test]
fn rejects_deep_unknown_fields_and_attempts_to_restore_raw_storage() {
    for key in ["future_field", "raw_record", "graphic_data", "has_ds_data"] {
        let (env, mut body) = wire::unpack(&fixture());
        let entity = body["entities"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entity| entity.get("Helix").is_some())
            .unwrap();
        entity["Helix"]["spline"]["common"][key] = json!(null);
        malformed(&wire::pack(env, &body));
    }
}

#[test]
fn rejects_missing_required_context_including_optional_typed_values() {
    for key in [
        "header",
        "allocation_floor",
        "source_path",
        "preview",
        "dwg_source_version",
    ] {
        let (env, mut body) = wire::unpack(&fixture());
        body["context"].as_object_mut().unwrap().remove(key);
        malformed(&wire::pack(env, &body));
    }
}

#[test]
fn rejects_duplicate_known_fields_in_envelope_and_body() {
    let mut bytes = fixture();
    assert_eq!(bytes[12], 0x85);
    bytes[12] += 1;
    bytes.extend_from_slice(&rmp_serde::to_vec("kind").unwrap());
    bytes.extend_from_slice(&rmp_serde::to_vec("typed-records").unwrap());
    malformed(&bytes);
    let (mut env, _) = wire::unpack(&fixture());
    assert!((0x80..0x8f).contains(&env.body.0[0]));
    env.body.0[0] += 1;
    env.body
        .0
        .extend_from_slice(&rmp_serde::to_vec("entities").unwrap());
    env.body.0.push(0x90);
    malformed(&wire::frame(&env));
}

#[test]
fn rejects_duplicate_record_keys() {
    let (env, mut body) = wire::unpack(&fixture());
    let objects = body["objects"].as_array_mut().unwrap();
    assert!(!objects.is_empty());
    objects.push(objects[0].clone());
    assert_eq!(
        TypedRecordContainer::from_bytes(&wire::pack(env, &body))
            .map(|_| ())
            .expect_err("expected rejection")
            .kind,
        StorageErrorKind::DuplicateIdentity
    );
}

#[test]
fn rejects_positional_structs_in_both_layers_and_nested_models() {
    let (env, body) = wire::unpack(&fixture());
    let mut bytes = b"OCCTSNAP\0\0\0\x01".to_vec();
    bytes.extend_from_slice(
        &rmp_serde::to_vec(&(
            &env.kind,
            &env.writer_codec_version,
            &env.namespace,
            &env.capture_report,
            &env.body,
        ))
        .unwrap(),
    );
    malformed(&bytes);
    let array = json!([
        body["entities"],
        body["objects"],
        body["classes"],
        body["tables"],
        body["context"],
        body["extended_data_evidence"]
    ]);
    malformed(&wire::pack(env.clone(), &array));
    let mut body = body;
    body["entities"][0]["Helix"]["axis_base_point"] = json!([0.0, 0.0, 0.0]);
    malformed(&wire::pack(env, &body));
}

#[test]
fn rejects_numeric_enum_tags() {
    let (mut env, _) = wire::unpack(&fixture());
    let pos = env
        .body
        .0
        .windows(6)
        .position(|window| window == b"\xa5Helix")
        .unwrap();
    env.body.0.splice(pos..pos + 6, [12]);
    malformed(&wire::frame(&env));
}

#[test]
fn rejects_duplicate_keys_in_retained_context_maps() {
    let (env, mut body) = wire::unpack(&fixture());
    body["context"]["reactor_links"] = json!([[123, [456]], [123, [789]]]);
    assert_eq!(
        TypedRecordContainer::from_bytes(&wire::pack(env, &body))
            .map(|_| ())
            .expect_err("expected duplicate rejection")
            .kind,
        StorageErrorKind::DuplicateIdentity
    );
}
