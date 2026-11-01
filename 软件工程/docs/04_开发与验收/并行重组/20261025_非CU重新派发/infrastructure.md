# 非CU路径夹具、计量采集与真实离线的完整基础接线补丁

负责人：实现窗口6；thread_id：`01a1396d-f9d8-7ac0-ae86-b93a5d28b7ac`。

范围：P0-01/NFR03/04/05/06/07/08和P2-01原共享计量所缺执行入口；完整隔离实现/测试/可应用补丁，不只再写方案或重跑30项旧归约测试。

唯一写入根：`.work/noncu-redispatch-20261025/infrastructure`。建议独立端口：27106（仅实际需要且确认未占用时使用）。

## 固定输入

共同合同：`docs/04_开发与验收/并行重组/20261025_非CU重新派发/driver-contract.json`；方法快照：`docs/04_开发与验收/并行重组/20261025_非CU重新派发/inputs/docs/04_开发与验收/无ComputerUse验收方案.json`。原包控制JSON已冻结在`docs/04_开发与验收/并行重组/20261025_非CU重新派发/inputs/docs/04_开发与验收/整包派发/`；下列docs/apps文件优先读取inputs/下的同路径副本，旧交付数据只读。

- `output/playwright/20261025-no-cu-method-pilot/receipt.json`
- `.work/parallel-closeout-20261024/measurements/reduce_measurements.py`
- `.work/parallel-closeout-20261024/measurements/traceio.py`
- `apps/planner/src-tauri/src/project_api.rs`
- `apps/planner/src/services/projectHost.ts`
- `apps/planner/src/features/factory/Workbench.tsx`

## 完整交付要求

1. 在私有镜像中实现仅测试构建/显式开启、仅本批允许根的最终路径夹具；仍经生产register_directory/SelectedFileRef/授权/Commands与真实I/O，不伪造令牌或mock生产IPC。
2. 实现调用方取得路径首次发请求到真实提交/反馈或正确可编辑资产首帧的同钟标记与采集；保留40 I/O、1800反馈、48FPS及NFR04原矩阵，不将rAF计数或隐藏窗口当帧通过。
3. 复用已有归约器，交新增采集器/运行计划/接口合同和实际生产文件的可应用最小补丁；只为新增行为做有限必要检查，不重跑未变30测试。
4. 给出可执行的进程级离线隔离入口或可复现的真实限制与精确未实现点；不能仅CDP offline就标全进程断网，不修改全局安全/隐私设置。
5. 禁止触碰主源码/共享cargo target或当前桌面；只做最小隔离编译/验证，不构建多份完整Native缓存；正式共享硬件采样统一归统筹后续执行，不能在此批私下跑出假正式组。

## 执行与收口

本轮是用户已授权的非Computer Use重新派发。只做本任务书完整范围，不创建新P编号，不降低原输入、阈值、负向或同候选要求。全部验收禁用Computer Use；本项目自有Playwright/WebView2已重新获授权。

写入只限本批output_root；需要源码修复时在其中建最小私有镜像，交补丁及对应必要验证，不修改产品主源码、原21包JSON、三层公共投影、旧交付或其他批目录。不操作共享桌面、现有Chrome、C36/共享构建缓存，不运行共享GPU或正式性能采样；不要扩大500设备素材。保留工程/Run/草稿，不自动commit/push。需要端口时先核对D:/Agent/全局端口注册表.md与实际监听，优先使用本批建议端口；未使用无需登记。

独立协议：只依赖已存在冻结输入与driver-contract.json。不存在的files/clock适配属于统筹集成工作；本批仍完整交付程序、预期、原gap映射和必要有限自检，不能等待、联系或要求其他窗口配合，也不能mock生产IPC以伪造实际通过。普通Native实际运行、正式NFR、浏览器/播放器、自检结果分别标明，程序完成不等于原包通过。

请在本窗口创建或接续一个无预算目标，持续完成本批；旧目标卡不能恢复时直接继续已授权工作，不因卡片或旧截止停止。先读必要文件，复用未变化的既有通过证据，不整仓审计、不复跑旧全集。只有真实失败才修复并做受影响必要检查。

完成必须提交：receipt.json（批状态、原task/gap、实际运行与未运行、证据引用、补丁、明确原因）、handoff.md、可执行入口/README、gap-coverage.json、原始新增结果；不以规划文档替代实现。最终回本窗口完整结果即可，无须主动消息其他窗口。
