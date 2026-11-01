//! Real frozen static instance declarations, not runtime resource/pose snapshots.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::sync::atomic::{AtomicU64, Ordering};
use support::*;

static NEXT: AtomicU64 = AtomicU64::new(1);
fn presentation(mark: f64) -> Value {
    let template: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/media/module.json"
    ))
    .unwrap();
    let mut p = template["data"]["definition"]["payload"]["data"].clone();
    let mut root = p["nodes"][0].clone();
    root["id"] = json!(91);
    root["transform"]["translation"]["x"] = json!(mark);
    root["geometry"]["data"]["x"] = json!(mark);
    let mut child = root.clone();
    child["id"] = json!(702);
    child["parent_index"] = json!(0);
    child["material"]["roughness"] = json!(0.43);
    child["transform"]["translation"]["z"] = json!(0.25);
    p["nodes"] = json!([root, child]);
    for field in ["poses", "actions", "timeline", "tags"] {
        p[field] = json!([]);
    }
    p["anchor_roles"] = json!({});
    p["port_anchors"] = json!({});
    p["lod"]["detailed"] = Value::Null;
    p["lod"]["simplified"] = Value::Null;
    serde_json::to_value(serde_json::from_value::<Presentation>(p).unwrap()).unwrap()
}
fn setup() -> (Harness, String) {
    let mut value = fixture();
    value["project"]["devices"][0]["presentation_override"] = presentation(1.0);
    value["project"]["resources"][0]["presentation"] = presentation(1.0);
    let resource = value["project"]["resources"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (Harness::from_value(value), resource)
}
fn params(kind: &str, id: &str, run: &RunId, at: u64, index: Value) -> Value {
    json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"run_snapshot","data":{"run_id":run,"at_ms":at}},"node_index":index})
}
fn object(h: &mut Harness, kind: &str, id: &str, run: &RunId, at: u64, index: Value) -> Value {
    let result: DomainObject = ok(h.call("get_object", params(kind, id, run, at, index)));
    serde_json::to_value(result).unwrap()
}
fn scoped(
    h: &mut Harness,
    session: &ObjectId,
    scope: Value,
    project: Value,
    operation: &str,
    params: Value,
) -> Value {
    let request = serde_json::from_value(json!({"api_version":"0.1","request_id":format!("frozen-instance-{}",NEXT.fetch_add(1,Ordering::Relaxed)),
        "project_id":project,"scope":scope,"operation":operation,"params":params})).unwrap();
    h.host.dispatch(session, request)
}
fn create(h: &mut Harness, definition: Value) -> ModuleId {
    let base = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(scope) = commit.scope else {
        panic!("draft")
    };
    scope.id
}
fn draft_call(h: &mut Harness, draft: &ModuleId, operation: &str, params: Value) -> Value {
    scoped(
        h,
        &h.ui.clone(),
        json!({"kind":"definition_draft","data":draft}),
        json!("P1"),
        operation,
        params,
    )
}
fn completed(h: &mut Harness) -> RunResult {
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 75000);
    run
}
fn objects_at(h: &mut Harness, run: &RunId, kind: &str, id: &str) -> Value {
    h.call(
        "get_run",
        json!({"run_id":run,"query":{"kind":"objects_at","data":{"at_ms":0,
        "objects":[{"kind":kind,"id":id}],"page":{"after_index":null,"limit":1}}}}),
    )
}
fn reject(reply: Value, code: ErrorCode) -> ApiError {
    let failure = error(reply);
    assert_eq!(
        format!("{:?}", failure.code),
        format!("{code:?}"),
        "{failure:#?}"
    );
    failure
}
fn state(h: &mut Harness) -> Value {
    json!({"objects":h.snapshot(),"context":h.context()})
}
fn external(name: &str, project: &str, operations: Vec<OperationName>) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: id(name),
        granted_project: Some(ProjectId::try_from(project.to_owned()).unwrap()),
        allowed_operations: operations,
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![],
    }
}

