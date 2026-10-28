//! Expectations fixed from the frozen Host authorization contract before execution.
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::collections::BTreeMap;

fn object_id(value: &str) -> ObjectId {
    ObjectId::try_from(value.to_owned()).unwrap()
}

fn revision(value: u64) -> Revision {
    Revision(SafeUint::try_from(value).unwrap())
}

fn time_reference() -> TimeReference {
    TimeReference {
        label: "relative milliseconds".into(),
        origin_iso8601: None,
        timezone: None,
    }
}

fn item(id: &str) -> ItemType {
    ItemType {
        contents_layout: None,
        id: object_id(id),
        revision: SubjectRevisionId::try_from("r1".to_owned()).unwrap(),
        title: format!("item {id}"),
        module_ref: None,
        unit: UnitId::try_from("piece".to_owned()).unwrap(),
        tracking: TrackingKind::Individual,
        spec_ref: None,
        attributes: BTreeMap::new(),
        attribute_schema: vec![],
    }
}

fn item_ref(id: &str) -> ObjectRef {
    ObjectRef {
        kind: ObjectKind::ItemType,
        id: object_id(id),
    }
}

fn put(value: ItemType) -> EditCommand {
    EditCommand::PutObject(EditableProjectObject::ItemType(value))
}

fn field(pointer: &str) -> HostFieldGrant {
    HostFieldGrant {
        pointer: pointer.into(),
        element_id: None,
    }
}

fn allowance(max: u64) -> HostCreateAllowance {
    HostCreateAllowance {
        kind: ObjectKind::ItemType,
        max_new_objects: SafeUint::try_from(max).unwrap(),
        fields: ["/revision", "/title", "/unit", "/tracking", "/spec_ref"]
            .into_iter()
            .map(field)
            .collect(),
    }
}

fn grant(project: &ProjectId) -> HostEditGrant {
    HostEditGrant {
        id: object_id("grant-auth-atomic"),
        scope: TaskScope::Edit(EditScope::Project(project.clone())),
        branches: vec![],
        objects: vec![],
        creates: vec![],
        source_versions: vec![],
        reusable_resources: vec![],
        created_objects: vec![],
    }
}

fn external(
    session: &ObjectId,
    project: Option<ProjectId>,
    grants: Vec<HostEditGrant>,
) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: session.clone(),
        granted_project: project,
        allowed_operations: vec![
            OperationName::GetContext,
            OperationName::GetObject,
            OperationName::ApplyCommands,
        ],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: grants,
    }
}

fn apply(
    host: &mut ProjectHost,
    session: &ObjectId,
    project: Option<&ProjectId>,
    base: Option<u64>,
    commands: Vec<EditCommand>,
) -> Value {
    host.dispatch(
        session,
        ApiRequest::ApplyCommands(Request {
            api_version: ApiVersion::V01,
            request_id: object_id("auth-atomic-edit"),
            project_id: project.cloned(),
            scope: if project.is_some() {
                ApiScope::Project
            } else {
                ApiScope::Workspace
            },
            operation: ApplyCommandsTag::Call,
            params: ApplyCommandsParams {
                base_revision: base.map(revision),
                commands,
            },
        }),
    )
}

fn assert_ok(reply: &Value) {
    assert_eq!(reply["outcome"]["status"], "ok", "{reply:#}");
}

fn assert_error(reply: &Value, code: &str) {
    assert_eq!(reply["outcome"]["status"], "error", "{reply:#}");
    assert_eq!(reply["outcome"]["data"]["code"], code, "{reply:#}");
}

