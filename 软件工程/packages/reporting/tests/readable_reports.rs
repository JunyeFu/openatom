use sf_domain::*;
use sf_reporting::{EvaluationOutputFormat as Format, EvaluationResources, render_evaluation};
use std::sync::atomic::AtomicBool;

fn saved(repair: bool) -> Versioned<EvaluationReport> {
    let text = if repair {
        include_str!("fixtures/readable/repair.json")
    } else {
        include_str!("fixtures/readable/success.json")
    };
    let mut report: Versioned<EvaluationReport> = serde_json::from_str(text).unwrap();
    // The content regression has no image dependency; real image/PDF re-export
    // is exercised separately against the unmodified saved reports.
    report.data.layout_and_schedule.clear();
    report
}

fn html(report: &Versioned<EvaluationReport>) -> String {
    let files = render_evaluation(
        report,
        &[Format::Html],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    String::from_utf8(
        files
            .into_iter()
            .find(|f| f.relative_path == "index.html")
            .unwrap()
            .bytes,
    )
    .unwrap()
}

fn reading(html: &str) -> &str {
    html.split("<div id=\"report-reading\">")
        .nth(1)
        .expect("separate human reading layer")
        .split("</div>")
        .next()
        .unwrap()
}

#[test]
fn real_repair_explains_quantities_unknown_time_and_exact_release_change() {
    let text = html(&saved(true));
    let body = reading(&text);
    for expected in [
        "基线：1 piece",
        "候选：2 piece",
        "候选：65000 ms",
        "基线：未知",
        "任务释放时点",
        "30000 ms → 0 ms",
        "冻结工程修订",
        "接受位置",
        "不能计算时间改善",
        "差值：+1 piece",
        "完整接受时刻 C=未知",
    ] {
        assert!(body.contains(expected), "missing readable fact: {expected}");
    }
    for raw in [
        "\"coefficient\":",
        "\"template_adoptions\":",
        "\"initial_conditions\":",
        "C=null",
    ] {
        assert!(
            !body.contains(raw),
            "raw object wall leaked into prose: {raw}"
        );
    }
}

#[test]
fn real_success_stays_one_run_and_retains_scope_and_completion() {
    let report = saved(false);
    let text = html(&report);
    let body = reading(&text);
    assert_eq!(report.data.run_refs.len(), 1);
    assert!(body.contains("单次实际Run可直接交付"));
    assert!(body.contains("实际接受量 A=2 piece"));
    assert!(body.contains("C=45000ms"));
    assert!(body.contains("factory_released"));
    assert!(body.contains("cu-initial:store-released"));
    assert!(!body.contains("候选："));
}

#[test]
fn known_completion_times_with_incomparable_conditions_are_not_called_unknown() {
    let mut report = saved(true);
    let comparison = report.data.scenario_comparison.as_mut().unwrap();
    let completion = comparison.metrics.iter_mut()
        .find(|m| matches!(m.metric, MetricKey::CompletionTime)).unwrap();
    completion.values[0].value = serde_json::from_str(r#"{"kind":"duration","data":70000}"#).unwrap();
    let text = html(&report);
    let body = reading(&text);
    assert!(body.contains("基线：70000 ms"));
    assert!(body.contains("候选：65000 ms"));
    assert!(body.contains("不满足可比条件，不能计算时间改善"));
    assert!(!body.contains("基线或候选完工时刻未知"));
}

#[test]
fn raw_json_csv_and_self_contained_html_remain_lossless() {
    let report = saved(true);
    let original = serde_json::to_value(&report).unwrap();
    let files = render_evaluation(
        &report,
        &[Format::Html, Format::Json, Format::Csv],
        &EvaluationResources::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        files
            .iter()
            .filter(|f| f.relative_path.starts_with("csv/"))
            .count(),
        15
    );
    let json = &files
        .iter()
        .find(|f| f.relative_path == "report.json")
        .unwrap()
        .bytes;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(json).unwrap(),
        original
    );
    let text = std::str::from_utf8(
        &files
            .iter()
            .find(|f| f.relative_path == "index.html")
            .unwrap()
            .bytes,
    )
    .unwrap();
    let embedded = text
        .split("<script id=report-data type=application/json>")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(embedded).unwrap(),
        original
    );
    assert_eq!(serde_json::to_value(&report).unwrap(), original);
}

#[test]
fn unrecognized_human_conclusions_are_preserved_and_html_escaped() {
    let mut report = saved(false);
    report
        .data
        .conclusions
        .push("用户自定义原话：{\"keep\":1} <保留>".into());
    assert!(reading(&html(&report)).contains("用户自定义原话：{&quot;keep&quot;:1} &lt;保留&gt;"));
}

#[test]
fn partial_run_and_design_mode_do_not_gain_complete_results() {
    let mut report = saved(false);
    report.data.conclusions.clear();
    let run = &mut report.data.run_evaluations[0];
    run.whole_window_computed = false;
    run.goal_met = None;
    run.computed_until_ms = Some(serde_json::from_str("20000").unwrap());
    let text = html(&report);
    let body = reading(&text);
    assert!(body.contains("未覆盖完整观察窗"));
    assert!(body.contains("20000 ms"));
    assert!(body.contains("目标达成：未知"));
    report.data.mode = ReportMode::DesignDescription;
    report.data.run_refs.clear();
    report.data.run_evaluations.clear();
    let text = html(&report);
    assert!(reading(&text).contains("设计说明：无实际Run"));
    assert!(!reading(&text).contains("实际接受量"));
}

#[test]
fn unknown_costs_and_different_currencies_stay_separate() {
    let mut report = saved(false);
    report.data.run_evaluations[0].costs.currencies = serde_json::from_value(serde_json::json!([
        {"currency":"CNY","cost_input_ids":[],"known_subtotal":0,"complete_total":null,"accepted_scope":null,"accepted_quantity":null,"unit_cost":null,"missing_conditions":[]},
        {"currency":"USD","cost_input_ids":[],"known_subtotal":1.005,"complete_total":1.005,"accepted_scope":null,"accepted_quantity":null,"unit_cost":null,"missing_conditions":[]}
    ])).unwrap();
    let text = html(&report);
    let body = reading(&text);
    assert!(body.contains("CNY：已知小计0.00；完整总额未知"));
    assert!(body.contains("USD：已知小计1.01"));
    assert!(body.contains("不跨币种合计"));
}
