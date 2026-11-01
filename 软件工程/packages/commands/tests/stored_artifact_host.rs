//! Actual Host/kernel and Save/Open checks, not Native or human acceptance.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

fn project_id() -> ProjectId {
    "P1".to_owned().try_into().unwrap()
}
fn artifact_id(value: &str) -> ArtifactId {
    value.to_owned().try_into().unwrap()
}
fn job() -> PackageStructureJob {
    let source: Value = serde_json::from_str(include_str!(
        "fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    let case = source["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "HOLD01")
        .unwrap();
    assert_eq!(case["expected"], "pass"); // Preserve the original independent expected outcome.
    serde_json::from_value(case["job"]["data"].clone()).unwrap()
}
fn scoped(h: &mut Harness, scope: &ApiScope, operation: &str, params: Value) -> Value {
    h.host.dispatch(&h.ui, serde_json::from_value(json!({"api_version":"0.1","request_id":"stored-artifact-owner","project_id":"P1","scope":scope,"operation":operation,"params":params})).unwrap())
}
fn context(h: &mut Harness, scope: &ApiScope) -> ProjectContext {
    ok(scoped(h, scope, "get_context", json!({"selection":[]})))
}
fn current_revision(h: &mut Harness, scope: &ApiScope) -> Revision {
    let context = context(h, scope);
    match scope {
        ApiScope::Project => context.project.unwrap().revision,
        ApiScope::DefinitionDraft(_) => context.definition.unwrap().revision,
        ApiScope::Workspace => unreachable!(),
    }
}
fn read(
    h: &mut Harness,
    scope: &ApiScope,
    artifact: &ArtifactId,
) -> Result<(ArtifactRecord, PackagingResult, PackageStructureJob), ApiError> {
    let revision = current_revision(h, scope);
    h.host
        .read_stored_packaging_artifact(&h.ui, &project_id(), scope, &revision, artifact)
}
fn produce(
    h: &mut Harness,
    scope: &ApiScope,
    job: &PackageStructureJob,
) -> (TaskHandle, PackagingResult) {
    let revision = current_revision(h, scope);
    let _: EditCommit = ok(scoped(
        h,
        scope,
        "apply_commands",
        json!({"base_revision":revision,"commands":[{"kind":"put_packaging_job","data":job}]}),
    ));
    let revision = current_revision(h, scope);
    let task: TaskHandle = ok(scoped(
        h,
        scope,
        "compute_packaging",
        json!({"packaging_job":job,"input_revision":revision,"computation_budget":job.budget}),
    ));
    h.finish_one();
    let state: TaskState = ok(scoped(h, scope, "get_task", json!({"task_id":task.id})));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Packaging(result)) = state.result else {
        panic!("actual packaging result")
    };
    assert!(matches!(
        result.evidence_origin,
        EvidenceOrigin::ActualSoftware
    ));
    assert!(
        matches!(result.outcome, CheckOutcome::Pass),
        "HOLD01 original Pass must stay: {result:?}"
    );
    (task, result)
}
fn draft(h: &mut Harness, name: &str) -> ApiScope {
    let mut document: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/professional/module.json"
    ))
    .unwrap();
    document["data"]["definition"]["id"] = json!(name);
    let revision = current_revision(h, &ApiScope::Project);
    let commit: EditCommit = ok(h.apply(
        revision.0.value(),
        json!([{"kind":"create_definition_draft","data":document["data"]["definition"]}]),
    ));
    let EditScope::DefinitionDraft(identity) = commit.scope else {
        panic!("actual definition scope")
    };
    ApiScope::DefinitionDraft(identity.id)
}

