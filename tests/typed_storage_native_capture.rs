//! Capturing production-reader output qualifies storage, not CAD reconstruction.
#![cfg(all(feature = "typed-storage", not(target_arch = "wasm32")))]
use acadrust::{
    entities::{EntityType, Line},
    storage::{StorageNamespace, TypedRecordContainer},
    types::{DxfVersion, Vector3},
    xdata::{ExtendedDataRecord, XDataValue},
    CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter,
};
use std::{
    fs,
    io::Cursor,
    time::{SystemTime, UNIX_EPOCH},
};

fn source() -> CadDocument {
    let mut source = CadDocument::with_version(DxfVersion::AC1032);
    source.summary_info.title = "Typed storage fixture".into();
    source.header.create_ellipse_as_polyline = true;
    let h = source
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(7.0, 8.0, 9.0),
        )))
        .unwrap();
    source
        .get_entity_mut(h)
        .unwrap()
        .common_mut()
        .extended_data
        .add_record(ExtendedDataRecord {
            application_name: "ACAD".into(),
            values: vec![
                XDataValue::String("Authored XDATA".into()),
                XDataValue::Integer16(37),
            ],
        });
    source
}
#[test]
fn production_ascii_binary_dxf_and_ac1032_dwg_sources_survive_a_storage_file() {
    let original = source();
    let ascii = DxfWriter::new(&original).write_to_vec().unwrap();
    let binary = DxfWriter::new_binary(&original).write_to_vec().unwrap();
    let dwg = DwgWriter::write_to_vec(&original).unwrap();
    let sources = [
        DxfReader::from_reader(Cursor::new(ascii))
            .unwrap()
            .read()
            .unwrap(),
        DxfReader::from_reader(Cursor::new(binary))
            .unwrap()
            .read()
            .unwrap(),
        DwgReader::from_stream(Cursor::new(dwg)).read().unwrap(),
    ];
    for (index, source) in sources.into_iter().enumerate() {
        // The storage boundary starts at production-reader output. The authored
        // true value is tested independently in typed_storage_records; DXF IO
        // need not support every DWG header variable to qualify typed storage.
        let expected_pellipse = source.header.create_ellipse_as_polyline;
        eprintln!("native reader fixture {index}: PELLIPSE={expected_pellipse}");
        if index == 2 {
            assert!(expected_pellipse, "AC1032 DWG retains PELLIPSE");
        }
        let captured = TypedRecordContainer::capture_document(
            &source,
            StorageNamespace::new(format!("native-{index}")).unwrap(),
        )
        .unwrap();
        let bytes = captured.container.to_bytes().unwrap();
        drop(source);
        drop(captured);
        let path = std::env::temp_dir().join(format!(
            "typed-storage-{}-{index}-{}.occts",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, &bytes).unwrap();
        drop(file);
        let read = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let container = TypedRecordContainer::from_bytes(&read).unwrap();
        let views = container.records();
        let line = views
            .entities()
            .iter()
            .find_map(|e| {
                if let EntityType::Line(line) = e {
                    Some(line)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(line.start, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(line.end, Vector3::new(7.0, 8.0, 9.0));
        assert_eq!(
            line.common.extended_data.get_record("ACAD").unwrap().values,
            vec![
                XDataValue::String("Authored XDATA".into()),
                XDataValue::Integer16(37)
            ]
        );
        assert_eq!(
            views.document_context().summary_info.title,
            "Typed storage fixture"
        );
        assert_eq!(
            views.document_context().header.create_ellipse_as_polyline,
            expected_pellipse,
            "reader fixture {index}"
        );
    }
}
