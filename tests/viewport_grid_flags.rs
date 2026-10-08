use opencadcodec::entities::{GridFlags, Viewport};
use opencadcodec::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};
use std::io::Cursor;

const GRID_SHIFT: u32 = 18;
const NON_GRID_FLAGS: i32 = 0x8000 | 0x4000 | 1;

fn drawing(bits: i16) -> CadDocument {
    let mut doc = CadDocument::new();
    let mut view = Viewport::new();
    view.id = 2;
    view.width = 160.;
    view.height = 100.;
    view.grid_flags = GridFlags::from_bits(bits);
    view.status = opencadcodec::entities::ViewportStatusFlags::from_bits(NON_GRID_FLAGS);
    doc.add_entity_to_layout(EntityType::Viewport(view), "Layout1")
        .unwrap();
    doc
}

fn viewport(doc: &CadDocument) -> &Viewport {
    doc.entities()
        .find_map(|e| match e {
            EntityType::Viewport(v) if v.width == 160. => Some(v),
            _ => None,
        })
        .unwrap()
}

#[test]
fn literal_dxf_reads_each_grid_behavior_bit_independently() {
    for bits in 0..16i16 {
        let status = NON_GRID_FLAGS | (i32::from(bits) << GRID_SHIFT);
        let source = format!(
            "0\nSECTION\n2\nENTITIES\n0\nVIEWPORT\n5\n1000\n100\nAcDbEntity\n8\n0\n100\nAcDbViewport\n40\n160\n41\n100\n90\n{status}\n0\nENDSEC\n0\nEOF\n"
        );
        let restored = DxfReader::from_reader(Cursor::new(source.into_bytes()))
            .unwrap()
            .read()
            .unwrap();
        let view = viewport(&restored);
        assert_eq!(view.grid_flags.to_bits(), bits, "grid flags {bits}");
        assert_eq!(view.status.to_bits(), NON_GRID_FLAGS);
    }
}

#[test]
fn dxf_writer_places_grid_behavior_in_group_90() {
    for bits in 0..16i16 {
        let doc = drawing(bits);
        let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
        let lines: Vec<_> = text.lines().collect();
        let (pairs, remainder) = lines.as_chunks::<2>();
        assert!(remainder.is_empty());
        let start = pairs
            .iter()
            .enumerate()
            .find_map(|(i, p)| {
                (p[0].trim() == "0"
                    && p[1].trim() == "VIEWPORT"
                    && pairs[i + 1..]
                        .iter()
                        .take_while(|p| p[0].trim() != "0")
                        .any(|p| p[0].trim() == "40" && p[1].trim().parse::<f64>() == Ok(160.)))
                .then_some(i)
            })
            .unwrap();
        let status: i32 = pairs[start + 1..]
            .iter()
            .take_while(|p| p[0].trim() != "0")
            .find(|p| p[0].trim() == "90")
            .unwrap()[1]
            .trim()
            .parse()
            .unwrap();
        assert_eq!(status, NON_GRID_FLAGS | (i32::from(bits) << GRID_SHIFT));
    }
}

#[test]
fn grid_behavior_survives_dxf_and_dwg_readback() {
    for bits in 0..16i16 {
        let doc = drawing(bits);
        let dxf = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
            .unwrap()
            .read()
            .unwrap();
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        for restored in [dxf, dwg] {
            let view = viewport(&restored);
            assert_eq!(view.grid_flags.to_bits(), bits, "grid flags {bits}");
            assert_eq!(view.status.to_bits(), NON_GRID_FLAGS);
        }
    }
}
