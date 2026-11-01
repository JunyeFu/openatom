use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput, check_inputs};
use std::collections::BTreeSet;

fn q(n: u64, unit: &str) -> Value {
    json!({"value":{"coefficient":n,"decimal_places":0},"unit":unit})
}
fn subject(id: &str) -> Value {
    json!({"id":id,"revision":"r1"})
}
fn module(id: &str) -> Value {
    json!({"id":id,"version":"0.1.0"})
}
fn cases() -> Value {
    serde_json::from_str(include_str!(
        "../../../scenarios/并行汇合与返工/cases.design.json"
    ))
    .unwrap()
}
// Formal DTO adapter only. Topology, stock, quantities, result sequences and all
// durations come from the frozen design/declared module sources, not an Engine.
fn fixture(case: &str) -> Value {
    let source: Value = serde_json::from_str(include_str!(
        "../../../scenarios/并行汇合与返工/project.design.json"
    ))
    .unwrap();
    let mut v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap();
    let template = v.clone();
    let manifests: Vec<Value> = [
        include_str!("../../../modules/devices/fixed-transform/manifest.json"),
        include_str!("../../../modules/devices/manual-assembly/manifest.json"),
        include_str!("../../../modules/devices/inspect-route/manifest.json"),
        include_str!("../../../modules/devices/identity-rework/manifest.json"),
        include_str!("../../../modules/devices/carton-pack/manifest.json"),
    ]
    .iter()
    .map(|s| serde_json::from_str(s).unwrap())
    .collect();
    v["project"]["title"] = source["title"].clone();
    v["project"]["halls"][0]["id"] = json!("H-1");
    v["project"]["halls"][0]["input_locations"] = json!(["raw-M", "raw-P", "cartons"]);
    v["project"]["halls"][0]["output_locations"] = json!(["finished"]);
    v["project"]["item_types"] = Value::Array(
        source["item_types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let mut row = template["project"]["item_types"][0].clone();
                row["id"] = s["id"].clone();
                row["title"] = s["id"].clone();
                row
            })
            .collect(),
    );
    v["project"]["locations"] = Value::Array(
        source["stock_locations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let mut row = template["project"]["locations"][0].clone();
                row["id"] = s["id"].clone();
                row["hall_id"] = json!("H-1");
                row["capacity"] = q(s["capacity"].as_u64().unwrap(), "piece");
                row["accepted_item_types"] = json!([s["item"]]);
                let isolated = s["quality_policy"] == "quarantine_not_allocatable";
                row["purpose"] = json!(if isolated { "isolation" } else { "buffer" });
                row["ordinary_issue_enabled"] = json!(!isolated);
                row
            })
            .collect(),
    );
    let end = source["time"]["observation_end"].as_u64().unwrap() * 1000;
    v["project"]["calendars"][0]["intervals"] = json!([{"start_ms":0,"end_ms":end}]);
    v["project"]["resources"] = json!([]);
    let mut definitions = vec![
        template["definitions"][2].clone(),
        template["definitions"][3].clone(),
    ];
    let mut devices = vec![];
    let mut steps = vec![];
    let mut links = vec![];
    let mut bom = vec![];
    for device in source["devices"].as_array().unwrap() {
        let did = device["id"].as_str().unwrap();
        let opid = format!("parallel.{did}.operation");
        let defid = format!("parallel.{did}.station");
        let manifest = manifests
            .iter()
            .find(|m| m["id"] == device["module"])
            .unwrap();
        let behavior = &manifest["behavior"];
        let builtin = behavior["builtin"].as_str().unwrap();
        let mut operation = template["definitions"][0].clone();
        operation["id"] = json!(opid);
        operation["title"] = json!(opid);
        let mut op = template["definitions"][0]["payload"]["data"].clone();
        op["builtin"] = json!(builtin);
        op["inputs"] = json!([]);
        op["outputs"] = json!([]);
        op["pack_identity"] = Value::Null;
        op["identity_policy"] = json!(match builtin {
            "inspect_and_route" | "rework" => "preserve",
            "assemble" => "assembly_with_composition",
            "pack" => "container_with_contents",
            _ => "transform_with_origin",
        });
        let seconds = if builtin == "transform" {
            device["parameter_values"]["cycle_seconds"]
                .as_u64()
                .unwrap()
        } else {
            behavior["duration"]["value"].as_u64().unwrap()
        };
        op["duration"] = json!({"value":seconds*1000,"unit":"millisecond","basis":"user_provided","source_ref":null});
        let mut resource_bindings = json!({"execution_capacity":format!("machine-{did}")});
        let mut worker_roles = vec![];
        for (role, worker) in device["resource_bindings"].as_object().unwrap() {
            resource_bindings[role] = worker.clone();
            worker_roles.push(json!({"role":role,"quantity":q(1,"person"),"qualifications":[]}));
        }
        op["resource_roles"] = json!(worker_roles);
        let mut ports = vec![];
        let mut item_bindings = json!({});
        let mut roles = vec![];
        for (port, location) in device["port_locations"].as_object().unwrap() {
            let stock = source["stock_locations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == *location)
                .unwrap();
            let item = stock["item"].as_str().unwrap();
            let input = behavior["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["port"] == port.as_str());
            let quality = if input {
                if builtin == "inspect_and_route" {
                    json!(["unchecked"])
                } else if builtin == "rework" {
                    json!(["rejected"])
                } else {
                    json!(["released"])
                }
            } else {
                json!([])
            };
            let output_quality = if !input && matches!(builtin, "transform" | "pack") {
                json!("released")
            } else if !input && matches!(builtin, "assemble" | "rework") {
                json!("unchecked")
            } else {
                Value::Null
            };
            let row = json!({"role":port,"quantity":q(1,"piece"),"port":port,"required_quality":quality,"declared_output_quality":output_quality});
            op[if input { "inputs" } else { "outputs" }]
                .as_array_mut()
                .unwrap()
                .push(row);
            item_bindings[port] = json!(item);
            roles.push(json!(port));
            ports.push(json!({"id":port,"direction":if input{"input"}else{"output"},"payload":{"kind":"material","data":{"item_role":port,"unit":"piece","quantity_rule":null}}}));
            let ep = json!({"kind":"port","data":{"module_ref":module(&defid),"instance_id":did,"port_id":port}});
            let loc = json!({"kind":"location","data":location});
            links.push(json!({"kind":"material","data":{"id":format!("{did}-{port}"),"source":if input{loc.clone()}else{ep.clone()},"destination":if input{ep}else{loc},"item_type":item,"unit":"piece","quantity_rule":null,"destination_store":location,"capacity_rule":null}}));
            if input {
                bom.push(json!({"item_type_ref":subject(item),"quantity":q(1,"piece"),"consume_at_operation":format!("step-{did}")}));
            }
        }
        op["item_roles"] = json!(roles);
        if builtin == "inspect_and_route" {
            op["quality_and_routes"] = json!({"source_kind":"preset","result_ref":null,"max_completed_reworks":behavior["max_reworks"],"pass_port":"passed","fail_port":"rework","pending_policy":"hold_station","pending_location_role":null,"isolation_location_role":"isolation","release_required":true});
        }
        if builtin == "pack" {
            op["pack_identity"] = json!({"output_role":"packed","container_material_roles":["carton"],"content_roles":["product"]});
        }
        operation["payload"]["data"] = op;
        definitions.push(operation);
        let mut definition = template["definitions"][4].clone();
        definition["id"] = json!(defid);
        definition["title"] = json!(defid);
        definition["payload"]["data"]["ports"] = json!(ports);
        definition["payload"]["data"]["behavior"]["operation_ref"] = module(&opid);
        definitions.push(definition);
        let mut instance = template["project"]["devices"][0].clone();
        instance["id"] = json!(did);
        instance["title"] = json!(did);
        instance["hall_id"] = json!("H-1");
        instance["module_ref"] = module(&defid);
        instance["operation_binding"] = module(&opid);
        instance["role_bindings"]["item_types"] = item_bindings.clone();
        instance["role_bindings"]["resources"] = resource_bindings.clone();
        if builtin == "inspect_and_route" {
            instance["role_bindings"]["targets"] =
                json!({"isolation":{"kind":"location","id":"quarantine"}});
        }
        devices.push(instance);
        let mut machine = template["project"]["resources"][1].clone();
        machine["id"] = json!(format!("machine-{did}"));
        machine["home_location"] = json!("inspect-in");
        v["project"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(machine);
        let predecessors = match did {
            "ASM-01" => json!(["step-M-01", "step-P-01"]),
            "QC-01" => json!(["step-ASM-01"]),
            "PACK-01" => json!(["step-ASM-01"]),
            _ => json!([]),
        };
        steps.push(json!({"id":format!("step-{did}"),"operation_ref":module(&opid),"device_id":did,"predecessors":predecessors,"item_bindings":item_bindings,"resource_bindings":device["resource_bindings"]}));
    }
    for worker in source["resources"].as_array().unwrap() {
        let mut row = template["project"]["resources"][0].clone();
        row["id"] = worker["id"].clone();
        row["home_location"] = json!("inspect-in");
        v["project"]["resources"].as_array_mut().unwrap().push(row);
    }
    v["project"]["devices"] = json!(devices);
    v["project"]["routes"][0]["steps"] = json!(steps);
    v["project"]["connections"] = json!(links);
    v["project"]["boms"][0]["rows"] = json!(bom);
    v["project"]["module_refs"] = Value::Array(
        definitions
            .iter()
            .map(|d| module(d["id"].as_str().unwrap()))
            .collect(),
    );
    v["definitions"] = json!(definitions);
    v["scenario"]["window"]["end_ms"] = json!(end);
    v["scenario"]["orders"][0]["due_at_ms"] = json!(end);
    v["scenario"]["orders"][0]["destination"] = json!("finished");
    v["scenario"]["scope"] = Value::Array(
        source["devices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| json!({"kind":"device","id":d["id"]}))
            .collect(),
    );
    v["scenario"]["initial_conditions"]["items"] = Value::Array(
        source["initial_items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let mut row = template["scenario"]["initial_conditions"]["items"][0].clone();
                row["id"] = s["id"].clone();
                row["item_type_ref"] = subject(s["type"].as_str().unwrap());
                row["holder"]["id"] = s["location"].clone();
                row
            })
            .collect(),
    );
    let cases = cases();
    let chosen = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == case)
        .unwrap();
    v["scenario"]["quality_presets"]=Value::Array(chosen["quality_inputs"].as_array().unwrap().iter().map(|source|json!({"id":format!("preset-{}",source["ancestor_id"].as_str().unwrap()),"item_or_ancestor":source["ancestor_id"],"match_kind":"ancestor","inspection_module":module("parallel.QC-01.operation"),"results":source["inspection_results"].as_array().unwrap().iter().map(|result|if result.is_null(){Value::Null}else{json!({"kind":"enum","value":result})}).collect::<Vec<_>>()})).collect());
    v
}
fn input(v: Value) -> EngineInput {
    let scenario: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
    EngineInput {
        run_id: RunId::try_from("parallel-rework-owner".to_owned()).unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
    }
}
fn engine(v: Value) -> Engine {
    Engine::new(input(v)).unwrap_or_else(|e| panic!("{e:#?}"))
}
fn run(v: Value) -> RunResult {
    let mut e = engine(v);
    e.run_to_end();
    e.snapshot()
}
fn t(n: u64) -> SimTimeMs {
    SimTimeMs(SafeUint::try_from(n).unwrap())
}
fn op<'a>(v: &'a mut Value, device: &str) -> &'a mut Value {
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == format!("parallel.{device}.operation"))
        .map(|d| &mut d["payload"]["data"])
        .unwrap()
}
fn at_device(job: &JobState, device: &str) -> bool {
    matches!(&job.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()==device)
}
fn decisions(r: &RunResult) -> Vec<&QualityDecision> {
    r.events
        .iter()
        .filter_map(|e| match &e.payload {
            SimulationEventPayload::QualityChanged(d) => Some(d),
            _ => None,
        })
        .collect()
}
fn summary(r: &RunResult) -> Value {
    json!({"package_finish_seconds":r.acceptance_events.iter().map(|a|a.at_ms.0.value()/1000).collect::<Vec<_>>(),"packages":r.final_items.iter().filter(|i|i.item_type_ref.id.as_str()=="package").count(),"quarantined":r.final_items.iter().filter(|i|matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="quarantine")).count(),"awaiting_result":r.final_items.iter().filter(|i|matches!(i.quality,QualityState::Pending)).count(),"rework_cycles":r.final_items.iter().map(|i|i.completed_reworks.value()).sum::<u64>(),"inspection_started_count":r.jobs.iter().filter(|j|at_device(j,"QC-01")&&j.started_at_ms.is_some()).count(),"quality_decisions":decisions(r).len(),"waiting_for_inspector":r.jobs.iter().filter(|j|at_device(j,"QC-01")&&j.started_at_ms.is_none()&&matches!(j.state,JobStatus::WaitingResource)).count()})
}
#[test]
fn frozen_three_original_cases_match_all_fields_and_same_id_ownership() {
    for case in cases()["cases"].as_array().unwrap() {
        let r = run(fixture(case["id"].as_str().unwrap()));
        assert!(r.whole_window_computed, "{:?}", r.limitations);
        assert!(r.limitations.is_empty(), "{:?}", r.limitations);
        let actual = summary(&r);
        println!(
            "P1_02_FIXED_CASE {}",
            json!({"id":case["id"],"expected":case["expected"],"actual":actual})
        );
        assert_eq!(actual, case["expected"], "{}", case["id"]);
        assert_eq!(r.goal_met, Some(case["id"] == "A"));
        let packages = actual["packages"].as_u64().unwrap();
        assert_eq!(r.orders[0].accepted.value.coefficient.value(), packages);
        assert_eq!(
            r.orders[0].outstanding.value.coefficient.value(),
            2 - packages
        );
        for resource in r
            .resource_claims
            .iter()
            .map(|c| c.resource_id.clone())
            .collect::<BTreeSet<_>>()
        {
            let mut intervals: Vec<_> = r
                .resource_claims
                .iter()
                .filter(|c| c.resource_id == resource)
                .flat_map(|c| {
                    c.intervals
                        .iter()
                        .map(|i| (i.start_ms.0.value(), i.end_ms.0.value()))
                })
                .collect();
            intervals.sort();
            assert!(
                intervals.windows(2).all(|pair| pair[0].1 <= pair[1].0),
                "exclusive resource {resource:?}: {intervals:?}"
            );
        }
        let ids: BTreeSet<_> = r.final_items.iter().map(|i| i.id.clone()).collect();
        assert_eq!(ids.len(), r.final_items.len());
        let assemblies: Vec<_> = r
            .final_items
            .iter()
            .filter(|i| i.item_type_ref.id.as_str() == "assembly")
            .collect();
        assert_eq!(assemblies.len(), 2);
        assert!(
            assemblies
                .iter()
                .all(|i| !matches!(i.lifecycle, Lifecycle::Scrapped | Lifecycle::Transformed))
        );
        for d in decisions(&r) {
            assert!(assemblies.iter().any(|i| i.id == d.item_ref));
        }
        for item in r
            .final_items
            .iter()
            .filter(|i| i.item_type_ref.id.as_str() == "package")
        {
            assert_eq!(item.contents.len(), 1);
            assert!(assemblies.iter().any(|i| i.id == item.contents[0]
                && matches!(&i.holder,HolderRef::Container(c)if *c==item.id)
                && matches!(i.quality, QualityState::Released)));
        }
    }
}
#[test]
fn unknown_holds_inspector_and_machine_but_only_ten_seconds_effective_work() {
    let r = run(fixture("U"));
    let held = r
        .jobs
        .iter()
        .find(|j| at_device(j, "QC-01") && j.started_at_ms.is_some())
        .unwrap();
    for resource in ["W-Q", "machine-QC-01"] {
        let claims: Vec<_> = r
            .resource_claims
            .iter()
            .filter(|c| c.job_id == held.id && c.resource_id.as_str() == resource)
            .collect();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].intervals[0].start_ms.0.value(), 90000);
        assert_eq!(claims[0].intervals[0].end_ms.0.value(), 180000);
        assert!(matches!(
            claims[0].interval_end_causes[0],
            ClaimIntervalEndCause::ObservationEnd
        ));
    }
    let work: u64 = r
        .phase_work_intervals
        .iter()
        .filter(|w| w.job_id == held.id)
        .map(|w| w.interval.end_ms.0.value() - w.interval.start_ms.0.value())
        .sum();
    assert_eq!(work, 10000);
    assert!(r.waits.iter().any(|w| {
        w.object_ref.id == held.id
            && w.interval.start_ms.0.value() == 100000
            && w.interval.end_ms.0.value() == 180000
            && w.reasons
                .iter()
                .any(|r| matches!(r.kind, WaitReasonKind::Quality))
    }));
    assert!(r.waits.iter().any(|w| {
        w.interval.start_ms.0.value() == 150000
            && w.reasons
                .iter()
                .filter(|r| matches!(r.kind, WaitReasonKind::ResourceBusy))
                .count()
                == 2
    }));
}
#[test]
fn parallel_assembly_takes_nothing_until_both_real_branches_are_ready() {
    let mut e = engine(fixture("A"));
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|at| at.0.value() < 20000)
    {
        e.step();
    }
    let r = e.snapshot();
    assert!(
        !r.jobs
            .iter()
            .filter(|j| at_device(j, "ASM-01"))
            .any(|j| j.started_at_ms.is_some())
    );
    assert!(
        !r.resource_claims
            .iter()
            .any(|c| c.resource_id.as_str() == "W-A")
    );
    assert!(e.objects_at(t(20000)).unwrap().iter().any(
        |i| i.item_type_ref.id.as_str() == "pcb" && matches!(i.holder, HolderRef::Location(_))
    ));
    e.run_to_end();
    assert_eq!(
        summary(&e.snapshot())["package_finish_seconds"],
        json!([145, 175])
    );
}

