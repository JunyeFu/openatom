# 按类别选择贡献起点

<!-- confirmed-choice-source:start -->
## 已确认的本文件设计要求（2026-10-14）

[统一选择](../../docs/00_项目基线/全量设计编码确认选择.json) · [SXX逐项要求](../../docs/04_开发与验收/全量设计编码确认/SXX-XX_确认选择落实.md)

下列为本文件当前设计/验收要求，具体细节仍按所属S任务落实；原文中的历史候选不代替这些选择。

|题号|已选A及本文件要求|
|---|---|
|Q20|共同身份/版本/来源页，加各类载荷表单；均支持草稿、检查、适合的预览、明确采用和导出。|
|Q25|详情明确显示三层和当前版本；改实例不改库，另存新定义后列变化并选择受影响实例升级。|
|Q27|导出选定组合及角色需求；采用前预览新ID/引用/资源复用，种子供料和订单单独确认。|
|Q55|带所选方案与必要定义/合法资产/成果/认可绑定/所选实际结果，并列缺项；其他历史按需选。|
|Q60|原创代码倾向MIT候选；第三方保留原许可，最终发布文本先结合真实资产清点核对。|
|Q61|本地类别包＋真实示例＋许可/版本/支持范围，经仓库Issue/PR审阅后显式采用；发布由你决定。|
<!-- confirmed-choice-source:end -->

各类别共同清单及业务字段见[九类协议](../../docs/03_架构与契约/九类模块清单与消费者.md)。既有[设备模板](module/README.md)只用于devices，不是其他类别的通用载荷。

|类别|可复制/阅读的声明例|贡献的内容|
|---|---|---|
|物品|[壳体规格](../../modules/items/shell-spec/manifest.json)|单位、规格、属性和可选外观，不带当前库存|
|固定设备|[三输入装配](../../modules/devices/control-box-assembly/manifest.json)|端口、公开参数、选定行为和表现|
|资源|[人员角色](../../modules/resources/operator-role/manifest.json)、[服务源](../../modules/resources/power-source/manifest.json)|角色/容量/日历或工况，不带当前占用|
|任务|[一次准备](../../modules/tasks/board-preparation/manifest.json)|目标/前置/步骤/资源与完成作用|
|工艺规则|[齐套规则](../../modules/operations/control-box-rule/manifest.json)|配比、身份/质量作用和资源，不内置第二库存|
|物流|[批次运输](../../modules/transport/batch-transport/manifest.json)|来源/目的、车辆/路线及移交策略|
|厂房与组合|[并行制造](../../modules/templates/parallel-electromechanical/manifest.json)|自由定义产线/区域/厂房及边界，对象、布局、连接与明确种子政策|
|专业计算|[提供者计划](../../modules/professional/machining-producer/manifest.json)|真实工具输入输出契约，支持范围和未实现状态|
|表现素材|[原生台面](../../modules/media/editable-bench/manifest.json)|格式/部件/姿态/锚点与逐资产许可|

全部为design，没有注册或运行发布。拷贝后使用自己的模块id和真实作者/许可，示例结果单独保存；不将手算填成actual_result。对照实际贡献责任提供必要输入与缺项，不为了统一格式强加设备的开始/播放按钮。

平台须允许人工和Agent从这些类别新建/修改、检查适用样例、采用到自己的工程，再导出类别贡献包。一个有明确数据流的插图设备可以先用于模拟；无需先制作精细3D。专业示例当前只说明载荷，实际首版纸包装提供者由P4-02实现；设计例不能叫可计算插件。

## 现行目标生成与开放主线

templates类别增加可选generation_profile；贡献行业/产品/场地匹配、参数/角色/布局模式，project仍是可编辑载荷。九类各有消费者，不新增第十业务类型。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。
