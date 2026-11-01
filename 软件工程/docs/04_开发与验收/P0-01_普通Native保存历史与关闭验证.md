# P0-01 普通 Native 保存、搬移、历史与关闭验证

2026-10-17，由实现窗口1独自实施、验证和复核，未派发其他窗口。本轮请求的四项验证通过；P0-01整包仍为`in_progress`，`whole_task_done=false`。实际结果见[验证JSON](../../.work/single-window-20261017/p001-native/validation.json)。

候选为[portable-candidate-07](../../.work/whole-packages/p001-20261017/portable-candidate-07/portable-build.json)，内嵌前端与Native构建均退出0。使用普通Windows窗口、真实目录选择器和编辑界面，没有以隐藏烟测、Browser或Core结果替代操作。准确链接前沿见候选的`build-proof/compiler-frontier.json`。

## 缺陷及修复

candidate05在设备字段未应用时，Alt+F4及标题栏X均直接退出。candidate06的Native事件拦截实验仍复现，已撤回该实验代码并保留失败候选。

已安装的`tauri-plugin-dialog 2.8.2/src/init-iife.js`将`window.confirm`替换为异步函数；原关闭服务将Promise当成“同意关闭”，且主窗口未授予消息对话框权限。最终修复仅涉及`apps/planner/src/services/windowClose.ts`和`apps/planner/src-tauri/capabilities/main-window.json`：显式等待Native确认结果，允许主窗口消息对话框，并合并确认期间的重复关闭请求。公共业务接口、类型、Schema和持久化实现未变。

`node --test tools/p0-01-host/window-close.test.mjs`实际9项通过、0失败，覆盖异步取消、未返回结果、对话框权限失败、活动任务、确认期间修订/编辑上下文变化、重复请求及干净关闭。原始输出见[测试日志](../../.work/single-window-20261017/p001-native/window-close-test.log)。

## 普通窗口实际结果

输入为既有完整类型化工程r2：2厂房、4设备、2线组、3物料连接、1信号连接、1成果连接及九类精确模块绑定。专业结果保持Undetermined，原始输入不改写。

| 验证 | 实际结果 |
| --- | --- |
| 未应用输入保护 | D1名称输入A后按Alt+F4，实际弹出“关闭工程”；选择“返回编辑”，A值与未应用状态保留。 |
| 手动保存 | 实际Ctrl+S及目录选择保存A/r3，未保存标记清除；工程载荷和清单所列20份文档核验通过。 |
| 会话历史 | 撤销得到原名/r4，重做A/r5，再撤销到原名/r6；此时重做可用。提交新B/r7后重做禁用。撤销时磁盘仍保留A/r3。 |
| 已提交修订保护 | 未保存B/r7时点击标题栏X，出现确认；“返回编辑”保留r7。保存后PID234332通过普通关闭退出，未强杀。 |
| 搬移与新进程 | 在已核实的独立测试目录内，将`saved-original`整体搬到`moved-project`；23个文件齐全，旧路径不存在。新PID139376通过实际选择器打开B/r7，对象与连接齐全；撤销、重做均禁用。 |
| 重开后继续编辑 | 在新进程中修改为C/r8并保存到搬移目录；20份引用文档核验通过，未保存标记清除，随后正常退出，未强杀。 |

A为`Native 保存验证 D1-A`，B为`Native 保存验证 D1-B`，C为`Native 搬移重开验证 D1-C`。磁盘核对使用`tools/p0-01-host/check-native-saved.py`，比较整个类型化工程，仅允许明确的D1名称与新修订、输入预先声明的人工认可缺项提示，以及每次编辑应更新的方案修订引用；其余引用文档逐项等于原输入。

[25条实际界面文本观察](../../.work/single-window-20261017/p001-native/ui-observations.jsonl)、[搬移及进程记录](../../.work/single-window-20261017/p001-native/move-and-process.json)、[最终磁盘核验](../../.work/single-window-20261017/p001-native/save-C-r8.json)保留原始结果。测试工程保留于`.work/single-window-20261017/p001-native/moved-project`。

## 验收边界

上述candidate07链路实际覆盖本地未应用输入、Host未保存修订的取消，以及保存后正常退出。后续candidate08已完成限定Q52异常组、F01/F02和真实Run活动关闭/离开保护，详见各独立记录；本节旧覆盖范围不提升为整个Q51/NFR通过。导入关闭重叠、完整并发对抗、F03/F04及实际Windows1176/1177、其余正式NFR、第二台Win11、真实外部Agent写入和真人/工业接受继续保留。

## candidate08同进程跨工程历史隔离（2026-10-17）

PID245328中，B60/r2仅修改L1-D01名称为`Native 跨工程历史验证 L1-D01`并实际保存r3；切换前撤销可用、重做禁用。通过“当前已保存，继续”采纳P1/r9，撤销/重做均禁用，CtrlZ/CtrlY未改修订或名称；随后另存快照，整个类型化工程及20文档一致。再按已保存路径回到B60/r3，名称保留、两个历史入口均禁用；15份原清单文档按预期完整比较，两份冻结Run内容不变。见[本轮JSON](../../.work/single-window-20261017/p001-cross-project-history/validation.json)、[B60内容](../../.work/single-window-20261017/p001-cross-project-history/B60-r3-save-content.json)及[P1内容](../../.work/single-window-20261017/p001-cross-project-history/P1-full-content-after-shortcuts.json)。

同一已保存B60/r3的导入关闭补充尝试，进程随后退出，但未取得关闭生效时的活动状态证据，仍[记为未判定](../../.work/single-window-20261017/p001-native-task/import-close-B60r3-attempt.json)，没有强杀或以此关闭原缺口。当前无活动Native；Vite保留。产品源未因本轮历史检查改变，不重复构建。
