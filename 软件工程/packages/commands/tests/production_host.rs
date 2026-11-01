//! Whole P0-02/P1-01: actual public Host calls consume the one production Engine.
#[path = "fixtures/production/p1_engine_support.rs"]
mod p1_support;
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;
#[path = "../../../tests/planner/acceptance/library_host.rs"]
mod library_host;

#[test]
fn production_work_freezes_the_real_input_while_a_new_project_revision_remains_editable() {
    let mut h = Harness::new();
    let frozen = h.object("scenario", "LOCAL");
    let handle = h.run("LOCAL");
    assert!(matches!(handle.state, ComputationStatus::Queued));
    assert!(h.task(&handle, true).result.is_none());
    let mut works = h.host.drain_io_work();
    assert_eq!(works.len(), 1);
    let mut changed = frozen.clone();
    changed["title"] = json!("new draft while old Run computes");
    let edit: EditCommit = ok(h.apply(1, json!([put("scenario", changed.clone())])));
    assert_eq!(edit.new_revision.0.value(), 2);
    h.host.finish_io(works.pop().unwrap().run());
    let result = h.completed_run(&handle);
    assert_eq!(result.input_ref.project_revision.0.value(), 1);
    assert_eq!(result.input_ref.scenario_revision.0.value(), 1);
    assert!(matches!(
        result.evidence_origin,
        EvidenceOrigin::ActualSoftware
    ));
    assert!(result.whole_window_computed);
    assert_eq!(result.acceptance_events.len(), 1);
    assert_eq!(result.acceptance_events[0].at_ms.0.value(), 75000);
    assert_eq!(h.object("project", "P1")["revision"], 2);
    same_business(h.object("scenario", "LOCAL"), changed);
    assert!(h.context().project.unwrap().dirty);
    let repeat = h.full(&result.id);
    assert_eq!(
        serde_json::to_value(repeat).unwrap(),
        serde_json::to_value(result).unwrap()
    );
}

