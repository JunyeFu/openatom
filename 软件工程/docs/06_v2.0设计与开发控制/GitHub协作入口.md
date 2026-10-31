# 下一完整版本：三人协作入口

2026-10-31。五个阶段和FV0首批三张Issue已建立；任务目前待认领，未绑定具体账号。后续12张任务保留设计，尚未建立Issue。任务进展在对应Issue记录。

## 从这里开始

- [完整制作与验收方案](入围后完整版制作与验收方案.md)：范围、质量标准、性能要求、阶段和分工。
- [15张任务卡](入围后完整版任务卡.md)：后续实施的输入、写范围、依赖、验证和交付。
- [FV0输入与交接说明](FV0输入与交接.md)：哪些材料能直接读取，哪些需要项目负责人交接。

|发给谁|第一张任务|现在做什么|交付到哪里|
|---|---|---|---|
|角色A：项目负责人/核心开发（你）|[#1 固定版本、范围和交接输入](https://github.com/JunyeFu/openatom/issues/1)|确定集成基点和负载，提供H1复现输入，明确范围与指标|#1结果，必要文档PR|
|角色B：另一位大规模开发者|[#2 诊断失败与性能基线](https://github.com/JunyeFu/openatom/issues/2)|先读代码和记录；H1拿到后做一次有边界的归档复现与测量|#2诊断结果，必要测量脚本PR|
|角色C：前端纠偏/真实使用同伴|[#3 首轮界面纠偏与独立使用查验](https://github.com/JunyeFu/openatom/issues/3)|用正式包和指南独立完成基础/进阶任务，记录看不懂、找不到、做不下去的位置|#3步骤结果和问题清单|

每张Issue已包含输入、操作步骤、允许范围、交付清单和接收方式。无需先阅读全部设计；C不要求写代码，B不在首卡直接重写产品。

## 五个阶段

|阶段|里程碑|出口|
|---|---|---|
|FV0 缺陷与基线|[打开FV0](https://github.com/JunyeFu/openatom/milestone/1)|事实分类、版本、输入、指标和责任明确|
|FV1 阻塞修复|[打开FV1](https://github.com/JunyeFu/openatom/milestone/2)|停止/排空、归档与链路的已复现阻塞闭合|
|FV2 完整工作流|[打开FV2](https://github.com/JunyeFu/openatom/milestone/3)|用户任务和数据、画面、报告、Agent消费者一致|
|FV3 整厂与独立使用|[打开FV3](https://github.com/JunyeFu/openatom/milestone/4)|整厂全订单、长历史、实际负载与独立使用通过|
|FV4 正式交付|[打开FV4](https://github.com/JunyeFu/openatom/milestone/5)|源码、包、指南、示例和材料版本一致，可实际接手|

## 分支和访问

规划分支：`codex/full-version-planning-20261031`，仅添加规划、任务和交接文档，继承源码提交`19d789b9520251ed2e9854e35c93b53f90227282`。这不是对下一轮所有未提交代码的认可，也不替代FV0冻结集成基点。

仓库`JunyeFu/openatom`是私有仓库；转发链接前，确认另外两人已获访问权限。当前按角色标注，没有自动邀请、分配真人账号或向他人发送消息。负责人拿到账号后可在Issue设置Assignee。

**已经提交的发行版、标签、运行包、下载页和原源码分支不在本次修改范围。** 当前正式下载仍为[超级工厂v2.0.1](https://github.com/JunyeFu/superfactory-releases/releases/tag/v2.0.1)。本次不更新发布渠道，不触发部署。

## 可直接转发给同伴

发给B：请先看[#2](https://github.com/JunyeFu/openatom/issues/2)，按步骤做失败与性能基线。先使用仓库代码和输入摘要；长历史H1由我交接后再实测。把定位依据、实际数字和未测项回复Issue，先不重写产品或启动整厂长跑。

发给C：请先看[#3](https://github.com/JunyeFu/openatom/issues/3)，下载正式包后只按指南独立操作，不需要写代码。把卡住的步骤、看不懂的文字、布局问题和实际/预期结果回复Issue，基础和进阶分别记录是否需要帮助。
