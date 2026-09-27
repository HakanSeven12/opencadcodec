use crate::io::dwg::dwg_reference_type::DwgReferenceType;
use crate::objects::*;
use crate::types::Handle;

use super::DwgObjectWriter;

impl<'a> DwgObjectWriter<'a> {
    fn write_assoc_handle(&mut self, kind: DwgReferenceType, value: Handle) {
        self.writer.write_handle(kind, value.value());
    }

    fn write_assoc_handles(&mut self, kind: DwgReferenceType, values: &[Handle]) {
        for value in values {
            self.write_assoc_handle(kind, *value);
        }
    }

    fn write_assoc_eval(&mut self, value: &AssocEvalVariant) {
        self.writer.write_bit_short(value.code);
        match &value.value {
            AssocEvalValue::None => {}
            AssocEvalValue::Real(value) => self.writer.write_bit_double(*value),
            AssocEvalValue::Long(value) => self.writer.write_bit_long(*value),
            AssocEvalValue::Short(value) => self.writer.write_bit_short(*value),
            AssocEvalValue::Byte(value) => self.writer.write_byte(*value),
            AssocEvalValue::Text(value) => self.writer.write_variable_text(value),
            AssocEvalValue::Handle(value) => {
                self.write_assoc_handle(DwgReferenceType::HardPointer, *value)
            }
        }
    }

    fn write_assoc_value_param(&mut self, value: &AssocValueParam) {
        self.writer.write_bit_long(value.class_version);
        self.writer.write_variable_text(&value.name);
        self.writer.write_bit_long(value.unit_type);
        self.writer.write_bit_long(value.variables.len() as i32);
        for variable in &value.variables {
            self.write_assoc_eval(&variable.value);
            self.write_assoc_handle(DwgReferenceType::SoftPointer, variable.handle);
        }
        self.write_assoc_handle(
            DwgReferenceType::SoftPointer,
            value.controlled_object_dependency,
        );
    }

    fn write_assoc_values(&mut self, values: &[AssocValueParam]) {
        for value in values {
            self.write_assoc_value_param(value);
        }
    }

    fn write_assoc_dependency(&mut self, value: &AssocDependency) {
        self.writer.write_bit_short(value.class_version);
        self.writer.write_bit_long(value.status);
        self.writer.write_bit(value.is_read_dependency);
        self.writer.write_bit(value.is_write_dependency);
        self.writer.write_bit(value.is_attached_to_object);
        self.writer.write_bit(value.is_delegating_to_owning_action);
        self.writer.write_bit_long(value.order);
        self.write_assoc_handle(DwgReferenceType::SoftPointer, value.dependent_on);
        self.writer.write_bit(value.name.is_some());
        if let Some(name) = &value.name {
            self.writer.write_variable_text(name);
        }
        self.write_assoc_handle(DwgReferenceType::SoftPointer, value.read_dependency);
        self.write_assoc_handle(DwgReferenceType::SoftPointer, value.node);
        self.write_assoc_handle(DwgReferenceType::HardOwnership, value.dependency_body);
        self.writer.write_bit_long(value.dependency_body_id);
    }

    fn write_assoc_action(&mut self, value: &AssocAction) {
        self.writer.write_bit_short(value.class_version);
        self.writer.write_bit_long(value.geometry_status);
        self.write_assoc_handle(DwgReferenceType::SoftPointer, value.owning_network);
        self.write_assoc_handle(DwgReferenceType::HardOwnership, value.action_body);
        self.writer.write_bit_long(value.action_index);
        self.writer.write_bit_long(value.max_dependency_index);
        self.writer.write_bit_long(value.dependencies.len() as i32);
        for dependency in &value.dependencies {
            self.writer.write_bit(dependency.is_owned);
            self.write_assoc_handle(
                if dependency.is_owned {
                    DwgReferenceType::HardOwnership
                } else {
                    DwgReferenceType::SoftPointer
                },
                dependency.dependency,
            );
        }
        if value.class_version > 1 {
            self.writer.write_bit_short(0);
            self.writer
                .write_bit_long(value.owned_parameters.len() as i32);
            self.write_assoc_handles(DwgReferenceType::HardOwnership, &value.owned_parameters);
            self.writer.write_bit_short(0);
            self.writer.write_bit_long(value.values.len() as i32);
            self.write_assoc_values(&value.values);
        }
    }

