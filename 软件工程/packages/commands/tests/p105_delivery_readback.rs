//! Explicit final-delivery readback worker. Ordinary cargo test needs no environment.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::json;
use sf_domain::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
use support::*;

#[test]
#[ignore = "Delivery verification only: SF_P105_PACKET and SF_P105_PROJECT select a self-built copied project"]
fn p105_final_delivery_reopens_both_actual_runs() {
    let packet =
        PathBuf::from(std::env::var_os("SF_P105_PACKET").expect("explicit delivery packet"));
    let project =
        PathBuf::from(std::env::var_os("SF_P105_PROJECT").expect("explicit copied project"));
    let output =
        PathBuf::from(std::env::var_os("SF_P105_READBACK_OUTPUT").expect("explicit output"));
    let expected = read(&packet);
    let mut h = Harness::from_seed(Sandbox::new(), Path::new(&project));
    let old: RunResult = serde_json::from_value(expected["old"].clone()).unwrap();
    let new: RunResult = serde_json::from_value(expected["new"].clone()).unwrap();
    assert_eq!(json!(h.full(&old.id)), expected["old"]);
    assert_eq!(json!(h.full(&new.id)), expected["new"]);
    assert_ne!(old.id, new.id);
    assert_eq!(
        h.context().project.unwrap().revision.0.value(),
        new.input_ref.project_revision.0.value()
    );
    assert_eq!(h.context().run_summaries.len(), 2);
    assert!(old.whole_window_computed && new.whole_window_computed);
    fs::write(output,serde_json::to_vec_pretty(&json!({"passed":true,"pid":std::process::id(),"project":project,"old_run_id":old.id,"new_run_id":new.id,"project_revision":new.input_ref.project_revision,"saved_runs":2,"read_only_no_rerun":true})).unwrap()).unwrap();
}
