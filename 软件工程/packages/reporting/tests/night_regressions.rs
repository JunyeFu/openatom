//! Finite typed counterexamples over recorded inputs, not new Engine executions.
use serde_json::{json, Value};
use sf_domain::*;
use sf_reporting::*;
use std::{collections::BTreeMap, path::Path, sync::atomic::AtomicBool};

type Frozen = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);

fn input(name: &str) -> Frozen {
    let recorded: Value = serde_json::from_str(include_str!(
        "fixtures/evaluation-acceptance/actual-engine-inputs.json"
    ))
    .unwrap();
    serde_json::from_value(recorded["inputs"][name].clone()).unwrap()
}

fn context(i: &Frozen) -> EvaluationReport {
    serde_json::from_value(json!({
        "id":"night-reporting-counterexample", "mode":"simulation_evaluation",
        "project_id":i.0.id, "project_revision":i.0.revision,
        "scenario_refs":[], "run_refs":[], "time_reference":null,
        "observation_window":null, "acceptance_scope":[],
        "question_and_scope":"有限类型反例，不作为新增实际运行或真实价格证据",
        "input_assumptions":[], "module_versions":[], "layout_and_schedule":[],
        "scenario_comparison":null, "conclusions":[], "missing_conditions":[],
        "supported_scope":[], "limitations":[], "editable_project_ref":null,
        "asset_rights":[], "generation_context":null, "run_evaluations":[]
    }))
    .unwrap()
}

fn report(i: &Frozen) -> EvaluationReport {
    build_evaluation(context(i), std::slice::from_ref(i), &[], &[], &[])
        .unwrap()
        .data
}

#[test]
fn consumed_material_remains_chargeable_while_its_job_waits_after_consumption() {
    let mut i = input("order_known");
    let item = i
        .3
        .final_items
        .iter()
        .find(|t| {
            t.item_type_ref.id.as_str() == "carton" && matches!(t.lifecycle, Lifecycle::Transformed)
        })
        .unwrap()
        .clone();
    // A phase can consume an auxiliary before its output/return phase completes.
    // The transformed item remains the actual consumption fact in the snapshot.
    for job in
        i.3.jobs
            .iter_mut()
            .filter(|j| j.input_refs.contains(&item.id))
    {
        job.finished_at_ms = None;
        job.state = JobStatus::WaitingOutput;
    }
    i.1.cost_inputs = vec![serde_json::from_value(json!({
        "id":"auxiliary", "object_ref":{"kind":"item","id":item.id},
        "component":"auxiliary", "basis":"consumed_material",
        "rate":{"coefficient":2,"decimal_places":0}, "currency":"CNY",
        "rate_quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},
        "includes":[], "source":{"target":{"kind":"item","id":item.id},
            "field_path":"/test_rate", "value":null,"basis":"user_provided","source_ref":null}
    }))
    .unwrap()];
    let r = report(&i);
    let line = &r.run_evaluations[0].costs.lines[0];
    assert_eq!(
        line.measured_quantity
            .as_ref()
            .unwrap()
            .value
            .coefficient
            .value(),
        1
    );
    assert_eq!(line.amount.as_ref().unwrap().value(), 2.0);
}

#[test]
fn distinct_components_can_use_calendar_and_occupied_bases_without_double_charging() {
    let mut i = input("order_known");
    let mut occupied = i.1.cost_inputs[0].clone();
    occupied.component = "active_service".into();
    let mut calendar = occupied.clone();
    calendar.id = ObjectId::try_from("shift-access".to_owned()).unwrap();
    calendar.component = "shift_access".into();
    calendar.basis = CostBasis::PaidCalendar;
    i.1.cost_inputs = vec![occupied, calendar];
    let r = report(&i);
    let costs = &r.run_evaluations[0].costs;
    assert_eq!(
        costs.lines[0].amount.as_ref().map(|n| n.value()),
        Some(1.35)
    );
    assert_eq!(costs.lines[1].amount.as_ref().map(|n| n.value()), Some(2.4));
    assert_eq!(
        costs.currencies[0]
            .complete_total
            .as_ref()
            .map(|n| n.value()),
        Some(3.75)
    );
}

