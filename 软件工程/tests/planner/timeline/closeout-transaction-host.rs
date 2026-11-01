//! Private stdio transport to the real frozen Host; no production API is added.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_domain::*;
use std::io::{self, BufRead, Write};
use support::*;

fn main() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../packages/commands/tests/fixtures/production/p202-ix01-baseline.json"
    )).unwrap();
    let mut h = Harness::from_value(fixture);
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let response = match request["op"].as_str().unwrap() {
            "call" => h.call(request["operation"].as_str().unwrap(), request["params"].clone()),
            "object" => h.object(request["kind"].as_str().unwrap(), request["id"].as_str().unwrap()),
            "context" => json!(h.context()),
            "finish_run" => {
                let task: TaskHandle = serde_json::from_value(request["task"].clone()).unwrap();
                h.finish_one(); json!(h.completed_run(&task))
            }
            "full" => json!(h.full(&serde_json::from_value(request["run"].clone()).unwrap())),
            _ => panic!("unknown private verification request"),
        };
        println!("{}", serde_json::to_string(&response).unwrap());
        io::stdout().flush().unwrap();
    }
}
