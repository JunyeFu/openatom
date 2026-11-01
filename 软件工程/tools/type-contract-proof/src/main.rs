//! Isolated compile/codegen/serialization proof. Does not implement product APIs.
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;
use ts_rs::TS;

#[path = "../../../packages/domain/types.design.rs"]
mod contract;
use contract::*;

#[path = "../../../packages/commands/api.design.rs"]
mod api;
use api::*;
mod api_export;
mod api_policy_proof;
mod fixed_pose_proof;
mod timeline_contract_proof;
mod managed_run_proof;

fn schema<T: JsonSchema>(out: &Path, name: &str) {
    fs::write(
        out.join(format!("{name}.schema.json")),
        serde_json::to_string_pretty(&schemars::schema_for!(T)).unwrap(),
    )
    .unwrap();
}

fn roundtrip<T: DeserializeOwned + Serialize>(root: &Path, name: &str) {
    let before = fs::read(root.join("fixtures").join(format!("{name}.json"))).unwrap();
    let typed: T = serde_json::from_slice(&before).unwrap();
    let after = serde_json::to_vec_pretty(&typed).unwrap();
    let first: serde_json::Value = serde_json::from_slice(&before).unwrap();
    let second: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(first, second, "typed roundtrip changed fixture {name}");
    fs::write(
        root.join("output").join(format!("{name}.roundtrip.json")),
        after,
    )
    .unwrap();
}

fn c02_consumer_handoff(root: &Path) {
    let definition_ref = "packages/packaging/consumer-device.design.json";
    let request_ref = "scenarios/九类模块采用/DRF03_C02_显式工位作业选择.request.json";
    let project = root.join("../..");
    let definition_raw = fs::read(project.join(definition_ref)).unwrap();
    let definition: Versioned<ModulePackage> = serde_json::from_slice(&definition_raw).unwrap();
    let definition_canonical = serde_json::to_value(&definition).unwrap();
    let definition_again: Versioned<ModulePackage> = serde_json::from_value(definition_canonical.clone()).unwrap();
    assert_eq!(definition_canonical, serde_json::to_value(&definition_again).unwrap());
    let request_raw = fs::read(project.join(request_ref)).unwrap();
    let request: ApiRequest = serde_json::from_slice(&request_raw).unwrap();
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&request_raw).unwrap(), serde_json::to_value(&request).unwrap());
    fs::write(root.join("output/c02-consumer-proof-result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "status": "passed_actual_canonical_Rust_parse_and_roundtrip",
        "definition_ref": definition_ref, "request_ref": request_ref,
        "source_version": "0.2.0", "legacy_0_1_0_changed": false,
        "definition_canonical": definition_canonical,
        "actual_Host_adoption": null, "actual_product_runtime": null
    })).unwrap()).unwrap();
}

