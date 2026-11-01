//! Complete explicit 3x3 edits, separate from the frozen independent assertions.
#[path = "support/noncu_cases.rs"]
mod cases;
use serde_json::{Value, json};
use sf_domain::*;
use sf_packaging::{PackagingArtifactContext, PackagingProduct, compute_packaging};
use std::{fs, path::Path, sync::atomic::AtomicBool, time::Instant};

fn context(id: &str) -> PackagingArtifactContext {
    PackagingArtifactContext {
        artifact_id: ArtifactId::try_from(format!("ART-{id}")).unwrap(),
        input_ref: serde_json::from_value(json!({"path":"professional/input.json","kind":"packaging_job"})).unwrap(),
        result_ref: serde_json::from_value(json!({"path":"professional/result.json","kind":"packaging_result"})).unwrap(),
        manufacturing_directory: RelativeFilePath::try_from("professional/manufacturing".to_owned()).unwrap(),
    }
}
fn run(input: &Value, cancelled: bool) -> (PackagingProduct, Value, f64) {
    let job: PackageStructureJob = serde_json::from_value(input.clone()).unwrap();
    let mut phases = vec![];
    let started = Instant::now();
    let output = compute_packaging(&job, job.budget.as_ref().unwrap(), context(job.id.as_str()), &AtomicBool::new(cancelled), &mut |p| {
        if phases.last().is_none_or(|last: &Value| last["phase"] != p.phase) {
            phases.push(json!({"phase":p.phase,"budget_used":p.budget_used}));
        }
    }).unwrap_or_else(|issues| panic!("explicit valid matrix input rejected: {issues:?}"));
    (output, json!(phases), started.elapsed().as_secs_f64())
}
fn save(key: &str, input: &Value, output: &PackagingProduct, phases: Value, elapsed: f64) {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/delivery/evidence").join(std::env::var("NONCU_RUN_LABEL").unwrap_or("manual".into())).join("matrix").join(key);
    fs::create_dir_all(&directory).unwrap();
    let record = json!({"input":input,"selected_job":output.selected_job,"result":output.result,"artifact":output.artifact,"provider":output.provider,"cancelled":output.cancelled,"progress":phases,"elapsed_seconds":elapsed});
    fs::write(directory.join("actual.json"), serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    for (i, (_, bytes)) in output.manufacturing_bytes.iter().enumerate() { fs::write(directory.join(format!("part-{i}.svg")), bytes).unwrap(); }
}
fn matrix(mechanism: &str, change: &str) {
    let key = format!("{mechanism}-{change}");
    let input = cases::matrix_job(mechanism, change);
    let (product, phases, elapsed) = run(&input, false);
    save(&key, &input, &product, phases, elapsed);
    let result = serde_json::to_value(&product.result).unwrap();
    eprintln!("NONCU_MATRIX {}", json!({"case":key,"outcome":result["outcome"],"checks":result["checks"],"budget":result["budget_used"],"elapsed_seconds":elapsed}));
    assert_eq!(result["outcome"], "pass", "{key}: {}", serde_json::to_string_pretty(&result).unwrap());
    for purpose in ["forming", "net_space", "opening", "linear_extraction", "manufacturing"] {
        let checks: Vec<_> = result["checks"].as_array().unwrap().iter().filter(|c|c["purpose"] == purpose).collect();
        assert!(!checks.is_empty() && checks.iter().all(|c|c["outcome"] == "pass"), "{key} missing {purpose}");
    }
    assert_eq!(serde_json::to_value(&product.selected_job).unwrap(), input);
    assert_eq!(result["subject_ref"], input["subject_ref"]);
    let area: f64 = product.result.material_counts.iter().map(|c|c.area_mm2.as_ref().expect("Pass must provide actual paper area").value()).sum();
    assert_eq!(area, cases::expected_area(mechanism, change), "independent rectangle+finite-band area");
    assert_eq!(product.manufacturing_bytes.len(), cases::expected_parts(mechanism, change));
    assert_eq!(product.result.material_counts.iter().map(|c|c.quantity.value()).sum::<u64>(), cases::expected_parts(mechanism, change) as u64);
    assert_eq!(result["budget_used"]["candidates_evaluated"], 1);
    assert!(result["budget_used"]["interval_queries"].as_u64().unwrap() <= input["budget"]["interval_query_limit"].as_u64().unwrap());
    assert!(result["budget_used"]["subdivisions"].as_u64().unwrap() <= input["budget"]["subdivision_limit"].as_u64().unwrap());
}
macro_rules! row { ($name:ident,$mechanism:literal,$change:literal) => { #[test] fn $name(){matrix($mechanism,$change);} }; }
row!(container_product,"container","product_function");
row!(container_material,"container","material_approved");
row!(container_repeat,"container","part_relations_or_repeat_order");
row!(outer_liner_product,"outer_liner","product_function");
row!(outer_liner_material,"outer_liner","material_approved");
row!(outer_liner_repeat,"outer_liner","part_relations_or_repeat_order");
row!(cover_product,"cover","product_function");
row!(cover_material,"cover","material_approved");
row!(cover_repeat,"cover","part_relations_or_repeat_order");
row!(sleeve_product,"sleeve","product_function");
row!(sleeve_material,"sleeve","material_approved");
row!(sleeve_repeat,"sleeve","part_relations_or_repeat_order");

fn public_case(id: &str) -> Value {
    let source: Value = serde_json::from_str(include_str!("../../commands/tests/fixtures/production/packaging-acceptance-cases.json")).unwrap();
    source["cases"].as_array().unwrap().iter().find(|c|c["id"]==id).unwrap()["job"]["data"].clone()
}

#[test]
fn unresolved_flat_geometry_overlap_and_missing_glue_never_emit_a_complete_affected_blank() {
    let mut nonplanar = public_case("MAN01");
    nonplanar["panels"][1]["flat_pose"]["translation"]["z"] = json!(1.0);
    let mut overlap = public_case("MAN01");
    // B's own polygon is x=12..22, so -7 places it at x=5..15.
    overlap["panels"][1]["flat_pose"]["translation"]["x"] = json!(-7.0);
    let mut glue = public_case("LOCAL_APPROACH");
    glue["steps"] = json!([]);
    glue["conditions"]["allowed_contacts"] = json!([]);
    for key in ["forming_required","opening_required","net_space_required","extraction_required"] { glue["conditions"][key] = json!(false); }
    glue["parts"][1]["placement"]["translation"]["z"] = json!(1.0);
    glue["connections"][0]["frame_a"]["translation"]["z"] = json!(0.5);
    glue["connections"][0]["frame_b"]["translation"]["z"] = json!(-0.5);
    glue["connections"][0]["allow_contact"] = json!(false);
    glue["connections"][0]["contact_allowance_refs"] = json!([]);
    glue["connections"][0]["engage_after_step_id"] = Value::Null;
    for (key,input,expected) in [("noncoplanar",nonplanar,"undetermined"),("flat-overlap",overlap,"fail"),("missing-glue",glue,"undetermined")] {
        let (product,phases,elapsed) = run(&input,false);
        save(key,&input,&product,phases,elapsed);
        let result = serde_json::to_value(&product.result).unwrap();
        let check = result["checks"].as_array().unwrap().iter().find(|c|c["purpose"]=="manufacturing").unwrap();
        assert_eq!(check["outcome"],expected,"{key}: {check}");
        assert!(product.result.manufacturing_files.iter().all(|f|f.part_id.as_str()!="PART_A"),"{key} emitted an incomplete blank");
        if key=="missing-glue" { assert!(check["unresolved"].as_array().unwrap().iter().any(|i|i["code"]=="packaging.missing_common_glue_region")); }
    }
}

#[test]
fn mixed_noncoplanar_and_overlapping_panels_keep_the_actual_witness_panel_ids() {
    let mut input = public_case("MAN01");
    input["panels"][1]["flat_pose"]["translation"]["z"] = json!(1.0);
    let mut c = input["panels"][1].clone();
    c["id"] = json!("C");
    c["flat_pose"]["translation"] = json!({"x":-7.0,"y":0.0,"z":0.0});
    input["panels"].as_array_mut().unwrap().push(c);
    input["parts"][0]["panel_ids"].as_array_mut().unwrap().push(json!("C"));
    let mut band = input["fold_bands"][0].clone();
    band["id"] = json!("C_BAND");
    band["panel_b"] = json!("C");
    input["fold_bands"].as_array_mut().unwrap().push(band);
    let (product,phases,elapsed) = run(&input,false);
    save("mixed-noncoplanar-overlap",&input,&product,phases,elapsed);
    let result = serde_json::to_value(&product.result).unwrap();
    let manufacturing = result["checks"].as_array().unwrap().iter().find(|c|c["purpose"]=="manufacturing").unwrap();
    assert_eq!(manufacturing["outcome"],"fail");
    assert!(manufacturing["unresolved"].as_array().unwrap().iter().any(|i|i["code"]=="packaging.noncoplanar_flat_geometry"));
    assert_eq!(manufacturing["witnesses"][0]["object_pair"],json!(["A","C"]),"B is noncoplanar and omitted from planar overlap; the certified 50mm2 overlap is A/C");
    assert!(product.manufacturing_bytes.is_empty());
}

#[test]
fn all_three_mechanisms_preserve_query_budget_and_cancellation_as_undetermined() {
    for mechanism in cases::MECHANISMS {
        let mut input = cases::matrix_job(mechanism, "product_function");
        input["budget"]["interval_query_limit"] = json!(0);
        let (product, phases, elapsed) = run(&input, false);
        save(&format!("{mechanism}-zero-query"), &input, &product, phases, elapsed);
        assert!(matches!(product.result.outcome, CheckOutcome::Undetermined));
        assert_eq!(product.result.budget_used.interval_queries.value(), 0);
        let (product, phases, elapsed) = run(&input, true);
        save(&format!("{mechanism}-cancelled"), &input, &product, phases, elapsed);
        assert!(product.cancelled && matches!(product.result.outcome, CheckOutcome::Undetermined));
        assert!(product.manufacturing_bytes.is_empty());
    }
}
