# DRF06-2a：共同算法、业务消费者与合入复核

2026-10-14。本轮共同算法/business-objects由窗口6直接修；共同Rust/API/派生合同窗口1唯一写；S04-02/04/05归窗口2。原P任务、17SF/21P/60S/Q01—64及产品准入不变。

## 已实际落地

共同[调度算法](../../03_架构与契约/调度与状态转换算法.json)、同名MD、[阶段资源消费](../../03_架构与契约/全厂时间轴与阶段资源设计.md)、[business-objects](../../../packages/domain/business-objects.design.json)和4份原业务MD已同步：材料FIFO与返工作业优先序、质量早到/错版本不应用、Job输出待交包络、发运点/卸载日历/跨日、报废holder/容量、预留与拆分、同刻闭包及部分结果。

## H01—H08当前状态

|交接|当前公共合同/本窗消费|实际证据边界|
|---|---|---|
|H01|TC01已合入；Claim.phase_ref和interval_end_causes已合入，intervals保留原Vec<IntervalMs>；Job只在全部声明结果/效果提交及释放后完成|T01的40/45/10秒、T07的20/40秒保持|
|H02|TC02 phase历史及PhaseCompleted绑定已合入|真实有效段/零点来自未来Engine，当前手工例不代填|
|H03|Wait.end_cause已合入；condition_changed/resolved/observation_end/computed_boundary；Claim每段released/observation_end/computed_boundary|原因不从end_ms猜；相同端点的正反例见D06-C02|
|H04|InitialConditions.material_allocations已合入；软/硬同起点分配，引用/单位/不超量/排他规范化|B01/B07，失败保留既有预留|
|H05|RuntimeItem.split_from_ref已合入；完整源transformed留来源，全部子批和=源量、源不再计现有/空间|B07的10=4+6、B08的箱内容2件保持；公共规范同措辞由窗口1同步|
|H06|本窗已实际写共同ALG-02/03/04/07/08/09/10/11/12/14及业务源|原所有独立expected数值保持|
|H07|现行get_run同快照/观察窗末点closed membership、interval=null仅去时间过滤；Scope/分页保持|本窗读法已对齐，不请求未计算ObjectsAt|
|H08|RunResult.computed_until_ms已合入；ALG-12闭包后控制确认/同快照封口|完整末窗observation_end，前缀/暂停/取消computed_boundary，真实自然结束先胜|

ResourceClaim采用现行平行数组：intervals[i]只对应interval_end_causes[i]，必须等长同序。嵌套段改型已撤下；旧手工intervals_ms显式映射，不改数值，也不补默认原因。最小字段/约束/生产者消费者/8组正反例见[delta JSON](DRF06_公共字段合入delta.json)。

## C01接点

已对齐窗口3的[逐类原子采用](DRF03_C01_逐类原子采用.json)：RouteStep.operation_ref按类别分Tasks/Operations。Tasks编Preparation阶段，显式解析RoleBindings.targets与资源；本Run同目标/配置实际完成才派生readiness并留完成Job引用，重复请求不重复资源/工作段/库存。维修/建设和未支持TaskKind仍后置。

Transport由ScenarioInput.supplies解析Transport payload，使用实际货物同ID；编辑仍PutObject(Scenario)，不新增Supply对象变体或由日目标生成库存。ALG-14及business-objects已写入，D06-C07/C08提供有限预期。

## 复核与剩余门槛

公共合入原13+新增7端点例共20通过，源为当前唯一[公共合同](../../03_架构与契约/全层类型与工程格式.json#/timeline_runtime_contract)及[有限类型证明](../../../tools/type-contract-proof/output/timeline-contract-proof-result.json)。本窗核验见[静态与有限计算结果](DRF06_静态与有限计算核验.json)，入口[DRF06_有限预期核验.py](DRF06_有限预期核验.py)。

本轮不写共同Rust/API、派生输出、docs/02、准入/公共状态或源生成器，不启动产品/依赖/发布，不改tools/3d-proof。8个时间轴例保持manual_expected；Engine/Host、原型和本人证据仍空，公共字段合入与本窗设计收口不等于运行验收。
