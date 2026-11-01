> 当前实施状态：v2已获授权并进入实施；本页为冻结设计参考，设计期“未实现/禁止实施/待v1”不再表示现行状态。原包V2-08、V2-09的进度、实际证据与剩余动作统一取[开发控制计划](开发控制计划.json)；本文用例不因代码存在而自动通过。

# v2.0 Agent与执行报告消费设计

本文为Agent与执行报告的契约及消费参考；v2实施已获授权，当前V2-08/V2-09的实际证据与剩余动作取[开发控制计划](开发控制计划.json)，不以历史设计期禁令阻塞实施。[结构化设计](Agent与执行报告消费设计.json)保留权限、字段、CSV表与设计用例，[schema](接口契约.schema.design.json)为sf.execution/2.0-draft11；组件、真实云/Host、界面及当前Native的验证范围分别记录。

## 1. 实际消费者与范围

当前MCP catalog从ApiRequest生成schema，Host.capabilities已经按实际caller.allowed_operations筛选操作。新增操作继续消费这条实际来源，不另写一份宣称可用的工具清单。Host仍逐调用授权，工具目录和注解不构成权限。

现有EvaluationReport／RunEvaluation及build_evaluation要求实际软件Run和冻结输入；render_evaluation、PDF及CSV投影也依赖此类型。执行会话采用独立ExecutionReport，由同一对象供PDF、HTML、CSV、JSON，保留旧Run和专业／成本报告的原含义。复用的是必要的精确数量／区间及输出底层，不能构造一个RunResult绕过旧校验。

当前新增界面仍限云端面板。执行页增加真实本机来源选择和“导出所选修订／截止报告”；原DeliveryWorkspace的Run选择、比较和报告流程保持。执行报告的主增量是来源、结果及状态数据表，不能把旧Run截图贴成执行证据。

## 2. Agent能看到和调用什么

|操作|普通Agent消费|实际条件|
|---|---|---|
|H01 start_execution_session|准备Task|获准的配置／启动范围；prepared不等于开工|
|H02 get_execution_snapshot|本机实际快照|具体Project／会话读取权限；归档数据不恢复控制权|
|H03 control_execution_session|有限控制|动作、设备及expected_execution_revision均须实际授权|
|H08 read_cloud_production|明确云只读选择及Task|读取范围和实际云配置；不导出云凭据|
|H09 read_execution_view|本机inputs／observations／history／request／catalog|读取、原Task所有权及具体Project范围|
|H10 export_execution_report|真实导出Task|报告读取权限、导出操作及本次真实目录选择|
|H11 export_cloud_data|固定选择的完整数据导出Task|云读取、数据导出操作及本次真实目录选择|

H04—H07仅供已认证网关。普通外部会话的授权更新不能把ExternalAgent变为Gateway或UI；实际Native管理入口登记网关身份和精确设备／epoch范围。CallerOrigin、grant和OS认证不来自请求JSON。不能仅凭Host已经实现网关操作，就把它们放进普通Agent的tools/list。

旧Agent建厂、修改、软件Run流程继续使用workflow.run_factory。拟新增execution_workflow.py只消费实际新能力，不把虚拟Run的修正循环套在活跃设备上。必要读取／控制／导出操作与Task读取权限在获准配置中明确列出，不因依赖关系自动升级授权。

### 2.1 MCP效果提示与实际协议

|新操作|readOnly|destructive|idempotent|openWorld|
|---|---|---|---|---|
|H01|false|false|false|true|
|H02|true|false|true|false|
|H03|false|true|false|true|
|H08|false|false|false|true|
|H09|true|false|true|false|
|H10|false|true|false|true|
|H11|false|false|false|true|

