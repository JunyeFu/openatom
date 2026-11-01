<!-- scoped-verification:start -->
日常改动优先使用本任务已有专用检查；需要选范围时用[tools/dev/verify.mjs](dev/verify.mjs)。默认列计划，--run才运行；docs仅解析明确文件，node指定测试，frontend复用typecheck，rust指定包/筛选。完整S07入口仅用于工程准入，不作每轮默认检查。[工作流与测试分级](../docs/04_开发与验收/项目搭建与执行工作流.md)。
<!-- scoped-verification:end -->

# tools

P3加入本地模块检查与场景检查工具，命令以实际实现为准。

## S07实际工程入口

S07实际验证入口为`tools/s07/verify.ps1`，结果为`tools/s07/output/verification.json`。
