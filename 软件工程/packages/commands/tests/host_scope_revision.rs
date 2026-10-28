//! Independent expectations from the frozen Scope/revision contract and the
//! P0-01 step2 dispatch. Every observation below comes from ProjectHost.dispatch.
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;

const HOST: &str = "W3-HOST";
const UI: &str = "W3-UI";

fn object_id(value: &str) -> ObjectId {
    value.to_owned().try_into().unwrap()
}

fn project_id(value: &str) -> ProjectId {
    value.to_owned().try_into().unwrap()
}

fn call(
    host: &mut ProjectHost,
    session: &str,
    request_id: &str,
    project: Option<&str>,
    scope: Value,
    operation: &str,
    params: Value,
) -> Value {
    let request: ApiRequest = serde_json::from_value(json!({
        "api_version":"0.1", "request_id":request_id, "project_id":project,
        "scope":scope, "operation":operation, "params":params
    }))
    .expect("the independent request must satisfy the frozen typed boundary");
    let response = host.dispatch(&object_id(session), request);
    let _: Response<Value> = serde_json::from_value(response.clone())
        .expect("real Host reply must retain the frozen Response envelope");
    assert_eq!(response["api_version"], "0.1");
    assert_eq!(response["request_id"], request_id);
    response
}

fn ok(response: &Value) -> &Value {
    assert_eq!(response["outcome"]["status"], "ok", "{response:#}");
    &response["outcome"]["data"]
}

fn error<'a>(response: &'a Value, code: &str) -> &'a Value {
    assert_eq!(response["outcome"]["status"], "error", "{response:#}");
    assert_eq!(response["outcome"]["data"]["code"], code, "{response:#}");
    &response["outcome"]["data"]
}

fn private_denial(response: &Value, code: &str) {
    let rejected = error(response, code);
    assert!(rejected["expected_revision"].is_null(), "{response:#}");
    assert!(rejected["actual_revision"].is_null(), "{response:#}");
    // A missing context and a Host/session-only context are both permitted.
    // Neither form may disclose the unauthorized active Project or its clock.
    for field in ["project_id", "project_revision", "scope", "scope_revision"] {
        assert!(
            response["context"][field].is_null(),
            "{field}: {response:#}"
        );
    }
    let text = response.to_string();
    assert!(!text.contains("scope-private-project"), "{response:#}");
    assert!(!text.contains("kept-item-private"), "{response:#}");
}

fn time_reference() -> Value {
    json!({"label":"relative time", "origin_iso8601":null, "timezone":null})
}

fn settings(title: &str) -> Value {
    json!({"kind":"set_project_settings","data":{
        "title":title,"time_reference":time_reference(),
        "generation_context":null,"sources":[]
    }})
}

fn put_item(id: &str, title: &str) -> Value {
    json!({"kind":"put_object","data":{"kind":"item_type","data":{
        "id":id,"revision":"r1","title":title,"unit":"piece",
        "tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]
    }}})
}

fn edit(host: &mut ProjectHost, project: &str, base: u64, commands: Vec<Value>) -> Value {
    call(
        host,
        UI,
        "W3-EDIT",
        Some(project),
        json!({"kind":"project"}),
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    )
}

fn new_project() -> (ProjectHost, String) {
    let mut host = ProjectHost::new(object_id(HOST), object_id(UI));
    let response = call(
        &mut host,
        UI,
        "W3-CREATE",
        None,
        json!({"kind":"workspace"}),
        "apply_commands",
        json!({"base_revision":null,"commands":[{
            "kind":"create_project","data":{"title":"scope-private-project",
                "time_reference":time_reference(),"departure":"stay"}
        }]}),
    );
    let commit: EditCommit = serde_json::from_value(ok(&response).clone()).unwrap();
    assert_eq!(commit.new_revision.0.value(), 1);
    // No P1/default/guessed ID: the actual Host owns the new project identity.
    let id = commit.project_id.as_str().to_owned();
    assert!(!id.is_empty());
    (host, id)
}

fn read_project(host: &mut ProjectHost, project: &str) -> Value {
    let response = call(
        host,
        UI,
        "W3-READ-PROJECT",
        Some(project),
        json!({"kind":"project"}),
        "get_object",
        json!({"object_id":{"kind":"project","id":project},
            "source":{"kind":"draft"},"node_index":null}),
    );
    let object = ok(&response);
    assert_eq!(object["kind"], "project");
    object["data"].clone()
}

