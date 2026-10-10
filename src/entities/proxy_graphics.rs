//! Typed access to entity proxy-graphics metafiles.
//!
//! The metafile is the ODA "Proxy Entity Graphics" format (ODA spec
//! section 29; the print edition renders its body as images, so this
//! documentation quotes the derived census instead): the entity-common
//! `graphic_data` field starts with `[u32 total_size][u32
//! record_count]` and then holds `record_count` records of
//! `[u32 record_size][u32 record_type][payload]`, all little-endian,
//! where `record_size` counts both the 8-byte header and the payload.
//!
//! Type census derived from four authored specimens (the gold-tree
//! 2018/Leader.dwg AcDbMLeader 564-byte metafile, BricsCAD- and
//! AutoCAD-authored mleader samples, and the AutoCAD-authored
//! gh44-error.dwg):
//!
//! | type | payload | observed role |
//! |------|---------|---------------|
//! | 16   | u32 = 0 | state marker |
//! | 18   | u32 = 0 / 0x7FFF | state word (0x7FFF = 32767, the "byblock/max" toggle) |
//! | 19   | u32 selector | property selector: 0x1389, 0x2711, 0x3A99, 0x3A9A, 1 |
//! | 20   | u32 = 1 | state marker |
//! | 22   | u32 = 0xC0000000 | state marker (reserved max value) |
//! | 23   | u32 = 0 / 0xFFFFFFFF | state marker |
//! | 51   | u32 = 0 | state marker |
//! | 21   | empty  | `FillOff` |
//! | 36   | 6 f64 + UTF-16 text | `UnicodeText` |
//! | 38   | f64 pairs + padding | mleader header block: anchor x/y, unit scalars, style dwords (incl. the record family's text height as an f64), trailing bit-packed dwords |
//! | 6    | u32 n + n×(3 f64) | n-point 3D primitive (leader geometry) |
//! | 7    | u32 n + n×(3 f64) | n-point 3D polyline (the leader line) |
//! | 32   | u32 n + n×([x][y f64] + tail) | point-forward geometry blocks |
//!
//! Unknown records stay opaque so decoding and re-encoding a metafile does not
//! discard primitives or traits that this module does not model yet.

use crate::types::Vector3;

const HEADER_SIZE: usize = 8;
const RECORD_HEADER_SIZE: usize = 8;
const UNICODE_TEXT_FIXED_SIZE: usize = 96;

/// An entity proxy-graphics metafile.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProxyGraphics {
    pub records: Vec<ProxyGraphicRecord>,
}

/// A typed proxy-graphics record.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProxyGraphicRecord {
    /// Type 21: an empty record that disables filling for later primitives.
    FillOff,
    /// Type 36: a single-line UTF-16 text primitive.
    UnicodeText(ProxyUnicodeText),
    /// The 12-byte single-word graphics-state records: a `u32` little-endian
    /// payload on the observed types 16, 18, 19, 20, 22, 23, and 51. Type 19
    /// is the property selector (census: 0x1389, 0x2711, 0x3A99, 0x3A9A, 1);
    /// 18 carries state words (0 and 0x7FFF); the rest are state markers.
    State { record_type: u32, value: u32 },
    /// A record whose payload is not interpreted by this version of acadrust.
    Unknown { record_type: u32, data: Vec<u8> },
}

/// Type-36 Unicode text data.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProxyUnicodeText {
    pub position: Vector3,
    pub normal: Vector3,
    pub direction: Vector3,
    pub height: f64,
    pub width_factor: f64,
    pub oblique_angle: f64,
    pub text: String,
}

impl ProxyGraphics {
    /// Decode a complete proxy-graphics metafile.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let total_size = read_u32(data, 0)? as usize;
        let record_count = read_u32(data, 4)? as usize;
        if total_size < HEADER_SIZE || total_size > data.len() {
            return None;
        }