fn new_project() -> (ProjectHost, ObjectId, ProjectId) {
    let ui = object_id("ui-auth-atomic");
    let mut host = ProjectHost::new(object_id("host-auth-atomic"), ui.clone());
    let reply = apply(
        &mut host,
        &ui,
        None,
        None,
        vec![EditCommand::CreateProject(NewProject {
            title: "authorization fixture".into(),
            time_reference: time_reference(),
            departure: DepartureDecision::Stay,
        })],
    );
    assert_ok(&reply);
    assert_eq!(reply["outcome"]["data"]["new_revision"], 1);
    let project = serde_json::from_value(reply["outcome"]["data"]["project_id"].clone()).unwrap();
    (host, ui, project)
}

fn settings(title: &str) -> ProjectSettings {
    ProjectSettings {
        title: title.into(),
        time_reference: time_reference(),
        generation_context: None,
        sources: vec![],
        setup_policies: None,
    }
}

fn title_grant(project: &ProjectId) -> HostEditGrant {
    let mut value = grant(project);
    value.branches = vec![HostEditBranch::SetProjectSettings];
    value.objects = vec![HostObjectEditGrant {
        target: ObjectRef {
            kind: ObjectKind::Project,
            id: object_id(project.as_str()),
        },
        fields: vec![field("/title")],
        can_remove: false,
    }];
    value
}

fn ledger(host: &ProjectHost, session: &ObjectId) -> Value {
    Value::Array(
        host.caller_context(session)
            .unwrap()
            .edit_grants
            .iter()
            .map(|g| json!({"grant_id": g.id, "created_objects": g.created_objects}))
            .collect(),
    )
}

// Read through dispatch, rather than a parallel test-only project store.
fn snapshot(
    host: &mut ProjectHost,
    ui: &ObjectId,
    project: &ProjectId,
    session: &ObjectId,
) -> Value {
    let reply = host.dispatch(
        ui,
        ApiRequest::GetObject(Request {
            api_version: ApiVersion::V01,
            request_id: object_id("auth-atomic-read-project"),
            project_id: Some(project.clone()),
            scope: ApiScope::Project,
            operation: GetObjectTag::Call,
            params: GetObjectParams {
                object_id: ObjectRef {
                    kind: ObjectKind::Project,
                    id: object_id(project.as_str()),
                },
                source: ObjectSource::Draft,
                node_index: None,
            },
        }),
    );
    assert_ok(&reply);
    assert_eq!(reply["outcome"]["data"]["kind"], "project");
    let context = host.dispatch(
        ui,
        ApiRequest::GetContext(Request {
            api_version: ApiVersion::V01,
            request_id: object_id("auth-atomic-read-context"),
            project_id: Some(project.clone()),
            scope: ApiScope::Project,
            operation: GetContextTag::Call,
            params: GetContextParams { selection: vec![] },
        }),
    );
    assert_ok(&context);
    json!({
        "project": reply["outcome"]["data"]["data"],
        "context": context["outcome"]["data"],
        "ledger": ledger(host, session),
    })
}

#[test]
fn external_workspace_cannot_create_even_with_a_host_registered_create_branch() {
    let ui = object_id("ui-auth-atomic");
    let host_id = object_id("host-auth-atomic");
    let session = object_id("external-workspace");
    let mut host = ProjectHost::new(host_id.clone(), ui.clone());
    host.register_external_session(external(
        &session,
        None,
        vec![HostEditGrant {
            id: object_id("workspace-create-grant"),
            scope: TaskScope::Workspace(host_id),
            branches: vec![HostEditBranch::CreateProject],
            objects: vec![],
            creates: vec![],
            source_versions: vec![],
            reusable_resources: vec![],
            created_objects: vec![],
        }],
    ))
    .unwrap();
    let before = ledger(&host, &session);
    let reply = apply(
        &mut host,
        &session,
        None,
        None,
        vec![EditCommand::CreateProject(NewProject {
            title: "must not exist".into(),
            time_reference: time_reference(),
            departure: DepartureDecision::Stay,
        })],
    );
    assert_error(&reply, "PermissionDenied");
    assert_eq!(ledger(&host, &session), before);
    // A subsequent genuine UI creation must still be the first project/revision.
    let created = apply(
        &mut host,
        &ui,
        None,
        None,
        vec![EditCommand::CreateProject(NewProject {
            title: "genuine UI project".into(),
            time_reference: time_reference(),
            departure: DepartureDecision::Stay,
        })],
    );
    assert_ok(&created);
    assert_eq!(created["outcome"]["data"]["new_revision"], 1);
}

