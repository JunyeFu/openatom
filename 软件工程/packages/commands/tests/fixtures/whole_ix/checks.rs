//! Assertions over actual public Host outputs; no scheduler or alternate execution.
use serde_json::{Value, json};

pub fn expected(family: &str, name: &str) -> Value {
    let frozen: Value = serde_json::from_str(include_str!("original-expectations.json")).unwrap();
    frozen[family]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["scenario"] == name || case["id"] == name)
        .unwrap()
        .clone()
}
pub fn jobs(run: &Value, operation: &str) -> Vec<Value> {
    run["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| j["operation_ref"]["id"] == operation)
        .cloned()
        .collect()
}
pub fn intervals(run: &Value, resource: &str) -> Vec<(u64, u64)> {
    let mut spans: Vec<_> = run["resource_claims"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["resource_id"] == resource)
        .flat_map(|c| {
            assert_eq!(
                c["intervals"].as_array().unwrap().len(),
                c["interval_end_causes"].as_array().unwrap().len()
            );
            c["intervals"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| {
                    (
                        s["start_ms"].as_u64().unwrap(),
                        s["end_ms"].as_u64().unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    spans.sort();
    spans
}
pub fn physical(run: &Value, kind: &str) -> Vec<Value> {
    run["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| {
            i["item_type_ref"]["id"] == kind
                && i["lifecycle"] != "transformed"
                && i["lifecycle"] != "scrapped"
        })
        .cloned()
        .collect()
}
fn ancestors(run: &Value, item: &Value) -> std::collections::BTreeSet<String> {
    let mut found = std::collections::BTreeSet::new();
    let mut pending = vec![item["id"].as_str().unwrap().to_owned()];
    while let Some(id) = pending.pop() {
        if !found.insert(id.clone()) {
            continue;
        }
        let actual = run["final_items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == id)
            .unwrap();
        pending.extend(
            actual["composition_refs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap().to_owned()),
        );
        pending.extend(
            actual["contents"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap().to_owned()),
        );
        if let Some(parent) = actual["split_from_ref"].as_str() {
            pending.push(parent.to_owned());
        }
    }
    found
}
pub fn original_materials(run: &Value, input: &Value, per_package: &[(&str, usize)]) {
    let mut originals = input["scenario"]["initial_conditions"]["items"]
        .as_array()
        .unwrap()
        .clone();
    for arrival in input["scenario"]["arrivals"].as_array().unwrap() {
        originals.extend(arrival["items"].as_array().unwrap().clone());
    }
    let mut consumed = std::collections::BTreeMap::<String, usize>::new();
    for package in physical(run, "package") {
        assert_eq!(package["quality"], "released");
        assert_eq!(package["quantity"]["value"]["coefficient"], 1);
        assert_eq!(package["quantity"]["unit"], "piece");
        let ids = ancestors(run, &package);
        for (kind, count) in per_package {
            let members: Vec<_> = originals
                .iter()
                .filter(|i| {
                    i["item_type_ref"]["id"] == *kind && ids.contains(i["id"].as_str().unwrap())
                })
                .collect();
            assert_eq!(members.len(), *count, "package original {kind} composition");
            for member in members {
                *consumed
                    .entry(member["id"].as_str().unwrap().to_owned())
                    .or_default() += 1;
            }
        }
    }
    for original in originals {
        assert_eq!(
            consumed.get(original["id"].as_str().unwrap()),
            Some(&1),
            "each declared physical input is used once, without replacement stock"
        );
    }
}
fn spans(source: &Value, kinds: &[&str]) -> Vec<(u64, u64)> {
    let mut v: Vec<_> = source["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| kinds.iter().any(|k| j["kind"] == *k))
        .map(|j| {
            (
                j["start_ms"].as_u64().unwrap(),
                j["end_ms"].as_u64().unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}
fn actual_spans(run: &Value, ops: &[&str], production: bool) -> Vec<(u64, u64)> {
    let mut v: Vec<_> = ops
        .iter()
        .flat_map(|op| jobs(run, op))
        .filter(|j| !production || !j["input_refs"].as_array().unwrap().is_empty())
        .map(|j| {
            (
                j["started_at_ms"].as_u64().unwrap(),
                j["finished_at_ms"].as_u64().unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}
pub fn common(run: &Value) {
    assert_eq!(run["evidence_origin"], "actual_software");
    assert_eq!(run["computation_status"], "completed");
    assert_eq!(run["whole_window_computed"], true);
    assert_eq!(run["computed_until_ms"], run["window"]["end_ms"]);
    for pair in run["events"].as_array().unwrap().windows(2) {
        assert!(pair[0]["sequence"].as_u64().unwrap() < pair[1]["sequence"].as_u64().unwrap());
        assert!(pair[0]["at_ms"].as_u64().unwrap() <= pair[1]["at_ms"].as_u64().unwrap());
    }
}
pub fn ix01(run: &Value, name: &str) {
    common(run);
    let e = expected("ix01", name);
    assert_eq!(physical(run, "package").len(), 6);
    let mut ends: Vec<_> = jobs(run, "w5.op.pack")
        .iter()
        .map(|j| j["finished_at_ms"].as_u64().unwrap())
        .collect();
    ends.sort();
    assert_eq!(
        ends,
        e["units"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["completed_ms"].as_u64().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        run["acceptance_events"].as_array().unwrap().last().unwrap()["at_ms"],
        e["expected_completion_ms"]
    );
    for (op, kind) in [
        ("w5.op.assembly", "assembly"),
        ("w5.op.rework", "rework"),
        ("w5.op.pack", "packing"),
    ] {
        assert_eq!(actual_spans(run, &[op], false), spans(&e, &[kind]));
    }
    let work = |operation: &str, phase: Option<&str>| {
        let ids: Vec<_> = jobs(run, operation)
            .iter()
            .map(|j| j["id"].clone())
            .collect();
        let mut v: Vec<_> = run["phase_work_intervals"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|w| ids.contains(&w["job_id"]) && phase.is_none_or(|p| w["phase_id"] == p))
            .map(|w| {
                (
                    w["interval"]["start_ms"].as_u64().unwrap(),
                    w["interval"]["end_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        v.sort();
        v
    };
    for (phase, kind) in [
        ("load", "cnc_load"),
        ("auto", "cnc_auto"),
        ("unload", "cnc_unload"),
    ] {
        assert_eq!(work("w5.op.shell", Some(phase)), spans(&e, &[kind]));
    }
    assert_eq!(
        work("w5.op.inspect", None),
        spans(&e, &["inspection", "reinspection"])
    );
    assert_eq!(intervals(run, "CAP-CNC01"), spans(&e, &["cnc_machine"]));
    for worker in ["W-A", "W-B", "W-C"] {
        let mut v: Vec<_> = e["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| {
                j["resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r == worker)
            })
            .map(|j| {
                (
                    j["start_ms"].as_u64().unwrap(),
                    j["end_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        v.sort();
        assert_eq!(intervals(run, worker), v);
    }
    let mut transfers = run["transport"].as_array().unwrap().clone();
    transfers.sort_by_key(|t| t["departed_ms"].as_u64());
    assert_eq!(transfers.len(), 6);
    for (t, e) in transfers.iter().zip(e["transfers"].as_array().unwrap()) {
        for (a, b) in [
            ("departed_ms", "depart_ms"),
            ("arrived_ms", "arrival_ms"),
            ("received_ms", "accepted_ms"),
            ("returned_ms", "resource_available_ms"),
        ] {
            assert_eq!(t[a], e[b]);
        }
        assert_eq!(t["cargo"].as_array().unwrap().len(), 1);
    }
    if name == "delayed_pcb" {
        assert!(
            run["waits"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |w| w["interval"] == json!({"start_ms":370000,"end_ms":420000})
                        && w["end_cause"] == "resolved"
                )
        );
        assert_eq!(transfers[5]["returned_ms"], 425000);
    }
}
pub fn ix02(run: &Value, name: &str) {
    common(run);
    let e = expected("ix02", name);
    assert_eq!(run["goal_met"], true);
    assert_eq!(physical(run, "package").len(), 4);
    assert_eq!(
        intervals(run, "W-A")
            .iter()
            .map(|(a, b)| b - a)
            .sum::<u64>(),
        e["expected_worker_a_occupied_ms"].as_u64().unwrap()
    );
    assert_eq!(
        run["acceptance_events"].as_array().unwrap().last().unwrap()["at_ms"],
        205000
    );
    for order in e["expected_orders"].as_array().unwrap() {
        let a = run["orders"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["order_id"] == order["order"])
            .unwrap();
        assert_eq!(a["completion_ms"], order["completion_ms"]);
        assert_eq!(a["tardiness_ms"], order["tardiness_ms"]);
        assert_eq!(a["accepted"]["value"]["coefficient"], 2);
        assert_eq!(a["outstanding"]["value"]["coefficient"], 0);
    }
    for (ops, kind) in [
        (vec!["w5.op.assembly-A", "w5.op.assembly-B"], "assembly"),
        (vec!["w5.op.inspect"], "inspection"),
        (vec!["w5.op.pack"], "packing"),
    ] {
        assert_eq!(actual_spans(run, &ops, true), spans(&e, &[kind]));
    }
    let mut preparation: Vec<_> = ["w5.op.assembly-A", "w5.op.assembly-B"]
        .iter()
        .flat_map(|op| jobs(run, op))
        .filter(|j| j["input_refs"].as_array().unwrap().is_empty())
        .map(|j| {
            (
                j["started_at_ms"].as_u64().unwrap(),
                j["finished_at_ms"].as_u64().unwrap(),
            )
        })
        .collect();
    preparation.sort();
    assert_eq!(preparation, spans(&e, &["setup"]));
    let mut ts = run["transport"].as_array().unwrap().clone();
    ts.sort_by_key(|t| t["departed_ms"].as_u64());
    assert_eq!(ts.len(), 2);
    for (t, depart, arrive, receive, returned) in [
        (&ts[0], 120000, 140000, 145000, 165000),
        (&ts[1], 180000, 200000, 205000, 225000),
    ] {
        assert_eq!(t["departed_ms"], depart);
        assert_eq!(t["arrived_ms"], arrive);
        assert_eq!(t["received_ms"], receive);
        assert_eq!(t["returned_ms"], returned);
        assert_eq!(t["cargo"].as_array().unwrap().len(), 2);
    }
    for r in ["W-A", "W-B", "T1"] {
        for pair in intervals(run, r).windows(2) {
            assert!(pair[0].1 <= pair[1].0);
        }
    }
}