        let mut records = Vec::with_capacity(record_count.min(1_000_000));
        let mut offset = HEADER_SIZE;
        for _ in 0..record_count {
            let record_size = read_u32(data, offset)? as usize;
            let record_type = read_u32(data, offset + 4)?;
            if record_size < RECORD_HEADER_SIZE {
                return None;
            }
            let record_end = offset.checked_add(record_size)?;
            if record_end > total_size {
                return None;
            }
            let payload = &data[offset + RECORD_HEADER_SIZE..record_end];
            records.push(decode_record(record_type, payload));
            offset = record_end;
        }
        if offset != total_size {
            return None;
        }
        Some(Self { records })
    }

    /// Encode the records as a complete proxy-graphics metafile.
    pub fn encode(&self) -> Option<Vec<u8>> {
        let record_count = u32::try_from(self.records.len()).ok()?;
        let mut output = vec![0u8; HEADER_SIZE];
        output[4..8].copy_from_slice(&record_count.to_le_bytes());
        for record in &self.records {
            let (record_type, payload) = encode_record(record)?;
            let record_size = u32::try_from(RECORD_HEADER_SIZE.checked_add(payload.len())?).ok()?;
            output.extend_from_slice(&record_size.to_le_bytes());
            output.extend_from_slice(&record_type.to_le_bytes());
            output.extend_from_slice(&payload);
        }
        let total_size = u32::try_from(output.len()).ok()?;
        output[0..4].copy_from_slice(&total_size.to_le_bytes());
        Some(output)
    }
}

impl super::EntityCommon {
    /// Decode this entity's cached proxy graphics.
    pub fn proxy_graphics(&self) -> Option<ProxyGraphics> {
        ProxyGraphics::decode(self.graphic_data.as_deref()?)
    }

    /// Replace this entity's cached proxy graphics.
    pub fn set_proxy_graphics(&mut self, graphics: &ProxyGraphics) -> bool {
        let Some(data) = graphics.encode() else {
            return false;
        };
        self.graphic_data = Some(data);
        true
    }
}

fn decode_record(record_type: u32, payload: &[u8]) -> ProxyGraphicRecord {
    match record_type {
        21 if payload.is_empty() => ProxyGraphicRecord::FillOff,
        36 => decode_unicode_text(payload)
            .map(ProxyGraphicRecord::UnicodeText)
            .unwrap_or_else(|| ProxyGraphicRecord::Unknown {
                record_type,
                data: payload.to_vec(),
            }),
        // The observed 12-byte single-word state records (census in the
        // module docs): a 4-byte little-endian u32 payload. Anything else
        // with these types (none observed) falls through as opaque.
        16 | 18 | 19 | 20 | 22 | 23 | 51 if payload.len() == 4 => {
            ProxyGraphicRecord::State {
                record_type,
                value: u32::from_le_bytes(payload.try_into().expect("len checked")),
            }
        }
        _ => ProxyGraphicRecord::Unknown {
            record_type,
            data: payload.to_vec(),
        },
    }
}

fn encode_record(record: &ProxyGraphicRecord) -> Option<(u32, Vec<u8>)> {
    match record {
        ProxyGraphicRecord::FillOff => Some((21, Vec::new())),
        ProxyGraphicRecord::UnicodeText(text) => {
            let mut data = Vec::with_capacity(UNICODE_TEXT_FIXED_SIZE + text.text.len() * 2 + 4);
            write_vector(&mut data, text.position);
            write_vector(&mut data, text.normal);
            write_vector(&mut data, text.direction);
            data.extend_from_slice(&text.height.to_le_bytes());
            data.extend_from_slice(&text.width_factor.to_le_bytes());
            data.extend_from_slice(&text.oblique_angle.to_le_bytes());
            for value in text.text.encode_utf16() {
                data.extend_from_slice(&value.to_le_bytes());
            }
            data.extend_from_slice(&0u16.to_le_bytes());
            while (RECORD_HEADER_SIZE + data.len()) % 4 != 0 {
                data.push(0);
            }
            Some((36, data))
        }
        ProxyGraphicRecord::State { record_type, value } => {
            Some((*record_type, value.to_le_bytes().to_vec()))
        }
        ProxyGraphicRecord::Unknown { record_type, data } => Some((*record_type, data.clone())),
    }
}

