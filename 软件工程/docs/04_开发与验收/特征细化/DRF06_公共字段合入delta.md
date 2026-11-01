# DRF06-2a：现行公共字段与生产/物流接点

当前唯一合同：[timeline_runtime_contract](../../03_架构与契约/全层类型与工程格式.json#/timeline_runtime_contract)。窗口1已合入Rust/Schema/TS，原13+新增7端点例共20；本窗消费其当前形状，Engine/Host未实现。

|字段|当前形状/约束|生产者→消费者|
|---|---|---|
|ResourceClaim.phase_ref|phase种类必须所属PhaseId；job/transport/output_hold为null，同资源job/phase不双扣|Worker资源账本→阶段资源行/报告|
|ResourceClaim.interval_end_causes|Vec<ClaimIntervalEndCause>，与现行intervals:Vec<IntervalMs>等长同序；released/observation_end/computed_boundary|Worker真实释放/快照封口→开放端/容量时间|
|WaitInterval.end_cause|condition_changed/resolved/observation_end/computed_boundary；同刻自然解除先胜、多原因并集|Worker阻断重评→等待/说明/报告|
|InitialConditions.material_allocations|同起点仅soft_plan/hard_reserved；实际引用/单位/不超量/排他；软计划不造库存|方案经Commands冻结→初始化预留/齐套|
|RuntimeItem.split_from_ref|直接源ID或null；完整源transformed留来源，全部子批守恒且源不双计|Worker原子拆批→库存/追踪/报告|
|Job完成包络|phase完成不关闭待交Job；全部结果/效果提交并释放后completed/finished_at|Worker结果槽→作业外框/完成口径|

最小正反例与显式别名见[JSON](DRF06_公共字段合入delta.json)：D06-C01阶段/原因向量约束；C02相同端点三种原因；C03初始预留反例；C04守恒和失败不重拆；C05前缀替换；C06输出包络；C07准备/readiness不造库存；C08实货运输同ID。

Tasks/Transport按[窗口3 C01](DRF03_C01_逐类原子采用.json)分派，已写共同ALG-14。这里记录已确定并实际消费的契约，不重新请求嵌套改型，不新增公开操作。手工字段intervals_ms映射到intervals并逐项配interval_end_causes，实际结果继续为空。
