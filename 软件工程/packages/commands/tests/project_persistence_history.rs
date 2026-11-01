//! Independent P0-01 steps 3/4 expectations: real Host dispatch and real directories.
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(1);

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
            "sf-p0-01-window5-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
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
        // Only the uniquely created canonical child of the actual temp directory.
        if self.path.parent() != Some(self.base.as_path()) {
            eprintln!("refusing cleanup outside test temp root: {:?}", self.path);
            return;
        }
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!("test fixture cleanup failed for {:?}: {error}", self.path);
        }
    }
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
            assert!(kind.is_file(), "fixture must not contain links");
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn active_document(directory: &Path) -> PathBuf {
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("project.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], "1.0");
    let entries = manifest["data"]["documents"].as_array().unwrap();
    let active: Vec<_> = entries
        .iter()
        .filter(|entry| entry["role"] == "active_project")
        .collect();
    assert_eq!(active.len(), 1);
    let relative = Path::new(active[0]["file_ref"]["path"].as_str().unwrap());
    assert!(relative.is_relative());
    assert!(
        !relative
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    );
    let document = directory.join(relative).canonicalize().unwrap();
    assert!(document.starts_with(directory.canonicalize().unwrap()));
    document
}

fn disk_project(directory: &Path) -> Value {
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("project.json")).unwrap()).unwrap();
    let document: Value =
        serde_json::from_slice(&fs::read(active_document(directory)).unwrap()).unwrap();
    assert_eq!(document["schema_version"], "1.0");
    assert_eq!(document["data"]["id"], manifest["data"]["project_id"]);
    assert_eq!(
        document["data"]["revision"],
        manifest["data"]["project_revision"]
    );
    document["data"].clone()
}

fn call(
    host: &mut ProjectHost,
    session: &ObjectId,
    project: Option<&str>,
    operation: &str,
    params: Value,
) -> Value {
    let request: ApiRequest = serde_json::from_value(json!({
        "api_version":"0.1", "request_id":format!("w5-ph-{}",SEQUENCE.fetch_add(1,Ordering::Relaxed)),
        "project_id":project, "scope":{"kind":if project.is_some(){"project"}else{"workspace"}},
        "operation":operation, "params":params
    })).unwrap();
    host.dispatch(session, request)
}

fn ok<T: DeserializeOwned>(reply: Value) -> T {
    let response: Response<T> = serde_json::from_value(reply.clone()).unwrap();
    match response.outcome {
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
        _ => panic!("expected failure: {reply:#}"),
    }
}

fn apply(
    host: &mut ProjectHost,
    session: &ObjectId,
    project: Option<&str>,
    revision: Option<u64>,
    commands: Value,
) -> Value {
    call(
        host,
        session,
        project,
        "apply_commands",
        json!({"base_revision":revision,"commands":commands}),
    )
}

fn settings(title: &str) -> Value {
    json!({"kind":"set_project_settings","data":{"title":title,"time_reference":{"label":"relative milliseconds","origin_iso8601":null,"timezone":null},"generation_context":null,"sources":[]}})
}

