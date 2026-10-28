//! Public Host consumers for initial array content under exact creation grants.
//! Existing-object structural editing is intentionally not widened.
#[path = "../../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

fn fields(paths: &[&str]) -> Vec<HostFieldGrant> {
    paths
        .iter()
        .map(|path| HostFieldGrant {
            pointer: (*path).into(),
            element_id: None,
        })
        .collect()
}
const PATHS: &[&str] = &[
    "/title",
    "/source",
    "/intervals/0/start_ms",
    "/intervals/0/end_ms",
];
fn caller(h: &mut Harness, paths: &[&str], maximum: u64) -> ObjectId {
    let session = id("exact-calendar-creator");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some("P1".to_owned().try_into().unwrap()),
            allowed_operations: vec![OperationName::ApplyCommands],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![HostEditGrant {
                id: id("calendar-creation-grant"),
                scope: TaskScope::Edit(EditScope::Project("P1".to_owned().try_into().unwrap())),
                branches: vec![HostEditBranch::PutObject],
                objects: vec![HostObjectEditGrant {
                    target: ObjectRef {
                        kind: ObjectKind::Calendar,
                        id: id("ALL"),
                    },
                    fields: fields(&["/intervals/0/end_ms"]),
                    can_remove: false,
                }],
                creates: vec![HostCreateAllowance {
                    kind: ObjectKind::Calendar,
                    max_new_objects: SafeUint::try_from(maximum).unwrap(),
                    fields: fields(paths),
                }],
                source_versions: vec![],
                reusable_resources: vec![],
                created_objects: vec![],
            }],
        })
        .unwrap();
    session
}
fn calendar(name: &str) -> Value {
    json!({"id":name,"title":"explicit example shift","source":"user_provided",
        "intervals":[{"start_ms":0,"end_ms":240000}]})
}
fn apply(h: &mut Harness, session: &ObjectId, base: u64, commands: Value) -> Value {
    h.call_as(
        session,
        true,
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    )
}

#[test]
fn exact_initial_nonempty_calendar_commits_then_distinct_id_budget_is_not_refunded() {
    let mut h = Harness::new();
    let session = caller(&mut h, PATHS, 1);
    let first: EditCommit = ok(apply(
        &mut h,
        &session,
        1,
        json!([put("calendar", calendar("NEW-CAL"))]),
    ));
    assert_eq!(first.new_revision.0.value(), 2);
    assert_eq!(h.object("calendar", "NEW-CAL"), calendar("NEW-CAL"));
    let before = h.snapshot();
    assert!(matches!(
        error(apply(
            &mut h,
            &session,
            2,
            json!([put("calendar", calendar("SECOND-CAL"))])
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
    assert_eq!(
        h.host.caller_context(&session).unwrap().edit_grants[0]
            .created_objects
            .len(),
        1
    );
}

#[test]
fn missing_initial_array_leaf_permission_denies_without_commit() {
    let mut h = Harness::new();
    let session = caller(&mut h, &["/title", "/source", "/intervals/0/start_ms"], 1);
    let before = h.snapshot();
    assert!(matches!(
        error(apply(
            &mut h,
            &session,
            1,
            json!([put("calendar", calendar("NEW-CAL"))])
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
    assert!(
        h.host.caller_context(&session).unwrap().edit_grants[0]
            .created_objects
            .is_empty()
    );
}

#[test]
fn unauthorized_second_array_slot_is_atomic_and_does_not_consume_creation_budget() {
    let mut h = Harness::new();
    let session = caller(&mut h, PATHS, 2);
    let mut extra = calendar("SECOND-CAL");
    extra["intervals"]
        .as_array_mut()
        .unwrap()
        .push(json!({"start_ms":240000,"end_ms":480000}));
    let before = h.snapshot();
    assert!(matches!(
        error(apply(
            &mut h,
            &session,
            1,
            json!([put("calendar", calendar("NEW-CAL")), put("calendar", extra)])
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
    assert!(
        h.host.caller_context(&session).unwrap().edit_grants[0]
            .created_objects
            .is_empty()
    );
    let committed: EditCommit = ok(apply(
        &mut h,
        &session,
        1,
        json!([put("calendar", calendar("NEW-CAL"))]),
    ));
    assert_eq!(committed.new_revision.0.value(), 2);
    assert_eq!(
        h.host.caller_context(&session).unwrap().edit_grants[0]
            .created_objects
            .len(),
        1
    );
}

#[test]
fn existing_non_id_array_edit_remains_unsupported_and_preserves_the_project() {
    let mut h = Harness::new();
    let session = caller(&mut h, PATHS, 1);
    let mut existing = h.object("calendar", "ALL");
    existing["intervals"][0]["end_ms"] = json!(230000);
    let before = h.snapshot();
    assert!(matches!(
        error(apply(
            &mut h,
            &session,
            1,
            json!([put("calendar", existing)])
        ))
        .code,
        ErrorCode::UnsupportedCapability
    ));
    assert_eq!(h.snapshot(), before);
}