#[test]
fn transport_comparison_resolves_actual_job_waits_without_changing_resource_union() {
    let mut i = input("halls_b");
    let claim =
        i.3.resource_claims
            .iter()
            .find(|c| matches!(c.kind, ResourceClaimKind::Transport))
            .unwrap()
            .clone();
    assert_eq!(claim.intervals[0].start_ms.0.value(), 60000);
    assert_eq!(claim.intervals[0].end_ms.0.value(), 140000);
    let mut first = i.3.waits[0].clone();
    first.object_ref = ObjectRef {
        kind: ObjectKind::Job,
        id: claim.job_id.clone(),
    };
    first.execution_ref = None;
    first.phase_ref = None;
    first.interval = serde_json::from_value(json!({"start_ms":70000,"end_ms":100000})).unwrap();
    let mut overlapping = first.clone();
    overlapping.interval =
        serde_json::from_value(json!({"start_ms":90000,"end_ms":120000})).unwrap();
    let mut unrelated = first.clone();
    unrelated.object_ref.id = ObjectId::try_from("unrelated-trip".to_owned()).unwrap();
    unrelated.interval = serde_json::from_value(json!({"start_ms":60000,"end_ms":140000})).unwrap();
    i.3.waits = vec![first, overlapping, unrelated];
    let r = report(&i);
    let resource = r.run_evaluations[0]
        .resource_metrics
        .iter()
        .find(|r| r.resource_id == claim.resource_id)
        .unwrap();
    assert_eq!(
        resource.waiting_ms.as_ref().map(|v| v.0.value()),
        Some(50000)
    );
    let mut candidate = i.clone();
    candidate.3.id = RunId::try_from("typed-transport-wait-comparison".to_owned()).unwrap();
    let compared = compare_runs(
        &[i, candidate],
        &[MetricKey::WaitingTime],
        &[ObjectRef {
            kind: ObjectKind::Transport,
            id: claim.job_id,
        }],
        &[],
    )
    .unwrap();
    assert!(
        compared.metrics[0]
            .values
            .iter()
            .all(|v| matches!(&v.value,MetricValue::Duration(t) if t.0.value()==50000)),
        "{:?}",
        compared.metrics
    );
}

#[test]
fn long_identifiers_and_chinese_are_written_in_the_same_four_format_design_report() {
    let i = input("order_known");
    let mut c = context(&i);
    c.mode = ReportMode::DesignDescription;
    c.question_and_scope = format!(
        "长标识：{}\n中文跨页与文本保留：{}",
        "W".repeat(110),
        "中文与宽字符W混合，逗号,引号\"和外部文本<script>不执行</script>。".repeat(35)
    );
    let r = build_evaluation(c, &[], &[], &[], &[]).unwrap();
    export(&r, "wide-design");
}

fn export(r: &Versioned<EvaluationReport>, name: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let resources = EvaluationResources {
        cjk_font: Some(
            std::fs::read(root.join("tools/architecture-proof/assets/SourceHanSansCN-Regular.otf"))
                .unwrap(),
        ),
        font_license: Some(
            std::fs::read(root.join("tools/architecture-proof/assets/LICENSE.SourceHanSans.txt"))
                .unwrap(),
        ),
        files: BTreeMap::new(),
    };
    let files = render_evaluation(
        r,
        &[
            EvaluationOutputFormat::Pdf,
            EvaluationOutputFormat::Html,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Json,
        ],
        &resources,
        &AtomicBool::new(false),
    )
    .unwrap();
    let encoded = files
        .iter()
        .find(|f| f.relative_path == "report.json")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&encoded.bytes).unwrap(),
        serde_json::to_value(r).unwrap()
    );
    assert_eq!(
        files
            .iter()
            .filter(|f| f.relative_path.ends_with(".csv"))
            .count(),
        15
    );
    if let Some(out) = std::env::var_os("NIGHT_DELIVERY_OUTPUT") {
        for file in files {
            let path = Path::new(&out).join(name).join(file.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, file.bytes).unwrap();
        }
    }
}

