//! Complete event-boundary snapshots from the existing unique Engine.
//! Separate evidence from Host ObjectsAt, which currently exposes Items only.
use serde_json::{Value, json};
use sf_simulation::{Engine, EngineInput};
use std::{fs, path::Path};

fn read(path: &Path) -> Value { serde_json::from_slice(&fs::read(path).unwrap()).unwrap() }
fn main() {
    let root = std::env::current_dir().unwrap();
    let out = root.join("scenarios/图片时间轴/runtime");
    for (fixture, bootstrap) in [("fixed", "bootstrap.actual.json"), ("phase", "phase.bootstrap.actual.json")] {
        let ready = read(&out.join(bootstrap));
        let input = read(&root.join(ready["fixture"].as_str().unwrap()));
        let formal = &ready["formal_run"];
        let mut engine = Engine::new(EngineInput {
            run_id: serde_json::from_value(formal["id"].clone()).unwrap(),
            input_ref: serde_json::from_value(formal["input_ref"].clone()).unwrap(),
            project: serde_json::from_value(ready["snapshot"]["project"].clone()).unwrap(),
            scenario: serde_json::from_value(ready["snapshot"]["scenarios"][0].clone()).unwrap(),
            definitions: serde_json::from_value(input["definitions"].clone()).unwrap(),
            scope: serde_json::from_value(input["scenario"]["scope"].clone()).unwrap(),
        }).unwrap();
        let mut snapshots = vec![];
        while !engine.is_finished() {
            engine.step();
            let current = serde_json::to_value(engine.snapshot()).unwrap();
            let mut objects: Vec<Value> = current["jobs"].as_array().unwrap().iter().map(|job| json!({"kind":"job","data":job})).collect();
            objects.extend(current["final_items"].as_array().unwrap().iter().map(|item| json!({"kind":"item","data":item})));
            objects.extend(ready["snapshot"]["project"]["resources"].as_array().unwrap().iter().map(|resource|json!({"kind":"resource","data":resource})));
            snapshots.push(json!({"run_id":formal["id"],"at_ms":current["computed_until_ms"],"objects":objects,"next_index":null}));
        }
        let actual = serde_json::to_value(engine.snapshot()).unwrap();
        for key in ["events","final_items","acceptance_events","orders","resource_claims","waits","phase_work_intervals","jobs","computed_until_ms"] {
            assert_eq!(actual[key], formal[key], "same actual Host input must reproduce exact {fixture}/{key}");
        }
        let job = formal["jobs"].as_array().unwrap().iter().find(|job|job["task_ref"]=="TASK-LOCAL").unwrap();
        let package = &ready["package"];
        let frames = snapshots.into_iter().map(|snapshot|json!({"run":formal,"snapshot":snapshot,"job_id":job["id"],"operation":package["definition"]["payload"]["data"]["behavior"]["inline"]})).collect::<Vec<_>>();
        let result = json!({"producer":"actual sf_simulation::Engine, complete captured event-boundary job/items plus exact frozen project resources","Host_ObjectsAt":false,"Native":false,"same_as_actual_Host_business_fields":true,"presentation":package["definition"]["payload"]["data"]["presentation"],"frames":frames});
        fs::write(out.join(format!("{fixture}.complete-engine-frames.json")), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
        println!("{fixture}: {} real complete Engine frames, actual Host business fields identical",frames.len());
    }
}
