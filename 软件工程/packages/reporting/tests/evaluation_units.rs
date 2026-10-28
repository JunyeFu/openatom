//! Actual frozen Host/Engine evidence and explicitly identified finite type counterexamples.
use serde_json::{Value, json};
use sf_domain::*;
use sf_reporting::*;
use std::{collections::BTreeMap, fs, path::Path, sync::atomic::AtomicBool};
type Frozen = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);
fn inputs() -> Vec<Frozen> {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/comparison/actual-frozen-inputs.json"
    ))
    .unwrap()
}
fn manual() -> Vec<Frozen> {
    let v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json"
    ))
    .unwrap();
    vec![serde_json::from_value(v["quality_a"].clone()).unwrap()]
}
fn context(i: &Frozen, mode: ReportMode) -> EvaluationReport {
    let evidence = match mode {
        ReportMode::DesignDescription => {
            "本示例仅说明设计输入与待验证方案；尚未运行，不包含实际产量或模拟结果。"
        }
        ReportMode::SimulationEvaluation => {
            "产线模拟结果来自真实软件Run；样例参数是自建测试输入，不是工业现场观测。"
        }
    };
    serde_json::from_value(json!({"id":"evaluation-owned","mode":mode,"project_id":i.0.id,"project_revision":i.0.revision,"scenario_refs":[],"run_refs":[],"time_reference":null,"observation_window":null,"acceptance_scope":[],"question_and_scope":"评估控制盒装配、检验与包装的目标接受量、工期和资源；工序完成不等于目的接受。","input_assumptions":[],"module_versions":[],"layout_and_schedule":[],"scenario_comparison":null,"conclusions":[evidence],"missing_conditions":[],"supported_scope":["有限离散生产/物流与包装工程范围；是否形成结果以报告模式和Run引用为准"],"limitations":[],"editable_project_ref":null,"asset_rights":[],"generation_context":null,"run_evaluations":[]})).unwrap()
}
fn build(i: &[Frozen]) -> Versioned<EvaluationReport> {
    build_evaluation(
        context(&i[0], ReportMode::SimulationEvaluation),
        i,
        &[],
        &[],
        &[],
    )
    .unwrap()
}
fn v<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}
#[test]
fn exact_decimal_cost_converts_only_after_multiplication_and_division() {
    // Typed counterexample derived from a frozen trace, not a new Engine run.
    let mut i = inputs();
    i.truncate(1);
    let item = i[0]
        .3
        .final_items
        .iter_mut()
        .find(|x| matches!(x.lifecycle, Lifecycle::Transformed))
        .unwrap();
    item.quantity =
        serde_json::from_value(json!({"value":{"coefficient":2,"decimal_places":1},"unit":"kg"}))
            .unwrap();
    let item_id = item.id.clone();
    let mut c = cost(
        "decimal-material",
        "unused",
        "consumed_material",
        json!({"coefficient":1,"decimal_places":1}),
        json!("CNY"),
    );
    c.object_ref = ObjectRef {
        kind: ObjectKind::Item,
        id: item_id,
    };
    c.rate_quantity =
        serde_json::from_value(json!({"value":{"coefficient":1,"decimal_places":0},"unit":"kg"}))
            .unwrap();
    i[0].1.cost_inputs = vec![c];
    let report = build(&i);
    let line = &report.data.run_evaluations[0].costs.lines[0];
    assert_eq!(line.amount.as_ref().unwrap().value(), 0.02);
    assert_eq!(v(line)["amount"], json!(0.02));
}

