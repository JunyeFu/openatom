//! Actual Host persistence for presentation-only item layouts.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::fs;
use support::*;
fn layout(x: f64, anchor: bool) -> Value {
    json!({"assignment":"item_id_order","slots":[{"item_type_ref":{"id":"shell","revision":"r1"},"anchor_role":if anchor {json!("workpiece")}else{Value::Null},"offset":{"translation":{"x":x,"y":0.0,"z":0.0},"rotation":{"x":0.0,"y":0.0,"z":0.0,"w":1.0},"scale":{"x":1.0,"y":1.0,"z":1.0},"unit":"meter","coordinate_frame":"right_handed_z_up"}}]})
}
fn seeded() -> Value {
    let mut v = fixture();
    v["project"]["devices"][0]["item_layout"] = layout(0.5, true);
    v["project"]["locations"][0]["item_layout"] = layout(1.0, false);
    v["project"]["item_types"][4]["contents_layout"] = layout(0.0, true);
    v
}
fn commands(h: &mut Harness, x: f64) -> Value {
    let mut d = h.object("device", "ASM-01");
    d["item_layout"] = layout(x, true);
    let mut l = h.object("location", "shells-A");
    l["item_layout"] = layout(x, false);
    let mut i = h.object("item_type", "package");
    i["contents_layout"] = layout(x, true);
    json!([{"kind":"put_object","data":{"kind":"device","data":d}},{"kind":"put_object","data":{"kind":"location","data":l}},{"kind":"put_object","data":{"kind":"item_type","data":i}}])
}
fn frozen(h: &mut Harness, run: &RunId) -> Value {
    let o:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"project","id":"P1"},"source":{"kind":"run_snapshot","data":{"run_id":run,"at_ms":0}},"node_index":null})));
    serde_json::to_value(o).unwrap()["data"].clone()
}
#[test]
fn actual_layout_write_history_frozen_run_and_moved_reopen_preserve_business() {
    let mut baseline = Harness::new();
    let task = baseline.run("LOCAL");
    baseline.finish_one();
    let original = baseline.completed_run(&task);
    let mut h = Harness::new();
    let legacy = h.object("project", "P1");
    assert!(legacy["devices"][0].get("item_layout").is_none());
    let cmds = commands(&mut h, 0.5);
    let _: EditCommit = ok(h.apply(1, cmds));
    let set = h.object("project", "P1");
    assert_eq!(set["devices"][0]["item_layout"], layout(0.5, true));
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"undo"}])));
    assert!(h.object("device", "ASM-01").get("item_layout").is_none());
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"redo"}])));
    let task = h.run("LOCAL");
    h.finish_one();
    let actual = h.completed_run(&task);
    let a = serde_json::to_value(&actual).unwrap();
    let b = serde_json::to_value(&original).unwrap();
    for key in [
        "final_items",
        "events",
        "jobs",
        "resource_claims",
        "phase_work_intervals",
        "acceptance_events",
        "transport",
        "orders",
    ] {
        assert_eq!(a[key], b[key], "business field {key}");
    }
    assert_eq!(actual.acceptance_events[0].at_ms.0.value(), 75000);
    let old = frozen(&mut h, &actual.id);
    let cmds = commands(&mut h, 2.0);
    let _: EditCommit = ok(h.apply(4, cmds));
    assert_eq!(frozen(&mut h, &actual.id), old);
    assert_eq!(
        serde_json::to_value(h.full(&actual.id)).unwrap(),
        serde_json::to_value(&actual).unwrap()
    );
    let before = h.snapshot();
    assert!(matches!(
        error(h.apply(4, json!([]))).code,
        ErrorCode::RevisionConflict
    ));
    assert_eq!(h.snapshot(), before);
    let directory = h.tree.directory("save-layout");
    let task = h.save(5, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let moved = h.tree.path.join("moved-layout");
    fs::rename(&directory, &moved).unwrap();
    let mut fresh = Harness::from_seed(Sandbox::new(), &moved);
    assert_eq!(
        fresh.object("device", "ASM-01")["item_layout"],
        layout(2.0, true)
    );
    assert_eq!(
        fresh.object("location", "shells-A")["item_layout"],
        layout(2.0, false)
    );
    assert_eq!(
        fresh.object("item_type", "package")["contents_layout"],
        layout(2.0, true)
    );
    assert_eq!(frozen(&mut fresh, &actual.id), old);
    assert_eq!(
        serde_json::to_value(fresh.full(&actual.id)).unwrap(),
        serde_json::to_value(&actual).unwrap()
    );
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/媒体表现/runtime-item-layout-producer/actual-host-reception.json");
    write(
        &output,
        &json!({"source_Project":fresh.object("project","P1"),"old_Run_frozen_Project":old,"actual_original_Run":original,"actual_layout_Run":actual,"Save_move_freshHost":true}),
    );
}
fn external(session: &str, fields: &[&str]) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: id(session),
        granted_project: Some("P1".to_owned().try_into().unwrap()),
        allowed_operations: vec![OperationName::GetObject, OperationName::ApplyCommands],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![HostEditGrant {
            id: id("layout-grant"),
            scope: TaskScope::Edit(EditScope::Project("P1".to_owned().try_into().unwrap())),
            branches: vec![HostEditBranch::PutObject],
            objects: vec![HostObjectEditGrant {
                target: ObjectRef {
                    kind: ObjectKind::Device,
                    id: id("ASM-01"),
                },
                fields: fields
                    .iter()
                    .map(|p| HostFieldGrant {
                        pointer: p.to_string(),
                        element_id: None,
                    })
                    .collect(),
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
fn exact_slot_leaf_authority_is_local_and_cannot_add_reorder_or_edit_other_fields() {
    let mut seed = seeded();
    let mut second = layout(1.5, true)["slots"][0].clone();
    second["offset"]["translation"]["x"] = json!(1.5);
    seed["project"]["devices"][0]["item_layout"]["slots"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let mut h = Harness::from_value(seed);
    let original = h.object("device", "ASM-01");
    let before = h.snapshot();
    for (name, fields) in [
        ("parent", vec!["/item_layout"]),
        ("array", vec!["/item_layout/slots"]),
        (
            "wildcard",
            vec!["/item_layout/slots/*/offset/translation/x"],
        ),
    ] {
        h.host
            .register_external_session(external(name, &fields))
            .unwrap();
        let mut d = original.clone();
        d["item_layout"]["slots"][0]["offset"]["translation"]["x"] = json!(0.9);
        let r=h.call_as(&id(name),true,"apply_commands",json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"device","data":d}}]}));
        assert!(matches!(error(r).code, ErrorCode::PermissionDenied));
        assert_eq!(h.snapshot(), before);
    }
    h.host
        .register_external_session(external(
            "exact",
            &["/item_layout/slots/0/offset/translation/x"],
        ))
        .unwrap();
    let mut d = original.clone();
    d["item_layout"]["slots"][0]["offset"]["translation"]["x"] = json!(0.9);
    let _:EditCommit=ok(h.call_as(&id("exact"),true,"apply_commands",json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"device","data":d}}]})));
    let before = h.snapshot();
    let mut d = h.object("device", "ASM-01");
    d["title"] = json!("not granted");
    let r=h.call_as(&id("exact"),true,"apply_commands",json!({"base_revision":2,"commands":[{"kind":"put_object","data":{"kind":"device","data":d}}]}));
    assert!(matches!(error(r).code, ErrorCode::PermissionDenied));
    assert_eq!(h.snapshot(), before);
    let mut d = h.object("device", "ASM-01");
    let mut reversed = d.clone();
    reversed["item_layout"]["slots"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let r=h.call_as(&id("exact"),true,"apply_commands",json!({"base_revision":2,"commands":[{"kind":"put_object","data":{"kind":"device","data":reversed}}]}));
    assert!(matches!(error(r).code, ErrorCode::PermissionDenied));
    assert_eq!(h.snapshot(), before);
    d["item_layout"]["slots"]
        .as_array_mut()
        .unwrap()
        .push(layout(0.0, true)["slots"][0].clone());
    let r=h.call_as(&id("exact"),true,"apply_commands",json!({"base_revision":2,"commands":[{"kind":"put_object","data":{"kind":"device","data":d}}]}));
    assert!(matches!(error(r).code, ErrorCode::UnsupportedCapability));
    assert_eq!(h.snapshot(), before);
}
#[test]
fn unknown_exact_child_type_fails_atomically_but_missing_visual_anchors_do_not_block_run() {
    let mut h = Harness::new();
    let before = h.snapshot();
    let mut cmds = commands(&mut h, 0.0);
    cmds[1]["data"]["data"]["item_layout"]["slots"][0]["item_type_ref"]["revision"] =
        json!("unknown-revision");
    let e = error(h.apply(1, cmds));
    assert!(matches!(e.code, ErrorCode::MissingReference));
    assert_eq!(h.snapshot(), before);
    let cmds = commands(&mut h, 0.0);
    let _: EditCommit = ok(h.apply(1, cmds));
    let issues = h.context().issues;
    assert!(
        issues
            .iter()
            .any(|i| i.code == "item_layout.missing_display_source"
                && i.blocks.is_empty()
                && i.field_path.as_deref() == Some("/devices/0/item_layout/slots/0/anchor_role"))
    );
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 75000);
}

