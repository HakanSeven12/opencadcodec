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
    // The authored genus: every node's parent id is the root sentinel â€”
    // the linkage lives in the evaluation graph's edges, not in parent
    // ids (the reference application stores ROOT_PARENT on every
    // history node and chains through ACAD_EVALUATION_GRAPH).
    for operation in &operations {
        assert_eq!(
            operation.base().unwrap().eval.parent_id,
            SolidHistoryNodeBase::ROOT_PARENT
        );
    }
    // The interposed evaluation graph carries the chain: two nodes,
    // one edge linking box -> fillet, the fillet node active.
    let graph = document.solid_history_graph(entity).unwrap();
    let evaluation = graph.evaluation_graph.expect("an evaluation graph");
    match document.objects.get(&evaluation) {
        Some(acadrust::objects::ObjectType::DynamicBlock(value)) => match &value.data {
            acadrust::objects::DynamicBlockData::EvaluationGraph(graph) => {
                assert_eq!(graph.nodes.len(), 2);
                assert_eq!(graph.edges.len(), 1);
                assert_eq!(graph.edges[0].source_node, graph.nodes[0].id);
                assert_eq!(graph.edges[0].destination_node, graph.nodes[1].id);
                // The active (latest) node has no outgoing edge; the
                // first node's outgoing slots name the linking edge.
                assert_eq!(graph.nodes[1].node_data[2], -1);
                assert_eq!(graph.nodes[0].node_data[2], graph.edges[0].id);
            }
            other => panic!("expected an evaluation graph, got {other:?}"),
        },
        _ => panic!("the evaluation graph object is missing"),
    }
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
    // The bogus parent id in the replacement is overwritten with the
    // node's own identity â€” in the authored genus that is the root
    // sentinel on every node; the linkage itself lives in the
    // evaluation graph, untouched by the step replacement.
    assert_eq!(
        operations[0].base().unwrap().eval.parent_id,
        SolidHistoryNodeBase::ROOT_PARENT
    );
    assert_eq!(
        operations[1].base().unwrap().eval.parent_id,
        SolidHistoryNodeBase::ROOT_PARENT
    );
    assert!(matches!(
        &operations[0],
        SolidHistoryOperation::Box(value) if value.length == 8.0
    ));
}

#[test]
fn dwg_save_preserves_constructed_history_trees() {
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

    // The constructed tree survives the save (2026-10-04: the authored
    // genus â€” evaluation graph interposed, typed node arms â€” is
    // loader-proven by the OCS 2026.40 release; the constructed-tree
    // elide this test once pinned was the regression that erased the
    // property panel's construction parameters on reload).
    assert!(matches!(
        roundtrip.get_entity(entity),
        Some(EntityType::Solid3D(_))
    ));
    let operations = roundtrip
        .solid_history_operations(entity)
        .expect("the re-read tree must resolve");
    assert_eq!(operations.len(), 1);
    assert!(matches!(operations[0], SolidHistoryOperation::Brep(_)));
}

#[test]
fn dwg_save_writes_the_history_pointer_real() {
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
    // The history soft-pointer is written REAL and resolves to the
    // re-read tree's root â€” the authored genus (2026-10-04). The NULL
    // this test once pinned was the elide's pointer-nulling half, the
    // regression that erased the property panel's construction
    // parameters on reload.
    match roundtrip.get_entity(entity) {
        Some(EntityType::Solid3D(s)) => {
            let history = s
                .history_handle
                .expect("the constructed tree's root pointer must be written real");
            assert_ne!(history.value(), 0);
            assert!(roundtrip.objects.contains_key(&history));
            let graph = roundtrip
                .solid_history_graph(entity)
                .expect("the re-read tree must resolve through the pointer");
            assert_eq!(graph.root, history);
            assert!(graph.evaluation_graph.is_some());
            assert_eq!(graph.nodes.len(), 1);
        }
        other => panic!("expected Solid3D, got {other:?}"),
    }
    let operations = roundtrip
        .solid_history_operations(entity)
        .expect("the re-read tree must resolve");
    assert_eq!(operations.len(), 1);
    assert!(matches!(operations[0], SolidHistoryOperation::Brep(_)));
}

/// The authored node genus: the transform translation names the solid's
/// world centre (the local primitive hangs centred on the frame origin),
/// while the crate's hosts build base-at-origin frames. A constructed
/// cylinder whose host frame sits at the base centre (10,5,2) with
/// height 10 must round-trip back to that exact base-at-origin frame —
/// the writer's centre shift and the reader's un-shift pair — and the
/// pair is stable across successive round trips.
#[test]
fn primitive_nodes_round_trip_the_base_at_origin_frame() {
    let cylinder = || {
        let mut base = SolidHistoryNodeBase::new(1);
        base.transform[12] = 10.0;
        base.transform[13] = 5.0;
        base.transform[14] = 2.0;
        SolidHistoryOperation::Cylinder(acadrust::objects::SolidHistoryCylinder {
            base,
            height: 10.0,
            major_radius: 5.0,
            minor_radius: 5.0,
            x_radius: 5.0,
            ..Default::default()
        })
    };

    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, cylinder()).unwrap();

    let mut current = DwgWriter::write_to_vec(&document).unwrap();
    for generation in 0..3 {
        let roundtrip = DwgReader::from_stream(Cursor::new(current)).read().unwrap();
        let base = roundtrip
            .solid_history_operations(entity)
            .expect("the tree survives every generation")
            .remove(0)
            .base()
            .unwrap()
            .clone();
        // The host frame: base centre (10,5,2), identity rotation —
        // column-major glam convention, translation at [12,13,14].
        assert_eq!(base.transform[12], 10.0, "generation {generation}");
        assert_eq!(base.transform[13], 5.0, "generation {generation}");
        assert_eq!(base.transform[14], 2.0, "generation {generation}");
        assert_eq!(base.transform[0], 1.0, "generation {generation}");
        assert_eq!(base.transform[5], 1.0, "generation {generation}");
        current = DwgWriter::write_to_vec(&roundtrip).unwrap();
    }
}
