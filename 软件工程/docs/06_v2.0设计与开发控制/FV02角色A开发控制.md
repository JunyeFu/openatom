# FV02 角色 A 开发控制

更新：2026-11-06。负责人 A（JunyeFu），执行入口 [#17](https://github.com/JunyeFu/openatom/issues/17)，剩余三批共 18 人时。本文细化同一张卡，不另起工作包，不追加工时。

**当前结论：文件及函数定位、设计和接收规则已细化；公共接点的行为复验、产品修改和三批接收均未在本次执行。** 下表的预期是验收依据，不是通过记录。沿用 [现行分工](完整版工作包与阶段分工.md)、[机器控制计划](开发控制计划.json)，首批 [#13 签收](https://github.com/JunyeFu/openatom/issues/13#issuecomment-6678966996) 保持有效。

## 1. 基点、输入与执行顺序

- 产品源码基点：95f411d8e37cffbde227f59c679a66ee8489d5b8。集成分支：codex/full-version-integration-20261101；本次文档更新不改变产品代码基点。后续两批以各自前一批实际合入的 SHA 为起点。
- 输入：[FV0 输入包](https://github.com/JunyeFu/openatom/releases/download/fv0-inputs-20261101/SuperFactory-FV0-Inputs-20261101.zip)、[H1 复现包](https://github.com/JunyeFu/openatom/releases/download/fv0-inputs-20261101/SuperFactory-FV0-H1-Repro-20261101.zip)、[冻结输入](FV0冻结输入.json)。沿用 92 台设备、69 个订单、944 条绑定、93 个端点；短执行必须创建新身份，旧会话仅作读取。H1 与百万 generated_load 查询负载分别使用，不互当来源。
- 顺序：A17.1 与 B #18 可并行 → 按需执行 A17.2 → A17.3 接收 #18 → B #19 及其 A 接收 → B #20 及其 A 接收 → A17.4 汇总。A17.2/A17.3 随三批交错执行，不要求 B 等到 A 的 18 小时全部结束。
- C 沿用 #12，收到对应候选后工具复验；重点是 #19 的比较 Q4/Q5/Q25、报告 Q8/Q23/Q24，以及实际受影响的指南。已接收的首批项不重做。
- 按实际代码及下列设计确定语义：[类型归属](类型归属与契约样例设计.md)、[接口与事务](接口与数据库事务设计.md)、[Agent 与报告消费](Agent与执行报告消费设计.md)、[预测与执行比较](预测与执行比较设计.md)。历史设计文件的旧状态不覆盖后续真实签收。

## 2. 六个设计视图落实到本卡

|视图|A 必须固定的内容|本卡输出与边界|
|---|---|---|
|任务与交互|查询选择/分页/取消；工艺维护与恢复；选 Run/比较/导出；Agent 调用及任务查询|沿 B 三卡现有路径接入，不增加新页面或第二套业务流程|
|结构与数据|工程、Run 与四字段 ExecutionIdentity；数据集/来源/修订/游标；数量单位、时钟、未知值；每个 Run 的冻结输入|公共字段由 Domain/Commands 定义。数据库行、Engine 内部结构归 B；A 不另造物品、库存或身份|
|状态与行为|prepared 只代表已准备；Task 完成/取消/失败与生产运行状态分开；旧历史只读；结果未明时找回原登记|请求接纳、任务完成、实际产出分别判断。取消读取不取消生产，传输中断不自动重发有副作用请求|
|接口与协作|H08 云只读任务、H10 报告、H11 数据导出及相关准备/控制；compare_runs 与实际 capabilities|请求/响应/错误复用当前类型；仅实际缺字段或不一致时修改。调用者权限由 Host 识别，不由请求自报|
|界面消费|载入/空结果/失败/取消/未知/历史只读；基准与候选来源；有效旧页的保留与重新查询入口|A 负责公共 service 接点，B 负责功能页面。错误须能识别原因和下一步；不得将缺数据显示成 0 或完成|
|构建与验证|既有 Rust/Node/npm/lock；Rust → schema/TS → Native/Planner/MCP 消费；同提交候选|复用已有工具链和制包入口，只查改动影响。桌面在本机主屏工具验证；不安排真人体验|

## 3. 写入文件、具体接点与消费者

以下定位均以产品基点 95f411d8 为准，后续按函数/类型名查找，不靠旧行号覆盖整文件。

|归属|文件和定位|何时修改、必须连同核对什么|
|---|---|---|
|A：公共数据|[packages/domain/src/cloud_execution.rs](../../packages/domain/src/cloud_execution.rs)：ExecutionIdentity、PredictionExecutionComparison；[cloud_execution_wire.rs](../../packages/domain/src/cloud_execution_wire.rs)：DaaSQuery、DaaSQueryResult、ExecutionCursor；[models.rs](../../packages/domain/src/models.rs)：RunInputRef、RunComparison、ReportRunInputRef|仅当真实消费者无法表达规定来源/单位/可比性等必要语义。cloud_execution_wire.rs 是 Rust 产品源；不能反向手改生成 TS 来替代源定义|
|A：公共请求及导出|[packages/commands/src/api.rs](../../packages/commands/src/api.rs)：OperationName、ApiRequest、CompareRunsParams；[execution_api.rs](../../packages/commands/src/execution_api.rs)：CloudReadSelection、ReadCloudProductionParams、HostCloudReadRequest、CloudReadResult 及报告/数据导出类型；[bin/export-contracts.rs](../../packages/commands/src/bin/export-contracts.rs)：main|同步真实请求、响应和错误，检查调用者授权及导出覆盖。lib.rs 只是模块/再导出入口，不是全部命令定义。生成物位于 packages/commands/generated|
|A：公共消费者适配|[apps/planner/src/services/cloudProduction.ts](../../apps/planner/src/services/cloudProduction.ts)：readCloud、recoverRegisteredCloudTask、prepareExecution、controlExecution、exportExecutionReport、exportCloudData、validateQueryPage、exportQuery；[projectHost.ts](../../apps/planner/src/services/projectHost.ts)：compareActualRuns、startEvaluationExport|查询、执行和导出的主要共享适配在 cloudProduction.ts。只改公共请求/任务观察/来源校验及必要适配，页面布局和业务计算仍由 B 修改|
|A：必要 Native 接点|[apps/planner/src-tauri/src/project_api.rs](../../apps/planner/src-tauri/src/project_api.rs)：project_api、project_read_full_run、project_select_directory；bin/export-engineering-types.rs|只有公共分发或私有 DTO 被实际变更影响时修改/生成，不借本卡重做窗口生命周期。已验收 Q9/保存路径不改不重验|
|B：#18 实现，A 只读接收|cloud-service/src/query.rs、store.rs、migrations；commands/src/host_cloud_read.rs 与执行运行时；simulation/src/engine.rs；features/cloud/CloudDataExplorer.tsx 等；模板/设备接入|查询与索引、维护改派、业务 UI 均归 B。A 不并发改这些文件；公共差异回 #17，最小例子说明问题，B 的其他实现继续|
|B：#19 实现，A 只读接收|reporting/src/comparison.rs、execution_evaluation.rs、evaluation_reading.rs 等；commands 的报告运行时；features/comparison/、features/reporting/|B 完成大报告、比较和格式输出。A 核对公共数据及来源，不在 service 内复制报告计算|
|B：#20 实现，A 只读接收|apps/mcp-bridge/src/main.rs、catalog.rs；integrations/agent；既有 tools/cloud、tools/release-preparation|B 维护实际 stdio/MCP 消费和制包。A 核对公共 schema、Host capabilities、权限与候选接点，不同时改 MCP catalog|

packages/commands/src/project_host.rs 的 dispatch 是共有路由；只有必要的公共接入差异由 A 修改。B 的运行时与该路由确实需要同时调整时，在同一包交接里给出具体函数，顺序提交，避免覆盖。依赖或锁文件只有本包真实需要时再处理。

## 4. A 的四步执行控制（合计 18h）

|步骤/预算|开始条件、操作|交付与完成条件|当前状态|
|---|---|---|---|
|A17.1 接点基线，3h|可独立开始。以固定输入逐项对照第 3 节真实类型和三批消费者；区分已有能力、已观测失败、待 B 候选的行为|在 #17 留一份接点结论：来源/字段/单位/权限/任务语义及实际缺口。已有能力直接复用；不为凑代码增加字段|文件/函数定位与设计已完成；行为基线待执行|
|A17.2 必要的公共修改，5h|仅在 B 给出最小失败例，或 A 观测到必要字段缺失/契约冲突时启动。计划分配 #18 2h、#19 2h、#20 1h，可在总额内调配|小范围 PR：真实失败输入、预期、Rust/适配改动、受影响消费者、定向结果。先改源再生成，不改单一生成物；无真实缺口则记“不需修改及依据”|待具体缺口，不预设必须改代码|
|A17.3 三批集成接收，6h|#18 → #19 → #20，每包约 2h；实际 PR、固定提交、同提交候选和必要输入齐备后进行。先看 diff/结果，再决定必要独立复验|按第 5 节记通过/失败/待验、准确覆盖与证据；原授权下审查后合并。给下一包实际已合入 SHA。C 的工具报告与 A 已验收结论分列|三包均待交付|
|A17.4 阶段汇总，4h|三批已接收，且 C 的相关候选复验已交；证据不足先停在待验，不重跑无关全套|在 #17 集中记录各包提交、公共差异、结果及未测项，更新原分工表/本 JSON；满足 FV02 出口才关闭本卡与阶段|待前三步及 C 对应结果|

A17.2 的 5h 不是要求必须花完；无缺口不造改动。出现超出预算的真实缺陷，在 #17 一次说明影响与必要新增范围，不靠降低预期标通过。机器编译/下载/运行等待不计为有效工作人时。

## 5. 三批接收的最小充分检查

|批次|A 核对的公共预期/实际入口|可复用及必须等待的结果|
|---|---|---|
|#18 数据查询与工艺|readCloud 接受固定 selection，Task 输出匹配；完整原始/聚合结果保留数据集、固定来源/修订、稳定游标。validateQueryPage 拒绝来源或页边界错误，不把无效新页混入已有有效页。prepareExecution 不直接发送 run；控制只对当前归属与允许动作开放。维护/替代/恢复和产出运输能贯通订单、物品、承运身份，无重复业务效果|B 提交 PF04：固定百万负载，原始/聚合各 30 次，完整结果正确性与 p95≤2s 分开记；三个工艺场景及受影响的正常停止/保存恢复。A 先复算固定输出与原始结果；只有受影响或结果矛盾才重跑对应有限路线。93 端点全厂长跑仍在 FV03|
|#19 大报告与比较|compareActualRuns 消费各 Run 自身冻结输入；执行来源独立选择，不能以当前草稿代替。可比/不可比/缺输入分别解释，缺值为未知且有原因，不补 0；缺失必要冻结输入明确失败/不可比，不重新仿真伪造。H10 prediction 可为 null，但字段必须按契约存在。规范化报告数据供四格式共用，所选 Run/来源/单位一致|B 提交固定 38,849 等待负载，Native/HTML 有意义首屏≤5s、20 次翻页/全表筛选 p95≤500ms，生成与打开分别计时；三类比较和受影响格式。A 对照真实输入、结果与导出；C 在同候选完成 #12 对应专项，未交继续待验|
|#20 Agent 与交付|MCP list_tools 来源于实际 Host capabilities 和 schema；未授权调用被拒绝，JSON 不能自报授权。授权内读取/修改、任务取消/失败、旧 Run 冻结读取与 Host 一致；结果未明按原 request/task 找回，不盲重发有副作用操作。H08 为业务只读但会登记 Task，不能要求“任何状态都不写”|B 提交实际 stdio/MCP 调用与隔离短流程、候选下载/解压/启动、保存及新进程恢复。A 查对应 Host 结果和同提交候选，按实际覆盖接收；C 只复验受影响的下载/指南路径。生产云部署、新费用、公开替换不在本卡执行|

公用回归只在相关代码被触及时启用：两个 Run 保存重开后各自冻结输入；旧历史只读；必要的离开/恢复路径。已通过的 #13、FV01 与 C 首批证据按实际范围复用。普通函数单测不能代替 Native 画面，UI 截图也不能代替数据库/Engine 结果。

## 6. 实施时的验证入口与交接格式

命令在“软件工程”目录执行，先按实际 diff 选取，以下不是必须全跑的流水账。本次只细化控制，尚未执行这些命令。

|受影响范围|现有入口|
|---|---|
|公共 Rust wire|cargo test --locked --offline -p sf-domain --test cloud_wire_parity；必要时同 crate 的 type_exports|
|Commands 公共请求/边界|cargo test --locked --offline -p sf-commands --test project_command_contracts；云读取变更按 --lib 的 request_scope_and_caller_authority_are_checked_before_any_read_work、queued_cloud_read_cancellation_preserves_original_request_and_has_no_result 选取|
|Run/保存公共适配|仅在影响时选 --lib 的 comparison_input_adapter_uses_each_actual_frozen_run_before_and_after_save_open、frozen_run_inputs_keep_the_original_scenario_and_exact_module_after_current_edits_save_and_open|
|真实 Rust 类型改变|npm run generate:contracts；仅私有 Native DTO 改变时 npm run generate:types。检查生成 diff 仅包含预期变化|
|Planner service/消费者改变|npm --workspace @superfactory/planner run typecheck；npm --workspace @superfactory/planner run build:web；相应已有 engineering 用例及一条受影响 Native 路径|
|报告/云/MCP 实现|由 B 使用各卡既有入口交结果，A 针对真实差异独立核对，不照抄一次整仓测试|

每包正常只交接一次：PR/精确 SHA → 对应内部候选和必要输入 → 已跑命令/业务预期与实际结果 → 未测/失败 → A 接收结论和下一包 SHA。A 将协作者自测、C 工具报告、A 已验收分开记在 #17，不新建重复证明台账。只有权限、不可替代输入、公共契约冲突、数据安全或正式发布边界阻塞即时沟通。

**阶段出口**：#18/#19/#20 对应实现与实际消费者已接收；公共源/生成契约/消费者一致；C #12 相关剩项已工具复验；该阶段要求的报告、比较、Agent 和短联验没有未解决阻断项。未测项与缺陷明确归属，不能用排队任务充当已完成。FV03/FV04 仍等后续启动。公开 v2.0.1、原下载页与已提交链接保持保护；赛事 PDF 和视频不列入本卡。