#[test]
fn external_registration_cannot_install_an_interactive_ui_origin() {
    let (mut host, ui, project) = new_project();
    let session = object_id("forged-ui-session");
    let mut caller = external(&session, Some(project.clone()), vec![]);
    caller.origin = CallerOrigin::InteractiveUi;
    let error = host
        .register_external_session(caller)
        .expect_err("external registration must not create UI authority");
    assert!(matches!(error.code, ErrorCode::PermissionDenied));
    assert!(host.caller_context(&session).is_none());
    let reply = apply(
        &mut host,
        &session,
        Some(&project),
        Some(1),
        vec![EditCommand::SetProjectSettings(settings("forged"))],
    );
    assert_error(&reply, "PermissionDenied");
    let read = apply(
        &mut host,
        &ui,
        Some(&project),
        Some(1),
        vec![EditCommand::SetProjectSettings(settings("real UI"))],
    );
    assert_ok(&read);
    assert_eq!(read["outcome"]["data"]["new_revision"], 2);
}

#[test]
fn caller_and_grants_are_not_wire_inputs_and_the_registered_session_stays_external() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-no-grants");
    host.register_external_session(external(&session, Some(project.clone()), vec![]))
        .unwrap();
    let wire = json!({
        "api_version":"0.1", "request_id":"forged-wire", "project_id":project,
        "scope":{"kind":"project"}, "operation":"apply_commands",
        "params":{"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings("forged")}]},
    });
    let before = snapshot(&mut host, &ui, &project, &session);
    for (key, value) in [
        ("caller", json!({"origin":"interactive_ui","session_id":ui})),
        (
            "edit_grants",
            json!([{"scope":{"kind":"project","data":project},"fields":["/"]}]),
        ),
    ] {
        let mut forged = wire.clone();
        forged[key] = value;
        assert!(
            serde_json::from_value::<ApiRequest>(forged).is_err(),
            "{key} accepted on wire"
        );
    }
    let reply = host.dispatch(&session, serde_json::from_value(wire).unwrap());
    assert_error(&reply, "PermissionDenied");
    assert!(matches!(
        host.caller_context(&session).unwrap().origin,
        CallerOrigin::ExternalAgent
    ));
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn project_title_leaf_allows_title_only_and_cannot_replace_the_rest_of_settings() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-title-only");
    host.register_external_session(external(
        &session,
        Some(project.clone()),
        vec![title_grant(&project)],
    ))
    .unwrap();
    let reply = apply(
        &mut host,
        &session,
        Some(&project),
        Some(1),
        vec![EditCommand::SetProjectSettings(settings(
            "authorized title",
        ))],
    );
    assert_ok(&reply);
    assert_eq!(reply["outcome"]["data"]["new_revision"], 2);
    let before = snapshot(&mut host, &ui, &project, &session);
    assert_eq!(before["project"]["title"], "authorized title");
    let mut replacement = settings("another title");
    replacement.time_reference.label = "unauthorized replacement".into();
    replacement.time_reference.timezone = Some("Asia/Shanghai".into());
    let rejected = apply(
        &mut host,
        &session,
        Some(&project),
        Some(2),
        vec![EditCommand::SetProjectSettings(replacement)],
    );
    assert_error(&rejected, "PermissionDenied");
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn legal_title_then_unauthorized_field_is_one_rejected_atomic_batch() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-mixed-fields");
    host.register_external_session(external(
        &session,
        Some(project.clone()),
        vec![title_grant(&project)],
    ))
    .unwrap();
    let before = snapshot(&mut host, &ui, &project, &session);
    let mut denied = settings("candidate title");
    denied.time_reference.label = "not granted".into();
    let reply = apply(
        &mut host,
        &session,
        Some(&project),
        Some(1),
        vec![
            EditCommand::SetProjectSettings(settings("candidate title")),
            EditCommand::SetProjectSettings(denied),
        ],
    );
    assert_error(&reply, "PermissionDenied");
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn put_branch_without_a_creation_allowance_cannot_create_an_item() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-no-create-allowance");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    let before = snapshot(&mut host, &ui, &project, &session);
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(1),
            vec![put(item("A"))],
        ),
        "PermissionDenied",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn creation_allowance_must_cover_the_actual_item_leaf_fields() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-incomplete-create-fields");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject];
    let mut create = allowance(1);
    create.fields = vec![field("/title")];
    permission.creates = vec![create];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    let before = snapshot(&mut host, &ui, &project, &session);
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(1),
            vec![put(item("A"))],
        ),
        "PermissionDenied",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn upsert_of_an_existing_item_cannot_use_a_create_only_allowance() {
    let (mut host, ui, project) = new_project();
    assert_ok(&apply(
        &mut host,
        &ui,
        Some(&project),
        Some(1),
        vec![put(item("existing"))],
    ));
    let session = object_id("external-create-only");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject];
    permission.creates = vec![allowance(1)];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    let before = snapshot(&mut host, &ui, &project, &session);
    let mut modified = item("existing");
    modified.title = "unauthorized update".into();
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(2),
            vec![put(modified)],
        ),
        "PermissionDenied",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
    let created = apply(
        &mut host,
        &session,
        Some(&project),
        Some(2),
        vec![put(item("new"))],
    );
    assert_ok(&created);
    assert_eq!(created["outcome"]["data"]["new_revision"], 3);
    assert_eq!(
        ledger(&host, &session)[0]["created_objects"],
        json!([item_ref("new")])
    );
}

