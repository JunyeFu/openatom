//! Read-only consumer of the existing single Engine, never an alternate simulator.
use serde_json::{Value, json};
use sf_simulation::{Engine, EngineInput};
use std::{fs, io::{self, BufRead, Write}, path::Path};
fn main() {
    let root = std::env::args().nth(1).expect("product root");
    let output = Path::new(&root).join("scenarios/从零搭建控制盒厂房/runtime-timeline");
    fs::create_dir_all(&output).unwrap();
    let mut engines = Vec::new();
    let mut records = Vec::new();
    for (case, last) in [("baseline", 440000), ("delayed-pcb", 570000)] {
        let fixture: Value = serde_json::from_slice(&fs::read(Path::new(&root).join(format!("packages/commands/tests/fixtures/production/p202-ix01-{case}.json"))).unwrap()).unwrap();
        let id = format!("P204-IX01-{case}");
        let mut engine = Engine::new(EngineInput {
            run_id: id.clone().try_into().unwrap(),
            input_ref: serde_json::from_value(fixture["input_ref"].clone()).unwrap(),
            project: serde_json::from_value(fixture["project"].clone()).unwrap(),
            scenario: serde_json::from_value(fixture["scenario"].clone()).unwrap(),
            definitions: serde_json::from_value(fixture["definitions"].clone()).unwrap(),
            scope: vec![],
        }).unwrap();
        if case == "baseline" {
            for _ in 0..10000 {
                if engine.step() == 0 { break; }
                let prefix = serde_json::to_value(engine.snapshot()).unwrap();
                if prefix["computed_until_ms"].as_u64().unwrap_or(0) >= 150000 {
                    let at = prefix["computed_until_ms"].as_u64().unwrap();
                    let objects = engine.objects_at(serde_json::from_value(json!(at)).unwrap()).unwrap().into_iter().map(|data|json!({"kind":"item","data":data})).collect::<Vec<_>>();
                    let partial = json!({"case":"baseline-prefix","project":fixture["project"],"scenario":fixture["scenario"],"definitions":fixture["definitions"],"run":prefix,"observations":[{"run_id":id,"at_ms":at,"objects":objects,"next_index":null}]});
                    fs::write(output.join("baseline-prefix.actual.json"),serde_json::to_vec_pretty(&partial).unwrap()).unwrap();
                    break;
                }
            }
        }
        engine.run_to_end();
        let result = serde_json::to_value(engine.snapshot()).unwrap();
        assert_eq!(result["goal_met"], true);
        assert_eq!(result["acceptance_events"].as_array().unwrap().iter().filter_map(|e| e["at_ms"].as_u64()).max(), Some(last));
        assert_eq!(result["transport"].as_array().unwrap().len(), 6);
        if case == "delayed-pcb" {
            let trip = result["transport"].as_array().unwrap().iter().find(|t| t["arrived_ms"] == 370000).unwrap();
            assert_eq!(trip["received_ms"], 420000); assert_eq!(trip["returned_ms"], 425000);
        }
        let mut frames = Vec::new();
        for at in [0, 70000, 370000, 419999, 420000, 425000, 440000, 570000, 600000] {
            let items = engine.objects_at(serde_json::from_value(json!(at)).unwrap());
            frames.push(json!({"run_id":id,"at_ms":at,"objects":items.map(|v| v.into_iter().map(|data|json!({"kind":"item","data":data})).collect::<Vec<_>>()),"next_index":null}));
        }
        let record = json!({"case":case,"project":fixture["project"],"scenario":fixture["scenario"],"definitions":fixture["definitions"],"run":result,"observations":frames});
        fs::write(output.join(format!("{case}.actual.json")), serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        records.push(record); engines.push((id, engine));
    }
    let fixture: Value = serde_json::from_slice(&fs::read(Path::new(&root).join("packages/commands/tests/fixtures/production/p1-phase-calendar.json")).unwrap()).unwrap();
    let id = "P204-phase-calendar".to_owned();
    let mut engine = Engine::new(EngineInput {
        run_id: id.clone().try_into().unwrap(), input_ref: serde_json::from_value(fixture["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(fixture["project"].clone()).unwrap(), scenario: serde_json::from_value(fixture["scenario"].clone()).unwrap(),
        definitions: serde_json::from_value(fixture["definitions"].clone()).unwrap(), scope: vec![],
    }).unwrap();
    engine.run_to_end();
    let result = serde_json::to_value(engine.snapshot()).unwrap();
    assert!(result["jobs"].as_array().unwrap().iter().any(|j|j["finished_at_ms"]==45000));
    let frames = [0,5000,35000,40000,45000,100000].map(|at| {
        let items = engine.objects_at(serde_json::from_value(json!(at)).unwrap()).unwrap();
        json!({"run_id":id,"at_ms":at,"objects":items.into_iter().map(|data|json!({"kind":"item","data":data})).collect::<Vec<_>>(),"next_index":null})
    });
    let record=json!({"case":"phase-calendar","project":fixture["project"],"scenario":fixture["scenario"],"definitions":fixture["definitions"],"run":result,"observations":frames});
    fs::write(output.join("phase-calendar.actual.json"),serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    records.push(record); engines.push((id,engine));
    println!("{}", json!({"ready":true,"runs":records.iter().map(|v| &v["run"]["id"]).collect::<Vec<_>>(),"actual_producer":"sf_simulation::Engine","new_Engine_count":3}));
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines() {
        let query: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let id = query["run_id"].as_str().unwrap(); let at = query["at_ms"].as_u64().unwrap();
        let engine = &engines.iter().find(|(run_id,_)| run_id==id).expect("known RunId").1;
        let before = serde_json::to_value(engine.snapshot()).unwrap();
        let objects = engine.objects_at(serde_json::from_value(json!(at)).unwrap()).map(|items|items.into_iter().map(|data|json!({"kind":"item","data":data})).collect::<Vec<_>>());
        assert_eq!(before,serde_json::to_value(engine.snapshot()).unwrap(),"seek must not advance production");
        println!("{}",json!({"run_id":id,"at_ms":at,"objects":objects,"next_index":null,"read_only":true}));
        io::stdout().flush().unwrap();
    }
}
