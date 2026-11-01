//! Generate each operation's concrete request/response schema and typed fixture proof.
use super::api::*;
use super::contract::*;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, path::Path};

fn one<T: DeserializeOwned + Serialize>(root: &Path, name: &str) {
    let raw = fs::read(root.join("api-fixtures").join(format!("{name}.json"))).unwrap();
    let typed: T = serde_json::from_slice(&raw).unwrap();
    let encoded = serde_json::to_vec_pretty(&typed).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&encoded).unwrap(),
        "API fixture changed: {name}"
    );
    fs::write(
        root.join("output/api-roundtrips")
            .join(format!("{name}.json")),
        encoded,
    )
    .unwrap();
}

fn case<
    Q: JsonSchema + DeserializeOwned + Serialize,
    R: JsonSchema + DeserializeOwned + Serialize,
>(
    root: &Path,
    name: &str,
) {
    super::schema::<Q>(&root.join("output/api-schemas"), &format!("{name}.request"));
    super::schema::<R>(
        &root.join("output/api-schemas"),
        &format!("{name}.response"),
    );
    if root
        .join("api-fixtures")
        .join(format!("{name}.request.json"))
        .exists()
    {
        one::<Q>(root, &format!("{name}.request"));
        one::<R>(root, &format!("{name}.response"));
    }
}

pub fn export(root: &Path) {
    fs::create_dir_all(root.join("output/api-schemas")).unwrap();
    fs::create_dir_all(root.join("output/api-roundtrips")).unwrap();
    case::<Request<DescribeCapabilitiesTag, DescribeCapabilitiesParams>, Response<CapabilityCatalog>>(
        root,
        "describe_capabilities",
    );
    case::<Request<GetContextTag, GetContextParams>, Response<ProjectContext>>(root, "get_context");
    case::<Request<GetObjectTag, GetObjectParams>, Response<DomainObject>>(root, "get_object");
    case::<Request<ApplyCommandsTag, ApplyCommandsParams>, Response<EditCommit>>(
        root,
        "apply_commands",
    );
    case::<Request<RenderViewTag, RenderViewParams>, Response<TaskHandle>>(root, "render_view");
    case::<Request<RunScenarioTag, RunScenarioParams>, Response<TaskHandle>>(root, "run_scenario");
    case::<Request<GetRunTag, GetRunParams>, Response<RunView>>(root, "get_run");
    case::<Request<CompareRunsTag, CompareRunsParams>, Response<RunComparison>>(
        root,
        "compare_runs",
    );
    case::<Request<ComputePackagingTag, ComputePackagingParams>, Response<TaskHandle>>(
        root,
        "compute_packaging",
    );
    case::<Request<GetTaskTag, GetTaskParams>, Response<TaskState>>(root, "get_task");
    case::<Request<CancelTaskTag, CancelTaskParams>, Response<CancellationState>>(
        root,
        "cancel_task",
    );
    case::<Request<AdoptArtifactTag, AdoptArtifactParams>, Response<BindingCommit>>(
        root,
        "adopt_artifact",
    );
    case::<Request<ImportPackageTag, ImportPackageParams>, Response<TaskHandle>>(
        root,
        "import_package",
    );
    case::<Request<SaveProjectTag, SaveProjectParams>, Response<TaskHandle>>(root, "save_project");
    case::<Request<ExportPackageTag, ExportPackageParams>, Response<TaskHandle>>(
        root,
        "export_package",
    );
    case::<Request<ExportEvaluationTag, ExportEvaluationParams>, Response<TaskHandle>>(
        root,
        "export_evaluation",
    );
    case::<
        Request<QueryModuleLibraryTag, QueryModuleLibraryParams>,
        Response<ModuleTemplateCandidates>,
    >(root, "query_module_library");
    case::<Request<CheckFactoryDraftTag, CheckFactoryDraftParams>, Response<FactoryDraftCheckResult>>(
        root,
        "check_factory_draft",
    );
    let cases: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("api-parse-cases.design.json")).unwrap())
            .unwrap();
    let mut outcomes = Vec::new();
    for case in cases["cases"].as_array().unwrap() {
        let parsed = serde_json::from_value::<ApiRequest>(case["request"].clone());
        let accepted = parsed.is_ok();
        assert_eq!(
            accepted,
            case["expected_accepted"].as_bool().unwrap(),
            "{}",
            case["id"]
        );
        outcomes.push(serde_json::json!({"id": case["id"], "accepted": accepted}));
    }
    fs::write(
        root.join("output/api-parse-proof-result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "status": "passed_actual_native_request_parse_cases",
            "cases": outcomes, "actual_product_runtime": null,
            "product_handlers_executed": false,
        }))
        .unwrap(),
    )
    .unwrap();
}
