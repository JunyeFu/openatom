use serde_json::{Value, json};
use sf_domain::cloud_execution::*;
use sf_domain::*;
use ts_rs::{Config, TS};

#[test]
fn execution_identifiers_reject_empty_and_nul_without_changing_legacy_ids() {
    macro_rules! check {
        ($($t:ty),+) => {$(
            assert!(serde_json::from_value::<$t>(json!("")).is_err());
            assert!(serde_json::from_value::<$t>(json!("bad\0id")).is_err());
            let id: $t = serde_json::from_value(json!("session-1")).unwrap();
            assert_eq!(serde_json::to_value(id).unwrap(), json!("session-1"));
        )+};
    }
    check!(
        ExecutionSessionId,
        ExecutionEpochId,
        DispatchCommandId,
        CloudProjectId,
        CloudDatasetId,
        DeviceBootId,
        ObservationProbeId,
        ProjectId
    );
}

#[test]
fn timestamp_normalizes_offset_and_keeps_microseconds() {
    let a: CloudTimestamp =
        serde_json::from_value(json!("2026-10-26T13:02:03.123456+08:00")).unwrap();
    let b: CloudTimestamp = serde_json::from_value(json!("2026-10-26T05:02:03.123456Z")).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        json!("2026-10-26T05:02:03.123456Z")
    );
    let whole: CloudTimestamp = serde_json::from_value(json!("2024-02-29T00:00:00+00:00")).unwrap();
    assert_eq!(
        serde_json::to_value(whole).unwrap(),
        json!("2024-02-29T00:00:00.000000Z")
    );
}

#[test]
fn timestamp_rejects_unknown_offset_precision_calendar_and_utc_year_overflow() {
    for input in [
        "2026-10-26T00:00:00-00:00",
        "2026-10-26T00:00:00.1234567Z",
        "2026-02-29T00:00:00Z",
        "2026-10-26T23:59:60Z",
        "2026-10-26t00:00:00z",
        "0000-01-01T00:00:00Z",
        "0001-01-01T00:00:00+01:00",
        "9999-12-31T23:59:59-01:00",
        "2026-10-26T00:00:00.Z",
        "2026-10-26T00:00:00+24:00",
        "2026-10-26T00:00:00+00:60",
        "2026-10-26 00:00:00Z",
        "2026-10-26T00:00:00",
        "短",
    ] {
        assert!(
            serde_json::from_value::<CloudTimestamp>(json!(input)).is_err(),
            "{input}"
        );
    }
}

fn receipt(kind: Value) -> Value {
    json!({"session_id":"s","execution_epoch_id":"e","command_id":"c","device_id":"d",
        "phase_id":"p","phase_token":"t","device_boot_id":"b","device_sequence":1,
        "device_timestamp":"2026-10-26T00:00:00Z","receipt":kind})
}

#[test]
fn completion_duration_is_required_nullable_and_unknown_receipt_fields_fail() {
    for duration in [Value::Null, json!(12)] {
        let input = receipt(json!({"kind":"completed","device_reported_duration_ms":duration}));
        let decoded: DeviceReceipt = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), input);
    }
    for kind in [
        json!({"kind":"completed"}),
        json!({"kind":"accepted","extra":1}),
        json!({"kind":"failed","code":"","message":"failure"}),
        json!({"kind":"not_a_receipt"}),
    ] {
        assert!(
            serde_json::from_value::<DeviceReceipt>(receipt(kind.clone())).is_err(),
            "{kind}"
        );
    }
}

