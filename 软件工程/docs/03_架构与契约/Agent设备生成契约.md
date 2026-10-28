# Agent在平台内生成原生设备

<!-- equipment-model-quality:start -->
## 设备建模共同基准

详细设备统一按[八设备同等级建模规范](../02_交互设计/设备建模精度与表现验收规范.md)：完整背底、关键机构与装配连接、同等级材质细部、命名活动部件及实际阶段动作。设备详情和流水线详细档一致；简化/区域LOD、插图接入或方块教学样例不降低详细档要求。面数/节点数不是独立通过条件，资产认可和产品实测仍据真实结果。
<!-- equipment-model-quality:end -->

状态：P3-04设备创作契约。当前已存在公共Commands、MCP桥和原生接入实现；某次连接可用的操作仍以实际Host能力目录、授权与提供者为准，不从本文件推定全部接收通过。下表早期设备能力名称是设计标识，不是另外开放的同名工具。目标是根据用户描述、合法参考图与当前渲染反馈，生成能在本平台继续编辑和复用的设备包。

2026-10-13首版已选择外部Agent＋统一Rust操作API＋本地MCP stdio桥，并扩展到工厂草案及真实等待解释。公共入口以[通用接入](通用Agent接入与专业软件连接器.md)与[操作契约](首版类型格式与操作契约.md)为准；下文describe_editor等为设备编辑能力标识，由共同操作封装，不各自建立服务或独立工程。

## 用户入口和过程

设备编辑器提供“让Agent生成／修改”入口。用户选择当前设备草稿，描述要加工什么、设备外形、输入输出和操作过程，可指定参考图。Agent先读取当前草稿与已支持能力，确认哪些信息已提供，再生成部件、端口、过程时间轴与标签。缺少真实尺寸、工时或工艺参数时，使用明确标记的示意假设，不把图片推断成生产测量值。

生成后在原生视口渲染：Agent读实际渲染图和结构摘要，检查可见部件、标签遮挡、输入输出位置、过程画面是否一致，再修改对应对象。用户可在同一编辑器直接改部件／标签／阶段，保存设备包或应用到选中设备。运行中的工程规则修改进入新方案；不能把生成草稿悄悄替换到正在运行的彩排。

详细设备允许借助外部建模工具制作曲面、机构、材质与刚性动画，再通过现行模块包导入；不把原生基础几何当作质量上限。Agent先说明工具需求与实际可用情况，按[建模技能指南](../../integrations/agent/skills/superfactory-workflow/references/equipment-modeling.md)执行可编辑源→GLB重开→平台渲染→逐模型返修与接收。平台不自动执行导入包内的建模脚本，也不因此增加Blender/CAD工具或公共API。模型逐项状态进入[质量清单](../../modules/media/model-quality-ledger.json)，同生成器、同设备家族和接口测试通过不能免除单模型审图。

生成任务可以只要求一张设备外观，也可要求完整设备包；前者必须显示“仅外观，未配置生产行为”。完成完整设备包需要端口、行为、参数、外观、时间轴、标签和隔离样例；流程预览用已实现的仿真引擎，外观渲染与生产验证分别报告。保存模块不自动采用、升级实例或取得生产能力。

## 共同数据与职责

Agent与人工编辑共用模块格式，不增加另一套设备体系。module-kit提供能力目录和草稿命令；domain定义数据；presentation渲染原生部件、图片序列与标签；simulation推进实际过程；planner提供编辑和反馈入口。Agent接入实现为编辑器能力桥，不先创建独立服务器或新的仿真系统。

参考图片帮助判断表现，已有参数与能力清单决定行为。生成输出使用声明数据，不执行图片里写的指令或上传包夹带的脚本。调用外部模型时只发送用户在本次生成任务中选择的素材与必要草稿字段，并在入口说明发送范围；预览图不默认包含整厂工程、原文资料或机器文件。

## 最小能力集合

|拟议操作|输入|返回／影响|
|---|---|---|
|describe_editor|选中草稿ID|版本、当前对象、支持的几何／字段／行为／操作；只读|
|create_device_draft|名称、目标任务、分类|新草稿ID；不放入运行工程|
|edit_device_draft|草稿ID、按稳定ID设置／移除的字段、部件、端口、时间片段和标签|更新后的同一草稿；与人工编辑相同|
|render_device_preview|草稿ID、状态／进度、详细或简化视角|实际渲染图及可见部件／标签摘要；这是外观检查|
|preview_device_cycle|草稿ID、示例物品／资源|隔离周期结果和逐步状态；只有引擎与所选能力实现后才可调用|
|save_device_package|草稿ID、模块版本、作者与素材声明|可重开编辑的本地模块包；未声明许可时仍为私人草稿|
|apply_device_draft|草稿ID、指定实例或新增位置|应用到当前设计方案；保留物品与端口绑定，变更连接时给出可编辑结果|

命令按草稿ID和对象ID操作。来自外部Agent的字段、类型、文件引用、能力名在同一导入边界校验；禁止Agent用未知操作冒充能力。渲染反馈不是仿真结果，preview_device_cycle没有实现时如实显示未可调用。apply_device_draft执行[显式采用流程](../02_交互设计/自定义设备创作与采用.md)：列受影响实例/端口，保存不自动升级；多阶段外观绑定实际阶段与等待姿态，不从播放位置猜工序。生成仅外观时仍保留生产能力缺项。

机器可读的拟议能力列表见[device-editor-capabilities.design.json](../../packages/module-kit/device-editor-capabilities.design.json)。它用于P3实现和生成任务约束，不是当前已开放的API。