#[test]
fn legacy_null_layouts_omit_and_new_layout_requires_explicit_assignment_and_pose() {
    let v = fixture();
    for (collection, field, kind) in [
        ("devices", "item_layout", "device"),
        ("locations", "item_layout", "location"),
        ("item_types", "contents_layout", "item_type"),
    ] {
        let mut data = v["project"][collection][0].clone();
        data[field] = Value::Null;
        let object: EditableProjectObject =
            serde_json::from_value(json!({"kind":kind,"data":data})).unwrap();
        assert!(
            serde_json::to_value(object).unwrap()["data"]
                .get(field)
                .is_none()
        );
    }
    for remove in ["assignment", "offset"] {
        let mut l = layout(0.0, true);
        if remove == "assignment" {
            l.as_object_mut().unwrap().remove(remove);
        } else {
            l["slots"][0].as_object_mut().unwrap().remove(remove);
        }
        assert!(serde_json::from_value::<ItemSlotLayout>(l).is_err());
    }
}
#[test]
fn definition_draft_scope_cannot_edit_project_layouts() {
    let mut h = Harness::new();
    let definition = fixture()["definitions"][0].clone();
    let c: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(scope) = c.scope else {
        panic!("real draft")
    };
    let before = h.snapshot();
    let commands = commands(&mut h, 0.5);
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"scope-layout","project_id":"P1","scope":{"kind":"definition_draft","data":scope.id},"operation":"apply_commands","params":{"base_revision":scope.revision,"commands":commands}})).unwrap();
    let reply = h.host.dispatch(&h.ui, request);
    assert!(matches!(
        error(reply).code,
        ErrorCode::UnsupportedCapability
    ));
    assert_eq!(h.snapshot(), before);
}

