//! DXF code page ($DWGCODEPAGE) to encoding mapping.
//!
//! Maps DXF code page names to `encoding_rs` encodings, following the same
//! mapping table used by the reference `CadUtils._dxfEncodingMap`.

use encoding_rs::Encoding;

/// Get the `encoding_rs` encoding for a DXF code page string.
///
/// Returns `None` if the encoding is UTF-8 (no transcoding needed) or the
/// code page string is not recognized.
///
/// # Rules
/// - If the DXF version is AC1021 (AutoCAD 2007+) or later, UTF-8 is always
///   used regardless of $DWGCODEPAGE — callers should not call this function.
/// - Otherwise, the code page string (case-insensitive) is looked up in the
///   mapping table.
pub fn encoding_from_code_page(code_page: &str) -> Option<&'static Encoding> {
    match code_page.to_ascii_lowercase().as_str() {
        // Asian encodings. §19 H7g review: the GB2312/ANSI_936 (and the
        // 932/949/950/1361) byte pairs share a CODEC FAMILY but NOT a
        // codepage byte — 31 is GB2312-EUC-CN where 39 is CP936-GBK,
        // 22 is DOS932 where 38 is ANSI_932 (windows-31j), 24 is BIG5
        // where 41 is ANSI_950, 25 is CP949-"korean" where 40 is
        // ANSI_949, 26 is JOHAB where 42 is ANSI_1361. The encoders
        // stay the practical family choice (the families are
        // byte-compatible on the low half); the name/index tables
        // below carry the distinction.
        "gb2312" | "ansi_936" => Some(encoding_rs::GBK),
        "big5" | "ansi_950" | "dos950" => Some(encoding_rs::BIG5),
        "korean" | "ansi_949" | "johab" | "ansi_1361" => Some(encoding_rs::EUC_KR),
        "ansi_932" | "dos932" => Some(encoding_rs::SHIFT_JIS),

        // DOS/OEM code pages
        "dos437" => Some(encoding_rs::IBM866), // closest available in encoding_rs
        "dos850" => Some(encoding_rs::WINDOWS_1252), // Western European
        "dos852" => Some(encoding_rs::WINDOWS_1250), // Central European
        "dos855" | "dos866" => Some(encoding_rs::IBM866), // Cyrillic
        "dos857" => Some(encoding_rs::WINDOWS_1254), // Turkish
        "dos860" => Some(encoding_rs::WINDOWS_1252), // Portuguese
        "dos861" => Some(encoding_rs::WINDOWS_1252), // Icelandic
        "dos863" => Some(encoding_rs::WINDOWS_1252), // Canadian-French
        "dos865" => Some(encoding_rs::WINDOWS_1252), // Nordic
        "dos869" => Some(encoding_rs::WINDOWS_1253), // Greek

        // Windows/ANSI code pages
        "ansi_874" => Some(encoding_rs::WINDOWS_874),
        "ansi_1250" => Some(encoding_rs::WINDOWS_1250),
        "ansi_1251" => Some(encoding_rs::WINDOWS_1251),
        "ansi_1252" => Some(encoding_rs::WINDOWS_1252),
        "ansi_1253" => Some(encoding_rs::WINDOWS_1253),
        "ansi_1254" => Some(encoding_rs::WINDOWS_1254),
        "ansi_1255" => Some(encoding_rs::WINDOWS_1255),
        "ansi_1256" => Some(encoding_rs::WINDOWS_1256),
        "ansi_1257" => Some(encoding_rs::WINDOWS_1257),
        "ansi_1258" => Some(encoding_rs::WINDOWS_1258),

        // ISO encodings
        "iso8859-1" | "iso_8859-1" => Some(encoding_rs::WINDOWS_1252),
        "iso8859-2" | "iso_8859-2" => Some(encoding_rs::ISO_8859_2),
        "iso8859-3" | "iso_8859-3" => Some(encoding_rs::ISO_8859_3),
        "iso8859-4" | "iso_8859-4" => Some(encoding_rs::ISO_8859_4),
        "iso8859-5" | "iso_8859-5" => Some(encoding_rs::ISO_8859_5),
        "iso8859-6" | "iso_8859-6" => Some(encoding_rs::ISO_8859_6),
        "iso8859-7" | "iso_8859-7" => Some(encoding_rs::ISO_8859_7),
        "iso8859-8" | "iso_8859-8" => Some(encoding_rs::ISO_8859_8),
        "iso8859-9" | "iso_8859-9" => Some(encoding_rs::WINDOWS_1254),
        "iso8859-10" | "iso_8859-10" => Some(encoding_rs::ISO_8859_10),
        "iso8859-13" | "iso_8859-13" => Some(encoding_rs::ISO_8859_13),
        "iso8859-14" | "iso_8859-14" => Some(encoding_rs::ISO_8859_14),
        "iso8859-15" | "iso_8859-15" => Some(encoding_rs::ISO_8859_15),

        // KOI8-R (Russian)
        "koi8-r" => Some(encoding_rs::KOI8_R),
        "koi8-u" => Some(encoding_rs::KOI8_U),

        // ASCII / UTF-8 / no fallback needed
        "ascii" | "utf-8" | "utf8" | "unicode" => None,

        // Default: Windows-1252 (most common DXF fallback)
        _ => Some(encoding_rs::WINDOWS_1252),
    }
}

