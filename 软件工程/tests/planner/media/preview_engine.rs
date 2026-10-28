use sf_simulation::{Engine,EngineInput};
use serde_json::{Value,json};
use std::fs;
fn main(){
    let root=std::env::current_dir().unwrap();let output=root.join("scenarios/媒体表现/runtime/shared-preview");
    let mut v:Value=serde_json::from_slice(&fs::read(root.join("packages/commands/tests/fixtures/production/p1-phase-calendar.json")).unwrap()).unwrap();
    v["scenario"]["id"]=json!("P302-PREFIX-20750");v["input_ref"]["scenario_id"]=v["scenario"]["id"].clone();v["scenario"]["window"]["end_ms"]=json!(20750);
    v["input_ref"]["frozen_scenario_file"]["path"]=json!("frozen/p302-prefix-20750.scenario.json");
    let mut e=Engine::new(EngineInput{run_id:serde_json::from_value(json!("P302-PREFIX-RUN-20750")).unwrap(),input_ref:serde_json::from_value(v["input_ref"].clone()).unwrap(),project:serde_json::from_value(v["project"].clone()).unwrap(),scenario:serde_json::from_value(v["scenario"].clone()).unwrap(),definitions:serde_json::from_value(v["definitions"].clone()).unwrap(),scope:serde_json::from_value(v["scenario"]["scope"].clone()).unwrap()}).unwrap();
    e.run_to_end();let run=serde_json::to_value(e.snapshot()).unwrap();let job=run["jobs"].as_array().unwrap().iter().find(|job|job["operation_ref"]["id"]=="w5.op.phased").unwrap();
    assert_eq!(run["computed_until_ms"],20750);assert_eq!(job["phase_ref"],"auto");assert_eq!(job["effective_progress_ms"],15750);assert_eq!(job["state"],"working");assert_eq!(run["acceptance_events"].as_array().unwrap().len(),0);
    let mut objects:Vec<Value>=run["jobs"].as_array().unwrap().iter().map(|j|json!({"kind":"job","data":j})).collect();
    objects.extend(run["final_items"].as_array().unwrap().iter().map(|item|json!({"kind":"item","data":item})));
    objects.extend(v["project"]["resources"].as_array().unwrap().iter().map(|resource|json!({"kind":"resource","data":resource})));
    let op=v["definitions"].as_array().unwrap().iter().find(|d|d["id"]=="w5.op.phased").unwrap()["payload"]["data"].clone();
    fs::write(output.join("actual-prefix.input.json"),serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    fs::write(output.join("actual-prefix.json"),serde_json::to_vec_pretty(&json!({"run":run,"snapshot":{"run_id":run["id"],"at_ms":20750,"objects":objects,"next_index":null},"job_id":job["id"],"operation":op})).unwrap()).unwrap();
    println!("1 separate finite-window actual Engine prefix: auto effective15750/30000 =0.525, no acceptance; not a Host snapshot or same original Run.");
}