fn csv_rows(report: &Versioned<EvaluationReport>, path: &str) -> Vec<BTreeMap<String, String>> {
    let files = render_evaluation(
        report,
        &[EvaluationOutputFormat::Csv],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let f = files.iter().find(|f| f.relative_path == path).unwrap();
    let mut reader = csv::Reader::from_reader(&f.bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    reader
        .records()
        .map(|r| {
            headers
                .iter()
                .zip(r.unwrap().iter())
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect()
        })
        .collect()
}

#[test]
fn no_run_and_cross_run_boundaries_are_not_applicable_but_real_missing_boundary_is_unknown() {
    let i = inputs();
    let design = build_evaluation(
        context(&i[0], ReportMode::DesignDescription),
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let rows = csv_rows(&design, "csv/context.csv");
    assert_eq!(rows[0]["computed_until_ms"], "");
    assert_eq!(rows[0]["computed_until_ms_state"], "not_applicable");
    assert!(rows[0]["computed_until_ms_reason"].contains("单一实际Run"));
    let mut actual = build(&i);
    let rows = csv_rows(&actual, "csv/changes.csv");
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row["computed_until_ms_state"], "not_applicable");
    }
    // Serializer type counterexample: actual Run identity is retained, its boundary absent.
    actual.data.run_evaluations[0].computed_until_ms = None;
    let rows = csv_rows(&actual, "csv/context.csv");
    let missing = rows
        .iter()
        .find(|r| r["run_id"] == i[0].3.id.as_str())
        .unwrap();
    assert_eq!(missing["computed_until_ms_state"], "unknown");
}
fn resource_fixture() -> Vec<Frozen> {
    let mut i = inputs();
    i.truncate(1);
    let resource = i[0].0.resources[0].id.clone();
    let mut r = v(&i[0]);
    r[0]["resources"][0]["capacity"]["value"]["coefficient"] = json!(2);
    r[0]["calendars"][0]["intervals"] =
        json!([{"start_ms":60000,"end_ms":90000},{"start_ms":110000,"end_ms":150000}]);
    r[3]["window"] = json!({"start_ms":50000,"end_ms":150000});
    r[3]["computed_until_ms"] = json!(150000);
    let mut a = r[3]["resource_claims"][0].clone();
    a["id"] = json!("unique-A");
    a["resource_id"] = json!(resource);
    a["intervals"] = json!([{"start_ms":50000,"end_ms":100000}]);
    a["interval_end_causes"] = json!(["released"]);
    a["units"] = json!(1);
    let mut b = a.clone();
    b["id"] = json!("unique-B");
    b["intervals"] = json!([{"start_ms":80000,"end_ms":140000}]);
    r[3]["resource_claims"] = json!([a, b, a]);
    vec![serde_json::from_value(r).unwrap()]
}
fn cost(id: &str, object: &str, basis: &str, rate: Value, currency: Value) -> CostInput {
    serde_json::from_value(json!({"id":id,"object_ref":{"kind":"resource","id":object},"component":"labor","basis":basis,"rate":rate,"currency":currency,"rate_quantity":{"value":{"coefficient":if matches!(basis,"occupied_time"|"paid_calendar"){3600000}else{1},"decimal_places":0},"unit":if matches!(basis,"occupied_time"|"paid_calendar"){"person_ms"}else{"piece"}},"includes":[],"source":{"target":{"kind":"resource","id":object},"field_path":"/hourly_rate","value":null,"basis":"user_provided","source_ref":null}})).unwrap()
}
#[test]
fn same_actual_frozen_runs_preserve_identity_completion_and_original_results() {
    let i = inputs();
    let before = v(&i);
    let report = build(&i);
    assert_eq!(v(&i), before);
    assert_eq!(report.data.run_evaluations.len(), 2);
    assert!(report.data.scenario_comparison.is_some());
    assert!(
        report.data.run_evaluations[0].order_metrics[0]
            .completion_ms
            .is_none()
    );
    assert_eq!(
        report.data.run_evaluations[1].order_metrics[0]
            .completion_ms
            .as_ref()
            .unwrap()
            .0
            .value(),
        165000
    );
    assert_eq!(
        report.data.run_evaluations[0].input_ref.project_revision.0,
        i[0].3.input_ref.project_revision.0
    );
}
#[test]
fn manual_goal_three_actual_two_and_missing_cost_are_preserved() {
    let i = manual();
    let report = build(&i);
    let run = &report.data.run_evaluations[0];
    assert!(run.order_metrics.is_empty());
    assert_eq!(run.acceptance_events.len(), 2);
    assert_eq!(run.goal_met, Some(false));
    assert!(
        run.input_assumptions
            .iter()
            .any(|p| p.field_path == "/acceptance_goal")
    );
    assert!(run.costs.currencies.is_empty());
    assert!(!run.costs.missing_conditions.is_empty());
}

#[test]
fn csv_context_retains_each_frozen_runs_target_and_assumptions() {
    let i = manual();
    let mut report = build(&i);
    // Export-level assumptions are not the selected historical Run's input.
    report.data.input_assumptions.clear();
    let rows = csv_rows(&report, "csv/context.csv");
    let row = rows.iter().find(|r| r["run_id"] == i[0].3.id.as_str()).unwrap();
    let assumptions: Vec<ParameterSource> =
        serde_json::from_str(&row["input_assumptions_json"]).unwrap();
    let source = assumptions.iter().find(|s| s.field_path == "/acceptance_goal").unwrap();
    let Some(TypedValue::String(goal)) = &source.value else { panic!("frozen goal absent") };
    let goal: Value = serde_json::from_str(goal).unwrap();
    assert_eq!(goal["accepted_quantity_per_day"], json!({"value":{"coefficient":3,"decimal_places":0},"unit":"piece"}));
    assert_eq!(goal["observation_window"], json!({"start_ms":0,"end_ms":120000}));
    assert_eq!(row["goal_met"], "false");
    assert_eq!(row["goal_met_state"], "known");
    assert!(row.contains_key("report_generation_context_json"));

    let design = build_evaluation(context(&i[0], ReportMode::DesignDescription), &[], &[], &[], &[]).unwrap();
    let rows = csv_rows(&design, "csv/context.csv");
    assert_eq!(rows[0]["goal_met"], "");
    assert_eq!(rows[0]["goal_met_state"], "not_applicable");
    assert_eq!(rows[0]["input_assumptions_json"], "[]");
}
#[test]
fn single_run_does_not_require_revised_run_or_comparison() {
    let mut i = inputs();
    i.remove(0);
    let report = build(&i);
    assert_eq!(report.data.run_refs.len(), 1);
    assert!(report.data.scenario_comparison.is_none());
    assert!(
        report
            .data
            .conclusions
            .iter()
            .any(|s| s.contains("产线模拟结果来自真实软件Run"))
    );
    assert_eq!(report.data.run_evaluations[0].goal_met, Some(true));
}
#[test]
fn no_run_design_description_has_no_actual_results() {
    let i = inputs();
    let report = build_evaluation(
        context(&i[0], ReportMode::DesignDescription),
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(report.data.run_refs.is_empty());
    assert!(report.data.run_evaluations.is_empty());
    assert!(report.data.scenario_comparison.is_none());
    assert!(
        !report
            .data
            .conclusions
            .iter()
            .any(|s| s.contains("产线模拟结果来自真实软件Run"))
    );
    assert!(
        build_evaluation(
            context(&i[0], ReportMode::SimulationEvaluation),
            &[],
            &[],
            &[],
            &[]
        )
        .is_err()
    );
}
#[test]
fn resource_union_weighted_integral_open_closed_and_duplicates_are_distinct() {
    let i = resource_fixture();
    let report = build(&i);
    let r = &report.data.run_evaluations[0].resource_metrics[0];
    assert_eq!(r.any_busy_ms.0.value(), 90000);
    assert_eq!(r.occupied_capacity_ms.value(), 110000);
    assert_eq!(
        r.open_calendar_occupied_capacity_ms
            .as_ref()
            .unwrap()
            .value(),
        70000
    );
    assert_eq!(
        r.closed_calendar_held_capacity_ms.as_ref().unwrap().value(),
        40000
    );
    assert_eq!(r.available_capacity_ms.as_ref().unwrap().value(), 140000);
    assert_eq!(r.utilization.as_ref().unwrap().value(), 0.5);
}

#[test]
fn design_mode_preserves_legitimate_host_custom_facts_without_global_text_filtering() {
    let i = inputs();
    let mut c = context(&i[0], ReportMode::DesignDescription);
    let custom = "Host确认：既有车间的实际场地尺寸20×10m；历史Run OLD-1不属于本次设计的运行结果。"
        .to_owned();
    c.conclusions.push(custom.clone());
    let report = build_evaluation(c, &[], &[], &[], &[]).unwrap();
    assert!(report.data.conclusions.contains(&custom));
    let files = render_evaluation(
        &report,
        &[EvaluationOutputFormat::Json, EvaluationOutputFormat::Html],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let json: Value = serde_json::from_slice(
        &files
            .iter()
            .find(|f| f.relative_path == "report.json")
            .unwrap()
            .bytes,
    )
    .unwrap();
    assert!(
        json["data"]["conclusions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some(custom.as_str()))
    );
    assert!(
        String::from_utf8_lossy(
            &files
                .iter()
                .find(|f| f.relative_path == "index.html")
                .unwrap()
                .bytes
        )
        .contains(&custom)
    );
}
#[test]
fn conditional_occupied_cost_uses_unit_milliseconds_and_frozen_price() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    i[0].1.cost_inputs = vec![cost(
        "labor",
        &id,
        "occupied_time",
        json!({"coefficient":36,"decimal_places":0}),
        json!("CNY"),
    )];
    let report = build(&i);
    let c = &report.data.run_evaluations[0].costs;
    assert_eq!(
        c.lines[0].measured_quantity.as_ref().unwrap().unit.as_str(),
        "person_ms"
    );
    assert!((c.lines[0].amount.as_ref().unwrap().value() - 1.1).abs() < 1e-12);
    assert_eq!(c.inputs[0].rate.as_ref().unwrap().coefficient.value(), 36);
}

#[test]
fn resource_capacity_time_keeps_declared_unit_and_rejects_incompatible_rate_unit() {
    for (capacity_unit, rate_unit, basis, expected) in [
        ("person", "person_ms", "occupied_time", Some(1.1)),
        ("machine", "machine_ms", "occupied_time", Some(1.1)),
        ("slot", "slot_ms", "paid_calendar", Some(1.4)),
        ("person", "machine_ms", "occupied_time", None),
        ("machine", "h", "occupied_time", None),
    ] {
        let mut i = resource_fixture();
        i[0].0.resources[0].capacity.unit = capacity_unit.to_owned().try_into().unwrap();
        let id = i[0].0.resources[0].id.as_str().to_owned();
        let mut c = cost(
            "time-unit",
            &id,
            basis,
            json!({"coefficient":36,"decimal_places":0}),
            json!("CNY"),
        );
        c.rate_quantity.unit = rate_unit.to_owned().try_into().unwrap();
        i[0].1.cost_inputs = vec![c];
        let r = build(&i);
        let c = &r.data.run_evaluations[0].costs.lines[0];
        assert_eq!(
            c.measured_quantity.as_ref().unwrap().unit.as_str(),
            format!("{capacity_unit}_ms")
        );
        assert_eq!(c.amount.as_ref().map(|v| v.value()), expected);
        if expected.is_none() {
            assert!(
                c.missing_conditions
                    .iter()
                    .any(|i| i.code == "evaluation.cost_unit")
            );
        }
    }
}
#[test]
fn paid_calendar_and_occupied_time_are_not_added_and_missing_rate_is_not_zero() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    i[0].1.cost_inputs = vec![
        cost(
            "paid",
            &id,
            "paid_calendar",
            json!({"coefficient":36,"decimal_places":0}),
            json!("CNY"),
        ),
        cost("occupied", &id, "occupied_time", Value::Null, json!("CNY")),
    ];
    let c = build(&i).data.run_evaluations.remove(0).costs;
    assert!(c.lines.iter().all(|l| l.amount.is_none()));
    assert!(c.currencies[0].complete_total.is_none());
    assert!(c.currencies[0].unit_cost.is_none());
}
#[test]
fn explicit_zero_price_is_known_zero_and_currencies_stay_separate() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    let mut a = cost(
        "cny",
        &id,
        "occupied_time",
        json!({"coefficient":0,"decimal_places":0}),
        json!("CNY"),
    );
    let mut b = cost(
        "usd",
        &id,
        "occupied_time",
        json!({"coefficient":72,"decimal_places":0}),
        json!("USD"),
    );
    a.component = "labor-cny".into();
    b.component = "equipment-usd".into();
    i[0].1.cost_inputs = vec![a, b];
    let c = build(&i).data.run_evaluations.remove(0).costs;
    assert_eq!(c.lines[0].amount.as_ref().unwrap().value(), 0.);
    assert_eq!(c.currencies.len(), 2);
    assert!((c.currencies[1].known_subtotal.value() - 2.2).abs() < 1e-12);
}
#[test]
fn prefix_cost_keeps_known_subtotal_but_not_complete_total_or_goal() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    i[0].3.whole_window_computed = false;
    i[0].3.computation_status = ComputationStatus::Cancelled;
    i[0].1.cost_inputs = vec![cost(
        "labor",
        &id,
        "occupied_time",
        json!({"coefficient":36,"decimal_places":0}),
        json!("CNY"),
    )];
    let r = build(&i).data.run_evaluations.remove(0);
    assert!(r.goal_met.is_none());
    assert!((r.costs.currencies[0].known_subtotal.value() - 1.1).abs() < 1e-12);
    assert!(r.costs.currencies[0].complete_total.is_none());
}
fn resources() -> EvaluationResources {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    EvaluationResources {
        cjk_font: Some(
            fs::read(root.join("tools/architecture-proof/assets/SourceHanSansCN-Regular.otf"))
                .unwrap(),
        ),
        font_license: Some(
            fs::read(root.join("tools/architecture-proof/assets/LICENSE.SourceHanSans.txt"))
                .unwrap(),
        ),
        files: BTreeMap::new(),
    }
}
#[test]
fn actual_four_format_bytes_reopen_from_one_report_and_write_three_demonstrations() {
    let i = inputs();
    let mut c = context(&i[0], ReportMode::SimulationEvaluation);
    c.question_and_scope
        .push_str(" <script>不执行外部文本</script> 逗号,引号\"与换行\n保留。 ");
    let report = build_evaluation(c, &i, &[], &[], &[]).unwrap();
    let bytes = render_evaluation(
        &report,
        &[
            EvaluationOutputFormat::Pdf,
            EvaluationOutputFormat::Html,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Json,
        ],
        &resources(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        bytes
            .iter()
            .filter(|f| f.relative_path.starts_with("csv/"))
            .count(),
        15
    );
    let raw = &bytes
        .iter()
        .find(|f| f.relative_path == "report.json")
        .unwrap()
        .bytes;
    assert_eq!(serde_json::from_slice::<Value>(raw).unwrap(), v(&report));
    let html = String::from_utf8(
        bytes
            .iter()
            .find(|f| f.relative_path == "index.html")
            .unwrap()
            .bytes
            .clone(),
    )
    .unwrap();
    assert!(html.contains("&lt;script&gt;不执行外部文本&lt;/script&gt;"));
    assert!(!html.contains("fetch("));
    assert!(!html.contains("src=\"https://"));
    for f in bytes.iter().filter(|f| f.relative_path.ends_with(".csv")) {
        assert!(f.bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        let mut reader = csv::Reader::from_reader(&f.bytes[3..]);
        assert!(reader.headers().unwrap().iter().any(|s| s == "report_id"));
        for row in reader.records() {
            row.unwrap();
        }
    }
    assert!(
        bytes
            .iter()
            .find(|f| f.relative_path == "report.pdf")
            .unwrap()
            .bytes
            .starts_with(b"%PDF-")
    );
    if let Some(out) = std::env::var_os("SF_EVALUATION_OUTPUT") {
        let out = Path::new(&out);
        for f in &bytes {
            let path = out.join("comparison").join(&f.relative_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &f.bytes).unwrap();
        }
        let single = build(&i[1..]);
        let design = build_evaluation(
            context(&i[0], ReportMode::DesignDescription),
            &[],
            &[],
            &[],
            &[],
        )
        .unwrap();
        for (name, r) in [("single", single), ("design", design)] {
            for f in render_evaluation(
                &r,
                &[
                    EvaluationOutputFormat::Pdf,
                    EvaluationOutputFormat::Html,
                    EvaluationOutputFormat::Json,
                    EvaluationOutputFormat::Csv,
                ],
                &resources(),
                &AtomicBool::new(false),
            )
            .unwrap()
            {
                let path = out.join(name).join(&f.relative_path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, f.bytes).unwrap();
            }
        }
    }
}
#[test]
fn cancelled_or_missing_font_never_returns_partial_files() {
    let r = build(&inputs());
    assert!(
        render_evaluation(
            &r,
            &[EvaluationOutputFormat::Json, EvaluationOutputFormat::Pdf],
            &EvaluationResources::default(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let e = render_evaluation(
        &r,
        &[EvaluationOutputFormat::Json],
        &EvaluationResources::default(),
        &AtomicBool::new(true),
    )
    .unwrap_err();
    assert_eq!(e[0].code, "evaluation.cancelled");
}
#[test]
fn mismatched_frozen_run_identity_is_rejected() {
    let mut i = inputs();
    i[0].0.revision = Revision(SafeUint::try_from(777u64).unwrap());
    assert!(
        build_evaluation(
            context(&i[0], ReportMode::SimulationEvaluation),
            &i,
            &[],
            &[],
            &[]
        )
        .is_err()
    );
}

#[test]
fn material_cost_counts_each_consumed_item_once_not_each_later_packaging_reference() {
    let mut i = inputs();
    i.truncate(1);
    let item = i[0]
        .3
        .final_items
        .iter()
        .find(|t| {
            matches!(t.lifecycle, Lifecycle::Transformed) && t.item_type_ref.id.as_str() == "pcb"
        })
        .unwrap();
    let mut c = cost(
        "material",
        "worker-assembly",
        "consumed_material",
        json!({"coefficient":2,"decimal_places":0}),
        json!("CNY"),
    );
    c.object_ref = ObjectRef {
        kind: ObjectKind::Item,
        id: item.id.clone(),
    };
    c.component = "material".into();
    c.rate_quantity.unit = UnitId::try_from("piece".to_owned()).unwrap();
    i[0].1.cost_inputs = vec![c];
    let report = build(&i);
    let c = &report.data.run_evaluations[0].costs;
    assert_eq!(
        c.lines[0]
            .measured_quantity
            .as_ref()
            .unwrap()
            .value
            .coefficient
            .value(),
        1
    );
    assert_eq!(c.lines[0].amount.as_ref().unwrap().value(), 2.);
}

#[test]
fn paid_shift_price_does_not_shrink_to_actual_busy_time_or_cancelled_prefix() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    i[0].3.computed_until_ms = Some(SimTimeMs(SafeUint::try_from(90000u64).unwrap()));
    i[0].3.whole_window_computed = false;
    i[0].3.computation_status = ComputationStatus::Cancelled;
    i[0].1.cost_inputs = vec![cost(
        "paid",
        &id,
        "paid_calendar",
        json!({"coefficient":36,"decimal_places":0}),
        json!("CNY"),
    )];
    let c = build(&i).data.run_evaluations.remove(0).costs;
    assert_eq!(
        c.lines[0]
            .measured_quantity
            .as_ref()
            .unwrap()
            .value
            .coefficient
            .value(),
        140000
    );
    assert!((c.lines[0].amount.as_ref().unwrap().value() - 1.4).abs() < 1e-12);
    assert!(c.currencies[0].complete_total.is_none());
}

#[test]
fn csv_json_and_html_preserve_base_float_precision_and_null_resource_reasons() {
    let mut r = build(&inputs());
    let value = 135000.0 / 7.0;
    r.data.run_evaluations[0].resource_metrics[0].utilization =
        Some(FiniteNumber::try_from(value).unwrap());
    r.data.run_evaluations[0].resource_metrics[0].available_capacity_ms = None;
    let files = render_evaluation(
        &r,
        &[
            EvaluationOutputFormat::Json,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Html,
        ],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let f = files
        .iter()
        .find(|f| f.relative_path == "csv/resources.csv")
        .unwrap();
    let mut reader = csv::Reader::from_reader(&f.bytes[3..]);
    let header = reader.headers().unwrap().clone();
    let n = header.iter().position(|k| k == "utilization").unwrap();
    let s = header
        .iter()
        .position(|k| k == "available_capacity_ms_state")
        .unwrap();
    let row = reader.records().next().unwrap().unwrap();
    assert_eq!(row[n].parse::<f64>().unwrap().to_bits(), value.to_bits());
    assert_eq!(&row[s], "unknown");
    let data: Value = serde_json::from_slice(
        &files
            .iter()
            .find(|f| f.relative_path == "report.json")
            .unwrap()
            .bytes,
    )
    .unwrap();
    assert_eq!(
        data["data"]["run_evaluations"][0]["resource_metrics"][0]["utilization"]
            .as_f64()
            .unwrap()
            .to_bits(),
        value.to_bits()
    );
    assert!(
        String::from_utf8_lossy(
            &files
                .iter()
                .find(|f| f.relative_path == "index.html")
                .unwrap()
                .bytes
        )
        .contains(&value.to_string())
    );
}

#[test]
fn human_money_half_up_does_not_round_the_frozen_base_data() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    i[0].1.cost_inputs = vec![cost(
        "labor",
        &id,
        "occupied_time",
        json!({"coefficient":36,"decimal_places":0}),
        json!("CNY"),
    )];
    for (base, display) in [(0.125, "0.13"), (1.005, "1.01"), (9.995, "10.00")] {
        let mut report = build(&i);
        report.data.run_evaluations[0].costs.lines[0].amount =
            Some(FiniteNumber::try_from(base).unwrap());
        let files = render_evaluation(
            &report,
            &[EvaluationOutputFormat::Html, EvaluationOutputFormat::Json],
            &EvaluationResources::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let html = String::from_utf8_lossy(
            &files
                .iter()
                .find(|f| f.relative_path == "index.html")
                .unwrap()
                .bytes,
        );
        assert!(html.contains(&format!("<td>{display}</td>")));
        let json: Value = serde_json::from_slice(
            &files
                .iter()
                .find(|f| f.relative_path == "report.json")
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert_eq!(
            json["data"]["run_evaluations"][0]["costs"]["lines"][0]["amount"]
                .as_f64()
                .unwrap()
                .to_bits(),
            base.to_bits()
        );
    }
}

#[test]
fn report_unit_cost_comparison_uses_same_measured_cost_snapshot() {
    let mut i = inputs();
    for (n, input) in i.iter_mut().enumerate() {
        input.1.cost_inputs = vec![cost(
            "labor",
            "worker-assembly",
            "occupied_time",
            json!({"coefficient":if n==0{36}else{72},"decimal_places":0}),
            json!("CNY"),
        )];
    }
    let report = build(&i);
    let c = report
        .data
        .scenario_comparison
        .as_ref()
        .unwrap()
        .metrics
        .iter()
        .find(|m| matches!(m.metric, MetricKey::UnitCost) && m.currency.as_deref() == Some("CNY"))
        .unwrap();
    let MetricValue::MoneyPerUnit { amount: a, .. } = &c.values[0].value else {
        panic!("{:?}", c.values)
    };
    let MetricValue::MoneyPerUnit { amount: b, .. } = &c.values[1].value else {
        panic!("{:?}", c.values)
    };
    assert!((a.value() - 0.6).abs() < 1e-12);
    assert!((b.value() - 1.2).abs() < 1e-12);
    assert!(c.pair_differences[0].comparable);
    assert!(
        (report.data.run_evaluations[0].costs.currencies[0]
            .unit_cost
            .as_ref()
            .unwrap()
            .value()
            - a.value())
        .abs()
            < 1e-12
    );
}

#[test]
fn absent_priced_object_is_unknown_instead_of_zero_consumption() {
    let mut i = inputs();
    i.truncate(1);
    let mut c = cost(
        "absent",
        "worker-assembly",
        "consumed_material",
        json!({"coefficient":2,"decimal_places":0}),
        json!("CNY"),
    );
    c.object_ref = ObjectRef {
        kind: ObjectKind::Item,
        id: ObjectId::try_from("absent-item".to_owned()).unwrap(),
    };
    c.rate_quantity.unit = UnitId::try_from("piece".to_owned()).unwrap();
    i[0].1.cost_inputs = vec![c];
    let report = build(&i);
    assert!(
        report.data.run_evaluations[0].costs.lines[0]
            .measured_quantity
            .is_none()
    );
    assert!(
        report.data.run_evaluations[0].costs.lines[0]
            .amount
            .is_none()
    );
}

#[test]
fn actual_transport_batch_basis_counts_unique_departed_trip_not_cargo_items() {
    let all: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json"
    ))
    .unwrap();
    let mut i: Vec<Frozen> = vec![serde_json::from_value(all["halls_b"].clone()).unwrap()];
    let trip = i[0]
        .3
        .transport
        .iter()
        .find(|t| t.departed_ms.is_some())
        .unwrap()
        .id
        .clone();
    let mut c = cost(
        "trip",
        "worker-assembly",
        "batch_transport",
        json!({"coefficient":3,"decimal_places":0}),
        json!("CNY"),
    );
    c.object_ref = ObjectRef {
        kind: ObjectKind::Transport,
        id: trip,
    };
    c.component = "freight".into();
    c.rate_quantity.unit = UnitId::try_from("batch".to_owned()).unwrap();
    i[0].1.cost_inputs = vec![c];
    let report = build(&i);
    let c = &report.data.run_evaluations[0].costs.lines[0];
    assert_eq!(
        c.measured_quantity
            .as_ref()
            .unwrap()
            .value
            .coefficient
            .value(),
        1
    );
    assert_eq!(c.amount.as_ref().unwrap().value(), 3.);
}

#[test]
fn explicit_disposal_uses_unique_scrap_item_not_repeated_decision_records() {
    // A finite typed counterexample; the source Engine trace is not relabeled as a new execution.
    let mut i = manual();
    let mut event = i[0]
        .3
        .events
        .iter()
        .find(|e| matches!(e.payload, SimulationEventPayload::QualityChanged(_)))
        .unwrap()
        .clone();
    let SimulationEventPayload::QualityChanged(d) = &mut event.payload else {
        unreachable!()
    };
    d.disposition = QualityDisposition::Scrap;
    let item = d.item_ref.clone();
    i[0].3.events = vec![event.clone(), event];
    let mut c = cost(
        "scrap",
        "worker-assembly",
        "explicit_disposal",
        json!({"coefficient":2,"decimal_places":0}),
        json!("CNY"),
    );
    c.object_ref = ObjectRef {
        kind: ObjectKind::Item,
        id: item,
    };
    c.component = "disposal".into();
    c.rate_quantity.unit = UnitId::try_from("piece".to_owned()).unwrap();
    i[0].1.cost_inputs = vec![c];
    let report = build(&i);
    let c = &report.data.run_evaluations[0].costs.lines[0];
    assert_eq!(
        c.measured_quantity
            .as_ref()
            .unwrap()
            .value
            .coefficient
            .value(),
        1
    );
    assert_eq!(c.amount.as_ref().unwrap().value(), 2.);
}

#[test]
fn repeated_cost_identity_is_counted_once_and_conflicting_price_never_chosen_silently() {
    let mut i = resource_fixture();
    let id = i[0].0.resources[0].id.as_str().to_owned();
    let c = cost(
        "same-cost",
        &id,
        "occupied_time",
        json!({"coefficient":36,"decimal_places":0}),
        json!("CNY"),
    );
    i[0].1.cost_inputs = vec![c.clone(), c];
    let report = build(&i);
    let costs = &report.data.run_evaluations[0].costs;
    assert_eq!(costs.lines.len(), 1);
    assert!((costs.lines[0].amount.as_ref().unwrap().value() - 1.1).abs() < 1e-12);
    assert_eq!(costs.inputs.len(), 2);
    i[0].1.cost_inputs[1].rate.as_mut().unwrap().coefficient = SafeUint::try_from(72u64).unwrap();
    let report = build(&i);
    let costs = &report.data.run_evaluations[0].costs;
    assert_eq!(costs.lines.len(), 1);
    assert!(costs.lines[0].amount.is_none());
    assert!(costs.currencies[0].complete_total.is_none());
}
