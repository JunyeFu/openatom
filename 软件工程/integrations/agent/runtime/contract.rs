//! Exact existing public response types and operation identities; no new wire.
use serde_json::Value;
use sf_commands::*;
use sf_domain::*;
use std::io;

pub fn operation_name(operation: &OperationName) -> String {
    serde_json::to_value(operation)
        .expect("typed operation")
        .as_str()
        .expect("operation string")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cloud_read_response_keeps_the_narrow_handle_at_the_bridge_boundary() {
        let response = json!({"api_version":"0.1","request_id":"read-cloud","context":null,
            "outcome":{"status":"ok","data":{"id":"cloud-task",
                "scope":{"kind":"edit","data":{"kind":"project","data":"P"}},
                "kind":"cloud_read","state":"queued","input_revision":1,
                "progress":null,"result_ref":null,"failure":null}}});
        validate_response(&OperationName::ReadCloudProduction, &response).unwrap();
        let mut wrong = response.clone();
        wrong["outcome"]["data"]["kind"] = json!("simulation");
        assert!(validate_response(&OperationName::ReadCloudProduction, &wrong).is_err());
        for field in ["progress", "result_ref", "failure"] {
            let mut missing = response.clone();
            missing["outcome"]["data"].as_object_mut().unwrap().remove(field);
            assert!(validate_response(&OperationName::ReadCloudProduction, &missing).is_err(), "{field}");
        }
    }
}

pub fn response_schema_ref(operation: &OperationName) -> String {
    format!(
        "packages/commands/generated/schemas/{}.response.schema.json",
        operation_name(operation)
    )
}

pub fn validate_response(operation: &OperationName, value: &Value) -> io::Result<()> {
    macro_rules! typed {
        ($ty:ty) => {
            serde_json::from_value::<Response<$ty>>(value.clone())
                .map(|_| ())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        };
    }
    match operation {
        OperationName::DescribeCapabilities => typed!(CapabilityCatalog),
        OperationName::GetContext => typed!(ProjectContext),
        OperationName::ReadCloudProduction => typed!(CloudReadTaskHandle),
        OperationName::StartExecutionSession => typed!(StartupTaskHandle),
        OperationName::GetExecutionSnapshot => typed!(sf_domain::cloud_execution::ExecutionReadEnvelope),
        OperationName::ControlExecutionSession => typed!(ControlAck),
        OperationName::ApplyDeviceReceipt | OperationName::ApplyDeviceObservation => typed!(sf_domain::cloud_execution::ReceiptAdmission),
        OperationName::GetExecutionCloudWork => typed!(CloudWorkBatch),
        OperationName::AcknowledgeExecutionCloudWork => typed!(CloudEffectAck),
        OperationName::ReadExecutionView => typed!(ExecutionViewRead),
        OperationName::ExportExecutionReport | OperationName::ExportCloudData => typed!(ExportingTaskHandle),
        OperationName::GetObject => typed!(DomainObject),
        OperationName::ApplyCommands => typed!(EditCommit),
        OperationName::RenderView
        | OperationName::RunScenario
        | OperationName::ComputePackaging
        | OperationName::ImportPackage
        | OperationName::SaveProject
        | OperationName::ExportPackage
        | OperationName::ExportEvaluation => typed!(TaskHandle),
        OperationName::GetRun => typed!(RunView),
        OperationName::CompareRuns => typed!(RunComparison),
        OperationName::GetTask => typed!(TaskState),
        OperationName::CancelTask => typed!(CancellationState),
        OperationName::AdoptArtifact => typed!(BindingCommit),
        OperationName::QueryModuleLibrary => typed!(ModuleTemplateCandidates),
        OperationName::CheckFactoryDraft => typed!(FactoryDraftCheckResult),
    }
}
