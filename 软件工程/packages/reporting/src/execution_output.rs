//! Four-format byte production from one canonical ExecutionReport. Host owns
//! authorization, real output directories, Task cancellation and atomic commit.
use crate::comparison::{issue, value};
use crate::evaluation_output::{
    EvaluationFile, EvaluationOutputFormat, EvaluationResources, cancelled, escape,
};
use crate::execution_evaluation::check_export_content;
use printpdf::*;
use serde_json::{Value, json};
use sf_domain::{Issue, cloud_execution::ExecutionReport};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};

struct Table {
    name: String,
    columns: Vec<String>,
    rows: Vec<Value>,
}
fn problem(text: impl Into<String>) -> Vec<Issue> {
    vec![issue("execution_report_format", text, None)]
}
fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}
fn field(v: &Value, p: &str) -> Value {
    v.pointer(p).cloned().unwrap_or(Value::Null)
}
fn prefix(report: &Value, session: Option<&Value>) -> Value {
    let mut v = json!({"report_id":report["report_id"],"project_id":report["project_id"]});
    if let Some(s) = session {
        let src = &s["source"];
        for (k, x) in [
            ("session_id", src["identity"]["session_id"].clone()),
            (
                "execution_epoch_id",
                src["identity"]["execution_epoch_id"].clone(),
            ),
            ("execution_revision", src["execution_revision"].clone()),
            ("observation_frontier", src["observation_frontier"].clone()),
        ] {
            v[k] = x;
        }
    }
    v
}
fn with_prefix(report: &Value, s: Option<&Value>, row: Value) -> Value {
    let mut v = prefix(report, s);
    v.as_object_mut()
        .unwrap()
        .extend(row.as_object().unwrap().clone());
    v
}
fn arrays<'a>(v: &'a Value, p: &str) -> &'a [Value] {
    v.pointer(p)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn collect_issues(v: &Value, path: &str, context: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m)
            if m.contains_key("code") && m.contains_key("message") && m.contains_key("blocks") =>
        {
            let mut row = context.clone();
            for (k, x) in [
                ("source_path", json!(path)),
                ("code", v["code"].clone()),
                ("object_ref", v["object_ref"].clone()),
                ("field_path", v["field_path"].clone()),
                ("message", v["message"].clone()),
                ("affected_conclusions", v["blocks"].clone()),
            ] {
                row[k] = x;
            }
            out.push(row);
        }
        Value::Object(m) => {
            for (k, x) in m {
                collect_issues(
                    x,
                    &format!("{path}/{}", k.replace('~', "~0").replace('/', "~1")),
                    context,
                    out,
                );
            }
        }
        Value::Array(a) => {
            for (n, x) in a.iter().enumerate() {
                collect_issues(x, &format!("{path}/{n}"), context, out);
            }
        }
        _ => {}
    }
}
fn tables(report: &Value) -> Vec<Table> {
    let contract: Value =
        serde_json::from_str(include_str!("../execution-report.contract.design.json"))
            .expect("frozen CSV contract");
    let mut out = vec![];
    for spec in contract["tables"].as_array().unwrap() {
        let name = spec["id"].as_str().unwrap();
        let session_table = contract["session_row_prefix_applies_to"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == name);
        let pre = if session_table {
            &contract["session_row_prefix"]
        } else {
            &contract["common_report_prefix"]
        };
        let mut columns: Vec<String> = pre
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().into())
            .collect();
        for c in spec["columns"].as_array().unwrap() {
            let c = c.as_str().unwrap();
            if !columns.iter().any(|x| x == c) {
                columns.push(c.into());
            }
        }
        let mut rows = vec![];
        match name {
            "report_context" => rows.push(report.clone()),
            "comparison" => {
                let c = &report["comparison"];
                if !c.is_null() {
                    let mut base = prefix(report, None);
                    for (role, path) in [
                        ("baseline", "/sessions/0/source"),
                        ("candidate", "/sessions/1/source"),
                    ] {
                        let s = field(report, path);
                        for (k, v) in [
                            ("session_id", s["identity"]["session_id"].clone()),
                            (
                                "execution_epoch_id",
                                s["identity"]["execution_epoch_id"].clone(),
                            ),
                            ("execution_revision", s["execution_revision"].clone()),
                            ("observation_frontier", s["observation_frontier"].clone()),
                        ] {
                            base[format!("{role}_{k}")] = v;
                        }
                    }
                    for x in arrays(c, "/conditions") {
                        let mut row = base.clone();
                        comparison_columns(&mut row, c, x);
                        rows.push(row);
                    }
                }
            }
            "prediction_sources" => {
                let p = &report["prediction_comparison"]["prediction"];
                if !p.is_null() {
                    let mut row = with_prefix(report, None, p.clone());
                    row["run_id"] = p["source"]["run_id"].clone();
                    for k in ["project_revision", "scenario_id", "scenario_revision"] {
                        row[if k == "project_revision" {
                            "input_project_revision"
                        } else {
                            k
                        }] = p["source"]["input_ref"][k].clone();
                    }
                    row["time_reference_json"] = p["time_reference"].clone();
                    row["window_start_ms"] = p["window"]["start_ms"].clone();
                    row["window_end_ms"] = p["window"]["end_ms"].clone();
                    row["target_quantity_json"] = p["target_quantity"].clone();
                    row["module_versions_json"] = p["module_versions"].clone();
                    rows.push(row);
                }
            }
            "prediction_accepted_by_scope" => {
                let p = &report["prediction_comparison"]["prediction"];
                for x in arrays(p, "/accepted") {
                    let mut row = accepted_columns(x);
                    row["run_id"] = p["source"]["run_id"].clone();
                    rows.push(with_prefix(report, None, row));
                }
            }
            "prediction_waiting" => {
                let p = &report["prediction_comparison"]["prediction"];
                for x in arrays(p, "/waiting") {
                    let mut row = x.clone();
                    row["run_id"] = p["source"]["run_id"].clone();
                    row["object_kind"] = x["object_ref"]["kind"].clone();
                    row["object_id"] = x["object_ref"]["id"].clone();
                    row["source_intervals_json"] = x["source_intervals"].clone();
                    rows.push(with_prefix(report, None, row));
                }
            }
            "prediction_comparison" => {
                let p = &report["prediction_comparison"];
                if !p.is_null() {
                    for c in arrays(p, "/conditions") {
                        let mut row = prefix(report, None);
                        row["prediction_run_id"] = p["prediction"]["source"]["run_id"].clone();
                        row["execution_read_origin"] = p["execution"]["read_origin"].clone();
                        row["execution_session_id"] =
                            p["execution"]["identity"]["session_id"].clone();
                        row["execution_epoch_id"] =
                            p["execution"]["identity"]["execution_epoch_id"].clone();
                        row["execution_revision"] = p["execution"]["execution_revision"].clone();
                        row["comparison_scope_json"] = p["comparison_scope"].clone();
                        comparison_columns(&mut row, p, c);
                        rows.push(row);
                    }
                }
            }
            "issues" => {
                for key in ["limitations", "comparison", "prediction_comparison"] {
                    collect_issues(
                        &report[key],
                        &format!("/{key}"),
                        &prefix(report, None),
                        &mut rows,
                    );
                }
                for (n, s) in arrays(report, "/sessions").iter().enumerate() {
                    collect_issues(
                        s,
                        &format!("/sessions/{n}"),
                        &prefix(report, Some(s)),
                        &mut rows,
                    );
                }
            }
            _ => {
                for s in arrays(report, "/sessions") {
                    let x = &s["snapshot"];
                    let mut local = vec![];
                    match name {
                        "execution_sources" => {
                            let mut row = s["source"].clone();
                            for k in ["host_context_session_id"] {
                                row[k] = s["source"]["identity"][k].clone();
                            }
                            for (k, path) in [
                                (
                                    "input_project_revision",
                                    "/project_input_ref/project_revision",
                                ),
                                ("scenario_id", "/project_input_ref/scenario_id"),
                                ("scenario_revision", "/project_input_ref/scenario_revision"),
                                ("source_mode", "/provenance/mode"),
                                ("clock_started", "/coverage/clock_started"),
                                ("observed_until_host_ms", "/coverage/observed_until_host_ms"),
                                (
                                    "requested_cutoff_reached",
                                    "/coverage/requested_cutoff_reached",
                                ),
                                ("lifecycle", "/lifecycle/state"),
                            ] {
                                row[k] = field(x, path);
                            }
                            row["goal_at_snapshot"] = x["goal_at_snapshot"].clone();
                            row["goal_at_cutoff"] = x["goal_at_cutoff"].clone();
                            row["cloud_published_revision"] =
                                s["source"]["delivery"]["cloud_published_revision"].clone();
                            local.push(row);
                        }
                        "accepted_by_scope" => {
                            for a in arrays(s, "/metrics/accepted") {
                                local.push(accepted_columns(a));
                            }
                        }
                        "command_counts" => local.push(s["metrics"]["commands"].clone()),
                        "waiting" => local.extend_from_slice(arrays(s, "/metrics/waiting")),
                        "work_timings" => local.extend_from_slice(arrays(x, "/timings")),
                        "receipt_admissions" => {
                            for a in arrays(x, "/receipt_admissions") {
                                let mut row = a.clone();
                                row["admission_execution_revision"] =
                                    a["execution_revision"].clone();
                                row.as_object_mut().unwrap().remove("execution_revision");
                                local.push(row);
                            }
                        }
                        "logical_work_and_attempts" => {
                            for (kind, path) in
                                [("logical_work", "/logical_work"), ("attempt", "/attempts")]
                            {
                                for a in arrays(x, path) {
                                    let r = if kind == "attempt" {
                                        &a["attempt"]
                                    } else {
                                        &a["current_attempt"]
                                    };
                                    let d = if kind == "attempt" {
                                        &a["dispatch_decision"]
                                    } else {
                                        &a["latest_dispatch_decision"]
                                    };
                                    let mut row = json!({"kind":kind,"logical_job_id":if kind=="attempt"{r["logical_job_id"].clone()}else{a["logical_job_id"].clone()},"command_id":r["command_id"],"phase_id":r["phase_id"],"attempt_ordinal":r["attempt_ordinal"],"selected_binding_id":d["selected_binding_id"],"target_device_boot_id":if kind=="attempt"{a["target_device_boot_id"].clone()}else{Value::Null},"state":a["state"],"decision_host_ms":d["decision_host_ms"],"decision_reason":d["reason"],"candidate_details_json":d["candidates"]});
                                    if row["selected_binding_id"].is_null() {
                                        row["selected_binding_id"] =
                                            r["selected_binding_id"].clone();
                                    }
                                    local.push(row);
                                }
                            }
                        }
                        "resource_claims" => {
                            for p in arrays(x, "/business_facts/resource_claims") {
                                let c = &p["claim"];
                                for (n, i) in arrays(c, "/intervals").iter().enumerate() {
                                    let open = p["open_interval_index"].as_u64() == Some(n as u64);
                                    local.push(json!({"resource_id":c["resource_id"],"job_id":c["job_id"],"units":c["units"],"interval_index":n,"interval_start_ms":i["start_ms"],"interval_end_ms":i["end_ms"],"is_open":open,"view_boundary":if open{p["view_boundary"].clone()}else{json!("actual_release")}}));
                                }
                            }
                        }
                        "state_history" => {
                            for r in arrays(s, "/history/records") {
                                let o = &r["object"];
                                let d = &o["data"];
                                let key = match o["kind"].as_str().unwrap() {
                                    "job" => "job_id",
                                    "logical_work" => "logical_job_id",
                                    "attempt" => "command_id",
                                    _ => "id",
                                };
                                let id = match o["kind"].as_str().unwrap() {
                                    "device" => d["binding"]["device_id"].clone(),
                                    "attempt" => d["attempt"][key].clone(),
                                    _ => d[key].clone(),
                                };
                                local.push(json!({"record_sequence":r["record_sequence"],"record_execution_revision":r["execution_revision"],"at_host_ms":r["at_host_ms"],"object_kind":o["kind"],"object_id":id,"typed_state_json":d}));
                            }
                        }
                        "module_versions_and_rights" => {
                            for m in arrays(s, "/module_versions") {
                                local.push(json!({"row_kind":"module_version","module_id":m["id"],"module_version":m["version"]}));
                            }
                            for a in arrays(s, "/asset_rights") {
                                let mut row = a["rights"].clone();
                                row["row_kind"] = json!("asset_rights");
                                row["module_id"] = a["module_ref"]["id"].clone();
                                row["module_version"] = a["module_ref"]["version"].clone();
                                local.push(row);
                            }
                        }
                        _ => unreachable!("all frozen CSV tables mapped"),
                    }
                    rows.extend(local.into_iter().map(|r| with_prefix(report, Some(s), r)));
                }
            }
        }
        out.push(Table {
            name: name.into(),
            columns,
            rows,
        });
    }
    out
}
fn accepted_columns(a: &Value) -> Value {
    json!({"product_ref":a["scope"]["product_ref"],"unit":a["scope"]["unit"],"destination":a["scope"]["destination"],"acceptance_stage":a["scope"]["stage"],"known":!a["accepted_quantity"].is_null(),"coefficient":a["accepted_quantity"]["value"]["coefficient"],"decimal_places":a["accepted_quantity"]["value"]["decimal_places"],"missing_conditions":a["missing_conditions"]})
}
fn comparison_columns(row: &mut Value, p: &Value, c: &Value) {
    for (k, v) in [
        ("status", p["status"].clone()),
        ("condition_key", c["key"].clone()),
        ("matches", c["matches"].clone()),
        ("detail", c["detail"].clone()),
        ("completion_delta_ms", p["completion_delta_ms"].clone()),
        (
            "accepted_delta_direction",
            p["accepted_delta"]["direction"].clone(),
        ),
        (
            "accepted_delta_coefficient",
            p["accepted_delta"]["magnitude"]["value"]["coefficient"].clone(),
        ),
        (
            "accepted_delta_decimal_places",
            p["accepted_delta"]["magnitude"]["value"]["decimal_places"].clone(),
        ),
        (
            "accepted_delta_unit",
            p["accepted_delta"]["magnitude"]["unit"].clone(),
        ),
    ] {
        row[k] = v;
    }
}
fn quantity(v: &Value) -> String {
    if v.is_null() {
        return "未知".into();
    }
    let q = &v["value"];
    let n = cell(&q["coefficient"]);
    let d = q["decimal_places"].as_u64().unwrap() as usize;
    let exact = if d == 0 {
        n
    } else {
        let pad = format!("{:0>width$}", n, width = d + 1);
        format!("{}.{}", &pad[..pad.len() - d], &pad[pad.len() - d..])
    };
    let unit = if v["unit"] == "piece" {
        "件".into()
    } else {
        cell(&v["unit"])
    };
    format!("{exact} {unit}")
}
fn known(v: &Value, unit: &str) -> String {
    if v.is_null() {
        "未知".into()
    } else {
        format!("{} {unit}", cell(v))
    }
}
fn business_message(message: &str) -> &str {
    match message {
        "Host execution clock has not started; acceptance coverage is unobserved" => "主机执行时钟尚未启动，验收覆盖尚未观察。",
        "Host clock has not started; waiting is not observed zero" => "主机时钟尚未启动；等待尚未观察，不能按零计。",
        "Host clock has not started" => "主机时钟尚未启动。",
        "Unique scope, positive fixed target, started Host clock and actual target-reaching prefix are required" => "首次达标时间需要唯一验收范围、正值固定目标、已启动的主机时钟和实际达标记录。",
        "No physical-device effective-work evidence; nominal, Host-confirmed and device-reported time remain separate" => {
            "缺少物理设备有效工时证据；名义加工、主机确认占用和设备自报时长分别保留。"
        }
        "Host-confirmed resource occupation is not measured physical effective work" => {
            "主机确认的资源占用不代表物理设备有效工时。"
        }
        "Execution Host time minus model virtual time is prediction deviation, not AI speedup, cost improvement or physical effective work" => {
            "主机时间与模型虚拟时间的差只表示预测偏差，不能据此认定智能加速、成本改善或物理有效工时。"
        }
        "Actual model wait intervals and Host logical waiting are source-qualified, not directly subtracted" => {
            "模型等待区间与主机逻辑作业等待分别标明来源，不能直接相减。"
        }
        "A null deviation retains missing common/goal/coverage conditions, unknown original metrics or unrepresentable exact arithmetic; no zero/float substitute is used" => {
            "差值为空时，保留共同条件、目标、覆盖范围、原指标未知或精确运算限制；不补零，也不用浮点近似。"
        }
        "软件在环或物理设备来源以每个快照provenance为准；不将Host确认占用当物理有效工时" => {
            "本报告按各执行来源区分软件在环与物理设备证据；主机确认占用不代表物理有效工时。"
        }
        "接受量来自冻结范围内的实际物料验收；设备完成、逻辑工作及接受量分别报告。" => {
            "接受量来自冻结范围内的实际物料验收；设备完成次数、逻辑作业数与接受量分别报告。"
        }
        "Host等待、资源占用与预测模型等待分别标源；任何差值均为观察比较，不表示因果改善。" => {
            "主机等待、资源占用与模型等待分别标明来源；观察差值不表示因果改善。"
        }
        _ => "未翻译的原始说明，见技术详情和完整附件中的对应原文。",
    }
}
fn reasons(v: &Value) -> String {
    arrays(v, "")
        .iter()
        .map(|x| business_message(x["message"].as_str().unwrap()).to_owned())
        .collect::<Vec<_>>()
        .join("；")
}
fn job_ids(v: &Value) -> Vec<String> {
    let mut ids = vec![];
    for s in arrays(v, "/sessions") {
        for (path, key) in [
            ("/snapshot/logical_work", "logical_job_id"),
            ("/metrics/waiting", "logical_job_id"),
            ("/snapshot/timings", "job_id"),
        ] {
            for row in arrays(s, path) {
                let id = cell(&row[key]);
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    for row in arrays(v, "/prediction_comparison/prediction/waiting") {
        if row["object_ref"]["kind"] == "job" {
            let id = cell(&row["object_ref"]["id"]);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}
fn job_label(ids: &[String], id: &Value) -> String {
    format!(
        "作业{}",
        ids.iter().position(|x| x == &cell(id)).unwrap() + 1
    )
}
fn table_title(name: &str) -> &str {
    match name {
        "report_context" => "报告背景",
        "execution_sources" => "执行来源",
        "accepted_by_scope" => "各范围物料接受量",
        "command_counts" => "命令与逻辑作业计数",
        "logical_work_and_attempts" => "逻辑作业与执行尝试",
        "receipt_admissions" => "设备回执接纳记录",
        "resource_claims" => "资源预留与占用",
        "work_timings" => "各作业时间事实",
        "waiting" => "执行作业等待",
        "state_history" => "完整状态历史",
        "module_versions_and_rights" => "模块版本与资产权利",
        "issues" => "缺失条件与限制原文",
        "comparison" => "两次执行比较条件",
        "prediction_sources" => "预测来源与窗口",
        "prediction_accepted_by_scope" => "预测各范围接受量",
        "prediction_comparison" => "预测与执行比较条件",
        "prediction_waiting" => "模型对象等待",
        _ => name,
    }
}
fn condition_label(key: &str) -> &str {
    match key {
        "project_scope" => "工程与来源身份",
        "frozen_input_content" => "冻结输入内容",
        "model_bindings" => "模型设备与定义绑定",
        "fixed_target" => "固定目标及单位",
        "acceptance_scope" => "产品验收范围",
        "clock_basis" => "时钟基准",
        "observation_window" => "观察窗口",
        "prediction_goal" => "预测首次达标证据",
        "execution_goal_and_settlement" => "执行目标与义务结清",
        "availability_record" => "设备可用性与数据质量记录",
        "waiting_basis" => "等待计量基准",
        _ => "其他比较条件（原文见详情）",
    }
}
fn condition_explanation(key: &str, matches: bool) -> &str {
    match (key, matches) {
        ("fixed_target", true) => "双方具有相同的正值固定目标及单位。",
        ("clock_basis", false) => "要求模型从零计时、主机时钟已启动且时间参照一致；当前未满足。",
        ("prediction_goal", true) => "原预测的实际接受记录已证明固定目标的首次达标时间。",
        ("execution_goal_and_settlement", false) => "执行目标或义务结清条件尚未满足。",
        ("project_scope", true) => "原工程、预测运行和执行来源身份已核对。",
        ("frozen_input_content", true) => {
            "实际类型化输入一致；修订、标题和视图元数据不参与内容比较。"
        }
        ("model_bindings", true) => "冻结的模型设备与精确版本定义已保留。",
        ("fixed_target", false) => "要求双方具有相同正值固定目标及单位；当前未满足。",
        ("acceptance_scope", true) => "产品修订、单位、目的地与验收阶段唯一匹配。",
        ("clock_basis", true) => "模型从零计时，主机执行已开始，时间参照相同。",
        ("observation_window", false) => "要求预测窗口完整计算且双方观察终点一致；当前未满足。",
        ("prediction_goal", false) => "原预测的实际接受记录未能证明固定目标的首次达标时间。",
        ("execution_goal_and_settlement", true) => "执行目标已达到，相关义务已结清。",
        ("availability_record", true) => "固定观察前沿的设备历史及数据质量条件已保留。",
        ("waiting_basis", false) => {
            "模型按对象等待区间并集计算，主机按逻辑作业状态计时，不能直接相减。"
        }
        _ => "原始判定与具体说明见技术详情及完整附件。",
    }
}
fn reading(v: &Value) -> Vec<(String, Vec<String>)> {
    let jobs = job_ids(v);
    let mut out = vec![("报告信息".into(), vec![
        format!("生成时间（UTC）：{}；导出工程修订：{}。", cell(&v["generated_at_utc"]), cell(&v["export_project_revision"])),
        "作业编号按原数据稳定顺序分配，不代表成品数；精确来源身份、编号对应及所有原始条件见HTML技术详情和PDF完整数据附件。".into(),
    ])];
    for (source_no, s) in arrays(v, "/sessions").iter().enumerate() {
        let x = &s["snapshot"];
        let m = &s["metrics"];
        let origin = match s["source"]["read_origin"].as_str().unwrap() {
            "cloud" => "云端读取",
            "host" => "本地 Host 读取",
            _ => "读取来源见技术详情",
        };
        let mode = match x["provenance"]["mode"].as_str().unwrap() {
            "software_in_loop" => "软件在环",
            "physical_device" => "物理设备",
            _ => "执行模式见技术详情",
        };
        let state = match x["lifecycle"]["state"].as_str().unwrap() {
            "prepared" => "已准备",
            "running" => "执行中",
            "dispatch_paused" => "暂停派发",
            "draining" => "等待在制任务结束",
            "needs_resolution" => "待处理",
            "completed" => "已完成",
            "stopped" => "已停止",
            "failed" => "失败",
            _ => "状态原文见技术详情",
        };
        let mut lines = vec![
            format!("工程：{}", cell(&s["project_title"])),
            format!("方案：{}", cell(&s["scenario_title"])),
            format!(
                "来源{}：{}，{}；执行修订{}，观察前沿{}。",
                source_no + 1,
                origin,
                mode,
                cell(&s["source"]["execution_revision"]),
                cell(&s["source"]["observation_frontier"])
            ),
            format!(
                "执行状态：{}；主机观察终点：{} ms；固定目标：{}；首次达标：{}。",
                state,
                cell(&x["coverage"]["observed_until_host_ms"]),
                quantity(&m["target_quantity"]),
                known(&m["target_reached_at_host_ms"], "主机ms")
            ),
        ];
        for (n, a) in arrays(m, "/accepted").iter().enumerate() {
            lines.push(format!(
                "验收范围{}：实际接受 {}。{}",
                n + 1,
                quantity(&a["accepted_quantity"]),
                reasons(&a["missing_conditions"])
            ));
        }
        let c = &m["commands"];
        lines.push(format!(
            "执行尝试{}次；已开始{}次；设备完成{}次；已接受逻辑作业{}项；未决命令{}条。",
            cell(&c["attempt_count"]),
            cell(&c["started_attempt_count"]),
            cell(&c["device_completed_attempt_count"]),
            cell(&c["accepted_logical_work_count"]),
            cell(&c["unresolved_command_count"])
        ));
        let missing = reasons(&m["missing_conditions"]);
        if !missing.is_empty() {
            lines.push(format!("本来源缺失条件：{missing}"));
        }
        out.push((format!("执行观察 · {}", cell(&s["project_title"])), lines));
        let mut waits =
            vec!["以下均为主机记录的状态等待时间，分别列示排队、开工前、输出与核对等待。".into()];
        for w in arrays(m, "/waiting") {
            waits.push(format!(
                "{}：排队{}；开工前{}；输出等待{}；核对等待{}。{}",
                job_label(&jobs, &w["logical_job_id"]),
                known(&w["queued_wait_ms"], "ms"),
                known(&w["pre_start_wait_ms"], "ms"),
                known(&w["output_wait_ms"], "ms"),
                known(&w["reconciliation_wait_ms"], "ms"),
                reasons(&w["missing_conditions"])
            ));
        }
        out.push((format!("来源{} · 作业等待", source_no + 1), waits));
        let mut timings = vec![
            "名义加工、设备自报和主机确认是不同时间事实。物理有效工时及逐项缺失原因按原记录保留。"
                .into(),
        ];
        for t in arrays(x, "/timings") {
            timings.push(format!(
                "{}：主机确认预留{}；确认占用{}；名义加工{}；设备自报{}；物理有效工时{}。",
                job_label(&jobs, &t["job_id"]),
                known(&t["host_confirmed_reservation_ms"], "ms"),
                known(&t["host_confirmed_occupation_ms"], "ms"),
                known(&t["nominal_work_ms"], "ms"),
                known(&t["device_reported_duration_ms"], "ms"),
                known(&t["physical_effective_work_ms"], "ms")
            ));
            for issue in arrays(t, "/missing_conditions") {
                if issue["message"]
                    != "No physical-device effective-work evidence; nominal, Host-confirmed and device-reported time remain separate"
                {
                    timings.push(format!(
                        "{}补充说明：{}",
                        job_label(&jobs, &t["job_id"]),
                        business_message(issue["message"].as_str().unwrap())
                    ));
                }
            }
        }
        for (n, r) in arrays(m, "/resources").iter().enumerate() {
            timings.push(format!(
                "资源{}：主机确认占用并集{}；占用量{}；物理有效工时{}。{}",
                n + 1,
                known(&r["confirmed_occupation_ms"], "ms"),
                known(&r["claimed_unit_ms"], "单位·ms"),
                known(&r["physical_effective_work_ms"], "ms"),
                reasons(&r["missing_conditions"])
            ));
        }
        if arrays(x,"/timings").iter().any(|t| arrays(t,"/missing_conditions").iter().any(|i| i["message"] == "No physical-device effective-work evidence; nominal, Host-confirmed and device-reported time remain separate")) {
            timings.push("各相关作业均缺少物理设备有效工时证据；完整附件和技术详情保留每条原始说明与命令身份。".into());
        }
        out.push((
            format!("来源{} · 加工时间与资源占用", source_no + 1),
            timings,
        ));
    }
    for (key, title) in [
        ("comparison", "两次执行的观察比较"),
        ("prediction_comparison", "预测与执行比较"),
    ] {
        let c = &v[key];
        if c.is_null() {
            continue;
        }
        let status = match c["status"].as_str().unwrap() {
            "side_by_side_only" => "仅并列展示",
            "observational" => "观察比较",
            _ => "状态原文见技术详情",
        };
        let time = if c["completion_delta_ms"].is_null() {
            "暂无可比结果".into()
        } else {
            known(&c["completion_delta_ms"], "ms")
        };
        let accepted = if c["accepted_delta"].is_null() {
            "暂无可比结果".into()
        } else {
            let direction = match c["accepted_delta"]["direction"].as_str().unwrap() {
                "increase" => "增加",
                "decrease" => "减少",
                "equal" => "不变",
                _ => "变化方向见详情",
            };
            format!(
                "{} {}",
                direction,
                quantity(&c["accepted_delta"]["magnitude"])
            )
        };
        let mut lines = vec![format!(
            "比较状态：{}；首次达标时间差：{}；接受量变化：{}。",
            status, time, accepted
        )];
        if key == "prediction_comparison" {
            let p = &c["prediction"];
            lines.push(format!(
                "预测采用模型虚拟时钟：窗口{}至{} ms，已计算至{}；固定目标{}；首次达标{}。",
                cell(&p["window"]["start_ms"]),
                cell(&p["window"]["end_ms"]),
                known(&p["computed_until_sim_ms"], "模拟ms"),
                quantity(&p["target_quantity"]),
                known(&p["target_reached_at_sim_ms"], "模拟ms")
            ));
            if p["target_quantity"].is_null() {
                lines.push("原报告的预测固定目标为空，不能用接受记录补推首次达标时间。".into());
            }
            for (n, a) in arrays(p, "/accepted").iter().enumerate() {
                lines.push(format!(
                    "预测验收范围{}：接受 {}。{}",
                    n + 1,
                    quantity(&a["accepted_quantity"]),
                    reasons(&a["missing_conditions"])
                ));
            }
            let missing = reasons(&p["missing_conditions"]);
            if !missing.is_empty() {
                lines.push(format!("预测缺失条件：{missing}"));
            }
        }
        for condition in arrays(c, "/conditions") {
            let key = condition["key"].as_str().unwrap();
            let matches = condition["matches"] == true;
            lines.push(format!(
                "{}：{}。{}",
                condition_label(key),
                if matches { "满足" } else { "未满足" },
                condition_explanation(key, matches)
            ));
        }
        for issue in arrays(c, "/limitations") {
            lines.push(business_message(issue["message"].as_str().unwrap()).into());
        }
        out.push((title.into(), lines));
        if key == "prediction_comparison" {
            let mut lines = vec![
                "以下为模型对象等待区间的并集；与主机逻辑作业等待分别展示，不计算等待改善。".into(),
            ];
            for (n, w) in arrays(c, "/prediction/waiting").iter().enumerate() {
                let label = if w["object_ref"]["kind"] == "job" {
                    job_label(&jobs, &w["object_ref"]["id"])
                } else {
                    format!("模型对象{}（身份见详情）", n + 1)
                };
                lines.push(format!(
                    "{}：等待并集{}；原区间{}条；覆盖{}至{}。{}",
                    label,
                    known(&w["known_sim_ms"], "模拟ms"),
                    arrays(w, "/source_intervals").len(),
                    cell(&w["covered_from_sim_ms"]),
                    known(&w["covered_until_sim_ms"], "模拟ms"),
                    reasons(&w["missing_conditions"])
                ));
            }
            out.push(("模型等待".into(), lines));
        }
    }
    out.push((
        "结论与限制".into(),
        arrays(v, "/conclusions")
            .iter()
            .map(|x| business_message(x.as_str().unwrap()).into())
            .chain(
                arrays(v, "/limitations")
                    .iter()
                    .map(|x| business_message(x["message"].as_str().unwrap()).into()),
            )
            .collect(),
    ));
    out
}
fn technical_details(v: &Value) -> String {
    let mut html = String::from(
        "<details id=technical-details><summary>技术详情：精确来源、作业编号与比较原文</summary><h3>作业编号对应</h3><p>编号按原数据顺序生成，不改原身份；同一作业在执行与预测概览中使用同一编号。</p>",
    );
    for (n, id) in job_ids(v).iter().enumerate() {
        html.push_str(&format!(
            "<p class=job-mapping>作业{}：<code>{}</code></p>",
            n + 1,
            escape(id)
        ));
    }
    for (n, s) in arrays(v, "/sessions").iter().enumerate() {
        html.push_str(&format!(
            "<h3>来源{}：{} / {}</h3>",
            n + 1,
            escape(&cell(&s["project_title"])),
            escape(&cell(&s["scenario_title"]))
        ));
        for (i, r) in arrays(s, "/metrics/resources").iter().enumerate() {
            html.push_str(&format!(
                "<p>资源{}：<code>{}</code></p>",
                i + 1,
                escape(&cell(&r["resource_id"]))
            ));
        }
        for (i, a) in arrays(s, "/metrics/accepted").iter().enumerate() {
            html.push_str(&format!(
                "<p>验收范围{}：<code>{}</code></p>",
                i + 1,
                escape(&cell(&a["scope"]))
            ));
        }
        html.push_str(&format!("<h4>精确来源身份</h4><pre class=source-identity>{}</pre>",escape(&serde_json::to_string_pretty(&s["source"]).unwrap())));
        let details = json!({"provenance":s["snapshot"]["provenance"],"coverage":s["snapshot"]["coverage"],"lifecycle":s["snapshot"]["lifecycle"],"metrics":s["metrics"],"timings":s["snapshot"]["timings"]});
        html.push_str(&format!(
            "<pre>{}</pre>",
            escape(&serde_json::to_string_pretty(&details).unwrap())
        ));
    }
    for (key, title) in [
        ("comparison", "两次执行比较原文"),
        ("prediction_comparison", "预测与执行比较原文"),
    ] {
        if !v[key].is_null() {
            html.push_str(&format!(
                "<h3>{}</h3><pre>{}</pre>",
                title,
                escape(&serde_json::to_string_pretty(&v[key]).unwrap())
            ));
        }
    }
    html.push_str(&format!(
        "<h3>结论与限制原文</h3><pre>{}</pre></details>",
        escape(
            &serde_json::to_string_pretty(
                &json!({"conclusions":v["conclusions"],"limitations":v["limitations"]})
            )
            .unwrap()
        )
    ));
    html
}

struct Pdf {
    doc: PdfDocument,
    font: FontId,
    metrics: ParsedFont,
    pages: Vec<PdfPage>,
    ops: Vec<Op>,
    y: f32,
}
impl Pdf {
    fn new(bytes: &[u8]) -> Result<Self, Vec<Issue>> {
        let metrics = ParsedFont::from_bytes(bytes, 0, &mut vec![])
            .ok_or_else(|| problem("实际中文字体无法解析"))?;
        let mut doc = PdfDocument::new("超级工厂执行观察与预测比较报告");
        let font = doc.add_font(&metrics);
        Ok(Self {
            doc,
            font,
            metrics,
            pages: vec![],
            ops: vec![],
            y: 785.,
        })
    }
    fn text(&mut self, s: &str, x: f32, y: f32, size: f32) {
        self.ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point { x: Pt(x), y: Pt(y) },
            },
            Op::SetFont {
                font: PdfFontHandle::External(self.font.clone()),
                size: Pt(size),
            },
            Op::ShowText {
                items: vec![TextItem::Text(s.into())],
            },
            Op::EndTextSection,
        ]);
    }
    fn page(&mut self) {
        self.text(
            &format!(
                "超级工厂执行报告 | 第 {} 页 | 未知不补零",
                self.pages.len() + 1
            ),
            48.,
            30.,
            9.,
        );
        self.pages.push(PdfPage::new(
            Mm(210.),
            Mm(297.),
            std::mem::take(&mut self.ops),
        ));
        self.y = 785.;
    }
    fn line(&mut self, s: &str, size: f32) {
        // Keep ordinary paragraphs together. Width uses the same embedded font
        // as the writer.
        let mut needed = 1u32;
        let mut used = 0.;
        for c in s.chars() {
            let advance = self
                .metrics
                .lookup_glyph_index(c as u32)
                .and_then(|gid| self.metrics.get_or_decode_glyph(gid))
                .map_or(0., |g| g.horz_advance as f32)
                * size
                / self.metrics.pdf_font_metrics.units_per_em as f32;
            if c == '\n' || used + advance > 496. {
                needed += 1;
                used = 0.;
            }
            if c != '\n' {
                used += advance;
            }
        }
        if needed <= 8 && self.y - needed as f32 * size * 1.5 < 55. {
            self.page();
        }
        let mut line = String::new();
        let mut width = 0.;
        for c in s.chars() {
            let w = self
                .metrics
                .lookup_glyph_index(c as u32)
                .and_then(|gid| self.metrics.get_or_decode_glyph(gid))
                .map_or(0., |g| g.horz_advance as f32)
                * size
                / self.metrics.pdf_font_metrics.units_per_em as f32;
            if c == '\n' || width + w > 496. {
                if self.y < 55. {
                    self.page();
                }
                let y = self.y;
                self.text(&line, 48., y, size);
                self.y -= size * 1.5;
                line.clear();
                width = 0.;
                if c == '\n' {
                    continue;
                }
            }
            line.push(c);
            width += w;
        }
        if !line.is_empty() {
            if self.y < 55. {
                self.page();
            }
            let y = self.y;
            self.text(&line, 48., y, size);
            self.y -= size * 1.5;
        }
    }
    fn section(&mut self, s: &str) {
        if self.y < 105. {
            self.page();
        }
        self.y -= 10.;
        self.line(s, 15.);
        self.y -= 5.;
    }
    fn finish(mut self, canonical: &[u8]) -> Result<Vec<u8>, Vec<Issue>> {
        self.page();
        let options = PdfSaveOptions::default();
        let mut doc = self
            .doc
            .with_pages(self.pages)
            .to_lopdf_document(&options, &mut vec![]);
        doc.version = "1.7".into();
        // printpdf exposes its existing PDF object API; no extra PDF dependency
        // is needed. Streams are indirect and the catalog uses the standard
        // EmbeddedFiles name tree, so this PDF alone retains the entire report.
        let name = |s: &str| DictItem::Name(s.as_bytes().to_vec());
        let text = |s: &str| DictItem::String {
            data: s.as_bytes().to_vec(),
            literal: true,
        };
        let reference = |id: (u32, u16)| DictItem::Ref {
            obj: id.0,
            r#gen: id.1,
        };
        let dictionary = |entries: Vec<(&str, DictItem)>| DictItem::Dict {
            map: entries.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        };
        let embedded = doc.add_object(
            DictItem::Stream {
                stream: ExternalStream {
                    dict: BTreeMap::from([
                        ("Type".into(), name("EmbeddedFile")),
                        ("Subtype".into(), name("application/json")),
                        (
                            "Params".into(),
                            dictionary(vec![("Size", DictItem::Int(canonical.len() as i64))]),
                        ),
                    ]),
                    content: canonical.to_vec(),
                    compress: true,
                },
            }
            .to_lopdf(),
        );
        let filespec = doc.add_object(
            dictionary(vec![
                ("Type", name("Filespec")),
                ("F", text("report.json")),
                ("UF", text("report.json")),
                ("Desc", text("Complete canonical execution report JSON")),
                (
                    "EF",
                    dictionary(vec![("F", reference(embedded)), ("UF", reference(embedded))]),
                ),
            ])
            .to_lopdf(),
        );
        let embedded_files = doc.add_object(
            dictionary(vec![(
                "Names",
                DictItem::Array(vec![text("report.json"), reference(filespec)]),
            )])
            .to_lopdf(),
        );
        let catalog = doc.catalog_mut().expect("printpdf created the catalog");
        catalog.set(
            "Names",
            dictionary(vec![("EmbeddedFiles", reference(embedded_files))]).to_lopdf(),
        );
        catalog.set("PageMode", name("UseAttachments").to_lopdf());
        if options.optimize {
            doc.compress();
        }
        let mut bytes = vec![];
        doc.save_to(&mut bytes)
            .map_err(|e| problem(format!("执行报告PDF保存失败：{e}")))?;
        Ok(bytes)
    }
}
fn file(p: &str, kind: &str, bytes: Vec<u8>) -> EvaluationFile {
    EvaluationFile {
        relative_path: p.into(),
        media_type: kind.into(),
        bytes,
    }
}
pub fn render_execution_report(
    report: &ExecutionReport,
    formats: &[EvaluationOutputFormat],
    resources: &EvaluationResources,
    cancel: &AtomicBool,
) -> Result<Vec<EvaluationFile>, Vec<Issue>> {
    cancelled(cancel)?;
    if formats.is_empty() {
        return Err(problem("至少选择一种执行报告格式"));
    }
    let v = value(report);
    check_export_content(&v)?;
    let tables = tables(&v);
    let sections = reading(&v);
    let canonical = serde_json::to_vec_pretty(report).map_err(|e| problem(e.to_string()))?;
    let mut files = BTreeMap::new();
    let human = formats.iter().any(|f| {
        matches!(
            f,
            EvaluationOutputFormat::Pdf | EvaluationOutputFormat::Html
        )
    });
    if human {
        let font = resources
            .cjk_font
            .as_ref()
            .filter(|f| !f.is_empty())
            .ok_or_else(|| problem("缺少实际中文字体字节"))?;
        let license = resources
            .font_license
            .as_ref()
            .filter(|x| !x.is_empty())
            .ok_or_else(|| problem("缺少字体许可证字节"))?;
        files.insert(
            "fonts/SourceHanSansCN-Regular.otf".into(),
            file(
                "fonts/SourceHanSansCN-Regular.otf",
                "font/otf",
                font.clone(),
            ),
        );
        files.insert(
            "licenses/font.txt".into(),
            file(
                "licenses/font.txt",
                "text/plain;charset=utf-8",
                license.clone(),
            ),
        );
    }
    for f in formats {
        cancelled(cancel)?;
        match f {
            EvaluationOutputFormat::Json => {
                files.insert(
                    "report.json".into(),
                    file("report.json", "application/json", canonical.clone()),
                );
            }
            EvaluationOutputFormat::Csv => {
                for t in &tables {
                    cancelled(cancel)?;
                    let mut w = csv::WriterBuilder::new()
                        .terminator(csv::Terminator::CRLF)
                        .from_writer(vec![]);
                    w.write_record(&t.columns)
                        .map_err(|e| problem(e.to_string()))?;
                    for row in &t.rows {
                        cancelled(cancel)?;
                        w.write_record(t.columns.iter().map(|c| cell(&row[c])))
                            .map_err(|e| problem(e.to_string()))?;
                    }
                    let mut bytes = vec![0xef, 0xbb, 0xbf];
                    bytes.extend(w.into_inner().map_err(|e| problem(e.to_string()))?);
                    let name = format!("csv/{}.csv", t.name);
                    files.insert(name.clone(), file(&name, "text/csv;charset=utf-8", bytes));
                }
            }
            EvaluationOutputFormat::Html => {
                let mut html = String::from(
                    "<!doctype html><html lang=zh-CN><meta charset=utf-8><meta name=viewport content='width=device-width'><title>超级工厂执行报告</title><style>@font-face{font-family:ReportCN;src:url('fonts/SourceHanSansCN-Regular.otf')}body{font:15px/1.65 ReportCN,system-ui;color:#163347;background:#edf3f6;margin:0}main{max-width:1200px;margin:auto;padding:32px}h1{color:#075a90}section,details{padding:20px;background:white;border:1px solid #cad9e3;border-radius:6px;margin:15px 0}p,pre{white-space:pre-wrap;overflow-wrap:anywhere}summary{cursor:pointer;font-weight:bold}.scroll{overflow:auto}table{border-collapse:collapse;font-size:12px}th,td{border:1px solid #cad9e3;padding:7px;vertical-align:top;min-width:100px;max-width:400px;overflow-wrap:anywhere}th{background:#e8f1f6}</style><main><h1>超级工厂 · 执行观察与预测比较</h1>",
                );
                for (title, lines) in &sections {
                    html.push_str(&format!("<section class=business-section><h2>{}</h2>", escape(title)));
                    for line in lines {
                        html.push_str(&format!("<p>{}</p>", escape(line)));
                    }
                    html.push_str("</section>");
                }
                html.push_str(&technical_details(&v));
                html.push_str("<h2>同源完整明细：17 张分表</h2>");
                for t in &tables {
                    cancelled(cancel)?;
                    html.push_str(&format!("<details class=data-table><summary>{} · {} 行 <small>（{}）</small></summary><div class=scroll><table data-table-name=\"{}\"><thead><tr>",escape(table_title(&t.name)),t.rows.len(),escape(&t.name),escape(&t.name)));
                    for c in &t.columns {
                        html.push_str(&format!("<th>{}</th>", escape(c)));
                    }
                    html.push_str("</tr></thead><tbody>");
                    for r in &t.rows {
                        html.push_str("<tr>");
                        for c in &t.columns {
                            html.push_str(&format!(
                                "<td>{}</td>",
                                if r[c].is_null() {
                                    "未知/不适用".into()
                                } else {
                                    escape(&cell(&r[c]))
                                }
                            ));
                        }
                        html.push_str("</tr>");
                    }
                    html.push_str("</tbody></table></div></details>");
                }
                let raw = String::from_utf8(canonical.clone())
                    .unwrap()
                    .replace('<', "\\u003c")
                    .replace('>', "\\u003e")
                    .replace('&', "\\u0026");
                html.push_str(&format!("<details><summary>完整原始JSON（含原历史和所有条件）</summary><pre id=raw></pre></details><script id=report-data type=application/json>{raw}</script><script>document.getElementById('raw').textContent=JSON.stringify(JSON.parse(document.getElementById('report-data').textContent),null,2)</script></main></html>"));
                files.insert(
                    "report.html".into(),
                    file("report.html", "text/html;charset=utf-8", html.into_bytes()),
                );
            }
            EvaluationOutputFormat::Pdf => {
                let mut pdf = Pdf::new(resources.cjk_font.as_ref().unwrap())?;
                let generated = printpdf::DateTime::from_unix_timestamp(
                    report.generated_at_utc.as_datetime().timestamp(),
                )
                .expect("validated report UTC timestamp");
                pdf.doc.metadata.info.creation_date = generated;
                pdf.doc.metadata.info.modification_date = generated;
                pdf.doc.metadata.info.metadata_date = generated;
                pdf.line("超级工厂执行观察与预测比较", 21.);
                for (title, lines) in &sections {
                    cancelled(cancel)?;
                    pdf.section(title);
                    for s in lines { pdf.line(s, 10.5); }
                }
                pdf.section("同源完整数据分表索引");
                pdf.line("原技术表名对应见HTML各表标题；完整原数据见附件，同包CSV保留原列顺序。",10.);
                for t in &tables {
                    pdf.line(
                        &format!(
                            "{}：{} 行。",
                            table_title(&t.name),
                            t.rows.len()
                        ),
                        10.,
                    );
                }
                pdf.section("完整数据附件");
                pdf.line("本PDF已内嵌report.json附件，完整保留同一报告的来源、全部历史、事实、指标和比较条件。可从PDF阅读器的附件面板提取；独立保存此PDF仍可读回全部原数据。",10.);
                pdf.line("同包report.json与附件内容一致；csv目录提供17张完整明细表，HTML提供同源阅读与明细。字体许可证见同包licenses/font.txt。",10.);
                cancelled(cancel)?;
                files.insert(
                    "report.pdf".into(),
                    file("report.pdf", "application/pdf", pdf.finish(&canonical)?),
                );
            }
        }
    }
    cancelled(cancel)?;
    Ok(files.into_values().collect())
}