/// Resolve the compact code-page index stored in DWG file metadata.
pub fn dwg_code_page_name(index: u16) -> &'static str {
    match index {
        1 => "ASCII",
        2 => "ISO8859-1",
        3 => "ISO8859-2",
        4 => "ISO8859-3",
        5 => "ISO8859-4",
        6 => "ISO8859-5",
        7 => "ISO8859-6",
        8 => "ISO8859-7",
        9 => "ISO8859-8",
        10 => "ISO8859-9",
        11 => "DOS437",
        12 => "DOS850",
        13 => "DOS852",
        14 => "DOS855",
        15 => "DOS857",
        16 => "DOS860",
        17 => "DOS861",
        18 => "DOS863",
        19 => "DOS864",
        20 => "DOS865",
        21 => "DOS869",
        22 => "DOS932",
        23 => "MAC-ROMAN",
        24 => "BIG5",
        25 => "KOREAN",
        26 => "JOHAB",
        27 => "DOS866",
        28 => "ANSI_1250",
        29 => "ANSI_1251",
        30 => "ANSI_1252",
        31 => "GB2312",
        32 => "ANSI_1253",
        33 => "ANSI_1254",
        34 => "ANSI_1255",
        35 => "ANSI_1256",
        36 => "ANSI_1257",
        37 => "ANSI_874",
        // The 38..42 windows-of-asia family (§19 H7g review): each pair
        // shares a codec family with its DOS-era sibling above but is a
        // distinct codepage byte.
        38 => "ANSI_932",
        39 => "ANSI_936",
        40 => "ANSI_949",
        41 => "ANSI_950",
        42 => "ANSI_1361",
        43 => "UTF-8",
        44 => "ANSI_1258",
        _ => "ANSI_1252",
    }
}

/// Convert a DXF `$DWGCODEPAGE` name to the compact DWG metadata index.
pub fn dwg_code_page_index(code_page: &str) -> u16 {
    match code_page.to_ascii_lowercase().as_str() {
        "ascii" => 1,
        "iso8859-1" | "iso_8859-1" => 2,
        "iso8859-2" | "iso_8859-2" => 3,
        "iso8859-3" | "iso_8859-3" => 4,
        "iso8859-4" | "iso_8859-4" => 5,
        "iso8859-5" | "iso_8859-5" => 6,
        "iso8859-6" | "iso_8859-6" => 7,
        "iso8859-7" | "iso_8859-7" => 8,
        "iso8859-8" | "iso_8859-8" => 9,
        "iso8859-9" | "iso_8859-9" => 10,
        "dos437" => 11,
        "dos850" => 12,
        "dos852" => 13,
        "dos855" => 14,
        "dos857" => 15,
        "dos860" => 16,
        "dos861" => 17,
        "dos863" => 18,
        "dos864" => 19,
        "dos865" => 20,
        "dos869" => 21,
        "dos932" => 22,
        "mac-roman" => 23,
        "big5" | "dos950" => 24,
        "korean" => 25,
        "johab" => 26,
        "dos866" => 27,
        "ansi_1250" | "ansi1250" => 28,
        "ansi_1251" | "ansi1251" => 29,
        "ansi_1252" | "ansi1252" => 30,
        "gb2312" => 31,
        // The 38..42 windows-of-asia family (§19 H7g review): distinct
        // codepage bytes — the historical table conflated them with
        // their DOS-era siblings and lost the author's byte on every
        // roundtrip (gh109_1: the author's 39 = ANSI_936 re-emitted as
        // 31 = GB2312).
        "ansi_932" => 38,
        "ansi_936" => 39,
        "ansi_949" => 40,
        "ansi_950" => 41,
        "ansi_1361" => 42,
        "ansi_1253" | "ansi1253" => 32,
        "ansi_1254" | "ansi1254" => 33,
        "ansi_1255" | "ansi1255" => 34,
        "ansi_1256" | "ansi1256" => 35,
        "ansi_1257" | "ansi1257" => 36,
        "ansi_874" => 37,
        "utf-8" | "utf8" | "unicode" => 43,
        "ansi_1258" | "ansi1258" => 44,
        _ => 30,
    }
}

