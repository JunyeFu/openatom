use serde_json::{json, Value};
use sf_domain::cloud_execution::{DevicePhaseProgress, ExecutionTelemetryObservation, TelemetryObservation};

fn legacy() -> Value {
    json!({"contract_version":"sf.execution/2.0-draft11","session_id":"session","execution_epoch_id":"epoch","device_id":"device","device_boot_id":"boot","device_sequence":1,"device_timestamp":"private-device-clock","accept_new_work":false,"device_state":"busy","active_command_id":"command","dataset_id":"dataset","source_purpose":"execution","observation_probe_id":"probe"})
}

#[test]
fn old_telemetry_and_saved_execution_observations_keep_their_missing_measurement() {
    let value=legacy();
    let host: ExecutionTelemetryObservation=serde_json::from_value(value.clone()).unwrap();
    assert!(host.phase_progress.is_none());
    assert_eq!(serde_json::to_value(host).unwrap(),value);
    let cloud: TelemetryObservation=serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(cloud).unwrap(),value);
}

#[test]
fn measured_phase_round_trips_through_both_real_wire_consumers() {
    let mut value=legacy();value["phase_progress"]=json!({"phase_id":"cycle","effective_work_ms":750,"duration_ms":1800});
    let host:ExecutionTelemetryObservation=serde_json::from_value(value.clone()).unwrap();
    assert_eq!(host.phase_progress.as_ref().unwrap().effective_work_ms.value(),750);
    assert_eq!(serde_json::to_value(host).unwrap(),value);
    let cloud:TelemetryObservation=serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(cloud).unwrap(),value);
}

#[test]
fn invalid_duration_out_of_range_work_and_unknown_fields_are_rejected() {
    for value in [json!({"phase_id":"cycle","effective_work_ms":0,"duration_ms":0}),json!({"phase_id":"cycle","effective_work_ms":2001,"duration_ms":2000}),json!({"phase_id":"cycle","effective_work_ms":500,"duration_ms":1000,"estimated":true})] {
        assert!(serde_json::from_value::<DevicePhaseProgress>(value).is_err());
    }
}
