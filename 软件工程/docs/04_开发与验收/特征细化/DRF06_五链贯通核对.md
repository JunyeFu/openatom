# DRF06：S06-05五链贯通业务侧核对

2026-10-14。沿原VF-01—VF-05、21P/17SF/11NFR/3IX核验；这是设计、字段接点与有限手算，不冻结S06-05、不派发编码或运行产品。精确操作/字段/状态/异常在[JSON](DRF06_五链贯通核对.json)，实际核验结果在[有限核验](DRF06_五链有限核验.json)。

## 五链最小可执行定义

### VF-01 空工程搭线/到料/等待/比较交付

原P任务：P0-01、P0-02、P1-01、P1-02、P1-03、P1-04、P1-05、P2-01、P2-03、P2-04、P5-01。原需求映射：SF-01、SF-03、SF-04、SF-05、SF-06、SF-07、SF-08、SF-09、SF-10、SF-14、SF-17；非功能约束仍由原阶段执行，不以本表关闭。

|最小输入→当前操作|公开DTO/返回|唯一状态与算法|
|---|---|---|
|NewProject.time_reference；显式Hall/Location/Device/Route/BOM/Product和ScenarioInput含初始实物/软硬分配/日历/质量预设/订单接受口径；base_revision匹配 → apply_commands|Request<ApplyCommandsParams> / EditCommand.CreateProject, PutObject → EditCommit|Host草稿唯一修订；库存未运行前仅初始声明；ALG-03/ALG-14|
|当前revision、实际引用、支持规则、显式设备/人员日历与阶段策略；几何未知保留Issue → check_factory_draft|CheckFactoryDraftParams / FactoryCheckScope → FactoryDraftCheckResult|草稿检查结果不等产能；ALG-03/ALG-04/ALG-14|
|scenario_id/input_revision/window/ProjectScenario；只取支持且齐输入scope → run_scenario → get_task → get_run|RunScenarioParams / TaskState / RunQuery.FullResult → TaskHandle → RunSummary → RunResult|单Worker冻结输入；pending预留不可领；齐套、阶段、等待、结果槽；ALG-01/ALG-03/ALG-04/ALG-07/ALG-08/ALG-12|
|比较时至少两个真实同口径Run；单Run可直接评估；报告窗口、接受Scope、冻结CostInput；选定目录 → compare_runs (optional) → export_evaluation → save_project|CompareRunsParams / ExportEvaluationParams / SaveProjectParams → RunComparison / ExportEvaluation / SavedProject|旧Run冻结，4格式同EvaluationReport，保存不改变物料；ALG-10/ALG-13|

阶段/等待/Claim：T01/T02及X01：pending等质量，phase日历中断，MC整Job/WC仅phase；输出23—40等待后Job完成；Claim两数组逐项同序。

输出/报告：AcceptanceEvent/OrderOutcome、RunEvaluation.requested_window/computed_until_ms/ResourceMetric/CostEvaluation；前缀0接受已知仅到边界，C/单位成本未知，不按完整窗口填0。

取消/失败/返回：Pause/Cancel仅计算Task，当前闭包后封口，原物料/已投入不回滚；输入缺项/RevisionConflict保草稿，ComputationFailed保已知一致结果；返回同Run已确认时点/原表单。

有限跨特征情景：X01；scenarios/从零搭建控制盒厂房/expected-timeline.design.json#baseline/delayed_pcb。X01是独立跨特征有限例，不替换原440/570秒IX-01

### VF-02 设备渐进创作与同一行为采用

原P任务：P0-03、P2-05、P3-01、P3-02、P3-03。原需求映射：SF-03、SF-04、SF-05、SF-06、SF-09、SF-10、SF-11、SF-12、SF-13、SF-15；非功能约束仍由原阶段执行，不以本表关闭。

