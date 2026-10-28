//! Non-GUI computation/Host samples; never claims visible Native p95/FPS.
#[path="fixtures/production/host_support.rs"] mod support;
use support::*;
use serde_json::{Value,json};
use sf_domain::TaskHandle;
use sf_simulation::{Engine,EngineInput};
use std::{collections::BTreeSet,path::{Path,PathBuf},time::Instant};
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")}
#[test]
fn nfr09_two_frozen_b60_loads_three_independent_runs_all_identities() {
    let mut receipt=Vec::new();
    for variant in ["balanced","dense"] {
        let source=root().join(format!("tests/planner/p201-whole/fixtures/b60/cut-r2/{variant}.input.json"));
        let v=read(&source);let mut previous=None;
        let expected_ids:BTreeSet<_>=v["scenario"]["initial_conditions"]["items"].as_array().unwrap().iter().map(|i|i["id"].as_str().unwrap().to_owned()).collect();
        assert_eq!(expected_ids.len(),3000);
        for round in 1..=3 {
            let run_id=format!("noncu-b60-{variant}-{round}");
            let mut e=Engine::new(EngineInput {run_id:run_id.clone().try_into().unwrap(),
                input_ref:serde_json::from_value(json!({"project_id":v["project"]["id"],"project_revision":v["project"]["revision"],
                    "scenario_id":v["scenario"]["id"],"scenario_revision":v["scenario"]["revision"],
                    "frozen_project_file":{"path":"b60-project.json","kind":"project"},"frozen_scenario_file":{"path":"b60-scenario.json","kind":"scenario"}})).unwrap(),
                project:serde_json::from_value(v["project"].clone()).unwrap(),scenario:serde_json::from_value(v["scenario"].clone()).unwrap(),
                definitions:serde_json::from_value(v["definitions"].clone()).unwrap(),scope:vec![]}).unwrap();
            let started=Instant::now();e.run_to_end();let result=json!(e.snapshot());
            let accepted=result["acceptance_events"].as_array().unwrap();
            assert_eq!(accepted.first().unwrap()["at_ms"],20000);assert_eq!(accepted.last().unwrap()["at_ms"],1018000);
            assert_eq!(accepted.iter().filter(|a|a["at_ms"].as_u64().unwrap()<=600000).count(),1746);assert_eq!(accepted.len(),3000);
            assert_eq!(result["jobs"].as_array().unwrap().len(),30000);
            let ids:BTreeSet<_>=result["final_items"].as_array().unwrap().iter().map(|i|i["id"].as_str().unwrap().to_owned()).collect();assert_eq!(ids,expected_ids);
            let mut observations=Vec::new();
            for at in [1200000,0,600000,1200000,600000,0] {
                let items=e.objects_at(serde_json::from_value(json!(at)).unwrap()).unwrap();
                assert_eq!(items.len(),3000);assert_eq!(items.iter().map(|i|i.id.as_str().to_owned()).collect::<BTreeSet<_>>(),expected_ids);
                let frame=json!(items);
                if at==1200000 {assert!(frame.as_array().unwrap().iter().all(|i|i["holder"]["kind"]=="location"&&i["holder"]["id"].as_str().unwrap().ends_with("-SINK")));}
                observations.push(json!({"at_ms":at,"all_3000_exact_ids":true,"items":frame}));
            }
            assert_eq!(json!(e.snapshot()),result,"readback must never rerun or mutate production");
            let mut semantic=result.clone();semantic.as_object_mut().unwrap().remove("id");
            if let Some(old)=&previous {assert_eq!(old,&semantic,"all Run fields except new identity must match");} else {previous=Some(semantic);}
            let location=root().join(format!(".work/delivery/nfr-release/b60-{variant}-r{round}.json"));
            write(&location,&json!({"actual_producer":"existing sf_simulation::Engine","input":source,"run":result,"observations":observations}));
            receipt.push(json!({"variant":variant,"round":round,"run_id":run_id,"elapsed_ms":started.elapsed().as_millis(),"accepted_600s":1746,"accepted_1200s":3000,"all_identities_checked":3000,"all_semantic_Run_fields_equal":true,"raw":location}));
        }
    }
    write(&root().join(".work/delivery/nfr-release/nfr09-compute.json"),&json!({"samples":receipt,"Native_UI_or_animation_executed":false,"whole_NFR09_passed":false,"compute_and_history_subset_passed":true}));
}
#[test]
fn nfr04_real_Host_admission_three_rounds_of_100_sequential_runs() {
    let value:Value=serde_json::from_str(include_str!("fixtures/production/p202-ix01-baseline.json")).unwrap();
    let mut h=Harness::from_value(value.clone());let mut samples=Vec::new();
    for round in 1..=3 {for sample in 1..=100 {
        let started=Instant::now();let task:TaskHandle=ok(h.call("run_scenario",json!({"scenario_id":value["scenario"]["id"],
            "input_revision":1,"observation_window":value["scenario"]["window"],"intent":{"kind":"project_scenario"}})));
        let admission_us=started.elapsed().as_micros();assert!(h.task(&task,true).result.is_none());
        h.finish_one();let terminal=h.task(&task,true);assert!(matches!(terminal.task.state,sf_domain::ComputationStatus::Completed));
        samples.push(json!({"round":round,"sample":sample,"TaskId":task.id,"Host_admission_us":admission_us,"terminal":"completed"}));
    }}
    write(&root().join(".work/delivery/nfr-release/nfr04-host-admission.json"),&json!({"samples":samples,
        "clock":"single caller Instant at the actual ProjectHost dispatch boundary", "sample_count":300,
        "receipt_visible_frame_or_IPC_clock_measured":false,"long_task_UI_overlap_measured":false,
        "whole_NFR04_passed":false,"Host_function_admission_subset_passed":true}));
}
