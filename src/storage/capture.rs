//! Capture only: no owner repair, inferred application/layer names or writeback decisions.
use super::{
    records::{CapturedExtendedData, CapturedXDataValue, RecordBody},
    CaptureEvidence, CaptureReport, ContentCoverage,
};
use crate::{
    entities::{EntityCommon, EntityType, ExtendedEntityData},
    objects::{ClassObjectData, ObjectType},
    types::Handle,
    xdata::{ExtendedData, ExtendedDataRecord, XDataValue},
    CadDocument,
};
use std::{cell::RefCell, collections::HashSet};
#[path = "nested.rs"]
mod nested;

pub(super) fn prepare(source: &CadDocument, body: &mut RecordBody) -> CaptureReport {
    let mut capture = Capture {
        source,
        entries: source.storage_root_exclusions(),
        evidence: Vec::new(),
        seen: HashSet::new(),
    };
    capture.note("document",ContentCoverage::PartialTyped,"storage.typed_only",
        "Typed content only; original records, private baselines and shared AcDs storage are excluded. Reconstruction and CAD writeback are not qualified.");
    macro_rules! side_views { ($($name:ident),*)=> { $(
        for (handle,_) in &body.context.$name {
            capture.note(&format!("context/{}/{}",stringify!($name),handle.value()),
                ContentCoverage::CompleteTyped,"context.side_view_unqualified",
                "Typed side-view values are retained independently; agreement with backing records and reconstruction are not qualified.");
        }
    )* } }
    side_views!(
        block_visibility_params,
        context_scales,
        block_representations,
        fields,
        dgn_ls_definitions,
        dgn_ls_components,
        object_xdata,
        xdictionary_links,
        reactor_links,
        view_rep_refs
    );
    for (index, entity) in body.entities.iter_mut().enumerate() {
        capture.entity(entity, &format!("entities/{index}"));
    }
    for (index, (_, object)) in body.objects.iter_mut().enumerate() {
        capture.object(object, &format!("objects/{index}"));
    }
    for (handle, xdata) in &mut body.context.object_xdata {
        capture.xdata(
            *handle,
            xdata,
            &format!("context/object_xdata/{}", handle.value()),
        );
    }
    for (handle, batches) in source.storage_source_eed() {
        for (app, bytes) in batches {
            if !capture.seen.contains(&(*handle, *app, bytes.clone())) {
                let mut destination = ExtendedData::new();
                capture.batch(
                    *handle,
                    *app,
                    bytes,
                    &mut destination,
                    &format!("document/eed/{}", handle.value()),
                    true,
                );
            }
        }
    }
    body.extended_data_evidence = capture.evidence;
    CaptureReport {
        entries: capture.entries,
    }
}