|最小输入→当前操作|公开DTO/返回|唯一状态与算法|
|---|---|---|
|精确ModuleDefinition类别/版本；DeviceDefinition选择inline或operation_ref一种；外观/行为/资源/参数来源分开 → apply_commands / render_view|CreateDefinitionDraft, PutDefinition, SetDeclaredParameter / RenderViewParams → EditCommit / TaskHandle→RenderOutput|定义草稿不携RuntimeItem或当前库存，渲染只读同工程；ALG-14|
|明确隔离ScenarioInput seed、definition_revision、阶段/资源/目标绑定；使用共同Engine → run_scenario (DefinitionCycle) → get_run|RunIntent.DefinitionCycle / RunQuery.FullResult → TaskHandle / RunResult|JobExecutionRef.definition_cycle，无正式DeviceInstance假身份；量/工作由规则决定；ALG-01/ALG-03/ALG-04/ALG-14|
|准确版本、角色/资源/目标映射、范围、来源与权利；当前修订；不自动替换未选实例 → apply_commands / export_package / import_package|AdoptModule, PutObject(Device/Route), InstantiateTemplate / ExportPackageParams / ImportPackageParams → EditCommit / ExportPackage / ImportDraft|同一行为版本进入真实所选实例，升级不热换旧Run；模型循环不领料；ALG-14|

阶段/等待/Claim：Tasks按C01编Preparation；同Run同目标/配置只完成一次，marker留完成Job引用；phase/执行容量Claims实际绑目标，准备不计产品。

输出/报告：隔离结果与正式Run分源；升级/采用变化列清，item/claim始终由真实Run产；外观仅展示。

取消/失败/返回：取消渲染/隔离计算保定义草稿和已知结果，不撤采用文件；规则未支持拒绝用途；缺目标/资源映射定位RoleBindings；返回当前定义/采用预览。

有限跨特征情景：X02；docs/04_开发与验收/特征细化/DRF03_C01_逐类原子采用.json。

### VF-03 目标建厂初案直交与独立不足例

原P任务：P1-03、P1-04、P2-01、P2-02、P2-03、P2-04、P3-01、P3-04、P4-01、P5-01。原需求映射：SF-01、SF-02、SF-03、SF-04、SF-05、SF-07、SF-08、SF-09、SF-10、SF-11、SF-14、SF-15、SF-16、SF-17；非功能约束仍由原阶段执行，不以本表关闭。

|最小输入→当前操作|公开DTO/返回|唯一状态与算法|
|---|---|---|
|真实授权库、当前工程/草稿字段范围，精确模板/定义版本；目标/场地/接受Scope/观察窗/来源明确 → describe_capabilities / get_context / query_module_library|CapabilityCatalog / ProjectContext / QueryModuleLibraryParams → CapabilityCatalog / ModuleTemplateCandidates|声明目录不产生实际能力/库存；Agent不自造认可；ALG-14|
|ID全量映射、locations/connections/资源日历、显式seed选择；FactoryGenerationGoal和范围齐全 → apply_commands → check_factory_draft|InstantiateTemplate / TemplateSubset / proposed_objects / CheckFactoryDraftParams → EditCommit / FactoryDraftCheckResult|模板参数和目标只形成草稿/明确初始输入；unknown height不site Pass；ALG-03/ALG-04/ALG-14|
|支持规则、真正物料、固定运输日历/容量、订单目的阶段；一条正式冻结Run → run_scenario → get_run|RunScenarioParams / RunQuery.FullResult → TaskHandle / RunResult|实际cargo同ID、接收与返回分开；接受只计指定Scope；ALG-09/ALG-10/ALG-12|
|初案已达目标直接单Run报告；不足才有获授权有效改动，最多3次修正；初案不计修正轮 → initial_success: export_evaluation/save_project; shortfall: authorized edit/new run then optional compare_runs|EvaluationReport.scenario_comparison optional / CompareRunsParams → ExportEvaluation / SavedProject or explicitly scoped draft|无有效变化/关键缺参停止；旧Run/目标/价格基准不被覆盖；ALG-13/ALG-14|

