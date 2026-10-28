//! Actual saved factory import and exact binding checks; no cloud requests.
#[path = "../../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod proof;
use proof::*;
use serde_json::json;
use sf_domain::cloud_execution::CandidateBinding;
use std::path::Path;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let host = Harness::from_seed(Sandbox::new(), Path::new(&args[1]));
    let scenario = args[3].clone().try_into().unwrap();
    let raw = serde_json::to_value(host.host.native_factory_bindings(&host.ui, &scenario).unwrap()).unwrap();
    write(Path::new(&args[2]), &raw);
    let bindings: Vec<CandidateBinding> = serde_json::from_value(raw.clone()).unwrap();
    host.host.validate_native_factory_bindings(&host.ui, &scenario, &bindings).unwrap();
    let mut checked = vec![];
    for case in ["model_version", "port", "unit", "missing_stage"] {
        let mut wrong = raw.clone();
        let index = wrong.as_array().unwrap().iter().position(|b| b["input_ports"].as_array().is_some_and(|p| !p.is_empty())).unwrap();
        match case {
            "model_version" => wrong[index]["device_module_ref"]["version"] = json!("999.0.0"),
            "port" => wrong[index]["input_ports"][0]["port_id"] = json!("wrong-port"),
            "unit" => wrong[index]["input_ports"][0]["unit_id"] = json!("wrong-unit"),
            _ => { wrong.as_array_mut().unwrap().pop(); }
        }
        // A schema rejection is also a rejected import, but never replaces a behavioral check.
        let result = serde_json::from_value::<Vec<CandidateBinding>>(wrong);
        let error = match result {
            Ok(value) => host.host.validate_native_factory_bindings(&host.ui, &scenario, &value).unwrap_err().message,
            Err(error) => panic!("{case} must exercise the semantic check: {error}"),
        };
        checked.push(json!({"case":case,"rejected":true,"message":error}));
    }
    write(Path::new(&args[4]), &json!({"valid_factory_bindings":bindings.len(),"negative_checks":checked,"cloud_verified":false}));
}
