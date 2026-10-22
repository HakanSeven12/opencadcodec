use super::{
    records::RecordBody, CaptureReport, StorageError, StorageErrorKind, StorageMetadata,
    StorageNamespace, TypedRecordContainer,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::Cursor;

const MAGIC: &[u8; 8] = b"OCCTSNAP";
const VERSION: u32 = 1;

struct ByteString(Vec<u8>);
impl Serialize for ByteString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}
impl<'de> Deserialize<'de> for ByteString {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Bytes;
        impl serde::de::Visitor<'_> for Bytes {
            type Value = ByteString;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a MessagePack binary body")
            }
            fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
                Ok(ByteString(value.to_vec()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                Ok(ByteString(value))
            }
        }
        de.deserialize_byte_buf(Bytes)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    kind: String,
    writer_codec_version: String,
    namespace: String,
    capture_report: CaptureReport,
    body: ByteString,
}

fn named_bytes<T: Serialize>(value: &T, location: &str) -> Result<Vec<u8>, StorageError> {
    let mut bytes = Vec::new();
    value
        .serialize(
            &mut rmp_serde::Serializer::new(&mut bytes)
                .with_struct_map()
                .with_binary(),
        )
        .map_err(|error| StorageError::malformed(location, error))?;
    Ok(bytes)
}
fn strict_decode<T: DeserializeOwned>(bytes: &[u8], location: &str) -> Result<T, StorageError> {
    let mut de = rmp_serde::Deserializer::new(Cursor::new(bytes)).with_binary();
    let mut ignored = Vec::new();
    let value = serde_ignored::deserialize(super::strict::Named(&mut de), |path| {
        ignored.push(path.to_string())
    })
    .map_err(|error| StorageError::malformed(location, error))?;
    if !ignored.is_empty() {
        return Err(StorageError::malformed(
            location,
            format!("unknown fields: {}", ignored.join(", ")),
        ));
    }
    if de.get_ref().position() != bytes.len() as u64 {
        return Err(StorageError::malformed(location, "trailing data"));
    }
    Ok(value)
}
pub(super) fn encode_body(body: &RecordBody) -> Result<Vec<u8>, StorageError> {
    body.validate_record_keys()?;
    named_bytes(body, "body")
}
pub(super) fn decode_body(bytes: &[u8]) -> Result<RecordBody, StorageError> {
    let body: RecordBody = strict_decode(bytes, "body")?;
    body.validate_record_keys()?;
    Ok(body)
}
pub(super) fn encode(container: &TypedRecordContainer) -> Result<Vec<u8>, StorageError> {
    let body = ByteString(encode_body(&container.body)?);
    let envelope = Envelope {
        kind: container.metadata.kind.clone(),
        writer_codec_version: container.metadata.writer_codec_version.clone(),
        namespace: container.metadata.namespace.as_str().into(),
        capture_report: container.report.clone(),
        body,
    };
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&VERSION.to_be_bytes());
    bytes.extend_from_slice(&named_bytes(&envelope, "envelope")?);
    Ok(bytes)
}
pub(super) fn decode(bytes: &[u8]) -> Result<TypedRecordContainer, StorageError> {
    if bytes.len() < 12 || &bytes[..8] != MAGIC {
        return Err(StorageError::malformed(
            "envelope",
            "missing magic or truncated header",
        ));
    }
    let version = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(StorageError::new(
            StorageErrorKind::UnsupportedVersion,
            "envelope.version",
            format!("unsupported format version {version}"),
        ));
    }
    let envelope: Envelope = strict_decode(&bytes[12..], "envelope")?;
    if envelope.kind != "typed-records" {
        return Err(StorageError::new(
            StorageErrorKind::UnsupportedKind,
            "envelope.kind",
            envelope.kind,
        ));
    }
    let namespace = StorageNamespace::new(envelope.namespace)?;
    let body = decode_body(&envelope.body.0)?;
    Ok(TypedRecordContainer {
        metadata: StorageMetadata {
            format_version: version,
            kind: envelope.kind,
            namespace,
            writer_codec_version: envelope.writer_codec_version,
        },
        report: envelope.capture_report,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize, Deserialize)]
    struct Floats {
        wide: f64,
        narrow: f32,
    }
    #[test]
    fn named_transport_keeps_f32_and_f64_bits() {
        for wide in [
            0x8000_0000_0000_0000,
            1,
            0x7ff0_0000_0000_0000,
            0x7ff8_0000_0000_0042,
        ] {
            for narrow in [0x8000_0000, 1, 0x7f80_0000, 0x7fc0_0042] {
                let value = Floats {
                    wide: f64::from_bits(wide),
                    narrow: f32::from_bits(narrow),
                };
                let read: Floats =
                    strict_decode(&named_bytes(&value, "test").unwrap(), "test").unwrap();
                assert_eq!(read.wide.to_bits(), wide);
                assert_eq!(read.narrow.to_bits(), narrow);
            }
        }
    }
}
