# DRF05 公共字段增量与消费映射

2026-10-14，r3。本轮共同Rust/API/派生类型、生成器仅窗口1写入；docs/02和S04-02/04/05实际视觉/原型/本人证据归窗口2。本文件和声明片段是设计增量，未实施产品导出器。

唯一增量定义：[public-delta.design.json](../../../packages/reporting/public-delta.design.json)；直接替换/迁移用[DRF05_公共增量.design.rs](DRF05_公共增量.design.rs)。同名记录按delta替换；已合入Wait/Claim声明保留。窗口1已合入r2主体；r3字段/变体已在当前源与派生物复核，结果见[消费者复核](DRF05_消费者复核.json)逐项确认。

|载体|结构与可空/单位|当前消费|
|---|---|---|
|EvaluationReport|run_evaluations:Vec<RunEvaluation>；删除旧全局订单/资源/成本/历史4数组；共同time_reference/observation_window为Option；scenario_comparison:Option<RunComparison>|设计说明空Run数组；两Run不同时顶层共同值null，逐Run保留。当前22字段映射见reporting契约。|
|ReportRunInputRef|project_id/project_revision/scenario_id/scenario_revision，全部必填；从原RunInputRef复制|报告保留R3/R4输入身份，即使导出工程R8。原工程冻结文件路径仍属工程包，不改成报告根下可打开文件。|
|RunEvaluation|同一Run身份/证据/范围、时间与计算边界、接受口径、模块版本/假设、jobs/transport、订单/资源/costs、事件/接受/等待/Claim/TC02历史|复用实际TC01/TC02及旧冻结输入；前缀不是整个观察窗产量。|
|RunComparison|run_ids、pairs:Vec<RunPairComparison>、metrics:Vec<MetricComparison>、结论/限制|Metric*唯一迁到domain，API与报告共同使用；删ScenarioComparison，不留兼容副本。|
|RunPairComparison|两RunID、ComparisonStatus、8项有类型条件、InputChange数组|严格/受控变化/仅分列；初始库存变化严格相同=false，但变化完整可受控比较。|
|InputChange|target/path/kind及Option<InputChangeValue> before/after、可空来源/原因|None是侧不存在，null变体是字段空；changed双侧存在，added/removed只有一侧。Scalar含TypedValue/单位；对象快照用已有domain类型。r3已合入setup_policy/parameter_override/source_record/definition。|
|MetricComparison|metric+object_ref+acceptance_scope+currency定位组；逐Run值与MetricPairDifference|分开不同资源/订单/接受单位/币种。NumericDifference数量/时长用符号+幅度；基线0不给百分比，不可比/未知不给差。|
|ResourceMetric|capacity_unit；总持有积分；开放/离班/可用容量积分、工作/等待分解可空并附Issue|基础单位为容量单位·ms，利用率分子只在开放日历；busy为区间并集。缺日历不补0，不裁超容量冲突。|
|CostInput|id/object/component/basis；Option<DecimalAmount> rate、Option<String> currency；正Quantity rate_quantity；includes/source|由ScenarioInput.cost_inputs冻结。36CNY/人时用分母3600000 person_ms。同对象同component占用/付费日历互斥。|
|CostEvaluation|inputs:Vec<CostInput>、lines:Vec<ConditionalCost>、currencies:Vec<CurrencyCostSummary>、unassigned_cost_input_ids、missing_conditions|r3已携inputs保存冻结费率；lines按输入ID取分母/包含项。全所选范围有缺项则完整额/单位成本null。|
|CurrencyCostSummary|已知小计；Option完整额/单一接受Scope/接受量/单位成本；缺项|不同币种分列；币种未定、前缀、接受量0、多Scope不暗分成本。|

CostBasis现有consumed_material/occupied_time/paid_calendar/batch_transport/explicit_disposal足够本轮支持费项。未支持的独立能耗、投资/折旧/收益/专业费用自动归因进成本缺项，不创建虚拟计量。Canonical amount沿现行FiniteNumber；费率与数量按精确输入计量后一次转换，显示舍入不反写基础数据。

WaitEndCause精确四值为condition_changed/resolved/observation_end/computed_boundary；条件变化只结束本原因段，resolved才解除等待。ClaimIntervalEndCause精确三值为released/observation_end/computed_boundary，按ResourceClaim.intervals序号一一对应，长度一致；phase_ref是实际开始时阶段。释放/解除恰在end发生仍为released/resolved，全窗未结束是observation_end，前缀/暂停/取消是computed_boundary。旧conditions_changed/conditions_satisfied/run_boundary只在明确旧设计转换中映射，API不接旧值；原数值不改，PhaseWorkEndCause仍用其现行三值。

四格式绑定见[reporting契约](../../../packages/reporting/evaluation-report.contract.design.json)的canonical_bindings：同一报告内部读逐Run量、costs.inputs、pairs与metrics，15分表保留精确单位/null状态/结束原因。输出器不查当前价格，不从可见事件分页重新计产量。

保存/ZIP/报告仍沿18操作。Host对实际解析目标按准入顺序提交，R8先准备好也等R7轮次；每任务只写冻结R。ReplaceFileW失败核对旧根/保留文件实际位置，恢复成功也仍是本次保存失败；恢复失败报授权范围内相对恢复位置/dirty。公共storage.replacement_recovery已收录，实际Windows未试验。新增HFC15/16只给独立预期。

生成器精确保留源为scenarios/模板与交接/expected-template-handoff.design.json，字段refinement_ref/drf05_handoff_cases/refinement_generation_note；窗口1已在推导模板接手预期.py最终write前保留。原10例、E01—E12和HFC01—14不改；本窗口未运行该覆写生成器。15组正反例见[增量JSON](DRF05_增量正反例.json)，实际导出/保存/跨机/发布均空。


本轮收口：r3当前公共同步剩余0；[四格式与保存试做](DRF05_四格式与保存试做.md)已实际写出/读回一个type_fixture报告的JSON/CSV/HTML/4页PDF，69共享字段匹配，PDF已逐页AI检查。HTML浏览器file协议预览被安全策略拒绝，呈现未验证。保存只新跑逻辑队列/6终态参考，原12 I/O实试单列未重跑；新增Windows恢复、16软件接手/跨机与本人证据仍空。
