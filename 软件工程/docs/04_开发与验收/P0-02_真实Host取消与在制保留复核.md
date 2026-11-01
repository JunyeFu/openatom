# P0-02 原步骤2—3真实 Host 取消与在制保留复核

本窗口复核当前已有真实公共 Host/唯一 Engine 消费者，并补充旧 Run 在制历史保留断言。`cargo test -p sf-commands --test production_host --locked --offline -j 1` 实际26项通过、0失败：20项生产 Host 检查和被同一测试目标纳入的6项库消费者检查。没有把原20项计数改成26项生产业务场景。

## 本次覆盖

原 `production_host.rs` 已经通过公共 `apply_commands/put_object` 接收独立取消方案，执行同一个 Engine，保存目录、搬移并重新采纳；旧记录“等待 Host/Engine 消费者”没有反映这份已有接线。本次实际运行重新确认其有限范围，没有另造库存、时钟或 wire 操作。

在原取消方案测试中增加11行断言：原60秒装配、15秒装箱输入，在30000ms的 `shell-001` 确实为 `in_process` 且由 job 持有。独立取消方案重跑后，以及保存/搬移/重开后，旧 Run 这个时点的完整 `objects_at` 页均保持相同。原测试继续检查仅去除指定未开工需求、保留另一硬预留、旧完整 Run 不变，以及共享/未分配/已投入材料要求显式重排。

[预先固定的预期](../../.work/single-window-20261017/p002-host-consumers/precondition.json)、[本次原始日志](../../.work/single-window-20261017/p002-host-consumers/production-host.log)、[限定结果 JSON](../../.work/single-window-20261017/p002-host-consumers/validation.json)与[准确测试增量](../../.work/single-window-20261017/p002-host-consumers/production-host-wip.patch)保留实际证据。改动只在集成测试中，产品行为、公共类型/API、Schema和candidate08字节没有因本次改动改变；不重复 Native 构建和无关前端测试。

## 未覆盖的原要求

这是进程内真实公共 Host/Engine 集成检查，不能替代普通 Native 业务表单、外部 Agent、正式 NFR、真人或整包验收。原步骤2的批次4+6拆分/硬预留、未来到料与当前库存分离、组成来源不重复取用，现已按[固定输入与预期](../../.work/single-window-20261017/p002-host-consumers/next-initialization-precondition.json)补齐实际Host快照和保存重开。普通Native取消新方案/显式重排/冲突及原release仍待继续。P0-02整包仍in_progress，完整actual_result和review仍为空。

## 原步骤2初始化矩阵补验

实际执行`cargo test -p sf-commands --test production_host actual_host_ --locked --offline -j 1`，6项通过、0失败、22项筛除；其中2项是新增初始化检查，4项为筛选命中的已有Host检查，未声称重跑完整28项。0ms批次10件分为硬预留4件与自由6件；5000ms库位6件、实际job在制4件；80000ms接受1件。未来PCB在89999ms仍不存在、90000ms才出现。初始组成来源PCB保持transformed，不成为额外自由库存。完整Run、指定历史页和源Scenario在Host保存搬移采纳后相同。

首轮5通过1失败：新增测试错误地期待已完成Run活动分配中仍有初始hard4 ID；既有契约在开工时转为执行分配、完成时释放。修正为检查0ms预留子件及5000ms实际执行分配引用，原固定4+6/到料/组成/重开预期不改。产品源码未改。首轮与修正日志、[验证JSON](../../.work/single-window-20261017/p002-host-consumers/initialization/validation.json)及增量均保留。

普通candidate08/PID212476已实际打开原LOCAL并运行，完整240000ms、75000ms接受1件、2件目标未达成；原Project、Scenario、6精确模块及冻结输入已按完整内容核对保存，见[Native保存核验](../../.work/single-window-20261017/p002-native-business/before-plan-content.json)。新副本scenario-99fb3935-ecdb-4987-b3f8-442dbcdd1ad6仅为本地待应用草稿，移除副本ORDER-01/TASK-LOCAL等待具体动作时确认；没有提前计为取消链路通过。

## 用户确认后的普通Native取消闭环

用户已明确确认仅移除副本scenario-99fb3935-ecdb-4987-b3f8-442dbcdd1ad6中的ORDER-01和TASK-LOCAL。原等待进程已不在，未观测到关闭/丢弃动作；本窗口用candidate13实际打开before-plan，通过新方案可编辑身份字段恢复同一副本ID，再仅移除这两条记录并原子应用r2。上述上一段等待确认状态据此结束。

从副本初始输入实际重跑产生desktop-host-215204:run:1:2，完整240000ms、0作业、0接受、目标达成状态为未知而非虚构成功，5件初始物品全部available。旧Run desktop-host-212476:run:1:2仍可查询；30秒的shell-001与pcb-001均in_process并由同一装配Job持有。

实际另存after-plan的19份清单文档均存在；原LOCAL业务值未变，只有既有原子提交规则将revision/project_revision推进到2。6模块、原Scenario支持文件及旧Run/冻结输入/完整历史共11份原文件逐字节一致；副本除身份/名称、工程修订和两组需求外，其余字段与原输入一致，包括完整5件初始输入。见[内容核验JSON](../../.work/single-window-20261017/p002-native-business/cancellation-recovery/content-validation.json)及[实际UI观察](../../.work/single-window-20261017/p002-native-business/cancellation-recovery/ui-observations.jsonl)。candidate13进程保存后正常关闭，OS已确认不在。

此次还发现画布标签仅显示质量released而省略生命周期及字段名，易混淆在制状态。现仅修改FactoryScene的一行标签，分别显示“状态 in_process”和“质量 released”；类型检查/前端及Native/桥构建均退出0，candidate14全新普通进程PID214432已读回同一保存档案，旧Run30秒标签与侧栏一致，无需求新Run240秒仍为5件可用/0作业/0接受；限定复验通过，见[修复记录](../../.work/single-window-20261017/p002-native-business/cancellation-recovery/label-repair.json)。此检查不替代显式重排/并发冲突、正式NFR及整包release。