    fn write_assoc_action_param(&mut self, value: &AssocActionParam) {
        self.writer.write_bit_short(value.is_r2013);
        if self.version.r2013_plus(self.dxf_version) {
            self.writer.write_bit_long(value.version);
        }
        self.writer.write_variable_text(&value.name);
    }

    fn write_assoc_action_body(&mut self, value: &AssocActionBody) {
        self.writer.write_bit_long(value.version);
    }

    fn write_assoc_parameter_body(&mut self, value: &AssocParamBasedActionBody) {
        if self.version.r2013_plus(self.dxf_version) {
            return;
        }
        self.writer.write_bit_long(value.version);
        self.writer.write_bit_long(value.minor);
        self.writer.write_bit_long(value.dependencies.len() as i32);
        // §19 H8h-ext-3: the author codes the pab dependency refs 3
        // (gold census corpus-wide: deps {3: 27}, never 4/5).
        self.write_assoc_handles(DwgReferenceType::HardOwnership, &value.dependencies);
        self.writer.write_bit_long(value.marker);
        self.writer.write_bit_long(value.values.len() as i32);
        if value.values.is_empty() {
            self.writer.write_bit_long(value.empty_value_marker);
            self.write_assoc_handle(DwgReferenceType::HardPointer, value.dependency);
        }
        self.write_assoc_values(&value.values);
    }

    fn write_assoc_surface(&mut self, value: &AssocSurfaceActionBody) {
        self.write_assoc_action_body(&value.action_body);
        self.write_assoc_parameter_body(&value.parameter_body);
        self.writer.write_bit_long(value.surface_body.version);
        // A NULL sab.assocdep means no slot was read: the truncated
        // 2004/Surface ORIG record physically ends one bit into where the
        // slot's form byte starts, and gold's overflowing bit_read_H prints
        // the [0,0] null pair. Echo that wire state — fabricating an
        // explicit (5,0) null form would make gold decode a real slot
        // ([5,0,0,0] handle-tuple print) and diverge from the orig pair.
        if value.surface_body.dependency.is_valid() {
            // §19 H8h-ext-3: the author codes sab.assocdep 4 (SoftPointer)
            // — gold census corpus-wide: assocdep {4: 23}, never 5.
            self.write_assoc_handle(DwgReferenceType::SoftPointer, value.surface_body.dependency);
        }
        self.writer
            .write_bit(value.surface_body.is_semi_associative);
        self.writer.write_bit_long(value.surface_body.marker);
        self.writer.write_bit(value.surface_body.is_semi_override);
        self.writer.write_bit_short(value.surface_body.grip_status);
        self.writer.write_bit_long(value.path_status);
        match value.kind {
            AssocSurfaceActionKind::Network
            | AssocSurfaceActionKind::Patch
            | AssocSurfaceActionKind::EdgeChamfer
            | AssocSurfaceActionKind::EdgeFillet => {}
            _ => self.writer.write_bit_long(value.class_version),
        }
        match value.kind {
            AssocSurfaceActionKind::Extend => self.writer.write_byte(value.option),
            AssocSurfaceActionKind::Offset => self.writer.write_bit(value.flags[0]),
            AssocSurfaceActionKind::Trim => {
                self.writer.write_bit(value.flags[0]);
                self.writer.write_bit(value.flags[1]);
                self.writer.write_bit_double(value.distance);
            }
            AssocSurfaceActionKind::Blend => {
                self.writer.write_bit(value.flags[0]);
                self.writer.write_bit(value.flags[1]);
                self.writer.write_bit(value.flags[2]);
                self.writer.write_bit_short(value.status);
                self.writer.write_bit(value.flags[3]);
                self.writer.write_bit(value.flags[4]);
                self.writer.write_bit_short(value.secondary_status);
            }
            AssocSurfaceActionKind::Fillet => {
                self.writer.write_bit_short(value.status);
                self.writer.write_2raw_double(value.first_point);
                self.writer.write_2raw_double(value.second_point);
            }
            // §19 H8h-ext-3: the author's REVOLVED body carries one
            // trailing B(0) after class_version (the RevolveM wire: her
            // main stream one bit longer, the extra '0' at the end; the
            // only corpus specimen — no other Revolve file has the class).
            AssocSurfaceActionKind::Revolved => {
                self.writer.write_bit(false);
            }
            _ => {}
        }
    }

