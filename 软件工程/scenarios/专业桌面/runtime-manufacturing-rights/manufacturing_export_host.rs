//! Explicitly registered by Root after the shared-source lease is released.
//! Actual Host/worker/ZIP consumers; no private source edits or archive re-import workaround.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use support::*;

fn project() -> ProjectId {
    "P1".to_owned().try_into().unwrap()
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn record(name: &str, value: &Value) {
    write(
        &root()
            .join("scenarios/专业桌面/runtime-manufacturing-rights")
            .join(name),
        value,
    );
}
fn scoped(h: &mut Harness, scope: &ApiScope, op: &str, params: Value) -> Value {
    h.host.dispatch(&h.ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"actual-manufacturing-export","project_id":"P1","scope":scope,"operation":op,"params":params})).unwrap())
}
fn scope_revision(h: &mut Harness, scope: &ApiScope) -> Revision {
    let context: ProjectContext = ok(scoped(h, scope, "get_context", json!({"selection":[]})));
    match scope {
        ApiScope::Project => context.project.unwrap().revision,
        ApiScope::DefinitionDraft(_) => context.definition.unwrap().revision,
        _ => unreachable!(),
    }
}
fn job(revision: u64) -> Value {
    let cases: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/production/packaging-acceptance-cases.json"
    )))
    .unwrap();
    let mut job = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "COVERAGE-restricted_folded_container")
        .unwrap()["job"]["data"]
        .clone();
    job["id"] = json!("MR-JOB");
    job["revision"] = json!(revision);
    job["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    job["conditions"]["product_envelope"]["subject"] = job["subject_ref"].clone();
    job
}
fn input() -> Value {
    let mut input = fixture();
    let declaration: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../packaging/consumer-device.design.json"
    )))
    .unwrap();
    input["definitions"][5]["parameters"] = json!(
        declaration["data"]["definition"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["id"] == "packaging_job_id" || p["id"] == "packaging_job_revision")
            .cloned()
            .collect::<Vec<_>>()
    );
    input["project"]["devices"][1]["configuration"] = json!({"packaging_job_id":{"kind":"string","value":"MR-JOB"},"packaging_job_revision":{"kind":"integer","value":1}});
    input
}
fn host_with_unknown_visual() -> Harness {
    let tree = Sandbox::new();
    let dir = tree.directory("ordinary-source");
    seed(&dir, &input());
    let right = json!({"asset_ref":"UNKNOWN-VISUAL","source":"Self-built test visual without redistribution authorization","license":null,"redistribution_confirmed":false,"missing_reason":"no permission was provided"});
    let path = "assets/unknown-visual.svg";
    fs::create_dir_all(dir.join("assets")).unwrap();
    fs::write(
        dir.join(path),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0L1 1\"/></svg>",
    )
    .unwrap();
    let mut module = read(&dir.join("module-0.json"));
    module["data"]["assets"] = json!([{"id":"UNKNOWN-VISUAL","path":path,"kind":"svg"}]);
    module["data"]["rights"] = json!([right.clone()]);
    module["data"]["definition"]["contribution"]["assets"] = json!([right.clone()]);
    write(&dir.join("module-0.json"), &module);
    let mut manifest = read(&dir.join("project.json"));
    manifest["data"]["assets"] = json!([right]);
    manifest["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(document(
            "asset",
            "asset",
            "asset",
            "UNKNOWN-VISUAL",
            path,
            Value::Null,
        ));
    write(&dir.join("project.json"), &manifest);
    Harness::from_seed(tree, &dir)
}
fn produce(h: &mut Harness, scope: &ApiScope, job: &Value) -> PackagingResult {
    let revision = scope_revision(h, scope);
    let _: EditCommit = ok(scoped(
        h,
        scope,
        "apply_commands",
        json!({"base_revision":revision,"commands":[{"kind":"put_packaging_job","data":job}]}),
    ));
    let revision = scope_revision(h, scope);
    let task: TaskHandle = ok(scoped(
        h,
        scope,
        "compute_packaging",
        json!({"packaging_job":job,"input_revision":revision,"computation_budget":job["budget"]}),
    ));
    h.finish_one();
    let state: TaskState = ok(scoped(h, scope, "get_task", json!({"task_id":task.id})));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Packaging(result)) = state.result else {
        panic!("actual result")
    };
    assert!(matches!(result.outcome, CheckOutcome::Pass));
    result
}
fn task(h: &mut Harness, scope: &ApiScope, handle: &TaskHandle) -> TaskState {
    ok(scoped(h, scope, "get_task", json!({"task_id":handle.id})))
}
fn finish(h: &mut Harness, scope: &ApiScope, handle: &TaskHandle) -> ExportPackage {
    h.finish_one();
    let state = task(h, scope, handle);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Package(output)) = state.result else {
        panic!("actual package")
    };
    output
}
fn params(
    h: &mut Harness,
    scope: &ApiScope,
    result: &PackagingResult,
    name: &str,
) -> (ExportPackageParams, PathBuf) {
    let path = h.tree.path.join(name);
    let selected = h
        .host
        .select_file(&h.ui, FilePurpose::ExportPackage, path.clone())
        .unwrap();
    let revision = scope_revision(h, scope);
    let (purpose, module_refs) = match scope {
        ApiScope::Project => ("handoff_project", json!([])),
        ApiScope::DefinitionDraft(_) => {
            let context: ProjectContext =
                ok(scoped(h, scope, "get_context", json!({"selection":[]})));
            (
                "private_module",
                json!([context.definition.unwrap().definition_ref]),
            )
        }
        _ => unreachable!(),
    };
    (serde_json::from_value(json!({"revision":revision,"selected_destination_ref":selected,"package_scope":{"purpose":purpose,"scenarios":[],"run_ids":[],"artifact_ids":[result.artifact_id],"module_refs":module_refs,"include_redistributable_assets":true}})).unwrap(),path)
}
fn declarations(result: &PackagingResult) -> Vec<ManufacturingFileDeclaration> {
    result.manufacturing_files.iter().map(|f|ManufacturingFileDeclaration{artifact_id:result.artifact_id.clone(),file:f.file.clone(),source:"This automated test's controlled synthetic geometry; explicit fixture-owner statement, not third-party permission".into(),license:"CC0-1.0".into()}).collect()
}
fn explicit(
    h: &mut Harness,
    scope: &ApiScope,
    params: ExportPackageParams,
    declarations: Vec<ManufacturingFileDeclaration>,
    confirm: bool,
) -> Result<TaskHandle, ApiError> {
    let revision = scope_revision(h, scope);
    h.host.export_manufacturing_package(
        &h.ui,
        &project(),
        scope,
        &revision,
        params,
        declarations,
        confirm,
    )
}
fn contents(path: &Path) -> (Value, std::collections::BTreeMap<String, Vec<u8>>) {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let manifest: Value =
        serde_json::from_reader(archive.by_name("project.json").unwrap()).unwrap();
    let mut files = std::collections::BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().unwrap().into_owned();
        let mut bytes = vec![];
        entry.read_to_end(&mut bytes).unwrap();
        files.insert(name, bytes);
    }
    (manifest, files)
}
fn assert_svg(bytes: &[u8], result: &PackagingResult, file: &ManufacturingFile) {
    let text = std::str::from_utf8(bytes).unwrap();
    for layer in ["cut", "fold", "glue"] {
        assert!(text.contains(&format!("id=\"{layer}\"")));
    }
    assert!(text.contains("mm\""));
    assert!(text.contains(&format!("data-part-ref=\"{}\"", file.part_id.as_str())));
    assert!(text.contains(&format!("data-job-ref=\"{}\"", result.job_ref.id.as_str())));
    assert!(text.contains(&format!(
        "data-job-revision=\"{}\"",
        result.job_ref.revision.0.value()
    )));
}
fn fresh_zip(path: &Path) -> Harness {
    let tree = Sandbox::new();
    let moved = tree.path.join("fresh-moved.zip");
    fs::copy(path, &moved).unwrap();
    let ui = id("manufacturing-fresh-ui");
    let mut h = Harness {
        host: ProjectHost::new(id("manufacturing-fresh-host"), ui.clone()),
        ui,
        tree,
    };
    let selected = h
        .host
        .select_file(&h.ui, FilePurpose::ImportPackage, moved)
        .unwrap();
    let handle: TaskHandle = ok(h.call_as(
        &h.ui.clone(),
        false,
        "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
    ));
    h.finish_one();
    let draft = h.imported(&handle);
    let _: EditCommit = ok(h.adopt(&handle, &draft, true));
    h
}
fn bind_and_run(h: &mut Harness, result: &PackagingResult) -> RunResult {
    let scope = json!([
        "forming",
        "opening",
        "linear_extraction",
        "net_space",
        "manufacturing"
    ]);
    let base = h.context().project.unwrap().revision;
    let _:EditCommit=ok(h.apply(base.0.value(),json!([{"kind":"record_artifact_acknowledgment","data":{"id":"MR-ACK","artifact_ref":result.artifact_id,"accepted_subject":{"id":"PACKAGE","revision":"r1"},"accepted_scope":scope,"decision_source":{"kind":"human_statement","data":"Explicit simulated UI purpose choice; not human acceptance"},"conditions":[],"recorded_at_iso8601":null}}])));
    let base = h.context().project.unwrap().revision;
    let _:BindingCommit=ok(h.call("adopt_artifact",json!({"artifact_id":result.artifact_id,"acceptance_ref":"MR-ACK","consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"requested_scope":scope,"base_revision":base})));
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 75000);
    assert_eq!(run.goal_met, Some(false));
    run
}

#[test]
fn actual_compute_to_private_ui_declaration_exports_exact_manufacturing_without_reimporting_or_changing_source_rights()
 {
    let scope = ApiScope::Project;
    let mut h = host_with_unknown_visual();
    let input = job(1);
    let result = produce(&mut h, &scope, &input);
    let run = bind_and_run(&mut h, &result);
    let project_before = h.object("project", "P1");
    let scenario_before = h.object("scenario", "LOCAL");
    let dirty = h.context().project.unwrap().dirty;
    let source_manifest = read(&h.tree.path.join("ordinary-source/project.json"));
    let (ordinary, ordinary_path) = params(&mut h, &scope, &result, "ordinary-before.zip");
    let queued: TaskHandle = ok(scoped(
        &mut h,
        &scope,
        "export_package",
        serde_json::to_value(ordinary).unwrap(),
    ));
    let ordinary = finish(&mut h, &scope, &queued);
    assert!(!ordinary.missing.is_empty());
    let (mut request, path) = params(&mut h, &scope, &result, "declared-complete.zip");
    request.package_scope.purpose = PackagePurpose::PrivateProject;
    request.package_scope.run_ids = vec![run.id.clone()];
    let queued = explicit(&mut h, &scope, request, declarations(&result), true).unwrap();
    assert!(matches!(queued.state, ComputationStatus::Queued));
    let output = finish(&mut h, &scope, &queued);
    let (manifest, files) = contents(&path);
    for file in &result.manufacturing_files {
        assert!(
            output
                .included_files
                .iter()
                .any(|f| serde_json::to_value(f).unwrap()
                    == serde_json::to_value(&file.file).unwrap())
        );
        assert_svg(&files[file.file.path.as_str()], &result, file);
        assert!(
            !output
                .missing
                .iter()
                .any(|i| i.field_path.as_deref() == Some(file.file.path.as_str()))
        );
    }
    assert!(!files.contains_key("assets/unknown-visual.svg"));
    assert!(
        output
            .missing
            .iter()
            .any(|i| i.field_path.as_deref() == Some("assets/unknown-visual.svg"))
    );
    assert_eq!(h.object("project", "P1"), project_before);
    assert_eq!(h.object("scenario", "LOCAL"), scenario_before);
    assert_eq!(h.context().project.unwrap().dirty, dirty);
    assert_eq!(
        read(&h.tree.path.join("ordinary-source/project.json")),
        source_manifest
    );
    let (ordinary, _) = params(&mut h, &scope, &result, "ordinary-after.zip");
    let queued: TaskHandle = ok(scoped(
        &mut h,
        &scope,
        "export_package",
        serde_json::to_value(ordinary).unwrap(),
    ));
    let after = finish(&mut h, &scope, &queued);
    for file in &result.manufacturing_files {
        assert!(
            after
                .missing
                .iter()
                .any(|i| i.field_path.as_deref() == Some(file.file.path.as_str()))
        );
    }
    let saved = h.tree.directory("source-save-after-export");
    let revision = h.context().project.unwrap().revision;
    let save = h.save(revision.0.value(), &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let saved_manifest = read(&saved.join("project.json"));
    for file in &result.manufacturing_files {
        assert_eq!(
            files[file.file.path.as_str()],
            fs::read(saved.join(file.file.path.as_str())).unwrap()
        );
        let entry = saved_manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["file_ref"]["path"] == file.file.path.as_str())
            .unwrap();
        assert_eq!(entry["role"], "supporting_file");
        assert!(
            !saved_manifest["data"]["assets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["asset_ref"] == entry["object_ref"]["id"])
        );
    }
    let mut opened = fresh_zip(&path);
    assert_eq!(
        serde_json::to_value(opened.full(&run.id)).unwrap(),
        serde_json::to_value(&run).unwrap()
    );
    let revision = scope_revision(&mut opened, &scope);
    let (_, actual, stored) = opened
        .host
        .read_stored_packaging_artifact(
            &opened.ui,
            &project(),
            &scope,
            &revision,
            &result.artifact_id,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    assert_eq!(
        serde_json::to_value(stored).unwrap(),
        serde_json::to_value(serde_json::from_value::<PackageStructureJob>(input).unwrap())
            .unwrap()
    );
    let (ordinary, _) = params(
        &mut opened,
        &scope,
        &result,
        "imported-declaration-not-current-confirmation.zip",
    );
    let queued: TaskHandle = ok(scoped(
        &mut opened,
        &scope,
        "export_package",
        serde_json::to_value(ordinary).unwrap(),
    ));
    let reexport = finish(&mut opened, &scope, &queued);
    for file in &result.manufacturing_files {
        assert!(
            reexport
                .missing
                .iter()
                .any(|i| i.field_path.as_deref() == Some(file.file.path.as_str()))
        );
    }
    fs::copy(
        path,
        root().join(
            "scenarios/专业桌面/runtime-manufacturing-rights/actual-private-manufacturing.zip",
        ),
    )
    .unwrap();
    record(
        "actual-project-export.json",
        &json!({"result":result,"run":run,"manifest":manifest,"output":output,"ordinary_before":ordinary_path.file_name().unwrap().to_string_lossy(),"ordinary_after":after,"source_manifest_unchanged":true,"normal_save_does_not_store_export_declarations":true,"import_does_not_restore_confirmation":true,"human_rights_acceptance":false}),
    );
}

#[test]
fn private_declarations_enforce_ui_scope_revision_exact_file_and_explicit_include_without_queuing_failures()
 {
    let scope = ApiScope::Project;
    let mut h = Harness::from_value(input());
    let result = produce(&mut h, &scope, &job(1));
    let (request, _) = params(&mut h, &scope, &result, "invalid-never-written.zip");
    let current = scope_revision(&mut h, &scope);
    let before = h.object("project", "P1");
    let external = id("MR-EXTERNAL");
    h.host
        .register_external_session(HostCallContext {
            allow_view_control: false,
            origin: CallerOrigin::ExternalAgent,
            session_id: external.clone(),
            granted_project: Some(project()),
            allowed_operations: vec![OperationName::GetObject, OperationName::ExportPackage],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    for session in [&external, &id("MR-UNKNOWN")] {
        let failure = h
            .host
            .export_manufacturing_package(
                session,
                &project(),
                &scope,
                &revision(0),
                request.clone(),
                declarations(&result),
                true,
            )
            .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
        assert!(failure.actual_revision.is_none());
    }
    let failure = h
        .host
        .export_manufacturing_package(
            &h.ui,
            &"OTHER".to_owned().try_into().unwrap(),
            &scope,
            &revision(0),
            request.clone(),
            declarations(&result),
            true,
        )
        .unwrap_err();
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    assert!(failure.actual_revision.is_none());
    let mut absent = declarations(&result);
    absent[0].artifact_id = "not-a-real-artifact".to_owned().try_into().unwrap();
    let stale = revision(current.0.value() - 1);
    let failure = h
        .host
        .export_manufacturing_package(
            &h.ui,
            &project(),
            &scope,
            &stale,
            request.clone(),
            absent,
            true,
        )
        .unwrap_err();
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    assert_eq!(
        failure.expected_revision.unwrap().0.value(),
        stale.0.value()
    );
    assert_eq!(
        failure.actual_revision.unwrap().0.value(),
        current.0.value()
    );
    assert!(matches!(
        explicit(
            &mut h,
            &scope,
            request.clone(),
            declarations(&result),
            false
        )
        .unwrap_err()
        .code,
        ErrorCode::PermissionDenied
    ));
    let mut no_include = request.clone();
    no_include.package_scope.include_redistributable_assets = false;
    assert!(matches!(
        explicit(&mut h, &scope, no_include, declarations(&result), true)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    ));
    let mut wrong = declarations(&result);
    wrong[0].file.path = "unrelated.svg".to_owned().try_into().unwrap();
    assert!(matches!(
        explicit(&mut h, &scope, request.clone(), wrong, true)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    let mut unknown_rights = declarations(&result);
    unknown_rights[0].license.clear();
    assert!(matches!(
        explicit(&mut h, &scope, request.clone(), unknown_rights, true)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    ));
    let mut wrong_revision = request.clone();
    wrong_revision.revision = revision(0);
    assert!(matches!(
        explicit(&mut h, &scope, wrong_revision, declarations(&result), true)
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    ));
    let mut wrong_token = request.clone();
    wrong_token.selected_destination_ref.selection_id = id("not-a-selected-file");
    assert!(matches!(
        explicit(&mut h, &scope, wrong_token, declarations(&result), true)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.object("project", "P1"), before);
}

#[test]
fn queued_export_freezes_old_revision_and_old_file_declaration_never_grants_a_new_result() {
    let scope = ApiScope::Project;
    let mut h = Harness::from_value(input());
    let first = produce(&mut h, &scope, &job(1));
    let (request, path) = params(&mut h, &scope, &first, "frozen-before-edit.zip");
    let frozen_revision = request.revision.clone();
    let queued = explicit(&mut h, &scope, request, declarations(&first), true).unwrap();
    let base = h.context().project.unwrap().revision.0.value();
    let p = h.object("project", "P1");
    let _:EditCommit=ok(h.apply(base,json!([{"kind":"set_project_settings","data":{"title":"changed after export admission","time_reference":p["time_reference"],"generation_context":p["generation_context"],"sources":p["sources"]}}])));
    finish(&mut h, &scope, &queued);
    let (manifest, files) = contents(&path);
    assert_eq!(
        manifest["data"]["project_revision"],
        serde_json::to_value(frozen_revision).unwrap()
    );
    let second = produce(&mut h, &scope, &job(2));
    assert_ne!(first.artifact_id, second.artifact_id);
    let (request, path) = params(&mut h, &scope, &second, "new-result-explicit.zip");
    let failure =
        explicit(&mut h, &scope, request.clone(), declarations(&first), true).unwrap_err();
    assert!(matches!(failure.code, ErrorCode::InvalidInput));
    let mut old_file = declarations(&second);
    old_file[0].file = first.manufacturing_files[0].file.clone();
    assert!(matches!(
        explicit(&mut h, &scope, request.clone(), old_file, true)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
    let queued = explicit(&mut h, &scope, request, declarations(&second), true).unwrap();
    let output = finish(&mut h, &scope, &queued);
    let (_, next_files) = contents(&path);
    for file in &second.manufacturing_files {
        assert_svg(&next_files[file.file.path.as_str()], &second, file);
    }
    for file in &first.manufacturing_files {
        assert_svg(&files[file.file.path.as_str()], &first, file);
    }
    record(
        "actual-revision-freeze.json",
        &json!({"first":first,"second":second,"old_manifest":manifest,"new_output":output}),
    );
}

#[test]
fn definition_scope_requires_committed_exact_version_and_cannot_declare_project_manufacturing() {
    let mut h = Harness::from_value(input());
    let project_result = produce(&mut h, &ApiScope::Project, &job(1));
    let mut definition: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../modules/devices/template-support/professional/module.json"
    )))
    .unwrap();
    definition["data"]["definition"]["id"] = json!("mr-professional-definition");
    let base = h.context().project.unwrap().revision.0.value();
    let created: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition["data"]["definition"]}]),
    ));
    let EditScope::DefinitionDraft(identity) = created.scope else {
        panic!("actual Draft")
    };
    let scope = ApiScope::DefinitionDraft(identity.id);
    let result = produce(&mut h, &scope, &job(1));
    let (request, _) = params(&mut h, &scope, &result, "uncommitted-not-exported.zip");
    assert!(matches!(
        explicit(&mut h, &scope, request, declarations(&result), true)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    ));
    assert!(h.host.drain_io_work().is_empty());
    let current = scope_revision(&mut h, &scope);
    let _: EditCommit = ok(scoped(
        &mut h,
        &scope,
        "apply_commands",
        json!({"base_revision":current,"commands":[{"kind":"commit_definition_version","data":{"version":"0.2.0"}}]}),
    ));
    let project_before = h.object("project", "P1");
    let (mut wrong, _) = params(&mut h, &scope, &result, "other-scope-not-exported.zip");
    wrong.package_scope.artifact_ids = vec![project_result.artifact_id.clone()];
    let failure = explicit(&mut h, &scope, wrong, declarations(&project_result), true).unwrap_err();
    assert!(
        matches!(failure.code, ErrorCode::PermissionDenied),
        "{failure:?}"
    );
    let (request, path) = params(&mut h, &scope, &result, "committed-definition-complete.zip");
    let queued = explicit(&mut h, &scope, request, declarations(&result), true).unwrap();
    let output = finish(&mut h, &scope, &queued);
    let (manifest, files) = contents(&path);
    for file in &result.manufacturing_files {
        assert_svg(&files[file.file.path.as_str()], &result, file);
        assert!(
            !output
                .missing
                .iter()
                .any(|i| i.field_path.as_deref() == Some(file.file.path.as_str()))
        );
    }
    for file in &project_result.manufacturing_files {
        assert!(!files.contains_key(file.file.path.as_str()));
    }
    assert_eq!(h.object("project", "P1"), project_before);
    record(
        "actual-definition-export.json",
        &json!({"manifest":manifest,"result":result,"output":output,"project_unchanged":true}),
    );
}

#[test]
fn cancel_before_worker_retains_previous_valid_package_and_does_not_store_a_source_confirmation() {
    let scope = ApiScope::Project;
    let mut h = Harness::from_value(input());
    let result = produce(&mut h, &scope, &job(1));
    let (request, path) = params(&mut h, &scope, &result, "cancelled-package.zip");
    let queued = explicit(&mut h, &scope, request, declarations(&result), true).unwrap();
    finish(&mut h, &scope, &queued);
    let original = fs::read(&path).unwrap();
    let before = h.object("project", "P1");
    let (request, _) = params(&mut h, &scope, &result, "cancelled-package.zip");
    let queued = explicit(&mut h, &scope, request, declarations(&result), true).unwrap();
    let cancelled: CancellationState = ok(scoped(
        &mut h,
        &scope,
        "cancel_task",
        json!({"task_id":queued.id,"action":"cancel"}),
    ));
    assert!(cancelled.action_accepted);
    h.finish_one();
    let cancelled = task(&mut h, &scope, &queued);
    assert!(matches!(cancelled.task.state, ComputationStatus::Cancelled));
    assert_eq!(fs::read(path).unwrap(), original);
    assert_eq!(h.object("project", "P1"), before);
    let (request, _) = params(
        &mut h,
        &scope,
        &result,
        "after-cancel-no-global-confirmation.zip",
    );
    let queued: TaskHandle = ok(scoped(
        &mut h,
        &scope,
        "export_package",
        serde_json::to_value(request).unwrap(),
    ));
    let output = finish(&mut h, &scope, &queued);
    for file in &result.manufacturing_files {
        assert!(
            output
                .missing
                .iter()
                .any(|i| i.field_path.as_deref() == Some(file.file.path.as_str()))
        );
    }
}
