//! Generate shared consumer artifacts from the actual product crates.
use schemars::JsonSchema;
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::Path};
use ts_rs::{Config, TS};

fn schema<T: JsonSchema>(out: &Path, name: &str) {
    fs::write(
        out.join(format!("{name}.schema.json")),
        serde_json::to_string_pretty(&schemars::schema_for!(T)).unwrap() + "\n",
    )
    .unwrap();
}

fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("generated");
    let cfg = Config::new().with_out_dir(out.join("typescript"));
    ContractTypeBundle::export_all(&cfg).unwrap();
    ApiContractBundle::export_all(&cfg).unwrap();
    let schemas = out.join("schemas");
    fs::create_dir_all(&schemas).unwrap();
    // This main-window DTO is owned by Commands. Exporting its actual Rust
    // declaration does not require linking the Native UI executable.
    if !std::env::args().any(|arg|arg=="--commands-only") { fs::write(
        out.join("../../../apps/planner/src/generated/DesktopCloseState.ts"),
        format!("// Generated from Rust DesktopCloseState; private main-window lifecycle DTO.\nexport {}\n", DesktopCloseState::decl(&Config::new())),
    ).unwrap(); }
    schema::<HostExportExecutionReportRequest>(&schemas, "export_execution_report.request");
    schema::<Response<ExportingTaskHandle>>(&schemas, "export_execution_report.response");
    schema::<HostExportCloudDataRequest>(&schemas, "export_cloud_data.request");
    schema::<Response<ExportingTaskHandle>>(&schemas, "export_cloud_data.response");
    schema::<HostCloudReadRequest>(&schemas, "read_cloud_production.request");
    schema::<Response<CloudReadTaskHandle>>(&schemas, "read_cloud_production.response");
    schema::<HostReadRequest>(&schemas, "get_execution_snapshot.request");
    schema::<HostStartRequest>(&schemas, "start_execution_session.request");
    schema::<Response<StartupTaskHandle>>(&schemas, "start_execution_session.response");
    schema::<Response<sf_domain::cloud_execution::ExecutionReadEnvelope>>(&schemas, "get_execution_snapshot.response");
    schema::<HostControlRequest>(&schemas, "control_execution_session.request");
    schema::<Response<ControlAck>>(&schemas, "control_execution_session.response");
    schema::<HostExecutionViewRequest>(&schemas, "read_execution_view.request");
    schema::<Response<ExecutionViewRead>>(&schemas, "read_execution_view.response");
    schema::<sf_domain::cloud_execution::ExecutionContractBundle>(&schemas, "execution-foundation");
    schema::<Versioned<ProjectState>>(&schemas, "project");
    schema::<Versioned<PackageManifest>>(&schemas, "manifest");
    schema::<Versioned<StoredExecutionArchive>>(&schemas, "execution-archive");
    schema::<ScenarioInput>(&schemas, "scenario");
    schema::<Versioned<ModulePackage>>(&schemas, "module");
    schema::<Request<DescribeCapabilitiesTag, DescribeCapabilitiesParams>>(
        &schemas,
        "describe_capabilities.request",
    );
    schema::<Response<CapabilityCatalog>>(&schemas, "describe_capabilities.response");
    schema::<Request<GetContextTag, GetContextParams>>(&schemas, "get_context.request");
    schema::<Response<ProjectContext>>(&schemas, "get_context.response");
    schema::<Request<GetObjectTag, GetObjectParams>>(&schemas, "get_object.request");
    schema::<Response<DomainObject>>(&schemas, "get_object.response");
    schema::<Request<ApplyCommandsTag, ApplyCommandsParams>>(&schemas, "apply_commands.request");
    schema::<Response<EditCommit>>(&schemas, "apply_commands.response");
    schema::<Request<SaveProjectTag, SaveProjectParams>>(&schemas, "save_project.request");
    schema::<Response<TaskHandle>>(&schemas, "save_project.response");
    schema::<Request<ImportPackageTag, ImportPackageParams>>(&schemas, "import_package.request");
    schema::<Response<TaskHandle>>(&schemas, "import_package.response");
    schema::<Request<GetTaskTag, GetTaskParams>>(&schemas, "get_task.request");
    schema::<Response<TaskState>>(&schemas, "get_task.response");
    schema::<Request<CancelTaskTag, CancelTaskParams>>(&schemas, "cancel_task.request");
    schema::<Response<CancellationState>>(&schemas, "cancel_task.response");
    schema::<Request<RenderViewTag, RenderViewParams>>(&schemas, "render_view.request");
    schema::<Response<TaskHandle>>(&schemas, "render_view.response");
    schema::<Request<RunScenarioTag, RunScenarioParams>>(&schemas, "run_scenario.request");
    schema::<Response<TaskHandle>>(&schemas, "run_scenario.response");
    schema::<Request<GetRunTag, GetRunParams>>(&schemas, "get_run.request");
    schema::<Response<RunView>>(&schemas, "get_run.response");
    schema::<Request<CompareRunsTag, CompareRunsParams>>(&schemas, "compare_runs.request");
    schema::<Response<RunComparison>>(&schemas, "compare_runs.response");
    schema::<Request<ComputePackagingTag, ComputePackagingParams>>(
        &schemas,
        "compute_packaging.request",
    );
    schema::<Response<TaskHandle>>(&schemas, "compute_packaging.response");
    schema::<Request<AdoptArtifactTag, AdoptArtifactParams>>(&schemas, "adopt_artifact.request");
    schema::<Response<BindingCommit>>(&schemas, "adopt_artifact.response");
    schema::<Request<ExportPackageTag, ExportPackageParams>>(&schemas, "export_package.request");
    schema::<Response<TaskHandle>>(&schemas, "export_package.response");
    schema::<Request<ExportEvaluationTag, ExportEvaluationParams>>(
        &schemas,
        "export_evaluation.request",
    );
    schema::<Response<TaskHandle>>(&schemas, "export_evaluation.response");
    schema::<Request<QueryModuleLibraryTag, QueryModuleLibraryParams>>(
        &schemas,
        "query_module_library.request",
    );
    schema::<Response<ModuleTemplateCandidates>>(&schemas, "query_module_library.response");
    schema::<Request<CheckFactoryDraftTag, CheckFactoryDraftParams>>(
        &schemas,
        "check_factory_draft.request",
    );
    schema::<Response<FactoryDraftCheckResult>>(&schemas, "check_factory_draft.response");
}
