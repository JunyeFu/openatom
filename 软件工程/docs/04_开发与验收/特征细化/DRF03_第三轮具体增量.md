# DRF03第三轮：实际模板内层与生成消费者

本文件是第三轮0.2.0原候选快照；当前公共强类型接入已由第四轮完成，见[正式合入复核](DRF03_C04_r4_交接.md)。旧候选与21项预期仍按原快照保留。
本轮公共源唯一写入仍由复审窗口1负责；窗口2的71字段UI、本轮其他窗口文件和公共进度只读。原22、29、声明7项预期不改；新增21项全部匹配。当前结果见[验证记录](DRF03_第三轮验证记录.json)和[有限输出](../../../scenarios/九类模块采用/DRF03_第三轮核验结果.json)。

## 已实际完成

- 已读取窗口1实际合入的 `HallDesign.height_m` 可空、`TemplateSubset.locations` 和 `MaterialEndpoint` port/location，不再把三个旧格式缺口列为待设计。当前411份TS检查是公共派生载体证据。
- [实际适配入口](../../../scenarios/九类模块采用/DRF03_旧清单适配试做.py)增加显式 `--inner-template`；默认第二轮路径不改。新输出独立放在 `template-output`，避免把原29项中的旧候选未就绪预期反写为通过。
- [真实内层模板](../../../scenarios/九类模块采用/template-output/template.json)保留12库位、18连接、6设备、3产线、10物品类型、4工人需求；未知厂房高度保留null，设备足迹/净高继续缺项。所有端点、module_ref、输入/输出方向、item_role和destination_store在当前Schema之后另做引用检查。
- [外层候选](../../../scenarios/九类模块采用/template-output/module.json)为未登记的模板0.2.0，依赖7份实际本地派生定义；内层、外层、7定义共9份当前Schema检查通过，全部列出的文件与局部资产能解析。原0.1.0没有被覆盖。
- 两组工序分别为10/45/5秒、5/20/5秒，保留阶段工人；整段duration=null，不叠加原cycle_seconds。准备作业仍为30秒、Board01、W-E、exclusive及零物料增量。两份专业声明完整保留为imported_claim，没有生产者结果或人认可。
- [492项原始叶字段去向](../../../scenarios/九类模块采用/DRF03_C04_逐字段去向.json)列出目标和缺项身份；完整原工程同时归档。40个布局对象ID及其关联端点/准备目标/声明消费者/质量目标已做独立新命名空间重映射，精确不可变ModuleRef不变。

## C02实际生成与任务投影

现存 `modules/templates/生成三类模板设计.py` 已由公共拥有者加入 `select_packaging_branch`，实际读取拥有者 `consumer-device.design.json` 的精确0.2.0。这里仅抽取其实际纯函数AST执行，未import或运行该文件的顶层写入。

五项预期覆盖：声明分支原样；未选Job时两个键均缺而configuration为空；明确Job选择才生成String/Integer覆盖；bool revision拒绝；P0-03/P4-02现行selection与编码包都引用拥有者定义。窗口1的原生Module/Device请求往返结果也已读回。`--design-handoffs-only`已经投影实际任务输入引用；没有默认Job、注册Provider或合格绑定结论。详[实际接入](DRF03_C02_实际工位定义与生成接入.json)。

## 仍需公共拥有者合入的精确义务

[伴随材料消费契约](DRF03_C04_伴随材料消费契约.json)是本轮选定设计。公共ImportDraft/模板采用编译器必须一起读取ModulePackage.files中的 `adoption-material.json`，保留准备前置、质量声明、源专业模式/声明和工人明确映射。尤其BoardFinish01的“输出pcb放行”是声明预设，当前InlineOperation没有对应输出质量效果字段，不能只加载TemplateSubset后悄悄删去，也不能让它获得真实工艺质量身份。

本轮已经给出确切消费者和触发规则，当前没有观察到公共导入/编译器完成这项接入，因此外层候选明确保留 `companion_consumer_requires_public_integration` 阻塞。这是**公共语义接入缺口**，不能包装成用户未填几何。

另有**真实输入缺项**：厂房/设备/净空几何、实际规格对象、Product/BOM/Route修订和工人采用选择。五行初始库存、数量2的需求和观察时间保留在草稿；seed=null，未造产品/订单/认可。指定专业模式还要取得原合同规定的实际结果与用途认可。

## 证据范围

原22/29/7和本轮21均匹配；本轮含当前Schema正反、端点错引用/错类型/错方向/错destination_store、新命名空间和真实生成纯函数。它们是隔离设计消费者证据，实际Host/Engine、完整专业运行、原型及本人走查均为空。S06-04既有FixedPose合入和独立设计通过记录保持只读；公共S06-05、通过数及产品任务状态未改。

本轮拥有者文本检查：50份JSON可解析、3份Python AST有效、92个本地Markdown链接存在；见[文本检查](DRF03_第三轮文本检查.json)。该数字只覆盖所列DRF03文本和新模板输出，不代表全仓库或产品运行验证。
