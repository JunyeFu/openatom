use serde_json::Value;
use sf_domain::cloud_execution::*;
#[test]
fn pure_wire_types_match_original_design_fixture_expectations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let cases: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(root.join("packages/domain/tests/fixtures/cloud_wire_cases.json"))
            .unwrap(),
    )
    .unwrap();
    let mut mismatches = Vec::new();
    for case in cases {
        let name = case["definition"].as_str().unwrap();
        let value = &case["value"];
        let result = match name {
            "AcceptanceRow" => serde_json::from_value::<AcceptanceRow>(value.clone()).map(|_| ()),
            "AcceptedQuantityRow" => {
                serde_json::from_value::<AcceptedQuantityRow>(value.clone()).map(|_| ())
            }
            "ActiveExecutionSlot" => {
                serde_json::from_value::<ActiveExecutionSlot>(value.clone()).map(|_| ())
            }
            "AttemptRef" => serde_json::from_value::<AttemptRef>(value.clone()).map(|_| ()),
            "AttemptView" => serde_json::from_value::<AttemptView>(value.clone()).map(|_| ()),
            "BoundExecutionJobSnapshot" => {
                serde_json::from_value::<BoundExecutionJobSnapshot>(value.clone()).map(|_| ())
            }
            "BusinessFacts" => serde_json::from_value::<BusinessFacts>(value.clone()).map(|_| ()),
            "CandidateBinding" => {
                serde_json::from_value::<CandidateBinding>(value.clone()).map(|_| ())
            }
            "CloudDataExportRecord" => {
                serde_json::from_value::<CloudDataExportRecord>(value.clone()).map(|_| ())
            }
            "CloudDataExportSelection" => {
                serde_json::from_value::<CloudDataExportSelection>(value.clone()).map(|_| ())
            }
            "CloudError" => serde_json::from_value::<CloudError>(value.clone()).map(|_| ()),
            "CloudEventRow" => serde_json::from_value::<CloudEventRow>(value.clone()).map(|_| ()),
            "CloudEventsPage" => {
                serde_json::from_value::<CloudEventsPage>(value.clone()).map(|_| ())
            }
            "CloudReceiptRequest" => {
                serde_json::from_value::<CloudReceiptRequest>(value.clone()).map(|_| ())
            }
            "CloudReleaseRequest" => {
                serde_json::from_value::<CloudReleaseRequest>(value.clone()).map(|_| ())
            }
            "CloudResponse" => serde_json::from_value::<CloudResponse>(value.clone()).map(|_| ()),
            "CloudSessionCreate" => {
                serde_json::from_value::<CloudSessionCreate>(value.clone()).map(|_| ())
            }
            "CloudTimestamp" => serde_json::from_value::<CloudTimestamp>(value.clone()).map(|_| ()),
            "CloudWithdrawRequest" => {
                serde_json::from_value::<CloudWithdrawRequest>(value.clone()).map(|_| ())
            }
            "Command" => serde_json::from_value::<Command>(value.clone()).map(|_| ()),
            "CommandCountRow" => {
                serde_json::from_value::<CommandCountRow>(value.clone()).map(|_| ())
            }
            "CommandReadEnvelope" => {
                serde_json::from_value::<CommandReadEnvelope>(value.clone()).map(|_| ())
            }
            "CommandReceiptsCursor" => {
                serde_json::from_value::<CommandReceiptsCursor>(value.clone()).map(|_| ())
            }
            "CommandReceiptsPage" => {
                serde_json::from_value::<CommandReceiptsPage>(value.clone()).map(|_| ())
            }
            "CommandReceiptsQuery" => {
                serde_json::from_value::<CommandReceiptsQuery>(value.clone()).map(|_| ())
            }
            "DaaSQuery" => serde_json::from_value::<DaaSQuery>(value.clone()).map(|_| ()),
            "DaaSQueryResult" => {
                serde_json::from_value::<DaaSQueryResult>(value.clone()).map(|_| ())
            }
            "DatasetDescriptor" => {
                serde_json::from_value::<DatasetDescriptor>(value.clone()).map(|_| ())
            }
            "DeviceBoot" => serde_json::from_value::<DeviceBoot>(value.clone()).map(|_| ()),
            "DeviceBootRegisterRequest" => {
                serde_json::from_value::<DeviceBootRegisterRequest>(value.clone()).map(|_| ())
            }
            "DeviceBootRetireRequest" => {
                serde_json::from_value::<DeviceBootRetireRequest>(value.clone()).map(|_| ())
            }
            "DeviceMinuteCountRow" => {
                serde_json::from_value::<DeviceMinuteCountRow>(value.clone()).map(|_| ())
            }
            "DevicePollRequest" => {
                serde_json::from_value::<DevicePollRequest>(value.clone()).map(|_| ())
            }
            "DevicePollResult" => {
                serde_json::from_value::<DevicePollResult>(value.clone()).map(|_| ())
            }
            "DeviceReceipt" => serde_json::from_value::<DeviceReceipt>(value.clone()).map(|_| ()),
            "DispatchCandidateView" => {
                serde_json::from_value::<DispatchCandidateView>(value.clone()).map(|_| ())
            }
            "DispatchExplanation" => {
                serde_json::from_value::<DispatchExplanation>(value.clone()).map(|_| ())
            }
            "ExecutionAcceptedMetric" => {
                serde_json::from_value::<ExecutionAcceptedMetric>(value.clone()).map(|_| ())
            }
            "ExecutionArchiveIndex" => {
                serde_json::from_value::<ExecutionArchiveIndex>(value.clone()).map(|_| ())
            }
            "ExecutionAssetBinding" => {
                serde_json::from_value::<ExecutionAssetBinding>(value.clone()).map(|_| ())
            }
            "ExecutionAttemptRow" => {
                serde_json::from_value::<ExecutionAttemptRow>(value.clone()).map(|_| ())
            }
            "ExecutionCommandMetrics" => {
                serde_json::from_value::<ExecutionCommandMetrics>(value.clone()).map(|_| ())
            }
            "ExecutionCursor" => {
                serde_json::from_value::<ExecutionCursor>(value.clone()).map(|_| ())
            }
            "ExecutionDeviceView" => {
                serde_json::from_value::<ExecutionDeviceView>(value.clone()).map(|_| ())
            }
            "ExecutionFrozenInputs" => {
                serde_json::from_value::<ExecutionFrozenInputs>(value.clone()).map(|_| ())
            }
            "ExecutionHistoryCursor" => {
                serde_json::from_value::<ExecutionHistoryCursor>(value.clone()).map(|_| ())
            }
            "ExecutionHistoryPage" => {
                serde_json::from_value::<ExecutionHistoryPage>(value.clone()).map(|_| ())
            }
            "ExecutionHistoryQuery" => {
                serde_json::from_value::<ExecutionHistoryQuery>(value.clone()).map(|_| ())
            }
            "ExecutionIdentity" => {
                serde_json::from_value::<ExecutionIdentity>(value.clone()).map(|_| ())
            }
            "ExecutionObservationCursor" => {
                serde_json::from_value::<ExecutionObservationCursor>(value.clone()).map(|_| ())
            }
            "ExecutionObservationHistoryDocument" => {
                serde_json::from_value::<ExecutionObservationHistoryDocument>(value.clone())
                    .map(|_| ())
            }
            "ExecutionObservationPage" => {
                serde_json::from_value::<ExecutionObservationPage>(value.clone()).map(|_| ())
            }
            "ExecutionObservationQuery" => {
                serde_json::from_value::<ExecutionObservationQuery>(value.clone()).map(|_| ())
            }
            "ExecutionObservationRecord" => {
                serde_json::from_value::<ExecutionObservationRecord>(value.clone()).map(|_| ())
            }
            "ExecutionObservedObject" => {
                serde_json::from_value::<ExecutionObservedObject>(value.clone()).map(|_| ())
            }
            "ExecutionObservedObjectRef" => {
                serde_json::from_value::<ExecutionObservedObjectRef>(value.clone()).map(|_| ())
            }
            "ExecutionPolicy" => {
                serde_json::from_value::<ExecutionPolicy>(value.clone()).map(|_| ())
            }
            "ExecutionReadAccess" => {
                serde_json::from_value::<ExecutionReadAccess>(value.clone()).map(|_| ())
            }
            "ExecutionReadEnvelope" => {
                serde_json::from_value::<ExecutionReadEnvelope>(value.clone()).map(|_| ())
            }
            "ExecutionReport" => {
                serde_json::from_value::<ExecutionReport>(value.clone()).map(|_| ())
            }
            "ExecutionReportAssetRights" => {
                serde_json::from_value::<ExecutionReportAssetRights>(value.clone()).map(|_| ())
            }
            "ExecutionReportComparison" => {
                serde_json::from_value::<ExecutionReportComparison>(value.clone()).map(|_| ())
            }
            "ExecutionReportCondition" => {
                serde_json::from_value::<ExecutionReportCondition>(value.clone()).map(|_| ())
            }
            "ExecutionReportMetrics" => {
                serde_json::from_value::<ExecutionReportMetrics>(value.clone()).map(|_| ())
            }
            "ExecutionReportSession" => {
                serde_json::from_value::<ExecutionReportSession>(value.clone()).map(|_| ())
            }
            "ExecutionReportSource" => {
                serde_json::from_value::<ExecutionReportSource>(value.clone()).map(|_| ())
            }
            "ExecutionReportSourceStamp" => {
                serde_json::from_value::<ExecutionReportSourceStamp>(value.clone()).map(|_| ())
            }
            "ExecutionResourceMetric" => {
                serde_json::from_value::<ExecutionResourceMetric>(value.clone()).map(|_| ())
            }
            "ExecutionSnapshot" => {
                serde_json::from_value::<ExecutionSnapshot>(value.clone()).map(|_| ())
            }
            "ExecutionTelemetryObservation" => {
                serde_json::from_value::<ExecutionTelemetryObservation>(value.clone()).map(|_| ())
            }
            "ExecutionWaitingMetric" => {
                serde_json::from_value::<ExecutionWaitingMetric>(value.clone()).map(|_| ())
            }
            "GeneratedLoadEvent" => {
                serde_json::from_value::<GeneratedLoadEvent>(value.clone()).map(|_| ())
            }
            "GeneratedLoadRow" => {
                serde_json::from_value::<GeneratedLoadRow>(value.clone()).map(|_| ())
            }
            "HostObservedDevice" => {
                serde_json::from_value::<HostObservedDevice>(value.clone()).map(|_| ())
            }
            "ItemPick" => serde_json::from_value::<ItemPick>(value.clone()).map(|_| ()),
            "LoadBatch" => serde_json::from_value::<LoadBatch>(value.clone()).map(|_| ()),
            "LogicalWorkView" => {
                serde_json::from_value::<LogicalWorkView>(value.clone()).map(|_| ())
            }
            "ObservationProbe" => {
                serde_json::from_value::<ObservationProbe>(value.clone()).map(|_| ())
            }
            "PageSelection" => serde_json::from_value::<PageSelection>(value.clone()).map(|_| ()),
            "PendingReservation" => {
                serde_json::from_value::<PendingReservation>(value.clone()).map(|_| ())
            }
            "PortBinding" => serde_json::from_value::<PortBinding>(value.clone()).map(|_| ()),
            "PredictionComparisonCondition" => {
                serde_json::from_value::<PredictionComparisonCondition>(value.clone()).map(|_| ())
            }
            "PredictionExecutionComparison" => {
                serde_json::from_value::<PredictionExecutionComparison>(value.clone()).map(|_| ())
            }
            "PredictionExecutionLink" => {
                serde_json::from_value::<PredictionExecutionLink>(value.clone()).map(|_| ())
            }
            "PredictionRunReference" => {
                serde_json::from_value::<PredictionRunReference>(value.clone()).map(|_| ())
            }
            "PredictionRunSummary" => {
                serde_json::from_value::<PredictionRunSummary>(value.clone()).map(|_| ())
            }
            "PredictionWaitingMetric" => {
                serde_json::from_value::<PredictionWaitingMetric>(value.clone()).map(|_| ())
            }
            "ProjectedResourceClaim" => {
                serde_json::from_value::<ProjectedResourceClaim>(value.clone()).map(|_| ())
            }
            "PublishAck" => serde_json::from_value::<PublishAck>(value.clone()).map(|_| ()),
            "ReceiptAdmission" => {
                serde_json::from_value::<ReceiptAdmission>(value.clone()).map(|_| ())
            }
            "ResourceBinding" => {
                serde_json::from_value::<ResourceBinding>(value.clone()).map(|_| ())
            }
            "ResourceUnits" => serde_json::from_value::<ResourceUnits>(value.clone()).map(|_| ()),
            "SignedQuantityChange" => {
                serde_json::from_value::<SignedQuantityChange>(value.clone()).map(|_| ())
            }
            "SnapshotPublication" => {
                serde_json::from_value::<SnapshotPublication>(value.clone()).map(|_| ())
            }
            "SnapshotSelection" => {
                serde_json::from_value::<SnapshotSelection>(value.clone()).map(|_| ())
            }
            "StoredReceipt" => serde_json::from_value::<StoredReceipt>(value.clone()).map(|_| ()),
            "TelemetryCursor" => {
                serde_json::from_value::<TelemetryCursor>(value.clone()).map(|_| ())
            }
            "TelemetryObservation" => {
                serde_json::from_value::<TelemetryObservation>(value.clone()).map(|_| ())
            }
            "TelemetryOnlyObservation" => {
                serde_json::from_value::<TelemetryOnlyObservation>(value.clone()).map(|_| ())
            }
            "TelemetryRow" => serde_json::from_value::<TelemetryRow>(value.clone()).map(|_| ()),
            "WaitingTimeRow" => serde_json::from_value::<WaitingTimeRow>(value.clone()).map(|_| ()),
            "WorkTiming" => serde_json::from_value::<WorkTiming>(value.clone()).map(|_| ()),
            _ => panic!("unknown parity type {name}"),
        };
        let actual = result.is_ok();
        if actual != case["expected_valid"].as_bool().unwrap() {
            mismatches.push(format!(
                "{} / {} expected {} actual {} error {:?}",
                name,
                case["label"],
                case["expected_valid"],
                actual,
                result.err()
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
