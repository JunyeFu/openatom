#![allow(dead_code)]
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
pub mod original;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn out() -> PathBuf {
    root().join(".work/full-closeout-20261021/delivery")
}
pub fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/quality/quality-original.json"
    ))
    .unwrap()
}
pub fn assembly(cycle: usize) -> String {
    let j = format!("job:10:TASK-LOCAL1:{cycle}13:step-assembly");
    format!("output:{}:{j}9:assembled1:0", j.len())
}
pub fn inspect(v: &mut Value) -> &mut Value {
    &mut v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.inspect")
        .unwrap()["payload"]["data"]
}
pub fn accepted(v: &Value) -> u64 {
    v["acceptance_events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["quantity"]["value"]["coefficient"].as_u64().unwrap())
        .sum()
}
pub fn explicit_order(v: &mut Value) {
    let goal = v["scenario"]["acceptance_goal"].clone();
    let quantity = goal["accepted_quantity_per_day"].clone();
    v["scenario"]["orders"] = json!([{"id":"NONCU-QUALITY-ORDER","product_ref":goal["acceptance_scope"]["product_ref"],"quantity":quantity,"release_at_ms":0,"due_at_ms":120000,"destination":goal["acceptance_scope"]["destination"],"acceptance_stage":goal["acceptance_scope"]["stage"],"allow_partial":true,"priority":0}]);
    v["scenario"]["production_tasks"][0]["order_allocations"] =
        json!([{"order_id":"NONCU-QUALITY-ORDER","quantity":quantity}]);
    v["scenario"]["acceptance_goal"] = Value::Null;
}
pub fn invalid_decisions(v: &mut Value) {
    let release = v["scenario"]["initial_conditions"]["declared_quality_decisions"][0].clone();
    let scrap = v["scenario"]["initial_conditions"]["declared_quality_decisions"][1].clone();
    let mut rows = vec![scrap];
    for kind in ["early", "wrong-version", "wrong-source", "wrong-item"] {
        let mut d = release.clone();
        d["id"] = json!(format!("invalid-{kind}"));
        match kind {
            "early" => d["effective_at_ms"] = json!(10000),
            "wrong-version" => d["inspection_ref"]["version"] = json!("0.2.0"),
            "wrong-source" => d["decision_source"]["data"] = json!("QC-2"),
            "wrong-item" => d["item_ref"] = json!(assembly(0)),
            _ => unreachable!(),
        }
        rows.push(d);
    }
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!(rows);
}
pub fn profile(name: &str) -> Value {
    let mut v = fixture();
    match name {
        "original" => {}
        "hold" => {
            inspect(&mut v)["quality_and_routes"]["pending_policy"] = json!("hold_station");
            v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
        }
        "full" => {
            v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
            let mut blocker = v["scenario"]["initial_conditions"]["items"][0].clone();
            blocker["id"] = json!("existing-pending-slot");
            blocker["item_type_ref"]["id"] = json!("assembly");
            blocker["quality"] = json!("pending");
            blocker["holder"]["id"] = json!("pending-store");
            v["scenario"]["initial_conditions"]["items"]
                .as_array_mut()
                .unwrap()
                .push(blocker);
        }
        "orders" => explicit_order(&mut v),
        "isolation" => {
            v["scenario"]["window"]["end_ms"] = json!(140000);
            v["scenario"]["acceptance_goal"]["observation_window"] =
                v["scenario"]["window"].clone();
            for c in v["project"]["calendars"].as_array_mut().unwrap() {
                c["intervals"][0]["end_ms"] = json!(140000);
            }
            v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
                json!("reinspect");
            v["scenario"]["quality_presets"][2]["results"] =
                json!([null,{"kind":"string","value":"FAIL"},{"kind":"string","value":"FAIL"}]);
        }
        "invalid" => invalid_decisions(&mut v),
        _ => unreachable!(),
    }
    v
}
pub fn edited(name: &str, input: &Value) -> Value {
    let mut v = input.clone();
    let source = fixture();
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
        source["scenario"]["initial_conditions"]["declared_quality_decisions"].clone();
    match name {
        "original" => {
            v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["effective_at_ms"] =
                json!(100000)
        }
        "hold" => {
            v["scenario"]["initial_conditions"]["declared_quality_decisions"][1]["effective_at_ms"] =
                json!(110000)
        }
        "full" => {
            v["project"]["locations"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|l| l["id"] == "pending-store")
                .unwrap()["capacity"]["value"]["coefficient"] = json!(2);
        }
        "orders" => {
            v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
                json!("reinspect");
            v["scenario"]["quality_presets"][2]["results"] =
                json!([null,{"kind":"string","value":"PASS"}]);
        }
        "invalid" => {}
        "isolation" => {
            v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
                input["scenario"]["initial_conditions"]["declared_quality_decisions"].clone();
            v["project"]["locations"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|l| l["id"] == "quarantine")
                .unwrap()["capacity"]["value"]["coefficient"] = json!(2);
        }
        _ => unreachable!(),
    }
    v
}
pub fn invariant(v: &Value, packages: usize, scrap: usize) {
    assert_eq!(original::physical(v, "package").len(), packages);
    let rows = v["final_items"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .filter(|i| i["item_type_ref"]["id"] == "assembly" && i["lifecycle"] == "scrapped")
            .count(),
        scrap
    );
    assert_eq!(rows.iter().filter(|i|i["item_type_ref"]["id"]=="assembly" && i["id"]!="existing-pending-slot").count(),3);
    for kind in ["shell", "pcb", "fastener"] {
        assert!(
            original::physical(v, kind).is_empty(),
            "scrap must not recreate {kind}"
        );
    }
    assert_eq!(original::physical(v, "carton").len(), 3 - packages);
    let mut ids = std::collections::BTreeSet::new();
    for item in rows {
        assert!(ids.insert(item["id"].as_str().unwrap()));
    }
    for package in original::physical(v, "package") {
        assert_eq!(package["contents"].as_array().unwrap().len(), 1);
        for contained in package["contents"].as_array().unwrap() {
            assert_eq!(
                original::named(v, contained.as_str().unwrap())["holder"],
                json!({"kind":"container","id":package["id"]})
            );
        }
    }
}
pub fn evidence(name: &str, v: &Value) {
    let p = out().join("artifacts/kernel");
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join(name), serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
