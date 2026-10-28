//! Independent P0-02 expectations: actual Host calls and actual archive files.
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn id(value: &str) -> ObjectId {
    ObjectId::try_from(value.to_owned()).unwrap()
}

struct Scratch {
    base: PathBuf,
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let base = std::env::temp_dir().canonicalize().unwrap();
        let path = base.join(format!(
            "sf-p0-02-window5-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let path = path.canonicalize().unwrap();
        assert_eq!(path.parent(), Some(base.as_path()));
        Self { base, path }
    }

    fn directory(&self, name: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::create_dir(&path).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.path.parent() == Some(self.base.as_path()) {
            if let Err(error) = fs::remove_dir_all(&self.path) {
                eprintln!("test directory cleanup failed: {error}");
            }
        }
    }
}

fn source(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/type-contract-proof/fixtures")
            .join(format!("{name}.json")),
    )
    .unwrap()
}

fn input() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/business-inputs/multi-order.dto.json"
    ))
    .unwrap()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn document_entry(role: &str, kind: &str, object: &str, name: &str, path: &str) -> Value {
    json!({"role":role,"object_ref":{"kind":object,"id":name},"revision":if kind=="project" || kind=="scenario" {json!(1)} else {Value::Null},"module_ref":null,"file_ref":{"path":path,"kind":kind},"required":true})
}

fn seed_archive(directory: &Path) {
    let mut manifest: Value = serde_json::from_slice(&source("manifest")).unwrap();
    let mut project: Value = serde_json::from_slice(&source("project")).unwrap();
    // Select the actual external operation, rather than claiming an arbitrary
    // Devices module is an Operations module or inventing an inline behavior.
    project["data"]["devices"][0]["operation_binding"] =
        json!({"id":"demo.operations","version":"0.1.0"});
    project["data"]["issues"] = json!([{"code":"W5_MANUAL","object_ref":null,"field_path":null,"message":"preserve independently supplied information","blocks":[]}]);
    for entry in manifest["data"]["documents"].as_array_mut().unwrap() {
        if entry["role"] == "active_project" {
            entry["file_ref"]["path"] = json!("active/project.json");
            write_json(&directory.join("active/project.json"), &project);
        } else {
            let path = entry["file_ref"]["path"].as_str().unwrap();
            fs::write(directory.join(path), source(path.trim_end_matches(".json"))).unwrap();
        }
    }
    write_json(&directory.join("snapshots/project.json"), &project);
    let scenario: Value = serde_json::from_slice(&source("scenario")).unwrap();
    write_json(&directory.join("snapshots/scenario.json"), &scenario);
    let mut run: Value = serde_json::from_slice(&source("run")).unwrap();
    run["data"]["input_ref"]["frozen_project_file"]["path"] = json!("snapshots/project.json");
    run["data"]["input_ref"]["frozen_scenario_file"]["path"] = json!("snapshots/scenario.json");
    write_json(&directory.join("run.json"), &run);
    fs::write(directory.join("evaluation.json"), source("evaluation")).unwrap();
    manifest["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .extend([
            document_entry(
                "frozen_run_project",
                "project",
                "project",
                "P1",
                "snapshots/project.json",
            ),
            document_entry(
                "frozen_run_scenario",
                "scenario",
                "scenario",
                "SC1",
                "snapshots/scenario.json",
            ),
            document_entry("run_result", "run_result", "run", "R1", "run.json"),
            document_entry(
                "evaluation",
                "evaluation",
                "report",
                "E1",
                "evaluation.json",
            ),
        ]);
    write_json(&directory.join("project.json"), &manifest);
}

fn call(
    host: &mut ProjectHost,
    session: &ObjectId,
    project: bool,
    operation: &str,
    params: Value,
) -> Value {
    let request: ApiRequest = serde_json::from_value(json!({
        "api_version":"0.1","request_id":format!("w5-bi-{}",NEXT.fetch_add(1,Ordering::Relaxed)),
        "project_id":if project {json!("P1")} else {Value::Null},
        "scope":{"kind":if project {"project"} else {"workspace"}},
        "operation":operation,"params":params
    }))
    .unwrap();
    host.dispatch(session, request)
}

fn ok<T: DeserializeOwned>(reply: Value) -> T {
    match serde_json::from_value::<Response<T>>(reply.clone())
        .unwrap()
        .outcome
    {
        ApiOutcome::Ok(value) => value,
        _ => panic!("expected success: {reply:#}"),
    }
}

fn error(reply: Value) -> ApiError {
    match serde_json::from_value::<Response<Value>>(reply.clone())
        .unwrap()
        .outcome
    {
        ApiOutcome::Error(error) => error,
        _ => panic!("expected error: {reply:#}"),
    }
}

fn apply(host: &mut ProjectHost, session: &ObjectId, base: u64, commands: Value) -> Value {
    call(
        host,
        session,
        true,
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    )
}

fn put(kind: &str, data: Value) -> Value {
    json!({"kind":"put_object","data":{"kind":kind,"data":data}})
}

fn remove(kind: &str, name: &str) -> Value {
    json!({"kind":"remove_object","data":{"kind":kind,"id":name}})
}

fn object_reply(host: &mut ProjectHost, session: &ObjectId, kind: &str, name: &str) -> Value {
    call(
        host,
        session,
        true,
        "get_object",
        json!({"object_id":{"kind":kind,"id":name},"source":{"kind":"draft"},"node_index":null}),
    )
}

fn object(host: &mut ProjectHost, session: &ObjectId, kind: &str, name: &str) -> Value {
    let result: DomainObject = ok(object_reply(host, session, kind, name));
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["kind"], kind);
    result["data"].clone()
}

fn context(host: &mut ProjectHost, session: &ObjectId) -> ProjectContext {
    ok(call(
        host,
        session,
        true,
        "get_context",
        json!({"selection":[]}),
    ))
}

fn snapshot(host: &mut ProjectHost, session: &ObjectId) -> Value {
    let project = object(host, session, "project", "P1");
    let mut scenarios = BTreeMap::new();
    for name in project["scenario_refs"].as_array().unwrap() {
        let name = name.as_str().unwrap();
        scenarios.insert(name.to_owned(), object(host, session, "scenario", name));
    }
    let context = context(host, session);
    json!({"project":project,"scenarios":scenarios,"issues":context.issues,"tasks":context.tasks})
}

fn history(host: &mut ProjectHost, session: &ObjectId, base: u64, direction: &str) -> Value {
    apply(host, session, base, json!([{"kind":direction}]))
}

fn revision(commit: &EditCommit, expected: u64) {
    assert_eq!(commit.new_revision.0.value(), expected);
}

fn finish_one(host: &mut ProjectHost) {
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    host.finish_io(work.pop().unwrap().run());
}

fn task(host: &mut ProjectHost, session: &ObjectId, project: bool, task: &TaskHandle) -> TaskState {
    ok(call(
        host,
        session,
        project,
        "get_task",
        json!({"task_id":task.id}),
    ))
}

fn import(host: &mut ProjectHost, session: &ObjectId, directory: &Path) -> TaskHandle {
    let selection = host
        .select_directory(session, FilePurpose::ImportPackage, directory.to_owned())
        .unwrap();
    let task: TaskHandle = ok(call(
        host,
        session,
        false,
        "import_package",
        json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null}),
    ));
    assert!(matches!(task.state, ComputationStatus::Queued));
    assert!(task.result_ref.is_none());
    task
}

fn imported(host: &mut ProjectHost, session: &ObjectId, handle: &TaskHandle) -> ImportDraft {
    let state = task(host, session, false, handle);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let TaskOutput::Import(draft) = state.result.unwrap() else {
        panic!("not an ImportDraft")
    };
    draft
}

fn adopt_reply(
    host: &mut ProjectHost,
    session: &ObjectId,
    handle: &TaskHandle,
    draft: &ImportDraft,
    accept_missing: bool,
) -> Value {
    call(
        host,
        session,
        false,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":{"chosen_policy":"open_project","objects_to_create":draft.proposed_objects,"module_versions":draft.proposed_definitions,"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":accept_missing,"departure":"discard_after_tasks_terminal"}}}]}),
    )
}

fn adopt(host: &mut ProjectHost, session: &ObjectId, handle: &TaskHandle) -> EditCommit {
    let draft = imported(host, session, handle);
    assert!(draft.missing.is_empty());
    ok(adopt_reply(host, session, handle, &draft, false))
}

fn fixture(scratch: &Scratch) -> (ProjectHost, ObjectId) {
    let directory = scratch.directory("seed");
    seed_archive(&directory);
    let ui = id("w5-bi-ui");
    let mut host = ProjectHost::new(id("w5-bi-host"), ui.clone());
    let handle = import(&mut host, &ui, &directory);
    finish_one(&mut host);
    revision(&adopt(&mut host, &ui, &handle), 1);
    (host, ui)
}

