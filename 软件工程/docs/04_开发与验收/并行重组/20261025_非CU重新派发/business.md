# 业务字段、物品和全厂时间轴的非CU自动验收完整驱动

负责人：实现窗口8 · 业务物流独立收口；thread_id：`01a16e4f-8e3f-7122-8fb3-2ce021a38a90`。

范围：P0-02/P1-01/P1-03/P1-04/P2-05/P2-01布局/P2-04等待及报告负向消费者的剩余自动化，固定原输入，不重复已接收A/Q/U和IX全链。

唯一写入根：`.work/noncu-redispatch-20261025/business`。建议独立端口：27108（仅实际需要且确认未占用时使用）。

## 固定输入

共同合同：`docs/04_开发与验收/并行重组/20261025_非CU重新派发/driver-contract.json`；方法快照：`docs/04_开发与验收/并行重组/20261025_非CU重新派发/inputs/docs/04_开发与验收/无ComputerUse验收方案.json`。原包控制JSON已冻结在`docs/04_开发与验收/并行重组/20261025_非CU重新派发/inputs/docs/04_开发与验收/整包派发/`；下列docs/apps文件优先读取inputs/下的同路径副本，旧交付数据只读。

- `.work/parallel-closeout-20261024/business/receipt.json`
- `.work/parallel-closeout-20261024/business/native-cells.json`
- `.work/parallel-closeout-20261024/native/parallel-AQU-Native.json`
- `.work/parallel-closeout-20261024/native/IX01-Native.json`
- `.work/parallel-closeout-20261024/native/IX02-Native.json`

## 完整交付要求

1. 复用43案例/66Run/554时点预期和已接收IX/AQU/第四质量分支，列原20分区尚缺字段及精确revision/实例/预留/容器/取消需求差额；不再请求删除C原决定。
2. 交runBatch真实DOM提交+生产Host读回、新Run/旧Run冻结对照，覆盖剩余到料/日历/订单/换单和同一物品holder/quality/allocation；原未知和拒绝保持。
3. 布局/时间轴脚本覆盖剩余两画布/LOD/键盘、Escape/Alt/真实capture丢失、undo/redo/重基/跨工程及暂停循环独立性；报告负向读取共用同一真实结果。
4. 冻结矩阵并交完整程序/断言/有限新自检；文件/clock按合同注入，缺公共实现明确integration pending，禁止mock Host或等待其他窗口。

## 执行与收口

本轮是用户已授权的非Computer Use重新派发。只做本任务书完整范围，不创建新P编号，不降低原输入、阈值、负向或同候选要求。全部验收禁用Computer Use；本项目自有Playwright/WebView2已重新获授权。

写入只限本批output_root；需要源码修复时在其中建最小私有镜像，交补丁及对应必要验证，不修改产品主源码、原21包JSON、三层公共投影、旧交付或其他批目录。不操作共享桌面、现有Chrome、C36/共享构建缓存，不运行共享GPU或正式性能采样；不要扩大500设备素材。保留工程/Run/草稿，不自动commit/push。需要端口时先核对D:/Agent/全局端口注册表.md与实际监听，优先使用本批建议端口；未使用无需登记。

独立协议：只依赖已存在冻结输入与driver-contract.json。不存在的files/clock适配属于统筹集成工作；本批仍完整交付程序、预期、原gap映射和必要有限自检，不能等待、联系或要求其他窗口配合，也不能mock生产IPC以伪造实际通过。普通Native实际运行、正式NFR、浏览器/播放器、自检结果分别标明，程序完成不等于原包通过。

请在本窗口创建或接续一个无预算目标，持续完成本批；旧目标卡不能恢复时直接继续已授权工作，不因卡片或旧截止停止。先读必要文件，复用未变化的既有通过证据，不整仓审计、不复跑旧全集。只有真实失败才修复并做受影响必要检查。

完成必须提交：receipt.json（批状态、原task/gap、实际运行与未运行、证据引用、补丁、明确原因）、handoff.md、可执行入口/README、gap-coverage.json、原始新增结果；不以规划文档替代实现。最终回本窗口完整结果即可，无须主动消息其他窗口。
