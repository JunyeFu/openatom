//! Independent consumers of actual packaging computation; no copied proof results.
use serde_json::{Value, json};
use sf_domain::*;
use sf_packaging::{PackagingArtifactContext, PackagingProduct, compute_packaging};
use std::sync::atomic::AtomicBool;

fn cases() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap()
}

fn case(id: &str) -> Value {
    cases()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("missing independent case {id}"))
        .clone()
}

fn job(case: &Value) -> PackageStructureJob {
    serde_json::from_value::<Versioned<PackageStructureJob>>(case["job"].clone())
        .unwrap()
        .data
}

fn context() -> PackagingArtifactContext {
    PackagingArtifactContext {
        artifact_id: ArtifactId::try_from("W5-ACTUAL-PACKAGING".to_owned()).unwrap(),
        input_ref: serde_json::from_value(
            json!({"path":"professional/w5-job.json","kind":"packaging_job"}),
        )
        .unwrap(),
        result_ref: serde_json::from_value(
            json!({"path":"professional/w5-result.json","kind":"packaging_result"}),
        )
        .unwrap(),
        manufacturing_directory: RelativeFilePath::try_from(
            "professional/manufacturing".to_owned(),
        )
        .unwrap(),
    }
}

fn compute_with(
    job: &PackageStructureJob,
    budget: &CalculationBudget,
    cancelled: bool,
) -> PackagingProduct {
    let cancelled = AtomicBool::new(cancelled);
    let product = compute_packaging(job, budget, context(), &cancelled, &mut |_| {})
        .unwrap_or_else(|issues| panic!("valid independent Job rejected: {issues:?}"));
    let usage = serde_json::to_value(&product.result.budget_used).unwrap();
    let budget = serde_json::to_value(budget).unwrap();
    assert!(
        usage["candidates_evaluated"].as_u64().unwrap()
            <= budget["candidate_limit"].as_u64().unwrap()
    );
    assert!(
        usage["interval_queries"].as_u64().unwrap()
            <= budget["interval_query_limit"].as_u64().unwrap()
    );
    assert!(
        usage["subdivisions"].as_u64().unwrap() <= budget["subdivision_limit"].as_u64().unwrap()
    );
    let result = serde_json::to_value(&product.result).unwrap();
    let artifact = serde_json::to_value(&product.artifact).unwrap();
    assert_eq!(result["evidence_origin"], "actual_software");
    assert_eq!(artifact["evidence_origin"], "actual_software");
    assert_eq!(artifact["actual_outcome"], result["outcome"]);
    assert_eq!(artifact["id"], result["artifact_id"]);
    assert_eq!(artifact["subject_ref"], result["subject_ref"]);
    assert_eq!(
        result["subject_ref"],
        serde_json::to_value(&job.subject_ref).unwrap()
    );
    assert_eq!(artifact["supported_scope"], result["supported_scope"]);
    assert_eq!(
        artifact["input_ref"],
        json!({"path":"professional/w5-job.json","kind":"packaging_job"})
    );
    assert_eq!(
        artifact["result_ref"],
        json!({"path":"professional/w5-result.json","kind":"packaging_result"})
    );
    assert_eq!(
        result["job_ref"]["id"],
        serde_json::to_value(&job.id).unwrap()
    );
    assert_eq!(
        result["job_ref"]["revision"],
        serde_json::to_value(&job.revision).unwrap()
    );
    product
}

fn compute_case(id: &str) -> PackagingProduct {
    let input = job(&case(id));
    compute_with(&input, input.budget.as_ref().unwrap(), false)
}

