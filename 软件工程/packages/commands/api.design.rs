//! S06-03 canonical API contract. No product dispatcher/Host/Engine implementation.
//! The product commands crate exposes its domain dependency as crate::contract.
use crate::contract::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// The request header always contains project_id; Workspace explicitly sends null.
fn present_nullable<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}
fn nullable_project_id_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ProjectId>>()
}

macro_rules! model {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub enum ApiVersion {
    #[serde(rename = "0.1")]
    V01,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum OperationName {
    DescribeCapabilities,
    GetContext,
    GetObject,
    ApplyCommands,
    RenderView,
    RunScenario,
    GetRun,
    CompareRuns,
    ComputePackaging,
    GetTask,
    CancelTask,
    AdoptArtifact,
    ImportPackage,
    SaveProject,
    ExportPackage,
    ExportEvaluation,
    QueryModuleLibrary,
    CheckFactoryDraft,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApiScope {
    Workspace,
    Project,
    DefinitionDraft(ModuleId),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Request<O, P> {
    pub api_version: ApiVersion,
    pub request_id: ObjectId,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_project_id_schema")]
    pub project_id: Option<ProjectId>,
    pub scope: ApiScope,
    pub operation: O,
    pub params: P,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub enum ErrorCode {
    NoActiveProject,
    UnsupportedCapability,
    InvalidInput,
    MissingReference,
    RevisionConflict,
    PermissionDenied,
    ComputationFailed,
    Cancelled,
    UnsupportedSchema,
    IoFailure,
}
model!(ApiFieldError {
    pointer: String,
    reason: String
});
model!(ApiError {
    code: ErrorCode, message: String, object_refs: Vec<ObjectRef>,
    field_errors: Vec<ApiFieldError>, expected_revision: Option<Revision>,
    actual_revision: Option<Revision>,
});
model!(ResponseContext {
    host_instance_id: ObjectId, session_id: ObjectId,
    project_id: Option<ProjectId>, project_revision: Option<Revision>,
    scope: Option<EditScope>, scope_revision: Option<Revision>,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "status",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApiOutcome<R> {
    Ok(R),
    Error(ApiError),
    // Transport wrapper only; CommandsService continues to return its typed R.
    Managed(ManagedJsonResultRef),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Response<R> {
    pub api_version: ApiVersion,
    pub request_id: Option<ObjectId>,
    pub context: Option<ResponseContext>,
    pub outcome: ApiOutcome<R>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ApiCategory {
    Items,
    Devices,
    Resources,
    Tasks,
    Operations,
    Transport,
    Templates,
    Professional,
    Media,
}
model!(OperationCapability {
    operation: OperationName, input_schema_ref: String, output_schema_ref: String,
    allowed_scopes: Vec<ApiScope>, available_controls: Vec<TaskControlAction>,
});
model!(CapabilityCatalog {
    api_version: ApiVersion, implemented_operations: Vec<OperationCapability>,
    available_command_kinds: Vec<String>, providers: Vec<ProviderCapability>,
    category_ids: Vec<ApiCategory>, media_kinds: Vec<PresentationKind>,
    native_primitives: Vec<String>, max_ipc_frame_bytes: SafeUint,
    limitations: Vec<Issue>,
});
model!(ProjectSummary {
    id: ProjectId,
    revision: Revision,
    title: String,
    dirty: bool
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSummary {
    pub draft_id: ModuleId,
    pub revision: Revision,
    pub definition_ref: ModuleRef,
    pub issues: Vec<Issue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub history: Option<HistoryAvailability>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectContext {
    pub host: HostContextIdentity,
    pub project: Option<ProjectSummary>,
    pub selected_objects: Vec<ObjectRef>,
    pub view_state: Option<ViewState>,
    pub definition: Option<DefinitionSummary>,
    pub tasks: Vec<TaskHandle>,
    pub authorized_operations: Vec<OperationName>,
    pub issues: Vec<Issue>,
    // Existing empty-context wire shape remains unchanged. These summaries come
    // only from actual stored Runs visible in this authorized Project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_summaries: Vec<RunSummary>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ObjectSource {
    Draft,
    RunSnapshot {
        run_id: RunId,
        at_ms: SimTimeMs,
    },
    ModuleVersion {
        module_ref: ModuleRef,
    },
    RunModuleVersion {
        run_id: RunId,
        at_ms: SimTimeMs,
        module_ref: ModuleRef,
    },
    TemplatePreview {
        module_ref: ModuleRef,
        base_revision: Revision,
        choices: Option<TemplatePreparationChoices>,
        seed: Option<TemplateSeedSelection>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DomainObject {
    PackagingJob(Box<PackageStructureJob>),
    Project(Box<ProjectState>),
    Scenario(Box<ScenarioInput>),
    Module(Box<ModulePackage>),
    TemplatePreview(Box<TemplatePreview>),
    Definition(Box<ModuleDefinition>),
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
    Item(RuntimeItem),
    Job(JobState),
    Transport(TransportState),
    Artifact(ArtifactRecord),
    Report(Box<EvaluationReport>),
    NativeScene(Presentation),
    SceneNode(SceneNode),
    Order(OrderLine),
    Task(ProductionTask),
    Supply(SupplyRelation),
}
model!(HistoryAvailability { can_undo: bool, can_redo: bool, boundary_reason: Option<String> });
model!(EditCommit {
    project_id: ProjectId, scope: EditScope, previous_revision: Option<Revision>,
    new_revision: Revision, changed_objects: Vec<ObjectRef>, issues: Vec<Issue>,
    history: HistoryAvailability,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EditableProjectObject {
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
    Scenario(Box<ScenarioInput>),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectSettings {
    pub title: String,
    pub time_reference: TimeReference,
    pub generation_context: Option<GenerationContext>,
    pub sources: Vec<SourceRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub setup_policies: Option<Vec<DeviceSetupPolicy>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DepartureDecision {
    Stay,
    DiscardAfterTasksTerminal,
    AlreadySaved,
}
model!(NewProject {
    title: String,
    time_reference: TimeReference,
    departure: DepartureDecision,
});
model!(ModuleAdoption {
    module_ref: ModuleRef, targets: Vec<ObjectRef>, bindings: RoleBindings,
    new_instances: Vec<EditableProjectObject>, explicit_seed: Option<ScenarioInput>,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateComposition {
    pub project_revision: Revision,
    pub definition: ModuleDefinition,
    pub subset: TemplateSubset,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub source_scenario: Option<ScenarioReference>,
}
model!(TemplatePreparationChoices {
    source_project: RelativeFilePath, namespace: String,
    // A present null chooses a new declared example resource; a present ID
    // chooses that actual resource. An absent key leaves the role unresolved.
    resources: std::collections::BTreeMap<ObjectId, Option<ObjectId>>,
    reuse_items: std::collections::BTreeMap<ObjectId, ObjectId>,
    include_example_product: bool, site_envelope: Option<SiteEnvelope>,
});
model!(TemplateSeedSelection {
    scenario_id: ScenarioId, supply: bool, demand: bool,
    initial_quality: std::collections::BTreeMap<ObjectId, QualityState>,
    adopt_declared_inspection_results: bool,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplatePreview {
    pub module_ref: ModuleRef,
    pub project_id: ProjectId,
    pub base_revision: Revision,
    pub subset: TemplateSubset,
    pub source_project: RelativeFilePath,
    pub resources: Vec<ResourceInstance>,
    pub command: Option<TemplateInstantiation>,
    pub issues: Vec<Issue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub source_scenario: Option<ScenarioReference>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateInstantiation {
    pub template_ref: ModuleRef,
    pub subset: TemplateSubset,
    pub mapped_bindings: RoleBindings,
    pub proposed_objects: Vec<EditableProjectObject>,
    pub explicit_new_scenario_seed: Option<ScenarioInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub project_setup_policies: Option<Vec<DeviceSetupPolicy>>,
    // Host replays these choices against the selected source bytes; this is
    // never a caller-supplied adoption, source record, capability or approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub preparation: Option<TemplatePreparationChoices>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EditCommand {
    CreateProject(NewProject),
    PutObject(EditableProjectObject),
    RemoveObject(ObjectRef),
    SetProjectSettings(ProjectSettings),
    CreateDefinitionDraft(ModuleDefinition),
    PutDefinition(ModuleDefinition),
    PutTemplateComposition(TemplateComposition),
    SetDeclaredParameter {
        target: ObjectRef,
        parameter_id: ObjectId,
        value: TypedValue,
        basis: SourceBasis,
    },
    AdoptModule(ModuleAdoption),
    InstantiateTemplate(TemplateInstantiation),
    // Same canonical Job as compute_packaging, including explicit FixedPose part_id.
    // Scoped input validation checks references; saving never certifies geometry.
    PutPackagingJob(Box<PackageStructureJob>),
    RecordArtifactAcknowledgment(ArtifactAcknowledgment),
    AdoptImportedDraft {
        task_id: TaskId,
        choices: ImportAcceptanceChoices,
    },
    CommitDefinitionVersion {
        version: VersionId,
    },
    Undo,
    Redo,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RenderDelivery {
    CaptureOnly,
    ShowAndCapture,
}
model!(RenderPreviewState {
    phase_id: Option<PhaseId>, progress: Option<FiniteNumber>, pose_name: Option<String>,
});
model!(RenderOutput {
    scene: RenderedSceneRef, requested_scope: EditScope,
    requested_scope_revision: Revision, visible_objects: Vec<ObjectRef>, issues: Vec<Issue>,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RunIntent {
    ProjectScenario,
    DefinitionCycle {
        definition_revision: Revision,
        test_input: Box<ScenarioInput>,
    },
}
model!(RunSummary {
    run_id: RunId, input: RunInputRef, computation_status: ComputationStatus,
    window: WindowMs, computed_until_ms: Option<SimTimeMs>,
    whole_window_computed: bool, goal_met: Option<bool>,
    accepted: Option<Quantity>, issues: Vec<Issue>,
});
model!(PageRequest { after_index: Option<SafeUint>, limit: SafeUint });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RunQuery {
    Summary,
    Events {
        interval: Option<IntervalMs>,
        page: PageRequest,
    },
    ObjectsAt {
        at_ms: SimTimeMs,
        objects: Vec<ObjectRef>,
        page: PageRequest,
    },
    /// Canonical RunResult: frozen execution refs, phase work intervals and computed frontier.
    FullResult,
}
model!(RunEventsPage { run_id: RunId, events: Vec<SimulationEvent>, next_index: Option<SafeUint> });
model!(RunObjectsPage { run_id: RunId, at_ms: SimTimeMs, objects: Vec<DomainObject>, next_index: Option<SafeUint> });
// Ephemeral immutable bytes of the original complete Response<R>.
// response_schema_ref must match originating_operation in the published catalog.
model!(ManagedJsonResultRef {
    result_id: ObjectId,
    response_schema_ref: String,
    byte_length: SafeUint,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RunView {
    Summary(RunSummary),
    Events(RunEventsPage),
    Objects(RunObjectsPage),
    /// Same input identity/frontier as Summary; effective work is never reconstructed by UI.
    Full(Box<RunResult>),
}
model!(MetricScope { keys: Vec<MetricKey>, objects: Vec<ObjectRef>, acceptance: Vec<AcceptanceScope> });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TaskControlAction {
    Cancel,
    PauseCalculation,
    ResumeCalculation,
    StepNextTimestamp,
}
model!(CancellationState {
    task: TaskHandle, requested_action: TaskControlAction, action_accepted: bool,
    terminal_won: bool, issues: Vec<Issue>,
});
model!(BindingCommit { project_id: ProjectId, previous_revision: Revision, new_revision: Revision, binding: ConsumerBinding, issues: Vec<Issue> });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum FilePurpose {
    ImportPackage,
    SaveProject,
    ExportPackage,
    ExportEvaluation,
}
model!(SelectedFileRef {
    selection_id: ObjectId,
    purpose: FilePurpose
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImportPolicy {
    OpenProject,
    IntoProject,
    IntoDefinitionDraft,
    AddToLocalLibrary,
}
model!(ImportDraft {
    input_selection: SelectedFileRef, proposed_definitions: Vec<ModuleRef>,
    proposed_objects: Vec<ObjectRef>, missing: Vec<Issue>, conflicts: Vec<Issue>,
    required_human_choices: Vec<String>,
    proposed_data: Vec<DomainObject>,
});
model!(ImportAcceptanceChoices {
    chosen_policy: ImportPolicy, objects_to_create: Vec<ObjectRef>,
    module_versions: Vec<ModuleRef>, reused_bindings: RoleBindings,
    selected_assets: Vec<AssetId>, accept_missing_as_draft: bool,
    departure: DepartureDecision,
});
model!(SavedProject {
    project_id: ProjectId,
    saved_revision: Revision,
    current_revision: Revision,
    still_dirty: bool,
    destination: SelectedFileRef,
    manifest: FileRef,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PackagePurpose {
    PrivateProject,
    HandoffProject,
    PrivateModule,
    CommunityModule,
}
model!(PackageSelection {
    purpose: PackagePurpose, scenarios: Vec<ScenarioId>, run_ids: Vec<RunId>,
    artifact_ids: Vec<ArtifactId>, module_refs: Vec<ModuleRef>,
    include_redistributable_assets: bool,
});
model!(ExportPackage { destination: SelectedFileRef, package: FileRef, included_files: Vec<FileRef>, missing: Vec<Issue> });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationFormat {
    Pdf,
    Html,
    Csv,
    Json,
}
model!(ScenarioReference {
    scenario_id: ScenarioId,
    revision: Revision
});
model!(ExportEvaluation { destination: SelectedFileRef, context: EvaluationReport, files: Vec<FileRef>, limitations: Vec<Issue> });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TaskOutput {
    Render(RenderOutput),
    Run(RunSummary),
    Packaging(PackagingResult),
    Import(ImportDraft),
    Save(SavedProject),
    Package(ExportPackage),
    Evaluation(Box<ExportEvaluation>),
}
model!(TaskState {
    task: TaskHandle, originating_operation: OperationName,
    allowed_controls: Vec<TaskControlAction>, result: Option<TaskOutput>,
    failure: Option<ApiError>,
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySourceScope {
    Bundled,
    ProjectPackages,
    UserGrantedLibraries,
}
model!(ModuleCandidate {
    module_ref: ModuleRef, category: ApiCategory, title: String,
    applicable: bool, missing: Vec<Issue>, capabilities: Vec<String>,
    template_profile: Option<TemplateGenerationProfile>, sources: Vec<SourceRecord>,
});
model!(ModuleTemplateCandidates { candidates: Vec<ModuleCandidate>, missing: Vec<Issue> });
model!(FactoryCheckScope { objects: Vec<ObjectRef>, check_references: bool, check_business: bool, check_static_layout: bool });

model!(DescribeCapabilitiesParams {});
model!(GetContextParams { selection: Vec<ObjectRef> });
model!(GetObjectParams { object_id: ObjectRef, source: ObjectSource, node_index: Option<SafeUint> });
model!(ApplyCommandsParams { base_revision: Option<Revision>, commands: Vec<EditCommand> });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RenderViewParams {
    pub revision: Revision,
    pub view: RenderViewSpec,
    pub object_id: Option<ObjectRef>,
    pub preview: Option<RenderPreviewState>,
    pub delivery: RenderDelivery,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub module_ref: Option<ModuleRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub comparison: Option<CompareRunsParams>,
}
model!(RunScenarioParams {
    scenario_id: ScenarioId,
    input_revision: Revision,
    observation_window: WindowMs,
    intent: RunIntent
});
model!(GetRunParams {
    run_id: RunId,
    query: RunQuery
});
model!(CompareRunsParams { run_ids: Vec<RunId>, metric_scope: MetricScope });
// FixedPoseMotion holds the named part's matching current root pose. It does not
// reset placement or bypass collision / already-active connection constraints.
model!(ComputePackagingParams { packaging_job: Box<PackageStructureJob>, input_revision: Revision, computation_budget: CalculationBudget });
model!(GetTaskParams { task_id: TaskId });
model!(CancelTaskParams {
    task_id: TaskId,
    action: TaskControlAction
});
model!(AdoptArtifactParams { artifact_id: ArtifactId, acceptance_ref: ObjectId, consumer_ref: ObjectRef, operation_ref: ModuleRef, requested_scope: Vec<String>, base_revision: Revision });
model!(ImportPackageParams { selected_file_ref: SelectedFileRef, import_policy: ImportPolicy, base_revision: Option<Revision> });
model!(SaveProjectParams {
    revision: Revision,
    selected_directory_ref: SelectedFileRef
});
model!(ExportPackageParams {
    revision: Revision,
    package_scope: PackageSelection,
    selected_destination_ref: SelectedFileRef
});
model!(ExportEvaluationParams { revision: Revision, report_mode: ReportMode, scenario_refs: Vec<ScenarioReference>, run_refs: Vec<RunId>, metric_scope: MetricScope, formats: Vec<EvaluationFormat>, selected_destination_ref: SelectedFileRef });
model!(QueryModuleLibraryParams { category_filters: Vec<ApiCategory>, industry: Option<String>, product_family: Option<String>, site_envelope: Option<SiteEnvelope>, source_scope: Vec<LibrarySourceScope> });
model!(CheckFactoryDraftParams {
    revision: Revision,
    generation_goal: FactoryGenerationGoal,
    check_scope: FactoryCheckScope
});

macro_rules! operation_tag {
    ($name:ident, $wire:literal) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
        pub enum $name {
            #[serde(rename = $wire)]
            Call,
        }
    };
}
operation_tag!(DescribeCapabilitiesTag, "describe_capabilities");
operation_tag!(GetContextTag, "get_context");
operation_tag!(GetObjectTag, "get_object");
operation_tag!(ApplyCommandsTag, "apply_commands");
operation_tag!(RenderViewTag, "render_view");
operation_tag!(RunScenarioTag, "run_scenario");
operation_tag!(GetRunTag, "get_run");
operation_tag!(CompareRunsTag, "compare_runs");
operation_tag!(ComputePackagingTag, "compute_packaging");
operation_tag!(GetTaskTag, "get_task");
operation_tag!(CancelTaskTag, "cancel_task");
operation_tag!(AdoptArtifactTag, "adopt_artifact");
operation_tag!(ImportPackageTag, "import_package");
operation_tag!(SaveProjectTag, "save_project");
operation_tag!(ExportPackageTag, "export_package");
operation_tag!(ExportEvaluationTag, "export_evaluation");
operation_tag!(QueryModuleLibraryTag, "query_module_library");
operation_tag!(CheckFactoryDraftTag, "check_factory_draft");

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(untagged)]
pub enum ApiRequest {
    DescribeCapabilities(Request<DescribeCapabilitiesTag, DescribeCapabilitiesParams>),
    GetContext(Request<GetContextTag, GetContextParams>),
    GetObject(Request<GetObjectTag, GetObjectParams>),
    ApplyCommands(Request<ApplyCommandsTag, ApplyCommandsParams>),
    RenderView(Request<RenderViewTag, RenderViewParams>),
    RunScenario(Request<RunScenarioTag, RunScenarioParams>),
    GetRun(Request<GetRunTag, GetRunParams>),
    CompareRuns(Request<CompareRunsTag, CompareRunsParams>),
    ComputePackaging(Request<ComputePackagingTag, ComputePackagingParams>),
    GetTask(Request<GetTaskTag, GetTaskParams>),
    CancelTask(Request<CancelTaskTag, CancelTaskParams>),
    AdoptArtifact(Request<AdoptArtifactTag, AdoptArtifactParams>),
    ImportPackage(Request<ImportPackageTag, ImportPackageParams>),
    SaveProject(Request<SaveProjectTag, SaveProjectParams>),
    ExportPackage(Request<ExportPackageTag, ExportPackageParams>),
    ExportEvaluation(Request<ExportEvaluationTag, ExportEvaluationParams>),
    QueryModuleLibrary(Request<QueryModuleLibraryTag, QueryModuleLibraryParams>),
    CheckFactoryDraft(Request<CheckFactoryDraftTag, CheckFactoryDraftParams>),
}

#[derive(Debug, Clone)]
pub enum CallerOrigin {
    InteractiveUi,
    ExternalAgent,
}
#[derive(Debug, Clone)]
pub enum HostEditBranch {
    // Semantic binding permission for an exact existing consumer, not a leaf edit.
    AdoptArtifact,
    CreateProject,
    PutObject,
    RemoveObject,
    SetProjectSettings,
    CreateDefinitionDraft,
    PutDefinition,
    SetDeclaredParameter,
    AdoptModule,
    InstantiateTemplate,
    PutPackagingJob,
    RecordArtifactAcknowledgment,
    AdoptImportedDraft,
    CommitDefinitionVersion,
    Undo,
    Redo,
}

#[derive(Debug, Clone)]
pub struct HostFieldGrant {
    // RFC6901 leaf pointer relative to the canonical target, not a script/glob/root.
    pub pointer: String,
    // For arrays, pin the ID of the containing declared element. Host resolves the
    // current numeric index; reordering must not transfer authority to a new ID.
    pub element_id: Option<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct HostObjectEditGrant {
    pub target: ObjectRef,
    pub fields: Vec<HostFieldGrant>,
    pub can_remove: bool,
}

#[derive(Debug, Clone)]
pub struct HostCreateAllowance {
    pub kind: ObjectKind,
    // Cumulative distinct new IDs across this grant, not a fresh allowance/batch.
    pub max_new_objects: SafeUint,
    pub fields: Vec<HostFieldGrant>,
}

#[derive(Debug, Clone)]
pub struct HostEditGrant {
    pub id: ObjectId,
    pub scope: TaskScope,
    pub branches: Vec<HostEditBranch>,
    pub objects: Vec<HostObjectEditGrant>,
    pub creates: Vec<HostCreateAllowance>,
    // Exact selected module/template versions for all adoption/creation branches.
    // Host resolves their use; a caller's origin/source tag is never authority.
    pub source_versions: Vec<ModuleRef>,
    pub reusable_resources: Vec<ObjectId>,
    // Host-owned committed creation ledger. Never serialized into project/MCP.
    // Deletes/Undo do not refund the budget; authorized Redo restores the same ID.
    pub created_objects: Vec<ObjectRef>,
}

// Injected by the authenticated Host/bridge. Never deserialize this from the request.
#[derive(Debug, Clone)]
pub struct HostCallContext {
    pub origin: CallerOrigin,
    pub session_id: ObjectId,
    pub granted_project: Option<ProjectId>,
    pub allowed_operations: Vec<OperationName>,
    pub allowed_definition_drafts: Vec<ModuleId>,
    pub allow_view_control: bool,
    pub selected_file_tokens: Vec<ObjectId>,
    pub edit_grants: Vec<HostEditGrant>,
}

// Resolved from envelope + authoritative Host state before service dispatch.
#[derive(Debug, Clone)]
pub struct ResolvedCallScope {
    pub project_id: Option<ProjectId>,
    pub project_revision: Option<Revision>,
    pub edit_scope: Option<EditScope>,
    pub scope_revision: Option<Revision>,
}

/// Each method performs only admission/read/short commit. Long work returns a real
/// TaskHandle and runs outside the Host transaction; results are read via get_task.
pub trait CommandsService {
    fn describe_capabilities(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: DescribeCapabilitiesParams,
    ) -> Result<CapabilityCatalog, ApiError>;
    fn get_context(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: GetContextParams,
    ) -> Result<ProjectContext, ApiError>;
    fn get_object(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: GetObjectParams,
    ) -> Result<DomainObject, ApiError>;
    fn apply_commands(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: ApplyCommandsParams,
    ) -> Result<EditCommit, ApiError>;
    fn render_view(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: RenderViewParams,
    ) -> Result<TaskHandle, ApiError>;
    fn run_scenario(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: RunScenarioParams,
    ) -> Result<TaskHandle, ApiError>;
    fn get_run(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: GetRunParams,
    ) -> Result<RunView, ApiError>;
    fn compare_runs(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: CompareRunsParams,
    ) -> Result<RunComparison, ApiError>;
    fn compute_packaging(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: ComputePackagingParams,
    ) -> Result<TaskHandle, ApiError>;
    fn get_task(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: GetTaskParams,
    ) -> Result<TaskState, ApiError>;
    fn cancel_task(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: CancelTaskParams,
    ) -> Result<CancellationState, ApiError>;
    fn adopt_artifact(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: AdoptArtifactParams,
    ) -> Result<BindingCommit, ApiError>;
    fn import_package(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: ImportPackageParams,
    ) -> Result<TaskHandle, ApiError>;
    fn save_project(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: SaveProjectParams,
    ) -> Result<TaskHandle, ApiError>;
    fn export_package(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: ExportPackageParams,
    ) -> Result<TaskHandle, ApiError>;
    fn export_evaluation(
        &mut self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: ExportEvaluationParams,
    ) -> Result<TaskHandle, ApiError>;
    fn query_module_library(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: QueryModuleLibraryParams,
    ) -> Result<ModuleTemplateCandidates, ApiError>;
    fn check_factory_draft(
        &self,
        caller: &HostCallContext,
        scope: ResolvedCallScope,
        params: CheckFactoryDraftParams,
    ) -> Result<FactoryDraftCheckResult, ApiError>;
}

// Generation roots include all 18 response shapes as well as the complete request union.
model!(ApiContractBundle {
    request: ApiRequest, capability: Response<CapabilityCatalog>, context: Response<ProjectContext>,
    object: Response<DomainObject>, edit: Response<EditCommit>, task_handle: Response<TaskHandle>,
    run: Response<RunView>, comparison: Response<RunComparison>, task: Response<TaskState>,
    control: Response<CancellationState>, binding: Response<BindingCommit>,
    candidates: Response<ModuleTemplateCandidates>, check: Response<FactoryDraftCheckResult>,
    ipc_hello: IpcHello, ipc_welcome: IpcWelcome, ipc_request: IpcRequestFrame,
    ipc_task_response: IpcResponseFrame<TaskState>,
    ipc_run_response: IpcResponseFrame<RunView>,
});

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub enum IpcWireVersion {
    #[serde(rename = "1")]
    V1,
}
model!(IpcHello {
    wire_version: IpcWireVersion,
    expected_host_instance_id: ObjectId
});
model!(IpcWelcome {
    wire_version: IpcWireVersion,
    host_instance_id: ObjectId,
    client_session_id: ObjectId,
    api_version: ApiVersion,
    max_frame_bytes: SafeUint,
});
model!(IpcRequestFrame {
    wire_version: IpcWireVersion,
    host_instance_id: ObjectId,
    client_session_id: ObjectId,
    request: ApiRequest,
});
model!(IpcManagedAssetAttachment {
    task_id: TaskId,
    asset_id: AssetId,
    scope: EditScope,
    scope_revision: Revision,
    mime_type: String,
    byte_length: SafeUint,
    // Windows handle already duplicated into the authenticated bridge process.
    // Host-to-bridge output only; never a path or client-supplied read capability.
    handle_value: SafeUint,
});

model!(IpcManagedResultAttachment {
    result_id: ObjectId,
    // Read-only, non-inherited handle duplicated into the OS-verified bridge.
    // Metadata and authorization bind to reply.outcome.Managed and this frame.
    handle_value: SafeUint,
});

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct IpcResponseFrame<R> {
    pub wire_version: IpcWireVersion,
    pub host_instance_id: ObjectId,
    pub client_session_id: ObjectId,
    pub originating_operation: OperationName,
    pub reply: Response<R>,
    #[ts(optional = nullable)]
    pub managed_assets: Option<Vec<IpcManagedAssetAttachment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub managed_results: Option<Vec<IpcManagedResultAttachment>>,
}
