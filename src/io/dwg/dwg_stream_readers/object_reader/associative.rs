use crate::entities::solid3d::AcisVersion;
use crate::io::dwg::dwg_stream_readers::merged_reader::DwgMergedReader;
use crate::io::dwg::dwg_version::DwgVersion;
use crate::objects::*;
use crate::types::{DxfVersion, Handle, Vector3};

use super::safe_count;

fn handle(reader: &mut DwgMergedReader) -> Handle {
    Handle::from(reader.read_handle())
}

fn read_handles(reader: &mut DwgMergedReader, count: i32) -> Vec<Handle> {
    let count = safe_count(count);
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        values.push(handle(reader));
    }
    values
}

fn eval_kind(code: i16) -> u8 {
    match code {
        i16::MIN..=-1 | 5 | 105 | 320..=329 | 390..=399 => 6,
        0..=9 | 100..=102 | 300..=309 | 410..=419 | 430..=439 | 470..=479 | 999 | 1000..=1009 => 5,
        10..=37 | 110..=139 | 210..=269 | 1010..=1039 | 1043..=1069 => 0,
        38..=59 | 140..=149 | 460..=469 | 1040..=1042 => 1,
        60..=79 | 170..=179 | 270..=279 | 370..=389 | 400..=409 | 1070 => 3,
        80..=99 | 420..=429 | 440..=459 | 1071 => 2,
        280..=289 => 4,
        _ => 0,
    }
}

fn read_eval_variant(reader: &mut DwgMergedReader) -> AssocEvalVariant {
    let code = reader.read_bit_short();
    let value = if code == 0 {
        AssocEvalValue::None
    } else {
        match eval_kind(code) {
            1 => AssocEvalValue::Real(reader.read_bit_double()),
            2 => AssocEvalValue::Long(reader.read_bit_long()),
            3 => AssocEvalValue::Short(reader.read_bit_short()),
            4 => AssocEvalValue::Byte(reader.read_byte()),
            5 => AssocEvalValue::Text(reader.read_variable_text()),
            6 => AssocEvalValue::Handle(handle(reader)),
            _ => AssocEvalValue::None,
        }
    };
    AssocEvalVariant { code, value }
}

fn read_value_param(reader: &mut DwgMergedReader) -> AssocValueParam {
    let class_version = reader.read_bit_long();
    let name = reader.read_variable_text();
    let unit_type = reader.read_bit_long();
    let count = safe_count(reader.read_bit_long());
    let mut variables = Vec::with_capacity(count as usize);
    for _ in 0..count {
        variables.push(AssocValueParamVariable {
            value: read_eval_variant(reader),
            handle: handle(reader),
        });
    }
    AssocValueParam {
        class_version,
        name,
        unit_type,
        variables,
        controlled_object_dependency: handle(reader),
    }
}

fn read_value_params(reader: &mut DwgMergedReader, count: i32) -> Vec<AssocValueParam> {
    let count = safe_count(count);
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        values.push(read_value_param(reader));
    }
    values
}

fn read_dependency(reader: &mut DwgMergedReader) -> AssocDependency {
    let class_version = reader.read_bit_short();
    let status = reader.read_bit_long();
    let is_read_dependency = reader.read_bit();
    let is_write_dependency = reader.read_bit();
    let is_attached_to_object = reader.read_bit();
    let is_delegating_to_owning_action = reader.read_bit();
    let order = reader.read_bit_long();
    let dependent_on = handle(reader);
    let has_name = reader.read_bit();
    let name = has_name.then(|| reader.read_variable_text());
    AssocDependency {
        class_version,
        status,
        is_read_dependency,
        is_write_dependency,
        is_attached_to_object,
        is_delegating_to_owning_action,
        order,
        dependent_on,
        name,
        read_dependency: handle(reader),
        node: handle(reader),
        dependency_body: handle(reader),
        dependency_body_id: reader.read_bit_long(),
    }
}