阶段/等待/Claim：X02准备一次；运输load/outbound/unload/return按声明时间，carrier全程，货物接收前不进目的库存；到期接受端点包含同刻。

输出/报告：初案成功scenario_comparison=None；不强制生成比较或第二Run；前缀/未达分别注明。

取消/失败/返回：PermissionDenied/RevisionConflict不部分写；budget/缺关键输入保Undetermined草案，不改目标凑成功；取消长计算保持已知事实；返回目标/缺项/授权变更预览。

有限跨特征情景：X02；scenarios/目标驱动建厂/acceptance.design.json。source VF-03 frames coverage_only_not_required_linear_sequence

### VF-04 完整纸包装结构/过程/制造/绑定

原P任务：P0-03、P2-06、P3-01、P4-02。原需求映射：SF-02、SF-04、SF-09、SF-10、SF-11、SF-12、SF-13、SF-15、SF-16、SF-17；非功能约束仍由原阶段执行，不以本表关闭。

|最小输入→当前操作|公开DTO/返回|唯一状态与算法|
|---|---|---|
|实际产品/Job版本、几何/单位/坐标/姿态/三机制与制造/预算；缺输入对应用途Undetermined → compute_packaging → get_task|ComputePackagingParams / PackageStructureJob / PackagingResult → TaskHandle → PackagingResult|专业计算不改变生产库存，Fail/Undetermined为业务判定；ALG-14|
|Host持有实际成果、可信人的用途认可、匹配版本/payload/scope/consumer/job；Agent不能代造ack → apply_commands → adopt_artifact|RecordArtifactAcknowledgment / AdoptArtifactParams / ConsumerBinding → EditCommit / BindingCommit|成果/认可/绑定/工序Run四对象分开，改版待复核；ALG-14|
|相应用途binding已有效，生产工序/BOM/节拍/资源独立声明，不能从几何Pass推节拍 → run_scenario → export_evaluation|RunScenarioParams / RunEvaluation / EvaluationReport → TaskHandle → RunResult → ExportEvaluation|接受与物料由Engine；专业制造输出和4格式不增加库存；ALG-03/ALG-07/ALG-10/ALG-13|

阶段/等待/Claim：缺/失效binding是相应用途准入缺项，不伪造已工作阶段；支持scope按已给节拍/资源执行；专业准备或运输仍同C01账本。

输出/报告：产品/Job/成果版本、用途和专业Outcome明确；未有实际Run只design_description，不生成模拟产量。

取消/失败/返回：取消专业Task不产假Artifact；若已完成先胜保真实结果；缺ack/错版本拒绝adopt，返回专业输入/认可/消费者定位；正常Fail/Undetermined非API执行错误。

有限跨特征情景：X03；docs/04_开发与验收/S06-04_纸包装算法与数值方法复审.json。缺真实几何/机器frame/payload/认可时给具体字段与用途，不借旧截图/文档代Pass

### VF-05 跨厂/比较/接手/再修改

原P任务：P0-01、P0-02、P2-01、P2-02、P2-03、P2-04、P3-03、P5-01。原需求映射：SF-01、SF-03、SF-04、SF-05、SF-06、SF-07、SF-08、SF-09、SF-10、SF-11、SF-14、SF-17；非功能约束仍由原阶段执行，不以本表关闭。

