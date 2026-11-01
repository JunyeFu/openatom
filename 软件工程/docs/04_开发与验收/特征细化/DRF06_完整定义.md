# DRF06：生产规则、订单物料、质量与跨厂物流完整定义

2026-10-14，设计修订DRF06-2a。按本轮DRF06授权直接补齐原业务文件和设计JSON；本文件是既有P/S任务的分工投影。原17SF、21P、60S、Q01—Q64身份和首版范围保持。公共类型/Commands由本轮窗口1写，共同算法本窗已修，当前不准入产品编码。

精确字段与生产者/消费者见[完整定义JSON](DRF06_完整定义.json)；案例输入见[业务输入](../../../scenarios/真实业务设计/business-inputs.design.json)、[18个业务边界](../../../scenarios/真实业务设计/business-rule-boundaries.design.json)；设备/阶段/等待/接受实例见[8个时间轴独立预期](DRF06_时间轴独立预期.json)。这些是实现前预期，所有actual_result保留null。

## 原任务与完成口径

|原任务|本轮已补齐的可执行定义|有限预期/消费者|
|---|---|---|
|P0-02|订单/BOM/路线版本、预留与唯一holder、批次拆分/容器和产品单位接受|B01/B02/B03/B07/B08/B16，订单/库存/物品追踪|
|P1-01|每当前工序原子齐套、材料资格/FIFO、设备/人员容量，失败不扣料|B01/B05/B07/B09/B18，共同simulation|
|P1-02|串并联汇合、检验单出口、同物品有限返工及复检|并行A/Q/U、T03，阶段/质量/返工行|
|P1-03|日历/有效阶段、人员阶段占用、机器持有/输出等待、同刻闭包和部分结果|T01/T02、B09—B12/B17，TC01/TC02|
|P1-04|多单分配、优先序、提前准备/家族换单、首件epoch门槛、目的接受|T04，两方案205秒、迟延5/45秒，W-A占135/160秒|
|P1-05|hold_station/pending_store、有效预设决定、隔离/报废保持ID和空间|B04/B15、T03/T05，80秒待判/复检独立，最终2成品1报废缺交1|
|P2-02|实际装载/发出/到达/卸载接收/返程，接收阻塞、班次与跨日|T06/T08、B13/B14/B16，固定时间批次运输|

设计复审关联仍为S03-02、S06-01、S06-02、S06-05；DG01/DG02由本轮规则与独立预期支撑，共同合同写入归本轮窗口1，S04消费归窗口2。本轮文件交付不改变原任务状态、设计通过数或实际运行结果。

## 原文修复和适用六视图

|视图|落实位置与执行边界|
|---|---|
|任务/交互|订单/路线/日历/质量/物流输入→缺项检查→冻结Scope运行→看实际等待/接受→修改新方案重跑；候选优先序明确，不自动最优|
|结构/数据|唯一类型源为packages/domain/types.design.rs；完整JSON的field_mappings逐项给路径、类型、单位、来源和消费者；Host冻输入，单Worker拥有业务状态|
|状态/行为|沿ALG-01—14；原子齐套、有限返工、一次结果槽/移交/接受；下阶段等待仍保持机器/物品；质量、生命周期、分配轴独立|
|接口/协作|沿packages/commands/api.design.rs的18操作；CALL01—04给请求/返回/错误及接收者，不新增预留或运行处置API；差异见公共交接|
|界面/表现|本窗口提供业务字段和8个可核对实例；复审窗口2统筹S04视觉、原型与本人走查准备。窗口菜单控制dock，底部工具，时间轴默认160px，描述独立dock|
|部署/验证|沿现行Rust 1.97.0、std确定性队列与单Worker设计；本轮只用Python标准库核验设计算术/源引用，不安装依赖、不动锁文件、不执行产品|

### 字段、单位与版本

订单公开字段为OrderLine.product_ref/quantity/release_at_ms/due_at_ms/destination/acceptance_stage/allow_partial/priority。Quantity.value使用DecimalAmount；piece为整数coefficient、decimal_places=0，显式unit。案例的product_revision/release_ms/due_ms是draft-1设计表别名，不是另一个公开DTO。

版本分别为ModuleRef.version、SubjectRevisionRef.revision、ScenarioInput.revision、工程revision与RunId。沿现行API 0.1、工程格式1.0；DRF06-1只标本次设计修订。旧Run引用冻结工程/方案，不按新草稿同名对象重绑。时间均相对整数ms；原秒例乘1000。区间[a,b)，事件为同刻闭包后的点，日统计[d,d+86400000)。

BOM用料绑定具体consume_at_operation：CNC先用毛坯，装配才用壳体/PCB/紧固件，包装才用已放行装配体/空盒。RouteStep.device_id与operation_ref分开，同一行为模块被多设备复用不能合并执行行。

