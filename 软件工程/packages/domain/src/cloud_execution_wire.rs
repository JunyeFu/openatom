// Initial Rust materialization of the frozen execution data boundary.
// This file is product source; subsequent schema/TS exports come from Rust.
use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
#[ts(type = "number")]
pub struct PageLimit(#[schemars(range(min = 1u64, max = 200u64))] u64);
impl TryFrom<u64> for PageLimit {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, String> {
        if (1..=200).contains(&value) {
            Ok(Self(value))
        } else {
            Err("wire integer outside bounds".into())
        }
    }
}
impl From<PageLimit> for u64 {
    fn from(value: PageLimit) -> Self {
        value.0
    }
}
impl PageLimit {
    pub fn value(&self) -> u64 {
        self.0
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(try_from = "i64", into = "i64")]
#[schemars(!try_from, !into)]
#[ts(type = "number")]
pub struct SignedSafeInt(
    #[schemars(range(min = -9007199254740991i64, max = 9007199254740991i64))] i64,
);
impl TryFrom<i64> for SignedSafeInt {
    type Error = String;
    fn try_from(value: i64) -> Result<Self, String> {
        if (-9007199254740991..=9007199254740991).contains(&value) {
            Ok(Self(value))
        } else {
            Err("wire integer outside bounds".into())
        }
    }
}
impl From<SignedSafeInt> for i64 {
    fn from(value: SignedSafeInt) -> Self {
        value.0
    }
}
impl SignedSafeInt {
    pub fn value(&self) -> i64 {
        self.0
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
#[ts(type = "1")]
pub struct CursorVersion(#[schemars(range(min = 1u64, max = 1u64))] u64);
impl TryFrom<u64> for CursorVersion {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, String> {
        if (1..=1).contains(&value) {
            Ok(Self(value))
        } else {
            Err("wire integer outside bounds".into())
        }
    }
}
impl From<CursorVersion> for u64 {
    fn from(value: CursorVersion) -> Self {
        value.0
    }
}
impl CursorVersion {
    pub fn value(&self) -> u64 {
        self.0
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
#[ts(type = "200")]
pub struct ExportPageSize(#[schemars(range(min = 200u64, max = 200u64))] u64);
impl TryFrom<u64> for ExportPageSize {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, String> {
        if (200..=200).contains(&value) {
            Ok(Self(value))
        } else {
            Err("wire integer outside bounds".into())
        }
    }
}
impl From<ExportPageSize> for u64 {
    fn from(value: ExportPageSize) -> Self {
        value.0
    }
}
impl ExportPageSize {
    pub fn value(&self) -> u64 {
        self.0
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(try_from = "bool", into = "bool")]
#[ts(type = "false")]
pub struct AlwaysFalse;
impl TryFrom<bool> for AlwaysFalse {
    type Error = String;
    fn try_from(value: bool) -> Result<Self, String> {
        if !value {
            Ok(Self)
        } else {
            Err("false required".into())
        }
    }
}
impl From<AlwaysFalse> for bool {
    fn from(_: AlwaysFalse) -> Self {
        false
    }
}
impl schemars::JsonSchema for AlwaysFalse {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "AlwaysFalse".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"const":false})
    }
}
fn unique_items_schema(schema: &mut schemars::Schema) {
    schema.insert("uniqueItems".into(), serde_json::json!(true));
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum AcceptanceRowKind {
    #[serde(rename = "acceptance")]
    Acceptance,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRow {
    pub kind: AcceptanceRowKind,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub event: crate::AcceptanceEvent,
    pub row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum AcceptedQuantityRowKind {
    #[serde(rename = "accepted_quantity")]
    AcceptedQuantity,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AcceptedQuantityRow {
    pub kind: AcceptedQuantityRowKind,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub host_from_ms: crate::SafeUint,
    pub host_until_ms: crate::SafeUint,
    pub quantity: crate::Quantity,
    pub row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ActiveExecutionSlotActiveSlotState {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "needs_resolution")]
    NeedsResolution,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ActiveExecutionSlotActiveSlot {
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub owner_host_context: crate::ObjectId,
    pub state: ActiveExecutionSlotActiveSlotState,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_1")]
    pub published_revision: Option<crate::SafeUint>,
    pub device_boots: Vec<DeviceBoot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ActiveExecutionSlot {
    pub cloud_project_id: crate::CloudProjectId,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_2")]
    pub active_slot: Option<ActiveExecutionSlotActiveSlot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum AttemptViewState {
    #[serde(rename = "reserved")]
    Reserved,
    #[serde(rename = "published")]
    Published,
    #[serde(rename = "device_accepted")]
    DeviceAccepted,
    #[serde(rename = "device_started")]
    DeviceStarted,
    #[serde(rename = "device_completed")]
    DeviceCompleted,
    #[serde(rename = "business_effect_committed")]
    BusinessEffectCommitted,
    #[serde(rename = "output_wait")]
    OutputWait,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "reconciliation_required")]
    ReconciliationRequired,
    #[serde(rename = "cancelled_not_started")]
    CancelledNotStarted,
    #[serde(rename = "device_failed")]
    DeviceFailed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AttemptView {
    pub attempt: AttemptRef,
    pub device_id: crate::ObjectId,
    pub state: AttemptViewState,
    pub receipt_ids: Vec<crate::ObjectId>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_3")]
    pub started_at_host_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_4")]
    pub completed_at_host_ms: Option<crate::SafeUint>,
    pub dispatch_decision: DispatchExplanation,
    pub target_device_boot_id: crate::DeviceBootId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum BoundExecutionJobSnapshotState {
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "working")]
    Working,
    #[serde(rename = "waiting_output")]
    WaitingOutput,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "needs_resolution")]
    NeedsResolution,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct BoundExecutionJobSnapshot {
    pub job_id: crate::ObjectId,
    pub logical_job_id: crate::ObjectId,
    pub execution_ref: crate::JobExecutionRef,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_5")]
    pub phase_id: Option<crate::PhaseId>,
    pub state: BoundExecutionJobSnapshotState,
    pub input_refs: Vec<crate::ObjectId>,
    pub started_at_host_ms: crate::SafeUint,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_6")]
    pub finished_at_host_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_7")]
    pub nominal_stage_work_ms: Option<crate::SafeUint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct BusinessFacts {
    #[serde(default)]
    pub transport: Vec<crate::TransportState>,
    pub items: Vec<crate::RuntimeItem>,
    pub allocations: Vec<crate::MaterialAllocation>,
    pub resource_claims: Vec<ProjectedResourceClaim>,
    pub acceptance_events: Vec<crate::AcceptanceEvent>,
    pub order_outcomes: Vec<crate::OrderOutcome>,
    pub pending_output_refs: Vec<crate::ObjectId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportRecordHeaderContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportRecordHeaderQueryTelemetryContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportRecordHeaderQueryTelemetryAggregation {
    #[serde(rename = "rows")]
    Rows,
    #[serde(rename = "count_by_device_minute")]
    CountByDeviceMinute,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudDataExportRecordHeaderQueryTelemetryPage {
    pub limit: ExportPageSize,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_8")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportRecordHeaderQueryExecutionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CloudDataExportRecordHeaderQueryExecutionSelection {
    #[serde(rename = "revision")]
    Revision { execution_revision: crate::SafeUint },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportRecordHeaderQueryExecutionAggregation {
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
pub struct CloudDataExportRecordHeaderQueryExecutionPage {
    pub limit: ExportPageSize,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_9")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "dataset", deny_unknown_fields)]
pub enum CloudDataExportRecordHeaderQuery {
    #[serde(rename = "telemetry")]
    Telemetry {
        contract_version: CloudDataExportRecordHeaderQueryTelemetryContractVersion,
        cloud_project_id: crate::CloudProjectId,
        dataset_id: crate::CloudDatasetId,
        device_ids: Vec<crate::ObjectId>,
        received_from: CloudTimestamp,
        received_until: CloudTimestamp,
        aggregation: CloudDataExportRecordHeaderQueryTelemetryAggregation,
        page: CloudDataExportRecordHeaderQueryTelemetryPage,
    },
    #[serde(rename = "execution")]
    Execution {
        contract_version: CloudDataExportRecordHeaderQueryExecutionContractVersion,
        identity: ExecutionIdentity,
        selection: CloudDataExportRecordHeaderQueryExecutionSelection,
        host_from_ms: crate::SafeUint,
        host_until_ms: crate::SafeUint,
        aggregation: CloudDataExportRecordHeaderQueryExecutionAggregation,
        page: CloudDataExportRecordHeaderQueryExecutionPage,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudDataExportRecordHeaderCsvCellEncoding {
    #[serde(rename = "json_value")]
    JsonValue,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "record_kind", deny_unknown_fields)]
pub enum CloudDataExportRecord {
    #[serde(rename = "header")]
    Header {
        contract_version: CloudDataExportRecordHeaderContractVersion,
        export_id: crate::ObjectId,
        project_id: crate::ProjectId,
        query: CloudDataExportRecordHeaderQuery,
        selection: CloudDataExportSelection,
        generated_at_utc: CloudTimestamp,
        #[schemars(length(min = 1))]
        #[schemars(transform = unique_items_schema)]
        #[serde(deserialize_with = "checked_array_10")]
        csv_columns: Vec<super::DeviceText>,
        csv_cell_encoding: CloudDataExportRecordHeaderCsvCellEncoding,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_11")]
        dataset_descriptor: Option<DatasetDescriptor>,
    },
    #[serde(rename = "page")]
    Page {
        export_id: crate::ObjectId,
        page_index: super::PositiveSafeUint,
        result: DaaSQueryResult,
    },
    #[serde(rename = "footer")]
    Footer {
        export_id: crate::ObjectId,
        page_count: super::PositiveSafeUint,
        row_count: crate::SafeUint,
        quality_issue_occurrence_count: crate::SafeUint,
        complete: super::StopAtCutoff,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "dataset", deny_unknown_fields)]
pub enum CloudDataExportSelection {
    #[serde(rename = "telemetry")]
    Telemetry {
        cloud_project_id: crate::CloudProjectId,
        dataset_id: crate::CloudDatasetId,
        selected_visible_frontier: crate::SafeUint,
        #[schemars(transform = unique_items_schema)]
        #[serde(deserialize_with = "checked_array_12")]
        selected_device_ids: Vec<crate::ObjectId>,
    },
    #[serde(rename = "execution")]
    Execution {
        identity: ExecutionIdentity,
        execution_revision: crate::SafeUint,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudErrorCode {
    #[serde(rename = "unauthenticated")]
    Unauthenticated,
    #[serde(rename = "permission_denied")]
    PermissionDenied,
    #[serde(rename = "unsupported_contract")]
    UnsupportedContract,
    #[serde(rename = "invalid_input")]
    InvalidInput,
    #[serde(rename = "missing_reference")]
    MissingReference,
    #[serde(rename = "revision_conflict")]
    RevisionConflict,
    #[serde(rename = "immutable_payload_conflict")]
    ImmutablePayloadConflict,
    #[serde(rename = "active_session_conflict")]
    ActiveSessionConflict,
    #[serde(rename = "epoch_mismatch")]
    EpochMismatch,
    #[serde(rename = "unsupported_capability")]
    UnsupportedCapability,
    #[serde(rename = "io_failure")]
    IoFailure,
    #[serde(rename = "device_boot_conflict")]
    DeviceBootConflict,
    #[serde(rename = "device_boot_mismatch")]
    DeviceBootMismatch,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudErrorFieldErrorsItem {
    pub pointer: String,
    pub reason: super::DeviceText,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudError {
    pub code: CloudErrorCode,
    pub message: super::DeviceText,
    pub field_errors: Vec<CloudErrorFieldErrorsItem>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_13")]
    pub expected_revision: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_14")]
    pub actual_revision: Option<crate::SafeUint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CloudEventRow {
    #[serde(rename = "receipt")]
    Receipt {
        receipt_id: crate::ObjectId,
        ingress_id: crate::SafeUint,
        cloud_received_at: CloudTimestamp,
        observation: DeviceReceipt,
    },
    #[serde(rename = "telemetry")]
    Telemetry {
        ingress_id: crate::SafeUint,
        cloud_received_at: CloudTimestamp,
        observation: ExecutionTelemetryObservation,
        receipt_id: crate::ObjectId,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudEventsPage {
    pub identity: ExecutionIdentity,
    pub rows: Vec<CloudEventRow>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_15")]
    pub next_ingress_id: Option<crate::SafeUint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudReceiptRequestContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudReceiptRequest {
    pub contract_version: CloudReceiptRequestContractVersion,
    pub observation: DeviceReceipt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudReleaseRequestContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudReleaseRequest {
    pub contract_version: CloudReleaseRequestContractVersion,
    pub identity: ExecutionIdentity,
    pub expected_published_revision: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudResponseVariant0ContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudResponseVariant0ResultSessionStoredDataStorageStatus {
    #[serde(rename = "stored")]
    Stored,
    #[serde(rename = "already_stored_same_payload")]
    AlreadyStoredSamePayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0ResultSessionStoredData {
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub storage_status: CloudResponseVariant0ResultSessionStoredDataStorageStatus,
    pub dataset_id: crate::CloudDatasetId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudResponseVariant0ResultCommandStoredDataStorageStatus {
    #[serde(rename = "stored")]
    Stored,
    #[serde(rename = "already_stored_same_payload")]
    AlreadyStoredSamePayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0ResultCommandStoredData {
    pub command_id: crate::DispatchCommandId,
    pub storage_status: CloudResponseVariant0ResultCommandStoredDataStorageStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudResponseVariant0ResultWithdrawalDataState {
    #[serde(rename = "withdrawn_before_delivery")]
    WithdrawnBeforeDelivery,
    #[serde(rename = "device_confirmation_required")]
    DeviceConfirmationRequired,
    #[serde(rename = "already_settled")]
    AlreadySettled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0ResultWithdrawalData {
    pub command_id: crate::DispatchCommandId,
    pub state: CloudResponseVariant0ResultWithdrawalDataState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0ResultSlotReleasedData {
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub released: super::StopAtCutoff,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0ResultLoadIngestedData {
    pub dataset_id: crate::CloudDatasetId,
    pub batch_id: crate::ObjectId,
    pub inserted_count: crate::SafeUint,
    pub reused_count: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0ResultDatasetCatalogData {
    pub cloud_project_id: crate::CloudProjectId,
    pub datasets: Vec<DatasetDescriptor>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CloudResponseVariant0Result {
    #[serde(rename = "session_stored")]
    SessionStored {
        data: CloudResponseVariant0ResultSessionStoredData,
    },
    #[serde(rename = "command_stored")]
    CommandStored {
        data: CloudResponseVariant0ResultCommandStoredData,
    },
    #[serde(rename = "device_poll")]
    DevicePoll { data: DevicePollResult },
    #[serde(rename = "event_stored")]
    EventStored { data: StoredReceipt },
    #[serde(rename = "events_page")]
    EventsPage { data: CloudEventsPage },
    #[serde(rename = "snapshot_published")]
    SnapshotPublished { data: PublishAck },
    #[serde(rename = "snapshot")]
    Snapshot { data: ExecutionReadEnvelope },
    #[serde(rename = "query_result")]
    QueryResult { data: DaaSQueryResult },
    #[serde(rename = "withdrawal")]
    Withdrawal {
        data: CloudResponseVariant0ResultWithdrawalData,
    },
    #[serde(rename = "slot_released")]
    SlotReleased {
        data: CloudResponseVariant0ResultSlotReleasedData,
    },
    #[serde(rename = "load_ingested")]
    LoadIngested {
        data: CloudResponseVariant0ResultLoadIngestedData,
    },
    #[serde(rename = "dataset_catalog")]
    DatasetCatalog {
        data: CloudResponseVariant0ResultDatasetCatalogData,
    },
    #[serde(rename = "active_execution")]
    ActiveExecution { data: ActiveExecutionSlot },
    #[serde(rename = "command")]
    Command { data: CommandReadEnvelope },
    #[serde(rename = "command_receipts")]
    CommandReceipts { data: CommandReceiptsPage },
    #[serde(rename = "inputs")]
    Inputs { data: ExecutionFrozenInputs },
    #[serde(rename = "observations")]
    Observations { data: ExecutionObservationPage },
    #[serde(rename = "history")]
    History { data: ExecutionHistoryPage },
    #[serde(rename = "device_boot")]
    DeviceBoot { data: DeviceBoot },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant0 {
    pub contract_version: CloudResponseVariant0ContractVersion,
    pub result: CloudResponseVariant0Result,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudResponseVariant1ContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudResponseVariant1 {
    pub contract_version: CloudResponseVariant1ContractVersion,
    pub error: CloudError,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(untagged)]
pub enum CloudResponse {
    Variant0(CloudResponseVariant0),
    Variant1(CloudResponseVariant1),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudSessionCreateContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudSessionCreate {
    pub contract_version: CloudSessionCreateContractVersion,
    pub identity: ExecutionIdentity,
    pub project_input_ref: crate::RunInputRef,
    pub policy: ExecutionPolicy,
    pub candidate_bindings: Vec<CandidateBinding>,
    pub frozen_inputs: ExecutionFrozenInputs,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CloudWithdrawRequestContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CloudWithdrawRequest {
    pub contract_version: CloudWithdrawRequestContractVersion,
    pub identity: ExecutionIdentity,
    pub command_id: crate::DispatchCommandId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandCountRowKind {
    #[serde(rename = "command_count")]
    CommandCount,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandCountRowState {
    #[serde(rename = "published")]
    Published,
    #[serde(rename = "device_accepted")]
    DeviceAccepted,
    #[serde(rename = "device_started")]
    DeviceStarted,
    #[serde(rename = "device_completed")]
    DeviceCompleted,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "device_failed")]
    DeviceFailed,
    #[serde(rename = "cancelled_not_started")]
    CancelledNotStarted,
    #[serde(rename = "reconciliation_required")]
    ReconciliationRequired,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandCountRow {
    pub kind: CommandCountRowKind,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub state: CommandCountRowState,
    pub count: crate::SafeUint,
    pub row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandReadEnvelopeCloudDeliveryState {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "claimed")]
    Claimed,
    #[serde(rename = "withdrawn")]
    Withdrawn,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReadEnvelopeCloudDelivery {
    pub state: CommandReadEnvelopeCloudDeliveryState,
    pub cancel_requested: bool,
    pub observed_at: CloudTimestamp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReadEnvelopeHostConfirmation {
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_16")]
    pub published_execution_revision: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_17")]
    pub latest_admission: Option<ReceiptAdmission>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReadEnvelope {
    pub command: Command,
    pub cloud_delivery: CommandReadEnvelopeCloudDelivery,
    pub host_confirmation: CommandReadEnvelopeHostConfirmation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandReceiptsCursorKind {
    #[serde(rename = "command_receipts")]
    CommandReceipts,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReceiptsCursorQuerySelectionPage {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_18")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandReceiptsCursorQuerySelectionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReceiptsCursorQuerySelection {
    pub identity: ExecutionIdentity,
    pub command_id: crate::DispatchCommandId,
    pub page: CommandReceiptsCursorQuerySelectionPage,
    pub contract_version: CommandReceiptsCursorQuerySelectionContractVersion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReceiptsCursor {
    pub kind: CommandReceiptsCursorKind,
    pub cursor_version: CursorVersion,
    pub query_selection: CommandReceiptsCursorQuerySelection,
    pub dataset_id: crate::CloudDatasetId,
    pub visible_frontier: crate::SafeUint,
    pub after_ingress_id: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandReceiptsPageRowsItemKind {
    #[serde(rename = "receipt")]
    Receipt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReceiptsPageRowsItem {
    pub kind: CommandReceiptsPageRowsItemKind,
    pub receipt_id: crate::ObjectId,
    pub ingress_id: crate::SafeUint,
    pub cloud_received_at: CloudTimestamp,
    pub observation: DeviceReceipt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReceiptsPage {
    pub identity: ExecutionIdentity,
    pub command_id: crate::DispatchCommandId,
    pub dataset_id: crate::CloudDatasetId,
    pub visible_frontier: crate::SafeUint,
    pub rows: Vec<CommandReceiptsPageRowsItem>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_19")]
    pub next_cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum CommandReceiptsQueryContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CommandReceiptsQuery {
    pub identity: ExecutionIdentity,
    pub command_id: crate::DispatchCommandId,
    pub page: PageSelection,
    pub contract_version: CommandReceiptsQueryContractVersion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryTelemetryContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryTelemetryAggregation {
    #[serde(rename = "rows")]
    Rows,
    #[serde(rename = "count_by_device_minute")]
    CountByDeviceMinute,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryExecutionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryExecutionAggregation {
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
#[serde(tag = "dataset", deny_unknown_fields)]
pub enum DaaSQuery {
    #[serde(rename = "telemetry")]
    Telemetry {
        contract_version: DaaSQueryTelemetryContractVersion,
        cloud_project_id: crate::CloudProjectId,
        dataset_id: crate::CloudDatasetId,
        device_ids: Vec<crate::ObjectId>,
        received_from: CloudTimestamp,
        received_until: CloudTimestamp,
        aggregation: DaaSQueryTelemetryAggregation,
        page: PageSelection,
    },
    #[serde(rename = "execution")]
    Execution {
        contract_version: DaaSQueryExecutionContractVersion,
        identity: ExecutionIdentity,
        selection: SnapshotSelection,
        host_from_ms: crate::SafeUint,
        host_until_ms: crate::SafeUint,
        aggregation: DaaSQueryExecutionAggregation,
        page: PageSelection,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryResultTelemetryContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryResultTelemetryRowsItemCountByDeviceMinuteSourceMode {
    #[serde(rename = "software_in_loop")]
    SoftwareInLoop,
    #[serde(rename = "physical_device")]
    PhysicalDevice,
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum DaaSQueryResultTelemetryRowsItem {
    #[serde(rename = "telemetry")]
    Telemetry {
        ingress_id: crate::SafeUint,
        dataset_id: crate::CloudDatasetId,
        cloud_received_at: CloudTimestamp,
        observation: TelemetryObservation,
    },
    #[serde(rename = "generated_load")]
    GeneratedLoad {
        ingress_id: crate::SafeUint,
        dataset_id: crate::CloudDatasetId,
        cloud_received_at: CloudTimestamp,
        event: GeneratedLoadEvent,
    },
    #[serde(rename = "count_by_device_minute")]
    CountByDeviceMinute {
        device_id: crate::ObjectId,
        source_mode: DaaSQueryResultTelemetryRowsItemCountByDeviceMinuteSourceMode,
        bucket_start: CloudTimestamp,
        count: crate::SafeUint,
        row_index: crate::SafeUint,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryResultExecutionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DaaSQueryResultExecutionRowsItemCommandCountState {
    #[serde(rename = "published")]
    Published,
    #[serde(rename = "device_accepted")]
    DeviceAccepted,
    #[serde(rename = "device_started")]
    DeviceStarted,
    #[serde(rename = "device_completed")]
    DeviceCompleted,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "device_failed")]
    DeviceFailed,
    #[serde(rename = "cancelled_not_started")]
    CancelledNotStarted,
    #[serde(rename = "reconciliation_required")]
    ReconciliationRequired,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum DaaSQueryResultExecutionRowsItem {
    #[serde(rename = "execution_attempt")]
    ExecutionAttempt {
        identity: ExecutionIdentity,
        execution_revision: crate::SafeUint,
        attempt: AttemptView,
        row_index: crate::SafeUint,
    },
    #[serde(rename = "acceptance")]
    Acceptance {
        identity: ExecutionIdentity,
        execution_revision: crate::SafeUint,
        event: crate::AcceptanceEvent,
        row_index: crate::SafeUint,
    },
    #[serde(rename = "accepted_quantity")]
    AcceptedQuantity {
        identity: ExecutionIdentity,
        execution_revision: crate::SafeUint,
        host_from_ms: crate::SafeUint,
        host_until_ms: crate::SafeUint,
        quantity: crate::Quantity,
        row_index: crate::SafeUint,
    },
    #[serde(rename = "command_count")]
    CommandCount {
        identity: ExecutionIdentity,
        execution_revision: crate::SafeUint,
        state: DaaSQueryResultExecutionRowsItemCommandCountState,
        count: crate::SafeUint,
        row_index: crate::SafeUint,
    },
    #[serde(rename = "waiting_time")]
    WaitingTime {
        identity: ExecutionIdentity,
        execution_revision: crate::SafeUint,
        logical_job_id: crate::ObjectId,
        reason: super::DeviceText,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_23")]
        known_host_ms: Option<crate::SafeUint>,
        missing_conditions: Vec<crate::Issue>,
        row_index: crate::SafeUint,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "dataset", deny_unknown_fields)]
pub enum DaaSQueryResult {
    #[serde(rename = "telemetry")]
    Telemetry {
        contract_version: DaaSQueryResultTelemetryContractVersion,
        selected_epoch: (),
        selected_execution_revision: (),
        #[schemars(length(max = 200))]
        #[serde(deserialize_with = "checked_array_20")]
        rows: Vec<DaaSQueryResultTelemetryRowsItem>,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_21")]
        next_cursor: Option<super::DeviceText>,
        data_quality_issues: Vec<crate::Issue>,
        dataset_id: crate::CloudDatasetId,
        selected_visible_frontier: crate::SafeUint,
        #[schemars(transform = unique_items_schema)]
        #[serde(deserialize_with = "checked_array_22")]
        selected_device_ids: Vec<crate::ObjectId>,
    },
    #[serde(rename = "execution")]
    Execution {
        contract_version: DaaSQueryResultExecutionContractVersion,
        selected_epoch: crate::ExecutionEpochId,
        selected_execution_revision: crate::SafeUint,
        #[schemars(length(max = 200))]
        #[serde(deserialize_with = "checked_array_24")]
        rows: Vec<DaaSQueryResultExecutionRowsItem>,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_25")]
        next_cursor: Option<super::DeviceText>,
        data_quality_issues: Vec<crate::Issue>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DatasetDescriptorSourceMode {
    #[serde(rename = "software_in_loop")]
    SoftwareInLoop,
    #[serde(rename = "physical_device")]
    PhysicalDevice,
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DatasetDescriptorPurpose {
    #[serde(rename = "execution")]
    Execution,
    #[serde(rename = "telemetry_only")]
    TelemetryOnly,
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DatasetDescriptor {
    pub dataset_id: crate::CloudDatasetId,
    pub cloud_project_id: crate::CloudProjectId,
    pub source_mode: DatasetDescriptorSourceMode,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_26")]
    pub session_id: Option<crate::ExecutionSessionId>,
    pub visible_frontier: crate::SafeUint,
    pub purpose: DatasetDescriptorPurpose,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DeviceBootState {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "retired")]
    Retired,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceBoot {
    pub device_id: crate::ObjectId,
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub device_boot_id: crate::DeviceBootId,
    pub state: DeviceBootState,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_27")]
    pub retired_at_published_revision: Option<crate::SafeUint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DeviceBootRegisterRequestContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceBootRegisterRequest {
    pub contract_version: DeviceBootRegisterRequestContractVersion,
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub device_boot_id: crate::DeviceBootId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DeviceBootRetireRequestContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceBootRetireRequest {
    pub contract_version: DeviceBootRetireRequestContractVersion,
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub expected_published_revision: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DeviceMinuteCountRowKind {
    #[serde(rename = "count_by_device_minute")]
    CountByDeviceMinute,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DeviceMinuteCountRowSourceMode {
    #[serde(rename = "software_in_loop")]
    SoftwareInLoop,
    #[serde(rename = "physical_device")]
    PhysicalDevice,
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceMinuteCountRow {
    pub kind: DeviceMinuteCountRowKind,
    pub device_id: crate::ObjectId,
    pub source_mode: DeviceMinuteCountRowSourceMode,
    pub bucket_start: CloudTimestamp,
    pub count: crate::SafeUint,
    pub row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DevicePollRequestContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DevicePollRequest {
    pub contract_version: DevicePollRequestContractVersion,
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub device_boot_id: crate::DeviceBootId,
    pub poll_request_id: crate::ObjectId,
    pub accept_commands: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum DevicePollResult {
    #[serde(rename = "command")]
    Command {
        poll_request_id: crate::ObjectId,
        command: Command,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_28")]
        observation_probe: Option<ObservationProbe>,
    },
    #[serde(rename = "empty")]
    Empty {
        poll_request_id: crate::ObjectId,
        command: (),
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_29")]
        observation_probe: Option<ObservationProbe>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DispatchCandidateView {
    pub binding_id: crate::ObjectId,
    pub eligible: bool,
    pub blocking_reasons: Vec<super::DeviceText>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_30")]
    pub estimated_completion_host_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_31")]
    pub rank: Option<super::PositiveSafeUint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum DispatchExplanationReason {
    #[serde(rename = "earliest_estimated_completion")]
    EarliestEstimatedCompletion,
    #[serde(rename = "stable_binding_tiebreak")]
    StableBindingTiebreak,
    #[serde(rename = "waiting_for_eligible_binding")]
    WaitingForEligibleBinding,
    #[serde(rename = "reselect_after_confirmed_withdrawal")]
    ReselectAfterConfirmedWithdrawal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DispatchExplanation {
    pub based_on_host_admission_sequence: crate::SafeUint,
    pub decision_host_ms: crate::SafeUint,
    pub reason: DispatchExplanationReason,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_32")]
    pub selected_binding_id: Option<crate::ObjectId>,
    pub candidates: Vec<DispatchCandidateView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAcceptedMetric {
    pub scope: crate::AcceptanceScope,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_33")]
    pub accepted_quantity: Option<crate::Quantity>,
    pub missing_conditions: Vec<crate::Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionArchiveIndexDocumentKind {
    #[serde(rename = "execution_archive_index")]
    ExecutionArchiveIndex,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionArchiveIndexContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionArchiveIndexSnapshotsItem {
    pub execution_revision: crate::SafeUint,
    pub snapshot_file: crate::FileRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionArchiveIndex {
    pub document_kind: ExecutionArchiveIndexDocumentKind,
    pub contract_version: ExecutionArchiveIndexContractVersion,
    pub identity: ExecutionIdentity,
    pub inputs_file: crate::FileRef,
    pub history_file: crate::FileRef,
    pub snapshots: Vec<ExecutionArchiveIndexSnapshotsItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAssetBinding {
    pub module_ref: crate::ModuleRef,
    pub asset: crate::AssetRef,
    pub archive_file: crate::FileRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionAttemptRowKind {
    #[serde(rename = "execution_attempt")]
    ExecutionAttempt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAttemptRow {
    pub kind: ExecutionAttemptRowKind,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub attempt: AttemptView,
    pub row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCommandMetrics {
    pub attempt_count: crate::SafeUint,
    pub started_attempt_count: crate::SafeUint,
    pub device_completed_attempt_count: crate::SafeUint,
    pub accepted_logical_work_count: crate::SafeUint,
    pub unresolved_command_count: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionCursorCursorVersion {
    #[serde(rename = "1")]
    N1,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionCursorQuerySelectionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionCursorQuerySelectionDataset {
    #[serde(rename = "execution")]
    Execution,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionCursorQuerySelectionAggregation {
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
pub struct ExecutionCursorQuerySelectionPage {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_34")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCursorQuerySelection {
    pub contract_version: ExecutionCursorQuerySelectionContractVersion,
    pub dataset: ExecutionCursorQuerySelectionDataset,
    pub identity: ExecutionIdentity,
    pub selection: SnapshotSelection,
    pub host_from_ms: crate::SafeUint,
    pub host_until_ms: crate::SafeUint,
    pub aggregation: ExecutionCursorQuerySelectionAggregation,
    pub page: ExecutionCursorQuerySelectionPage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCursor {
    pub cursor_version: ExecutionCursorCursorVersion,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub query_selection: ExecutionCursorQuerySelection,
    pub after_row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionDeviceViewFreshness {
    #[serde(rename = "fresh")]
    Fresh,
    #[serde(rename = "stale")]
    Stale,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionDeviceView {
    pub binding: CandidateBinding,
    pub operator_accept_new_work: bool,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_35")]
    pub latest_observation: Option<HostObservedDevice>,
    pub freshness: ExecutionDeviceViewFreshness,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_36")]
    pub effective_accept_new_work: Option<bool>,
    pub blocking_reasons: Vec<super::DeviceText>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_37")]
    pub registered_device_boot_id: Option<crate::DeviceBootId>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_38")]
    pub pending_observation_probe: Option<ObservationProbe>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionFrozenInputsDocumentKind {
    #[serde(rename = "execution_frozen_inputs")]
    ExecutionFrozenInputs,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionFrozenInputsContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionFrozenInputs {
    pub document_kind: ExecutionFrozenInputsDocumentKind,
    pub contract_version: ExecutionFrozenInputsContractVersion,
    pub identity: ExecutionIdentity,
    pub project_input_ref: crate::RunInputRef,
    pub project: crate::ProjectState,
    pub scenario: crate::ScenarioInput,
    pub modules: Vec<crate::ModulePackage>,
    pub execution_definitions: Vec<crate::ModuleDefinition>,
    pub scope: Vec<crate::ObjectRef>,
    pub candidate_bindings: Vec<CandidateBinding>,
    pub asset_bindings: Vec<ExecutionAssetBinding>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionHistoryCursorKind {
    #[serde(rename = "execution_history")]
    ExecutionHistory,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionHistoryCursorQuerySelectionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionHistoryCursorQuerySelectionPage {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_39")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionHistoryCursorQuerySelection {
    pub contract_version: ExecutionHistoryCursorQuerySelectionContractVersion,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub host_from_ms: crate::SafeUint,
    pub host_until_ms: crate::SafeUint,
    pub objects: Vec<ExecutionObservedObjectRef>,
    pub page: ExecutionHistoryCursorQuerySelectionPage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionHistoryCursor {
    pub kind: ExecutionHistoryCursorKind,
    pub cursor_version: CursorVersion,
    pub query_selection: ExecutionHistoryCursorQuerySelection,
    pub observation_frontier: crate::SafeUint,
    pub after_record_sequence: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionHistoryPage {
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub observation_frontier: crate::SafeUint,
    pub host_from_ms: crate::SafeUint,
    pub host_until_ms: crate::SafeUint,
    pub records: Vec<ExecutionObservationRecord>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_40")]
    pub next_cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionHistoryQueryContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionHistoryQuery {
    pub contract_version: ExecutionHistoryQueryContractVersion,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub host_from_ms: crate::SafeUint,
    pub host_until_ms: crate::SafeUint,
    pub objects: Vec<ExecutionObservedObjectRef>,
    pub page: PageSelection,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionObservationCursorKind {
    #[serde(rename = "execution_observations")]
    ExecutionObservations,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionObservationCursorQuerySelectionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationCursorQuerySelectionPage {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_41")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationCursorQuerySelection {
    pub contract_version: ExecutionObservationCursorQuerySelectionContractVersion,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub at_host_ms: crate::SafeUint,
    pub objects: Vec<ExecutionObservedObjectRef>,
    pub page: ExecutionObservationCursorQuerySelectionPage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationCursor {
    pub kind: ExecutionObservationCursorKind,
    pub cursor_version: CursorVersion,
    pub query_selection: ExecutionObservationCursorQuerySelection,
    pub observation_frontier: crate::SafeUint,
    pub after_row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionObservationHistoryDocumentDocumentKind {
    #[serde(rename = "execution_observation_history")]
    ExecutionObservationHistory,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionObservationHistoryDocumentContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationHistoryDocument {
    pub document_kind: ExecutionObservationHistoryDocumentDocumentKind,
    pub contract_version: ExecutionObservationHistoryDocumentContractVersion,
    pub identity: ExecutionIdentity,
    pub observation_frontier: crate::SafeUint,
    pub records: Vec<ExecutionObservationRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationPage {
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub observation_frontier: crate::SafeUint,
    pub at_host_ms: crate::SafeUint,
    pub clock_started: bool,
    pub objects: Vec<ExecutionObservedObject>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_42")]
    pub next_cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionObservationQueryContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationQuery {
    pub contract_version: ExecutionObservationQueryContractVersion,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub at_host_ms: crate::SafeUint,
    pub objects: Vec<ExecutionObservedObjectRef>,
    pub page: PageSelection,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationRecord {
    pub record_sequence: super::PositiveSafeUint,
    pub execution_revision: crate::SafeUint,
    pub at_host_ms: crate::SafeUint,
    pub object: ExecutionObservedObject,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ExecutionObservedObject {
    #[serde(rename = "transport")]
    Transport { data: crate::TransportState },
    #[serde(rename = "item")]
    Item { data: crate::RuntimeItem },
    #[serde(rename = "job")]
    Job { data: BoundExecutionJobSnapshot },
    #[serde(rename = "device")]
    Device { data: ExecutionDeviceView },
    #[serde(rename = "logical_work")]
    LogicalWork { data: LogicalWorkView },
    #[serde(rename = "attempt")]
    Attempt { data: AttemptView },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionObservedObjectRefKind {
    #[serde(rename = "transport")]
    Transport,
    #[serde(rename = "item")]
    Item,
    #[serde(rename = "job")]
    Job,
    #[serde(rename = "device")]
    Device,
    #[serde(rename = "logical_work")]
    LogicalWork,
    #[serde(rename = "attempt")]
    Attempt,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservedObjectRef {
    pub kind: ExecutionObservedObjectRefKind,
    pub id: crate::ObjectId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReadAccessOwnership {
    #[serde(rename = "current_host_owner")]
    CurrentHostOwner,
    #[serde(rename = "other_host")]
    OtherHost,
    #[serde(rename = "unverified")]
    Unverified,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReadAccessAllowedSessionControlsItem {
    #[serde(rename = "run")]
    Run,
    #[serde(rename = "pause_dispatch")]
    PauseDispatch,
    #[serde(rename = "resume_dispatch")]
    ResumeDispatch,
    #[serde(rename = "stop_dispatch")]
    StopDispatch,
    #[serde(rename = "request_reconciliation")]
    RequestReconciliation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReadAccess {
    pub ownership: ExecutionReadAccessOwnership,
    pub allowed_session_controls: Vec<ExecutionReadAccessAllowedSessionControlsItem>,
    pub device_acceptance_control_ids: Vec<crate::ObjectId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReadEnvelopeDeliveryReadOrigin {
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "cloud")]
    Cloud,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReadEnvelopeDeliveryRelation {
    #[serde(rename = "synced")]
    Synced,
    #[serde(rename = "publication_pending")]
    PublicationPending,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReadEnvelopeDelivery {
    pub read_origin: ExecutionReadEnvelopeDeliveryReadOrigin,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_43")]
    pub local_execution_revision: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_44")]
    pub cloud_published_revision: Option<crate::SafeUint>,
    pub relation: ExecutionReadEnvelopeDeliveryRelation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReadEnvelope {
    pub snapshot: ExecutionSnapshot,
    pub delivery: ExecutionReadEnvelopeDelivery,
    pub access: ExecutionReadAccess,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportDocumentKind {
    #[serde(rename = "execution_report")]
    ExecutionReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReport {
    pub document_kind: ExecutionReportDocumentKind,
    pub contract_version: ExecutionReportContractVersion,
    pub report_id: crate::ObjectId,
    pub project_id: crate::ProjectId,
    pub export_project_revision: crate::SafeUint,
    pub generated_at_utc: CloudTimestamp,
    #[schemars(length(min = 1, max = 2))]
    #[serde(deserialize_with = "checked_array_45")]
    pub sessions: Vec<ExecutionReportSession>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_46")]
    pub comparison: Option<ExecutionReportComparison>,
    pub conclusions: Vec<String>,
    pub limitations: Vec<crate::Issue>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_47")]
    pub prediction_comparison: Option<PredictionExecutionComparison>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportAssetRights {
    pub module_ref: crate::ModuleRef,
    pub rights: crate::AssetRights,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportComparisonStatus {
    #[serde(rename = "observational")]
    Observational,
    #[serde(rename = "side_by_side_only")]
    SideBySideOnly,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportComparison {
    pub baseline: ExecutionReportSource,
    pub candidate: ExecutionReportSource,
    pub status: ExecutionReportComparisonStatus,
    pub conditions: Vec<ExecutionReportCondition>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_48")]
    pub completion_delta_ms: Option<SignedSafeInt>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_49")]
    pub accepted_delta: Option<SignedQuantityChange>,
    pub limitations: Vec<crate::Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportConditionKey {
    #[serde(rename = "source_mode")]
    SourceMode,
    #[serde(rename = "clock_policy")]
    ClockPolicy,
    #[serde(rename = "fixed_target")]
    FixedTarget,
    #[serde(rename = "frozen_inputs")]
    FrozenInputs,
    #[serde(rename = "observation_window")]
    ObservationWindow,
    #[serde(rename = "goal_and_settlement")]
    GoalAndSettlement,
    #[serde(rename = "availability_changes")]
    AvailabilityChanges,
    #[serde(rename = "quantity_coverage")]
    QuantityCoverage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportCondition {
    pub key: ExecutionReportConditionKey,
    pub matches: bool,
    pub detail: super::DeviceText,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportMetrics {
    pub accepted: Vec<ExecutionAcceptedMetric>,
    pub target_quantity: crate::Quantity,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_50")]
    pub target_reached_at_host_ms: Option<crate::SafeUint>,
    pub commands: ExecutionCommandMetrics,
    pub waiting: Vec<ExecutionWaitingMetric>,
    pub resources: Vec<ExecutionResourceMetric>,
    pub missing_conditions: Vec<crate::Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportSession {
    pub source: ExecutionReportSourceStamp,
    pub project_title: String,
    pub scenario_title: String,
    pub module_versions: Vec<crate::ModuleRef>,
    pub asset_rights: Vec<ExecutionReportAssetRights>,
    pub snapshot: ExecutionSnapshot,
    pub history: ExecutionObservationHistoryDocument,
    pub metrics: ExecutionReportMetrics,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportSourceReadOrigin {
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "cloud")]
    Cloud,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportSource {
    pub read_origin: ExecutionReportSourceReadOrigin,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportSourceStampReadOrigin {
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "cloud")]
    Cloud,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportSourceStampDeliveryReadOrigin {
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "cloud")]
    Cloud,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionReportSourceStampDeliveryRelation {
    #[serde(rename = "synced")]
    Synced,
    #[serde(rename = "publication_pending")]
    PublicationPending,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportSourceStampDelivery {
    pub read_origin: ExecutionReportSourceStampDeliveryReadOrigin,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_51")]
    pub local_execution_revision: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_52")]
    pub cloud_published_revision: Option<crate::SafeUint>,
    pub relation: ExecutionReportSourceStampDeliveryRelation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportSourceStamp {
    pub read_origin: ExecutionReportSourceStampReadOrigin,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub observation_frontier: crate::SafeUint,
    pub delivery: ExecutionReportSourceStampDelivery,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResourceMetric {
    pub resource_id: crate::ObjectId,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_53")]
    pub confirmed_occupation_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_54")]
    pub claimed_unit_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_55")]
    pub physical_effective_work_ms: Option<crate::SafeUint>,
    pub missing_conditions: Vec<crate::Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionSnapshotContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionSnapshotProvenanceMode {
    #[serde(rename = "software_in_loop")]
    SoftwareInLoop,
    #[serde(rename = "physical_device")]
    PhysicalDevice,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionSnapshotProvenanceProducer {
    #[serde(rename = "host")]
    Host,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSnapshotProvenance {
    pub mode: ExecutionSnapshotProvenanceMode,
    pub cloud_connection_ref: crate::ObjectId,
    pub device_binding_refs: Vec<crate::ObjectId>,
    pub producer: ExecutionSnapshotProvenanceProducer,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionSnapshotLifecycleState {
    #[serde(rename = "prepared")]
    Prepared,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "dispatch_paused")]
    DispatchPaused,
    #[serde(rename = "draining")]
    Draining,
    #[serde(rename = "needs_resolution")]
    NeedsResolution,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSnapshotLifecycle {
    pub state: ExecutionSnapshotLifecycleState,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_56")]
    pub reason: Option<super::DeviceText>,
    pub new_dispatch_allowed: bool,
    pub unresolved_command_ids: Vec<crate::ObjectId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSnapshotCoverage {
    pub observed_until_host_ms: crate::SafeUint,
    pub requested_cutoff_reached: bool,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_57")]
    pub cutoff_snapshot_revision: Option<crate::SafeUint>,
    pub data_quality_issues: Vec<crate::Issue>,
    pub clock_started: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionSnapshotGoalAtSnapshot {
    #[serde(rename = "satisfied")]
    Satisfied,
    #[serde(rename = "not_satisfied")]
    NotSatisfied,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionSnapshotGoalAtCutoff {
    #[serde(rename = "satisfied")]
    Satisfied,
    #[serde(rename = "not_satisfied")]
    NotSatisfied,
    #[serde(rename = "unobserved")]
    Unobserved,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSnapshot {
    pub contract_version: ExecutionSnapshotContractVersion,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub project_input_ref: crate::RunInputRef,
    pub provenance: ExecutionSnapshotProvenance,
    pub policy: ExecutionPolicy,
    pub lifecycle: ExecutionSnapshotLifecycle,
    pub coverage: ExecutionSnapshotCoverage,
    pub goal_at_snapshot: ExecutionSnapshotGoalAtSnapshot,
    pub goal_at_cutoff: ExecutionSnapshotGoalAtCutoff,
    pub logical_work: Vec<LogicalWorkView>,
    pub pending_reservations: Vec<PendingReservation>,
    pub bound_jobs: Vec<BoundExecutionJobSnapshot>,
    pub business_facts: BusinessFacts,
    pub timings: Vec<WorkTiming>,
    pub attempts: Vec<AttemptView>,
    pub receipt_admissions: Vec<ReceiptAdmission>,
    pub devices: Vec<ExecutionDeviceView>,
    pub observation_frontier: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionTelemetryObservationContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionTelemetryObservationDeviceState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "busy")]
    Busy,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ExecutionTelemetryObservationSourcePurpose {
    #[serde(rename = "execution")]
    Execution,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionTelemetryObservation {
    pub contract_version: ExecutionTelemetryObservationContractVersion,
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub device_id: crate::ObjectId,
    pub device_boot_id: crate::DeviceBootId,
    pub device_sequence: crate::SafeUint,
    pub device_timestamp: super::DeviceText,
    pub accept_new_work: bool,
    pub device_state: ExecutionTelemetryObservationDeviceState,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_58")]
    pub active_command_id: Option<crate::DispatchCommandId>,
    pub dataset_id: crate::CloudDatasetId,
    pub source_purpose: ExecutionTelemetryObservationSourcePurpose,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_59")]
    pub observation_probe_id: Option<crate::ObservationProbeId>,
    /// Optional extension so existing saved executions retain their exact missing-measurement meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub phase_progress: Option<super::DevicePhaseProgress>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionWaitingMetric {
    pub logical_job_id: crate::ObjectId,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_60")]
    pub queued_wait_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_61")]
    pub pre_start_wait_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_62")]
    pub output_wait_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_63")]
    pub reconciliation_wait_ms: Option<crate::SafeUint>,
    pub missing_conditions: Vec<crate::Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum GeneratedLoadEventSourceMode {
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum GeneratedLoadEventMetric {
    #[serde(rename = "status_sample")]
    StatusSample,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GeneratedLoadEvent {
    pub device_ref: crate::ObjectId,
    pub device_sequence: crate::SafeUint,
    pub source_mode: GeneratedLoadEventSourceMode,
    pub observed_at: CloudTimestamp,
    pub metric: GeneratedLoadEventMetric,
    pub value: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum GeneratedLoadRowKind {
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GeneratedLoadRow {
    pub kind: GeneratedLoadRowKind,
    pub ingress_id: crate::SafeUint,
    pub dataset_id: crate::CloudDatasetId,
    pub cloud_received_at: CloudTimestamp,
    pub event: GeneratedLoadEvent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HostObservedDevice {
    pub receipt_id: crate::ObjectId,
    pub host_admission_sequence: crate::SafeUint,
    pub host_received_elapsed_ms: crate::SafeUint,
    pub observation: ExecutionTelemetryObservation,
    pub observation_probe: ObservationProbe,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ItemPick {
    pub item_id: crate::ObjectId,
    pub quantity: crate::Quantity,
    pub input_ordinal: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum LoadBatchContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum LoadBatchSourceMode {
    #[serde(rename = "generated_load")]
    GeneratedLoad,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct LoadBatch {
    pub contract_version: LoadBatchContractVersion,
    pub dataset_id: crate::CloudDatasetId,
    pub batch_id: crate::ObjectId,
    pub source_mode: LoadBatchSourceMode,
    #[schemars(length(min = 1, max = 1000))]
    #[serde(deserialize_with = "checked_array_64")]
    pub events: Vec<GeneratedLoadEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum LogicalWorkViewState {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "reserved")]
    Reserved,
    #[serde(rename = "published")]
    Published,
    #[serde(rename = "device_accepted")]
    DeviceAccepted,
    #[serde(rename = "device_started")]
    DeviceStarted,
    #[serde(rename = "device_completed")]
    DeviceCompleted,
    #[serde(rename = "business_effect_committed")]
    BusinessEffectCommitted,
    #[serde(rename = "output_wait")]
    OutputWait,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "reconciliation_required")]
    ReconciliationRequired,
    #[serde(rename = "cancelled_not_started")]
    CancelledNotStarted,
    #[serde(rename = "device_failed")]
    DeviceFailed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct LogicalWorkView {
    pub logical_job_id: crate::ObjectId,
    pub production_task_id: crate::ObjectId,
    pub route_step_id: Option<crate::ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub supply_id: Option<crate::ObjectId>,
    pub cycle: crate::SafeUint,
    pub state: LogicalWorkViewState,
    pub allowed_binding_ids: Vec<crate::ObjectId>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_65")]
    pub planned_binding_id: Option<crate::ObjectId>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_66")]
    pub current_attempt: Option<AttemptRef>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_67")]
    pub latest_dispatch_decision: Option<DispatchExplanation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ObservationProbe {
    pub probe_id: crate::ObservationProbeId,
    pub device_id: crate::ObjectId,
    pub device_boot_id: crate::DeviceBootId,
    pub issued_execution_revision: crate::SafeUint,
    pub issued_at_host_ms: crate::SafeUint,
    pub expires_at_host_ms: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PageSelection {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_68")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PendingReservation {
    pub reservation_id: crate::ObjectId,
    pub logical_job_id: crate::ObjectId,
    pub command_id: crate::DispatchCommandId,
    pub item_picks: Vec<ItemPick>,
    pub resource_units: Vec<ResourceUnits>,
    pub reserved_at_host_ms: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PredictionComparisonConditionKey {
    #[serde(rename = "project_scope")]
    ProjectScope,
    #[serde(rename = "frozen_input_content")]
    FrozenInputContent,
    #[serde(rename = "model_bindings")]
    ModelBindings,
    #[serde(rename = "fixed_target")]
    FixedTarget,
    #[serde(rename = "acceptance_scope")]
    AcceptanceScope,
    #[serde(rename = "clock_basis")]
    ClockBasis,
    #[serde(rename = "observation_window")]
    ObservationWindow,
    #[serde(rename = "prediction_goal")]
    PredictionGoal,
    #[serde(rename = "execution_goal_and_settlement")]
    ExecutionGoalAndSettlement,
    #[serde(rename = "availability_record")]
    AvailabilityRecord,
    #[serde(rename = "waiting_basis")]
    WaitingBasis,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PredictionComparisonCondition {
    pub key: PredictionComparisonConditionKey,
    pub matches: bool,
    pub detail: super::DeviceText,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PredictionExecutionLink {
    pub prediction: PredictionRunReference,
    pub execution: ExecutionReportSource,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PredictionRunReference {
    pub run_id: crate::RunId,
    pub input_ref: crate::ReportRunInputRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PredictionRunSummaryProvenance {
    #[serde(rename = "model_prediction")]
    ModelPrediction,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PredictionRunSummaryEvidenceOrigin {
    #[serde(rename = "actual_software")]
    ActualSoftware,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PredictionRunSummaryClockBasis {
    #[serde(rename = "model_virtual_ms")]
    ModelVirtualMs,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PredictionRunSummaryComputationStatus {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PredictionRunSummary {
    pub source: PredictionRunReference,
    pub provenance: PredictionRunSummaryProvenance,
    pub evidence_origin: PredictionRunSummaryEvidenceOrigin,
    pub clock_basis: PredictionRunSummaryClockBasis,
    pub time_reference: crate::TimeReference,
    pub window: crate::WindowMs,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_69")]
    pub computed_until_sim_ms: Option<crate::SafeUint>,
    pub computation_status: PredictionRunSummaryComputationStatus,
    pub whole_window_computed: bool,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_70")]
    pub goal_met: Option<bool>,
    pub scope: Vec<crate::ObjectRef>,
    pub module_versions: Vec<crate::ModuleRef>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_71")]
    pub target_quantity: Option<crate::Quantity>,
    pub accepted: Vec<ExecutionAcceptedMetric>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_72")]
    pub target_reached_at_sim_ms: Option<crate::SafeUint>,
    pub missing_conditions: Vec<crate::Issue>,
    pub waiting: Vec<PredictionWaitingMetric>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PredictionWaitingMetricBasis {
    #[serde(rename = "model_object_interval_union")]
    ModelObjectIntervalUnion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PredictionWaitingMetric {
    pub object_ref: crate::ObjectRef,
    pub covered_from_sim_ms: crate::SafeUint,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_73")]
    pub covered_until_sim_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_74")]
    pub known_sim_ms: Option<crate::SafeUint>,
    pub basis: PredictionWaitingMetricBasis,
    pub source_intervals: Vec<crate::WaitInterval>,
    pub missing_conditions: Vec<crate::Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ProjectedResourceClaimViewBoundary {
    #[serde(rename = "current_frontier")]
    CurrentFrontier,
    #[serde(rename = "report_cutoff")]
    ReportCutoff,
    #[serde(rename = "actual_release")]
    ActualRelease,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectedResourceClaim {
    pub claim: crate::ResourceClaim,
    pub view_boundary: ProjectedResourceClaimViewBoundary,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_75")]
    pub open_interval_index: Option<crate::SafeUint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum PublishAckPublicationStatus {
    #[serde(rename = "stored")]
    Stored,
    #[serde(rename = "already_stored_same_payload")]
    AlreadyStoredSamePayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PublishAck {
    pub session_id: crate::ExecutionSessionId,
    pub execution_epoch_id: crate::ExecutionEpochId,
    pub execution_revision: crate::SafeUint,
    pub publication_status: PublishAckPublicationStatus,
    pub observation_frontier: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum ReceiptAdmissionDecision {
    #[serde(rename = "applied")]
    Applied,
    #[serde(rename = "duplicate")]
    Duplicate,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "pending_dependency")]
    PendingDependency,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ReceiptAdmission {
    pub receipt_id: crate::ObjectId,
    pub decision: ReceiptAdmissionDecision,
    pub execution_revision: crate::SafeUint,
    pub host_admission_sequence: crate::SafeUint,
    pub host_received_elapsed_ms: crate::SafeUint,
    pub business_effect_committed: bool,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_76")]
    pub reason: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceUnits {
    pub resource_id: crate::ObjectId,
    pub units: super::PositiveSafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum SignedQuantityChangeDirection {
    #[serde(rename = "increase")]
    Increase,
    #[serde(rename = "decrease")]
    Decrease,
    #[serde(rename = "equal")]
    Equal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct SignedQuantityChange {
    pub direction: SignedQuantityChangeDirection,
    pub magnitude: crate::Quantity,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum SnapshotPublicationContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct SnapshotPublication {
    pub contract_version: SnapshotPublicationContractVersion,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_77")]
    pub expected_published_revision: Option<crate::SafeUint>,
    pub snapshot: ExecutionSnapshot,
    pub as_cutoff_snapshot: bool,
    pub observation_records: Vec<ExecutionObservationRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum SnapshotSelection {
    #[serde(rename = "latest")]
    Latest {},
    #[serde(rename = "revision")]
    Revision { execution_revision: crate::SafeUint },
    #[serde(rename = "cutoff")]
    Cutoff {},
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum StoredReceiptStorageStatus {
    #[serde(rename = "stored")]
    Stored,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum StoredReceiptHostAdmissionState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "not_applicable")]
    NotApplicable,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct StoredReceipt {
    pub receipt_id: crate::ObjectId,
    pub cloud_received_at: CloudTimestamp,
    pub storage_status: StoredReceiptStorageStatus,
    pub host_admission_state: StoredReceiptHostAdmissionState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryRowsCursorVersion {
    #[serde(rename = "1")]
    N1,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryRowsQuerySelectionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryRowsQuerySelectionDataset {
    #[serde(rename = "telemetry")]
    Telemetry,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryRowsQuerySelectionAggregation {
    #[serde(rename = "rows")]
    Rows,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct TelemetryCursorTelemetryRowsQuerySelectionPage {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_78")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct TelemetryCursorTelemetryRowsQuerySelection {
    pub contract_version: TelemetryCursorTelemetryRowsQuerySelectionContractVersion,
    pub dataset: TelemetryCursorTelemetryRowsQuerySelectionDataset,
    pub cloud_project_id: crate::CloudProjectId,
    pub dataset_id: crate::CloudDatasetId,
    pub device_ids: Vec<crate::ObjectId>,
    pub received_from: CloudTimestamp,
    pub received_until: CloudTimestamp,
    pub aggregation: TelemetryCursorTelemetryRowsQuerySelectionAggregation,
    pub page: TelemetryCursorTelemetryRowsQuerySelectionPage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryAggregateCursorVersion {
    #[serde(rename = "1")]
    N1,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryAggregateQuerySelectionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryAggregateQuerySelectionDataset {
    #[serde(rename = "telemetry")]
    Telemetry,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryCursorTelemetryAggregateQuerySelectionAggregation {
    #[serde(rename = "count_by_device_minute")]
    CountByDeviceMinute,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct TelemetryCursorTelemetryAggregateQuerySelectionPage {
    pub limit: PageLimit,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_79")]
    pub cursor: Option<super::DeviceText>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct TelemetryCursorTelemetryAggregateQuerySelection {
    pub contract_version: TelemetryCursorTelemetryAggregateQuerySelectionContractVersion,
    pub dataset: TelemetryCursorTelemetryAggregateQuerySelectionDataset,
    pub cloud_project_id: crate::CloudProjectId,
    pub dataset_id: crate::CloudDatasetId,
    pub device_ids: Vec<crate::ObjectId>,
    pub received_from: CloudTimestamp,
    pub received_until: CloudTimestamp,
    pub aggregation: TelemetryCursorTelemetryAggregateQuerySelectionAggregation,
    pub page: TelemetryCursorTelemetryAggregateQuerySelectionPage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum TelemetryCursor {
    #[serde(rename = "telemetry_rows")]
    TelemetryRows {
        cursor_version: TelemetryCursorTelemetryRowsCursorVersion,
        dataset_id: crate::CloudDatasetId,
        query_selection: TelemetryCursorTelemetryRowsQuerySelection,
        visible_frontier: crate::SafeUint,
        after_ingress_id: crate::SafeUint,
    },
    #[serde(rename = "telemetry_aggregate")]
    TelemetryAggregate {
        cursor_version: TelemetryCursorTelemetryAggregateCursorVersion,
        dataset_id: crate::CloudDatasetId,
        query_selection: TelemetryCursorTelemetryAggregateQuerySelection,
        visible_frontier: crate::SafeUint,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_80")]
        after_row_index: Option<crate::SafeUint>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryObservationExecutionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryObservationExecutionDeviceState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "busy")]
    Busy,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryObservationTelemetryOnlyContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryObservationTelemetryOnlyDeviceState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(tag = "source_purpose", deny_unknown_fields)]
pub enum TelemetryObservation {
    #[serde(rename = "execution")]
    Execution {
        contract_version: TelemetryObservationExecutionContractVersion,
        session_id: crate::ExecutionSessionId,
        execution_epoch_id: crate::ExecutionEpochId,
        device_id: crate::ObjectId,
        device_boot_id: crate::DeviceBootId,
        device_sequence: crate::SafeUint,
        device_timestamp: super::DeviceText,
        accept_new_work: bool,
        device_state: TelemetryObservationExecutionDeviceState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        phase_progress: Option<super::DevicePhaseProgress>,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_81")]
        active_command_id: Option<crate::DispatchCommandId>,
        dataset_id: crate::CloudDatasetId,
        #[serde(deserialize_with = "super::present_nullable")]
        #[schemars(required, schema_with = "nullable_schema_82")]
        observation_probe_id: Option<crate::ObservationProbeId>,
    },
    #[serde(rename = "telemetry_only")]
    TelemetryOnly {
        contract_version: TelemetryObservationTelemetryOnlyContractVersion,
        session_id: (),
        execution_epoch_id: (),
        device_id: crate::ObjectId,
        device_boot_id: crate::DeviceBootId,
        device_sequence: crate::SafeUint,
        device_timestamp: super::DeviceText,
        accept_new_work: AlwaysFalse,
        device_state: TelemetryObservationTelemetryOnlyDeviceState,
        active_command_id: (),
        dataset_id: crate::CloudDatasetId,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryOnlyObservationContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    SfExecution20Draft11,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryOnlyObservationDeviceState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryOnlyObservationSourcePurpose {
    #[serde(rename = "telemetry_only")]
    TelemetryOnly,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct TelemetryOnlyObservation {
    pub contract_version: TelemetryOnlyObservationContractVersion,
    pub session_id: (),
    pub execution_epoch_id: (),
    pub device_id: crate::ObjectId,
    pub device_boot_id: crate::DeviceBootId,
    pub device_sequence: crate::SafeUint,
    pub device_timestamp: super::DeviceText,
    pub accept_new_work: AlwaysFalse,
    pub device_state: TelemetryOnlyObservationDeviceState,
    pub active_command_id: (),
    pub dataset_id: crate::CloudDatasetId,
    pub source_purpose: TelemetryOnlyObservationSourcePurpose,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum TelemetryRowKind {
    #[serde(rename = "telemetry")]
    Telemetry,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRow {
    pub kind: TelemetryRowKind,
    pub ingress_id: crate::SafeUint,
    pub dataset_id: crate::CloudDatasetId,
    pub cloud_received_at: CloudTimestamp,
    pub observation: TelemetryObservation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub enum WaitingTimeRowKind {
    #[serde(rename = "waiting_time")]
    WaitingTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct WaitingTimeRow {
    pub kind: WaitingTimeRowKind,
    pub identity: ExecutionIdentity,
    pub execution_revision: crate::SafeUint,
    pub logical_job_id: crate::ObjectId,
    pub reason: super::DeviceText,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_83")]
    pub known_host_ms: Option<crate::SafeUint>,
    pub missing_conditions: Vec<crate::Issue>,
    pub row_index: crate::SafeUint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct WorkTiming {
    pub job_id: crate::ObjectId,
    pub command_id: crate::DispatchCommandId,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_84")]
    pub host_confirmed_reservation_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_85")]
    pub host_confirmed_occupation_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_86")]
    pub nominal_work_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_87")]
    pub device_reported_duration_ms: Option<crate::SafeUint>,
    #[serde(deserialize_with = "super::present_nullable")]
    #[schemars(required, schema_with = "nullable_schema_88")]
    pub physical_effective_work_ms: Option<crate::SafeUint>,
    pub missing_conditions: Vec<crate::Issue>,
}

fn nullable_schema_1(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_2(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ActiveExecutionSlotActiveSlot>>()
}
fn nullable_schema_3(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_4(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_5(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::PhaseId>>()
}
fn nullable_schema_6(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_7(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_8(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_9(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn checked_array_10<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<super::DeviceText>, D::Error> {
    let value = <Vec<super::DeviceText> as serde::Deserialize>::deserialize(deserializer)?;
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
fn nullable_schema_11(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<DatasetDescriptor>>()
}
fn checked_array_12<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<crate::ObjectId>, D::Error> {
    let value = <Vec<crate::ObjectId> as serde::Deserialize>::deserialize(deserializer)?;
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
fn nullable_schema_13(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_14(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_15(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_16(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_17(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ReceiptAdmission>>()
}
fn nullable_schema_18(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_19(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn checked_array_20<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<DaaSQueryResultTelemetryRowsItem>, D::Error> {
    let value =
        <Vec<DaaSQueryResultTelemetryRowsItem> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() > 200 {
        return Err(serde::de::Error::custom("at most 200 items allowed"));
    }
    Ok(value)
}
fn nullable_schema_21(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn checked_array_22<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<crate::ObjectId>, D::Error> {
    let value = <Vec<crate::ObjectId> as serde::Deserialize>::deserialize(deserializer)?;
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
fn nullable_schema_23(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn checked_array_24<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<DaaSQueryResultExecutionRowsItem>, D::Error> {
    let value =
        <Vec<DaaSQueryResultExecutionRowsItem> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() > 200 {
        return Err(serde::de::Error::custom("at most 200 items allowed"));
    }
    Ok(value)
}
fn nullable_schema_25(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_26(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::ExecutionSessionId>>()
}
fn nullable_schema_27(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_28(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ObservationProbe>>()
}
fn nullable_schema_29(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ObservationProbe>>()
}
fn nullable_schema_30(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_31(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::PositiveSafeUint>>()
}
fn nullable_schema_32(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::ObjectId>>()
}
fn nullable_schema_33(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::Quantity>>()
}
fn nullable_schema_34(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_35(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<HostObservedDevice>>()
}
fn nullable_schema_36(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<bool>>()
}
fn nullable_schema_37(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::DeviceBootId>>()
}
fn nullable_schema_38(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ObservationProbe>>()
}
fn nullable_schema_39(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_40(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_41(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_42(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_43(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_44(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn checked_array_45<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ExecutionReportSession>, D::Error> {
    let value = <Vec<ExecutionReportSession> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() < 1 {
        return Err(serde::de::Error::custom("at least 1 items required"));
    }
    if value.len() > 2 {
        return Err(serde::de::Error::custom("at most 2 items allowed"));
    }
    Ok(value)
}
fn nullable_schema_46(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<ExecutionReportComparison>>()
}
fn nullable_schema_47(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<PredictionExecutionComparison>>()
}
fn nullable_schema_48(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<SignedSafeInt>>()
}
fn nullable_schema_49(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<SignedQuantityChange>>()
}
fn nullable_schema_50(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_51(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_52(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_53(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_54(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_55(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_56(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_57(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_58(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::DispatchCommandId>>()
}
fn nullable_schema_59(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::ObservationProbeId>>()
}
fn nullable_schema_60(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_61(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_62(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_63(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn checked_array_64<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<GeneratedLoadEvent>, D::Error> {
    let value = <Vec<GeneratedLoadEvent> as serde::Deserialize>::deserialize(deserializer)?;
    if value.len() < 1 {
        return Err(serde::de::Error::custom("at least 1 items required"));
    }
    if value.len() > 1000 {
        return Err(serde::de::Error::custom("at most 1000 items allowed"));
    }
    Ok(value)
}
fn nullable_schema_65(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::ObjectId>>()
}
fn nullable_schema_66(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<AttemptRef>>()
}
fn nullable_schema_67(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<DispatchExplanation>>()
}
fn nullable_schema_68(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_69(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_70(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<bool>>()
}
fn nullable_schema_71(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::Quantity>>()
}
fn nullable_schema_72(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_73(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_74(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_75(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_76(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_77(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_78(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_79(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<super::DeviceText>>()
}
fn nullable_schema_80(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_81(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::DispatchCommandId>>()
}
fn nullable_schema_82(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::ObservationProbeId>>()
}
fn nullable_schema_83(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_84(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_85(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_86(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_87(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
fn nullable_schema_88(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    generator.subschema_for::<Option<crate::SafeUint>>()
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct CloudWireContractBundle {
    pub acceptance_row: Vec<AcceptanceRow>,
    pub accepted_quantity_row: Vec<AcceptedQuantityRow>,
    pub active_execution_slot: Vec<ActiveExecutionSlot>,
    pub attempt_ref: Vec<AttemptRef>,
    pub attempt_view: Vec<AttemptView>,
    pub bound_execution_job_snapshot: Vec<BoundExecutionJobSnapshot>,
    pub business_facts: Vec<BusinessFacts>,
    pub candidate_binding: Vec<CandidateBinding>,
    pub cloud_data_export_record: Vec<CloudDataExportRecord>,
    pub cloud_data_export_selection: Vec<CloudDataExportSelection>,
    pub cloud_error: Vec<CloudError>,
    pub cloud_event_row: Vec<CloudEventRow>,
    pub cloud_events_page: Vec<CloudEventsPage>,
    pub cloud_receipt_request: Vec<CloudReceiptRequest>,
    pub cloud_release_request: Vec<CloudReleaseRequest>,
    pub cloud_response: Vec<CloudResponse>,
    pub cloud_session_create: Vec<CloudSessionCreate>,
    pub cloud_timestamp: Vec<CloudTimestamp>,
    pub cloud_withdraw_request: Vec<CloudWithdrawRequest>,
    pub command: Vec<Command>,
    pub command_count_row: Vec<CommandCountRow>,
    pub command_read_envelope: Vec<CommandReadEnvelope>,
    pub command_receipts_cursor: Vec<CommandReceiptsCursor>,
    pub command_receipts_page: Vec<CommandReceiptsPage>,
    pub command_receipts_query: Vec<CommandReceiptsQuery>,
    pub daa_s_query: Vec<DaaSQuery>,
    pub daa_s_query_result: Vec<DaaSQueryResult>,
    pub dataset_descriptor: Vec<DatasetDescriptor>,
    pub device_boot: Vec<DeviceBoot>,
    pub device_boot_register_request: Vec<DeviceBootRegisterRequest>,
    pub device_boot_retire_request: Vec<DeviceBootRetireRequest>,
    pub device_minute_count_row: Vec<DeviceMinuteCountRow>,
    pub device_poll_request: Vec<DevicePollRequest>,
    pub device_poll_result: Vec<DevicePollResult>,
    pub device_receipt: Vec<DeviceReceipt>,
    pub dispatch_candidate_view: Vec<DispatchCandidateView>,
    pub dispatch_explanation: Vec<DispatchExplanation>,
    pub execution_accepted_metric: Vec<ExecutionAcceptedMetric>,
    pub execution_archive_index: Vec<ExecutionArchiveIndex>,
    pub execution_asset_binding: Vec<ExecutionAssetBinding>,
    pub execution_attempt_row: Vec<ExecutionAttemptRow>,
    pub execution_command_metrics: Vec<ExecutionCommandMetrics>,
    pub execution_cursor: Vec<ExecutionCursor>,
    pub execution_device_view: Vec<ExecutionDeviceView>,
    pub execution_frozen_inputs: Vec<ExecutionFrozenInputs>,
    pub execution_history_cursor: Vec<ExecutionHistoryCursor>,
    pub execution_history_page: Vec<ExecutionHistoryPage>,
    pub execution_history_query: Vec<ExecutionHistoryQuery>,
    pub execution_identity: Vec<ExecutionIdentity>,
    pub execution_observation_cursor: Vec<ExecutionObservationCursor>,
    pub execution_observation_history_document: Vec<ExecutionObservationHistoryDocument>,
    pub execution_observation_page: Vec<ExecutionObservationPage>,
    pub execution_observation_query: Vec<ExecutionObservationQuery>,
    pub execution_observation_record: Vec<ExecutionObservationRecord>,
    pub execution_observed_object: Vec<ExecutionObservedObject>,
    pub execution_observed_object_ref: Vec<ExecutionObservedObjectRef>,
    pub execution_policy: Vec<ExecutionPolicy>,
    pub execution_read_access: Vec<ExecutionReadAccess>,
    pub execution_read_envelope: Vec<ExecutionReadEnvelope>,
    pub execution_report: Vec<ExecutionReport>,
    pub execution_report_asset_rights: Vec<ExecutionReportAssetRights>,
    pub execution_report_comparison: Vec<ExecutionReportComparison>,
    pub execution_report_condition: Vec<ExecutionReportCondition>,
    pub execution_report_metrics: Vec<ExecutionReportMetrics>,
    pub execution_report_session: Vec<ExecutionReportSession>,
    pub execution_report_source: Vec<ExecutionReportSource>,
    pub execution_report_source_stamp: Vec<ExecutionReportSourceStamp>,
    pub execution_resource_metric: Vec<ExecutionResourceMetric>,
    pub execution_snapshot: Vec<ExecutionSnapshot>,
    pub execution_telemetry_observation: Vec<ExecutionTelemetryObservation>,
    pub execution_waiting_metric: Vec<ExecutionWaitingMetric>,
    pub generated_load_event: Vec<GeneratedLoadEvent>,
    pub generated_load_row: Vec<GeneratedLoadRow>,
    pub host_observed_device: Vec<HostObservedDevice>,
    pub item_pick: Vec<ItemPick>,
    pub load_batch: Vec<LoadBatch>,
    pub logical_work_view: Vec<LogicalWorkView>,
    pub observation_probe: Vec<ObservationProbe>,
    pub page_selection: Vec<PageSelection>,
    pub pending_reservation: Vec<PendingReservation>,
    pub port_binding: Vec<PortBinding>,
    pub prediction_comparison_condition: Vec<PredictionComparisonCondition>,
    pub prediction_execution_comparison: Vec<PredictionExecutionComparison>,
    pub prediction_execution_link: Vec<PredictionExecutionLink>,
    pub prediction_run_reference: Vec<PredictionRunReference>,
    pub prediction_run_summary: Vec<PredictionRunSummary>,
    pub prediction_waiting_metric: Vec<PredictionWaitingMetric>,
    pub projected_resource_claim: Vec<ProjectedResourceClaim>,
    pub publish_ack: Vec<PublishAck>,
    pub receipt_admission: Vec<ReceiptAdmission>,
    pub resource_binding: Vec<ResourceBinding>,
    pub resource_units: Vec<ResourceUnits>,
    pub signed_quantity_change: Vec<SignedQuantityChange>,
    pub snapshot_publication: Vec<SnapshotPublication>,
    pub snapshot_selection: Vec<SnapshotSelection>,
    pub stored_receipt: Vec<StoredReceipt>,
    pub telemetry_cursor: Vec<TelemetryCursor>,
    pub telemetry_observation: Vec<TelemetryObservation>,
    pub telemetry_only_observation: Vec<TelemetryOnlyObservation>,
    pub telemetry_row: Vec<TelemetryRow>,
    pub waiting_time_row: Vec<WaitingTimeRow>,
    pub work_timing: Vec<WorkTiming>,
}