#[test]
fn creation_budget_is_cumulative_distinct_ids_across_commits() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-budget-two");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject];
    permission.creates = vec![allowance(2)];
    permission.objects = vec![HostObjectEditGrant {
        target: item_ref("A"),
        fields: vec![field("/title")],
        can_remove: false,
    }];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    assert_ok(&apply(
        &mut host,
        &session,
        Some(&project),
        Some(1),
        vec![put(item("A"))],
    ));
    let mut update = item("A");
    update.title = "authorized same ID update".into();
    assert_ok(&apply(
        &mut host,
        &session,
        Some(&project),
        Some(2),
        vec![put(update)],
    ));
    assert_eq!(
        ledger(&host, &session)[0]["created_objects"],
        json!([item_ref("A")])
    );
    assert_ok(&apply(
        &mut host,
        &session,
        Some(&project),
        Some(3),
        vec![put(item("B"))],
    ));
    let before = snapshot(&mut host, &ui, &project, &session);
    assert_eq!(before["project"]["revision"], 4);
    assert_eq!(
        ledger(&host, &session)[0]["created_objects"],
        json!([item_ref("A"), item_ref("B")])
    );
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(4),
            vec![put(item("C"))],
        ),
        "PermissionDenied",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn removing_a_created_item_does_not_refund_the_distinct_creation_budget() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-delete-no-refund");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject, HostEditBranch::RemoveObject];
    permission.creates = vec![allowance(1)];
    permission.objects = vec![HostObjectEditGrant {
        target: item_ref("A"),
        fields: vec![],
        can_remove: true,
    }];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    assert_ok(&apply(
        &mut host,
        &session,
        Some(&project),
        Some(1),
        vec![put(item("A"))],
    ));
    assert_ok(&apply(
        &mut host,
        &session,
        Some(&project),
        Some(2),
        vec![EditCommand::RemoveObject(item_ref("A"))],
    ));
    let before = snapshot(&mut host, &ui, &project, &session);
    assert_eq!(before["project"]["item_types"], json!([]));
    assert_eq!(
        ledger(&host, &session)[0]["created_objects"],
        json!([item_ref("A")])
    );
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(3),
            vec![put(item("B"))],
        ),
        "PermissionDenied",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}

