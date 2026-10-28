//! Batch counterexamples use frozen public inputs; original acceptance tests stay unchanged.
use serde_json::{Value, json};
use sf_domain::*;
use sf_packaging::{PackagingArtifactContext, PackagingProduct, compute_packaging};
use std::sync::atomic::{AtomicBool, Ordering};

fn case(id: &str) -> PackageStructureJob {
    let cases: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    serde_json::from_value(
        cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap()["job"]["data"]
            .clone(),
    )
    .unwrap()
}
fn context() -> PackagingArtifactContext {
    PackagingArtifactContext {
        artifact_id: "NIGHT-ARTIFACT".to_owned().try_into().unwrap(),
        input_ref: serde_json::from_value(json!({"path":"night/job.json","kind":"packaging_job"}))
            .unwrap(),
        result_ref: serde_json::from_value(
            json!({"path":"night/result.json","kind":"packaging_result"}),
        )
        .unwrap(),
        manufacturing_directory: "night/manufacturing".to_owned().try_into().unwrap(),
    }
}
fn evidence(name: &str, value: Value) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.work/independent-closeout-20261020/main-reception/packaging-evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(name), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
fn cancel_at(job: &PackageStructureJob, phase: &str) -> PackagingProduct {
    cancel_at_occurrence(job, phase, 1)
}
fn cancel_at_occurrence(
    job: &PackageStructureJob,
    phase: &str,
    occurrence: usize,
) -> PackagingProduct {
    let cancel = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let (reached_tx, reached_rx) = std::sync::mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
        let flag = &cancel;
        let worker = scope.spawn(move || {
            let mut reached = 0;
            compute_packaging(
                job,
                job.budget.as_ref().unwrap(),
                context(),
                flag,
                &mut |p| {
                    if p.phase == phase {
                        reached += 1;
                    }
                    if p.phase == phase && reached == occurrence {
                        reached_tx.send(()).unwrap();
                        resume_rx.recv().unwrap();
                    }
                },
            )
        });
        reached_rx.recv().unwrap_or_else(|_| {
            panic!("worker finished without cancellable manufacturing checkpoint {phase}")
        });
        cancel.store(true, Ordering::Relaxed);
        resume_tx.send(()).unwrap();
        worker.join().unwrap().unwrap()
    })
}
fn assert_cancelled(p: &PackagingProduct) {
    assert!(p.cancelled);
    assert_eq!(
        p.result.budget_used.stopped_by.as_deref(),
        Some("cancelled")
    );
    assert!(!matches!(p.result.outcome, CheckOutcome::Pass));
    assert!(p.result.checks.iter().any(|c| matches!(
        c.purpose,
        PackagingStepPurpose::Manufacturing
    ) && matches!(c.outcome, CheckOutcome::Undetermined)
        && c.unresolved.iter().any(|i| i.code == "packaging.cancelled")));
}

#[test]
fn a_connection_cannot_be_activated_by_an_opening_or_measurement_step() {
    for purpose in ["opening", "net_space", "linear_extraction", "manufacturing"] {
        let mut input = serde_json::to_value(case("LOCAL_APPROACH")).unwrap();
        input["id"] = json!(format!("NIGHT-NONFORMING-{purpose}"));
        input["revision"] = json!(2);
        input["conditions"]["forming_required"] = json!(false);
        input["steps"][0]["purpose"] = json!(purpose);
        let job: PackageStructureJob = serde_json::from_value(input.clone()).unwrap();
        let result = compute_packaging(
            &job,
            job.budget.as_ref().unwrap(),
            context(),
            &AtomicBool::new(false),
            &mut |_| {},
        );
        let observation = match &result {
            Ok(p) => json!({"input":input,"outcome":p.result.outcome,"checks":p.result.checks,
                "manufacturing_files":p.result.manufacturing_files}),
            Err(issues) => json!({"input":input,"issues":issues}),
        };
        evidence(
            &format!(
                "nonforming-{purpose}-{}.json",
                if result.is_ok() {
                    "accepted-red"
                } else {
                    "rejected"
                }
            ),
            observation,
        );
        assert!(
            matches!(result, Err(ref issues) if issues.iter().any(|i|
            i.code == "packaging.invalid_connection" && i.field_path.as_deref() == Some("/connections/0"))),
            "a {purpose} step incorrectly activated a forming-only connection"
        );
    }
}

#[test]
fn actual_worker_cancel_inside_manufacturing_union_retains_unknown_and_partial_outputs() {
    let job = case("LOCAL_APPROACH");
    let p = cancel_at(&job, "manufacturing_union_cells");
    assert_cancelled(&p);
    assert!(p.manufacturing_bytes.is_empty());
    assert!(p.result.material_counts.is_empty());
    evidence(
        "cancel-manufacturing-union.json",
        serde_json::to_value(&p.result).unwrap(),
    );
}