fn seeded_project() -> (ProjectHost, String) {
    let (mut host, project) = new_project();
    let response = edit(
        &mut host,
        &project,
        1,
        vec![put_item("W3-KEPT", "kept-item-private")],
    );
    assert_eq!(ok(&response)["new_revision"], 2);
    (host, project)
}

fn title_grant(project: &str) -> HostEditGrant {
    HostEditGrant {
        id: object_id("W3-TITLE-GRANT"),
        scope: TaskScope::Edit(EditScope::Project(project_id(project))),
        branches: vec![HostEditBranch::SetProjectSettings],
        objects: vec![HostObjectEditGrant {
            target: ObjectRef {
                kind: ObjectKind::Project,
                id: object_id(project),
            },
            fields: vec![HostFieldGrant {
                pointer: "/title".into(),
                element_id: None,
            }],
            can_remove: false,
        }],
        creates: vec![],
        source_versions: vec![],
        reusable_resources: vec![],
        created_objects: vec![],
    }
}

fn external(session: &str, project: &str, operations: Vec<OperationName>) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: object_id(session),
        granted_project: Some(project_id(project)),
        allowed_operations: operations,
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![title_grant(project)],
    }
}

fn creation_ledger(host: &ProjectHost, session: &str) -> Value {
    let caller = host
        .caller_context(&object_id(session))
        .expect("registered session");
    json!(
        caller
            .edit_grants
            .iter()
            .flat_map(|grant| &grant.created_objects)
            .collect::<Vec<_>>()
    )
}

#[test]
fn no_active_project_read_is_not_a_fabricated_project_or_revision() {
    let mut host = ProjectHost::new(object_id(HOST), object_id(UI));
    let response = call(
        &mut host,
        UI,
        "W3-NO-PROJECT",
        Some("W3-NOT-CREATED"),
        json!({"kind":"project"}),
        "get_object",
        json!({
            "object_id":{"kind":"project","id":"W3-NOT-CREATED"},
            "source":{"kind":"draft"},"node_index":null
        }),
    );
    private_denial(&response, "NoActiveProject");
    // Workspace get_context has a different frozen meaning: an empty context.
    let context = call(
        &mut host,
        UI,
        "W3-EMPTY-CONTEXT",
        None,
        json!({"kind":"workspace"}),
        "get_context",
        json!({"selection":[]}),
    );
    assert!(ok(&context)["project"].is_null());
    assert!(ok(&context)["host"]["active_project_id"].is_null());
    assert!(ok(&context)["host"]["active_project_revision"].is_null());
}

#[test]
fn unregistered_session_and_wrong_active_project_precede_stale_revision() {
    let (mut host, project) = seeded_project();
    let before = read_project(&mut host, &project);
    for (session, requested_project) in [
        ("W3-UNREGISTERED", project.as_str()),
        (UI, "W3-OTHER-PROJECT"),
    ] {
        let response = call(
            &mut host,
            session,
            "W3-SCOPE-FIRST",
            Some(requested_project),
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":0,"commands":[settings("must-not-write")]}),
        );
        private_denial(&response, "PermissionDenied");
        assert_eq!(read_project(&mut host, &project), before);
    }
    assert!(host.caller_context(&object_id("W3-UNREGISTERED")).is_none());
}

#[test]
fn foreign_project_grant_and_disabled_operation_do_not_disclose_or_mutate() {
    let (mut host, project) = seeded_project();
    host.register_external_session(external(
        "W3-FOREIGN",
        "W3-OTHER-PROJECT",
        vec![OperationName::GetObject],
    ))
    .expect("trusted Host registers an ExternalAgent session");
    host.register_external_session(external(
        "W3-READ-ONLY",
        &project,
        vec![OperationName::GetContext],
    ))
    .expect("trusted Host registers an ExternalAgent session");
    let before = read_project(&mut host, &project);
    let foreign_ledger = creation_ledger(&host, "W3-FOREIGN");
    let read_only_ledger = creation_ledger(&host, "W3-READ-ONLY");
    let read = call(
        &mut host,
        "W3-FOREIGN",
        "W3-FOREIGN-READ",
        Some(&project),
        json!({"kind":"project"}),
        "get_object",
        json!({
            "object_id":{"kind":"item_type","id":"W3-KEPT"},
            "source":{"kind":"draft"},"node_index":null
        }),
    );
    private_denial(&read, "PermissionDenied");
    let edit = call(
        &mut host,
        "W3-READ-ONLY",
        "W3-DISABLED-EDIT",
        Some(&project),
        json!({"kind":"project"}),
        "apply_commands",
        json!({"base_revision":0,"commands":[settings("must-not-write")]}),
    );
    private_denial(&edit, "PermissionDenied");
    assert_eq!(read_project(&mut host, &project), before);
    assert_eq!(creation_ledger(&host, "W3-FOREIGN"), foreign_ledger);
    assert_eq!(creation_ledger(&host, "W3-READ-ONLY"), read_only_ledger);
}

