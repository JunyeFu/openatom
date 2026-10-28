# 目标驱动建厂：设计验收入口

[验收情景](acceptance.design.json)映射SF-17及P3-04/P4-01；没有提供真实场地、机器节拍或运行结果。纸箱制造样例不同于成品装箱。完整链见[生成规格](../../docs/03_架构与契约/目标驱动厂房生成.md)。实现后使用完整标注输入实际执行GF-01至GF-07，再填写结果；当前仅设计。

GF-07专门检查初案达标直接交付，一次实际运行即可，修正RunId/比较可为空。GF-05单独检查不足时有限修正/重跑比较或不能修正说明；不把修正强加到每次成功任务。通用计量另以纸箱（束按内容换算）和外购件装配（成品件）作合同走查，不当实测。

DRF04补[24个独立有限预期](finite-cases.design.json)与[7个MCP请求形状样例](mcp-calls.design.json)：目标/单位/布局、ID与资源复用、种子、直交/3轮/无效变化、字段和文件越界、任务取消/失联、时间轴缺记录/边界。所有输入是明确自建设计条件，actual_result为空。

以`& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 'docs/04_开发与验收/特征细化/DRF04_验证定义.py'`核验绑定、JSON/skill结构、同源请求Schema及有限数学/决策定义；结果记[静态与数学核验](../../docs/04_开发与验收/特征细化/DRF04_静态与数学核验.json)。协议形状通过不等于Host权限/Engine或Codex互通；真实GF仍在原实现阶段执行。