#[test]
fn frozen_instances_and_nodes_stay_r1_after_put_undo_redo_save_move_and_fresh_host() {
    let (mut h, resource) = setup();
    let old_device = h.object("device", "ASM-01");
    let old_resource = h.object("resource", &resource);
    let old_run = completed(&mut h);
    assert_eq!(old_run.input_ref.project_revision.0.value(), 1);
    for at in [
        0,
        75000,
        old_run.computed_until_ms.as_ref().unwrap().0.value(),
    ] {
        assert_eq!(
            object(&mut h, "device", "ASM-01", &old_run.id, at, Value::Null)["data"],
            old_device
        );
        assert_eq!(
            object(&mut h, "resource", &resource, &old_run.id, at, json!(1))["data"],
            presentation(1.0)["nodes"][1]
        );
    }
    let mut new_device = old_device.clone();
    new_device["presentation_override"] = presentation(2.0);
    let mut new_resource = old_resource.clone();
    new_resource["presentation"] = presentation(2.0);
    let _: EditCommit = ok(h.apply(
        1,
        json!([
            put("device", new_device.clone()),
            put("resource", new_resource.clone())
        ]),
    ));
    assert_eq!(h.context().project.unwrap().revision.0.value(), 2);
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(h.object("device", "ASM-01"), old_device);
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(h.object("device", "ASM-01"), new_device);
    assert_eq!(h.object("resource", &resource), new_resource);
    let current_run = completed(&mut h);
    assert_eq!(current_run.input_ref.project_revision.0.value(), 4);
    let old = serde_json::to_value(&old_run).unwrap();
    let current = serde_json::to_value(&current_run).unwrap();
    for key in old
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| !matches!(key.as_str(), "id" | "input_ref" | "limitations"))
    {
        assert_eq!(old[key], current[key], "unchanged business field {key}");
    }
    let before = state(&mut h);
    for (kind, id, old, field) in [
        ("device", "ASM-01", &old_device, "presentation_override"),
        ("resource", resource.as_str(), &old_resource, "presentation"),
    ] {
        assert_eq!(
            object(&mut h, kind, id, &old_run.id, 0, Value::Null)["data"],
            *old
        );
        assert_eq!(
            object(&mut h, kind, id, &old_run.id, 0, json!(1)),
            json!({"kind":"scene_node","data":old[field]["nodes"][1]})
        );
        assert_eq!(
            object(&mut h, kind, id, &current_run.id, 0, json!(1))["data"],
            presentation(2.0)["nodes"][1]
        );
    }
    assert_eq!(state(&mut h), before);
    assert_eq!(serde_json::to_value(h.full(&old_run.id)).unwrap(), old);
    assert!(h.host.drain_io_work().is_empty());
    let saved = h.tree.directory("saved");
    let task = h.save(4, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.path.join("physical-move");
    std::fs::rename(&saved, &moved).unwrap();
    assert!(!saved.exists());
    let mut fresh = Harness::from_seed(tree, &moved);
    let before = state(&mut fresh);
    for (kind, id, old, field) in [
        ("device", "ASM-01", &old_device, "presentation_override"),
        ("resource", resource.as_str(), &old_resource, "presentation"),
    ] {
        assert_eq!(
            object(&mut fresh, kind, id, &old_run.id, 0, Value::Null)["data"],
            *old
        );
        assert_eq!(
            object(&mut fresh, kind, id, &old_run.id, 0, json!(1))["data"],
            old[field]["nodes"][1]
        );
        assert_eq!(
            object(&mut fresh, kind, id, &current_run.id, 0, json!(1))["data"],
            presentation(2.0)["nodes"][1]
        );
        reject(
            objects_at(&mut fresh, &old_run.id, kind, id),
            ErrorCode::UnsupportedCapability,
        );
    }
    assert_eq!(fresh.object("device", "ASM-01"), new_device);
    assert_eq!(fresh.object("resource", &resource), new_resource);
    assert_eq!(serde_json::to_value(fresh.full(&old_run.id)).unwrap(), old);
    assert_eq!(state(&mut fresh), before);
    write(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scenarios/媒体表现/runtime-frozen-instance-nodes/actual-reception.json"),
        &json!({"evidence":"actual_same_Host_ProjectR1_putR2_undoR3_redoR4_Save_physical_move_fresh_Host",
        "frozen_device_r1":old_device,"frozen_resource_r1":old_resource,"current_device_after_redo_r4":new_device,"current_resource_after_redo_r4":new_resource,
        "old_run":old_run,"current_run":current_run,"business_fields_equal":true,"objects_at_device_resource":"UnsupportedCapability",
        "static_frozen_input_not_runtime_pose":true,"ui_native_rendering_or_human_accepted":false}),
    );
}