### 齐套、预留、批次与容器

soft_plan不扣库存；hard_reserved仍在原holder但排除其他任务，自己的预留还需满足版本/质量；committed归作业。当前工序全部物料、执行容量、job和第一阶段资源齐套后一次提交，失败不留部分取得且不解除已有预留。后续人员不在开工时提前要求全程连续可用。

B07的10件批次划出4件预留和6件自由量，源10件仅留转化/来源记录；失败取得保持4/6不变，不能再计源10件或再拆一次。初始硬预留和拆分来源字段已合入，H04/H05对应ALG-03与business-objects的现行规则；输入/数量不改。

装配组成件为transformed关系；箱内内容holder指容器，箱自身指外部holder。B08一箱含两件产品，以piece接受2件，箱数另为1；同阶段重复查询/移交不再履约。产品接受不消灭实物或释放库位。B16的非部分2件订单、目的容量1，无终端出库规则时只能接收1件、接受0件，第二件留车上；计算可正常结束而目标未达。

### 串并联、质量出口与处置

选料FIFO与作业候选排序分开；候选包括继续阶段、准备、返工，继承任务优先序。原并行工程三对照仍使用同一拓扑/工时/资源：A首件145秒、第二件175秒装盒；Q首件隔离且仅第二件175秒；U首件待判、第二件待检，成品0。

检验完成才取对应结果；PASS按放行条件走合格出口，FAIL按已完成返工次数走返工或隔离，UNKNOWN/null/序列耗尽待判，单件只走一个出口。返工完成才加计数，复检独立，达到上限不自动报废。UNKNOWN不生成有效质量决定；检查进入次数与决定次数分开。

hold_station维持原声明job资源至决定与成功移交；pending_store先成功接收有限库位才释放。库位满仍属原job；已结束phase独有人按声明释放，不擅自改变原无phase全作业人员占用。U例QC-01只检查90—100秒，但工位/W-Q持有到180秒；第二件150—180秒等待30秒，不能画首件仍在有效检查。

T05的80秒assembly3在pending-store且pending，assembly2在75—85秒复检；90秒预设放行assembly3，100秒明确报废assembly2，不回收组成件、不自动补产，订单缺1。原scrap_record是处置记录，实际holder保持isolation-store；报废仍占物理空间，非报废隔离数可为0。早到/对象或检查规则失配的决定不预放行未来物品（B15）。

### 阶段、日历、持有与换单

阶段策略non_preemptive需当前阶段连续可工作段；pause_resume累计共同有效工作段。全job机器/物品保持，phase辅助人员中断释放，恢复原子再取得。已被本job持有的同资源不重复扣容量。

T01：load 0—5、auto 5—35、等下料人35—40、unload 40—45秒；有效工作40秒，机器持有45秒，W-C占10秒，等待5秒。历史wait.phase_ref为unload；设备可停auto末姿态。T02：5秒开工需8秒有效工作，工作5—10/20—23秒、等待10—20秒，机器保持5—23秒，phase人员仅占两段。

T07的独立输出等待例：0—20秒有效加工，20秒形成唯一候选结果但库位满，20—40秒等待接收，40秒一次移交。机器/job占0—40秒，phase独有人员只占0—20秒；20秒PhaseCompleted不能冒充Job完成，Job.finished_at_ms在成功移交前为null、40秒才completed。等待输出不再属于某个未开始phase，wait.phase_ref=null。

一段工作可以恰在闭班b完成；t=b新取得不得借已关闭日历。需资源的零时长阶段也如此，B11在20秒闭班只能等40秒；无资源逻辑转换可同刻推进，点事件不伪造1ms条。

内部换单占设备和声明人，允许料未到先准备；multi_a_first 60—75秒准备B，75—100秒待PCB，不能全画成换单。首件门槛含设备/工艺版本/家族/准备epoch，旧放行不能解除新门槛；本例没有首件要求，不额外强加。

### 固定运输、接收与跨日

装载开始取得来源货物预留、载具及当前段人，货物仍在源；depart才改transport holder。arrival不入目的库存，卸载须人员/日历/空间，完成再次核实容量，receive一次移交到目的。载具保持至return_complete，目的接受不等载具返程。

固定发运点t映射公开departure_windows=[t,t+1ms)的装载开始机会，实际发出在load完成；不在窗外补发。allow_partial_load只控制不足运输批量，与订单allow_partial独立。首版不从画线长度/镜头/动画速度算路程，也不启用交通/维修/公用服务求解。

T06晚到PCB的第六件：360秒发出、370到达、370—420接收阻塞、420卸载接收、425返程完成。420秒PCB到料→总装取得shell004腾位→零时长卸载完成→shell006一次接收→返程，同刻闭包后缓冲为shell005/shell006。

