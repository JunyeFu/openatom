# 纸箱厂：目标与场地生成样例

状态：行业生成种子/参数契约设计，非已实例化可运行工厂。原三类通用离散模板保留；本例新增纸箱行业配置，用共同离散转换与资源机制，不增加瓦楞物理内核。

[生成流程](../../../docs/03_架构与契约/目标驱动厂房生成.md) · [行业/场地参数](generation-profile.design.json) · [工程种子](project.design.json) · [manifest](manifest.json)。

本样例限定需模切/折糊的纸箱产品族，实际规格与工艺需确认。采购纸板入料→可选印刷→模切→折糊成箱→检验/返工或隔离→打包→成品接受。目标计合格纸箱，不计束数；成品打包保留内容关系，不重复制造纸箱。工艺图的逻辑角色由九类库提供实际定义与版本，生成后仍能人工编辑。

Agent把用户目标绑定需求、把长宽高绑定厂房，读取模板/模块的工时、占地、高度、间距、班次、用料和检验来源，实例化后检查布局并实际模拟。当前没有真实设备参数、给定场地或班次，字段保留待填；不能把这份种子直接用作产能结果，不能将用户目标直接写到accepted数量。

软件实现后应从一句话和完整参数生成实际工程，初案实际达标直接交付；不足才在获准字段内最多3轮修正，无有效变化停止，四格式读同一真实结果。模板参数可以是明确标注的自建示例或获准工业输入；本文件未提供虚构机器额定参数或已达日产一万的结论。

工艺角色参照[BOBST原厂行业页](https://www.bobst.com/na/en/industry/corrugated-board)与[PK06核读](../../../../研究资料/阅读卡/PK06_核读.md)，未读取机器额定产能。用户L/W/H映射工程width_m(X)/depth_m(Y)/height_m(Z)，保留既有字段，场地以m、包装结构以mm填写。

generation-profile.canonical_bindings明确把目标转换为Quantity.value（DecimalAmount coefficient/decimal_places）与unit，场地转换为SiteEnvelope.length_m/width_m/height_m。旧逻辑别名不直接提交API。采用ID/资源/种子规则见[实例化契约](../instantiation.design.json)；依实际可用布局变换填参，候选模式名本身不是已实现算法。

## P4-01 当前公共类型候选与实际消费者

本目录 `candidate/module.json`、`candidate/template.json`、`candidate/example-project.json`、`candidate/definitions.json` 是当前公共类型输入，与以上历史设计稿分开。Rust实例化、独立种子、来源/缺项以及实际Host运行/修改/保存重开证据见[实现与复现](../../../scenarios/模板与交接/runtime/README.md)。原设计输入保持，不将局部运行或未知许可当整包发布接收；共享采用记录/参数来源接点已由W00修复并经本窗口真实复验；完整UI/IX/NFR/最终接收仍独立。