#[test]
fn actual_generated_old_artifact_reads_original_files_after_new_job_revision_save_move_and_reopen()
{
    let mut h = Harness::new();
    let scope = ApiScope::Project;
    let input = job();
    let (task, result) = produce(&mut h, &scope, &input);
    let original = read(&mut h, &scope, &result.artifact_id).unwrap();
    assert_eq!(
        serde_json::to_value(&original.2).unwrap(),
        serde_json::to_value(&input).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&original.1).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    let mut next = input.clone();
    next.revision = revision(input.revision.0.value() + 1);
    next.materials[0].title.push_str(" explicit later revision");
    let base = current_revision(&mut h, &scope);
    let _: EditCommit = ok(h.apply(
        base.0.value(),
        json!([{"kind":"put_packaging_job","data":next}]),
    ));
    let latest: DomainObject = ok(h.call("get_object", json!({"object_id":{"kind":"packaging_job","id":input.id},"source":{"kind":"draft"},"node_index":null})));
    let DomainObject::PackagingJob(latest) = latest else {
        panic!("actual latest job")
    };
    assert_eq!(latest.revision.0.value(), next.revision.0.value());
    let before = json!({"business":h.snapshot(),"context":h.context()});
    let exact = read(&mut h, &scope, &result.artifact_id).unwrap();
    assert_eq!(
        serde_json::to_value(&exact).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(
        json!({"business":h.snapshot(),"context":h.context()}),
        before
    );
    assert!(h.host.drain_io_work().is_empty());
    let saved = h.tree.directory("stored-professional");
    let moved = h.tree.directory("stored-relocated");
    let base = current_revision(&mut h, &scope);
    let save = h.save(base.0.value(), &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&saved, &moved);
    // A fresh application Host proves that no old Task/cache supplies these files.
    // Reopening the same Project inside one Host does not promise Task removal.
    h.host = ProjectHost::new(id("stored-reopened-host"), h.ui.clone());
    let imported = h.import(&moved);
    h.finish_one();
    let imported_draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &imported_draft, true));
    assert!(
        !h.context()
            .tasks
            .iter()
            .any(|current| current.id == task.id)
    );
    let before = json!({"business":h.snapshot(),"context":h.context()});
    let reopened = read(&mut h, &scope, &result.artifact_id).unwrap();
    assert_eq!(
        serde_json::to_value(&reopened).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(
        json!({"business":h.snapshot(),"context":h.context()}),
        before
    );
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/专业桌面/runtime-stored-artifacts");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("actual-saved-roundtrip.json"), serde_json::to_vec_pretty(&json!({"evidence_kind":"actual_Host_kernel_Save_Open_finite_test_not_Native_or_human","artifact":reopened.0,"result":reopened.1,"input":reopened.2,"current_job_revision":next.revision,"old_task_survived":false})).unwrap()).unwrap();
}

