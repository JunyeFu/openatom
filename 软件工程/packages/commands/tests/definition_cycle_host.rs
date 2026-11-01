//! Actual Host samples, fixed expectations are in runtime-definition-cycle.
//! Reuse the existing real-archive fixture without modifying its expectations.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use support::*;

static REQUEST: AtomicU64 = AtomicU64::new(1);

#[test]
fn definition_context_reopens_current_history_and_commit_boundary_without_mutation() {
    let mut h = Harness::new();
    let mut definition = fixture()["definitions"][0].clone();
    let draft = create(&mut h, definition.clone());
    let read = |h: &mut Harness| -> Value {
        let context: ProjectContext = ok(call(h, &draft, "get_context", json!({"selection":[]})));
        serde_json::to_value(context).unwrap()["definition"].clone()
    };
    assert_eq!(read(&mut h)["history"]["can_undo"], false);
    definition["title"] = json!("History reopen test");
    let _: EditCommit = ok(call(&mut h, &draft, "apply_commands", json!({"base_revision":1,"commands":[{"kind":"put_definition","data":definition}]})));
    let summary = read(&mut h);
    assert_eq!(summary["revision"], 2);
    assert_eq!(summary["history"]["can_undo"], true);
    assert_eq!(read(&mut h), summary, "reading history must not change the draft");
    let _: EditCommit = ok(call(&mut h, &draft, "apply_commands", json!({"base_revision":2,"commands":[{"kind":"undo"}]})));
    let summary = read(&mut h);
    assert_eq!(summary["revision"], 3);
    assert_eq!(summary["history"]["can_undo"], false);
    assert_eq!(summary["history"]["can_redo"], true);
    let _: EditCommit = ok(call(&mut h, &draft, "apply_commands", json!({"base_revision":3,"commands":[{"kind":"redo"}]})));
    let _: EditCommit = ok(call(&mut h, &draft, "apply_commands", json!({"base_revision":4,"commands":[{"kind":"commit_definition_version","data":{"version":"0.1.1"}}]})));
    let summary = read(&mut h);
    assert_eq!(summary["revision"], 5);
    assert_eq!(summary["history"]["can_undo"], false);
    assert_eq!(summary["history"]["can_redo"], false);
    assert!(summary["history"]["boundary_reason"].as_str().unwrap().contains("history boundary"));
}
fn scoped(
    h: &mut Harness,
    session: &ObjectId,
    draft: &ModuleId,
    operation: &str,
    params: Value,
) -> Value {
    let request: ApiRequest = serde_json::from_value(json!({"api_version":"0.1","request_id":format!("p301-cycle-{}",REQUEST.fetch_add(1,Ordering::Relaxed)),"project_id":"P1","scope":{"kind":"definition_draft","data":draft},"operation":operation,"params":params})).unwrap();
    h.host.dispatch(session, request)
}
fn call(h: &mut Harness, draft: &ModuleId, operation: &str, params: Value) -> Value {
    scoped(h, &h.ui.clone(), draft, operation, params)
}
fn create(h: &mut Harness, definition: Value) -> ModuleId {
    let base = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(identity) = commit.scope else {
        panic!("actual definition scope")
    };
    identity.id
}
fn sample_params(scenario: &Value, revision: u64) -> Value {
    json!({"scenario_id":scenario["id"],"input_revision":revision,"observation_window":scenario["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":revision,"test_input":scenario}}})
}
fn sample(h: &mut Harness, draft: &ModuleId, scenario: &Value, revision: u64) -> TaskHandle {
    ok(call(
        h,
        draft,
        "run_scenario",
        sample_params(scenario, revision),
    ))
}
fn task(h: &mut Harness, draft: &ModuleId, handle: &TaskHandle) -> TaskState {
    ok(call(h, draft, "get_task", json!({"task_id":handle.id})))
}
fn full(h: &mut Harness, draft: &ModuleId, run: &RunId) -> RunResult {
    let view: RunView = ok(call(
        h,
        draft,
        "get_run",
        json!({"run_id":run,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(result) = view else {
        panic!("actual full result")
    };
    *result
}
fn complete(h: &mut Harness, draft: &ModuleId, handle: &TaskHandle) -> RunResult {
    h.finish_one();
    let state = task(h, draft, handle);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Run(summary)) = state.result else {
        panic!("actual Run output")
    };
    full(h, draft, &summary.run_id)
}
fn business(h: &mut Harness) -> Value {
    let project = h.object("project", "P1");
    let scenarios = project["scenario_refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| {
            (
                name.as_str().unwrap().to_owned(),
                h.object("scenario", name.as_str().unwrap()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let context = h.context();
    json!({"project":project,"scenarios":scenarios,"dirty":context.project.unwrap().dirty,"formal_runs":context.run_summaries})
}
fn accepted(result: &RunResult) -> Vec<u64> {
    result
        .acceptance_events
        .iter()
        .map(|event| event.at_ms.0.value())
        .collect()
}
fn frozen_object(h: &mut Harness, draft: &ModuleId, run: &RunId, kind: &str, id: &str) -> Value {
    let object: DomainObject = ok(call(
        h,
        draft,
        "get_object",
        json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"run_snapshot","data":{"run_id":run,"at_ms":0}},"node_index":null}),
    ));
    serde_json::to_value(object).unwrap()["data"].clone()
}

#[test]
fn actual_operation_sample_uses_explicit_draft_and_preserves_formal_run_dirty_history_and_files() {
    let mut h = Harness::new();
    let baseline = h.run("LOCAL");
    h.finish_one();
    let baseline = h.completed_run(&baseline);
    assert_eq!(accepted(&baseline), vec![75000]);
    let save_dir = h.tree.directory("formal-before-sample");
    let selection = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, save_dir.clone())
        .unwrap();
    let _: TaskHandle = ok(h.call(
        "save_project",
        json!({"revision":1,"selected_directory_ref":selection}),
    ));
    h.finish_one();
    let saved_manifest = std::fs::read(save_dir.join("project.json")).unwrap();
    let mut definition = fixture()["definitions"][0].clone();
    definition["payload"]["data"]["duration"]["value"] = json!(30000);
    let definition =
        serde_json::to_value(serde_json::from_value::<ModuleDefinition>(definition).unwrap())
            .unwrap();
    let draft = create(&mut h, definition.clone());
    let before = business(&mut h);
    let scenario = h.object("scenario", "LOCAL");
    let handle = sample(&mut h, &draft, &scenario, 1);
    assert!(matches!(
        handle.scope,
        TaskScope::Edit(EditScope::DefinitionDraft(_))
    ));
    let result = complete(&mut h, &draft, &handle);
    assert_eq!(accepted(&result), vec![45000]);
    assert!(matches!(
        result.evidence_origin,
        EvidenceOrigin::ActualSoftware
    ));
    assert!(
        result
            .limitations
            .iter()
            .any(|i| i.code == "definition_cycle.frozen_draft_source"
                && i.object_ref.as_ref().unwrap().id.as_str() == draft.as_str()
                && i.message.contains("revision 1"))
    );
    assert_eq!(
        result.limitations.iter().filter(|i| i.code == "definition_cycle.frozen_draft_source").count(),
        1,
        "one frozen-draft source is one limitation, including after full Run readback"
    );
    assert_eq!(business(&mut h), before);
    assert_eq!(
        serde_json::to_value(h.full(&baseline.id)).unwrap(),
        serde_json::to_value(&baseline).unwrap()
    );
    assert_eq!(
        std::fs::read(save_dir.join("project.json")).unwrap(),
        saved_manifest
    );
    let mut changed = definition.clone();
    changed["payload"]["data"]["duration"]["value"] = json!(90000);
    let current: EditCommit = ok(call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_definition","data":changed}]}),
    ));
    assert_eq!(current.new_revision.0.value(), 2);
    assert_eq!(
        frozen_object(
            &mut h,
            &draft,
            &result.id,
            "definition_draft",
            draft.as_str()
        ),
        definition
    );
    let module: DomainObject = ok(call(
        &mut h,
        &draft,
        "get_object",
        json!({"object_id":{"kind":"module","id":"fixture.assemble"},"source":{"kind":"run_module_version","data":{"run_id":result.id,"at_ms":0,"module_ref":{"id":"fixture.assemble","version":"0.1.0"}}},"node_index":null}),
    ));
    let DomainObject::Module(package) = module else {
        panic!("actual frozen package")
    };
    assert_eq!(
        serde_json::to_value(package.definition).unwrap(),
        definition
    );
    assert!(
        package
            .issues
            .iter()
            .any(|issue| issue.code == "definition_cycle.frozen_draft_source"
                && issue.message.contains("revision 1"))
    );
    assert_eq!(accepted(&full(&mut h, &draft, &result.id)), vec![45000]);
    assert_eq!(business(&mut h), before);
    let late: CancellationState = ok(call(
        &mut h,
        &draft,
        "cancel_task",
        json!({"task_id":handle.id,"action":"cancel"}),
    ));
    assert!(late.terminal_won && !late.action_accepted);
    let after_dir = h.tree.directory("formal-after-sample");
    let _save = h.save(1, &after_dir);
    h.finish_one();
    let manifest = read(&after_dir.join("project.json"));
    assert!(
        !serde_json::to_string(&manifest)
            .unwrap()
            .contains(result.id.as_str())
    );
    assert!(
        manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| {
                !entry["file_ref"]["path"]
                    .as_str()
                    .unwrap()
                    .starts_with("session/definition-cycle/")
            })
    );
    assert!(!after_dir.join("session").exists());
    let tree = Sandbox::new();
    let moved = tree.directory("moved-formal-project");
    copy(&after_dir, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(accepted(&reopened.full(&baseline.id)), vec![75000]);
    assert!(matches!(
        error(reopened.call(
            "get_run",
            json!({"run_id":result.id,"query":{"kind":"summary"}})
        ))
        .code,
        ErrorCode::MissingReference
    ));
    assert!(
        !reopened
            .context()
            .run_summaries
            .iter()
            .any(|summary| summary.run_id == result.id)
    );
}

#[test]
fn sample_revision_scope_input_identity_and_real_engine_failures_are_atomic() {
    let mut h = Harness::new();
    let draft = create(&mut h, fixture()["definitions"][0].clone());
    let original = h.object("scenario", "LOCAL");
    let before = business(&mut h);
    let mut stale_input = sample_params(&original, 1);
    stale_input["input_revision"] = json!(0);
    let mut stale_definition = sample_params(&original, 1);
    stale_definition["intent"]["data"]["definition_revision"] = json!(0);
    for params in [stale_input, stale_definition] {
        assert!(matches!(
            error(call(&mut h, &draft, "run_scenario", params)).code,
            ErrorCode::RevisionConflict
        ));
    }
    let mut wrong = sample_params(&original, 1);
    wrong["intent"]["data"]["test_input"]["project_revision"] = json!(0);
    assert!(matches!(
        error(call(&mut h, &draft, "run_scenario", wrong)).code,
        ErrorCode::RevisionConflict
    ));
    let mut wrong = sample_params(&original, 1);
    wrong["scenario_id"] = json!("different");
    assert!(matches!(
        error(call(&mut h, &draft, "run_scenario", wrong)).code,
        ErrorCode::InvalidInput
    ));
    let mut wrong = sample_params(&original, 1);
    wrong["observation_window"]["end_ms"] = json!(1000);
    assert!(matches!(
        error(call(&mut h, &draft, "run_scenario", wrong)).code,
        ErrorCode::InvalidInput
    ));
    let mut no_consumer = original.clone();
    no_consumer["production_tasks"] = json!([]);
    assert!(matches!(
        error(call(
            &mut h,
            &draft,
            "run_scenario",
            sample_params(&no_consumer, 1)
        ))
        .code,
        ErrorCode::MissingReference
    ));
    let mut invalid = original.clone();
    invalid["initial_conditions"]["items"][0]["quantity"]["unit"] = json!("kg");
    assert!(!matches!(
        error(call(
            &mut h,
            &draft,
            "run_scenario",
            sample_params(&invalid, 1)
        ))
        .code,
        ErrorCode::UnsupportedCapability
    ));
    assert_eq!(business(&mut h), before);
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn explicit_empty_seed_is_not_replaced_by_saved_scenario_and_cancel_uses_actual_task_lifecycle() {
    let mut h = Harness::new();
    let draft = create(&mut h, fixture()["definitions"][0].clone());
    let mut input = h.object("scenario", "LOCAL");
    input["initial_conditions"]["items"] = json!([]);
    let before = business(&mut h);
    let cancelled = sample(&mut h, &draft, &input, 1);
    let ack: CancellationState = ok(call(
        &mut h,
        &draft,
        "cancel_task",
        json!({"task_id":cancelled.id,"action":"cancel"}),
    ));
    assert!(ack.action_accepted);
    h.finish_one();
    let state = task(&mut h, &draft, &cancelled);
    assert!(matches!(state.task.state, ComputationStatus::Cancelled));
    assert!(state.result.is_none());
    let handle = sample(&mut h, &draft, &input, 1);
    let result = complete(&mut h, &draft, &handle);
    assert!(result.final_items.is_empty());
    assert!(result.acceptance_events.is_empty());
    assert!(!result.waits.is_empty());
    assert_eq!(
        frozen_object(&mut h, &draft, &result.id, "scenario", "LOCAL"),
        input
    );
    assert_eq!(business(&mut h), before);
}

#[test]
fn actual_definition_sample_does_not_leak_to_project_other_draft_or_ungranted_sessions() {
    let mut h = Harness::new();
    let draft = create(&mut h, fixture()["definitions"][0].clone());
    let other = create(&mut h, fixture()["definitions"][1].clone());
    let input = h.object("scenario", "LOCAL");
    let handle = sample(&mut h, &draft, &input, 1);
    let result = complete(&mut h, &draft, &handle);
    assert!(matches!(
        error(h.call(
            "get_run",
            json!({"run_id":result.id,"query":{"kind":"summary"}})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        error(call(
            &mut h,
            &other,
            "get_run",
            json!({"run_id":result.id,"query":{"kind":"summary"}})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    for (name, project, drafts, operations) in [
        (
            "no-project",
            None,
            vec![draft.clone()],
            vec![
                OperationName::RunScenario,
                OperationName::GetRun,
                OperationName::GetObject,
            ],
        ),
        (
            "no-draft",
            Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            vec![],
            vec![
                OperationName::RunScenario,
                OperationName::GetRun,
                OperationName::GetObject,
            ],
        ),
        (
            "no-project-read",
            Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            vec![draft.clone()],
            vec![OperationName::RunScenario, OperationName::GetRun],
        ),
        (
            "no-run-read",
            Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            vec![draft.clone()],
            vec![OperationName::RunScenario, OperationName::GetObject],
        ),
    ] {
        let session = id(name);
        h.host
            .register_external_session(HostCallContext {
                origin: CallerOrigin::ExternalAgent,
                allow_view_control: false,
                session_id: session.clone(),
                granted_project: project,
                allowed_operations: operations,
                allowed_definition_drafts: drafts,
                selected_file_tokens: vec![],
                edit_grants: vec![],
            })
            .unwrap();
        let reply = if name == "no-run-read" {
            scoped(
                &mut h,
                &session,
                &draft,
                "get_object",
                json!({"object_id":{"kind":"scenario","id":"LOCAL"},"source":{"kind":"run_snapshot","data":{"run_id":result.id,"at_ms":0}},"node_index":null}),
            )
        } else {
            scoped(
                &mut h,
                &session,
                &draft,
                "run_scenario",
                sample_params(&input, 1),
            )
        };
        assert!(
            matches!(error(reply).code, ErrorCode::PermissionDenied),
            "{name}"
        );
    }
    assert!(h.host.drain_io_work().is_empty());
    assert!(matches!(error(call(&mut h,&draft,"get_object",json!({"object_id":{"kind":"scenario","id":"LOCAL"},"source":{"kind":"run_snapshot","data":{"run_id":result.id,"at_ms":240001}},"node_index":null}))).code,ErrorCode::InvalidInput));
}

#[test]
fn nine_category_entry_distinguishes_actual_engine_consumers_from_five_non_simulation_previews() {
    let mut h = Harness::new();
    let input = h.object("scenario", "LOCAL");
    let packages = sf_module_kit::builtin_template_support().unwrap();
    for category in ["items", "resources", "templates", "professional", "media"] {
        let source = packages
            .iter()
            .find(|p| serde_json::to_value(&p.definition.payload).unwrap()["category"] == category)
            .unwrap();
        let draft = create(&mut h, serde_json::to_value(&source.definition).unwrap());
        let capabilities: CapabilityCatalog =
            ok(h.call_as(&h.ui.clone(), false, "describe_capabilities", json!({})));
        assert!(
            capabilities
                .implemented_operations
                .iter()
                .filter(|op| matches!(op.operation, OperationName::RunScenario))
                .all(|op| !op
                    .allowed_scopes
                    .iter()
                    .any(|scope| matches!(scope, ApiScope::DefinitionDraft(id) if id == &draft)))
        );
        let before = business(&mut h);
        let failure = error(call(
            &mut h,
            &draft,
            "run_scenario",
            sample_params(&input, 1),
        ));
        assert!(
            matches!(failure.code, ErrorCode::UnsupportedCapability),
            "{category}: {failure:?}"
        );
        assert!(!failure.message.is_empty());
        assert_eq!(business(&mut h), before);
        assert!(h.host.drain_io_work().is_empty());
    }
    let source = packages
        .iter()
        .find(|p| matches!(&p.definition.payload, ModulePayload::Tasks(_)))
        .unwrap();
    for kind in ["deferred_maintenance", "deferred_construction"] {
        let mut definition = serde_json::to_value(&source.definition).unwrap();
        definition["payload"]["data"]["kind"] = json!(kind);
        let draft = create(&mut h, definition);
        let capabilities: CapabilityCatalog =
            ok(h.call_as(&h.ui.clone(), false, "describe_capabilities", json!({})));
        assert!(
            capabilities
                .implemented_operations
                .iter()
                .filter(|op| matches!(op.operation, OperationName::RunScenario))
                .all(|op| !op
                    .allowed_scopes
                    .iter()
                    .any(|scope| matches!(scope, ApiScope::DefinitionDraft(id) if id == &draft)))
        );
        let before = business(&mut h);
        assert!(matches!(
            error(call(
                &mut h,
                &draft,
                "run_scenario",
                sample_params(&input, 1)
            ))
            .code,
            ErrorCode::UnsupportedCapability
        ));
        assert_eq!(business(&mut h), before);
        assert!(h.host.drain_io_work().is_empty());
    }
    let draft = create(&mut h, fixture()["definitions"][4].clone());
    let capabilities: CapabilityCatalog =
        ok(h.call_as(&h.ui.clone(), false, "describe_capabilities", json!({})));
    assert!(
        capabilities
            .implemented_operations
            .iter()
            .filter(|op| matches!(op.operation, OperationName::RunScenario))
            .any(|op| op
                .allowed_scopes
                .iter()
                .any(|scope| matches!(scope, ApiScope::DefinitionDraft(id) if id == &draft)))
    );
    let handle = sample(&mut h, &draft, &input, 1);
    let result = complete(&mut h, &draft, &handle);
    assert_eq!(accepted(&result), vec![75000]);
    let mut unrelated = fixture()["definitions"][0].clone();
    unrelated["id"] = json!("unmapped.new.operation");
    let draft = create(&mut h, unrelated);
    assert!(matches!(
        error(call(
            &mut h,
            &draft,
            "run_scenario",
            sample_params(&input, 1)
        ))
        .code,
        ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn actual_task_definition_sample_runs_explicit_preparation_on_the_same_engine() {
    let candidate =
        sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::ParallelElectromechanical)
            .unwrap();
    let mut h = Harness::new();
    h.host
        .register_bundled_template_directory(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/templates/parallel-electromechanical/candidate"),
        )
        .unwrap();
    let reference = json!({"id":candidate.package.definition.id,"version":candidate.package.definition.version});
    let meta: DomainObject = ok(h.call("get_object", json!({"object_id":{"kind":"module","id":reference["id"]},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":1,"choices":null,"seed":null}},"node_index":null})));
    let DomainObject::TemplatePreview(meta) = meta else {
        panic!("actual template metadata")
    };
    let choices = json!({"source_project":meta.source_project,"namespace":"cycle-test","resources":meta.resources.iter().map(|r|(r.id.clone(),Option::<ObjectId>::None)).collect::<BTreeMap<_,_>>(),"reuse_items":{},"include_example_product":true,"site_envelope":null});
    let quality = meta
        .subset
        .adoption
        .as_ref()
        .unwrap()
        .deferred_seed
        .as_ref()
        .unwrap()
        .initial_stock
        .iter()
        .map(|stock| (stock.location_id.clone(), QualityState::Released))
        .collect::<BTreeMap<_, _>>();
    let seed = json!({"scenario_id":"cycle-explicit","supply":true,"demand":true,"initial_quality":quality,"adopt_declared_inspection_results":true});
    let prepared: DomainObject = ok(h.call("get_object", json!({"object_id":{"kind":"module","id":reference["id"]},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":1,"choices":choices,"seed":seed}},"node_index":null})));
    let DomainObject::TemplatePreview(prepared) = prepared else {
        panic!("actual template preparation")
    };
    let _: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"instantiate_template","data":prepared.command.unwrap()}]),
    ));
    let mut definition = candidate
        .definitions
        .iter()
        .find(|d| matches!(&d.payload, ModulePayload::Tasks(_)))
        .map(|d| serde_json::to_value(d).unwrap())
        .unwrap();
    definition["payload"]["data"]["duration"]["value"] = json!(60000);
    let draft = create(&mut h, definition);
    let scenario = h.object("scenario", "cycle-explicit");
    let before = business(&mut h);
    let handle = sample(&mut h, &draft, &scenario, 1);
    let result = complete(&mut h, &draft, &handle);
    assert_eq!(accepted(&result), vec![190000, 230000]);
    assert_eq!(business(&mut h), before);
}

#[test]
fn actual_transport_draft_changes_only_sample_trip_times_and_preserves_cargo_ids() {
    let input: Value =
        serde_json::from_str(include_str!("fixtures/production/p202-identity.json")).unwrap();
    let mut definition = input["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["payload"]["category"] == "transport")
        .unwrap()
        .clone();
    definition["payload"]["data"]["times"]["outbound"]["value"] = json!(20000);
    let mut h = Harness::from_value(input);
    let draft = create(&mut h, definition);
    let scenario = h.object("scenario", "CARGO-identity");
    let before = business(&mut h);
    let handle = sample(&mut h, &draft, &scenario, 1);
    let result = complete(&mut h, &draft, &handle);
    let mut received: Vec<_> = result
        .transport
        .iter()
        .map(|trip| trip.received_ms.as_ref().unwrap().0.value())
        .collect();
    let mut returned: Vec<_> = result
        .transport
        .iter()
        .map(|trip| trip.returned_ms.as_ref().unwrap().0.value())
        .collect();
    received.sort();
    returned.sort();
    assert_eq!(received, vec![40000, 110000]);
    assert_eq!(returned, vec![70000, 140000]);
    let mut items: Vec<_> = result
        .final_items
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    items.sort();
    assert_eq!(items, vec!["cargo-0", "cargo-1"]);
    assert!(result.final_items.iter().all(
        |item| matches!(&item.holder, HolderRef::Location(id) if id.as_str()=="cargo-destination")
    ));
    assert_eq!(business(&mut h), before);
}
