//! S06-02 proposed first-release data contract. No Engine/UI/Commands implementation.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

pub const MAX_SAFE_UINT: u64 = 9_007_199_254_740_991;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
        )]
        #[serde(try_from = "String", into = "String")]
        #[schemars(!try_from, !into)]
        pub struct $name(#[schemars(length(min = 1), pattern(r"^[^\u0000]+$"))] String);
        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() || value.contains('\0') {
                    Err("nonempty ID without NUL required".into())
                } else {
                    Ok(Self(value))
                }
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
#[ts(type = "number")]
pub struct SafeUint(#[schemars(range(min = 0, max = 9007199254740991u64))] u64);
impl TryFrom<u64> for SafeUint {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value <= MAX_SAFE_UINT {
            Ok(Self(value))
        } else {
            Err("outside JavaScript safe integer range".into())
        }
    }
}
impl From<SafeUint> for u64 {
    fn from(value: SafeUint) -> Self {
        value.0
    }
}
impl SafeUint {
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "f64", into = "f64")]
#[ts(type = "number")]
pub struct FiniteNumber(f64);
impl TryFrom<f64> for FiniteNumber {
    type Error = String;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err("finite value required".into())
        }
    }
}
impl From<FiniteNumber> for f64 {
    fn from(value: FiniteNumber) -> Self {
        value.0
    }
}
impl FiniteNumber {
    pub fn value(&self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "String", into = "String")]
#[schemars(!try_from, !into)]
pub struct RelativeFilePath(
    #[schemars(pattern(
        r"^(?!\.{1,2}(?:/|$))[^/\\:\u0000]+(?:/(?!\.{1,2}(?:/|$))[^/\\:\u0000]+)*$"
    ))]
    String,
);
impl TryFrom<String> for RelativeFilePath {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.contains(['\\', ':', '\0'])
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            Err("package-relative forward-slash file path required".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl From<RelativeFilePath> for String {
    fn from(value: RelativeFilePath) -> Self {
        value.0
    }
}
impl RelativeFilePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(transparent)]
pub struct Revision(pub SafeUint);
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(transparent)]
pub struct SimTimeMs(pub SafeUint);
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(transparent)]
pub struct DurationMs(pub SafeUint);

