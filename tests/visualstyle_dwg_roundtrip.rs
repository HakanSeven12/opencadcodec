//! Regression test for AcDbVisualStyle pre-R2010 DWG serialization.
//!
//! The legacy (pre-R2010) VisualStyle writer consumes a 24-element property
//! vector in a fixed order.  The final element (`bd2007_45`) is version-gated:
//! it is only serialized for R2007 and later (gold spec `SINCE R_2007a`).  This
//! test round-trips a VisualStyle through R2000 (23 stored properties, no
//! `bd2007_45`) and R2007 (24 stored properties) and asserts the recovered
//! properties match the version's expected field set.

use std::io::Cursor;

use opencadcodec::objects::{
    ObjectType, VisualStyle, VisualStyleProperty, VisualStylePropertyValue,
};
use opencadcodec::types::{Color, DxfVersion};
use opencadcodec::{CadDocument, DwgReader, DwgWriter};

/// Build a VisualStyle with the full 24-element legacy property bag.
fn make_style(doc: &mut CadDocument) -> opencadcodec::types::Handle {
    let mut style = VisualStyle::new();
    style.handle = doc.allocate_handle();
    let style_handle = style.handle;
    style.owner = doc.header.named_objects_dict_handle;
    style.description = "TestStyle".to_string();
    style.style_type = 1;
    style.face_lighting_model = 2;
    style.face_lighting_quality = 1;
    style.face_color_mode = 3;
    style.face_modifier = 5;
    style.edge_model = 1;
    style.edge_style = 2;
    style.internal_use_only = true;

    // Pre-R2010 property order, matching the writer in
    // dwg_stream_writers/object_writer/objects.rs::write_visual_style.
    style.properties = vec![
        VisualStyleProperty { value: VisualStylePropertyValue::Double(0.75), enabled: 1 }, // 0 face_opacity
        VisualStyleProperty { value: VisualStylePropertyValue::Double(45.0), enabled: 1 }, // 1 face_specular
        VisualStyleProperty { value: VisualStylePropertyValue::Color(Color::Index(5)), enabled: 1 }, // 2 face_mono_color
        VisualStyleProperty { value: VisualStylePropertyValue::Color(Color::Index(6)), enabled: 1 }, // 3 edge_intersection_color
        VisualStyleProperty { value: VisualStylePropertyValue::Color(Color::Index(7)), enabled: 1 }, // 4 edge_obscured_color
        VisualStyleProperty { value: VisualStylePropertyValue::Long(2), enabled: 1 }, // 5 edge_obscured_ltype
        VisualStyleProperty { value: VisualStylePropertyValue::Double(0.5), enabled: 1 }, // 6 edge_crease_angle
        VisualStyleProperty { value: VisualStylePropertyValue::Long(1), enabled: 1 }, // 7 edge_modifier
        VisualStyleProperty { value: VisualStylePropertyValue::Color(Color::Index(8)), enabled: 1 }, // 8 edge_color
        VisualStyleProperty { value: VisualStylePropertyValue::Double(0.9), enabled: 1 }, // 9 edge_opacity
        VisualStyleProperty { value: VisualStylePropertyValue::Short(4), enabled: 1 }, // 10 edge_width
        VisualStyleProperty { value: VisualStylePropertyValue::Short(12), enabled: 1 }, // 11 edge_overhang
        VisualStyleProperty { value: VisualStylePropertyValue::Long(7), enabled: 1 }, // 12 edge_jitter
        VisualStyleProperty { value: VisualStylePropertyValue::Color(Color::Index(9)), enabled: 1 }, // 13 edge_silhouette_color
        VisualStyleProperty { value: VisualStylePropertyValue::Short(8), enabled: 1 }, // 14 edge_silhouette_width
        VisualStyleProperty { value: VisualStylePropertyValue::Long(1), enabled: 1 }, // 15 edge_halo_gap (written as byte, read as long)
        VisualStyleProperty { value: VisualStylePropertyValue::Short(24), enabled: 1 }, // 16 edge_isolines
        VisualStyleProperty { value: VisualStylePropertyValue::Bool(true), enabled: 1 }, // 17 edge_do_hide_precision
        VisualStyleProperty { value: VisualStylePropertyValue::Short(1), enabled: 1 }, // 18 edge_style_apply
        VisualStyleProperty { value: VisualStylePropertyValue::Short(2), enabled: 1 }, // 19 display_settings
        VisualStyleProperty { value: VisualStylePropertyValue::Long(25), enabled: 1 }, // 20 display_brightness (written/read as BLd)
        VisualStyleProperty { value: VisualStylePropertyValue::Long(1), enabled: 1 }, // 21 display_shadow_type
        VisualStyleProperty { value: VisualStylePropertyValue::Long(0), enabled: 1 }, // 22 reserved long
        VisualStyleProperty { value: VisualStylePropertyValue::Double(0.0), enabled: 1 }, // 23 bd2007_45 (R2007+ only)
    ];

    doc.objects
        .insert(style.handle, ObjectType::VisualStyle(style));
    style_handle
}