|最小输入→当前操作|公开DTO/返回|唯一状态与算法|
|---|---|---|
|两厂同工程时间/初始库存、真实source cargo、固定装卸/去返时长/日历/接收容量、目的接受单位 → apply_commands → run_scenario → get_run|PutObject(Scenario) with SupplyRelation / TransportState / RunResult → EditCommit / TaskHandle / RunResult|source预留→depart→arrival→receive→return，原cargo/container保持ID；ALG-09/ALG-10|
|已有至少两实际Run，同基准/窗口/接受/价格可比；差异与不可比条件明示 → compare_runs → export_evaluation|CompareRunsParams / RunComparison / ExportEvaluationParams → RunComparison / ExportEvaluation|同实际snapshot四格式，不由UI页补全；ALG-13|
|Host选目录/文件；原工程输入/资源相对引用/权利/结果；打开前处理dirty/任务；修改后新Run → save_project/export_package → import_package → apply_commands → run_scenario|SaveProjectParams / ImportDraft / adopted typed project / new Scenario revision → SavedProject / ExportPackage / ImportDraft / EditCommit / TaskHandle|跨机接手设计输入，不把old final_items当新初始库存；模板插入不叠demo seed；ALG-12/ALG-14|

阶段/等待/Claim：原两厂例货物110秒接收可用、140秒carrier返回；X02的3500接收/4000返回同义；接收满/日历未开仍车上，Claim开放端明确。

输出/报告：比较/保存/重开/修改完整引用路径；重跑仍同initial input，旧结果与新工程rev分开。

取消/失败/返回：保存pre-commit取消保旧有效root、replacement失败按实际恢复/IoFailure不谎称成功；进口缺ref/权利留ImportDraft，不部分采用；返回原工程/差异/缺项。

有限跨特征情景：X02；X03；scenarios/两厂房装配包装/expected-events.design.json；docs/03_架构与契约/工程交接与打开.md。

## 仅三组跨特征情景

这些是独立manual_expected，借已有有限规则组成，不改B01—B18、原6业务预期或8个手工时间轴。

|情景|显式输入|独立精确预期|
|---|---|---|
|X01：pending预留→跨班→输出等待→前缀费用|4个pending单件全部预留给T；5秒明确放行；work需8秒pause_resume；WC日历0—10/20—30，MC日历0—30；输出库位至40秒满；50秒观察窗|5秒才原子领4件；有效段5—10/20—23，10—20等日历，23—40等输出；Job40秒完成。MC持有35秒，其中开放25秒/离班10秒，利用率25/30=5/6；人占8秒。接受40秒1件、交期40秒迟延0|
|X01前缀|暂停在10秒完整闭包后；MC 1 CNY/1000 machine_ms，WC 2 CNY/1000 person_ms，材料价格未知|进度5秒；MC段5—10=computed_boundary，人段5—10=released；刚开始等日历，没有伪造1ms等待。前缀已知接受0、C/全窗goal/单位成本null；已知费用5+10=15 CNY，完整额null。恢复到全窗已知资源小计35+16=51，材料未知仍不叫总成本|
|X02：准备一次→同cargo运输→初案直交|D1同cfg-r1准备2秒，重复请求2次；源已有3件released货物；2000ms发运，去程1000、卸载500、返程500，目标/目的容量3|只1个准备Job/2秒工作，无产出库存；cargo c1/c2/c3不换ID，3000到达、3500接收/接受、4000返回；目的接收后仍占3位。末点查询[0,3500]含接受3；完整Run1条、修正0轮、compare_runs调用0，报告comparison=null|
|X03：模板/专业绑定/接手反例|C04的12库位/18连接、height=null，既有工程seed不选；独立绑定对照给Outcome/可信ack/版本匹配|插入不加demo库存，既有pcb=7仍7；site高度不Pass。独立完整用途输入的绑定对照为true/false/false/false；缺receiver、方向错、目的库位冲突、越权、多源歧义部分预留、缺机台日历均拒绝；隔离不可普通领用|

X03的绑定条件对照与C04缺失几何分支分开：不会借一个Pass给缺输入的模板/真实机加工贴装用途。专业计算预算、真实payload和人的认可仍取原S06-04/共享合同；本轮不编造几何数据或实际认可。

## C04：12库位与18连接

当前MaterialEndpoint为port(PortRef)|location(ObjectId)，TemplateSubset.locations与nullable Hall.height_m已合入并完成原生/Schema/TS验证。下表只核对源结构及业务绑定；库位不持第二套库存。