#[test]
fn exact_nullable_anchor_leaf_is_editable_without_authorizing_the_layout_container() {
    let mut v = seeded();
    v["project"]["devices"][0]["item_layout"]["slots"][0]["anchor_role"] = Value::Null;
    let mut h = Harness::from_value(v);
    h.host
        .register_external_session(external("anchor", &["/item_layout/slots/0/anchor_role"]))
        .unwrap();
    let mut d = h.object("device", "ASM-01");
    d["item_layout"]["slots"][0]["anchor_role"] = json!("workpiece");
    let _:EditCommit=ok(h.call_as(&id("anchor"),true,"apply_commands",json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"device","data":d}}]})));
    assert_eq!(
        h.object("device", "ASM-01")["item_layout"]["slots"][0]["anchor_role"],
        "workpiece"
    );
}
fn scoped(h: &mut Harness, draft: &ModuleId, op: &str, params: Value) -> Value {
    let req:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"slot-template","project_id":"P1","scope":{"kind":"definition_draft","data":draft},"operation":op,"params":params})).unwrap();
    h.host.dispatch(&h.ui, req)
}
#[test]
fn actual_template_composition_captures_three_layouts_and_requires_explicit_child_subset() {
    let mut h = Harness::new();
    let cmds = commands(&mut h, 0.5);
    let _: EditCommit = ok(h.apply(1, cmds));
    let project = h.object("project", "P1");
    let definition = json!({"id":"layout.composition","version":"1.0.0","title":"Actual slot subset","definition_state":"design","contribution":{"author":null,"license":null,"assets":[]},"parameters":[],"attribute_schema":[],"payload":{"category":"templates","data":{"project":"template/slots.json","generation_profile":null}}});
    let c: EditCommit = ok(h.apply(
        2,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(draft) = c.scope else {
        panic!("real draft")
    };
    let subset = json!({"layout_scope":"region","halls":project["halls"],"lines":project["lines"],"devices":project["devices"],"connections":project["connections"],"module_refs":project["module_refs"],"resource_requirements":[],"item_types":project["item_types"],"role_requirements":[],"suggested_role_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"seed":null,"locations":project["locations"],"adoption":null});
    let _: EditCommit = ok(scoped(
        &mut h,
        &draft.id,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":2,"definition":definition,"subset":subset,"source_scenario":null}}]}),
    ));
    let object: DomainObject = ok(scoped(
        &mut h,
        &draft.id,
        "get_object",
        json!({"object_id":{"kind":"module","id":"layout.composition"},"source":{"kind":"template_preview","data":{"module_ref":{"id":"layout.composition","version":"1.0.0"},"base_revision":2,"choices":null,"seed":null}},"node_index":null}),
    ));
    let DomainObject::TemplatePreview(preview) = object else {
        panic!("actual template preview")
    };
    let captured = serde_json::to_value(preview.subset).unwrap();
    for (collection, field, index) in [
        ("devices", "item_layout", 0),
        ("locations", "item_layout", 0),
        ("item_types", "contents_layout", 4),
    ] {
        assert_eq!(
            captured[collection][index][field],
            project[collection][index][field]
        );
    }
    let mut missing = subset.clone();
    missing["item_types"]
        .as_array_mut()
        .unwrap()
        .retain(|i| i["id"] != "shell");
    let reply = scoped(
        &mut h,
        &draft.id,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"put_template_composition","data":{"project_revision":2,"definition":definition,"subset":missing,"source_scenario":null}}]}),
    );
    assert!(matches!(error(reply).code, ErrorCode::MissingReference));
    assert_eq!(h.object("project", "P1"), project);
    let object: DomainObject = ok(scoped(
        &mut h,
        &draft.id,
        "get_object",
        json!({"object_id":{"kind":"module","id":"layout.composition"},"source":{"kind":"template_preview","data":{"module_ref":{"id":"layout.composition","version":"1.0.0"},"base_revision":2,"choices":null,"seed":null}},"node_index":null}),
    ));
    let DomainObject::TemplatePreview(after) = object else {
        panic!("unchanged actual preview")
    };
    assert_eq!(serde_json::to_value(after.subset).unwrap(), captured);
}