pub fn encoding_from_dwg_code_page(index: u16) -> &'static Encoding {
    encoding_from_code_page(dwg_code_page_name(index)).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Encode a string to a legacy (pre-UTF-16) DWG code page.
///
/// Characters the code page cannot represent are emitted as AutoCAD MIF
/// `\U+XXXX` escapes (astral-plane characters as a surrogate pair of
/// escapes) instead of `encoding_rs`'s HTML `&#NNNNN;` references, which
/// no CAD application understands. Well-formed MIF escapes round-trip
/// through [`decode_mif_escapes`].
/// Encode text into a legacy (pre-R2007) DWG string using the document's
/// code page.
///
/// Characters the code page cannot represent become MIF `\U+XXXX`
/// escapes instead of `&#NNNNN;` references. The authored wire form of a
/// representable non-ASCII char is AUTHOR DATA, not a convention —
/// example_2004's `108\U+00B0` carries the in-band escape while
/// example_2000's identical text carries the raw 0xB0 byte — so a
/// DWG-read record that must re-emit verbatim uses the captured
/// `dwg_wire_text` (the MText reader), never a derived escape rule.
pub fn encode_legacy_string(text: &str, encoding: &'static Encoding) -> Vec<u8> {
    let (encoded, _, unmappable) = encoding.encode(text);
    if !unmappable {
        return encoded.into_owned();
    }
    let mut out = Vec::with_capacity(text.len() + 6);
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        // Every DWG code page is stateless, so per-char encoding is safe.
        let (bytes, _, err) = encoding.encode(ch.encode_utf8(&mut buf));
        if err {
            let mut units = [0u16; 2];
            for unit in ch.encode_utf16(&mut units) {
                out.extend_from_slice(format!("\\U+{:04X}", unit).as_bytes());
            }
        } else {
            out.extend_from_slice(&bytes);
        }
    }
    out
}