#[test]
fn unsupported_definition_draft_never_borrows_project_revision() {
    let (mut host, project) = seeded_project();
    let before = read_project(&mut host, &project);
    let response = call(
        &mut host,
        UI,
        "W3-DEFINITION-SCOPE",
        Some(&project),
        json!({"kind":"definition_draft","data":"W3-DRAFT"}),
        "apply_commands",
        json!({"base_revision":0,"commands":[settings("not-a-definition-edit")]}),
    );
    let rejected = error(&response, "UnsupportedCapability");
    assert!(rejected["expected_revision"].is_null(), "{response:#}");
    assert!(rejected["actual_revision"].is_null(), "{response:#}");
    assert!(
        response["context"]["project_revision"].is_null(),
        "{response:#}"
    );
    assert!(
        response["context"]["scope_revision"].is_null(),
        "{response:#}"
    );
    assert_ne!(response["context"]["scope"]["kind"], "project");
    assert_eq!(read_project(&mut host, &project), before);
}

#[test]
fn multiple_edits_commit_exactly_one_new_project_revision() {
    let (mut host, project) = new_project();
    let response = edit(
        &mut host,
        &project,
        1,
        vec![
            put_item("W3-A", "A"),
            put_item("W3-B", "B"),
            settings("committed-batch"),
        ],
    );
    let commit = ok(&response);
    assert_eq!(commit["project_id"], project);
    assert_eq!(commit["previous_revision"], 1);
    assert_eq!(commit["new_revision"], 2);
    assert_eq!(response["context"]["project_revision"], 2);
    assert_eq!(response["context"]["scope_revision"], 2);
    let current = read_project(&mut host, &project);
    assert_eq!(current["revision"], 2);
    assert_eq!(current["title"], "committed-batch");
    let items = current["item_types"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .any(|item| item["id"] == "W3-A" && item["title"] == "A")
    );
    assert!(
        items
            .iter()
            .any(|item| item["id"] == "W3-B" && item["title"] == "B")
    );
}

#[test]
fn stale_edit_preserves_current_values_and_reports_the_resolved_project_scope() {
    let (mut host, project) = seeded_project();
    let before = read_project(&mut host, &project);
    let commands = vec![
        settings("after-conflict"),
        put_item("W3-KEPT", "after-conflict-item"),
    ];
    let rejected = edit(&mut host, &project, 1, commands.clone());
    let conflict = error(&rejected, "RevisionConflict");
    assert_eq!(conflict["expected_revision"], 1);
    assert_eq!(conflict["actual_revision"], 2);
    assert_eq!(rejected["context"]["host_instance_id"], HOST);
    assert_eq!(rejected["context"]["session_id"], UI);
    assert_eq!(rejected["context"]["project_id"], project);
    assert_eq!(rejected["context"]["project_revision"], 2);
    assert_eq!(
        rejected["context"]["scope"],
        json!({"kind":"project","data":project})
    );
    assert_eq!(rejected["context"]["scope_revision"], 2);
    assert_eq!(read_project(&mut host, &project), before);
    let committed = edit(&mut host, &project, 2, commands);
    assert_eq!(ok(&committed)["previous_revision"], 2);
    assert_eq!(ok(&committed)["new_revision"], 3);
    let current = read_project(&mut host, &project);
    assert_eq!(current["revision"], 3);
    assert_eq!(current["title"], "after-conflict");
    assert_eq!(current["item_types"][0]["title"], "after-conflict-item");
}
