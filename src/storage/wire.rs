//! Explicit V1 exception DTOs; ordinary model Serde remains unchanged.
use crate::objects::{ImageDefinition, MLineStyle, SectionViewStyle};
use crate::{entities::*, objects::*, types::*};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredXRecord {
    handle: Handle,
    owner: Handle,
    reactors: Vec<Handle>,
    #[serde(deserialize_with = "crate::storage::records::required_option")]
    xdictionary_handle: Option<Handle>,
    name: String,
    cloning_flags: DictionaryCloningFlags,
    entries: Vec<XRecordEntry>,
    object_references: Vec<ProxyObjectReference>,
    preserve_object_reference_stream: bool,
    entries_complete: bool,
}
impl From<&XRecord> for StoredXRecord {
    fn from(value: &XRecord) -> Self {
        Self {
            handle: value.handle,
            owner: value.owner,
            reactors: value.reactors.clone(),
            xdictionary_handle: value.xdictionary_handle,
            name: value.name.clone(),
            cloning_flags: value.cloning_flags,
            entries: value.entries.clone(),
            object_references: value.object_references.clone(),
            preserve_object_reference_stream: value.preserve_object_reference_stream,
            entries_complete: value.entries_complete,
        }
    }
}
impl From<StoredXRecord> for XRecord {
    fn from(value: StoredXRecord) -> Self {
        Self {
            handle: value.handle,
            owner: value.owner,
            reactors: value.reactors,
            xdictionary_handle: value.xdictionary_handle,
            name: value.name,
            cloning_flags: value.cloning_flags,
            entries: value.entries,
            object_references: value.object_references,
            preserve_object_reference_stream: value.preserve_object_reference_stream,
            entries_complete: value.entries_complete,
            raw_data: Vec::new(),
            raw_dwg_data: None,
            raw_dwg_handle_bits: 0,
            raw_dwg_version: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredUnknownEntity {
    common: EntityCommon,
    dxf_name: String,
    dwg_type_code: i16,
}
impl From<&UnknownEntity> for StoredUnknownEntity {
    fn from(value: &UnknownEntity) -> Self {
        Self {
            common: value.common.clone(),
            dxf_name: value.dxf_name.clone(),
            dwg_type_code: value.dwg_type_code,
        }
    }
}
impl From<StoredUnknownEntity> for UnknownEntity {
    fn from(value: StoredUnknownEntity) -> Self {
        Self {
            common: value.common,
            dxf_name: value.dxf_name,
            dwg_type_code: value.dwg_type_code,
            raw_dwg_data: None,
            dwg_handle_bits: 0,
            raw_dxf_codes: None,
            dwg_source_version: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredRegisteredClassObject {
    handle: Handle,
    owner: Handle,
    reactors: Vec<Handle>,
    #[serde(deserialize_with = "crate::storage::records::required_option")]
    xdictionary_handle: Option<Handle>,
    dxf_name: String,
    cpp_class_name: String,
    properties: Vec<SemanticProperty>,
    payload: ProxyPayload,
    object_ids: Vec<ProxyObjectReference>,
}
impl From<&RegisteredClassObject> for StoredRegisteredClassObject {
    fn from(value: &RegisteredClassObject) -> Self {
        Self {
            handle: value.handle,
            owner: value.owner,
            reactors: value.reactors.clone(),
            xdictionary_handle: value.xdictionary_handle,
            dxf_name: value.dxf_name.clone(),
            cpp_class_name: value.cpp_class_name.clone(),
            properties: value.properties.clone(),
            payload: value.payload.clone(),
            object_ids: value.object_ids.clone(),
        }
    }
}
impl From<StoredRegisteredClassObject> for RegisteredClassObject {
    fn from(value: StoredRegisteredClassObject) -> Self {
        Self {
            handle: value.handle,
            owner: value.owner,
            reactors: value.reactors,
            xdictionary_handle: value.xdictionary_handle,
            dxf_name: value.dxf_name,
            cpp_class_name: value.cpp_class_name,
            properties: value.properties,
            payload: value.payload,
            object_ids: value.object_ids,
            raw_dwg_data: None,
            raw_dwg_handle_bits: 0,
            raw_dwg_version: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredLayoutPrintConfigData {
    class_version: i16,
    flag: i16,
}
impl From<&LayoutPrintConfigData> for StoredLayoutPrintConfigData {
    fn from(value: &LayoutPrintConfigData) -> Self {
        Self {
            class_version: value.class_version,
            flag: value.flag,
        }
    }
}
impl From<StoredLayoutPrintConfigData> for LayoutPrintConfigData {
    fn from(value: StoredLayoutPrintConfigData) -> Self {
        Self {
            class_version: value.class_version,
            flag: value.flag,
            raw_dwg_data: None,
            raw_dwg_handle_bits: 0,
            raw_dwg_version: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredFormatData {}
impl From<&FormatData> for StoredFormatData {
    fn from(_value: &FormatData) -> Self {
        Self {}
    }
}
impl From<StoredFormatData> for FormatData {
    fn from(_value: StoredFormatData) -> Self {
        Self {
            raw_dwg_data: None,
            raw_dwg_handle_bits: 0,
            raw_dwg_version: None,
            raw_dxf_codes: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredCsacDocumentOptions {
    class_version: i16,
}
impl From<&CsacDocumentOptions> for StoredCsacDocumentOptions {
    fn from(value: &CsacDocumentOptions) -> Self {
        Self {
            class_version: value.class_version,
        }
    }
}
impl From<StoredCsacDocumentOptions> for CsacDocumentOptions {
    fn from(value: StoredCsacDocumentOptions) -> Self {
        Self {
            class_version: value.class_version,
            raw_dwg_data: None,
            raw_dwg_handle_bits: 0,
            raw_dwg_version: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum StoredClassObjectData {
    Empty,
    SpatialIndex(Box<SpatialIndex>),
    LayerFilter(Box<LayerFilter>),
    PartialViewingIndex(Box<PartialViewingIndex>),
    VbaProject(Box<VbaProject>),
    SectionManager(Box<SectionManager>),
    SectionSettings(Box<SectionSettings>),
    LightList(Box<LightList>),
    Sun(Box<Sun>),
    RenderSettings(Box<RenderSettings>),
    MentalRayRenderSettings(Box<MentalRayRenderSettings>),
    RapidRtRenderSettings(Box<RapidRtRenderSettings>),
    GradientBackground(Box<GradientBackground>),
    GroundPlaneBackground(Box<GroundPlaneBackground>),
    IblBackground(Box<IblBackground>),
    ImageBackground(Box<ImageBackground>),
    SkyLightBackground(Box<SkyLightBackground>),
    SolidBackground(Box<SolidBackground>),
    RenderEntry(Box<RenderEntry>),
    RenderEnvironment(Box<RenderEnvironment>),
    RenderGlobal(Box<RenderGlobal>),
    MotionPath(Box<MotionPath>),
    CurvePath(Box<CurvePath>),
    PointPath(Box<PointPath>),
    TvDeviceProperties(Box<TvDeviceProperties>),
    PointCloudDefinition(Box<PointCloudDefinition>),
    PointCloudDefinitionEx(Box<PointCloudDefinition>),
    PointCloudDefinitionReactor(Box<PointCloudDefinitionReactor>),
    PointCloudDefinitionReactorEx(Box<PointCloudDefinitionReactor>),
    PointCloudColorMap(Box<PointCloudColorMap>),
    NavisworksModelDefinition(Box<NavisworksModelDefinition>),
    ContextDataManager(Box<ContextDataManager>),
    SunStudy(Box<SunStudy>),
    DataTable(Box<DataTable>),
    DataLink(Box<DataLink>),
    PersistentSubentityManager(Box<PersistentSubentityManager>),
    GeoMapImage(Box<GeoMapImage>),
    DetailViewStyle(Box<DetailViewStyle>),
    SectionViewStyle(Box<SectionViewStyle>),
    AcMeCommandHistory(Box<AcMeCommandHistory>),
    AcMeScope(Box<AcMeScope>),
    AcMeStateManager(Box<AcMeStateManager>),
    CsacDocumentOptions(Box<StoredCsacDocumentOptions>),
    ViewRepSourceManager(Box<ViewRepSourceManager>),
    ViewRepStandard(Box<ViewRepStandard>),
    ViewRepOrientationDefinition,
    ViewRepOrientation(Box<ViewRepOrientation>),
    ViewRepSectionDefinition(Box<ViewRepSectionDefinition>),
    ViewRepModelSpaceViewSelectionSet(Box<ViewRepModelSpaceViewSelectionSet>),
    ViewRep(Box<ViewRep>),
    ViewRepModelSpaceSource(Box<ViewRepModelSpaceSource>),
}
impl From<&ClassObjectData> for StoredClassObjectData {
    fn from(value: &ClassObjectData) -> Self {
        match value {
            ClassObjectData::Empty => Self::Empty,
            ClassObjectData::SpatialIndex(value) => Self::SpatialIndex(Box::new(value.clone())),
            ClassObjectData::LayerFilter(value) => Self::LayerFilter(Box::new(value.clone())),
            ClassObjectData::PartialViewingIndex(value) => {
                Self::PartialViewingIndex(Box::new(value.clone()))
            }
            ClassObjectData::VbaProject(value) => Self::VbaProject(Box::new(value.clone())),
            ClassObjectData::SectionManager(value) => Self::SectionManager(Box::new(value.clone())),
            ClassObjectData::SectionSettings(value) => {
                Self::SectionSettings(Box::new(value.clone()))
            }
            ClassObjectData::LightList(value) => Self::LightList(Box::new(value.clone())),
            ClassObjectData::Sun(value) => Self::Sun(Box::new(value.clone())),
            ClassObjectData::RenderSettings(value) => Self::RenderSettings(Box::new(value.clone())),
            ClassObjectData::MentalRayRenderSettings(value) => {
                Self::MentalRayRenderSettings(Box::new(value.clone()))
            }
            ClassObjectData::RapidRtRenderSettings(value) => {
                Self::RapidRtRenderSettings(Box::new(value.clone()))
            }
            ClassObjectData::GradientBackground(value) => {
                Self::GradientBackground(Box::new(value.clone()))
            }
            ClassObjectData::GroundPlaneBackground(value) => {
                Self::GroundPlaneBackground(Box::new(value.clone()))
            }
            ClassObjectData::IblBackground(value) => Self::IblBackground(Box::new(value.clone())),
            ClassObjectData::ImageBackground(value) => {
                Self::ImageBackground(Box::new(value.clone()))
            }
            ClassObjectData::SkyLightBackground(value) => {
                Self::SkyLightBackground(Box::new(value.clone()))
            }
            ClassObjectData::SolidBackground(value) => {
                Self::SolidBackground(Box::new(value.clone()))
            }
            ClassObjectData::RenderEntry(value) => Self::RenderEntry(Box::new(value.clone())),
            ClassObjectData::RenderEnvironment(value) => {
                Self::RenderEnvironment(Box::new(value.clone()))
            }
            ClassObjectData::RenderGlobal(value) => Self::RenderGlobal(Box::new(value.clone())),
            ClassObjectData::MotionPath(value) => Self::MotionPath(Box::new(value.clone())),
            ClassObjectData::CurvePath(value) => Self::CurvePath(Box::new(value.clone())),
            ClassObjectData::PointPath(value) => Self::PointPath(Box::new(value.clone())),
            ClassObjectData::TvDeviceProperties(value) => {
                Self::TvDeviceProperties(Box::new(value.clone()))
            }
            ClassObjectData::PointCloudDefinition(value) => {
                Self::PointCloudDefinition(Box::new(value.clone()))
            }
            ClassObjectData::PointCloudDefinitionEx(value) => {
                Self::PointCloudDefinitionEx(Box::new(value.clone()))
            }
            ClassObjectData::PointCloudDefinitionReactor(value) => {
                Self::PointCloudDefinitionReactor(Box::new(value.clone()))
            }
            ClassObjectData::PointCloudDefinitionReactorEx(value) => {
                Self::PointCloudDefinitionReactorEx(Box::new(value.clone()))
            }
            ClassObjectData::PointCloudColorMap(value) => {
                Self::PointCloudColorMap(Box::new(value.clone()))
            }
            ClassObjectData::NavisworksModelDefinition(value) => {
                Self::NavisworksModelDefinition(Box::new(value.clone()))
            }
            ClassObjectData::ContextDataManager(value) => {
                Self::ContextDataManager(Box::new(value.clone()))
            }
            ClassObjectData::SunStudy(value) => Self::SunStudy(Box::new(value.clone())),
            ClassObjectData::DataTable(value) => Self::DataTable(Box::new(value.clone())),
            ClassObjectData::DataLink(value) => Self::DataLink(Box::new(value.clone())),
            ClassObjectData::PersistentSubentityManager(value) => {
                Self::PersistentSubentityManager(Box::new(value.clone()))
            }
            ClassObjectData::GeoMapImage(value) => Self::GeoMapImage(Box::new(value.clone())),
            ClassObjectData::DetailViewStyle(value) => {
                Self::DetailViewStyle(Box::new(value.clone()))
            }
            ClassObjectData::SectionViewStyle(value) => {
                Self::SectionViewStyle(Box::new(value.clone()))
            }
            ClassObjectData::AcMeCommandHistory(value) => {
                Self::AcMeCommandHistory(Box::new(value.clone()))
            }
            ClassObjectData::AcMeScope(value) => Self::AcMeScope(Box::new(value.clone())),
            ClassObjectData::AcMeStateManager(value) => {
                Self::AcMeStateManager(Box::new(value.clone()))
            }
            ClassObjectData::CsacDocumentOptions(value) => {
                Self::CsacDocumentOptions(Box::new(value.into()))
            }
            ClassObjectData::ViewRepSourceManager(value) => {
                Self::ViewRepSourceManager(Box::new(value.clone()))
            }
            ClassObjectData::ViewRepStandard(value) => {
                Self::ViewRepStandard(Box::new(value.clone()))
            }
            ClassObjectData::ViewRepOrientationDefinition => Self::ViewRepOrientationDefinition,
            ClassObjectData::ViewRepOrientation(value) => {
                Self::ViewRepOrientation(Box::new(value.clone()))
            }
            ClassObjectData::ViewRepSectionDefinition(value) => {
                Self::ViewRepSectionDefinition(Box::new(value.clone()))
            }
            ClassObjectData::ViewRepModelSpaceViewSelectionSet(value) => {
                Self::ViewRepModelSpaceViewSelectionSet(Box::new(value.clone()))
            }
            ClassObjectData::ViewRep(value) => Self::ViewRep(Box::new(value.clone())),
            ClassObjectData::ViewRepModelSpaceSource(value) => {
                Self::ViewRepModelSpaceSource(Box::new(value.clone()))
            }
        }
    }
}
impl From<StoredClassObjectData> for ClassObjectData {
    fn from(value: StoredClassObjectData) -> Self {
        match value {
            StoredClassObjectData::Empty => Self::Empty,
            StoredClassObjectData::SpatialIndex(value) => Self::SpatialIndex(*value),
            StoredClassObjectData::LayerFilter(value) => Self::LayerFilter(*value),
            StoredClassObjectData::PartialViewingIndex(value) => Self::PartialViewingIndex(*value),
            StoredClassObjectData::VbaProject(value) => Self::VbaProject(*value),
            StoredClassObjectData::SectionManager(value) => Self::SectionManager(*value),
            StoredClassObjectData::SectionSettings(value) => Self::SectionSettings(*value),
            StoredClassObjectData::LightList(value) => Self::LightList(*value),
            StoredClassObjectData::Sun(value) => Self::Sun(*value),
            StoredClassObjectData::RenderSettings(value) => Self::RenderSettings(*value),
            StoredClassObjectData::MentalRayRenderSettings(value) => {
                Self::MentalRayRenderSettings(*value)
            }
            StoredClassObjectData::RapidRtRenderSettings(value) => {
                Self::RapidRtRenderSettings(*value)
            }
            StoredClassObjectData::GradientBackground(value) => Self::GradientBackground(*value),
            StoredClassObjectData::GroundPlaneBackground(value) => {
                Self::GroundPlaneBackground(*value)
            }
            StoredClassObjectData::IblBackground(value) => Self::IblBackground(*value),
            StoredClassObjectData::ImageBackground(value) => Self::ImageBackground(*value),
            StoredClassObjectData::SkyLightBackground(value) => Self::SkyLightBackground(*value),
            StoredClassObjectData::SolidBackground(value) => Self::SolidBackground(*value),
            StoredClassObjectData::RenderEntry(value) => Self::RenderEntry(*value),
            StoredClassObjectData::RenderEnvironment(value) => Self::RenderEnvironment(*value),
            StoredClassObjectData::RenderGlobal(value) => Self::RenderGlobal(*value),
            StoredClassObjectData::MotionPath(value) => Self::MotionPath(*value),
            StoredClassObjectData::CurvePath(value) => Self::CurvePath(*value),
            StoredClassObjectData::PointPath(value) => Self::PointPath(*value),
            StoredClassObjectData::TvDeviceProperties(value) => Self::TvDeviceProperties(*value),
            StoredClassObjectData::PointCloudDefinition(value) => {
                Self::PointCloudDefinition(*value)
            }
            StoredClassObjectData::PointCloudDefinitionEx(value) => {
                Self::PointCloudDefinitionEx(*value)
            }
            StoredClassObjectData::PointCloudDefinitionReactor(value) => {
                Self::PointCloudDefinitionReactor(*value)
            }
            StoredClassObjectData::PointCloudDefinitionReactorEx(value) => {
                Self::PointCloudDefinitionReactorEx(*value)
            }
            StoredClassObjectData::PointCloudColorMap(value) => Self::PointCloudColorMap(*value),
            StoredClassObjectData::NavisworksModelDefinition(value) => {
                Self::NavisworksModelDefinition(*value)
            }
            StoredClassObjectData::ContextDataManager(value) => Self::ContextDataManager(*value),
            StoredClassObjectData::SunStudy(value) => Self::SunStudy(*value),
            StoredClassObjectData::DataTable(value) => Self::DataTable(*value),
            StoredClassObjectData::DataLink(value) => Self::DataLink(*value),
            StoredClassObjectData::PersistentSubentityManager(value) => {
                Self::PersistentSubentityManager(*value)
            }
            StoredClassObjectData::GeoMapImage(value) => Self::GeoMapImage(*value),
            StoredClassObjectData::DetailViewStyle(value) => Self::DetailViewStyle(*value),
            StoredClassObjectData::SectionViewStyle(value) => Self::SectionViewStyle(*value),
            StoredClassObjectData::AcMeCommandHistory(value) => Self::AcMeCommandHistory(*value),
            StoredClassObjectData::AcMeScope(value) => Self::AcMeScope(*value),
            StoredClassObjectData::AcMeStateManager(value) => Self::AcMeStateManager(*value),
            StoredClassObjectData::CsacDocumentOptions(value) => {
                Self::CsacDocumentOptions((*value).into())
            }
            StoredClassObjectData::ViewRepSourceManager(value) => {
                Self::ViewRepSourceManager(*value)
            }
            StoredClassObjectData::ViewRepStandard(value) => Self::ViewRepStandard(*value),
            StoredClassObjectData::ViewRepOrientationDefinition => {
                Self::ViewRepOrientationDefinition
            }
            StoredClassObjectData::ViewRepOrientation(value) => Self::ViewRepOrientation(*value),
            StoredClassObjectData::ViewRepSectionDefinition(value) => {
                Self::ViewRepSectionDefinition(*value)
            }
            StoredClassObjectData::ViewRepModelSpaceViewSelectionSet(value) => {
                Self::ViewRepModelSpaceViewSelectionSet(*value)
            }
            StoredClassObjectData::ViewRep(value) => Self::ViewRep(*value),
            StoredClassObjectData::ViewRepModelSpaceSource(value) => {
                Self::ViewRepModelSpaceSource(*value)
            }
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredClassObject {
    handle: Handle,
    owner: Handle,
    reactors: Vec<Handle>,
    #[serde(deserialize_with = "crate::storage::records::required_option")]
    xdictionary_handle: Option<Handle>,
    data: StoredClassObjectData,
}
impl From<&ClassObject> for StoredClassObject {
    fn from(value: &ClassObject) -> Self {
        Self {
            handle: value.handle,
            owner: value.owner,
            reactors: value.reactors.clone(),
            xdictionary_handle: value.xdictionary_handle,
            data: (&value.data).into(),
        }
    }
}
impl From<StoredClassObject> for ClassObject {
    fn from(value: StoredClassObject) -> Self {
        Self {
            handle: value.handle,
            owner: value.owner,
            reactors: value.reactors,
            xdictionary_handle: value.xdictionary_handle,
            data: value.data.into(),
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum StoredExtendedEntityData {
    Camera { view_handle: Handle },
    SectionObject(Box<SectionObjectData>),
    ArcAlignedText(Box<ArcAlignedTextData>),
    RemoteText(Box<RemoteTextData>),
    GeoPositionMarker(Box<GeoPositionMarkerData>),
    CoordinationModel(Box<CoordinationModelData>),
    PointCloud(Box<PointCloudData>),
    PointCloudEx(Box<PointCloudExData>),
    Proxy(Box<ProxyEntityData>),
    OleFrame(Box<OleFrameData>),
    LayoutPrintConfig(Box<StoredLayoutPrintConfigData>),
    Format(Box<StoredFormatData>),
    Legacy(Box<LegacyEntityData>),
    DynamicBlock(Box<crate::objects::DynamicBlockData>),
    RegisteredClass(Box<RegisteredClassEntityData>),
}
impl From<&ExtendedEntityData> for StoredExtendedEntityData {
    fn from(value: &ExtendedEntityData) -> Self {
        match value {
            ExtendedEntityData::Camera { view_handle, .. } => Self::Camera {
                view_handle: *view_handle,
            },
            ExtendedEntityData::SectionObject(value) => {
                Self::SectionObject(Box::new(value.clone()))
            }
            ExtendedEntityData::ArcAlignedText(value) => {
                Self::ArcAlignedText(Box::new(value.clone()))
            }
            ExtendedEntityData::RemoteText(value) => Self::RemoteText(Box::new(value.clone())),
            ExtendedEntityData::GeoPositionMarker(value) => {
                Self::GeoPositionMarker(Box::new(value.clone()))
            }
            ExtendedEntityData::CoordinationModel(value) => {
                Self::CoordinationModel(Box::new(value.clone()))
            }
            ExtendedEntityData::PointCloud(value) => Self::PointCloud(Box::new(value.clone())),
            ExtendedEntityData::PointCloudEx(value) => Self::PointCloudEx(Box::new(value.clone())),
            ExtendedEntityData::Proxy(value) => Self::Proxy(Box::new(value.clone())),
            ExtendedEntityData::OleFrame(value) => Self::OleFrame(Box::new(value.clone())),
            ExtendedEntityData::LayoutPrintConfig(value) => {
                Self::LayoutPrintConfig(Box::new(value.into()))
            }
            ExtendedEntityData::Format(value) => Self::Format(Box::new(value.into())),
            ExtendedEntityData::Legacy(value) => Self::Legacy(Box::new(value.clone())),
            ExtendedEntityData::DynamicBlock(value) => Self::DynamicBlock(Box::new(value.clone())),
            ExtendedEntityData::RegisteredClass(value) => {
                Self::RegisteredClass(Box::new(value.clone()))
            }
        }
    }
}
impl From<StoredExtendedEntityData> for ExtendedEntityData {
    fn from(value: StoredExtendedEntityData) -> Self {
        match value {
            StoredExtendedEntityData::Camera { view_handle } => Self::Camera { view_handle },
            StoredExtendedEntityData::SectionObject(value) => Self::SectionObject(*value),
            StoredExtendedEntityData::ArcAlignedText(value) => Self::ArcAlignedText(*value),
            StoredExtendedEntityData::RemoteText(value) => Self::RemoteText(*value),
            StoredExtendedEntityData::GeoPositionMarker(value) => Self::GeoPositionMarker(*value),
            StoredExtendedEntityData::CoordinationModel(value) => Self::CoordinationModel(*value),
            StoredExtendedEntityData::PointCloud(value) => Self::PointCloud(*value),
            StoredExtendedEntityData::PointCloudEx(value) => Self::PointCloudEx(*value),
            StoredExtendedEntityData::Proxy(value) => Self::Proxy(*value),
            StoredExtendedEntityData::OleFrame(value) => Self::OleFrame(*value),
            StoredExtendedEntityData::LayoutPrintConfig(value) => {
                Self::LayoutPrintConfig((*value).into())
            }
            StoredExtendedEntityData::Format(value) => Self::Format((*value).into()),
            StoredExtendedEntityData::Legacy(value) => Self::Legacy(*value),
            StoredExtendedEntityData::DynamicBlock(value) => Self::DynamicBlock(*value),
            StoredExtendedEntityData::RegisteredClass(value) => Self::RegisteredClass(*value),
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredExtendedEntity {
    common: EntityCommon,
    data: StoredExtendedEntityData,
}
impl From<&ExtendedEntity> for StoredExtendedEntity {
    fn from(value: &ExtendedEntity) -> Self {
        Self {
            common: value.common.clone(),
            data: (&value.data).into(),
        }
    }
}
impl From<StoredExtendedEntity> for ExtendedEntity {
    fn from(value: StoredExtendedEntity) -> Self {
        Self {
            common: value.common,
            data: value.data.into(),
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum StoredEntityType {
    Point(Box<Point>),
    Line(Box<Line>),
    Circle(Box<Circle>),
    Arc(Box<Arc>),
    Ellipse(Box<Ellipse>),
    Polyline(Box<Polyline>),
    Polyline2D(Box<Polyline2D>),
    Polyline3D(Box<Polyline3D>),
    LwPolyline(Box<LwPolyline>),
    Text(Box<Text>),
    MText(Box<MText>),
    Spline(Box<Spline>),
    Helix(Box<Helix>),
    Dimension(Box<Dimension>),
    Hatch(Box<Hatch>),
    Solid(Box<Solid>),
    Face3D(Box<Face3D>),
    Insert(Box<Insert>),
    Block(Box<Block>),
    BlockEnd(Box<BlockEnd>),
    Ray(Box<Ray>),
    XLine(Box<XLine>),
    Viewport(Box<Viewport>),
    AttributeDefinition(Box<AttributeDefinition>),
    AttributeEntity(Box<AttributeEntity>),
    Leader(Box<Leader>),
    MultiLeader(Box<MultiLeader>),
    MLine(Box<MLine>),
    Mesh(Box<Mesh>),
    RasterImage(Box<RasterImage>),
    Solid3D(Box<Solid3D>),
    Region(Box<Region>),
    Body(Box<Body>),
    Surface(Box<Surface>),
    Table(Box<Table>),
    Tolerance(Box<Tolerance>),
    PolyfaceMesh(Box<PolyfaceMesh>),
    Wipeout(Box<Wipeout>),
    Shape(Box<Shape>),
    Underlay(Box<Underlay>),
    Seqend(Box<Seqend>),
    Ole2Frame(Box<Ole2Frame>),
    PolygonMesh(Box<PolygonMeshEntity>),
    Light(Box<Light>),
    SectionSymbol(Box<SectionSymbol>),
    ViewBorder(Box<ViewBorder>),
    Extended(Box<StoredExtendedEntity>),
    Unknown(Box<StoredUnknownEntity>),
}
impl From<&EntityType> for StoredEntityType {
    fn from(value: &EntityType) -> Self {
        match value {
            EntityType::Point(value) => Self::Point(Box::new(value.clone())),
            EntityType::Line(value) => Self::Line(Box::new(value.clone())),
            EntityType::Circle(value) => Self::Circle(Box::new(value.clone())),
            EntityType::Arc(value) => Self::Arc(Box::new(value.clone())),
            EntityType::Ellipse(value) => Self::Ellipse(Box::new(value.clone())),
            EntityType::Polyline(value) => Self::Polyline(Box::new(value.clone())),
            EntityType::Polyline2D(value) => Self::Polyline2D(Box::new(value.clone())),
            EntityType::Polyline3D(value) => Self::Polyline3D(Box::new(value.clone())),
            EntityType::LwPolyline(value) => Self::LwPolyline(Box::new(value.clone())),
            EntityType::Text(value) => Self::Text(Box::new(value.clone())),
            EntityType::MText(value) => Self::MText(value.clone()),
            EntityType::Spline(value) => Self::Spline(Box::new(value.clone())),
            EntityType::Helix(value) => Self::Helix(value.clone()),
            EntityType::Dimension(value) => Self::Dimension(value.clone()),
            EntityType::Hatch(value) => Self::Hatch(value.clone()),
            EntityType::Solid(value) => Self::Solid(Box::new(value.clone())),
            EntityType::Face3D(value) => Self::Face3D(Box::new(value.clone())),
            EntityType::Insert(value) => Self::Insert(Box::new(value.clone())),
            EntityType::Block(value) => Self::Block(Box::new(value.clone())),
            EntityType::BlockEnd(value) => Self::BlockEnd(Box::new(value.clone())),
            EntityType::Ray(value) => Self::Ray(Box::new(value.clone())),
            EntityType::XLine(value) => Self::XLine(Box::new(value.clone())),
            EntityType::Viewport(value) => Self::Viewport(value.clone()),
            EntityType::AttributeDefinition(value) => {
                Self::AttributeDefinition(Box::new(value.clone()))
            }
            EntityType::AttributeEntity(value) => Self::AttributeEntity(Box::new(value.clone())),
            EntityType::Leader(value) => Self::Leader(Box::new(value.clone())),
            EntityType::MultiLeader(value) => Self::MultiLeader(value.clone()),
            EntityType::MLine(value) => Self::MLine(Box::new(value.clone())),
            EntityType::Mesh(value) => Self::Mesh(Box::new(value.clone())),
            EntityType::RasterImage(value) => Self::RasterImage(Box::new(value.clone())),
            EntityType::Solid3D(value) => Self::Solid3D(Box::new(value.clone())),
            EntityType::Region(value) => Self::Region(Box::new(value.clone())),
            EntityType::Body(value) => Self::Body(Box::new(value.clone())),
            EntityType::Surface(value) => Self::Surface(value.clone()),
            EntityType::Table(value) => Self::Table(value.clone()),
            EntityType::Tolerance(value) => Self::Tolerance(Box::new(value.clone())),
            EntityType::PolyfaceMesh(value) => Self::PolyfaceMesh(Box::new(value.clone())),
            EntityType::Wipeout(value) => Self::Wipeout(Box::new(value.clone())),
            EntityType::Shape(value) => Self::Shape(Box::new(value.clone())),
            EntityType::Underlay(value) => Self::Underlay(Box::new(value.clone())),
            EntityType::Seqend(value) => Self::Seqend(Box::new(value.clone())),
            EntityType::Ole2Frame(value) => Self::Ole2Frame(value.clone()),
            EntityType::PolygonMesh(value) => Self::PolygonMesh(Box::new(value.clone())),
            EntityType::Light(value) => Self::Light(value.clone()),
            EntityType::SectionSymbol(value) => Self::SectionSymbol(Box::new(value.clone())),
            EntityType::ViewBorder(value) => Self::ViewBorder(Box::new(value.clone())),
            EntityType::Extended(value) => Self::Extended(Box::new(value.as_ref().into())),
            EntityType::Unknown(value) => Self::Unknown(Box::new(value.into())),
        }
    }
}
impl From<StoredEntityType> for EntityType {
    fn from(value: StoredEntityType) -> Self {
        match value {
            StoredEntityType::Point(value) => Self::Point(*value),
            StoredEntityType::Line(value) => Self::Line(*value),
            StoredEntityType::Circle(value) => Self::Circle(*value),
            StoredEntityType::Arc(value) => Self::Arc(*value),
            StoredEntityType::Ellipse(value) => Self::Ellipse(*value),
            StoredEntityType::Polyline(value) => Self::Polyline(*value),
            StoredEntityType::Polyline2D(value) => Self::Polyline2D(*value),
            StoredEntityType::Polyline3D(value) => Self::Polyline3D(*value),
            StoredEntityType::LwPolyline(value) => Self::LwPolyline(*value),
            StoredEntityType::Text(value) => Self::Text(*value),
            StoredEntityType::MText(value) => Self::MText(value),
            StoredEntityType::Spline(value) => Self::Spline(*value),
            StoredEntityType::Helix(value) => Self::Helix(value),
            StoredEntityType::Dimension(value) => Self::Dimension(value),
            StoredEntityType::Hatch(value) => Self::Hatch(value),
            StoredEntityType::Solid(value) => Self::Solid(*value),
            StoredEntityType::Face3D(value) => Self::Face3D(*value),
            StoredEntityType::Insert(value) => Self::Insert(*value),
            StoredEntityType::Block(value) => Self::Block(*value),
            StoredEntityType::BlockEnd(value) => Self::BlockEnd(*value),
            StoredEntityType::Ray(value) => Self::Ray(*value),
            StoredEntityType::XLine(value) => Self::XLine(*value),
            StoredEntityType::Viewport(value) => Self::Viewport(value),
            StoredEntityType::AttributeDefinition(value) => Self::AttributeDefinition(*value),
            StoredEntityType::AttributeEntity(value) => Self::AttributeEntity(*value),
            StoredEntityType::Leader(value) => Self::Leader(*value),
            StoredEntityType::MultiLeader(value) => Self::MultiLeader(value),
            StoredEntityType::MLine(value) => Self::MLine(*value),
            StoredEntityType::Mesh(value) => Self::Mesh(*value),
            StoredEntityType::RasterImage(value) => Self::RasterImage(*value),
            StoredEntityType::Solid3D(value) => Self::Solid3D(*value),
            StoredEntityType::Region(value) => Self::Region(*value),
            StoredEntityType::Body(value) => Self::Body(*value),
            StoredEntityType::Surface(value) => Self::Surface(value),
            StoredEntityType::Table(value) => Self::Table(value),
            StoredEntityType::Tolerance(value) => Self::Tolerance(*value),
            StoredEntityType::PolyfaceMesh(value) => Self::PolyfaceMesh(*value),
            StoredEntityType::Wipeout(value) => Self::Wipeout(*value),
            StoredEntityType::Shape(value) => Self::Shape(*value),
            StoredEntityType::Underlay(value) => Self::Underlay(*value),
            StoredEntityType::Seqend(value) => Self::Seqend(*value),
            StoredEntityType::Ole2Frame(value) => Self::Ole2Frame(value),
            StoredEntityType::PolygonMesh(value) => Self::PolygonMesh(*value),
            StoredEntityType::Light(value) => Self::Light(value),
            StoredEntityType::SectionSymbol(value) => Self::SectionSymbol(*value),
            StoredEntityType::ViewBorder(value) => Self::ViewBorder(*value),
            StoredEntityType::Extended(value) => Self::Extended(Box::new((*value).into())),
            StoredEntityType::Unknown(value) => Self::Unknown((*value).into()),
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum StoredObjectType {
    Dictionary(Box<Dictionary>),
    Layout(Box<Layout>),
    XRecord(Box<StoredXRecord>),
    Group(Box<Group>),
    MLineStyle(Box<MLineStyle>),
    ImageDefinition(Box<ImageDefinition>),
    UnderlayDefinition(Box<UnderlayDefinition>),
    PlotSettings(Box<StoredPlotSettings>),
    MultiLeaderStyle(Box<MultiLeaderStyle>),
    TableStyle(Box<TableStyle>),
    TableContent(Box<crate::entities::Table>),
    Scale(Box<Scale>),
    ObjectContextData(Box<ObjectContextData>),
    SortEntitiesTable(Box<StoredSortEntitiesTable>),
    DictionaryVariable(Box<DictionaryVariable>),
    VisualStyle(Box<VisualStyle>),
    Material(Box<Material>),
    ImageDefinitionReactor(Box<ImageDefinitionReactor>),
    GeoData(Box<GeoData>),
    SpatialFilter(Box<SpatialFilter>),
    RasterVariables(Box<RasterVariables>),
    BookColor(Box<BookColor>),
    PlaceHolder(Box<PlaceHolder>),
    DictionaryWithDefault(Box<DictionaryWithDefault>),
    WipeoutVariables(Box<WipeoutVariables>),
    BlockVisibilityParameter(Box<BlockVisibilityParameter>),
    DynamicBlock(Box<DynamicBlockObject>),
    Associative(Box<AssociativeObject>),
    ClassObject(Box<StoredClassObject>),
    DataObject(Box<DataObject>),
    Field(Box<Field>),
    FieldList(Box<FieldList>),
    RegisteredClass(Box<StoredRegisteredClassObject>),
    DgnLineStyle(Box<DgnLineStyleObject>),
    ProxyObject(Box<ProxyObject>),
    Unknown {
        type_name: String,
        handle: Handle,
        owner: Handle,
    },
}
impl From<&ObjectType> for StoredObjectType {
    fn from(value: &ObjectType) -> Self {
        match value {
            ObjectType::Dictionary(value) => Self::Dictionary(Box::new(value.clone())),
            ObjectType::Layout(value) => Self::Layout(Box::new(value.clone())),
            ObjectType::XRecord(value) => Self::XRecord(Box::new(value.into())),
            ObjectType::Group(value) => Self::Group(Box::new(value.clone())),
            ObjectType::MLineStyle(value) => Self::MLineStyle(Box::new(value.clone())),
            ObjectType::ImageDefinition(value) => Self::ImageDefinition(Box::new(value.clone())),
            ObjectType::UnderlayDefinition(value) => {
                Self::UnderlayDefinition(Box::new(value.clone()))
            }
            ObjectType::PlotSettings(value) => Self::PlotSettings(Box::new(value.into())),
            ObjectType::MultiLeaderStyle(value) => Self::MultiLeaderStyle(Box::new(value.clone())),
            ObjectType::TableStyle(value) => Self::TableStyle(Box::new(value.clone())),
            ObjectType::TableContent(value) => Self::TableContent(Box::new(value.clone())),
            ObjectType::Scale(value) => Self::Scale(Box::new(value.clone())),
            ObjectType::ObjectContextData(value) => {
                Self::ObjectContextData(Box::new(value.clone()))
            }
            ObjectType::SortEntitiesTable(value) => Self::SortEntitiesTable(Box::new(value.into())),
            ObjectType::DictionaryVariable(value) => {
                Self::DictionaryVariable(Box::new(value.clone()))
            }
            ObjectType::VisualStyle(value) => Self::VisualStyle(Box::new(value.clone())),
            ObjectType::Material(value) => Self::Material(Box::new(value.clone())),
            ObjectType::ImageDefinitionReactor(value) => {
                Self::ImageDefinitionReactor(Box::new(value.clone()))
            }
            ObjectType::GeoData(value) => Self::GeoData(Box::new(value.clone())),
            ObjectType::SpatialFilter(value) => Self::SpatialFilter(Box::new(value.clone())),
            ObjectType::RasterVariables(value) => Self::RasterVariables(Box::new(value.clone())),
            ObjectType::BookColor(value) => Self::BookColor(Box::new(value.clone())),
            ObjectType::PlaceHolder(value) => Self::PlaceHolder(Box::new(value.clone())),
            ObjectType::DictionaryWithDefault(value) => {
                Self::DictionaryWithDefault(Box::new(value.clone()))
            }
            ObjectType::WipeoutVariables(value) => Self::WipeoutVariables(Box::new(value.clone())),
            ObjectType::BlockVisibilityParameter(value) => {
                Self::BlockVisibilityParameter(Box::new(value.clone()))
            }
            ObjectType::DynamicBlock(value) => Self::DynamicBlock(Box::new(value.clone())),
            ObjectType::Associative(value) => Self::Associative(Box::new(value.clone())),
            ObjectType::ClassObject(value) => Self::ClassObject(Box::new(value.into())),
            ObjectType::DataObject(value) => Self::DataObject(Box::new(value.clone())),
            ObjectType::Field(value) => Self::Field(Box::new(value.clone())),
            ObjectType::FieldList(value) => Self::FieldList(Box::new(value.clone())),
            ObjectType::RegisteredClass(value) => Self::RegisteredClass(Box::new(value.into())),
            ObjectType::DgnLineStyle(value) => Self::DgnLineStyle(Box::new(value.clone())),
            ObjectType::ProxyObject(value) => Self::ProxyObject(Box::new(value.clone())),
            ObjectType::Unknown {
                type_name,
                handle,
                owner,
                ..
            } => Self::Unknown {
                type_name: type_name.clone(),
                handle: *handle,
                owner: *owner,
            },
        }
    }
}
impl From<StoredObjectType> for ObjectType {
    fn from(value: StoredObjectType) -> Self {
        match value {
            StoredObjectType::Dictionary(value) => Self::Dictionary(*value),
            StoredObjectType::Layout(value) => Self::Layout(*value),
            StoredObjectType::XRecord(value) => Self::XRecord((*value).into()),
            StoredObjectType::Group(value) => Self::Group(*value),
            StoredObjectType::MLineStyle(value) => Self::MLineStyle(*value),
            StoredObjectType::ImageDefinition(value) => Self::ImageDefinition(*value),
            StoredObjectType::UnderlayDefinition(value) => Self::UnderlayDefinition(*value),
            StoredObjectType::PlotSettings(value) => Self::PlotSettings((*value).into()),
            StoredObjectType::MultiLeaderStyle(value) => Self::MultiLeaderStyle(*value),
            StoredObjectType::TableStyle(value) => Self::TableStyle(*value),
            StoredObjectType::TableContent(value) => Self::TableContent(*value),
            StoredObjectType::Scale(value) => Self::Scale(*value),
            StoredObjectType::ObjectContextData(value) => Self::ObjectContextData(*value),
            StoredObjectType::SortEntitiesTable(value) => Self::SortEntitiesTable((*value).into()),
            StoredObjectType::DictionaryVariable(value) => Self::DictionaryVariable(*value),
            StoredObjectType::VisualStyle(value) => Self::VisualStyle(*value),
            StoredObjectType::Material(value) => Self::Material(*value),
            StoredObjectType::ImageDefinitionReactor(value) => Self::ImageDefinitionReactor(*value),
            StoredObjectType::GeoData(value) => Self::GeoData(*value),
            StoredObjectType::SpatialFilter(value) => Self::SpatialFilter(*value),
            StoredObjectType::RasterVariables(value) => Self::RasterVariables(*value),
            StoredObjectType::BookColor(value) => Self::BookColor(*value),
            StoredObjectType::PlaceHolder(value) => Self::PlaceHolder(*value),
            StoredObjectType::DictionaryWithDefault(value) => Self::DictionaryWithDefault(*value),
            StoredObjectType::WipeoutVariables(value) => Self::WipeoutVariables(*value),
            StoredObjectType::BlockVisibilityParameter(value) => {
                Self::BlockVisibilityParameter(*value)
            }
            StoredObjectType::DynamicBlock(value) => Self::DynamicBlock(*value),
            StoredObjectType::Associative(value) => Self::Associative(*value),
            StoredObjectType::ClassObject(value) => Self::ClassObject((*value).into()),
            StoredObjectType::DataObject(value) => Self::DataObject(*value),
            StoredObjectType::Field(value) => Self::Field(*value),
            StoredObjectType::FieldList(value) => Self::FieldList(*value),
            StoredObjectType::RegisteredClass(value) => Self::RegisteredClass((*value).into()),
            StoredObjectType::DgnLineStyle(value) => Self::DgnLineStyle(*value),
            StoredObjectType::ProxyObject(value) => Self::ProxyObject(*value),
            StoredObjectType::Unknown {
                type_name,
                handle,
                owner,
            } => Self::Unknown {
                type_name,
                handle,
                owner,
                raw_dxf_codes: None,
                raw_dwg_data: None,
                raw_dwg_handle_bits: 0,
                raw_dwg_version: None,
            },
        }
    }
}

pub(super) mod entity_records {
    use super::*;
    pub fn serialize<S: serde::Serializer>(values: &[EntityType], s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(
            &values
                .iter()
                .map(StoredEntityType::from)
                .collect::<Vec<_>>(),
            s,
        )
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        de: D,
    ) -> Result<Vec<EntityType>, D::Error> {
        let records: Vec<StoredEntityType> = serde::Deserialize::deserialize(de)?;
        Ok(records.into_iter().map(Into::into).collect())
    }
}
pub(super) mod object_records {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        values: &[(Handle, ObjectType)],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(
            &values
                .iter()
                .map(|(h, v)| (*h, StoredObjectType::from(v)))
                .collect::<Vec<_>>(),
            s,
        )
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        de: D,
    ) -> Result<Vec<(Handle, ObjectType)>, D::Error> {
        let records: Vec<(Handle, StoredObjectType)> = serde::Deserialize::deserialize(de)?;
        Ok(records.into_iter().map(|(h, v)| (h, v.into())).collect())
    }
}
