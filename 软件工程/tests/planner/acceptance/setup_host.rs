//! P205 common-input consumers. The business editor remains owner-in-progress.
use super::{host, simulation, source};
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
fn policies(h: &mut host::Harness) -> Value {
    h.object("project", "P1")["setup_policies"].clone()
}
fn changed(h: &mut host::Harness) -> Value {
    let mut value = policies(h);
    value[0]["changeovers"][0]["duration"]["value"] = json!(23000);
    value
}
fn settings(h: &mut host::Harness, title: &str, setup: Option<Value>) -> Value {
    let p = h.object("project", "P1");
    let mut v = json!({"title":title,"time_reference":p["time_reference"],"generation_context":p["generation_context"],"sources":p["sources"]});
    if let Some(setup) = setup {
        v["setup_policies"] = setup;
    }
    json!({"kind":"set_project_settings","data":v})
}
fn apply(h: &mut host::Harness, command: Value) -> EditCommit {
    let base = h.context().project.unwrap().revision.0.value();
    host::ok(h.apply(base, json!([command])))
}
fn run(h: &mut host::Harness) -> RunResult {
    let task = h.run("ORDERS-A");
    h.finish_one();
    h.completed_run(&task)
}
#[test]
fn p205_optional_setup_input_preserves_old_matrix_and_explicit_23s_change_reaches_the_same_actual_engine()
 {
    let mut h = host::Harness::from_value(source("order_a"));
    let old = run(&mut h);
    let original = serde_json::to_value(&old).unwrap();
    let original_policy = policies(&mut h);
    assert_eq!(simulation::held_ms(&original, "W-A"), 135000);
    assert_eq!(
        original["orders"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|o| o["completion_ms"].as_u64())
            .max(),
        Some(205000)
    );
    let command = settings(&mut h, "Explicit metadata edit, no setup field", None);
    let _: EditCommit = apply(&mut h, command);
    assert_eq!(policies(&mut h), original_policy);
    let changed = changed(&mut h);
    let command = settings(
        &mut h,
        "Explicit user-provided23s changeover",
        Some(changed.clone()),
    );
    let _: EditCommit = apply(&mut h, command);
    assert_eq!(policies(&mut h), changed);
    let next = run(&mut h);
    let next = serde_json::to_value(&next).unwrap();
    let work: Vec<_> = next["phase_work_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["phase_id"] == "setup")
        .collect();
    assert_eq!(work.len(), 1);
    assert_eq!(
        work[0]["interval"]["end_ms"].as_u64().unwrap()
            - work[0]["interval"]["start_ms"].as_u64().unwrap(),
        23000
    );
    assert_eq!(simulation::held_ms(&next, "W-A"), 143000);
    assert_eq!(
        next["orders"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|o| o["completion_ms"].as_u64())
            .max(),
        Some(205000)
    );
    assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), original);
}
#[test]
fn p205_matrix_clear_undo_redo_and_moved_reopen_keep_units_source_old_run_and_new_revisions() {
    let mut h = host::Harness::from_value(source("order_a"));
    let old = run(&mut h);
    let frozen = serde_json::to_value(&old).unwrap();
    let original = policies(&mut h);
    let changed = changed(&mut h);
    let command = settings(&mut h, "Changed matrix", Some(changed.clone()));
    let c = apply(&mut h, command);
    assert_eq!(c.new_revision.0.value(), 2);
    let _: EditCommit = host::ok(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(policies(&mut h), original);
    let _: EditCommit = host::ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(policies(&mut h), changed);
    let command = settings(&mut h, "Explicitly cleared matrix", Some(json!([])));
    let _: EditCommit = apply(&mut h, command);
    assert_eq!(policies(&mut h), json!([]));
    let _: EditCommit = host::ok(h.apply(5, json!([{"kind":"undo"}])));
    assert_eq!(policies(&mut h), changed);
    let command = settings(&mut h, "New edit discards redo branch", None);
    let _: EditCommit = apply(&mut h, command);
    let before = h.snapshot();
    let _ = host::error(h.apply(7, json!([{"kind":"redo"}])));
    assert_eq!(h.snapshot(), before);
    let directory = h.tree.directory("setup-save");
    let task = h.save(7, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let moved = h.tree.directory("setup-moved");
    host::copy(&directory, &moved);
    let mut opened = host::Harness::from_seed(host::Sandbox::new(), &moved);
    assert_eq!(policies(&mut opened), changed);
    assert_eq!(
        policies(&mut opened)[0]["changeovers"][0]["duration"],
        json!({"value":23000,"unit":"millisecond","basis":"user_provided","source_ref":null})
    );
    assert_eq!(serde_json::to_value(opened.full(&old.id)).unwrap(), frozen);
}
fn caller(session: &str, allow_setup: bool) -> HostCallContext {
    let mut fields: Vec<_> = [
        "/title",
        "/time_reference",
        "/generation_context",
        "/sources",
    ]
    .into_iter()
    .map(|p| HostFieldGrant {
        pointer: p.into(),
        element_id: None,
    })
    .collect();
    if allow_setup {
        fields.push(HostFieldGrant {
            pointer: "/setup_policies/99/initial_family".into(),
            element_id: Some(host::id("Assembly")),
        });
    }
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: host::id(session),
        granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
        allowed_operations: vec![OperationName::ApplyCommands],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![HostEditGrant {
            id: host::id(session),
            scope: TaskScope::Edit(EditScope::Project(
                ProjectId::try_from("P1".to_owned()).unwrap(),
            )),
            branches: vec![HostEditBranch::SetProjectSettings],
            objects: vec![HostObjectEditGrant {
                target: ObjectRef {
                    kind: ObjectKind::Project,
                    id: host::id("P1"),
                },
                fields,
                can_remove: false,
            }],
            creates: vec![],
            source_versions: vec![],
            reusable_resources: vec![],
            created_objects: vec![],
        }],
    }
}
#[test]
fn p205_setup_field_requires_its_own_grant_and_stale_authorized_candidate_is_atomic() {
    let mut h = host::Harness::from_value(source("order_a"));
    let old = run(&mut h);
    let frozen = serde_json::to_value(&old).unwrap();
    let nested = changed(&mut h);
    let denied = caller("setup-no-field", false);
    let session = denied.session_id.clone();
    h.host.register_external_session(denied).unwrap();
    let command = settings(&mut h, "This title must not partially commit", Some(nested));
    let before = h.snapshot();
    let failure = host::error(h.call_as(
        &session,
        true,
        "apply_commands",
        json!({"base_revision":1,"commands":[command]}),
    ));
    assert!(
        matches!(failure.code, ErrorCode::UnsupportedCapability),
        "unproven nested array edits must fail closed: {failure:?}"
    );
    assert_eq!(h.snapshot(), before);
    let mut next = policies(&mut h);
    next[0]["initial_family"] = json!("B");
    let command = settings(
        &mut h,
        "This title must not partially commit",
        Some(next.clone()),
    );
    let failure = host::error(h.call_as(
        &session,
        true,
        "apply_commands",
        json!({"base_revision":1,"commands":[command]}),
    ));
    assert!(
        matches!(failure.code, ErrorCode::PermissionDenied),
        "{failure:?}"
    );
    assert_eq!(h.snapshot(), before);
    let allowed = caller("setup-explicit-field", true);
    let session = allowed.session_id.clone();
    h.host.register_external_session(allowed).unwrap();
    let command = settings(&mut h, "Allowed common matrix input", Some(next.clone()));
    let _: EditCommit = host::ok(h.call_as(
        &session,
        true,
        "apply_commands",
        json!({"base_revision":1,"commands":[command.clone()]}),
    ));
    assert_eq!(policies(&mut h), next);
    let committed = h.snapshot();
    let failure = host::error(h.call_as(
        &session,
        true,
        "apply_commands",
        json!({"base_revision":1,"commands":[command]}),
    ));
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    assert_eq!(h.snapshot(), committed);
    assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), frozen);
}
