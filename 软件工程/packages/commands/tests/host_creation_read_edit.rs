//! P0-01 step 2: real Host dispatch; expectations fixed from the frozen contract.
//! No persistence, undo/redo, UI transport, or schema-only roundtrip substitutes.
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::{ItemType, ObjectId, ProjectId, ProjectState};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn id(value: &str) -> ObjectId {
    ObjectId::try_from(value.to_owned()).unwrap()
}

fn host() -> (ProjectHost, ObjectId) {
    let ui = id("w5-ui-session");
    (ProjectHost::new(id("w5-host"), ui.clone()), ui)
}

fn dispatch(
    host: &mut ProjectHost,
    session: &ObjectId,
    project: Option<&str>,
    operation: &str,
    params: Value,
) -> Value {
    let request_id = format!(
        "w5-request-{}",
        REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let request = serde_json::from_value::<ApiRequest>(json!({
        "api_version": "0.1", "request_id": request_id, "project_id": project,
        "scope": {"kind": if project.is_some() { "project" } else { "workspace" }},
        "operation": operation, "params": params
    }))
    .expect("test input must satisfy the frozen typed API before Host dispatch");
    let reply = host.dispatch(session, request);
    assert_eq!(reply["request_id"], request_id);
    reply
}

fn ok<R: DeserializeOwned>(reply: Value) -> R {
    let response: Response<R> = serde_json::from_value(reply.clone())
        .unwrap_or_else(|error| panic!("invalid frozen response: {error}; {reply:#}"));
    match response.outcome {
        ApiOutcome::Ok(result) => result,
        _ => panic!("expected successful Host operation, got {reply:#}"),
    }
}

fn error(reply: Value) -> ApiError {
    let response: Response<Value> = serde_json::from_value(reply.clone()).unwrap();
    match response.outcome {
        ApiOutcome::Error(error) => error,
        _ => panic!("expected rejected Host operation, got {reply:#}"),
    }
}

fn create(host: &mut ProjectHost, ui: &ObjectId, title: &str, departure: &str) -> Value {
    dispatch(
        host,
        ui,
        None,
        "apply_commands",
        json!({
            "base_revision": null,
            "commands": [{"kind": "create_project", "data": {
                "title": title,
                "time_reference": {"label": "相对毫秒", "origin_iso8601": null, "timezone": null},
                "departure": departure
            }}]
        }),
    )
}

fn context(host: &mut ProjectHost, caller: &ObjectId, project: &str) -> ProjectContext {
    ok(dispatch(
        host,
        caller,
        Some(project),
        "get_context",
        json!({"selection": []}),
    ))
}

fn object_reply(
    host: &mut ProjectHost,
    caller: &ObjectId,
    project: &str,
    kind: &str,
    object_id: &str,
) -> Value {
    dispatch(
        host,
        caller,
        Some(project),
        "get_object",
        json!({
            "object_id": {"kind": kind, "id": object_id},
            "source": {"kind": "draft"}, "node_index": null
        }),
    )
}

fn project(host: &mut ProjectHost, ui: &ObjectId, project_id: &str) -> ProjectState {
    match ok::<DomainObject>(object_reply(host, ui, project_id, "project", project_id)) {
        DomainObject::Project(project) => *project,
        other => panic!("expected project, got {other:?}"),
    }
}

fn item(value_id: &str, title: &str) -> Value {
    json!({
        "kind": "put_object", "data": {"kind": "item_type", "data": {
            "id": value_id, "revision": "r1", "title": title,
            "unit": "piece", "tracking": "individual", "spec_ref": null,
            "attributes": {}, "attribute_schema": []
        }}
    })
}

fn settings(title: &str) -> Value {
    json!({"kind": "set_project_settings", "data": {
        "title": title,
        "time_reference": {"label": "编辑后相对毫秒", "origin_iso8601": null, "timezone": null},
        "generation_context": null, "sources": []
    }})
}

fn create_and_edit(host: &mut ProjectHost, ui: &ObjectId) -> String {
    let created: EditCommit = ok(create(host, ui, "窗口5原工程", "stay"));
    let project_id = created.project_id.as_str().to_owned();
    assert_eq!(created.new_revision.0.value(), 1);
    assert!(created.previous_revision.is_none());
    let committed: EditCommit = ok(dispatch(
        host,
        ui,
        Some(&project_id),
        "apply_commands",
        json!({
            "base_revision": 1,
            "commands": [settings("窗口5已编辑工程"), item("w5-part", "真实零件类型")]
        }),
    ));
    assert_eq!(committed.project_id.as_str(), project_id);
    assert_eq!(committed.previous_revision.unwrap().0.value(), 1);
    assert_eq!(committed.new_revision.0.value(), 2);
    project_id
}

fn register_reader(host: &mut ProjectHost, project_id: &str) -> ObjectId {
    let session = id("w5-external-reader");
    host.register_external_session(HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: session.clone(),
        granted_project: Some(ProjectId::try_from(project_id.to_owned()).unwrap()),
        allowed_operations: vec![OperationName::GetContext, OperationName::GetObject],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![],
    })
    .expect("trusted Host may register a read-only external session for the active project");
    session
}