fn assert_check(product: &PackagingProduct, purpose: &str, expected: &str) {
    let result = serde_json::to_value(&product.result).unwrap();
    let checks: Vec<_> = result["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|check| check["purpose"] == purpose)
        .collect();
    assert!(
        !checks.is_empty(),
        "required purpose {purpose} missing: {result:#}"
    );
    if expected == "fail" {
        assert!(
            checks.iter().any(|check| check["outcome"] == "fail"),
            "strict counterexample lost: {result:#}"
        );
        assert!(
            checks
                .iter()
                .any(|check| !check["witnesses"].as_array().unwrap().is_empty()),
            "Fail must expose its actual witness"
        );
        assert_eq!(result["outcome"], "fail");
    } else {
        assert!(
            checks.iter().all(|check| check["outcome"] == expected),
            "{purpose} expected {expected}: {result:#}"
        );
    }
}

// The producer exports straight material boundaries as SVG M/L segments.
fn svg_segments(bytes: &[u8], layer: &str) -> Vec<([f64; 2], [f64; 2])> {
    let svg = std::str::from_utf8(bytes).unwrap();
    let group = svg.split_once(&format!("id=\"{layer}\"")).unwrap().1;
    let group = group
        .split_once('>')
        .unwrap()
        .1
        .split_once("</g>")
        .unwrap()
        .0;
    let mut edges = Vec::new();
    for path in group.split(" d=\"").skip(1) {
        let values: Vec<_> = path.split('"').next().unwrap().split_whitespace().collect();
        assert_eq!(values.len() % 6, 0, "unexpected straight SVG path");
        for segment in values.chunks_exact(6) {
            assert_eq!(segment[0], "M");
            assert_eq!(segment[3], "L");
            edges.push((
                [segment[1].parse().unwrap(), segment[2].parse().unwrap()],
                [segment[4].parse().unwrap(), segment[5].parse().unwrap()],
            ));
        }
    }
    edges
}

#[test]
fn fixed_pose_holds_the_current_part_and_cannot_teleport_or_ignore_material_product_and_bond_collision()
 {
    for id in ["HOLD01", "HOLD03", "HOLD04", "HOLD05", "HOLD06"] {
        let input = case(id);
        let product = compute_case(id);
        assert_check(&product, "forming", input["expected"].as_str().unwrap());
    }
    let invalid = job(&case("HOLD02"));
    let result = compute_packaging(
        &invalid,
        invalid.budget.as_ref().unwrap(),
        context(),
        &AtomicBool::new(false),
        &mut |_| {},
    );
    assert!(
        matches!(result, Err(ref issues) if !issues.is_empty()),
        "a missing part is invalid input, not a successful empty check"
    );
}

#[test]
fn local_contact_permission_is_limited_to_its_actual_polygon_depth_step_and_matched_faces() {
    for id in [
        "LOCAL_APPROACH",
        "LOCAL_OUTSIDE_PATCH",
        "LOCAL_EXCESS_DEPTH",
        "LOCAL_INACTIVE_PERMISSION",
        "LOCAL_TRIANGLE",
        "LOCAL_CONCAVE",
        "LOCAL_WRONG_FACE",
        "LOCAL_BOUNDARY",
    ] {
        let input = case(id);
        let product = compute_case(id);
        assert_check(&product, "forming", input["expected"].as_str().unwrap());
    }
}

#[test]
fn continuous_motion_detects_collision_between_free_endpoints_and_between_zero_half_one_samples() {
    assert_check(&compute_case("KIN_TRANSLATION_MID"), "forming", "fail");
    assert_check(&compute_case("KIN_ROTATION_HIDDEN"), "forming", "fail");
    assert_check(&compute_case("KIN_TRANSLATION_CLEAR"), "forming", "pass");
}

#[test]
fn explicit_budget_missing_tolerance_and_cancellation_never_become_a_complete_pass() {
    assert_check(&compute_case("LOCAL_ZERO_QUERY"), "forming", "undetermined");
    let mut input = job(&case("HOLD01"));
    let budget = input.budget.clone().unwrap();
    let mut mismatched_budget = serde_json::to_value(&budget).unwrap();
    mismatched_budget["candidate_limit"] = json!(1);
    let mismatched_budget = serde_json::from_value(mismatched_budget).unwrap();
    assert!(
        matches!(compute_packaging(&input, &mismatched_budget, context(), &AtomicBool::new(false), &mut |_| {}),
        Err(ref issues) if !issues.is_empty()),
        "a frozen Job budget cannot silently be replaced"
    );
    let cancelled = compute_with(&input, &budget, true);
    assert!(cancelled.cancelled);
    assert_ne!(
        serde_json::to_value(&cancelled.result).unwrap()["outcome"],
        "pass"
    );

    input.budget = None;
    let adopted_budget = compute_with(&input, &budget, false);
    assert_eq!(
        serde_json::to_value(&adopted_budget.selected_job.budget).unwrap(),
        serde_json::to_value(&budget).unwrap()
    );
    assert!(
        input.budget.is_none(),
        "caller-owned frozen input cannot be mutated"
    );

    input.tolerance = None;
    match compute_packaging(
        &input,
        &budget,
        context(),
        &AtomicBool::new(false),
        &mut |_| {},
    ) {
        Err(issues) => assert!(!issues.is_empty()),
        Ok(product) => {
            assert!(
                product.selected_job.tolerance.is_none(),
                "numeric tolerance requires explicit adoption"
            );
            assert_ne!(
                serde_json::to_value(&product.result).unwrap()["outcome"],
                "pass"
            );
        }
    }
}

#[test]
fn three_mechanisms_and_three_explicit_changes_are_consumed_as_editable_jobs_with_actual_results() {
    for input in cases()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|input| input["expected"] == "observed")
    {
        let input = job(input);
        let before = serde_json::to_value(&input).unwrap();
        let product = compute_with(&input, input.budget.as_ref().unwrap(), false);
        let selected = serde_json::to_value(&product.selected_job).unwrap();
        assert_eq!(selected["mechanism"], before["mechanism"]);
        assert_eq!(selected["changes"], before["changes"]);
        assert_eq!(selected["subject_ref"], before["subject_ref"]);
        assert_eq!(selected["materials"], before["materials"]);
        assert_eq!(selected["parts"], before["parts"]);
        assert_eq!(serde_json::to_value(&input).unwrap(), before);
        let result = serde_json::to_value(&product.result).unwrap();
        eprintln!(
            "W5 actual mechanism/change: {}",
            json!({
                "job_id":before["id"],"job_revision":before["revision"],
                "mechanism":before["mechanism"],"changes":before["changes"],
                "outcome":result["outcome"],"checks":result["checks"],
                "budget_used":result["budget_used"],"supported_scope":result["supported_scope"],
                "limitations":result["limitations"],"material_counts":result["material_counts"],
                "manufacturing_file_count":product.manufacturing_bytes.len()
            })
        );
        assert!(
            !result["checks"].as_array().unwrap().is_empty(),
            "empty checks cannot establish mechanism coverage"
        );
        if result["outcome"] == "undetermined" {
            assert!(
                !result["limitations"].as_array().unwrap().is_empty()
                    || result["checks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|check| !check["unresolved"].as_array().unwrap().is_empty())
            );
        }
        for sample in result["process_poses"].as_array().unwrap() {
            assert!(
                selected["steps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|step| step["id"] == sample["step_id"])
            );
            assert!((0.0..=1.0).contains(&sample["progress"].as_f64().unwrap()));
            for panel in sample["panel_poses"].as_array().unwrap() {
                assert!(
                    selected["panels"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|declared| declared["id"] == panel["panel_id"])
                );
                assert_eq!(panel["pose"]["unit"], "millimeter");
                assert_eq!(panel["pose"]["coordinate_frame"], "right_handed_z_up");
            }
        }
        if result["outcome"] == "pass" {
            for (required, purpose) in [
                ("forming_required", "forming"),
                ("opening_required", "opening"),
                ("net_space_required", "net_space"),
                ("extraction_required", "linear_extraction"),
            ] {
                if selected["conditions"][required] == true {
                    assert_check(&product, purpose, "pass");
                }
            }
            assert_check(&product, "manufacturing", "pass");
            assert_eq!(product.manufacturing_bytes.len(), input.parts.len());
            assert!(!result["process_poses"].as_array().unwrap().is_empty());
        }
        // A coverage run alone does not assert whole-mechanism professional success.
    }
}

#[test]
fn three_explicit_flat_parts_export_three_actual_svgs_and_three_hundred_square_millimeters() {
    let product = compute_case("MAN04");
    let result = serde_json::to_value(&product.result).unwrap();
    let counts = result["material_counts"].as_array().unwrap();
    assert_eq!(counts.len(), 3);
    assert!(counts.iter().all(|count| count["quantity"] == 1));
    let area: f64 = counts
        .iter()
        .map(|count| count["area_mm2"].as_f64().unwrap())
        .sum();
    assert!((area - 300.0).abs() < 1e-6);
    assert_eq!(result["manufacturing_files"].as_array().unwrap().len(), 3);
    assert_eq!(product.manufacturing_bytes.len(), 3);
    for (file, bytes) in &product.manufacturing_bytes {
        let svg = std::str::from_utf8(bytes).unwrap();
        assert!(svg.contains("<svg") && svg.contains("</svg>"));
        assert!(
            svg.contains("<path") || svg.contains("<polygon") || svg.contains("<polyline"),
            "empty SVG is not a manufactured part"
        );
        let file = serde_json::to_value(file).unwrap();
        assert!(
            result["manufacturing_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["file"] == file)
        );
    }
    assert_eq!(
        serde_json::to_value(&product.selected_job.parts)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn finite_search_includes_an_approved_constraint_target_and_an_empty_search_stays_undetermined() {
    let found = compute_case("SEARCH01");
    let selected = serde_json::to_value(&found.selected_job).unwrap();
    assert_eq!(
        selected["panels"][0]["polygon_mm"][1]["x"].as_f64(),
        Some(6.0)
    );
    assert_eq!(
        serde_json::to_value(&found.result).unwrap()["outcome"],
        "pass"
    );
    let empty = compute_case("SEARCH02");
    let result = serde_json::to_value(&empty.result).unwrap();
    assert_eq!(
        result["outcome"], "undetermined",
        "no candidate is not a geometric counterexample"
    );
    assert_eq!(result["budget_used"]["candidates_evaluated"], 0);
    assert!(!result["limitations"].as_array().unwrap().is_empty());
}

#[test]
fn manufacturing_includes_the_finite_fold_band_and_exports_glue_for_the_actual_paired_joint() {
    let folded_blank = compute_case("MAN01");
    let result = serde_json::to_value(&folded_blank.result).unwrap();
    let counts = result["material_counts"].as_array().unwrap();
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0]["quantity"], 1);
    assert!(
        (counts[0]["area_mm2"].as_f64().unwrap() - 220.0).abs() < 1e-6,
        "two 100mm2 panels and their explicit 20mm2 strip form the blank"
    );
    assert!(
        result["manufacturing_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["layers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|layer| layer == "fold"))
    );
    assert!(!folded_blank.manufacturing_bytes.is_empty());
    let (_, bytes) = &folded_blank.manufacturing_bytes[0];
    let cut_length: f64 = svg_segments(bytes, "cut")
        .iter()
        .map(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]))
        .sum();
    let fold_length: f64 = svg_segments(bytes, "fold")
        .iter()
        .map(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]))
        .sum();
    assert!(
        (cut_length - 64.0).abs() < 1e-6,
        "only the actual 22x10 blank boundary is cut"
    );
    assert!((fold_length - 10.0).abs() < 1e-6);

    let glued = compute_case("LOCAL_APPROACH");
    let result = serde_json::to_value(&glued.result).unwrap();
    let total_area: f64 = result["material_counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|count| count["area_mm2"].as_f64().unwrap())
        .sum();
    assert!((total_area - 6100.0).abs() < 1e-6);
    assert!(
        result["manufacturing_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["layers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|layer| layer == "glue"))
    );
    let glue_area: f64 = glued
        .manufacturing_bytes
        .iter()
        .map(|(_, bytes)| {
            svg_segments(bytes, "glue")
                .iter()
                .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
                .sum::<f64>()
                .abs()
                / 2.0
        })
        .sum();
    assert!(
        (glue_area - 100.0).abs() < 1e-6,
        "actual paired projection is counted once across both parts"
    );
}

