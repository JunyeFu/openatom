//! Actual Core Host consumers; these tests never claim native product UI acceptance.
use super::host::*;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../packages/commands/tests/fixtures/quality/quality-original.json"
    ))
    .unwrap()
}
fn exact(h: &mut Harness, id: &str, version: &str) -> Value {
    h.call("get_object",json!({"object_id":{"kind":"module","id":id},"source":{"kind":"module_version","data":{"module_ref":{"id":id,"version":version}}},"node_index":null}))
}
#[test]
fn exact_device_versions_are_separate_real_sources_and_unreferenced_or_mismatched_reads_never_choose_latest()
 {
    let mut v = fixture();
    let module = "w5.station.ASM-01";
    let old = v["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == module)
        .unwrap()
        .clone();
    let mut new = old.clone();
    new["version"] = json!("0.2.0");
    new["title"] = json!("independent exact second version");
    v["definitions"].as_array_mut().unwrap().push(new);
    v["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":module,"version":"0.2.0"}));
    let mut h = Harness::from_value(v);
    for (version, title) in [
        ("0.1.0", old["title"].clone()),
        ("0.2.0", json!("independent exact second version")),
    ] {
        let value: DomainObject = ok(exact(&mut h, module, version));
        let value = serde_json::to_value(value).unwrap();
        assert_eq!(value["data"]["definition"]["version"], version);
        assert_eq!(value["data"]["definition"]["title"], title);
    }
    let ambiguous=error(h.call("get_object",json!({"object_id":{"kind":"module","id":module},"source":{"kind":"draft"},"node_index":null})));
    assert!(matches!(
        ambiguous.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert!(matches!(
        error(exact(&mut h, module, "9.0.0")).code,
        ErrorCode::MissingReference
    ));
    let wrong=error(h.call("get_object",json!({"object_id":{"kind":"device","id":"Assembly01"},"source":{"kind":"module_version","data":{"module_ref":{"id":module,"version":"0.1.0"}}},"node_index":null})));
    assert!(matches!(wrong.code, ErrorCode::InvalidInput));
}
#[test]
fn real_saved_run_summaries_are_discovered_after_move_reopen_without_session_task_inference() {
    let mut h = Harness::from_value(fixture());
    let task = h.run("QUALITY-ORIGINAL");
    h.finish_one();
    let run = h.completed_run(&task);
    let before: RunView = ok(h.call(
        "get_run",
        json!({"run_id":run.id,"query":{"kind":"summary"}}),
    ));
    let expected = match before {
        RunView::Summary(s) => s,
        _ => panic!("actual summary required"),
    };
    let saved = h.tree.directory("saved-for-ui");
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("moved-for-ui");
    copy(&saved, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    let context = reopened.context();
    let summaries = serde_json::to_value(&context).unwrap()["run_summaries"].clone();
    assert_eq!(summaries.as_array().unwrap().len(), 1);
    assert_eq!(summaries[0], serde_json::to_value(expected).unwrap());
    assert_eq!(summaries[0]["run_id"], run.id.as_str());
    // Summary must preserve the existing producer's known/unknown value. The
    // actual cumulative count remains available in this same frozen full Run.
    assert_eq!(
        run.acceptance_events
            .iter()
            .map(|e| e.quantity.value.coefficient.value())
            .sum::<u64>(),
        2
    );
    assert_eq!(summaries[0]["goal_met"], false);
    assert_eq!(
        serde_json::to_value(reopened.full(&run.id)).unwrap(),
        serde_json::to_value(run).unwrap()
    );
}
#[test]
fn actual_paged_observation_uses_requested_time_and_frozen_project_after_new_draft_edits() {
    let mut h = Harness::from_value(fixture());
    let task = h.run("QUALITY-ORIGINAL");
    h.finish_one();
    let run = h.completed_run(&task);
    let frozen = h.object("project", "P1");
    let mut changed = h.object("device", "Assembly01");
    changed["pose"]["translation"]["x"] = json!(99);
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("device", changed)])));
    let project:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"project","id":"P1"},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":0}},"node_index":null})));
    assert_eq!(serde_json::to_value(project).unwrap()["data"], frozen);
    let mut cursor = None;
    let mut all = vec![];
    loop {
        let page:RunView=ok(h.call("get_run",json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":0,"objects":[],"page":{"after_index":cursor,"limit":1}}}})));
        let RunView::Objects(page) = page else {
            panic!("real object page required")
        };
        all.extend(page.objects);
        cursor = page.next_index;
        if cursor.is_none() {
            break;
        }
    }
    let serialized = serde_json::to_value(all).unwrap();
    assert!(
        serialized
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "item" && o["data"]["id"] == "shell01")
    );
    assert!(
        !serialized
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "item" && o["data"]["item_type_ref"]["id"] == "assembly")
    );
    let third =
        fixture()["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["item_ref"]
            .clone();
    let at80:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"item","id":third},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":80000}},"node_index":null})));
    let at80 = serde_json::to_value(at80).unwrap();
    assert_eq!(at80["data"]["quality"], "pending");
    assert_eq!(at80["data"]["holder"]["id"], "pending-store");
    let beyond=error(h.call("get_run",json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":120001,"objects":[],"page":{"after_index":null,"limit":1}}}})));
    assert!(matches!(beyond.code, ErrorCode::InvalidInput));
    assert!(
        run.final_items
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "assembly")
    );
    assert_eq!(
        serde_json::to_value(h.full(&run.id)).unwrap(),
        serde_json::to_value(run).unwrap()
    );
    assert_eq!(
        h.object("device", "Assembly01")["pose"]["translation"]["x"].as_f64(),
        Some(99.0)
    );
}

