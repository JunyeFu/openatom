use serde_json::{Value, json};
use sf_domain::*;
use sf_packaging::{PackagingArtifactContext, PackagingProduct, compute_packaging};
use std::sync::atomic::{AtomicBool, Ordering};

fn pose(x: f64, y: f64, z: f64) -> Value {
    json!({"translation":{"x":x,"y":y,"z":z},"rotation":{"x":0,"y":0,"z":0,"w":1},"scale":{"x":1,"y":1,"z":1},"unit":"millimeter","coordinate_frame":"right_handed_z_up"})
}
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
    json!([{"x":x0,"y":y0},{"x":x1,"y":y0},{"x":x1,"y":y1},{"x":x0,"y":y1}])
}
fn panel(id: &str, part: &str, polygon: Value, x: f64, y: f64) -> Value {
    json!({"id":id,"part_id":part,"polygon_mm":polygon,"thickness_mm":1,"material_ref":"CARD","flat_pose":pose(x,y,0.0)})
}
fn part(id: &str, panels: &[&str], root: &str, x: f64, y: f64, z: f64) -> Value {
    json!({"id":id,"title":"explicit test blank","panel_ids":panels,"root_panel":root,"placement":pose(x,y,z)})
}
fn base() -> Value {
    json!({"variables":[],"dimension_constraints":[],"id":"KERNEL","revision":3,"mechanism":"blank_restricted_structure","changes":[],"subject_ref":{"id":"PRODUCT","revision":"r2"},"panels":[panel("A","PART_A",rect(0.0,0.0,10.0,10.0),0.0,0.0)],"fold_bands":[],"connections":[],"parts":[part("PART_A",&["A"],"A",0.0,0.0,0.0)],"materials":[{"id":"CARD","title":"explicit accepted geometric thickness","thickness_mm":1,"basis":"user_provided","source_ref":null}],"conditions":{"net_space_required":false,"forming_required":false,"opening_required":false,"extraction_required":false,"allowed_contacts":[],"product_envelope":null,"extraction":null},"steps":[],"tolerance":{"distance_mm":1e-6,"angle_rad":1e-8,"constraint_mm":4e-6,"basis":"explicit test comparison profile; not material physics"},"budget":{"candidate_limit":64,"interval_query_limit":50000,"subdivision_limit":8192,"basis":"explicit integration test budget"},"unit":"millimeter","coordinate_frame":"right_handed_z_up","issues":[]})
}
fn context() -> PackagingArtifactContext {
    PackagingArtifactContext {
        artifact_id: "ACTUAL-PACK".to_string().try_into().unwrap(),
        input_ref: FileRef {
            path: "artifacts/pack/job.json".to_string().try_into().unwrap(),
            kind: DocumentKind::PackagingJob,
        },
        result_ref: FileRef {
            path: "artifacts/pack/result.json".to_string().try_into().unwrap(),
            kind: DocumentKind::PackagingResult,
        },
        manufacturing_directory: "artifacts/pack/manufacturing"
            .to_string()
            .try_into()
            .unwrap(),
    }
}
fn typed(j: Value) -> PackageStructureJob {
    serde_json::from_value(j).unwrap()
}
fn normalized(j: Value) -> Value {
    serde_json::to_value(typed(j)).unwrap()
}
fn run(j: Value) -> PackagingProduct {
    let j = typed(j);
    compute_packaging(
        &j,
        j.budget.as_ref().unwrap(),
        context(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap()
}
fn brief(p: &PackagingProduct) -> String {
    format!(
        "outcome={:?}; checks={:?}; limitations={:?}",
        p.result.outcome, p.result.checks, p.result.limitations
    )
}
fn fail(p: &PackagingProduct) {
    assert!(
        matches!(p.result.outcome, CheckOutcome::Fail),
        "{}",
        brief(p)
    );
    assert!(p.result.checks.iter().any(|c| !c.witnesses.is_empty()));
}
fn translation(j: &mut Value, start: Value, delta: Value) {
    j["conditions"]["forming_required"] = json!(true);
    j["steps"] = json!([{"id":"MOVE","purpose":"forming","duration_hint_ms":null,"motion":{"kind":"rigid_translation","data":{"part_id":"PART_B","start_pose":start,"displacement_mm":delta}},"prerequisites":[]}]);
}
fn two_parts() -> Value {
    let mut j = base();
    j["panels"] = json!([
        panel("A", "PART_A", rect(0.0, 0.0, 100.0, 60.0), 0.0, 0.0),
        panel("B", "PART_B", rect(0.0, 0.0, 10.0, 10.0), 0.0, 0.0)
    ]);
    j["parts"] = json!([
        part("PART_A", &["A"], "A", 0.0, 0.0, 0.0),
        part("PART_B", &["B"], "B", -20.0, 20.0, 0.0)
    ]);
    j
}
fn fixture(name: &str) -> Value {
    let id = match name {
        "local-contact-job" => "LOCAL_APPROACH",
        "restricted_folded_container-seed" => "COVERAGE-restricted_folded_container",
        "outer_box_and_inner_liner-seed" => "COVERAGE-outer_box_and_inner_liner",
        "cover_or_sliding_sleeve-seed" => "COVERAGE-cover_or_sliding_sleeve",
        "material-change-seed" => "COVERAGE-material-change",
        "explicit-repeat-seed" => "COVERAGE-explicit-repeat",
        "product-change-seed" => "PRODUCT_CHANGE_STALE_ENVELOPE",
        "sliding-sleeve-front-lap-invalid-job" => "SLIDING_FRONT_LAP_INVALID",
        "sliding-sleeve-job" => "SLIDING_HISTORY_R2",
        _ => panic!("unknown frozen fixture {name}"),
    };
    let inputs: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    inputs["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()["job"]["data"]
        .clone()
}

#[test]
fn actual_blank_emits_mm_layers_material_identity_and_no_approval_or_stock() {
    let j = base();
    let source = j.clone();
    let p = run(j.clone());
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
    assert_eq!(
        serde_json::to_value(&p.selected_job).unwrap(),
        normalized(source.clone())
    );
    assert_eq!(
        p.result.material_counts[0]
            .area_mm2
            .as_ref()
            .unwrap()
            .value(),
        100.0
    );
    assert_eq!(p.result.material_counts[0].quantity.value(), 1);
    assert_eq!(p.result.job_ref.revision.0.value(), 3);
    assert_eq!(p.result.subject_ref.revision.as_str(), "r2");
    assert_eq!(p.artifact.unit.as_str(), "millimeter");
    for purpose in [
        "forming",
        "opening",
        "net_space",
        "linear_extraction",
        "manufacturing",
    ] {
        assert!(p.provider.supported_scope.iter().any(|p| p == purpose));
    }
    assert!(matches!(
        p.result.evidence_origin,
        EvidenceOrigin::ActualSoftware
    ));
    let svg = std::str::from_utf8(&p.manufacturing_bytes[0].1).unwrap();
    for token in [
        "width=\"10mm\"",
        "id=\"cut\"",
        "id=\"fold\"",
        "id=\"glue\"",
        "data-job-revision=\"3\"",
    ] {
        assert!(svg.contains(token), "{svg}");
    }
    assert!(p.result.process_poses.is_empty());
    assert_eq!(j, source);
}

#[test]
fn concave_blank_keeps_the_notch_and_actual_area() {
    let mut j = base();
    j["panels"][0]["polygon_mm"] = json!([{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":5},{"x":5,"y":5},{"x":5,"y":10},{"x":0,"y":10}]);
    let p = run(j);
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
    assert_eq!(
        p.result.material_counts[0]
            .area_mm2
            .as_ref()
            .unwrap()
            .value(),
        75.0
    );
    let svg = std::str::from_utf8(&p.manufacturing_bytes[0].1).unwrap();
    assert!(svg.contains("5 5"));
}

#[test]
fn finite_fold_strip_adds_twenty_square_mm_without_internal_cut_edges() {
    let mut j = base();
    j["panels"] = json!([
        panel("A", "PART_A", rect(0.0, 0.0, 10.0, 10.0), 0.0, 0.0),
        panel("B", "PART_A", rect(0.0, 0.0, 10.0, 10.0), 12.0, 0.0)
    ]);
    j["parts"] = json!([part("PART_A", &["A", "B"], "A", 0.0, 0.0, 0.0)]);
    j["fold_bands"] = json!([{"id":"F","panel_a":"A","panel_b":"B","axis_start_mm":{"x":11,"y":0,"z":0},"axis_end_mm":{"x":11,"y":10,"z":0},"width_mm":2,"target_angle_rad":0,"allowed_range_rad":[0,1.57]}]);
    let p = run(j);
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
    assert_eq!(
        p.result.material_counts[0]
            .area_mm2
            .as_ref()
            .unwrap()
            .value(),
        220.0
    );
    let svg = std::str::from_utf8(&p.manufacturing_bytes[0].1).unwrap();
    assert!(svg.contains("data-width-mm=\"2\""));
    assert!(svg.contains("width=\"22mm\""));
}

#[test]
fn mid_path_translation_interference_is_not_endpoint_only() {
    let mut j = two_parts();
    translation(&mut j, pose(-20.0, 20.0, 0.0), json!({"x":140,"y":0,"z":0}));
    let p = run(j);
    fail(&p);
    assert!(
        p.result
            .checks
            .iter()
            .flat_map(|c| &c.witnesses)
            .any(|w| w.contact_point_mm.is_some())
    );
}

#[test]
fn narrow_interference_requires_real_subdivision() {
    let mut j = two_parts();
    j["panels"][0]["polygon_mm"] = rect(28.0, 20.0, 29.0, 22.0);
    j["panels"][1]["polygon_mm"] = rect(0.0, 0.0, 1.0, 2.0);
    j["parts"][1]["placement"] = pose(0.0, 20.0, 0.0);
    translation(&mut j, pose(0.0, 20.0, 0.0), json!({"x":100,"y":0,"z":0}));
    let p = run(j);
    fail(&p);
    assert!(p.result.budget_used.subdivisions.value() > 0);
    assert!(p.result.budget_used.interval_queries.value() > 1);
}

#[test]
fn rotation_checks_the_full_sweep() {
    let mut j = two_parts();
    j["panels"][0]["polygon_mm"] = rect(-1.0, -1.0, 1.0, 1.0);
    j["panels"][1]["polygon_mm"] = rect(9.0, -1.0, 11.0, 1.0);
    j["parts"][0]["placement"] = pose(0.0, 10.0, 0.0);
    j["parts"][1]["placement"] = pose(0.0, 0.0, 0.0);
    j["conditions"]["forming_required"] = json!(true);
    j["steps"] = json!([{"id":"ROTATE","purpose":"forming","duration_hint_ms":null,"motion":{"kind":"rigid_rotation","data":{"part_id":"PART_B","pivot_mm":{"x":0,"y":0,"z":0},"axis":{"x":0,"y":0,"z":1},"from_angle_rad":0,"to_angle_rad":std::f64::consts::PI}},"prerequisites":[]}]);
    fail(&run(j));
}

#[test]
fn fixed_pose_cannot_teleport_or_reset_folded_joints() {
    let mut j = base();
    j["conditions"]["forming_required"] = json!(true);
    j["steps"] = json!([{"id":"HOLD","purpose":"forming","duration_hint_ms":null,"motion":{"kind":"fixed_pose","data":{"part_id":"PART_A","pose":pose(1.0,0.0,0.0)}},"prerequisites":[]}]);
    let p = run(j);
    fail(&p);
    assert!(
        p.result
            .process_poses
            .iter()
            .all(|s| s.panel_poses[0].pose.translation.x.value() == 0.0)
    );
}

#[test]
fn explicit_three_repeated_parts_are_three_material_blanks() {
    let mut j = base();
    j["panels"] = json!([
        panel("A", "P1", rect(0.0, 0.0, 10.0, 10.0), 0.0, 0.0),
        panel("B", "P2", rect(0.0, 0.0, 10.0, 10.0), 0.0, 0.0),
        panel("C", "P3", rect(0.0, 0.0, 10.0, 10.0), 0.0, 0.0)
    ]);
    j["parts"] = json!([
        part("P1", &["A"], "A", 0.0, 0.0, 0.0),
        part("P2", &["B"], "B", 20.0, 0.0, 0.0),
        part("P3", &["C"], "C", 40.0, 0.0, 0.0)
    ]);
    let p = run(j);
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
    assert_eq!(p.result.material_counts.len(), 3);
    assert_eq!(
        p.result
            .material_counts
            .iter()
            .map(|m| m.quantity.value())
            .sum::<u64>(),
        3
    );
    assert_eq!(
        p.result
            .material_counts
            .iter()
            .map(|m| m.area_mm2.as_ref().unwrap().value())
            .sum::<f64>(),
        300.0
    );
}

#[test]
fn malformed_polygon_material_and_fold_tree_are_input_issues() {
    for kind in ["polygon", "material", "tree"] {
        let mut j = base();
        match kind {
            "polygon" => {
                j["panels"][0]["polygon_mm"] =
                    json!([{"x":0,"y":0},{"x":10,"y":10},{"x":0,"y":10},{"x":10,"y":0}])
            }
            "material" => j["panels"][0]["thickness_mm"] = json!(0.5),
            _ => {
                j["parts"][0]["panel_ids"] = json!(["A", "A"]);
            }
        }
        let j = typed(j);
        let e = compute_packaging(
            &j,
            j.budget.as_ref().unwrap(),
            context(),
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(e.iter().all(|i| i.field_path.is_some()), "{e:?}");
    }
}

#[test]
fn empty_input_missing_tolerance_zero_budget_and_cancel_never_pass() {
    for kind in ["empty", "tolerance", "budget", "cancel"] {
        let mut j = base();
        match kind {
            "empty" => {
                j["panels"] = json!([]);
                j["parts"] = json!([]);
            }
            "tolerance" => j["tolerance"] = Value::Null,
            "budget" => j["budget"]["candidate_limit"] = json!(0),
            _ => {}
        }
        let j = typed(j);
        let cancel = AtomicBool::new(kind == "cancel");
        let p = compute_packaging(
            &j,
            j.budget.as_ref().unwrap(),
            context(),
            &cancel,
            &mut |_| {},
        )
        .unwrap();
        assert!(
            matches!(p.result.outcome, CheckOutcome::Undetermined),
            "{kind}: {}",
            brief(&p)
        );
        assert_eq!(p.cancelled, kind == "cancel");
    }
}

#[test]
fn budget_is_global_and_request_freezes_none_without_silent_expansion() {
    let mut j = two_parts();
    translation(&mut j, pose(-20.0, 20.0, 0.0), json!({"x":140,"y":0,"z":0}));
    j["budget"]["interval_query_limit"] = json!(0);
    let p = run(j.clone());
    assert!(matches!(p.result.outcome, CheckOutcome::Undetermined));
    assert_eq!(p.result.budget_used.interval_queries.value(), 0);
    let mut j = typed(j);
    let budget = j.budget.take().unwrap();
    let p =
        compute_packaging(&j, &budget, context(), &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(
        p.selected_job
            .budget
            .as_ref()
            .unwrap()
            .interval_query_limit
            .value(),
        0
    );
    let mut mismatch = p.selected_job.clone();
    mismatch.budget.as_mut().unwrap().candidate_limit = 1_u64.try_into().unwrap();
    assert!(
        compute_packaging(
            &mismatch,
            &budget,
            context(),
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
}

#[test]
fn real_public_local_contact_closes_and_glues_only_the_common_area() {
    let p = run(fixture("local-contact-job"));
    assert_eq!(
        p.result
            .checks
            .iter()
            .map(|c| c.queries_used.value())
            .sum::<u64>(),
        p.result.budget_used.interval_queries.value(),
        "single candidate includes actual connection activation queries"
    );
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
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
    assert!(svgs.contains("data-area-mm2=\"100\""), "{svgs}");
}

#[test]
fn triangle_allowance_is_not_its_bounding_rectangle() {
    let mut j = fixture("local-contact-job");
    j["conditions"]["allowed_contacts"][0]["polygon_a_mm"] =
        json!([{"x":19,"y":19},{"x":35,"y":19},{"x":19,"y":35}]);
    let p = run(j);
    assert!(
        !matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
}

#[test]
fn late_connection_frame_mismatch_is_a_real_fail() {
    let mut j = fixture("local-contact-job");
    j["connections"][0]["frame_b"]["translation"]["x"] = json!(8.0);
    fail(&run(j));
}

#[test]
fn cancel_callback_keeps_the_unfinished_prefix_visible() {
    let mut j = two_parts();
    translation(&mut j, pose(-20.0, 20.0, 0.0), json!({"x":140,"y":0,"z":0}));
    let j = typed(j);
    let cancel = AtomicBool::new(false);
    let p = compute_packaging(
        &j,
        j.budget.as_ref().unwrap(),
        context(),
        &cancel,
        &mut |p| {
            if p.phase.starts_with("forming:") {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    )
    .unwrap();
    assert!(p.cancelled);
    assert!(!matches!(p.result.outcome, CheckOutcome::Pass));
    assert!(
        !p.result
            .process_poses
            .iter()
            .any(|p| p.progress.value() == 1.0)
    );
}

#[test]
fn three_mechanisms_material_and_repeat_changes_are_actual_editable_computations() {
    for name in [
        "restricted_folded_container-seed",
        "outer_box_and_inner_liner-seed",
        "cover_or_sliding_sleeve-seed",
        "material-change-seed",
        "explicit-repeat-seed",
    ] {
        let source = fixture(name);
        let p = run(source.clone());
        assert!(
            matches!(p.result.outcome, CheckOutcome::Pass),
            "{name}: {}",
            brief(&p)
        );
        assert!(!p.result.process_poses.is_empty());
        assert_eq!(
            p.result.manufacturing_files.len(),
            source["parts"].as_array().unwrap().len()
        );
        for sample in &p.result.process_poses {
            assert_eq!(
                sample.panel_poses.len(),
                source["panels"].as_array().unwrap().len()
            );
        }
        assert_eq!(
            serde_json::to_value(&p.selected_job).unwrap(),
            normalized(source.clone())
        );
    }
}

#[test]
fn product_change_must_explicitly_rebind_its_envelope_to_the_new_subject() {
    let source = fixture("product-change-seed");
    let j = typed(source.clone());
    let err = compute_packaging(
        &j,
        j.budget.as_ref().unwrap(),
        context(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap_err();
    assert!(
        err.iter()
            .any(|i| i.code == "packaging.invalid_product_envelope")
    );
    let mut fixed = source;
    fixed["revision"] = json!(2);
    fixed["conditions"]["product_envelope"]["subject"] = fixed["subject_ref"].clone();
    let p = run(fixed);
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
    assert_eq!(p.result.subject_ref.revision.as_str(), "r2");
}

#[test]
fn sleeve_forming_checks_keep_old_collision_and_current_unresolved_reasons() {
    let old = run(fixture("sliding-sleeve-front-lap-invalid-job"));
    fail(&old);
    let current = run(fixture("sliding-sleeve-job"));
    for c in &current.result.checks {
        println!(
            "sleeve {:?}: {:?}; queries={}; unresolved={:?}",
            c.purpose,
            c.outcome,
            c.queries_used.value(),
            c.unresolved
        );
    }
    assert!(
        current
            .result
            .checks
            .iter()
            .any(|c| matches!(c.purpose, PackagingStepPurpose::Forming)
                && matches!(c.outcome, CheckOutcome::Pass)),
        "{}",
        brief(&current)
    );
}

#[test]
fn explicit_new_binary_grid_sleeve_candidate_has_real_manufacturing() {
    let source: Value = serde_json::from_str(include_str!(
        "fixtures/explicit-binary-grid-sleeve.job.json"
    ))
    .unwrap();
    let p = run(source["data"].clone());
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
    assert_eq!(p.result.manufacturing_files.len(), 1);
    assert_eq!(
        p.result.material_counts[0]
            .area_mm2
            .as_ref()
            .unwrap()
            .value(),
        2212.5
    );
    let svg = std::str::from_utf8(&p.manufacturing_bytes[0].1).unwrap();
    assert!(svg.contains("data-connection-ref=\"SEAM\""));
    let area = svg
        .split("data-area-mm2=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((area - 120.0).abs() < 1e-6, "{area}");
}

#[test]
fn positive_clearance_deficit_is_certified_without_inventing_penetration() {
    let mut j = base();
    j["conditions"]["net_space_required"] = json!(true);
    let vertices: Vec<_> = [4.0, 6.0]
        .into_iter()
        .flat_map(|x| {
            [4.0, 6.0].into_iter().flat_map(move |y| {
                [0.6, 1.0]
                    .into_iter()
                    .map(move |z| json!({"x":x,"y":y,"z":z}))
            })
        })
        .collect();
    j["conditions"]["product_envelope"] = json!({"subject":j["subject_ref"],"vertices_mm":vertices,"allowed_poses":[pose(0.0,0.0,0.0)],"clearance_mm":0.5});
    let p = run(j);
    fail(&p);
    assert!(
        p.result
            .checks
            .iter()
            .flat_map(|c| &c.witnesses)
            .any(|w| w.message.contains("clearance_deficit") && w.contact_point_mm.is_none())
    );
}

#[test]
fn professional_module_and_editable_templates_use_the_real_public_format() {
    let source: Versioned<ModulePackage> = serde_json::from_str(include_str!(
        "../../../modules/professional/paper-packaging/module.json"
    ))
    .unwrap();
    assert!(matches!(
        source.data.definition.payload,
        ModulePayload::Professional(_)
    ));
    assert_eq!(source.data.files.len(), 5);
    for file in &source.data.files {
        assert!(matches!(file.kind, DocumentKind::PackagingJob));
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/professional/paper-packaging")
            .join(file.path.as_str());
        let j: Versioned<PackageStructureJob> =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(matches!(j.schema_version, SchemaVersion::V1));
        assert!(!j.data.id.as_str().is_empty());
    }
    assert!(source.data.definition.contribution.license.is_none());
}

#[test]
fn an_unknown_approved_pose_is_not_eliminated_by_a_different_failed_pose() {
    let mut j = base();
    j["conditions"]["net_space_required"] = json!(true);
    let vertices: Vec<_> = [4.0, 6.0]
        .into_iter()
        .flat_map(|x| {
            [4.0, 6.0].into_iter().flat_map(move |y| {
                [-0.125, 0.125]
                    .into_iter()
                    .map(move |z| json!({"x":x,"y":y,"z":z}))
            })
        })
        .collect();
    j["conditions"]["product_envelope"] = json!({"subject":j["subject_ref"],"vertices_mm":vertices,"allowed_poses":[pose(0.0,0.0,0.0),pose(0.0,0.0,0.625)],"clearance_mm":0});
    let p = run(j.clone());
    assert!(
        matches!(p.result.outcome, CheckOutcome::Undetermined),
        "{}",
        brief(&p)
    );
    j["conditions"]["product_envelope"]["allowed_poses"]
        .as_array_mut()
        .unwrap()
        .push(pose(0.0, 0.0, 2.0));
    let p = run(j);
    assert!(
        matches!(p.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&p)
    );
}

#[test]
fn explicit_execution_block_is_preserved_without_fake_computation() {
    let mut j = base();
    j["issues"] = json!([{"code":"input.missing_approved_condition","object_ref":null,"field_path":"/materials","message":"explicit unresolved execution condition","blocks":["execution"]}]);
    let p = run(j);
    assert!(matches!(p.result.outcome, CheckOutcome::Undetermined));
    assert_eq!(p.result.budget_used.candidates_evaluated.value(), 0);
    assert!(
        p.result
            .limitations
            .iter()
            .any(|i| i.code == "input.missing_approved_condition")
    );
}

#[test]
fn incomplete_continuous_search_keeps_a_concrete_counterexample_without_global_infeasibility() {
    let mut j = two_parts();
    j["parts"][1]["placement"] = pose(20.0, 20.0, 0.0);
    j["steps"] = json!([{"id":"HOLD","purpose":"forming","duration_hint_ms":null,"motion":{"kind":"fixed_pose","data":{"part_id":"PART_B","pose":pose(20.0,20.0,0.0)}},"prerequisites":[]}]);
    j["conditions"]["forming_required"] = json!(true);
    j["variables"] = json!([{"id":"X","field_path":"parts/PART_B/placement/translation/x","minimum":20,"maximum":30,"unit":"mm","integer":false,"basis":"user_provided"}]);
    let p = run(j.clone());
    assert!(
        matches!(p.result.outcome, CheckOutcome::Undetermined),
        "{}",
        brief(&p)
    );
    assert!(
        p.result
            .checks
            .iter()
            .any(|c| matches!(c.outcome, CheckOutcome::Fail) && !c.witnesses.is_empty())
    );
    assert!(
        p.result
            .limitations
            .iter()
            .any(|i| i.code == "packaging.search_unresolved")
    );
    let j = typed(j);
    let cancel = AtomicBool::new(false);
    let mut count = 0;
    let p = compute_packaging(
        &j,
        j.budget.as_ref().unwrap(),
        context(),
        &cancel,
        &mut |event| {
            if event.phase == "candidate" {
                count += 1;
                if count == 2 {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
        },
    )
    .unwrap();
    assert!(p.cancelled);
    fail(&p);
}

#[test]
fn contact_permission_boolean_and_local_references_must_agree_before_computation() {
    let original = fixture("local-contact-job");
    for (allow, refs) in [(false, json!(["L1"])), (true, json!([]))] {
        let mut value = original.clone();
        value["connections"][0]["allow_contact"] = json!(allow);
        value["connections"][0]["contact_allowance_refs"] = refs;
        let job = typed(value);
        let before = serde_json::to_value(&job).unwrap();
        let issues = compute_packaging(
            &job,
            job.budget.as_ref().unwrap(),
            context(),
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.code == "packaging.contact_permission_contract"
                    && i.field_path.as_deref() == Some("/connections/0/contact_allowance_refs")),
            "{issues:?}"
        );
        assert_eq!(serde_json::to_value(&job).unwrap(), before);
    }
    let positive = run(original.clone());
    assert!(
        matches!(positive.result.outcome, CheckOutcome::Pass),
        "{}",
        brief(&positive)
    );
    let mut without_permission = original;
    without_permission["connections"][0]["allow_contact"] = json!(false);
    without_permission["connections"][0]["contact_allowance_refs"] = json!([]);
    without_permission["conditions"]["allowed_contacts"] = json!([]);
    // false/empty is admissible. The real unlicensed overlap still fails; the
    // bool neither removes ordinary collision checks nor grants whole panels.
    fail(&run(without_permission));
}

fn product_for_cancel(job: &mut Value) {
    let vertices: Vec<_> = (0..4)
        .flat_map(|x| (0..4).flat_map(move |y| (0..4).map(move |z| json!({"x":x,"y":y,"z":z}))))
        .collect();
    job["conditions"]["net_space_required"] = json!(true);
    job["conditions"]["product_envelope"] = json!({"subject":job["subject_ref"],"vertices_mm":vertices,"allowed_poses":[pose(0.0,0.0,10.0)],"clearance_mm":0});
}

fn worker_cancel_at(job: PackageStructureJob, phase: &str) -> PackagingProduct {
    let cancel = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let (reached_tx, reached_rx) = std::sync::mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
        let cancel_ref = &cancel;
        let worker = scope.spawn(move || {
            let mut reported = false;
            compute_packaging(
                &job,
                job.budget.as_ref().unwrap(),
                context(),
                cancel_ref,
                &mut |event| {
                    if !reported && event.phase == phase {
                        reported = true;
                        reached_tx.send(event.phase.clone()).unwrap();
                        // A test barrier only. The production loop has already
                        // performed real geometry work at this progress checkpoint.
                        resume_rx.recv().unwrap();
                    }
                },
            )
        });
        assert_eq!(
            reached_rx
                .recv()
                .expect("actual worker entered geometry loop"),
            phase
        );
        cancel.store(true, Ordering::Relaxed);
        resume_tx.send(()).unwrap();
        worker.join().unwrap().unwrap()
    })
}

#[test]
fn actual_worker_cancels_inside_product_hull_support_vertices() {
    let mut job = base();
    product_for_cancel(&mut job);
    let product = worker_cancel_at(typed(job), "product_hull_support_vertices");
    assert!(product.cancelled);
    assert!(
        matches!(product.result.outcome, CheckOutcome::Undetermined),
        "{}",
        brief(&product)
    );
    assert_eq!(
        product.result.budget_used.stopped_by.as_deref(),
        Some("cancelled")
    );
    assert_eq!(product.result.budget_used.interval_queries.value(), 0);
    assert!(product.result.checks.iter().any(|c| matches!(
        c.purpose,
        PackagingStepPurpose::NetSpace
    )
        && c.unresolved.iter().any(|i| i.code == "packaging.cancelled")));
    assert!(product.result.checks.iter().all(|c| c.witnesses.is_empty()));
}

#[test]
fn actual_worker_cancels_inside_separating_axis_edge_pairs() {
    let mut job = two_parts();
    translation(
        &mut job,
        pose(-20.0, 20.0, 0.0),
        json!({"x":140,"y":0,"z":0}),
    );
    let product = worker_cancel_at(typed(job), "separating_axes_edge_pairs");
    assert!(product.cancelled);
    assert!(
        matches!(product.result.outcome, CheckOutcome::Undetermined),
        "{}",
        brief(&product)
    );
    assert_eq!(
        product.result.budget_used.stopped_by.as_deref(),
        Some("cancelled")
    );
    assert_eq!(product.result.budget_used.interval_queries.value(), 1);
    assert!(product.result.checks.iter().all(|c| c.witnesses.is_empty()));
    assert!(
        !product
            .result
            .process_poses
            .iter()
            .any(|s| s.progress.value() == 1.0)
    );
}

#[test]
fn actual_worker_hull_cancellation_keeps_an_already_proved_forming_counterexample() {
    let mut job = two_parts();
    translation(
        &mut job,
        pose(-20.0, 20.0, 0.0),
        json!({"x":140,"y":0,"z":0}),
    );
    job["conditions"]["opening_required"] = json!(true);
    product_for_cancel(&mut job);
    job["steps"].as_array_mut().unwrap().push(json!({"id":"OPEN","purpose":"opening","duration_hint_ms":null,"motion":{"kind":"rigid_translation","data":{"part_id":"PART_B","start_pose":pose(120.0,20.0,0.0),"displacement_mm":{"x":0,"y":0,"z":0}}},"prerequisites":["MOVE"]}));
    let product = worker_cancel_at(typed(job), "product_hull_support_vertices");
    assert!(product.cancelled);
    fail(&product);
    assert!(
        product
            .result
            .checks
            .iter()
            .any(|c| matches!(c.purpose, PackagingStepPurpose::Forming) && !c.witnesses.is_empty())
    );
    assert!(
        !product
            .result
            .process_poses
            .iter()
            .any(|s| s.step_id.as_str() == "OPEN" && s.progress.value() == 1.0)
    );
}
