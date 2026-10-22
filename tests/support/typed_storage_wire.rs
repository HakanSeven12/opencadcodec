//! Malformed-input helpers only: JSON values never participate in production storage.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct Bytes(pub Vec<u8>);
impl Serialize for Bytes {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}
impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct V;
        impl serde::de::Visitor<'_> for V {
            type Value = Bytes;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("binary bytes")
            }
            fn visit_bytes<E: serde::de::Error>(self, b: &[u8]) -> Result<Bytes, E> {
                Ok(Bytes(b.to_vec()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, b: Vec<u8>) -> Result<Bytes, E> {
                Ok(Bytes(b))
            }
        }
        de.deserialize_byte_buf(V)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub kind: String,
    pub writer_codec_version: String,
    pub namespace: String,
    pub capture_report: Value,
    pub body: Bytes,
}
pub fn unpack(bytes: &[u8]) -> (Envelope, Value) {
    let env: Envelope = rmp_serde::from_slice(&bytes[12..]).unwrap();
    let body = rmp_serde::from_slice(&env.body.0).unwrap();
    (env, body)
}
pub fn pack(mut env: Envelope, body: &Value) -> Vec<u8> {
    env.body = Bytes(rmp_serde::to_vec_named(body).unwrap());
    frame(&env)
}
pub fn frame(env: &Envelope) -> Vec<u8> {
    let mut bytes = b"OCCTSNAP\0\0\0\x01".to_vec();
    bytes.extend_from_slice(&rmp_serde::to_vec_named(&env).unwrap());
    bytes
}
