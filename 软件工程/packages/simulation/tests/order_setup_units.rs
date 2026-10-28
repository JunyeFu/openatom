use serde_json::{Value, json};
use sf_simulation::{Engine, EngineInput};

pub fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../commands/tests/fixtures/production")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

pub fn input(v: &Value) -> EngineInput {
    EngineInput {
        run_id: serde_json::from_value(json!("joint-owner")).unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        scenario: serde_json::from_value(v["scenario"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: serde_json::from_value(v["scenario"]["scope"].clone()).unwrap(),
    }
}
pub fn run(v: &Value) -> Value {
    let mut e = Engine::new(input(v)).unwrap_or_else(|i| panic!("{i:#?}"));
    e.run_to_end();
    serde_json::to_value(e.snapshot()).unwrap()
}
pub fn held(v: &Value, id: &str) -> u64 {
    v["resource_claims"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["resource_id"] == id)
        .flat_map(|c| c["intervals"].as_array().unwrap())
        .map(|i| i["end_ms"].as_u64().unwrap() - i["start_ms"].as_u64().unwrap())
        .sum()
}
fn single_family_b() -> Value {
    let mut v = fixture("p104-B-first");
    v["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .retain(|t| t["id"] == "TASK-B");
    v["scenario"]["orders"]
        .as_array_mut()
        .unwrap()
        .retain(|o| o["id"] == "B");
    v["project"]["products"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["id"] != "A");
    v["project"]["routes"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["id"] != "ROUTE-A");
    let device = v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "Assembly")
        .unwrap();
    device["operation_binding"] = json!({"id":"w5.op.assembly-B","version":"0.1.0"});
    device["route_operation_refs"] = json!([]);
    v
}

#[test]
fn external_setup_can_work_during_exclusive_preparation_but_activates_only_after_equipment_is_free()
{
    for (mode, setup_start, work_start) in [("external", 0, 30000), ("internal", 30000, 45000)] {
        let mut v = single_family_b();
        v["project"]["setup_policies"][0]["changeovers"][0]["preparation_mode"] = json!(mode);
        let mut def = v["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == "w5.op.assembly-B")
            .unwrap()
            .clone();
        def["id"] = json!("owner.calibration");
        def["payload"] = json!({"category":"tasks","data":{"kind":"manual_preparation","target_role":"device","prerequisites":[],"steps":[],"resource_roles":[],"duration":{"value":30000,"unit":"millisecond","basis":"user_provided","source_ref":null},"equipment_access":"exclusive","completion_effect":{"target_role":"device","marker":"calibrated","produces_material":false}}});
        v["definitions"].as_array_mut().unwrap().push(def);
        v["project"]["module_refs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"owner.calibration","version":"0.1.0"}));
        let device = v["project"]["devices"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["id"] == "Assembly")
            .unwrap();
        device["role_bindings"]["targets"]["device"] = json!({"kind":"device","id":"Assembly"});
        let mut route = v["project"]["routes"][0].clone();
        route["id"] = json!("CAL");
        route["steps"] = json!([{"id":"calibrate","device_id":"Assembly","operation_ref":{"id":"owner.calibration","version":"0.1.0"},"predecessors":[],"item_bindings":{},"resource_bindings":{}}]);
        v["project"]["routes"].as_array_mut().unwrap().push(route);
        v["scenario"]["production_tasks"].as_array_mut().unwrap().push(json!({"id":"CAL","routing_ref":{"id":"CAL","revision":"r1"},"order_allocations":[],"planned_quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"release_at_ms":0,"priority":2}));
        let r = run(&v);
        let setup = r["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["id"].as_str().unwrap().starts_with("setup:"))
            .unwrap();
        assert_eq!(setup["started_at_ms"], setup_start);
        assert_eq!(setup["finished_at_ms"], work_start);
        let first = r["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| {
                j["operation_ref"]["id"] == "w5.op.assembly-B"
                    && !j["input_refs"].as_array().unwrap().is_empty()
            })
            .map(|j| j["started_at_ms"].as_u64().unwrap())
            .min();
        assert_eq!(first, Some(work_start));
        let work = r["phase_work_intervals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["job_id"] == setup["id"])
            .unwrap();
        assert_eq!(
            work["interval"],
            json!({"start_ms":setup_start,"end_ms":setup_start+15000})
        );
    }
}

#[test]
fn unknown_cold_start_never_becomes_a_free_family_change() {
    let mut v = single_family_b();
    v["project"]["setup_policies"][0]["initial_family"] = Value::Null;
    v["scenario"]["initial_conditions"]["initial_setup_families"] = json!({});
    let before = v.clone();
    let errors = Engine::new(input(&v)).err().unwrap();
    assert!(
        errors
            .iter()
            .any(|i| i.code == "simulation.setup_transition_missing")
    );
    assert_eq!(v, before);
}
#[test]
fn original_multi_order_fixed_endpoints_and_setup_costs_use_one_physical_assembly() {
    for (first, work, late) in [("A", 135000, 5000), ("B", 160000, 45000)] {
        let r = run(&fixture(&format!("p104-{first}-first")));
        assert_eq!(r["goal_met"], true, "{r:#}");
        assert_eq!(held(&r, "W-A"), work);
        let last = r["acceptance_events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["at_ms"].as_u64().unwrap())
            .max();
        assert_eq!(last, Some(205000));
        assert_eq!(
            r["orders"]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o["tardiness_ms"].as_u64().unwrap())
                .max(),
            Some(late)
        );
    }
}
#[test]
fn actual_first_article_pass_and_unknown_gate_the_remaining_batch() {
    for pending in [false, true] {
        let mut v = fixture("p104-B-first");
        v["scenario"]["production_tasks"]
            .as_array_mut()
            .unwrap()
            .retain(|t| t["id"] == "TASK-B");
        v["scenario"]["orders"]
            .as_array_mut()
            .unwrap()
            .retain(|o| o["id"] == "B");
        // This owner test selects one recipe through the legacy exact default;
        // the independent A/B tests additionally exercise the shared allowlist.
        v["project"]["products"]
            .as_array_mut()
            .unwrap()
            .retain(|p| p["id"] != "A");
        v["project"]["routes"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["id"] != "ROUTE-A");
        let device = v["project"]["devices"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["id"] == "Assembly")
            .unwrap();
        device["operation_binding"] = json!({"id":"w5.op.assembly-B","version":"0.1.0"});
        device["route_operation_refs"] = json!([]);
        v["project"]["setup_policies"][0]["first_article"] = json!({"route_ref":{"id":"ROUTE-B","revision":"r1"},"family":"B","inspection_module":{"id":"w5.op.inspect","version":"0.1.0"},"gate_id":"B-FIRST-ARTICLE"});
        if pending {
            v["scenario"]["quality_presets"][0]["results"] = json!([null]);
        }
        let r = run(&v);
        let starts: Vec<_> = r["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| {
                j["operation_ref"]["id"] == "w5.op.assembly-B"
                    && !j["input_refs"].as_array().unwrap().is_empty()
            })
            .map(|j| j["started_at_ms"].as_u64().unwrap())
            .collect();
        assert_eq!(
            starts,
            if pending {
                vec![15000]
            } else {
                vec![15000, 55000]
            }
        );
    }
}
