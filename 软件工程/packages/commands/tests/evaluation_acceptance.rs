//! Independent full P2-06 consumers and actual Engine evidence for P2-04.
#[path = "fixtures/production/host_support.rs"]
mod host;
#[path = "fixtures/production/p1_engine_support.rs"]
mod simulation;
use serde_json::{Value, json};
use sf_commands::*;
#[path = "../../../tests/planner/acceptance/evaluation_host.rs"]
mod evaluation_host;
#[path = "../../../tests/planner/acceptance/setup_host.rs"]
mod setup_host;

fn source(name: &str) -> Value {
    serde_json::from_str(match name {
        "ix_base" => include_str!("fixtures/production/p202-ix01-baseline.json"),
        "ix_late" => include_str!("fixtures/production/p202-ix01-delayed-pcb.json"),
        "halls_a" => include_str!("fixtures/production/p202-halls-A.json"),
        "halls_b" => include_str!("fixtures/production/p202-halls-B.json"),
        "order_a" => include_str!("fixtures/production/p104-A-first.json"),
        "order_b" => include_str!("fixtures/production/p104-B-first.json"),
        "quality" => include_str!("fixtures/quality/quality-original.json"),
        "phase" => include_str!("fixtures/production/p1-phase-calendar.json"),
        _ => panic!("unknown frozen source"),
    })
    .unwrap()
}
fn q(n: u64, unit: &str) -> Value {
    json!({"value":{"coefficient":n,"decimal_places":0},"unit":unit})
}
fn price(
    id: &str,
    target: &str,
    name: &str,
    basis: &str,
    rate: u64,
    denominator: u64,
    unit: &str,
) -> Value {
    json!({"id":id,"object_ref":{"kind":target,"id":name},"component":if id.starts_with("labor"){"labor"}else{id},
        "basis":basis,"rate":{"coefficient":rate,"decimal_places":0},"currency":"CNY","rate_quantity":q(denominator,unit),"includes":[],
        "source":{"target":{"kind":target,"id":name},"field_path":"/declared_cost_rate","value":null,"basis":"user_provided","source_ref":null}})
}
fn priced_order(variant: &str) -> Value {
    let mut v = source(if variant == "order_b" {
        "order_b"
    } else {
        "order_a"
    });
    v["scenario"]["cost_inputs"] = json!([
        price(
            "labor",
            "resource",
            "W-A",
            "occupied_time",
            36,
            3600000,
            "person_ms"
        ),
        price(
            "cartons",
            "item_type",
            "carton",
            "consumed_material",
            10,
            4,
            "piece"
        ),
        price(
            "fasteners",
            "item_type",
            "fastener",
            "consumed_material",
            0,
            1,
            "piece"
        )
    ]);
    if variant == "unknown_currency" {
        v["scenario"]["cost_inputs"][2]["currency"] = Value::Null;
    }
    if variant == "dual_labor" {
        v["scenario"]["cost_inputs"]
            .as_array_mut()
            .unwrap()
            .push(price(
                "labor-calendar",
                "resource",
                "W-A",
                "paid_calendar",
                36,
                3600000,
                "person_ms",
            ));
    }
    if variant == "capacity_two" {
        v["project"]["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == "W-A")
            .unwrap()["capacity"] = q(2, "person");
    }
    v
}
fn fractional() -> Value {
    let mut v = source("quality");
    v["scenario"]["cost_inputs"] = json!([price(
        "fractional-cartons",
        "item_type",
        "carton",
        "consumed_material",
        67500,
        7,
        "piece"
    )]);
    v
}
fn escaped() -> Value {
    let mut v = fractional();
    v["scenario"]["cost_inputs"][0]["component"] = json!("纸箱，\"7件计价\"\n=SUM(1+1)");
    v
}
fn exact_decimal_paid_calendar() -> Value {
    let mut v = source("order_a");
    v["scenario"]["window"]["end_ms"] = json!(3);
    let mut input = price(
        "exact-decimal",
        "resource",
        "W-A",
        "paid_calendar",
        1,
        15,
        "person_ms",
    );
    input["rate"] = json!({"coefficient":1,"decimal_places":1});
    v["scenario"]["cost_inputs"] = json!([input]);
    v
}
fn recorded(mut value: Value, name: &str, partial: bool) -> (Value, Value) {
    value["scenario"]["id"] = json!(format!("W5-EVALUATION-{name}"));
    value["project"]["scenario_refs"] = json!([value["scenario"]["id"]]);
    value["input_ref"]["scenario_id"] = value["scenario"]["id"].clone();
    let mut e = simulation::engine(&value);
    if partial {
        for _ in 0..3 {
            e.step();
        }
        e.cancel();
    } else {
        e.run_to_end();
    }
    let result = simulation::snapshot(&e);
    assert_eq!(result["evidence_origin"], "actual_software");
    let mut observation = serde_json::Map::new();
    if !partial {
        for at in [0, 80000, 370000, 420000, 425000] {
            if at <= value["scenario"]["window"]["end_ms"].as_u64().unwrap() {
                observation.insert(
                    at.to_string(),
                    serde_json::to_value(
                        e.objects_at(serde_json::from_value(json!(at)).unwrap())
                            .unwrap(),
                    )
                    .unwrap(),
                );
            }
        }
    }
    (
        json!([
            value["project"],
            value["scenario"],
            value["definitions"],
            result
        ]),
        Value::Object(observation),
    )
}
#[test]
fn record_real_frozen_evaluation_and_timeline_inputs_without_any_report_or_alternate_engine() {
    let definitions = vec![
        ("ix_base", source("ix_base"), false),
        ("ix_late", source("ix_late"), false),
        ("halls_a", source("halls_a"), false),
        ("halls_b", source("halls_b"), false),
        ("order_known", priced_order("known"), false),
        ("order_b", priced_order("order_b"), false),
        ("order_unknown", priced_order("unknown_currency"), false),
        ("order_dual", priced_order("dual_labor"), false),
        ("order_capacity_two", priced_order("capacity_two"), false),
        ("quality_fraction", fractional(), false),
        ("quality_partial", fractional(), true),
        ("phase_calendar", source("phase"), false),
        ("quality_escaped", escaped(), false),
        ("exact_decimal", exact_decimal_paid_calendar(), false),
    ];
    let mut inputs = serde_json::Map::new();
    let mut observations = serde_json::Map::new();
    for (name, v, partial) in definitions {
        let (frozen, items) = recorded(v, name, partial);
        inputs.insert(name.into(), frozen);
        observations.insert(name.into(), items);
    }
    for (name, end) in [("ix_base", 440000), ("ix_late", 570000)] {
        let r = &inputs[name][3];
        let max = simulation::jobs(r, "w5.op.pack")
            .iter()
            .filter_map(|j| j["finished_at_ms"].as_u64())
            .max();
        assert_eq!(max, Some(end));
        assert_eq!(simulation::physical(r, "package").len(), 6);
    }
    assert_eq!(
        simulation::held_ms(&inputs["order_known"][3], "W-A"),
        135000
    );
    assert_eq!(simulation::held_ms(&inputs["order_b"][3], "W-A"), 160000);
    assert_eq!(
        simulation::held_ms(&inputs["order_capacity_two"][3], "W-A"),
        135000
    );
    assert_eq!(inputs["quality_partial"][3]["whole_window_computed"], false);
    let output = json!({"document_kind":"actual_software_frozen_inputs_recorded_by_independent_commands_consumer","inputs":inputs,"observations":observations,"no_report_metrics_copied_from_owner":true});
    if let Some(path) = std::env::var_os("W5_RECORD_EVALUATION_INPUTS") {
        host::write(std::path::Path::new(&path), &output);
    }
}

fn proposal_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/planner/acceptance/timeline-input-proposal.json"
    ))
    .unwrap()
}
fn proposal_seed() -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../reporting/tests/fixtures/evaluation-acceptance/actual-engine-inputs.json"
    ))
    .unwrap();
    let input = &fixture["inputs"]["ix_base"];
    json!({"project":input[0],"scenario":input[1],"definitions":input[2]})
}
#[test]
fn actual_timeline_proposal_is_one_real_host_commit_and_only_explicit_rerun_gets_new_input() {
    let proposal = proposal_fixture();
    let mut h = host::Harness::from_value(proposal_seed());
    let scenario = proposal["scenario_id"].as_str().unwrap();
    let task = h.run(scenario);
    h.finish_one();
    let old = h.completed_run(&task);
    let old_json = serde_json::to_value(&old).unwrap();
    assert_eq!(old_json["input_ref"]["scenario_revision"], 1);
    let before = h.context().run_summaries.len();
    let base = proposal["base_project_revision"].as_u64().unwrap();
    assert_eq!(h.context().project.unwrap().revision.0.value(), base);
    let commit: EditCommit = host::ok(h.apply(base, json!([proposal["command"]])));
    assert_eq!(commit.new_revision.0.value(), base + 1);
    let changed = h.object("scenario", scenario);
    assert_eq!(changed["revision"], 2);
    assert_eq!(changed["project_revision"], 2);
    assert_eq!(changed["arrivals"][0]["at_ms"], 330000);
    assert_eq!(changed["production_tasks"][0]["release_at_ms"], 90000);
    assert_eq!(
        h.context().run_summaries.len(),
        before,
        "input commit must not automatically compute another Run"
    );
    assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), old_json);
    let task = h.run(scenario);
    h.finish_one();
    let next = h.completed_run(&task);
    assert_ne!(next.id, old.id);
    assert_ne!(
        serde_json::to_value(&next.input_ref).unwrap(),
        serde_json::to_value(&old.input_ref).unwrap()
    );
    let next = serde_json::to_value(next).unwrap();
    assert!(
        next["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["payload"]["kind"] == "arrival_accepted"
                && e["payload"]["data"] == "pcb-batch2"
                && e["at_ms"] == 330000)
    );
    assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), old_json);
}
#[test]
fn stale_timeline_proposal_is_rejected_by_real_host_without_overwriting_new_input_or_preview() {
    let proposal = proposal_fixture();
    let preserved = proposal.clone();
    let mut h = host::Harness::from_value(proposal_seed());
    let name = proposal["scenario_id"].as_str().unwrap();
    let mut current = h.object("scenario", name);
    current["title"] = json!("真实并发修改后的方案");
    let _: EditCommit = host::ok(h.apply(1, json!([host::put("scenario", current)])));
    let snapshot = h.object("scenario", name);
    let error = host::error(h.apply(1, json!([proposal["command"]])));
    assert!(matches!(error.code, ErrorCode::RevisionConflict));
    assert_eq!(h.object("scenario", name), snapshot);
    assert_eq!(proposal, preserved);
    assert_eq!(snapshot["arrivals"][0]["at_ms"], 240000);
}