本表是依据本系统副作用作出的设计映射。H08读取业务数据，但会注册真实Task，所以不宣称整个调用无状态副作用；其业务权限仍是读取。H03具有外部生产效果，H10写实际选择目录；存在云读取的导出按openWorld=true描述。提示只是客户端信息，授权及实际行为仍由Host处理，不据此新增一轮批准门槛。[MCP ToolAnnotations原始定义](https://modelcontextprotocol.io/specification/2025-11-25/schema#toolannotations)

当前协议继续返回产品TaskHandle并调用superfactory_get_task。产品Task不是MCP规范的原生tasks能力，本轮不升级MCP协议或声称已经实现它。大JSON继续走现有实际managed结果解码，匹配operation的生成schema、请求ID和上下文，不能截断成成功摘要。[MCP工具执行元数据](https://modelcontextprotocol.io/specification/2025-11-25/schema#toolexecution)

## 3. 原请求查询与有限观察流程

ProjectContext.tasks只列TaskHandle，没有原request_id；仅按种类／修订匹配可能选错历史Task。新H01／H08／H10在登记Task时，同一Host短事务插入原请求索引。键是实际Native Host上下文、实际调用者、Project和request_id，不接受请求中的caller字段替换。

同键、同operation和typed params返回原Task；不同params以InvalidInput定位request_id，保留原工作。get_task的TaskState增加可选originating_request_id，旧Task保持省略，新v2任务必须填并供消费者核对。该索引只对当前实际Host上下文有效，不声称跨重启恰好一次。

H09(request)返回ExecutionRequestStatus；必须通过现有owned_task／scope规则。not_found不证明原网络调用永远不会到达，不能据此自动另发生产。Host上下文变化时停止这段观察，转原epoch核对，不恢复旧授权或造新身份继续。

Agent的有限流程为：

1. 读取真实get_context和capabilities，记住Host／Project及本次已授权任务。
2. H09(catalog)发现本机实际保存／当前执行记录；目录只负责发现，操作前固定具体identity及revision。
3. H01发一次并记住request_id；丢响应查原请求，拿到Task后校验ID、scope、kind、input_revision、operation及originating_request_id。
4. 以H02／H09读取同源同修订；H08明确选择云来源。授权控制逐次比较执行修订，不重算接受量。
5. 演示脚本显式给观察预算和节奏。预算耗尽只返回“尚待观察／核对”，不自动stop、重开epoch或宣布目标失败。
6. 把当前／截止选择解析为精确R，取得真实选定目录，再H10导出并观察真实Task终态。

取消数据观察或导出Task不取消生产。准备Task完成、设备Completed、接受目标、业务结清、云同步和文件导出完成分别判断。

### 3.1 Agent读取DaaS的有限流程

H08(data_query)仍返回实际cloud_read Task；先核对原operation/request/scope/kind和输出，再核对telemetry的cloud_project/dataset、首响应selected_visible_frontier及原筛选。随后页保持原page limit/cursor和同一F；结果行型须对应请求aggregation，不能只因通用联合可解析就采纳。data_quality_issues及来源一并保留。execution查询仍按其原identity和固定执行修订消费。

未来L56 execution_workflow.py显式接收观察时间/页数预算。当前workflow.py:27的wait_task是无限循环，不直接充当新的有限消费者。预算耗尽只结束观察：返回实际已观察页数/行数、dataset/F、未完整状态和待取cursor；不声称全量统计，不停止生产。从中途cursor恢复而缺前段消费记录时，到末页也只证明所观察后段完整。

完整分钟聚合只有全部组页实际取得后才可汇总，原始页数或当前页计数不推算总数。百万逐行正确性及60项计量由L31专用验证工具消费，不能把百万原始行全部塞入模型上下文。这些覆盖说明属于消费者本地结果，不新增Host操作、线上根类型或数据库记录。H08 Task completed只表示本页完成，与完整筛选、生产成功和导出成功分别判断。

## 4. 固定历史前缀与报告源

H09增加history和catalog选择；H08(history)经C18查询固定R／F的状态记录。ExecutionHistoryQuery的两个时间端点均包含点记录，按record_sequence顺序分页；空对象选择返回该范围全部类别，cursor固定原选择。它不是任意SQL，也不授予设备或网关写权限。

LocalExecutionCatalog从实际本机执行注册表／归档产生ExecutionSourceSummary，列出身份、可读修订、输入引用、观察前沿、状态和实际active_owner。活动slot释放后仍可发现历史记录；目录不恢复Engine或控制权。目录是动态发现，报告仍固定明确源，不把目录项当完整前缀证明。

H10请求只携带当前授权Project修订、1—2个不同执行身份的明确host／cloud修订源（重复session／epoch拒绝）、格式及SelectedFileRef，不能上传自填的接受量或报告正文。Host在受管worker中取得：

|内容|来源与约束|
|---|---|
|Snapshot R|精确源及修订；冻结read_origin及当时同步元数据|
|输入|实际ExecutionFrozenInputs；与Snapshot.input_ref及绑定一致|
|历史|完整必要前缀到F；记录修订／时间不能超出R覆盖，不跳页|
|来源章|原identity、R／F、模型版本、声明范围、观察与截止状态|
|指标|只读上述事实及精确算法，不运行Engine补结果|

本机读取无云I/O；显式云源经C07／C16／C18固定读取。缺少必要输入、快照或历史前缀，导出不能宣称完整成功。实际业务前缀可以未达到目标或尚有未知事项，报告须原样呈现，不因此重跑生产。

## 5. 指标和比较的具体口径

|指标|定义|未知条件|
|---|---|---|
|口径接受量|按冻结目标／订单口径及实际接受事件精确累计Quantity，保留单位|沿用旧逻辑对重叠产品口径的歧义限制，不能补0|
|达到目标时间|首次实际接受前缀达到固定目标的Host毫秒|时钟未启动、目标未证实或口径不明时为空；命令数不是数量|
|命令计数|尝试、Started、设备Completed、已接受逻辑工作、未知命令分别计数|计数不能代替合格接受量或生产目标|
|等待|按逻辑工作状态区间分别累计queued、预开始确认、output_wait、reconciliation_required|未启动时钟或缺依据为空；同毫秒变化不多加1ms|
|资源|确认占用区间并集时长、按units累计unit-ms；都裁到已观察前沿|物理有效工作保持独立来源／缺值，不由占用或名义工时补齐|
|两次观测|候选减基线；逐条件显示输入／目标／时钟／设备变化／结清和覆盖关系|不满足指标条件只并列，不伪填改善量|

ProjectedResourceClaim用open_interval_index精确指出开放段（或null）；不能把整条多区间claim都标为开放。CSV的is_open逐段派生，view_boundary是current_frontier／report_cutoff／actual_release枚举，不是毫秒。接纳修订另名admission_execution_revision，避免覆盖CSV共同前缀中的报告修订；素材权利按module_ref限定，防止同名asset id串属。

数量计算沿用精确十进制系数与小数位，不走浮点累加。区间只按实际长度积分。每个数值及缺项理由在四格式共享，不能用HTML显示值覆盖JSON原值。

完成时间差允许比较同冻结输入、同固定目标／时钟策略且各自目标及结清均有证据的两次观测；早结束不补满报告窗。接受量差还要求相同观察截止和足够覆盖，否则单列各自数量，不给窗口差值。数量差用direction＋非负Quantity magnitude，避免给现有SafeUint系数塞负数。比较属于本次软件在环观测，不产生工业因果验证结论。

## 6. 一个ExecutionReport与四格式输出

ExecutionReport包含报告身份、导出时当前Project修订、生成UTC元数据、各源stamp／snapshot／history／metrics、条件比较及限制。冻结输入修订分别保留在各源，导出修订不能覆盖它们。UTC生成时间不参与Host经过时间计算。调用者权限、token、选择凭据、绝对本机路径及IPC句柄不进入报告。

PDF／HTML／CSV／JSON从同一对象生成。计划输出report.pdf、report.html、report.json及csv目录；所需已有字体／许可证资源随实际输出方法处理。缺资源或写入失败不冒充四格式完成。

执行报告PDF正文呈现业务来源、时钟与数量口径、结论、比较条件及未知原因、关键指标和17张明细表索引。完整canonical JSON以标准PDF内嵌文件report.json（application/json）保存在同一PDF中，与同包report.json逐字节一致；不逐行打印为正文附录，不截断历史或缺失条件。独立保存PDF后仍能提取完整原报告，正文说明附件入口、同包JSON/CSV和字体许可位置。验收实际提取附件与原对象精确核对，并逐页查看正文。HTML／CSV／JSON合同及原未知／可比性语义不变；此规则仅适用于ExecutionReport，既有EvaluationReport规格不变。

PDF/HTML正文优先显示原工程/方案标题、原时间与中文状态、条件及未知原因；长身份、上下文和原始对象保留在技术详情及完整附件。作业按原数据稳定编号，同一作业在预测与执行中沿用同一编号；17张HTML表及JSON/CSV机器数据保持原值。来源只映射正式host/cloud；生命周期按prepared（已准备）、running（执行中）、dispatch_paused（暂停派发）、draining（等待在制任务结束）、needs_resolution（待处理）、completed（已完成）、stopped（已停止）、failed（失败）呈现，不添加不存在的local/paused/created值。空差值显示“暂无可比结果”，未知不补零。

CSV有17组明确表，见结构化设计。尝试行保留target_device_boot_id以对应原Command；逻辑工作行该列为null，通过精确attempt行查询，不用设备当前boot代替历史目标。来源表达式用于设计说明，运行时使用对应的有类型表生产者，不eval这些字符串。会话表统一带report_id、Project、session／epoch、R／F；新增3个预测表用实际Run及目标执行键，不给Run编造session；比较表分别带基线／候选的session／epoch／R／F；嵌套状态以typed JSON单元格保留。未知数值单元格为空，配known／原因字段；JSON保留null，PDF／HTML标明未知及原因。源章、数量／命令、逻辑工作、接纳、资源、工时、等待、状态历史、模型／权利、缺项及比较不能各用不同修订。

H10使用真实FilePurpose.ExportExecutionReport的目录选择。旧ExportEvaluation目的、其他调用者选择或任意路径不能借用。Task采用现有exporting种类、新ExecutionReport结果分支及原子文件发布；文件选择、任务归属和取消消费现有实际Host规则。

ExportExecutionReportResult只返回实际报告FileRef、源stamp、文件表和限制，完整对象在report.json中，避免get_task重复携带整段历史。Task completed表示文件提交完成，生产目标仍以其固定Snapshot为准。

### 6.1 数据查询证明与预测／执行报告缺口

selected_visible_frontier是dataset ingress前沿；执行报告中的observation_frontier是Host状态历史前沿，两者都曾简写F，但不能互换。四格式继续消费同一ExecutionReport，百万生成量、遥测条数和Q03计量不填入生产接受量或执行目标结论。报告生成UTC元数据也不证明数据端到端延迟。[数据规模设计](数据查询与规模验收设计.md)具有自己的原始输入和实际计量出口。

T8-A07原缺口现由[预测与执行比较设计](预测与执行比较设计.md)细化：H10显式选择真实终态Run和已选执行源，canonical新增独立prediction_comparison，回执返回实际link；执行间比较保持独立。输入内容、模型、目标及观察范围按指标判断，未知不补零；实际Host／类型生成／四格式消费尚未验证，G-DESIGN仍未满足。

## 7. 文件位置与接入方法

|接点|未来改动|
|---|---|
|project_host.rs 1001／1019／2177／2452／2918行|新Task原请求索引、header传递、真实角色能力与本机来源目录|
|host_authority.rs 21／30／38行|普通外部会话不能成为受信网关，消费原所有权／scope|
|mcp catalog.rs 43／78／104行及decode.rs 37／83／99行|实际schema、效果注解及managed原响应／schema匹配|
|integrations/agent/workflow.py 27／74／171行|保留旧Run流程；拟新增execution_workflow.py有限消费|
|commands/api.rs 485／539／557／635行|新操作／文件目的／结果、可选原请求ID；复用格式枚举，保留旧导出参数|
|host_evaluation.rs 170／513／560行|复用真实文件选择和发布经验，新执行生产者单独实现|
|reporting/evaluation.rs 15／53／552／717行|必要精确计算／区间复用，旧Run检查不放宽|
|evaluation_output.rs 97／114行及evaluation_pdf.rs 178行|按新对象生成四格式，只共享必要低层绘制／转义／字体方法|
|DeliveryWorkspace／ReportPanel／useDeliveryTask|保留Run消费，云执行页接新输出种类、实际选择与Task结果|

拟新增host_execution_report.rs、execution_evaluation.rs、execution_output.rs及执行CSV契约；它们当前只在控制计划定位，不创建于产品目录。Rust实际类型仍为生成源，schema设计稿不覆盖generated文件。

## 8. 设计核对与下一接点

AR01—20均未执行；当前全部行为设计预期为160项，本轮只补已有案例的查询消费口径，不新增通过。预测来源契约现见专项设计；实际验证包含MCP权限及大结果、原请求查询、完整历史分页、真实目录／取消／原子提交和四格式字段一致；不能用设计JSON可解析代替这些行为。

本轮只完成设计与静态核对。剩余重点是设备进程／网关的生命周期与协议消费者、实际PaaS／版本／预算、v1最终工程／模型身份，以及实际生成契约／运行／输出检查。v1仍按原验收计划处理，v2实施继续禁止。

设备状态历史保留V2-A10原probe及期限；报告和普通Agent消费固定R/F/T的Host结论，不发探测或按新读取时间续期。正在等待新probe与上一确认仍在原有效期内可以同时存在，具体见[观测有效期设计](设备进程网关与部署设计.md#41-v2-a10运行后探测与有效期)。

H01/H08/H10/H11返回和get_task新任务的严格wire视图在[类型与契约设计](类型归属与契约样例设计.md)落实；必填可空字段须在Option吞掉缺省前验证，原请求operation与Task kind的对应同时消费。原Task注册表和实际调用者权限保持唯一来源。

H11的数据导出与H10报告共享exporting handle，按各自原operation/请求和结果分支区分；新增hint说明Task注册、云读与新文件写入。真实目录目的、有限观察及逐页文件交付见[筛选数据导出设计](筛选数据导出设计.md)，不把百万行搬入模型上下文。