#[test]
fn workspace_create_then_one_project_batch_is_read_back_at_revisions_one_and_two() {
    let (mut host, ui) = host();
    let created: EditCommit = ok(create(&mut host, &ui, "窗口5新工程", "stay"));
    let project_id = created.project_id.as_str().to_owned();
    assert!(!project_id.is_empty());
    assert!(created.previous_revision.is_none());
    assert_eq!(created.new_revision.0.value(), 1);

    let initial = context(&mut host, &ui, &project_id).project.unwrap();
    assert_eq!(initial.id.as_str(), project_id);
    assert_eq!(initial.revision.0.value(), 1);
    assert_eq!(initial.title, "窗口5新工程");
    assert!(initial.dirty);
    let initial_object = project(&mut host, &ui, &project_id);
    assert_eq!(initial_object.id.as_str(), project_id);
    assert_eq!(initial_object.revision.0.value(), 1);
    assert_eq!(initial_object.title, initial.title);
    assert!(initial_object.item_types.is_empty());

    let committed: EditCommit = ok(dispatch(
        &mut host,
        &ui,
        Some(&project_id),
        "apply_commands",
        json!({
            "base_revision": 1,
            "commands": [settings("同批编辑后的工程"), item("w5-part", "已应用零件类型")]
        }),
    ));
    assert_eq!(committed.project_id.as_str(), project_id);
    assert_eq!(committed.previous_revision.unwrap().0.value(), 1);
    assert_eq!(
        committed.new_revision.0.value(),
        2,
        "one batch increments once, not once per command"
    );
    let updated = project(&mut host, &ui, &project_id);
    assert_eq!(updated.id.as_str(), project_id);
    assert_eq!(updated.revision.0.value(), 2);
    assert_eq!(updated.title, "同批编辑后的工程");
    assert_eq!(updated.time_reference.label, "编辑后相对毫秒");
    assert_eq!(updated.item_types.len(), 1);
    let read_item: ItemType = match ok(object_reply(
        &mut host,
        &ui,
        &project_id,
        "item_type",
        "w5-part",
    )) {
        DomainObject::ItemType(item) => item,
        other => panic!("expected item type, got {other:?}"),
    };
    assert_eq!(read_item.id.as_str(), "w5-part");
    assert_eq!(read_item.title, "已应用零件类型");
    assert_eq!(read_item.unit.as_str(), "piece");
    assert_eq!(
        serde_json::to_value(&read_item).unwrap(),
        serde_json::to_value(&updated.item_types[0]).unwrap()
    );
    let summary = context(&mut host, &ui, &project_id).project.unwrap();
    assert_eq!(summary.revision.0.value(), 2);
    assert_eq!(summary.title, updated.title);
    assert!(summary.dirty);
}

#[test]
fn failed_batch_keeps_the_whole_project_and_revision_without_staged_item_leakage() {
    let (mut host, ui) = host();
    let project_id = create_and_edit(&mut host, &ui);
    let before = serde_json::to_value(project(&mut host, &ui, &project_id)).unwrap();
    let missing = error(object_reply(
        &mut host,
        &ui,
        &project_id,
        "item_type",
        "w5-absent",
    ));
    assert!(matches!(missing.code, ErrorCode::MissingReference));
    let rejected = error(dispatch(
        &mut host,
        &ui,
        Some(&project_id),
        "apply_commands",
        json!({
            "base_revision": 2,
            "commands": [settings("不可部分提交"), item("w5-staged-part", "不可泄漏"),
                         {"kind": "remove_object", "data": {"kind": "item_type", "id": "w5-absent"}}]
        }),
    ));
    assert!(matches!(rejected.code, ErrorCode::MissingReference));
    assert_eq!(
        serde_json::to_value(project(&mut host, &ui, &project_id)).unwrap(),
        before
    );
    assert!(matches!(
        error(object_reply(
            &mut host,
            &ui,
            &project_id,
            "item_type",
            "w5-staged-part"
        ))
        .code,
        ErrorCode::MissingReference
    ));
    let summary = context(&mut host, &ui, &project_id).project.unwrap();
    assert_eq!(summary.revision.0.value(), 2);
    assert!(summary.dirty);
}

