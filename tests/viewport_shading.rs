use opencadcodec::entities::Viewport;
use opencadcodec::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};
use std::io::Cursor;

fn drawing(mode: i16) -> CadDocument {
    let mut doc = CadDocument::new();
    let mut view = Viewport::new();
    view.id = 2;
    view.width = 160.;
    view.height = 100.;
    view.shade_plot_mode = mode;
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
fn literal_dxf_reads_viewport_shade_plot_mode() {
    for mode in 0..=3 {
        let source = format!(
            "0\nSECTION\n2\nENTITIES\n0\nVIEWPORT\n5\n1000\n100\nAcDbEntity\n8\n0\n100\nAcDbViewport\n40\n160\n41\n100\n90\n32768\n170\n{mode}\n0\nENDSEC\n0\nEOF\n"
        );
        let doc = DxfReader::from_reader(Cursor::new(source.into_bytes()))
            .unwrap()
            .read()
            .unwrap();
        assert_eq!(viewport(&doc).shade_plot_mode, mode);
    }
}

#[test]
fn dxf_writer_emits_viewport_shade_plot_group_170() {
    for mode in 0..=3 {
        let doc = drawing(mode);
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
        let stored = pairs[start + 1..]
            .iter()
            .take_while(|p| p[0].trim() != "0")
            .find(|p| p[0].trim() == "170")
            .map(|p| p[1].trim().parse::<i16>().unwrap());
        assert_eq!(stored, Some(mode));
    }
}

#[test]
fn viewport_shading_survives_dxf_and_dwg_readback() {
    for mode in 0..=3 {
        let doc = drawing(mode);
        let dxf = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
            .unwrap()
            .read()
            .unwrap();
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        for restored in [dxf, dwg] {
            assert_eq!(viewport(&restored).shade_plot_mode, mode);
        }
    }
}