#[test]
fn newly_created_layout_requires_every_explicit_creation_leaf_and_commits_one_budget() {
    let mut h = Harness::new();
    let mut item = h.object("item_type", "package");
    item["id"] = json!("created-layout-owner");
    item["contents_layout"] = layout(0.5, true);
    let fields = [
        "/revision",
        "/title",
        "/unit",
        "/tracking",
        "/spec_ref",
        "/contents_layout/assignment",
        "/contents_layout/slots/0/item_type_ref/id",
        "/contents_layout/slots/0/item_type_ref/revision",
        "/contents_layout/slots/0/anchor_role",
        "/contents_layout/slots/0/offset/translation/x",
        "/contents_layout/slots/0/offset/translation/y",
        "/contents_layout/slots/0/offset/translation/z",
        "/contents_layout/slots/0/offset/rotation/x",
        "/contents_layout/slots/0/offset/rotation/y",
        "/contents_layout/slots/0/offset/rotation/z",
        "/contents_layout/slots/0/offset/rotation/w",
        "/contents_layout/slots/0/offset/scale/x",
        "/contents_layout/slots/0/offset/scale/y",
        "/contents_layout/slots/0/offset/scale/z",
        "/contents_layout/slots/0/offset/unit",
        "/contents_layout/slots/0/offset/coordinate_frame",
    ];
    for (name, complete) in [("create-incomplete", false), ("create-exact", true)] {
        let mut caller = external(name, &[]);
        caller.edit_grants[0].objects.clear();
        caller.edit_grants[0].creates = vec![HostCreateAllowance {
            kind: ObjectKind::ItemType,
            max_new_objects: SafeUint::try_from(1u64).unwrap(),
            fields: fields
                .iter()
                .filter(|p| complete || **p != "/contents_layout/slots/0/offset/translation/x")
                .map(|p| HostFieldGrant {
                    pointer: (*p).into(),
                    element_id: None,
                })
                .collect(),
        }];
        h.host.register_external_session(caller).unwrap();
        let before = h.snapshot();
        let r=h.call_as(&id(name),true,"apply_commands",json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"item_type","data":item}}]}));
        if complete {
            let _: EditCommit = ok(r);
            assert_eq!(
                h.host.caller_context(&id(name)).unwrap().edit_grants[0]
                    .created_objects
                    .len(),
                1
            );
        } else {
            assert!(matches!(error(r).code, ErrorCode::PermissionDenied));
            assert_eq!(h.snapshot(), before);
            assert!(
                h.host.caller_context(&id(name)).unwrap().edit_grants[0]
                    .created_objects
                    .is_empty()
            );
        }
    }
    assert_eq!(
        h.object("item_type", "created-layout-owner")["contents_layout"],
        layout(0.5, true)
    );
}

