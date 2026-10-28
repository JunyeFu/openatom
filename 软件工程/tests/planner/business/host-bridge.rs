//! Private stdio verification driver. Every edit/run/save uses the frozen real Host.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_domain::*;
use std::{io::{self, BufRead, Write}, path::Path};
use support::*;

fn snapshot(h: &mut Harness) -> Value {
    let mut value = h.snapshot();
    value["scenarios"] = Value::Array(value["scenarios"].as_object().unwrap().values().cloned().collect());
    value
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut h = match args.get(1).map(String::as_str) {
        Some("--open") => Harness::from_seed(Sandbox::new(), Path::new(&args[2])),
        Some("--fixture") => Harness::from_value(read(Path::new(&args[2]))),
        None => Harness::new(),
        _ => panic!("expected --open or --fixture"),
    };
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = match request["op"].as_str().unwrap() {
            "snapshot" => snapshot(&mut h),
            "call" => h.call(request["operation"].as_str().unwrap(), request["params"].clone()),
            "run" => {
                let task = h.run(request["scenario"].as_str().unwrap());
                h.finish_one();
                serde_json::to_value(h.completed_run(&task)).unwrap()
            }
            "full" => serde_json::to_value(h.full(&serde_json::from_value(request["run"].clone()).unwrap())).unwrap(),
            "fork" => {
                let source: ScenarioInput = serde_json::from_value(h.object("scenario", request["scenario"].as_str().unwrap())).unwrap();
                let revision = h.context().project.unwrap().revision;
                let orders: Vec<ObjectId> = serde_json::from_value(request["orders"].clone()).unwrap();
                let tasks: Vec<ObjectId> = serde_json::from_value(request["tasks"].clone()).unwrap();
                let new_id: ScenarioId = serde_json::from_value(request["new_id"].clone()).unwrap();
                match sf_domain::fork_without_unstarted_demands(&source, new_id, revision, &orders, &tasks) {
                    Ok(value) => json!({"candidate":value}),
                    Err(issues) => json!({"issues":issues}),
                }
            }
            "save" => {
                let base = h.context().project.unwrap().revision.0.value();
                let handle = h.save(base, Path::new(request["directory"].as_str().unwrap()));
                h.finish_one();
                serde_json::to_value(h.task(&handle, true)).unwrap()
            }
            _ => panic!("unknown verification operation"),
        };
        println!("{}", serde_json::to_string(&result).unwrap());
        io::stdout().flush().unwrap();
    }
}