#[test]
fn stay_keeps_the_dirty_project_and_its_registered_reader() {
    let (mut host, ui) = host();
    let project_id = create_and_edit(&mut host, &ui);
    let reader = register_reader(&mut host, &project_id);
    let before = serde_json::to_value(project(&mut host, &ui, &project_id)).unwrap();
    error(create(&mut host, &ui, "不应创建的工程", "stay"));
    assert_eq!(
        serde_json::to_value(project(&mut host, &ui, &project_id)).unwrap(),
        before
    );
    let summary = context(&mut host, &ui, &project_id).project.unwrap();
    assert_eq!(summary.id.as_str(), project_id);
    assert_eq!(summary.revision.0.value(), 2);
    assert!(summary.dirty);
    assert!(host.caller_context(&reader).is_some());
    assert_eq!(
        context(&mut host, &reader, &project_id)
            .project
            .unwrap()
            .id
            .as_str(),
        project_id
    );
}

#[test]
fn already_saved_cannot_replace_a_dirty_project_without_an_actual_save() {
    let (mut host, ui) = host();
    let project_id = create_and_edit(&mut host, &ui);
    let reader = register_reader(&mut host, &project_id);
    let before = serde_json::to_value(project(&mut host, &ui, &project_id)).unwrap();
    error(create(
        &mut host,
        &ui,
        "虚称已保存的新工程",
        "already_saved",
    ));
    assert_eq!(
        serde_json::to_value(project(&mut host, &ui, &project_id)).unwrap(),
        before
    );
    let summary = context(&mut host, &ui, &project_id).project.unwrap();
    assert_eq!(summary.id.as_str(), project_id);
    assert_eq!(summary.revision.0.value(), 2);
    assert!(summary.dirty);
    assert!(host.caller_context(&reader).is_some());
}

#[test]
fn explicit_discard_replaces_the_project_and_revokes_old_external_sessions() {
    let (mut host, ui) = host();
    let old_project = create_and_edit(&mut host, &ui);
    let reader = register_reader(&mut host, &old_project);
    assert_eq!(
        context(&mut host, &reader, &old_project)
            .project
            .unwrap()
            .id
            .as_str(),
        old_project
    );

    let created: EditCommit = ok(create(
        &mut host,
        &ui,
        "明确丢弃后的新工程",
        "discard_after_tasks_terminal",
    ));
    let new_project = created.project_id.as_str().to_owned();
    assert_ne!(new_project, old_project);
    assert!(created.previous_revision.is_none());
    assert_eq!(created.new_revision.0.value(), 1);
    let summary = context(&mut host, &ui, &new_project).project.unwrap();
    assert_eq!(summary.id.as_str(), new_project);
    assert_eq!(summary.revision.0.value(), 1);
    assert_eq!(summary.title, "明确丢弃后的新工程");
    assert!(summary.dirty);
    let new_state = project(&mut host, &ui, &new_project);
    assert!(
        new_state.item_types.is_empty(),
        "old project objects must not cross the replacement boundary"
    );

    assert!(
        host.caller_context(&reader).is_none(),
        "old external session must be revoked on switch"
    );
    error(object_reply(
        &mut host,
        &ui,
        &old_project,
        "project",
        &old_project,
    ));
    error(dispatch(
        &mut host,
        &reader,
        Some(&old_project),
        "get_context",
        json!({"selection": []}),
    ));
    error(dispatch(
        &mut host,
        &reader,
        Some(&new_project),
        "get_context",
        json!({"selection": []}),
    ));
    assert!(matches!(
        error(object_reply(
            &mut host,
            &ui,
            &new_project,
            "item_type",
            "w5-part"
        ))
        .code,
        ErrorCode::MissingReference
    ));
    assert_eq!(
        context(&mut host, &ui, &new_project)
            .project
            .unwrap()
            .id
            .as_str(),
        new_project
    );
}
