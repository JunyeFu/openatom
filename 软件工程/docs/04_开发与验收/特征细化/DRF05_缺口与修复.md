# DRF05 缺口与修复（本轮收口）

2026-10-14，r3。r2公共载荷已合入并完成拥有者消费映射；r3字段/变体已合入，当前对应关系见[消费者复核](DRF05_消费者复核.json)为准。本轮共同合同/生成源由窗口1唯一写，docs/02及S04-02/04/05实际视觉由窗口2负责，DRF05只改本窗口范围。

直接合入材料：[字段delta JSON](../../../packages/reporting/public-delta.design.json) · [Rust声明片段](DRF05_公共增量.design.rs) · [结构说明](DRF05_公共增量.md)。每个类型已有完整字段、嵌套、Option/单位、序列化、源/消费者及正反例。

|原请求|当前事实与本轮处理|剩余入口|
|---|---|---|
|H01|TC01/TC02、computed_until_ms、Events interval:null及精确Wait/Claim结束原因均在公共Rust/Schema/TS；报告按观察点闭包/区间半开消费，不从end_ms猜结束。|定义/类型已合入；真实Engine/Host/产品运行另取证。|
|H02|22字段EvaluationReport含逐Run载荷、4字段ReportRunInputRef、实际jobs/transport/模块版本/假设及TC数据；旧全局4数组已移除。报告/CSV消费已对齐。|r3成本快照依H05；导出器尚未实现。|
|H03|同源RunComparison/Metric*与配对、条件、typed before/after、符号差值已合入。必要快照明确覆盖domain输入。|r3已合入InputChangeValue的setup_policy/parameter_override/source_record/definition四种已有类型；当前复核剩余0。|
|H04|容量单位/开放分子/离班量、可空日历/实际工作/等待、Claim.phase_ref与结束数组已合入。报告按真实交集与区间并集。|实际simulation指标运行仍空，既有80k/60k/20k/160k独立预期不改。|
|H05|冻结ScenarioInput.cost_inputs、费率分母/币种/包含项、ConditionalCost与币种小计/完整额/单位成本已合入。CostBasis五类够支持范围。|r3已合入CostEvaluation.inputs:Vec<CostInput>及missing_conditions:Vec<Issue>，使离线报告同源读费率且不漏未支持的所选费用。|
|H06|实际目录键顺序提交、ReplaceFileW失败核对/恢复与IoFailure/dirty已纳入公共storage/ownership。HFC15/16补顺序与恢复成功反例。|实际Windows恢复、跨机软件接手未执行。|
|H07|comparison_export_display_contract已接入两Run/变化/未知/前缀/末刻/失败返回及dirty处理。|实际母版/可点击/本人证据归窗口2。|
|H08|推导模板接手预期.py已保留refinement_ref/drf05_handoff_cases/refinement_generation_note。原10例与原14接手值不变，新增2例。|正式任务包仍须S06冻结/S07实际入口后刷新/派发；本窗口不改生成源或跑覆写生成器。|

r3确切差异在delta.post_r2_consumer_correction：两个CostEvaluation字段和四个InputChangeValue快照变体，复用已存在domain类型，不加操作/接口层。字段已准备到可直接替换；是否实际进入共同源/Schema/TS按复核逐项记录。

验证为原12独立预期（9项数学重算）、15组独立正反例、16个接手定义及当前Rust/Schema/TS只读核对。原实际结果不代填；声明片段未由本窗口编译，未调用产品导出/保存/打开或跨机试验。原17SF/21P/60S/Q01—64、S06/S07门与draft_not_dispatchable不变。


本轮收口：r3当前公共同步剩余0；[四格式与保存试做](DRF05_四格式与保存试做.md)记录type_fixture四格式69行读回、4页PDF检查及新授权受限HTTP的HTML实际浏览器呈现/截图。窄窗口单位断行和表头丢失已修复，临时27855已释放，窗口2的8921保持监听。新增两项隔离Windows文件接点：实际准备8→7、替换提交7→8；文件占用错误32且旧R7根保留。与6逻辑参考/原12 I/O实试分开；Host目录键、1176/1177恢复、产品/16软件交接/跨机与本人证据仍待取得。