    fn write_assoc_annotation_base(&mut self, value: &AssocAnnotationBase) {
        if self.version.r2010_plus() {
            self.writer.write_bit_short(value.version);
            self.write_assoc_handle(DwgReferenceType::HardPointer, value.dependency);
        } else {
            self.write_assoc_action_body(&value.action_body);
            self.write_assoc_parameter_body(&value.parameter_body);
        }
    }

    fn write_assoc_annotation(&mut self, value: &AssocAnnotationActionBody) {
        if value.kind == AssocAnnotationKind::RestoreEntityState {
            self.write_assoc_action_body(&value.action_body);
            self.writer.write_bit_long(value.class_version);
            self.write_assoc_handle(DwgReferenceType::HardPointer, value.entity);
            return;
        }
        self.write_assoc_annotation_base(&value.annotation);
        match value.kind {
            AssocAnnotationKind::ThreePointAngularDimension
            | AssocAnnotationKind::RotatedDimension => {
                self.writer.write_bit_short(value.class_version as i16)
            }
            _ => self.writer.write_bit_long(value.class_version),
        }
        match value.kind {
            AssocAnnotationKind::MLeader => {
                self.writer.write_bit_long(value.actions.len() as i32);
                for action in &value.actions {
                    self.writer.write_bit_long(action.dependency_id);
                    self.write_assoc_handle(DwgReferenceType::HardPointer, action.dependency);
                }
            }
            AssocAnnotationKind::AlignedDimension => {
                self.write_assoc_handle(DwgReferenceType::SoftPointer, value.read_node);
                self.write_assoc_handle(DwgReferenceType::SoftPointer, value.dimension_node);
            }
            AssocAnnotationKind::ThreePointAngularDimension => {
                self.write_assoc_handle(DwgReferenceType::SoftPointer, value.read_node);
                self.write_assoc_handle(DwgReferenceType::SoftPointer, value.dimension_node);
                self.write_assoc_handle(DwgReferenceType::HardPointer, value.dependency);
            }
            AssocAnnotationKind::OrdinateDimension | AssocAnnotationKind::RotatedDimension => {
                self.write_assoc_handle(DwgReferenceType::HardPointer, value.read_node);
                self.write_assoc_handle(DwgReferenceType::HardPointer, value.dimension_node);
            }
            AssocAnnotationKind::RestoreEntityState => {}
        }
    }

    fn write_assoc_single_dependency(&mut self, value: &AssocSingleDependencyActionParam) {
        self.write_assoc_action_param(&value.action_param);
        self.writer.write_bit_long(value.dependency_class_version);
        self.write_assoc_handle(DwgReferenceType::SoftPointer, value.dependency);
        self.writer.write_bit_long(value.class_version);
    }