#[test]
fn different_actual_windows_keep_all_four_formats_side_by_side_without_delta() {
    let inputs = [input("order_known"), input("exact_decimal")];
    assert_ne!(
        inputs[0].3.window.end_ms.0.value(),
        inputs[1].3.window.end_ms.0.value()
    );
    let mut c = context(&inputs[0]);
    c.question_and_scope = "既有冻结软件Run的不同观察窗分列，非现场观测；不报告同窗改善量".into();
    let r = build_evaluation(
        c,
        &inputs,
        &[MetricKey::AcceptedQuantity, MetricKey::CompletionTime],
        &[],
        &[],
    )
    .unwrap();
    let comparison = r.data.scenario_comparison.as_ref().unwrap();
    assert!(matches!(
        comparison.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert!(comparison
        .metrics
        .iter()
        .flat_map(|m| &m.pair_differences)
        .all(|d| !d.comparable && d.difference.is_none() && d.change_percent.is_none()));
    assert!(r.data.observation_window.is_none());
    export(&r, "incomparable");
}

#[test]
fn actual_cancelled_prefix_is_exported_without_full_window_or_complete_cost() {
    let i = input("quality_partial");
    let mut c = context(&i);
    c.question_and_scope =
        "既有真实取消前缀，只呈现已知计算边界；未知尾项、完整接受和成本不补零".into();
    let r = build_evaluation(c, std::slice::from_ref(&i), &[], &[], &[]).unwrap();
    let run = &r.data.run_evaluations[0];
    assert!(!run.whole_window_computed && run.goal_met.is_none());
    assert!(run
        .costs
        .currencies
        .iter()
        .all(|c| c.complete_total.is_none() && c.unit_cost.is_none()));
    export(&r, "cancelled-prefix");
}

#[test]
fn wide_run_identity_is_wrapped_in_repeated_page_headings() {
    let mut i = input("halls_b");
    // Only the identity is changed: a typography type fixture, not another Run.
    i.3.id = RunId::try_from("W".repeat(110)).unwrap();
    let mut c = context(&i);
    c.question_and_scope = "有限排版类型反例：长Run标识与跨页续标，不是新的实际执行".into();
    let r = build_evaluation(c, std::slice::from_ref(&i), &[], &[], &[]).unwrap();
    export(&r, "wide-run");
}

#[test]
fn comparison_csv_units_follow_the_metric_instead_of_the_accepted_product() {
    fn rows(r: &Versioned<EvaluationReport>) -> Vec<BTreeMap<String, String>> {
        let files = render_evaluation(
            r,
            &[EvaluationOutputFormat::Csv],
            &EvaluationResources::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let file = files
            .iter()
            .find(|f| f.relative_path == "csv/comparison.csv")
            .unwrap();
        let mut reader = csv::Reader::from_reader(&file.bytes[3..]);
        let columns = reader.headers().unwrap().clone();
        reader
            .records()
            .map(|r| {
                columns
                    .iter()
                    .zip(r.unwrap().iter())
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect()
            })
            .collect()
    }
    let inputs = [input("halls_a"), input("halls_b")];
    let r = build_evaluation(
        context(&inputs[0]),
        &inputs,
        &[
            MetricKey::CompletionTime,
            MetricKey::WaitingTime,
            MetricKey::ResourceUtilization,
        ],
        &[],
        &[],
    )
    .unwrap();
    let actual = rows(&r);
    assert!(actual.iter().any(|r| r["metric_key"] == "completion_time"));
    assert!(actual
        .iter()
        .any(|r| r["metric_key"] == "resource_utilization"));
    for row in actual {
        assert_eq!(
            row["value_unit"],
            if row["metric_key"] == "resource_utilization" {
                "ratio"
            } else {
                "ms"
            }
        );
        assert_eq!(row["value_unit_state"], "known");
    }
    let a = input("quality_fraction");
    let mut b = a.clone();
    b.3.id = RunId::try_from("typed-repeat-for-unit-serialization".to_owned()).unwrap();
    let r = build_evaluation(context(&a), &[a, b], &[MetricKey::ReworkQuantity], &[], &[]).unwrap();
    let actual = rows(&r);
    assert!(!actual.is_empty());
    assert!(actual.iter().all(|r| r["value_unit"] == "piece"));
}

#[test]
fn optional_asset_metadata_preserves_unknown_empty_and_known_without_granting_empty_rights() {
    let i = input("halls_b");
    let mut c = context(&i);
    c.mode = ReportMode::DesignDescription;
    c.question_and_scope = "可空元数据序列化类型反例；不引用或分发真实素材，不新增许可授权".into();
    c.asset_rights = serde_json::from_value(json!([
        {"asset_ref":"unknown-metadata","source":null,"license":null,"redistribution_confirmed":false,"missing_reason":"来源与许可未提供"},
        {"asset_ref":"empty-metadata","source":"","license":"","redistribution_confirmed":true,"missing_reason":""},
        {"asset_ref":"known-metadata","source":"self-authored metadata fixture","license":"CC0-1.0","redistribution_confirmed":true,"missing_reason":null}
    ])).unwrap();
    c.missing_conditions = serde_json::from_value(json!([
        {"code":"typed.unknown","message":"未知对象与字段保留原因","object_ref":null,"field_path":null,"blocks":[]},
        {"code":"typed.known","message":"已知对象与字段保持原值","object_ref":{"kind":"project","id":i.0.id},"field_path":"/fixture","blocks":[]},
        {"code":"typed.empty","message":"显式空字符串保持empty","object_ref":null,"field_path":"","blocks":[]}
    ])).unwrap();
    let r = build_evaluation(c, &[], &[], &[], &[]).unwrap();
    let files = render_evaluation(
        &r,
        &[EvaluationOutputFormat::Csv],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let file = files
        .iter()
        .find(|f| f.relative_path == "csv/assets.csv")
        .unwrap();
    let mut reader = csv::Reader::from_reader(&file.bytes[3..]);
    let columns = reader.headers().unwrap().clone();
    let rows: Vec<BTreeMap<String, String>> = reader
        .records()
        .map(|r| {
            columns
                .iter()
                .zip(r.unwrap().iter())
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect()
        })
        .collect();
    assert_eq!(rows[0]["source_state"], "unknown");
    assert!(!rows[0]["source_reason"].is_empty());
    assert_eq!(rows[1]["source_state"], "empty");
    assert_eq!(rows[1]["license_state"], "empty");
    assert_eq!(rows[1]["actual_export_authorized"], "false");
    assert_eq!(rows[2]["source_state"], "known");
    assert_eq!(rows[2]["actual_export_authorized"], "true");
    export(&r, "option-metadata");
}

#[test]
fn subtotal_and_unit_cost_convert_once_after_the_full_exact_cost_calculation() {
    let mut i = input("order_known");
    let mut a = i.1.cost_inputs[0].clone();
    a.component = "active_service".into();
    a.rate = Some(serde_json::from_value(json!({"coefficient":1,"decimal_places":1})).unwrap());
    a.rate_quantity.value.coefficient = SafeUint::try_from(135000u64).unwrap();
    let mut b = a.clone();
    b.id = ObjectId::try_from("fixed-access".to_owned()).unwrap();
    b.component = "shift_access".into();
    b.basis = CostBasis::PaidCalendar;
    b.rate = Some(serde_json::from_value(json!({"coefficient":2,"decimal_places":1})).unwrap());
    b.rate_quantity.value.coefficient = SafeUint::try_from(240000u64).unwrap();
    i.1.cost_inputs = vec![a, b];
    let order =
        i.1.orders
            .iter()
            .find(|o| o.product_ref.id.as_str() == "A")
            .unwrap();
    let scope = AcceptanceScope {
        product_ref: order.product_ref.clone(),
        unit: order.quantity.as_ref().unwrap().unit.clone(),
        destination: order.destination.clone(),
        stage: order.acceptance_stage.clone(),
    };
    let r = build_evaluation(context(&i), std::slice::from_ref(&i), &[], &[], &[scope]).unwrap();
    let cost = &r.data.run_evaluations[0].costs.currencies[0];
    assert_eq!(cost.known_subtotal.value().to_bits(), 0.3f64.to_bits());
    assert_eq!(
        cost.complete_total.as_ref().unwrap().value().to_bits(),
        0.3f64.to_bits()
    );
    assert_eq!(
        cost.unit_cost.as_ref().unwrap().value().to_bits(),
        0.15f64.to_bits()
    );
}
