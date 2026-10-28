//! Shared lossless rows. Each output consumes this projection, not another calculator.
use crate::comparison::value;
use serde_json::{Value, json};
use sf_domain::*;
use std::collections::BTreeMap;

pub(crate) type Row = BTreeMap<String, Value>;
pub(crate) struct Table {
    pub name: String,
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}
pub(crate) fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}
fn array<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn assign(row: &mut Row, src: &Value, fields: &[&str]) {
    for f in fields {
        row.insert((*f).into(), src[*f].clone());
    }
}
fn prefix(report: &Value, run: Option<&Value>) -> Row {
    let mut r = Row::new();
    r.insert("report_id".into(), report["id"].clone());
    r.insert("project_id".into(), report["project_id"].clone());
    if let Some(s) = run {
        for (key, v) in [
            ("run_id", &s["run_id"]),
            ("project_id", &s["input_ref"]["project_id"]),
            (
                "input_project_revision",
                &s["input_ref"]["project_revision"],
            ),
            ("scenario_id", &s["input_ref"]["scenario_id"]),
            (
                "input_scenario_revision",
                &s["input_ref"]["scenario_revision"],
            ),
            ("window_start_ms", &s["requested_window"]["start_ms"]),
            ("window_end_ms", &s["requested_window"]["end_ms"]),
            ("computed_until_ms", &s["computed_until_ms"]),
        ] {
            r.insert(key.into(), v.clone());
        }
        r.insert(
            "value_state".into(),
            json!(if s["whole_window_computed"] == true {
                "known"
            } else {
                "partial"
            }),
        );
        r.insert(
            "reason".into(),
            json!(if s["whole_window_computed"] == true {
                ""
            } else {
                "仅已计算前缀，未知尾项未补齐"
            }),
        );
    } else {
        r.insert("computed_until_ms".into(), Value::Null);
        state(
            &mut r,
            "computed_until_ms",
            "not_applicable",
            "此行没有适用的单一实际Run，不能指定其计算边界",
        );
        r.insert("value_state".into(), json!("not_applicable"));
        r.insert(
            "reason".into(),
            json!("没有适用的单一实际Run；设计说明或跨Run行"),
        );
    }
    r
}
fn add(t: &mut Table, mut row: Row) {
    let cols = t.columns.clone();
    for key in &cols {
        if let Some(field) = key.strip_suffix("_state") {
            if row.contains_key(key) {
                continue;
            }
            let state = if row.get(field).is_none_or(Value::is_null) {
                "unknown"
            } else if row[field] == "" {
                "empty"
            } else {
                "known"
            };
            row.insert(key.clone(), json!(state));
            row.entry(format!("{field}_reason")).or_insert_with(|| {
                json!(if state == "unknown" {
                    "未声明、不适用或尚未观测；参见该行来源和限制"
                } else {
                    ""
                })
            });
        }
    }
    for col in &cols {
        row.entry(col.clone()).or_insert(Value::Null);
    }
    t.rows.push(row);
}
fn state(row: &mut Row, field: &str, status: &str, reason: &str) {
    row.insert(format!("{field}_state"), json!(status));
    row.insert(format!("{field}_reason"), json!(reason));
}
fn interval(row: &mut Row, t: &Value, run: &Value) {
    let a = t["start_ms"].as_u64().unwrap();
    let b = t["end_ms"].as_u64().unwrap();
    let start = run["requested_window"]["start_ms"].as_u64().unwrap();
    let end = run["computed_until_ms"].as_u64().unwrap_or(start);
    row.insert("true_start_ms".into(), json!(a));
    row.insert("true_end_ms".into(), json!(b));
    row.insert("clipped_start_ms".into(), json!(a.max(start).min(end)));
    row.insert(
        "clipped_end_ms".into(),
        json!(b.min(end).max(a.max(start).min(end))),
    );
}
fn execution(row: &mut Row, e: &Value) {
    if e["kind"] == "project_device" {
        row.insert("execution_device_id".into(), e["data"]["device_id"].clone());
        row.insert("route_step_id".into(), e["data"]["route_step_id"].clone());
    }
}
fn issue_row(t: &mut Table, report: &Value, run: Option<&Value>, i: &Value, index: usize) {
    let mut r = prefix(report, run);
    r.insert("issue_index".into(), json!(index));
    assign(&mut r, i, &["object_ref", "field_path", "message"]);
    r.insert("affected_conclusion".into(), i["blocks"].clone());
    add(t, r);
}
pub(crate) fn tables(report: &Versioned<EvaluationReport>) -> Vec<Table> {
    let contract: Value =
        serde_json::from_str(include_str!("../evaluation-report.contract.design.json"))
            .expect("frozen CSV contract");
    let data = value(&report.data);
    let spec = &contract["csv_projection"];
    let titles = [
        "报告上下文与接受口径",
        "订单接受与完工",
        "资源占用与工作",
        "资源持有原始区间",
        "等待区间",
        "等待原因",
        "有效阶段工作",
        "原始事件",
        "实际接受事件",
        "条件成本与分币种汇总",
        "方案输入变化",
        "精确模块版本",
        "素材许可",
        "缺项与限制",
        "共同条件与指标比较",
    ];
    let mut tables: Vec<Table> = spec["tables"]
        .as_array()
        .unwrap()
        .iter()
        .zip(titles)
        .map(|(s, title)| Table {
            name: s["file"].as_str().unwrap().into(),
            title: title.into(),
            columns: spec["prefix_columns"]
                .as_array()
                .unwrap()
                .iter()
                .chain(s["columns"].as_array().unwrap())
                .map(|v| v.as_str().unwrap().into())
                .collect(),
            rows: vec![],
        })
        .collect();
    // The current column list omitted state/reason for these actual Option
    // fields. Additive columns preserve their null semantics without changing
    // any existing field or manufacturing zero measurements.
    for (index, fields) in [
        (
            2,
            &[
                "available_capacity_ms",
                "open_calendar_occupied_capacity_ms",
                "closed_calendar_held_capacity_ms",
            ][..],
        ),
        (3, &["execution_device_id"][..]),
        (8, &["order_id"][..]),
        (14, &["currency", "value_unit", "metric_object_ref"][..]),
    ] {
        for field in fields {
            for suffix in ["state", "reason"] {
                let name = format!("{field}_{suffix}");
                if !tables[index].columns.contains(&name) {
                    tables[index].columns.push(name);
                }
            }
        }
    }
    for field in ["component", "source_json"] {
        if !tables[9].columns.iter().any(|name| name == field) {
            tables[9].columns.push(field.into());
        }
    }
    let runs = array(&data, "run_evaluations");
    if runs.is_empty() {
        let mut row = prefix(&data, None);
        row.insert("mode".into(), data["mode"].clone());
        row.insert(
            "export_project_revision".into(),
            data["project_revision"].clone(),
        );
        row.insert("input_assumptions_json".into(), data["input_assumptions"].clone());
        row.insert("report_generation_context_json".into(), data["generation_context"].clone());
        state(&mut row, "goal_met", "not_applicable", "设计说明没有实际Run，不形成达标结论");
        add(&mut tables[0], row);
    }
    for run in runs {
        let scopes = array(run, "acceptance_scope");
        let scopes: Vec<Value> = if scopes.is_empty() {
            vec![Value::Null]
        } else {
            scopes.to_vec()
        };
        for scope in &scopes {
            let mut r = prefix(&data, Some(run));
            assign(
                &mut r,
                run,
                &[
                    "evidence_origin",
                    "computation_status",
                    "whole_window_computed",
                ],
            );
            r.insert("mode".into(), data["mode"].clone());
            // This frozen Run carries its own saved goal in its assumptions.
            // Export-level generation context must not replace historical input.
            r.insert("input_assumptions_json".into(), run["input_assumptions"].clone());
            r.insert("report_generation_context_json".into(), data["generation_context"].clone());
            r.insert("goal_met".into(), run["goal_met"].clone());
            if run["goal_met"].is_null() {
                state(&mut r, "goal_met", "unknown", "未声明可判定目标或实际观察窗尚未完整计算");
            }
            r.insert(
                "export_project_revision".into(),
                data["project_revision"].clone(),
            );
            r.insert(
                "time_origin".into(),
                run["time_reference"]["origin_iso8601"].clone(),
            );
            r.insert("timezone".into(), run["time_reference"]["timezone"].clone());
            r.insert("scope_refs".into(), run["scope"].clone());
            for (k, v) in [
                ("product_revision", &scope["product_ref"]),
                ("accepted_unit", &scope["unit"]),
                ("destination", &scope["destination"]),
                ("acceptance_stage", &scope["stage"]),
            ] {
                r.insert(k.into(), v.clone());
            }
            add(&mut tables[0], r);
        }
        for o in array(run, "order_metrics") {
            let mut r = prefix(&data, Some(run));
            assign(
                &mut r,
                o,
                &["order_id", "completion_ms", "due_at_ms", "tardiness_ms"],
            );
            for kind in ["demand", "accepted", "outstanding"] {
                r.insert(
                    format!("{kind}_coefficient"),
                    o[kind]["value"]["coefficient"].clone(),
                );
                r.insert(
                    format!("{kind}_decimal_places"),
                    o[kind]["value"]["decimal_places"].clone(),
                );
            }
            r.insert("unit".into(), o["demand"]["unit"].clone());
            if o["completion_ms"].is_null() {
                state(&mut r, "completion_ms", "unknown", "观察窗内需求未完整接受");
            }
            if o["due_at_ms"].is_null() {
                state(&mut r, "due_at_ms", "not_applicable", "未声明交期");
            }
            if o["tardiness_ms"].is_null() {
                state(
                    &mut r,
                    "tardiness_ms",
                    if o["due_at_ms"].is_null() {
                        "not_applicable"
                    } else {
                        "unknown"
                    },
                    "未完成或未声明交期，不能记迟延0",
                );
            }
            add(&mut tables[1], r);
        }
        for s in array(run, "resource_metrics") {
            let mut r = prefix(&data, Some(run));
            for c in &tables[2].columns {
                if let Some(v) = s.get(c) {
                    r.insert(c.clone(), v.clone());
                }
            }
            for field in [
                "available_capacity_ms",
                "open_calendar_occupied_capacity_ms",
                "closed_calendar_held_capacity_ms",
                "utilization",
                "effective_work_ms",
                "waiting_ms",
            ] {
                if s[field].is_null() {
                    let reason = if field == "utilization" && s["available_capacity_ms"] == 0 {
                        "可用容量分母为0，利用率未知".to_owned()
                    } else {
                        let details: Vec<_> = array(s, "missing_conditions")
                            .iter()
                            .filter_map(|i| i["message"].as_str())
                            .collect();
                        if details.is_empty() {
                            "缺少该资源可归属的实际区间或日历信息".into()
                        } else {
                            details.join("；")
                        }
                    };
                    state(&mut r, field, "unknown", &reason);
                }
            }
            add(&mut tables[2], r);
        }
        let jobs = array(run, "jobs");
        for c in array(run, "resource_claims") {
            for (n, t) in array(c, "intervals").iter().enumerate() {
                let mut r = prefix(&data, Some(run));
                r.insert("claim_id".into(), c["id"].clone());
                assign(&mut r, c, &["resource_id", "job_id", "units", "kind"]);
                r.insert("phase_id".into(), c["phase_ref"].clone());
                r.insert("interval_index".into(), json!(n));
                interval(&mut r, t, run);
                let end = &c["interval_end_causes"][n];
                r.insert("end_cause".into(), end.clone());
                r.insert("end_known".into(), json!(end == "released"));
                if let Some(j) = jobs.iter().find(|j| j["id"] == c["job_id"]) {
                    execution(&mut r, &j["execution_ref"]);
                }
                add(&mut tables[3], r);
            }
        }
        for (n, w) in array(run, "waits").iter().enumerate() {
            let key = format!("{}:{n}", cell(&run["run_id"]));
            let mut r = prefix(&data, Some(run));
            r.insert("wait_key".into(), json!(key));
            r.insert("object_ref".into(), w["object_ref"].clone());
            r.insert("phase_id".into(), w["phase_ref"].clone());
            execution(&mut r, &w["execution_ref"]);
            interval(&mut r, &w["interval"], run);
            r.insert("end_cause".into(), w["end_cause"].clone());
            r.insert(
                "end_known".into(),
                json!(w["end_cause"] == "resolved" || w["end_cause"] == "condition_changed"),
            );
            add(&mut tables[4], r);
            for (j, reason) in array(w, "reasons").iter().enumerate() {
                let mut r = prefix(&data, Some(run));
                r.insert("wait_key".into(), json!(key));
                r.insert("reason_index".into(), json!(j));
                assign(
                    &mut r,
                    reason,
                    &[
                        "kind",
                        "blocking_object",
                        "required",
                        "next_known_boundary_ms",
                    ],
                );
                add(&mut tables[5], r);
            }
        }
        for (n, w) in array(run, "phase_work_intervals").iter().enumerate() {
            let mut r = prefix(&data, Some(run));
            assign(&mut r, w, &["job_id", "phase_id", "end_cause"]);
            r.insert("interval_index".into(), json!(n));
            interval(&mut r, &w["interval"], run);
            r.insert(
                "effective_start_ms".into(),
                w["effective_progress_start_ms"].clone(),
            );
            r.insert(
                "effective_end_ms".into(),
                w["effective_progress_end_ms"].clone(),
            );
            add(&mut tables[6], r);
        }
        for e in array(run, "events") {
            let mut r = prefix(&data, Some(run));
            assign(&mut r, e, &["sequence", "at_ms"]);
            r.insert("subject_ref".into(), e["subject"].clone());
            r.insert("payload_kind".into(), e["payload"]["kind"].clone());
            r.insert("payload_json".into(), e["payload"].clone());
            add(&mut tables[7], r);
        }
        for (n, e) in array(run, "acceptance_events").iter().enumerate() {
            let mut r = prefix(&data, Some(run));
            r.insert("acceptance_index".into(), json!(n));
            assign(
                &mut r,
                e,
                &["order_id", "destination", "acceptance_stage", "at_ms"],
            );
            r.insert("item_refs_json".into(), e["item_refs"].clone());
            r.insert(
                "quantity_coefficient".into(),
                e["quantity"]["value"]["coefficient"].clone(),
            );
            r.insert(
                "quantity_decimal_places".into(),
                e["quantity"]["value"]["decimal_places"].clone(),
            );
            r.insert("unit".into(), e["quantity"]["unit"].clone());
            add(&mut tables[8], r);
        }
        for line in array(&run["costs"], "lines") {
            let mut r = prefix(&data, Some(run));
            let source = array(&run["costs"], "inputs")
                .iter()
                .find(|v| v["id"] == line["cost_input_id"]);
            r.insert("cost_line_id".into(), line["cost_input_id"].clone());
            r.insert("row_kind".into(), json!("cost_line"));
            assign(&mut r, line, &["object_ref", "basis", "currency", "amount"]);
            r.insert(
                "measured_quantity".into(),
                line["measured_quantity"].clone(),
            );
            r.insert(
                "measure_unit".into(),
                line["measured_quantity"]["unit"].clone(),
            );
            r.insert(
                "missing_conditions_json".into(),
                line["missing_conditions"].clone(),
            );
            if let Some(s) = source {
                r.insert("component".into(), s["component"].clone());
                r.insert("source_json".into(), s["source"].clone());
                r.insert("rate".into(), s["rate"].clone());
                r.insert("rate_unit".into(), s["rate_quantity"].clone());
                r.insert("includes_json".into(), s["includes"].clone());
            }
            add(&mut tables[9], r);
        }
        for c in array(&run["costs"], "currencies") {
            for field in ["known_subtotal", "complete_total", "unit_cost"] {
                let mut r = prefix(&data, Some(run));
                r.insert("row_kind".into(), json!(field));
                r.insert("currency".into(), c["currency"].clone());
                r.insert("amount".into(), c[field].clone());
                r.insert(
                    "accepted_unit".into(),
                    c["accepted_quantity"]["unit"].clone(),
                );
                r.insert(
                    "missing_conditions_json".into(),
                    c["missing_conditions"].clone(),
                );
                add(&mut tables[9], r);
            }
        }
        for m in array(run, "module_versions") {
            let mut r = prefix(&data, Some(run));
            r.insert("module_id".into(), m["id"].clone());
            r.insert("module_version".into(), m["version"].clone());
            r.insert("source_ref".into(), json!("frozen Run module_versions"));
            add(&mut tables[11], r);
        }
        for (n, i) in array(run, "limitations")
            .iter()
            .chain(array(&run["costs"], "missing_conditions"))
            .enumerate()
        {
            issue_row(&mut tables[13], &data, Some(run), i, n);
        }
    }
    for a in array(&data, "asset_rights") {
        let mut r = prefix(&data, None);
        r.insert("asset_id".into(), a["asset_ref"].clone());
        assign(
            &mut r,
            a,
            &[
                "source",
                "license",
                "redistribution_confirmed",
                "missing_reason",
            ],
        );
        r.insert(
            "actual_export_authorized".into(),
            json!(
                a["redistribution_confirmed"] == true
                    && a["license"].as_str().is_some_and(|s| !s.is_empty())
                    && a["source"].as_str().is_some_and(|s| !s.is_empty())
            ),
        );
        add(&mut tables[12], r);
    }
    for (n, i) in array(&data, "missing_conditions")
        .iter()
        .chain(array(&data, "limitations"))
        .enumerate()
    {
        issue_row(&mut tables[13], &data, None, i, n);
    }
    let comp = &data["scenario_comparison"];
    for pair in array(comp, "pairs") {
        for (n, c) in array(pair, "input_changes").iter().enumerate() {
            let mut r = prefix(&data, None);
            r.insert("change_index".into(), json!(n));
            r.insert("object_ref".into(), c["target"].clone());
            assign(&mut r, c, &["path", "kind"]);
            r.insert("before_typed_json".into(), c["before"].clone());
            r.insert("after_typed_json".into(), c["after"].clone());
            r.insert("source_basis".into(), c["basis"].clone());
            r.insert("change_reason".into(), c["reason"].clone());
            for field in ["before", "after"] {
                if c[field].is_null() {
                    state(
                        &mut r,
                        &format!("{field}_typed_json"),
                        "not_applicable",
                        "新增/移除一侧不存在，区别于存在的null类型值",
                    );
                }
            }
            add(&mut tables[10], r);
        }
        for m in array(comp, "metrics") {
            let Some(d) = array(m, "pair_differences").iter().find(|d| {
                d["baseline_run_id"] == pair["baseline_run_id"]
                    && d["candidate_run_id"] == pair["candidate_run_id"]
            }) else {
                continue;
            };
            let mut r = prefix(&data, None);
            r.insert("metric_key".into(), m["metric"].clone());
            r.insert("metric_object_ref".into(), m["object_ref"].clone());
            r.insert("currency".into(), m["currency"].clone());
            r.insert("comparison_status".into(), pair["status"].clone());
            assign(
                &mut r,
                d,
                &[
                    "baseline_run_id",
                    "candidate_run_id",
                    "comparable",
                    "change_percent",
                ],
            );
            r.insert("delta_typed_json".into(), d["difference"].clone());
            r.insert("condition_issues_json".into(), d["conditions"].clone());
            for side in ["baseline", "candidate"] {
                let id = &d[format!("{side}_run_id")];
                if let Some(run) = runs.iter().find(|v| &v["run_id"] == id) {
                    r.insert(format!("{side}_input_ref_json"), run["input_ref"].clone());
                    r.insert(
                        format!("{side}_window_json"),
                        run["requested_window"].clone(),
                    );
                    r.insert(
                        format!("{side}_acceptance_scope_json"),
                        run["acceptance_scope"].clone(),
                    );
                }
                if let Some(v) = array(m, "values").iter().find(|v| &v["run_id"] == id) {
                    r.insert(format!("{side}_typed_json"), v["value"].clone());
                }
            }
            if d["difference"].is_null() {
                state(
                    &mut r,
                    "delta_typed_json",
                    "unknown",
                    "共同条件不满足或实际数值未知，不能补0",
                );
            }
            if d["change_percent"].is_null() {
                state(
                    &mut r,
                    "change_percent",
                    "unknown",
                    "不可比较、基数为0或实际数值未知",
                );
            }
            r.insert("direction".into(), json!("candidate_minus_baseline"));
            let unit = match m["metric"].as_str() {
                Some("completion_time" | "waiting_time") => json!("ms"),
                Some("resource_utilization") => json!("ratio"),
                _ => {
                    let units: Vec<_> = array(m, "values")
                        .iter()
                        .filter_map(|v| v["value"]["data"]["unit"].as_str())
                        .collect();
                    if let Some(unit) = units.first() {
                        if units.iter().all(|v| v == unit) {
                            json!(unit)
                        } else {
                            Value::Null
                        }
                    } else {
                        m["acceptance_scope"]["unit"].clone()
                    }
                }
            };
            r.insert("value_unit".into(), unit);
            add(&mut tables[14], r);
        }
    }
    tables
}
