//! Real Commands/Host/packaging/Engine consumers. UI-origin records below are
//! programmatic purpose-gate tests, not human acceptance or industrial approval.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};
use support::*;

const PURPOSES: [&str; 5] = ["forming", "opening", "linear_extraction", "net_space", "manufacturing"];

fn output(label: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/professional-binding-closeout/delivery/evidence")
        .join(format!("{label}-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&root).unwrap();
    root
}
fn base(h: &mut Harness) -> u64 { h.context().project.unwrap().revision.0.value() }
fn edit(h: &mut Harness, commands: Value) -> EditCommit {
    let revision = base(h);
    ok(h.apply(revision, commands))
}
fn host() -> Harness {
    let mut value = fixture();
    let declaration: Value = serde_json::from_str(include_str!("../../packaging/consumer-device.design.json")).unwrap();
    value["definitions"][5]["parameters"] = json!(declaration["data"]["definition"]["parameters"].as_array().unwrap().iter()
        .filter(|p| p["id"] == "packaging_job_id" || p["id"] == "packaging_job_revision").cloned().collect::<Vec<_>>());
    value["project"]["devices"][1]["configuration"] = json!({"packaging_job_id":{"kind":"string","value":"P003-JOB"},"packaging_job_revision":{"kind":"integer","value":1}});
    Harness::from_value(value)
}
fn job(case: &str, revision: u64, product_revision: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!("fixtures/production/packaging-acceptance-cases.json")).unwrap();
    let mut value = cases["cases"].as_array().unwrap().iter().find(|row| row["id"] == case).unwrap()["job"]["data"].clone();
    value["id"] = json!("P003-JOB"); value["revision"] = json!(revision);
    value["subject_ref"] = json!({"id":"PACKAGE","revision":product_revision});
    if let Some(envelope) = value["conditions"]["product_envelope"].as_object_mut() {
        envelope.insert("subject".into(), json!({"id":"PACKAGE","revision":product_revision}));
    }
    serde_json::to_value(serde_json::from_value::<PackageStructureJob>(value).unwrap()).unwrap()
}
fn produce(h: &mut Harness, input: &Value, root: &Path, label: &str) -> PackagingResult {
    edit(h, json!([{"kind":"put_packaging_job","data":input}]));
    let project = h.object("project", "P1"); let physical = h.object("scenario", "LOCAL")["initial_conditions"].clone();
    let revision = base(h);
    let request = json!({"packaging_job":input,"input_revision":revision,"computation_budget":input["budget"]});
    let task: TaskHandle = ok(h.call("compute_packaging", request.clone()));
    assert!(h.task(&task, true).result.is_none());
    h.finish_one();
    let state = h.task(&task, true);
    write(&root.join(format!("{label}-input.json")), input);
    write(&root.join(format!("{label}-request.json")), &request);
    write(&root.join(format!("{label}-task.json")), &serde_json::to_value(&state).unwrap());
    assert!(matches!(state.task.state, ComputationStatus::Completed), "{state:?}");
    let TaskOutput::Packaging(result) = state.result.unwrap() else { panic!("actual packaging output") };
    assert!(matches!(result.evidence_origin, EvidenceOrigin::ActualSoftware));
    assert_eq!(h.object("project", "P1")["acknowledgments"], project["acknowledgments"]);
    assert_eq!(h.object("project", "P1")["bindings"], project["bindings"]);
    assert_eq!(h.object("scenario", "LOCAL")["initial_conditions"], physical);
    result
}
fn acknowledgment(result: &PackagingResult, name: &str, scopes: Value) -> Value {
    json!({"id":name,"artifact_ref":result.artifact_id,"accepted_subject":result.subject_ref,"accepted_scope":scopes,
        "decision_source":{"kind":"human_statement","data":"Programmatic trusted UI purpose gate, not actual human acceptance"},"conditions":[],"recorded_at_iso8601":null})
}
fn record(h: &mut Harness, value: Value) { edit(h, json!([{"kind":"record_artifact_acknowledgment","data":value}])); }
fn adoption(h: &mut Harness, result: &PackagingResult, ack: &str) -> Value {
    json!({"artifact_id":result.artifact_id,"acceptance_ref":ack,"consumer_ref":{"kind":"device","id":"PACK-01"},
        "operation_ref":{"id":"fixture.pack","version":"0.1.0"},"requested_scope":PURPOSES,"base_revision":base(h)})
}
fn rejected_adoption(h: &mut Harness, request: Value) -> ApiError {
    let before = h.snapshot();
    let result = error(h.call("adopt_artifact", request));
    assert_eq!(h.snapshot(), before, "rejected adoption must be atomic");
    assert!(h.host.drain_io_work().is_empty());
    result
}
fn adopt(h: &mut Harness, result: &PackagingResult, ack: &str) -> ConsumerBinding {
    let request = adoption(h, result, ack);
    let commit: BindingCommit = ok(h.call("adopt_artifact", request)); commit.binding
}
fn blocked_run(h: &mut Harness) -> ApiError {
    let revision = base(h); let before = h.snapshot();
    let failure = error(h.call("run_scenario", json!({"scenario_id":"LOCAL","input_revision":revision,
        "observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert_eq!(h.snapshot(), before); assert!(h.host.drain_io_work().is_empty()); failure
}
fn run(h: &mut Harness) -> RunResult {
    let task = h.run("LOCAL"); h.finish_one(); let result = h.completed_run(&task);
    assert_eq!(result.acceptance_events.len(), 1);
    assert_eq!(result.acceptance_events[0].at_ms.0.value(), 75000);
    result
}
fn stored(h: &mut Harness, result: &PackagingResult) -> Value {
    let revision = h.context().project.unwrap().revision;
    serde_json::to_value(h.host.read_stored_packaging_artifact(&h.ui, &"P1".to_owned().try_into().unwrap(), &ApiScope::Project, &revision, &result.artifact_id).unwrap()).unwrap()
}

#[test]
fn noncu_p003_actual_fail_and_unknown_remain_nonpass_after_explicit_acknowledgment_and_relocation() {
    let root = output("nonpass");
    for (case, expected) in [("LOCAL_OUTSIDE_PATCH", "fail"), ("LOCAL_ZERO_QUERY", "undetermined")] {
        let mut h = host(); let result = produce(&mut h, &job(case, 1, "r1"), &root, case);
        assert_eq!(serde_json::to_value(&result.outcome).unwrap(), expected);
        let artifact = h.object("artifact", result.artifact_id.as_str());
        let original = stored(&mut h, &result);
        assert!(!result.supported_scope.is_empty());
        record(&mut h, acknowledgment(&result, "ACK-NONPASS", json!(result.supported_scope)));
        let request = adoption(&mut h, &result, "ACK-NONPASS");
        let refusal = rejected_adoption(&mut h, request);
        assert!(matches!(refusal.code, ErrorCode::InvalidInput | ErrorCode::MissingReference));
        assert_eq!(h.object("artifact", result.artifact_id.as_str()), artifact);
        assert_eq!(stored(&mut h, &result), original);
        assert!(h.object("project", "P1")["bindings"].as_array().unwrap().is_empty());
        let saved = root.join(format!("{case}-saved")); fs::create_dir(&saved).unwrap();
        let revision = base(&mut h); let task = h.save(revision, &saved); h.finish_one();
        assert!(matches!(h.task(&task, true).task.state, ComputationStatus::Completed));
        let expected = json!({"parent_pid":std::process::id(),"result":result,"stored":original,"artifact":artifact,
            "project":h.object("project", "P1"),"physical":h.object("scenario", "LOCAL")["initial_conditions"]});
        let expected_path = root.join(format!("{case}-expected.json")); write(&expected_path, &expected);
        drop(h);
        let moved = root.join(format!("{case}-moved")); fs::rename(&saved, &moved).unwrap(); assert!(!saved.exists());
        child_check("noncu_p003_nonpass_worker", &moved, &expected_path);
        write(&root.join(format!("{case}-refusal.json")), &serde_json::to_value(refusal).unwrap());
    }
}

#[test]
fn noncu_p003_rejection_matrix_and_undo_redo_keep_professional_facts_distinct() {
    let root = output("rejections"); let mut h = host();
    let result = produce(&mut h, &job("COVERAGE-restricted_folded_container", 1, "r1"), &root, "actual-pass");
    assert!(matches!(result.outcome, CheckOutcome::Pass));
    let artifact = h.object("artifact", result.artifact_id.as_str());
    let mut refusals = vec![];
    let request = adoption(&mut h, &result, "MISSING-ACK");
    refusals.push(serde_json::to_value(rejected_adoption(&mut h, request)).unwrap());
    record(&mut h, acknowledgment(&result, "ACK-PARTIAL", json!(["forming"])));
    let request = adoption(&mut h, &result, "ACK-PARTIAL");
    refusals.push(serde_json::to_value(rejected_adoption(&mut h, request)).unwrap());
    let mut conditional = acknowledgment(&result, "ACK-CONDITIONAL", json!(PURPOSES));
    conditional["conditions"] = json!([{"code":"needs_review","object_ref":null,"field_path":"/acceptance_ref","message":"Explicit unresolved execution condition","blocks":["execution"]}]);
    record(&mut h, conditional);
    let request = adoption(&mut h, &result, "ACK-CONDITIONAL");
    refusals.push(serde_json::to_value(rejected_adoption(&mut h, request)).unwrap());
    record(&mut h, acknowledgment(&result, "ACK-FULL", json!(PURPOSES)));
    for variant in ["scope", "consumer", "operation", "revision"] {
        let mut request = adoption(&mut h, &result, "ACK-FULL");
        match variant {
            "scope" => request["requested_scope"] = json!(["forming"]),
            "consumer" => request["consumer_ref"]["id"] = json!("NO-SUCH-CONSUMER"),
            "operation" => request["operation_ref"]["version"] = json!("99.0.0"),
            "revision" => request["base_revision"] = json!(base(&mut h) - 1),
            _ => unreachable!(),
        }
        refusals.push(serde_json::to_value(rejected_adoption(&mut h, request)).unwrap());
    }
    let binding = adopt(&mut h, &result, "ACK-FULL");
    assert_eq!(adopt(&mut h, &result, "ACK-FULL").id, binding.id);
    let facts = h.object("project", "P1");
    let before = h.snapshot(); let revision = base(&mut h);
    error(h.apply(revision, json!([{"kind":"undo"}])));
    assert_eq!(h.snapshot(), before, "a binding fact is not rolled back as an ordinary draft");
    edit(&mut h, json!([{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":999},"basis":"user_provided"}}]));
    blocked_run(&mut h);
    edit(&mut h, json!([{"kind":"undo"}]));
    assert!(!h.context().issues.iter().any(|i| i.code.starts_with("professional_inputs.")));
    edit(&mut h, json!([{"kind":"redo"}])); blocked_run(&mut h);
    edit(&mut h, json!([{"kind":"undo"}])); let actual_run = run(&mut h);
    assert_eq!(h.object("artifact", result.artifact_id.as_str()), artifact);
    let after = h.object("project", "P1");
    assert_eq!(after["acknowledgments"], facts["acknowledgments"]);
    assert_eq!(after["bindings"], facts["bindings"]);
    write(&root.join("atomic-refusals.json"), &json!(refusals));
    write(&root.join("actual-run.json"), &serde_json::to_value(actual_run).unwrap());
}

#[test]
fn noncu_p003_versions_explicit_old_reselection_and_fresh_process_reopen() {
    let root = output("versions"); let mut h = host();
    let old = produce(&mut h, &job("COVERAGE-restricted_folded_container", 1, "r1"), &root, "r1");
    assert!(matches!(old.outcome, CheckOutcome::Pass));
    record(&mut h, acknowledgment(&old, "ACK-R1", json!(PURPOSES)));
    let first_binding = adopt(&mut h, &old, "ACK-R1"); let first_run = run(&mut h);
    let original_artifact = h.object("artifact", old.artifact_id.as_str());
    let old_stored = stored(&mut h, &old); let old_run = serde_json::to_value(&first_run).unwrap();
    let mut product = h.object("product", "PACKAGE"); product["revision"] = json!("r2");
    let mut scenario = h.object("scenario", "LOCAL"); scenario["orders"][0]["product_ref"]["revision"] = json!("r2");
    let new_input = job("COVERAGE-restricted_folded_container", 2, "r2");
    edit(&mut h, json!([put("product",product),put("scenario",scenario),{"kind":"put_packaging_job","data":new_input},
        {"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":2},"basis":"user_provided"}}]));
    blocked_run(&mut h);
    assert_eq!(h.object("artifact", old.artifact_id.as_str()), original_artifact);
    assert_eq!(stored(&mut h, &old), old_stored);
    assert_eq!(serde_json::to_value(h.full(&first_run.id)).unwrap(), old_run);
    let current = produce(&mut h, &new_input, &root, "r2");
    assert!(matches!(current.outcome, CheckOutcome::Pass)); blocked_run(&mut h);
    record(&mut h, acknowledgment(&current, "ACK-R2", json!(PURPOSES)));
    blocked_run(&mut h); let next_binding = adopt(&mut h, &current, "ACK-R2");
    assert_ne!(first_binding.id, next_binding.id); let second_run = run(&mut h);
    let mut product = h.object("product", "PACKAGE"); product["revision"] = json!("r1");
    let mut scenario = h.object("scenario", "LOCAL"); scenario["orders"][0]["product_ref"]["revision"] = json!("r1");
    edit(&mut h, json!([put("product",product),put("scenario",scenario),{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":1},"basis":"user_provided"}}]));
    blocked_run(&mut h);
    let reselected = adopt(&mut h, &old, "ACK-R1"); assert_ne!(reselected.id, first_binding.id);
    let third_run = run(&mut h); let project = h.object("project", "P1");
    assert_eq!(project["bindings"].as_array().unwrap().len(), 3);
    assert_eq!(project["bindings"][0], serde_json::to_value(first_binding).unwrap());
    assert_eq!(project["bindings"][1], serde_json::to_value(next_binding).unwrap());
    assert_eq!(stored(&mut h, &old), old_stored);
    assert_eq!(serde_json::to_value(h.full(&first_run.id)).unwrap(), old_run);
    let saved = root.join("saved"); let moved = root.join("moved"); fs::create_dir(&saved).unwrap();
    let revision = base(&mut h); let save = h.save(revision, &saved); h.finish_one();
    assert!(matches!(h.task(&save, true).task.state, ComputationStatus::Completed), "{:?}", h.task(&save, true));
    let expected = json!({"parent_pid":std::process::id(),"project":project,"old_result":old,"new_result":current,
        "old_stored":old_stored,"new_stored":stored(&mut h,&current),"runs":[first_run,second_run,third_run]});
    let expected_path = root.join("expected.json"); write(&expected_path, &expected);
    drop(h); fs::rename(&saved, &moved).unwrap(); assert!(!saved.exists());
    child_check("noncu_p003_readback_worker", &moved, &expected_path);
    let proof = read(&root.join("fresh-process-proof.json"));
    assert_ne!(proof["pid"], expected["parent_pid"]); assert_eq!(proof["old_run_preserved"], true);
}

#[test]
#[ignore = "Executed by the lifecycle parent in a fresh OS process with its real saved directory"]
fn noncu_p003_readback_worker() {
    let moved = PathBuf::from(std::env::var_os("NONCU_P003_SAVED_DIR").unwrap());
    let expected_path = PathBuf::from(std::env::var_os("NONCU_P003_EXPECTED").unwrap());
    let expected = read(&expected_path); let root = expected_path.parent().unwrap();
    let mut h = Harness::from_seed(Sandbox::new(), &moved);
    let project = h.object("project", "P1");
    assert_eq!(project["acknowledgments"], expected["project"]["acknowledgments"]);
    assert_eq!(project["bindings"], expected["project"]["bindings"]);
    let old: PackagingResult = serde_json::from_value(expected["old_result"].clone()).unwrap();
    let current: PackagingResult = serde_json::from_value(expected["new_result"].clone()).unwrap();
    assert_eq!(stored(&mut h, &old), expected["old_stored"]);
    assert_eq!(stored(&mut h, &current), expected["new_stored"]);
    for previous in expected["runs"].as_array().unwrap() {
        let run: RunResult = serde_json::from_value(previous.clone()).unwrap();
        assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), *previous);
    }
    let denied = blocked_run(&mut h);
    assert!(matches!(denied.code, ErrorCode::PermissionDenied), "serialized producer/UI claims are not fresh Host authority: {denied:?}");
    let exact_input: PackageStructureJob = serde_json::from_value(expected["old_stored"][2].clone()).unwrap();
    let recomputed = produce(&mut h, &serde_json::to_value(exact_input).unwrap(), root, "fresh-host");
    assert!(matches!(recomputed.outcome, CheckOutcome::Pass));
    record(&mut h, acknowledgment(&recomputed, "ACK-FRESH-HOST", json!(PURPOSES)));
    adopt(&mut h, &recomputed, "ACK-FRESH-HOST"); let rerun = run(&mut h);
    for previous in expected["runs"].as_array().unwrap() {
        let old: RunResult = serde_json::from_value(previous.clone()).unwrap();
        assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), *previous);
    }
    let final_saved = root.join("fresh-host-resaved"); fs::create_dir(&final_saved).unwrap();
    let mut runs = expected["runs"].as_array().unwrap().clone(); runs.push(serde_json::to_value(&rerun).unwrap());
    let final_expected = json!({"parent_pid":std::process::id(),"project":h.object("project", "P1"),
        "results":[[old,expected["old_stored"]],[current,expected["new_stored"]],[recomputed,stored(&mut h,&recomputed)]],
        "runs":runs,"physical":h.object("scenario", "LOCAL")["initial_conditions"]});
    let final_path = root.join("resaved-expected.json"); write(&final_path, &final_expected);
    let revision = base(&mut h); let save = h.save(revision, &final_saved); h.finish_one();
    assert!(matches!(h.task(&save, true).task.state, ComputationStatus::Completed), "{:?}", h.task(&save, true));
    drop(h); child_check("noncu_p003_resaved_worker", &final_saved, &final_path);
    write(&root.join("fresh-process-proof.json"), &json!({"pid":std::process::id(),"saved_directory":moved,
        "serialized_authority_rejected":true,"actual_new_producer":recomputed.artifact_id,"actual_new_run":rerun.id,"old_run_preserved":true}));
}

fn child_check(name: &str, directory: &Path, expected: &Path) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--ignored", "--exact", name, "--nocapture"])
        .env("NONCU_P003_SAVED_DIR", directory).env("NONCU_P003_EXPECTED", expected);
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let output = command.output().unwrap();
    fs::write(expected.with_extension("stdout.log"), &output.stdout).unwrap();
    fs::write(expected.with_extension("stderr.log"), &output.stderr).unwrap();
    assert!(output.status.success(), "{}: {} {}", name, String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}
#[test]
#[ignore = "Fresh OS process invoked by parent"]
fn noncu_p003_nonpass_worker() {
    let path = PathBuf::from(std::env::var_os("NONCU_P003_EXPECTED").unwrap()); let e = read(&path);
    assert_ne!(json!(std::process::id()), e["parent_pid"]);
    let directory = PathBuf::from(std::env::var_os("NONCU_P003_SAVED_DIR").unwrap());
    let mut h = Harness::from_seed(Sandbox::new(), &directory);
    let result: PackagingResult = serde_json::from_value(e["result"].clone()).unwrap();
    assert!(!matches!(result.outcome, CheckOutcome::Pass));
    assert_eq!(stored(&mut h, &result), e["stored"]);
    assert_eq!(h.object("artifact", result.artifact_id.as_str()), e["artifact"]);
    let project = h.object("project", "P1");
    assert_eq!(project["acknowledgments"], e["project"]["acknowledgments"]);
    assert_eq!(project["bindings"], e["project"]["bindings"]);
    assert_eq!(h.object("scenario", "LOCAL")["initial_conditions"], e["physical"]);
    let request = adoption(&mut h, &result, "ACK-NONPASS"); let refusal = rejected_adoption(&mut h, request);
    write(&path.with_extension("proof.json"), &json!({"pid":std::process::id(),"outcome":result.outcome,"refusal":refusal,"inventory_unchanged":true,"saved_directory":directory}));
}
#[test]
#[ignore = "Second fresh OS process reads resaved results"]
fn noncu_p003_resaved_worker() {
    let path = PathBuf::from(std::env::var_os("NONCU_P003_EXPECTED").unwrap()); let e = read(&path);
    assert_ne!(json!(std::process::id()), e["parent_pid"]);
    let directory = PathBuf::from(std::env::var_os("NONCU_P003_SAVED_DIR").unwrap());
    let mut h = Harness::from_seed(Sandbox::new(), &directory);
    let project = h.object("project", "P1");
    for key in ["acknowledgments", "bindings"] { assert_eq!(project[key], e["project"][key]); }
    for pair in e["results"].as_array().unwrap() {
        let result: PackagingResult = serde_json::from_value(pair[0].clone()).unwrap();
        assert_eq!(stored(&mut h, &result), pair[1]);
    }
    for previous in e["runs"].as_array().unwrap() {
        let run: RunResult = serde_json::from_value(previous.clone()).unwrap();
        assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), *previous);
    }
    assert_eq!(e["runs"].as_array().unwrap().len(), 4);
    assert_eq!(h.object("scenario", "LOCAL")["initial_conditions"], e["physical"]);
    write(&path.with_extension("proof.json"), &json!({"pid":std::process::id(),"runs_read":4,"professional_results_read":3,"saved_directory":directory}));
}

