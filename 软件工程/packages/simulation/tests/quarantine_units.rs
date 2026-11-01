use serde_json::{Value, json};
use sf_simulation::{Engine, EngineInput};

fn source(file: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../scenarios/真实业务设计")
                .join(file),
        )
        .unwrap(),
    )
    .unwrap()
}
fn quality_case(file: &str) -> Value {
    source(file)["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "quality_pending_and_scrap")
        .unwrap()
        .clone()
}
fn q(n: u64) -> Value {
    json!({"value":{"coefficient":n,"decimal_places":0},"unit":"piece"})
}
fn stable(tag: &str, parts: &[&str]) -> String {
    format!(
        "{tag}:{}",
        parts
            .iter()
            .map(|p| format!("{}:{p}", p.len()))
            .collect::<String>()
    )
}
fn assembly(cycle: u64) -> String {
    stable(
        "output",
        &[
            &stable("job", &["TASK-LOCAL", &cycle.to_string(), "step-assembly"]),
            "assembled",
            "0",
        ],
    )
}
fn decision(cycle: u64, at: u64, kind: &str) -> Value {
    json!({"id":format!("quality-{cycle}-{at}-{kind}"),"item_ref":assembly(cycle),"inspection_ref":{"id":"w5.op.inspect","version":"0.1.0"},"source_kind":"manual","observed_result":null,"disposition":kind,"effective_at_ms":at,"decision_source":{"kind":"human_statement","data":"self-built scheduled decision; no live human action claimed"}})
}
// Formal projection of the frozen local-quality case. The existing canonical
// DTO template provides only field shapes and exact selected module references.
fn fixture(hold: bool) -> Value {
    let s = quality_case("business-inputs.design.json");
    let mut v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/p202-ix01-baseline.json"
    ))
    .unwrap();
    v["scenario"]["window"]["end_ms"] = s["time"]["observation_end_ms"].clone();
    v["scenario"]["supplies"] = json!([]);
    v["scenario"]["arrivals"] = json!([]);
    v["scenario"]["orders"] = json!([]);
    v["scenario"]["production_tasks"][0]["planned_quantity"] = q(s["demand"].as_u64().unwrap());
    v["scenario"]["production_tasks"][0]["order_allocations"] = json!([]);
    v["project"]["setup_policies"] = json!([]);
    v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .retain(|d| d["id"] != "CNC01");
    v["project"]["connections"]
        .as_array_mut()
        .unwrap()
        .retain(|c| {
            !["source", "destination"].iter().any(|field| {
                c["data"][field]["kind"] == "port"
                    && c["data"][field]["data"]["instance_id"] == "CNC01"
            })
        });
    for route in v["project"]["routes"].as_array_mut().unwrap() {
        route["steps"]
            .as_array_mut()
            .unwrap()
            .retain(|step| step["id"] != "step-shell");
        for step in route["steps"].as_array_mut().unwrap() {
            step["predecessors"]
                .as_array_mut()
                .unwrap()
                .retain(|id| id != "step-shell");
            match step["device_id"].as_str().unwrap() {
                "Inspection01" => step["resource_bindings"]["worker"] = json!("W-Q"),
                "Packing01" => step["resource_bindings"]["worker"] = json!("W-P"),
                _ => {}
            }
        }
    }
    for bom in v["project"]["boms"].as_array_mut().unwrap() {
        bom["rows"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["consume_at_operation"] != "step-shell");
    }
    let worker = v["project"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "W-B")
        .unwrap()
        .clone();
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["id"] != "W-B");
    for name in ["W-Q", "W-P"] {
        let mut r = worker.clone();
        r["id"] = json!(name);
        v["project"]["resources"].as_array_mut().unwrap().push(r);
    }
    for calendar in v["project"]["calendars"].as_array_mut().unwrap() {
        calendar["intervals"] = json!([{"start_ms":0,"end_ms":s["time"]["observation_end_ms"]}]);
    }
    for device in v["project"]["devices"].as_array_mut().unwrap() {
        match device["id"].as_str().unwrap() {
            "Inspection01" => {
                device["role_bindings"]["resources"]["worker"] = json!("W-Q");
                device["role_bindings"]["targets"]["pending"] =
                    json!({"kind":"location","id":"pending-store"});
            }
            "Packing01" => device["role_bindings"]["resources"]["worker"] = json!("W-P"),
            _ => {}
        }
    }
    let mut pending = v["project"]["locations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == "quarantine")
        .unwrap()
        .clone();
    pending["id"] = json!("pending-store");
    pending["purpose"] = json!("pending");
    pending["capacity"] = q(s["pending_capacity"].as_u64().unwrap());
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .push(pending);
    for location in v["project"]["locations"].as_array_mut().unwrap() {
        let cap = match location["id"].as_str().unwrap() {
            "quarantine" => 1,
            "pending-store" => 1,
            "fastener-store" => 12,
            _ => 3,
        };
        location["capacity"] = q(cap);
    }
    let mut items = vec![];
    for (kind, location) in [
        ("shell", "shell-buffer"),
        ("pcb", "pcb-store"),
        ("fastener", "fastener-store"),
        ("carton", "carton-store"),
    ] {
        for n in 1..=s["initial_stock"][kind].as_u64().unwrap() {
            items.push(json!({"id":format!("{kind}{n:03}"),"item_type_ref":{"id":kind,"revision":"r1"},"quantity":q(1),"holder":{"kind":"location","id":location},"quality":"released","components":[],"contents":[],"source":"user_provided"}));
        }
    }
    v["scenario"]["initial_conditions"]["items"] = json!(items);
    for definition in v["definitions"].as_array_mut().unwrap() {
        let (key, field) = match definition["id"].as_str().unwrap() {
            "w5.op.assembly" => ("assembly", "duration"),
            "w5.op.inspect" => ("inspection", "duration"),
            "w5.op.rework" => ("rework", "duration"),
            "w5.op.pack" => ("packing", "duration"),
            _ => continue,
        };
        let op = &mut definition["payload"]["data"];
        op[field]["value"] = s["durations_ms"][key].clone();
        if key == "inspection" {
            op["quality_and_routes"]["max_completed_reworks"] = s["max_reworks"].clone();
            op["quality_and_routes"]["pending_policy"] = json!(if hold {
                "hold_station"
            } else {
                "pending_store"
            });
            op["quality_and_routes"]["pending_location_role"] =
                if hold { Value::Null } else { json!("pending") };
        }
    }
    v["scenario"]["quality_presets"]=json!((1..=3).map(|n|json!({"id":format!("QC-{n}"),"item_or_ancestor":format!("shell{n:03}"),"match_kind":"ancestor","inspection_module":{"id":"w5.op.inspect","version":"0.1.0"},"results":s["quality_presets"][format!("assembly{n}")].as_array().unwrap().iter().map(|r|if r=="UNKNOWN"{Value::Null}else{json!({"kind":"string","value":r})}).collect::<Vec<_>>()})).collect::<Vec<_>>());
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
        json!([decision(2, 90000, "release"), decision(1, 100000, "scrap")]);
    v["scenario"]["acceptance_goal"] = json!({"industry":"declared quality case","product_family":"control-box","product_specification_ref":null,"accepted_quantity_per_day":q(3),"acceptance_scope":{"product_ref":{"id":"PACKAGE","revision":"r1"},"unit":"piece","destination":"finished","stage":"factory_released"},"production_calendar_ref":"ALL","observation_window":v["scenario"]["window"],"site_envelope":{"regions":[],"length_m":null,"width_m":null,"height_m":null,"usable_polygon_m":[],"clearance_m":null},"existing_equipment":[],"constraints":[]});
    v
}
fn input(v: &Value) -> EngineInput {
    EngineInput {
        run_id: serde_json::from_value(json!("P105-owner")).unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        scenario: serde_json::from_value(v["scenario"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: vec![],
    }
}
fn engine(v: &Value) -> Engine {
    Engine::new(input(v)).unwrap_or_else(|issues| panic!("{issues:#?}"))
}
fn run(v: &Value) -> Value {
    let mut e = engine(v);
    e.run_to_end();
    serde_json::to_value(e.snapshot()).unwrap()
}
fn item<'a>(r: &'a Value, id: &str) -> &'a Value {
    r["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == id)
        .unwrap()
}

#[test]
fn frozen_pending_and_scrap_has_two_packages_one_scrapped_identity_and_one_unmet_goal() {
    let expected = quality_case("expected-business.design.json");
    let mut e = engine(&fixture(false));
    e.run_to_end();
    let r = serde_json::to_value(e.snapshot()).unwrap();
    assert_eq!(r["computation_status"], "completed");
    assert_eq!(r["goal_met"], false);
    assert!(r["orders"].as_array().unwrap().is_empty());
    assert_eq!(
        r["acceptance_events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["quantity"]["value"]["coefficient"].as_u64().unwrap())
            .sum::<u64>(),
        expected["expected_order"]["accepted_at_factory_release"]
            .as_u64()
            .unwrap()
    );
    let packages = r["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["item_type_ref"]["id"] == "package" && i["lifecycle"] == "available")
        .count();
    assert_eq!(packages, 2);
    for (operation, kinds) in [
        ("w5.op.assembly", vec!["assembly"]),
        ("w5.op.inspect", vec!["inspection", "reinspection"]),
        ("w5.op.rework", vec!["rework"]),
        ("w5.op.pack", vec!["packing"]),
    ] {
        let mut actual: Vec<_> = r["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| {
                j["operation_ref"]["id"] == operation
                    && !j["input_refs"].as_array().unwrap().is_empty()
            })
            .map(|j| {
                (
                    j["started_at_ms"].as_u64().unwrap(),
                    j["finished_at_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        let mut frozen: Vec<_> = expected["jobs"]
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
        actual.sort();
        frozen.sort();
        assert_eq!(actual, frozen, "{operation}");
    }
    assert_eq!(
        r["acceptance_events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["at_ms"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![40000, 100000]
    );
    let discarded = item(&r, &assembly(1));
    assert_eq!(discarded["lifecycle"], "scrapped");
    assert_eq!(
        discarded["holder"],
        json!({"kind":"location","id":"quarantine"})
    );
    assert_eq!(discarded["completed_reworks"], 1);
    assert_eq!(discarded["composition_refs"].as_array().unwrap().len(), 6);
    assert_eq!(item(&r, "carton003")["lifecycle"], "available");
    for kind in ["shell", "pcb", "fastener"] {
        assert!(
            !r["final_items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["item_type_ref"]["id"] == kind && i["lifecycle"] == "available")
        );
    }
    let at80 = serde_json::to_value(
        e.objects_at(serde_json::from_value(json!(80000)).unwrap())
            .unwrap(),
    )
    .unwrap();
    let pending = at80
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == assembly(2))
        .unwrap();
    assert_eq!(pending["quality"], "pending");
    assert_eq!(
        pending["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    let rechecking = at80
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == assembly(1))
        .unwrap();
    assert_eq!(rechecking["lifecycle"], "in_process");
    assert_eq!(rechecking["holder"]["kind"], "job");
    let old = r["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| {
            j["operation_ref"]["id"] == "w5.op.inspect" && j["input_refs"] == json!([assembly(2)])
        })
        .unwrap();
    assert_eq!(old["started_at_ms"], 60000);
    assert_eq!(old["finished_at_ms"], 70000);
}

#[test]
fn hold_station_and_full_pending_store_keep_actual_item_and_declared_claims_until_a_real_transfer()
{
    for full in [false, true] {
        let mut v = fixture(!full);
        v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
        if full {
            v["project"]["locations"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|l| l["id"] == "pending-store")
                .unwrap()["capacity"] = q(0);
        }
        let r = run(&v);
        let pending = item(&r, &assembly(2));
        assert_eq!(pending["quality"], "pending");
        assert_eq!(pending["holder"]["kind"], "job");
        assert!(r["resource_claims"].as_array().unwrap().iter().any(|c| {
            c["resource_id"] == "W-Q"
                && c["intervals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|i| i["end_ms"] == 120000)
        }));
        assert_eq!(
            r["final_items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["lifecycle"] == "scrapped")
                .count(),
            0
        );
    }
}

#[test]
fn a_postcheck_release_waits_at_the_same_pending_holder_without_reopening_the_finished_inspection()
{
    let mut v = fixture(false);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "passed-in")
        .unwrap()["capacity"] = q(1);
    // A real declared calendar delays packing, so the first released assembly
    // occupies the sole pass receiver when the later release arrives.
    let calendar = json!({"id":"late-pack","title":"late-pack","source":"user_provided","intervals":[{"start_ms":110000,"end_ms":120000}]});
    v["project"]["calendars"]
        .as_array_mut()
        .unwrap()
        .push(calendar);
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "W-P")
        .unwrap()["calendar_id"] = json!("late-pack");
    let mut e = engine(&v);
    e.run_to_end();
    let r = serde_json::to_value(e.snapshot()).unwrap();
    let at100 = serde_json::to_value(
        e.objects_at(serde_json::from_value(json!(100000)).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        at100
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == assembly(2))
            .unwrap()["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    assert!(
        r["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |w| w["object_ref"] == json!({"kind":"item","id":assembly(2)})
                    && w["interval"] == json!({"start_ms":90000,"end_ms":110000})
            )
    );
    let check = r["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| {
            j["operation_ref"]["id"] == "w5.op.inspect" && j["input_refs"] == json!([assembly(2)])
        })
        .unwrap();
    assert_eq!(check["finished_at_ms"], 70000);
}

#[test]
fn explicit_reinspection_waits_for_real_resources_and_hands_the_same_id_over_atomically() {
    for hold in [false, true] {
        let mut v = fixture(hold);
        v["scenario"]["quality_presets"][1]["results"] = json!([{"kind":"string","value":"PASS"}]);
        v["scenario"]["quality_presets"][2]["results"] =
            json!([null,{"kind":"string","value":"PASS"}]);
        v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
            json!([decision(2, 90000, "reinspect")]);
        v["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"QC-shift","title":"QC-shift","source":"user_provided","intervals":[{"start_ms":0,"end_ms":80000},{"start_ms":100000,"end_ms":120000}]}));
        v["project"]["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == "W-Q")
            .unwrap()["calendar_id"] = json!("QC-shift");
        let mut e = engine(&v);
        e.run_to_end();
        let r = serde_json::to_value(e.snapshot()).unwrap();
        let mut checks: Vec<_> = r["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| {
                j["operation_ref"]["id"] == "w5.op.inspect"
                    && j["input_refs"] == json!([assembly(2)])
            })
            .collect();
        checks.sort_by_key(|j| j["started_at_ms"].as_u64().unwrap());
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0]["started_at_ms"], 60000);
        assert_eq!(
            checks[0]["finished_at_ms"],
            if hold { 100000 } else { 70000 }
        );
        assert_eq!(checks[1]["started_at_ms"], 100000);
        assert_eq!(checks[1]["finished_at_ms"], 110000);
        assert_eq!(item(&r, &assembly(2))["completed_reworks"], 0);
        assert_eq!(r["goal_met"], true);
        let at95 = serde_json::to_value(
            e.objects_at(serde_json::from_value(json!(95000)).unwrap())
                .unwrap(),
        )
        .unwrap();
        let prior = at95
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == assembly(2))
            .unwrap();
        assert_eq!(prior["quality"], "pending");
        assert_eq!(
            prior["holder"],
            if hold {
                json!({"kind":"job","id":checks[0]["id"]})
            } else {
                json!({"kind":"location","id":"pending-store"})
            }
        );
        let at100 = serde_json::to_value(
            e.objects_at(serde_json::from_value(json!(100000)).unwrap())
                .unwrap(),
        )
        .unwrap();
        let during = at100
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == assembly(2))
            .unwrap();
        assert_eq!(during["holder"], json!({"kind":"job","id":checks[1]["id"]}));
        assert_eq!(during["lifecycle"], "in_process");
    }
}

#[test]
fn early_wrong_version_and_mislabeled_sources_never_become_future_release_permissions() {
    let mut v = fixture(false);
    let early = decision(2, 60000, "release");
    let mut wrong = decision(2, 80000, "release");
    wrong["inspection_ref"]["version"] = json!("9.0.0");
    let mut wrong_source = decision(2, 85000, "release");
    wrong_source["source_kind"] = json!("preset");
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
        json!([early, wrong, wrong_source, decision(1, 100000, "scrap")]);
    let r = run(&v);
    assert_eq!(item(&r, &assembly(2))["quality"], "pending");
    assert_eq!(
        item(&r, &assembly(2))["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    assert_eq!(r["acceptance_events"].as_array().unwrap().len(), 1);
    assert_eq!(
        r["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["code"] == "simulation.quality_decision_unapplied")
            .count(),
        3
    );
}

#[test]
fn a_loaded_module_alone_cannot_release_initial_pending_stock_without_its_actual_check() {
    let mut v = fixture(false);
    let mut incoming = v["scenario"]["initial_conditions"]["items"][0].clone();
    incoming["id"] = json!("incoming-pending");
    incoming["item_type_ref"]["id"] = json!("assembly");
    incoming["holder"] = json!({"kind":"location","id":"pending-store"});
    incoming["quality"] = json!("pending");
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(incoming);
    let mut release = decision(0, 0, "release");
    release["item_ref"] = json!("incoming-pending");
    release["id"] = json!("initial-without-check");
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([release]);
    let r = run(&v);
    assert_eq!(item(&r, "incoming-pending")["quality"], "pending");
    assert!(
        r["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "simulation.quality_decision_unapplied")
    );
}

#[test]
fn a_rejected_item_rechecked_to_unknown_uses_its_current_held_check_for_later_release() {
    let mut v = fixture(true);
    v["scenario"]["quality_presets"][1]["results"] = json!([{"kind":"string","value":"PASS"}]);
    v["scenario"]["quality_presets"][2]["results"] = json!([{"kind":"string","value":"FAIL"},null]);
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.inspect")
        .unwrap()["payload"]["data"]["quality_and_routes"]["max_completed_reworks"] = json!(0);
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([
        decision(2, 90000, "reinspect"),
        decision(2, 110000, "release")
    ]);
    let r = run(&v);
    let held = r["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| {
            j["id"].as_str().unwrap().starts_with("reinspection:")
                && j["input_refs"] == json!([assembly(2)])
        })
        .unwrap();
    assert_eq!(held["started_at_ms"], 90000);
    assert_eq!(held["finished_at_ms"], 110000);
    assert_eq!(held["state"], "completed");
    assert!(!r["resource_claims"].as_array().unwrap().iter().any(|c| {
        c["job_id"] == held["id"]
            && c["intervals"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["end_ms"] == 120000)
    }));
    assert_eq!(item(&r, &assembly(2))["completed_reworks"], 0);
    assert_eq!(r["goal_met"], true);
}

#[test]
fn released_pending_transfers_keep_ready_time_order_when_one_real_receiver_slot_opens() {
    let mut v = fixture(false);
    for n in [1, 2] {
        v["scenario"]["quality_presets"][n]["results"] = json!([null]);
    }
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "pending-store")
        .unwrap()["capacity"] = q(2);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "passed-in")
        .unwrap()["capacity"] = q(1);
    v["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"later-packing","title":"later-packing","source":"user_provided","intervals":[{"start_ms":100000,"end_ms":120000}]}));
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "W-P")
        .unwrap()["calendar_id"] = json!("later-packing");
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
        json!([decision(2, 80000, "release"), decision(1, 90000, "release")]);
    let r = run(&v);
    let time = |cycle| {
        r["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| {
                e["payload"]["kind"] == "item_transferred"
                    && e["payload"]["data"]["item_ref"] == assembly(cycle)
                    && e["payload"]["data"]["to_holder"]
                        == json!({"kind":"location","id":"passed-in"})
            })
            .unwrap()["at_ms"]
            .as_u64()
            .unwrap()
    };
    assert_eq!(time(2), 100000);
    assert_eq!(time(1), 110000);
}

#[test]
fn an_explicit_rework_for_an_already_queued_attempt_cannot_duplicate_the_actual_job_identity() {
    let mut v = fixture(false);
    v["scenario"]["initial_conditions"]["declared_quality_decisions"]
        .as_array_mut()
        .unwrap()
        .push(decision(1, 55000, "rework"));
    let r = run(&v);
    let jobs = r["jobs"].as_array().unwrap();
    let ids: std::collections::BTreeSet<_> =
        jobs.iter().map(|j| j["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), jobs.len());
    assert_eq!(item(&r, &assembly(1))["completed_reworks"], 1);
}

#[test]
fn scheduled_quality_handoffs_and_fresh_inspection_outputs_share_one_ready_time_queue() {
    let mut v = fixture(false);
    v["scenario"]["quality_presets"][1]["results"] = json!([null]);
    v["scenario"]["quality_presets"][2]["results"] = json!([{"kind":"string","value":"PASS"}]);
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] =
        json!([decision(1, 60000, "release")]);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "passed-in")
        .unwrap()["capacity"] = q(1);
    v["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"shared-queue-packing","title":"shared-queue-packing","source":"user_provided","intervals":[{"start_ms":100000,"end_ms":120000}]}));
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "W-P")
        .unwrap()["calendar_id"] = json!("shared-queue-packing");
    let r = run(&v);
    let time = |cycle| {
        r["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| {
                e["payload"]["kind"] == "item_transferred"
                    && e["payload"]["data"]["item_ref"] == assembly(cycle)
                    && e["payload"]["data"]["to_holder"]
                        == json!({"kind":"location","id":"passed-in"})
            })
            .unwrap()["at_ms"]
            .as_u64()
            .unwrap()
    };
    assert_eq!(time(1), 100000);
    assert_eq!(time(2), 110000);
}

#[test]
fn explicit_scrap_cancels_an_unreceived_quality_move_and_preserves_the_holder_at_disposal_time() {
    let mut v = fixture(false);
    v["scenario"]["quality_presets"][0]["results"] = json!([{"kind":"string","value":"FAIL"}]);
    v["scenario"]["quality_presets"][1]["results"] = json!([null]);
    v["scenario"]["quality_presets"][2]["results"] = json!([{"kind":"string","value":"PASS"}]);
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.inspect")
        .unwrap()["payload"]["data"]["quality_and_routes"]["max_completed_reworks"] = json!(0);
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([
        decision(1, 60000, "isolate"),
        decision(1, 70000, "scrap"),
        decision(0, 90000, "release")
    ]);
    let r = run(&v);
    let scrapped = item(&r, &assembly(1));
    assert_eq!(scrapped["lifecycle"], "scrapped");
    assert_eq!(
        scrapped["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    assert!(
        r["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |w| w["object_ref"] == json!({"kind":"item","id":assembly(1)})
                    && w["interval"] == json!({"start_ms":60000,"end_ms":70000})
                    && w["end_cause"] == "condition_changed"
            )
    );
}

#[test]
fn a_new_explicit_recheck_replaces_the_unreceived_isolation_effect_before_taking_its_item() {
    let mut v = fixture(false);
    v["scenario"]["quality_presets"][0]["results"] = json!([{"kind":"string","value":"FAIL"}]);
    v["scenario"]["quality_presets"][1]["results"] = json!([null,{"kind":"string","value":"PASS"}]);
    v["scenario"]["quality_presets"][2]["results"] = json!([{"kind":"string","value":"PASS"}]);
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.inspect")
        .unwrap()["payload"]["data"]["quality_and_routes"]["max_completed_reworks"] = json!(0);
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([
        decision(1, 60000, "isolate"),
        decision(1, 70000, "reinspect"),
        decision(0, 90000, "release")
    ]);
    let r = run(&v);
    assert_eq!(item(&r, &assembly(1))["holder"]["kind"], "container");
    assert_eq!(item(&r, &assembly(1))["completed_reworks"], 0);
    assert!(
        !r["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["payload"]["kind"] == "item_transferred"
                && e["payload"]["data"]["item_ref"] == assembly(1)
                && e["payload"]["data"]["to_holder"]
                    == json!({"kind":"location","id":"quarantine"}))
    );
    assert_eq!(r["goal_met"], true);
}
