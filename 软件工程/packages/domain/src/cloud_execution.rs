//! Pure execution wire values. Host admission, database ownership and Engine
//! transitions belong to their consumers, never to these data types.
use crate::{
    DeviceBootId, DispatchCommandId, ExecutionEpochId, ExecutionSessionId, ModuleRef, ObjectId,
    ObjectRef, PhaseId, PortId, ProjectId, Quantity, RoleId, SafeUint, UnitId,
};
use chrono::{DateTime, Datelike, SecondsFormat, Utc};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use ts_rs::TS;

#[path = "cloud_execution_wire.rs"]
mod wire;
pub use wire::*;

pub const EXECUTION_CONTRACT_VERSION: &str = "sf.execution/2.0-draft11";

/// Device-reported phase work, never inferred from the Host or nominal dispatch time.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "DevicePhaseProgressWire")]
pub struct DevicePhaseProgress {
    pub phase_id: PhaseId,
    pub effective_work_ms: SafeUint,
    pub duration_ms: SafeUint,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DevicePhaseProgressWire {
    phase_id: PhaseId,
    effective_work_ms: SafeUint,
    #[schemars(range(min = 1))]
    duration_ms: SafeUint,
}
impl TryFrom<DevicePhaseProgressWire> for DevicePhaseProgress {
    type Error = String;
    fn try_from(value: DevicePhaseProgressWire) -> Result<Self, String> {
        if value.duration_ms.value() == 0 || value.effective_work_ms.value() > value.duration_ms.value() {
            return Err("Reported phase work must be within a positive device-reported duration".into());
        }
        Ok(Self { phase_id: value.phase_id, effective_work_ms: value.effective_work_ms, duration_ms: value.duration_ms })
    }
}
const TIMESTAMP_PATTERN: &str = r"^[0-9]{4}-(0[1-9]|1[0-2])-([0-2][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](\.[0-9]{1,6})?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$";

fn present_nullable<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

fn nullable_duration_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<Option<SafeUint>>()
}

/// RFC3339 with a known offset and microsecond precision. Storage is always UTC.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(try_from = "String", into = "String")]
#[ts(type = "string")]
pub struct CloudTimestamp(DateTime<Utc>);

impl TryFrom<String> for CloudTimestamp {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let invalid = || {
            "RFC3339 timestamp with known offset, year 0001..9999 and at most 6 fractional digits required".to_string()
        };
        let b = value.as_bytes();
        if !value.is_ascii() || b.len() < 20 {
            return Err(invalid());
        }
        if b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
            || ![(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]
                .iter()
                .all(|&(a, z)| b[a..z].iter().all(u8::is_ascii_digit))
            || &value[..4] == "0000"
            || &value[17..19] >= "60"
        {
            return Err(invalid());
        }
        let zone_start = if b.last() == Some(&b'Z') {
            b.len() - 1
        } else {
            if b.len() < 25 {
                return Err(invalid());
            }
            let i = b.len() - 6;
            if !matches!(b[i], b'+' | b'-')
                || b[i + 3] != b':'
                || !b[i + 1..i + 3].iter().all(u8::is_ascii_digit)
                || !b[i + 4..].iter().all(u8::is_ascii_digit)
                || &value[i..] == "-00:00"
            {
                return Err(invalid());
            }
            i
        };
        if zone_start != 19
            && (zone_start < 21
                || zone_start > 26
                || b[19] != b'.'
                || !b[20..zone_start].iter().all(u8::is_ascii_digit))
        {
            return Err(invalid());
        }
        let parsed = DateTime::parse_from_rfc3339(&value).map_err(|_| invalid())?;
        let utc = parsed.with_timezone(&Utc);
        if !(1..=9999).contains(&utc.year()) {
            return Err(invalid());
        }
        Ok(Self(utc))
    }
}