#[test]
fn exact_ids_canonical_indexes_and_empty_media_shells_never_select_other_owners() {
    let (mut h, resource) = setup();
    h.host
        .register_bundled_module_directory(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/devices/template-support/media"),
        )
        .unwrap();
    let media = json!({"id":"p401.support.media","version":"1.0.0"});
    let _: EditCommit = ok(h.apply(1,json!([{"kind":"adopt_module","data":{"module_ref":media,"targets":[{"kind":"device","id":"ASM-01"}],
        "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
    let wrapper = h.object("device", "ASM-01")["presentation_override"].clone();
    assert!(wrapper["nodes"].as_array().unwrap().is_empty());
    let mut r = h.object("resource", &resource);
    r["presentation"] = wrapper;
    let _: EditCommit = ok(h.apply(2, json!([put("resource", r.clone())])));
    let run = completed(&mut h);
    let before = state(&mut h);
    for (kind, id) in [
        ("device", "ASM-01"),
        ("resource", resource.as_str()),
        ("device", "PACK-01"),
    ] {
        let own = h.object(kind, id);
        assert_eq!(
            object(&mut h, kind, id, &run.id, 0, Value::Null)["data"],
            own
        );
        let failure = reject(
            h.call("get_object", params(kind, id, &run.id, 0, json!(0))),
            ErrorCode::MissingReference,
        );
        assert_eq!(failure.field_errors[0].pointer, "/params/node_index");
    }
    // Empty wrapper cannot borrow this real final Media owner's node.
    let media_node: DomainObject = ok(h.call("get_object",json!({"object_id":{"kind":"module","id":"p401.support.media"},
        "source":{"kind":"run_module_version","data":{"run_id":run.id,"at_ms":0,"module_ref":media}},"node_index":5})));
    assert_eq!(
        serde_json::to_value(media_node).unwrap()["data"]["name"],
        json!("anchor:input")
    );
    for (kind, id) in [
        ("device", "no-such-device"),
        ("resource", "ASM-01"),
        ("device", resource.as_str()),
    ] {
        let failure = reject(
            h.call("get_object", params(kind, id, &run.id, 0, Value::Null)),
            ErrorCode::MissingReference,
        );
        assert_eq!(failure.object_refs[0].id.as_str(), id);
    }
    assert_eq!(state(&mut h), before);
    // Inline nodes prove canonical position differs from both 91/702 IDs.
    let (mut inline, resource) = setup();
    let run = completed(&mut inline);
    for kind_id in [("device", "ASM-01"), ("resource", resource.as_str())] {
        assert_eq!(
            object(&mut inline, kind_id.0, kind_id.1, &run.id, 0, json!(1))["data"],
            presentation(1.0)["nodes"][1]
        );
        for index in [2, 91, 702, 9007199254740991u64] {
            let failure = reject(
                inline.call(
                    "get_object",
                    params(kind_id.0, kind_id.1, &run.id, 0, json!(index)),
                ),
                ErrorCode::MissingReference,
            );
            assert_eq!(failure.field_errors[0].pointer, "/params/node_index");
        }
    }
}

#[test]
fn original_project_scope_run_frontier_and_get_object_get_run_authorities_precede_details() {
    let (mut h, resource) = setup();
    let run = completed(&mut h);
    for (name, project, operations) in [
        (
            "frozen-both",
            "P1",
            vec![OperationName::GetObject, OperationName::GetRun],
        ),
        ("frozen-no-run", "P1", vec![OperationName::GetObject]),
        ("frozen-no-object", "P1", vec![OperationName::GetRun]),
        (
            "frozen-foreign",
            "P2",
            vec![OperationName::GetObject, OperationName::GetRun],
        ),
    ] {
        h.host
            .register_external_session(external(name, project, operations))
            .unwrap();
    }
    let before = state(&mut h);
    for (kind, id) in [("device", "ASM-01"), ("resource", resource.as_str())] {
        let result: DomainObject = ok(h.call_as(
            &support::id("frozen-both"),
            true,
            "get_object",
            params(kind, id, &run.id, 0, json!(1)),
        ));
        assert_eq!(
            serde_json::to_value(result).unwrap()["data"],
            presentation(1.0)["nodes"][1]
        );
    }
    for name in [
        "frozen-no-run",
        "frozen-no-object",
        "frozen-foreign",
        "frozen-unknown",
    ] {
        let failure = reject(
            h.call_as(
                &id(name),
                true,
                "get_object",
                params("device", "unknown-id", &run.id, 240001, json!(900)),
            ),
            ErrorCode::PermissionDenied,
        );
        assert!(failure.field_errors.is_empty() && failure.object_refs.is_empty());
    }
    let ui = h.ui.clone();
    reject(
        scoped(
            &mut h,
            &ui,
            json!({"kind":"project"}),
            json!("P2"),
            "get_object",
            params("device", "ASM-01", &run.id, 0, Value::Null),
        ),
        ErrorCode::PermissionDenied,
    );
    reject(
        h.call_as(
            &ui,
            false,
            "get_object",
            params("resource", &resource, &run.id, 0, Value::Null),
        ),
        ErrorCode::UnsupportedCapability,
    );
    let unknown: RunId = serde_json::from_value(json!("no-such-frozen-run")).unwrap();
    reject(
        h.call(
            "get_object",
            params("device", "ASM-01", &unknown, 0, Value::Null),
        ),
        ErrorCode::MissingReference,
    );
    for at in [240001, 9007199254740991u64] {
        let failure = reject(
            h.call(
                "get_object",
                params("resource", "missing-resource", &run.id, at, json!(900)),
            ),
            ErrorCode::InvalidInput,
        );
        assert_eq!(failure.field_errors[0].pointer, "/params/source/data/at_ms");
    }
    let draft = create(&mut h, fixture()["definitions"][4].clone());
    let failure = reject(
        draft_call(
            &mut h,
            &draft,
            "get_object",
            params("device", "ASM-01", &run.id, 0, Value::Null),
        ),
        ErrorCode::InvalidInput,
    );
    assert_eq!(failure.field_errors[0].pointer, "/params/source");
    for kind_id in [("device", "ASM-01"), ("resource", resource.as_str())] {
        reject(
            objects_at(&mut h, &run.id, kind_id.0, kind_id.1),
            ErrorCode::UnsupportedCapability,
        );
    }
    reject(
        h.call_as(
            &id("frozen-both"),
            true,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"undo"}]}),
        ),
        ErrorCode::PermissionDenied,
    );
    assert_eq!(state(&mut h), before);
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn definition_cycle_private_scope_keeps_existing_sources_and_never_falls_back_to_instances() {
    let (mut h, _) = setup();
    let mut definition = fixture()["definitions"][4].clone();
    definition["payload"]["data"]["presentation"] = presentation(5.0);
    let draft = create(&mut h, definition.clone());
    let scenario = h.object("scenario", "LOCAL");
    let task: TaskHandle = ok(draft_call(
        &mut h,
        &draft,
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":1,
        "observation_window":scenario["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":1,"test_input":scenario}}}),
    ));
    h.finish_one();
    let state: TaskState = ok(draft_call(
        &mut h,
        &draft,
        "get_task",
        json!({"task_id":task.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let Some(TaskOutput::Run(run)) = state.result else {
        panic!("sample")
    };
    let before = h.snapshot();
    let result: DomainObject = ok(draft_call(
        &mut h,
        &draft,
        "get_object",
        params("definition_draft", draft.as_str(), &run.run_id, 0, json!(1)),
    ));
    assert_eq!(
        serde_json::to_value(result).unwrap()["data"],
        presentation(5.0)["nodes"][1]
    );
    reject(
        draft_call(
            &mut h,
            &draft,
            "get_object",
            params("device", "ASM-01", &run.run_id, 0, Value::Null),
        ),
        ErrorCode::MissingReference,
    );
    reject(
        h.call(
            "get_object",
            params("device", "ASM-01", &run.run_id, 0, Value::Null),
        ),
        ErrorCode::PermissionDenied,
    );
    let other = create(&mut h, definition);
    reject(
        draft_call(
            &mut h,
            &other,
            "get_object",
            params("device", "ASM-01", &run.run_id, 0, Value::Null),
        ),
        ErrorCode::PermissionDenied,
    );
    let failure = reject(
        draft_call(
            &mut h,
            &draft,
            "get_object",
            params("device", "ASM-01", &run.run_id, 240001, json!(1)),
        ),
        ErrorCode::InvalidInput,
    );
    assert_eq!(failure.field_errors[0].pointer, "/params/source/data/at_ms");
    assert_eq!(h.snapshot(), before);
}