string_id!(ProjectId);
string_id!(ObjectId);
string_id!(ScenarioId);
string_id!(RunId);
string_id!(TaskId);
string_id!(ModuleId);
string_id!(AssetId);
string_id!(ArtifactId);
string_id!(RoleId);
string_id!(PortId);
string_id!(PhaseId);
string_id!(UnitId);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "String", into = "String")]
#[schemars(!try_from, !into)]
pub struct VersionId(
    #[schemars(pattern(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"))]
     String,
);
impl TryFrom<String> for VersionId {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        fn identifiers(s: &str, prerelease: bool) -> bool {
            s.split('.').all(|p| {
                !p.is_empty()
                    && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    && (!prerelease
                        || !p.bytes().all(|c| c.is_ascii_digit())
                        || p.len() == 1
                        || !p.starts_with('0'))
            })
        }
        let build: Vec<&str> = value.split('+').collect();
        if build.len() > 2 || (build.len() == 2 && !identifiers(build[1], false)) {
            return Err("invalid SemVer build identifiers".into());
        }
        let (core, prerelease) = match build[0].split_once('-') {
            Some((a, b)) => (a, Some(b)),
            None => (build[0], None),
        };
        let numbers: Vec<&str> = core.split('.').collect();
        if numbers.len() != 3
            || !numbers.iter().all(|n| {
                !n.is_empty()
                    && n.bytes().all(|c| c.is_ascii_digit())
                    && (n.len() == 1 || !n.starts_with('0'))
            })
            || prerelease.is_some_and(|s| !identifiers(s, true))
        {
            Err("exact SemVer required".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl From<VersionId> for String {
    fn from(value: VersionId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub enum SchemaVersion {
    #[serde(rename = "1.0")]
    V1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Project,
    Scenario,
    Run,
    DefinitionDraft,
    PackagingJob,
    Asset,
    NativeScene,
    ItemType,
    Item,
    Location,
    Hall,
    Line,
    Device,
    Resource,
    Calendar,
    Product,
    Bom,
    Route,
    Connection,
    Order,
    Task,
    Job,
    Supply,
    Transport,
    Artifact,
    Module,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    ArtifactRecord,
    Project,
    Scenario,
    Module,
    Template,
    GenerationProfile,
    Presentation,
    PackagingJob,
    PackagingResult,
    RunInput,
    RunResult,
    Evaluation,
    Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRole {
    ActiveProject,
    ScenarioInput,
    ModuleDefinition,
    TemplateData,
    GenerationProfile,
    Presentation,
    Asset,
    ProfessionalInput,
    ProfessionalResult,
    ArtifactMetadata,
    FrozenRunProject,
    FrozenRunScenario,
    RunResult,
    Evaluation,
    SupportingFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum SourceBasis {
    UserProvided,
    ModuleDeclared,
    TemplateDeclaredExample,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    Meter,
    Millimeter,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateFrame {
    RightHandedZUp,
    RightHandedYUp,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalTimeUnit {
    Millisecond,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TrackingKind {
    Individual,
    Batch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Available,
    InProcess,
    Transformed,
    Scrapped,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum QualityState {
    Unchecked,
    Pending,
    Released,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AllocationMode {
    SoftPlan,
    HardReserved,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LocationPurpose {
    Input,
    Buffer,
    Output,
    Pending,
    Isolation,
    ExternalDestination,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Worker,
    Fixture,
    Vehicle,
    ReusableCarrier,
    ServiceDeclaration,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceStage {
    FactoryReleased,
    Received,
    UnloadedAndReleased,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePolicy {
    UncheckedAllowed,
    InspectionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionState {
    Design,
    EditableExample,
    RuntimeCandidate,
    Released,
    Unmaintained,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    Number,
    Integer,
    Boolean,
    String,
    Enum,
    Vector3,
    Duration,
    Quantity,
    ItemType,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PlacementKind {
    FixedDuringSimulation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorSourceKind {
    Inline,
    OperationRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Input,
    Output,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    ManualPreparation,
    SupportedAutomatic,
    DeferredMaintenance,
    DeferredConstruction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentAccess {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    Transform,
    Assemble,
    InspectAndRoute,
    Rework,
    Pack,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionPolicy {
    NonPreemptive,
    PauseResume,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PhaseWorkKind {
    Loading,
    Processing,
    Unloading,
    Inspection,
    Packing,
    Preparation,
    Transport,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PendingPolicy {
    HoldStation,
    PendingStore,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum IdentityPolicy {
    Preserve,
    TransformWithOrigin,
    AssemblyWithComposition,
    ContainerWithContents,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TransportBuiltin {
    Transport,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TransportIdentityPolicy {
    PreserveCargoIds,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DestinationConditionKind {
    Capacity,
    QualityReleased,
    DeclaredReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TemplateScope {
    Hall,
    Line,
    Region,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProfessionalMode {
    DeclaredProcess,
    CalculatedNominal,
    SoftwareInLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    NotImplemented,
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Png,
    Jpeg,
    Gif,
    Glb,
    Svg,
    Csv,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PresentationKind {
    Image,
    ImageSequence,
    Gif,
    Model,
    NativeScene,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum NativeTransition {
    Hold,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImageTransition {
    Hold,
    Crossfade,
    Pan,
    Zoom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RegionPolicy {
    DeviceIcons,
    LineWhole,
    RegionWhole,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProductionEffect {
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CanvasKind {
    LineCanvas,
    MultiHallCanvas,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LodLevel {
    Detailed,
    Simplified,
    Region,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DeviceAction {
    Configure,
    Inspect,
    PreviewCycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PackagingMechanism {
    RestrictedFoldedContainer,
    OuterBoxAndInnerLiner,
    CoverOrSlidingSleeve,
    BlankRestrictedStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PackagingChangeKind {
    ProductFunction,
    MaterialApproved,
    PartRelationsOrRepeatOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMaterialKind {
    Glue,
    MechanicalTab,
    FixedSeam,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PackagingStepPurpose {
    Forming,
    Opening,
    LinearExtraction,
    NetSpace,
    Manufacturing,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Pass,
    Fail,
    Undetermined,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum SvgLayer {
    Cut,
    Fold,
    Glue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum QualitySourceKind {
    Preset,
    ToolResult,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum QualityDisposition {
    Pending,
    Release,
    Reinspect,
    Rework,
    Isolate,
    Scrap,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClaimKind {
    Job,
    Phase,
    OutputHold,
    Transport,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Ready,
    Working,
    Paused,
    WaitingResource,
    WaitingOutput,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TransportPhase {
    Reserved,
    Loading,
    Outbound,
    WaitingReceive,
    Unloading,
    Returning,
    Returned,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum WaitReasonKind {
    MissingMaterial,
    ResourceBusy,
    Calendar,
    Preparation,
    Quality,
    OutputCapacity,
    ReceiverCapacity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComputationStatus {
    Queued,
    Running,
    Paused,
    CancelRequested,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LongTaskKind {
    Simulation,
    Packaging,
    Rendering,
    Importing,
    Exporting,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DraftCheckKind {
    ReferencesAndUnits,
    MaterialProcessConnectivity,
    SiteBounds,
    FootprintOverlap,
    Height,
    ClearanceAndStaticLanes,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStopReason {
    InitialTargetMet,
    RepairedTargetMet,
    MissingConditions,
    UnrepairableShortfall,
    FiniteBudgetExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CostBasis {
    ConsumedMaterial,
    OccupiedTime,
    PaidCalendar,
    BatchTransport,
    ExplicitDisposal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReportMode {
    DesignDescription,
    SimulationEvaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    DesignPreview,
    ActualPresentation,
    ActualRunView,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    RestartFromInitialInput,
}

// Core and project wire/persistence data.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TimeReference {
    pub label: String,
    #[ts(optional = nullable)]
    pub origin_iso8601: Option<String>,
    #[ts(optional = nullable)]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct WindowMs {
    pub start_ms: SimTimeMs,
    pub end_ms: SimTimeMs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct IntervalMs {
    pub start_ms: SimTimeMs,
    pub end_ms: SimTimeMs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ModuleRef {
    pub id: ModuleId,
    pub version: VersionId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SubjectRevisionRef {
    pub id: ObjectId,
    pub revision: SubjectRevisionId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ObjectRef {
    pub kind: ObjectKind,
    pub id: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    pub path: RelativeFilePath,
    pub kind: DocumentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct AssetRef {
    pub id: AssetId,
    pub path: RelativeFilePath,
    pub kind: AssetKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub id: ObjectId,
    pub title: String,
    #[ts(optional = nullable)]
    pub locator: Option<String>,
    #[ts(optional = nullable)]
    pub read_scope: Option<String>,
    #[ts(optional = nullable)]
    pub read_date: Option<String>,
    pub basis: SourceBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub value: DecimalAmount,
    pub unit: UnitId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DurationValue {
    pub value: DurationMs,
    pub unit: CanonicalTimeUnit,
    pub basis: SourceBasis,
    #[ts(optional = nullable)]
    pub source_ref: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Vec2 {
    pub x: FiniteNumber,
    pub y: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Vec3 {
    pub x: FiniteNumber,
    pub y: FiniteNumber,
    pub z: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Quaternion {
    pub x: FiniteNumber,
    pub y: FiniteNumber,
    pub z: FiniteNumber,
    pub w: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Pose {
    pub translation: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
    pub unit: LengthUnit,
    pub coordinate_frame: CoordinateFrame,
}

/// Explicit snapshot display placement; never business capacity or permanent occupancy.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ItemSlotAssignment {
    ItemIdOrder,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ItemDisplaySlot {
    pub item_type_ref: SubjectRevisionRef,
    #[ts(optional = nullable)]
    pub anchor_role: Option<RoleId>,
    pub offset: Pose,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ItemSlotLayout {
    pub assignment: ItemSlotAssignment,
    pub slots: Vec<ItemDisplaySlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub code: String,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    #[ts(optional = nullable)]
    pub field_path: Option<String>,
    pub message: String,
    pub blocks: Vec<CapabilityPurpose>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct AssetRights {
    pub asset_ref: AssetId,
    #[ts(optional = nullable)]
    pub source: Option<String>,
    #[ts(optional = nullable)]
    pub license: Option<String>,
    pub redistribution_confirmed: bool,
    #[ts(optional = nullable)]
    pub missing_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    #[ts(optional = nullable)]
    pub author: Option<String>,
    #[ts(optional = nullable)]
    pub license: Option<String>,
    pub assets: Vec<AssetRights>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct AttributeDeclaration {
    pub name: String,
    pub value_type: ValueKind,
    #[ts(optional = nullable)]
    pub unit: Option<UnitId>,
    #[ts(optional = nullable)]
    pub shape: Option<Vec<SafeUint>>,
    pub options: Vec<String>,
    pub basis: SourceBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ParameterDeclaration {
    pub id: ObjectId,
    pub value_type: ValueKind,
    pub binding: String,
    #[ts(optional = nullable)]
    pub label: Option<String>,
    #[ts(optional = nullable)]
    pub unit: Option<UnitId>,
    #[ts(optional = nullable)]
    pub minimum: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub maximum: Option<FiniteNumber>,
    pub integer: bool,
    pub options: Vec<String>,
    pub basis: SourceBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RoleQuantity {
    pub role: RoleId,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RoleBindings {
    pub item_types: BTreeMap<RoleId, ObjectId>,
    pub resources: BTreeMap<RoleId, ObjectId>,
    pub targets: BTreeMap<RoleId, ObjectRef>,
    pub ports: BTreeMap<RoleId, PortRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PortRef {
    pub module_ref: ModuleRef,
    pub instance_id: ObjectId,
    pub port_id: PortId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DataFieldDeclaration {
    pub name: String,
    pub value_type: ValueKind,
    #[ts(optional = nullable)]
    pub unit: Option<UnitId>,
    pub required: bool,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DocumentEntry {
    pub role: DocumentRole,
    #[ts(optional = nullable)]
    pub module_ref: Option<ModuleRef>,
    pub object_ref: ObjectRef,
    #[ts(optional = nullable)]
    pub revision: Option<Revision>,
    pub file_ref: FileRef,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub project_id: ProjectId,
    pub project_revision: Revision,
    pub title: String,
    pub generation: SafeUint,
    pub documents: Vec<DocumentEntry>,
    pub module_refs: Vec<ModuleRef>,
    pub assets: Vec<AssetRights>,
    pub missing: Vec<Issue>,
    pub restart_policy: RestartPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ItemType {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub contents_layout: Option<ItemSlotLayout>,
    pub id: ObjectId,
    pub revision: SubjectRevisionId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub module_ref: Option<ModuleRef>,
    pub unit: UnitId,
    pub tracking: TrackingKind,
    #[ts(optional = nullable)]
    pub spec_ref: Option<SubjectRevisionRef>,
    pub attributes: BTreeMap<String, TypedValue>,
    pub attribute_schema: Vec<AttributeDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct InitialItem {
    pub id: ObjectId,
    pub item_type_ref: SubjectRevisionRef,
    pub quantity: Quantity,
    pub holder: HolderRef,
    pub quality: QualityState,
    pub components: Vec<ObjectId>,
    pub contents: Vec<ObjectId>,
    pub source: SourceBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct StockLocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub item_layout: Option<ItemSlotLayout>,
    pub id: ObjectId,
    pub title: String,
    #[ts(optional = nullable)]
    pub hall_id: Option<ObjectId>,
    pub position: Pose,
    #[ts(optional = nullable)]
    pub capacity: Option<Quantity>,
    pub accepted_item_types: Vec<ObjectId>,
    pub purpose: LocationPurpose,
    pub ordinary_issue_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct HallDesign {
    pub id: ObjectId,
    pub title: String,
    pub width_m: FiniteNumber,
    pub depth_m: FiniteNumber,
    #[ts(optional = nullable)]
    pub height_m: Option<FiniteNumber>,
    pub origin: Pose,
    pub input_locations: Vec<ObjectId>,
    pub output_locations: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct LineGroup {
    pub id: ObjectId,
    pub title: String,
    pub hall_id: ObjectId,
    pub transform: Pose,
    pub members: Vec<ObjectId>,
    pub boundary_ports: Vec<PortRef>,
    pub region_polygon_m: Vec<Vec2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceInstance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub item_layout: Option<ItemSlotLayout>,
    pub id: ObjectId,
    pub title: String,
    pub hall_id: ObjectId,
    #[ts(optional = nullable)]
    pub line_id: Option<ObjectId>,
    pub module_ref: ModuleRef,
    pub pose: Pose,
    pub footprint_m: Vec<Vec2>,
    #[ts(optional = nullable)]
    pub height_m: Option<FiniteNumber>,
    pub configuration: BTreeMap<String, TypedValue>,
    pub role_bindings: RoleBindings,
    #[ts(optional = nullable)]
    pub presentation_override: Option<Presentation>,
    #[ts(optional = nullable)]
    pub operation_binding: Option<ModuleRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_operation_refs: Vec<ModuleRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceInstance {
    pub id: ObjectId,
    pub definition_ref: ModuleRef,
    pub title: String,
    pub kind: ResourceKind,
    pub role: RoleId,
    pub capacity: Quantity,
    pub qualifications: Vec<String>,
    pub calendar_id: ObjectId,
    #[ts(optional = nullable)]
    pub home_location: Option<ObjectId>,
    #[ts(optional = nullable)]
    pub presentation: Option<Presentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    pub id: ObjectId,
    pub title: String,
    pub intervals: Vec<IntervalMs>,
    pub source: SourceBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProductRevision {
    pub id: ObjectId,
    pub revision: SubjectRevisionId,
    pub item_type_ref: SubjectRevisionRef,
    pub bom_ref: SubjectRevisionRef,
    pub routing_ref: SubjectRevisionRef,
    pub artifact_refs: Vec<ArtifactId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct BomRow {
    pub item_type_ref: SubjectRevisionRef,
    pub quantity: Quantity,
    pub consume_at_operation: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct BomDefinition {
    pub id: ObjectId,
    pub revision: SubjectRevisionId,
    pub rows: Vec<BomRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RouteStep {
    pub id: ObjectId,
    pub operation_ref: ModuleRef,
    pub device_id: ObjectId,
    pub predecessors: Vec<ObjectId>,
    pub item_bindings: BTreeMap<RoleId, ObjectId>,
    pub resource_bindings: BTreeMap<RoleId, ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RouteDefinition {
    pub id: ObjectId,
    pub revision: SubjectRevisionId,
    pub steps: Vec<RouteStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct OrderLine {
    pub id: ObjectId,
    pub product_ref: SubjectRevisionRef,
    #[ts(optional = nullable)]
    pub quantity: Option<Quantity>,
    pub release_at_ms: SimTimeMs,
    #[ts(optional = nullable)]
    pub due_at_ms: Option<SimTimeMs>,
    pub destination: ObjectId,
    pub acceptance_stage: AcceptanceStage,
    pub allow_partial: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct OrderAllocation {
    pub order_id: ObjectId,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProductionTask {
    pub id: ObjectId,
    pub routing_ref: SubjectRevisionRef,
    pub order_allocations: Vec<OrderAllocation>,
    pub planned_quantity: Quantity,
    pub release_at_ms: SimTimeMs,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ArrivalPlan {
    pub id: ObjectId,
    pub at_ms: SimTimeMs,
    pub destination: ObjectId,
    pub items: Vec<InitialItem>,
    pub release_policy: ReleasePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SupplyRelation {
    pub id: ObjectId,
    pub source: ObjectId,
    pub destination: ObjectId,
    pub item_type_ref: SubjectRevisionRef,
    pub target_quantity: Quantity,
    pub window: WindowMs,
    pub transport_ref: ModuleRef,
    pub resources: BTreeMap<RoleId, ObjectId>,
    pub departure_windows: Vec<IntervalMs>,
    pub allow_partial_load: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct InitialConditions {
    pub items: Vec<InitialItem>,
    pub initial_setup_families: BTreeMap<ObjectId, String>,
    pub declared_quality_decisions: Vec<QualityDecision>,
    pub material_allocations: Vec<MaterialAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub acceptance_goal: Option<FactoryGenerationGoal>,
    pub quality_presets: Vec<InspectionPreset>,
    pub id: ScenarioId,
    pub revision: Revision,
    pub project_revision: Revision,
    pub title: String,
    pub time_reference: TimeReference,
    pub window: WindowMs,
    pub initial_conditions: InitialConditions,
    pub orders: Vec<OrderLine>,
    pub production_tasks: Vec<ProductionTask>,
    pub arrivals: Vec<ArrivalPlan>,
    pub supplies: Vec<SupplyRelation>,
    pub parameter_overrides: Vec<ParameterOverride>,
    pub scope: Vec<ObjectRef>,
    pub issues: Vec<Issue>,
    pub cost_inputs: Vec<CostInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ParameterOverride {
    pub target: ObjectRef,
    pub path: String,
    pub value: TypedValue,
    pub basis: SourceBasis,
    #[ts(optional = nullable)]
    pub source_ref: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectState {
    pub setup_policies: Vec<DeviceSetupPolicy>,
    pub id: ProjectId,
    pub revision: Revision,
    pub title: String,
    pub time_reference: TimeReference,
    pub item_types: Vec<ItemType>,
    pub locations: Vec<StockLocation>,
    pub halls: Vec<HallDesign>,
    pub lines: Vec<LineGroup>,
    pub devices: Vec<DeviceInstance>,
    pub resources: Vec<ResourceInstance>,
    pub calendars: Vec<Calendar>,
    pub products: Vec<ProductRevision>,
    pub boms: Vec<BomDefinition>,
    pub routes: Vec<RouteDefinition>,
    pub connections: Vec<Connection>,
    pub module_refs: Vec<ModuleRef>,
    pub scenario_refs: Vec<ScenarioId>,
    pub artifact_refs: Vec<ArtifactId>,
    pub acknowledgments: Vec<ArtifactAcknowledgment>,
    pub bindings: Vec<ConsumerBinding>,
    pub views: ViewState,
    #[ts(optional = nullable)]
    pub generation_context: Option<GenerationContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_sources: Vec<ParameterSource>,
    pub sources: Vec<SourceRecord>,
    pub issues: Vec<Issue>,
    pub template_adoptions: Vec<TemplateAdoption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewState {
    pub normal_canvas: CanvasKind,
    #[ts(optional = nullable)]
    pub selected_hall: Option<ObjectId>,
    pub selected_objects: Vec<ObjectRef>,
    pub lod: LodLevel,
    pub line_camera: CameraState,
    pub multi_hall_camera: CameraState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CameraState {
    pub coordinate_frame: CoordinateFrame,
    pub unit: LengthUnit,
    pub eye: Vec3,
    pub target: Vec3,
    pub zoom: FiniteNumber,
}

// Nine categories and presentation; availability comes only from Host registration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ModuleDefinition {
    pub id: ModuleId,
    pub version: VersionId,
    pub title: String,
    pub definition_state: DefinitionState,
    pub contribution: Contribution,
    pub parameters: Vec<ParameterDeclaration>,
    pub attribute_schema: Vec<AttributeDeclaration>,
    pub payload: ModulePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ModulePackage {
    pub assets: Vec<AssetRef>,
    pub definition: ModuleDefinition,
    pub files: Vec<FileRef>,
    pub dependencies: Vec<ModuleRef>,
    pub rights: Vec<AssetRights>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ItemDefinition {
    #[ts(optional = nullable)]
    pub presentation_or_media_ref: Option<Presentation>,
    #[ts(optional = nullable)]
    pub unit: Option<UnitId>,
    #[ts(optional = nullable)]
    pub tracking: Option<TrackingKind>,
    #[ts(optional = nullable)]
    pub spec_ref: Option<SubjectRevisionRef>,
    pub attributes: BTreeMap<String, TypedValue>,
    pub attribute_schema: Vec<AttributeDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceDefinition {
    pub placement: PlacementKind,
    pub ports: Vec<PortDefinition>,
    pub behavior: BehaviorSources,
    pub configuration: BTreeMap<String, TypedValue>,
    #[ts(optional = nullable)]
    pub presentation: Option<Presentation>,
    pub operations: Vec<DeviceAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct BehaviorSources {
    #[ts(optional = nullable)]
    pub inline: Option<OperationDefinition>,
    #[ts(optional = nullable)]
    pub operation_ref: Option<ModuleRef>,
    #[ts(optional = nullable)]
    pub selected: Option<BehaviorSourceKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PortDefinition {
    pub id: PortId,
    pub direction: Direction,
    pub payload: PortPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceDefinition {
    #[ts(optional = nullable)]
    pub kind: Option<ResourceKind>,
    #[ts(optional = nullable)]
    pub role: Option<RoleId>,
    #[ts(optional = nullable)]
    pub capacity: Option<Quantity>,
    pub qualifications: Vec<String>,
    pub default_calendar_ms: Vec<IntervalMs>,
    #[ts(optional = nullable)]
    pub presentation: Option<Presentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskDefinition {
    pub kind: TaskKind,
    #[ts(optional = nullable)]
    pub target_role: Option<RoleId>,
    pub prerequisites: Vec<ReadinessRef>,
    pub steps: Vec<TaskStep>,
    pub resource_roles: Vec<ResourceRequirement>,
    #[ts(optional = nullable)]
    pub duration: Option<DurationValue>,
    pub equipment_access: EquipmentAccess,
    pub completion_effect: ReadinessEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ReadinessRef {
    pub target_role: RoleId,
    pub marker: String,
    #[ts(optional = nullable)]
    pub required_revision: Option<Revision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ReadinessEffect {
    pub target_role: RoleId,
    pub marker: String,
    pub produces_material: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskStep {
    pub id: PhaseId,
    pub target_role: RoleId,
    pub resources: Vec<ResourceRequirement>,
    #[ts(optional = nullable)]
    pub duration: Option<DurationValue>,
    pub completion_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct OperationDefinition {
    pub builtin: RuleKind,
    pub item_roles: Vec<RoleId>,
    pub inputs: Vec<MaterialRoleQuantity>,
    pub outputs: Vec<MaterialRoleQuantity>,
    pub resource_roles: Vec<ResourceRequirement>,
    #[ts(optional = nullable)]
    pub duration: Option<DurationValue>,
    pub phases: Vec<PhaseDefinition>,
    pub hold_on_output_wait: Vec<RoleId>,
    #[ts(optional = nullable)]
    pub quality_and_routes: Option<QualityRule>,
    pub identity_policy: IdentityPolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub pack_identity: Option<PackIdentityRule>,
    #[ts(optional = nullable)]
    pub family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackIdentityRule {
    pub output_role: RoleId,
    pub container_material_roles: Vec<RoleId>,
    pub content_roles: Vec<RoleId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialRoleQuantity {
    pub role: RoleId,
    pub quantity: Quantity,
    pub port: PortId,
    pub required_quality: Vec<QualityState>,
    #[ts(optional = nullable)]
    pub declared_output_quality: Option<QualityState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PhaseDefinition {
    pub id: PhaseId,
    #[ts(optional = nullable)]
    pub duration: Option<DurationValue>,
    pub phase_resources: Vec<ResourceRequirement>,
    pub interruption_policy: InterruptionPolicy,
    pub work_kind: PhaseWorkKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct QualityRule {
    pub source_kind: QualitySourceKind,
    #[ts(optional = nullable)]
    pub result_ref: Option<ObjectId>,
    pub max_completed_reworks: SafeUint,
    pub pass_port: PortId,
    pub fail_port: PortId,
    pub pending_policy: PendingPolicy,
    #[ts(optional = nullable)]
    pub pending_location_role: Option<RoleId>,
    #[ts(optional = nullable)]
    pub isolation_location_role: Option<RoleId>,
    pub release_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TransportDefinition {
    pub ports: Vec<PortDefinition>,
    pub builtin: TransportBuiltin,
    #[ts(optional = nullable)]
    pub batch_capacity: Option<Quantity>,
    pub resources: Vec<ResourceRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loading_resources: Vec<ResourceRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unloading_resources: Vec<ResourceRequirement>,
    pub times: TransportTimes,
    pub acceptance_conditions: Vec<DestinationCondition>,
    pub identity_policy: TransportIdentityPolicy,
    #[ts(optional = nullable)]
    pub presentation: Option<Presentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TransportTimes {
    #[ts(optional = nullable)]
    pub loading: Option<DurationValue>,
    #[ts(optional = nullable)]
    pub outbound: Option<DurationValue>,
    #[ts(optional = nullable)]
    pub unloading: Option<DurationValue>,
    #[ts(optional = nullable)]
    pub returning: Option<DurationValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DestinationCondition {
    pub kind: DestinationConditionKind,
    #[ts(optional = nullable)]
    pub target_role: Option<RoleId>,
    #[ts(optional = nullable)]
    pub required_value: Option<TypedValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateDefinition {
    pub project: RelativeFilePath,
    #[ts(optional = nullable)]
    pub generation_profile: Option<RelativeFilePath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateSubset {
    pub layout_scope: TemplateScope,
    pub halls: Vec<HallDesign>,
    pub lines: Vec<LineGroup>,
    pub devices: Vec<DeviceInstance>,
    pub connections: Vec<Connection>,
    pub module_refs: Vec<ModuleRef>,
    pub resource_requirements: Vec<ResourceDefinition>,
    pub item_types: Vec<ItemType>,
    pub role_requirements: Vec<RoleQuantity>,
    pub suggested_role_bindings: RoleBindings,
    #[ts(optional = nullable)]
    pub seed: Option<TemplateSeed>,
    pub locations: Vec<StockLocation>,
    #[ts(optional = nullable)]
    pub adoption: Option<TemplateAdoption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateSeed {
    pub initial_conditions: InitialConditions,
    pub orders: Vec<OrderLine>,
    pub arrivals: Vec<ArrivalPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProfessionalDefinition {
    pub input_fields: Vec<DataFieldDeclaration>,
    pub output_artifact_type: String,
    pub output_fields: Vec<DataFieldDeclaration>,
    pub intended_modes: Vec<ProfessionalMode>,
    pub unit: UnitId,
    pub coordinate_frame: CoordinateFrame,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProviderCapability {
    pub provider_id: ModuleId,
    pub available_modes: Vec<ProfessionalMode>,
    pub producer_status: ProviderStatus,
    pub supported_input_schema: String,
    pub supported_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DefinitionDraft {
    pub id: ModuleId,
    pub revision: Revision,
    pub definition: ModuleDefinition,
    pub issues: Vec<Issue>,
    pub edit_history_version: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    pub kind: PresentationKind,
    pub source: MediaLocator,
    pub unit: LengthUnit,
    pub coordinate_frame: CoordinateFrame,
    pub scale: Vec3,
    pub nodes: Vec<SceneNode>,
    pub poses: Vec<NamedPose>,
    pub actions: Vec<MotionTrack>,
    pub timeline: Vec<TimelineClip>,
    pub tags: Vec<DeviceTag>,
    pub anchor_roles: BTreeMap<RoleId, AnchorRef>,
    pub port_anchors: BTreeMap<PortId, AnchorRef>,
    pub lod: LodPresentation,
    pub production_effect: ProductionEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SceneNode {
    pub id: SafeUint,
    pub name: String,
    #[ts(optional = nullable)]
    pub parent_index: Option<SafeUint>,
    pub transform: Pose,
    pub geometry: SceneGeometry,
    pub material: SceneMaterial,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SceneMaterial {
    pub color_rgba: [FiniteNumber; 4],
    pub metallic: FiniteNumber,
    pub roughness: FiniteNumber,
    #[ts(optional = nullable)]
    pub texture: Option<AssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct NamedPose {
    pub id: ObjectId,
    pub node_transforms: Vec<NodePoseTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MotionTrack {
    pub id: ObjectId,
    pub node_index: SafeUint,
    pub keys: Vec<PoseKey>,
    pub transition: NativeTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PoseKey {
    pub at_progress: FiniteNumber,
    pub pose: Pose,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TimelineClip {
    pub id: ObjectId,
    #[ts(optional = nullable)]
    pub phase_ref: Option<PhaseId>,
    pub start_progress: FiniteNumber,
    pub end_progress: FiniteNumber,
    pub source: ClipSource,
    pub transition: ImageTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceTag {
    pub id: ObjectId,
    pub label: String,
    pub anchor: AnchorRef,
    #[ts(optional = nullable)]
    pub progress_range: Option<[FiniteNumber; 2]>,
    pub payload: TagPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct LodPresentation {
    #[ts(optional = nullable)]
    pub detailed: Option<MediaLocator>,
    #[ts(optional = nullable)]
    pub simplified: Option<MediaLocator>,
    pub region: RegionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RenderViewSpec {
    pub kind: RenderViewKind,
    pub canvas: CanvasKind,
    #[ts(optional = nullable)]
    pub hall_id: Option<ObjectId>,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    pub project_revision: Revision,
    #[ts(optional = nullable)]
    pub run_id: Option<RunId>,
    #[ts(optional = nullable)]
    pub view_time_ms: Option<SimTimeMs>,
    pub lod: LodLevel,
    pub camera: CameraState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RenderedSceneRef {
    pub evidence_origin: EvidenceOrigin,
    pub id: ObjectId,
    pub file: AssetRef,
    pub view: RenderViewSpec,
    pub scene_revision: Revision,
}

// Full selected packaging and readonly runtime DTOs; no algorithm implementation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PanelDefinition {
    pub id: ObjectId,
    pub part_id: ObjectId,
    pub polygon_mm: Vec<Vec2>,
    pub thickness_mm: FiniteNumber,
    pub material_ref: ObjectId,
    pub flat_pose: Pose,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FoldBand {
    pub id: ObjectId,
    pub panel_a: ObjectId,
    pub panel_b: ObjectId,
    pub axis_start_mm: Vec3,
    pub axis_end_mm: Vec3,
    pub width_mm: FiniteNumber,
    pub target_angle_rad: FiniteNumber,
    pub allowed_range_rad: [FiniteNumber; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FixedConnection {
    pub id: ObjectId,
    pub panel_a: ObjectId,
    pub panel_b: ObjectId,
    pub frame_a: Pose,
    pub frame_b: Pose,
    pub kind: ConnectionMaterialKind,
    pub allow_contact: bool,
    pub contact_allowance_refs: Vec<ObjectId>,
    #[ts(optional = nullable)]
    pub engage_after_step_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PanelFaceSide {
    Front,
    Back,
}

/// Local panel-face patches, never an exemption for an entire object pair.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct LocalContactAllowance {
    pub id: ObjectId,
    pub panel_a: ObjectId,
    pub panel_b: ObjectId,
    pub face_a: PanelFaceSide,
    pub face_b: PanelFaceSide,
    pub polygon_a_mm: Vec<Vec2>,
    pub polygon_b_mm: Vec<Vec2>,
    pub normal_overlap_limit_mm: FiniteNumber,
    pub active_step_ids: Vec<ObjectId>,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagePart {
    pub id: ObjectId,
    pub title: String,
    pub panel_ids: Vec<ObjectId>,
    pub root_panel: ObjectId,
    pub placement: Pose,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialDeclaration {
    pub id: ObjectId,
    pub title: String,
    pub thickness_mm: FiniteNumber,
    pub basis: SourceBasis,
    #[ts(optional = nullable)]
    pub source_ref: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProductEnvelope {
    pub subject: SubjectRevisionRef,
    pub vertices_mm: Vec<Vec3>,
    pub allowed_poses: Vec<Pose>,
    pub clearance_mm: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingConditions {
    pub net_space_required: bool,
    pub forming_required: bool,
    pub opening_required: bool,
    pub extraction_required: bool,
    pub allowed_contacts: Vec<LocalContactAllowance>,
    #[ts(optional = nullable)]
    pub product_envelope: Option<ProductEnvelope>,
    #[ts(optional = nullable)]
    pub extraction: Option<ExtractionPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ExtractionPlan {
    pub part_ref: ObjectId,
    pub product_pose: Pose,
    pub direction: Vec3,
    pub distance_mm: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingStep {
    pub id: ObjectId,
    pub purpose: PackagingStepPurpose,
    #[ts(optional = nullable)]
    pub duration_hint_ms: Option<DurationMs>,
    pub motion: PackagingMotion,
    pub prerequisites: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct NumericTolerance {
    pub distance_mm: FiniteNumber,
    pub angle_rad: FiniteNumber,
    pub constraint_mm: FiniteNumber,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CalculationBudget {
    pub candidate_limit: SafeUint,
    pub interval_query_limit: SafeUint,
    pub subdivision_limit: SafeUint,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackageStructureJob {
    pub variables: Vec<StructuralVariable>,
    pub dimension_constraints: Vec<DimensionConstraint>,
    pub id: ObjectId,
    pub revision: Revision,
    pub mechanism: PackagingMechanism,
    pub changes: Vec<PackagingChangeKind>,
    pub subject_ref: SubjectRevisionRef,
    pub panels: Vec<PanelDefinition>,
    pub fold_bands: Vec<FoldBand>,
    pub connections: Vec<FixedConnection>,
    pub parts: Vec<PackagePart>,
    pub materials: Vec<MaterialDeclaration>,
    pub conditions: PackagingConditions,
    pub steps: Vec<PackagingStep>,
    #[ts(optional = nullable)]
    pub tolerance: Option<NumericTolerance>,
    #[ts(optional = nullable)]
    pub budget: Option<CalculationBudget>,
    pub unit: LengthUnit,
    pub coordinate_frame: CoordinateFrame,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingWitness {
    pub step_id: ObjectId,
    #[ts(optional = nullable)]
    pub object_pair: Option<[ObjectId; 2]>,
    pub interval: [FiniteNumber; 2],
    #[ts(optional = nullable)]
    pub contact_point_mm: Option<Vec3>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingCheckResult {
    pub purpose: PackagingStepPurpose,
    pub outcome: CheckOutcome,
    pub witnesses: Vec<PackagingWitness>,
    pub unresolved: Vec<Issue>,
    pub queries_used: SafeUint,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ManufacturingFile {
    pub part_id: ObjectId,
    pub file: FileRef,
    pub layers: Vec<SvgLayer>,
    pub unit: LengthUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialCount {
    pub material_id: ObjectId,
    pub part_id: ObjectId,
    pub quantity: SafeUint,
    #[ts(optional = nullable)]
    pub area_mm2: Option<FiniteNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingResult {
    pub evidence_origin: EvidenceOrigin,
    pub artifact_id: ArtifactId,
    pub job_ref: JobInputRevisionRef,
    pub subject_ref: SubjectRevisionRef,
    pub outcome: CheckOutcome,
    pub checks: Vec<PackagingCheckResult>,
    pub process_poses: Vec<PackagingProcessSample>,
    pub manufacturing_files: Vec<ManufacturingFile>,
    pub material_counts: Vec<MaterialCount>,
    pub budget_used: BudgetUsage,
    pub supported_scope: Vec<String>,
    pub limitations: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub evidence_origin: EvidenceOrigin,
    pub id: ArtifactId,
    pub artifact_type: String,
    pub schema_version: SchemaVersion,
    pub subject_ref: SubjectRevisionRef,
    pub input_ref: FileRef,
    pub result_ref: FileRef,
    pub unit: UnitId,
    pub coordinate_frame: CoordinateFrame,
    pub supported_scope: Vec<String>,
    pub actual_outcome: CheckOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAcknowledgment {
    pub id: ObjectId,
    pub artifact_ref: ArtifactId,
    pub accepted_subject: SubjectRevisionRef,
    pub accepted_scope: Vec<String>,
    pub decision_source: DecisionSource,
    pub conditions: Vec<Issue>,
    #[ts(optional = nullable)]
    pub recorded_at_iso8601: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ConsumerBinding {
    pub id: ObjectId,
    pub consumer_ref: ObjectRef,
    pub operation_ref: ModuleRef,
    pub artifact_ref: ArtifactId,
    pub expected_subject_revision: SubjectRevisionId,
    pub required_payload: Vec<String>,
    pub requested_scope: Vec<String>,
    pub acceptance_ref: ObjectId,
    pub mode: ProfessionalMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialAllocation {
    pub id: ObjectId,
    pub demand_ref: ObjectId,
    pub item_refs: Vec<ObjectId>,
    pub quantity: Quantity,
    pub mode: AllocationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RuntimeItem {
    pub id: ObjectId,
    pub item_type_ref: SubjectRevisionRef,
    pub quantity: Quantity,
    pub holder: HolderRef,
    pub lifecycle: Lifecycle,
    pub quality: QualityState,
    pub allocation_refs: Vec<ObjectId>,
    pub composition_refs: Vec<ObjectId>,
    pub contents: Vec<ObjectId>,
    pub completed_reworks: SafeUint,
    #[ts(optional = nullable)]
    pub split_from_ref: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ClaimIntervalEndCause {
    Released,
    ObservationEnd,
    ComputedBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceClaim {
    pub id: ObjectId,
    pub resource_id: ObjectId,
    pub job_id: ObjectId,
    pub units: SafeUint,
    pub intervals: Vec<IntervalMs>,
    pub kind: ResourceClaimKind,
    #[ts(optional = nullable)]
    pub phase_ref: Option<PhaseId>,
    /// One declared end cause per interval, in the same ordinal order.
    pub interval_end_causes: Vec<ClaimIntervalEndCause>,
}

/// Frozen execution identity; operation/module identity is not device identity.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum JobExecutionRef {
    ProjectDevice {
        device_id: ObjectId,
        #[ts(optional = nullable)]
        route_step_id: Option<ObjectId>,
        execution_capacity_ref: ObjectRef,
    },
    DefinitionCycle {
        module_ref: ModuleRef,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PhaseWorkEndCause {
    PhaseCompleted,
    Interrupted,
    ComputedBoundary,
}

/// Positive, half-open effective work, excluding waiting and calendar pauses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PhaseWorkInterval {
    pub job_id: ObjectId,
    pub phase_id: PhaseId,
    pub interval: IntervalMs,
    pub effective_progress_start_ms: DurationMs,
    pub effective_progress_end_ms: DurationMs,
    pub end_cause: PhaseWorkEndCause,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct JobState {
    pub id: ObjectId,
    pub task_ref: ObjectId,
    pub operation_ref: ModuleRef,
    pub execution_ref: JobExecutionRef,
    pub input_refs: Vec<ObjectId>,
    pub job_resources: Vec<ObjectId>,
    #[ts(optional = nullable)]
    pub phase_ref: Option<PhaseId>,
    pub phase_resources: Vec<ObjectId>,
    pub effective_progress_ms: DurationMs,
    #[ts(optional = nullable)]
    pub started_at_ms: Option<SimTimeMs>,
    #[ts(optional = nullable)]
    pub finished_at_ms: Option<SimTimeMs>,
    pub state: JobStatus,
    pub pending_outputs: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TransportState {
    pub id: ObjectId,
    pub supply_ref: ObjectId,
    pub cargo: Vec<ObjectId>,
    pub carrier_id: ObjectId,
    pub phase: TransportPhase,
    #[ts(optional = nullable)]
    pub departed_ms: Option<SimTimeMs>,
    #[ts(optional = nullable)]
    pub arrived_ms: Option<SimTimeMs>,
    #[ts(optional = nullable)]
    pub received_ms: Option<SimTimeMs>,
    #[ts(optional = nullable)]
    pub returned_ms: Option<SimTimeMs>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct QualityDecision {
    pub id: ObjectId,
    pub item_ref: ObjectId,
    pub inspection_ref: ModuleRef,
    pub source_kind: QualitySourceKind,
    #[ts(optional = nullable)]
    pub observed_result: Option<TypedValue>,
    pub disposition: QualityDisposition,
    pub effective_at_ms: SimTimeMs,
    pub decision_source: DecisionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct WaitInterval {
    pub object_ref: ObjectRef,
    #[ts(optional = nullable)]
    pub execution_ref: Option<JobExecutionRef>,
    #[ts(optional = nullable)]
    pub phase_ref: Option<PhaseId>,
    pub interval: IntervalMs,
    pub reasons: Vec<WaitReason>,
    pub end_cause: WaitEndCause,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum WaitEndCause {
    ConditionChanged,
    Resolved,
    ObservationEnd,
    ComputedBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct WaitReason {
    pub kind: WaitReasonKind,
    #[ts(optional = nullable)]
    pub blocking_object: Option<ObjectRef>,
    pub required: String,
    #[ts(optional = nullable)]
    pub next_known_boundary_ms: Option<SimTimeMs>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceEvent {
    #[ts(optional = nullable)]
    pub order_id: Option<ObjectId>,
    pub item_refs: Vec<ObjectId>,
    pub quantity: Quantity,
    pub destination: ObjectId,
    pub acceptance_stage: AcceptanceStage,
    pub at_ms: SimTimeMs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SimulationEvent {
    pub sequence: SafeUint,
    pub at_ms: SimTimeMs,
    pub subject: ObjectRef,
    pub payload: SimulationEventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct OrderOutcome {
    pub order_id: ObjectId,
    pub demand: Quantity,
    pub accepted: Quantity,
    pub outstanding: Quantity,
    #[ts(optional = nullable)]
    pub completion_ms: Option<SimTimeMs>,
    #[ts(optional = nullable)]
    pub due_at_ms: Option<SimTimeMs>,
    #[ts(optional = nullable)]
    pub tardiness_ms: Option<DurationMs>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunInputRef {
    pub project_id: ProjectId,
    pub project_revision: Revision,
    pub scenario_id: ScenarioId,
    pub scenario_revision: Revision,
    pub frozen_project_file: FileRef,
    pub frozen_scenario_file: FileRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub evidence_origin: EvidenceOrigin,
    pub id: RunId,
    pub input_ref: RunInputRef,
    pub scope: Vec<ObjectRef>,
    pub window: WindowMs,
    #[ts(optional = nullable)]
    pub computed_until_ms: Option<SimTimeMs>,
    pub computation_status: ComputationStatus,
    pub whole_window_computed: bool,
    #[ts(optional = nullable)]
    pub goal_met: Option<bool>,
    pub final_items: Vec<RuntimeItem>,
    pub jobs: Vec<JobState>,
    pub phase_work_intervals: Vec<PhaseWorkInterval>,
    pub transport: Vec<TransportState>,
    pub resource_claims: Vec<ResourceClaim>,
    pub waits: Vec<WaitInterval>,
    pub allocations: Vec<MaterialAllocation>,
    pub events: Vec<SimulationEvent>,
    pub acceptance_events: Vec<AcceptanceEvent>,
    pub orders: Vec<OrderOutcome>,
    pub limitations: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskProgress {
    pub phase: String,
    pub processed: SafeUint,
    #[ts(optional = nullable)]
    pub total: Option<SafeUint>,
    #[ts(optional = nullable)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskHandle {
    pub id: TaskId,
    pub scope: TaskScope,
    pub kind: LongTaskKind,
    pub state: ComputationStatus,
    pub input_revision: Revision,
    #[ts(optional = nullable)]
    pub progress: Option<TaskProgress>,
    #[ts(optional = nullable)]
    pub result_ref: Option<FileRef>,
    #[ts(optional = nullable)]
    pub failure: Option<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct HostContextIdentity {
    pub session_id: ObjectId,
    #[ts(optional = nullable)]
    pub active_project_id: Option<ProjectId>,
    #[ts(optional = nullable)]
    pub active_project_revision: Option<Revision>,
    #[ts(optional = nullable)]
    pub active_run_id: Option<RunId>,
    pub selected_scope: Option<EditScope>,
}

// Generation, reports and finite execution/geometry auxiliaries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SiteEnvelope {
    pub regions: Vec<LayoutRegion>,
    #[ts(optional = nullable)]
    pub length_m: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub width_m: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub height_m: Option<FiniteNumber>,
    pub usable_polygon_m: Vec<Vec2>,
    #[ts(optional = nullable)]
    pub clearance_m: Option<FiniteNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceScope {
    pub product_ref: SubjectRevisionRef,
    pub unit: UnitId,
    pub destination: ObjectId,
    pub stage: AcceptanceStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FactoryGenerationGoal {
    pub industry: String,
    pub product_family: String,
    #[ts(optional = nullable)]
    pub product_specification_ref: Option<SubjectRevisionRef>,
    #[ts(optional = nullable)]
    pub accepted_quantity_per_day: Option<Quantity>,
    pub acceptance_scope: AcceptanceScope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<_>")]
    pub additional_acceptance_scopes: Vec<AcceptanceScope>,
    #[ts(optional = nullable)]
    pub production_calendar_ref: Option<ObjectId>,
    pub observation_window: WindowMs,
    pub site_envelope: SiteEnvelope,
    pub existing_equipment: Vec<ObjectRef>,
    pub constraints: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ParameterSource {
    pub target: ObjectRef,
    pub field_path: String,
    #[ts(optional = nullable)]
    pub value: Option<TypedValue>,
    pub basis: SourceBasis,
    #[ts(optional = nullable)]
    pub source_ref: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateGenerationProfile {
    pub industry: String,
    pub product_family: String,
    pub site_applicability: SiteEnvelope,
    pub process_graph: Vec<RouteStep>,
    pub nine_category_roles: Vec<RoleQuantity>,
    pub required_parameters: Vec<ParameterDeclaration>,
    pub parameter_sources: Vec<ParameterSource>,
    pub layout_modes: Vec<String>,
    pub role_bindings: RoleBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FactoryDraftCheckResult {
    pub project_revision: Revision,
    pub scope: Vec<ObjectRef>,
    pub checks: Vec<DraftCheck>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DraftCheck {
    pub kind: DraftCheckKind,
    pub outcome: CheckOutcome,
    pub objects: Vec<ObjectRef>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct GenerationContext {
    pub goal: FactoryGenerationGoal,
    #[ts(optional = nullable)]
    pub template_ref: Option<ModuleRef>,
    pub parameter_sources: Vec<ParameterSource>,
    #[ts(optional = nullable)]
    pub draft_check: Option<FactoryDraftCheckResult>,
    #[ts(optional = nullable)]
    pub initial_run_id: Option<RunId>,
    pub revised_run_ids: Vec<RunId>,
    pub changes: Vec<ParameterOverride>,
    #[ts(optional = nullable)]
    pub stop_reason: Option<GenerationStopReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceMetric {
    pub resource_id: ObjectId,
    pub any_busy_ms: DurationMs,
    pub occupied_capacity_ms: SafeUint,
    #[ts(optional = nullable)]
    pub available_capacity_ms: Option<SafeUint>,
    #[ts(optional = nullable)]
    pub utilization: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub effective_work_ms: Option<DurationMs>,
    #[ts(optional = nullable)]
    pub waiting_ms: Option<DurationMs>,
    pub capacity_unit: UnitId,
    #[ts(optional = nullable)]
    pub open_calendar_occupied_capacity_ms: Option<SafeUint>,
    #[ts(optional = nullable)]
    pub closed_calendar_held_capacity_ms: Option<SafeUint>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ReportImage {
    pub title: String,
    pub scene_ref: RenderedSceneRef,
    pub evidence_kind: EvidenceKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct EvaluationReport {
    pub id: ObjectId,
    pub mode: ReportMode,
    pub project_id: ProjectId,
    pub project_revision: Revision,
    pub scenario_refs: Vec<ScenarioRevisionRef>,
    pub run_refs: Vec<RunId>,
    #[ts(optional = nullable)]
    pub time_reference: Option<TimeReference>,
    #[ts(optional = nullable)]
    pub observation_window: Option<WindowMs>,
    pub acceptance_scope: Vec<AcceptanceScope>,
    pub question_and_scope: String,
    pub input_assumptions: Vec<ParameterSource>,
    pub module_versions: Vec<ModuleRef>,
    pub layout_and_schedule: Vec<ReportImage>,
    #[ts(optional = nullable)]
    pub scenario_comparison: Option<RunComparison>,
    pub conclusions: Vec<String>,
    pub missing_conditions: Vec<Issue>,
    pub supported_scope: Vec<String>,
    pub limitations: Vec<Issue>,
    #[ts(optional = nullable)]
    pub editable_project_ref: Option<FileRef>,
    pub asset_rights: Vec<AssetRights>,
    #[ts(optional = nullable)]
    pub generation_context: Option<GenerationContext>,
    pub run_evaluations: Vec<RunEvaluation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialPort {
    pub item_role: RoleId,
    pub unit: UnitId,
    #[ts(optional = nullable)]
    pub quantity_rule: Option<Quantity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SignalPort {
    pub value_type: ValueKind,
    pub enum_values: Vec<String>,
    #[ts(optional = nullable)]
    pub effect_target: Option<RoleId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPort {
    pub artifact_type: String,
    pub schema_version: SchemaVersion,
    pub unit: UnitId,
    pub coordinate_frame: CoordinateFrame,
    pub supported_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialConnection {
    pub id: ObjectId,
    pub source: MaterialEndpoint,
    pub destination: MaterialEndpoint,
    pub item_type: ObjectId,
    pub unit: UnitId,
    #[ts(optional = nullable)]
    pub quantity_rule: Option<Quantity>,
    pub destination_store: ObjectId,
    #[ts(optional = nullable)]
    pub capacity_rule: Option<Quantity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SignalConnection {
    pub id: ObjectId,
    pub source: PortRef,
    pub destination: PortRef,
    pub value_type: ValueKind,
    #[ts(optional = nullable)]
    pub effect_target: Option<ObjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ArtifactConnection {
    pub id: ObjectId,
    pub source: PortRef,
    pub destination: PortRef,
    pub binding_ref: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CylinderGeometry {
    pub radius: FiniteNumber,
    pub height: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ImportedNodeGeometry {
    pub asset_ref: AssetRef,
    pub node_index: SafeUint,
    pub unsupported_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct GifSegment {
    pub asset_ref: AssetRef,
    pub start_ms: DurationMs,
    pub end_ms: DurationMs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ModelAnimationRef {
    pub asset_ref: AssetRef,
    pub animation_index: SafeUint,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DataTagBinding {
    pub source_field: String,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    #[ts(optional = nullable)]
    pub port_ref: Option<PortRef>,
    pub value_type: ValueKind,
    #[ts(optional = nullable)]
    pub unit: Option<UnitId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FoldMotion {
    pub fold_band: ObjectId,
    pub from_angle_rad: FiniteNumber,
    pub to_angle_rad: FiniteNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TranslationMotion {
    pub part_id: ObjectId,
    pub start_pose: Pose,
    pub displacement_mm: Vec3,
}

/// Hold the named part at its matching current root pose; never teleport it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FixedPoseMotion {
    pub part_id: ObjectId,
    pub pose: Pose,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TransferEvent {
    pub item_ref: ObjectId,
    pub from_holder: HolderRef,
    pub to_holder: HolderRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ChangeoverRule {
    #[ts(optional = nullable)]
    pub from_family: Option<String>,
    pub to_family: String,
    pub duration: DurationValue,
    pub resources: Vec<ResourceRequirement>,
    pub preparation_mode: PreparationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceSetupPolicy {
    pub device_id: ObjectId,
    #[ts(optional = nullable)]
    pub initial_family: Option<String>,
    pub changeovers: Vec<ChangeoverRule>,
    #[ts(optional = nullable)]
    pub first_article: Option<FirstArticleRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct FirstArticleRequirement {
    pub route_ref: SubjectRevisionRef,
    pub family: String,
    pub inspection_module: ModuleRef,
    pub gate_id: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct InspectionPreset {
    pub id: ObjectId,
    pub item_or_ancestor: ObjectId,
    pub match_kind: QualityMatchKind,
    pub inspection_module: ModuleRef,
    pub results: Vec<Option<TypedValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequirement {
    pub role: RoleId,
    pub quantity: Quantity,
    pub qualifications: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct StructuralVariable {
    pub id: ObjectId,
    pub field_path: String,
    pub minimum: FiniteNumber,
    pub maximum: FiniteNumber,
    pub unit: UnitId,
    pub integer: bool,
    pub basis: SourceBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DimensionConstraint {
    pub id: ObjectId,
    pub variable_id: ObjectId,
    pub relation: ComparisonOperator,
    pub target: FiniteNumber,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DecimalAmount {
    pub coefficient: SafeUint,
    pub decimal_places: DecimalPlaces,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityPurpose {
    Execution,
    Preview,
    Adoption,
    Redistribution,
    CompleteConclusion,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PreparationMode {
    Internal,
    External,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum QualityMatchKind {
    ExactItem,
    Ancestor,
    Batch,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Equal,
    LessOrEqual,
    GreaterOrEqual,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "u8", into = "u8")]
#[schemars(!try_from, !into)]
pub struct DecimalPlaces(#[schemars(range(min = 0, max = 9))] u8);
impl TryFrom<u8> for DecimalPlaces {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 9 {
            Ok(Self(value))
        } else {
            Err("decimal places must be0..9".into())
        }
    }
}
impl From<DecimalPlaces> for u8 {
    fn from(value: DecimalPlaces) -> Self {
        value.0
    }
}
impl DecimalPlaces {
    pub fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TypedValue {
    Number(FiniteNumber),
    Integer(SafeUint),
    Boolean(bool),
    String(String),
    Enum(String),
    Vector3(Vec3),
    Duration(DurationValue),
    Quantity(Quantity),
    ItemType(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HolderRef {
    Location(ObjectId),
    Job(ObjectId),
    Transport(ObjectId),
    Container(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "category",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ModulePayload {
    Items(ItemDefinition),
    Devices(DeviceDefinition),
    Resources(ResourceDefinition),
    Tasks(TaskDefinition),
    Operations(OperationDefinition),
    Transport(TransportDefinition),
    Templates(TemplateDefinition),
    Professional(ProfessionalDefinition),
    Media(Presentation),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PortPayload {
    Material(MaterialPort),
    Signal(SignalPort),
    Artifact(ArtifactPort),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Connection {
    Material(MaterialConnection),
    Signal(SignalConnection),
    Artifact(ArtifactConnection),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MediaLocator {
    Asset(AssetRef),
    MediaRef(ModuleRef),
    ImageSequence(Vec<AssetRef>),
    NativeScene(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SceneGeometry {
    Box(Vec3),
    Cylinder(CylinderGeometry),
    Plane(Vec2),
    ImportedNode(ImportedNodeGeometry),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ClipSource {
    ImageAsset(AssetRef),
    GifSegment(GifSegment),
    ModelAnimation(ModelAnimationRef),
    NativePose(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AnchorRef {
    ImageNormalized(Vec2),
    SceneNode(SafeUint),
    Viewport(Vec2),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TagPayload {
    Data(DataTagBinding),
    Parameter(ObjectId),
    Action(DeviceAction),
    Annotation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PackagingMotion {
    Fold(FoldMotion),
    RigidTranslation(TranslationMotion),
    RigidRotation(RotationMotion),
    FixedPose(FixedPoseMotion),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DecisionSource {
    HumanStatement(String),
    ScheduledPreset(ObjectId),
    ProviderResult(ArtifactId),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EditScope {
    Project(ProjectId),
    DefinitionDraft(DefinitionScopeIdentity),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SimulationEventPayload {
    JobStarted(ObjectId),
    PhaseCompleted(PhaseId),
    ItemTransferred(TransferEvent),
    ArrivalAccepted(ObjectId),
    QualityChanged(QualityDecision),
    ResourceReleased(ObjectId),
    OrderAccepted(AcceptanceEvent),
    TransportChanged(TransportState),
    WaitChanged(WaitInterval),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DefinitionScopeIdentity {
    pub id: ModuleId,
    pub revision: Revision,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Versioned<T> {
    pub schema_version: SchemaVersion,
    pub data: T,
}
/// Generation reachability anchor only; never a project or package on disk.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct ContractTypeBundle {
    pub measurement_groups: Vec<MeasurementGroupRecord>,
    pub manifest: Versioned<PackageManifest>,
    pub project: Versioned<ProjectState>,
    pub scenario: Versioned<ScenarioInput>,
    pub module_package: Versioned<ModulePackage>,
    pub template: Versioned<TemplateSubset>,
    pub generation_profile: Versioned<TemplateGenerationProfile>,
    pub packaging_job: Versioned<PackageStructureJob>,
    pub packaging_result: Versioned<PackagingResult>,
    pub run_result: Versioned<RunResult>,
    pub report: Versioned<EvaluationReport>,
    pub definition_draft: DefinitionDraft,
    pub task_handle: TaskHandle,
    pub provider: ProviderCapability,
    pub host_context: HostContextIdentity,
    pub artifacts: Vec<ArtifactRecord>,
}

string_id!(SubjectRevisionId);
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRevisionRef {
    pub id: ScenarioId,
    pub revision: Revision,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct JobInputRevisionRef {
    pub id: ObjectId,
    pub revision: Revision,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct NodePoseTransform {
    pub node_index: SafeUint,
    pub pose: Pose,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LayoutRegionKind {
    Usable,
    KeepOut,
    StaticLane,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct LayoutRegion {
    pub id: ObjectId,
    pub polygon_m: Vec<Vec2>,
    pub kind: LayoutRegionKind,
    pub clearance_m: Option<FiniteNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    ActualSoftware,
    DesignArtifact,
    ManualExpected,
    TypeFixture,
    ImportedClaim,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RenderViewKind {
    LineCanvas,
    MultiHallCanvas,
    DeviceDetail,
    ModulePreview,
    PackagingWorkbench,
    PackagingManufacturing,
    DeviceCyclePreview,
    TimelineSnapshot,
    Comparison,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct BudgetUsage {
    pub candidates_evaluated: SafeUint,
    pub interval_queries: SafeUint,
    pub subdivisions: SafeUint,
    pub stopped_by: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RotationMotion {
    pub part_id: ObjectId,
    pub pivot_mm: Vec3,
    pub axis: Vec3,
    pub from_angle_rad: FiniteNumber,
    pub to_angle_rad: FiniteNumber,
}

// Test evidence records required by S03-04, not product audit logs or APIs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementStatus {
    NotExecuted,
    NotEvaluable,
    Pass,
    Fail,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementClock {
    Renderer,
    LocalExternalClient,
    NativeStage,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MeasurementMarker {
    pub label: String,
    pub time_ms: FiniteNumber,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MeasurementEvidence {
    pub file_ref: Option<FileRef>,
    pub missing_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MeasurementEnvironment {
    pub valid: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MeasurementInput {
    pub project_ref: Option<ProjectId>,
    pub revision: Option<Revision>,
    pub run_ref: Option<RunId>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MeasurementAttemptRecord {
    pub attempt_index: SafeUint,
    pub target_ref: Option<ObjectRef>,
    pub expected_outcome: String,
    pub input_reference: MeasurementInput,
    pub start_marker: Option<MeasurementMarker>,
    pub end_marker: Option<MeasurementMarker>,
    pub elapsed_ms: Option<FiniteNumber>,
    pub actual_outcome: Option<String>,
    pub evidence: MeasurementEvidence,
    pub attempt_status: MeasurementStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MeasurementGroupRecord {
    pub candidate_reference: String,
    pub machine_reference: String,
    pub nfr_id: String,
    pub subcheck_id: String,
    pub load_variant: Option<String>,
    pub condition_or_action: String,
    pub round: SafeUint,
    pub frozen_input_and_asset_references: Vec<FileRef>,
    pub planned_attempt_count: Option<SafeUint>,
    pub observation_window_ms: Option<DurationMs>,
    pub clock_source: MeasurementClock,
    pub environment: MeasurementEnvironment,
    pub group_status: MeasurementStatus,
    pub attempts: Vec<MeasurementAttemptRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TaskScope {
    Workspace(ObjectId),
    Edit(EditScope),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingPanelPose {
    pub panel_id: ObjectId,
    pub pose: Pose,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingProcessSample {
    pub step_id: ObjectId,
    pub progress: FiniteNumber,
    pub panel_poses: Vec<PackagingPanelPose>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum MetricKey {
    AcceptedQuantity,
    CompletionTime,
    WaitingTime,
    ResourceUtilization,
    UnitCost,
    ReworkQuantity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MetricValue {
    Quantity(Quantity),
    Duration(DurationMs),
    Scalar {
        value: FiniteNumber,
        unit: UnitId,
    },
    MoneyPerUnit {
        amount: FiniteNumber,
        currency: String,
        unit: UnitId,
    },
    Unknown(Issue),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunMetric {
    pub run_id: RunId,
    pub metric: MetricKey,
    pub value: MetricValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum InputChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Strict,
    ControlledChanges,
    SideBySideOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonConditionKey {
    TimeReference,
    ObservationWindow,
    AcceptanceScope,
    Demand,
    SelectedScope,
    InitialConditions,
    SupportedCapabilities,
    ExplainedChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InputChangeValue {
    Null,
    Scalar {
        value: TypedValue,
        #[ts(optional = nullable)]
        unit: Option<UnitId>,
    },
    InitialItem(InitialItem),
    Arrival(ArrivalPlan),
    Order(OrderLine),
    Task(ProductionTask),
    Supply(SupplyRelation),
    ModuleRef(ModuleRef),
    ItemType(ItemType),
    Location(StockLocation),
    Hall(HallDesign),
    Line(LineGroup),
    Device(DeviceInstance),
    Resource(ResourceInstance),
    Calendar(Calendar),
    Product(ProductRevision),
    Bom(BomDefinition),
    Route(RouteDefinition),
    Connection(Connection),
    Binding(ConsumerBinding),
    Presentation(Presentation),
    QualityDecision(QualityDecision),
    CostInput(CostInput),
    Window(WindowMs),
    SetupPolicy(DeviceSetupPolicy),
    ParameterOverride(ParameterOverride),
    SourceRecord(SourceRecord),
    Definition(Box<ModuleDefinition>),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NumericDifference {
    Quantity {
        negative: bool,
        magnitude: Quantity,
    },
    Duration {
        negative: bool,
        magnitude: DurationMs,
    },
    Scalar {
        value: FiniteNumber,
        unit: UnitId,
    },
    MoneyPerUnit {
        amount: FiniteNumber,
        currency: String,
        unit: UnitId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct InputChange {
    pub target: ObjectRef,
    pub path: String,
    pub kind: InputChangeKind,
    #[ts(optional = nullable)]
    pub before: Option<InputChangeValue>,
    #[ts(optional = nullable)]
    pub after: Option<InputChangeValue>,
    #[ts(optional = nullable)]
    pub basis: Option<SourceBasis>,
    #[ts(optional = nullable)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ComparisonCondition {
    pub key: ComparisonConditionKey,
    pub matches: bool,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunPairComparison {
    pub baseline_run_id: RunId,
    pub candidate_run_id: RunId,
    pub status: ComparisonStatus,
    pub conditions: Vec<ComparisonCondition>,
    pub input_changes: Vec<InputChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MetricPairDifference {
    pub baseline_run_id: RunId,
    pub candidate_run_id: RunId,
    pub comparable: bool,
    #[ts(optional = nullable)]
    pub difference: Option<NumericDifference>,
    #[ts(optional = nullable)]
    pub change_percent: Option<FiniteNumber>,
    pub conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MetricComparison {
    pub metric: MetricKey,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    #[ts(optional = nullable)]
    pub acceptance_scope: Option<AcceptanceScope>,
    #[ts(optional = nullable)]
    pub currency: Option<String>,
    pub values: Vec<RunMetric>,
    pub pair_differences: Vec<MetricPairDifference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunComparison {
    pub run_ids: Vec<RunId>,
    pub pairs: Vec<RunPairComparison>,
    pub metrics: Vec<MetricComparison>,
    pub conclusions: Vec<String>,
    pub limitations: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CostInput {
    pub id: ObjectId,
    pub object_ref: ObjectRef,
    pub component: String,
    pub basis: CostBasis,
    #[ts(optional = nullable)]
    pub rate: Option<DecimalAmount>,
    #[ts(optional = nullable)]
    pub currency: Option<String>,
    pub rate_quantity: Quantity,
    pub includes: Vec<String>,
    pub source: ParameterSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ConditionalCost {
    pub cost_input_id: ObjectId,
    pub object_ref: ObjectRef,
    pub basis: CostBasis,
    #[ts(optional = nullable)]
    pub measured_quantity: Option<Quantity>,
    #[ts(optional = nullable)]
    pub amount: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub currency: Option<String>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CurrencyCostSummary {
    pub currency: String,
    pub cost_input_ids: Vec<ObjectId>,
    pub known_subtotal: FiniteNumber,
    #[ts(optional = nullable)]
    pub complete_total: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub accepted_scope: Option<AcceptanceScope>,
    #[ts(optional = nullable)]
    pub accepted_quantity: Option<Quantity>,
    #[ts(optional = nullable)]
    pub unit_cost: Option<FiniteNumber>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CostEvaluation {
    pub lines: Vec<ConditionalCost>,
    pub currencies: Vec<CurrencyCostSummary>,
    pub unassigned_cost_input_ids: Vec<ObjectId>,
    pub inputs: Vec<CostInput>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunEvaluation {
    pub run_id: RunId,
    pub input_ref: ReportRunInputRef,
    pub evidence_origin: EvidenceOrigin,
    pub scope: Vec<ObjectRef>,
    pub time_reference: TimeReference,
    pub requested_window: WindowMs,
    #[ts(optional = nullable)]
    pub computed_until_ms: Option<SimTimeMs>,
    pub computation_status: ComputationStatus,
    pub whole_window_computed: bool,
    #[ts(optional = nullable)]
    pub goal_met: Option<bool>,
    pub acceptance_scope: Vec<AcceptanceScope>,
    pub order_metrics: Vec<OrderOutcome>,
    pub resource_metrics: Vec<ResourceMetric>,
    pub costs: CostEvaluation,
    pub events: Vec<SimulationEvent>,
    pub acceptance_events: Vec<AcceptanceEvent>,
    pub resource_claims: Vec<ResourceClaim>,
    pub waits: Vec<WaitInterval>,
    pub phase_work_intervals: Vec<PhaseWorkInterval>,
    pub limitations: Vec<Issue>,
    pub module_versions: Vec<ModuleRef>,
    pub input_assumptions: Vec<ParameterSource>,
    pub jobs: Vec<JobState>,
    pub transport: Vec<TransportState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ReportRunInputRef {
    pub project_id: ProjectId,
    pub project_revision: Revision,
    pub scenario_id: ScenarioId,
    pub scenario_revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MaterialEndpoint {
    Port(PortRef),
    Location(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateAdoption {
    pub id: ObjectId,
    pub template_ref: ModuleRef,
    pub source_archive: FileRef,
    pub resource_choices: Vec<TemplateResourceChoice>,
    pub preparation_bindings: Vec<TemplatePreparationBinding>,
    pub imported_claims: Vec<ImportedProfessionalClaim>,
    pub inspection_presets: Vec<TemplateInspectionPreset>,
    #[ts(optional = nullable)]
    pub deferred_seed: Option<DeferredTemplateSeed>,
    pub missing_inputs: Vec<TemplateMissingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateResourceChoice {
    pub source_id: ObjectId,
    pub requirement_index: SafeUint,
    #[ts(optional = nullable)]
    pub resource_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplatePreparationBinding {
    pub task_ref: ModuleRef,
    pub target_device_id: ObjectId,
    pub before_device_ids: Vec<ObjectId>,
    // Values name source_id in resource_choices, never implicit current Project resources.
    pub worker_roles: BTreeMap<RoleId, ObjectId>,
    pub route_refs: Vec<SubjectRevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ImportedProfessionalClaim {
    pub id: ObjectId,
    pub artifact_type: String,
    pub mode: ProfessionalMode,
    pub consumer_device_ids: Vec<ObjectId>,
    // RFC6901 pointer into TemplateAdoption.source_archive, UI/input preservation only.
    pub source_pointer: String,
    pub declared_scope: Vec<String>,
    pub required_fields: Vec<String>,
    pub missing_fields: Vec<String>,
    // Type is exclusively imported_claim: no actual_outcome/result/ack/accepted flags.
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateInspectionPreset {
    pub id: ObjectId,
    pub consumer_device_id: ObjectId,
    pub inspection_module: ModuleRef,
    #[ts(optional = nullable)]
    pub item_or_ancestor: Option<ObjectId>,
    #[ts(optional = nullable)]
    pub match_kind: Option<QualityMatchKind>,
    pub results: Vec<Option<TypedValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeferredTemplateSeed {
    pub time_reference: TimeReference,
    pub window: WindowMs,
    pub initial_stock: Vec<TemplateStockSeed>,
    pub demand: TemplateDemandSeed,
    #[ts(optional = nullable)]
    pub product_ref: Option<SubjectRevisionRef>,
    #[ts(optional = nullable)]
    pub bom_ref: Option<SubjectRevisionRef>,
    #[ts(optional = nullable)]
    pub routing_ref: Option<SubjectRevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateStockSeed {
    pub location_id: ObjectId,
    pub item_type_id: ObjectId,
    pub quantity: Quantity,
    #[ts(optional = nullable)]
    pub quality: Option<QualityState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateDemandSeed {
    pub item_type_id: ObjectId,
    pub quantity: Quantity,
    pub destination_location_id: ObjectId,
    pub acceptance_stage: AcceptanceStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateMissingInput {
    pub kind: TemplateMissingInputKind,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    pub source_pointer: String,
    pub required_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TemplateMissingInputKind {
    SiteGeometry,
    Specification,
    ProfessionalInput,
    ProductBomRoute,
    InitialQuality,
}