struct Capture<'a> {
    source: &'a CadDocument,
    entries: Vec<CaptureEvidence>,
    evidence: Vec<CapturedExtendedData>,
    seen: HashSet<(Handle, u64, Vec<u8>)>,
}
impl Capture<'_> {
    fn note(&mut self, location: &str, coverage: ContentCoverage, code: &str, message: &str) {
        self.entries.push(CaptureEvidence {
            location: location.into(),
            coverage,
            code: code.into(),
            message: message.into(),
        });
    }
    fn common(&mut self, common: &mut EntityCommon, path: &str) {
        if common.graphic_data.is_some() {
            self.note(
                &format!("{path}/graphic_data"),
                ContentCoverage::PartialTyped,
                "storage.graphic_data_excluded",
                "Opaque graphics are excluded; typed geometry is independent of this cache.",
            );
        }
        if common.raw_record.is_some() {
            self.note(
                &format!("{path}/raw_record"),
                ContentCoverage::CompleteTyped,
                "storage.raw_record_excluded",
                "Original DWG record and replay framing are excluded.",
            );
        }
        if common.has_ds_data {
            self.note(&format!("{path}/has_ds_data"),ContentCoverage::PartialTyped,"storage.data_store_flag_excluded","The source data-store flag is not restored or treated as permission to reuse AcDs data.");
        }
        self.xdata(
            common.handle,
            &mut common.extended_data,
            &format!("{path}/extended_data"),
        );
    }
    fn xdata(&mut self, handle: Handle, data: &mut ExtendedData, path: &str) {
        let batches = std::mem::take(&mut data.raw_dwg_eed);
        for (index, (app, bytes)) in batches.into_iter().enumerate() {
            self.seen.insert((handle, app, bytes.clone()));
            self.batch(
                handle,
                app,
                &bytes,
                data,
                &format!("{path}/source_eed/{index}"),
                false,
            );
        }
    }
    fn batch(
        &mut self,
        handle: Handle,
        app: u64,
        bytes: &[u8],
        destination: &mut ExtendedData,
        path: &str,
        force_evidence: bool,
    ) {
        let Some(version) = self
            .source
            .dwg_source_version
            .filter(|v| *v != crate::types::DxfVersion::Unknown)
        else {
            self.note(path,ContentCoverage::PartialTyped,"eed.source_version_missing","EED source version is unavailable; bytes are excluded without guessing an encoding family.");
            return;
        };
        let applications: Vec<_> = self
            .source
            .app_ids
            .iter()
            .filter(|entry| entry.handle.value() == app)
            .collect();
        let application_name = (applications.len() == 1).then(|| applications[0].name.clone());
        let layer_refs = RefCell::new(Vec::new());
        let decoded = crate::io::dwg::eed_codec::decode_values(
            bytes,
            version >= crate::types::DxfVersion::AC1021,
            |raw_handle| {
                let layers: Vec<_> = self
                    .source
                    .layers
                    .iter()
                    .filter(|layer| layer.handle.value() == raw_handle)
                    .collect();
                let name = (layers.len() == 1).then(|| layers[0].name.clone());
                layer_refs
                    .borrow_mut()
                    .push((Handle::new(raw_handle), name.is_some()));
                name
            },
        );
        let Some(values) = decoded else {
            self.note(path,ContentCoverage::PartialTyped,"eed.undecodable","The existing EED decoder cannot decode the complete payload; no partial prefix or raw bytes are stored.");
            return;
        };
        let mut refs = layer_refs.into_inner().into_iter();
        let mut unresolved = false;
        let evidence_values: Vec<_> = values
            .iter()
            .map(|value| {
                if matches!(value, XDataValue::LayerName(_)) {
                    let (layer, resolved) =
                        refs.next().expect("one decoder callback per layer value");
                    if !resolved {
                        unresolved = true;
                        return CapturedXDataValue::UnresolvedLayer(layer);
                    }
                }
                CapturedXDataValue::Value(value.clone())
            })
            .collect();
        let mut reason = "eed.decoded_source_values";
        if !force_evidence && !unresolved {
            if let Some(name) = application_name.as_ref() {
                let authored: Vec<_> = destination
                    .records()
                    .iter()
                    .filter(|record| record.application_name.eq_ignore_ascii_case(name))
                    .collect();
                if authored.is_empty() {
                    destination.add_record(ExtendedDataRecord {
                        application_name: name.clone(),
                        values,
                    });
                    self.note(path,ContentCoverage::CompleteTyped,"eed.decoded","Decoded values retained using actual application and layer context; original bytes excluded.");
                    return;
                }
                if authored.len() == 1 && values_equal(&authored[0].values, &values) {
                    self.note(path,ContentCoverage::CompleteTyped,"eed.already_typed","Available decoded values are already retained in typed XDATA; original bytes excluded.");
                    return;
                }
                reason = "eed.authored_values_retained";
            }
        }
        if application_name.is_none() {
            reason = "eed.unresolved_application";
        }
        if unresolved {
            reason = "eed.unresolved_layer";
        }
        self.evidence.push(CapturedExtendedData {
            location: path.into(),
            source_handle: handle,
            application_handle: Handle::new(app),
            application_name,
            values: evidence_values,
        });
        self.note(path,ContentCoverage::PartialTyped,reason,
            "Available typed source values and unresolved references are retained separately; authored XDATA is unchanged and missing names are not invented.");
    }
    fn entity(&mut self, entity: &mut EntityType, path: &str) {
        self.common(entity.common_mut(), &format!("{path}/common"));
        match entity {
            EntityType::Helix(value)=>self.common(&mut value.spline.common,&format!("{path}/spline/common")),
            EntityType::Insert(value)=>for (i,child) in value.attributes.iter_mut().enumerate() {
                self.common(&mut child.common,&format!("{path}/attributes/{i}/common"));
                self.mtext(child.embedded_mtext.as_deref_mut(),&format!("{path}/attributes/{i}/embedded_mtext"));
            },
            EntityType::AttributeDefinition(value)=>self.mtext(value.embedded_mtext.as_deref_mut(),&format!("{path}/embedded_mtext")),
            EntityType::AttributeEntity(value)=>self.mtext(value.embedded_mtext.as_deref_mut(),&format!("{path}/embedded_mtext")),
            EntityType::Surface(value)=>self.surface(&mut value.surface_data,&format!("{path}/surface_data")),
            EntityType::PolyfaceMesh(value)=>{
                for (i,child) in value.vertices.iter_mut().enumerate() { self.common(&mut child.common,&format!("{path}/vertices/{i}/common")); }
                for (i,child) in value.faces.iter_mut().enumerate() { self.common(&mut child.common,&format!("{path}/faces/{i}/common")); }
            }
            EntityType::PolygonMesh(value)=>for (i,child) in value.vertices.iter_mut().enumerate() { self.common(&mut child.common,&format!("{path}/vertices/{i}/common")); },
            EntityType::Unknown(_)=>self.note(path,ContentCoverage::ShellOnly,"entity.unknown_shell","Only typed common/type identity is captured; unknown entity source payload is excluded."),
            EntityType::Extended(value)=>match &mut value.data {
                ExtendedEntityData::GeoPositionMarker(data)=>self.mtext(data.embedded_mtext.as_mut(),&format!("{path}/data/GeoPositionMarker/embedded_mtext")),
                ExtendedEntityData::DynamicBlock(data)=>self.dynamic(data,&format!("{path}/data/DynamicBlock")),
                ExtendedEntityData::Format(_)=>self.note(path,ContentCoverage::ShellOnly,"entity.format_shell","The Format payload is source-record storage, not a typed semantic body."),
                ExtendedEntityData::LayoutPrintConfig(_)=>self.note(path,ContentCoverage::PartialTyped,"entity.layout_print_config_partial","Known class version/flag are retained; opaque source fields are excluded."),
                _=>{}
            },
            EntityType::Point(_) | EntityType::Line(_) | EntityType::Circle(_) | EntityType::Arc(_)
            | EntityType::Ellipse(_) | EntityType::Polyline(_) | EntityType::Polyline2D(_) | EntityType::Polyline3D(_)
            | EntityType::LwPolyline(_) | EntityType::Text(_) | EntityType::MText(_) | EntityType::Spline(_)
            | EntityType::Dimension(_) | EntityType::Hatch(_) | EntityType::Solid(_) | EntityType::Face3D(_)
            | EntityType::Block(_) | EntityType::BlockEnd(_) | EntityType::Ray(_) | EntityType::XLine(_)
            | EntityType::Viewport(_)
            | EntityType::Leader(_) | EntityType::MultiLeader(_) | EntityType::MLine(_) | EntityType::Mesh(_)
            | EntityType::RasterImage(_) | EntityType::Solid3D(_) | EntityType::Region(_) | EntityType::Body(_)
            | EntityType::Table(_) | EntityType::Tolerance(_) | EntityType::Wipeout(_)
            | EntityType::Shape(_) | EntityType::Underlay(_) | EntityType::Seqend(_) | EntityType::Ole2Frame(_)
            | EntityType::Light(_) | EntityType::SectionSymbol(_) | EntityType::ViewBorder(_)=>{}
        }
    }
    fn object(&mut self, object: &mut ObjectType, path: &str) {
        match object {
            ObjectType::Unknown{..}=>self.note(path,ContentCoverage::ShellOnly,"object.unknown_shell","Only typed type/handle/owner identity is captured; opaque source payload is excluded."),
            ObjectType::Field(value)=>self.xdata(value.handle,&mut value.xdata,&format!("{path}/xdata")),
            ObjectType::TableContent(value)=>self.common(&mut value.common,&format!("{path}/common")),
            ObjectType::XRecord(value)=>{
                if !value.entries_complete {
                    self.note(path,ContentCoverage::PartialTyped,"xrecord.incomplete","Only available entries/references are retained; entries_complete remains false and opaque raw_data is excluded.");
                }
                if !value.raw_data.is_empty() {
                    let coverage=if value.entries_complete {ContentCoverage::CompleteTyped} else {ContentCoverage::PartialTyped};
                    self.note(&format!("{path}/raw_data"),coverage,"xrecord.raw_data_excluded","Original XRecord data bytes are excluded; available typed entries and the source completeness flag remain unchanged.");
                }
            },
            ObjectType::SortEntitiesTable(value)=>if value.raw_dxf_codes.is_some() {
                self.note(&format!("{path}/raw_dxf_codes"),ContentCoverage::PartialTyped,"sort_entities.raw_codes_excluded","Typed draw order is retained; original DXF group codes are excluded.");
            },
            ObjectType::DynamicBlock(value)=>self.dynamic(&mut value.data,&format!("{path}/data")),
            ObjectType::Layout(value)=>if value.raw_plot_settings_codes.is_some() {
                self.note(path,ContentCoverage::PartialTyped,"layout.raw_plot_settings_excluded","Typed layout values are retained; additional source-only plot settings are excluded.");
            },
            ObjectType::TableStyle(value)=>if value.raw_dxf_codes.is_some() {
                self.note(path,ContentCoverage::PartialTyped,"table_style.raw_codes_excluded","Typed style values are retained; unmodeled source group codes are excluded.");
            },
            ObjectType::ClassObject(value)=>if let ClassObjectData::CsacDocumentOptions(data)=&value.data {
                if data.raw_dwg_data.is_some() { self.note(path,ContentCoverage::PartialTyped,"class.raw_record_excluded","Known class options remain typed; original record bytes/framing are excluded."); }
            },
            ObjectType::RegisteredClass(value)=>if value.raw_dwg_data.is_some() {
                self.note(path,ContentCoverage::PartialTyped,"class.raw_record_excluded","Typed properties, references and standalone payload are retained; original record bytes/framing are excluded.");
            },
            ObjectType::Dictionary(_) | ObjectType::Group(_) | ObjectType::MLineStyle(_) | ObjectType::ImageDefinition(_)
            | ObjectType::UnderlayDefinition(_) | ObjectType::PlotSettings(_) | ObjectType::MultiLeaderStyle(_)
            | ObjectType::Scale(_) | ObjectType::ObjectContextData(_)
            | ObjectType::DictionaryVariable(_) | ObjectType::VisualStyle(_) | ObjectType::Material(_)
            | ObjectType::ImageDefinitionReactor(_) | ObjectType::GeoData(_) | ObjectType::SpatialFilter(_)
            | ObjectType::RasterVariables(_) | ObjectType::BookColor(_) | ObjectType::PlaceHolder(_)
            | ObjectType::DictionaryWithDefault(_) | ObjectType::WipeoutVariables(_) | ObjectType::BlockVisibilityParameter(_)
            | ObjectType::Associative(_) | ObjectType::DataObject(_)
            | ObjectType::FieldList(_) | ObjectType::DgnLineStyle(_) | ObjectType::ProxyObject(_)=>{}
        }
    }
}

fn values_equal(a: &[XDataValue], b: &[XDataValue]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| match (a, b) {
            (XDataValue::Real(a), XDataValue::Real(b))
            | (XDataValue::Distance(a), XDataValue::Distance(b))
            | (XDataValue::ScaleFactor(a), XDataValue::ScaleFactor(b)) => {
                a.to_bits() == b.to_bits()
            }
            (XDataValue::Point3D(a), XDataValue::Point3D(b))
            | (XDataValue::Position3D(a), XDataValue::Position3D(b))
            | (XDataValue::Displacement3D(a), XDataValue::Displacement3D(b))
            | (XDataValue::Direction3D(a), XDataValue::Direction3D(b)) => {
                a.x.to_bits() == b.x.to_bits()
                    && a.y.to_bits() == b.y.to_bits()
                    && a.z.to_bits() == b.z.to_bits()
            }
            _ => a == b,
        })
}
