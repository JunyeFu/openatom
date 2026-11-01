//! Frozen discrete-transport boundary against the existing fixed-scale auxiliary consumer.
//! Original inputs stay read-only; this test derives an explicit mixed-capability variant.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_simulation::{Engine, check_inputs};
use std::{fs, path::Path};

fn mixed_input() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut v: Value = serde_json::from_slice(
        &fs::read(root.join("scenarios/媒体表现/runtime-l03/engine-input.json")).unwrap(),
    )
    .unwrap();
    let cargo: Value = serde_json::from_slice(
        &fs::read(root.join("packages/commands/tests/fixtures/production/p202-identity.json"))
            .unwrap(),
    )
    .unwrap();
    v["scenario"]["scope"] = json!([]);
    v["input_ref"] = cargo["input_ref"].clone();
    v["input_ref"]["project_id"] = v["project"]["id"].clone();
    v["input_ref"]["project_revision"] = v["project"]["revision"].clone();
    v["input_ref"]["scenario_id"] = v["scenario"]["id"].clone();
    v["input_ref"]["scenario_revision"] = v["scenario"]["revision"].clone();
    let source = v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "stock-sealing_tape")
        .unwrap();
    source["capacity"] = json!({"value":{"coefficient":100,"decimal_places":0},"unit":"meter"});
    let mut receiver = source.clone();
    receiver["id"] = json!("night-tape-receiver");
    receiver["title"] = json!("Explicit finite tape receiver");
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .push(receiver);
    let mut carrier = cargo["project"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "T1")
        .unwrap()
        .clone();
    carrier["id"] = json!("night-tape-carrier");
    carrier["calendar_id"] = v["project"]["calendars"][0]["id"].clone();
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(carrier);
    let mut def = cargo["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["payload"]["category"] == "transport")
        .unwrap()
        .clone();
    def["id"] = json!("night.transport.tape");
    def["payload"]["data"]["batch_capacity"]["unit"] = json!("meter");
    for port in def["payload"]["data"]["ports"].as_array_mut().unwrap() {
        port["payload"]["data"]["unit"] = json!("meter");
    }
    def["payload"]["data"]["acceptance_conditions"] = json!([
        {"kind":"capacity","target_role":null,"required_value":null}
    ]);
    v["definitions"].as_array_mut().unwrap().push(def);
    v["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"night.transport.tape","version":"0.1.0"}));
    let mut supply = cargo["scenario"]["supplies"][0].clone();
    supply["id"] = json!("night-tape-supply");
    supply["source"] = json!("stock-sealing_tape");
    supply["destination"] = json!("night-tape-receiver");
    supply["item_type_ref"] = json!({"id":"sealing_tape","revision":"r1"});
    supply["target_quantity"]["unit"] = json!("meter");
    supply["transport_ref"]["id"] = json!("night.transport.tape");
    supply["resources"]["carrier"] = json!("night-tape-carrier");
    v["scenario"]["supplies"]
        .as_array_mut()
        .unwrap()
        .push(supply);
    v
}

#[test]
fn discrete_transport_rejects_fractional_source_instead_of_shipping_coefficients_as_whole_units() {
    let v = mixed_input();
    // Actual source: coefficient 36 / 10 = 3.6 meters. Integer transport may
    // not count coefficient 2 (0.2m) as two whole meters against its 2m goal.
    let before = v.clone();
    let result = check_inputs(&support::input(&v));
    if result.is_ok() {
        let mut engine = Engine::new(support::input(&v)).unwrap();
        engine.run_to_end();
        let actual = serde_json::to_value(engine.snapshot()).unwrap();
        let trips: Vec<_> = actual["transport"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["supply_ref"] == "night-tape-supply")
            .collect();
        let delivered: Vec<_> = actual["final_items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["holder"] == json!({"kind":"location","id":"night-tape-receiver"}))
            .collect();
        panic!(
            "Unsupported mixed-scale transport was admitted: trips={trips:#?}, delivered={delivered:#?}"
        );
    }
    let issues = result.unwrap_err();
    assert!(
        issues
            .iter()
            .any(|i| i.code == "simulation.transport_fractional_stock_scope"
                && i.object_ref
                    .as_ref()
                    .is_some_and(|o| o.id.as_str() == "night-tape-supply")
                && i.field_path.as_deref() == Some("/item_type_ref")),
        "{issues:#?}"
    );
    assert_eq!(
        v, before,
        "Admission must not rewrite either quantity scale"
    );
}

#[test]
fn unrelated_fractional_auxiliary_stock_does_not_block_an_empty_integer_supply() {
    let mut v = mixed_input();
    let supply = v["scenario"]["supplies"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    supply["source"] = json!("night-tape-receiver");
    supply["destination"] = json!("stock-sealing_tape");
    let mut e = Engine::new(support::input(&v)).unwrap();
    e.run_to_end();
    let r = e.snapshot();
    assert!(r.transport.is_empty());
    assert_eq!(
        r.acceptance_events.len(),
        6,
        "Existing fixed-scale sealer consumption must still execute"
    );
}
