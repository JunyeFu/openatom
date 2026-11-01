//! Host-owned execution requests and strict task views. Pure cloud payloads
//! remain in sf-domain; this module never supplies caller authority.
use crate::api::*;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use sf_domain::cloud_execution::*;
use sf_domain::*;
use ts_rs::TS;

#[path = "execution_host_wire.rs"]
mod host_wire;
pub use host_wire::*;

impl TryFrom<TaskHandle> for StartupTaskHandle {
    type Error = String;
    fn try_from(task: TaskHandle) -> Result<Self, String> {
        if !matches!(task.kind, LongTaskKind::ExecutionPreparation) {
            return Err("Task is not an execution preparation".into());
        }
        Ok(Self { id: task.id, scope: task.scope, kind: StartupTaskHandleKind::ExecutionPreparation,
            state: task.state, input_revision: task.input_revision.0, progress: task.progress,
            result_ref: task.result_ref, failure: task.failure })
    }
}
impl From<ExecutionPreparationTaskState> for TaskState {
    fn from(state: ExecutionPreparationTaskState) -> Self {
        let handle = state.task;
        Self {
            task: TaskHandle { id: handle.id, scope: handle.scope, kind: LongTaskKind::ExecutionPreparation,
                state: handle.state, input_revision: Revision(handle.input_revision), progress: handle.progress,
                result_ref: handle.result_ref, failure: handle.failure },
            originating_operation: OperationName::StartExecutionSession,
            allowed_controls: state.allowed_controls.into_iter().map(|_| TaskControlAction::Cancel).collect(),
            result: state.result.map(|output| TaskOutput::ExecutionSession(Box::new(output.data))),
            failure: state.failure, originating_request_id: Some(state.originating_request_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlAction {
    Run {},
    PauseDispatch {},
    ResumeDispatch {},
    StopDispatch {},
    RequestReconciliation {},
    SetDeviceAcceptance { device_id: ObjectId, accept_new_work: bool },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ControlAdmission { Applied, AlreadyApplied }
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ControlAck {
    pub identity: ExecutionIdentity,
    pub execution_revision: SafeUint,
    pub action: ControlAction,
    pub admission: ControlAdmission,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CloudWorkItem {
    PublishCommand { work_id: ObjectId, command: Command },
    PublishSnapshot { work_id: ObjectId, publication: SnapshotPublication },
    WithdrawCommand { work_id: ObjectId, identity: ExecutionIdentity, command_id: DispatchCommandId },
}
impl CloudWorkItem {
    pub fn work_id(&self) -> &ObjectId {
        match self { Self::PublishCommand { work_id, .. } | Self::PublishSnapshot { work_id, .. }
            | Self::WithdrawCommand { work_id, .. } => work_id }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CloudWorkBatch { pub identity: ExecutionIdentity, pub items: Vec<CloudWorkItem> }
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CloudEffectAck {
    CommandStored { work_id: ObjectId, command_id: DispatchCommandId,
        storage_status: CloudResponseVariant0ResultCommandStoredDataStorageStatus },
    SnapshotPublished { work_id: ObjectId, ack: PublishAck },
    WithdrawalResult { work_id: ObjectId, command_id: DispatchCommandId,
        state: CloudResponseVariant0ResultWithdrawalDataState },
}
impl CloudEffectAck {
    pub fn work_id(&self) -> &ObjectId {
        match self { Self::CommandStored { work_id, .. } | Self::SnapshotPublished { work_id, .. }
            | Self::WithdrawalResult { work_id, .. } => work_id }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCoreContractBundle {
    pub actions: Vec<ControlAction>, pub acknowledgements: Vec<ControlAck>,
    pub work_batches: Vec<CloudWorkBatch>, pub cloud_effects: Vec<CloudEffectAck>,
}

fn present_nullable<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}
fn nullable_progress(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<TaskProgress>>()
}
fn nullable_file(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<FileRef>>()
}
fn nullable_issue(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<Issue>>()
}
fn nullable_id(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<ObjectId>>()
}
fn nullable_cloud_project(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<CloudProjectId>>()
}
fn nullable_task_output(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<CloudReadTaskOutput>>()
}
fn nullable_failure(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<Option<ApiError>>()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CloudReadSelection {
    Configuration {
        base_revision: Revision,
        scenario_id: ScenarioId,
        scenario_revision: Revision,
    },
    DatasetCatalog {},
    ActiveExecution {},
    DataQuery {
        query: DaaSQuery,
    },
    Snapshot {
        identity: ExecutionIdentity,
        selection: SnapshotSelection,
    },
    Command {
        identity: ExecutionIdentity,
        command_id: DispatchCommandId,
    },
    CommandReceipts {
        query: CommandReceiptsQuery,
    },
    Inputs {
        identity: ExecutionIdentity,
    },
    Observations {
        query: ExecutionObservationQuery,
    },
    History {
        query: ExecutionHistoryQuery,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ReadCloudProductionParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub selection: CloudReadSelection,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    try_from = "Request<ReadCloudProductionTag, ReadCloudProductionParams>",
    into = "Request<ReadCloudProductionTag, ReadCloudProductionParams>"
)]
#[ts(
    type = "import('./Request').Request<import('./ReadCloudProductionTag').ReadCloudProductionTag, import('./ReadCloudProductionParams').ReadCloudProductionParams> & { scope: { kind: 'project' } }"
)]
pub struct HostCloudReadRequest(Request<ReadCloudProductionTag, ReadCloudProductionParams>);
impl TryFrom<Request<ReadCloudProductionTag, ReadCloudProductionParams>> for HostCloudReadRequest {
    type Error = String;
    fn try_from(
        request: Request<ReadCloudProductionTag, ReadCloudProductionParams>,
    ) -> Result<Self, String> {
        if matches!(request.scope, ApiScope::Project) {
            Ok(Self(request))
        } else {
            Err("Cloud reads require Project scope".into())
        }
    }
}
impl From<HostCloudReadRequest> for Request<ReadCloudProductionTag, ReadCloudProductionParams> {
    fn from(request: HostCloudReadRequest) -> Self {
        request.0
    }
}
impl HostCloudReadRequest {
    pub fn into_inner(self) -> Request<ReadCloudProductionTag, ReadCloudProductionParams> {
        self.0
    }
}
impl JsonSchema for HostCloudReadRequest {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "HostCloudReadRequest".into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let mut schema =
            <Request<ReadCloudProductionTag, ReadCloudProductionParams>>::json_schema(g);
        schema.as_object_mut().unwrap().get_mut("properties").unwrap().as_object_mut().unwrap()
            .insert("scope".into(),serde_json::json!({"type":"object","properties":{"kind":{"const":"project"}},"required":["kind"],"additionalProperties":false}));
        schema
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CloudSourceMode {
    SoftwareInLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CloudProductionConfiguration {
    pub project_revision: Revision,
    pub scenario_id: ScenarioId,
    pub scenario_revision: Revision,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_id")]
    pub cloud_connection_ref: Option<ObjectId>,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_cloud_project")]
    pub cloud_project_id: Option<CloudProjectId>,
    pub source_mode: CloudSourceMode,
    pub candidate_bindings: Vec<CandidateBinding>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CloudReadResult {
    Configuration(CloudProductionConfiguration),
    Snapshot(ExecutionReadEnvelope),
    QueryResult(DaaSQueryResult),
    DatasetCatalog(CloudResponseVariant0ResultDatasetCatalogData),
    ActiveExecution(ActiveExecutionSlot),
    Command(CommandReadEnvelope),
    CommandReceipts(CommandReceiptsPage),
    Inputs(ExecutionFrozenInputs),
    Observations(ExecutionObservationPage),
    History(ExecutionHistoryPage),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CloudReadTaskKind {
    CloudRead,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CloudReadTaskHandle {
    pub id: TaskId,
    pub scope: TaskScope,
    pub kind: CloudReadTaskKind,
    pub state: ComputationStatus,
    pub input_revision: Revision,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_progress")]
    pub progress: Option<TaskProgress>,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_file")]
    pub result_ref: Option<FileRef>,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_issue")]
    pub failure: Option<Issue>,
}
impl TryFrom<TaskHandle> for CloudReadTaskHandle {
    type Error = String;
    fn try_from(t: TaskHandle) -> Result<Self, String> {
        if !matches!(t.kind, LongTaskKind::CloudRead) {
            return Err("cloud-read task kind required".into());
        }
        Ok(Self {
            id: t.id,
            scope: t.scope,
            kind: CloudReadTaskKind::CloudRead,
            state: t.state,
            input_revision: t.input_revision,
            progress: t.progress,
            result_ref: t.result_ref,
            failure: t.failure,
        })
    }
}
impl From<CloudReadTaskHandle> for TaskHandle {
    fn from(t: CloudReadTaskHandle) -> Self {
        Self {
            id: t.id,
            scope: t.scope,
            kind: LongTaskKind::CloudRead,
            state: t.state,
            input_revision: t.input_revision,
            progress: t.progress,
            result_ref: t.result_ref,
            failure: t.failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CloudReadTaskOutput {
    CloudRead(CloudReadResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum CloudReadTaskControl {
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CloudReadTaskState {
    pub task: CloudReadTaskHandle,
    pub originating_operation: ReadCloudProductionTag,
    pub allowed_controls: Vec<CloudReadTaskControl>,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_task_output")]
    pub result: Option<CloudReadTaskOutput>,
    #[serde(deserialize_with = "present_nullable")]
    #[schemars(required, schema_with = "nullable_failure")]
    pub failure: Option<ApiError>,
    pub originating_request_id: ObjectId,
}
impl From<CloudReadTaskState> for TaskState {
    fn from(s: CloudReadTaskState) -> Self {
        Self {
            task: s.task.into(),
            originating_operation: OperationName::ReadCloudProduction,
            allowed_controls: s
                .allowed_controls
                .into_iter()
                .map(|_| TaskControlAction::Cancel)
                .collect(),
            result: s
                .result
                .map(|CloudReadTaskOutput::CloudRead(data)| TaskOutput::CloudRead(Box::new(data))),
            failure: s.failure,
            originating_request_id: Some(s.originating_request_id),
        }
    }
}
