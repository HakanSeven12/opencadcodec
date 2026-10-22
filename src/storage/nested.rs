//! Embedded model content shares capture rules, without database identity repair.
use super::Capture;
use crate::{
    entities::{EmbeddedEntity, MText, SurfaceData},
    objects::{DynamicBlockData, SolidHistoryOperation},
};

impl Capture<'_> {
    pub(super) fn mtext(&mut self, text: Option<&mut MText>, path: &str) {
        if let Some(text) = text {
            self.common(&mut text.common, &format!("{path}/common"));
        }
    }
    fn embedded(&mut self, entity: &mut EmbeddedEntity, path: &str) {
        macro_rules! common { ($($variant:ident),*)=> { match entity {
            $(EmbeddedEntity::$variant(value)=>self.common(&mut value.common,&format!("{path}/common")),)*
            // These own standalone geometry bytes, with no EntityCommon.
            EmbeddedEntity::Body{..} | EmbeddedEntity::Unknown{..}=>{},
        } } }
        common!(Point, Line, Arc, Circle, Ellipse, Spline, LwPolyline, Region, Ray, XLine);
    }
    fn optional(&mut self, entity: Option<&mut EmbeddedEntity>, path: &str) {
        if let Some(entity) = entity {
            self.embedded(entity, path);
        }
    }
    fn geometry_list(&mut self, entities: &mut [EmbeddedEntity], path: &str) {
        for (index, entity) in entities.iter_mut().enumerate() {
            self.embedded(entity, &format!("{path}/{index}"));
        }
    }
    pub(super) fn surface(&mut self, data: &mut SurfaceData, path: &str) {
        match data {
            SurfaceData::Extruded { sweep_entity, .. } => {
                self.optional(sweep_entity.as_mut(), &format!("{path}/sweep_entity"))
            }
            SurfaceData::Lofted {
                cross_section_entities,
                guide_entities,
                path_entity,
                ..
            } => {
                self.geometry_list(
                    cross_section_entities,
                    &format!("{path}/cross_section_entities"),
                );
                self.geometry_list(guide_entities, &format!("{path}/guide_entities"));
                self.optional(path_entity.as_mut(), &format!("{path}/path_entity"));
            }
            SurfaceData::Revolved { revolve_entity, .. } => {
                self.optional(revolve_entity.as_mut(), &format!("{path}/revolve_entity"))
            }
            SurfaceData::Swept {
                sweep_entity,
                path_entity,
                ..
            } => {
                self.optional(sweep_entity.as_mut(), &format!("{path}/sweep_entity"));
                self.optional(path_entity.as_mut(), &format!("{path}/path_entity"));
            }
            SurfaceData::Generic | SurfaceData::Plane { .. } | SurfaceData::Nurb { .. } => {}
        }
    }
    pub(super) fn dynamic(&mut self, data: &mut DynamicBlockData, path: &str) {
        if let DynamicBlockData::SolidHistoryNode(operation) = data {
            let path = format!("{path}/SolidHistoryNode");
            match operation {
                SolidHistoryOperation::Sweep(value) | SolidHistoryOperation::Extrusion(value) => {
                    self.optional(value.sweep_entity.as_mut(), &format!("{path}/sweep_entity"));
                    self.optional(value.path_entity.as_mut(), &format!("{path}/path_entity"));
                }
                SolidHistoryOperation::Loft(value) => {
                    self.geometry_list(
                        &mut value.cross_sections,
                        &format!("{path}/cross_sections"),
                    );
                    self.geometry_list(&mut value.guides, &format!("{path}/guides"));
                    self.optional(value.path_entity.as_mut(), &format!("{path}/path_entity"));
                    if let Some(parameters) = &mut value.parameters {
                        self.optional(
                            parameters.path_entity.as_mut(),
                            &format!("{path}/parameters/path_entity"),
                        );
                    }
                }
                SolidHistoryOperation::Revolve(value) => {
                    self.optional(value.sweep_entity.as_mut(), &format!("{path}/sweep_entity"))
                }
                SolidHistoryOperation::Unknown
                | SolidHistoryOperation::Box(_)
                | SolidHistoryOperation::Wedge(_)
                | SolidHistoryOperation::Sphere(_)
                | SolidHistoryOperation::Cylinder(_)
                | SolidHistoryOperation::Cone(_)
                | SolidHistoryOperation::Pyramid(_)
                | SolidHistoryOperation::Torus(_)
                | SolidHistoryOperation::Boolean(_)
                | SolidHistoryOperation::Brep(_)
                | SolidHistoryOperation::Fillet(_)
                | SolidHistoryOperation::Chamfer(_) => {}
            }
        }
    }
}