#[test]
fn resolved_exact_media_anchor_does_not_need_nodes_copied_into_device_wrapper() {
    let mut h = Harness::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    h.host
        .register_bundled_module_directory(&root.join("modules/devices/template-support/media"))
        .unwrap();
    let _:EditCommit=ok(h.apply(1,json!([{"kind":"adopt_module","data":{"module_ref":{"id":"p401.support.media","version":"1.0.0"},"targets":[{"kind":"device","id":"ASM-01"}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
    let mut device = h.object("device", "ASM-01");
    assert!(
        device["presentation_override"]["nodes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        device["presentation_override"]["anchor_roles"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    let mut input = layout(0.0, true);
    input["slots"][0]["anchor_role"] = json!("input");
    device["item_layout"] = input;
    let _: EditCommit = ok(h.apply(
        2,
        json!([{"kind":"put_object","data":{"kind":"device","data":device}}]),
    ));
    assert!(!h.context().issues.iter().any(|i| {
        i.code.starts_with("item_layout.")
            && i.object_ref
                .as_ref()
                .is_some_and(|r| r.id.as_str() == "ASM-01")
    }));
    let task = h.run("LOCAL");
    h.finish_one();
    assert_eq!(
        h.completed_run(&task).acceptance_events[0].at_ms.0.value(),
        75000
    );
}