#[test]
fn typed_identity_keeps_exact_fields_and_contract_constants_are_closed() {
    let input = json!({"session_id":"s","execution_epoch_id":"e","project_id":"p","host_context_session_id":"h"});
    let decoded: ExecutionIdentity = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), input);
    let mut extra = input;
    extra["grants"] = json!(["control"]);
    assert!(serde_json::from_value::<ExecutionIdentity>(extra).is_err());
    assert!(
        serde_json::from_value::<ExecutionContractVersion>(json!("sf.execution/2.0-draft10"))
            .is_err()
    );
    assert!(serde_json::from_value::<StopAtCutoff>(json!(false)).is_err());
    assert!(serde_json::from_value::<PredictionWindowStart>(json!(1)).is_err());
    assert!(serde_json::from_value::<PositiveSafeUint>(json!(0)).is_err());
    assert!(serde_json::from_value::<PositiveSafeUint>(json!(MAX_SAFE_UINT + 1)).is_err());
}

#[test]
fn generated_schema_and_types_keep_wire_constraints() {
    let schema = serde_json::to_value(schemars::schema_for!(DeviceReceiptKind)).unwrap();
    let completed = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["properties"]["kind"]["const"] == "completed")
        .unwrap();
    assert!(
        completed["required"]
            .as_array()
            .unwrap()
            .contains(&json!("device_reported_duration_ms"))
    );
    assert!(
        completed["properties"]["device_reported_duration_ms"]["anyOf"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["type"] == "null")
    );
    let ts = DeviceReceiptKind::decl(&Config::default());
    assert!(
        ts.contains("device_reported_duration_ms: SafeUint | null"),
        "{ts}"
    );
    assert!(!ts.contains("device_reported_duration_ms?:"), "{ts}");
    let timestamp = serde_json::to_value(schemars::schema_for!(CloudTimestamp)).unwrap();
    assert_eq!(timestamp["format"], json!("date-time"));
    assert_eq!(CloudTimestamp::inline(&Config::default()), "string");
}

#[test]
fn actual_typescript_consumer_preserves_closed_tags_and_required_nullability() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let out = root.join(".work/v2-implementation-20261026/type-consumer");
    std::fs::create_dir_all(&out).unwrap();
    ExecutionContractBundle::export_all(&Config::new().with_out_dir(out.join("typescript")))
        .unwrap();
    std::fs::write(
        out.join("execution-foundation.schema.json"),
        serde_json::to_string_pretty(&schemars::schema_for!(ExecutionContractBundle)).unwrap(),
    )
    .unwrap();
    let consumer = out.join("consumer.ts");
    std::fs::write(
        &consumer,
        r#"
import type { DeviceReceiptKind } from './typescript/DeviceReceiptKind';
import type { CloudTimestamp } from './typescript/CloudTimestamp';
import type { ExecutionContractVersion } from './typescript/ExecutionContractVersion';
import type { StopAtCutoff } from './typescript/StopAtCutoff';
import type { ExecutionContractBundle } from './typescript/ExecutionContractBundle';
declare const wholeContract: ExecutionContractBundle;
const allWireTypes: unknown = wholeContract.wire;
const receipt: DeviceReceiptKind = {kind:'completed', device_reported_duration_ms:null};
const timestamp: CloudTimestamp = '2026-10-26T00:00:00.123456Z';
const version: ExecutionContractVersion = 'sf.execution/2.0-draft11';
const cutoff: StopAtCutoff = true;
// @ts-expect-error Completed always carries the nullable duration field.
const missing: DeviceReceiptKind = {kind:'completed'};
// @ts-expect-error Unknown properties cannot be attached to accepted.
const extra: DeviceReceiptKind = {kind:'accepted', extra:1};
// @ts-expect-error A historical version is not the current execution contract.
const wrongVersion: ExecutionContractVersion = 'sf.execution/2.0-draft10';
// @ts-expect-error The cutoff policy does not permit automatic new dispatch.
const wrongCutoff: StopAtCutoff = false;
"#,
    )
    .unwrap();
    let result = std::process::Command::new("node")
        .arg(root.join("node_modules/typescript/bin/tsc"))
        .args([
            "--strict",
            "--noEmit",
            "--target",
            "ES2022",
            "--module",
            "ESNext",
            "--moduleResolution",
            "Bundler",
        ])
        .arg(consumer)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