fn item(item_id: &str) -> Value {
    json!({"kind":"put_object","data":{"kind":"item_type","data":{"id":item_id,"revision":"r1","title":format!("part {item_id}"),"unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]}}})
}

fn create(host: &mut ProjectHost, ui: &ObjectId, title: &str) -> String {
    let commit: EditCommit = ok(apply(
        host,
        ui,
        None,
        None,
        json!([{"kind":"create_project","data":{"title":title,"time_reference":{"label":"relative milliseconds","origin_iso8601":null,"timezone":null},"departure":"discard_after_tasks_terminal"}}]),
    ));
    assert_eq!(commit.new_revision.0.value(), 1);
    commit.project_id.as_str().to_owned()
}

fn fixture() -> (ProjectHost, ObjectId, String) {
    let ui = id("w5-ph-ui");
    let mut host = ProjectHost::new(id("w5-ph-host"), ui.clone());
    let project = create(&mut host, &ui, "initial");
    (host, ui, project)
}

fn project(host: &mut ProjectHost, ui: &ObjectId, project: &str) -> Value {
    let object: DomainObject = ok(call(
        host,
        ui,
        Some(project),
        "get_object",
        json!({"object_id":{"kind":"project","id":project},"source":{"kind":"draft"},"node_index":null}),
    ));
    match object {
        DomainObject::Project(value) => serde_json::to_value(*value).unwrap(),
        other => panic!("wrong object: {other:?}"),
    }
}

fn summary(host: &mut ProjectHost, ui: &ObjectId, project: &str) -> ProjectSummary {
    let context: ProjectContext = ok(call(
        host,
        ui,
        Some(project),
        "get_context",
        json!({"selection":[]}),
    ));
    context.project.unwrap()
}

fn revision(commit: &EditCommit, expected: u64) {
    assert_eq!(commit.new_revision.0.value(), expected);
}

fn edit(
    host: &mut ProjectHost,
    ui: &ObjectId,
    project: &str,
    base: u64,
    title: &str,
) -> EditCommit {
    ok(apply(
        host,
        ui,
        Some(project),
        Some(base),
        json!([settings(title)]),
    ))
}

fn history(
    host: &mut ProjectHost,
    caller: &ObjectId,
    project: &str,
    base: u64,
    branch: &str,
) -> Value {
    apply(
        host,
        caller,
        Some(project),
        Some(base),
        json!([{"kind":branch}]),
    )
}

fn select(
    host: &mut ProjectHost,
    caller: &ObjectId,
    purpose: FilePurpose,
    directory: &Path,
) -> SelectedFileRef {
    host.select_directory(caller, purpose, directory.to_path_buf())
        .unwrap()
}

fn save(
    host: &mut ProjectHost,
    caller: &ObjectId,
    project: &str,
    revision: u64,
    selection: &SelectedFileRef,
) -> TaskHandle {
    ok(call(
        host,
        caller,
        Some(project),
        "save_project",
        json!({"revision":revision,"selected_directory_ref":selection}),
    ))
}

fn task(
    host: &mut ProjectHost,
    caller: &ObjectId,
    project: Option<&str>,
    handle: &TaskHandle,
) -> TaskState {
    ok(call(
        host,
        caller,
        project,
        "get_task",
        json!({"task_id":handle.id}),
    ))
}

fn queued(host: &mut ProjectHost, caller: &ObjectId, project: Option<&str>, handle: &TaskHandle) {
    assert!(matches!(handle.state, ComputationStatus::Queued));
    assert!(handle.result_ref.is_none());
    let state = task(host, caller, project, handle);
    assert!(matches!(state.task.state, ComputationStatus::Queued));
    assert!(state.result.is_none() && state.failure.is_none());
}

fn finish_one(host: &mut ProjectHost) {
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    let completion = work.pop().unwrap().run();
    host.finish_io(completion);
}

fn saved(
    host: &mut ProjectHost,
    ui: &ObjectId,
    project: &str,
    handle: &TaskHandle,
) -> SavedProject {
    let state = task(host, ui, Some(project), handle);
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert!(state.failure.is_none());
    match state.result.unwrap() {
        TaskOutput::Save(result) => result,
        other => panic!("wrong output: {other:?}"),
    }
}

fn seed_saved(
    host: &mut ProjectHost,
    ui: &ObjectId,
    project: &str,
    directory: &Path,
) -> SelectedFileRef {
    let commit: EditCommit = ok(apply(
        host,
        ui,
        Some(project),
        Some(1),
        json!([settings("saved-r2"), item("PART-A")]),
    ));
    revision(&commit, 2);
    let selection = select(host, ui, FilePurpose::SaveProject, directory);
    let handle = save(host, ui, project, 2, &selection);
    queued(host, ui, Some(project), &handle);
    finish_one(host);
    assert!(!saved(host, ui, project, &handle).still_dirty);
    selection
}

fn import(host: &mut ProjectHost, ui: &ObjectId, directory: &Path) -> TaskHandle {
    let selection = select(host, ui, FilePurpose::ImportPackage, directory);
    let handle: TaskHandle = ok(call(
        host,
        ui,
        None,
        "import_package",
        json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null}),
    ));
    assert!(matches!(&handle.scope,TaskScope::Workspace(session) if session==ui));
    assert_eq!(handle.input_revision.0.value(), 0);
    queued(host, ui, None, &handle);
    handle
}

fn imported(host: &mut ProjectHost, ui: &ObjectId, handle: &TaskHandle) -> ImportDraft {
    let state = task(host, ui, None, handle);
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert!(state.failure.is_none());
    match state.result.unwrap() {
        TaskOutput::Import(draft) => draft,
        other => panic!("wrong output: {other:?}"),
    }
}

fn open_choices(draft: &ImportDraft, accept_missing: bool) -> Value {
    json!({"chosen_policy":"open_project","objects_to_create":draft.proposed_objects,"module_versions":draft.proposed_definitions,"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":accept_missing,"departure":"discard_after_tasks_terminal"})
}

fn adopt(
    host: &mut ProjectHost,
    ui: &ObjectId,
    handle: &TaskHandle,
    draft: &ImportDraft,
    accept_missing: bool,
) -> Value {
    apply(
        host,
        ui,
        None,
        None,
        json!([{"kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":open_choices(draft, accept_missing)}}]),
    )
}

#[test]
fn frozen_save_finishes_at_r_while_r_plus_one_stays_dirty_and_save_keeps_undo() {
    let scratch = Scratch::new();
    let directory = scratch.directory("project");
    let (mut host, ui, p) = fixture();
    let initial: EditCommit = ok(apply(
        &mut host,
        &ui,
        Some(&p),
        Some(1),
        json!([settings("frozen-r2"), item("PART-A")]),
    ));
    revision(&initial, 2);
    let frozen = project(&mut host, &ui, &p);
    let selection = select(&mut host, &ui, FilePurpose::SaveProject, &directory);
    let handle = save(&mut host, &ui, &p, 2, &selection);
    queued(&mut host, &ui, Some(&p), &handle);
    let mut works = host.drain_io_work();
    assert_eq!(works.len(), 1);
    let work = works.pop().unwrap();
    queued(&mut host, &ui, Some(&p), &handle);
    revision(&edit(&mut host, &ui, &p, 2, "edited-r3"), 3);
    let completion = work.run();
    assert_eq!(disk_project(&directory), frozen);
    let running = task(&mut host, &ui, Some(&p), &handle);
    assert!(matches!(running.task.state, ComputationStatus::Running));
    assert!(running.result.is_none());
    host.finish_io(completion);
    let result = saved(&mut host, &ui, &p, &handle);
    assert_eq!(result.saved_revision.0.value(), 2);
    assert_eq!(result.current_revision.0.value(), 3);
    assert!(result.still_dirty);
    assert!(summary(&mut host, &ui, &p).dirty);
    let next = save(&mut host, &ui, &p, 3, &selection);
    queued(&mut host, &ui, Some(&p), &next);
    finish_one(&mut host);
    let result = saved(&mut host, &ui, &p, &next);
    assert_eq!(result.saved_revision.0.value(), 3);
    assert_eq!(result.current_revision.0.value(), 3);
    assert!(!result.still_dirty);
    assert!(!summary(&mut host, &ui, &p).dirty);
    let disk = disk_project(&directory);
    assert_eq!(disk["title"], "edited-r3");
    assert_eq!(disk["revision"], 3);
    let undone: EditCommit = ok(history(&mut host, &ui, &p, 3, "undo"));
    revision(&undone, 4);
    assert!(undone.history.can_redo);
    assert_eq!(project(&mut host, &ui, &p)["title"], "frozen-r2");
    assert!(summary(&mut host, &ui, &p).dirty);
    assert_eq!(disk_project(&directory), disk);
}

#[test]
fn copied_directory_import_is_staged_until_explicit_open_then_can_edit_and_save() {
    let scratch = Scratch::new();
    let from = scratch.directory("source");
    let to = scratch.directory("copy");
    let (mut host, ui, p) = fixture();
    seed_saved(&mut host, &ui, &p, &from);
    let original = disk_project(&from);
    copy_directory(&from, &to);
    let active = active_document(&to);
    let mut info: Value = serde_json::from_slice(&fs::read(&active).unwrap()).unwrap();
    info["data"]["issues"] = json!([{"code":"W5_INFO","object_ref":null,"field_path":null,"message":"ordinary information does not require missing-draft acceptance","blocks":[]}]);
    fs::write(&active, serde_json::to_vec(&info).unwrap()).unwrap();
    let expected = disk_project(&to);
    revision(
        &edit(&mut host, &ui, &p, 2, "keep-current-until-adopted"),
        3,
    );
    let before = project(&mut host, &ui, &p);
    let handle = import(&mut host, &ui, &to);
    finish_one(&mut host);
    let draft = imported(&mut host, &ui, &handle);
    assert!(draft.missing.is_empty());
    assert_eq!(project(&mut host, &ui, &p), before);
    assert!(summary(&mut host, &ui, &p).dirty);
    assert!(draft.proposed_objects.len() > 1);
    let mut subset = open_choices(&draft, false);
    subset["objects_to_create"].as_array_mut().unwrap().pop();
    let mut duplicate = open_choices(&draft, false);
    duplicate["objects_to_create"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(&draft.proposed_objects[0]).unwrap());
    let mut remapped = open_choices(&draft, false);
    remapped["reused_bindings"]["item_types"] = json!({"W5-role":"PART-A"});
    for choices in [subset, duplicate, remapped] {
        let failure = error(apply(
            &mut host,
            &ui,
            None,
            None,
            json!([{"kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":choices}}]),
        ));
        assert!(matches!(failure.code, ErrorCode::UnsupportedCapability));
        assert_eq!(project(&mut host, &ui, &p), before);
        assert!(summary(&mut host, &ui, &p).dirty);
    }
    let undone: EditCommit = ok(history(&mut host, &ui, &p, 3, "undo"));
    revision(&undone, 4);
    assert_eq!(project(&mut host, &ui, &p)["title"], "saved-r2");
    let redone: EditCommit = ok(history(&mut host, &ui, &p, 4, "redo"));
    revision(&redone, 5);
    assert_eq!(
        project(&mut host, &ui, &p)["title"],
        "keep-current-until-adopted"
    );
    let committed: EditCommit = ok(adopt(&mut host, &ui, &handle, &draft, false));
    assert_eq!(committed.project_id.as_str(), p);
    assert!(!committed.history.can_undo && !committed.history.can_redo);
    assert_eq!(project(&mut host, &ui, &p), expected);
    let loaded = summary(&mut host, &ui, &p);
    assert!(!loaded.dirty);
    let rejected = history(&mut host, &ui, &p, loaded.revision.0.value(), "undo");
    error(rejected);
    assert_eq!(project(&mut host, &ui, &p), expected);
    let base = loaded.revision.0.value();
    revision(
        &edit(&mut host, &ui, &p, base, "continued-after-open"),
        base + 1,
    );
    let selection = select(&mut host, &ui, FilePurpose::SaveProject, &to);
    let handle = save(&mut host, &ui, &p, base + 1, &selection);
    finish_one(&mut host);
    assert!(!saved(&mut host, &ui, &p, &handle).still_dirty);
    assert_eq!(disk_project(&to), project(&mut host, &ui, &p));
    assert_eq!(disk_project(&from), original);
}