fn business_commands() -> Value {
    let input = input();
    // Deliberately refer forward to objects in the same atomic batch.
    let mut commands = vec![put("scenario", input["scenario"].clone())];
    commands.extend(
        input["products"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(|v| put("product", v)),
    );
    commands.extend(
        input["boms"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(|v| put("bom", v)),
    );
    commands.push(put("route", input["route"].clone()));
    commands.extend(
        input["items"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(|v| put("item_type", v)),
    );
    json!(commands)
}

fn load_business(host: &mut ProjectHost, ui: &ObjectId) {
    revision(&ok(apply(host, ui, 1, business_commands())), 2);
}

fn business_issues(host: &mut ProjectHost, ui: &ObjectId) -> Vec<Issue> {
    context(host, ui)
        .issues
        .into_iter()
        .filter(|issue| issue.code.starts_with("business_inputs."))
        .collect()
}

fn assert_clean(host: &mut ProjectHost, ui: &ObjectId) {
    let issues = business_issues(host, ui);
    assert!(issues.is_empty(), "complete input has gaps: {issues:?}");
}

fn scenario_edit(mut scenario: Value, base: u64, coefficient: u64) -> Value {
    scenario["project_revision"] = json!(base);
    scenario["orders"][0]["quantity"]["value"]["coefficient"] = json!(coefficient);
    scenario["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] =
        json!(coefficient);
    scenario["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(coefficient);
    scenario
}

#[test]
fn complete_forward_references_commit_once_and_get_exact_formal_business_objects() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    let commit: EditCommit = ok(apply(&mut host, &ui, 1, business_commands()));
    revision(&commit, 2);
    assert_eq!(commit.previous_revision.unwrap().0.value(), 1);
    let expected = input();
    for kind in ["product", "bom"] {
        let key = if kind == "product" {
            "products"
        } else {
            "boms"
        };
        for value in expected[key].as_array().unwrap() {
            assert_eq!(
                object(&mut host, &ui, kind, value["id"].as_str().unwrap()),
                *value
            );
        }
    }
    assert_eq!(
        object(&mut host, &ui, "route", "ROUTE-AB"),
        expected["route"]
    );
    let mut scenario = expected["scenario"].clone();
    scenario["project_revision"] = json!(2);
    assert_eq!(object(&mut host, &ui, "scenario", "ORDERS"), scenario);
    let project = object(&mut host, &ui, "project", "P1");
    assert_eq!(project["revision"], 2);
    assert!(
        project["scenario_refs"]
            .as_array()
            .unwrap()
            .contains(&json!("ORDERS"))
    );
    assert_eq!(
        object(&mut host, &ui, "scenario", "SC1")["project_revision"],
        2
    );
    assert!(
        context(&mut host, &ui)
            .issues
            .iter()
            .any(|issue| issue.code == "W5_MANUAL")
    );
    assert_clean(&mut host, &ui);
}

#[test]
fn subject_revision_updates_never_retarget_orders_and_removal_keeps_explicit_draft() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let original = input()["products"][0].clone();
    let mut changed = original.clone();
    changed["revision"] = json!("r2");
    revision(
        &ok(apply(
            &mut host,
            &ui,
            2,
            json!([put("product", changed.clone())]),
        )),
        3,
    );
    assert_eq!(object(&mut host, &ui, "product", "A"), changed);
    assert_eq!(
        object(&mut host, &ui, "scenario", "ORDERS")["orders"][0]["product_ref"],
        json!({"id":"A","revision":"r1"})
    );
    let issues = business_issues(&mut host, &ui);
    assert!(issues.iter().any(|issue| {
        issue
            .object_ref
            .as_ref()
            .is_some_and(|r| r.id.as_str() == "ORDERS")
            && issue
                .field_path
                .as_ref()
                .is_some_and(|p| p.contains("product_ref"))
    }));
    revision(
        &ok(apply(&mut host, &ui, 3, json!([remove("product", "A")]))),
        4,
    );
    assert!(matches!(
        error(object_reply(&mut host, &ui, "product", "A")).code,
        ErrorCode::MissingReference
    ));
    assert_eq!(
        object(&mut host, &ui, "scenario", "ORDERS")["orders"][0]["product_ref"]["revision"],
        "r1"
    );
    revision(
        &ok(apply(&mut host, &ui, 4, json!([put("product", original)]))),
        5,
    );
    assert_clean(&mut host, &ui);
}

#[test]
fn nullable_demand_and_missing_references_are_saved_as_located_drafts_while_legacy_orders_stay_empty()
 {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let mut scenario = object(&mut host, &ui, "scenario", "ORDERS");
    scenario["project_revision"] = json!(2);
    scenario["orders"][0]["quantity"] = Value::Null;
    scenario["orders"][0]["due_at_ms"] = Value::Null;
    scenario["orders"][0]["product_ref"] = json!({"id":"missing-product","revision":"r9"});
    scenario["production_tasks"] = json!([]);
    revision(
        &ok(apply(&mut host, &ui, 2, json!([put("scenario", scenario)]))),
        3,
    );
    let saved = object(&mut host, &ui, "scenario", "ORDERS");
    assert!(saved["orders"][0]["quantity"].is_null());
    assert!(saved["orders"][0]["due_at_ms"].is_null());
    let issues = business_issues(&mut host, &ui);
    for field in ["product_ref", "quantity"] {
        assert!(
            issues.iter().any(|issue| issue
                .object_ref
                .as_ref()
                .is_some_and(|r| r.id.as_str() == "ORDERS")
                && issue.field_path.as_ref().is_some_and(|p| p.contains(field))
                && !issue.blocks.is_empty()),
            "unlocated missing {field}: {issues:?}"
        );
    }
    let mut bom = input()["boms"][0].clone();
    bom["rows"][0]["quantity"]["unit"] = json!("box");
    let mut route = input()["route"].clone();
    route["steps"][0]["operation_ref"] = json!({"id":"unimplemented-operation","version":"9.0.0"});
    revision(
        &ok(apply(
            &mut host,
            &ui,
            3,
            json!([put("bom", bom.clone()), put("route", route.clone())]),
        )),
        4,
    );
    assert_eq!(object(&mut host, &ui, "bom", "BOM-A"), bom);
    assert_eq!(object(&mut host, &ui, "route", "ROUTE-AB"), route);
    let issues = business_issues(&mut host, &ui);
    for (name, field) in [("BOM-A", "quantity"), ("ROUTE-AB", "operation_ref")] {
        assert!(
            issues.iter().any(|issue| issue
                .object_ref
                .as_ref()
                .is_some_and(|r| r.id.as_str() == name)
                && issue.field_path.as_ref().is_some_and(|p| p.contains(field))),
            "unlocated gap {name}: {issues:?}"
        );
    }
    let mut legacy: Value = serde_json::from_slice(&source("scenario")).unwrap();
    legacy = legacy["data"].clone();
    legacy["id"] = json!("LEGACY");
    legacy["project_revision"] = json!(4);
    legacy["orders"] = json!([]);
    legacy["production_tasks"] = json!([]);
    revision(
        &ok(apply(
            &mut host,
            &ui,
            4,
            json!([put("scenario", legacy.clone())]),
        )),
        5,
    );
    let mut expected = legacy;
    expected["project_revision"] = json!(5);
    assert_eq!(object(&mut host, &ui, "scenario", "LEGACY"), expected);

    // The archive itself reports no business Issue. Import must derive the gap
    // before asking for adoption, using the same gap that adoption will require.
    let directory = scratch.directory("import-business-gap");
    seed_archive(&directory);
    let mut archived_scenario = read_json(&directory.join("scenario.json"));
    archived_scenario["data"]["orders"][0]["quantity"] = Value::Null;
    archived_scenario["data"]["orders"][0]["due_at_ms"] = Value::Null;
    archived_scenario["data"]["production_tasks"] = json!([]);
    write_json(&directory.join("scenario.json"), &archived_scenario);
    assert!(
        read_json(&directory.join("project.json"))["data"]["missing"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let handle = import(&mut host, &ui, &directory);
    finish_one(&mut host);
    let draft = imported(&mut host, &ui, &handle);
    let preview = draft
        .proposed_data
        .iter()
        .find_map(|object| match object {
            DomainObject::Project(project) => Some(project.as_ref()),
            _ => None,
        })
        .unwrap();
    let preview_issues: Vec<_> = preview
        .issues
        .iter()
        .filter(|issue| issue.code.starts_with("business_inputs."))
        .map(|issue| serde_json::to_value(issue).unwrap())
        .collect();
    assert!(!preview_issues.is_empty());
    let missing = serde_json::to_value(&draft.missing).unwrap();
    for issue in &preview_issues {
        assert!(missing.as_array().unwrap().contains(issue));
    }
    assert!(draft.missing.iter().any(|issue| {
        issue.code.starts_with("business_inputs.")
            && issue
                .field_path
                .as_ref()
                .is_some_and(|path| path.contains("quantity"))
    }));
    let before = snapshot(&mut host, &ui);
    assert!(matches!(
        error(adopt_reply(&mut host, &ui, &handle, &draft, false)).code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    revision(&ok(adopt_reply(&mut host, &ui, &handle, &draft, true)), 1);
    let accepted_issues = serde_json::to_value(context(&mut host, &ui).issues).unwrap();
    for issue in preview_issues {
        assert!(accepted_issues.as_array().unwrap().contains(&issue));
    }
    let accepted = object(&mut host, &ui, "scenario", "SC1");
    assert!(accepted["orders"][0]["quantity"].is_null());
    assert!(accepted["orders"][0]["due_at_ms"].is_null());
}

#[test]
fn put_remove_get_business_kinds_use_the_actual_host_and_sync_scenario_refs() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let kinds = [
        ("product", "A"),
        ("bom", "BOM-A"),
        ("route", "ROUTE-AB"),
        ("scenario", "ORDERS"),
    ];
    let values: Vec<_> = kinds
        .iter()
        .map(|(kind, name)| object(&mut host, &ui, kind, name))
        .collect();
    revision(
        &ok(apply(
            &mut host,
            &ui,
            2,
            json!(
                kinds
                    .iter()
                    .map(|(kind, name)| remove(kind, name))
                    .collect::<Vec<_>>()
            ),
        )),
        3,
    );
    for (kind, name) in kinds {
        assert!(matches!(
            error(object_reply(&mut host, &ui, kind, name)).code,
            ErrorCode::MissingReference
        ));
    }
    assert!(
        !object(&mut host, &ui, "project", "P1")["scenario_refs"]
            .as_array()
            .unwrap()
            .contains(&json!("ORDERS"))
    );
    let commands: Vec<_> = kinds
        .iter()
        .zip(values)
        .map(|((kind, _), mut value)| {
            if *kind == "scenario" {
                value["revision"] = json!(1);
                value["project_revision"] = json!(3);
            }
            put(kind, value)
        })
        .collect();
    revision(&ok(apply(&mut host, &ui, 3, json!(commands))), 4);
    assert_eq!(object(&mut host, &ui, "scenario", "ORDERS")["revision"], 2);
    assert!(
        object(&mut host, &ui, "project", "P1")["scenario_refs"]
            .as_array()
            .unwrap()
            .contains(&json!("ORDERS"))
    );
    assert_clean(&mut host, &ui);

    // A declared missing Scenario is a retained input reference, even though it
    // has no loaded document. An unrelated settings edit must not erase it.
    let directory = scratch.directory("reported-missing-scenario");
    seed_archive(&directory);
    let mut archived_project = read_json(&directory.join("active/project.json"));
    archived_project["data"]["scenario_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!("UNRESOLVED"));
    // Put the old derived Issue in Project.issues, with an empty manifest
    // missing list, so refreshing it cannot retain the old direct declaration
    // as an alternative justification for a missing Scenario document.
    archived_project["data"]["issues"].as_array_mut().unwrap().push(json!({
        "code":"business_inputs.legacy_missing","object_ref":{"kind":"scenario","id":"UNRESOLVED"},
        "field_path":"/scenario_refs/1","message":"explicitly reported Scenario input not supplied",
        "blocks":["execution"]
    }));
    write_json(&directory.join("active/project.json"), &archived_project);
    assert!(
        read_json(&directory.join("project.json"))["data"]["missing"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let handle = import(&mut host, &ui, &directory);
    finish_one(&mut host);
    let draft = imported(&mut host, &ui, &handle);
    assert!(draft.missing.iter().any(|issue| {
        issue.code == "business_inputs.missing_scenario"
            && issue.object_ref.as_ref().is_some_and(|target| {
                matches!(target.kind, ObjectKind::Project) && target.id.as_str() == "P1"
            })
            && issue.field_path.as_deref() == Some("/scenario_refs/1")
    }));
    revision(&ok(adopt_reply(&mut host, &ui, &handle, &draft, true)), 1);
    let opened = object(&mut host, &ui, "project", "P1");
    let refs = opened["scenario_refs"].clone();
    assert!(refs.as_array().unwrap().contains(&json!("UNRESOLVED")));
    revision(
        &ok(apply(
            &mut host,
            &ui,
            1,
            json!([{"kind":"set_project_settings","data":{
                "title":"unrelated title edit","time_reference":opened["time_reference"],
                "generation_context":opened["generation_context"],"sources":opened["sources"]
            }}]),
        )),
        2,
    );
    assert_eq!(
        object(&mut host, &ui, "project", "P1")["scenario_refs"],
        refs
    );
    assert!(matches!(
        error(object_reply(&mut host, &ui, "scenario", "UNRESOLVED")).code,
        ErrorCode::MissingReference
    ));
    let current = object(&mut host, &ui, "project", "P1");
    assert!(
        !current["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "business_inputs.legacy_missing")
    );
    let canonical: Vec<_> = current["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|issue| {
            issue["code"] == "business_inputs.missing_scenario"
                && issue["object_ref"] == json!({"kind":"project","id":"P1"})
                && issue["field_path"] == "/scenario_refs/1"
        })
        .cloned()
        .collect();
    assert_eq!(canonical.len(), 1);
    let persisted = scratch.directory("missing-scenario-saved");
    let reopened = scratch.directory("missing-scenario-reopened");
    let selected = host
        .select_directory(&ui, FilePurpose::SaveProject, persisted.clone())
        .unwrap();
    let handle = save(&mut host, &ui, 2, &selected);
    finish_one(&mut host);
    assert!(!saved(&mut host, &ui, &handle).still_dirty);
    let disk = disk_document(&persisted, "active_project", "P1");
    assert_eq!(disk["scenario_refs"], refs);
    assert!(disk["issues"].as_array().unwrap().contains(&canonical[0]));
    copy_directory(&persisted, &reopened);
    let handle = import(&mut host, &ui, &reopened);
    finish_one(&mut host);
    let draft = imported(&mut host, &ui, &handle);
    assert!(
        serde_json::to_value(&draft.missing)
            .unwrap()
            .as_array()
            .unwrap()
            .contains(&canonical[0])
    );
    revision(&ok(adopt_reply(&mut host, &ui, &handle, &draft, true)), 2);
    assert_eq!(
        object(&mut host, &ui, "project", "P1")["scenario_refs"],
        refs
    );
    assert!(
        serde_json::to_value(context(&mut host, &ui).issues)
            .unwrap()
            .as_array()
            .unwrap()
            .contains(&canonical[0])
    );
    let mut supplied: Value = serde_json::from_slice(&source("scenario")).unwrap();
    supplied = supplied["data"].clone();
    supplied["id"] = json!("UNRESOLVED");
    supplied["project_revision"] = json!(2);
    revision(
        &ok(apply(&mut host, &ui, 2, json!([put("scenario", supplied)]))),
        3,
    );
    assert_eq!(
        object(&mut host, &ui, "project", "P1")["scenario_refs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| **r == json!("UNRESOLVED"))
            .count(),
        1
    );
    revision(
        &ok(apply(
            &mut host,
            &ui,
            3,
            json!([remove("scenario", "UNRESOLVED")]),
        )),
        4,
    );
    let refs = object(&mut host, &ui, "project", "P1")["scenario_refs"].clone();
    assert!(!refs.as_array().unwrap().contains(&json!("UNRESOLVED")));
    assert!(refs.as_array().unwrap().contains(&json!("SC1")));
}

#[test]
fn invalid_scenario_metadata_and_stale_batches_leave_documents_history_and_tasks_unchanged() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let before = snapshot(&mut host, &ui);
    let original = object(&mut host, &ui, "scenario", "ORDERS");
    let mut new_wrong = original.clone();
    new_wrong["id"] = json!("NEW");
    new_wrong["revision"] = json!(2);
    let mut update_wrong = original.clone();
    update_wrong["revision"] = json!(2);
    let mut anchor_wrong = original;
    anchor_wrong["project_revision"] = json!(1);
    let mut temporary = input()["products"][0].clone();
    temporary["id"] = json!("TEMP");
    for scenario in [new_wrong, update_wrong, anchor_wrong] {
        let failure = error(apply(
            &mut host,
            &ui,
            2,
            json!([put("product", temporary.clone()), put("scenario", scenario)]),
        ));
        assert!(!matches!(failure.code, ErrorCode::UnsupportedCapability));
        assert_eq!(snapshot(&mut host, &ui), before);
        assert!(matches!(
            error(object_reply(&mut host, &ui, "product", "TEMP")).code,
            ErrorCode::MissingReference
        ));
        assert!(host.drain_io_work().is_empty());
    }
    revision(&ok(history(&mut host, &ui, 2, "undo")), 3);
    assert!(matches!(
        error(object_reply(&mut host, &ui, "scenario", "ORDERS")).code,
        ErrorCode::MissingReference
    ));
    revision(&ok(history(&mut host, &ui, 3, "redo")), 4);
    let before = snapshot(&mut host, &ui);
    assert!(matches!(
        error(apply(&mut host, &ui, 2, json!([put("product", temporary)]))).code,
        ErrorCode::RevisionConflict
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
}

fn field(pointer: &str) -> HostFieldGrant {
    HostFieldGrant {
        pointer: pointer.to_owned(),
        element_id: None,
    }
}

fn grant(branches: Vec<HostEditBranch>) -> HostEditGrant {
    HostEditGrant {
        id: id("w5-bi-grant"),
        scope: TaskScope::Edit(EditScope::Project(
            ProjectId::try_from("P1".to_owned()).unwrap(),
        )),
        branches,
        objects: vec![],
        creates: vec![],
        source_versions: vec![],
        reusable_resources: vec![],
        created_objects: vec![],
    }
}

fn external(session: &ObjectId, grant: HostEditGrant) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: session.clone(),
        granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
        allowed_operations: vec![
            OperationName::GetContext,
            OperationName::GetObject,
            OperationName::ApplyCommands,
        ],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![grant],
    }
}

fn ledger(host: &ProjectHost, session: &ObjectId) -> Value {
    json!(
        host.caller_context(session)
            .unwrap()
            .edit_grants
            .iter()
            .map(|g| json!({"id":g.id,"created_objects":g.created_objects}))
            .collect::<Vec<_>>()
    )
}

#[test]
fn external_authority_is_current_and_failed_batches_preserve_all_documents_history_and_creation_ledger()
 {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let session = id("w5-bi-external");
    let mut g = grant(vec![
        HostEditBranch::PutObject,
        HostEditBranch::Undo,
        HostEditBranch::Redo,
    ]);
    g.creates.push(HostCreateAllowance {
        kind: ObjectKind::Product,
        max_new_objects: SafeUint::try_from(1).unwrap(),
        fields: [
            "/revision",
            "/item_type_ref/id",
            "/item_type_ref/revision",
            "/bom_ref/id",
            "/bom_ref/revision",
            "/routing_ref/id",
            "/routing_ref/revision",
        ]
        .into_iter()
        .map(field)
        .collect(),
    });
    g.objects.push(HostObjectEditGrant {
        target: ObjectRef {
            kind: ObjectKind::Scenario,
            id: id("ORDERS"),
        },
        fields: vec![field("/window/end_ms")],
        can_remove: false,
    });
    g.objects.push(HostObjectEditGrant {
        target: ObjectRef {
            kind: ObjectKind::Product,
            id: id("C"),
        },
        fields: vec![],
        can_remove: true,
    });
    host.register_external_session(external(&session, g))
        .unwrap();
    let before = snapshot(&mut host, &ui);
    let before_ledger = ledger(&host, &session);
    let mut product = input()["products"][0].clone();
    product["id"] = json!("C");
    let mut scenario = object(&mut host, &ui, "scenario", "ORDERS");
    scenario["title"] = json!("not authorized");
    let failure = error(apply(
        &mut host,
        &session,
        2,
        json!([put("product", product.clone()), put("scenario", scenario)]),
    ));
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    assert_eq!(snapshot(&mut host, &ui), before);
    assert_eq!(ledger(&host, &session), before_ledger);
    revision(&ok(history(&mut host, &ui, 2, "undo")), 3);
    assert!(matches!(
        error(object_reply(&mut host, &ui, "scenario", "ORDERS")).code,
        ErrorCode::MissingReference
    ));
    revision(&ok(history(&mut host, &ui, 3, "redo")), 4);
    revision(
        &ok(apply(
            &mut host,
            &session,
            4,
            json!([put("product", product)]),
        )),
        5,
    );
    let spent = ledger(&host, &session);
    assert_eq!(
        spent[0]["created_objects"],
        json!([{"kind":"product","id":"C"}])
    );
    let before = snapshot(&mut host, &ui);
    assert!(matches!(
        error(apply(
            &mut host,
            &session,
            5,
            json!([remove("product", "C")])
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    assert_eq!(ledger(&host, &session), spent);
    let no_history = id("w5-bi-no-history");
    host.register_external_session(external(
        &no_history,
        grant(vec![HostEditBranch::PutObject]),
    ))
    .unwrap();
    assert!(matches!(
        error(history(&mut host, &no_history, 5, "undo")).code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    revision(&ok(history(&mut host, &session, 5, "undo")), 6);
    assert!(matches!(
        error(object_reply(&mut host, &ui, "product", "C")).code,
        ErrorCode::MissingReference
    ));
    assert_eq!(ledger(&host, &session), spent);
    revision(&ok(history(&mut host, &session, 6, "redo")), 7);
    assert_eq!(object(&mut host, &ui, "product", "C")["id"], "C");
    assert_eq!(ledger(&host, &session), spent);
}

#[test]
fn external_bom_and_route_array_edits_stay_unsupported_but_ui_edits_real_lists() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let session = id("w5-bi-array");
    let mut g = grant(vec![HostEditBranch::PutObject]);
    g.objects = vec![
        HostObjectEditGrant {
            target: ObjectRef {
                kind: ObjectKind::Bom,
                id: id("BOM-A"),
            },
            fields: vec![field("/rows/0/quantity/value/coefficient")],
            can_remove: false,
        },
        HostObjectEditGrant {
            target: ObjectRef {
                kind: ObjectKind::Route,
                id: id("ROUTE-AB"),
            },
            fields: vec![HostFieldGrant {
                pointer: "/steps/1/predecessors/0".to_owned(),
                element_id: Some(id("step-packing")),
            }],
            can_remove: false,
        },
    ];
    host.register_external_session(external(&session, g))
        .unwrap();
    let before = snapshot(&mut host, &ui);
    let before_ledger = ledger(&host, &session);
    let mut bom = input()["boms"][0].clone();
    bom["rows"][0]["quantity"]["value"]["coefficient"] = json!(2);
    let mut route = input()["route"].clone();
    route["steps"][1]["predecessors"] = json!([]);
    for command in [put("bom", bom.clone()), put("route", route.clone())] {
        assert!(matches!(
            error(apply(&mut host, &session, 2, json!([command]))).code,
            ErrorCode::UnsupportedCapability
        ));
        assert_eq!(snapshot(&mut host, &ui), before);
        assert_eq!(ledger(&host, &session), before_ledger);
    }
    revision(
        &ok(apply(
            &mut host,
            &ui,
            2,
            json!([put("bom", bom.clone()), put("route", route.clone())]),
        )),
        3,
    );
    assert_eq!(object(&mut host, &ui, "bom", "BOM-A"), bom);
    assert_eq!(object(&mut host, &ui, "route", "ROUTE-AB"), route);
    assert_clean(&mut host, &ui);
}

fn assert_scenario(host: &mut ProjectHost, ui: &ObjectId, r: u64, s: u64, n: u64) {
    assert_eq!(object(host, ui, "project", "P1")["revision"], r);
    let scenario = object(host, ui, "scenario", "ORDERS");
    assert_eq!(scenario["revision"], s);
    assert_eq!(scenario["project_revision"], r);
    assert_eq!(scenario["orders"][0]["quantity"]["value"]["coefficient"], n);
}

#[test]
fn scenario_history_restores_whole_documents_with_monotonic_host_revision_high_water() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    assert_scenario(&mut host, &ui, 2, 1, 2);
    let next = scenario_edit(object(&mut host, &ui, "scenario", "ORDERS"), 2, 3);
    revision(
        &ok(apply(&mut host, &ui, 2, json!([put("scenario", next)]))),
        3,
    );
    assert_scenario(&mut host, &ui, 3, 2, 3);
    revision(&ok(history(&mut host, &ui, 3, "undo")), 4);
    assert_scenario(&mut host, &ui, 4, 3, 2);
    revision(&ok(history(&mut host, &ui, 4, "redo")), 5);
    assert_scenario(&mut host, &ui, 5, 4, 3);
    revision(
        &ok(apply(
            &mut host,
            &ui,
            5,
            json!([remove("scenario", "ORDERS")]),
        )),
        6,
    );
    assert!(
        !object(&mut host, &ui, "project", "P1")["scenario_refs"]
            .as_array()
            .unwrap()
            .contains(&json!("ORDERS"))
    );
    revision(&ok(history(&mut host, &ui, 6, "undo")), 7);
    assert_scenario(&mut host, &ui, 7, 5, 3);
    revision(&ok(history(&mut host, &ui, 7, "redo")), 8);
    assert!(matches!(
        error(object_reply(&mut host, &ui, "scenario", "ORDERS")).code,
        ErrorCode::MissingReference
    ));
    let mut recreated = input()["scenario"].clone();
    recreated["project_revision"] = json!(8);
    revision(
        &ok(apply(
            &mut host,
            &ui,
            8,
            json!([put("scenario", recreated)]),
        )),
        9,
    );
    assert_scenario(&mut host, &ui, 9, 6, 2);
}

fn copy_directory(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            fs::create_dir(&target).unwrap();
            copy_directory(&entry.path(), &target);
        } else {
            assert!(kind.is_file());
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn disk_document(directory: &Path, role: &str, name: &str) -> Value {
    let manifest = read_json(&directory.join("project.json"));
    let entries: Vec<_> = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["role"] == role && e["object_ref"]["id"] == name)
        .collect();
    assert_eq!(entries.len(), 1);
    read_json(&directory.join(entries[0]["file_ref"]["path"].as_str().unwrap()))["data"].clone()
}

fn frozen_documents(directory: &Path) -> BTreeMap<String, Vec<u8>> {
    let manifest = read_json(&directory.join("project.json"));
    manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            matches!(
                e["role"].as_str().unwrap(),
                "frozen_run_project" | "frozen_run_scenario" | "run_result" | "evaluation"
            )
        })
        .map(|entry| {
            let path = entry["file_ref"]["path"].as_str().unwrap();
            (path.to_owned(), fs::read(directory.join(path)).unwrap())
        })
        .collect()
}

fn save(host: &mut ProjectHost, ui: &ObjectId, r: u64, selected: &SelectedFileRef) -> TaskHandle {
    ok(call(
        host,
        ui,
        true,
        "save_project",
        json!({"revision":r,"selected_directory_ref":selected}),
    ))
}

fn saved(host: &mut ProjectHost, ui: &ObjectId, handle: &TaskHandle) -> SavedProject {
    let state = task(host, ui, true, handle);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let TaskOutput::Save(saved) = state.result.unwrap() else {
        panic!("not SavedProject")
    };
    saved
}

#[test]
fn frozen_project_and_scenarios_save_at_r_reopen_elsewhere_and_preserve_old_frozen_documents() {
    let scratch = Scratch::new();
    let (mut host, ui) = fixture(&scratch);
    load_business(&mut host, &ui);
    let directory = scratch.directory("saved");
    let copied = scratch.directory("reopened");
    let old = frozen_documents(&scratch.path.join("seed"));
    assert_eq!(old.len(), 4);
    let frozen = snapshot(&mut host, &ui);
    let selected = host
        .select_directory(&ui, FilePurpose::SaveProject, directory.clone())
        .unwrap();
    let handle = save(&mut host, &ui, 2, &selected);
    assert!(matches!(handle.state, ComputationStatus::Queued));
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    let next = scenario_edit(object(&mut host, &ui, "scenario", "ORDERS"), 2, 3);
    revision(
        &ok(apply(&mut host, &ui, 2, json!([put("scenario", next)]))),
        3,
    );
    host.finish_io(work.pop().unwrap().run());
    let result = saved(&mut host, &ui, &handle);
    assert_eq!(result.saved_revision.0.value(), 2);
    assert_eq!(result.current_revision.0.value(), 3);
    assert!(result.still_dirty);
    assert!(context(&mut host, &ui).project.unwrap().dirty);
    assert_eq!(
        disk_document(&directory, "active_project", "P1"),
        frozen["project"]
    );
    for (name, scenario) in frozen["scenarios"].as_object().unwrap() {
        assert_eq!(disk_document(&directory, "scenario_input", name), *scenario);
    }
    assert_eq!(frozen_documents(&directory), old);
    copy_directory(&directory, &copied);
    let current = snapshot(&mut host, &ui);
    let handle = import(&mut host, &ui, &copied);
    finish_one(&mut host);
    assert_eq!(snapshot(&mut host, &ui)["project"], current["project"]);
    assert_scenario(&mut host, &ui, 3, 2, 3);
    assert!(context(&mut host, &ui).project.unwrap().dirty);
    revision(&adopt(&mut host, &ui, &handle), 2);
    assert_scenario(&mut host, &ui, 2, 1, 2);
    assert_eq!(snapshot(&mut host, &ui)["project"], frozen["project"]);
    assert!(!context(&mut host, &ui).project.unwrap().dirty);
    assert_clean(&mut host, &ui);
    let next = scenario_edit(object(&mut host, &ui, "scenario", "ORDERS"), 2, 4);
    revision(
        &ok(apply(&mut host, &ui, 2, json!([put("scenario", next)]))),
        3,
    );
    let selected = host
        .select_directory(&ui, FilePurpose::SaveProject, copied.clone())
        .unwrap();
    let handle = save(&mut host, &ui, 3, &selected);
    finish_one(&mut host);
    assert!(!saved(&mut host, &ui, &handle).still_dirty);
    assert_eq!(
        disk_document(&copied, "scenario_input", "ORDERS"),
        object(&mut host, &ui, "scenario", "ORDERS")
    );
    assert_eq!(frozen_documents(&copied), old);
    assert_eq!(frozen_documents(&directory), old);
    assert_eq!(
        disk_document(&directory, "active_project", "P1"),
        frozen["project"]
    );
    assert_eq!(
        disk_document(&directory, "scenario_input", "ORDERS"),
        frozen["scenarios"]["ORDERS"]
    );

    // Current SC1 can be deleted while the old Run/Evaluation still retain its
    // frozen input. Reopening must initialize the ID's revision high water
    // from those actual retained documents, before a new Put of the same ID.
    revision(
        &ok(apply(&mut host, &ui, 3, json!([remove("scenario", "SC1")]))),
        4,
    );
    let handle = save(&mut host, &ui, 4, &selected);
    finish_one(&mut host);
    assert!(!saved(&mut host, &ui, &handle).still_dirty);
    assert_eq!(frozen_documents(&copied), old);
    let manifest = read_json(&copied.join("project.json"));
    let retained: Vec<_> = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["object_ref"] == json!({"kind":"scenario","id":"SC1"}))
        .collect();
    assert!(!retained.is_empty());
    assert!(
        retained
            .iter()
            .all(|entry| entry["role"] != "scenario_input")
    );
    let maximum_retained = retained
        .iter()
        .map(|entry| {
            let revision = entry["revision"].as_u64().unwrap();
            let document = read_json(&copied.join(entry["file_ref"]["path"].as_str().unwrap()));
            assert_eq!(document["data"]["id"], "SC1");
            assert_eq!(document["data"]["revision"], revision);
            revision
        })
        .max()
        .unwrap();
    let handle = import(&mut host, &ui, &copied);
    finish_one(&mut host);
    revision(&adopt(&mut host, &ui, &handle), 4);
    assert!(matches!(
        error(object_reply(&mut host, &ui, "scenario", "SC1")).code,
        ErrorCode::MissingReference
    ));
    let mut recreated: Value = serde_json::from_slice(&source("scenario")).unwrap();
    recreated = recreated["data"].clone();
    recreated["revision"] = json!(1);
    recreated["project_revision"] = json!(4);
    revision(
        &ok(apply(
            &mut host,
            &ui,
            4,
            json!([put("scenario", recreated)]),
        )),
        5,
    );
    let restored = object(&mut host, &ui, "scenario", "SC1");
    assert!(restored["revision"].as_u64().unwrap() > maximum_retained);
    assert_eq!(restored["project_revision"], 5);
    assert_eq!(frozen_documents(&copied), old);

    // This optional declaration has no file and no current Scenario reference.
    // Its advertised MAX_SAFE_UINT is not a retained input version.
    let optional = scratch.directory("optional-scenario-not-included");
    seed_archive(&optional);
    let mut manifest = read_json(&optional.join("project.json"));
    let mut declaration = document_entry(
        "supporting_file",
        "scenario",
        "scenario",
        "OPTIONAL",
        "not-included/optional.json",
    );
    declaration["required"] = json!(false);
    declaration["revision"] = json!(9_007_199_254_740_991_u64);
    manifest["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(declaration);
    write_json(&optional.join("project.json"), &manifest);
    assert!(!optional.join("not-included/optional.json").exists());
    assert!(
        !disk_document(&optional, "active_project", "P1")["scenario_refs"]
            .as_array()
            .unwrap()
            .contains(&json!("OPTIONAL"))
    );
    let handle = import(&mut host, &ui, &optional);
    finish_one(&mut host);
    revision(&adopt(&mut host, &ui, &handle), 1);
    assert!(matches!(
        error(object_reply(&mut host, &ui, "scenario", "OPTIONAL")).code,
        ErrorCode::MissingReference
    ));
    let mut first: Value = serde_json::from_slice(&source("scenario")).unwrap();
    first = first["data"].clone();
    first["id"] = json!("OPTIONAL");
    first["revision"] = json!(1);
    first["project_revision"] = json!(1);
    revision(
        &ok(apply(&mut host, &ui, 1, json!([put("scenario", first)]))),
        2,
    );
    let created = object(&mut host, &ui, "scenario", "OPTIONAL");
    assert_eq!(created["revision"], 1);
    assert_eq!(created["project_revision"], 2);
}

fn stock_input() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/business-inputs/initial-stock.scenario.json"
    ))
    .unwrap()
}

fn stock_fixture(scratch: &Scratch) -> (ProjectHost, ObjectId) {
    let directory = scratch.directory("stock-seed");
    seed_archive(&directory);
    let mut project = read_json(&directory.join("active/project.json"));
    for (name, title, unit, tracking) in [
        ("RAW-STOCK", "declared raw batch", "piece", "batch"),
        ("RAW-BOX", "declared container", "box", "individual"),
    ] {
        project["data"]["item_types"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":name,"revision":"r1","title":title,"unit":unit,"tracking":tracking,"spec_ref":null,"attributes":{},"attribute_schema":[]}));
        for location in project["data"]["locations"].as_array_mut().unwrap() {
            location["accepted_item_types"]
                .as_array_mut()
                .unwrap()
                .push(json!(name));
        }
    }
    write_json(&directory.join("active/project.json"), &project);
    write_json(&directory.join("snapshots/project.json"), &project);
    let ui = id("w5-stock-ui");
    let mut host = ProjectHost::new(id("w5-stock-host"), ui.clone());
    let handle = import(&mut host, &ui, &directory);
    finish_one(&mut host);
    revision(&adopt(&mut host, &ui, &handle), 1);
    load_business(&mut host, &ui);
    (host, ui)
}

fn stock_put(host: &mut ProjectHost, ui: &ObjectId, base: u64, mut value: Value) -> Value {
    value["project_revision"] = json!(base);
    value["revision"] = if base == 2 {
        json!(1)
    } else {
        object(host, ui, "scenario", "STOCK")["revision"].clone()
    };
    revision(
        &ok(apply(host, ui, base, json!([put("scenario", value)]))),
        base + 1,
    );
    object(host, ui, "scenario", "STOCK")
}

fn assert_stock_content(mut actual: Value, mut expected: Value) {
    for key in ["revision", "project_revision"] {
        actual.as_object_mut().unwrap().remove(key);
        expected.as_object_mut().unwrap().remove(key);
    }
    assert_eq!(actual, expected);
}

fn material_issues(host: &mut ProjectHost, ui: &ObjectId) -> Vec<Issue> {
    context(host, ui)
        .issues
        .into_iter()
        .filter(|issue| issue.code.starts_with("business_inputs.material_"))
        .collect()
}

fn assert_material_blocked(host: &mut ProjectHost, ui: &ObjectId, prefix: &str) {
    let scenario = object(host, ui, "scenario", "STOCK");
    let issues = material_issues(host, ui);
    assert!(
        issues.iter().any(|issue| issue
            .field_path
            .as_ref()
            .is_some_and(|path| path.starts_with(prefix))),
        "missing material issue at {prefix}: {issues:?}"
    );
    for issue in issues {
        let owner = issue.object_ref.as_ref().unwrap();
        assert!(matches!(owner.kind, ObjectKind::Scenario));
        assert_eq!(owner.id.as_str(), "STOCK");
        let pointer = issue.field_path.as_ref().unwrap();
        assert!(
            scenario.pointer(pointer).is_some(),
            "unresolved pointer {pointer}"
        );
        assert!(
            issue
                .blocks
                .iter()
                .any(|p| matches!(p, CapabilityPurpose::Execution))
        );
        assert!(
            issue
                .blocks
                .iter()
                .any(|p| matches!(p, CapabilityPurpose::CompleteConclusion))
        );
    }
}

#[test]
fn initial_stock_future_supply_demand_and_plans_remain_separate_without_finished_quantity_cap() {
    let scratch = Scratch::new();
    let (mut host, ui) = stock_fixture(&scratch);
    let input = stock_input();
    let actual = stock_put(&mut host, &ui, 2, input.clone());
    assert_stock_content(actual.clone(), input);
    assert_eq!(actual["revision"], 1);
    assert_eq!(actual["project_revision"], 3);
    assert_eq!(
        actual["initial_conditions"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        actual["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"],
        10
    );
    assert_eq!(
        actual["initial_conditions"]["items"][0]["quality"],
        "pending"
    );
    assert_eq!(
        actual["arrivals"][0]["items"][0]["quantity"]["value"]["coefficient"],
        5
    );
    assert_eq!(
        actual["supplies"][0]["target_quantity"]["value"]["coefficient"],
        20
    );
    let demand: u64 = actual["orders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|order| order["quantity"]["value"]["coefficient"].as_u64().unwrap())
        .sum();
    assert_eq!(demand, 8);
    assert_clean(&mut host, &ui);
    // Removing one declaration does not release quality or manufacture a
    // derived inventory query. D06-C03's 4/6/0 remains a design expectation.
    let mut partial = actual;
    partial["initial_conditions"]["material_allocations"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let actual = stock_put(&mut host, &ui, 3, partial.clone());
    assert_stock_content(actual, partial);
    assert_clean(&mut host, &ui);
}

#[test]
fn material_allocation_semantic_gaps_commit_new_revision_with_execution_and_conclusion_blocks() {
    let scratch = Scratch::new();
    let (mut host, ui) = stock_fixture(&scratch);
    stock_put(&mut host, &ui, 2, stock_input());
    let mut variants = Vec::new();
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][1]["quantity"]["value"]["coefficient"] =
        json!(7);
    variants.push(("overallocated", value));
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][0]["item_refs"] =
        json!(["BATCH10", "BATCH10"]);
    variants.push(("duplicate source", value));
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][1]["id"] = json!("HARD-A");
    variants.push(("duplicate allocation identity", value));
    let mut multi = stock_input();
    let mut second = multi["initial_conditions"]["items"][0].clone();
    second["id"] = json!("SECOND2");
    second["quantity"]["value"]["coefficient"] = json!(2);
    second["holder"] = json!({"kind":"location","id":"S2"});
    multi["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(second);
    multi["initial_conditions"]["material_allocations"][0]["item_refs"] =
        json!(["BATCH10", "SECOND2"]);
    multi["initial_conditions"]["material_allocations"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    variants.push(("multiple sources only partially assigned", multi.clone()));
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][0]["quantity"]["unit"] = json!("kg");
    variants.push(("wrong quantity unit", value));
    let mut value = multi.clone();
    value["initial_conditions"]["items"][1]["item_type_ref"]["revision"] = json!("r2");
    value["initial_conditions"]["material_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(12);
    variants.push(("mixed exact revisions", value));
    let mut value = multi.clone();
    value["initial_conditions"]["items"][1]["item_type_ref"] =
        json!({"id":"RAW-BOX","revision":"r1"});
    value["initial_conditions"]["items"][1]["quantity"]["unit"] = json!("box");
    value["initial_conditions"]["material_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(12);
    variants.push(("mixed source units", value));
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][0]["demand_ref"] = json!("ORDER-A");
    variants.push(("order identity is not a hard reservation task", value));
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][0]["mode"] = json!("committed");
    variants.push(("runtime allocation mode at initial input", value));
    let mut base = 3;
    for (name, candidate) in variants {
        let actual = stock_put(&mut host, &ui, base, candidate.clone());
        assert_stock_content(actual, candidate);
        assert_material_blocked(&mut host, &ui, "/initial_conditions/");
        assert_eq!(
            object(&mut host, &ui, "project", "P1")["revision"],
            base + 1,
            "{name}"
        );
        base += 1;
    }
    // Twelve raw pieces for a task producing four finished pieces is valid.
    multi["initial_conditions"]["material_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(12);
    let actual = stock_put(&mut host, &ui, base, multi.clone());
    assert_stock_content(actual, multi);
    assert_clean(&mut host, &ui);
}

#[test]
fn future_arrival_and_supply_gaps_are_located_drafts_without_synthesized_stock() {
    let scratch = Scratch::new();
    let (mut host, ui) = stock_fixture(&scratch);
    stock_put(&mut host, &ui, 2, stock_input());
    let mut variants = Vec::new();
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][0]["item_refs"] = json!(["FUTURE5"]);
    value["initial_conditions"]["material_allocations"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    variants.push(("/initial_conditions/material_allocations", value));
    let mut value = stock_input();
    value["initial_conditions"]["material_allocations"][0]["item_refs"] = json!(["UNKNOWN-STOCK"]);
    variants.push(("/initial_conditions/material_allocations", value));
    let mut value = stock_input();
    value["initial_conditions"]["items"][0]["holder"]["id"] = json!("UNKNOWN-LOCATION");
    variants.push(("/initial_conditions/items", value));
    let mut value = stock_input();
    value["arrivals"][0]["destination"] = json!("UNKNOWN-LOCATION");
    variants.push(("/arrivals", value));
    let mut value = stock_input();
    value["supplies"][0]["source"] = json!("UNKNOWN-LOCATION");
    variants.push(("/supplies", value));
    let mut value = stock_input();
    value["supplies"][0]["resources"]["worker"] = json!("UNKNOWN-WORKER");
    variants.push(("/supplies", value));
    let mut value = stock_input();
    value["supplies"][0]["target_quantity"]["unit"] = json!("kg");
    variants.push(("/supplies", value));
    let mut value = stock_input();
    value["arrivals"][0]["items"][0]["item_type_ref"]["revision"] = json!("r2");
    variants.push(("/arrivals", value));
    let mut missing_transport = stock_input();
    missing_transport["supplies"][0]["transport_ref"] =
        json!({"id":"missing.transport","version":"9.0.0"});
    variants.push(("/supplies/0/transport_ref", missing_transport.clone()));
    let mut base = 3;
    for (pointer, candidate) in variants {
        let actual = stock_put(&mut host, &ui, base, candidate.clone());
        assert_stock_content(actual.clone(), candidate.clone());
        assert_material_blocked(&mut host, &ui, pointer);
        assert_eq!(
            actual["initial_conditions"]["items"],
            candidate["initial_conditions"]["items"]
        );
        assert_eq!(actual["arrivals"], candidate["arrivals"]);
        base += 1;
    }
    assert!(
        material_issues(&mut host, &ui)
            .iter()
            .any(
                |issue| issue.code == "business_inputs.material_missing_transport_module"
                    && issue.field_path.as_deref() == Some("/supplies/0/transport_ref")
            )
    );
    let directory = scratch.directory("transport-gap-saved");
    let copied = scratch.directory("transport-gap-reopened");
    let selected = host
        .select_directory(&ui, FilePurpose::SaveProject, directory.clone())
        .unwrap();
    let handle = save(&mut host, &ui, base, &selected);
    finish_one(&mut host);
    assert!(!saved(&mut host, &ui, &handle).still_dirty);
    assert_stock_content(
        disk_document(&directory, "scenario_input", "STOCK"),
        missing_transport.clone(),
    );
    copy_directory(&directory, &copied);
    let handle = import(&mut host, &ui, &copied);
    finish_one(&mut host);
    let draft = imported(&mut host, &ui, &handle);
    assert!(
        draft
            .missing
            .iter()
            .any(|issue| issue.code == "business_inputs.material_missing_transport_module")
    );
    revision(
        &ok(adopt_reply(&mut host, &ui, &handle, &draft, true)),
        base,
    );
    assert_stock_content(
        object(&mut host, &ui, "scenario", "STOCK"),
        missing_transport,
    );
    assert_material_blocked(&mut host, &ui, "/supplies/0/transport_ref");
}

fn container_stock() -> Value {
    let mut scenario = stock_input();
    let mut child4 = scenario["initial_conditions"]["items"][0].clone();
    child4["id"] = json!("CHILD4");
    child4["quantity"]["value"]["coefficient"] = json!(4);
    child4["holder"] = json!({"kind":"container","id":"BOX1"});
    let mut child6 = child4.clone();
    child6["id"] = json!("CHILD6");
    child6["quantity"]["value"]["coefficient"] = json!(6);
    let mut container = child4.clone();
    container["id"] = json!("BOX1");
    container["item_type_ref"] = json!({"id":"RAW-BOX","revision":"r1"});
    container["quantity"] = json!({"value":{"coefficient":1,"decimal_places":0},"unit":"box"});
    container["holder"] = json!({"kind":"location","id":"S1"});
    container["quality"] = json!("released");
    container["contents"] = json!(["CHILD4", "CHILD6"]);
    scenario["initial_conditions"]["items"] = json!([container, child4, child6]);
    scenario["initial_conditions"]["material_allocations"][0]["item_refs"] = json!(["CHILD4"]);
    scenario["initial_conditions"]["material_allocations"][1]["item_refs"] = json!(["CHILD6"]);
    scenario
}

#[test]
fn container_holder_and_composition_relations_are_input_graphs_without_duplicate_physical_ownership()
 {
    let scratch = Scratch::new();
    let (mut host, ui) = stock_fixture(&scratch);
    let positive = container_stock();
    let actual = stock_put(&mut host, &ui, 2, positive.clone());
    assert_stock_content(actual, positive.clone());
    assert_clean(&mut host, &ui);
    let mut variants = Vec::new();
    let mut value = positive.clone();
    let mut other = value["initial_conditions"]["items"][0].clone();
    other["id"] = json!("BOX2");
    other["contents"] = json!(["CHILD4"]);
    value["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(other);
    variants.push(value);
    let mut value = positive.clone();
    value["initial_conditions"]["items"][1]["holder"] = json!({"kind":"location","id":"S1"});
    variants.push(value);
    let mut value = positive.clone();
    value["initial_conditions"]["items"][0]["contents"]
        .as_array_mut()
        .unwrap()
        .push(json!("BOX1"));
    value["initial_conditions"]["items"][0]["holder"] = json!({"kind":"container","id":"BOX1"});
    variants.push(value);
    let mut value = positive.clone();
    let mut other = value["initial_conditions"]["items"][0].clone();
    other["id"] = json!("BOX2");
    other["contents"] = json!(["BOX1"]);
    other["holder"] = json!({"kind":"container","id":"BOX1"});
    value["initial_conditions"]["items"][0]["holder"] = json!({"kind":"container","id":"BOX2"});
    value["initial_conditions"]["items"][0]["contents"]
        .as_array_mut()
        .unwrap()
        .push(json!("BOX2"));
    value["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(other);
    variants.push(value);
    let mut value = positive.clone();
    value["initial_conditions"]["items"][1]["components"] = json!(["UNKNOWN-COMPONENT"]);
    variants.push(value);
    for holder in ["job", "transport"] {
        let mut value = positive.clone();
        value["initial_conditions"]["items"][1]["holder"] =
            json!({"kind":holder,"id":"RUNTIME-ONLY"});
        value["initial_conditions"]["items"][0]["contents"] = json!(["CHILD6"]);
        variants.push(value);
    }
    let mut value = positive;
    value["initial_conditions"]["items"][1]["holder"] =
        json!({"kind":"container","id":"UNKNOWN-CONTAINER"});
    value["initial_conditions"]["items"][0]["contents"] = json!(["CHILD6"]);
    variants.push(value);
    let mut base = 3;
    for candidate in variants {
        let actual = stock_put(&mut host, &ui, base, candidate.clone());
        assert_stock_content(actual, candidate);
        assert_material_blocked(&mut host, &ui, "/initial_conditions/items");
        base += 1;
    }
    // Composition sources are records in the same source group. Their normal
    // Location holder is historical; it is not a second physical owner.
    let mut composed = container_stock();
    let mut source_record = composed["initial_conditions"]["items"][1].clone();
    source_record["id"] = json!("SOURCE2");
    source_record["quantity"]["value"]["coefficient"] = json!(2);
    source_record["holder"] = json!({"kind":"location","id":"S2"});
    source_record["quality"] = json!("released");
    let mut assembly = source_record.clone();
    assembly["id"] = json!("ASSEMBLY1");
    assembly["quantity"]["value"]["coefficient"] = json!(1);
    assembly["components"] = json!(["SOURCE2"]);
    composed["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .extend([source_record, assembly]);
    let actual = stock_put(&mut host, &ui, base, composed.clone());
    assert_stock_content(actual.clone(), composed.clone());
    assert_eq!(
        actual["initial_conditions"]["items"][3]["holder"],
        json!({"kind":"location","id":"S2"})
    );
    assert_clean(&mut host, &ui);
    base += 1;
    let mut composition_gaps = Vec::new();
    let mut value = composed.clone();
    value["initial_conditions"]["items"][4]["components"] = json!(["FUTURE5"]);
    composition_gaps.push(value);
    let mut value = composed.clone();
    let mut second_assembly = value["initial_conditions"]["items"][4].clone();
    second_assembly["id"] = json!("ASSEMBLY2");
    value["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(second_assembly);
    composition_gaps.push(value);
    let mut value = composed.clone();
    let mut second_assembly = value["initial_conditions"]["items"][4].clone();
    second_assembly["id"] = json!("ASSEMBLY2");
    second_assembly["components"] = json!(["ASSEMBLY1"]);
    value["initial_conditions"]["items"][4]["components"] = json!(["ASSEMBLY2"]);
    value["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(second_assembly);
    composition_gaps.push(value);
    let mut value = composed.clone();
    // Keep the contents/holder pair internally consistent, so this negative
    // cannot pass merely because the old container validator sees a mismatch.
    value["initial_conditions"]["items"][3]["holder"] = json!({"kind":"container","id":"BOX1"});
    value["initial_conditions"]["items"][0]["contents"]
        .as_array_mut()
        .unwrap()
        .push(json!("SOURCE2"));
    composition_gaps.push(value);
    let mut value = composed;
    value["initial_conditions"]["material_allocations"].as_array_mut().unwrap().push(json!({
        "id":"COMPONENT-HARD","demand_ref":"TASK-A","item_refs":["SOURCE2"],
        "quantity":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"
    }));
    composition_gaps.push(value);
    for candidate in composition_gaps {
        let actual = stock_put(&mut host, &ui, base, candidate.clone());
        assert_stock_content(actual, candidate);
        assert_material_blocked(&mut host, &ui, "/initial_conditions/");
        base += 1;
    }
    let before = snapshot(&mut host, &ui);
    let mut duplicate = object(&mut host, &ui, "scenario", "STOCK");
    let mut repeated = duplicate["initial_conditions"]["items"][1].clone();
    repeated["holder"] = json!({"kind":"location","id":"S2"});
    duplicate["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(repeated);
    let mut product = input()["products"][0].clone();
    product["id"] = json!("STOCK-NOT-COMMITTED");
    assert!(matches!(
        error(apply(
            &mut host,
            &ui,
            base,
            json!([put("product", product), put("scenario", duplicate)])
        ))
        .code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    assert!(host.drain_io_work().is_empty());
}

#[test]
fn material_scenario_history_frozen_save_reopen_and_rejected_batches_preserve_atomic_state() {
    let scratch = Scratch::new();
    let (mut host, ui) = stock_fixture(&scratch);
    let original = stock_input();
    stock_put(&mut host, &ui, 2, original.clone());
    let old = frozen_documents(&scratch.path.join("stock-seed"));
    let mut changed = original.clone();
    changed["initial_conditions"]["material_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(3);
    changed["initial_conditions"]["material_allocations"][1]["quantity"]["value"]["coefficient"] =
        json!(7);
    changed["arrivals"][0]["items"][0]["quantity"]["value"]["coefficient"] = json!(6);
    changed["supplies"][0]["target_quantity"]["value"]["coefficient"] = json!(21);
    changed["orders"][0]["quantity"]["value"]["coefficient"] = json!(5);
    changed["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(5);
    changed["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(5);
    stock_put(&mut host, &ui, 3, changed.clone());
    revision(&ok(history(&mut host, &ui, 4, "undo")), 5);
    let actual = object(&mut host, &ui, "scenario", "STOCK");
    assert_eq!(actual["revision"], 3);
    assert_stock_content(actual, original);
    revision(&ok(history(&mut host, &ui, 5, "redo")), 6);
    let actual = object(&mut host, &ui, "scenario", "STOCK");
    assert_eq!(actual["revision"], 4);
    assert_stock_content(actual, changed.clone());
    changed["initial_conditions"]["material_allocations"][1]["quantity"]["value"]["coefficient"] =
        json!(8);
    let frozen = stock_put(&mut host, &ui, 6, changed.clone());
    assert_material_blocked(&mut host, &ui, "/initial_conditions/material_allocations");
    let directory = scratch.directory("stock-saved");
    let copied = scratch.directory("stock-reopened");
    let selected = host
        .select_directory(&ui, FilePurpose::SaveProject, directory.clone())
        .unwrap();
    let handle = save(&mut host, &ui, 7, &selected);
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    let mut later = changed.clone();
    later["title"] = json!("edited after frozen stock input");
    stock_put(&mut host, &ui, 7, later);
    host.finish_io(work.pop().unwrap().run());
    let result = saved(&mut host, &ui, &handle);
    assert_eq!(result.saved_revision.0.value(), 7);
    assert_eq!(result.current_revision.0.value(), 8);
    assert!(result.still_dirty);
    assert_eq!(disk_document(&directory, "scenario_input", "STOCK"), frozen);
    assert_eq!(frozen_documents(&directory), old);
    copy_directory(&directory, &copied);
    let handle = import(&mut host, &ui, &copied);
    finish_one(&mut host);
    let draft = imported(&mut host, &ui, &handle);
    let before = snapshot(&mut host, &ui);
    assert!(matches!(
        error(adopt_reply(&mut host, &ui, &handle, &draft, false)).code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    revision(&ok(adopt_reply(&mut host, &ui, &handle, &draft, true)), 7);
    assert_eq!(object(&mut host, &ui, "scenario", "STOCK"), frozen);
    assert_material_blocked(&mut host, &ui, "/initial_conditions/material_allocations");
    let mut edited = object(&mut host, &ui, "scenario", "STOCK");
    edited["title"] = json!("one legitimate edit before rejected batches");
    stock_put(&mut host, &ui, 7, edited.clone());
    let session = id("w5-stock-external");
    let mut g = grant(vec![HostEditBranch::PutObject]);
    g.creates.push(HostCreateAllowance {
        kind: ObjectKind::Product,
        max_new_objects: SafeUint::try_from(1).unwrap(),
        fields: [
            "/revision",
            "/item_type_ref/id",
            "/item_type_ref/revision",
            "/bom_ref/id",
            "/bom_ref/revision",
            "/routing_ref/id",
            "/routing_ref/revision",
        ]
        .into_iter()
        .map(field)
        .collect(),
    });
    g.objects.push(HostObjectEditGrant {
        target: ObjectRef {
            kind: ObjectKind::Scenario,
            id: id("STOCK"),
        },
        fields: vec![field("/window/end_ms")],
        can_remove: false,
    });
    host.register_external_session(external(&session, g))
        .unwrap();
    let before = snapshot(&mut host, &ui);
    let before_ledger = ledger(&host, &session);
    let mut product = input()["products"][0].clone();
    product["id"] = json!("STOCK-NOT-COMMITTED");
    let mut denied = object(&mut host, &ui, "scenario", "STOCK");
    denied["title"] = json!("unauthorized title");
    assert!(matches!(
        error(apply(
            &mut host,
            &session,
            8,
            json!([put("product", product.clone()), put("scenario", denied)])
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    assert_eq!(ledger(&host, &session), before_ledger);
    assert!(matches!(
        error(apply(
            &mut host,
            &ui,
            7,
            json!([put("product", product.clone())])
        ))
        .code,
        ErrorCode::RevisionConflict
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    let mut duplicate = object(&mut host, &ui, "scenario", "STOCK");
    let repeated = duplicate["initial_conditions"]["items"][0].clone();
    duplicate["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(repeated);
    assert!(matches!(
        error(apply(
            &mut host,
            &ui,
            8,
            json!([put("product", product.clone()), put("scenario", duplicate)])
        ))
        .code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(snapshot(&mut host, &ui), before);
    assert_eq!(ledger(&host, &session), before_ledger);
    revision(&ok(history(&mut host, &ui, 8, "undo")), 9);
    assert_stock_content(object(&mut host, &ui, "scenario", "STOCK"), changed);
    revision(&ok(history(&mut host, &ui, 9, "redo")), 10);
    assert_stock_content(object(&mut host, &ui, "scenario", "STOCK"), edited);
    revision(
        &ok(apply(
            &mut host,
            &session,
            10,
            json!([put("product", product)]),
        )),
        11,
    );
    assert_eq!(
        ledger(&host, &session)[0]["created_objects"],
        json!([{"kind":"product","id":"STOCK-NOT-COMMITTED"}])
    );
    assert_eq!(frozen_documents(&copied), old);
}
