#![allow(dead_code)]
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
pub mod original;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn operation(v: &mut Value) -> &mut Value {
    &mut v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.phased")
        .unwrap()["payload"]["data"]
}
pub fn combined(policy: &str, output_capacity: u64) -> Value {
    let mut v = original::phased();
    let mut incoming = v["scenario"]["initial_conditions"]["items"][0].clone();
    incoming["id"] = json!("noncu-future-raw");
    v["scenario"]["initial_conditions"]["items"] = json!([]);
    v["scenario"]["arrivals"] = json!([{"id":"noncu-arrival","at_ms":10000,"destination":"raw-store","items":[incoming],"release_policy":"unchecked_allowed"}]);
    for c in v["project"]["calendars"].as_array_mut().unwrap() {
        c["intervals"] = if c["id"] == "ALL" {
            json!([{"start_ms":0,"end_ms":20000},{"start_ms":40000,"end_ms":100000}])
        } else {
            json!([{"start_ms":0,"end_ms":20000},{"start_ms":80000,"end_ms":100000}])
        };
    }
    for l in v["project"]["locations"].as_array_mut().unwrap() {
        if l["id"] == "finished" {
            l["capacity"]["value"]["coefficient"] = json!(output_capacity);
        }
    }
    operation(&mut v)["phases"][1]["interruption_policy"] = json!(policy);
    v
}
pub fn work(v: &Value) -> Vec<(String, u64, u64)> {
    v["phase_work_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["phase_id"].as_str().unwrap().to_owned(),
                w["interval"]["start_ms"].as_u64().unwrap(),
                w["interval"]["end_ms"].as_u64().unwrap(),
            )
        })
        .collect()
}
pub fn expected_work(policy: &str, unload_at: u64) -> Vec<(String, u64, u64)> {
    let mut rows = vec![("load".into(), 10000, 15000)];
    if policy == "pause_resume" {
        rows.extend([("auto".into(), 15000, 20000), ("auto".into(), 40000, 65000)]);
    } else {
        rows.push(("auto".into(), 40000, 70000));
    }
    rows.push(("unload".into(), unload_at, unload_at + 5000));
    rows
}
pub fn arrival_count(v: &Value) -> usize {
    v["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            e["payload"]["kind"] == "arrival_accepted" && e["payload"]["data"] == "noncu-arrival"
        })
        .count()
}
pub fn evidence(name: &str, v: &Value) {
    let dir = root().join(".work/delivery/artifacts/kernel");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