impl CloudTimestamp {
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }
}
impl From<CloudTimestamp> for String {
    fn from(value: CloudTimestamp) -> Self {
        value.0.to_rfc3339_opts(SecondsFormat::Micros, true)
    }
}
impl JsonSchema for CloudTimestamp {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("CloudTimestamp")
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type": "string", "format": "date-time", "pattern": TIMESTAMP_PATTERN})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub enum ExecutionContractVersion {
    #[serde(rename = "sf.execution/2.0-draft11")]
    Draft11,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionIdentity {
    pub session_id: ExecutionSessionId,
    pub execution_epoch_id: ExecutionEpochId,
    pub project_id: ProjectId,
    pub host_context_session_id: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ClockMode {
    HostMonotonic,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum FinishPolicy {
    DrainAdmittedThenFinish,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(try_from = "bool", into = "bool")]
#[ts(type = "true")]
pub struct StopAtCutoff;
impl TryFrom<bool> for StopAtCutoff {
    type Error = String;
    fn try_from(value: bool) -> Result<Self, String> {
        if value {
            Ok(Self)
        } else {
            Err("stop_new_dispatch_on_cutoff must be true".into())
        }
    }
}
impl From<StopAtCutoff> for bool {
    fn from(_: StopAtCutoff) -> bool {
        true
    }
}
impl JsonSchema for StopAtCutoff {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("StopAtCutoff")
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"const": true})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(try_from = "u64", into = "u64")]
#[ts(type = "0")]
pub struct PredictionWindowStart;
impl TryFrom<u64> for PredictionWindowStart {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, String> {
        if value == 0 {
            Ok(Self)
        } else {
            Err("prediction window must start at zero".into())
        }
    }
}
impl From<PredictionWindowStart> for u64 {
    fn from(_: PredictionWindowStart) -> u64 {
        0
    }
}
impl JsonSchema for PredictionWindowStart {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("PredictionWindowStart")
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"const": 0})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SourcePredictionWindow {
    pub start_ms: PredictionWindowStart,
    pub end_ms: SafeUint,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
#[ts(type = "number")]
pub struct PositiveSafeUint(#[schemars(range(min = 1, max = 9007199254740991u64))] u64);
impl TryFrom<u64> for PositiveSafeUint {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, String> {
        if (1..=crate::MAX_SAFE_UINT).contains(&value) {
            Ok(Self(value))
        } else {
            Err("positive JavaScript safe integer required".into())
        }
    }
}
impl From<PositiveSafeUint> for u64 {
    fn from(value: PositiveSafeUint) -> u64 {
        value.0
    }
}
impl PositiveSafeUint {
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicy {
    pub clock_mode: ClockMode,
    pub report_cutoff_ms: SafeUint,
    pub stop_new_dispatch_on_cutoff: StopAtCutoff,
    pub finish_policy: FinishPolicy,
    pub target_quantity: Quantity,
    pub source_prediction_window: SourcePredictionWindow,
    pub observation_max_age_ms: PositiveSafeUint,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PortBinding {
    pub role_id: RoleId,
    pub port_id: PortId,
    pub location_id: ObjectId,
    pub item_type_id: ObjectId,
    pub unit_id: UnitId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceBinding {
    pub role_id: RoleId,
    pub resource_id: ObjectId,
    pub units: PositiveSafeUint,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CandidateBinding {
    pub binding_id: ObjectId,
    pub device_id: ObjectId,
    pub device_module_ref: ModuleRef,
    pub operation_ref: ModuleRef,
    pub execution_capacity_ref: ObjectRef,
    pub input_ports: Vec<PortBinding>,
    pub output_ports: Vec<PortBinding>,
    pub resources: Vec<ResourceBinding>,
    pub phase_id: PhaseId,
    pub nominal_work_ms: SafeUint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub setup_transition: Option<SetupTransition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub transport_supply_id: Option<ObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct SetupTransition {
    pub from_family: Option<String>,
    pub to_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct AttemptRef {
    pub logical_job_id: ObjectId,
    pub phase_id: PhaseId,
    pub attempt_ordinal: SafeUint,
    pub command_id: DispatchCommandId,
    pub selected_binding_id: ObjectId,
    pub phase_token: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CommandParameters {
    pub quantity: Quantity,
    pub phase_id: PhaseId,
    pub nominal_work_ms: SafeUint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub setup_transition: Option<SetupTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "CloudDeviceAction")]
#[schemars(rename = "CloudDeviceAction")]
pub enum DeviceAction {
    ProcessUnit,
    PrepareEquipment,
    TransportCargo,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub contract_version: ExecutionContractVersion,
    pub identity: ExecutionIdentity,
    pub attempt: AttemptRef,
    pub device_id: ObjectId,
    pub action: DeviceAction,
    pub parameters: CommandParameters,
    pub target_device_boot_id: DeviceBootId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "String", into = "String")]
#[schemars(!try_from, !into)]
#[ts(type = "string")]
pub struct DeviceText(#[schemars(length(min = 1))] String);
impl TryFrom<String> for DeviceText {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        if value.is_empty() {
            Err("nonempty device text required".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl From<DeviceText> for String {
    fn from(value: DeviceText) -> String {
        value.0
    }
}
impl DeviceText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeviceReceiptKind {
    Accepted {},
    Started {},
    Paused {},
    Resumed {},
    CancelledNotStarted {},
    CancelRejectedStarted {},
    Completed {
        #[serde(deserialize_with = "present_nullable")]
        #[schemars(required, schema_with = "nullable_duration_schema")]
        device_reported_duration_ms: Option<SafeUint>,
    },
    Failed {
        code: DeviceText,
        message: DeviceText,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeviceReceipt {
    pub session_id: ExecutionSessionId,
    pub execution_epoch_id: ExecutionEpochId,
    pub command_id: DispatchCommandId,
    pub device_id: ObjectId,
    pub phase_id: PhaseId,
    pub phase_token: ObjectId,
    pub device_boot_id: DeviceBootId,
    pub device_sequence: SafeUint,
    pub device_timestamp: DeviceText,
    pub receipt: DeviceReceiptKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ExecutionContractBundle {
    pub wire: CloudWireContractBundle,
    pub identity: ExecutionIdentity,
    pub policy: ExecutionPolicy,
    pub candidates: Vec<CandidateBinding>,
    pub command: Command,
    pub receipt: DeviceReceipt,
    pub cloud_timestamp: CloudTimestamp,
    pub cloud_project_id: crate::CloudProjectId,
    pub dataset_id: crate::CloudDatasetId,
    pub observation_probe_id: crate::ObservationProbeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum PredictionComparisonStatus {
    Observational,
    SideBySideOnly,
}

fn nullable_acceptance_scope_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<Option<crate::AcceptanceScope>>()
}
fn nullable_signed_time_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<Option<SignedSafeInt>>()
}
fn nullable_quantity_change_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<Option<SignedQuantityChange>>()
}
fn prediction_comparison_schema(schema: &mut Schema) {
    schema.insert("allOf".into(), serde_json::json!([
        {"if":{"properties":{"status":{"const":"side_by_side_only"}}},
         "then":{"properties":{"completion_delta_ms":{"type":"null"},"accepted_delta":{"type":"null"}}}},
        {"if":{"properties":{"status":{"const":"observational"}}},
         "then":{"anyOf":[{"properties":{"completion_delta_ms":{"not":{"type":"null"}}}},
                           {"properties":{"accepted_delta":{"not":{"type":"null"}}}}]}},
        {"if":{"properties":{"comparison_scope":{"type":"null"}}},
         "then":{"properties":{"accepted_delta":{"type":"null"}}}}
    ]));
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "PredictionExecutionComparisonWire", deny_unknown_fields)]
#[schemars(!try_from, transform = prediction_comparison_schema)]
pub struct PredictionExecutionComparison {
    pub prediction: PredictionRunSummary,
    pub execution: ExecutionReportSource,
    #[schemars(required, schema_with = "nullable_acceptance_scope_schema")]
    pub comparison_scope: Option<crate::AcceptanceScope>,
    pub status: PredictionComparisonStatus,
    pub conditions: Vec<PredictionComparisonCondition>,
    #[schemars(required, schema_with = "nullable_signed_time_schema")]
    pub completion_delta_ms: Option<SignedSafeInt>,
    #[schemars(required, schema_with = "nullable_quantity_change_schema")]
    pub accepted_delta: Option<SignedQuantityChange>,
    pub limitations: Vec<crate::Issue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PredictionExecutionComparisonWire {
    prediction: PredictionRunSummary,
    execution: ExecutionReportSource,
    #[serde(deserialize_with = "present_nullable")]
    comparison_scope: Option<crate::AcceptanceScope>,
    status: PredictionComparisonStatus,
    conditions: Vec<PredictionComparisonCondition>,
    #[serde(deserialize_with = "present_nullable")]
    completion_delta_ms: Option<SignedSafeInt>,
    #[serde(deserialize_with = "present_nullable")]
    accepted_delta: Option<SignedQuantityChange>,
    limitations: Vec<crate::Issue>,
}

impl TryFrom<PredictionExecutionComparisonWire> for PredictionExecutionComparison {
    type Error = String;
    fn try_from(value: PredictionExecutionComparisonWire) -> Result<Self, String> {
        let has_delta = value.completion_delta_ms.is_some() || value.accepted_delta.is_some();
        if (value.status == PredictionComparisonStatus::SideBySideOnly && has_delta)
            || (value.status == PredictionComparisonStatus::Observational && !has_delta)
            || (value.comparison_scope.is_none() && value.accepted_delta.is_some())
        {
            return Err("prediction comparison status, scope and known deltas disagree".into());
        }
        Ok(Self {
            prediction: value.prediction,
            execution: value.execution,
            comparison_scope: value.comparison_scope,
            status: value.status,
            conditions: value.conditions,
            completion_delta_ms: value.completion_delta_ms,
            accepted_delta: value.accepted_delta,
            limitations: value.limitations,
        })
    }
}