#[test]
fn corrupt_or_missing_required_documents_leave_project_revision_and_history_intact() {
    let scratch = Scratch::new();
    let from = scratch.directory("source");
    let (mut host, ui, p) = fixture();
    seed_saved(&mut host, &ui, &p, &from);
    revision(&edit(&mut host, &ui, &p, 2, "current-unsaved"), 3);
    let before = project(&mut host, &ui, &p);
    for name in ["bad-json", "missing-active"] {
        let bad = scratch.directory(name);
        copy_directory(&from, &bad);
        if name == "bad-json" {
            fs::write(bad.join("project.json"), b"{").unwrap();
        } else {
            fs::remove_file(active_document(&bad)).unwrap();
        }
        let handle = import(&mut host, &ui, &bad);
        finish_one(&mut host);
        let state = task(&mut host, &ui, None, &handle);
        assert!(matches!(state.task.state, ComputationStatus::Failed));
        assert!(state.result.is_none() && state.failure.is_some());
        assert_eq!(project(&mut host, &ui, &p), before);
        assert!(summary(&mut host, &ui, &p).dirty);
    }
    let undone: EditCommit = ok(history(&mut host, &ui, &p, 3, "undo"));
    revision(&undone, 4);
    assert_eq!(project(&mut host, &ui, &p)["title"], "saved-r2");
    for name in ["declared-missing", "blocked-project-issue"] {
        let from = scratch.directory(&format!("source-{name}"));
        let candidate = scratch.directory(name);
        let (mut host, ui, p) = fixture();
        seed_saved(&mut host, &ui, &p, &from);
        copy_directory(&from, &candidate);
        let path = if name == "declared-missing" {
            candidate.join("project.json")
        } else {
            active_document(&candidate)
        };
        let mut document: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let issue = json!({"code":"W5_DECLARED_CONDITION","object_ref":null,"field_path":null,"message":"requires explicit draft acceptance","blocks":if name=="declared-missing"{json!([])}else{json!(["complete_conclusion"])}});
        document["data"][if name == "declared-missing" {
            "missing"
        } else {
            "issues"
        }] = json!([issue.clone()]);
        fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        let expected = disk_project(&candidate);
        revision(
            &edit(&mut host, &ui, &p, 2, "keep-until-explicit-acceptance"),
            3,
        );
        let before = project(&mut host, &ui, &p);
        let handle = import(&mut host, &ui, &candidate);
        finish_one(&mut host);
        let draft = imported(&mut host, &ui, &handle);
        if name == "declared-missing" {
            assert!(!draft.missing.is_empty());
        }
        error(adopt(&mut host, &ui, &handle, &draft, false));
        assert_eq!(project(&mut host, &ui, &p), before);
        assert!(summary(&mut host, &ui, &p).dirty);
        let undone: EditCommit = ok(history(&mut host, &ui, &p, 3, "undo"));
        revision(&undone, 4);
        assert_eq!(project(&mut host, &ui, &p)["title"], "saved-r2");
        let adopted: EditCommit = ok(adopt(&mut host, &ui, &handle, &draft, true));
        assert_eq!(adopted.project_id.as_str(), p);
        assert_eq!(project(&mut host, &ui, &p), expected);
        let context: ProjectContext = ok(call(
            &mut host,
            &ui,
            Some(&p),
            "get_context",
            json!({"selection":[]}),
        ));
        let visible_issues = serde_json::to_value(context.issues).unwrap();
        assert!(visible_issues.as_array().unwrap().contains(&issue));
    }
}

