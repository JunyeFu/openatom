# FV02 角色 A 开发控制

更新：2026-11-14。负责人 A（JunyeFu），执行入口 [#17](https://github.com/JunyeFu/openatom/issues/17)，剩余三批共 18 人时。本文细化同一张卡，不另起工作包，不追加工时。

**当前结论：A17.1与#18保持完成；A17.2及当前#19/#20的A独立技术工作全部完成。A20-U1已在344be249完整下载件实际Native复验关闭，PR23/24均获A技术批准，B本轮无待修项。C原比较/报告专项与受影响指南尚未交付；A17.3正式接收及A17.4出口继续待C前置，不合并、不关闭阶段或推进下一阶段。** 逐批预期不当作完整通过记录。

## 1. 基点、输入与执行顺序

- 产品源码基点：95f411d8e37cffbde227f59c679a66ee8489d5b8。集成分支：codex/full-version-integration-20261101。#18 已合入 6bd50c867da1da4186f0446a1912cdcfbce9bf98，后续两批分别以前一批实际合入 SHA 为产品起点；当前仅正式接收第一包；#19 A技术批准待C；#20实际堆叠交付344be249并经A技术验收，图标已通过；正式集成签收等待C。
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
|A17.1 接点基线，3h|可独立开始。以固定输入逐项对照第 3 节真实类型和三批消费者；区分已有能力、已观测失败、待 B 候选的行为|在 #17 留一份接点结论：来源/字段/单位/权限/任务语义及实际缺口。已有能力直接复用；不为凑代码增加字段|已完成：40 项通过，3 项实服务检查未执行；准确范围见第 6 节|
|A17.2 必要的公共修改，5h|仅在 B 给出最小失败例，或 A 观测到必要字段缺失/契约冲突时启动。计划分配 #18 2h、#19 2h、#20 1h，可在总额内调配|小范围 PR：真实失败输入、预期、Rust/适配改动、受影响消费者、定向结果。先改源再生成，不改单一生成物；无真实缺口则记“不需修改及依据”|已完成当前三批公共/私有接点审查；沿用既有Host授权，无公共契约/生成物改动必要|
|A17.3 三批集成接收，6h|#18 → #19 → #20，每包约 2h；实际 PR、固定提交、同提交候选和必要输入齐备后进行。先看 diff/结果，再决定必要独立复验|按第 5 节记通过/失败/待验、准确覆盖与证据；原授权下审查后合并。给下一包实际已合入 SHA。C 的工具报告与 A 已验收结论分列|#18已接收；#19/#20均获A技术批准、A20-U1关闭；正式接收与按依赖合入等待C|
|A17.4 阶段汇总，4h|三批已接收，且 C 的相关候选复验已交；证据不足先停在待验，不重跑无关全套|在 #17 集中记录各包提交、公共差异、结果及未测项，更新原分工表/本 JSON；满足 FV02 出口才关闭本卡与阶段|A汇总与控制更新已完成；最终出口仍待C及正式接收/合入，不标通过|

A17.2 的 5h 不是要求必须花完；无缺口不造改动。出现超出预算的真实缺陷，在 #17 一次说明影响与必要新增范围，不靠降低预期标通过。机器编译/下载/运行等待不计为有效工作人时。

## 5. 三批接收的最小充分检查

|批次|A 核对的公共预期/实际入口|可复用及必须等待的结果|
|---|---|---|
|#18 数据查询与工艺|readCloud 接受固定 selection，Task 输出匹配；完整原始/聚合结果保留数据集、固定来源/修订、稳定游标。validateQueryPage 拒绝来源或页边界错误，不把无效新页混入已有有效页。prepareExecution 不直接发送 run；控制只对当前归属与允许动作开放。维护/替代/恢复和产出运输能贯通订单、物品、承运身份，无重复业务效果|B 提交 PF04：固定百万负载，原始/聚合各 30 次，完整结果正确性与 p95≤2s 分开记；三个工艺场景及受影响的正常停止/保存恢复。A 先复算固定输出与原始结果；只有受影响或结果矛盾才重跑对应有限路线。93 端点全厂长跑仍在 FV03|
|#19 大报告与比较|compareActualRuns 消费各 Run 自身冻结输入；执行来源独立选择，不能以当前草稿代替。可比/不可比/缺输入分别解释，缺值为未知且有原因，不补 0；缺失必要冻结输入明确失败/不可比，不重新仿真伪造。H10 prediction 可为 null，但字段必须按契约存在。规范化报告数据供四格式共用，所选 Run/来源/单位一致|B 提交固定 38,849 等待负载，Native/HTML 有意义首屏≤5s、20 次翻页/全表筛选 p95≤500ms，生成与打开分别计时；三类比较和受影响格式。A 对照真实输入、结果与导出；C 在同候选完成 #12 对应专项，未交继续待验|
|#20 Agent 与交付|MCP list_tools 来源于实际 Host capabilities 和 schema；未授权调用被拒绝，JSON 不能自报授权。授权内读取/修改、任务取消/失败、旧 Run 冻结读取与 Host 一致；结果未明按原 request/task 找回，不盲重发有副作用操作。H08 为业务只读但会登记 Task，不能要求“任何状态都不写”|B 提交实际 stdio/MCP 调用与隔离短流程、候选下载/解压/启动、保存及新进程恢复。A 查对应 Host 结果和同提交候选，按实际覆盖接收；C 只复验受影响的下载/指南路径。生产云部署、新费用、公开替换不在本卡执行|

公用回归只在相关代码被触及时启用：两个 Run 保存重开后各自冻结输入；旧历史只读；必要的离开/恢复路径。已通过的 #13、FV01 与 C 首批证据按实际范围复用。普通函数单测不能代替 Native 画面，UI 截图也不能代替数据库/Engine 结果。

## 6. A 已完成的独立基线与验证入口

2026-11-06 在本机对 a8227d3587eee67894bb172bceed8e8441e9fbb7（产品代码仍为 95f411d8）执行以下检查。新测试源码为 [cloud-shared-contract.test.mjs](../../tests/engineering/cloud-shared-contract.test.mjs)，它执行真实 cloudProduction.ts，仅替换 Native 传输，不声称完成桌面或生产云验收。

|执行范围|结果|接收范围|
|---|---|---|
|sf-domain：cloud_execution_contracts、cloud_wire_parity|8 通过|四字段身份、版本、精确时间、必填可空、设计 wire 夹具与真实 TS 消费|
|sf-commands：project_command_contracts|5 通过|真实 Rust 请求/响应、伪造授权拒绝、修订冲突及生成 TS 消费；重新导出后生成物无差异|
|sf-commands --lib host_cloud_read::tests::|7 通过、2 ignored|固定身份/来源、权限和 scope、真实 Task 登记、取消与原请求；两个既有真实服务用例未执行|
|sf-commands --lib host_execution_exports::tests::|9 通过、1 ignored|实际 Host、测试专用回环 HTTP、H10 四格式、H11 固定来源、原请求恢复、取消及提交前权限检查；保留 PG 用例未执行|
|sf-mcp-bridge --lib catalog::tests::|3 通过|实际 schema、effect 提示与 Host 能力目录；这是库目标，不把 bin 的 0 测试当通过|
|新增共享适配测试|8 通过|准备不自动 run、未知请求不重发并原身份恢复、只取消读取 Task、跨工程 Task 拒绝、历史禁控制、分页固定来源与进度、错误导出目录用途拒绝|
|Planner typecheck|通过|当前真实生成类型与 Planner 消费者兼容|

合计 40 项通过、0 失败、3 ignored。三项未执行分别依赖实际云服务、FV1 五事件私有 reader、保留 PostgreSQL reader；本次未启动这些环境。此前冻结 Run/保存重开及 Native 证据按未受影响范围复用，没有重跑或扩大结论。当前未观测到公共 DTO、注册/生成或 service 必须修复的差异，因此不增加字段、不重写运行时。#18 的百万查询与三种工艺、#19 的 PF09/比较候选、#20 的真实 stdio/候选短联验以及 C 的相关专项仍待交付，不能由本表代替。

## 7. 后续按影响选择验证与交接

命令在“软件工程”目录执行，先按实际 diff 选取，以下不是必须全跑的流水账。第 6 节已执行的命令有实际结果，其余仍只是按影响选取的入口。

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

## 8. #18 同候选 Native 补验（2026-11-09）

[A逐项验收](https://github.com/JunyeFu/openatom/issues/18#issuecomment-6731569839)与[#17汇总](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6731573974)为最新结果。候选固定 f32c6f81af87f5fee4424e171df7fd69b050addf，PR #22仍为草稿、未合入。A本轮没有修改产品源码或公共契约。

- **已通过：** Playwright CLI经CDP操作实际Tauri WebView2；普通Native查询405行9页、固定数据集/前沿405及分钟聚合405，与真实数据库一致。准备后0秒/0指令；维护A停接时B兼容执行，恢复A后A重新获派；手动停止后R110/F113/117.935秒，3命令全部关闭、0/0/0，2件到L1-SINK、4件留源区，未达到6件目标照实保留。
- **保存与恢复通过：** 真实系统目录选择器保存，PID32044正常退出，全新PID3844打开同一保存件。只读身份/R/F/T/业务事实与保存前及数据库一致，无启动按钮；原3条预测Run的12文件逐字节一致，执行画布及113条过程记录可读。
- **失败：** NATIVE-18-U1，CloudProductionPanel.tsx:581，下拉仍保留已准备R2/0秒而当前来源为R110已停止；NATIVE-18-U2，同文件:385，已停止且0/0/0后提示仍称等待收尾。两项均归B #18；按该Issue中的标准修复/明确处理后，只复验受影响路径。没有据此错误宣称数据丢失。
- **证据层级：** A独立Host/归档/停止37个父用例通过；整厂20命令用例本机未运行。B百万PF04的60组结果及p95已复算（原始47ms、聚合1172ms），未再次运行百万负载。Native未强制构造已发布指令恰好交付前的维护撤回时序，Host回归范围单列；无承运设备的短输入不代替运输链路验证。
- **材料与收尾：** [既有私有Release](https://github.com/JunyeFu/openatom/releases/tag/untagged-65a481ce993d4d2d636c)新增FV02-A18-f32c6f81-Native-20261109.zip（975,289字节、88项），含原输入、保存件、5张截图及原始结果，无凭据。原用户连接恢复，验收Native均正常退出，专属服务与数据库停止、数据保留、端口释放。

A17.3本次验收工作已完成，#18整包签收等待两项界面失败的处理与复验；A17.4继续未完成。#19/#20与C对应报告/比较专项维持排队；不推进FV03/FV04，不改变公开v2.0.1和已提交链接。


## 9. #18 最终接收 · 2026-11-09

[逐项接收](https://github.com/JunyeFu/openatom/issues/18#issuecomment-6735025505)、[A 任务汇总](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6735030634)。受测候选 `798fdcdd8c542821dcc95b158cd34b7e898400d2`；[PR #22](https://github.com/JunyeFu/openatom/pull/22) 已批准并合入 `6bd50c867da1da4186f0446a1912cdcfbce9bf98`。第 8 节保留首轮观察，本节为当前结论。

- U1：实际 Native 准备 R2 → 运行 R4 → 停止 R57，下拉摘要与所选快照一致；U2：停止反馈仅确认请求接纳，无陈旧等待收尾断言。固定旧 R2 保持只读，不回滚当前 R57 摘要。三条定向路径均通过，类型检查与既有组件检查 3/3 通过。
- 用户完成系统保存选择及正常关闭；A 核对实际保存件末快照 R57/F64/66.554 秒与数据库一致，原 3 条预测 Run 的 12 文件字节不变。该次未产生新增产品；两件真实产出、405 条查询与聚合、维护恢复、正常停止及新进程只读重开沿用未改核心链路的 f32c6f81 已接收证据，不冒称新候选全套重跑。新保存件未另做新进程重开。
- [Native 复验包](https://github.com/JunyeFu/openatom/releases/download/untagged-65a481ce993d4d2d636c/FV02-A18-798fdcdd-Native-retest-20261109.zip)：480,249 字节，含原始结果、截图、数据库末快照和保存工程，无凭据。窗口正常关闭，原云配置恢复，专属服务停止且数据保留、端口释放。

#18 关闭完成。A17.2 对本包无需公共契约或适配修改；A17.3 仅第一包完成；#19由B开工、尚未交付，A等待接收；#20排队，C #12 报告/比较专项待对应候选。A17.4 与 #17 继续未完成，不推进 FV03/FV04，不改变公开 v2.0.1 和已提交链接。


## 10. 后续实施状态同步 · 2026-11-10

[B 开工记录](https://github.com/JunyeFu/openatom/issues/19#issuecomment-6740276537)确认 #19 的基点为 `8416a03c17cc2a4b7cefc0a2b32e6aeff1c64bf5`，计划分支 `codex/fv02-b-report-comparison`。B 本人已声明开工，尚未提交 PR／同提交候选；这不是交付或 A 验收通过。

A17.2 暂无新公共接口缺口；A17.3 等待 #19 集中交付后接收，#20 排队，C 对应专项待候选，A17.4 未完成。本次仅同步状态；既有验收和各历史章节按原日期保留，下一阶段未启动。


## 独立验收 · 2026-11-11（原候选记录）

[PR #23 请求修改](https://github.com/JunyeFu/openatom/pull/23#pullrequestreview-5861540045)／[#19 完整结果](https://github.com/JunyeFu/openatom/issues/19#issuecomment-6800543881)／[#17 汇总](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6800552802)。受测提交 `64592e54`，已下载同提交普通运行包，校验与交付值一致。

- **已通过**：类型检查、Web 构建；报告库 93 项通过、1 ignored；实际 Native 三类冻结比较，81 个输入工程文件不变。报告库实际生成四格式和15张 CSV；原 Run 的38,849等待完整保留，离线 HTML 的20次全表筛选/翻页与独立CSV预期一致，反馈p95为80.206ms，标题首帧220.862ms，无外部请求。PDF仅核对93页文档的首末页。
- **失败**：本机主屏dpr=1.25下，小比较及原38,849等待报告均在生成实际画面时失败：`Ready viewport does not match the real task window`；新进程、不设置模拟视口的Playwright CDP仍复现。失败出口位于 `native_render.rs:548`，就绪尺寸生产位于 `RenderTaskSurface.tsx:39`；根因与是否由本提交引入尚未确定。比较未知原因仍含英文与内部码，另列P2。
- **待验**：修复后同提交Native四格式成功导出/文件打开、38,849全表筛选/定位/PF09及C#12对应专项。库生成结果不能代替Native出口；本轮没有整包接收、合并或公共契约变更。

A本轮独立验收已完成，A17.3第二包尚未接收。B先处理上述阻塞，#20继续排队，A17.4与下一阶段等待。测试Native正常关闭，公开正式链接未改；旧记录是历史快照，以本节为当前状态。

## 补充复验 · 2026-11-12（003b4f24历史记录）

[#19逐项结果](https://github.com/JunyeFu/openatom/issues/19#issuecomment-6817419599)／[#17汇总](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6817433210)／[复验材料](https://github.com/JunyeFu/openatom/releases/download/untagged-65a481ce993d4d2d636c/FV02-A19-003b4f24-Native-supplement-20261112.zip)。固定源码003b4f24，A独立release构建，普通Native、本机主屏实际DPR1.5；尚不是B最终同提交运行包接收。

- 通过：三类比较与中文原因；小报告9/9；38,849等待大报告7/7，生成与提交176.950秒，325页PDF仅首末页视觉核对、15CSV、HTML/JSON一致。6,657个原文件及全部原冻结Run字节不变，活动工程换代语义相同。Native与实际导出HTML各20次独立预期筛选/翻页正确，p95为93.219/114.734ms，跨页运输作业定位和导出工程新进程只读重开通过。
- 失败：A19-U1（P2），ReportPanel.tsx:33先铺限制列表，概览被压出首屏；需概览前置、完整限制按需查看，不能删明细。正式Native有意义首屏≤5秒未得到合格计时，DOM约1.976秒与HTML标题首帧都不能替代。
- 定位：os error5出口为project_store.rs:1989的MoveFileW。受控ACL拒绝可复现5，恢复权限成功；原次现场根因未证实。B原8,326/6,660材料待核对，不能用A本次6,682/6,657替签。
- 待验：原DPR1.25环境A19-F1、首屏修复与正式计时、原目录失败现场/结果附件、B最终同提交候选和C专项。A17.2无公共类型/命令/生成差异，B宿主尺寸语义差异已审查且仅在本次1.5环境验证。A未改产品源码。

本轮A独立工作完成，A17.3第二包仍未签收；PR23不合并，#20及A17.4继续等待，不推进下一阶段。两个测试Native正常退出，专属23272端口释放；公开正式链接不变。历史章节以各自日期范围解读。

## 00f82acf复验 · 2026-11-12（当前结论）

[#19详细结果](https://github.com/JunyeFu/openatom/issues/19#issuecomment-6820254635)／[#17本轮汇总](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6820255143)。A独立源码构建；按用户最新指令，后半轮以--verify-on-secondary直接原生副屏启动，未改变Windows主屏设置或缩放。以后工具验收使用当前非主显示器，不先在主屏显示后移窗。

|实际环境|Native概览首屏|20动作反馈p95|同候选功能|
|---|---:|---:|---|
|主屏150%|2.663秒|399.376毫秒|小报告9/9、大报告7/7、完整筛选/定位、新进程只读重开通过|
|副屏100%|3.553秒|259.903毫秒|小报告9/9、大报告7/7、完整筛选/定位、新进程只读重开通过|

计时从实际收到“交付文件已提交”的Host响应到两帧后概览完整可见，生成/提交88.744及103.936秒另计。等待明细首次展开前未挂载，展开后保留全部38,849等待。两份导出各325页PDF、15CSV、6,682文件，HTML/JSON一致；PDF只新看主屏候选首末页。实际离线HTML问题/范围正文首屏0.533秒、20动作p95=266.216毫秒，不声称完整结果概览首屏。Web类型/构建及视口定向3/3通过，未修改产品源码或公共契约。

A19-U1已测范围通过；原125%实际Native、B原os5现场/8,326与6,660附件、B最终同提交运行包及C专项仍待验。持久化未改，原冻结Run字节保留复用003b证据；本轮新进程身份已分别实测。A17.3第二包不签收、不合并PR23，#20与下一阶段不启动，公开正式链接不变；四个测试进程正常退出，23272已释放。


## 历史独立待办收口 · 2026-11-14（cd4104d8）

A【A17.2接点与#19/#20独立验收】（已完成）→ B【A20-U1静态图标修正】（待交付）；C【原比较/报告专项】（并行待交付）→ A【A17.3正式接收、A17.4出口】（待前置）。本节覆盖下方及前文旧日期的状态快照。

- [#19 A明细](https://github.com/JunyeFu/openatom/issues/19#issuecomment-6858809083)：95fea7cb下载组合7,044文件一致，实际Native大报告7/7；38,849等待、15 CSV、325页结构计数及HTML/JSON一致，20动作p95=90.153ms，有意义概览1.089秒。目录提交7项回归+Host1项通过。B125%及原两套8326/6660记录按工具材料范围接收，A实际125%未重跑。原os5最终提交已定位、瞬时根因未知，旧失败保留。PR23获A技术批准，C原Q4/Q5/Q25和Q8/Q23/Q24专项尚未交付。
- [#20 A明细](https://github.com/JunyeFu/openatom/issues/20#issuecomment-6858809720)：cd4104d8完整包5,733声明文件匹配；41项检查通过。真实stdio/Host/HTTP隔离服务/SDK：新peer及撤权拒绝、精确主窗口授权、维护恢复、正常停止通过。维护路线R136/F196/93.327秒、0件/1次返程；两件路线R186/F314/129.671秒、2件/2次返程，均0/0/0及发布同步、保存完成。写者15800正常退出，新进程27084完整快照和冻结输入一致，other_host且无控制权。
- A20-U1（P3，B）：标题栏PNG地址实际返回HTML616字节、naturalWidth=0。按Workbench.tsx:1182引用→Vite public→tauri frontendDist定位实际构建/嵌入闭包，B交同提交修正候选，A只补受影响资源启动验收；已通过业务闭环按影响复用。PR24请求修改，整包资源未标全通过。
- A17.2：B的Native私有授权转发仍调用既有Host grant_execution_controls，主窗口/当前已授权未撤销peer/精确执行身份边界成立；无需新增公共DTO/schema、改锁或重写生成物，A本轮未改产品源码。新49项与此前40项基线不混为重跑总数。
- [A汇总](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6858823549)。本轮单屏经用户明确允许主屏；写/读Native正常关闭，本机原云配置恢复，专属服务/隧道/容器停止且数据保留。PR23/24未合并、#17/#19/#20保持打开，C任务范围与工时不变，公开v2.0.1和提交链接不变，不启动FV03/FV04。当前325页未逐页视觉复查；旧Run及未变比较链路证据按已测范围复用。


## A20-U1关闭与A技术收口 · 2026-11-14（344be249）

A【#19/#20当前独立技术任务】（已完成）→ C【原比较/报告专项与受影响指南】（待交付）→ A【正式接收、按依赖合入与阶段签收】（待前置）。B本轮已无待修项。

- [#20新验收](https://github.com/JunyeFu/openatom/issues/20#issuecomment-6874091314)固定344be249；下载完整168831204字节候选及B结果附件，ZIP/SHA一致，5733声明文件匹配，bridge未变。实际源码仅图标构建模块、类型声明和Vite配置3文件；A检查匹配PNG/缺源/缺输出/输出不一致4项hook边界，4/4通过。
- 普通下载件Native18264实际两处图标均1254×1254且可见；图标URL为200/image/png/387774字节，实际响应与源码PNG一致。空工程弹窗打开、取消正常，仍未打开工程；已正常退出，23272释放，没有云配置变化或启动生产服务。A20-U1关闭，PR24获A技术批准，旧失败记录作为历史保留。
- 本轮未再构建Native或重做完整Web/类型检查、未重跑生产或报告矩阵。此前A在cd4104d8通过的权限/维护恢复/正常停止/两件产运/保存/新进程完整快照及输入读回复用；新4项与旧49项不冒称本候选全套重跑。
- [A最新收口](https://github.com/JunyeFu/openatom/issues/17#issuecomment-6874092071)。A当前没有可独立执行的剩余技术项；C仍缺95fea7cb的Q4/Q5/Q25比较、Q8/Q23/Q24报告及受影响指南，不恢复真人任务、不重做已接收首批。PR23/24仍未合并，#17/#19/#20保留打开，最终签收与FV03/FV04继续等待；公开v2.0.1、原下载页与提交链接不变。