    fn write_assoc_compound(&mut self, value: &AssocCompoundActionParam) {
        self.write_assoc_action_param(&value.action_param);
        self.writer.write_bit_short(value.class_version);
        self.writer.write_bit_short(value.status);
        self.writer.write_bit_long(value.parameters.len() as i32);
        // §19 H8h-ext-3: the author codes the compound parameter refs 3
        // (gold census corpus-wide: params {3: 41}, never 4/5).
        self.write_assoc_handles(DwgReferenceType::HardOwnership, &value.parameters);
        if let Some(child) = &value.child_parameter {
            self.writer.write_bit_short(child.status);
            self.writer.write_bit_long(child.id);
            self.write_assoc_handle(DwgReferenceType::HardOwnership, child.parameter);
            if child.id != 0 {
                self.write_assoc_handle(DwgReferenceType::HardOwnership, child.secondary_parameter);
                self.writer.write_bit_long(child.marker);
                self.write_assoc_handle(DwgReferenceType::HardOwnership, child.tertiary_parameter);
            }
        }
    }

    fn write_assoc_array_body(&mut self, value: &AssocArrayActionBody) {
        self.write_assoc_action_body(&value.action_body);
        self.write_assoc_parameter_body(&value.parameter_body);
        self.writer.write_bit_long(value.version);
        self.writer.write_variable_text(&value.parameter_block);
        for item in value.transform {
            self.writer.write_bit_double(item);
        }
    }

    fn write_assoc_array_parameters(&mut self, value: &AssocArrayParameters) {
        self.writer.write_bit_long(value.version);
        self.writer.write_bit_long(value.items.len() as i32);
        self.writer.write_variable_text(&value.class_name);
        for item in &value.items {
            self.writer.write_bit_long(item.class_version);
            for location in item.location {
                self.writer.write_bit_long(location);
            }
            self.writer.write_bit_long(item.flags);
            if item.uses_default_transform {
                self.writer.write_3bit_double(item.x_direction);
            } else {
                for matrix_value in item.transform {
                    self.writer.write_bit_double(matrix_value);
                }
            }
            if let Some(matrix) = item.relative_transform {
                for matrix_value in matrix {
                    self.writer.write_bit_double(matrix_value);
                }
            }
            if let Some(first) = item.first_handle {
                self.write_assoc_handle(DwgReferenceType::SoftPointer, first);
            }
            if item.flags & 0x10 != 0 {
                self.write_assoc_handle(
                    DwgReferenceType::SoftPointer,
                    item.second_handle.unwrap_or(Handle::NULL),
                );
            }
        }
        self.writer.write_bit_long(value.item_count);
        self.writer.write_bit_long(value.row_count);
        self.writer.write_bit_long(value.level_count);
    }

    fn write_dimension_association(&mut self, value: &AssocDimensionAssociation) {
        self.writer.write_bit_long(value.associativity);
        self.writer.write_bit(value.trans_space);
        self.writer.write_byte(value.rotated_type);
        self.write_assoc_handle(DwgReferenceType::SoftPointer, value.dimension);
        for slot in 0..4 {
            if value.associativity & (1 << slot) == 0 {
                continue;
            }
            let fallback = AssocDimensionReference::default();
            let stored = value
                .references
                .get(slot)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let references = if stored.is_empty() {
                std::slice::from_ref(&fallback)
            } else {
                stored
            };
            for (index, reference) in references.iter().enumerate() {
                self.writer.write_variable_text(&reference.class_name);
                self.writer.write_byte(reference.osnap_type);
                self.writer.write_bit_long(reference.xrefs.len() as i32);
                self.write_assoc_handles(DwgReferenceType::SoftPointer, &reference.xrefs);
                if reference.osnap_type != 0 {
                    self.writer.write_bit_long(reference.main_subent_type);
                    self.writer.write_bit_long(reference.main_gs_marker);
                    self.writer
                        .write_bit_long(reference.xref_paths.len() as i32);
                    for path in &reference.xref_paths {
                        self.writer.write_variable_text(path);
                    }
                }
                self.writer.write_bit_double(reference.osnap_distance);
                self.writer.write_3bit_double(reference.osnap_point);
                if reference.osnap_type == 6 || reference.osnap_type == 11 {
                    self.writer
                        .write_bit_long(reference.intersection_objects.len() as i32);
                    self.write_assoc_handles(
                        DwgReferenceType::HardPointer,
                        &reference.intersection_objects,
                    );
                    self.writer
                        .write_bit_long(reference.intersection_subent_type);
                    self.writer.write_bit_long(reference.intersection_gs_marker);
                    self.writer
                        .write_bit_long(reference.intersection_xref_paths.len() as i32);
                    for path in &reference.intersection_xref_paths {
                        self.writer.write_variable_text(path);
                    }
                }
                self.writer.write_bit(index + 1 < references.len());
            }
        }
    }

