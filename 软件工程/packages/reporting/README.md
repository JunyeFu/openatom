# 评估文档与数据导出

当前已有Rust产品导出器与真实Host消费者；2026-10-20已接收独立修复，CSV逐Run冻结目标/前提补充已通过反例验证，组合联测进行中。原Native/正式NFR/人工与整包release仍按原P包接收。下文较早设计与试做状态保留为历史。P2-03/P2-06实现，P3-04消费共同导出操作，P5-01验收。

读取domain工程、simulation实际RunResult/比较和presentation带修订的渲染，构造一份EvaluationReport，生成PDF/离线HTML/CSV分表/JSON。这里不运行另一套仿真、不重新计算库存，也不增加第十类业务模块。

[报告规格](../../docs/03_架构与契约/评估报告与数据导出.md) · [数据契约](evaluation-report.contract.design.json) · [共同操作](../commands/command-catalog.design.json)。未运行工程只能导出设计说明，不生成假RunId或产量。

## 现行目标生成与开放主线

生成上下文与实际RunId一并形成PDF/HTML/CSV/JSON；目标、假设与接受量分别列出，带初案RunId和停止状态；初案达标不要求修正RunId/比较，修正发生时才附新RunId及差异原因，工程另存。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

实现路径与TF-04见[S01-05](../../docs/00_项目基线/技术与经济可行性.md)：PDF先试Rust printpdf的显式页面/字体/图像；HTML/CSV/JSON消费同一EvaluationReport。依赖未引入，中文/分页/场景图及另目录重开须真实验证，不假定HTML与PDF像素一致。

## S05-04具体技术选择

首版同一Rust ReportDocument选printpdf0.12.8直接PDF排版＋离线HTML/CSV/JSON；中文字体候选Source Han Sans CN2.005R/OFL1.1。真实分页/字形/嵌图和保存重开尚待试做。 [技术栈/部署](../../docs/03_架构与契约/首版技术栈与部署设计.md)。

S06-02报告结构取[公共Rust源](../domain/types.design.rs)中的EvaluationReport与[格式规范](../../docs/03_架构与契约/全层类型与工程格式.md)。同源上下文包含实际Run、等待／资源／成本／限制／比较及许可图片；无Run导出设计说明，未知成本保留未知。清单指向选定旧上下文，不重写冻结输入或把导入声明当本机运行；四格式实际产品实现仍待。

## DRF05实现接点

[契约](evaluation-report.contract.design.json)映射当前22个EvaluationReport字段，并以canonical_bindings明确15张CSV表的逐Run来源、单位、端点、结束原因及未知值。Host冻结所选Run与输入，simulation提供真实阶段/等待/Claim；reporting构造一次EvaluationReport及共享投影，四输出器只排版/序列化。`get_task`沿TaskOutput::Evaluation/ExportEvaluation，不增加Handle种类。

Engine末刻闭包计入观察窗、区间积分半开；前缀只显示已知范围。TC01/TC02/端点、逐Run归属、同源比较、容量、成本及保存恢复定义已由窗口1合入；[r3消费者复核](../../docs/04_开发与验收/特征细化/DRF05_消费者复核.json)的19个载体/union检查与精确枚举均匹配，同步剩余0。CostEvaluation.inputs/missing_conditions及4快照变体已实际进入共同源与派生物。本窗口只维护拥有者消费，公共Rust/API/生成源仍由窗口1写；docs02/实际视觉由窗口2完成。

验证入口为`python docs/04_开发与验收/特征细化/DRF05_核对独立预期.py`（原12例、9项重算、16接手定义）及`python docs/04_开发与验收/特征细化/DRF05_复核增量消费者.py`（15组正反例、当前Rust/Schema/TS只读核对）。产品Windows恢复/实际目录键并发、另一机器修改重跑仍按三P任务取证，draft_not_dispatchable不变。

## 四格式与保存有限试做

[evaluation_projection.proof.py](evaluation_projection.proof.py)用现有bundled Python/ReportLab/pypdf/PDFium及只读现有jsonschema验证一个canonical EvaluationReport，所有逐Run来源均为type_fixture，未调用Engine/Host/Commands。实际生成并读回JSON、CSV、HTML及4页PDF；69行同源字段、输入修订3/4/5与导出R8、冻结费率、CNY/USD分列、null/0、0基数百分比及仅并列分支保持一致。PDF已逐页AI图像检查。原file://拒绝保留；本轮新授权受限HTTP已在实际浏览器检查69行、窄窗口/横向滚动/页面放大/前缀/比较并留截图。只修拥有者HTML样式，其他三格式未重生；临时27855已释放，窗口2的8921保持监听。参考PDF路径为ReportLab＋系统字体，不替代选定printpdf产品路径的原验证要求。

[save_commit.reference.py](save_commit.reference.py)现有逻辑队列/8终态参考（初版6项，DQ-02补写入失败与旧根仍有效）；不调用文件替换、注入Windows故障或运行Host。[save_contact.proof.py](save_contact.proof.py)另在自建临时工程执行2项Windows文件接点：实际准备8→7/ReplaceFileW提交7→8，以及不分享DELETE句柄占用导致错误32/旧根R7保留。初版6逻辑参考、补充后的8项与原12 I/O实试分开记录；1176/1177恢复、Host目录键与16产品交接仍待实做。实际产物/命令/限制见[试做收口](../../docs/04_开发与验收/特征细化/DRF05_四格式与保存试做.md)。这些证据不关闭产品导出、跨机或本人关口。

## 2026-10-15 报告关联核验

RR-W5-01修复了有限投影对基础float使用9位有效数字提前舍入的问题：现按可往返表示序列化。合法类型样例的1 CNY/7 person_ms×135000 person_ms在JSON、CSV、HTML和4页参考PDF中保留同一FiniteNumber；69行实际生成与逐字段读回通过。它证明该反例的投影一致性，不证明真实Run计量或产品输出器完成。

本轮另在隔离Windows目录实际取得写入/替换错误2、5、32、183及一次成功替换，每次从原根或成功替换保留的旧根读回20份文档及其嵌套引用。错误2/32保留旧根属于F02，写入183/只读写入5属于F01；没有取得1176/1177，不能把这些错误或DQ-02构造恢复状态说成F03/F04真实产品恢复。逐项原始输出、缺失实现及原P任务前置见[窗口5核验交付](../../docs/04_开发与验收/特征细化/RR_窗口5_报告关联核验.json)。


当前CSV上下文按`evaluation-report.contract.design.json`的draft-5-CSV-context：逐Run input_assumptions_json保留完整冻结目标/单位/观察窗；report_generation_context_json明确为报告上层生成上下文；goal_met的未知、不适用与false分开，不查当前工程替换旧目标。15表、原列与共同业务类型不变。
