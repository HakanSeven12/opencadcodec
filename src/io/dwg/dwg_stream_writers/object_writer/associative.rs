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
            // §19 H8h-ext-12: the author codes the empty-values
            // dependency ref 4 (SoftPointer) — her h=392 (example_2007,
            // ACDBASSOCALIGNEDDIMACTIONBODY) writes (4.2.397) where this
            // emitted 5 (gold's dwg2.spec pab block declares 5; the wire
            // is the authority — the same spec-vs-wire split as the
            // H8h-ext-3 deps census and the H8h-ext-4 edge param).
            self.write_assoc_handle(DwgReferenceType::SoftPointer, value.dependency);
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
            // §19 H8h-ext-12: the author codes the child parameter ref 4
            // (SoftPointer) — her h=393 (example_2007,
            // ACDBASSOCOSNAPPOINTREFACTIONPARAM, child id=0) writes
            // (4.0.0) where this emitted 3 (gold's dwg2.spec child_param
            // block declares 3; the wire is the authority). The
            // secondary/tertiary refs (child id != 0) have no corpus
            // specimen — they keep gold's declared 3.
            self.write_assoc_handle(DwgReferenceType::SoftPointer, child.parameter);
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
        // §19 H8h-ext-6: the captured tail BLs in order (see
        // PersSubentManager::tail_bls).
        for bl in &value.tail_bls {
            self.writer.write_bit_long(*bl);
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

    /// Write `count` raw bits of `value`, MSB first — the measured
    /// NURB3D subcurve constants (TODO A8, 2026-10-02) are not
    /// whole-bitcode fields: the 12-bit header 0x103 and the 4-bit
    /// 0x4 straddle the byte/bitcode boundaries.
    fn write_raw_bits(&mut self, value: u32, count: u32) {
        for index in (0..count).rev() {
            self.writer.write_bit((value >> index) & 1 == 1);
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
        // §20 the R2018 record-identity packet (the rewrite-rejection
        // campaign): a DWG-read body replays its captured wire
        // verbatim when the class's modeled emission drifts from the
        // author's bytes — ACDBASSOCALIGNEDDIMACTIONBODY (520) first:
        // gold has no decoder for the class, the ODA spec documents
        // nothing, and the modeled emission loses form bits the model
        // never retained (example_2018 h=392: her bitsize 55, our
        // rewrite 53). The modeled emission stays the fallback for
        // DXF-built and programmatic records (no capture) and for
        // conversions that target another version.
        if self.write_wire_body(
            &object.wire_main,
            object.wire_main_bit_len,
            &object.wire_text,
            object.wire_text_bit_len,
            &object.wire_handles,
            object.wire_handles_bit_len,
            object.wire_dxf_version,
        ) {
            self.register_object(object.handle);
            return;
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
                // §19 H8h-ext-15: mirror the author's stream PRESENCE —
                // on a no-stream record (the captured wire_no_text_
                // stream; her R2013 geomdeps) the classname TU is
                // skipped so the merge emits has_strings: 0; records
                // whose author wrote a stream (the AC1021 corpus,
                // even empty-only) keep the normal TU emission.
                if !value.persistent_subent.wire_no_text_stream {
                    self.writer
                        .write_variable_text(&value.persistent_subent.class_name);
                }
                self.writer
                    .write_bit(value.persistent_subent.dependent_on_compound_object);
                // §19 H8h-ext-10: re-emit the captured persubent-id tail
                // (the undocumented main-stream bits after dependent_on_
                // compound_object); absent for DXF-built and edited
                // documents.
                if let Some(bytes) = &value.persistent_subent.tail_bits {
                    let bits = (value.persistent_subent.tail_bit_len as usize)
                        .min(bytes.len() * 8);
                    for index in 0..bits {
                        let byte = bytes[index / 8];
                        let bit = (byte >> (7 - index % 8)) & 1;
                        self.writer.write_bit(bit == 1);
                    }
                }
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
                // §19 H8h-ext-6: the gold dwg2.spec field order — see the
                // reader arm. The subents count is the vector length; the
                // cv2 tail (unknown_bl3 + B) emits only when captured.
                self.writer.write_bit_long(value.class_version);
                for marker in value.markers {
                    self.writer.write_bit_long(marker);
                }
                self.writer.write_bit_long(value.bl1);
                self.writer.write_bit_long(value.bl2);
                self.writer.write_bit_long(value.steps.len() as i32);
                for step in &value.steps {
                    self.writer.write_bit_long(*step);
                }
                self.writer.write_bit_long(value.subents.len() as i32);
                for item in &value.subents {
                    self.writer.write_bit_long(*item);
                }
                // §19 H8h-ext-6: the captured tail BLs in order, then
                // the trailing B (see the reader arm).
                for bl in &value.tail_bls {
                    self.writer.write_bit_long(*bl);
                }
                self.writer.write_bit(value.trailing_b);
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
                // §19 H8h-ext-4 + TODO B2 (2026-10-01): the subcurve
                // region. The typed kinds emit their measured BD
                // sequences; ARC (11) and ELLIPSE (17) append the
                // R2013+ frames' constant two-bit `10` trailing form
                // (BD 0.0 — the wire cannot name the field, see the
                // model docs; the pre-B2 writer omitted it, a latent
                // 2-bit drift on the conventional path). LINESEG3D
                // (23) closes at its sixth BD on every frame. The
                // untyped kinds — NURB3D (42), the gold-unknown 47 and
                // any 19/27 — replay their captured verbatim region on
                // a same-version write (the H8h-ext-8
                // `nodes_wire_main` pattern); cross-version conversions
                // and DXF-built records keep the pre-B2 emission (no
                // subcurve).
                let r2013_plus = self.version.r2013_plus(self.dxf_version);
                match &value.subcurve {
                    Some(AssocSubcurve::Arc(subcurve)) => {
                        self.writer.write_3bit_double(subcurve.center);
                        self.writer.write_3bit_double(subcurve.normal);
                        self.writer.write_3bit_double(subcurve.x_axis);
                        self.writer.write_bit_double(subcurve.radius);
                        self.writer.write_bit_double(subcurve.start_angle);
                        self.writer.write_bit_double(subcurve.end_angle);
                        if r2013_plus {
                            self.writer.write_bit_double(0.0);
                        }
                    }
                    Some(AssocSubcurve::Ellipse(subcurve)) => {
                        self.writer.write_3bit_double(subcurve.center);
                        self.writer.write_3bit_double(subcurve.major_axis);
                        self.writer.write_3bit_double(subcurve.minor_axis);
                        self.writer.write_bit_double(subcurve.major_radius);
                        self.writer.write_bit_double(subcurve.minor_radius);
                        self.writer.write_bit_double(subcurve.start_angle);
                        self.writer.write_bit_double(subcurve.end_angle);
                        if r2013_plus {
                            self.writer.write_bit_double(0.0);
                        }
                    }
                    Some(AssocSubcurve::LineSegment3d(subcurve)) => {
                        self.writer.write_3bit_double(subcurve.start_point);
                        self.writer.write_3bit_double(subcurve.end_point);
                    }
                    Some(AssocSubcurve::Nurb3d(subcurve)) => {
                        // TODO A8 (2026-10-02): the measured NURB3D
                        // grammar (see `AssocNurb3dSubcurve`). The form
                        // is ERA-STABLE — the 2007 and 2018 regions
                        // are bit-identical on every specimen — so the
                        // emission is not version-gated (unlike the
                        // ARC/ELLIPSE trailing form): a cross-version
                        // conversion re-emits the region correctly.
                        self.write_raw_bits(0x103, 12);
                        self.writer.write_bit_double(subcurve.knot_tolerance);
                        self.write_raw_bits(0x4, 4);
                        self.write_raw_bits(subcurve.flags as u32, 6);
                        self.writer
                            .write_bit_long(subcurve.knots.len() as i32);
                        self.writer.write_bit_long(8);
                        for knot in &subcurve.knots {
                            self.writer.write_bit_double(*knot);
                        }
                        self.writer.write_bit_long(0);
                        self.writer.write_bit_long(0);
                        self.writer.write_bit_long(8);
                        self.writer.write_bit_long(
                            subcurve.control_points.len() as i32,
                        );
                        self.writer.write_bit_long(subcurve.gap_b);
                        self.writer.write_bit_long(8);
                        for point in &subcurve.control_points {
                            self.writer.write_3bit_double(*point);
                        }
                    }
                    Some(AssocSubcurve::Composite(subcurve)) => {
                        // TODO A8 (2026-10-03): the measured composite
                        // (47) grammar (see `AssocCompositeSubcurve`):
                        // BL num_segments, then per segment BS kind +
                        // the kind's own form — 23: absolute start 3BD
                        // + delta 3BD; 11: the arc form with the R2013+
                        // constant two-bit trailing tail (the same
                        // per-frame rule as the standalone ARC region;
                        // the line segments close at their sixth BD on
                        // every frame). The wire region is otherwise
                        // era-stable, so the emission is ungated beyond
                        // the arc tail.
                        self.writer
                            .write_bit_long(subcurve.segments.len() as i32);
                        for segment in &subcurve.segments {
                            match segment {
                                AssocCompositeSegment::Line { start, delta } => {
                                    self.writer.write_bit_short(23);
                                    self.writer.write_3bit_double(*start);
                                    self.writer.write_3bit_double(*delta);
                                }
                                AssocCompositeSegment::Arc(arc) => {
                                    self.writer.write_bit_short(11);
                                    self.writer.write_3bit_double(arc.center);
                                    self.writer.write_3bit_double(arc.normal);
                                    self.writer.write_3bit_double(arc.x_axis);
                                    self.writer.write_bit_double(arc.radius);
                                    self.writer
                                        .write_bit_double(arc.start_angle);
                                    self.writer.write_bit_double(arc.end_angle);
                                    if r2013_plus {
                                        self.writer.write_bit_double(0.0);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        if let Some(bytes) = &value.subcurve_wire {
                            if value.subcurve_wire_dxf_version
                                == Some(self.dxf_version)
                            {
                                let bits = (value.subcurve_wire_bit_len as usize)
                                    .min(bytes.len() * 8);
                                for index in 0..bits {
                                    let byte = bytes[index / 8];
                                    let bit = (byte >> (7 - index % 8)) & 1;
                                    self.writer.write_bit(bit == 1);
                                }
                            }
                        }
                    }
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
                // §19 H8h-ext-8: DWG-read AC1021 records re-emit their
                // captured node region verbatim — the per-node
                // class-name TUs into the text stream (walk order),
                // the main bits, then the captured handle tail (the
                // per-node geometry-dependency reads and her closing
                // 1s pad). The modeled REPEAT stays the fallback for
                // DXF-built and programmatic records (no capture).
                if value.nodes_wire_main.is_some() {
                    for name in &value.nodes_wire_names {
                        self.writer.write_variable_text(name);
                    }
                    // §19 H8h-ext-14: the R2010+ raw text-region
                    // re-emission (the AC1021 path rides the names
                    // loop above; the R2010+ capture never decoded
                    // the stream, so its bits replay verbatim).
                    if let Some(bytes) = &value.nodes_wire_text {
                        let bits = (value.nodes_wire_text_bit_len as usize)
                            .min(bytes.len() * 8);
                        for index in 0..bits {
                            let byte = bytes[index / 8];
                            let bit = (byte >> (7 - index % 8)) & 1;
                            self.writer.write_text_bit(bit == 1);
                        }
                    }
                    if let Some(bytes) = &value.nodes_wire_main {
                        let bits = (value.nodes_wire_main_bit_len as usize)
                            .min(bytes.len() * 8);
                        for index in 0..bits {
                            let byte = bytes[index / 8];
                            let bit = (byte >> (7 - index % 8)) & 1;
                            self.writer.write_bit(bit == 1);
                        }
                    }
                    if let Some(bytes) = &value.nodes_wire_handles {
                        // §19 H8h-ext-13: re-create the author's
                        // closing 1s pad explicitly (the ext-12
                        // lesson): extend the captured bits to the
                        // byte boundary with 1s so the merged writer's
                        // own zero-pad never fires — the DWG
                        // final-partial-byte convention (§19.4.C),
                        // bit-continuous for the TwoStream eras and
                        // the appended-buffer close for AC1021 alike.
                        // A no-op when the capture is byte-aligned
                        // (the AC1021 corpus specimens).
                        let mut bytes = bytes.clone();
                        let mut bit_len = value.nodes_wire_handles_bit_len;
                        let rem = bit_len % 8;
                        if rem != 0 {
                            let pad = 8 - rem;
                            if let Some(last) = bytes.last_mut() {
                                *last |= ((1u16 << pad) - 1) as u8;
                            }
                            bit_len += pad;
                        }
                        self.writer.write_handle_bits(&bytes, bit_len);
                    }
                } else {
                    for node in &value.nodes {
                        self.write_constraint_node_common(node);
                    }
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