fn decode_unicode_text(data: &[u8]) -> Option<ProxyUnicodeText> {
    if data.len() < UNICODE_TEXT_FIXED_SIZE + 2 {
        return None;
    }
    let position = read_vector(data, 0)?;
    let normal = read_vector(data, 24)?;
    let direction = read_vector(data, 48)?;
    let height = read_f64(data, 72)?;
    let width_factor = read_f64(data, 80)?;
    let oblique_angle = read_f64(data, 88)?;
    let tail = &data[UNICODE_TEXT_FIXED_SIZE..];
    let terminator = tail.chunks_exact(2).position(|value| value == [0, 0])?;
    let units = tail[..terminator * 2]
        .chunks_exact(2)
        .map(|value| u16::from_le_bytes([value[0], value[1]]))
        .collect::<Vec<_>>();
    let text = String::from_utf16(&units).ok()?;
    Some(ProxyUnicodeText {
        position,
        normal,
        direction,
        height,
        width_factor,
        oblique_angle,
        text,
    })
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let value = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(value.try_into().ok()?))
}

fn read_f64(data: &[u8], offset: usize) -> Option<f64> {
    let value = data.get(offset..offset + 8)?;
    Some(f64::from_le_bytes(value.try_into().ok()?))
}

fn read_vector(data: &[u8], offset: usize) -> Option<Vector3> {
    Some(Vector3::new(
        read_f64(data, offset)?,
        read_f64(data, offset + 8)?,
        read_f64(data, offset + 16)?,
    ))
}