/// Decode AutoCAD MIF `\U+XXXX` escapes (exactly four hex digits) into
/// Unicode characters.
///
/// A high-surrogate escape followed by a low-surrogate escape combines into
/// a scalar value. Malformed or unterminated escapes are left as literal
/// text, and invalid code points are dropped — matching the MTEXT
/// formatter's behavior for the same escapes.
pub fn decode_mif_escapes(text: &str) -> String {
    if !text.contains("\\U+") {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let hex_at = |start: usize| -> Option<u16> {
        if start + 4 > len {
            return None;
        }
        let hex: String = chars[start..start + 4].iter().collect();
        u16::from_str_radix(&hex, 16).ok()
    };
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < len {
        if chars[i] == '\\' && i + 7 <= len && chars[i + 1] == 'U' && chars[i + 2] == '+' {
            if let Some(unit) = hex_at(i + 3) {
                i += 7;
                let mut units = [unit, 0];
                let mut count = 1usize;
                // Combine a following low-surrogate escape into one scalar.
                if (0xD800..0xDC00).contains(&unit)
                    && i + 7 <= len
                    && chars[i] == '\\'
                    && chars[i + 1] == 'U'
                    && chars[i + 2] == '+'
                {
                    if let Some(low) = hex_at(i + 3) {
                        if (0xDC00..0xE000).contains(&low) {
                            units[1] = low;
                            count = 2;
                            i += 7;
                        }
                    }
                }
                if let Some(Ok(ch)) = std::char::decode_utf16(units[..count].iter().copied()).next()
                {
                    out.push(ch);
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_1252() {
        let enc = encoding_from_code_page("ANSI_1252");
        assert_eq!(enc, Some(encoding_rs::WINDOWS_1252));
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(
            encoding_from_code_page("ansi_1251"),
            encoding_from_code_page("ANSI_1251")
        );
    }

    #[test]
    fn test_ascii_returns_none() {
        assert_eq!(encoding_from_code_page("ASCII"), None);
    }

    #[test]
    fn test_utf8_returns_none() {
        assert_eq!(encoding_from_code_page("UTF-8"), None);
    }

    #[test]
    fn test_unknown_returns_windows1252() {
        let enc = encoding_from_code_page("SOMETHING_UNKNOWN");
        assert_eq!(enc, Some(encoding_rs::WINDOWS_1252));
    }

    #[test]
    fn test_codepage_index_name_roundtrip() {
        // §19 H7g review: codepage byte → name → byte must round-trip.
        // The historical tables conflated the DOS-era/Windows asian
        // pairs (22|38, 24|41, 25|40, 26|42, 31|39) into one name, so
        // the byte → model-string conversion lost the author's byte on
        // every same-version roundtrip (gh109_1: the author's 39 =
        // ANSI_936 re-emitted as 31 = GB2312; gold's enum pins the
        // distinction, codepages.h: CP_GB2312 = 31, CP_ANSI_936 = 39).
        for byte in [
            22u16, 24, 25, 26, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44,
        ] {
            let name = dwg_code_page_name(byte);
            assert_eq!(
                dwg_code_page_index(name),
                byte,
                "roundtrip failed for byte {byte} ({name})"
            );
        }
        // The new names still resolve an encoder (the codec families).
        for name in ["DOS932", "ANSI_932", "ANSI_950", "ANSI_1361", "ANSI_936"] {
            assert!(
                encoding_from_code_page(name).is_some(),
                "no encoder for {name}"
            );
        }
    }

    #[test]
    fn test_asian_encodings() {
        assert_eq!(encoding_from_code_page("GB2312"), Some(encoding_rs::GBK));
        assert_eq!(encoding_from_code_page("BIG5"), Some(encoding_rs::BIG5));
        assert_eq!(
            encoding_from_code_page("ANSI_932"),
            Some(encoding_rs::SHIFT_JIS)
        );
        assert_eq!(encoding_from_code_page("KOREAN"), Some(encoding_rs::EUC_KR));
    }

    #[test]
    fn test_decode_mif_escapes() {
        assert_eq!(decode_mif_escapes("ab\\U+4E2Dcd"), "ab中cd");
        assert_eq!(decode_mif_escapes("\\U+0041\\U+0042"), "AB");
        // Exactly four hex digits: a fifth character stays literal.
        assert_eq!(decode_mif_escapes("\\U+00412"), "A2");
        // Malformed escapes stay literal.
        assert_eq!(decode_mif_escapes("\\U+GGGG"), "\\U+GGGG");
        assert_eq!(decode_mif_escapes("\\U+4E"), "\\U+4E");
        assert_eq!(decode_mif_escapes("no escapes here"), "no escapes here");
        // Invalid code points are dropped.
        assert_eq!(decode_mif_escapes("\\U+D800"), "");
        // Surrogate pair combines into a scalar.
        assert_eq!(decode_mif_escapes("\\U+D83D\\U+DE00"), "😀");
    }

    #[test]
    fn test_encode_legacy_string_mif_escapes() {
        // Mappable chars encode directly.
        assert_eq!(encode_legacy_string("AB", encoding_rs::WINDOWS_1252), b"AB");
        // Unmappable chars become MIF escapes, not HTML references.
        let encoded = encode_legacy_string("中", encoding_rs::WINDOWS_1252);
        assert_eq!(String::from_utf8(encoded).unwrap(), "\\U+4E2D");
        // Astral-plane chars become a surrogate pair of escapes.
        let encoded = encode_legacy_string("😀", encoding_rs::WINDOWS_1252);
        assert_eq!(String::from_utf8(encoded).unwrap(), "\\U+D83D\\U+DE00");
        // GBK encodes Chinese directly with no escapes.
        assert_eq!(encode_legacy_string("中", encoding_rs::GBK), &[0xD6, 0xD0]);
        // Round-trip through the decoder.
        let encoded = encode_legacy_string("a中b😀c", encoding_rs::WINDOWS_1252);
        let text = String::from_utf8(encoded).unwrap();
        assert_eq!(decode_mif_escapes(&text), "a中b😀c");
    }
}
