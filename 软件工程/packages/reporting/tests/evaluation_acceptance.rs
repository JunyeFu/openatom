//! Independent P2-06 contract consumers over actual recorded Engine Runs.
use serde_json::{Value, json};
use sf_domain::*;
use sf_reporting::*;
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
type Frozen = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);
fn recorded() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/evaluation-acceptance/actual-engine-inputs.json"
    ))
    .unwrap()
}
fn input(name: &str) -> Frozen {
    serde_json::from_value(recorded()["inputs"][name].clone()).unwrap()
}
fn scope_a() -> AcceptanceScope {
    serde_json::from_value(json!({"product_ref":{"id":"A","revision":"r1"},"unit":"piece","destination":"customer","stage":"unloaded_and_released"})).unwrap()
}
fn context(inputs: &[Frozen], design: bool) -> EvaluationReport {
    let p = &inputs[0].0;
    serde_json::from_value(json!({"id":"W5-P206-independent","mode":if design{"design_description"}else{"simulation_evaluation"},
        "project_id":p.id,"project_revision":p.revision,"scenario_refs":inputs.iter().map(|i|json!({"id":i.1.id,"revision":i.1.revision})).collect::<Vec<_>>(),
        "run_refs":if design{vec![]}else{inputs.iter().map(|i|i.3.id.clone()).collect::<Vec<_>>()},"time_reference":inputs[0].1.time_reference,
        "observation_window":inputs[0].1.window,"acceptance_scope":[],
        "question_and_scope":"独立评估，\"保留逗号与引号\"\n第二行 =SUM(1+1) <script>window.__w5_injected=1</script>",
        "input_assumptions":[],"module_versions":p.module_refs,"layout_and_schedule":[],"scenario_comparison":null,
        "conclusions":[],"missing_conditions":[],"supported_scope":["声明输入下的软件生产结果；未取得现场或软件性能实测"],
        "limitations":[],"editable_project_ref":null,"asset_rights":[],"generation_context":p.generation_context,"run_evaluations":[]})).unwrap()
}
fn build(names: &[&str], scopes: &[AcceptanceScope]) -> Versioned<EvaluationReport> {
    let inputs: Vec<_> = names.iter().map(|n| input(n)).collect();
    build_evaluation(context(&inputs, false), &inputs, &[], &[], scopes)
        .unwrap_or_else(|e| panic!("actual frozen report input rejected {e:#?}"))
}
fn value<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}
fn record_report(name: &str, r: &Versioned<EvaluationReport>) {
    if let Some(base) = std::env::var_os("W5_EVALUATION_OUTPUT") {
        let p = std::path::PathBuf::from(base)
            .join(name)
            .join("context.json");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, serde_json::to_vec_pretty(r).unwrap()).unwrap();
    }
}
fn metric<'a>(r: &'a Versioned<EvaluationReport>, id: &str) -> &'a ResourceMetric {
    r.data.run_evaluations[0]
        .resource_metrics
        .iter()
        .find(|m| m.resource_id.as_str() == id)
        .unwrap()
}
fn currency(r: &Versioned<EvaluationReport>) -> &CurrencyCostSummary {
    r.data.run_evaluations[0]
        .costs
        .currencies
        .iter()
        .find(|c| c.currency == "CNY")
        .unwrap()
}
fn write_files(name: &str, files: &[EvaluationFile]) {
    if let Some(base) = std::env::var_os("W5_EVALUATION_OUTPUT") {
        let base = std::path::PathBuf::from(base).join(name);
        for f in files {
            let p = base.join(&f.relative_path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, &f.bytes).unwrap();
        }
    }
}
fn resources() -> EvaluationResources {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    EvaluationResources {
        cjk_font: Some(
            std::fs::read(root.join("tools/architecture-proof/assets/SourceHanSansCN-Regular.otf"))
                .unwrap(),
        ),
        font_license: Some(
            std::fs::read(root.join("tools/architecture-proof/assets/LICENSE.SourceHanSans.txt"))
                .unwrap(),
        ),
        files: BTreeMap::new(),
    }
}
#[test]
fn single_real_success_is_delivered_without_an_invented_revision_or_comparison() {
    let r = build(&["halls_b"], &[]);
    let run = &r.data.run_evaluations[0];
    assert_eq!(r.data.run_refs.len(), 1);
    assert_eq!(r.data.run_evaluations.len(), 1);
    assert!(r.data.scenario_comparison.is_none());
    assert_eq!(run.goal_met, Some(true));
    assert_eq!(
        run.order_metrics[0]
            .completion_ms
            .as_ref()
            .unwrap()
            .0
            .value(),
        205000
    );
    assert_eq!(run.order_metrics[0].accepted.value.coefficient.value(), 2);
    assert_eq!(
        value(&run.input_ref),
        json!({"project_id":input("halls_b").3.input_ref.project_id,"project_revision":1,"scenario_id":input("halls_b").1.id,"scenario_revision":1})
    );
    assert!(!r.data.module_versions.is_empty());
    assert!(r.data.layout_and_schedule.is_empty());
}
#[test]
fn incomplete_order_and_real_cancelled_prefix_never_become_zero_tardiness_or_full_window_goal() {
    let r = build(&["halls_a"], &[]);
    let order = &r.data.run_evaluations[0].order_metrics[0];
    assert_eq!(order.accepted.value.coefficient.value(), 1);
    assert_eq!(order.outstanding.value.coefficient.value(), 1);
    assert!(order.completion_ms.is_none());
    assert!(order.tardiness_ms.is_none());
    let r = build(&["quality_partial"], &[]);
    let run = &r.data.run_evaluations[0];
    assert!(!run.whole_window_computed);
    assert!(run.goal_met.is_none());
    assert!(
        run.computed_until_ms.as_ref().unwrap().0.value() < run.requested_window.end_ms.0.value()
    );
    assert!(currency(&r).complete_total.is_none());
    assert!(currency(&r).unit_cost.is_none());
}
#[test]
fn actual_resource_union_and_capacity_integral_are_separate_and_known_cost_is_not_double_counted() {
    let a = build(&["order_known"], &[scope_a()]);
    record_report("known-cost", &a);
    let m = metric(&a, "W-A");
    assert_eq!(m.any_busy_ms.0.value(), 135000);
    assert_eq!(m.occupied_capacity_ms.value(), 135000);
    assert_eq!(m.available_capacity_ms.as_ref().unwrap().value(), 240000);
    assert_eq!(m.utilization.as_ref().unwrap().value(), 0.5625);
    assert_eq!(m.effective_work_ms.as_ref().unwrap().0.value(), 135000);
    let b = build(&["order_capacity_two"], &[scope_a()]);
    let m = metric(&b, "W-A");
    assert_eq!(m.any_busy_ms.0.value(), 135000);
    assert_eq!(m.occupied_capacity_ms.value(), 135000);
    assert_eq!(m.available_capacity_ms.as_ref().unwrap().value(), 480000);
    assert_eq!(m.utilization.as_ref().unwrap().value(), 0.28125);
    let c = currency(&a);
    assert_eq!(c.known_subtotal.value(), 11.35);
    assert_eq!(c.complete_total.as_ref().unwrap().value(), 11.35);
    assert_eq!(
        c.accepted_quantity
            .as_ref()
            .unwrap()
            .value
            .coefficient
            .value(),
        2
    );
    assert_eq!(c.unit_cost.as_ref().unwrap().value(), 5.675);
    let amounts: Vec<_> = a.data.run_evaluations[0]
        .costs
        .lines
        .iter()
        .map(|c| c.amount.as_ref().unwrap().value())
        .collect();
    assert_eq!(amounts, vec![1.35, 10.0, 0.0]);
}
#[test]
fn missing_currency_and_conflicting_labor_bases_leave_complete_cost_unknown() {
    let r = build(&["order_unknown"], &[scope_a()]);
    record_report("unknown-cost", &r);
    let c = currency(&r);
    assert_eq!(c.known_subtotal.value(), 11.35);
    assert!(c.complete_total.is_none());
    assert!(c.unit_cost.is_none());
    assert!(r.data.run_evaluations[0].costs.inputs[2].currency.is_none());
    let r = build(&["order_dual"], &[scope_a()]);
    assert!(currency(&r).complete_total.is_none());
    assert!(
        r.data.run_evaluations[0]
            .costs
            .lines
            .iter()
            .any(|l| !l.missing_conditions.is_empty())
            || !r.data.run_evaluations[0]
                .costs
                .missing_conditions
                .is_empty()
    );
}
#[test]
fn multiple_acceptance_scopes_are_not_implicitly_assigned_a_unit_cost() {
    let r = build(&["order_known"], &[]);
    record_report("multi-scope-cost", &r);
    let c = currency(&r);
    assert_eq!(c.complete_total.as_ref().unwrap().value(), 11.35);
    assert!(c.unit_cost.is_none());
    assert!(c.accepted_scope.is_none());
}
#[test]
fn original_ix01_times_and_different_windows_retain_each_real_run_condition() {
    let r = build(&["ix_base", "ix_late"], &[]);
    assert_eq!(r.data.run_evaluations.len(), 2);
    assert!(r.data.scenario_comparison.is_some());
    for (run, last) in r.data.run_evaluations.iter().zip([440000, 570000]) {
        assert_eq!(
            run.order_metrics[0]
                .completion_ms
                .as_ref()
                .unwrap()
                .0
                .value(),
            last
        );
        assert_eq!(run.order_metrics[0].accepted.value.coefficient.value(), 6);
        assert_eq!(run.requested_window.end_ms.0.value(), 600000);
    }
    let r = build(&["ix_base", "halls_b"], &[]);
    assert!(r.data.observation_window.is_none());
    assert_eq!(
        r.data.run_evaluations[0].requested_window.end_ms.0.value(),
        600000
    );
    assert_eq!(
        r.data.run_evaluations[1].requested_window.end_ms.0.value(),
        240000
    );
}
#[test]
fn no_run_design_has_no_fabricated_metrics_and_fractional_base_survives_json_and_standard_csv() {
    let inputs = vec![input("halls_b")];
    let design = build_evaluation(context(&inputs, true), &[], &[], &[], &[]).unwrap();
    assert!(design.data.run_refs.is_empty());
    assert!(design.data.run_evaluations.is_empty());
    assert!(design.data.scenario_comparison.is_none());
    let report = build(&["quality_fraction"], &[]);
    assert_eq!(
        currency(&report).complete_total.as_ref().unwrap().value(),
        135000.0 / 7.0
    );
    let files = render_evaluation(
        &report,
        &[
            EvaluationOutputFormat::Json,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Html,
        ],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let json_file = files
        .iter()
        .find(|f| f.relative_path == "report.json")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&json_file.bytes).unwrap(),
        value(&report)
    );
    let costs = files
        .iter()
        .find(|f| f.relative_path == "csv/costs.csv")
        .unwrap();
    assert_eq!(&costs.bytes[..3], &[239, 187, 191]);
    let mut reader = csv::Reader::from_reader(&costs.bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let column = headers.iter().position(|h| h == "amount").unwrap();
    let id = headers.iter().position(|h| h == "cost_line_id").unwrap();
    let values: Vec<_> = reader
        .records()
        .map(|r| r.unwrap())
        .filter(|r| &r[id] == "fractional-cartons")
        .map(|r| r[column].parse::<f64>().unwrap())
        .collect();
    assert_eq!(values, vec![135000.0 / 7.0]);
    assert_eq!(
        files
            .iter()
            .filter(|f| f.relative_path.starts_with("csv/"))
            .count(),
        15
    );
}
#[test]
fn design_description_retains_known_declared_module_versions_instead_of_erasing_them() {
    let inputs = vec![input("halls_b")];
    let c = context(&inputs, true);
    let known = value(&c.module_versions);
    assert!(!c.module_versions.is_empty());
    let r = build_evaluation(c, &[], &[], &[], &[]).unwrap();
    record_report("design-metadata", &r);
    assert_eq!(value(&r.data.module_versions), known);
    assert!(r.data.run_evaluations.is_empty());
    assert!(r.data.run_refs.is_empty());
}
#[test]
fn standard_csv_retains_frozen_cost_component_source_and_rate_denominator_with_quotes_and_newline()
{
    let report = build(&["quality_escaped"], &[]);
    let input = &report.data.run_evaluations[0].costs.inputs[0];
    let files = render_evaluation(
        &report,
        &[
            EvaluationOutputFormat::Json,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Html,
        ],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    write_files("escaped", &files);
    let file = files
        .iter()
        .find(|f| f.relative_path == "csv/costs.csv")
        .unwrap();
    let mut reader = csv::Reader::from_reader(&file.bytes[3..]);
    let h = reader.headers().unwrap().clone();
    let id = h.iter().position(|v| v == "cost_line_id").unwrap();
    let row = reader
        .records()
        .map(Result::unwrap)
        .find(|v| &v[id] == input.id.as_str())
        .unwrap();
    // CSV cells contain raw scalar text or the contract's JSON-valued fields.
    // Decode those fields to test preservation without inventing new CSV aliases.
    fn leaves(v: &Value, out: &mut Vec<Value>) {
        match v {
            Value::Array(a) => a.iter().for_each(|v| leaves(v, out)),
            Value::Object(o) => o.values().for_each(|v| leaves(v, out)),
            _ => out.push(v.clone()),
        }
    }
    let mut fields = vec![];
    for cell in &row {
        let decoded =
            serde_json::from_str::<Value>(cell).unwrap_or_else(|_| Value::String(cell.into()));
        leaves(&decoded, &mut fields);
    }
    assert!(
        fields.contains(&json!(input.component)),
        "Frozen cost component disappeared from CSV"
    );
    assert!(
        fields.contains(&json!(input.source.field_path)),
        "Frozen source path disappeared from CSV"
    );
    assert!(
        fields.contains(&json!("user_provided")),
        "Frozen source basis disappeared from CSV"
    );
    assert!(
        fields.contains(&json!(7)) || fields.contains(&json!("7")),
        "Frozen rate denominator7 disappeared from CSV"
    );
}
#[test]
fn missing_pdf_resource_and_precommit_cancellation_return_no_partial_bundle() {
    let report = build(&["quality_fraction"], &[]);
    let before = value(&report);
    let e = render_evaluation(
        &report,
        &[
            EvaluationOutputFormat::Json,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Pdf,
        ],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(e.iter().any(|i| i.code == "evaluation.missing_font"));
    let e = render_evaluation(
        &report,
        &[EvaluationOutputFormat::Json],
        &EvaluationResources::default(),
        &AtomicBool::new(true),
    )
    .unwrap_err();
    assert!(e.iter().any(|i| i.code == "evaluation.cancelled"));
    assert_eq!(value(&report), before);
}
#[test]
fn actual_printpdf_four_formats_read_one_context_for_fractional_actual_and_no_run_design() {
    let inputs = vec![input("halls_b")];
    let design = build_evaluation(context(&inputs, true), &[], &[], &[], &[]).unwrap();
    for (name, report) in [
        ("fractional", build(&["quality_fraction"], &[])),
        ("design", design),
    ] {
        let before = value(&report);
        let files = render_evaluation(
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
        assert_eq!(value(&report), before);
        let pdf = files
            .iter()
            .find(|f| f.relative_path == "report.pdf")
            .unwrap();
        assert!(pdf.bytes.starts_with(b"%PDF-"));
        assert!(
            files
                .iter()
                .any(|f| f.relative_path == "licenses/SourceHanSans-LICENSE.txt")
        );
        let encoded = files
            .iter()
            .find(|f| f.relative_path == "report.json")
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&encoded.bytes).unwrap(),
            before
        );
        write_files(name, &files);
    }
}

#[test]
fn printable_history_examples_keep_each_run_and_complete_machine_readable_details() {
    fn pdf_text(files: &[EvaluationFile]) -> (usize, String) {
        let file = files
            .iter()
            .find(|f| f.relative_path == "report.pdf")
            .unwrap();
        let doc = printpdf::PdfDocument::parse(
            &file.bytes,
            &printpdf::PdfParseOptions::default(),
            &mut vec![],
        )
        .unwrap();
        let text = doc.extract_text().into_iter().flatten().collect::<String>();
        (
            doc.pages.len(),
            text.chars().filter(|c| !c.is_whitespace()).collect(),
        )
    }
    // Projection-only fixtures cover late conditions, repeated object warnings
    // and a serialized initial-item snapshot; they are not extra Engine Runs.
    let mut report = build(&["halls_a", "halls_b"], &[]);
    report.data.run_evaluations[0]
        .waits
        .last_mut()
        .unwrap()
        .reasons
        .push(WaitReason {
            kind: WaitReasonKind::Quality,
            blocking_object: None,
            required: "output-projection-only late condition".into(),
            next_known_boundary_ms: None,
        });
    let repeated: Issue = serde_json::from_value(json!({"code":"render.fixture_limitation","object_ref":{"kind":"item","id":"PDF-item-0"},"field_path":"/display","message":"Repeated display limitation","blocks":[]})).unwrap();
    report.data.missing_conditions.push(repeated.clone());
    let mut distinct = repeated.clone();
    distinct.field_path = Some("/different-field".into());
    distinct.message = "Distinct decision condition".into();
    report.data.missing_conditions.push(distinct);
    let scenario = input("halls_a").1;
    report.data.conclusions.push(format!(
        "明确选择的方案输入快照：{}",
        serde_json::to_string(&scenario).unwrap()
    ));
    let formats = [
        EvaluationOutputFormat::Pdf,
        EvaluationOutputFormat::Csv,
        EvaluationOutputFormat::Json,
    ];
    let original_files =
        render_evaluation(&report, &formats, &resources(), &AtomicBool::new(false)).unwrap();
    let (original_pages, _) = pdf_text(&original_files);
    // Output-size fixture only: repeats exercise pagination without pretending
    // that these extra rows are a newly computed Engine Run.
    let mut expanded = report.clone();
    let example_event = expanded.data.run_evaluations[0].events[0].clone();
    expanded.data.run_evaluations[0]
        .events
        .extend(vec![example_event; 500]);
    for index in 1..=100 {
        let mut issue = repeated.clone();
        issue.object_ref = Some(
            serde_json::from_value(json!({"kind":"item","id":format!("PDF-item-{index}")}))
                .unwrap(),
        );
        expanded.data.missing_conditions.push(issue);
    }
    let mut larger_scenario = scenario.clone();
    larger_scenario.initial_conditions.items.extend(vec![
        scenario.initial_conditions.items[0]
            .clone();
        500
    ]);
    *expanded.data.conclusions.last_mut().unwrap() = format!(
        "明确选择的方案输入快照：{}",
        serde_json::to_string(&larger_scenario).unwrap()
    );
    let before = value(&expanded);
    let files =
        render_evaluation(&expanded, &formats, &resources(), &AtomicBool::new(false)).unwrap();
    let (pages, text) = pdf_text(&files);
    assert!(
        pages <= original_pages + 2,
        "Raw history expanded a printable report from {original_pages} to {pages} pages"
    );
    assert!(text.contains("每Run前3条阅读示例"));
    assert!(text.contains("csv/events.csv"));
    assert!(text.contains("等待原因类别quality："));
    assert!(text.contains("仅resolved表示该等待已解除"));
    assert!(text.contains("相同条件共101条"));
    assert!(text.contains("Distinctdecisioncondition"));
    assert!(text.contains("/different-field"));
    assert!(text.contains("初始物品共505条记录"));
    for (index, run) in expanded.data.run_evaluations.iter().enumerate() {
        assert!(text.contains(&format!(
            "Run{}：{}条；展示3条",
            run.run_id.as_str(),
            run.events.len()
        )));
        assert!(text.contains(&format!("report.json#/data/run_evaluations/{index}/events")));
    }
    let encoded = files
        .iter()
        .find(|f| f.relative_path == "report.json")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&encoded.bytes).unwrap(),
        before
    );
    let events = files
        .iter()
        .find(|f| f.relative_path == "csv/events.csv")
        .unwrap();
    let mut reader = csv::Reader::from_reader(&events.bytes[3..]);
    let records: Vec<_> = reader.records().collect::<Result<_, _>>().unwrap();
    assert_eq!(
        records.len(),
        expanded
            .data
            .run_evaluations
            .iter()
            .map(|r| r.events.len())
            .sum::<usize>()
    );
    for run in &expanded.data.run_evaluations {
        assert_eq!(
            records
                .iter()
                .filter(|row| &row[1] == run.run_id.as_str())
                .count(),
            run.events.len()
        );
    }
    assert_eq!(value(&expanded), before);
    write_files("readable-history", &files);

    let only_pdf = render_evaluation(
        &expanded,
        &[EvaluationOutputFormat::Pdf],
        &resources(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let (_, text) = pdf_text(&only_pdf);
    assert!(text.contains("本次未选择JSON或CSV"));
    assert!(!text.contains("report.json#/"));
    assert!(!text.contains("csv/events.csv"));
    assert!(
        !only_pdf
            .iter()
            .any(|f| f.relative_path == "report.json" || f.relative_path.starts_with("csv/"))
    );
    assert_eq!(value(&expanded), before);
}

#[test]
fn actual_paid_calendar_decimal_rate_multiplies_before_one_binary_conversion() {
    let report = build(&["exact_decimal"], &[]);
    let cost = &report.data.run_evaluations[0].costs;
    assert_eq!(value(&cost.lines[0].amount), json!(0.02));
    assert_eq!(value(&cost.currencies[0].known_subtotal), json!(0.02));
    let files = render_evaluation(
        &report,
        &[
            EvaluationOutputFormat::Json,
            EvaluationOutputFormat::Csv,
            EvaluationOutputFormat::Html,
        ],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let csv = files
        .iter()
        .find(|f| f.relative_path == "csv/costs.csv")
        .unwrap();
    let mut reader = csv::Reader::from_reader(csv.bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap());
    let headers = reader.headers().unwrap().clone();
    let id = headers.iter().position(|x| x == "cost_line_id").unwrap();
    let amount = headers.iter().position(|x| x == "amount").unwrap();
    let rows: Vec<_> = reader.records().map(Result::unwrap).collect();
    let row = rows.iter().find(|x| &x[id] == "exact-decimal").unwrap();
    assert_eq!(row[amount].parse::<f64>().unwrap(), 0.02);
    write_files("exact-decimal", &files);
}

#[test]
fn design_and_cross_run_csv_boundaries_are_not_applicable_while_each_actual_run_keeps_its_boundary()
{
    let input = input("quality_fraction");
    let design = build_evaluation(
        context(std::slice::from_ref(&input), true),
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let comparison = build(&["order_known", "order_b"], &[]);
    let mut design_rows = 0;
    let mut cross_rows = 0;
    let mut actual_rows = 0;
    for (is_design, report) in [(true, &design), (false, &comparison)] {
        let files = render_evaluation(
            report,
            &[EvaluationOutputFormat::Csv],
            &EvaluationResources::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        for file in files.iter().filter(|f| f.relative_path.ends_with(".csv")) {
            let mut reader =
                csv::Reader::from_reader(file.bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap());
            let headers = reader.headers().unwrap().clone();
            let run = headers.iter().position(|v| v == "run_id").unwrap();
            let boundary = headers
                .iter()
                .position(|v| v == "computed_until_ms")
                .unwrap();
            let state = headers
                .iter()
                .position(|v| v == "computed_until_ms_state")
                .unwrap();
            for row in reader.records() {
                let row = row.unwrap();
                if is_design || row[run].is_empty() {
                    assert_eq!(&row[boundary], "");
                    assert_eq!(&row[state], "not_applicable");
                    if is_design {
                        design_rows += 1;
                    } else {
                        cross_rows += 1;
                    }
                } else {
                    assert_eq!(&row[state], "known");
                    assert!(!row[boundary].is_empty());
                    actual_rows += 1;
                }
            }
        }
    }
    assert!(design_rows > 0 && cross_rows > 0 && actual_rows > 0);
}