fn external(session: &ObjectId, p: &str, grants: Vec<HostEditGrant>) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: session.clone(),
        granted_project: Some(ProjectId::try_from(p.to_owned()).unwrap()),
        allowed_operations: vec![
            OperationName::GetContext,
            OperationName::GetObject,
            OperationName::ApplyCommands,
            OperationName::SaveProject,
            OperationName::GetTask,
            OperationName::CancelTask,
        ],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: grants,
    }
}

#[test]
fn selected_directories_cannot_be_forged_repurposed_or_used_by_another_session() {
    let scratch = Scratch::new();
    let directory = scratch.directory("destination");
    let (mut host, ui, p) = fixture();
    revision(&edit(&mut host, &ui, &p, 1, "unsaved"), 2);
    let before = project(&mut host, &ui, &p);
    let selected = select(&mut host, &ui, FilePurpose::SaveProject, &directory);
    let other = select(&mut host, &ui, FilePurpose::ImportPackage, &directory);
    for token in [
        json!({"selection_id":"W5-unknown-token","purpose":"save_project"}),
        serde_json::to_value(other).unwrap(),
        json!({"selection_id":selected.selection_id,"purpose":"import_package"}),
    ] {
        let failure = error(call(
            &mut host,
            &ui,
            Some(&p),
            "save_project",
            json!({"revision":2,"selected_directory_ref":token}),
        ));
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
        assert_eq!(project(&mut host, &ui, &p), before);
        assert!(host.drain_io_work().is_empty());
    }
    let session = id("w5-cross-session");
    host.register_external_session(external(&session, &p, vec![]))
        .unwrap();
    assert!(matches!(
        error(call(
            &mut host,
            &session,
            Some(&p),
            "save_project",
            json!({"revision":2,"selected_directory_ref":selected})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(project(&mut host, &ui, &p), before);
    assert!(host.drain_io_work().is_empty());
    assert!(!directory.join("project.json").exists());

    let import_directory = scratch.directory("valid-import");
    let (mut writer, writer_ui, writer_project) = fixture();
    seed_saved(&mut writer, &writer_ui, &writer_project, &import_directory);
    let external_ui_only = id("w5-external-workspace-open");
    let mut caller = external(&external_ui_only, &p, vec![]);
    caller.allowed_operations.push(OperationName::ImportPackage);
    host.register_external_session(caller).unwrap();
    let import_token = select(
        &mut host,
        &external_ui_only,
        FilePurpose::ImportPackage,
        &import_directory,
    );
    let registered = host.caller_context(&external_ui_only).unwrap();
    assert!(
        registered
            .allowed_operations
            .iter()
            .any(|op| matches!(op, OperationName::ImportPackage))
    );
    assert!(
        registered
            .selected_file_tokens
            .contains(&import_token.selection_id)
    );
    let context: ProjectContext = ok(call(
        &mut host,
        &external_ui_only,
        Some(&p),
        "get_context",
        json!({"selection":[]}),
    ));
    assert!(
        !context
            .authorized_operations
            .iter()
            .any(|op| matches!(op, OperationName::ImportPackage))
    );
    let failure = error(call(
        &mut host,
        &external_ui_only,
        None,
        "import_package",
        json!({"selected_file_ref":import_token,"import_policy":"open_project","base_revision":null}),
    ));
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    assert_eq!(project(&mut host, &ui, &p), before);
    assert!(host.drain_io_work().is_empty());
}

#[test]
fn cancel_before_io_preserves_old_root_but_committed_work_wins_late_cancel() {
    let scratch = Scratch::new();
    let directory = scratch.directory("project");
    let (mut host, ui, p) = fixture();
    let selected = seed_saved(&mut host, &ui, &p, &directory);
    let old_root = fs::read(directory.join("project.json")).unwrap();
    revision(&edit(&mut host, &ui, &p, 2, "r3-to-save"), 3);
    let cancelled = save(&mut host, &ui, &p, 3, &selected);
    queued(&mut host, &ui, Some(&p), &cancelled);
    let _: CancellationState = ok(call(
        &mut host,
        &ui,
        Some(&p),
        "cancel_task",
        json!({"task_id":cancelled.id,"action":"cancel"}),
    ));
    for work in host.drain_io_work() {
        let completion = work.run();
        host.finish_io(completion);
    }
    let state = task(&mut host, &ui, Some(&p), &cancelled);
    assert!(matches!(state.task.state, ComputationStatus::Cancelled));
    assert!(state.result.is_none());
    assert_eq!(fs::read(directory.join("project.json")).unwrap(), old_root);
    assert!(summary(&mut host, &ui, &p).dirty);
    let completed = save(&mut host, &ui, &p, 3, &selected);
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    let completion = work.pop().unwrap().run();
    assert_eq!(disk_project(&directory)["revision"], 3);
    let _: CancellationState = ok(call(
        &mut host,
        &ui,
        Some(&p),
        "cancel_task",
        json!({"task_id":completed.id,"action":"cancel"}),
    ));
    host.finish_io(completion);
    assert!(!saved(&mut host, &ui, &p, &completed).still_dirty);
    let late: CancellationState = ok(call(
        &mut host,
        &ui,
        Some(&p),
        "cancel_task",
        json!({"task_id":completed.id,"action":"cancel"}),
    ));
    assert!(late.terminal_won);
    assert!(!late.action_accepted);
    assert!(matches!(
        task(&mut host, &ui, Some(&p), &completed).task.state,
        ComputationStatus::Completed
    ));
    assert_eq!(disk_project(&directory)["title"], "r3-to-save");
}

#[test]
fn undo_redo_replays_batches_with_monotonic_revisions_and_new_edits_clear_redo() {
    let (mut host, ui, p) = fixture();
    let batch: EditCommit = ok(apply(
        &mut host,
        &ui,
        Some(&p),
        Some(1),
        json!([settings("batch-r2"), item("PART-A")]),
    ));
    revision(&batch, 2);
    revision(&edit(&mut host, &ui, &p, 2, "second-r3"), 3);
    for (branch, base, next, title, count) in [
        ("undo", 3, 4, "batch-r2", 1),
        ("undo", 4, 5, "initial", 0),
        ("redo", 5, 6, "batch-r2", 1),
        ("redo", 6, 7, "second-r3", 1),
    ] {
        let committed: EditCommit = ok(history(&mut host, &ui, &p, base, branch));
        revision(&committed, next);
        let state = project(&mut host, &ui, &p);
        assert_eq!(state["title"], title);
        assert_eq!(state["item_types"].as_array().unwrap().len(), count);
    }
    let before = project(&mut host, &ui, &p);
    assert!(matches!(
        error(history(&mut host, &ui, &p, 6, "undo")).code,
        ErrorCode::RevisionConflict
    ));
    assert_eq!(project(&mut host, &ui, &p), before);
    let undone: EditCommit = ok(history(&mut host, &ui, &p, 7, "undo"));
    revision(&undone, 8);
    assert!(undone.history.can_redo);
    assert_eq!(project(&mut host, &ui, &p)["title"], "batch-r2");
    let edited = edit(&mut host, &ui, &p, 8, "new-branch");
    revision(&edited, 9);
    assert!(!edited.history.can_redo);
    let before = project(&mut host, &ui, &p);
    error(history(&mut host, &ui, &p, 9, "redo"));
    assert_eq!(project(&mut host, &ui, &p), before);
    let new_project = create(&mut host, &ui, "other-project");
    assert_ne!(new_project, p);
    error(history(&mut host, &ui, &new_project, 1, "undo"));
    assert_eq!(
        project(&mut host, &ui, &new_project)["title"],
        "other-project"
    );
}

fn grant(p: &str, grant_id: &str, branches: Vec<HostEditBranch>) -> HostEditGrant {
    HostEditGrant {
        id: id(grant_id),
        scope: TaskScope::Edit(EditScope::Project(
            ProjectId::try_from(p.to_owned()).unwrap(),
        )),
        branches,
        objects: vec![],
        creates: vec![],
        source_versions: vec![],
        reusable_resources: vec![],
        created_objects: vec![],
    }
}

fn field(pointer: &str) -> HostFieldGrant {
    HostFieldGrant {
        pointer: pointer.to_owned(),
        element_id: None,
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
fn history_uses_current_undo_redo_branches_and_fields_not_original_edit_authority() {
    let (mut host, ui, p) = fixture();
    revision(&edit(&mut host, &ui, &p, 1, "edited"), 2);
    for (name, branches, pointer) in [
        (
            "old-set-only",
            vec![HostEditBranch::SetProjectSettings],
            "/title",
        ),
        (
            "wrong-field",
            vec![HostEditBranch::Undo, HostEditBranch::Redo],
            "/time_reference/label",
        ),
    ] {
        let session = id(name);
        let mut g = grant(&p, name, branches);
        g.objects.push(HostObjectEditGrant {
            target: ObjectRef {
                kind: ObjectKind::Project,
                id: id(&p),
            },
            fields: vec![field(pointer)],
            can_remove: false,
        });
        host.register_external_session(external(&session, &p, vec![g]))
            .unwrap();
        let before = project(&mut host, &ui, &p);
        let before_ledger = ledger(&host, &session);
        assert!(matches!(
            error(history(&mut host, &session, &p, 2, "undo")).code,
            ErrorCode::PermissionDenied
        ));
        assert_eq!(project(&mut host, &ui, &p), before);
        assert_eq!(ledger(&host, &session), before_ledger);
    }
    let session = id("current-history-grant");
    let mut g = grant(
        &p,
        "history-only",
        vec![HostEditBranch::Undo, HostEditBranch::Redo],
    );
    g.objects.push(HostObjectEditGrant {
        target: ObjectRef {
            kind: ObjectKind::Project,
            id: id(&p),
        },
        fields: vec![field("/title")],
        can_remove: false,
    });
    host.register_external_session(external(&session, &p, vec![g]))
        .unwrap();
    let undo: EditCommit = ok(history(&mut host, &session, &p, 2, "undo"));
    revision(&undo, 3);
    assert_eq!(project(&mut host, &ui, &p)["title"], "initial");
    let before_ledger = ledger(&host, &session);
    let redo: EditCommit = ok(history(&mut host, &session, &p, 3, "redo"));
    revision(&redo, 4);
    assert_eq!(project(&mut host, &ui, &p)["title"], "edited");
    assert_eq!(ledger(&host, &session), before_ledger);
}

#[test]
fn undoing_creation_never_refunds_budget_and_redo_same_id_does_not_charge_twice() {
    let (mut host, ui, p) = fixture();
    let session = id("creation-history");
    let mut g = grant(
        &p,
        "creation-history-grant",
        vec![
            HostEditBranch::PutObject,
            HostEditBranch::Undo,
            HostEditBranch::Redo,
        ],
    );
    g.creates.push(HostCreateAllowance {
        kind: ObjectKind::ItemType,
        max_new_objects: SafeUint::try_from(1_u64).unwrap(),
        fields: ["/revision", "/title", "/unit", "/tracking", "/spec_ref"]
            .into_iter()
            .map(field)
            .collect(),
    });
    g.objects.push(HostObjectEditGrant {
        target: ObjectRef {
            kind: ObjectKind::ItemType,
            id: id("PART-A"),
        },
        fields: vec![],
        can_remove: true,
    });
    host.register_external_session(external(&session, &p, vec![g]))
        .unwrap();
    let created: EditCommit = ok(apply(
        &mut host,
        &session,
        Some(&p),
        Some(1),
        json!([item("PART-A")]),
    ));
    revision(&created, 2);
    let spent = ledger(&host, &session);
    assert_eq!(
        spent[0]["created_objects"],
        json!([{"kind":"item_type","id":"PART-A"}])
    );
    let undone: EditCommit = ok(history(&mut host, &session, &p, 2, "undo"));
    revision(&undone, 3);
    assert!(undone.history.can_redo);
    assert!(
        project(&mut host, &ui, &p)["item_types"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(ledger(&host, &session), spent);
    let before = project(&mut host, &ui, &p);
    assert!(matches!(
        error(apply(
            &mut host,
            &session,
            Some(&p),
            Some(3),
            json!([item("PART-B")])
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(project(&mut host, &ui, &p), before);
    assert_eq!(ledger(&host, &session), spent);
    let redone: EditCommit = ok(history(&mut host, &session, &p, 3, "redo"));
    revision(&redone, 4);
    assert_eq!(project(&mut host, &ui, &p)["item_types"][0]["id"], "PART-A");
    assert_eq!(ledger(&host, &session), spent);
}