|库位|物品/单位|容量|本轮业务用途/领用|
|---|---|---|---|
|store-blank|blank/piece|4|input；普通领用允许但仍需满足工序质量/版本|
|store-shell|shell/piece|4|buffer；普通领用允许但仍需满足工序质量/版本|
|store-bare_pcb|bare_pcb/piece|4|input；普通领用允许但仍需满足工序质量/版本|
|store-component|component/piece|16|input；普通领用允许但仍需满足工序质量/版本|
|store-mounted_pcb|mounted_pcb/piece|4|buffer；普通领用允许但仍需满足工序质量/版本|
|store-pcb|pcb/piece|4|buffer；普通领用允许但仍需满足工序质量/版本|
|store-fastener|fastener/piece|16|input；普通领用允许但仍需满足工序质量/版本|
|store-assembly|assembly/piece|4|buffer；普通领用允许但仍需满足工序质量/版本|
|store-carton|carton/piece|4|input；普通领用允许但仍需满足工序质量/版本|
|store-package|package/piece|4|output；普通领用允许但仍需满足工序质量/版本|
|store-released|assembly/piece|2|buffer；普通领用允许但仍需满足工序质量/版本|
|store-isolate|assembly/piece|2|isolation；普通领用禁止|

|源序号|原端点→原端点|精确destination_store|物料/容量|方向与单一库存动作|
|---|---|---|---|---|
|1|store-blank → CNC01.input|store-blank|blank/piece；4|位置→in Port；齐套开工才取得|
|2|CNC01.output → store-shell|store-shell|shell/piece；4|out Port→位置；候选结果一次移交|
|3|store-bare_pcb → Board01.bare_board|store-bare_pcb|bare_pcb/piece；4|位置→in Port；齐套开工才取得|
|4|store-component → Board01.component|store-component|component/piece；16|位置→in Port；齐套开工才取得|
|5|Board01.mounted → store-mounted_pcb|store-mounted_pcb|mounted_pcb/piece；4|out Port→位置；候选结果一次移交|
|6|store-mounted_pcb → BoardFinish01.input|store-mounted_pcb|mounted_pcb/piece；4|位置→in Port；齐套开工才取得|
|7|BoardFinish01.output → store-pcb|store-pcb|pcb/piece；4|out Port→位置；候选结果一次移交|
|8|store-shell → Assembly01.shell|store-shell|shell/piece；4|位置→in Port；齐套开工才取得|
|9|store-pcb → Assembly01.pcb|store-pcb|pcb/piece；4|位置→in Port；齐套开工才取得|
|10|store-fastener → Assembly01.fastener|store-fastener|fastener/piece；16|位置→in Port；齐套开工才取得|
|11|Assembly01.assembled → store-assembly|store-assembly|assembly/piece；4|out Port→位置；候选结果一次移交|
|12|store-assembly → Inspection01.input|store-assembly|assembly/piece；4|位置→in Port；齐套开工才取得|
|13|Inspection01.passed → store-released|store-released|assembly/piece；2|out Port→位置；候选结果一次移交|
|14|Inspection01.rework → store-isolate|store-isolate|assembly/piece；2|out Port→位置；候选结果一次移交|
|15|Inspection01.quarantine → store-isolate|store-isolate|assembly/piece；2|out Port→位置；候选结果一次移交|
|16|store-released → Packing01.product|store-released|assembly/piece；2|位置→in Port；齐套开工才取得|
|17|store-carton → Packing01.carton|store-carton|carton/piece；4|位置→in Port；齐套开工才取得|
|18|Packing01.packed → store-package|store-package|package/piece；4|out Port→位置；候选结果一次移交|

18连接的Port版本从原module_refs精确取0.1.0，方向/物品由实际manifest声明与参数解析，接收位置由源port_bindings明确取得，未给库位伪造module_ref。Inspection.max_reworks=0：两条拒收相关输出仍完整保留，rework端不会自动启回路；隔离空间为2，报废仍占原位。