B14到达110秒、卸载人120秒可用、120—125卸载、125接收、135返程；等卸载期间车上货物不进目的库存。B13第1日23:59:50出发，次日00:00:10到达、00:00:25接收/接受、00:00:35返回；两日接受为0/2，原订单跨日保留，午夜不造库存、不清WIP/占用。

### 同刻闭包、多原因和部分结果

当前时刻先完成/释放，再日历/到料/质量，再移交，再派工；派生零时长完成立即回处理，直到无当前事件/移交/可开工，再跳下一真实边界。B17 release/arrival/start/zero_done/transfer都在10000ms，接受1且资源释放，下一事件20000ms。

B09在0—10秒同时缺PCB和人员日历关闭，10—20秒仅日历，总等待20秒；按理由集合变化分段，多原因不加总。无需求/未释放不是等待。未知next_known_boundary为null，有边界也不保证届时开工。

B12的auto 5秒开始、需30秒：计算只到20秒时段5—20、有效15秒、computed_boundary，20—35未知；恰到35秒完成同刻闭包才有PhaseCompleted和phase_completed端。等待的解除、理由变化、计算截断、观察窗截断也必须区分，H03/H08字段已合入，当前等待/Claim每段结束与FullResult快照边界共同消费。

## 既有调用、异常与返回

CALL01：apply_commands.base_revision＋PutObject(ScenarioInput/声明对象)，成功返回EditCommit.new_revision/changed_objects/issues/history；InvalidInput/MissingReference定位字段，RevisionConflict保留草稿并定位差异，权限/能力错误不部分提交。

CALL02：run_scenario(scenario_id,input_revision,observation_window,intent=ProjectScenario)返回TaskHandle，get_task取真实TaskState/RunSummary。定义隔离周期沿既有DefinitionCycle。ComputationFailed保留输入与已知不完整结果，修问题后新Run，不静默重试。

CALL03：get_run Summary确认身份/范围，FullResult取实际阶段/claim/wait，Events按after_index/limit取至next_index=null才称完整，ObjectsAt取同Run/at_ms闭包后对象。缺历史/未计算显示未知；不从final_items补历史。现行Events按观察窗末点闭包包含，持续区间半开；期末点可从同快照FullResult或interval=null完整分页取得，按(run_id,sequence)去重；不查超计算边界的ObjectsAt。

CALL04：cancel_task的PauseCalculation/ResumeCalculation/StepNextTimestamp/Cancel沿既有allowed_controls；单步一个同刻闭包，返回CancellationState.action_accepted/terminal_won及实际TaskState。取消计算不撤订单/回滚实物；completed先胜保持completed。业务修改返回CALL01新方案重跑。

只读时间轴/描述点击引用定位同Run对象；未计算不显示名义未来工作/产量。Back恢复父布局并保留用户主动选时，切Run清掉旧绑定。字段错误返回原表单；结果读取错误保留已确认视图，迟到或异Run回复不替换。

## 本轮证据与下一接点

核验入口：[DRF06_有限预期核验.py](DRF06_有限预期核验.py)；实际结果：[静态与有限计算核验](DRF06_静态与有限计算核验.json)。核验调用既有业务verify函数仅在内存，保护原六变体及A/Q/U/440—570预期，新增边界按声明算术推导；不执行其改写源文件入口。

本窗口交付是文档、静态/有限数学预期和公共逐字段交接；真实原型、产品运行、本人走查与剩余产品运行证据仍为null。H01—H08的公共字段已由窗口1合入，共同算法/business-objects/本窗消费者已按现行合同收口；窗口2继续S04原型/走查准备。原S04/S05/S06阶段证据与编码准入保持。


## 本轮共同收口结果

[公共字段delta](DRF06_公共字段合入delta.json)现为已合入合同的字段/约束/正反例交接，不再要求嵌套占用段改型。ResourceClaim.intervals保留Vec<IntervalMs>，interval_end_causes是Vec<ClaimIntervalEndCause>，等长同序逐项配对。完整窗末仍持有/未解为observation_end，前缀/暂停/取消为computed_boundary，自然同刻released/resolved先胜。原阶段三种结束枚举保持。

H04/H05消费现行初始material_allocations与split_from_ref：同起点软/硬分配，完整源transformed、不再计现有/空间，全部子批守恒，失败不重拆。共同ALG-14已对齐窗口3的[C01 task/transport lowering](DRF03_C01_逐类原子采用.json)：Preparation/readiness不造物品，同目标/配置复用已完成准备；Transport由supplies绑定真实货物同ID，不新增Supply对象变体。

公共原13+新增7端点例共20已有Rust/标准Schema/TS证明，本窗另核验B01—B18、原6变体、8手工时间轴与8组字段/接点正反例。两者均不执行Engine/Host，manual_expected与实际运行严格分开。