#[test]
fn rework_counts_only_at_actual_completion_then_reinspection_resets_no_identity() {
    let mut e = engine(fixture("A"));
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|t| t.0.value() < 100000)
    {
        e.step();
    }
    let before = e.snapshot();
    let first = decisions(&before)[0].item_ref.clone();
    assert_eq!(
        before
            .final_items
            .iter()
            .find(|i| i.id == first)
            .unwrap()
            .completed_reworks
            .value(),
        0
    );
    assert!(matches!(
        before
            .final_items
            .iter()
            .find(|i| i.id == first)
            .unwrap()
            .quality,
        QualityState::Rejected
    ));
    e.step();
    assert_eq!(e.snapshot().computed_until_ms.unwrap().0.value(), 120000);
    let after = e.snapshot();
    let object = after.final_items.iter().find(|i| i.id == first).unwrap();
    assert_eq!(object.completed_reworks.value(), 1);
    assert!(matches!(object.quality, QualityState::Pending));
    assert!(e.item_history().iter().any(|(at, i)| at.0.value() == 120000
        && i.id == first
        && i.completed_reworks.value() == 1
        && matches!(i.quality, QualityState::Unchecked)));
    assert!(
        !decisions(&after)
            .iter()
            .any(|d| d.item_ref == first && matches!(d.disposition, QualityDisposition::Release))
    );
    e.run_to_end();
    assert_eq!(
        e.snapshot()
            .final_items
            .iter()
            .find(|i| i.id == first)
            .unwrap()
            .completed_reworks
            .value(),
        1
    );
}
#[test]
fn zero_and_two_rework_limits_are_real_bounded_loops_never_automatic_scrap() {
    let mut zero = fixture("A");
    op(&mut zero, "QC-01")["quality_and_routes"]["max_completed_reworks"] = json!(0);
    let r = run(zero);
    assert_eq!(summary(&r)["rework_cycles"], 0);
    assert_eq!(summary(&r)["quarantined"], 1);
    assert_eq!(summary(&r)["quality_decisions"], 2);
    let mut two = fixture("A");
    op(&mut two, "QC-01")["quality_and_routes"]["max_completed_reworks"] = json!(2);
    two["scenario"]["quality_presets"][0]["results"] = json!([{"kind":"enum","value":"fail"},{"kind":"enum","value":"fail"},{"kind":"enum","value":"pass"}]);
    let r = run(two);
    let d = decisions(&r);
    let first = &d[0].item_ref;
    let object = r.final_items.iter().find(|i| i.id == *first).unwrap();
    assert_eq!(object.completed_reworks.value(), 2);
    assert!(matches!(object.quality, QualityState::Released));
    assert!(
        !r.final_items
            .iter()
            .any(|i| matches!(i.lifecycle, Lifecycle::Scrapped))
    );
    assert_eq!(d.iter().filter(|d| d.item_ref == *first).count(), 3);
    assert_eq!(
        r.jobs
            .iter()
            .filter(|j| at_device(j, "RW-01") && j.finished_at_ms.is_some())
            .count(),
        2
    );
}
#[test]
fn full_fail_receiver_retains_one_decision_one_candidate_and_inspector() {
    let mut v = fixture("A");
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "rework-in")
        .unwrap()["capacity"] = q(0, "piece");
    let r = run(v);
    assert_eq!(decisions(&r).len(), 1);
    assert_eq!(summary(&r)["inspection_started_count"], 1);
    assert_eq!(summary(&r)["rework_cycles"], 0);
    let qc = r
        .jobs
        .iter()
        .find(|j| at_device(j, "QC-01") && j.started_at_ms.is_some())
        .unwrap();
    assert_eq!(qc.pending_outputs.len(), 1);
    assert!(qc.finished_at_ms.is_none());
    let object = r
        .final_items
        .iter()
        .find(|i| i.id == qc.pending_outputs[0])
        .unwrap();
    assert!(matches!(&object.holder,HolderRef::Job(id)if *id==qc.id));
    assert!(
        r.resource_claims
            .iter()
            .any(|c| c.resource_id.as_str() == "W-Q"
                && c.intervals.last().unwrap().end_ms.0.value() == 180000)
    );
}
fn inspector_calendar(v: &mut Value, intervals: Value) {
    v["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"QC-CALENDAR","title":"explicit check availability","intervals":intervals,"source":"user_provided"}));
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "W-Q")
        .unwrap()["calendar_id"] = json!("QC-CALENDAR");
}
#[test]
fn calendar_wait_does_not_consume_material_or_skip_first_preset_result() {
    let mut v = fixture("A");
    inspector_calendar(&mut v, json!([{"start_ms":110000,"end_ms":180000}]));
    let mut e = engine(v);
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|t| t.0.value() < 90000)
    {
        e.step();
    }
    let r = e.snapshot();
    assert_eq!(summary(&r)["inspection_started_count"], 0);
    assert!(
        r.final_items
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "assembly"
                && matches!(i.quality, QualityState::Unchecked)
                && matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="inspect-in"))
    );
    e.run_to_end();
    let r = e.snapshot();
    let d = decisions(&r);
    assert_eq!(d[0].effective_at_ms.0.value(), 120000);
    assert!(matches!(d[0].disposition, QualityDisposition::Rework));
    assert_eq!(summary(&r)["rework_cycles"], 1);
}
#[test]
fn paused_effective_inspection_resumes_same_locked_result_once() {
    let mut v = fixture("A");
    inspector_calendar(
        &mut v,
        json!([{"start_ms":0,"end_ms":95000},{"start_ms":115000,"end_ms":180000}]),
    );
    let check = op(&mut v, "QC-01");
    let duration = check["duration"].clone();
    check["duration"] = Value::Null;
    check["phases"] = json!([{"id":"check","duration":duration,"phase_resources":[],"interruption_policy":"pause_resume","work_kind":"inspection"}]);
    let r = run(v);
    let d = decisions(&r);
    assert_eq!(d[0].effective_at_ms.0.value(), 120000);
    assert!(matches!(d[0].disposition, QualityDisposition::Rework));
    assert_eq!(summary(&r)["quality_decisions"], 3);
    let job = r
        .jobs
        .iter()
        .find(|j| {
            at_device(j, "QC-01")
                && j.started_at_ms
                    .as_ref()
                    .is_some_and(|t| t.0.value() == 90000)
        })
        .unwrap();
    let work: Vec<_> = r
        .phase_work_intervals
        .iter()
        .filter(|w| w.job_id == job.id)
        .collect();
    assert_eq!(work.len(), 2);
    assert_eq!(work[0].interval.end_ms.0.value(), 95000);
    assert_eq!(work[1].interval.start_ms.0.value(), 115000);
}
#[test]
fn missing_null_exhausted_and_unrecognized_results_remain_unknown_without_decision() {
    for variant in 0..4 {
        let mut v = fixture("A");
        match variant {
            0 => {
                v["scenario"]["quality_presets"]
                    .as_array_mut()
                    .unwrap()
                    .remove(0);
            }
            1 => v["scenario"]["quality_presets"][0]["results"] = json!([null]),
            2 => v["scenario"]["quality_presets"][0]["results"] = json!([]),
            _ => {
                v["scenario"]["quality_presets"][0]["results"] =
                    json!([{"kind":"string","value":"unknown"}])
            }
        }
        let r = run(v);
        assert_eq!(summary(&r), cases()["cases"][2]["expected"]);
    }
}
#[test]
fn wrong_version_and_duplicate_sources_are_located_at_pure_admission() {
    let mut v = fixture("A");
    v["scenario"]["quality_presets"][0]["inspection_module"]["version"] = json!("0.2.0");
    assert!(
        check_inputs(&input(v))
            .unwrap_err()
            .iter()
            .any(|i| i.code == "simulation.quality_preset_module")
    );
    let mut v = fixture("A");
    let mut duplicate = v["scenario"]["quality_presets"][0].clone();
    duplicate["id"] = json!("duplicate-scope");
    v["scenario"]["quality_presets"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(
        check_inputs(&input(v))
            .unwrap_err()
            .iter()
            .any(|i| i.code == "simulation.quality_preset_ambiguous")
    );
}
#[test]
fn two_actual_matching_ancestors_fail_before_check_takes_item_or_resources() {
    let mut v = fixture("A");
    let mut duplicate = v["scenario"]["quality_presets"][0].clone();
    duplicate["id"] = json!("pcb-ancestor-source");
    duplicate["item_or_ancestor"] = json!("raw_pcb-001");
    v["scenario"]["quality_presets"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let r = run(v);
    assert!(matches!(r.computation_status, ComputationStatus::Failed));
    assert_eq!(summary(&r)["inspection_started_count"], 0);
    assert!(
        r.limitations
            .iter()
            .any(|i| i.code == "simulation.quality_preset_ambiguous")
    );
    assert!(
        r.final_items
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "assembly"
                && matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="inspect-in"))
    );
    assert!(
        !r.resource_claims
            .iter()
            .any(|c| c.resource_id.as_str() == "W-Q")
    );
}
fn pending_store(v: &mut Value, capacity: u64) {
    let mut location = v["project"]["locations"][0].clone();
    location["id"] = json!("pending");
    location["accepted_item_types"] = json!(["assembly"]);
    location["purpose"] = json!("pending");
    location["ordinary_issue_enabled"] = json!(false);
    location["capacity"] = q(capacity, "piece");
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .push(location);
    let rule = &mut op(v, "QC-01")["quality_and_routes"];
    rule["pending_policy"] = json!("pending_store");
    rule["pending_location_role"] = json!("pending");
    v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "QC-01")
        .unwrap()["role_bindings"]["targets"]["pending"] =
        json!({"kind":"location","id":"pending"});
}
#[test]
fn unknown_moves_only_to_explicit_pending_capacity_and_frees_inspector_after_receive() {
    let mut v = fixture("U");
    pending_store(&mut v, 1);
    let r = run(v);
    assert_eq!(summary(&r)["package_finish_seconds"], json!([175]));
    assert_eq!(decisions(&r).len(), 1);
    assert_eq!(summary(&r)["awaiting_result"], 1);
    assert!(
        r.final_items
            .iter()
            .any(|i| matches!(i.quality, QualityState::Pending)
                && matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="pending"))
    );
    let mut v = fixture("U");
    pending_store(&mut v, 0);
    let r = run(v);
    assert_eq!(summary(&r), cases()["cases"][2]["expected"]);
}
fn pending_item() -> ObjectId {
    let r = run(fixture("U"));
    r.final_items
        .iter()
        .find(|i| matches!(i.quality, QualityState::Pending))
        .unwrap()
        .id
        .clone()
}
fn declare(v: &mut Value, item: &ObjectId, at: u64, version: &str) {
    v["scenario"]["initial_conditions"]["declared_quality_decisions"].as_array_mut().unwrap().push(json!({"id":format!("declared-{at}-{version}"),"item_ref":item,"inspection_ref":{"id":"parallel.QC-01.operation","version":version},"source_kind":"manual","observed_result":null,"disposition":"release","effective_at_ms":at,"decision_source":{"kind":"human_statement","data":"explicit scheduled release"}}));
}
#[test]
fn scheduled_decision_requires_actual_completed_pending_check_and_exact_version() {
    let item = pending_item();
    let mut v = fixture("U");
    declare(&mut v, &item, 95000, "0.1.0");
    declare(&mut v, &item, 110000, "0.2.0");
    let r = run(v);
    assert_eq!(summary(&r), cases()["cases"][2]["expected"]);
    assert_eq!(
        r.limitations
            .iter()
            .filter(|i| i.code == "simulation.quality_decision_unapplied")
            .count(),
        2
    );
    let mut v = fixture("U");
    pending_store(&mut v, 1);
    declare(&mut v, &item, 120000, "0.1.0");
    let r = run(v);
    assert_eq!(summary(&r)["package_finish_seconds"], json!([135, 175]));
    assert_eq!(decisions(&r).len(), 2);
    assert!(r.limitations.is_empty());
}
#[test]
fn reordered_sources_pause_cancel_and_fresh_run_preserve_deterministic_real_prefix() {
    let original = fixture("A");
    let expected = run(original.clone());
    let mut reverse = original.clone();
    reverse["definitions"].as_array_mut().unwrap().reverse();
    reverse["scenario"]["quality_presets"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reverse["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        serde_json::to_value(run(reverse)).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    let mut e = engine(original.clone());
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|t| t.0.value() < 100000)
    {
        e.step();
    }
    e.pause();
    let paused = serde_json::to_value(e.snapshot()).unwrap();
    assert_eq!(e.step(), 0);
    assert_eq!(serde_json::to_value(e.snapshot()).unwrap(), paused);
    e.resume();
    e.cancel();
    assert_eq!(e.step(), 0);
    assert!(!e.snapshot().whole_window_computed);
    assert!(e.objects_at(t(100001)).is_none());
    assert!(!e.item_history().iter().any(|(at, _)| at.0.value() > 100000));
    assert_eq!(
        serde_json::to_value(run(original)).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

#[test]
fn two_real_items_from_same_batch_share_one_run_scoped_result_sequence() {
    let mut v = fixture("A");
    v["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "raw_shell")
        .unwrap()["tracking"] = json!("batch");
    let items = v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap();
    let mut batch = items
        .iter()
        .find(|i| i["id"] == "raw_shell-001")
        .unwrap()
        .clone();
    batch["id"] = json!("shell-batch");
    batch["quantity"] = q(2, "piece");
    items.retain(|i| i["item_type_ref"]["id"] != "raw_shell");
    items.push(batch);
    let mut preset = v["scenario"]["quality_presets"][0].clone();
    preset["id"] = json!("one-batch-preset");
    preset["item_or_ancestor"] = json!("shell-batch");
    preset["match_kind"] = json!("batch");
    preset["results"] = json!([{"kind":"enum","value":"fail"},{"kind":"enum","value":"pass"},{"kind":"enum","value":"pass"}]);
    v["scenario"]["quality_presets"] = json!([preset]);
    let r = run(v);
    assert!(r.limitations.is_empty(), "{:?}", r.limitations);
    assert_eq!(summary(&r), cases()["cases"][0]["expected"]);
    assert_eq!(
        decisions(&r)
            .iter()
            .map(|d| d.item_ref.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    assert!(decisions(&r).iter().all(|d|matches!(&d.decision_source,DecisionSource::ScheduledPreset(id)if id.as_str()=="one-batch-preset")));
}
#[test]
fn failed_check_without_an_explicit_rework_return_path_isolates_without_scrap() {
    let mut v = fixture("A");
    v["project"]["routes"][0]["steps"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["device_id"] != "RW-01");
    v["project"]["boms"][0]["rows"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["consume_at_operation"] != "step-RW-01");
    let r = run(v);
    assert_eq!(summary(&r)["quarantined"], 1);
    assert_eq!(summary(&r)["rework_cycles"], 0);
    assert!(matches!(
        decisions(&r)[0].disposition,
        QualityDisposition::Isolate
    ));
    assert!(
        !r.final_items
            .iter()
            .any(|i| matches!(i.lifecycle, Lifecycle::Scrapped))
    );
}
#[test]
fn explicitly_declared_scrap_keeps_pending_and_isolation_physical_ownership() {
    for case in ["U", "Q"] {
        let mut v = fixture(case);
        let baseline = run(v.clone());
        let object = if case == "U" {
            baseline
                .final_items
                .iter()
                .find(|i| matches!(i.quality, QualityState::Pending))
        } else {
            baseline
                .final_items
                .iter()
                .find(|i| matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="quarantine"))
        }
        .unwrap();
        declare(&mut v, &object.id, 140000, "0.1.0");
        v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
            json!("scrap");
        let r = run(v);
        let scrapped = r.final_items.iter().find(|i| i.id == object.id).unwrap();
        assert!(matches!(scrapped.lifecycle, Lifecycle::Scrapped));
        assert_eq!(
            serde_json::to_value(&scrapped.holder).unwrap(),
            serde_json::to_value(&object.holder).unwrap()
        );
        assert_eq!(scrapped.composition_refs, object.composition_refs);
        assert_eq!(
            decisions(&r)
                .iter()
                .filter(|d| matches!(d.disposition, QualityDisposition::Scrap))
                .count(),
            1
        );
    }
}
#[test]
fn a_mislabeled_declared_quality_source_does_not_release_pending_material() {
    let mut v = fixture("U");
    let item = pending_item();
    declare(&mut v, &item, 120000, "0.1.0");
    v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["source_kind"] =
        json!("tool_result");
    let r = run(v);
    assert_eq!(summary(&r), cases()["cases"][2]["expected"]);
    assert!(
        r.limitations
            .iter()
            .any(|i| i.code == "simulation.quality_decision_unapplied")
    );
}