## DRF04：能力到真实18操作的映射

公共[API](公开操作与宿主调用契约.json)是唯一wire格式；上表七个名称仅标设备能力，不能注册七套同名工具。

|设备能力|实际操作/载荷|结果与采用|
|---|---|---|
|describe_editor|describe_capabilities＋get_context/get_object，DefinitionDraft Scope|真实可用参数、几何、媒体和行为；无能力返回UnsupportedCapability|
|create_device_draft|apply_commands.CreateDefinitionDraft，明确ModuleDefinition|EditCommit新草稿/修订，不改Run|
|edit_device_draft|apply_commands.PutDefinition或SetDeclaredParameter|同一草稿、声明parameter_id和TypedValue；binding为相对定义RFC6901|
|render_device_preview|render_view(revision、view、preview、delivery)|TaskHandle→get_task.result.Render；校验草稿修订，外观不计生产|
|preview_device_cycle|run_scenario(intent=definition_cycle，definition_revision、明确test_input)|TaskHandle→RunSummary→get_run；同Engine隔离样例，不复制正式库存|
|save_device_package|CommitDefinitionVersion＋export_package(private_module/community_module)|版本提交为本地冻结，导出为Task；私人文件不自动公开|
|apply_device_draft|apply_commands.AdoptModule，固定module_ref、targets、bindings、new_instances|列受影响实例/端口；运行中的旧Run保持冻结输入|

端口、部件、媒体、热点、阶段/标签取同一ModulePayload.Devices；引用已存在的ID，部件几何/姿态以正式类型及坐标系表达，不从图片捏造工时。定义级编辑与实例configuration覆盖分开；参数超范围InvalidInput，陈旧修订RevisionConflict，整对象修改也限获准字段。绘制阶段必须引用实际阶段身份，纯外观关键帧按过程编辑u/phase进度，不能往只读实际子轴写阶段完成。

长任务、取消、模板/报告权限和时间轴接点沿[DRF04定义](../04_开发与验收/特征细化/DRF04_完整定义.json)，实际使用[项目workflow](../../integrations/agent/skills/superfactory-workflow/SKILL.md)。先检查then渲染/隔离样例，分别返回外观缺项、行为缺项和真实结果；取消后不写成功，完成先胜仍Completed。声明社区许可不替代Host确认，人工认可仅实际UI身份可提交。

## 原生装配台生成示例

用户说：“做一台壳体与PCB装配台，台面、立柱和上下运动的执行头；标签显示PCB库存、进度和缺料原因，点工时标签能改工时。时间轴演示定位、下压、保持、回位。”

1. 读取assemble、固定工时、三个操作与box/cylinder/plane能力，建立草稿。
2. 添加台面、立柱、执行头和治具；建立壳体／PCB输入端口和装配体输出端口，放在可见位置。
3. 设置60秒示意周期，划分四个阶段；给执行头添加姿态关键帧。标注尺寸和动作不是实测。
4. 绑定端口可用量、周期进度和等待原因标签；工时热点绑定configure，检查热点绑定inspect。
5. 渲染空闲、下压、保持、回位四个阶段，再看简化档；修正标签遮挡和视口构图。
6. 引擎具备后，用壳体2件／PCB1片／工人1名试运行，要求完成1件后显示缺PCB等待。仅外观生成不能填写此验证通过。
7. 保存与[人工可编辑样例](../../modules/devices/manual-assembly/README.md)同类的包；用户可继续改图片、部件和行为。

这里“原生”指平台对象可编辑、参与端口连接并共享运行状态。它不表示真实设备数字孪生、硬件控制或工业参数自动辨识。

## 社区与比赛展示

贡献的资产是同一设备包、隔离场景及说明，而非一张AI效果图。几何、时间轴、标签和行为可以分别改进；贡献者声明参考图和模型来源，第三方资产不能因被Agent处理就变成自己的版权。生成的部件数据可独立审查和编辑；本项目代码与素材分别声明许可。

比赛可展示“描述→原生可编辑设备→人工调整→插入产线→缺料等待→新增来料再运行”的过程。设备生成、时间轴、模块化都是已存在方法，不能单独宣称独创；最终增量需要用同一任务说明用户省去哪些操作、交付何种可接手资产。当前无已测工时改善或运行演示。

## 类别边界

本文件的部件、端口、时间轴和标签命令用于固定设备创作，不强加给物品、资源或专业计算。Agent选择其他贡献类别时按[九类清单](九类模块清单与消费者.md)填写相应载荷，导入仍由module-kit采用；当前不额外建立服务API。设备采用独立工艺规则需明确版本和角色映射，Agent也不能同时执行内联规则。

## 现行主线在本文件的责任

SF-17在设备生成之上增加工厂任务：query_module_library选行业/场地模板和九类角色，apply_commands生成可编辑对象、布局/连接/排期，check_factory_draft检查，render_view与实际run_scenario/get_run反馈修正。仍与人工共用入口；仅生成图片不完成任何建厂验收。

依据：[项目主线](../00_项目基线/项目主线.json) · [SF-17生成规格](目标驱动厂房生成.md)。本节是待实现要求，沿用本文件其他业务规则。

现行公开字段／原子编辑／文件采用／真实任务回执统一消费[公开操作与宿主调用契约](公开操作与宿主调用契约.md)。设备功能ID不是新增MCP工具；九类载荷、原生部件／媒体／热点通过有类型定义编辑进入同一15分支命令，外部Agent不能填写Host身份、许可确认或人工认可。