#[test]
fn execution_issues_stale_inputs_and_other_sessions_do_not_queue_a_production_run() {
    let mut h = Harness::new();
    let original = h.object("scenario", "LOCAL");
    let mut missing = original.clone();
    missing["orders"][0]["quantity"] = Value::Null;
    let _: EditCommit = ok(h.apply(1, json!([put("scenario", missing)])));
    let before = h.snapshot();
    let failure = error(h.call(
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":2,
        "observation_window":original["window"],"intent":{"kind":"project_scenario"}}),
    ));
    assert!(!matches!(failure.code, ErrorCode::UnsupportedCapability));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
    assert!(h.host.drain_io_work().is_empty());
    let stale = error(h.call(
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":1,
        "observation_window":original["window"],"intent":{"kind":"project_scenario"}}),
    ));
    assert!(matches!(stale.code, ErrorCode::RevisionConflict));
    assert_eq!(h.snapshot(), before);
    let session = id("independent-ungranted-run-session");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::GetContext],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    assert!(matches!(
        error(h.call_as(
            &session,
            true,
            "run_scenario",
            json!({"scenario_id":"LOCAL","input_revision":2,
        "observation_window":original["window"],"intent":{"kind":"project_scenario"}})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn cancelling_computation_does_not_cancel_business_demand_and_a_completed_task_still_wins() {
    let mut h = Harness::new();
    let before = h.object("scenario", "LOCAL");
    let cancelled = h.run("LOCAL");
    let _: CancellationState = ok(h.call(
        "cancel_task",
        json!({"task_id":cancelled.id,"action":"cancel"}),
    ));
    for work in h.host.drain_io_work() {
        h.host.finish_io(work.run());
    }
    assert!(matches!(
        h.task(&cancelled, true).task.state,
        ComputationStatus::Cancelled
    ));
    assert_eq!(h.object("scenario", "LOCAL"), before);
    let completed = h.run("LOCAL");
    h.finish_one();
    let result = h.completed_run(&completed);
    let reply: CancellationState = ok(h.call(
        "cancel_task",
        json!({"task_id":completed.id,"action":"cancel"}),
    ));
    assert!(reply.terminal_won);
    assert!(!reply.action_accepted);
    assert!(matches!(
        h.task(&completed, true).task.state,
        ComputationStatus::Completed
    ));
    assert_eq!(h.object("scenario", "LOCAL"), before);
    assert_eq!(
        serde_json::to_value(h.full(&result.id)).unwrap(),
        serde_json::to_value(result).unwrap()
    );
}

#[test]
fn a_new_cancellation_plan_keeps_other_reservations_and_old_run_then_survives_save_and_reopen() {
    let mut h = Harness::new();
    let old_task = h.run("LOCAL");
    h.finish_one();
    let old_run = h.completed_run(&old_task);
    let old_json = serde_json::to_value(&old_run).unwrap();
    let old_wip = run_objects(&mut h, &old_run.id, 30000);
    let shell_in_process = old_wip["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|object| object["data"]["id"] == "shell-001")
        .unwrap();
    assert_eq!(shell_in_process["data"]["lifecycle"], "in_process");
    assert_eq!(shell_in_process["data"]["holder"]["kind"], "job");
    let mut source = h.object("scenario", "LOCAL");
    let mut order = source["orders"][0].clone();
    order["id"] = json!("ORDER-OTHER");
    order["quantity"]["value"]["coefficient"] = json!(1);
    source["orders"].as_array_mut().unwrap().push(order);
    let mut task = source["production_tasks"][0].clone();
    task["id"] = json!("TASK-OTHER");
    task["planned_quantity"]["value"]["coefficient"] = json!(1);
    task["order_allocations"] = json!([{"order_id":"ORDER-OTHER","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}}]);
    source["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(task);
    source["initial_conditions"]["material_allocations"] = json!([
        {"id":"RESERVE-ORIGINAL","demand_ref":"TASK-LOCAL","item_refs":["shell-001"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"},
        {"id":"RESERVE-OTHER","demand_ref":"TASK-OTHER","item_refs":["shell-002"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"}
    ]);
    let _: EditCommit = ok(h.apply(1, json!([put("scenario", source)])));
    let source = h.object("scenario", "LOCAL");
    let typed: ScenarioInput = serde_json::from_value(source.clone()).unwrap();
    let derived = fork_without_unstarted_demands(
        &typed,
        ScenarioId::try_from("CANCELLED-PLAN".to_owned()).unwrap(),
        revision(2),
        &[id("ORDER-01")],
        &[],
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap(), source);
    let new = serde_json::to_value(&derived).unwrap();
    assert_eq!(new["orders"].as_array().unwrap().len(), 1);
    assert_eq!(new["orders"][0]["id"], "ORDER-OTHER");
    assert_eq!(new["production_tasks"].as_array().unwrap().len(), 1);
    assert_eq!(
        new["initial_conditions"]["material_allocations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        new["initial_conditions"]["material_allocations"][0]["id"],
        "RESERVE-OTHER"
    );
    assert_eq!(
        new["initial_conditions"]["items"],
        source["initial_conditions"]["items"]
    );
    let _: EditCommit = ok(h.apply(2, json!([put("scenario", new.clone())])));
    same_business(h.object("scenario", "LOCAL"), source);
    let task = h.run("CANCELLED-PLAN");
    h.finish_one();
    let new_run = h.completed_run(&task);
    assert_eq!(new_run.input_ref.project_revision.0.value(), 3);
    assert_eq!(new_run.input_ref.scenario_revision.0.value(), 1);
    assert_eq!(new_run.orders.len(), 1);
    assert_eq!(new_run.orders[0].order_id.as_str(), "ORDER-OTHER");
    assert_eq!(new_run.orders[0].accepted.value.coefficient.value(), 1);
    let new_json = serde_json::to_value(&new_run).unwrap();
    let live_shell = new_json["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "shell-001")
        .unwrap();
    assert_eq!(live_shell["lifecycle"], "available");
    assert_eq!(
        live_shell["holder"],
        json!({"kind":"location","id":"shells-A"})
    );
    assert_eq!(serde_json::to_value(h.full(&old_run.id)).unwrap(), old_json);
    assert_eq!(run_objects(&mut h, &old_run.id, 30000), old_wip);
    let directory = h.tree.directory("saved");
    let copied = h.tree.directory("relocated");
    let saved = h.save(3, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&saved, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&directory, &copied);
    let imported = h.import(&copied);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    same_business(h.object("scenario", "CANCELLED-PLAN"), new);
    assert_eq!(serde_json::to_value(h.full(&old_run.id)).unwrap(), old_json);
    assert_eq!(serde_json::to_value(h.full(&new_run.id)).unwrap(), new_json);
    assert_eq!(run_objects(&mut h, &old_run.id, 30000), old_wip);
}

#[test]
fn shared_unassigned_or_committed_demand_requires_explicit_replanning_without_mutating_the_host() {
    let mut h = Harness::new();
    let source = h.object("scenario", "LOCAL");
    let before = h.snapshot();
    let mut shared = source.clone();
    let mut other = shared["orders"][0].clone();
    other["id"] = json!("ORDER-OTHER");
    other["quantity"]["value"]["coefficient"] = json!(1);
    shared["orders"][0]["quantity"]["value"]["coefficient"] = json!(1);
    shared["orders"].as_array_mut().unwrap().push(other);
    shared["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(1);
    shared["production_tasks"][0]["order_allocations"].as_array_mut().unwrap()
        .push(json!({"order_id":"ORDER-OTHER","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}}));
    let mut unassigned = source.clone();
    unassigned["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(3);
    let mut committed = source;
    committed["initial_conditions"]["material_allocations"] = json!([{"id":"NO-REFUND","demand_ref":"TASK-LOCAL",
        "item_refs":["shell-001"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"committed"}]);
    for (candidate, expected) in [
        (shared, "business_plans.shared_task_requires_replan"),
        (unassigned, "business_plans.unassigned_task_requires_replan"),
        (committed, "business_plans.committed_not_cancellable"),
    ] {
        let typed: ScenarioInput = serde_json::from_value(candidate.clone()).unwrap();
        let issues = fork_without_unstarted_demands(
            &typed,
            ScenarioId::try_from("REQUIRES-PLAN".to_owned()).unwrap(),
            revision(1),
            &[id("ORDER-01")],
            &[],
        )
        .expect_err("ambiguous/live allocation was automatically discarded");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
        assert_eq!(serde_json::to_value(typed).unwrap(), candidate);
        assert_eq!(h.snapshot(), before);
    }
}

#[test]
fn merging_orders_into_one_task_requires_the_exact_same_product_revision_even_when_routes_match() {
    let mut h = Harness::new();
    let mut same = h.object("scenario", "LOCAL");
    same["orders"][0]["quantity"]["value"]["coefficient"] = json!(1);
    let mut other = same["orders"][0].clone();
    other["id"] = json!("ORDER-OTHER");
    same["orders"].as_array_mut().unwrap().push(other);
    same["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(1);
    same["production_tasks"][0]["order_allocations"].as_array_mut().unwrap().push(json!({
        "order_id":"ORDER-OTHER","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}
    }));
    let _: EditCommit = ok(h.apply(1, json!([put("scenario", same.clone())])));
    assert!(
        !h.context()
            .issues
            .iter()
            .any(|issue| issue.code == "business_inputs.task_product_mismatch")
    );
    let mut product = h.object("product", "PACKAGE");
    product["id"] = json!("OTHER");
    let mut mixed = h.object("scenario", "LOCAL");
    mixed["orders"][1]["product_ref"] = json!({"id":"OTHER","revision":"r1"});
    let _: EditCommit = ok(h.apply(
        2,
        json!([put("product", product), put("scenario", mixed.clone())]),
    ));
    let issues = h.context().issues;
    let issue = issues
        .iter()
        .find(|issue| issue.code == "business_inputs.task_product_mismatch")
        .unwrap();
    assert_eq!(issue.object_ref.as_ref().unwrap().id.as_str(), "LOCAL");
    assert_eq!(
        issue.field_path.as_deref(),
        Some("/production_tasks/0/order_allocations/1/order_id")
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
    same_business(h.object("scenario", "LOCAL"), mixed);
    let before = h.snapshot();
    let input = h.object("scenario", "LOCAL");
    assert!(matches!(
        error(h.call(
            "run_scenario",
            json!({"scenario_id":"LOCAL","input_revision":input["revision"],
        "observation_window":input["window"],"intent":{"kind":"project_scenario"}})
        ))
        .code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
    assert!(h.host.drain_io_work().is_empty());
    let mut version_mix = h.object("scenario", "LOCAL");
    version_mix["orders"][1]["product_ref"] = json!({"id":"PACKAGE","revision":"r2"});
    let _: EditCommit = ok(h.apply(3, json!([put("scenario", version_mix)])));
    assert!(
        h.context()
            .issues
            .iter()
            .any(|issue| issue.code == "business_inputs.task_product_mismatch")
    );
    let mut split = h.object("scenario", "LOCAL");
    split["orders"][1]["product_ref"] = json!({"id":"OTHER","revision":"r1"});
    split["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(1);
    split["production_tasks"][0]["order_allocations"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let mut task = split["production_tasks"][0].clone();
    task["id"] = json!("TASK-OTHER");
    task["order_allocations"][0]["order_id"] = json!("ORDER-OTHER");
    split["production_tasks"].as_array_mut().unwrap().push(task);
    let _: EditCommit = ok(h.apply(4, json!([put("scenario", split)])));
    assert!(
        !h.context()
            .issues
            .iter()
            .any(|issue| issue.code == "business_inputs.task_product_mismatch")
    );
}

fn run_objects(h: &mut Harness, run_id: &RunId, at_ms: u64) -> Value {
    let view: RunView = ok(h.call("get_run", json!({"run_id":run_id,
        "query":{"kind":"objects_at","data":{"at_ms":at_ms,"objects":[],"page":{"after_index":null,"limit":100}}}})));
    match view {
        RunView::Objects(page) => {
            assert!(page.next_index.is_none());
            serde_json::to_value(page).unwrap()
        }
        other => panic!("expected actual historical objects: {other:?}"),
    }
}

#[test]
fn actual_host_initial_batch_reservation_and_future_arrival_remain_distinct_after_relocation() {
    let mut value = fixture();
    value["scenario"]["orders"][0]["quantity"]["value"]["coefficient"] = json!(1);
    value["scenario"]["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(1);
    value["scenario"]["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] = json!(1);
    value["project"]["item_types"].as_array_mut().unwrap().iter_mut()
        .find(|item| item["id"] == "shell").unwrap()["tracking"] = json!("batch");
    value["scenario"]["initial_conditions"]["items"][0]["id"] = json!("batch10");
    value["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"] = json!(10);
    value["scenario"]["initial_conditions"]["items"].as_array_mut().unwrap().remove(1);
    value["project"]["locations"].as_array_mut().unwrap().iter_mut()
        .find(|location| location["id"] == "shells-A").unwrap()["capacity"]["value"]["coefficient"] = json!(10);
    value["definitions"].as_array_mut().unwrap().iter_mut()
        .find(|definition| definition["id"] == "fixture.assemble").unwrap()
        ["payload"]["data"]["inputs"][0]["quantity"]["value"]["coefficient"] = json!(4);
    value["project"]["boms"][0]["rows"][0]["quantity"]["value"]["coefficient"] = json!(4);
    value["scenario"]["initial_conditions"]["material_allocations"] = json!([
        {"id":"hard4","demand_ref":"TASK-LOCAL","item_refs":["batch10"],
         "quantity":{"value":{"coefficient":4,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"}
    ]);
    value["project"]["calendars"][0]["intervals"][0]["start_ms"] = json!(5000);
    let mut future_pcb = value["scenario"]["initial_conditions"]["items"][1].clone();
    assert_eq!(future_pcb["item_type_ref"]["id"], "pcb");
    future_pcb["id"] = json!("pcb-future");
    value["scenario"]["arrivals"] = json!([
        {"id":"future-pcb","at_ms":90000,"destination":"pcbs-A","items":[future_pcb],"release_policy":"unchecked_allowed"}
    ]);
    let mut h = Harness::from_value(value);
    let source = h.object("scenario", "LOCAL");
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    let frozen = serde_json::to_value(&run).unwrap();
    assert_eq!(run.acceptance_events.len(), 1);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 80000);
    let initial = run_objects(&mut h, &run.id, 0);
    let initial_shells: Vec<_> = initial["objects"].as_array().unwrap().iter()
        .map(|object| &object["data"])
        .filter(|item| item["item_type_ref"]["id"] == "shell" && item["lifecycle"] != "transformed")
        .collect();
    assert_eq!(initial_shells.len(), 2);
    assert_eq!(initial_shells.iter().map(|item| item["quantity"]["value"]["coefficient"].as_u64().unwrap()).sum::<u64>(), 10);
    assert!(initial_shells.iter().all(|item| item["lifecycle"] == "available" && item["holder"]["kind"] == "location"));
    let reserved = initial_shells.iter().find(|item| item["allocation_refs"].as_array().unwrap().contains(&json!("hard4"))).unwrap();
    assert_eq!(reserved["quantity"]["value"]["coefficient"], 4);
    let started = run_objects(&mut h, &run.id, 5000);
    let mut location_quantity = 0;
    let mut job_quantity = 0;
    for object in started["objects"].as_array().unwrap() {
        let item = &object["data"];
        if item["item_type_ref"]["id"] != "shell" { continue; }
        let quantity = item["quantity"]["value"]["coefficient"].as_u64().unwrap();
        if item["lifecycle"] == "available" && item["holder"]["kind"] == "location" { location_quantity += quantity; }
        if item["lifecycle"] == "in_process" && item["holder"]["kind"] == "job" {
            assert!(!item["allocation_refs"].as_array().unwrap().is_empty());
            job_quantity += quantity;
        }
    }
    assert_eq!((location_quantity, job_quantity), (6, 4));
    let before_arrival = run_objects(&mut h, &run.id, 89999);
    for page in [&initial, &before_arrival] {
        assert!(!page["objects"].as_array().unwrap().iter().any(|object| object["kind"] == "item" && object["data"]["id"] == "pcb-future"));
    }
    let arrived = run_objects(&mut h, &run.id, 90000);
    let pcb = arrived["objects"].as_array().unwrap().iter()
        .find(|object| object["kind"] == "item" && object["data"]["id"] == "pcb-future").unwrap();
    assert_eq!(pcb["data"]["lifecycle"], "available");
    assert_eq!(pcb["data"]["holder"], json!({"kind":"location","id":"pcbs-A"}));
    assert_eq!(h.object("scenario", "LOCAL"), source);
    let directory = h.tree.directory("batch-original");
    let relocated = h.tree.directory("batch-relocated");
    let saved = h.save(1, &directory);
    h.finish_one();
    assert!(matches!(h.task(&saved, true).task.state, ComputationStatus::Completed));
    copy(&directory, &relocated);
    let imported = h.import(&relocated);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(h.object("scenario", "LOCAL"), source);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), frozen);
    assert_eq!(run_objects(&mut h, &run.id, 0), initial);
    assert_eq!(run_objects(&mut h, &run.id, 5000), started);
    assert_eq!(run_objects(&mut h, &run.id, 89999), before_arrival);
    assert_eq!(run_objects(&mut h, &run.id, 90000), arrived);
}

#[test]
fn actual_host_composition_sources_are_not_free_stock_and_survive_reopen() {
    let mut value = fixture();
    value["scenario"]["initial_conditions"]["items"][0]["components"] = json!(["pcb-001"]);
    let mut h = Harness::from_value(value);
    let source = h.object("scenario", "LOCAL");
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    let frozen = serde_json::to_value(&run).unwrap();
    assert!(run.whole_window_computed);
    assert!(run.acceptance_events.is_empty());
    assert!(run.jobs.iter().all(|job| job.started_at_ms.is_none()));
    let initial = run_objects(&mut h, &run.id, 0);
    let pcb = initial["objects"].as_array().unwrap().iter()
        .find(|object| object["kind"] == "item" && object["data"]["id"] == "pcb-001").unwrap();
    assert_eq!(pcb["data"]["lifecycle"], "transformed");
    assert_eq!(pcb["data"]["holder"], json!({"kind":"location","id":"pcbs-A"}));
    assert_eq!(h.object("scenario", "LOCAL"), source);
    let directory = h.tree.directory("composition-original");
    let relocated = h.tree.directory("composition-relocated");
    let saved = h.save(1, &directory);
    h.finish_one();
    assert!(matches!(h.task(&saved, true).task.state, ComputationStatus::Completed));
    copy(&directory, &relocated);
    let imported = h.import(&relocated);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(h.object("scenario", "LOCAL"), source);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), frozen);
    assert_eq!(run_objects(&mut h, &run.id, 0), initial);
}

#[test]
fn an_unproved_unloading_acceptance_stage_is_rejected_without_queuing_or_lowering_the_request() {
    let mut value = fixture();
    value["scenario"]["orders"][0]["acceptance_stage"] = json!("unloaded_and_released");
    let mut h = Harness::from_value(value);
    let before = h.snapshot();
    let failure = error(h.call(
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":1,
        "observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}}),
    ));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
    assert_eq!(
        h.object("scenario", "LOCAL")["orders"][0]["acceptance_stage"],
        "unloaded_and_released"
    );
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn actual_run_object_history_survives_relocation_and_does_not_replay_the_final_stock_into_the_past()
{
    let mut h = Harness::new();
    let task = h.run("LOCAL");
    h.finish_one();
    let result = h.completed_run(&task);
    let before = run_objects(&mut h, &result.id, 74999);
    assert!(
        !before["objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|object| object["data"]["item_type_ref"]["id"] == "package"
                && object["data"]["lifecycle"] == "available")
    );
    let accepted = run_objects(&mut h, &result.id, 75000);
    assert_eq!(
        accepted["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|object| object["data"]["item_type_ref"]["id"] == "package"
                && object["data"]["lifecycle"] == "available")
            .count(),
        1
    );
    let directory = h.tree.directory("run-history");
    let relocated = h.tree.directory("run-history-relocated");
    let task = h.save(1, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&directory, &relocated);
    let imported = h.import(&relocated);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(run_objects(&mut h, &result.id, 74999), before);
    assert_eq!(run_objects(&mut h, &result.id, 75000), accepted);
    assert_eq!(
        serde_json::to_value(h.full(&result.id)).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    let rejected = error(h.call("get_run",json!({"run_id":result.id,
        "query":{"kind":"objects_at","data":{"at_ms":240001,"objects":[],"page":{"after_index":null,"limit":100}}}})));
    assert!(matches!(rejected.code, ErrorCode::InvalidInput));
}

#[test]
fn an_incomplete_other_scenario_keeps_its_diagnosis_without_blocking_the_complete_current_run() {
    let mut h = Harness::new();
    let original = h.object("scenario", "LOCAL");
    let mut other = original.clone();
    other["id"] = json!("OTHER");
    other["title"] = json!("independent incomplete plan");
    other["orders"][0]["quantity"] = Value::Null;
    let _: EditCommit = ok(h.apply(1, json!([put("scenario", other)])));
    let incomplete = h.object("scenario", "OTHER");
    let issue_is_other = |issue: &Issue| {
        issue.object_ref.as_ref().is_some_and(|owner| {
            matches!(owner.kind, ObjectKind::Scenario) && owner.id.as_str() == "OTHER"
        }) && issue
            .blocks
            .iter()
            .any(|purpose| matches!(purpose, CapabilityPurpose::Execution))
    };
    assert!(h.context().issues.iter().any(issue_is_other));
    let selected_scenario_revision = h.object("scenario", "LOCAL")["revision"].as_u64().unwrap();
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.input_ref.project_revision.0.value(), 2);
    assert_eq!(
        run.input_ref.scenario_revision.0.value(),
        selected_scenario_revision
    );
    assert_eq!(run.acceptance_events.len(), 1);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 75000);
    same_business(h.object("scenario", "LOCAL"), original);
    assert_eq!(h.object("scenario", "OTHER"), incomplete);
    assert!(
        h.context().issues.iter().any(issue_is_other),
        "global diagnosis is retained"
    );
    let before = h.snapshot();
    let failure = error(h.call(
        "run_scenario",
        json!({"scenario_id":"OTHER","input_revision":2,
        "observation_window":incomplete["window"],"intent":{"kind":"project_scenario"}}),
    ));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn original_parallel_a_q_u_are_consumed_by_the_actual_host_with_frozen_times_quality_and_historical_objects()
 {
    for (case, packages, rejected, pending) in [("A", 2, 0, 0), ("Q", 1, 1, 0), ("U", 0, 0, 1)] {
        let value = p1_support::parallel(case);
        let expected = p1_support::original_expectation(case);
        let mut h = Harness::from_value(value);
        let task = h.run(case);
        assert!(matches!(task.state, ComputationStatus::Queued));
        h.finish_one();
        let run = h.completed_run(&task);
        let result = serde_json::to_value(&run).unwrap();
        assert_eq!(result["evidence_origin"], "actual_software");
        assert_eq!(result["whole_window_computed"], true);
        assert_eq!(p1_support::physical(&result, "package").len(), packages);
        let assemblies = p1_support::physical(&result, "assembly");
        assert_eq!(assemblies.len(), 2);
        assert_eq!(
            assemblies
                .iter()
                .filter(|i| i["quality"] == "rejected")
                .count(),
            rejected
        );
        assert_eq!(
            assemblies
                .iter()
                .filter(|i| i["quality"] == "pending")
                .count(),
            pending
        );
        let mut actual_times: Vec<_> = p1_support::jobs(&result, "w5.op.pack")
            .iter()
            .filter_map(|j| j["finished_at_ms"].as_u64())
            .collect();
        actual_times.sort();
        let expected_times: Vec<_> = expected["package_finish_seconds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() * 1000)
            .collect();
        assert_eq!(actual_times, expected_times);
        let historical = run_objects(&mut h, &run.id, 99999);
        assert!(
            !historical["objects"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["kind"] == "item" && o["data"]["item_type_ref"]["id"] == "package")
        );
        assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), result);
    }
}

#[test]
fn actual_host_phase_run_keeps_45_seconds_10_worker_seconds_and_frozen_history_after_save_move_open()
 {
    let value = p1_support::phased();
    let mut h = Harness::from_value(value);
    let task = h.run("PHASE");
    h.finish_one();
    let run = h.completed_run(&task);
    let frozen = serde_json::to_value(&run).unwrap();
    assert_eq!(
        p1_support::jobs(&frozen, "w5.op.phased")[0]["finished_at_ms"],
        45000
    );
    assert_eq!(p1_support::held_ms(&frozen, "W-C"), 10000);
    assert_eq!(p1_support::intervals(&frozen, "CAP-CNC"), vec![(0, 45000)]);
    let original = h.tree.directory("phase-original");
    let moved = h.tree.directory("phase-moved");
    let base = h.context().project.unwrap().revision.0.value();
    let save = h.save(base, &original);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&original, &moved);
    let imported = h.import(&moved);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), frozen);
    let rerun = h.run("PHASE");
    h.finish_one();
    let actual = serde_json::to_value(h.completed_run(&rerun)).unwrap();
    assert_eq!(
        p1_support::jobs(&actual, "w5.op.phased")[0]["finished_at_ms"],
        45000
    );
    assert_eq!(p1_support::held_ms(&actual, "W-C"), 10000);
}

#[test]
fn saved_no_order_goal_has_three_real_null_order_acceptances_and_goal_edits_do_not_cap_production_or_rewrite_old_run()
 {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/production/p1-legacy-goal.json")).unwrap();
    let mut h = Harness::from_value(value);
    let first = h.run("LEGACY");
    h.finish_one();
    let first = h.completed_run(&first);
    let frozen = serde_json::to_value(&first).unwrap();
    assert_eq!(frozen["goal_met"], true);
    assert_eq!(frozen["orders"], json!([]));
    assert_eq!(first.acceptance_events.len(), 3);
    for event in &first.acceptance_events {
        assert!(event.order_id.is_none());
        assert!(!event.item_refs.is_empty());
        for item in &event.item_refs {
            assert_eq!(
                p1_support::named(&frozen, item.as_str())["quality"],
                "released"
            );
        }
    }
    let project = h.object("project", "P1");
    assert!(project["generation_context"].is_null());
    let mut changed = h.object("scenario", "LEGACY");
    changed["acceptance_goal"]["accepted_quantity_per_day"]["value"]["coefficient"] = json!(4);
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("scenario", changed.clone())])));
    let second = h.run("LEGACY");
    h.finish_one();
    let second = h.completed_run(&second);
    let second_value = serde_json::to_value(&second).unwrap();
    assert_eq!(second_value["goal_met"], false);
    assert_eq!(
        second_value["acceptance_events"],
        frozen["acceptance_events"]
    );
    assert_eq!(second_value["final_items"], frozen["final_items"]);
    let original = h.tree.directory("legacy-original");
    let moved = h.tree.directory("legacy-moved");
    let base = h.context().project.unwrap().revision.0.value();
    let save = h.save(base, &original);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&original, &moved);
    let imported = h.import(&moved);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(
        h.object("scenario", "LEGACY")["acceptance_goal"],
        changed["acceptance_goal"]
    );
    assert!(h.object("project", "P1")["generation_context"].is_null());
    assert_eq!(serde_json::to_value(h.full(&first.id)).unwrap(), frozen);
    assert_eq!(
        serde_json::to_value(h.full(&second.id)).unwrap(),
        second_value
    );
    let third = h.run("LEGACY");
    h.finish_one();
    let third = h.completed_run(&third);
    assert_eq!(third.acceptance_events.len(), 3);
    assert_eq!(third.goal_met, Some(false));
}

fn actual_template_host(
    which: sf_module_kit::BuiltinTemplate,
) -> (Harness, sf_module_kit::TemplateCandidate) {
    let candidate = sf_module_kit::builtin_template(which).unwrap();
    let mut input = fixture();
    let mut definitions = candidate.definitions.clone();
    definitions.push(candidate.package.definition.clone());
    input["definitions"]
        .as_array_mut()
        .unwrap()
        .extend(definitions.iter().map(|d| serde_json::to_value(d).unwrap()));
    input["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .extend(
            definitions
                .iter()
                .map(|d| json!({"id":d.id,"version":d.version})),
        );
    let tree = Sandbox::new();
    let directory = tree.directory("selected-template-source");
    seed(&directory, &input);
    let mut manifest = read(&directory.join("project.json"));
    let main_id = candidate.package.definition.id.as_str();
    let entry = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["role"] == "module_definition" && e["object_ref"]["id"] == main_id)
        .unwrap();
    write(
        &directory.join(entry["file_ref"]["path"].as_str().unwrap()),
        &json!({"schema_version":"1.0","data":candidate.package}),
    );
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/templates")
        .join(match which {
            sf_module_kit::BuiltinTemplate::MachiningDelivery => "machining-delivery",
            sf_module_kit::BuiltinTemplate::PurchasedAssembly => "purchased-assembly",
            sf_module_kit::BuiltinTemplate::ParallelElectromechanical => {
                "parallel-electromechanical"
            }
            sf_module_kit::BuiltinTemplate::CartonFactory => "carton-factory",
        })
        .join("candidate");
    for file in &candidate.package.files {
        assert_ne!(
            file.path.as_str(),
            "project.json",
            "template payload cannot overwrite active project root manifest"
        );
        let from = source.join(file.path.as_str());
        let to = directory.join(file.path.as_str());
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(from, to).unwrap();
        manifest["data"]["documents"]
            .as_array_mut()
            .unwrap()
            .push(document(
                "supporting_file",
                serde_json::to_value(&file.kind).unwrap().as_str().unwrap(),
                "module",
                main_id,
                file.path.as_str(),
                Value::Null,
            ));
    }
    write(&directory.join("project.json"), &manifest);
    (Harness::from_seed(tree, &directory), candidate)
}
fn template_choices(
    candidate: &sf_module_kit::TemplateCandidate,
    namespace: &str,
) -> sf_module_kit::TemplateChoices {
    sf_module_kit::TemplateChoices {
        namespace: namespace.into(),
        resources: candidate
            .example_project
            .resources
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    sf_module_kit::TemplateResourceSelection::CreateDeclaredExample,
                )
            })
            .collect(),
        reuse_items: std::collections::BTreeMap::new(),
        include_example_product: true,
    }
}
#[test]
fn three_actual_templates_require_separate_seed_then_run_original_times_and_keep_old_runs_after_edit_save_move_open()
 {
    for (which, namespace, expected) in [
        (
            sf_module_kit::BuiltinTemplate::MachiningDelivery,
            "w5-machine",
            vec![90000, 150000],
        ),
        (
            sf_module_kit::BuiltinTemplate::PurchasedAssembly,
            "w5-purchased",
            vec![70000, 110000],
        ),
        (
            sf_module_kit::BuiltinTemplate::ParallelElectromechanical,
            "w5-parallel",
            vec![160000, 200000],
        ),
    ] {
        let (mut h, candidate) = actual_template_host(which);
        let original_scenario = h.object("scenario", "LOCAL");
        let old = h.run("LOCAL");
        h.finish_one();
        let old = h.completed_run(&old);
        let old_value = serde_json::to_value(&old).unwrap();
        let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
        let plan = sf_module_kit::prepare_template(
            &candidate,
            &target,
            &template_choices(&candidate, namespace),
        )
        .unwrap();
        let base = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(
            base,
            json!([{"kind":"instantiate_template","data":plan.command_data(None)}]),
        ));
        same_business(h.object("scenario", "LOCAL"), original_scenario.clone());
        assert_eq!(h.object("project", "P1")["scenario_refs"], json!(["LOCAL"]));
        let mut scenario = sf_module_kit::prepare_template_seed(
            &candidate,
            &plan,
            &sf_module_kit::TemplateSeedChoices {
                scenario_id: ScenarioId::try_from(format!("{namespace}-explicit")).unwrap(),
                supply: true,
                demand: true,
                initial_quality: candidate
                    .subset
                    .adoption
                    .as_ref()
                    .unwrap()
                    .deferred_seed
                    .as_ref()
                    .unwrap()
                    .initial_stock
                    .iter()
                    .map(|stock| (stock.location_id.clone(), QualityState::Released))
                    .collect(),
                adopt_declared_inspection_results: true,
            },
        )
        .unwrap();
        let scenario_id = scenario.id.as_str().to_owned();
        let base = h.context().project.unwrap().revision.0.value();
        scenario.project_revision = revision(base);
        let _: EditCommit = ok(h.apply(
            base,
            json!([put("scenario", serde_json::to_value(&scenario).unwrap())]),
        ));
        let task = h.run(&scenario_id);
        h.finish_one();
        let result = h.completed_run(&task);
        let frozen = serde_json::to_value(&result).unwrap();
        let times: Vec<_> = result
            .acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect();
        assert_eq!(times, expected);
        assert_eq!(
            p1_support::physical(&frozen, "package").len(),
            0,
            "template instance item IDs must be fresh"
        );
        assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), old_value);
        // A declared calendar edit yields a new Run; original behavior and source definitions remain unchanged.
        let mut edits = vec![];
        for calendar in &plan.proposed_project.calendars {
            let mut changed = h.object("calendar", calendar.id.as_str());
            changed["intervals"][0]["start_ms"] = json!(1000);
            edits.push(put("calendar", changed));
        }
        let base = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(base, json!(edits)));
        let next = h.run(&scenario_id);
        h.finish_one();
        let next = h.completed_run(&next);
        assert_eq!(
            next.acceptance_events
                .iter()
                .map(|a| a.at_ms.0.value())
                .collect::<Vec<_>>(),
            expected.iter().map(|t| t + 1000).collect::<Vec<_>>()
        );
        let original = h.tree.directory("template-saved");
        let moved = h.tree.directory("template-reopened");
        let base = h.context().project.unwrap().revision.0.value();
        let save = h.save(base, &original);
        h.finish_one();
        assert!(matches!(
            h.task(&save, true).task.state,
            ComputationStatus::Completed
        ));
        copy(&original, &moved);
        let imported = h.import(&moved);
        h.finish_one();
        let draft = h.imported(&imported);
        let _: EditCommit = ok(h.adopt(&imported, &draft, false));
        assert_eq!(serde_json::to_value(h.full(&result.id)).unwrap(), frozen);
        assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), old_value);
        same_business(h.object("scenario", "LOCAL"), original_scenario.clone());
        let again = h.run(&scenario_id);
        h.finish_one();
        assert_eq!(
            h.completed_run(&again)
                .acceptance_events
                .iter()
                .map(|a| a.at_ms.0.value())
                .collect::<Vec<_>>(),
            expected.iter().map(|t| t + 1000).collect::<Vec<_>>()
        );
    }
}
#[test]
fn a_template_with_missing_behavior_remains_editable_but_cannot_silently_execute_a_zero_time_replacement()
 {
    let (mut h, candidate) =
        actual_template_host(sf_module_kit::BuiltinTemplate::PurchasedAssembly);
    let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let plan = sf_module_kit::prepare_template(
        &candidate,
        &target,
        &template_choices(&candidate, "w5-missing-behavior"),
    )
    .unwrap();
    let scenario = sf_module_kit::prepare_template_seed(
        &candidate,
        &plan,
        &sf_module_kit::TemplateSeedChoices {
            scenario_id: ScenarioId::try_from("w5-missing-operation-scenario".to_owned()).unwrap(),
            supply: true,
            demand: true,
            initial_quality: candidate
                .subset
                .adoption
                .as_ref()
                .unwrap()
                .deferred_seed
                .as_ref()
                .unwrap()
                .initial_stock
                .iter()
                .map(|stock| (stock.location_id.clone(), QualityState::Released))
                .collect(),
            adopt_declared_inspection_results: true,
        },
    )
    .unwrap();
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"instantiate_template","data":plan.command_data(Some(scenario.clone()))}]),
    ));
    let device = plan
        .proposed_project
        .devices
        .iter()
        .find(|d| d.id.as_str().ends_with(":Assembly01"))
        .unwrap();
    let mut missing = h.object("device", device.id.as_str());
    missing["operation_binding"] = json!({"id":"w5.absent-operation","version":"0.1.0"});
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("device", missing)])));
    let before = h.object("scenario", scenario.id.as_str());
    let base = h.context().project.unwrap().revision.0.value();
    let failure=error(h.call("run_scenario",json!({"scenario_id":scenario.id,"input_revision":base,"observation_window":scenario.window,"intent":{"kind":"project_scenario"}})));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.object("scenario", scenario.id.as_str()), before);
}
#[test]
fn original_two_hall_and_full_ix01_transport_runs_use_the_actual_host_and_survive_relocation() {
    for (name, source, last, count) in [
        (
            "HALLS-B",
            include_str!("fixtures/production/p202-halls-B.json"),
            205000,
            2,
        ),
        (
            "IX01-delayed-pcb",
            include_str!("fixtures/production/p202-ix01-delayed-pcb.json"),
            570000,
            6,
        ),
    ] {
        let mut h = Harness::from_value(serde_json::from_str(source).unwrap());
        let task = h.run(name);
        h.finish_one();
        let run = h.completed_run(&task);
        let frozen = serde_json::to_value(&run).unwrap();
        assert_eq!(p1_support::physical(&frozen, "package").len(), count);
        assert_eq!(run.acceptance_events.last().unwrap().at_ms.0.value(), last);
        assert!(!run.transport.is_empty());
        assert!(run.transport.iter().all(|t| t.returned_ms.is_some()));
        let directory = h.tree.directory("transport-save");
        let moved = h.tree.directory("transport-open");
        let base = h.context().project.unwrap().revision.0.value();
        let saved = h.save(base, &directory);
        h.finish_one();
        assert!(matches!(
            h.task(&saved, true).task.state,
            ComputationStatus::Completed
        ));
        copy(&directory, &moved);
        let imported = h.import(&moved);
        h.finish_one();
        let draft = h.imported(&imported);
        let _: EditCommit = ok(h.adopt(&imported, &draft, false));
        assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), frozen);
        let again = h.run(name);
        h.finish_one();
        let again = h.completed_run(&again);
        assert_eq!(
            again.acceptance_events.last().unwrap().at_ms.0.value(),
            last
        );
    }
}