    fn write_static_pers_subent_manager(&mut self, value: &PersSubentManager) {
        self.writer.write_bit_long(value.class_version);
        self.writer.write_bit_long(value.marker_zero);
        self.writer.write_bit_long(value.marker_two);
        self.writer.write_bit_long(value.associative_step_count);
        self.writer.write_bit_long(value.associative_subent_count);
        self.writer.write_bit_long(value.steps.len() as i32);
        for step in &value.steps {
            self.writer.write_bit_long(*step);
        }
        if value.associative_subent_count != 0 || !value.subents.is_empty() {
            self.writer.write_bit_long(value.subents.len() as i32);
            for subent in &value.subents {
                self.writer.write_bit_long(*subent);
            }
        }
    }

    fn write_constraint_node_common(&mut self, node: &AssocConstraintNode) {
        self.writer.write_bit_long(node.node_id);
        if !self.version.r2013_plus(self.dxf_version) {
            self.writer.write_byte(node.status);
        }
        self.writer.write_bit_long(node.connections.len() as i32);
        for connection in &node.connections {
            self.writer.write_bit_long(*connection);
        }
        if self.version.r2013_plus(self.dxf_version) {
            self.writer.write_byte(node.status);
        }
    }

    pub(super) fn write_associative_object(&mut self, object: &AssociativeObject) {
        let canonical = associative_canonical_name(&object.dxf_name);
        let prefixed = format!("ACDB{canonical}");
        let type_code = self
            .document
            .classes
            .get_by_name(&object.dxf_name)
            .or_else(|| self.document.classes.get_by_name(&prefixed))
            .map(|class| class.class_number)
            .unwrap_or(500);
        if matches!(&object.data, AssociativeData::ViewRepActionBody(_)) {
            self.write_common_non_entity_data_relative_owner(
                type_code,
                object.handle,
                object.owner,
                &object.reactors,
                &object.xdictionary_handle,
            );
        } else {
            self.write_common_non_entity_data(
                type_code,
                object.handle,
                object.owner,
                &object.reactors,
                &object.xdictionary_handle,
            );
        }
        match &object.data {
            AssociativeData::Unknown => {}
            AssociativeData::Dependency(value) => self.write_assoc_dependency(value),
            AssociativeData::ValueDependency(value) => {
                self.write_assoc_dependency(&value.dependency);
                self.writer.write_bit_long(value.class_version);
                self.writer.write_variable_text(&value.name);
                self.write_assoc_eval(&value.value);
            }
            AssociativeData::GeomDependency(value) => {
                self.write_assoc_dependency(&value.dependency);
                self.writer.write_bit_short(value.class_version);
                self.writer.write_bit(value.enabled);
                self.writer
                    .write_variable_text(&value.persistent_subent.class_name);
                self.writer
                    .write_bit(value.persistent_subent.dependent_on_compound_object);
            }
            AssociativeData::SurfaceActionBody(value) => self.write_assoc_surface(value),
            AssociativeData::Action(value) => self.write_assoc_action(value),
            AssociativeData::Network(value) => {
                self.write_assoc_action(&value.action);
                self.writer.write_bit_short(value.network_version);
                self.writer.write_bit_long(value.network_action_index);
                self.writer.write_bit_long(value.actions.len() as i32);
                for action in &value.actions {
                    self.writer.write_bit(action.is_owned);
                    self.write_assoc_handle(
                        if action.is_owned {
                            DwgReferenceType::HardOwnership
                        } else {
                            DwgReferenceType::SoftPointer
                        },
                        action.dependency,
                    );
                }
                self.writer.write_bit_long(value.owned_actions.len() as i32);
                self.write_assoc_handles(DwgReferenceType::SoftPointer, &value.owned_actions);
            }
            AssociativeData::AnnotationActionBody(value) => self.write_assoc_annotation(value),
            AssociativeData::PersSubentManager(value) => {
                self.writer.write_bit_long(value.class_version);
                for marker in value.markers {
                    self.writer.write_bit_long(marker);
                }
                self.writer.write_bit_long(value.steps.len() as i32);
                for step in &value.steps {
                    self.writer.write_bit_long(*step);
                }
                self.writer.write_bit_long(value.subent_count);
                for item in &value.subent_data {
                    self.writer.write_bit_long(*item);
                }
                self.writer.write_bit(value.final_flag);
            }
            AssociativeData::EdgeActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                // §19 H8h-ext-4: the author codes the param ref 4
                // (SoftPointer) — the specimens' null params are (4.0.0)
                // on her wire (gold's spec block declares 3, but the wire
                // evidence is uniform across all seven records).
                self.write_assoc_handle(DwgReferenceType::SoftPointer, value.parameter);
                self.writer.write_bit(value.has_action);
                self.writer.write_bit_long(value.action_type);
                // §19 H8h-ext-4: the subcurve geometry region — twelve BDs
                // in the specimen-attested order (center, normal, x-axis,
                // radius, start/end angles). Emitted only when the reader
                // captured one (the attested ARC form); DXF-built records
                // keep the pre-H8h-ext-4 emission.
                if let Some(subcurve) = &value.subcurve {
                    self.writer.write_3bit_double(subcurve.center);
                    self.writer.write_3bit_double(subcurve.normal);
                    self.writer.write_3bit_double(subcurve.x_axis);
                    self.writer.write_bit_double(subcurve.radius);
                    self.writer.write_bit_double(subcurve.start_angle);
                    self.writer.write_bit_double(subcurve.end_angle);
                }
            }
            AssociativeData::ConstraintGroup(value) => {
                self.write_assoc_action(&value.action);
                self.writer.write_bit_long(value.version);
                self.writer.write_bit(value.flag);
                for point in value.work_plane {
                    self.writer.write_3bit_double(point);
                }
                self.write_assoc_handle(DwgReferenceType::HardOwnership, value.dependency);
                self.writer.write_bit_long(value.actions.len() as i32);
                self.write_assoc_handles(DwgReferenceType::HardOwnership, &value.actions);
                // gold dwg2.spec ASSOC2DCONSTRAINTGROUP: num_nodes BL
                // then the FLAT per-node REPEAT (nodeid BLd + status RC
                // era-gated around num_connections + the BL vector) —
                // write_constraint_node_common is exactly that shape;
                // mirrors the reader.
                self.writer.write_bit_long(value.nodes.len() as i32);
                for node in &value.nodes {
                    self.write_constraint_node_common(node);
                }
            }
            AssociativeData::Variable(value) => {
                self.write_assoc_action(&value.action);
                self.writer.write_bit_long(value.class_version);
                self.writer.write_variable_text(&value.name);
                self.writer.write_variable_text(&value.expression);
                self.writer.write_variable_text(&value.evaluator);
                self.writer.write_variable_text(&value.description);
                self.write_assoc_eval(&value.value);
                self.writer.write_bit(value.has_cached_value);
                if value.has_cached_value {
                    self.writer.write_variable_text(&value.cached_value);
                }
                self.writer.write_bit(value.flag);
                self.writer.write_bit_long(value.reserved);
            }
            AssociativeData::ActionParam(value) => self.write_assoc_action_param(value),
            AssociativeData::CompoundActionParam(value)
            | AssociativeData::PointRefActionParam(value) => self.write_assoc_compound(value),
            AssociativeData::OsnapPointRefActionParam(value) => {
                self.write_assoc_compound(&value.compound);
                self.writer.write_bit_short(value.status);
                self.writer.write_byte(value.osnap_mode);
                self.writer.write_bit_double(value.parameter);
            }
            AssociativeData::ObjectActionParam(value) => self.write_assoc_single_dependency(value),
            AssociativeData::PathActionParam(value) => {
                self.write_assoc_compound(&value.compound);
                self.writer.write_bit_long(value.version);
            }
            AssociativeData::DimDependencyBody(value) => {
                self.writer.write_bit_short(value.dependency_body_version);
                self.writer.write_bit_short(value.base_version);
                self.writer.write_variable_text(&value.name);
                self.writer.write_bit_short(value.class_version);
            }
            AssociativeData::FaceActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                self.writer.write_bit_long(value.index);
            }
            AssociativeData::VertexActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                self.writer.write_3bit_double(value.point);
            }
            AssociativeData::AsmBodyActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                self.write_acis_data(
                    value.point_of_reference,
                    &value.acis_data,
                    &value.wires,
                    &value.silhouettes,
                );
                if value.acis_data.is_binary {
                    self.write_assoc_handle(DwgReferenceType::SoftPointer, value.history);
                }
            }
            AssociativeData::ArrayParameters(value) => self.write_assoc_array_parameters(value),
            AssociativeData::ArrayActionBody(value) => self.write_assoc_array_body(value),
            AssociativeData::ArrayModifyActionBody(value) => {
                self.write_assoc_array_body(&value.body);
                self.writer.write_bit_short(value.status);
                self.writer
                    .write_bit_long(value.item_locations.len() as i32);
                for location in &value.item_locations {
                    for item in location {
                        self.writer.write_bit_long(*item);
                    }
                }
            }
            AssociativeData::DimensionAssociation(value) => self.write_dimension_association(value),
            AssociativeData::PersSubentManagerStatic(value) => {
                self.write_static_pers_subent_manager(value)
            }
            AssociativeData::ViewRepActionBody(value) => {
                self.write_assoc_action_body(&value.action_body);
                self.writer.write_bit_short(value.class_version);
                self.write_assoc_handle(DwgReferenceType::HardOwnership, value.view_rep);
                self.writer.write_bit_long(value.view_type);
                self.writer.write_bit_double(value.rotation);
            }
            AssociativeData::ViewObjectActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                self.writer.write_bit_short(value.class_version);
            }
            AssociativeData::ViewRepHatchManager(value) => {
                self.write_assoc_compound(&value.compound);
                self.writer.write_bit_short(value.class_version);
                self.writer.write_bit_long(value.items.len() as i32);
                for item in &value.items {
                    self.writer.write_bit_long_long(item.first_id);
                    self.writer.write_bit_long_long(item.second_id);
                    self.writer.write_bit_long(item.status);
                    self.write_assoc_handle(DwgReferenceType::SoftPointer, item.parameter);
                }
            }
            AssociativeData::ViewRepHatchActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                self.writer.write_bit_short(value.class_version);
                self.writer.write_3bit_double(value.normal);
                self.writer.write_bit_long(value.hatch_index);
                self.writer.write_bit_long(value.flags);
            }
            AssociativeData::ViewLabelActionParam(value) => {
                self.write_assoc_single_dependency(&value.single_dependency);
                self.writer.write_bit_short(value.class_version);
                self.writer.write_bit_short(value.label_version);
                self.writer.write_2raw_double(value.offset);
                self.writer.write_byte(value.flag);
            }
        }
        self.register_object(object.handle);
    }
}
