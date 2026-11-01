//! Canonical DTO and finite consumer proof, not a production simulation.
use super::contract::RunResult;
use super::api::{Response, RunView};
use serde_json::{Value, json};
use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};

fn number(v: &Value) -> u64 { v.as_u64().unwrap() }

fn valid_execution(v: &Value) -> bool {
    match v["kind"].as_str().unwrap() {
        "definition_cycle" => true,
        "project_device" => {
            let cap = &v["data"]["execution_capacity_ref"];
            match cap["kind"].as_str().unwrap() {
                "device" => cap["id"] == v["data"]["device_id"],
                "resource" => true,
                _ => false,
            }
        },
        _ => false,
    }
}

fn valid_segments(run: &Value) -> bool {
    let mut previous: BTreeMap<(String, String), (u64, u64)> = BTreeMap::new();
    let jobs = run["jobs"].as_array().unwrap();
    if jobs.iter().any(|j| !valid_execution(&j["execution_ref"])) { return false; }
    for claim in run["resource_claims"].as_array().unwrap() {
        if claim["kind"] == "phase" && claim["phase_ref"].is_null() { return false; }
        let intervals = claim["intervals"].as_array().unwrap();
        let causes = claim["interval_end_causes"].as_array().unwrap();
        if intervals.len() != causes.len() { return false; }
        for (interval, cause) in intervals.iter().zip(causes) {
            let Some(frontier) = run["computed_until_ms"].as_u64() else { return false; };
            let end = number(&interval["end_ms"]);
            if end > frontier { return false; }
            if cause == "computed_boundary" && end != frontier { return false; }
            if cause == "observation_end" &&
                (run["whole_window_computed"] != true || end != number(&run["window"]["end_ms"])) { return false; }
        }
    }
    for wait in run["waits"].as_array().unwrap() {
        if !wait["execution_ref"].is_null() && !valid_execution(&wait["execution_ref"]) { return false; }
        let Some(frontier) = run["computed_until_ms"].as_u64() else { return false; };
        let end = number(&wait["interval"]["end_ms"]);
        if end > frontier { return false; }
        if wait["end_cause"] == "computed_boundary" && end != frontier { return false; }
        if wait["end_cause"] == "observation_end" &&
            (run["whole_window_computed"] != true || end != number(&run["window"]["end_ms"])) { return false; }
    }
    for part in run["phase_work_intervals"].as_array().unwrap() {
        let start = number(&part["interval"]["start_ms"]);
        let end = number(&part["interval"]["end_ms"]);
        let p0 = number(&part["effective_progress_start_ms"]);
        let p1 = number(&part["effective_progress_end_ms"]);
        let Some(frontier) = run["computed_until_ms"].as_u64() else { return false; };
        if end <= start || end > frontier || start < number(&run["window"]["start_ms"]) ||
            end > number(&run["window"]["end_ms"]) || p1 < p0 || p1 - p0 != end - start { return false; }
        if !jobs.iter().any(|j| j["id"] == part["job_id"]) { return false; }
        let key = (part["job_id"].as_str().unwrap().to_owned(), part["phase_id"].as_str().unwrap().to_owned());
        if let Some((old_end, old_progress)) = previous.get(&key) {
            if start < *old_end || p0 != *old_progress { return false; }
        }
        previous.insert(key, (end, p1));
        match part["end_cause"].as_str().unwrap() {
            "computed_boundary" if end != frontier => return false,
            "phase_completed" => {
                if !run["events"].as_array().unwrap().iter().any(|event|
                    number(&event["at_ms"]) == end && event["subject"]["kind"] == "job" &&
                    event["subject"]["id"] == part["job_id"] &&
                    event["payload"]["kind"] == "phase_completed" && event["payload"]["data"] == part["phase_id"]
                ) { return false; }
            },
            _ => {},
        }
    }
    true
}