#[test]
fn actual_template_adoption_persists_remapped_preparation_resource_choices_and_parameter_source_records()
 {
    let mut mismatches = vec![];
    for (i, which) in [
        sf_module_kit::BuiltinTemplate::MachiningDelivery,
        sf_module_kit::BuiltinTemplate::PurchasedAssembly,
        sf_module_kit::BuiltinTemplate::ParallelElectromechanical,
        sf_module_kit::BuiltinTemplate::CartonFactory,
    ]
    .into_iter()
    .enumerate()
    {
        let (mut h, candidate) = actual_template_host(which);
        let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
        let plan = sf_module_kit::prepare_template(
            &candidate,
            &target,
            &template_choices(&candidate, &format!("w5-metadata-{i}")),
        )
        .unwrap();
        let expected = serde_json::to_value(&plan.proposed_project).unwrap();
        assert!(
            !expected["sources"].as_array().unwrap().is_empty(),
            "candidate must disclose example sources"
        );
        assert!(
            !expected["parameter_sources"].as_array().unwrap().is_empty(),
            "instance parameters need their actual basis"
        );
        let base = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(
            base,
            json!([{"kind":"instantiate_template","data":plan.command_data(None)}]),
        ));
        let actual = h.object("project", "P1");
        for field in ["template_adoptions", "sources", "parameter_sources"] {
            if actual[field] != expected[field] {
                mismatches.push(format!("candidate {i}: {field} differs from actual source plus explicit fresh-ID/resource mapping"));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn original_multi_order_priority_variants_use_actual_host_and_preserve_both_205_second_runs_after_save_move_open()
 {
    let input: Value =
        serde_json::from_str(include_str!("fixtures/production/p104-A-first.json")).unwrap();
    let mut h = Harness::from_value(input);
    let first = h.run("ORDERS-A");
    h.finish_one();
    let first = h.completed_run(&first);
    let first_value = serde_json::to_value(&first).unwrap();
    assert_eq!(p1_support::held_ms(&first_value, "W-A"), 135000);
    let order_b = first_value["orders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["order_id"] == "B")
        .unwrap();
    assert_eq!(order_b["completion_ms"], 205000);
    assert_eq!(order_b["tardiness_ms"], 5000);
    let mut scenario = h.object("scenario", "ORDERS-A");
    for task in scenario["production_tasks"].as_array_mut().unwrap() {
        task["priority"] = json!(if task["id"] == "TASK-B" { 1 } else { 0 });
    }
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("scenario", scenario)])));
    let second = h.run("ORDERS-A");
    h.finish_one();
    let second = h.completed_run(&second);
    let second_value = serde_json::to_value(&second).unwrap();
    assert_eq!(p1_support::held_ms(&second_value, "W-A"), 160000);
    let order_a = second_value["orders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["order_id"] == "A")
        .unwrap();
    assert_eq!(order_a["completion_ms"], 205000);
    assert_eq!(order_a["tardiness_ms"], 45000);
    assert_ne!(first.id, second.id);
    assert_eq!(
        serde_json::to_value(h.full(&first.id)).unwrap(),
        first_value
    );
    let directory = h.tree.directory("orders-save");
    let moved = h.tree.directory("orders-open");
    let base = h.context().project.unwrap().revision.0.value();
    let saved = h.save(base, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&saved, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&directory, &moved);
    let imported = h.import(&moved);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(
        serde_json::to_value(h.full(&first.id)).unwrap(),
        first_value
    );
    assert_eq!(
        serde_json::to_value(h.full(&second.id)).unwrap(),
        second_value
    );
    let current = h.run("ORDERS-A");
    h.finish_one();
    let current = h.completed_run(&current);
    assert_eq!(
        p1_support::held_ms(&serde_json::to_value(current).unwrap(), "W-A"),
        160000
    );
}

#[test]
fn carton_manufacturing_template_consumes_board_to_make_cartons_instead_of_loading_products_into_purchased_cartons()
 {
    let (mut h, candidate) = actual_template_host(sf_module_kit::BuiltinTemplate::CartonFactory);
    let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let plan = sf_module_kit::prepare_template(
        &candidate,
        &target,
        &template_choices(&candidate, "w5-carton-manufacture"),
    )
    .unwrap();
    let scenario = sf_module_kit::prepare_template_seed(
        &candidate,
        &plan,
        &sf_module_kit::TemplateSeedChoices {
            scenario_id: ScenarioId::try_from("w5-carton-explicit-demo".to_owned()).unwrap(),
            supply: true,
            demand: true,
            initial_quality: candidate
                .subset
                .adoption
                .as_ref()
                .unwrap()
                .deferred_seed
                .as_ref()
                .unwrap()
                .initial_stock
                .iter()
                .map(|stock| (stock.location_id.clone(), QualityState::Released))
                .collect(),
            adopt_declared_inspection_results: true,
        },
    )
    .unwrap();
    assert!(
        scenario.initial_conditions.items.iter().all(|i| !i
            .item_type_ref
            .id
            .as_str()
            .ends_with(":carton"))
    );
    let expected: u64 = scenario
        .orders
        .iter()
        .map(|o| o.quantity.as_ref().unwrap().value.coefficient.value())
        .sum();
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"instantiate_template","data":plan.command_data(Some(scenario.clone()))}]),
    ));
    let task = h.run(scenario.id.as_str());
    h.finish_one();
    let run = h.completed_run(&task);
    let result = serde_json::to_value(&run).unwrap();
    let output: Vec<_> = result["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| {
            i["item_type_ref"]["id"]
                .as_str()
                .is_some_and(|id| id.ends_with(":carton"))
                && i["quality"] == "released"
                && i["lifecycle"] == "available"
        })
        .collect();
    let actual: u64 = output
        .iter()
        .map(|i| i["quantity"]["value"]["coefficient"].as_u64().unwrap())
        .sum();
    assert_eq!(actual, expected);
    assert!(result["final_items"].as_array().unwrap().iter().all(|i| {
        !i["item_type_ref"]["id"]
            .as_str()
            .is_some_and(|id| id.ends_with(":package"))
    }));
    assert_eq!(
        run.acceptance_events
            .iter()
            .map(|a| a.quantity.value.coefficient.value())
            .sum::<u64>(),
        expected
    );
    assert!(
        h.object("project", "P1")["artifact_refs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn explicit_template_request(
    plan: &sf_module_kit::PreparedTemplate,
    choices: &sf_module_kit::TemplateChoices,
    site: Option<SiteEnvelope>,
) -> Value {
    let preparation = TemplatePreparationChoices {
        source_project: RelativeFilePath::try_from("example-project.json".to_owned()).unwrap(),
        namespace: choices.namespace.clone(),
        resources: choices
            .resources
            .iter()
            .map(|(source, selection)| {
                (
                    source.clone(),
                    match selection {
                        sf_module_kit::TemplateResourceSelection::CreateDeclaredExample => None,
                        sf_module_kit::TemplateResourceSelection::Reuse(actual) => {
                            Some(actual.clone())
                        }
                    },
                )
            })
            .collect(),
        reuse_items: choices.reuse_items.clone(),
        include_example_product: choices.include_example_product,
        site_envelope: site.clone(),
    };
    let mut request = plan.command_data(None);
    request["preparation"] = serde_json::to_value(&preparation).unwrap();
    let _: TemplateInstantiation = serde_json::from_value(request.clone()).unwrap();
    let sdk = plan.command_data_with_preparation(
        &preparation.source_project,
        choices,
        site.as_ref(),
        None,
    );
    assert_eq!(
        sdk, request,
        "actual SDK must carry the explicit current public choices"
    );
    sdk
}

#[test]
fn explicit_template_namespace_resource_item_reuse_and_site_sources_survive_edit_save_move_open_with_old_run_unchanged()
 {
    for height in [Value::Null, json!(5)] {
        let (mut h, candidate) =
            actual_template_host(sf_module_kit::BuiltinTemplate::PurchasedAssembly);
        let old = h.run("LOCAL");
        h.finish_one();
        let old = h.completed_run(&old);
        let frozen_old = serde_json::to_value(&old).unwrap();
        let original_scenario = h.object("scenario", "LOCAL");
        let baseline_worker = h.object("resource", "worker-assembly");
        let source_shell = candidate
            .example_project
            .item_types
            .iter()
            .find(|i| i.id.as_str() == "shell")
            .unwrap();
        let mut existing_shell = source_shell.clone();
        existing_shell.id = id("w5-existing-shell");
        let mut existing_worker = candidate
            .example_project
            .resources
            .iter()
            .find(|r| r.id.as_str() == "W-A")
            .unwrap()
            .clone();
        existing_worker.id = id("w5-existing-worker");
        existing_worker.calendar_id = id("ALL");
        existing_worker.home_location = Some(id("shells-A"));
        let actual_worker = serde_json::to_value(&existing_worker).unwrap();
        let base = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(
            base,
            json!([
                put("item_type", serde_json::to_value(&existing_shell).unwrap()),
                put("resource", actual_worker.clone())
            ]),
        ));
        let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
        let namespace = if height.is_null() {
            "w5-explicit-unknown-height"
        } else {
            "w5-explicit-known-height"
        };
        let mut choices = template_choices(&candidate, namespace);
        choices.resources.insert(
            id("W-A"),
            sf_module_kit::TemplateResourceSelection::Reuse(id("w5-existing-worker")),
        );
        choices
            .reuse_items
            .insert(source_shell.id.clone(), existing_shell.id.clone());
        assert!(
            source_shell.spec_ref.is_none(),
            "the selected original shell still has unknown specification"
        );
        let refusal = sf_module_kit::prepare_template(&candidate, &target, &choices).unwrap_err();
        assert!(
            refusal
                .iter()
                .any(|i| i.field_path.as_deref() == Some("/reuse_items")),
            "unknown specifications cannot authorize item reuse"
        );
        choices.reuse_items.clear();
        let mut plan = sf_module_kit::prepare_template(&candidate, &target, &choices).unwrap();
        let site:SiteEnvelope=serde_json::from_value(json!({"regions":[],"length_m":31,"width_m":13,"height_m":height,"usable_polygon_m":[],"clearance_m":0})).unwrap();
        sf_module_kit::fill_template_site(&mut plan, &site).unwrap();
        assert!(
            !plan
                .proposed_project
                .resources
                .iter()
                .any(|r| r.id.as_str() == "w5-existing-worker")
        );
        assert!(
            !plan
                .proposed_project
                .item_types
                .iter()
                .any(|i| i.id == existing_shell.id)
        );
        let expected = serde_json::to_value(&plan.proposed_project).unwrap();
        let request = explicit_template_request(&plan, &choices, Some(site));
        assert_eq!(
            request["subset"],
            serde_json::to_value(&candidate.subset).unwrap()
        );
        let base = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(
            base,
            json!([{"kind":"instantiate_template","data":request}]),
        ));
        let adopted = h.object("project", "P1");
        assert_eq!(h.object("resource", "worker-assembly"), baseline_worker);
        for field in ["template_adoptions", "sources", "parameter_sources"] {
            assert_eq!(
                adopted[field], expected[field],
                "{namespace}: actual source/mapping {field}"
            );
        }
        assert_eq!(h.object("resource", "w5-existing-worker"), actual_worker);
        assert_eq!(
            h.object("item_type", "w5-existing-shell"),
            serde_json::to_value(&existing_shell).unwrap()
        );
        same_business(h.object("scenario", "LOCAL"), original_scenario.clone());
        assert_eq!(adopted["scenario_refs"], json!(["LOCAL"]));
        let hall_id = plan.proposed_project.halls[0].id.as_str();
        let hall = h.object("hall", hall_id);
        assert_eq!(hall["width_m"].as_f64(), Some(31.0));
        assert_eq!(hall["depth_m"].as_f64(), Some(13.0));
        assert_eq!(hall["height_m"].as_f64(), height.as_f64());
        let height_source = adopted["parameter_sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["target"]["id"] == hall_id && p["field_path"] == "/height_m")
            .unwrap();
        assert_eq!(
            height_source["basis"],
            if height.is_null() {
                "unknown"
            } else {
                "user_provided"
            }
        );
        if height.is_null() {
            assert!(height_source["value"].is_null());
        }
        let base = h.context().project.unwrap().revision.0.value();
        let mut scenario = sf_module_kit::prepare_template_seed(
            &candidate,
            &plan,
            &sf_module_kit::TemplateSeedChoices {
                scenario_id: ScenarioId::try_from(format!("{namespace}-seed")).unwrap(),
                supply: true,
                demand: true,
                initial_quality: candidate
                    .subset
                    .adoption
                    .as_ref()
                    .unwrap()
                    .deferred_seed
                    .as_ref()
                    .unwrap()
                    .initial_stock
                    .iter()
                    .map(|stock| (stock.location_id.clone(), QualityState::Released))
                    .collect(),
                adopt_declared_inspection_results: true,
            },
        )
        .unwrap();
        scenario.project_revision = revision(base);
        let scenario_id = scenario.id.as_str().to_owned();
        let _: EditCommit = ok(h.apply(
            base,
            json!([put("scenario", serde_json::to_value(&scenario).unwrap())]),
        ));
        let task = h.run(&scenario_id);
        h.finish_one();
        let run = h.completed_run(&task);
        let frozen = serde_json::to_value(&run).unwrap();
        assert_eq!(
            run.acceptance_events
                .iter()
                .map(|a| a.at_ms.0.value())
                .collect::<Vec<_>>(),
            vec![70000, 110000]
        );
        assert_eq!(p1_support::held_ms(&frozen, "w5-existing-worker"), 80000);
        let device_id = plan
            .proposed_project
            .devices
            .iter()
            .find(|d| d.id.as_str().ends_with(":Assembly01"))
            .unwrap()
            .id
            .as_str();
        let mut edited = h.object("device", device_id);
        edited["title"] = json!("explicit receiver edit after adoption");
        let base = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(base, json!([put("device", edited.clone())])));
        let before_save = h.object("project", "P1");
        let directory = h.tree.directory("explicit-template-save");
        let moved = h.tree.directory("explicit-template-open");
        let base = h.context().project.unwrap().revision.0.value();
        let saved = h.save(base, &directory);
        h.finish_one();
        assert!(matches!(
            h.task(&saved, true).task.state,
            ComputationStatus::Completed
        ));
        copy(&directory, &moved);
        let imported = h.import(&moved);
        h.finish_one();
        let draft = h.imported(&imported);
        let _: EditCommit = ok(h.adopt(&imported, &draft, false));
        let opened = h.object("project", "P1");
        for field in ["template_adoptions", "sources", "parameter_sources"] {
            assert_eq!(opened[field], before_save[field]);
        }
        same_business(h.object("device", device_id), edited);
        assert_eq!(h.object("resource", "w5-existing-worker"), actual_worker);
        assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), frozen_old);
        assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), frozen);
        let rerun = h.run(&scenario_id);
        h.finish_one();
        assert_eq!(
            h.completed_run(&rerun)
                .acceptance_events
                .iter()
                .map(|a| a.at_ms.0.value())
                .collect::<Vec<_>>(),
            vec![70000, 110000]
        );
    }
}

