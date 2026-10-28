//! Independent whole-package expectations from the frozen local production case.
use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap()
}

fn engine_input(value: &Value) -> EngineInput {
    EngineInput {
        run_id: RunId::try_from("W5-INDEPENDENT-RUN".to_owned()).unwrap(),
        input_ref: serde_json::from_value(value["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(value["project"].clone()).unwrap(),
        scenario: serde_json::from_value(value["scenario"].clone()).unwrap(),
        definitions: serde_json::from_value(value["definitions"].clone()).unwrap(),
        scope: serde_json::from_value(value["scenario"]["scope"].clone()).unwrap(),
    }
}

fn engine(value: &Value) -> Engine {
    Engine::new(engine_input(value))
        .unwrap_or_else(|issues| panic!("complete independent input rejected: {issues:?}"))
}

fn snapshot(engine: &Engine) -> Value {
    serde_json::to_value(engine.snapshot()).unwrap()
}

fn run(value: &Value) -> Value {
    let mut engine = engine(value);
    engine.run_to_end();
    let result = snapshot(&engine);
    assert_eq!(result["evidence_origin"], "actual_software");
    assert_eq!(result["computation_status"], "completed");
    assert_eq!(
        result["computed_until_ms"],
        value["scenario"]["window"]["end_ms"]
    );
    assert_eq!(result["whole_window_computed"], true);
    assert_eq!(result["input_ref"], value["input_ref"]);
    assert_eq!(
        snapshot(&engine),
        result,
        "reading cannot produce another output"
    );
    let events = result["events"].as_array().unwrap();
    for pair in events.windows(2) {
        assert!(pair[0]["sequence"].as_u64().unwrap() < pair[1]["sequence"].as_u64().unwrap());
        assert!(pair[0]["at_ms"].as_u64().unwrap() <= pair[1]["at_ms"].as_u64().unwrap());
    }
    result
}

fn items(result: &Value, kind: &str) -> Vec<Value> {
    result["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            item["item_type_ref"]["id"] == kind
                && item["lifecycle"] != "transformed"
                && item["lifecycle"] != "scrapped"
        })
        .cloned()
        .collect()
}

fn named(result: &Value, id: &str) -> Value {
    result["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("missing {id}: {result:#}"))
        .clone()
}

fn quantity(item: &Value) -> u64 {
    assert_eq!(item["quantity"]["value"]["decimal_places"], 0);
    item["quantity"]["value"]["coefficient"].as_u64().unwrap()
}

fn assembly_only(mut value: Value) -> Value {
    value["project"]["routes"][0]["steps"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    value["project"]["boms"][0]["rows"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["consume_at_operation"] == "step-assembly");
    value["project"]["products"][0]["item_type_ref"]["id"] = json!("assembly");
    value["project"]["locations"][2]["capacity"]["value"]["coefficient"] = json!(2);
    value["scenario"]["orders"][0]["destination"] = json!("assemblies-A");
    value["scenario"]["scope"] = json!([{"kind":"device","id":"ASM-01"}]);
    value
}

#[test]
fn original_a_local_case_waits_for_missing_pcb_and_produces_one_identity_preserving_package() {
    let value = fixture();
    let result = run(&value);
    let packages = items(&result, "package");
    let assemblies = items(&result, "assembly");
    assert_eq!(packages.len(), 1);
    assert_eq!(assemblies.len(), 1);
    assert_eq!(quantity(&packages[0]), 1);
    assert_eq!(
        packages[0]["holder"],
        json!({"kind":"location","id":"packages-B"})
    );
    assert_eq!(
        assemblies[0]["holder"],
        json!({"kind":"container","id":packages[0]["id"]})
    );
    assert_eq!(packages[0]["contents"], json!([assemblies[0]["id"]]));
    assert_ne!(packages[0]["id"], json!("carton-001"));
    assert_ne!(packages[0]["id"], assemblies[0]["id"]);
    assert_eq!(packages[0]["composition_refs"], json!(["carton-001"]));
    assert_eq!(named(&result, "carton-001")["lifecycle"], "transformed");
    let mut components = assemblies[0]["composition_refs"]
        .as_array()
        .unwrap()
        .clone();
    components.sort_by_key(|v| v.as_str().unwrap().to_owned());
    assert_eq!(components, vec![json!("pcb-001"), json!("shell-001")]);
    for id in ["shell-001", "pcb-001"] {
        assert_eq!(named(&result, id)["lifecycle"], "transformed");
    }
    assert_eq!(
        named(&result, "shell-002")["holder"],
        json!({"kind":"location","id":"shells-A"})
    );
    assert_eq!(named(&result, "shell-002")["lifecycle"], "available");
    assert_eq!(
        named(&result, "carton-002")["holder"],
        json!({"kind":"location","id":"cartons-B"})
    );
    let completed: Vec<_> = result["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|job| job["state"] == "completed")
        .collect();
    assert_eq!(completed.len(), 2);
    let assembly = completed
        .iter()
        .find(|j| j["operation_ref"]["id"] == "fixture.assemble")
        .unwrap();
    assert_eq!(assembly["started_at_ms"], 0);
    assert_eq!(assembly["finished_at_ms"], 60000);
    let packing = completed
        .iter()
        .find(|j| j["operation_ref"]["id"] == "fixture.pack")
        .unwrap();
    assert_eq!(packing["started_at_ms"], 60000);
    assert_eq!(packing["finished_at_ms"], 75000);
    assert_eq!(result["acceptance_events"].as_array().unwrap().len(), 1);
    assert_eq!(result["acceptance_events"][0]["at_ms"], 75000);
    assert_eq!(result["orders"][0]["accepted"]["value"]["coefficient"], 1);
    assert_eq!(
        result["orders"][0]["outstanding"]["value"]["coefficient"],
        1
    );
    assert_eq!(result["goal_met"], false);
    assert!(result["waits"].as_array().unwrap().iter().any(|wait| {
        wait["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason["kind"] == "missing_material")
    }));
    assert_eq!(
        fixture(),
        value,
        "the frozen input is not a runtime inventory"
    );
}

#[test]
fn a_declared_second_pcb_arrival_is_not_stock_before_90000_and_finishes_local_second_package_at_165000()
 {
    let mut value = fixture();
    let mut pcb = value["scenario"]["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    value["scenario"]["arrivals"] = json!([{"id":"ADD-PCB","at_ms":90000,"destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
    let mut engine = engine(&value);
    engine.step();
    assert!(
        engine
            .objects_at(SimTimeMs(SafeUint::try_from(90000).unwrap()))
            .is_none()
    );
    let prefix = snapshot(&engine);
    assert!(
        !prefix["final_items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "pcb-002")
    );
    engine.run_to_end();
    let result = snapshot(&engine);
    assert_eq!(items(&result, "package").len(), 2);
    let times: Vec<_> = result["acceptance_events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["at_ms"].as_u64().unwrap())
        .collect();
    assert_eq!(times, vec![75000, 165000]);
    assert_eq!(
        result["orders"][0]["outstanding"]["value"]["coefficient"],
        0
    );
    assert_eq!(result["goal_met"], true);
}

#[test]
fn missing_one_required_material_never_partially_holds_other_material_or_resources() {
    let mut value = fixture();
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["item_type_ref"]["id"] != "pcb");
    let result = run(&value);
    assert!(items(&result, "package").is_empty());
    assert!(result["resource_claims"].as_array().unwrap().is_empty());
    assert!(
        !result["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| !job["started_at_ms"].is_null())
    );
    for id in ["shell-001", "shell-002"] {
        let item = named(&result, id);
        assert_eq!(item["lifecycle"], "available");
        assert_eq!(item["holder"], json!({"kind":"location","id":"shells-A"}));
    }
    assert_eq!(result["orders"][0]["accepted"]["value"]["coefficient"], 0);
    assert_eq!(result["goal_met"], false);
}

fn shared_worker_case() -> Value {
    let mut value = assembly_only(fixture());
    let mut pcb = value["scenario"]["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(pcb);
    let mut machine = value["project"]["resources"][1].clone();
    machine["id"] = json!("machine-assembly-2");
    value["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(machine);
    let mut device = value["project"]["devices"][0].clone();
    device["id"] = json!("ASM-02");
    device["role_bindings"]["resources"]["execution_capacity"] = json!("machine-assembly-2");
    value["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .push(device);
    let mut route = value["project"]["routes"][0].clone();
    route["id"] = json!("ROUTE-SECOND");
    route["steps"][0]["id"] = json!("step-assembly-2");
    route["steps"][0]["device_id"] = json!("ASM-02");
    value["project"]["routes"]
        .as_array_mut()
        .unwrap()
        .push(route);
    let mut bom = value["project"]["boms"][0].clone();
    bom["id"] = json!("BOM-SECOND");
    for row in bom["rows"].as_array_mut().unwrap() {
        row["consume_at_operation"] = json!("step-assembly-2");
    }
    value["project"]["boms"].as_array_mut().unwrap().push(bom);
    let mut product = value["project"]["products"][0].clone();
    product["id"] = json!("ASSEMBLY-SECOND");
    product["bom_ref"]["id"] = json!("BOM-SECOND");
    product["routing_ref"]["id"] = json!("ROUTE-SECOND");
    value["project"]["products"]
        .as_array_mut()
        .unwrap()
        .push(product);
    value["scenario"]["orders"][0]["quantity"]["value"]["coefficient"] = json!(1);
    value["scenario"]["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(1);
    value["scenario"]["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(1);
    let mut order = value["scenario"]["orders"][0].clone();
    order["id"] = json!("ORDER-SECOND");
    order["product_ref"]["id"] = json!("ASSEMBLY-SECOND");
    value["scenario"]["orders"]
        .as_array_mut()
        .unwrap()
        .push(order);
    let mut task = value["scenario"]["production_tasks"][0].clone();
    task["id"] = json!("TASK-SECOND");
    task["routing_ref"]["id"] = json!("ROUTE-SECOND");
    task["order_allocations"][0]["order_id"] = json!("ORDER-SECOND");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(task);
    let connections = value["project"]["connections"].as_array().unwrap().clone();
    for mut connection in connections.into_iter().take(3) {
        let data = &mut connection["data"];
        data["id"] = json!(format!("{}-2", data["id"].as_str().unwrap()));
        for key in ["source", "destination"] {
            if data[key]["kind"] == "port" {
                data[key]["data"]["instance_id"] = json!("ASM-02");
            }
        }
        value["project"]["connections"]
            .as_array_mut()
            .unwrap()
            .push(connection);
    }
    value["scenario"]["scope"] =
        json!([{"kind":"device","id":"ASM-01"},{"kind":"device","id":"ASM-02"}]);
    value
}

#[test]
fn two_actual_machine_capacities_share_one_worker_without_overlap_and_input_order_does_not_change_events()
 {
    let value = shared_worker_case();
    let result = run(&value);
    let mut intervals = Vec::new();
    for claim in result["resource_claims"].as_array().unwrap() {
        if claim["resource_id"] == "worker-assembly" {
            for interval in claim["intervals"].as_array().unwrap() {
                intervals.push((
                    interval["start_ms"].as_u64().unwrap(),
                    interval["end_ms"].as_u64().unwrap(),
                ));
            }
        }
    }
    intervals.sort();
    assert_eq!(intervals, vec![(0, 60000), (60000, 120000)]);
    assert_eq!(items(&result, "assembly").len(), 2);
    let mut reordered = value;
    for key in [
        "item_types",
        "locations",
        "devices",
        "resources",
        "routes",
        "boms",
        "products",
        "connections",
    ] {
        reordered["project"][key].as_array_mut().unwrap().reverse();
    }
    reordered["definitions"].as_array_mut().unwrap().reverse();
    reordered["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let second = run(&reordered);
    assert_eq!(second["events"], result["events"]);
    let normalized_items = |result: &Value| {
        let mut items = result["final_items"].as_array().unwrap().clone();
        items.sort_by_key(|item| item["id"].as_str().unwrap().to_owned());
        items
    };
    assert_eq!(normalized_items(&second), normalized_items(&result));
}

#[test]
fn a_full_output_location_keeps_one_pending_package_with_its_job_and_never_accepts_it() {
    let mut value = fixture();
    value["project"]["locations"][4]["capacity"]["value"]["coefficient"] = json!(0);
    let result = run(&value);
    let packages = items(&result, "package");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["holder"]["kind"], "job");
    let job = result["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|job| job["operation_ref"]["id"] == "fixture.pack" && !job["started_at_ms"].is_null())
        .unwrap();
    assert_eq!(job["state"], "waiting_output");
    assert!(job["finished_at_ms"].is_null());
    assert_eq!(job["pending_outputs"], json!([packages[0]["id"]]));
    assert!(result["acceptance_events"].as_array().unwrap().is_empty());
    assert_eq!(result["orders"][0]["accepted"]["value"]["coefficient"], 0);
    assert_eq!(result["goal_met"], false);
    assert!(
        result["resource_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|claim| claim["resource_id"] == "machine-packing"
                && claim["job_id"] == job["id"]
                && claim["intervals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|i| i["end_ms"] == 240000))
    );
}

#[test]
fn real_initial_reservation_partition_is_4_plus_6_and_resource_failures_do_not_split_it_again() {
    let mut value = assembly_only(fixture());
    value["project"]["item_types"][0]["tracking"] = json!("batch");
    value["project"]["locations"][0]["capacity"]["value"]["coefficient"] = json!(10);
    let mut batch = value["scenario"]["initial_conditions"]["items"][0].clone();
    batch["id"] = json!("BATCH10");
    batch["quantity"]["value"]["coefficient"] = json!(10);
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["item_type_ref"]["id"] != "shell");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(batch);
    value["scenario"]["initial_conditions"]["material_allocations"] = json!([{
        "id":"RESERVE4","demand_ref":"TASK-LOCAL","item_refs":["BATCH10"],
        "quantity":{"value":{"coefficient":4,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"
    }]);
    value["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"CLOSED","title":"explicit no worker interval","intervals":[],"source":"user_provided"}));
    value["project"]["resources"][0]["calendar_id"] = json!("CLOSED");
    let mut engine = engine(&value);
    engine.step();
    let initial = snapshot(&engine);
    let children = |result: &Value| {
        let mut children: Vec<_> = result["final_items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["split_from_ref"] == "BATCH10")
            .cloned()
            .collect();
        children.sort_by_key(|item| item["id"].as_str().unwrap().to_owned());
        children
    };
    let actual_children = children(&initial);
    assert_eq!(actual_children.len(), 2);
    let mut quantities: Vec<_> = actual_children.iter().map(quantity).collect();
    quantities.sort();
    assert_eq!(quantities, vec![4, 6]);
    assert_eq!(named(&initial, "BATCH10")["lifecycle"], "transformed");
    assert!(
        actual_children
            .iter()
            .all(|item| item["holder"] == json!({"kind":"location","id":"shells-A"}))
    );
    assert_ne!(initial["allocations"][0]["item_refs"], json!(["BATCH10"]));
    engine.run_to_end();
    let final_result = snapshot(&engine);
    assert_eq!(children(&final_result), actual_children);
    assert!(
        final_result["resource_claims"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(items(&final_result, "assembly").is_empty());
}

#[test]
fn initial_composition_sources_become_transformed_records_without_becoming_free_material_again() {
    let mut value = assembly_only(fixture());
    let mut assembly = value["scenario"]["initial_conditions"]["items"][0].clone();
    assembly["id"] = json!("PREASSEMBLED");
    assembly["item_type_ref"]["id"] = json!("assembly");
    assembly["holder"] = json!({"kind":"location","id":"assemblies-A"});
    assembly["components"] = json!(["shell-001", "pcb-001"]);
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != "shell-002");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(assembly);
    let mut engine = engine(&value);
    engine.step();
    let result = snapshot(&engine);
    assert_eq!(named(&result, "shell-001")["lifecycle"], "transformed");
    assert_eq!(named(&result, "pcb-001")["lifecycle"], "transformed");
    assert_eq!(
        named(&result, "shell-001")["holder"],
        json!({"kind":"location","id":"shells-A"})
    );
    assert_eq!(
        named(&result, "PREASSEMBLED")["composition_refs"],
        json!(["shell-001", "pcb-001"])
    );
    assert!(items(&result, "shell").is_empty());
    assert!(items(&result, "pcb").is_empty());
    assert!(
        !result["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| !job["started_at_ms"].is_null())
    );
}

#[test]
fn pause_step_cancel_and_observation_end_use_real_computed_boundaries() {
    let mut value = fixture();
    value["scenario"]["window"]["end_ms"] = json!(75000);
    let mut runner = engine(&value);
    runner.step();
    assert_eq!(snapshot(&runner)["computed_until_ms"], 0);
    assert!(
        runner
            .objects_at(SimTimeMs(SafeUint::try_from(75000).unwrap()))
            .is_none()
    );
    runner.pause();
    let paused = snapshot(&runner);
    assert_eq!(paused["computation_status"], "paused");
    assert_eq!(runner.step(), 0);
    assert_eq!(snapshot(&runner), paused);
    runner.resume();
    runner.run_to_end();
    let complete = snapshot(&runner);
    assert_eq!(complete["acceptance_events"][0]["at_ms"], 75000);
    assert_eq!(complete["whole_window_computed"], true);
    assert!(
        runner
            .objects_at(SimTimeMs(SafeUint::try_from(75000).unwrap()))
            .is_some()
    );
    let mut cancelled = engine(&fixture());
    cancelled.step();
    let before = snapshot(&cancelled);
    cancelled.cancel();
    let after = snapshot(&cancelled);
    assert_eq!(after["computation_status"], "cancelled");
    assert_eq!(after["whole_window_computed"], false);
    assert!(after["goal_met"].is_null());
    assert_eq!(after["final_items"], before["final_items"]);
    assert!(after["acceptance_events"].as_array().unwrap().is_empty());
}

#[test]
fn pack_identity_mapping_is_required_disjoint_known_and_covers_the_single_output() {
    let base = fixture();
    let mut variants = Vec::new();
    let mut missing = base.clone();
    missing["definitions"][1]["payload"]["data"]["pack_identity"] = Value::Null;
    variants.push(missing);
    let mut overlap = base.clone();
    overlap["definitions"][1]["payload"]["data"]["pack_identity"]["content_roles"] =
        json!(["product", "carton"]);
    variants.push(overlap);
    let mut unknown = base.clone();
    unknown["definitions"][1]["payload"]["data"]["pack_identity"]["content_roles"] =
        json!(["UNKNOWN-ROLE"]);
    variants.push(unknown);
    let mut multiple = base;
    let mut output = multiple["definitions"][1]["payload"]["data"]["outputs"][0].clone();
    output["role"] = json!("package2");
    output["port"] = json!("packed2");
    multiple["definitions"][1]["payload"]["data"]["outputs"]
        .as_array_mut()
        .unwrap()
        .push(output);
    multiple["definitions"][1]["payload"]["data"]["item_roles"]
        .as_array_mut()
        .unwrap()
        .push(json!("package2"));
    let mut port = multiple["definitions"][5]["payload"]["data"]["ports"][2].clone();
    port["id"] = json!("packed2");
    port["payload"]["data"]["item_role"] = json!("package2");
    multiple["definitions"][5]["payload"]["data"]["ports"]
        .as_array_mut()
        .unwrap()
        .push(port);
    multiple["project"]["devices"][1]["role_bindings"]["item_types"]["package2"] = json!("package");
    multiple["project"]["routes"][0]["steps"][1]["item_bindings"]["package2"] = json!("package");
    let mut connection = multiple["project"]["connections"][5].clone();
    connection["data"]["id"] = json!("pack-output-2");
    connection["data"]["source"]["data"]["port_id"] = json!("packed2");
    multiple["project"]["connections"]
        .as_array_mut()
        .unwrap()
        .push(connection);
    variants.push(multiple);
    for value in variants {
        let result = Engine::new(EngineInput {
            run_id: RunId::try_from("W5-INVALID-PACK".to_owned()).unwrap(),
            input_ref: serde_json::from_value(value["input_ref"].clone()).unwrap(),
            project: serde_json::from_value(value["project"].clone()).unwrap(),
            scenario: serde_json::from_value(value["scenario"].clone()).unwrap(),
            definitions: serde_json::from_value(value["definitions"].clone()).unwrap(),
            scope: serde_json::from_value(value["scenario"]["scope"].clone()).unwrap(),
        });
        let issues = match result {
            Err(issues) => issues,
            Ok(_) => panic!("invalid explicit Pack mapping executed"),
        };
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|issue| {
            issue
                .blocks
                .iter()
                .any(|p| matches!(p, CapabilityPurpose::Execution))
        }));
    }
}

#[test]
fn a_legacy_no_order_task_still_produces_and_never_invents_an_order_line_or_partial_flag() {
    let mut value = fixture();
    value["scenario"]["orders"] = json!([]);
    value["scenario"]["production_tasks"][0]["order_allocations"] = json!([]);
    let result = run(&value);
    assert_eq!(items(&result, "package").len(), 1);
    assert!(result["orders"].as_array().unwrap().is_empty());
    assert!(
        result["acceptance_events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["order_id"].is_null())
    );
    assert!(value["scenario"]["orders"].as_array().unwrap().is_empty());
    assert!(
        result["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| job["operation_ref"]["id"] == "fixture.pack" && job["state"] == "completed")
    );
}

#[test]
fn a_fork_with_no_remaining_orders_or_tasks_does_not_turn_initial_stock_into_unscheduled_production()
 {
    let mut value = fixture();
    value["scenario"]["orders"] = json!([]);
    value["scenario"]["production_tasks"] = json!([]);
    let result = run(&value);
    assert!(result["jobs"].as_array().unwrap().is_empty());
    assert!(items(&result, "package").is_empty());
    assert!(result["acceptance_events"].as_array().unwrap().is_empty());
    for initial in value["scenario"]["initial_conditions"]["items"]
        .as_array()
        .unwrap()
    {
        let actual = named(&result, initial["id"].as_str().unwrap());
        assert_eq!(actual["holder"], initial["holder"]);
        assert_eq!(actual["quantity"], initial["quantity"]);
        assert_eq!(actual["lifecycle"], "available");
    }
    value["scenario"]["scope"] = json!([{"kind":"device","id":"MISSING-DEVICE"}]);
    assert!(
        Engine::new(engine_input(&value)).is_err(),
        "empty demand does not authorize an unknown selected device"
    );
}

fn collision_reservation_fixture(closed: bool) -> Value {
    let mut value = assembly_only(fixture());
    value["project"]["item_types"][0]["tracking"] = json!("batch");
    value["project"]["locations"][0]["capacity"]["value"]["coefficient"] = json!(20);
    let mut batch = value["scenario"]["initial_conditions"]["items"][0].clone();
    batch["id"] = json!("BATCH10");
    batch["quantity"]["value"]["coefficient"] = json!(10);
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["item_type_ref"]["id"] != "shell");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(batch);
    value["scenario"]["initial_conditions"]["material_allocations"] = json!([{
        "id":"RESERVE4","demand_ref":"TASK-LOCAL","item_refs":["BATCH10"],
        "quantity":{"value":{"coefficient":4,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"
    }]);
    if closed {
        value["project"]["calendars"].as_array_mut().unwrap().push(
            json!({"id":"CLOSED","title":"no acquisition","intervals":[],"source":"user_provided"}),
        );
        value["project"]["resources"][0]["calendar_id"] = json!("CLOSED");
    }
    value
}

fn assert_identity_admission_rejected(value: &Value) {
    let issues = match Engine::new(engine_input(value)) {
        Err(issues) => issues,
        Ok(_) => panic!("a declared physical identity was silently replaced or renamed"),
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "simulation.generated_item_identity_collision"),
        "{issues:?}"
    );
}

#[test]
fn a_future_declared_arrival_cannot_overwrite_an_initial_reserved_child_or_turn_fifteen_into_eleven()
 {
    let mut value = collision_reservation_fixture(true);
    let mut probe = engine(&value);
    probe.step();
    let baseline = snapshot(&probe);
    let reserved = baseline["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["split_from_ref"] == "BATCH10" && quantity(item) == 4)
        .unwrap();
    let mut arrival = value["scenario"]["initial_conditions"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "BATCH10")
        .unwrap()
        .clone();
    arrival["id"] = json!("NONCOLLIDING-ARRIVAL5");
    arrival["quantity"]["value"]["coefficient"] = json!(5);
    value["scenario"]["arrivals"] = json!([{"id":"FIVE-MORE","at_ms":1000,"destination":"shells-A","items":[arrival],"release_policy":"unchecked_allowed"}]);
    let good = run(&value);
    assert_eq!(items(&good, "shell").iter().map(quantity).sum::<u64>(), 15);
    value["scenario"]["arrivals"][0]["items"][0]["id"] = reserved["id"].clone();
    let original = value.clone();
    assert_identity_admission_rejected(&value);
    assert_eq!(value, original);
    assert_eq!(
        value["scenario"]["arrivals"][0]["items"][0]["quantity"]["value"]["coefficient"],
        5
    );
}

#[test]
fn running_split_collision_fails_before_material_holder_quantity_or_resource_claims_change() {
    let mut value = collision_reservation_fixture(false);
    let mut probe = engine(&value);
    probe.step();
    let baseline = snapshot(&probe);
    let generated = baseline["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| {
            item["split_from_ref"]
                .as_str()
                .is_some_and(|id| id != "BATCH10")
                && item["lifecycle"] == "in_process"
        })
        .unwrap();
    let mut declared = value["scenario"]["initial_conditions"]["items"][0].clone();
    declared["id"] = generated["id"].clone();
    declared["item_type_ref"]["id"] = json!("package");
    declared["quantity"]["value"]["coefficient"] = json!(1);
    declared["holder"] = json!({"kind":"location","id":"packages-B"});
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(declared);
    let mut runner = engine(&value);
    let before = snapshot(&runner);
    runner.step();
    let after = snapshot(&runner);
    assert_eq!(after["computation_status"], "failed");
    assert!(
        after["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "simulation.generated_item_identity_collision")
    );
    assert_eq!(after["final_items"], before["final_items"]);
    assert_eq!(after["resource_claims"], before["resource_claims"]);
    assert_eq!(items(&after, "shell").iter().map(quantity).sum::<u64>(), 10);
    assert_eq!(
        named(&after, generated["id"].as_str().unwrap())["item_type_ref"]["id"],
        "package"
    );
}

#[test]
fn a_new_package_output_id_cannot_replace_a_declared_material_record_or_form_a_self_reference() {
    let mut value = fixture();
    let baseline = run(&value);
    let package = items(&baseline, "package")[0].clone();
    value["scenario"]["initial_conditions"]["items"][3]["id"] = package["id"].clone();
    let original = value.clone();
    assert_identity_admission_rejected(&value);
    assert_eq!(value, original);
    assert_eq!(
        value["scenario"]["initial_conditions"]["items"][3]["item_type_ref"]["id"],
        "carton"
    );
}

#[test]
fn a_real_preparation_task_on_the_selected_device_finishes_before_material_processing_and_runs_once()
 {
    let mut value = fixture();
    let mut preparation = value["definitions"][0].clone();
    preparation["id"] = json!("fixture.prepare");
    preparation["title"] = json!("explicit same-device preparation");
    preparation["payload"] = json!({"category":"tasks","data":{
        "kind":"manual_preparation","target_role":"device","prerequisites":[],"steps":[],
        "resource_roles":[{"role":"worker","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"person"},"qualifications":[]}],
        "duration":{"value":500,"unit":"millisecond","basis":"user_provided","source_ref":null},
        "equipment_access":"exclusive",
        "completion_effect":{"target_role":"device","marker":"ready","produces_material":false}
    }});
    value["definitions"]
        .as_array_mut()
        .unwrap()
        .push(preparation);
    value["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"fixture.prepare","version":"0.1.0"}));
    value["project"]["devices"][0]["role_bindings"]["targets"]["device"] =
        json!({"kind":"device","id":"ASM-01"});
    let steps = value["project"]["routes"][0]["steps"]
        .as_array_mut()
        .unwrap();
    steps[0]["predecessors"] = json!(["step-prepare"]);
    steps.insert(0,json!({"id":"step-prepare","operation_ref":{"id":"fixture.prepare","version":"0.1.0"},
        "device_id":"ASM-01","predecessors":[],"item_bindings":{},"resource_bindings":{"worker":"worker-assembly"}}));
    let result = run(&value);
    let jobs = result["jobs"].as_array().unwrap();
    let preparation_jobs: Vec<_> = jobs
        .iter()
        .filter(|job| job["operation_ref"]["id"] == "fixture.prepare")
        .collect();
    assert_eq!(
        preparation_jobs.len(),
        1,
        "one target/configuration has one actual preparation"
    );
    assert_eq!(preparation_jobs[0]["state"], "completed");
    assert_eq!(preparation_jobs[0]["started_at_ms"], 0);
    assert_eq!(preparation_jobs[0]["finished_at_ms"], 500);
    let assembly = jobs
        .iter()
        .find(|job| job["operation_ref"]["id"] == "fixture.assemble" && job["state"] == "completed")
        .unwrap();
    assert_eq!(assembly["started_at_ms"], 500);
    assert_eq!(assembly["finished_at_ms"], 60500);
    assert_eq!(result["acceptance_events"].as_array().unwrap().len(), 1);
    assert_eq!(result["acceptance_events"][0]["at_ms"], 75500);
    assert_eq!(items(&result, "assembly").len(), 1);
    assert_eq!(items(&result, "package").len(), 1);
    assert_eq!(named(&result, "shell-002")["lifecycle"], "available");
    assert_eq!(named(&result, "carton-002")["lifecycle"], "available");
}

#[test]
fn a_requested_unloading_acceptance_stage_cannot_be_claimed_by_a_production_only_engine() {
    let mut value = fixture();
    value["scenario"]["orders"][0]["acceptance_stage"] = json!("unloaded_and_released");
    let before = value.clone();
    let issues = match Engine::new(engine_input(&value)) {
        Err(issues) => issues,
        Ok(_) => {
            panic!("production without transport/unloading cannot establish unloaded_and_released")
        }
    };
    assert!(
        issues.iter().any(|issue| {
            issue.field_path.as_deref() == Some("/orders/0/acceptance_stage")
                && issue
                    .blocks
                    .iter()
                    .any(|purpose| matches!(purpose, CapabilityPurpose::Execution))
        }),
        "unsupported actual stage needs a located execution blocker: {issues:?}"
    );
    assert_eq!(
        value, before,
        "do not silently lower the requested acceptance stage"
    );
}

#[test]
fn dispatch_candidates_keep_task_reservations_and_rebuild_after_actual_stock_changes() {
    let mut value = assembly_only(fixture());
    let shell = value["scenario"]["initial_conditions"]["items"][0].clone();
    let pcb = value["scenario"]["initial_conditions"]["items"][2].clone();
    let shells: Vec<_> = ["A-FREE-SHELL", "Z-A-SHELL", "Z-B-SHELL"].into_iter().map(|id| {
        let mut item = shell.clone();
        item["id"] = json!(id);
        item
    }).collect();
    value["scenario"]["initial_conditions"]["items"] = json!([
        shells[0], shells[1], shells[2], pcb
    ]);
    value["project"]["locations"][0]["capacity"]["value"]["coefficient"] = json!(3);
    let mut a = value["scenario"]["production_tasks"][0].clone();
    a["id"] = json!("TASK-A");
    a["planned_quantity"]["value"]["coefficient"] = json!(1);
    a["order_allocations"][0]["quantity"]["value"]["coefficient"] = json!(1);
    let mut b = a.clone();
    b["id"] = json!("TASK-B");
    b["priority"] = json!(10);
    value["scenario"]["production_tasks"] = json!([a, b]);
    value["scenario"]["initial_conditions"]["material_allocations"] = json!([
        {"id":"RA","demand_ref":"TASK-A","item_refs":["Z-A-SHELL"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"},
        {"id":"RB","demand_ref":"TASK-B","item_refs":["Z-B-SHELL"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"},
        {"id":"RP","demand_ref":"TASK-A","item_refs":["pcb-001"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"}
    ]);
    let mut later_pcb = pcb;
    later_pcb["id"] = json!("pcb-002");
    value["scenario"]["arrivals"] = json!([{
        "id":"later-pcb","at_ms":90000,"destination":"pcbs-A",
        "items":[later_pcb],"release_policy":"unchecked_allowed"
    }]);

    let result = run(&value);
    let job = |task: &str| result["jobs"].as_array().unwrap().iter()
        .find(|job| job["id"].as_str().unwrap().contains(task)).unwrap();
    let a = job("TASK-A");
    let b = job("TASK-B");
    // B is inspected first but cannot take A's PCB. A must prefer its own
    // reserved shell over the lexically earlier free shell in the same bucket.
    assert_eq!(a["started_at_ms"], 0);
    assert_eq!(a["input_refs"], json!(["Z-A-SHELL", "pcb-001"]));
    assert_eq!(b["started_at_ms"], 90000);
    assert_eq!(b["input_refs"], json!(["Z-B-SHELL", "pcb-002"]));
    assert_eq!(named(&result, "A-FREE-SHELL")["lifecycle"], "available");
    assert_eq!(items(&result, "assembly").len(), 2);
    assert_eq!(result["orders"][0]["accepted"]["value"]["coefficient"], 2);
    assert!(result["waits"].as_array().unwrap().iter().any(|wait| {
        wait["object_ref"]["id"] == b["id"]
            && ["missing_material", "resource_busy"].iter().all(|kind| {
                wait["reasons"].as_array().unwrap().iter().any(|reason| reason["kind"] == *kind)
            })
    }));
}