#[test]
fn actual_definition_professional_files_remain_in_their_own_scope_and_stale_revision_is_rejected() {
    let mut h = Harness::new();
    let scope = draft(&mut h, "stored.paper.a");
    let other = draft(&mut h, "stored.paper.b");
    let project_before = h.object("project", "P1");
    let (_, result) = produce(&mut h, &scope, &job());
    let before = context(&mut h, &scope);
    let value = read(&mut h, &scope, &result.artifact_id).unwrap();
    assert_eq!(value.0.id, result.artifact_id);
    assert_eq!(value.2.id, result.job_ref.id);
    assert_eq!(
        serde_json::to_value(context(&mut h, &scope)).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    assert_eq!(h.object("project", "P1"), project_before);
    assert!(matches!(
        read(&mut h, &other, &result.artifact_id).unwrap_err().code,
        ErrorCode::MissingReference
    ));
    assert!(matches!(
        read(&mut h, &ApiScope::Project, &result.artifact_id)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    let current = current_revision(&mut h, &scope);
    let stale = revision(current.0.value() - 1);
    let failure = h
        .host
        .read_stored_packaging_artifact(&h.ui, &project_id(), &scope, &stale, &result.artifact_id)
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
}

#[test]
fn actual_ui_and_project_admission_precedes_artifact_lookup_and_unknown_id_never_becomes_latest() {
    let mut h = Harness::new();
    let scope = ApiScope::Project;
    let (_, result) = produce(&mut h, &scope, &job());
    let current = current_revision(&mut h, &scope);
    let external = id("stored-external");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(project_id()),
            allowed_operations: vec![OperationName::GetObject],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    for caller in [&external, &id("unknown-ui")] {
        for selected in [&result.artifact_id, &artifact_id("not-existing")] {
            let failure = h
                .host
                .read_stored_packaging_artifact(caller, &project_id(), &scope, &current, selected)
                .unwrap_err();
            assert!(matches!(failure.code, ErrorCode::PermissionDenied));
            assert!(failure.actual_revision.is_none());
        }
    }
    let other_project: ProjectId = "wrong-project".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .read_stored_packaging_artifact(
                &h.ui,
                &other_project,
                &scope,
                &current,
                &result.artifact_id
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        read(&mut h, &scope, &artifact_id("unknown"))
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    let stale = revision(current.0.value() - 1);
    assert!(matches!(
        h.host
            .read_stored_packaging_artifact(
                &h.ui,
                &project_id(),
                &scope,
                &stale,
                &result.artifact_id
            )
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    ));
}

#[test]
fn switching_actual_project_cannot_reuse_the_previous_artifact_or_its_file_maps() {
    let mut h = Harness::new();
    let (_, result) = produce(&mut h, &ApiScope::Project, &job());
    let _:EditCommit=ok(h.call_as(&h.ui.clone(),false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"new actual project","time_reference":{"label":"explicit"},"departure":"discard_after_tasks_terminal"}}]})));
    let current: ProjectContext =
        ok(h.call_as(&h.ui.clone(), false, "get_context", json!({"selection":[]})));
    let project = current.project.unwrap();
    assert_ne!(project.id, project_id());
    assert!(matches!(
        h.host
            .read_stored_packaging_artifact(
                &h.ui,
                &project_id(),
                &ApiScope::Project,
                &project.revision,
                &result.artifact_id
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        h.host
            .read_stored_packaging_artifact(
                &h.ui,
                &project.id,
                &ApiScope::Project,
                &project.revision,
                &result.artifact_id
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
}

#[test]
fn stored_list_uses_only_the_exact_scope_records_without_loading_or_mutating_files() {
    let mut h = Harness::new();
    let a = draft(&mut h, "stored.list.a");
    let b = draft(&mut h, "stored.list.b");
    let list = |h: &mut Harness, scope: &ApiScope| {
        let revision = current_revision(h, scope);
        h.host
            .list_stored_packaging_artifacts(&h.ui, &project_id(), scope, &revision)
            .unwrap()
    };
    for scope in [&ApiScope::Project, &a, &b] {
        assert!(list(&mut h, scope).is_empty());
    }
    let (_, project_result) = produce(&mut h, &ApiScope::Project, &job());
    let (_, draft_result) = produce(&mut h, &a, &job());
    let before = json!({"business":h.snapshot(), "project":context(&mut h, &ApiScope::Project), "a":context(&mut h, &a), "b":context(&mut h, &b)});
    assert_eq!(
        list(&mut h, &ApiScope::Project),
        vec![project_result.artifact_id]
    );
    assert_eq!(list(&mut h, &a), vec![draft_result.artifact_id]);
    assert!(list(&mut h, &b).is_empty());
    assert_eq!(
        json!({"business":h.snapshot(), "project":context(&mut h, &ApiScope::Project), "a":context(&mut h, &a), "b":context(&mut h, &b)}),
        before
    );
    assert!(h.host.drain_io_work().is_empty());
    let actual = current_revision(&mut h, &a);
    let stale = revision(actual.0.value() - 1);
    let failure = h
        .host
        .list_stored_packaging_artifacts(&h.ui, &project_id(), &a, &stale)
        .unwrap_err();
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    assert_eq!(failure.actual_revision.unwrap().0.value(), actual.0.value());
    assert_eq!(
        failure.expected_revision.unwrap().0.value(),
        stale.0.value()
    );
}

#[test]
fn stored_list_admits_actual_ui_current_project_and_legal_draft_before_disclosing_records() {
    let mut h = Harness::new();
    let a = draft(&mut h, "stored.list.authorization");
    let (_, result) = produce(&mut h, &a, &job());
    let current = current_revision(&mut h, &a);
    let external = id("stored-list-external");
    let ApiScope::DefinitionDraft(draft_id) = &a else {
        unreachable!()
    };
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(project_id()),
            allowed_operations: vec![OperationName::GetObject],
            allowed_definition_drafts: vec![draft_id.clone()],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    for caller in [&external, &id("stored-list-unknown")] {
        let failure = h
            .host
            .list_stored_packaging_artifacts(caller, &project_id(), &a, &revision(999))
            .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
        assert!(failure.actual_revision.is_none());
    }
    let wrong: ProjectId = "wrong-project".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .list_stored_packaging_artifacts(&h.ui, &wrong, &a, &current)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    let missing =
        ApiScope::DefinitionDraft("stored-list-missing-draft".to_owned().try_into().unwrap());
    assert!(
        h.host
            .list_stored_packaging_artifacts(&h.ui, &project_id(), &missing, &current)
            .is_err()
    );
    assert_eq!(
        h.host
            .list_stored_packaging_artifacts(&h.ui, &project_id(), &a, &current)
            .unwrap(),
        vec![result.artifact_id]
    );
    let _:EditCommit=ok(h.call_as(&h.ui.clone(),false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"list new project","time_reference":{"label":"explicit"},"departure":"discard_after_tasks_terminal"}}]})));
    let context: ProjectContext =
        ok(h.call_as(&h.ui.clone(), false, "get_context", json!({"selection":[]})));
    let project = context.project.unwrap();
    assert!(matches!(
        h.host
            .list_stored_packaging_artifacts(&h.ui, &project_id(), &a, &current)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(
        h.host
            .list_stored_packaging_artifacts(
                &h.ui,
                &project.id,
                &ApiScope::Project,
                &project.revision
            )
            .unwrap()
            .is_empty()
    );
}