fn read_constraint_node_common(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> AssocConstraintNode {
    let node_id = reader.read_bit_long();
    let status_before = !version.r2013_plus(dxf_version);
    let mut status = if status_before { reader.read_byte() } else { 0 };
    let connection_count = safe_count(reader.read_bit_long());
    let mut connections = Vec::with_capacity(connection_count as usize);
    for _ in 0..connection_count {
        connections.push(reader.read_bit_long());
    }
    if !status_before {
        status = reader.read_byte();
    }
    AssocConstraintNode {
        node_id,
        status,
        connections,
        class_name: String::new(),
        registry_flag: false,
        data: AssocConstraintNodeData::None,
    }
}

fn is_plain_geometrical_constraint(class_name: &str) -> bool {
    matches!(
        class_name.to_ascii_uppercase().as_str(),
        "ACCENTERPOINTCONSTRAINT"
            | "ACCOLINEARCONSTRAINT"
            | "ACCONCENTRICCONSTRAINT"
            | "ACEQUALCURVATURECONSTRAINT"
            | "ACEQUALDISTANCECONSTRAINT"
            | "ACEQUALHELPPARAMETERCONSTRAINT"
            | "ACEQUALLENGTHCONSTRAINT"
            | "ACEQUALRADIUSCONSTRAINT"
            | "ACFIXEDCONSTRAINT"
            | "ACMIDPOINTCONSTRAINT"
            | "ACNORMALCONSTRAINT"
            | "ACPERPENDICULARCONSTRAINT"
            | "ACPOINTCOINCIDENCECONSTRAINT"
            | "ACPOINTCURVECONSTRAINT"
            | "ACSYMMETRICCONSTRAINT"
            | "ACTANGENTCONSTRAINT"
    )
}

fn read_geometrical_constraint(reader: &mut DwgMergedReader) -> (i32, bool, bool) {
    (reader.read_bit_long(), reader.read_bit(), reader.read_bit())
}

fn read_explicit_constraint(reader: &mut DwgMergedReader) -> (i32, bool, bool, Handle, Handle) {
    let (owner_id, is_implied, is_active) = read_geometrical_constraint(reader);
    (
        owner_id,
        is_implied,
        is_active,
        handle(reader),
        handle(reader),
    )
}

fn read_constraint_node_data(
    reader: &mut DwgMergedReader,
    class_name: &str,
) -> AssocConstraintNodeData {
    match class_name.to_ascii_uppercase().as_str() {
        "ACG2SMOOTHCONSTRAINT" => {
            let (owner_id, is_implied, is_active) = read_geometrical_constraint(reader);
            let count = safe_count(reader.read_bit_long());
            let mut owned_constraint_ids = Vec::with_capacity(count as usize);
            for _ in 0..count {
                owned_constraint_ids.push(reader.read_bit_long());
            }
            AssocConstraintNodeData::Composite {
                owner_id,
                is_implied,
                is_active,
                owned_constraint_ids,
            }
        }
        "ACHELPPARAMETER" => AssocConstraintNodeData::HelpParameter {
            value: reader.read_bit_double(),
            reserved: reader.read_bit(),
        },
        "ACCONSTRAINEDCIRCLE" => AssocConstraintNodeData::Circle {
            geometry_dependency: handle(reader),
            geometry_node_id: reader.read_bit_long(),
            center: reader.read_3bit_double(),
            normal: reader.read_3bit_double(),
            direction: reader.read_3bit_double(),
            radius: reader.read_bit_double(),
            start_parameter: reader.read_bit_double(),
            end_parameter: reader.read_bit_double(),
            reserved: reader.read_bit_double(),
        },
        "ACCONSTRAINEDARC" => AssocConstraintNodeData::Arc {
            geometry_dependency: handle(reader),
            geometry_node_id: reader.read_bit_long(),
            center: reader.read_3bit_double(),
            normal: reader.read_3bit_double(),
            direction: reader.read_3bit_double(),
            radius: reader.read_bit_double(),
            start_parameter: reader.read_bit_double(),
            end_parameter: reader.read_bit_double(),
            reserved: reader.read_bit_double(),
            start_point: reader.read_3bit_double(),
            end_point: reader.read_3bit_double(),
        },
        "ACCONSTRAINEDIMPLICITPOINT" => {
            let geometry_dependency = handle(reader);
            AssocConstraintNodeData::ImplicitPoint {
                geometry_dependency,
                geometry_node_id: reader.read_bit_long(),
                point: (!geometry_dependency.is_null()).then(|| reader.read_3bit_double()),
                point_type: reader.read_byte(),
                point_index: reader.read_bit_long(),
                curve_id: reader.read_bit_long(),
            }
        }
        "ACCONSTRAINEDPOINT" => {
            let geometry_dependency = handle(reader);
            AssocConstraintNodeData::Point {
                geometry_dependency,
                geometry_node_id: reader.read_bit_long(),
                point: (!geometry_dependency.is_null()).then(|| reader.read_3bit_double()),
            }
        }
        "ACCONSTRAINEDRIGIDSET" => {
            let geometry_dependency = handle(reader);
            let geometry_node_id = reader.read_bit_long();
            let reserved = reader.read_bit();
            let mut transform = [0.0; 16];
            for value in &mut transform {
                *value = reader.read_bit_double();
            }
            let count = safe_count(reader.read_bit_long());
            let mut geometry_ids = Vec::with_capacity(count as usize);
            for _ in 0..count {
                geometry_ids.push(reader.read_bit_long());
            }
            AssocConstraintNodeData::RigidSet {
                geometry_dependency,
                geometry_node_id,
                reserved,
                transform,
                geometry_ids,
            }
        }
        "ACCONSTRAINEDLINE"
        | "ACCONSTRAINEDCONSTRUCTIONLINE"
        | "ACCONSTRAINED2POINTSCONSTRUCTIONLINE"
        | "ACCONSTRAINEDDATUMLINE" => AssocConstraintNodeData::Line {
            geometry_dependency: handle(reader),
            geometry_node_id: reader.read_bit_long(),
            point: reader.read_3bit_double(),
            direction: reader.read_3bit_double(),
        },
        "ACCONSTRAINEDBOUNDEDLINE" => AssocConstraintNodeData::BoundedLine {
            geometry_dependency: handle(reader),
            geometry_node_id: reader.read_bit_long(),
            point: reader.read_3bit_double(),
            direction: reader.read_3bit_double(),
            is_ray: reader.read_bit(),
            start_point: reader.read_3bit_double(),
            end_point: reader.read_3bit_double(),
        },
        "ACANGLECONSTRAINT" | "AC3POINTANGLECONSTRAINT" => {
            let (owner_id, is_implied, is_active, value_dependency, dimension_dependency) =
                read_explicit_constraint(reader);
            AssocConstraintNodeData::Angle {
                owner_id,
                is_implied,
                is_active,
                value_dependency,
                dimension_dependency,
                sector_type: reader.read_byte(),
            }
        }
        "ACPARALLELCONSTRAINT" | "ACHORIZONTALCONSTRAINT" | "ACVERTICALCONSTRAINT" => {
            let (owner_id, is_implied, is_active) = read_geometrical_constraint(reader);
            AssocConstraintNodeData::Parallel {
                owner_id,
                is_implied,
                is_active,
                datum_line_index: (!class_name.eq_ignore_ascii_case("AcParallelConstraint"))
                    .then(|| reader.read_bit_long()),
            }
        }
        "ACDISTANCECONSTRAINT" => {
            let (owner_id, is_implied, is_active, value_dependency, dimension_dependency) =
                read_explicit_constraint(reader);
            let direction_type = reader.read_byte();
            AssocConstraintNodeData::Distance {
                owner_id,
                is_implied,
                is_active,
                value_dependency,
                dimension_dependency,
                direction_type,
                distance: (direction_type != 0).then(|| reader.read_3bit_double()),
            }
        }
        "ACRADIUSDIAMETERCONSTRAINT" => {
            let (owner_id, is_implied, is_active, value_dependency, dimension_dependency) =
                read_explicit_constraint(reader);
            AssocConstraintNodeData::RadiusDiameter {
                owner_id,
                is_implied,
                is_active,
                value_dependency,
                dimension_dependency,
                mode: reader.read_byte(),
            }
        }
        "ACCONSTRAINEDELLIPSE" => AssocConstraintNodeData::Ellipse {
            geometry_dependency: handle(reader),
            geometry_node_id: reader.read_bit_long(),
            center: reader.read_3bit_double(),
            major_axis: reader.read_3bit_double(),
            axis_ratio: reader.read_bit_double(),
        },
        "ACCONSTRAINEDBOUNDEDELLIPSE" => AssocConstraintNodeData::BoundedEllipse {
            geometry_dependency: handle(reader),
            geometry_node_id: reader.read_bit_long(),
            center: reader.read_3bit_double(),
            major_axis: reader.read_3bit_double(),
            axis_ratio: reader.read_bit_double(),
            start_point: reader.read_3bit_double(),
            end_point: reader.read_3bit_double(),
        },
        "ACCONSTRAINEDSPLINE" => {
            let geometry_dependency = handle(reader);
            let geometry_node_id = reader.read_bit_long();
            let rational = reader.read_bit();
            let periodic = reader.read_bit();
            let degree = reader.read_bit_long();
            let knot_tolerance = reader.read_bit_double();
            let knot_count = safe_count(reader.read_bit_long());
            let knot_physical_length = reader.read_bit_long();
            let knot_grow_length = reader.read_bit_long();
            let mut knots = Vec::with_capacity(knot_count as usize);
            for _ in 0..knot_count {
                knots.push(reader.read_bit_double());
            }
            let weight_count = safe_count(reader.read_bit_long());
            let weight_physical_length = reader.read_bit_long();
            let weight_grow_length = reader.read_bit_long();
            let mut weights = Vec::with_capacity(weight_count as usize);
            for _ in 0..weight_count {
                weights.push(reader.read_bit_double());
            }
            let control_point_count = safe_count(reader.read_bit_long());
            let control_point_physical_length = reader.read_bit_long();
            let control_point_grow_length = reader.read_bit_long();
            let mut control_points = Vec::with_capacity(control_point_count as usize);
            for _ in 0..control_point_count {
                control_points.push(reader.read_3bit_double());
            }
            let implicit_point_count = safe_count(reader.read_bit_long());
            let mut implicit_point_ids = Vec::with_capacity(implicit_point_count as usize);
            for _ in 0..implicit_point_count {
                implicit_point_ids.push(reader.read_bit_long());
            }
            AssocConstraintNodeData::Spline {
                geometry_dependency,
                geometry_node_id,
                rational,
                periodic,
                degree,
                knot_tolerance,
                knot_physical_length,
                knot_grow_length,
                knots,
                weight_physical_length,
                weight_grow_length,
                weights,
                control_point_physical_length,
                control_point_grow_length,
                control_points,
                implicit_point_ids,
            }
        }
        _ if is_plain_geometrical_constraint(class_name) => {
            let (owner_id, is_implied, is_active) = read_geometrical_constraint(reader);
            AssocConstraintNodeData::Geometrical {
                owner_id,
                is_implied,
                is_active,
            }
        }
        _ => AssocConstraintNodeData::None,
    }
}


/// The typed registry form this crate writes for programmatic
/// constraint-group records: a root node (its status a single bit), a
/// class-name table, a registry of (flag, class index, node id), then the
/// per-node common + typed arms. Strictly validated: any out-of-range
/// count, class index or stream overrun returns `None` (the caller
/// rewinds and takes the flat walk + verbatim capture), and the parse
/// must land within the record's trailing pad of the main-data end.
#[allow(clippy::too_many_lines)]
fn try_read_registry_nodes(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
    node_count: i32,
) -> Option<Vec<AssocConstraintNode>> {
    if !(1..=100_000).contains(&node_count) {
        return None;
    }
    // Root node: id, connections, status as a single bit.
    let root_id = reader.read_bit_long();
    let connection_count = safe_count(reader.read_bit_long());
    if !(0..=100_000).contains(&connection_count) {
        return None;
    }
    let mut root_connections = Vec::with_capacity(connection_count as usize);
    for _ in 0..connection_count {
        root_connections.push(reader.read_bit_long());
    }
    let root_status = u8::from(reader.read_bit());
    let mut nodes = vec![AssocConstraintNode {
        node_id: root_id,
        status: root_status,
        connections: root_connections,
        class_name: String::new(),
        registry_flag: false,
        data: AssocConstraintNodeData::None,
    }];
    // Class-name table.
    let class_type_count = reader.read_bit_long();
    if !(1..=4096).contains(&class_type_count) {
        return None;
    }
    let mut class_types = Vec::with_capacity(class_type_count as usize);
    for _ in 0..class_type_count {
        if reader.text_remaining_bits() < 0 {
            return None;
        }
        class_types.push(reader.read_variable_text());
    }
    // Registry: flag, class index, node id per registered node.
    let registered_count = reader.read_bit_long();
    if registered_count < 0 || registered_count > node_count {
        return None;
    }
    let mut registry = Vec::with_capacity(registered_count as usize);
    for _ in 0..registered_count {
        let registry_flag = reader.read_bit();
        let class_index = reader.read_bit_long();
        if class_index < 1 || class_index > class_type_count {
            return None;
        }
        let node_id = reader.read_bit_long();
        let class_name = class_types
            .get((class_index - 1) as usize)
            .cloned()
            .unwrap_or_default();
        registry.push((class_name, node_id, registry_flag));
    }
    for (class_name, registered_node_id, registry_flag) in registry {
        let mut node = read_constraint_node_common(reader, version, dxf_version);
        if node.node_id == 0 {
            node.node_id = registered_node_id;
        }
        node.data = read_constraint_node_data(reader, &class_name);
        node.class_name = class_name;
        node.registry_flag = registry_flag;
        nodes.push(node);
    }
    // The parse must land within the record's trailing pad of the
    // main-data end (the merged writer closes on the byte boundary).
    let position = reader.position_in_bits();
    let end = reader.main_data_end();
    if position > end || end - position > 7 {
        return None;
    }
    Some(nodes)
}

fn read_action(reader: &mut DwgMergedReader) -> AssocAction {
    let class_version = reader.read_bit_short();
    let geometry_status = reader.read_bit_long();
    let owning_network = handle(reader);
    let action_body = handle(reader);
    let action_index = reader.read_bit_long();
    let max_dependency_index = reader.read_bit_long();
    let count = safe_count(reader.read_bit_long());
    let mut dependencies = Vec::with_capacity(count as usize);
    for _ in 0..count {
        dependencies.push(AssocActionDependency {
            is_owned: reader.read_bit(),
            dependency: handle(reader),
        });
    }
    let mut owned_parameters = Vec::new();
    let mut values = Vec::new();
    if class_version > 1 {
        let _zero = reader.read_bit_short();
        let count = reader.read_bit_long();
        owned_parameters = read_handles(reader, count);
        let _zero = reader.read_bit_short();
        let count = reader.read_bit_long();
        values = read_value_params(reader, count);
    }
    AssocAction {
        class_version,
        geometry_status,
        owning_network,
        action_body,
        action_index,
        max_dependency_index,
        dependencies,
        owned_parameters,
        values,
    }
}

fn read_action_param(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> AssocActionParam {
    let is_r2013 = reader.read_bit_short();
    let version_value = if version.r2013_plus(dxf_version) {
        reader.read_bit_long()
    } else {
        0
    };
    AssocActionParam {
        is_r2013,
        version: version_value,
        name: reader.read_variable_text(),
    }
}

fn read_action_body(reader: &mut DwgMergedReader) -> AssocActionBody {
    AssocActionBody {
        version: reader.read_bit_long(),
    }
}

fn read_parameter_body(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> AssocParamBasedActionBody {
    if version.r2013_plus(dxf_version) {
        return AssocParamBasedActionBody::default();
    }
    let body_version = reader.read_bit_long();
    let minor = reader.read_bit_long();
    let count = reader.read_bit_long();
    let dependencies = read_handles(reader, count);
    let marker = reader.read_bit_long();
    let value_count = safe_count(reader.read_bit_long());
    let (empty_value_marker, dependency) = if value_count == 0 {
        (reader.read_bit_long(), handle(reader))
    } else {
        (0, Handle::NULL)
    };
    AssocParamBasedActionBody {
        version: body_version,
        minor,
        dependencies,
        marker,
        values: read_value_params(reader, value_count),
        empty_value_marker,
        dependency,
    }
}

/// Handle pull with LibreDWG's object-dat end bound.
///
/// Gold's `bit_read_H` refuses a handle whose one-byte form (code<<4 |
/// counter) would cross the record's data end and yields the null handle
/// ("bit_read_RC buffer overflow"), which out_json prints as the [0,0]
/// pair. The 2004/Surface.dwg ORIG record is truncated mid-payload: 23
/// data bytes whose handle region carries exactly [owner (8.0.0)][deps
/// ((3.2)ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢1294)][pab.assocdep ((4.2)ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢1293)] plus one leftover bit, so the
/// sab.assocdep form starts at that very last bit ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the zero-filling
/// reader would turn it into code 8, counter 0 and resolve `refÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã¢â‚¬Â¹ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢1`
/// garbage (1291) where gold reads NULL. Intact records (silver's own
/// rewrite and every non-truncated action-body record) keep every handle
/// slot inside the record, so the guard never fires on them.
fn surface_bounded_handle(reader: &mut DwgMergedReader) -> Handle {
    if reader.handle_remaining_bits() < 8 {
        return Handle::NULL;
    }
    handle(reader)
}

/// BitLong with LibreDWG's object-dat end bound (the action-body tails).
///
/// Gold's `bit_read_BL` consumes the 2-bit code and then refuses the
/// value bytes that would cross the record's data end, printing 0 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the
/// truncated 2004/Surface record reads pbsab_status as '01' + a byte
/// starting at the last data bit (0-padded read would give 128) and
/// class_version as a code starting past the end; both print 0. The
/// cursor discipline mirrors gold: the code is consumed when it fits,
/// everything else is left untouched. Intact records never cross.
fn surface_bounded_bit_long(reader: &mut DwgMergedReader) -> i32 {
    if reader.main_record_remaining_bits() < 2 {
        return 0;
    }
    let first = reader.read_bit();
    let second = reader.read_bit();
    match (first, second) {
        (false, false) => {
            if reader.main_record_remaining_bits() < 32 {
                0
            } else {
                reader.read_raw_long() as i32
            }
        }
        (false, true) => {
            if reader.main_record_remaining_bits() < 8 {
                0
            } else {
                reader.read_byte() as i32
            }
        }
        _ => 0,
    }
}

/// Byte tail with LibreDWG's object-dat end bound (see
/// `surface_bounded_bit_long`).
fn surface_bounded_byte(reader: &mut DwgMergedReader) -> u8 {
    if reader.main_record_remaining_bits() < 8 {
        return 0;
    }
    reader.read_byte()
}

fn read_surface_body(reader: &mut DwgMergedReader) -> AssocSurfaceBody {
    AssocSurfaceBody {
        version: reader.read_bit_long(),
        dependency: surface_bounded_handle(reader),
        is_semi_associative: reader.read_bit(),
        marker: reader.read_bit_long(),
        is_semi_override: reader.read_bit(),
        grip_status: reader.read_bit_short(),
    }
}

fn surface_kind(name: &str) -> Option<AssocSurfaceActionKind> {
    Some(match name {
        "ASSOCPLANESURFACEACTIONBODY" => AssocSurfaceActionKind::Plane,
        "ASSOCEXTENDSURFACEACTIONBODY" => AssocSurfaceActionKind::Extend,
        "ASSOCEXTRUDEDSURFACEACTIONBODY" => AssocSurfaceActionKind::Extruded,
        "ASSOCLOFTEDSURFACEACTIONBODY" => AssocSurfaceActionKind::Lofted,
        "ASSOCNETWORKSURFACEACTIONBODY" => AssocSurfaceActionKind::Network,
        "ASSOCOFFSETSURFACEACTIONBODY" => AssocSurfaceActionKind::Offset,
        "ASSOCREVOLVEDSURFACEACTIONBODY" => AssocSurfaceActionKind::Revolved,
        "ASSOCTRIMSURFACEACTIONBODY" => AssocSurfaceActionKind::Trim,
        "ASSOCBLENDSURFACEACTIONBODY" => AssocSurfaceActionKind::Blend,
        "ASSOCPATCHSURFACEACTIONBODY" => AssocSurfaceActionKind::Patch,
        "ASSOCFILLETSURFACEACTIONBODY" => AssocSurfaceActionKind::Fillet,
        "ASSOCSWEPTSURFACEACTIONBODY" => AssocSurfaceActionKind::Swept,
        "ASSOCEDGECHAMFERACTIONBODY" => AssocSurfaceActionKind::EdgeChamfer,
        "ASSOCEDGEFILLETACTIONBODY" => AssocSurfaceActionKind::EdgeFillet,
        _ => return None,
    })
}

fn read_surface_action(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
    kind: AssocSurfaceActionKind,
) -> AssocSurfaceActionBody {
    let action_body = read_action_body(reader);
    let parameter_body = read_parameter_body(reader, version, dxf_version);
    let surface_body = read_surface_body(reader);
    let path_status = surface_bounded_bit_long(reader);
    let mut value = AssocSurfaceActionBody {
        kind,
        action_body,
        parameter_body,
        surface_body,
        path_status,
        ..AssocSurfaceActionBody::default()
    };
    match kind {
        AssocSurfaceActionKind::Network
        | AssocSurfaceActionKind::Patch
        | AssocSurfaceActionKind::EdgeChamfer
        | AssocSurfaceActionKind::EdgeFillet => {}
        _ => value.class_version = surface_bounded_bit_long(reader),
    }
    match kind {
        AssocSurfaceActionKind::Extend => value.option = surface_bounded_byte(reader),
        AssocSurfaceActionKind::Offset => value.flags[0] = reader.read_bit(),
        AssocSurfaceActionKind::Trim => {
            value.flags[0] = reader.read_bit();
            value.flags[1] = reader.read_bit();
            value.distance = reader.read_bit_double();
        }
        AssocSurfaceActionKind::Blend => {
            value.flags[0] = reader.read_bit();
            value.flags[1] = reader.read_bit();
            value.flags[2] = reader.read_bit();
            value.status = reader.read_bit_short();
            value.flags[3] = reader.read_bit();
            value.flags[4] = reader.read_bit();
            value.secondary_status = reader.read_bit_short();
        }
        AssocSurfaceActionKind::Fillet => {
            value.status = reader.read_bit_short();
            value.first_point = reader.read_2raw_double();
            value.second_point = reader.read_2raw_double();
        }
        _ => {}
    }
    value
}

fn read_annotation_base(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> AssocAnnotationBase {
    if version.r2010_plus() {
        AssocAnnotationBase {
            version: reader.read_bit_short(),
            dependency: handle(reader),
            ..AssocAnnotationBase::default()
        }
    } else {
        AssocAnnotationBase {
            action_body: read_action_body(reader),
            parameter_body: read_parameter_body(reader, version, dxf_version),
            ..AssocAnnotationBase::default()
        }
    }
}

fn read_annotation_action(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
    kind: AssocAnnotationKind,
) -> AssocAnnotationActionBody {
    let mut value = AssocAnnotationActionBody {
        kind,
        ..AssocAnnotationActionBody::default()
    };
    if kind == AssocAnnotationKind::RestoreEntityState {
        value.action_body = read_action_body(reader);
        value.class_version = reader.read_bit_long();
        value.entity = handle(reader);
        return value;
    }
    value.annotation = read_annotation_base(reader, version, dxf_version);
    value.class_version = match kind {
        AssocAnnotationKind::ThreePointAngularDimension | AssocAnnotationKind::RotatedDimension => {
            reader.read_bit_short() as i32
        }
        _ => reader.read_bit_long(),
    };
    match kind {
        AssocAnnotationKind::MLeader => {
            let count = safe_count(reader.read_bit_long());
            value.actions.reserve(count as usize);
            for _ in 0..count {
                value.actions.push(AssocAnnotationDependency {
                    dependency_id: reader.read_bit_long(),
                    dependency: handle(reader),
                });
            }
        }
        AssocAnnotationKind::AlignedDimension
        | AssocAnnotationKind::OrdinateDimension
        | AssocAnnotationKind::RotatedDimension => {
            value.read_node = handle(reader);
            value.dimension_node = handle(reader);
        }
        AssocAnnotationKind::ThreePointAngularDimension => {
            value.read_node = handle(reader);
            value.dimension_node = handle(reader);
            value.dependency = handle(reader);
        }
        AssocAnnotationKind::RestoreEntityState => {}
    }
    value
}

fn read_single_dependency(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> AssocSingleDependencyActionParam {
    AssocSingleDependencyActionParam {
        action_param: read_action_param(reader, version, dxf_version),
        dependency_class_version: reader.read_bit_long(),
        dependency: handle(reader),
        class_version: reader.read_bit_long(),
    }
}

fn read_compound(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
    has_child_parameter: bool,
) -> AssocCompoundActionParam {
    let action_param = read_action_param(reader, version, dxf_version);
    let class_version = reader.read_bit_short();
    let status = reader.read_bit_short();
    let count = reader.read_bit_long();
    let parameters = read_handles(reader, count);
    let child_parameter = has_child_parameter.then(|| {
        let status = reader.read_bit_short();
        let id = reader.read_bit_long();
        let parameter = handle(reader);
        let (secondary_parameter, marker, tertiary_parameter) = if id != 0 {
            (handle(reader), reader.read_bit_long(), handle(reader))
        } else {
            (Handle::NULL, 0, Handle::NULL)
        };
        AssocChildParameter {
            status,
            id,
            parameter,
            secondary_parameter,
            marker,
            tertiary_parameter,
        }
    });
    AssocCompoundActionParam {
        action_param,
        class_version,
        status,
        parameters,
        child_parameter,
    }
}

fn read_array_action_body(
    reader: &mut DwgMergedReader,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> AssocArrayActionBody {
    let mut transform = [0.0; 16];
    let action_body = read_action_body(reader);
    let parameter_body = read_parameter_body(reader, version, dxf_version);
    let body_version = reader.read_bit_long();
    let parameter_block = reader.read_variable_text();
    for item in &mut transform {
        *item = reader.read_bit_double();
    }
    AssocArrayActionBody {
        action_body,
        parameter_body,
        version: body_version,
        parameter_block,
        transform,
    }
}

fn read_array_parameters(reader: &mut DwgMergedReader) -> AssocArrayParameters {
    let version = reader.read_bit_long();
    let count = safe_count(reader.read_bit_long());
    let class_name = reader.read_variable_text();
    let mut items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let class_version = reader.read_bit_long();
        let location = [
            reader.read_bit_long(),
            reader.read_bit_long(),
            reader.read_bit_long(),
        ];
        let flags = reader.read_bit_long();
        let uses_default_transform = false;
        let x_direction = crate::types::Vector3::ZERO;
        let mut transform = [0.0; 16];
        for item in &mut transform {
            *item = reader.read_bit_double();
        }
        let relative_transform = if flags & 2 != 0 {
            let mut matrix = [0.0; 16];
            for item in &mut matrix {
                *item = reader.read_bit_double();
            }
            Some(matrix)
        } else {
            None
        };
        let second_handle = (flags & 0x10 != 0).then(|| handle(reader));
        items.push(AssocArrayItem {
            class_version,
            location,
            flags,
            uses_default_transform,
            x_direction,
            transform,
            relative_transform,
            first_handle: None,
            second_handle,
        });
    }
    AssocArrayParameters {
        version,
        class_name,
        items,
        item_count: reader.read_bit_long(),
        row_count: reader.read_bit_long(),
        level_count: reader.read_bit_long(),
    }
}

fn read_dimension_association(reader: &mut DwgMergedReader) -> AssocDimensionAssociation {
    let associativity = reader.read_bit_long();
    let trans_space = reader.read_bit();
    let rotated_type = reader.read_byte();
    let dimension = handle(reader);
    let mut references: [Vec<AssocDimensionReference>; 4] = std::array::from_fn(|_| Vec::new());
    let mut total_references = 0usize;
    for slot in 0..4 {
        if associativity & (1 << slot) == 0 {
            continue;
        }
        loop {
            if total_references >= 6 {
                break;
            }
            let class_name = reader.read_variable_text();
            let osnap_type = reader.read_byte();
            let count = reader.read_bit_long();
            let xrefs = read_handles(reader, count);
            let (main_subent_type, main_gs_marker, xref_paths) = if osnap_type != 0 {
                let main_subent_type = reader.read_bit_long();
                let main_gs_marker = reader.read_bit_long();
                let count = safe_count(reader.read_bit_long());
                let mut xref_paths = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    xref_paths.push(reader.read_variable_text());
                }
                (main_subent_type, main_gs_marker, xref_paths)
            } else {
                (0, 0, Vec::new())
            };
            let osnap_distance = reader.read_bit_double();
            let osnap_point = reader.read_3bit_double();
            let (
                intersection_objects,
                intersection_subent_type,
                intersection_gs_marker,
                intersection_xref_paths,
            ) = if osnap_type == 6 || osnap_type == 11 {
                let count = reader.read_bit_long();
                let intersection_objects = read_handles(reader, count);
                let intersection_subent_type = reader.read_bit_long();
                let intersection_gs_marker = reader.read_bit_long();
                let count = safe_count(reader.read_bit_long());
                let mut paths = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    paths.push(reader.read_variable_text());
                }
                (
                    intersection_objects,
                    intersection_subent_type,
                    intersection_gs_marker,
                    paths,
                )
            } else {
                (Vec::new(), 0, 0, Vec::new())
            };
            let has_last_point_reference = reader.read_bit();
            references[slot].push(AssocDimensionReference {
                class_name,
                osnap_type,
                xrefs,
                main_subent_type,
                main_gs_marker,
                xref_paths,
                osnap_distance,
                osnap_point,
                intersection_objects,
                intersection_subent_type,
                intersection_gs_marker,
                intersection_xref_paths,
                has_last_point_reference,
            });
            total_references += 1;
            if !has_last_point_reference {
                break;
            }
        }
    }
    AssocDimensionAssociation {
        associativity,
        trans_space,
        rotated_type,
        dimension,
        references,
    }
}

fn read_static_pers_subent_manager(reader: &mut DwgMergedReader) -> PersSubentManager {
    let class_version = reader.read_bit_long();
    let marker_zero = reader.read_bit_long();
    let marker_two = reader.read_bit_long();
    let associative_step_count = reader.read_bit_long();
    let associative_subent_count = reader.read_bit_long();
    let count = safe_count(reader.read_bit_long());
    let mut steps = Vec::with_capacity(count as usize);
    for _ in 0..count {
        steps.push(reader.read_bit_long());
    }
    let mut subents = Vec::new();
    if reader.main_remaining_bits() > 0 {
        let count = safe_count(reader.read_bit_long());
        subents.reserve(count as usize);
        for _ in 0..count {
            subents.push(reader.read_bit_long());
        }
    }
    // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-6: the undocumented tail after the subents vector ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
    // a variable BL run captured verbatim (the loft specimens carry
    // two; the Chamfer/Fillet 2DF records carry the ~1224-BL history
    // blob; the count-0 records carry none). The run ends flush at the
    // main content end ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the bit after the content is the merged
    // stream's no-text flag, never a record field (the H8h-ext-5
    // lesson). See PersSubentManager::tail_bls.
    let mut tail_bls = Vec::new();
    while reader.main_remaining_bits() >= 2 {
        tail_bls.push(reader.read_bit_long());
    }
    PersSubentManager {
        class_version,
        marker_zero,
        marker_two,
        associative_step_count,
        associative_subent_count,
        steps,
        subents,
        tail_bls,
    }
}

pub fn read_associative_data(
    reader: &mut DwgMergedReader,
    dxf_name: &str,
    version: DwgVersion,
    dxf_version: DxfVersion,
) -> Option<AssociativeData> {
    let name = associative_canonical_name(dxf_name);
    let value = match name.as_str() {
        "ASSOCDEPENDENCY" => AssociativeData::Dependency(read_dependency(reader)),
        "ASSOCVALUEDEPENDENCY" => AssociativeData::ValueDependency(AssocValueDependency {
            dependency: read_dependency(reader),
            class_version: reader.read_bit_long(),
            name: reader.read_variable_text(),
            value: read_eval_variant(reader),
        }),
        "ASSOCGEOMDEPENDENCY" => {
            let dependency = read_dependency(reader);
            let class_version = reader.read_bit_short();
            let enabled = reader.read_bit();
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-15: capture the text stream's PRESENCE at the
            // classname TU ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the author's PER-RECORD form: her R2013
            // Constraints geomdeps carry has_strings: 0 (no stream; the
            // TU read returns "" at 0 bits) while the AC1021 corpus
            // authors write has_strings: 1 even with empty-only
            // streams. The writer skips the TU on a no-stream record so
            // the merge emits no stream. Gated to R2007+ ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the
            // pre-2007 classname is an inline main TV (no text
            // streams exist; the flag stays false and the write is
            // the normal inline form).
            let wire_no_text_stream =
                dxf_version >= DxfVersion::AC1021 && reader.text_remaining_bits() <= 0;
            let class_name = reader.read_variable_text();
            let dependent_on_compound_object = reader.read_bit();
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-10: capture the undocumented persubent-id
            // tail ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the main-stream bits after dependent_on_compound_
            // object that gold's spec block (dwg2.spec 3148) does not
            // cover and its own -v9 walk parks as unknown (example_
            // 2007 h=396: 46 bits; the handle stream holds only the
            // five parsed refs plus the closing 1s pad, so the main
            // tail is the whole delta). Re-emitted verbatim by the
            // writer; records whose parse consumes the region exactly
            // capture nothing and keep the modeled emission.
            let mut persistent_subent = AssocPersistentSubentId {
                class_name,
                dependent_on_compound_object,
                tail_bits: None,
                tail_bit_len: 0,
                wire_no_text_stream,
            };
            let tail_from = reader.position_in_bits();
            let tail_to = reader.main_data_end();
            if tail_to > tail_from {
                let count = (tail_to - tail_from) as u32;
                if let Some(bytes) = reader.peek_window_bytes(tail_from, count) {
                    persistent_subent.tail_bits = Some(bytes);
                    persistent_subent.tail_bit_len = count;
                }
            }
            AssociativeData::GeomDependency(AssocGeomDependency {
                dependency,
                class_version,
                enabled,
                persistent_subent,
            })
        }
        "ASSOCACTION" => AssociativeData::Action(read_action(reader)),
        "ASSOCNETWORK" => {
            let action = read_action(reader);
            let network_version = reader.read_bit_short();
            let network_action_index = reader.read_bit_long();
            let count = safe_count(reader.read_bit_long());
            let mut actions = Vec::with_capacity(count as usize);
            for _ in 0..count {
                actions.push(AssocActionDependency {
                    is_owned: reader.read_bit(),
                    dependency: handle(reader),
                });
            }
            let count = reader.read_bit_long();
            let owned_actions = read_handles(reader, count);
            AssociativeData::Network(AssocNetwork {
                action,
                network_version,
                network_action_index,
                actions,
                owned_actions,
            })
        }
        name if surface_kind(name).is_some() => AssociativeData::SurfaceActionBody(
            read_surface_action(reader, version, dxf_version, surface_kind(name).unwrap()),
        ),
        "ASSOCRESTOREENTITYSTATEACTIONBODY" => {
            AssociativeData::AnnotationActionBody(read_annotation_action(
                reader,
                version,
                dxf_version,
                AssocAnnotationKind::RestoreEntityState,
            ))
        }
        "ASSOCMLEADERACTIONBODY" => AssociativeData::AnnotationActionBody(read_annotation_action(
            reader,
            version,
            dxf_version,
            AssocAnnotationKind::MLeader,
        )),
        "ASSOCALIGNEDDIMACTIONBODY" => {
            AssociativeData::AnnotationActionBody(read_annotation_action(
                reader,
                version,
                dxf_version,
                AssocAnnotationKind::AlignedDimension,
            ))
        }
        "ASSOC3POINTANGULARDIMACTIONBODY" => {
            AssociativeData::AnnotationActionBody(read_annotation_action(
                reader,
                version,
                dxf_version,
                AssocAnnotationKind::ThreePointAngularDimension,
            ))
        }
        "ASSOCORDINATEDIMACTIONBODY" => {
            AssociativeData::AnnotationActionBody(read_annotation_action(
                reader,
                version,
                dxf_version,
                AssocAnnotationKind::OrdinateDimension,
            ))
        }
        "ASSOCROTATEDDIMACTIONBODY" => {
            AssociativeData::AnnotationActionBody(read_annotation_action(
                reader,
                version,
                dxf_version,
                AssocAnnotationKind::RotatedDimension,
            ))
        }
        "ASSOCPERSSUBENTMANAGER" => {
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-6: the gold dwg2.spec field order ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
            // class_version, unknown_3/0/2 (the markers), unknown_bl1,
            // unknown_bl2, num_steps, steps, num_subents, subents, and
            // the class_version-2 tail (unknown_bl3 + B). The old parse
            // skipped bl1/bl2 (desyncing the record) and read BLs until
            // the stream end plus a final_flag bit (the H8h-ext-5
            // lesson: the bit after the content is the merged stream's
            // no-text flag, never a record field).
            let class_version = reader.read_bit_long();
            let markers = [
                reader.read_bit_long(),
                reader.read_bit_long(),
                reader.read_bit_long(),
            ];
            let bl1 = reader.read_bit_long();
            let bl2 = reader.read_bit_long();
            let steps = {
                let count = safe_count(reader.read_bit_long());
                let mut result = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    result.push(reader.read_bit_long());
                }
                result
            };
            let subents = {
                let count = safe_count(reader.read_bit_long());
                let mut result = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    result.push(reader.read_bit_long());
                }
                result
            };
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-6: the undocumented tail after the subents
            // vector ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â a variable BL run (captured verbatim; the gold
            // spec declares only the cv2 [BL][B] pair, but the cv=1
            // corpus records carry more there, e.g. LoftCSurf/LoftM
            // 2DD's [0,0,0,1,1,0]), then the trailing B (the last
            // content bit). The bit after the content is the merged
            // stream's no-text flag ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â never a record field (the
            // H8h-ext-5 lesson).
            let mut tail_bls = Vec::new();
            while reader.main_remaining_bits() > 1 {
                tail_bls.push(reader.read_bit_long());
            }
            let trailing_b = reader.read_bit();
            AssociativeData::PersSubentManager(AssocPersSubentManager {
                class_version,
                markers,
                bl1,
                bl2,
                steps,
                subents,
                tail_bls,
                trailing_b,
            })
        }
        "ASSOCEDGEACTIONPARAM" => {
            let single_dependency = read_single_dependency(reader, version, dxf_version);
            let parameter = handle(reader);
            let has_action = reader.read_bit();
            let action_type = reader.read_bit_long();
            let subcurve_kind = match action_type {
                11 => AssocSubcurveKind::Arc,
                17 => AssocSubcurveKind::Ellipse,
                19 => AssocSubcurveKind::Line,
                23 => AssocSubcurveKind::LineSegment3d,
                42 => AssocSubcurveKind::Nurb3d,
                27 => AssocSubcurveKind::Curve3d,
                _ => AssocSubcurveKind::None,
            };
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-4 + TODO B2 (2026-10-01): the subcurve
            // geometry region after action_type. The typed forms:
            // ARC (11) ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â twelve BDs: center, normal, x-axis (3BD
            // each), radius, start/end angles (the H8h-ext-4
            // reverse-engineering); the R2013+ frames append a
            // constant two-bit `10` trailing form but the read stops
            // at the twelfth BD (the tail is a write-side emission,
            // see the writer arm). ELLIPSE (17) ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â thirteen BDs:
            // center, major/minor-axis unit vectors, major/minor
            // radii, start/end angles (the B2 authored quads + the
            // 2004/Surface.dwg corpus specimens). LINESEG3D (23) ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
            // six BDs: start/end points (same double-source
            // evidence). The remaining kinds ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â NURB3D (42, a
            // ~1300-bit parameterized form), the gold-unknown 47 and
            // any future 19/27 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â stay unread and their region is
            // captured verbatim for same-version replay (the
            // H8h-ext-8 `nodes_wire_main` pattern).
            let mut subcurve = None;
            let mut curve = Vec::new();
            let mut subcurve_wire = None;
            let mut subcurve_wire_bit_len = 0u32;
            match action_type {
                11 => {
                    let (center, normal, x_axis) = (
                        reader.read_3bit_double(),
                        reader.read_3bit_double(),
                        reader.read_3bit_double(),
                    );
                    let (radius, start_angle, end_angle) = (
                        reader.read_bit_double(),
                        reader.read_bit_double(),
                        reader.read_bit_double(),
                    );
                    subcurve = Some(AssocSubcurve::Arc(AssocArcSubcurve {
                        center,
                        normal,
                        x_axis,
                        radius,
                        start_angle,
                        end_angle,
                    }));
                    curve = vec![
                        AssocCurveValue::Point(center),
                        AssocCurveValue::Point(normal),
                        AssocCurveValue::Point(x_axis),
                        AssocCurveValue::Real(radius),
                        AssocCurveValue::Real(start_angle),
                        AssocCurveValue::Real(end_angle),
                    ];
                }
                17 => {
                    let (center, major_axis, minor_axis) = (
                        reader.read_3bit_double(),
                        reader.read_3bit_double(),
                        reader.read_3bit_double(),
                    );
                    let (major_radius, minor_radius, start_angle, end_angle) = (
                        reader.read_bit_double(),
                        reader.read_bit_double(),
                        reader.read_bit_double(),
                        reader.read_bit_double(),
                    );
                    subcurve = Some(AssocSubcurve::Ellipse(AssocEllipseSubcurve {
                        center,
                        major_axis,
                        minor_axis,
                        major_radius,
                        minor_radius,
                        start_angle,
                        end_angle,
                    }));
                    curve = vec![
                        AssocCurveValue::Point(center),
                        AssocCurveValue::Point(major_axis),
                        AssocCurveValue::Point(minor_axis),
                        AssocCurveValue::Real(major_radius),
                        AssocCurveValue::Real(minor_radius),
                        AssocCurveValue::Real(start_angle),
                        AssocCurveValue::Real(end_angle),
                    ];
                }
                23 => {
                    let (start_point, end_point) = (
                        reader.read_3bit_double(),
                        reader.read_3bit_double(),
                    );
                    subcurve = Some(AssocSubcurve::LineSegment3d(
                        AssocLineSegment3dSubcurve {
                            start_point,
                            end_point,
                        },
                    ));
                    curve = vec![
                        AssocCurveValue::Point(start_point),
                        AssocCurveValue::Point(end_point),
                    ];
                }
                _ => {
                    let region_start = reader.position_in_bits();
                    let region_end = reader.main_data_end();
                    if region_end > region_start {
                        let count = (region_end - region_start) as u32;
                        if let Some(bytes) =
                            reader.peek_window_bytes(region_start, count)
                        {
                            // TODO A8 (2026-10-02): the NURB3D (42)
                            // region parses TYPED ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the measured
                            // grammar (see `AssocNurb3dSubcurve`):
                            // fully self-delimiting and era-stable
                            // (bit-identical 2007/2018 regions on
                            // every specimen). The parser gates on
                            // the measured constants and exact
                            // closure; any deviation (a future
                            // variant) rides the verbatim
                            // capture+replay net instead.
                            if action_type == 42 {
                                if let Some(nurb) =
                                    parse_nurb3d_region(&bytes, count)
                                {
                                    subcurve =
                                        Some(AssocSubcurve::Nurb3d(nurb));
                                } else {
                                    subcurve_wire = Some(bytes);
                                    subcurve_wire_bit_len = count;
                                }
                            } else if action_type == 47 {
                                // TODO A8 (2026-10-03): the composite
                                // (47) region parses TYPED ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the
                                // segment-list grammar (see
                                // `AssocCompositeSubcurve`): BL count +
                                // per segment BS kind (23 line
                                // start/delta, 11 the arc form with the
                                // R2013+ trailing tail). Same gates: the
                                // known kinds and exact closure, else
                                // the verbatim net.
                                let r2013_plus =
                                    version.r2013_plus(dxf_version);
                                if let Some(composite) =
                                    parse_composite47_region(
                                        &bytes,
                                        count,
                                        r2013_plus,
                                    )
                                {
                                    subcurve = Some(
                                        AssocSubcurve::Composite(composite),
                                    );
                                } else {
                                    subcurve_wire = Some(bytes);
                                    subcurve_wire_bit_len = count;
                                }
                            } else {
                                subcurve_wire = Some(bytes);
                                subcurve_wire_bit_len = count;
                            }
                        }
                    }
                }
            }
            let subcurve_wire_dxf_version =
                if subcurve_wire.is_some() { Some(dxf_version) } else { None };
            AssociativeData::EdgeActionParam(AssocEdgeActionParam {
                curve,
                single_dependency,
                parameter,
                has_action,
                action_type,
                subcurve_kind,
                subcurve,
                subcurve_wire,
                subcurve_wire_bit_len,
                subcurve_wire_dxf_version,
            })
        }
        "ASSOC2DCONSTRAINTGROUP" => {
            let action = read_action(reader);
            let group_version = reader.read_bit_long();
            let flag = reader.read_bit();
            let work_plane = [
                reader.read_3bit_double(),
                reader.read_3bit_double(),
                reader.read_3bit_double(),
            ];
            let dependency = handle(reader);
            let count = reader.read_bit_long();
            let actions = read_handles(reader, count);
            let node_count = safe_count(reader.read_bit_long());
            // gold dwg2.spec ASSOC2DCONSTRAINTGROUP (5682 + AcConstraint
            // GroupNode_fields 5576): num_nodes BL then a FLAT REPEAT ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
            // per node: nodeid BLd, [pre-R2013b: status RC], num_
            // connections BL, connections BL-vector, [R2013b+: status
            // RC]. The old root-node + class-registry shape misparsed the
            // per-node data as one global connection vector (garbage
            // signed BLs) and lost the node count: gold reads 9 nodes on
            // Constraints.dwg where this read 1, 129 where this read 113.
            let node_region_start = reader.position_in_bits();
            // Records this crate authored (the programmatic graph the
            // host builds, or a reload of one) carry the typed registry
            // form: a root node, a class-name table, a registry, then the
            // per-node common + typed arms. Real-file records (the
            // author's wire) fail its strict validation and fall through
            // to the flat walk + verbatim capture below, unchanged.
            let snapshot = reader.positions_snapshot();
            let mut nodes = Vec::new();
            let mut typed = false;
            if node_count > 0 {
                if let Some(typed_nodes) = try_read_registry_nodes(
                    reader,
                    version,
                    dxf_version,
                    node_count,
                ) {
                    nodes = typed_nodes;
                    typed = true;
                } else {
                    reader.restore_positions(snapshot);
                }
            }
            if !typed {
                nodes = Vec::with_capacity(node_count as usize);
                for _ in 0..node_count {
                    let node_id = reader.read_bit_long();
                    let mut status = 0u8;
                    if !version.r2013_plus(dxf_version) {
                        status = reader.read_byte();
                    }
                    let num_connections = safe_count(reader.read_bit_long());
                    let mut connections = Vec::with_capacity(num_connections as usize);
                    for _ in 0..num_connections {
                        connections.push(reader.read_bit_long());
                    }
                    if version.r2013_plus(dxf_version) {
                        status = reader.read_byte();
                    }
                    nodes.push(AssocConstraintNode {
                        node_id,
                        status,
                        connections,
                        class_name: String::new(),
                        registry_flag: false,
                        data: AssocConstraintNodeData::None,
                    });
                }
            }
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-8: the node-region wire capture. Gold's flat
            // REPEAT misparses the authored records ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â on the 2007/
            // Constraints.dwg group (h 3E3, nine nodes) gold's own -v9
            // walk desyncs at node[1] and parks 5249 unknown bits. The
            // real wire (cross-verified against the R2000/R2004
            // ancestors of the same drawing ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the circle node's data
            // region is bit-identical across eras once the inline
            // class-name TV of the pre-2007 records is discounted)
            // carries, per node: a class-name TU consumed from the
            // TEXT stream in walk order ("AcConstrainedCircle",
            // "AcConstrainedImplicitPoint", "AcCenterPointConstraint",
            // ...), a class data arm (the circle: connection BLs, the
            // center 3BD, normal/x-axis 3BD shorts, radius BD, 0.0,
            // 2ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚ÂÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬; the implicit points: connection BLs, point_idx BLd
            // -1, curve_id BLd; ...) and per-node geometry handles in
            // the HANDLE stream (two soft pointers into the group's
            // two ASSOCGEOMDEPENDENCYs plus three inline nulls). None
            // of it is documented (the ODA spec and libredwg cover
            // only the flat base form). Retain the region verbatim so
            // the conventional rewrite re-emits her bytes: the main
            // bits from the end of num_nodes to the record's
            // main-data end, the per-node class-name TUs, and the
            // handle bits from the drain position after this record's
            // own head reads to the record end (the captured tail
            // includes her closing 1s pad; the merged writer's own pad
            // is a no-op once aligned).
            //
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-13: the capture extends to the TwoStream
            // eras (AC1015/AC1018) ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the same drawing's R2000/R2004
            // specimens carry the node class names INLINE as main TVs
            // (inside the captured region, so no separate names
            // capture), and their handle streams are bit-continuous
            // at the RL (the authored ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19.4.C frame our merge already
            // mirrors). DXF/programmatic reads keep the naive modeled
            // emission (`nodes_wire_main` stays `None`).
            //
            // ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19 H8h-ext-14: the capture extends to the R2010/R2013
            // frames (AC1024/AC1027 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the MC handle-bits header, the
            // BOT type, the flag at handle_startÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã¢â‚¬Â¹ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢1). Their text
            // streams (has_strings: 1) hold content this campaign
            // never decoded ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the AC21 raw-stream dump instrument
            // does not cover the R2010+ containers ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â so the region
            // is retained VERBATIM (the ext-12 TABLECONTENT
            // `wire_text` pattern) instead of re-encoding class-name
            // TUs; AC1021 keeps the decoded-names path (verified
            // 58/58). The naive walk desyncs on these records exactly
            // as on AC1021 (gold's own -v9 walk errors at node[1]:
            // nconn 2800028726 / 68456580), but the capture is
            // peek-based ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the bounds come from the frame, not the
            // walk.
            let mut nodes_wire_names: Vec<String> = Vec::new();
            let mut nodes_wire_main: Option<Vec<u8>> = None;
            let mut nodes_wire_main_bit_len: u32 = 0;
            let mut nodes_wire_handles: Option<Vec<u8>> = None;
            let mut nodes_wire_handles_bit_len: u32 = 0;
            let mut nodes_wire_text: Option<Vec<u8>> = None;
            let mut nodes_wire_text_bit_len: u32 = 0;
            // TODO A5 family 2 (2026-10-01): AC1032 (2018) joins the
            // captured frames ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the Dynblocks specimen's two
            // ACDBASSOC2DCONSTRAINTGROUP records (0xBB1B/0xBB81) have
            // the same R2010+ container shape (the MC handle-bits
            // header, the BOT type, the flag at handle_startÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã¢â‚¬Â¹ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢1, an
            // undecoded text region); without the capture the naive
            // typed REPEAT balloons (200k phantom nodes read from her
            // ~170-node region) and the rewrite explodes 2948ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢29650
            // bytes. The capture is peek-based ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the bounds come from
            // the frame, not the walk.
            let era_wire = matches!(
                dxf_version,
                DxfVersion::AC1015
                    | DxfVersion::AC1018
                    | DxfVersion::AC1021
                    | DxfVersion::AC1024
                    | DxfVersion::AC1027
                    | DxfVersion::AC1032
            );
            if era_wire && node_count > 0 && !typed {
                let node_region_end = reader.main_data_end();
                if node_region_end > node_region_start {
                    let count = (node_region_end - node_region_start) as u32;
                    if let Some(bytes) =
                        reader.peek_window_bytes(node_region_start, count)
                    {
                        nodes_wire_main = Some(bytes);
                        nodes_wire_main_bit_len = count;
                    }
                }
                // The class-name TUs, in walk order, bounded by what
                // the record's text stream actually holds (the
                // dissected corpus specimen carries exactly nine).
                // TwoStream eras: the names are inline main TVs ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
                // already inside the captured region.
                if dxf_version == DxfVersion::AC1021 {
                    for _ in 0..node_count {
                        if reader.text_remaining_bits() <= 0 {
                            break;
                        }
                        nodes_wire_names.push(reader.read_variable_text());
                    }
                }
                // R2010+: the whole text region, verbatim (the naive
                // walk reads no text, so the remaining count is the
                // full region). TODO A5 family 2 (2026-10-01): AC1032
                // joins the captured frames (the 2018 container is
                // the same MC/BOT shape).
                if matches!(
                    dxf_version,
                    DxfVersion::AC1024 | DxfVersion::AC1027 | DxfVersion::AC1032
                )
                {
                    let text_len = reader.text_remaining_bits().max(0) as u32;
                    if text_len > 0 {
                        if let Some(bytes) =
                            reader.peek_window_bytes(node_region_end, text_len)
                        {
                            nodes_wire_text = Some(bytes);
                            nodes_wire_text_bit_len = text_len;
                        }
                    }
                }
                let handle_from = reader.handle_position_in_bits();
                // Trim the author's closing 1s pad (ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§19.4: the record's
                // final partial byte is the 1s pad; our merged writer
                // re-creates it at close). The pad is at most 7 bits ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
                // a wider run means real handle bits, not padding.
                let mut handle_to = reader.record_end_bits();
                let mut pad_bits: i64 = 0;
                while pad_bits < 7 && handle_to - pad_bits > handle_from {
                    let bit = reader
                        .peek_window_bytes(handle_to - pad_bits - 1, 1)
                        .map(|bytes| bytes[0] >> 7);
                    if bit != Some(1) {
                        break;
                    }
                    pad_bits += 1;
                }
                handle_to -= pad_bits;
                if handle_to > handle_from {
                    let count = (handle_to - handle_from) as u32;
                    if let Some(bytes) =
                        reader.peek_window_bytes(handle_from, count)
                    {
                        nodes_wire_handles = Some(bytes);
                        nodes_wire_handles_bit_len = count;
                    }
                }
            }
            AssociativeData::ConstraintGroup(Assoc2dConstraintGroup {
                action,
                version: group_version,
                flag,
                work_plane,
                dependency,
                actions,
                nodes,
                nodes_wire_names,
                nodes_wire_main,
                nodes_wire_main_bit_len,
                nodes_wire_handles,
                nodes_wire_handles_bit_len,
                nodes_wire_text,
                nodes_wire_text_bit_len,
            })
        }
        "ASSOCVARIABLE" => {
            let action = read_action(reader);
            let class_version = reader.read_bit_long();
            let name = reader.read_variable_text();
            let expression = reader.read_variable_text();
            let evaluator = reader.read_variable_text();
            let description = reader.read_variable_text();
            let value = read_eval_variant(reader);
            let has_cached_value = reader.read_bit();
            let cached_value = if has_cached_value {
                reader.read_variable_text()
            } else {
                String::new()
            };
            let flag = reader.read_bit();
            let count = if reader.main_remaining_bits() >= 2 {
                safe_count(reader.read_bit_long())
            } else {
                0
            };
            let mut dependencies = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let dependency = handle(reader);
                dependencies.push(AssocVariableDependency {
                    dependency,
                    flags: reader.read_bit_long(),
                });
            }
            AssociativeData::Variable(AssocVariable {
                action,
                class_version,
                name,
                expression,
                evaluator,
                description,
                value,
                has_cached_value,
                cached_value,
                flag,
                dependencies,
            })
        }
        "ASSOCACTIONPARAM" => {
            AssociativeData::ActionParam(read_action_param(reader, version, dxf_version))
        }
        "ASSOCCOMPOUNDACTIONPARAM" => {
            AssociativeData::CompoundActionParam(read_compound(reader, version, dxf_version, false))
        }
        "ASSOCOSNAPPOINTREFACTIONPARAM" => {
            let compound = read_compound(reader, version, dxf_version, true);
            AssociativeData::OsnapPointRefActionParam(AssocOsnapPointRefActionParam {
                compound,
                status: reader.read_bit_short(),
                osnap_mode: reader.read_byte(),
                parameter: reader.read_bit_double(),
            })
        }
        "ASSOCPOINTREFACTIONPARAM" => {
            AssociativeData::PointRefActionParam(read_compound(reader, version, dxf_version, true))
        }
        "ASSOCOBJECTACTIONPARAM" => {
            AssociativeData::ObjectActionParam(read_single_dependency(reader, version, dxf_version))
        }
        "ASSOCPATHACTIONPARAM" => {
            let compound = read_compound(reader, version, dxf_version, false);
            AssociativeData::PathActionParam(AssocPathActionParam {
                compound,
                version: reader.read_bit_long(),
            })
        }
        "ASSOCDIMDEPENDENCYBODY" => AssociativeData::DimDependencyBody(AssocDimDependencyBody {
            dependency_body_version: reader.read_bit_short(),
            base_version: reader.read_bit_short(),
            name: reader.read_variable_text(),
            class_version: reader.read_bit_short(),
        }),
        "ASSOCFACEACTIONPARAM" => {
            let single_dependency = read_single_dependency(reader, version, dxf_version);
            AssociativeData::FaceActionParam(AssocFaceActionParam {
                single_dependency,
                index: reader.read_bit_long(),
            })
        }
        "ASSOCVERTEXACTIONPARAM" => {
            let single_dependency = read_single_dependency(reader, version, dxf_version);
            AssociativeData::VertexActionParam(AssocVertexActionParam {
                single_dependency,
                point: reader.read_3bit_double(),
            })
        }
        "ASSOCASMBODYACTIONPARAM" => {
            let single_dependency = read_single_dependency(reader, version, dxf_version);
            let data = super::entities::read_acis_entity(reader, version, dxf_version, false);
            let history = if data.version > 1 {
                handle(reader)
            } else {
                Handle::NULL
            };
            let mut acis_data = crate::entities::AcisData::new();
            acis_data.version = if data.is_binary {
                AcisVersion::Version2
            } else {
                AcisVersion::Version1
            };
            acis_data.sat_data = data.sat_data;
            acis_data.sab_data = data.sab_data;
            acis_data.is_binary = data.is_binary;
            acis_data.revision = data.revision;
            acis_data.materials = data.materials;
            acis_data.wireframe_data_present = data.wireframe_data_present;
            acis_data.wireframe_point_present = data.wireframe_point_present;
            acis_data.wireframe_isoline_present = data.wireframe_isoline_present;
            acis_data.acis_empty_bit = data.acis_empty_bit;
            acis_data.extra_acis_data = data.extra_acis_data.map(Box::new);
            acis_data.wireframe_isolines = data.isolines;
            AssociativeData::AsmBodyActionParam(AssocAsmBodyActionParam {
                single_dependency,
                acis_data,
                point_of_reference: data.point,
                wires: data.wires,
                silhouettes: data.silhouettes,
                history,
            })
        }
        "ASSOCARRAYMODIFYPARAMETERS"
        | "ASSOCARRAYPATHPARAMETERS"
        | "ASSOCARRAYPOLARPARAMETERS"
        | "ASSOCARRAYRECTANGULARPARAMETERS" => {
            AssociativeData::ArrayParameters(read_array_parameters(reader))
        }
        "ASSOCARRAYACTIONBODY" => {
            AssociativeData::ArrayActionBody(read_array_action_body(reader, version, dxf_version))
        }
        "ASSOCARRAYMODIFYACTIONBODY" => {
            let body = read_array_action_body(reader, version, dxf_version);
            let status = reader.read_bit_short();
            let count = safe_count(reader.read_bit_long());
            let mut item_locations = Vec::with_capacity(count as usize);
            for _ in 0..count {
                item_locations.push([
                    reader.read_bit_long(),
                    reader.read_bit_long(),
                    reader.read_bit_long(),
                ]);
            }
            AssociativeData::ArrayModifyActionBody(AssocArrayModifyActionBody {
                body,
                status,
                item_locations,
            })
        }
        "DIMASSOC" => AssociativeData::DimensionAssociation(read_dimension_association(reader)),
        "ACDBCENTERMARKACTIONBODY" | "ACDBCENTERLINEACTIONBODY" => {
            AssociativeData::SmartCenterActionBody(AssocSmartCenterActionBody {
                action_body: read_action_body(reader),
                parameter_body: read_parameter_body(reader, version, dxf_version),
                version: reader.read_bit_long(),
            })
        }
        "PERSUBENTMGR" => {
            AssociativeData::PersSubentManagerStatic(read_static_pers_subent_manager(reader))
        }
        "ASSOCVIEWREPACTIONBODY" => AssociativeData::ViewRepActionBody(AssocViewRepActionBody {
            action_body: read_action_body(reader),
            class_version: reader.read_bit_short(),
            view_rep: handle(reader),
            view_type: reader.read_bit_long(),
            rotation: reader.read_bit_double(),
        }),
        "ASSOCVIEWBORDERACTIONPARAM"
        | "ASSOCVIEWREPACTIONPARAM"
        | "ASSOCVIEWSYMBOLACTIONPARAM"
        | "ASSOCVIEWSTYLEACTIONPARAM" => {
            let kind = match name.as_str() {
                "ASSOCVIEWREPACTIONPARAM" => AssocViewObjectActionParamKind::ViewRep,
                "ASSOCVIEWSYMBOLACTIONPARAM" => AssocViewObjectActionParamKind::ViewSymbol,
                "ASSOCVIEWSTYLEACTIONPARAM" => AssocViewObjectActionParamKind::ViewStyle,
                _ => AssocViewObjectActionParamKind::ViewBorder,
            };
            AssociativeData::ViewObjectActionParam(AssocViewObjectActionParam {
                kind,
                single_dependency: read_single_dependency(reader, version, dxf_version),
                class_version: reader.read_bit_short(),
            })
        }
        "ASSOCVIEWREPHATCHMANAGER" => {
            let compound = read_compound(reader, version, dxf_version, false);
            let class_version = reader.read_bit_short();
            let count = safe_count(reader.read_bit_long());
            let mut items = Vec::with_capacity(count as usize);
            for _ in 0..count {
                items.push(AssocViewRepHatchManagerItem {
                    first_id: reader.read_bit_long_long(),
                    second_id: reader.read_bit_long_long(),
                    status: reader.read_bit_long(),
                    parameter: handle(reader),
                });
            }
            AssociativeData::ViewRepHatchManager(AssocViewRepHatchManager {
                compound,
                class_version,
                items,
            })
        }
        "ASSOCVIEWREPHATCHACTIONPARAM" => {
            AssociativeData::ViewRepHatchActionParam(AssocViewRepHatchActionParam {
                single_dependency: read_single_dependency(reader, version, dxf_version),
                class_version: reader.read_bit_short(),
                normal: reader.read_3bit_double(),
                hatch_index: reader.read_bit_long(),
                flags: reader.read_bit_long(),
            })
        }
        "ASSOCVIEWLABELACTIONPARAM" => {
            AssociativeData::ViewLabelActionParam(AssocViewLabelActionParam {
                single_dependency: read_single_dependency(reader, version, dxf_version),
                class_version: reader.read_bit_short(),
                label_version: reader.read_bit_short(),
                offset: reader.read_2raw_double(),
                flag: reader.read_byte(),
            })
        }
        _ => return None,
    };
    Some(value)
}

