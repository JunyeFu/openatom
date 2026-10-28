//! An order counts finished pieces; a declared six-output operation counts cycles.
use serde_json::{Value, json};
use sf_simulation::{Engine, EngineInput};

fn fixture(planned: u64, allocated: bool) -> EngineInput {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    )).unwrap();
    v["project"]["routes"][0]["steps"].as_array_mut().unwrap().truncate(1);
    v["project"]["boms"][0]["rows"].as_array_mut().unwrap().truncate(2);
    v["project"]["products"][0]["item_type_ref"]["id"] = json!("assembly");
    v["scenario"]["orders"][0]["destination"] = json!("assemblies-A");
    v["scenario"]["orders"][0]["quantity"]["value"]["coefficient"] = json!(planned);
    v["scenario"]["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(planned);
    v["scenario"]["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] = json!(planned);
    let op = v["definitions"].as_array_mut().unwrap().iter_mut()
        .find(|d| d["id"] == "fixture.assemble").unwrap();
    op["payload"]["data"]["builtin"] = json!("transform");
    op["payload"]["data"]["identity_policy"] = json!("transform_with_origin");
    op["payload"]["data"]["outputs"][0]["quantity"]["value"]["coefficient"] = json!(6);
    let original = v["scenario"]["initial_conditions"]["items"].as_array().unwrap().clone();
    v["scenario"]["initial_conditions"]["items"] = json!([]);
    for kind in ["shell", "pcb"] {
        let seed = original.iter().find(|i| i["item_type_ref"]["id"] == kind).unwrap();
        for i in 0..3 {
            let mut item = seed.clone(); item["id"] = json!(format!("{kind}-{i}"));
            v["scenario"]["initial_conditions"]["items"].as_array_mut().unwrap().push(item);
        }
    }
    for location in v["project"]["locations"].as_array_mut().unwrap() {
        if location["id"] == "assemblies-A" {
            location["capacity"]["value"]["coefficient"] = json!(18);
        }
    }
    if !allocated {
        v["scenario"]["orders"] = json!([]);
        v["scenario"]["production_tasks"][0]["order_allocations"] = json!([]);
    }
    EngineInput {
        run_id: serde_json::from_value(json!("yield-run")).unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        scenario: serde_json::from_value(v["scenario"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: vec![],
    }
}

#[test]
fn eighteen_finished_pieces_execute_three_six_output_cycles() {
    let mut engine = Engine::new(fixture(18, true)).unwrap();
    engine.run_to_end();
    let run = engine.snapshot();
    assert_eq!(run.jobs.len(), 3);
    assert_eq!(run.orders[0].accepted.value.coefficient.value(), 18);
    assert_eq!(run.orders[0].outstanding.value.coefficient.value(), 0);
}

#[test]
fn a_partial_six_piece_cycle_requires_an_explicit_replan() {
    let Err(issues) = Engine::new(fixture(17, true)) else { panic!("Cannot invent a partial cycle") };
    assert!(issues.iter().any(|i| i.code == "simulation.product_cycle_quantity"));
}

#[test]
fn an_unallocated_normal_use_task_still_declares_operation_cycles() {
    let mut engine = Engine::new(fixture(3, false)).unwrap();
    engine.run_to_end();
    let run = engine.snapshot();
    assert_eq!(run.jobs.len(), 3);
    assert!(run.orders.is_empty());
    assert_eq!(run.final_items.iter().filter(|i| i.item_type_ref.id.as_str() == "assembly").count(), 18);
}