fn ipc_asset_shape_proof(root: &Path) {
    let data: serde_json::Value = serde_json::from_slice(&fs::read(root.join("ipc-managed-asset-cases.design.json")).unwrap()).unwrap();
    let mut results = Vec::new();
    for row in data["cases"].as_array().unwrap() {
        let input = row["input"].clone();
        let parsed = if row["carrier"] == "response" {
            serde_json::from_value::<IpcResponseFrame<TaskState>>(input.clone()).map(|x| serde_json::to_value(x).unwrap())
        } else {
            serde_json::from_value::<IpcRequestFrame>(input.clone()).map(|x| serde_json::to_value(x).unwrap())
        };
        assert_eq!(parsed.is_ok(), row["expected_shape_accept"].as_bool().unwrap(), "IPC asset fixture {}", row["id"]);
        if let Ok(value) = parsed { assert_eq!(value, input); }
        results.push(serde_json::json!({"id":row["id"], "shape_accept":row["expected_shape_accept"]}));
    }
    fs::write(root.join("output/ipc-managed-asset-proof-result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "status":"passed_actual_canonical_native_parse_roundtrip_shapes", "cases":results,
        "actual_Host_handles_or_bytes":null, "actual_product_runtime":null
    })).unwrap()).unwrap();
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("output");
    let schemas = out.join("schemas");
    fs::create_dir_all(&schemas).unwrap();
    let cfg = ts_rs::Config::default().with_out_dir(out.join("typescript"));
    ContractTypeBundle::export_all(&cfg).unwrap();
    ApiContractBundle::export_all(&cfg).unwrap();
    schema::<ContractTypeBundle>(&schemas, "all-types");
    schema::<Versioned<PackageManifest>>(&schemas, "manifest");
    schema::<Versioned<ProjectState>>(&schemas, "project");
    schema::<Versioned<ScenarioInput>>(&schemas, "scenario");
    schema::<Versioned<ModulePackage>>(&schemas, "module");
    schema::<Versioned<TemplateSubset>>(&schemas, "template");
    schema::<Versioned<TemplateGenerationProfile>>(&schemas, "generation-profile");
    schema::<Versioned<PackageStructureJob>>(&schemas, "packaging-job");
    schema::<Versioned<PackagingResult>>(&schemas, "packaging-result");
    schema::<Versioned<ArtifactRecord>>(&schemas, "artifact");
    schema::<Versioned<RunResult>>(&schemas, "run");
    schema::<Versioned<EvaluationReport>>(&schemas, "evaluation");
    schema::<Versioned<MeasurementGroupRecord>>(&schemas, "measurement");
    schema::<ApiContractBundle>(&schemas, "api-all-types");
    schema::<ApiRequest>(&schemas, "api-request");
    api_export::export(root);
    api_policy_proof::run(root);
    fixed_pose_proof::run(root);
    timeline_contract_proof::run(root);
    c02_consumer_handoff(root);
    ipc_asset_shape_proof(root);
    managed_run_proof::run(root);
    if root.join("fixtures/manifest.json").exists() {
        roundtrip::<Versioned<PackageManifest>>(root, "manifest");
        roundtrip::<Versioned<ProjectState>>(root, "project");
        roundtrip::<Versioned<ScenarioInput>>(root, "scenario");
        roundtrip::<Versioned<TemplateSubset>>(root, "template");
        roundtrip::<Versioned<TemplateGenerationProfile>>(root, "generation-profile");
        roundtrip::<Versioned<PackageStructureJob>>(root, "packaging-job");
        roundtrip::<Versioned<PackagingResult>>(root, "packaging-result");
        roundtrip::<Versioned<ArtifactRecord>>(root, "artifact");
        roundtrip::<Versioned<RunResult>>(root, "run");
        roundtrip::<Versioned<EvaluationReport>>(root, "evaluation");
        roundtrip::<Versioned<MeasurementGroupRecord>>(root, "measurement");
        for category in [
            "items",
            "devices",
            "resources",
            "tasks",
            "operations",
            "transport",
            "templates",
            "professional",
            "media",
        ] {
            roundtrip::<Versioned<ModulePackage>>(root, &format!("module-{category}"));
        }
    }
    assert!(serde_json::from_str::<SafeUint>("9007199254740992").is_err());
    assert!(serde_json::from_str::<SafeUint>("-1").is_err());
    assert!(serde_json::from_str::<SafeUint>("1.5").is_err());
    assert!(serde_json::from_str::<ProjectId>("\"\"").is_err());
    assert!(serde_json::from_str::<DecimalPlaces>("10").is_err());
    assert!(serde_json::from_str::<RelativeFilePath>("\"../escape.json\"").is_err());
    assert!(serde_json::from_str::<SchemaVersion>("\"99.0\"").is_err());
    assert!(FiniteNumber::try_from(f64::NAN).is_err());
    assert!(serde_json::from_str::<VersionId>("\"01.2.3\"").is_err());
    assert!(serde_json::from_str::<VersionId>("\"1.2.3-01\"").is_err());
    assert!(serde_json::from_str::<VersionId>("\"1.2.3-rc.1+build\"").is_ok());
    let fixture_roundtrips = if root.join("fixtures/manifest.json").exists() {
        20
    } else {
        0
    };
    let result = serde_json::json!({
        "status": "passed_actual_native_type_compile_and_codegen",
        "source_ref": "packages/domain/types.design.rs",
        "typescript_files": fs::read_dir(out.join("typescript")).unwrap().count(),
        "schema_documents": fs::read_dir(&schemas).unwrap().count(),
        "domain_schema_documents": 13,
        "api_operation_schemas": fs::read_dir(out.join("api-schemas")).unwrap().count(),
        "api_fixture_roundtrips": fs::read_dir(out.join("api-roundtrips")).unwrap().count(),
        "fixture_roundtrips": fixture_roundtrips,
        "boundary_negative_checks": 10,
        "actual_product_runtime": null,
        "product_APIs_implemented": false
    });
    fs::write(
        out.join("type-proof-result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!("{}", result);
}