/// TODO A8 (2026-10-02): the typed NURB3D (action_type 42) region
/// parser ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the measured grammar (see AssocNurb3dSubcurve for the
/// dissection record): a 12-bit header constant, the knot-tolerance
/// BD, a 4-bit constant, the 6 flag bits, BL num_knots + a constant
/// BL 8, the knot array (BD[]; 0.0 as the 2-bit short), the gap (BL
/// 0, BL 0, BL 8, BL num_ctrl, BL gap_b, BL 8) and the control-point
/// array (3BD[]) closing the region exactly. The parse gates on the
/// measured constants and exact closure; None on any deviation
/// (the caller then keeps the verbatim capture+replay net).
fn parse_nurb3d_region(bytes: &[u8], bit_len: u32) -> Option<AssocNurb3dSubcurve> {
    // a minimal MSB-first bit cursor over the captured window
    struct WindowBits<'a> {
        bytes: &'a [u8],
        pos: u32,
        len: u32,
    }

    impl WindowBits<'_> {
        fn bit(&mut self) -> Option<bool> {
            if self.pos >= self.len {
                return None;
            }
            let byte = *self.bytes.get((self.pos / 8) as usize)?;
            let value = (byte >> (7 - self.pos % 8)) & 1 == 1;
            self.pos += 1;
            Some(value)
        }

        fn raw(&mut self, count: u32) -> Option<u32> {
            let mut value = 0u32;
            for _ in 0..count {
                value = (value << 1) | self.bit()? as u32;
            }
            Some(value)
        }

        fn bl(&mut self) -> Option<i32> {
            match self.raw(2)? {
                0 => Some(self.raw(32)? as i32),
                1 => Some(self.raw(8)? as i32),
                2 => Some(0),
                _ => Some(256),
            }
        }

        fn bd(&mut self) -> Option<f64> {
            match self.raw(2)? {
                0 => {
                    let mut array = [0u8; 8];
                    for slot in array.iter_mut() {
                        *slot = self.raw(8)? as u8;
                    }
                    Some(f64::from_le_bytes(array))
                }
                1 => Some(1.0),
                2 => Some(0.0),
                _ => None,
            }
        }
    }

    let mut bits = WindowBits { bytes, pos: 0, len: bit_len };
    // the measured constants gate the typed path
    if bits.raw(12)? != 0x103 {
        return None;
    }
    let knot_tolerance = bits.bd()?;
    if bits.raw(4)? != 0x4 {
        return None;
    }
    let flags = bits.raw(6)? as u8;
    let num_knots = bits.bl()?;
    if bits.bl()? != 8 {
        return None;
    }
    if num_knots <= 0 || num_knots > 8192 {
        return None;
    }
    let mut knots = Vec::with_capacity(num_knots as usize);
    for _ in 0..num_knots {
        knots.push(bits.bd()?);
    }
    if bits.bl()? != 0 || bits.bl()? != 0 || bits.bl()? != 8 {
        return None;
    }
    let num_ctrl = bits.bl()?;
    let gap_b = bits.bl()?;
    if bits.bl()? != 8 {
        return None;
    }
    if num_ctrl <= 0 || num_ctrl > 8192 {
        return None;
    }
    let mut control_points = Vec::with_capacity(num_ctrl as usize);
    for _ in 0..num_ctrl {
        let x = bits.bd()?;
        let y = bits.bd()?;
        let z = bits.bd()?;
        control_points.push(Vector3::new(x, y, z));
    }
    // the region must close exactly ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the typed form's own gate
    if bits.pos != bit_len {
        return None;
    }
    Some(AssocNurb3dSubcurve {
        flags,
        knot_tolerance,
        knots,
        gap_b,
        control_points,
    })
}