#[test]
fn a_nonconvex_material_notch_remains_empty_instead_of_becoming_its_convex_hull() {
    assert_check(&compute_case("NONCONVEX_PANEL_NOTCH"), "forming", "pass");
}

#[test]
fn product_change_preserves_the_stale_envelope_rejection_and_explicitly_rebinds_a_new_job_revision()
{
    let stale = job(&case("PRODUCT_CHANGE_STALE_ENVELOPE"));
    let corrected = job(&case("COVERAGE-product-change"));
    let before = serde_json::to_value(&stale).unwrap();
    let next = serde_json::to_value(&corrected).unwrap();
    assert_ne!(
        before["subject_ref"],
        before["conditions"]["product_envelope"]["subject"]
    );
    assert_eq!(
        next["subject_ref"],
        next["conditions"]["product_envelope"]["subject"]
    );
    assert_eq!(next["id"], before["id"]);
    assert_eq!(
        next["revision"].as_u64().unwrap(),
        before["revision"].as_u64().unwrap() + 1
    );
    assert!(
        matches!(compute_packaging(&stale,stale.budget.as_ref().unwrap(),context(),&AtomicBool::new(false),&mut |_| {}),
        Err(ref issues) if !issues.is_empty())
    );
    assert_eq!(serde_json::to_value(&stale).unwrap(), before);
}

