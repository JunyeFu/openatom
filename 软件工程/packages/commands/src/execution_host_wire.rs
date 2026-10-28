//! Strict Host execution wire boundary. Admission and side effects belong to Host.
//! Kept separate so schema/TS exports derive from the same real product types.
use super::*;

fn host_present_nullable<'de, T: serde::Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    <Option<T> as serde::Deserialize>::deserialize(deserializer)
}

fn host_unique_items_schema(schema: &mut schemars::Schema) {
    schema
        .ensure_object()
        .insert("uniqueItems".into(), true.into());
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportDestinationPurpose {
    #[serde(rename = "export_cloud_data")]
    ExportCloudData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudDataExportDestination {
    pub selection_id: sf_domain::ObjectId,
    pub purpose: CloudDataExportDestinationPurpose,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportTaskOutputKind {
    #[serde(rename = "cloud_data_export")]
    CloudDataExport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudDataExportTaskOutput {
    pub kind: CloudDataExportTaskOutputKind,
    pub data: ExportCloudDataResult,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportTaskStateOriginatingOperation {
    #[serde(rename = "export_cloud_data")]
    ExportCloudData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportTaskStateAllowedControlsItem {
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudDataExportTaskState {
    pub task: ExportingTaskHandle,
    pub originating_operation: CloudDataExportTaskStateOriginatingOperation,
    pub allowed_controls: Vec<CloudDataExportTaskStateAllowedControlsItem>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_1")]
    pub result: Option<CloudDataExportTaskOutput>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_2")]
    #[ts(
        type = "(Omit<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'> & Partial<Pick<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'>>) | null"
    )]
    pub failure: Option<ApiError>,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionPreparationTaskOutputKind {
    #[serde(rename = "execution_session")]
    ExecutionSession,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPreparationTaskOutput {
    pub kind: ExecutionPreparationTaskOutputKind,
    pub data: SessionAdmissionResult,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionPreparationTaskStateOriginatingOperation {
    #[serde(rename = "start_execution_session")]
    StartExecutionSession,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionPreparationTaskStateAllowedControlsItem {
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPreparationTaskState {
    pub task: StartupTaskHandle,
    pub originating_operation: ExecutionPreparationTaskStateOriginatingOperation,
    pub allowed_controls: Vec<ExecutionPreparationTaskStateAllowedControlsItem>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_3")]
    pub result: Option<ExecutionPreparationTaskOutput>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_4")]
    #[ts(
        type = "(Omit<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'> & Partial<Pick<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'>>) | null"
    )]
    pub failure: Option<ApiError>,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportDestinationPurpose {
    #[serde(rename = "export_execution_report")]
    ExportExecutionReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportDestination {
    pub selection_id: sf_domain::ObjectId,
    pub purpose: ExecutionReportDestinationPurpose,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportTaskOutputKind {
    #[serde(rename = "execution_report")]
    ExecutionReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportTaskOutput {
    pub kind: ExecutionReportTaskOutputKind,
    pub data: ExportExecutionReportResult,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportTaskStateOriginatingOperation {
    #[serde(rename = "export_execution_report")]
    ExportExecutionReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportTaskStateAllowedControlsItem {
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportTaskState {
    pub task: ExportingTaskHandle,
    pub originating_operation: ExecutionReportTaskStateOriginatingOperation,
    pub allowed_controls: Vec<ExecutionReportTaskStateAllowedControlsItem>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_5")]
    pub result: Option<ExecutionReportTaskOutput>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_6")]
    #[ts(
        type = "(Omit<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'> & Partial<Pick<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'>>) | null"
    )]
    pub failure: Option<ApiError>,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredStartExecutionRequestState {
    #[serde(rename = "registered")]
    Registered,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredStartExecutionRequestOperation {
    #[serde(rename = "start_execution_session")]
    StartExecutionSession,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct RegisteredStartExecutionRequest {
    pub host_context_session_id: sf_domain::ObjectId,
    pub state: RegisteredStartExecutionRequestState,
    pub operation: RegisteredStartExecutionRequestOperation,
    pub task: StartupTaskHandle,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredCloudReadRequestState {
    #[serde(rename = "registered")]
    Registered,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredCloudReadRequestOperation {
    #[serde(rename = "read_cloud_production")]
    ReadCloudProduction,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct RegisteredCloudReadRequest {
    pub host_context_session_id: sf_domain::ObjectId,
    pub state: RegisteredCloudReadRequestState,
    pub operation: RegisteredCloudReadRequestOperation,
    pub task: CloudReadTaskHandle,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredExecutionReportRequestState {
    #[serde(rename = "registered")]
    Registered,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredExecutionReportRequestOperation {
    #[serde(rename = "export_execution_report")]
    ExportExecutionReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct RegisteredExecutionReportRequest {
    pub host_context_session_id: sf_domain::ObjectId,
    pub state: RegisteredExecutionReportRequestState,
    pub operation: RegisteredExecutionReportRequestOperation,
    pub task: ExportingTaskHandle,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredCloudDataExportRequestState {
    #[serde(rename = "registered")]
    Registered,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum RegisteredCloudDataExportRequestOperation {
    #[serde(rename = "export_cloud_data")]
    ExportCloudData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct RegisteredCloudDataExportRequest {
    pub host_context_session_id: sf_domain::ObjectId,
    pub state: RegisteredCloudDataExportRequestState,
    pub operation: RegisteredCloudDataExportRequestOperation,
    pub task: ExportingTaskHandle,
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionRequestNotFoundState {
    #[serde(rename = "not_found")]
    NotFound,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequestNotFound {
    pub host_context_session_id: sf_domain::ObjectId,
    pub state: ExecutionRequestNotFoundState,
    pub operation: (),
    pub task: (),
    pub originating_request_id: sf_domain::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(untagged)]
pub enum ExecutionRequestStatus {
    RegisteredStart(RegisteredStartExecutionRequest),
    RegisteredCloudRead(RegisteredCloudReadRequest),
    RegisteredReportExport(RegisteredExecutionReportRequest),
    RegisteredDataExport(RegisteredCloudDataExportRequest),
    NotFound(ExecutionRequestNotFound),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSourceSummary {
    pub identity: ExecutionIdentity,
    pub execution_revision: sf_domain::SafeUint,
    pub project_input_ref: sf_domain::RunInputRef,
    pub observed_until_host_ms: sf_domain::SafeUint,
    pub clock_started: bool,
    pub lifecycle_state: ExecutionSnapshotLifecycleState,
    pub goal_at_snapshot: ExecutionSnapshotGoalAtSnapshot,
    pub active_owner: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ExecutionViewRead {
    #[serde(rename = "inputs")]
    Inputs { data: ExecutionFrozenInputs },
    #[serde(rename = "observations")]
    Observations { data: ExecutionObservationPage },
    #[serde(rename = "history")]
    History { data: ExecutionHistoryPage },
    #[serde(rename = "request")]
    Request { data: ExecutionRequestStatus },
    #[serde(rename = "catalog")]
    Catalog { data: LocalExecutionCatalog },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ExecutionViewSelection {
    #[serde(rename = "inputs")]
    Inputs { identity: ExecutionIdentity },
    #[serde(rename = "observations")]
    Observations { query: ExecutionObservationQuery },
    #[serde(rename = "history")]
    History { query: ExecutionHistoryQuery },
    #[serde(rename = "request")]
    Request {
        originating_request_id: sf_domain::ObjectId,
    },
    #[serde(rename = "catalog")]
    Catalog {},
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExportQueryTelemetryAggregation {
    #[serde(rename = "rows")]
    Rows,
    #[serde(rename = "count_by_device_minute")]
    CountByDeviceMinute,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportQueryTelemetryPage {
    #[ts(type = "200")]
    pub limit: ExportPageSize,
    pub cursor: (),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExportQueryExecutionSelectionKind {
    #[serde(rename = "revision")]
    Revision,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportQueryExecutionSelection {
    pub kind: HostExportQueryExecutionSelectionKind,
    pub execution_revision: sf_domain::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExportQueryExecutionAggregation {
    #[serde(rename = "rows")]
    Rows,
    #[serde(rename = "accepted_quantity")]
    AcceptedQuantity,
    #[serde(rename = "command_counts")]
    CommandCounts,
    #[serde(rename = "waiting_time")]
    WaitingTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportQueryExecutionPage {
    #[ts(type = "200")]
    pub limit: ExportPageSize,
    pub cursor: (),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "dataset", deny_unknown_fields)]
pub enum HostExportQuery {
    #[serde(rename = "telemetry")]
    Telemetry {
        contract_version: ExecutionContractVersion,
        cloud_project_id: sf_domain::CloudProjectId,
        dataset_id: sf_domain::CloudDatasetId,
        device_ids: Vec<sf_domain::ObjectId>,
        received_from: CloudTimestamp,
        received_until: CloudTimestamp,
        aggregation: HostExportQueryTelemetryAggregation,
        page: HostExportQueryTelemetryPage,
    },
    #[serde(rename = "execution")]
    Execution {
        contract_version: ExecutionContractVersion,
        identity: ExecutionIdentity,
        selection: HostExportQueryExecutionSelection,
        host_from_ms: sf_domain::SafeUint,
        host_until_ms: sf_domain::SafeUint,
        aggregation: HostExportQueryExecutionAggregation,
        page: HostExportQueryExecutionPage,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExportCloudDataResult {
    pub export_id: sf_domain::ObjectId,
    pub destination: CloudDataExportDestination,
    pub query: HostExportQuery,
    pub selection: CloudDataExportSelection,
    pub page_count: PositiveSafeUint,
    pub row_count: sf_domain::SafeUint,
    pub quality_issue_occurrence_count: sf_domain::SafeUint,
    #[schemars(length(min = 2, max = 2))]
    #[serde(deserialize_with = "checked_array_7")]
    pub files: [FileRef; 2],
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_8")]
    pub dataset_descriptor: Option<DatasetDescriptor>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExportExecutionReportResult {
    pub destination: ExecutionReportDestination,
    pub report_file: FileRef,
    pub report_id: sf_domain::ObjectId,
    #[schemars(length(min = 1, max = 2))]
    #[serde(deserialize_with = "checked_array_9")]
    pub sources: Vec<ExecutionReportSourceStamp>,
    pub files: Vec<FileRef>,
    pub limitations: Vec<sf_domain::Issue>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_10")]
    pub prediction: Option<PredictionExecutionLink>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExportingTaskHandleKind {
    #[serde(rename = "exporting")]
    Exporting,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExportingTaskHandle {
    pub id: TaskId,
    pub scope: TaskScope,
    pub kind: ExportingTaskHandleKind,
    pub state: ComputationStatus,
    pub input_revision: sf_domain::SafeUint,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_11")]
    pub progress: Option<TaskProgress>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_12")]
    pub result_ref: Option<FileRef>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_13")]
    pub failure: Option<Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostProjectScopeKind {
    #[serde(rename = "project")]
    Project,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostProjectScope {
    pub kind: HostProjectScopeKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostAckCloudWorkRequestOperation {
    #[serde(rename = "acknowledge_execution_cloud_work")]
    AcknowledgeExecutionCloudWork,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostAckCloudWorkRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
    pub ack: CloudEffectAck,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostAckCloudWorkRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_14")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostAckCloudWorkRequestOperation,
    pub params: HostAckCloudWorkRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostApplyReceiptRequestOperation {
    #[serde(rename = "apply_device_receipt")]
    ApplyDeviceReceipt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostApplyReceiptRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
    pub receipt_id: sf_domain::ObjectId,
    pub cloud_received_at: CloudTimestamp,
    pub observation: DeviceReceipt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostApplyReceiptRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_15")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostApplyReceiptRequestOperation,
    pub params: HostApplyReceiptRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostControlRequestOperation {
    #[serde(rename = "control_execution_session")]
    ControlExecutionSession,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostControlRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
    pub expected_execution_revision: sf_domain::SafeUint,
    pub action: ControlAction,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostControlRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_16")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostControlRequestOperation,
    pub params: HostControlRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExecutionViewRequestOperation {
    #[serde(rename = "read_execution_view")]
    ReadExecutionView,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExecutionViewRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub selection: ExecutionViewSelection,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExecutionViewRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_17")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostExecutionViewRequestOperation,
    pub params: HostExecutionViewRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExportCloudDataRequestOperation {
    #[serde(rename = "export_cloud_data")]
    ExportCloudData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportCloudDataRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub revision: sf_domain::SafeUint,
    pub query: HostExportQuery,
    pub selection: CloudDataExportSelection,
    pub selected_destination_ref: CloudDataExportDestination,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportCloudDataRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_18")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostExportCloudDataRequestOperation,
    pub params: HostExportCloudDataRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExportExecutionReportRequestOperation {
    #[serde(rename = "export_execution_report")]
    ExportExecutionReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostExportExecutionReportRequestParamsFormatsItem {
    #[serde(rename = "pdf")]
    Pdf,
    #[serde(rename = "html")]
    Html,
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "json")]
    Json,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportExecutionReportRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub revision: sf_domain::SafeUint,
    #[schemars(length(min = 1, max = 2))]
    #[serde(deserialize_with = "checked_array_20")]
    pub sources: Vec<ExecutionReportSource>,
    #[schemars(length(min = 1))]
    #[schemars(transform = host_unique_items_schema)]
    #[serde(deserialize_with = "checked_array_21")]
    pub formats: Vec<HostExportExecutionReportRequestParamsFormatsItem>,
    pub selected_destination_ref: ExecutionReportDestination,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_22")]
    pub prediction: Option<PredictionExecutionLink>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExportExecutionReportRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_19")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostExportExecutionReportRequestOperation,
    pub params: HostExportExecutionReportRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostGetCloudWorkRequestOperation {
    #[serde(rename = "get_execution_cloud_work")]
    GetExecutionCloudWork,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostGetCloudWorkRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostGetCloudWorkRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_23")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostGetCloudWorkRequestOperation,
    pub params: HostGetCloudWorkRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostObservationRequestOperation {
    #[serde(rename = "apply_device_observation")]
    ApplyDeviceObservation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostObservationRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
    pub receipt_id: sf_domain::ObjectId,
    pub cloud_received_at: CloudTimestamp,
    pub observation: ExecutionTelemetryObservation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostObservationRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_24")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostObservationRequestOperation,
    pub params: HostObservationRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostSessionDataKind {
    #[serde(rename = "session")]
    Session,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostSessionData {
    pub kind: HostSessionDataKind,
    pub data: SessionAdmissionResult,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostSnapshotDataKind {
    #[serde(rename = "snapshot")]
    Snapshot,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostSnapshotData {
    pub kind: HostSnapshotDataKind,
    pub data: ExecutionReadEnvelope,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostControlDataKind {
    #[serde(rename = "control")]
    Control,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostControlData {
    pub kind: HostControlDataKind,
    pub data: ControlAck,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostReceiptAdmissionDataKind {
    #[serde(rename = "receipt_admission")]
    ReceiptAdmission,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostReceiptAdmissionData {
    pub kind: HostReceiptAdmissionDataKind,
    pub data: ReceiptAdmission,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostCloudWorkDataKind {
    #[serde(rename = "cloud_work")]
    CloudWork,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostCloudWorkData {
    pub kind: HostCloudWorkDataKind,
    pub data: CloudWorkBatch,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostCloudCommitDataKind {
    #[serde(rename = "cloud_commit")]
    CloudCommit,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostCloudCommitData {
    pub kind: HostCloudCommitDataKind,
    pub data: CloudEffectAck,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(untagged)]
pub enum HostOkData {
    PreparationTask(StartupTaskHandle),
    Session(HostSessionData),
    Snapshot(HostSnapshotData),
    Control(HostControlData),
    ReceiptAdmission(HostReceiptAdmissionData),
    CloudWork(HostCloudWorkData),
    CloudCommit(HostCloudCommitData),
    PreparationTaskState(ExecutionPreparationTaskState),
    CloudReadTask(CloudReadTaskHandle),
    CloudReadTaskState(CloudReadTaskState),
    ExecutionView(ExecutionViewRead),
    ExportTask(ExportingTaskHandle),
    ReportExportTaskState(ExecutionReportTaskState),
    DataExportTaskState(CloudDataExportTaskState),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostReadRequestOperation {
    #[serde(rename = "get_execution_snapshot")]
    GetExecutionSnapshot,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostReadRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
    pub selection: SnapshotSelection,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostReadRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_25")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostReadRequestOperation,
    pub params: HostReadRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum HostResponseOutcome {
    #[serde(rename = "ok")]
    Ok { data: HostOkData },
    #[serde(rename = "error")]
    Error {
        #[ts(
            type = "Omit<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'> & Partial<Pick<import('./ApiError').ApiError, 'expected_revision' | 'actual_revision'>>"
        )]
        data: ApiError,
    },
    #[serde(rename = "managed")]
    Managed { data: ManagedJsonResultRef },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostResponse {
    pub api_version: ApiVersion,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_26")]
    pub request_id: Option<sf_domain::ObjectId>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_27")]
    #[ts(
        type = "(Omit<import('./ResponseContext').ResponseContext, 'project_id' | 'project_revision' | 'scope' | 'scope_revision'> & Partial<Pick<import('./ResponseContext').ResponseContext, 'project_id' | 'project_revision' | 'scope' | 'scope_revision'>>) | null"
    )]
    pub context: Option<ResponseContext>,
    pub outcome: HostResponseOutcome,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum HostStartRequestOperation {
    #[serde(rename = "start_execution_session")]
    StartExecutionSession,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostStartRequestParams {
    pub execution_contract_version: ExecutionContractVersion,
    pub base_revision: sf_domain::SafeUint,
    pub scenario_id: sf_domain::ScenarioId,
    pub scenario_revision: sf_domain::SafeUint,
    pub candidate_bindings: Vec<CandidateBinding>,
    pub policy: ExecutionPolicy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostStartRequest {
    pub api_version: ApiVersion,
    pub request_id: sf_domain::ObjectId,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_28")]
    pub project_id: Option<sf_domain::ProjectId>,
    pub scope: HostProjectScope,
    pub operation: HostStartRequestOperation,
    pub params: HostStartRequestParams,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct LocalExecutionCatalog {
    pub project_id: sf_domain::ProjectId,
    pub entries: Vec<ExecutionSourceSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum SessionAdmissionResultState {
    #[serde(rename = "prepared")]
    Prepared,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct SessionAdmissionResult {
    pub identity: ExecutionIdentity,
    pub state: SessionAdmissionResultState,
    pub execution_revision: sf_domain::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum StartupTaskHandleKind {
    #[serde(rename = "execution_preparation")]
    ExecutionPreparation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct StartupTaskHandle {
    pub id: TaskId,
    pub scope: TaskScope,
    pub kind: StartupTaskHandleKind,
    pub state: ComputationStatus,
    pub input_revision: sf_domain::SafeUint,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_29")]
    pub progress: Option<TaskProgress>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_30")]
    pub result_ref: Option<FileRef>,
    #[serde(deserialize_with = "host_present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_31")]
    pub failure: Option<Issue>,
}

fn nullable_schema_1(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<CloudDataExportTaskOutput>>()
}
fn nullable_schema_2(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ApiError>>()
}
fn nullable_schema_3(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ExecutionPreparationTaskOutput>>()
}
fn nullable_schema_4(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ApiError>>()
}
fn nullable_schema_5(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ExecutionReportTaskOutput>>()
}
fn nullable_schema_6(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ApiError>>()
}
fn checked_array_7<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[FileRef; 2], D::Error> {
    let value = <[FileRef; 2] as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() < 2 {
        return Err(serde::de::Error::custom("at least 2 items required"));
    }
    if value.len() > 2 {
        return Err(serde::de::Error::custom("at most 2 items allowed"));
    }
    Ok(value)
}
fn nullable_schema_8(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<DatasetDescriptor>>()
}
fn checked_array_9<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ExecutionReportSourceStamp>, D::Error> {
    let value = <Vec<ExecutionReportSourceStamp> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() < 1 {
        return Err(serde::de::Error::custom("at least 1 items required"));
    }
    if value.len() > 2 {
        return Err(serde::de::Error::custom("at most 2 items allowed"));
    }
    Ok(value)
}
fn nullable_schema_10(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<PredictionExecutionLink>>()
}
fn nullable_schema_11(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<TaskProgress>>()
}
fn nullable_schema_12(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<FileRef>>()
}
fn nullable_schema_13(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<Issue>>()
}
fn nullable_schema_14(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_15(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_16(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_17(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_18(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_19(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn checked_array_20<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ExecutionReportSource>, D::Error> {
    let value = <Vec<ExecutionReportSource> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() < 1 {
        return Err(serde::de::Error::custom("at least 1 items required"));
    }
    if value.len() > 2 {
        return Err(serde::de::Error::custom("at most 2 items allowed"));
    }
    Ok(value)
}
fn checked_array_21<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<HostExportExecutionReportRequestParamsFormatsItem>, D::Error> {
    let value = <Vec<HostExportExecutionReportRequestParamsFormatsItem> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() < 1 {
        return Err(serde::de::Error::custom("at least 1 items required"));
    }
    let mut seen = Vec::new();
    for item in &value {
        let item = serde_json::to_value(item).map_err(serde::de::Error::custom)?;
        if seen.contains(&item) {
            return Err(serde::de::Error::custom("unique items required"));
        }
        seen.push(item);
    }
    Ok(value)
}
fn nullable_schema_22(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<PredictionExecutionLink>>()
}
fn nullable_schema_23(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_24(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_25(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_26(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ObjectId>>()
}
fn nullable_schema_27(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ResponseContext>>()
}
fn nullable_schema_28(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<sf_domain::ProjectId>>()
}
fn nullable_schema_29(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<TaskProgress>>()
}
fn nullable_schema_30(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<FileRef>>()
}
fn nullable_schema_31(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<Issue>>()
}
impl HostAckCloudWorkRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostAckCloudWorkRequestOperation, HostAckCloudWorkRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostApplyReceiptRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostApplyReceiptRequestOperation, HostApplyReceiptRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostControlRequest {
    pub fn into_inner(self) -> Request<HostControlRequestOperation, HostControlRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostExecutionViewRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostExecutionViewRequestOperation, HostExecutionViewRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostExportCloudDataRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostExportCloudDataRequestOperation, HostExportCloudDataRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostExportExecutionReportRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostExportExecutionReportRequestOperation, HostExportExecutionReportRequestParams>
    {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostGetCloudWorkRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostGetCloudWorkRequestOperation, HostGetCloudWorkRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostObservationRequest {
    pub fn into_inner(
        self,
    ) -> Request<HostObservationRequestOperation, HostObservationRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostReadRequest {
    pub fn into_inner(self) -> Request<HostReadRequestOperation, HostReadRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}

impl HostStartRequest {
    pub fn into_inner(self) -> Request<HostStartRequestOperation, HostStartRequestParams> {
        Request {
            api_version: self.api_version,
            request_id: self.request_id,
            project_id: self.project_id,
            scope: ApiScope::Project,
            operation: self.operation,
            params: self.params,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostExecutionContractBundle {
    pub cloud_data_export_destination: Vec<CloudDataExportDestination>,
    pub cloud_data_export_task_output: Vec<CloudDataExportTaskOutput>,
    pub cloud_data_export_task_state: Vec<CloudDataExportTaskState>,
    pub cloud_effect_ack: Vec<CloudEffectAck>,
    pub cloud_production_configuration: Vec<CloudProductionConfiguration>,
    pub cloud_read_result: Vec<CloudReadResult>,
    pub cloud_read_selection: Vec<CloudReadSelection>,
    pub cloud_read_task_handle: Vec<CloudReadTaskHandle>,
    pub cloud_read_task_output: Vec<CloudReadTaskOutput>,
    pub cloud_read_task_state: Vec<CloudReadTaskState>,
    pub cloud_work_batch: Vec<CloudWorkBatch>,
    pub cloud_work_item: Vec<CloudWorkItem>,
    pub control_ack: Vec<ControlAck>,
    pub control_action: Vec<ControlAction>,
    pub execution_preparation_task_output: Vec<ExecutionPreparationTaskOutput>,
    pub execution_preparation_task_state: Vec<ExecutionPreparationTaskState>,
    pub execution_report_destination: Vec<ExecutionReportDestination>,
    pub execution_report_task_output: Vec<ExecutionReportTaskOutput>,
    pub execution_report_task_state: Vec<ExecutionReportTaskState>,
    pub execution_request_status: Vec<ExecutionRequestStatus>,
    pub execution_source_summary: Vec<ExecutionSourceSummary>,
    pub execution_view_read: Vec<ExecutionViewRead>,
    pub execution_view_selection: Vec<ExecutionViewSelection>,
    pub export_cloud_data_result: Vec<ExportCloudDataResult>,
    pub export_execution_report_result: Vec<ExportExecutionReportResult>,
    pub exporting_task_handle: Vec<ExportingTaskHandle>,
    pub host_ack_cloud_work_request: Vec<HostAckCloudWorkRequest>,
    pub host_apply_receipt_request: Vec<HostApplyReceiptRequest>,
    pub host_cloud_read_request: Vec<HostCloudReadRequest>,
    pub host_control_request: Vec<HostControlRequest>,
    pub host_execution_view_request: Vec<HostExecutionViewRequest>,
    pub host_export_cloud_data_request: Vec<HostExportCloudDataRequest>,
    pub host_export_execution_report_request: Vec<HostExportExecutionReportRequest>,
    pub host_get_cloud_work_request: Vec<HostGetCloudWorkRequest>,
    pub host_observation_request: Vec<HostObservationRequest>,
    pub host_ok_data: Vec<HostOkData>,
    pub host_read_request: Vec<HostReadRequest>,
    pub host_response: Vec<HostResponse>,
    pub host_start_request: Vec<HostStartRequest>,
    pub local_execution_catalog: Vec<LocalExecutionCatalog>,
    pub session_admission_result: Vec<SessionAdmissionResult>,
    pub startup_task_handle: Vec<StartupTaskHandle>,
    pub cloud_read_request_envelope:
        Vec<Request<ReadCloudProductionTag, ReadCloudProductionParams>>,
    pub common_api_errors: Vec<ApiError>,
    pub common_response_contexts: Vec<ResponseContext>,
}