fn metrics(run: &Value) -> Value {
    let mut rows = BTreeMap::new();
    let mut occupied = BTreeSet::new();
    for job in run["jobs"].as_array().unwrap() {
        let ex = &job["execution_ref"];
        if ex["kind"] != "project_device" { continue; }
        let device = ex["data"]["device_id"].as_str().unwrap();
        rows.insert(job["id"].as_str().unwrap(), device);
        // Match the explicitly frozen capacity identity for this job, never an arbitrary person claim.
        if run["resource_claims"].as_array().unwrap().iter().any(|claim|
            claim["job_id"] == job["id"] && claim["resource_id"] == ex["data"]["execution_capacity_ref"]["id"] &&
            claim["intervals"].as_array().unwrap().iter().any(|i| number(&i["end_ms"]) > number(&i["start_ms"]))
        ) { occupied.insert(device); }
    }
    let work: u64 = run["phase_work_intervals"].as_array().unwrap().iter()
        .map(|p| number(&p["interval"]["end_ms"]) - number(&p["interval"]["start_ms"])).sum();
    // Case-scope union, not a sum of overlapping wait reasons or a factory-wide cost metric.
    let mut waits: Vec<(u64, u64)> = run["waits"].as_array().unwrap().iter()
        .map(|w| (number(&w["interval"]["start_ms"]), number(&w["interval"]["end_ms"]))).collect();
    waits.sort();
    let (mut wait_ms, mut covered_until) = (0, 0);
    for (start, end) in waits {
        wait_ms += end.saturating_sub(start.max(covered_until));
        covered_until = covered_until.max(end);
    }
    let points = run["events"].as_array().unwrap().iter()
        .filter(|e| e["payload"]["kind"] == "phase_completed").count();
    json!({"job_device_rows": rows, "occupied_devices": occupied, "effective_work_ms": work,
        "wait_ms": wait_ms, "phase_completed_points": points})
}

pub fn run(root: &Path) {
    let cases: Value = serde_json::from_slice(&fs::read(root.join("timeline-contract-cases.design.json")).unwrap()).unwrap();
    let response_template: Value = serde_json::from_slice(&fs::read(root.join("api-fixtures/get_run.response.json")).unwrap()).unwrap();
    let mut results = Vec::new();
    for case in cases["cases"].as_array().unwrap() {
        let parsed = serde_json::from_value::<RunResult>(case["run"].clone());
        let shape = parsed.is_ok();
        assert_eq!(shape, case["expected_shape_accepted"].as_bool().unwrap(), "{} shape", case["id"]);
        let mut response = response_template.clone();
        response["outcome"]["data"] = json!({"kind":"full", "data":case["run"]});
        let api_parsed = serde_json::from_value::<Response<RunView>>(response.clone());
        assert_eq!(api_parsed.is_ok(), shape, "{} FullResult API shape", case["id"]);
        if let Ok(typed) = api_parsed { assert_eq!(serde_json::to_value(typed).unwrap(), response); }
        let mut semantic = false;
        let mut projection = Value::Null;
        if let Ok(typed) = parsed {
            let encoded = serde_json::to_value(typed).unwrap();
            assert_eq!(encoded, case["run"], "{} typed roundtrip", case["id"]);
            semantic = valid_segments(&encoded);
            if semantic {
                projection = metrics(&encoded);
                assert_eq!(projection, case["expected_metrics"], "{} finite projection", case["id"]);
            }
        }
        assert_eq!(semantic, case["expected_semantics_accepted"].as_bool().unwrap(), "{} semantics", case["id"]);
        results.push(json!({"id": case["id"], "shape_accepted": shape, "full_result_API_shape_accepted":shape, "finite_semantics_accepted": semantic, "projection": projection}));
    }
    fs::write(root.join("output/timeline-contract-proof-result.json"), serde_json::to_vec_pretty(&json!({
        "status":"passed_actual_canonical_timeline_DTO_and_finite_consumer_proof", "cases":results,
        "Engine_or_Host_executed":false, "actual_product_runtime":null
    })).unwrap()).unwrap();
}
