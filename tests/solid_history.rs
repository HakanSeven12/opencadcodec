use acadrust::entities::{solid3d::Solid3D, EntityType};
use acadrust::objects::{
    SolidHistoryBox, SolidHistoryBrep, SolidHistoryFillet, SolidHistoryNodeBase,
    SolidHistoryOperation,
};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

fn box_step(step_id: i32) -> SolidHistoryOperation {
    SolidHistoryOperation::Box(SolidHistoryBox {
        base: SolidHistoryNodeBase::new(step_id),
        length: 2.0,
        width: 3.0,
        height: 4.0,
        ..SolidHistoryBox::default()
    })
}

fn fillet_step() -> SolidHistoryOperation {
    SolidHistoryOperation::Fillet(SolidHistoryFillet {
        base: SolidHistoryNodeBase::new(0),
        radii: vec![0.25],
        ..SolidHistoryFillet::default()
    })
}

#[test]
fn appended_history_is_returned_root_to_active() {
    let mut document = CadDocument::new();
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, box_step(1)).unwrap();
    document
        .append_solid_history(entity, fillet_step())
        .unwrap();

    let operations = document.solid_history_operations(entity).unwrap();
    assert_eq!(operations.len(), 2);
    assert!(matches!(operations[0], SolidHistoryOperation::Box(_)));
    assert!(matches!(operations[1], SolidHistoryOperation::Fillet(_)));
    assert_eq!(operations[0].base().unwrap().eval.parent_id, 0);
    assert_eq!(operations[1].base().unwrap().eval.parent_id, 1);
}

#[test]
fn updating_a_step_preserves_its_graph_identity() {
    let mut document = CadDocument::new();
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, box_step(1)).unwrap();
    document
        .append_solid_history(entity, fillet_step())
        .unwrap();

    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let base = replacement.base_mut().unwrap();
    base.eval.parent_id = 99;
    if let SolidHistoryOperation::Box(value) = &mut replacement {
        value.length = 8.0;
    }
    document
        .update_solid_history_step(entity, replacement)
        .unwrap();

    let operations = document.solid_history_operations(entity).unwrap();
    assert_eq!(operations[0].base().unwrap().eval.parent_id, 0);
    assert_eq!(operations[1].base().unwrap().eval.parent_id, 1);
    assert!(matches!(
        &operations[0],
        SolidHistoryOperation::Box(value) if value.length == 8.0
    ));
}

#[test]
fn dwg_save_elides_sh_history_records_for_strict_loaders() {
    let sat = acadrust::entities::acis::primitives::build_planar_body(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        &[
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![3, 7, 6, 2],
            vec![1, 2, 6, 5],
            vec![0, 4, 7, 3],
        ],
    )
    .unwrap();
    let sab = acadrust::SabWriter::write(&sat);
    let operation = SolidHistoryOperation::Brep(SolidHistoryBrep {
        base: SolidHistoryNodeBase::new(1),
        acis_data: acadrust::entities::AcisData::from_sab(sab.clone()),
        ..SolidHistoryBrep::default()
    });
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, operation).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();

    // The catch-all SH class records are elided at save: their true DWG
    // layouts are undocumented, and class-numbered catch-all records make
    // strict readers refuse the WHOLE file (BricsCAD: "Cannot open file:
    // Object improperly read: <AcDbShExtrusion>"). The modeler data
    // lives in the entity's ACIS, so the solid itself survives while the
    // parametric tree is gone by design.
    assert!(matches!(
        roundtrip.get_entity(entity),
        Some(EntityType::Solid3D(_))
    ));
    assert!(roundtrip
        .solid_history_operations(entity)
        .map_or(true, |operations| operations.is_empty()));
}

#[test]
fn dwg_save_elides_constructed_history_trees_entirely() {
    let sat = acadrust::entities::acis::primitives::build_box(
        [0.0, 0.0, 0.0],
        2.0,
        3.0,
        4.0,
    );
    let sab = acadrust::SabWriter::write(&sat);
    let operation = SolidHistoryOperation::Brep(SolidHistoryBrep {
        base: SolidHistoryNodeBase::new(1),
        acis_data: acadrust::entities::AcisData::from_sab(sab.clone()),
        ..SolidHistoryBrep::default()
    });
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, operation).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();

    // The solid survives (SAT self-contained).
    assert!(matches!(
        roundtrip.get_entity(entity),
        Some(EntityType::Solid3D(_))
    ));
    // Constructed-tree verdict (2026-09-28 cylinder audit): trees
    // assembled by the factory — whose genus (payload owner duplicating
    // the ownerhandle, a node id referencing an elided class, the 1/0
    // version trio, no eval-graph interposition) no strict loader
    // accepts ("Duplicate ownership of reference" dragging the solid
    // out with it) — elide in EVERY class until a constructed probe
    // passes a strict loader. The discriminating flag is
    // DynamicBlockObject::captured (set by the DWG reader); the
    // history soft-pointer is written NULL rather than dangling.
    match roundtrip.get_entity(entity) {
        Some(EntityType::Solid3D(s)) => {
            assert!(
                s.history_handle.map_or(true, |h| h.value() == 0),
                "the constructed ACSH_HISTORY_CLASS root pointer must be \
                 written NULL (the constructed tree elides): got {:?}",
                s.history_handle
            );
        }
        other => panic!("expected Solid3D, got {other:?}"),
    }
    assert!(roundtrip
        .solid_history_operations(entity)
        .map_or(true, |operations| operations.is_empty()));
}