/// The first 23 legacy properties are version-independent (R2000â€“R2007).
fn expected_common_properties() -> Vec<VisualStylePropertyValue> {
    vec![
        VisualStylePropertyValue::Double(0.75),
        VisualStylePropertyValue::Double(45.0),
        VisualStylePropertyValue::Color(Color::Index(5)),
        VisualStylePropertyValue::Color(Color::Index(6)),
        VisualStylePropertyValue::Color(Color::Index(7)),
        VisualStylePropertyValue::Long(2),
        VisualStylePropertyValue::Double(0.5),
        VisualStylePropertyValue::Long(1),
        VisualStylePropertyValue::Color(Color::Index(8)),
        VisualStylePropertyValue::Double(0.9),
        VisualStylePropertyValue::Short(4),
        VisualStylePropertyValue::Short(12),
        VisualStylePropertyValue::Long(7),
        VisualStylePropertyValue::Color(Color::Index(9)),
        VisualStylePropertyValue::Short(8),
        VisualStylePropertyValue::Long(1),
        VisualStylePropertyValue::Short(24),
        VisualStylePropertyValue::Bool(true),
        VisualStylePropertyValue::Short(1),
        VisualStylePropertyValue::Short(2),
        VisualStylePropertyValue::Long(25),
        VisualStylePropertyValue::Long(1),
        VisualStylePropertyValue::Long(0),
    ]
}

fn roundtrip(version: DxfVersion) -> (VisualStyle, opencadcodec::types::Handle) {
    let mut doc = CadDocument::with_version(version);
    let style_handle = make_style(&mut doc);
    let bytes = DwgWriter::write_to_vec(&doc).expect("DWG write failed");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let rt_doc = reader.read().expect("DWG read failed");
    let ObjectType::VisualStyle(rt) = rt_doc
        .objects
        .get(&style_handle)
        .expect("VisualStyle missing")
    else {
        panic!("round-tripped object is not a VisualStyle");
    };
    (rt.clone(), style_handle)
}

fn assert_common_fields(rt: &VisualStyle) {
    assert_eq!(rt.description, "TestStyle");
    assert_eq!(rt.style_type, 1);
    assert_eq!(rt.face_lighting_model, 2);
    assert_eq!(rt.face_lighting_quality, 1);
    assert_eq!(rt.face_color_mode, 3);
    assert_eq!(rt.face_modifier, 5);
    assert_eq!(rt.edge_model, 1);
    assert_eq!(rt.edge_style, 2);
    assert!(rt.internal_use_only);
}

#[test]
fn visualstyle_survives_dwg_roundtrip_r2000() {
    let (rt, _) = roundtrip(DxfVersion::AC1015);
    assert_common_fields(&rt);

    // R2000 predates bd2007_45, so only the 23 common properties are stored.
    let expected = expected_common_properties();
    assert_eq!(rt.properties.len(), 23, "expected 23 legacy properties for R2000");
    for (i, (actual, exp)) in rt.properties.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            &actual.value, exp,
            "property {i} mismatch: got {:?}, expected {:?}",
            actual.value, exp
        );
    }
}

#[test]
fn visualstyle_survives_dwg_roundtrip_r2007() {
    let (rt, _) = roundtrip(DxfVersion::AC1021);
    assert_common_fields(&rt);

    // R2007 adds bd2007_45 as the 24th property.
    let mut expected = expected_common_properties();
    expected.push(VisualStylePropertyValue::Double(0.0)); // bd2007_45
    assert_eq!(rt.properties.len(), 24, "expected 24 legacy properties for R2007");
    for (i, (actual, exp)) in rt.properties.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            &actual.value, exp,
            "property {i} mismatch: got {:?}, expected {:?}",
            actual.value, exp
        );
    }
}
