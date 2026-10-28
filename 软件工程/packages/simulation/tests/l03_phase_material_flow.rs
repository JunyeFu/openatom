use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput};
use std::{fs, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn id(s: &str) -> ObjectId {
    ObjectId::try_from(s.to_owned()).unwrap()
}
fn time(t: u64) -> SimTimeMs {
    SimTimeMs(t.try_into().unwrap())
}
fn fixture() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("scenarios/媒体表现/runtime-l03/engine-input.json")).unwrap(),
    )
    .unwrap()
}
fn input(value: Value) -> EngineInput {
    let scenario: ScenarioInput = serde_json::from_value(value["scenario"].clone()).unwrap();
    EngineInput{run_id:RunId::try_from("l03-canonical-engine-test".to_owned()).unwrap(),input_ref:serde_json::from_value(json!({
  "project_id":value["project"]["id"],"project_revision":1,"scenario_id":scenario.id,"scenario_revision":1,
  "frozen_project_file":{"path":"test-only/project.json","kind":"project"},
  "frozen_scenario_file":{"path":"test-only/scenario.json","kind":"scenario"}})).unwrap(),
  project:serde_json::from_value(value["project"].clone()).unwrap(),definitions:serde_json::from_value(value["definitions"].clone()).unwrap(),
  scope:scenario.scope.clone(),scenario}
}
fn engine(value: Value) -> Engine {
    match Engine::new(input(value)) {
        Ok(e) => e,
        Err(e) => panic!("actual admission: {e:#?}"),
    }
}
fn jobs<'a>(result: &'a RunResult, device: &str) -> Vec<&'a JobState> {
    let mut rows:Vec<_>=result.jobs.iter().filter(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..} if device_id.as_str()==device)&&j.started_at_ms.is_some()).collect();
    rows.sort_by_key(|j| j.started_at_ms.as_ref().unwrap().0.value());
    rows
}
fn item_at(e: &Engine, item: &ObjectId, t: u64) -> RuntimeItem {
    e.objects_at(time(t))
        .unwrap()
        .into_iter()
        .find(|i| i.id == *item)
        .unwrap()
}
fn is_state(item: &RuntimeItem, state: &str) -> bool {
    matches!(item.attributes.get("closure"),Some(TypedValue::Enum(v)) if v==state)
}

#[test]
fn original_l03_six_packages_times_composition_rework_and_real_stage_holders() {
    let mut e = engine(fixture());
    e.run_to_end();
    let result = e.snapshot();
    fs::write(
        root().join("scenarios/媒体表现/runtime-l03/engine-last.actual.json"),
        serde_json::to_vec_pretty(
            &json!({"kind":"actual_Engine_diagnostic_not_Host_or_test_pass","run":result}),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        result.computation_status,
        ComputationStatus::Completed
    ));
    assert_eq!(
        result
            .acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect::<Vec<_>>(),
        [90000, 130000, 240000, 255000, 290000, 330000]
    );
    assert_eq!(
        result
            .acceptance_events
            .iter()
            .map(|a| a.quantity.value.coefficient.value())
            .sum::<u64>(),
        6
    );
    let robots = jobs(&result, "R01");
    let first = robots[0];
    assert_eq!(first.started_at_ms.as_ref().unwrap().0.value(), 40000);
    assert_eq!(first.finished_at_ms.as_ref().unwrap().0.value(), 52000);
    let cargo = &first.input_refs[0];
    assert!(
        matches!(item_at(&e,cargo,42999).holder,HolderRef::Location(ref l) if l.as_str()=="out-ASM01")
    );
    assert!(matches!(item_at(&e,cargo,43000).holder,HolderRef::Job(ref j) if *j==first.id));
    assert!(matches!(item_at(&e,cargo,48999).holder,HolderRef::Job(ref j) if *j==first.id));
    let at49 = e.jobs_at(time(49000)).unwrap();
    let running = at49.iter().find(|j| j.id == first.id).unwrap();
    assert!(matches!(running.state, JobStatus::Working));
    assert_eq!(running.phase_ref.as_ref().unwrap().as_str(), "return");
    assert!(running.finished_at_ms.is_none());
    let sealers = jobs(&result, "CASE01");
    let first_sealer = sealers[0];
    assert_eq!(
        first_sealer.started_at_ms.as_ref().unwrap().0.value(),
        82000
    );
    let package = first_sealer
        .input_refs
        .iter()
        .find(|i| {
            result
                .final_items
                .iter()
                .any(|item| item.id == **i && item.item_type_ref.id.as_str() == "package")
        })
        .unwrap();
    assert!(is_state(&item_at(&e, package, 87999), "open"));
    assert!(is_state(&item_at(&e, package, 88000), "sealed"));
    assert!(
        matches!(item_at(&e,package,89999).holder,HolderRef::Job(ref j) if *j==first_sealer.id)
    );
    assert!(
        matches!(item_at(&e,package,90000).holder,HolderRef::Location(ref l) if l.as_str()=="PAL01-store")
    );
    let accepted: Vec<_> = result
        .acceptance_events
        .iter()
        .flat_map(|a| a.item_refs.iter())
        .collect();
    let mut contents = std::collections::BTreeSet::new();
    for item in accepted {
        let p = result.final_items.iter().find(|i| i.id == *item).unwrap();
        assert!(is_state(p, "sealed"));
        assert_eq!(p.contents.len(), 1);
        contents.insert(p.contents[0].clone());
        let content = result
            .final_items
            .iter()
            .find(|i| i.id == p.contents[0])
            .unwrap();
        assert!(matches!(&content.holder,HolderRef::Container(c) if *c==p.id));
        assert_eq!(content.composition_refs.len(), 6);
        if content.composition_refs.contains(&id("shell-003")) {
            assert_eq!(content.completed_reworks.value(), 1);
        }
    }
    assert_eq!(contents.len(), 6);
    let tape: Vec<_> = sealers
        .iter()
        .flat_map(|j| j.input_refs.iter())
        .filter_map(|key| {
            result
                .final_items
                .iter()
                .find(|i| &i.id == key && i.item_type_ref.id.as_str() == "sealing_tape")
        })
        .collect();
    assert_eq!(tape.len(), 6);
    assert_eq!(
        tape.iter()
            .map(|i| i.quantity.value.coefficient.value())
            .sum::<u64>(),
        36
    );
    assert!(
        tape.iter()
            .all(|i| i.quantity.value.decimal_places.value() == 1
                && matches!(i.lifecycle, Lifecycle::Transformed))
    );
    let output = root().join("scenarios/媒体表现/runtime-l03/engine-only.actual.json");
    fs::write(output,serde_json::to_vec_pretty(&json!({"kind":"actual_single_Engine_test_not_Host_save_or_Native","run":result,"source_expected_times_ms":[90000,130000,240000,255000,290000,330000]})).unwrap()).unwrap();
}