fn write_vector(output: &mut Vec<u8>, value: Vector3) {
    output.extend_from_slice(&value.x.to_le_bytes());
    output.extend_from_slice(&value.y.to_le_bytes());
    output.extend_from_slice(&value.z.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The complete proxy-graphics metafile of the AcDbMLeader record
    /// (handle 0x732) in the gold tree's
    /// `~/work/libredwg/test/test-data/2018/Leader.dwg`, at record
    /// window byte offset 7 (right after the entity's handle + zero
    /// EED marker + graphic-present flag + size word). This is the
    /// campaign's derivation fixture: 564 bytes, 12 records (the
    /// envelope follows at once from `[u32 total][u32 count]`).
    /// Live-file equivalence is reproducible with
    /// `cargo run --bin dump_proxy_graphics -- <gold>/2018/Leader.dwg 732`.
    const GOLD_LEADER_MLEADER_METAFILE: [u8; 564] = [
    0x34, 0x02, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
    0x13, 0x00, 0x00, 0x00, 0x99, 0x3A, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
    0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xD0, 0x00, 0x00, 0x00,
    0x26, 0x00, 0x00, 0x00, 0x35, 0x21, 0xF7, 0x77, 0x91, 0xC1, 0x30, 0x40,
    0x14, 0x1F, 0xC2, 0xF7, 0xFD, 0xE7, 0x2C, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x4C, 0x00, 0x4C, 0x00, 0x4C, 0x00, 0x4C, 0x00,
    0x4C, 0x00, 0x4C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x0A, 0xD7, 0xA3, 0x70, 0x3D, 0x0A, 0xC7, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x00,
    0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x00, 0x00,
    0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00,
    0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x11, 0x27, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0xFF, 0x7F, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x54, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x65, 0xD6, 0xDC, 0x62, 0x3C, 0x33, 0x34, 0x40, 0x45, 0xF3, 0xA2, 0x5B,
    0x97, 0xA8, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x9F, 0x6C, 0xE5, 0x37, 0xB0, 0x5B, 0x34, 0x40, 0xE7, 0xF5, 0x03, 0x39,
    0xDC, 0x79, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xD6, 0x92, 0x69, 0x31, 0xD8, 0x2D, 0x34, 0x40, 0x44, 0x23, 0xCC, 0x67,
    0xD3, 0x8B, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x89, 0x13, 0x00, 0x00,
    0x54, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x9E, 0x34, 0x23, 0x4A, 0x8A, 0x30, 0x34, 0x40, 0x44, 0x8B, 0xB7, 0x61,
    0x35, 0x9A, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x2C, 0x4B, 0x2B, 0xBA, 0xDA, 0x35, 0x32, 0x40, 0xBC, 0x66, 0xA3, 0x72,
    0x12, 0x16, 0x2D, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x11, 0x27, 0x00, 0x00,
    0x54, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x2C, 0x4B, 0x2B, 0xBA, 0xDA, 0x35, 0x32, 0x40, 0xBC, 0x66, 0xA3, 0x72,
    0x12, 0x16, 0x2D, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xD0, 0xBB, 0x68, 0xC4, 0xB1, 0xD9, 0x31, 0x40, 0xBC, 0x66, 0xA3, 0x72,
    0x12, 0x16, 0x2D, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    ];

    #[test]
    fn decodes_and_reencodes_the_gold_derived_metafile() {
        let graphics = ProxyGraphics::decode(&GOLD_LEADER_MLEADER_METAFILE)
            .expect("gold metafile decodes");
        assert_eq!(graphics.records.len(), 12);
        let reencoded = graphics.encode().expect("re-encode yields data");
        assert_eq!(reencoded, GOLD_LEADER_MLEADER_METAFILE.to_vec());
    }

    #[test]
    fn gold_derived_metafile_matches_the_derived_census() {
        let graphics = ProxyGraphics::decode(&GOLD_LEADER_MLEADER_METAFILE)
            .expect("gold metafile decodes");
        // The census record stream: the color context opens with the
        // 0x3A99 selector + a zero state word, then the type-38
        // mleader header (208-byte record: anchor 16.7561/14.4531 +
        // scalars), the 0x2711 and 1 selectors, the 0x7FFF state word,
        // a type-20 marker, the type-7 three-vertex polyline (the
        // leader line), the 0x1389 selector, and two type-32 geometry
        // blocks, the second re-selected by 0x2711.
        use ProxyGraphicRecord as R;
        let expected = [
            (19u32, 0x3A99u32),
            (18, 0),
            (38, u32::MAX), // u32::MAX marks an expected Unknown record
            (19, 0x2711),
            (19, 1),
            (18, 0x7FFF),
            (20, 1),
            (7, u32::MAX),
            (19, 0x1389),
            (32, u32::MAX),
            (19, 0x2711),
            (32, u32::MAX),
        ];
        assert_eq!(graphics.records.len(), expected.len());
        for (record, (record_type, word)) in graphics.records.iter().zip(expected) {
            match record {
                R::State {
                    record_type: ty,
                    value,
                } => {
                    assert_eq!(*ty, record_type);
                    assert_eq!(*value, word);
                }
                R::Unknown { record_type: ty, data } => {
                    assert_eq!(*ty, record_type);
                    assert_eq!(word, u32::MAX);
                    let size = match record_type {
                        38 => 200,
                        7 | 32 => 76,
                        other => panic!("unexpected unknown type {other}"),
                    };
                    assert_eq!(data.len(), size);
                }
                other => panic!("unexpected record in the census: {other:?}"),
            }
        }
    }

    #[test]
    fn state_records_roundtrip() {
        let graphics = ProxyGraphics {
            records: vec![
                ProxyGraphicRecord::FillOff,
                ProxyGraphicRecord::State {
                    record_type: 19,
                    value: 0x3A99,
                },
                ProxyGraphicRecord::State {
                    record_type: 18,
                    value: 0x7FFF,
                },
                ProxyGraphicRecord::State {
                    record_type: 22,
                    value: 0xC000_0000,
                },
            ],
        };
        let encoded = graphics.encode().expect("encode");
        let decoded = ProxyGraphics::decode(&encoded).expect("decode");
        assert_eq!(decoded.records, graphics.records);
    }

    #[test]
    fn odd_length_state_candidate_stays_opaque() {
        // A type-19 record whose payload is not the observed 4 bytes
        // must not be modeled as a State word: it stays Unknown and
        // byte-exact.
        let raw = ProxyGraphics {
            records: vec![ProxyGraphicRecord::Unknown {
                record_type: 19,
                data: vec![0x99, 0x3A, 0x00, 0x00, 0x00],
            }],
        };
        let encoded = raw.encode().expect("encode");
        let decoded = ProxyGraphics::decode(&encoded).expect("decode");
        assert_eq!(decoded.records, raw.records);
    }
}