#[test]
fn sliding_sleeve_preserves_the_historical_counterexample_and_calculates_an_explicit_new_candidate()
{
    let invalid = compute_case("SLIDING_FRONT_LAP_INVALID");
    assert_check(&invalid, "forming", "fail");
    let historical = compute_case("SLIDING_HISTORY_R2");
    assert_check(&historical, "forming", "pass");
    let revised = compute_case("SLIDING_EXPLICIT_NEW");
    let result = serde_json::to_value(&revised.result).unwrap();
    let historical_result = serde_json::to_value(&historical.result).unwrap();
    eprintln!(
        "W5 actual sleeve: {}",
        json!({
            "historical_revision":2,"historical_outcome":historical_result["outcome"],
            "historical_checks":historical_result["checks"],
            "new_revision":3,"new_outcome":result["outcome"],"new_checks":result["checks"],
            "new_budget_used":result["budget_used"],"new_material_counts":result["material_counts"],
            "invalid_revision":1,"invalid_forming_outcome":"fail"
        })
    );
    assert_eq!(
        result["outcome"], "pass",
        "the explicit new candidate must establish every requested condition: {result:#}"
    );
    for purpose in [
        "forming",
        "opening",
        "net_space",
        "linear_extraction",
        "manufacturing",
    ] {
        assert_check(&revised, purpose, "pass");
    }
    assert_eq!(revised.manufacturing_bytes.len(), 1);
    assert_eq!(
        serde_json::to_value(&job(&case("SLIDING_HISTORY_R2"))).unwrap()["revision"],
        2
    );
    assert_eq!(
        serde_json::to_value(&job(&case("SLIDING_FRONT_LAP_INVALID"))).unwrap()["revision"],
        1
    );
}

