//! Entities of a layout other than the active one keep their owner through
//! DWG: only the active layout's *Paper_Space is implied by paper-space mode.
use opencadcodec::entities::Line;
use opencadcodec::types::Vector3;
use opencadcodec::{CadDocument, DwgReader, DwgWriter, EntityType};
use std::io::Cursor;

#[test]
fn a_second_layouts_block_marker_and_entities_keep_their_record() {
    let mut source = CadDocument::new();
    source.add_layout("Layout2").unwrap();
    let line = source
        .add_entity_to_layout(
            EntityType::Line(Line::from_points(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(10.0, 0.0, 0.0),
            )),
            "Layout2",
        )
        .unwrap();

    let bytes = DwgWriter::write_to_vec(&source).unwrap();
    let loaded = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let extra = loaded.block_records.get("*Paper_Space0").unwrap();
    let Some(EntityType::Block(marker)) = loaded.get_entity(extra.block_entity_handle) else {
        panic!("the second layout's BLOCK marker is missing");
    };
    assert_eq!(marker.name, extra.name);
    assert_eq!(marker.common.owner_handle, extra.handle);
    assert_eq!(loaded.get_entity(line).unwrap().common().owner_handle, extra.handle);
    assert!(extra.entity_handles.contains(&line));
}
