use opencadcodec::entities::{solid3d::Solid3D, EntityType};
use opencadcodec::objects::{
    SolidHistoryBoolean, SolidHistoryBox, SolidHistoryBrep, SolidHistoryFillet,
    SolidHistoryNodeBase, SolidHistoryOperation,
};
use opencadcodec::types::DxfVersion;
use opencadcodec::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};
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
    assert_eq!(operations[0].base().unwrap().eval.parent_id, SolidHistoryNodeBase::ROOT_PARENT);
    // Nodes are linked through the evaluation graph; every node stores the
    // root parent id.
    assert_eq!(operations[1].base().unwrap().eval.parent_id, SolidHistoryNodeBase::ROOT_PARENT);
    assert!(document.solid_history_graph(entity).unwrap().evaluation_graph.is_some());
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
    assert_eq!(operations[0].base().unwrap().eval.parent_id, SolidHistoryNodeBase::ROOT_PARENT);
    assert_eq!(operations[1].base().unwrap().eval.parent_id, SolidHistoryNodeBase::ROOT_PARENT);
    assert!(matches!(
        &operations[0],
        SolidHistoryOperation::Box(value) if value.length == 8.0
    ));
}

#[test]
fn binary_brep_history_survives_r2018_dwg_roundtrip() {
    let sat = opencadcodec::entities::acis::primitives::build_planar_body(
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
    let sab = opencadcodec::SabWriter::write(&sat);
    let operation = SolidHistoryOperation::Brep(SolidHistoryBrep {
        base: SolidHistoryNodeBase::new(1),
        acis_data: opencadcodec::entities::AcisData::from_sab(sab.clone()),
        ..SolidHistoryBrep::default()
    });
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, operation).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let operations = roundtrip.solid_history_operations(entity).unwrap();

    assert_eq!(operations.len(), 1);
    let SolidHistoryOperation::Brep(value) = &operations[0] else {
        panic!("history operation changed type: {:?}", operations[0]);
    };
    assert_eq!(value.acis_data.sab_data, sab);
}

#[test]
fn a_boolean_keeps_both_solids_histories_through_dwg() {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let base = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    let tool = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(base, box_step(1)).unwrap();
    document.append_solid_history(base, fillet_step()).unwrap();
    document.create_solid_history(tool, box_step(1)).unwrap();
    document
        .merge_solid_history_boolean(base, tool, SolidHistoryBoolean::SUBTRACT)
        .unwrap();
    assert!(document.solid_history_graph(tool).is_none());

    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let tree = roundtrip.solid_history_tree(base).unwrap();
    let SolidHistoryOperation::Boolean(boolean) = &tree.operation else {
        panic!("the active step is not the boolean: {:?}", tree.operation);
    };
    assert_eq!(boolean.operation, SolidHistoryBoolean::SUBTRACT);
    assert_eq!(tree.operands.len(), 2);
    assert!(matches!(tree.operands[0].operation, SolidHistoryOperation::Fillet(_)));
    assert!(matches!(tree.operands[1].operation, SolidHistoryOperation::Box(_)));
    assert_eq!(boolean.first_operand, tree.operands[0].operation.base().unwrap().node_id());
    assert_eq!(boolean.second_operand, tree.operands[1].operation.base().unwrap().node_id());
    // The root may name the active node by its step id.
    assert_eq!(boolean.base.step_id, boolean.base.node_id());

    // Every node is reachable and named once.
    let mut ids = vec![tree.operation.base().unwrap().node_id()];
    ids.extend(tree.operands.iter().flat_map(|operand| {
        std::iter::once(operand.operation.base().unwrap().node_id()).chain(
            operand.operands.iter().map(|inner| inner.operation.base().unwrap().node_id()),
        )
    }));
    let count = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), count, "node ids collide: {ids:?}");

    // The tool's box is a step of the composite and can be edited.
    let mut replacement = tree.operands[1].operation.clone();
    if let SolidHistoryOperation::Box(value) = &mut replacement {
        value.length = 9.0;
    }
    let mut roundtrip = roundtrip;
    roundtrip.update_solid_history_step(base, replacement).unwrap();
    let tree = roundtrip.solid_history_tree(base).unwrap();
    assert!(matches!(
        &tree.operands[1].operation,
        SolidHistoryOperation::Box(value) if value.length == 9.0
    ));

    let dxf = DxfWriter::new(&roundtrip).write_to_vec().unwrap();
    let dxf = DxfReader::from_reader(Cursor::new(dxf)).unwrap().read().unwrap();
    assert_eq!(dxf.solid_history_tree(base).unwrap(), tree);
}
