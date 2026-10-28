//! Actual same-Host historical Job consumers, including a fresh reopened Host.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

fn phased() -> Value {
    serde_json::from_str(include_str!("fixtures/production/p1-phase-calendar.json")).unwrap()
}
fn query(h: &mut Harness, run: &RunId, at: u64, objects: Value, after: Value, limit: u64) -> Value {
    h.call("get_run",json!({"run_id":run,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":objects,"page":{"after_index":after,"limit":limit}}}}))
}
fn page(
    h: &mut Harness,
    run: &RunId,
    at: u64,
    objects: Value,
    after: Value,
    limit: u64,
) -> RunObjectsPage {
    let view: RunView = ok(query(h, run, at, objects, after, limit));
    let RunView::Objects(page) = view else {
        panic!("actual objects page")
    };
    page
}
fn job_id(run: &RunResult) -> ObjectId {
    run.jobs
        .iter()
        .find(|job| job.operation_ref.id.as_str() == "w5.op.phased")
        .unwrap()
        .id
        .clone()
}
fn job(h: &mut Harness, run: &RunResult, at: u64) -> Value {
    let page = page(
        h,
        &run.id,
        at,
        json!([{"kind":"job","id":job_id(run)}]),
        Value::Null,
        100,
    );
    assert_eq!(page.objects.len(), 1);
    let DomainObject::Job(value) = &page.objects[0] else {
        panic!("canonical JobState")
    };
    serde_json::to_value(value).unwrap()
}
fn execute(value: Value) -> (Harness, RunResult) {
    let scenario = value["scenario"]["id"].as_str().unwrap().to_owned();
    let mut h = Harness::from_value(value);
    let task = h.run(&scenario);
    h.finish_one();
    let run = h.completed_run(&task);
    (h, run)
}

#[test]
fn mixed_item_job_pages_and_reverse_seek_keep_actual_historical_progress() {
    let (mut h, run) = execute(phased());
    let before = h.snapshot();
    let first = job(&mut h, &run, 2500);
    assert_eq!(first["phase_ref"], "load");
    assert_eq!(first["effective_progress_ms"], 2500);
    let items = page(&mut h, &run.id, 2500, json!([]), Value::Null, 1);
    assert!(!items.objects.is_empty());
    assert_eq!(job(&mut h, &run, 2500), first);
    let later = job(&mut h, &run, 20000);
    assert_eq!(later["phase_ref"], "auto");
    assert_eq!(later["effective_progress_ms"], 15000);
    assert_eq!(job(&mut h, &run, 2500), first);
    assert_eq!(h.snapshot(), before);
}

#[test]
fn private_scene_observation_equals_public_actual_items_and_jobs_and_retains_ui_scope() {
    let (mut h, run) = execute(phased());
    let project: ProjectId = "P1".to_owned().try_into().unwrap();
    let revision = h.context().project.unwrap().revision;
    for at in [2500, 35000, 42500, 2500] {
        let items = page(&mut h, &run.id, at, json!([]), Value::Null, 100);
        let jobs = page(&mut h, &run.id, at, json!([{"kind":"job","id":job_id(&run)}]), Value::Null, 100);
        let actual = h.host.read_run_observation_ui(&h.ui, &project, &ApiScope::Project, &revision, &run.id, &SimTimeMs(at.try_into().unwrap())).unwrap();
        assert!(actual.next_index.is_none());
        assert_eq!(json!(actual.objects), json!([items.objects, jobs.objects].concat()));
    }
    let at = SimTimeMs(2500u64.try_into().unwrap());
    assert!(matches!(h.host.read_run_observation_ui(&h.ui, &project, &ApiScope::Project, &support::revision(99), &run.id, &at).unwrap_err().code, ErrorCode::RevisionConflict));
    assert!(matches!(h.host.read_run_observation_ui(&h.ui, &"other".to_owned().try_into().unwrap(), &ApiScope::Project, &revision, &run.id, &at).unwrap_err().code, ErrorCode::PermissionDenied));
    assert!(matches!(h.host.read_run_observation_ui(&h.ui, &project, &ApiScope::Project, &revision, &run.id, &SimTimeMs(100001u64.try_into().unwrap())).unwrap_err().code, ErrorCode::InvalidInput));
    let external = id("scene-read-agent");
    h.host.register_external_session(HostCallContext { origin: CallerOrigin::ExternalAgent, allow_view_control: false, session_id: external.clone(), granted_project: Some(project.clone()), allowed_operations: vec![OperationName::GetRun], allowed_definition_drafts: vec![], selected_file_tokens: vec![], edit_grants: vec![] }).unwrap();
    assert!(matches!(h.host.read_run_observation_ui(&external, &project, &ApiScope::Project, &revision, &run.id, &at).unwrap_err().code, ErrorCode::PermissionDenied));
    let definition = phased()["definitions"][0].clone();
    let created: EditCommit = ok(h.apply(1, json!([{"kind":"create_definition_draft","data":definition}])));
    let EditScope::DefinitionDraft(draft) = created.scope else { panic!("draft") };
    assert!(matches!(h.host.read_run_observation_ui(&h.ui, &project, &ApiScope::DefinitionDraft(draft.id), &support::revision(1), &run.id, &at).unwrap_err().code, ErrorCode::PermissionDenied));
}

#[test]
#[ignore = "Explicit original Native85 B60 archive diagnostic; no new Run is computed"]
fn stored_b60_paged_observation_keeps_sixty_actual_working_jobs() {
    let directory = std::path::PathBuf::from(std::env::var("SF_B60_ARCHIVE").expect("SF_B60_ARCHIVE must name the retained balanced r2 archive"));
    let manifest_before = std::fs::read(directory.join("project.json")).unwrap();
    let mut h = Harness::from_seed(Sandbox::new(), &directory);
    let read = |h: &mut Harness, query: Value| -> RunView {
        let request: ApiRequest = serde_json::from_value(json!({"api_version":"0.1","request_id":"b60-history-diagnostic",
            "project_id":"NFR-B60-balanced","scope":{"kind":"project"},"operation":"get_run",
            "params":{"run_id":"P201-core-164896:run:1:2","query":query}})).unwrap();
        ok(h.host.dispatch(&h.ui, request))
    };
    let RunView::Full(run) = read(&mut h, json!({"kind":"full_result"})) else { panic!("original full Run") };
    assert_eq!(run.jobs.len(), 30000);
    let started = std::time::Instant::now();
    let mut jobs = vec![];
    for chunk in run.jobs.chunks(200) {
        let selectors: Vec<_> = chunk.iter().map(|job| json!({"kind":"job","id":job.id})).collect();
        let RunView::Objects(page) = read(&mut h, json!({"kind":"objects_at","data":{"at_ms":602000,"objects":selectors,
            "page":{"after_index":null,"limit":200}}})) else { panic!("actual jobs page") };
        assert!(page.next_index.is_none());
        jobs.extend(page.objects.into_iter().map(|object| match object {DomainObject::Job(job)=>job,_=>panic!("Job only")}));
    }
    let elapsed_ms = started.elapsed().as_secs_f64()*1000.0;
    let bulk_start = std::time::Instant::now();
    let bulk = h.host.read_run_observation_ui(&h.ui, &"NFR-B60-balanced".to_owned().try_into().unwrap(), &ApiScope::Project, &revision(2), &run.id, &SimTimeMs(602000u64.try_into().unwrap())).unwrap();
    let bulk_elapsed_ms = bulk_start.elapsed().as_secs_f64()*1000.0;
    let bulk_jobs: Vec<_> = bulk.objects.iter().filter_map(|object| match object { DomainObject::Job(job) => Some(job), _ => None }).collect();
    assert_eq!(json!(bulk_jobs), json!(jobs));
    assert_eq!(bulk.objects.iter().filter(|object| matches!(object, DomainObject::Item(_))).count(), 3000);
    let working: Vec<_> = jobs.iter().filter(|job| matches!(job.state, JobStatus::Working)).collect();
    assert_eq!(working.len(),60);
    let devices: std::collections::BTreeSet<_> = working.iter().map(|job| match &job.execution_ref {
        JobExecutionRef::ProjectDevice { device_id, .. }=>device_id.clone(), _=>panic!("actual project device")}).collect();
    assert_eq!(devices.len(),60);
    assert_eq!(std::fs::read(directory.join("project.json")).unwrap(),manifest_before);
    println!("{}",json!({"source":"original stored balanced r2 Run, no recomputation","at_ms":602000,"queries":150,
        "job_states":jobs.len(),"working_jobs":working.len(),"devices":devices.len(),"public_Host_paged_read_ms":elapsed_ms,"private_UI_complete_observation_ms":bulk_elapsed_ms,
        "clock":"Rust Instant, Host diagnostic only; no comparison with JS clock and no Native/FPS acceptance"}));
}

#[test]
fn reversed_duplicate_and_unknown_job_selectors_keep_actual_page_order() {
    let source = serde_json::from_str(include_str!(
        "fixtures/production/local-assembly-pack.json"
    )).unwrap();
    let (mut h, run) = execute(source);
    let expected: Vec<_> = run.jobs.iter().map(|job| job.id.clone()).collect();
    assert!(expected.len() > 1);
    let mut selectors: Vec<_> = expected.iter().rev()
        .map(|id| json!({"kind":"job","id":id})).collect();
    selectors.push(selectors[0].clone());
    selectors.push(json!({"kind":"job","id":"not-a-runtime-job"}));
    let mut after = Value::Null;
    let mut actual = vec![];
    loop {
        let current = page(&mut h, &run.id, run.computed_until_ms.as_ref().unwrap().0.value(),
            json!(selectors), after, 1);
        for object in current.objects {
            let DomainObject::Job(job) = object else { panic!("selected Job only") };
            actual.push(job.id);
        }
        if current.next_index.is_none() { break; }
        after = json!(current.next_index);
    }
    assert_eq!(actual, expected);
}

fn save(h: &mut Harness, name: &str) -> std::path::PathBuf {
    let directory = h.tree.directory(name);
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &directory);
    h.finish_one();
    let state = h.task(&task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    directory
}

fn assert_moved_history(h: &mut Harness, run: &RunResult, moments: &[u64]) {
    let expected: Vec<_> = moments.iter().map(|at| job(h, run, *at)).collect();
    let saved = save(h, "state-original");
    let tree = Sandbox::new();
    let moved = tree.directory("state-moved");
    copy(&saved, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    for (at, expected) in moments.iter().zip(expected) {
        assert_eq!(job(&mut reopened, run, *at), expected, "saved state at{at}");
    }
    assert_eq!(
        serde_json::to_value(reopened.full(&run.id)).unwrap(),
        serde_json::to_value(run).unwrap()
    );
}

#[test]
fn actual_multiphase_jobs_and_item_job_pages_survive_move_open_and_current_edits() {
    let source = phased();
    let (mut h, run) = execute(source.clone());
    let id = job_id(&run);
    let final_job = run.jobs.iter().find(|job| job.id == id).unwrap();
    assert_eq!(final_job.finished_at_ms.as_ref().unwrap().0.value(), 45000);
    let worker: u64 = run
        .resource_claims
        .iter()
        .filter(|claim| claim.resource_id.as_str() == "W-C")
        .flat_map(|claim| &claim.intervals)
        .map(|interval| interval.end_ms.0.value() - interval.start_ms.0.value())
        .sum();
    assert_eq!(worker, 10000);
    let before = h.snapshot();
    let full = serde_json::to_value(&run).unwrap();
    let moments = [
        0, 2500, 4999, 5000, 20000, 34999, 35000, 37500, 40000, 42500, 45000, 100000,
    ];
    let states: Vec<_> = moments.iter().map(|at| job(&mut h, &run, *at)).collect();
    for (at, phase, progress) in [
        (0, "load", 0),
        (2500, "load", 2500),
        (4999, "load", 4999),
        (5000, "auto", 0),
        (20000, "auto", 15000),
        (34999, "auto", 29999),
        (40000, "unload", 0),
        (42500, "unload", 2500),
    ] {
        let state = &states[moments.iter().position(|x| *x == at).unwrap()];
        assert_eq!(state["state"], "working", "at{at}");
        assert_eq!(state["phase_ref"], phase, "at{at}");
        assert_eq!(state["effective_progress_ms"], progress, "at{at}");
        assert_eq!(state["job_resources"], json!(["CAP-CNC"]));
    }
    for at in [35000, 37500] {
        let state = &states[moments.iter().position(|x| *x == at).unwrap()];
        assert_eq!(state["state"], "waiting_resource");
        assert_eq!(state["phase_ref"], "unload");
        assert_eq!(state["effective_progress_ms"], 0);
        assert_eq!(state["job_resources"], json!(["CAP-CNC"]));
        // The declared machine is held through the wait; its worker is not.
        assert_eq!(state["phase_resources"], json!(["CAP-CNC"]));
    }
    assert_eq!(states[10]["state"], "completed");
    assert_eq!(states[10]["finished_at_ms"], 45000);
    let default = page(&mut h, &run.id, 2500, json!([]), Value::Null, 100);
    assert!(!default.objects.is_empty());
    assert!(
        default
            .objects
            .iter()
            .all(|object| matches!(object, DomainObject::Item(_)))
    );
    let selected = json!([{"kind":"job","id":id},{"kind":"item","id":"raw-1"}]);
    let mixed = page(&mut h, &run.id, 2500, selected.clone(), Value::Null, 100);
    assert_eq!(mixed.objects.len(), 2);
    assert!(matches!(mixed.objects[0], DomainObject::Item(_)));
    assert!(matches!(mixed.objects[1], DomainObject::Job(_)));
    let first = page(&mut h, &run.id, 2500, selected.clone(), Value::Null, 1);
    assert_eq!(first.next_index.as_ref().unwrap().value(), 0);
    let second = page(
        &mut h,
        &run.id,
        2500,
        selected.clone(),
        json!(first.next_index),
        1,
    );
    assert!(second.next_index.is_none());
    assert_eq!(
        serde_json::to_value([first.objects, second.objects].concat()).unwrap(),
        serde_json::to_value(&mixed.objects).unwrap()
    );
    let wrong_kind = page(
        &mut h,
        &run.id,
        2500,
        json!([{"kind":"job","id":"raw-1"},{"kind":"item","id":id}]),
        Value::Null,
        100,
    );
    assert!(wrong_kind.objects.is_empty());
    assert!(matches!(
        error(query(
            &mut h,
            &run.id,
            2500,
            json!([{"kind":"device","id":"CNC"}]),
            Value::Null,
            100
        ))
        .code,
        ErrorCode::UnsupportedCapability
    ));
    assert!(matches!(
        error(query(
            &mut h,
            &run.id,
            100001,
            json!([{"kind":"job","id":id}]),
            Value::Null,
            100
        ))
        .code,
        ErrorCode::InvalidInput
    ));
    assert!(matches!(
        error(query(
            &mut h,
            &run.id,
            2500,
            selected.clone(),
            Value::Null,
            0
        ))
        .code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(h.snapshot(), before);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), full);
    let saved = save(&mut h, "original");
    let tree = Sandbox::new();
    let moved = tree.directory("moved");
    copy(&saved, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    for (at, expected) in moments.iter().zip(&states) {
        assert_eq!(job(&mut reopened, &run, *at), *expected, "reopened{at}");
    }
    assert_eq!(
        serde_json::to_value(page(
            &mut reopened,
            &run.id,
            2500,
            json!([]),
            Value::Null,
            100
        ))
        .unwrap(),
        serde_json::to_value(&default).unwrap()
    );
    assert_eq!(
        serde_json::to_value(page(
            &mut reopened,
            &run.id,
            2500,
            selected,
            Value::Null,
            100
        ))
        .unwrap(),
        serde_json::to_value(&mixed).unwrap()
    );
    let project = reopened.object("project", "P1");
    let _:EditCommit=ok(reopened.apply(1,json!([{"kind":"set_project_settings","data":{"title":"New draft title","time_reference":project["time_reference"],"generation_context":project["generation_context"],"sources":project["sources"]}}])));
    let definition = source["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == "w5.op.phased")
        .unwrap()
        .clone();
    let created: EditCommit = ok(reopened.apply(
        2,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(draft) = created.scope else {
        panic!("real draft")
    };
    let mut changed = definition;
    changed["payload"]["data"]["phases"][1]["duration"]["value"] = json!(1000);
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"new-rule","project_id":"P1","scope":{"kind":"definition_draft","data":draft.id},"operation":"apply_commands","params":{"base_revision":1,"commands":[{"kind":"put_definition","data":changed}]}})).unwrap();
    let _: EditCommit = ok(reopened.host.dispatch(&reopened.ui, request));
    for (at, expected) in moments.iter().zip(&states) {
        assert_eq!(job(&mut reopened, &run, *at), *expected, "edited{at}");
    }
    assert_eq!(serde_json::to_value(reopened.full(&run.id)).unwrap(), full);
}

#[test]
fn actual_paused_calendar_busy_wait_and_waiting_output_are_not_final_job_backfill() {
    let mut paused = phased();
    paused["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"MACHINE-GAP","title":"explicit gap","source":"user_provided","intervals":[{"start_ms":0,"end_ms":20000},{"start_ms":40000,"end_ms":100000}]}));
    for resource in paused["project"]["resources"].as_array_mut().unwrap() {
        if resource["id"] == "CAP-CNC" {
            resource["calendar_id"] = json!("MACHINE-GAP");
        }
        if resource["id"] == "W-C" {
            resource["calendar_id"] = json!("ALL");
        }
    }
    paused["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "w5.op.phased")
        .unwrap()["payload"]["data"]["phases"][1]["interruption_policy"] = json!("pause_resume");
    let (mut h, run) = execute(paused);
    assert_eq!(job(&mut h, &run, 60000)["finished_at_ms"], 60000);
    for at in [20000, 25000, 39999] {
        let state = job(&mut h, &run, at);
        // The completed instant includes redispatch after interruption.
        assert_eq!(state["state"], "waiting_resource");
        assert_eq!(state["phase_ref"], "auto");
        assert_eq!(state["effective_progress_ms"], 15000);
    }
    assert_eq!(job(&mut h, &run, 42500)["effective_progress_ms"], 17500);
    assert_moved_history(
        &mut h,
        &run,
        &[0, 20000, 25000, 39999, 40000, 42500, 55000, 60000],
    );
    let busy: Value =
        serde_json::from_str(include_str!("fixtures/production/p1-phase-busy.json")).unwrap();
    let (mut h, run) = execute(busy);
    assert_eq!(job(&mut h, &run, 55000)["finished_at_ms"], 55000);
    let waiting = job(&mut h, &run, 45000);
    assert_eq!(waiting["state"], "waiting_resource");
    assert_eq!(waiting["phase_ref"], "unload");
    assert_eq!(waiting["effective_progress_ms"], 0);
    assert_moved_history(&mut h, &run, &[35000, 45000, 50000, 52500, 55000]);
    let mut blocked = phased();
    blocked["project"]["locations"][1]["capacity"]["value"]["coefficient"] = json!(0);
    let (mut h, run) = execute(blocked);
    assert_eq!(job(&mut h, &run, 42500)["pending_outputs"], json!([]));
    for at in [45000, 75000, 100000] {
        let state = job(&mut h, &run, at);
        assert_eq!(state["state"], "waiting_output");
        assert!(state["finished_at_ms"].is_null());
        assert_eq!(state["pending_outputs"].as_array().unwrap().len(), 1);
        let id = state["pending_outputs"][0].as_str().unwrap();
        let actual = page(
            &mut h,
            &run.id,
            at,
            json!([{"kind":"item","id":id}]),
            Value::Null,
            100,
        );
        let DomainObject::Item(item) = &actual.objects[0] else {
            panic!("real output")
        };
        assert_eq!(json!(item.holder), json!({"kind":"job","id":job_id(&run)}));
    }
    assert_moved_history(&mut h, &run, &[42500, 45000, 75000, 100000]);
}

#[test]
fn old_item_only_history_is_explicitly_unknown_for_jobs_and_scope_authority_still_applies() {
    let (mut h, run) = execute(phased());
    let items = page(&mut h, &run.id, 2500, json!([]), Value::Null, 100);
    let saved = save(&mut h, "modern");
    let tree = Sandbox::new();
    let legacy = tree.directory("legacy");
    copy(&saved, &legacy);
    let manifest = read(&legacy.join("project.json"));
    let entry = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["object_ref"]["id"] == json!(run.id)
                && entry["file_ref"]["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("/item-history.json"))
        })
        .unwrap();
    let path = legacy.join(entry["file_ref"]["path"].as_str().unwrap());
    let mut history = read(&path);
    assert!(
        history["data"]
            .as_object_mut()
            .unwrap()
            .remove("job_records")
            .is_some()
    );
    write(&path, &history);
    let mut reopened = Harness::from_seed(tree, &legacy);
    let unavailable = error(query(
        &mut reopened,
        &run.id,
        2500,
        json!([{"kind":"job","id":job_id(&run)}]),
        Value::Null,
        100,
    ));
    assert!(matches!(unavailable.code, ErrorCode::UnsupportedCapability));
    assert!(unavailable.message.contains("no actual Job history"));
    assert!(matches!(reopened.host.read_run_observation_ui(&reopened.ui, &"P1".to_owned().try_into().unwrap(), &ApiScope::Project, &revision(1), &run.id, &SimTimeMs(2500u64.try_into().unwrap())).unwrap_err().code, ErrorCode::UnsupportedCapability));
    assert_eq!(
        serde_json::to_value(page(
            &mut reopened,
            &run.id,
            2500,
            json!([]),
            Value::Null,
            100
        ))
        .unwrap(),
        serde_json::to_value(items).unwrap()
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&run.id)).unwrap(),
        serde_json::to_value(&run).unwrap()
    );
    let denied = id("history-no-read");
    reopened
        .host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: denied.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::GetContext],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    let params = json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":2500,"objects":[{"kind":"job","id":job_id(&run)}],"page":{"after_index":null,"limit":1}}}});
    assert!(matches!(
        error(reopened.call_as(&denied, true, "get_run", params.clone())).code,
        ErrorCode::PermissionDenied
    ));
    let definition = phased()["definitions"][0].clone();
    let created: EditCommit = ok(reopened.apply(
        1,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(draft) = created.scope else {
        panic!("draft")
    };
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"wrong-history-scope","project_id":"P1","scope":{"kind":"definition_draft","data":draft.id},"operation":"get_run","params":params})).unwrap();
    assert!(matches!(
        error(reopened.host.dispatch(&reopened.ui, request)).code,
        ErrorCode::PermissionDenied
    ));
}

#[test]
fn actual_definition_cycle_jobs_use_their_own_scope_and_frozen_rule_history() {
    let source = phased();
    let mut h = Harness::from_value(source.clone());
    let before = h.object("project", "P1");
    let source_scenario = h.object("scenario", "PHASE");
    let definition = source["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == "w5.op.phased")
        .unwrap()
        .clone();
    let created: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(draft) = created.scope else {
        panic!("actual definition scope")
    };
    let call = |h: &mut Harness, operation: &str, params: Value| {
        let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"cycle-job-history","project_id":"P1","scope":{"kind":"definition_draft","data":draft.id},"operation":operation,"params":params})).unwrap();
        h.host.dispatch(&h.ui, request)
    };
    let task: TaskHandle = ok(call(
        &mut h,
        "run_scenario",
        json!({"scenario_id":"PHASE","input_revision":1,"observation_window":source_scenario["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":1,"test_input":source_scenario}}}),
    ));
    h.finish_one();
    let state: TaskState = ok(call(&mut h, "get_task", json!({"task_id":task.id})));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Run(summary)) = state.result else {
        panic!("actual sample")
    };
    let result: RunView = ok(call(
        &mut h,
        "get_run",
        json!({"run_id":summary.run_id,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(run) = result else {
        panic!("actual full sample")
    };
    let full = serde_json::to_value(&run).unwrap();
    let params = json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":2500,"objects":[{"kind":"job","id":job_id(&run)}],"page":{"after_index":null,"limit":100}}}});
    let view: RunView = ok(call(&mut h, "get_run", params.clone()));
    let RunView::Objects(page) = view else {
        panic!("actual sample job")
    };
    let DomainObject::Job(job) = &page.objects[0] else {
        panic!("canonical job")
    };
    assert!(matches!(job.state, JobStatus::Working));
    assert_eq!(job.phase_ref.as_ref().unwrap().as_str(), "load");
    assert_eq!(job.effective_progress_ms.0.value(), 2500);
    let complete = h.host.read_run_observation_ui(&h.ui, &"P1".to_owned().try_into().unwrap(), &ApiScope::DefinitionDraft(draft.id.clone()), &revision(1), &run.id, &SimTimeMs(2500u64.try_into().unwrap())).unwrap();
    assert!(complete.objects.iter().any(|object| matches!(object, DomainObject::Job(value) if value.id == job.id && value.effective_progress_ms.0.value() == job.effective_progress_ms.0.value())));
    assert!(matches!(
        error(h.call("get_run", params.clone())).code,
        ErrorCode::PermissionDenied
    ));
    let mut changed = definition;
    changed["payload"]["data"]["phases"][1]["duration"]["value"] = json!(1000);
    let _: EditCommit = ok(call(
        &mut h,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_definition","data":changed}]}),
    ));
    let retained: RunView = ok(call(&mut h, "get_run", params));
    let RunView::Objects(retained) = retained else {
        panic!("retained job")
    };
    assert_eq!(
        serde_json::to_value(retained).unwrap(),
        serde_json::to_value(page).unwrap()
    );
    let retained: RunView = ok(call(
        &mut h,
        "get_run",
        json!({"run_id":run.id,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(retained) = retained else {
        panic!("retained result")
    };
    assert_eq!(serde_json::to_value(retained).unwrap(), full);
    assert_eq!(h.object("project", "P1"), before);
    assert_eq!(h.object("scenario", "PHASE"), source_scenario);
}