#[test]
fn no_penetration_does_not_satisfy_a_larger_explicit_product_clearance() {
    assert_check(
        &compute_case("BOX_INSUFFICIENT_CLEARANCE"),
        "net_space",
        "fail",
    );
}

#[test]
fn contact_permission_switch_and_explicit_references_must_agree_before_calculation() {
    for id in ["CONTACT_DISABLED_WITH_REFS", "CONTACT_ENABLED_WITHOUT_REFS"] {
        let input = job(&case(id));
        let before = serde_json::to_value(&input).unwrap();
        assert!(
            matches!(compute_packaging(&input,input.budget.as_ref().unwrap(),context(),&AtomicBool::new(false),&mut |_| {}),
            Err(ref issues) if !issues.is_empty()),
            "{id} cannot silently apply a contact permission"
        );
        assert_eq!(serde_json::to_value(&input).unwrap(), before);
    }
    assert_check(&compute_case("LOCAL_APPROACH"), "forming", "pass");
}

#[test]
fn an_actual_worker_can_be_cancelled_after_hull_and_axis_work_starts_without_a_fixed_timeout() {
    for (id, phase) in [
        (
            "COVERAGE-restricted_folded_container",
            "product_hull_support_vertices",
        ),
        ("CANCEL_DURING_AXES", "separating_axes_edge_pairs"),
    ] {
        let input = job(&case(id));
        let flag = AtomicBool::new(false);
        let product = std::thread::scope(|scope| {
            let (reached_tx, reached_rx) = std::sync::mpsc::sync_channel(0);
            let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
            let flag_ref = &flag;
            let worker = scope.spawn(move || {
                let mut reached = false;
                compute_packaging(
                    &input,
                    input.budget.as_ref().unwrap(),
                    context(),
                    flag_ref,
                    &mut |progress| {
                        if !reached && progress.phase == phase {
                            reached = true;
                            reached_tx.send(progress.phase).unwrap();
                            resume_rx.recv().unwrap();
                        }
                    },
                )
                .unwrap()
            });
            assert_eq!(
                reached_rx
                    .recv()
                    .expect("actual geometry worker reached the traversal checkpoint"),
                phase
            );
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            resume_tx.send(()).unwrap();
            worker.join().unwrap()
        });
        assert!(product.cancelled);
        let result = serde_json::to_value(&product.result).unwrap();
        assert_eq!(result["outcome"], "undetermined");
        assert_eq!(result["budget_used"]["stopped_by"], "cancelled");
        assert!(
            result["checks"]
                .as_array()
                .unwrap()
                .iter()
                .all(|check| check["witnesses"].as_array().unwrap().is_empty()),
            "cancellation is not a physical counterexample"
        );
    }
}