#[test]
fn template_preparation_namespace_source_candidate_and_target_tampering_is_rejected_atomically_without_a_metadata_authority_channel()
 {
    let (mut h, candidate) =
        actual_template_host(sf_module_kit::BuiltinTemplate::ParallelElectromechanical);
    let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let choices = template_choices(&candidate, "w5-exact-preparation");
    let plan = sf_module_kit::prepare_template(&candidate, &target, &choices).unwrap();
    let original = explicit_template_request(&plan, &choices, None);
    for mutation in [
        "namespace",
        "source",
        "pose",
        "preparation_target",
        "forged_sources",
    ] {
        let mut request = original.clone();
        match mutation {
            "namespace" => request["preparation"]["namespace"] = json!("w5-other-namespace"),
            "source" => {
                request["preparation"]["source_project"] = json!("not-in-selected-source.json")
            }
            "pose" => {
                request["proposed_objects"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|o| o["kind"] == "device")
                    .unwrap()["data"]["pose"]["translation"]["x"] = json!(999)
            }
            "preparation_target" => {
                request["subset"]["adoption"]["preparation_bindings"][0]["target_device_id"] =
                    json!("PACK-01")
            }
            "forged_sources" => {
                request["sources"] =
                    json!([{"origin":"caller_asserted_actual_software","accepted":true}])
            }
            _ => unreachable!(),
        }
        let before = h.snapshot();
        let base = h.context().project.unwrap().revision.0.value();
        if mutation == "forged_sources" {
            let wire = json!({"api_version":"0.1","request_id":"w5-forged-metadata-wire","project_id":"P1","scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":base,"commands":[{"kind":"instantiate_template","data":request}]}});
            assert!(
                serde_json::from_value::<ApiRequest>(wire).is_err(),
                "unknown metadata channel is unrepresentable at the actual request decoder"
            );
            assert_eq!(h.snapshot(), before);
            assert!(h.host.drain_io_work().is_empty());
            continue;
        }
        let failure = error(h.apply(
            base,
            json!([{"kind":"instantiate_template","data":request}]),
        ));
        assert!(
            matches!(
                failure.code,
                ErrorCode::InvalidInput | ErrorCode::MissingReference
            ),
            "{mutation}: {failure:?}"
        );
        assert_eq!(
            h.snapshot(),
            before,
            "{mutation}: no partial adoption or source/approval creation"
        );
        assert!(h.host.drain_io_work().is_empty());
    }
}