/// Parse the composite (47) subcurve region ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â TODO A8 (2026-10-03).
///
/// The grammar (measured on the ExtrudePline/Extrude3DPoly/
/// RevolvePline/LoftMixed quads, all four eras; gold's spec has no
/// case 47 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the corpus is the authority): `BL num_segments`, then
/// per segment `BS kind` + the kind's own typed form. The measured
/// kinds: 23 (LINESEG3D ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â six BDs: absolute start 3BD + delta 3BD)
/// and 11 (ARC ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the twelve-BD arc form, plus the constant two-bit
/// `10` trailing form on the R2013+ frames, exactly like the
/// standalone ARC region). Gates on the segment count, the known
/// kinds, and exact closure; any deviation (a future kind, a variant
/// form) returns None and the caller rides the verbatim
/// capture+replay net.
fn parse_composite47_region(
    bytes: &[u8],
    bit_len: u32,
    r2013_plus: bool,
) -> Option<AssocCompositeSubcurve> {
    // a minimal MSB-first bit cursor over the captured window
    struct Cursor<'a> {
        bytes: &'a [u8],
        pos: u32,
        len: u32,
    }

    impl Cursor<'_> {
        fn bit(&mut self) -> Option<bool> {
            if self.pos >= self.len {
                return None;
            }
            let byte = *self.bytes.get((self.pos / 8) as usize)?;
            let value = (byte >> (7 - self.pos % 8)) & 1 == 1;
            self.pos += 1;
            Some(value)
        }

        fn raw(&mut self, count: u32) -> Option<u32> {
            let mut value = 0u32;
            for _ in 0..count {
                value = (value << 1) | self.bit()? as u32;
            }
            Some(value)
        }

        fn bl(&mut self) -> Option<i32> {
            match self.raw(2)? {
                0 => Some(self.raw(32)? as i32),
                1 => Some(self.raw(8)? as i32),
                2 => Some(0),
                _ => Some(256),
            }
        }

        fn bs(&mut self) -> Option<i32> {
            match self.raw(2)? {
                0 => Some(self.raw(16)? as i32),
                1 => Some(self.raw(8)? as i32),
                2 => Some(0),
                _ => Some(256),
            }
        }

        fn bd(&mut self) -> Option<f64> {
            match self.raw(2)? {
                0 => {
                    let mut array = [0u8; 8];
                    for slot in array.iter_mut() {
                        *slot = self.raw(8)? as u8;
                    }
                    Some(f64::from_le_bytes(array))
                }
                1 => Some(1.0),
                2 => Some(0.0),
                _ => None,
            }
        }
    }

    let mut bits = Cursor { bytes, pos: 0, len: bit_len };
    let num_segments = bits.bl()?;
    if num_segments <= 0 || num_segments > 1024 {
        return None;
    }
    let mut segments = Vec::with_capacity(num_segments as usize);
    for _ in 0..num_segments {
        let kind = bits.bs()?;
        match kind {
            23 => {
                let start = Vector3::new(bits.bd()?, bits.bd()?, bits.bd()?);
                let delta = Vector3::new(bits.bd()?, bits.bd()?, bits.bd()?);
                segments.push(AssocCompositeSegment::Line { start, delta });
            }
            11 => {
                let center = Vector3::new(bits.bd()?, bits.bd()?, bits.bd()?);
                let normal = Vector3::new(bits.bd()?, bits.bd()?, bits.bd()?);
                let x_axis = Vector3::new(bits.bd()?, bits.bd()?, bits.bd()?);
                let radius = bits.bd()?;
                let start_angle = bits.bd()?;
                let end_angle = bits.bd()?;
                if r2013_plus && bits.raw(2)? != 0b10 {
                    // the R2013+ arc tail is the constant `10` ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â any
                    // other two bits mean a variant: fall to the net.
                    return None;
                }
                segments.push(AssocCompositeSegment::Arc(AssocArcSubcurve {
                    center,
                    normal,
                    x_axis,
                    radius,
                    start_angle,
                    end_angle,
                }));
            }
            // an unmeasured kind (17, 42, 19, 27, ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¦) ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the net.
            _ => return None,
        }
    }
    // the region must close exactly ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the typed form's own gate.
    if bits.pos != bit_len {
        return None;
    }
    Some(AssocCompositeSubcurve { segments })
}