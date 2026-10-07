//! Drawing properties (DWGPROPS) survive a DXF write and read. R2004+ files
//! carry them as header variables: `$TITLE` .. `$REVISIONNUMBER` right after
//! `$DWGCODEPAGE`, the custom properties as `$CUSTOMPROPERTYTAG` /
//! `$CUSTOMPROPERTY` pairs after `$LASTSAVEDBY`, and `$HYPERLINKBASE`.

use std::io::Cursor;

use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DxfReader, DxfWriter};

const PROPERTY_VARIABLES: [&str; 10] = [
    "$TITLE",
    "$SUBJECT",
    "$AUTHOR",
    "$KEYWORDS",
    "$COMMENTS",
    "$LASTSAVEDBY",
    "$REVISIONNUMBER",
    "$CUSTOMPROPERTYTAG",
    "$CUSTOMPROPERTY",
    "$HYPERLINKBASE",
];

fn document_with_properties(version: DxfVersion) -> CadDocument {
    let mut document = CadDocument::with_version(version);
    let info = &mut document.summary_info;
    info.title = "Kanalbau Hauptstra\u{df}e".into();
    info.subject = "Ausf\u{fc}hrungsplanung".into();
    info.author = "Planungsb\u{fc}ro".into();
    info.keywords = "Kanal; Stra\u{df}enbau".into();
    info.comments = "Stand 07.10.2026".into();
    info.last_saved_by = "Bauleitung".into();
    info.revision_number = "3".into();
    info.hyperlink_base = "https://example.com/plaene/".into();
    info.custom_properties = vec![
        ("Plansatz".into(), "Ausf\u{fc}hrung".into()),
        ("H\u{f6}henbezug".into(), "m NHN".into()),
        ("Leer".into(), String::new()),
    ];
    document
}

fn write(document: &CadDocument, binary: bool) -> Vec<u8> {
    let mut writer = DxfWriter::new(document);
    writer.set_binary(binary);
    writer.write_to_vec().expect("DXF write")
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("DXF open")
        .read()
        .expect("DXF read")
}

/// Whether the text DXF holds the header variable `name` (as a whole line).
fn has_variable(text: &str, name: &str) -> bool {
    text.lines().any(|line| line == name)
}

#[test]
fn summary_info_round_trips_through_dxf() {
    for version in [
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let document = document_with_properties(version);
        for binary in [false, true] {
            let read = read(write(&document, binary));
            assert_eq!(
                read.summary_info, document.summary_info,
                "{version:?} binary={binary}"
            );
        }
    }
}

#[test]
fn properties_follow_dwgcodepage_and_custom_ones_follow_lastsavedby() {
    let document = document_with_properties(DxfVersion::AC1032);
    let text = String::from_utf8(write(&document, false)).expect("UTF-8 DXF");
    let lines: Vec<&str> = text.lines().collect();
    let position = |name: &str| {
        lines
            .iter()
            .position(|line| *line == name)
            .unwrap_or_else(|| panic!("{name} missing"))
    };
    let order = [
        "$DWGCODEPAGE",
        "$TITLE",
        "$SUBJECT",
        "$AUTHOR",
        "$KEYWORDS",
        "$COMMENTS",
        "$LASTSAVEDBY",
        "$REVISIONNUMBER",
        "$CUSTOMPROPERTYTAG",
    ];
    for pair in order.windows(2) {
        assert!(
            position(pair[0]) < position(pair[1]),
            "{} before {}",
            pair[0],
            pair[1]
        );
    }
    let count = |name: &str| lines.iter().filter(|line| **line == name).count();
    assert_eq!(
        (count("$CUSTOMPROPERTYTAG"), count("$CUSTOMPROPERTY")),
        (3, 3)
    );
}

#[test]
fn reads_the_header_layout_other_applications_write() {
    // Only the filled standard properties, $LASTSAVEDBY, then the custom
    // pairs, one of them with an empty value.
    let dxf = "  0
SECTION
  2
HEADER
  9
$ACADVER
  1
AC1027
  9
$DWGCODEPAGE
  3
ANSI_1252
  9
$AUTHOR
  1
Planer
  9
$LASTSAVEDBY
  1
Bauleitung
  9
$CUSTOMPROPERTYTAG
  1
Projekt
  9
$CUSTOMPROPERTY
  1

  9
$CUSTOMPROPERTYTAG
  1
Plannummer
  9
$CUSTOMPROPERTY
  1
K-101
  0
ENDSEC
  0
EOF
";
    let read = read(dxf.as_bytes().to_vec());
    let info = &read.summary_info;
    assert_eq!(info.author, "Planer");
    assert_eq!(info.title, "");
    assert_eq!(info.last_saved_by, "Bauleitung");
    assert_eq!(read.header.last_saved_by, "Bauleitung");
    assert_eq!(
        info.custom_properties,
        vec![
            ("Projekt".to_string(), String::new()),
            ("Plannummer".to_string(), "K-101".to_string()),
        ]
    );
}

#[test]
fn empty_summary_info_writes_no_property_variables() {
    let document = CadDocument::with_version(DxfVersion::AC1032);
    let text = String::from_utf8(write(&document, false)).expect("UTF-8 DXF");
    for name in PROPERTY_VARIABLES {
        assert!(!has_variable(&text, name), "{name}");
    }
    assert_eq!(read(text.into_bytes()).summary_info, document.summary_info);
}

#[test]
fn r2000_keeps_only_the_hyperlink_base() {
    let document = document_with_properties(DxfVersion::AC1015);
    let text = String::from_utf8(write(&document, false)).expect("UTF-8 DXF");
    for name in &PROPERTY_VARIABLES[..9] {
        assert!(!has_variable(&text, name), "{name}");
    }
    assert!(has_variable(&text, "$HYPERLINKBASE"));
    assert_eq!(
        read(text.into_bytes()).summary_info.hyperlink_base,
        document.summary_info.hyperlink_base
    );
}
