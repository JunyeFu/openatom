# P1-03 Host历史消费者完整续作

Root/Core内部委托，继续P1-03原完整三步骤/release，以及P3-02/P3-03实际时点消费者。P1/P2 targeted已冻原样保留，不以新增Run/断言替换其原反例和业务值。本轮唯一新目标：真实同Host `get_run(objects_at)` 能在显式Job查询时返回当前或保存重开的原时点 canonical JobState，支撑实际媒体阶段/等待。无第二Engine、库存或时钟，无第19操作/API/domain变更。

## 已冻结的Engine消费签名

W01 独占 `packages/simulation/src/`，其源尚在实现，不代写：

```rust
Engine::job_history(&self) -> Vec<(SimTimeMs, JobState)>
Engine::jobs_at(&self, at: SimTimeMs) -> Option<Vec<JobState>>
sf_simulation::jobs_at_from_history(
    records: &[(SimTimeMs, JobState)],
    work: &[PhaseWorkInterval],
    window_start: SimTimeMs,
    computed_until: Option<SimTimeMs>,
    at: SimTimeMs,
) -> Option<Vec<JobState>>
```

只取真实同刻闭包后的实际Job记录；两事件间进度只按实际有效工作区间读取。None=无真实已算范围/越前沿。原Run最终jobs不能补不存在的历史。

## Core唯一写范围与完整行为

仍独占 `project_host.rs`，必要新Host历史私有helper文件与新 `packages/commands/tests/job_history_host.rs`、`scenarios/真实业务设计/runtime-job-history-host/`。其它独立测试、Engine/W01、专业stored owner文件、RootNative/服务、公共类型/API/配置锁只读。新的模块声明也只由你顺序接，不让Root并写。

1. HostRun完成时把Engine真实job_history存入现Run-owned内部StoredRunHistory；原 item records与元数据/保存搬移/精确冻结Run不变。新job记录必须明确其可用性；已有文件未保存job记录时只报具名未支持/未知，不从最终Job/新Project/当前规则补旧数据。内部存储可加必要可选记录，但不改公开Domain/Manifest角色或加任意读取路径。
2. 真实 `ObjectsAt` 支持显式 ObjectKind::Job，使用同Engine `jobs_at` 或重开后同一纯只读历史函数；按当前Run/Scope/任务授权/窗口/前沿/分页规则。**既有空objects默认读取全部RuntimeItem的行为保留**，原固定items分页/期待不弱化；显式Job或Item+Job请求按各自kind/id返回实际数据，unsupported其他kind继续具名。Root媒体将显式读取选定Job与实际Item页，不把合成夹具当Host。
3. 必要实际Host测试：多阶段实际中间时点/同刻边界、等待/日历/完成待输出、混合Item/Job过滤及分页、读不改变R/Run/库存/事件；原Run保存→搬目录→重开在无内存Engine时逐时点完全等同原读取；编辑新Project/规则不改旧记录；越前沿/错误Scope或无读取授权拒绝，旧只有Item记录明确无Job历史。使用已有同一Engine和真实公共调用，不反写旧5/30/5=45s、人员10秒或75/135等原期待。专业stored owner原targeted源码保持独立。
4. 等W01 **真正Engine冻源**、专业stored **targeted冻源**、本Host接线与targeted结束后，统一一次完整 `cargo test -p sf-commands --tests`（沿原项目有效命令），保原253和当前新增全部组/原独立期待；当前唯一数以真实结果统计，不加总跨轮。完成必要格式/自审、对应Root集成JSON和runtime后一次完整交接，Cargo交Root再构建Native并让W5独立实际UI重验。

P1/P2状态、SDK55/Engine8、专业29及历史Native249/新private单元层次分别记录；目前均不代表新二进制通过，也不标任何整包done。真正全厂IX/NFR/真人/完整release沿原门保留。
