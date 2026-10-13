# 评估文档与数据导出

状态：设计目录，尚无可运行导出器。P2-03/P2-06实现，P3-04消费共同导出操作，P5-01验收。

读取domain工程、simulation实际RunResult/比较和presentation带修订的渲染，构造一份EvaluationReport，生成PDF/离线HTML/CSV分表/JSON。这里不运行另一套仿真、不重新计算库存，也不增加第十类业务模块。

[报告规格](../../docs/03_架构与契约/评估报告与数据导出.md) · [数据契约](evaluation-report.contract.design.json) · [共同操作](../commands/command-catalog.design.json)。未运行工程只能导出设计说明，不生成假RunId或产量。

## 现行目标生成与开放主线

生成上下文与实际RunId一并形成PDF/HTML/CSV/JSON；目标、假设与接受量分别列出，带初案RunId和停止状态；初案达标不要求修正RunId/比较，修正发生时才附新RunId及差异原因，工程另存。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

实现路径与TF-04见[S01-05](../../docs/00_项目基线/技术与经济可行性.md)：PDF先试Rust printpdf的显式页面/字体/图像；HTML/CSV/JSON消费同一EvaluationReport。依赖未引入，中文/分页/场景图及另目录重开须真实验证，不假定HTML与PDF像素一致。
