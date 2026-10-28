# P0-01 原 Q51 普通 Native 实际任务关闭验证

本窗口在 candidate08 普通 Windows 应用中通过本次限定检查：真实 B60 Run 运行时拒绝关闭、禁用工程离开入口，显式取消后到达取消终态，保存并由全新进程重开。P0-01 整包与正式 NFR-04 仍未通过。

## 实际操作与结果

- 输入是原 B60 balanced cut-r2 的独立完整副本，60 设备、3 厂房、6 线组、r2；[输入与预期](../../.work/single-window-20261017/p001-native-task/precondition.json)在运行前固定。
- PID165628 的真实任务 `desktop-host-165628:task:2` 显示 running；普通 AltF4 得到“当前操作尚未结束，请完成后再关闭”，窗口及工程保留。文件菜单新建、目录/ZIP 打开、保存、刷新及方案选择禁用；运行中的 CtrlS 未打开目录选择器。
- 依预先定义的取消分支，通过“取消计算”看到 `cancel_requested`，随后显示“任务已取消”，操作入口恢复。未强杀进程、未添加 sleep 或假任务，也未将观察耗时当作失败。
- 取消后的实际保存成功为 r2。旧输入 11 文件路径和字节不变，旧工程及全部 10 份清单文档内容相同；保存清单为 15 份，额外保留真实取消前缀及其冻结输入。原完整 Run `P201-core-164896:run:1:2` 完全不变。
- 新 Run `desktop-host-165628:run:1:2` 为 `cancelled`，`computed_until_ms=784000`、`whole_window_computed=false`、`goal_met=null`，实际前缀接受记录 2298。它不是 1200 秒完整运行通过；作业数组含预先声明作业，不能据其长度声称全部已执行。
- PID165628 保存后正常退出。准确候选路径核对后的全新 PID245328 重开并采纳同一保存目录，337 个对象、4 个精确模块、r2；实际下拉框区分 completed 的旧 Run 与 cancelled 的新 Run。选中取消 Run 后显示冻结 r2 / 784s，并将之后的时间轴标为“未计算·未知”。随后实际回看旧完整 Run，显示冻结 r2 / 1200s、155994 事件与 3000 接受记录。

## 证据与保留项

[实际结果 JSON](../../.work/single-window-20261017/p001-native-task/validation.json)、[全部旧内容比较](../../.work/single-window-20261017/p001-native-task/cancel-save-content.json)、[输入字节核对](../../.work/single-window-20261017/p001-native-task/input-preserved-after-cancel.json)及 [Native 观察记录](../../.work/single-window-20261017/p001-native-task/ui-observations.jsonl)保存具体范围。

前一次导入关闭尝试没有取得关闭生效时任务仍活动的证据，进程退出不能区分完成竞争和保护缺陷，继续记为未判定；[原记录](../../.work/single-window-20261017/p001-native-task/import-close-attempt.json)不删、不计通过。启动器一次误选旧安装路径的空白进程已正常退出，未用于本次读回证据；后续用明确 `process:` 路径启动并核对了 PID245328 的 candidate08 实际路径。

取消 Run 首次呈现时的四个可见设备标签提示“表现素材未连接”，同一次回看后续已变为“实际阶段”；换旧 Run 首次又出现该加载提示。保留给原 P2-01 核对加载状态表达，不据瞬时提示判定持久素材丢失，也未验收模型精度或视觉表现。没有产品源改动，不重复既有不受影响的构建与测试。原 F03/F04、实际 Windows1176/1177、完整并发对抗、正式 NFR 矩阵、第二台 Win11 及真人/工业认可继续开放。

下一原项为 Q10/Q11 同会话跨工程编辑历史隔离，[预先固定的输入与预期](../../.work/single-window-20261017/p001-cross-project-history/precondition.json)已准备；由本窗口继续，不派发其他窗口。