#[test]
fn old_nine_full_runs_roundtrip_without_new_instance_attribute_keys() {
    let old = root().join("scenarios/真实业务设计/runtime-job-history");
    for name in [
        "root_original",
        "root_sdk_mapped",
        "w5_original_source",
        "w5_original_target",
        "existing_seed",
        "internal-pass",
        "external-pass",
        "internal-pending",
        "p303-actual-history",
    ] {
        let file = if name == "p303-actual-history" {
            format!("{name}.json")
        } else {
            format!("{name}.actual.json")
        };
        let value: Value = serde_json::from_slice(&fs::read(old.join(file)).unwrap()).unwrap();
        let actual: RunResult = serde_json::from_value(value["run"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            value["run"],
            "old complete Run changed: {name}"
        );
    }
    let old: Value = serde_json::from_slice(
        &fs::read(
            root().join("packages/commands/tests/fixtures/production/local-assembly-pack.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let definition: ModuleDefinition =
        serde_json::from_value(old["definitions"][0].clone()).unwrap();
    assert!(
        serde_json::to_value(definition).unwrap()["payload"]["data"]
            .get("material_flow")
            .is_none()
    );
}

#[test]
fn phase_flow_rejects_wrong_main_identity_missing_phase_and_attribute_enum() {
    for mutation in ["primary", "phase", "enum"] {
        let mut value = fixture();
        let d = value["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["id"] == "p302.l03.operation.CASE01")
            .unwrap();
        let flow = &mut d["payload"]["data"]["material_flow"];
        match mutation {
            "primary" => flow["primary_role"] = json!("sealing_tape"),
            "phase" => flow["consume_after_phase"] = json!("absent-phase"),
            _ => {
                flow["attributes_after_consume"]["closure"] =
                    json!({"kind":"enum","value":"invented"})
            }
        }
        assert!(Engine::new(input(value)).is_err(), "{mutation}");
    }
}

#[test]
fn full_robot_destination_freezes_before_release_with_same_closed_grip_cargo() {
    let mut value = fixture();
    let mut blocker = value["scenario"]["initial_conditions"]["items"][0].clone();
    blocker["id"] = json!("R01-destination-pending-item");
    blocker["item_type_ref"] = json!({"id":"assembly","revision":"r1"});
    blocker["holder"] = json!({"kind":"location","id":"out-R01"});
    blocker["quality"] = json!("pending");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(blocker);
    let mut e = engine(value);
    e.run_to_end();
    let result = e.snapshot();
    let robot = jobs(&result, "R01")[0];
    assert!(matches!(robot.state, JobStatus::WaitingOutput));
    assert_eq!(robot.phase_ref.as_ref().unwrap().as_str(), "release");
    assert!(robot.finished_at_ms.is_none());
    let cargo = &robot.input_refs[0];
    assert!(matches!(item_at(&e,cargo,60000).holder,HolderRef::Job(ref j) if *j==robot.id));
    assert!(result.waits.iter().any(|w| {
        w.object_ref.id == robot.id
            && w.interval.start_ms.0.value() == 48000
            && w.reasons
                .iter()
                .any(|r| matches!(r.kind, WaitReasonKind::OutputCapacity))
    }));
    assert!(
        !result
            .phase_work_intervals
            .iter()
            .any(|w| w.job_id == robot.id && w.phase_id.as_str() == "release")
    );
    assert!(!e.port_output_history().iter().any(|r| r.1 == robot.id));
    fs::write(root().join("scenarios/媒体表现/runtime-l03/robot-output-wait.engine.actual.json"),serde_json::to_vec_pretty(&json!({"kind":"actual_Engine_distinct_initial_destination_pending_variant","run":result})).unwrap()).unwrap();
}

#[test]
fn full_pallet_holds_same_sealed_package_without_second_tape_consumption() {
    let mut value = fixture();
    let template = value["scenario"]["initial_conditions"]["items"][0].clone();
    for i in 0..12 {
        let mut blocker = template.clone();
        blocker["id"] = json!(format!("PAL01-unreleased-{i:02}"));
        blocker["item_type_ref"] = json!({"id":"package","revision":"r1"});
        blocker["holder"] = json!({"kind":"location","id":"PAL01-store"});
        blocker["quality"] = json!("pending");
        value["scenario"]["initial_conditions"]["items"]
            .as_array_mut()
            .unwrap()
            .push(blocker);
    }
    let mut e = engine(value);
    e.run_to_end();
    let result = e.snapshot();
    let sealer = jobs(&result, "CASE01")[0];
    assert!(matches!(sealer.state, JobStatus::WaitingOutput));
    assert!(sealer.finished_at_ms.is_none());
    assert_eq!(sealer.pending_outputs.len(), 1);
    let package = &sealer.pending_outputs[0];
    let at90 = item_at(&e, package, 90000);
    let end = item_at(&e, package, 360000);
    assert!(is_state(&at90, "sealed") && is_state(&end, "sealed"));
    assert_eq!(at90.contents, end.contents);
    assert!(matches!(end.holder,HolderRef::Job(ref j) if *j==sealer.id));
    let rows: Vec<_> = e
        .port_output_history()
        .into_iter()
        .filter(|r| r.1 == sealer.id)
        .collect();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].6);
    assert_eq!(rows[0].0.0.value(), 90000);
    let tape = sealer
        .input_refs
        .iter()
        .find_map(|key| {
            result
                .final_items
                .iter()
                .find(|i| i.id == *key && i.quantity.unit.as_str() == "meter")
        })
        .unwrap();
    assert_eq!(tape.quantity.value.coefficient.value(), 6);
    assert!(matches!(tape.lifecycle, Lifecycle::Transformed));
    assert_eq!(jobs(&result, "CASE01").len(), 1);
    fs::write(root().join("scenarios/媒体表现/runtime-l03/sealer-output-wait.engine.actual.json"),serde_json::to_vec_pretty(&json!({"kind":"actual_Engine_distinct_initial_full_pallet_unreleased_variant","run":result})).unwrap()).unwrap();
}

#[test]
fn unknown_inspection_keeps_same_item_and_inspector_claim_without_fabricated_pass() {
    let mut value = fixture();
    value["scenario"]["quality_presets"][2]["results"] = json!([null]);
    let mut e = engine(value);
    e.run_to_end();
    let result = e.snapshot();
    let inspector = jobs(&result, "V01")[2];
    assert!(inspector.finished_at_ms.is_none());
    let item = item_at(&e, &inspector.input_refs[0], 200000);
    assert!(matches!(item.quality, QualityState::Pending));
    assert!(matches!(item.holder,HolderRef::Job(ref j) if *j==inspector.id));
    assert!(result.resource_claims.iter().any(|c| {
        c.job_id == inspector.id
            && c.resource_id.as_str() == "W-INSPECTOR-01"
            && c.intervals
                .iter()
                .any(|t| t.start_ms.0.value() <= 183000 && t.end_ms.0.value() == 360000)
    }));
    assert_eq!(result.acceptance_events.len(), 2);
    fs::write(
        root().join("scenarios/媒体表现/runtime-l03/quality-pending.engine.actual.json"),
        serde_json::to_vec_pretty(
            &json!({"kind":"actual_Engine_distinct_third_quality_unknown_variant","run":result}),
        )
        .unwrap(),
    )
    .unwrap();
}