use opencadcodec::entities::{Viewport, ViewportStatusFlags};
use opencadcodec::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};
use std::io::Cursor;

const ON: i32 = 0x8000;
const OFF: i32 = 0x20000;

fn drawing(bits: i32, off_screen: bool) -> CadDocument {
    let mut doc = CadDocument::new();
    let mut view = Viewport::new();
    view.id = 2;
    view.width = 160.;
    view.height = 100.;
    view.status = ViewportStatusFlags::from_bits(bits);
    view.off_screen = off_screen;
    doc.add_entity_to_layout(EntityType::Viewport(view), "Layout1")
        .unwrap();
    doc
}

fn viewport(doc: &CadDocument) -> &Viewport {
    doc.entities()
        .find_map(|entity| match entity {
            EntityType::Viewport(view) if view.width == 160. => Some(view),
            _ => None,
        })
        .unwrap()
}

#[test]
fn explicit_off_bit_is_independent_of_legacy_on_and_clipping() {
    for bits in [0, ON, OFF, ON | OFF, ON | OFF | 0x10000 | 0x4000] {
        assert_eq!(ViewportStatusFlags::from_bits(bits).to_bits(), bits);
    }
}

#[test]
fn viewport_visibility_helpers_respect_explicit_off() {
    let mut view = Viewport::new();
    view.status = ViewportStatusFlags::from_bits(ON | OFF);
    assert!(!view.is_on());
    view.turn_on();
    assert!(view.is_on());
    assert_eq!(view.status.to_bits() & (ON | OFF), ON);
    view.turn_off();
    assert!(!view.is_on());
    assert_eq!(view.status.to_bits() & (ON | OFF), OFF);
}

#[test]
fn dxf_status_group_68_respects_explicit_off_and_off_screen() {
    for bits in [0, ON, OFF, ON | OFF] {
        for off_screen in [false, true] {
            let doc = drawing(bits, off_screen);
            let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
            let lines: Vec<_> = text.lines().collect();
            let fields: Vec<_> = lines.chunks_exact(2).collect();
            let start = fields
                .iter()
                .enumerate()
                .find_map(|(index, pair)| {
                    (pair[0].trim() == "0"
                        && pair[1].trim() == "VIEWPORT"
                        && fields[index + 1..]
                            .iter()
                            .take_while(|p| p[0].trim() != "0")
                            .any(|p| p[0].trim() == "40" && p[1].trim().parse::<f64>() == Ok(160.)))
                    .then_some(index)
                })
                .unwrap();
            let view_fields: Vec<_> = fields[start + 1..]
                .iter()
                .take_while(|p| p[0].trim() != "0")
                .collect();
            let group = |code: &str| -> i32 {
                view_fields.iter().find(|p| p[0].trim() == code).unwrap()[1]
                    .trim()
                    .parse()
                    .unwrap()
            };
            let expected = if bits & ON == 0 || bits & OFF != 0 {
                0
            } else if off_screen {
                -1
            } else {
                2
            };
            assert_eq!(
                group("68"),
                expected,
                "bits={bits:x}, off_screen={off_screen}"
            );
            assert_eq!(group("90") & (ON | OFF), bits & (ON | OFF));
        }
    }
}

#[test]
fn explicit_off_survives_dxf_and_dwg_readback() {
    for bits in [OFF, ON | OFF] {
        let doc = drawing(bits, false);
        let dxf = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
            .unwrap()
            .read()
            .unwrap();
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        for restored in [dxf, dwg] {
            let view = viewport(&restored);
            assert_eq!(view.status.to_bits() & (ON | OFF), bits);
            assert!(!view.is_on());
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn old_serialized_status_defaults_explicit_off_to_false() {
    let mut value = serde_json::to_value(ViewportStatusFlags::default_on()).unwrap();
    value.as_object_mut().unwrap().remove("is_off");
    let restored: ViewportStatusFlags = serde_json::from_value(value).unwrap();
    assert_eq!(restored.to_bits() & (ON | OFF), ON);
}
