#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_simulation::Engine;
use support::{held_ms as held, input, run};
fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../commands/tests/fixtures/production")
                .join(format!("{name}.json")),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn full_ix01_frozen_six_packages_and_true_cart_wait() {
    for (case, last) in [("baseline", 440000), ("delayed-pcb", 570000)] {
        let r = run(&fixture(&format!("p202-ix01-{case}")));
        assert_eq!(r["goal_met"], true, "{r:#}");
        assert_eq!(
            r["acceptance_events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["at_ms"].as_u64().unwrap())
                .max(),
            Some(last)
        );
        assert_eq!(r["transport"].as_array().unwrap().len(), 6);
        if case == "delayed-pcb" {
            let trip = r["transport"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["arrived_ms"] == 370000)
                .unwrap();
            assert_eq!(trip["received_ms"], 420000);
            assert_eq!(trip["returned_ms"], 425000);
        }
    }
}

#[test]
fn concurrent_completed_unloads_recheck_capacity_without_duplicating_cargo_or_freeing_carriers() {
    let mut v = fixture("p202-identity");
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "T1")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(2);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "cargo-destination")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(1);
    let r = run(&v);
    let trips = r["transport"].as_array().unwrap();
    assert_eq!(trips.len(), 2);
    assert_eq!(
        trips.iter().filter(|t| t["received_ms"] == 50000).count(),
        1
    );
    assert_eq!(
        trips.iter().filter(|t| t["received_ms"].is_null()).count(),
        1
    );
    assert_eq!(
        trips.iter().filter(|t| t["returned_ms"] == 80000).count(),
        1
    );
    let physical: Vec<_> = r["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["lifecycle"] != "transformed")
        .collect();
    assert_eq!(physical.len(), 2);
    assert_eq!(
        physical
            .iter()
            .filter(|i| i["holder"]["kind"] == "transport")
            .count(),
        1
    );
    assert_eq!(held(&r, "T1"), 80000 + 240000);
}

#[test]
fn partial_batch_splits_once_and_reservation_stays_in_source_until_real_departure() {
    let mut v = fixture("p202-identity");
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    v["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"] = json!(3);
    v["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "assembly")
        .unwrap()["tracking"] = json!("batch");
    v["scenario"]["supplies"][0]["target_quantity"]["value"]["coefficient"] = json!(3);
    v["scenario"]["supplies"][0]["allow_partial_load"] = json!(true);
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["payload"]["category"] == "transport")
        .unwrap()["payload"]["data"]["batch_capacity"]["value"]["coefficient"] = json!(2);
    let mut e = Engine::new(input(&v)).unwrap();
    e.step();
    let prefix = serde_json::to_value(e.snapshot()).unwrap();
    assert!(
        prefix["final_items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["lifecycle"] != "transformed")
            .all(|i| i["holder"] == json!({"kind":"location","id":"cargo-source"}))
    );
    assert_eq!(prefix["allocations"].as_array().unwrap().len(), 1);
    e.run_to_end();
    let r = serde_json::to_value(e.snapshot()).unwrap();
    assert_eq!(r["transport"].as_array().unwrap().len(), 2);
    let physical: Vec<_> = r["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["lifecycle"] != "transformed")
        .collect();
    assert_eq!(
        physical
            .iter()
            .map(|i| i["quantity"]["value"]["coefficient"].as_u64().unwrap())
            .sum::<u64>(),
        3
    );
    assert!(
        physical
            .iter()
            .all(|i| i["holder"] == json!({"kind":"location","id":"cargo-destination"}))
    );
    assert!(r["allocations"].as_array().unwrap().is_empty());
}

#[test]
fn missing_transport_duration_is_rejected_and_closed_zero_loading_never_takes_resources() {
    let mut missing = fixture("p202-identity");
    missing["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["payload"]["category"] == "transport")
        .unwrap()["payload"]["data"]["times"]["loading"] = Value::Null;
    assert!(Engine::new(input(&missing)).is_err());
    let mut v = fixture("p202-identity");
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["payload"]["category"] == "transport")
        .unwrap()["payload"]["data"]["times"]["loading"]["value"] = json!(0);
    let calendar = v["project"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "T1")
        .unwrap()["calendar_id"]
        .clone();
    v["project"]["calendars"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["id"] == calendar)
        .unwrap()["intervals"] = json!([]);
    let r = run(&v);
    assert!(r["transport"].as_array().unwrap().is_empty());
    assert_eq!(held(&r, "T1"), 0);
}
