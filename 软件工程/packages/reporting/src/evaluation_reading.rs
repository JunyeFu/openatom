//! Human reading projection. The original report and tabular evidence are unchanged.
use crate::evaluation_output::money;
use serde::Serialize;
use serde_json::Value;
use sf_domain::*;
use std::collections::BTreeMap;

pub(crate) struct Section {
    pub title: String,
    pub rows: Vec<(String, String)>,
}
impl Section {
    fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: vec![],
        }
    }
    fn row(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.rows.push((label.into(), value.into()));
    }
}
fn atom<T: Serialize>(v: &T) -> String {
    human(&serde_json::to_value(v).unwrap())
}
// Composite values are labelled phrases, never a serialized object in body prose.
fn human(v: &Value) -> String {
    match v {
        Value::Null => "未知/未声明".into(),
        Value::String(s) => s.clone(),
        Value::Array(a) if a.is_empty() => "无".into(),
        Value::Array(a) => a.iter().map(human).collect::<Vec<_>>().join("；"),
        Value::Object(m) => m
            .iter()
            .map(|(k, v)| format!("{k}：{}", human(v)))
            .collect::<Vec<_>>()
            .join("；"),
        _ => v.to_string(),
    }
}
fn quantity(q: &Quantity) -> String {
    let mut digits = q.value.coefficient.value().to_string();
    let places = q.value.decimal_places.value() as usize;
    if places > 0 {
        if digits.len() <= places {
            digits = "0".repeat(places + 1 - digits.len()) + &digits;
        }
        digits.insert(digits.len() - places, '.');
    }
    format!("{digits} {}", q.unit.as_str())
}
fn window(w: &WindowMs) -> String {
    format!("{}–{} ms", w.start_ms.0.value(), w.end_ms.0.value())
}
fn scope(s: &AcceptanceScope) -> String {
    format!(
        "产品 {}@{}；单位 {}；接受位置 {}；阶段 {}",
        s.product_ref.id.as_str(),
        s.product_ref.revision.as_str(),
        s.unit.as_str(),
        s.destination.as_str(),
        atom(&s.stage)
    )
}
fn goal(s: &mut Section, g: &FactoryGenerationGoal) {
    s.row(
        "声明目标",
        format!(
            "日目标 {}；这是输入目标，不由短观察窗外推日产量。",
            g.accepted_quantity_per_day
                .as_ref()
                .map_or("未知".into(), quantity)
        ),
    );
    s.row("目标观察窗", window(&g.observation_window));
    s.row("接受口径", scope(&g.acceptance_scope));
    for extra in &g.additional_acceptance_scopes {
        s.row("其他接受口径", scope(extra));
    }
    s.row(
        "行业 / 产品族",
        format!("{} / {}", atom(&g.industry), atom(&g.product_family)),
    );
    s.row("场地约束", atom(&g.site_envelope));
    issues(s, "目标待补条件", &g.constraints);
}
fn sources(s: &mut Section, entries: &[ParameterSource], common: &[ParameterSource]) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut repeated = 0;
    for p in entries {
        if common
            .iter()
            .any(|c| serde_json::to_value(c).unwrap() == serde_json::to_value(p).unwrap())
        {
            repeated += 1;
            continue;
        }
        let value = match &p.value {
            Some(TypedValue::Quantity(q)) => quantity(q),
            Some(TypedValue::Duration(d)) => format!("{} ms", d.value.0.value()),
            Some(TypedValue::String(raw)) if p.field_path == "/acceptance_goal" => {
                if let Ok(g) = serde_json::from_str::<FactoryGenerationGoal>(raw) {
                    goal(s, &g);
                    continue;
                }
                raw.clone()
            }
            Some(v) => atom(v),
            None => "此来源条目未重复数值（不表示数值为零或字段缺失）".into(),
        };
        let source = format!(
            "{}；来源 {}",
            atom(&p.basis),
            p.source_ref.as_ref().map_or("未声明", |id| id.as_str())
        );
        groups.entry(source).or_default().push(format!(
            "{} {}：{value}",
            p.target.id.as_str(),
            p.field_path
        ));
    }
    if repeated > 0 {
        s.row(
            "共用输入来源",
            format!("{repeated}条与上文导出工程的来源条目完全相同；其余条目列于本Run。"),
        );
    }
    for (source, fields) in groups {
        s.row(source, fields.join("\n"));
    }
}
fn issues(s: &mut Section, label: &str, entries: &[Issue]) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for issue in entries {
        let key = format!(
            "{}：{}；字段 {}；影响 {}",
            issue.code,
            issue.message,
            issue.field_path.as_deref().unwrap_or("未指定"),
            atom(&issue.blocks)
        );
        groups.entry(key).or_default().push(
            issue.object_ref.as_ref().map_or("未指定对象".into(), |r| {
                format!("{} {}", atom(&r.kind), r.id.as_str())
            }),
        );
    }
    for (description, objects) in groups {
        s.row(
            label,
            format!(
                "{description}\n相同条件共{}条；前3个对象阅读示例：{}。完整对象保留在原始数据，例子不代表全部对象。",
                objects.len(),
                objects.iter().take(3).cloned().collect::<Vec<_>>().join("、")
            ),
        );
    }
}
fn provenance(s: &mut Section, value: &Value) {
    if let Some(revision) = value.get("revision") {
        s.row("冻结工程修订", human(revision));
    }
    if let Some(halls) = value.get("halls").and_then(Value::as_array) {
        for h in halls {
            s.row(
                "场地声明",
                format!(
                    "{}；宽 {} m × 深 {} m × 高 {} m；声明几何不等于现场实测。",
                    human(&h["id"]),
                    human(&h["width_m"]),
                    human(&h["depth_m"]),
                    human(&h["height_m"])
                ),
            );
        }
    }
    if let Some(calendars) = value.get("calendars").and_then(Value::as_array) {
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for c in calendars {
            let key = format!("{}；来源 {}", human(&c["intervals"]), human(&c["source"]));
            groups.entry(key).or_default().push(human(&c["id"]));
        }
        for (detail, ids) in groups {
            s.row(
                "班次 / 日历",
                format!("{}：{detail}（时间基础单位 ms）", ids.join("、")),
            );
        }
    }
    if let Some(records) = value.get("sources").and_then(Value::as_array) {
        for r in records {
            s.row("来源记录", human(r));
        }
    }
    if let Some(adoptions) = value.get("template_adoptions").and_then(Value::as_array) {
        for a in adoptions {
            s.row("模板采用", format!("{}；资源选择{}项；检查预设{}项；待补输入{}项。模板示例和预设不构成现场检查或专业认可。", human(&a["template_ref"]), a["resource_choices"].as_array().map_or(0, Vec::len), a["inspection_presets"].as_array().map_or(0, Vec::len), a["missing_inputs"].as_array().map_or(0, Vec::len)));
            if let Some(inputs) = a["missing_inputs"].as_array().filter(|v| !v.is_empty()) {
                let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for input in inputs {
                    let key = format!(
                        "{}；字段{}",
                        human(&input["kind"]),
                        human(&input["required_fields"])
                    );
                    groups
                        .entry(key)
                        .or_default()
                        .push(human(&input["object_ref"]["id"]));
                }
                for (fields, objects) in groups {
                    s.row(
                        "模板历史待补",
                        format!(
                            "{fields}；{}项；对象{}。这是原模板登记，不是本次重新检查的结论。",
                            objects.len(),
                            objects.join("、")
                        ),
                    );
                }
            }
        }
    }
    for (key, label) in [
        ("artifacts", "专业成果"),
        ("results", "专业结果"),
        ("acknowledgments", "人工认可"),
        ("current_bindings", "当前采用绑定"),
    ] {
        if let Some(v) = value.get(key) {
            s.row(
                label,
                match v.as_array() {
                    Some(a) if a.is_empty() => "无保存记录；不据来源标签新增运行授权。".into(),
                    _ => human(v),
                },
            );
        }
    }
}
fn snapshot(s: &mut Section, input: &ScenarioInput) {
    s.row(
        "明确选择的方案",
        format!(
            "{}；方案修订{}；工程修订{}；观察窗{}",
            input.id.as_str(),
            input.revision.0.value(),
            input.project_revision.0.value(),
            window(&input.window)
        ),
    );
    if let Some(g) = &input.acceptance_goal {
        goal(s, g);
    }
    for t in &input.production_tasks {
        s.row(
            "任务输入",
            format!(
                "{}；计划{}；任务释放时点{} ms；优先级{}；工艺{}@{}",
                t.id.as_str(),
                quantity(&t.planned_quantity),
                t.release_at_ms.0.value(),
                t.priority,
                t.routing_ref.id.as_str(),
                t.routing_ref.revision.as_str()
            ),
        );
    }
    s.row("初始库存记录", format!("初始物品共{}条记录；记录数不是库存Quantity。完全相同的声明合并显示次数，数量不作推算。", input.initial_conditions.items.len()));
    let mut stocks = BTreeMap::new();
    for item in &input.initial_conditions.items {
        let entry = stocks
            .entry(serde_json::to_string(item).unwrap())
            .or_insert((0, item));
        entry.0 += 1;
    }
    for (count, item) in stocks.values() {
        s.row(
            "初始库存",
            format!(
                "{}：{}；物料{}@{}；所在{}；质量{}；来源{}；相同声明{}次",
                item.id.as_str(),
                quantity(&item.quantity),
                item.item_type_ref.id.as_str(),
                item.item_type_ref.revision.as_str(),
                atom(&item.holder),
                atom(&item.quality),
                atom(&item.source),
                count
            ),
        );
    }
    s.row(
        "其他方案输入",
        format!(
            "订单{}项；到货{}项；供应关系{}项；参数覆盖{}项；成本输入{}项。",
            input.orders.len(),
            input.arrivals.len(),
            input.supplies.len(),
            input.parameter_overrides.len(),
            input.cost_inputs.len()
        ),
    );
    for order in &input.orders {
        s.row("订单输入", atom(order));
    }
    for arrival in &input.arrivals {
        s.row("到货输入", atom(arrival));
    }
    for supply in &input.supplies {
        s.row("供应关系", atom(supply));
    }
    for p in &input.parameter_overrides {
        s.row("参数覆盖", atom(p));
    }
    for preset in &input.quality_presets {
        s.row("质量预设", atom(preset));
    }
    issues(s, "方案待补条件", &input.issues);
}
fn conclusion(s: &mut Section, text: &str, recorded: &BTreeMap<String, Value>) {
    if let Some(raw) = text.strip_prefix("明确选择的方案输入快照：") {
        if let Ok(input) = serde_json::from_str::<ScenarioInput>(raw) {
            snapshot(s, &input);
            return;
        }
    }
    for prefix in [
        "导出工程的来源/模板/场地/班次记录（不是本次重新检查的结论）：",
        "保存的专业成果/人工认可/当前采用事实（不据来源标签新增运行授权）：",
    ] {
        if let Some(raw) = text.strip_prefix(prefix) {
            if let Ok(v) = serde_json::from_str::<Value>(raw) {
                s.row("记录性质", prefix.trim_end_matches('：'));
                provenance(s, &v);
                return;
            }
        }
    }
    if text.starts_with("Run ") {
        if let Some((heading, raw)) = text.split_once(" 的冻结来源/模板/场地/班次/专业采用记录：")
        {
            if let Ok(v) = serde_json::from_str::<Value>(raw) {
                s.row("冻结来源所属Run", heading);
                if v.as_object().is_some_and(|m| {
                    m.iter().all(|(key, value)| {
                        key == "revision"
                            || key == "generation_context" && value.is_null()
                            || recorded.get(key) == Some(value)
                    })
                }) {
                    s.row("冻结工程修订", human(&v["revision"]));
                    s.row("冻结来源记录", "来源、模板、场地、班次和专业采用记录与上文保存记录逐项相同，合并阅读展示；此处仍属于该Run的冻结输入。");
                    return;
                }
                provenance(s, &v);
                return;
            }
        }
        if let Some((run, rest)) = text.split_once(" / 接受口径 ") {
            if let Some((raw, fact)) = rest.split_once('：') {
                if let Ok(acceptance) = serde_json::from_str::<AcceptanceScope>(raw) {
                    // The exact quantity is already present in the human A= sentence.
                    s.row(
                        run,
                        format!(
                            "{}\n{}",
                            scope(&acceptance),
                            fact.split("，精确基础值").next().unwrap()
                        ),
                    );
                    return;
                }
            }
        }
    }
    s.row("结论", text);
}
fn metric(v: &MetricValue) -> String {
    match v {
        MetricValue::Quantity(q) => quantity(q),
        MetricValue::Duration(d) => format!("{} ms", d.0.value()),
        MetricValue::Scalar { value, unit } => format!("{} {}", value.value(), unit.as_str()),
        MetricValue::MoneyPerUnit {
            amount,
            currency,
            unit,
        } => format!("{} {currency}/{}", money(amount.value()), unit.as_str()),
        MetricValue::Unknown(i) => format!("未知；{}（{}）", i.message, i.code),
    }
}
fn metric_name(key: &MetricKey) -> &'static str {
    match key {
        MetricKey::AcceptedQuantity => "接受量",
        MetricKey::CompletionTime => "完整接受时刻",
        MetricKey::WaitingTime => "等待时间",
        MetricKey::ResourceUtilization => "资源利用率",
        MetricKey::UnitCost => "单位成本",
        MetricKey::ReworkQuantity => "返工量",
    }
}
fn run_fact(text: &str, run_id: &RunId) -> Option<(AcceptanceScope, String)> {
    let prefix = format!("Run {} / 接受口径 ", run_id.as_str());
    let (raw, fact) = text.strip_prefix(&prefix)?.split_once('：')?;
    if !(fact.starts_with("实际接受量 A=")
        || fact.starts_with("完整接受时刻 C=")
        || fact.starts_with("C=null（未知）："))
    {
        return None;
    }
    let acceptance = serde_json::from_str(raw).ok()?;
    let fact = fact.split("，精确基础值").next().unwrap();
    let fact = fact.strip_prefix("C=null（未知）：").map_or_else(
        || fact.replace("C=null", "C=未知"),
        |reason| format!("完整接受时刻 C=未知；{reason}"),
    );
    Some((acceptance, fact))
}
fn difference(v: &NumericDifference) -> String {
    match v {
        NumericDifference::Quantity {
            negative,
            magnitude,
        } => format!(
            "{}{}",
            if *negative { "-" } else { "+" },
            quantity(magnitude)
        ),
        NumericDifference::Duration {
            negative,
            magnitude,
        } => format!(
            "{}{} ms",
            if *negative { "-" } else { "+" },
            magnitude.0.value()
        ),
        NumericDifference::Scalar { value, unit } => format!("{} {}", value.value(), unit.as_str()),
        NumericDifference::MoneyPerUnit {
            amount,
            currency,
            unit,
        } => format!("{} {currency}/{}", money(amount.value()), unit.as_str()),
    }
}
fn changed_leaves(s: &mut Section, path: &str, before: &Value, after: &Value) {
    if before == after {
        return;
    }
    if let (Some(a), Some(b)) = (before.as_object(), after.as_object()) {
        let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for key in keys {
            changed_leaves(s, &format!("{path}/{key}"), &before[key], &after[key]);
        }
    } else {
        let label = if path.ends_with("/release_at_ms") {
            "任务释放时点"
        } else {
            path
        };
        let unit = if path.ends_with("_ms") { " ms" } else { "" };
        s.row(
            label,
            format!(
                "{}{unit} → {}{unit}；字段{path}",
                human(before),
                human(after)
            ),
        );
    }
}
pub(crate) fn sections(report: &Versioned<EvaluationReport>) -> Vec<Section> {
    let r = &report.data;
    let mut result = vec![];
    let mut displayed_facts = std::collections::BTreeSet::new();
    let mut s = Section::new("任务、边界与评价口径");
    s.row("评价问题", &r.question_and_scope);
    s.row(
        "证据性质",
        if matches!(r.mode, ReportMode::DesignDescription) {
            "设计说明：无实际Run。"
        } else {
            "软件Run在声明模型、输入和观察窗下的模拟结果；不是现场观测或软件性能测量。"
        },
    );
    if let Some(t) = &r.time_reference {
        s.row("时间基准", atom(t));
    }
    if let Some(w) = &r.observation_window {
        s.row("报告观察窗", window(w));
    }
    for a in &r.acceptance_scope {
        s.row("接受口径", scope(a));
    }
    for supported in &r.supported_scope {
        s.row("支持范围", supported);
    }
    result.push(s);

    if !r.run_evaluations.is_empty() {
        let mut s = Section::new("实际Run与结果");
        for run in &r.run_evaluations {
            s.row(
                "Run / 冻结输入",
                format!(
                    "{}；工程{}；冻结工程修订{}；方案{}；方案修订{}",
                    run.run_id.as_str(),
                    run.input_ref.project_id.as_str(),
                    run.input_ref.project_revision.0.value(),
                    run.input_ref.scenario_id.as_str(),
                    run.input_ref.scenario_revision.0.value()
                ),
            );
            s.row(
                "计算边界",
                format!(
                    "请求窗{}；已计算至{}；{}；状态{}；目标达成：{}",
                    window(&run.requested_window),
                    run.computed_until_ms
                        .as_ref()
                        .map_or("未知".into(), |t| format!("{} ms", t.0.value())),
                    if run.whole_window_computed {
                        "覆盖完整观察窗"
                    } else {
                        "未覆盖完整观察窗"
                    },
                    atom(&run.computation_status),
                    run.goal_met.map_or("未知", |v| if v { "是" } else { "否" })
                ),
            );
            s.row(
                "Run时间基准 / 来源",
                format!(
                    "{}；{}",
                    atom(&run.time_reference),
                    atom(&run.evidence_origin)
                ),
            );
            for a in &run.acceptance_scope {
                s.row("Run接受口径", scope(a));
            }
            for text in &r.conclusions {
                if let Some((acceptance, fact)) = run_fact(text, &run.run_id) {
                    if !run.acceptance_scope.iter().any(|a| {
                        serde_json::to_value(a).unwrap()
                            == serde_json::to_value(&acceptance).unwrap()
                    }) {
                        s.row("结果接受口径", scope(&acceptance));
                    }
                    s.row("接受结果", fact);
                    displayed_facts.insert(text);
                }
            }
            for order in &run.order_metrics {
                s.row(
                    "订单结果",
                    format!(
                        "{}；需求{}；已接受{}；未完成{}；完成时刻{}；交期{}；迟交{}",
                        order.order_id.as_str(),
                        quantity(&order.demand),
                        quantity(&order.accepted),
                        quantity(&order.outstanding),
                        order
                            .completion_ms
                            .as_ref()
                            .map_or("未知".into(), |t| format!("{} ms", t.0.value())),
                        order
                            .due_at_ms
                            .as_ref()
                            .map_or("未知/未声明".into(), |t| format!("{} ms", t.0.value())),
                        order
                            .tardiness_ms
                            .as_ref()
                            .map_or("未知".into(), |t| format!("{} ms", t.0.value()))
                    ),
                );
            }
            if run.order_metrics.is_empty() {
                s.row("订单结果", "无订单指标记录；目标口径结果见结论与比较。");
            }
            if run.costs.currencies.is_empty() {
                s.row(
                    "条件成本",
                    "未形成币种成本汇总；完整成本、单位成本未知，不能以零成本代替。",
                );
            }
            for c in &run.costs.currencies {
                s.row(
                    "条件成本",
                    format!(
                        "{}：已知小计{}；完整总额{}；单位成本{}",
                        c.currency,
                        money(c.known_subtotal.value()),
                        c.complete_total
                            .as_ref()
                            .map_or("未知".into(), |v| money(v.value())),
                        c.unit_cost.as_ref().map_or("未知".into(), |v| format!(
                            "{} {}/{}",
                            money(v.value()),
                            c.currency,
                            c.accepted_scope
                                .as_ref()
                                .map_or("单位未知", |a| a.unit.as_str())
                        ))
                    ),
                );
                if let Some(a) = &c.accepted_scope {
                    s.row(
                        "成本分母口径",
                        format!(
                            "{}；数量{}",
                            scope(a),
                            c.accepted_quantity.as_ref().map_or("未知".into(), quantity)
                        ),
                    );
                }
                issues(&mut s, "币种缺项", &c.missing_conditions);
            }
            if !run.costs.currencies.is_empty() {
                s.row(
                    "币种规则",
                    "各币种独立列示，不跨币种合计；显示金额保留两位，原始精度保存在完整数据中。",
                );
            }
            for resource in &run.resource_metrics {
                s.row(format!("资源 {}", resource.resource_id.as_str()), format!("忙碌{} ms；占用容量积分{}；可用容量积分{}；积分单位{}·ms；利用率{}；有效工作{}；等待{}", resource.any_busy_ms.0.value(),resource.occupied_capacity_ms.value(),resource.available_capacity_ms.as_ref().map_or("未知".into(), |v|v.value().to_string()),resource.capacity_unit.as_str(),resource.utilization.as_ref().map_or("未知".into(),|v|format!("{:.2}%",v.value()*100.)),resource.effective_work_ms.as_ref().map_or("未知".into(),|v|format!("{} ms",v.0.value())),resource.waiting_ms.as_ref().map_or("未知".into(),|v|format!("{} ms",v.0.value()))));
                s.row(
                    "日历内外占用",
                    format!(
                        "开班占用{}；闭班保有{}（{}·ms）；利用率仅作两位小数显示，基础值不变。",
                        resource
                            .open_calendar_occupied_capacity_ms
                            .as_ref()
                            .map_or("未知".into(), |v| v.value().to_string()),
                        resource
                            .closed_calendar_held_capacity_ms
                            .as_ref()
                            .map_or("未知".into(), |v| v.value().to_string()),
                        resource.capacity_unit.as_str()
                    ),
                );
                issues(&mut s, "资源缺项", &resource.missing_conditions);
            }
            issues(&mut s, "Run限制", &run.limitations);
            issues(&mut s, "成本缺项", &run.costs.missing_conditions);
        }
        result.push(s);
    }
    let mut s = Section::new("方案比较与实际变化");
    if let Some(c) = &r.scenario_comparison {
        for pair in &c.pairs {
            s.row(
                "比较双方",
                format!(
                    "基线{}；候选{}；{}",
                    pair.baseline_run_id.as_str(),
                    pair.candidate_run_id.as_str(),
                    match pair.status {
                        ComparisonStatus::Strict => "严格比较",
                        ComparisonStatus::ControlledChanges => "受控变更比较",
                        ComparisonStatus::SideBySideOnly => "仅分列证据",
                    }
                ),
            );
            for condition in &pair.conditions {
                s.row("共同条件", atom(condition));
            }
            for change in &pair.input_changes {
                s.row(
                    "冻结输入变化",
                    format!(
                        "{}；{}；依据{}；{}",
                        change.target.id.as_str(),
                        change.path,
                        atom(&change.basis),
                        change.reason.as_deref().unwrap_or("未声明原因")
                    ),
                );
                changed_leaves(
                    &mut s,
                    &change.path,
                    &serde_json::to_value(&change.before).unwrap(),
                    &serde_json::to_value(&change.after).unwrap(),
                );
            }
            for m in &c.metrics {
                let values: Vec<_> = m
                    .values
                    .iter()
                    .filter_map(|v| {
                        if v.run_id == pair.baseline_run_id {
                            Some(format!("基线：{}", metric(&v.value)))
                        } else if v.run_id == pair.candidate_run_id {
                            Some(format!("候选：{}", metric(&v.value)))
                        } else {
                            None
                        }
                    })
                    .collect();
                s.row(
                    format!("比较指标 {}", metric_name(&m.metric)),
                    format!(
                        "{}；对象{}；口径{}",
                        values.join("；"),
                        atom(&m.object_ref),
                        m.acceptance_scope.as_ref().map_or("未指定".into(), scope)
                    ),
                );
                for d in m.pair_differences.iter().filter(|d| {
                    d.baseline_run_id == pair.baseline_run_id
                        && d.candidate_run_id == pair.candidate_run_id
                }) {
                    if d.comparable {
                        s.row(
                            "比较差值",
                            format!(
                                "差值：{}；变化率{}",
                                d.difference.as_ref().map_or("未知".into(), difference),
                                d.change_percent
                                    .as_ref()
                                    .map_or("未知".into(), |v| format!("{}%", v.value()))
                            ),
                        );
                    } else {
                        s.row(
                            "比较限制",
                            if matches!(m.metric, MetricKey::CompletionTime) {
                                "完整接受时刻不满足可比条件，不能计算时间改善；具体原因见指标条件，未知不等于0。"
                            } else {
                                "该指标不可计算改善量；双方仅分列。"
                            },
                        );
                    }
                    issues(&mut s, "指标条件", &d.conditions);
                }
            }
        }
        for text in &c.conclusions {
            s.row("比较结论", text);
        }
        issues(&mut s, "比较限制", &c.limitations);
    } else {
        s.row(
            "比较状态",
            if matches!(r.mode, ReportMode::DesignDescription) {
                "设计说明：无实际Run；比较不适用。"
            } else if r.run_refs.len() == 1 {
                "单次实际Run可直接交付；没有比较，不强制生成修正Run或改善量。"
            } else {
                "未附比较，各Run仅分列证据。"
            },
        );
    }
    result.push(s);

    if !r.run_evaluations.is_empty() {
        let mut s = Section::new("运行历史阅读示例与等待摘要");
        s.row("阅读规则", "按冻结顺序列每Run前3条阅读示例；示例不是统计抽样，不据此推断产量或等待总量。完整事件保存在原始数据中。");
        for (index, run) in r.run_evaluations.iter().enumerate() {
            s.row(
                "事件记录",
                format!(
                    "Run {}：{}条；展示{}条。完整结构字段 /data/run_evaluations/{index}/events。",
                    run.run_id.as_str(),
                    run.events.len(),
                    run.events.len().min(3)
                ),
            );
            for event in run.events.iter().take(3) {
                s.row(
                    "事件示例",
                    format!(
                        "序号{}；时点{} ms；对象{}；{}",
                        event.sequence.value(),
                        event.at_ms.0.value(),
                        event.subject.id.as_str(),
                        atom(&event.payload)
                    ),
                );
            }
            let mut reasons = BTreeMap::new();
            let mut endings = BTreeMap::new();
            for wait in &run.waits {
                *endings.entry(atom(&wait.end_cause)).or_insert(0usize) += 1;
                for reason in &wait.reasons {
                    *reasons.entry(atom(&reason.kind)).or_insert(0usize) += 1;
                }
            }
            s.row(
                "等待记录",
                format!(
                    "Run {}：{}条等待区间；完整记录包含每个区间的起止、结束原因和所有等待原因。",
                    run.run_id.as_str(),
                    run.waits.len()
                ),
            );
            for (kind, count) in reasons {
                s.row(
                    "等待原因计数",
                    format!("等待原因类别 {kind}：{count}条记录（不是时长或产量）。"),
                );
            }
            for (cause, count) in endings {
                s.row(
                    "等待区间计数",
                    format!("等待结束原因 {cause}：{count}条记录。"),
                );
            }
        }
        s.row("等待结束含义","仅resolved表示该等待已解除；condition_changed仅表示原原因段结束，observation_end/computed_boundary表示在观察窗或计算边界截断。");
        result.push(s);
    }
    let mut s = Section::new("结论与保存的输入记录");
    let mut recorded = BTreeMap::new();
    for text in &r.conclusions {
        if displayed_facts.contains(text) {
            continue;
        }
        conclusion(&mut s, text, &recorded);
        for prefix in [
            "导出工程的来源/模板/场地/班次记录（不是本次重新检查的结论）：",
            "保存的专业成果/人工认可/当前采用事实（不据来源标签新增运行授权）：",
        ] {
            if let Some(raw) = text.strip_prefix(prefix) {
                if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(raw) {
                    recorded.extend(m);
                }
            }
        }
    }
    result.push(s);
    let mut s = Section::new("参数来源、模块与生成检查");
    sources(&mut s, &r.input_assumptions, &[]);
    s.row(
        "导出工程模块版本",
        r.module_versions
            .iter()
            .map(|m| format!("{}@{}", m.id.as_str(), atom(&m.version)))
            .collect::<Vec<_>>()
            .join("；"),
    );
    for run in &r.run_evaluations {
        s.row(
            "冻结Run模块版本",
            format!(
                "{}：{}",
                run.run_id.as_str(),
                run.module_versions
                    .iter()
                    .map(|m| format!("{}@{}", m.id.as_str(), atom(&m.version)))
                    .collect::<Vec<_>>()
                    .join("；")
            ),
        );
        sources(&mut s, &run.input_assumptions, &r.input_assumptions);
    }
    if let Some(context) = &r.generation_context {
        goal(&mut s, &context.goal);
        s.row("采用模板", atom(&context.template_ref));
        s.row("设计检查", atom(&context.draft_check));
        s.row("停止原因", atom(&context.stop_reason));
        s.row(
            "生成Run记录",
            format!(
                "初始{}；修正{}",
                atom(&context.initial_run_id),
                atom(&context.revised_run_ids)
            ),
        );
        for change in &context.changes {
            s.row("生成变化", atom(change));
        }
        sources(&mut s, &context.parameter_sources, &r.input_assumptions);
    } else {
        s.row(
            "生成检查 / 停止原因",
            "未保存 generation_context；不能从成功Run或模板标签推定检查通过、修正历史或停止原因。",
        );
    }
    result.push(s);
    let mut s = Section::new("待补条件、限制与接手");
    issues(&mut s, "报告待补条件", &r.missing_conditions);
    issues(&mut s, "报告限制", &r.limitations);
    s.row("可编辑工程引用", atom(&r.editable_project_ref));
    s.row("接手方式","报告数据不是可恢复运行快照；可编辑工程须明确随附，从初始输入重跑；历史Run仍属于其冻结输入。");
    for rights in &r.asset_rights {
        s.row("资产权利声明", atom(rights));
    }
    result.push(s);
    result
}