#[test]
fn actual_common_undo_redo_new_revision_conflict_and_project_switch_preserve_old_run_and_clear_branch()
 {
    let mut h = Harness::from_value(fixture());
    let task = h.run("QUALITY-ORIGINAL");
    h.finish_one();
    let run = h.completed_run(&task);
    let mut device = h.object("device", "Assembly01");
    let original = device.clone();
    let base = h.context().project.unwrap().revision.0.value();
    device["pose"]["translation"]["x"] = json!(99.0);
    let moved: EditCommit = ok(h.apply(base, json!([put("device", device)])));
    let undone: EditCommit = ok(h.apply(moved.new_revision.0.value(), json!([{"kind":"undo"}])));
    assert!(undone.new_revision.0.value() > moved.new_revision.0.value());
    assert!(undone.history.can_redo);
    assert_eq!(h.object("device", "Assembly01"), original);
    let conflict = error(h.apply(base, json!([{"kind":"redo"}])));
    assert!(matches!(conflict.code, ErrorCode::RevisionConflict));
    let redone: EditCommit = ok(h.apply(undone.new_revision.0.value(), json!([{"kind":"redo"}])));
    assert_eq!(
        h.object("device", "Assembly01")["pose"]["translation"]["x"].as_f64(),
        Some(99.0)
    );
    let undone: EditCommit = ok(h.apply(redone.new_revision.0.value(), json!([{"kind":"undo"}])));
    let mut branch = h.object("device", "Assembly01");
    branch["title"] = json!("new edit invalidates redo");
    let branch: EditCommit = ok(h.apply(
        undone.new_revision.0.value(),
        json!([put("device", branch)]),
    ));
    assert!(!branch.history.can_redo);
    assert!(matches!(
        error(h.apply(branch.new_revision.0.value(), json!([{"kind":"redo"}]))).code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(
        serde_json::to_value(h.full(&run.id)).unwrap(),
        serde_json::to_value(run).unwrap()
    );
    let created:EditCommit=ok(h.call_as(&h.ui.clone(),false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"another project history boundary","time_reference":{"label":"new explicit clock"},"departure":"discard_after_tasks_terminal"}}]})));
    let context: ProjectContext =
        ok(h.call_as(&h.ui.clone(), false, "get_context", json!({"selection":[]})));
    assert!(context.run_summaries.is_empty());
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"new-project-no-history","project_id":created.project_id,"scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":created.new_revision,"commands":[{"kind":"undo"}]}})).unwrap();
    assert!(matches!(
        error(h.host.dispatch(&h.ui, request)).code,
        ErrorCode::InvalidInput
    ));
}
