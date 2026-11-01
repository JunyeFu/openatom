//! A distinct real B Run for comparing resource ownership (different conditions, parallel display).
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

#[test]
fn comparison_phase_record_has_a_distinct_actual_run_identity() {
    let value: Value = serde_json::from_str(include_str!(
        "../../../packages/commands/tests/fixtures/production/p1-phase-calendar.json"
    )).unwrap();
    let mut h = Harness::from_value(value);
    let warmup = h.run("PHASE"); h.finish_one(); let first = h.completed_run(&warmup);
    let task = h.run("PHASE"); h.finish_one(); let run = h.completed_run(&task);
    assert_ne!(first.id, run.id);
    let mut frozen = |kind: &str, name: &str| {
        let object: DomainObject = ok(h.call("get_object", json!({
            "object_id":{"kind":kind,"id":name},
            "source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":0}},"node_index":null
        })));
        serde_json::to_value(object).unwrap()["data"].clone()
    };
    let project = frozen("project", "P1"); let scenario = frozen("scenario", "PHASE");
    assert!(run.resource_claims.iter().any(|c| c.resource_id.as_str() == "W-C"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/delivery/host");
    write(&root.join("comparison-phase.actual.json"), &json!({
        "project":project,"scenario":scenario,"run":run,
        "producer":"actual frozen Host; second PHASE Run; parallel comparison with IX01 conditions"
    }));
}