#[test]
fn over_budget_batch_commits_neither_project_revision_nor_creation_ledger() {
    let (mut host, ui, project) = new_project();
    let session = object_id("external-over-budget-batch");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject];
    permission.creates = vec![allowance(1)];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    let before = snapshot(&mut host, &ui, &project, &session);
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(1),
            vec![put(item("A")), put(item("B"))],
        ),
        "PermissionDenied",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
    assert_ok(&apply(
        &mut host,
        &session,
        Some(&project),
        Some(1),
        vec![put(item("A"))],
    ));
    assert_eq!(
        ledger(&host, &session)[0]["created_objects"],
        json!([item_ref("A")])
    );
}

#[test]
fn legal_creation_followed_by_denied_or_unsupported_command_is_fully_atomic() {
    for (suffix, trailing, branch, code) in [
        (
            "unauthorized",
            EditCommand::SetProjectSettings(settings("not granted")),
            HostEditBranch::SetProjectSettings,
            "PermissionDenied",
        ),
        (
            "unsupported",
            EditCommand::Undo,
            HostEditBranch::Undo,
            "UnsupportedCapability",
        ),
    ] {
        let (mut host, ui, project) = new_project();
        let session = object_id(&format!("external-atomic-{suffix}"));
        let mut permission = grant(&project);
        permission.branches = vec![HostEditBranch::PutObject, branch];
        permission.creates = vec![allowance(1)];
        host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
            .unwrap();
        let before = snapshot(&mut host, &ui, &project, &session);
        assert_error(
            &apply(
                &mut host,
                &session,
                Some(&project),
                Some(1),
                vec![put(item("A")), trailing],
            ),
            code,
        );
        assert_eq!(
            snapshot(&mut host, &ui, &project, &session),
            before,
            "{suffix}"
        );
        let accepted = apply(
            &mut host,
            &session,
            Some(&project),
            Some(1),
            vec![put(item("A"))],
        );
        assert_ok(&accepted);
        assert_eq!(accepted["outcome"]["data"]["new_revision"], 2);
        assert_eq!(
            ledger(&host, &session)[0]["created_objects"],
            json!([item_ref("A")])
        );
    }
}

#[test]
fn numeric_array_index_does_not_become_a_stable_external_field_grant() {
    let (mut host, ui, project) = new_project();
    let mut original = item("A");
    original.attribute_schema = vec![AttributeDeclaration {
        name: "grade".into(),
        value_type: ValueKind::Enum,
        unit: None,
        shape: None,
        options: vec!["A".into(), "B".into()],
        basis: SourceBasis::UserProvided,
    }];
    assert_ok(&apply(
        &mut host,
        &ui,
        Some(&project),
        Some(1),
        vec![put(original.clone())],
    ));
    let session = object_id("external-array-index");
    let mut permission = grant(&project);
    permission.branches = vec![HostEditBranch::PutObject];
    permission.objects = vec![HostObjectEditGrant {
        target: item_ref("A"),
        fields: vec![field("/attribute_schema/0/name")],
        can_remove: false,
    }];
    host.register_external_session(external(&session, Some(project.clone()), vec![permission]))
        .unwrap();
    let before = snapshot(&mut host, &ui, &project, &session);
    original.attribute_schema[0].name = "different field".into();
    assert_error(
        &apply(
            &mut host,
            &session,
            Some(&project),
            Some(2),
            vec![put(original)],
        ),
        "UnsupportedCapability",
    );
    assert_eq!(snapshot(&mut host, &ui, &project, &session), before);
}