原工人日历W-B/W-C/W-A/W-E都是0—300000ms；board-preparation是原30秒一次准备，CNC仍10/45/5、Board仍5/20/5。本轮按窗口3伴随selected_policy明确pause_resume（替换先前本窗non_preemptive选择）；真实新输入缺策略不在Engine临时猜。机台执行日历/实际机位绑定不能由人员日历或观察窗代填，可用现有ResourceInstance.calendar_id承接明确机位；未完成映射的用途保持缺项。

端点为location时destination_store必须同ID；端点为in Port时destination_store是显式接收绑定，已有port_bindings必须一致。输入连接只选料不搬料；来源不同于接收位置时需要声明运输/移交，不瞬移。数量来自BOM/工序，不给每条边默认1件；库位/边容量同单位约束，一份实物只在RuntimeItem账本出现一次。

采用全量重写Location/Port/Device/seed/资源/Task引用，source/destination/receiver及定义访问在授权scope。未知高度不site Pass，越权/缺引用/绑定冲突在Commands原子提交前拒绝，不部分采用。既有工程seed未明确选不增加原示例库存/需求。

## 已直接定明与窗口1交接

共同[算法MD](../../03_架构与契约/调度与状态转换算法.md)/[JSON](../../03_架构与契约/调度与状态转换算法.json)及[business-objects](../../../packages/domain/business-objects.design.json)、订单/班次/物流原业务消费者已写入上述规则。新载体已经合入，不重复要求Rust改型。

|精确交接|窗口1消费位置|需要采用的语义|
|---|---|---|
|F06-D01|公开操作契约apply_commands/check_factory_draft/run_scenario.conditions|端点/接收绑定/方向/物料单位/容量/全部授权引用原子准入；缺receiver/calendar定位原对象，不默认缓存/全天|
|F06-D02|全层类型/格式的C04消费者语义|locations只是声明；location端点与destination_store同ID，Port端点明确绑定；null高度不Pass；全量remap保12/18；seed须显式|
|F06-D03|窗口1协调窗口3的C04适配candidate|机台Resource/Calendar绑定、board-preparation持久Tasks/RouteStep引用、产品/BOM/Route/采用后版本明确；原30秒/工时/4人日历不改，缺项留ImportDraft|

## 本轮核验结果

25项静态/有限核验通过：原五链与21P/17SF/11NFR/3IX映射、3个独立跨特征情景、C04全部18条端点/方向/物料/接收容量及权限反例、当前Rust/API/派生消费者字段核对。原预期只读保留，未执行Engine/Host，也未冻结S06-05。

## 实质剩余与阶段边界

1. C04真实高度、设备footprint/clearance缺失：完整site/高度/布局不能Pass，nullable载体只解决保存未知。
2. C04机台执行日历/机位、原准备Task持久引用及完整产品/BOM/Route/采用后的版本映射需适配明确：缺时冻结准入阻塞对应用途，不能让编码者猜。
3. machining-job真实输入/result、placement-job真实component_library/native_job/supported_machine_profile以及可信用途认可缺失：保declared_process示例，不冒充真实专业生产。
4. 真实Engine/Host、原型、本人走查、S06-05全层关口与原验收仍取各阶段证据。本轮五链已给设计动作/状态/返回及有限核验，不因未实现停在总述，也不代填通过。

核验入口：[DRF06_五链有限核验.py](DRF06_五链有限核验.py)。原35项业务核验与公共20例仍保留其原证明范围；本轮检查不更新准入/公共状态/任务生成器，不安装依赖、不动UI、不运行产品。


C04显式绑定追加以[收口记录](DRF06_C04_显式绑定收口.json)为准：真实输入仍未回填；原25检查不重跑、不改数值，阶段策略采用当前伴随pause_resume。字段/游标/品质effects和21任务输入分型按本轮记录交窗口1。