#[test]
fn actual_worker_cancel_inside_manufacturing_boundary_does_not_emit_an_unfinished_blank() {
    let job = case("MAN04");
    let p = cancel_at(&job, "manufacturing_boundary_edges");
    assert_cancelled(&p);
    assert!(p.manufacturing_bytes.is_empty());
    assert!(p.result.material_counts.is_empty());
    evidence(
        "cancel-manufacturing-boundary.json",
        serde_json::to_value(&p.result).unwrap(),
    );
}

#[test]
fn cancellation_during_manufacturing_preserves_a_real_forming_counterexample() {
    let job = case("KIN_TRANSLATION_MID");
    let p = cancel_at(&job, "manufacturing_union_cells");
    assert_cancelled(&p);
    assert!(matches!(p.result.outcome, CheckOutcome::Fail));
    assert!(
        p.result
            .checks
            .iter()
            .any(|c| matches!(c.purpose, PackagingStepPurpose::Forming)
                && matches!(c.outcome, CheckOutcome::Fail)
                && !c.witnesses.is_empty())
    );
    evidence(
        "cancel-manufacturing-preserves-fail.json",
        serde_json::to_value(&p.result).unwrap(),
    );
}

#[test]
fn overlapping_matched_glue_allowances_are_unioned_without_doubling_material_or_glue() {
    let mut input = serde_json::to_value(case("LOCAL_APPROACH")).unwrap();
    input["id"] = json!("NIGHT-OVERLAPPING-GLUE");
    input["revision"] = json!(2);
    let mut second = input["conditions"]["allowed_contacts"][0].clone();
    second["id"] = json!("NIGHT-L2");
    input["conditions"]["allowed_contacts"]
        .as_array_mut()
        .unwrap()
        .push(second);
    input["connections"][0]["contact_allowance_refs"] = json!(["L1", "NIGHT-L2"]);
    let job: PackageStructureJob = serde_json::from_value(input.clone()).unwrap();
    let p = compute_packaging(
        &job,
        job.budget.as_ref().unwrap(),
        context(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{:?}",
        p.result.checks
    );
    assert_eq!(
        p.result
            .material_counts
            .iter()
            .map(|m| m.area_mm2.as_ref().unwrap().value())
            .sum::<f64>(),
        6100.0
    );
    let svgs = p
        .manufacturing_bytes
        .iter()
        .map(|(_, b)| std::str::from_utf8(b).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(svgs.matches("data-connection-ref=\"GLUE1\"").count(), 1);
    assert!(svgs.contains("data-area-mm2=\"100\""));
    assert!(svgs.contains("data-job-ref=\"NIGHT-OVERLAPPING-GLUE\""));
    assert!(svgs.contains("data-job-revision=\"2\""));
    assert_eq!(
        serde_json::to_value(&p.result.subject_ref).unwrap(),
        serde_json::to_value(&job.subject_ref).unwrap()
    );
    assert_eq!(p.result.job_ref.id, job.id);
    assert_eq!(p.result.job_ref.revision.0.value(), job.revision.0.value());
    assert_eq!(
        serde_json::to_value(&p.artifact.subject_ref).unwrap(),
        serde_json::to_value(&job.subject_ref).unwrap()
    );
    let export_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.work/independent-closeout-20261020/main-reception/packaging-evidence/overlapping-glue-svg");
    std::fs::create_dir_all(&export_root).unwrap();
    for (index, (_, bytes)) in p.manufacturing_bytes.iter().enumerate() {
        std::fs::write(export_root.join(format!("part-{index}.svg")), bytes).unwrap();
    }
    evidence(
        "overlapping-glue-identity.json",
        json!({"input":input,"result":p.result,"artifact":p.artifact}),
    );
}

#[test]
fn manufacturing_cancellation_keeps_only_the_already_completed_blank_and_its_material() {
    let job = case("MAN04");
    let p = cancel_at_occurrence(&job, "manufacturing_boundary_edges", 2);
    assert_cancelled(&p);
    assert!(matches!(p.result.outcome, CheckOutcome::Undetermined));
    assert_eq!(p.manufacturing_bytes.len(), 1);
    assert_eq!(p.result.manufacturing_files.len(), 1);
    assert_eq!(p.result.material_counts.len(), 1);
    assert_eq!(p.result.material_counts[0].quantity.value(), 1);
    assert_eq!(
        p.result.material_counts[0]
            .area_mm2
            .as_ref()
            .unwrap()
            .value(),
        100.0
    );
    assert_eq!(p.result.manufacturing_files[0].part_id, job.parts[0].id);
    evidence(
        "cancel-manufacturing-partial-prefix.json",
        serde_json::to_value(&p.result).unwrap(),
    );
}
