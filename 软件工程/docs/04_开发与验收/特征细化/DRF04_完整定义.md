# DRF04 Agent、目标建厂与模板收口

2026-10-14，DRF04-3。P3-04/P4-01原身份、17SF/21P/60S/Q01—64和首版边界保持。完整六视图/字段/单位/版本/来源/算法/状态/调用/错误继续见[完整JSON](DRF04_完整定义.json)；业务源是[目标建厂](../../03_架构与契约/目标驱动厂房生成.md)、[设备生成](../../03_架构与契约/Agent设备生成契约.md)、[模板采用](../../../modules/templates/instantiation.design.json)。

本轮公共Host/API/派生类型及生成器由复审窗口1/W00唯一写；S04及docs/02由窗口2写。本窗口完成消费者、精确差异、有限预期及隔离子集试做，未修改共享源或增加产品关口通过数。

|对象|当前已落实|证据与实际边界|
|---|---|---|
|对象/字段/数量/复用授权|Host私有grant已公共合入并原生编译；全部15分支归一规则和原子拒绝|[合入定义](DRF04_Host授权合入.json)、[40例核验](DRF04_授权差异核验.json)；实际Host分派/候选编译未执行|
|stdio/Host/任务/画面|完整启动、身份、失联、取消、只读画面回传语义|[连接器](../../../integrations/agent/stdio-connector.design.json)、[原生fixture](DRF04_stdio-proof/output/native-trial-result.json)、[只读句柄](DRF04_stdio-proof/output/managed-asset-result.json)；实际Codex/产品Renderer/规范IPC附件未接线|
|时间轴/报告消费者|TC01/TC02/frontier和Events末刻已公共合入；清理旧pending，按同Run读取|execution_ref、历史blocked phase、phase_work_intervals；Events点包含end，区间仍半开；字段存在不等于Engine已产出|
|窗口2界面接点|32个现有字段组绑定，控件/来源/状态/返回/预期完整，6个状态例|[H04-04](DRF04_H04-04界面绑定.json)；未代做原型/本人走查|
|目标/采用/反馈|精确Quantity/单位/场地映射，新ID/资源/种子分离；初案单Run直交、最多3次采用修正、无变化停止|[24个独立预期](../../../scenarios/目标驱动建厂/finite-cases.design.json)；轮次按确认EditCommit计，失联不明不重发|
|生成器/投影|SKILL/YAML插入已由窗口1修复并3例隔离验证|保持合法skill头；原S/P投影归窗口1，真实准入/结果仍独立|

静态/Schema/数学/决策/字段绑定验证见[本轮核验](DRF04_静态与数学核验.json)，7个MCP请求形状和3个负例保留。复跑两组定义核验使用现有Python3.12：

```powershell
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 'docs/04_开发与验收/特征细化/DRF04_授权差异核验.py'
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 'docs/04_开发与验收/特征细化/DRF04_验证定义.py'
```

[缺口与修复](DRF04_缺口与修复.json)、[H04-01—05交接](DRF04_公共交接.json)给出当前状态。只读附件载体已由窗口1合入并完成native/标准Schema/411TS形状验证；实际Host接线仍未执行；连接器SC01—05明确规范Host/授权编译/画面/断线/Codex的可复现剩余接点。actual_result、真实Host/Codex/产品/原型/本人证据保持空；普通SDK/管道/句柄fixture结果单列。S06-05/S07-05及原任务派发条件继续适用。

本轮新增证据与准备：[14个附件身份/生命周期案例](DRF04_stdio-proof/output/attachment-lifecycle-result.json)、[C02纯选择与内存生成11例](DRF04_C02生成分支核验.json)、[S05接点准备](DRF04_S05接点准备.json)、[新增核验](DRF04_本轮新增核验.json)。已计入旧试验未重跑；32字段/6状态已逐项标定义已交付、实际原型待核。真实Host/Codex/Renderer/Tauri/本人结果仍空。
