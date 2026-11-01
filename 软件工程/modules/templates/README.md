# templates

<!-- confirmed-choice-source:start -->
## 已确认的本文件设计要求（2026-10-14）

[统一选择](../../docs/00_项目基线/全量设计编码确认选择.json) · [SXX逐项要求](../../docs/04_开发与验收/全量设计编码确认/SXX-XX_确认选择落实.md)

下列为本文件当前设计/验收要求，具体细节仍按所属S任务落实；原文中的历史候选不代替这些选择。

|题号|已选A及本文件要求|
|---|---|
|Q04|展示任务/模板/空工程三入口；按当前目标逐步补必要参数，允许保存草稿。|
|Q09|多选、编组/解组、整体移动、复制为新ID；连接/角色待重绑和种子供给单独确认。|
|Q27|导出选定组合及角色需求；采用前预览新ID/引用/资源复用，种子供料和订单单独确认。|
|Q46|关键缺项集中询问，普通示例假设显式标注并可改；无数据不填虚构充足库存或产量。|
|Q48|先限排期、可选模板布局与明确允许的设备实例数；工时/BOM/质量、人员增补和成本需明确授权。|
|Q49|候选默认3轮，每轮真实检查/运行并保留变化；达标即停，关键缺项或无有效改动则说明。|
|Q50|保存已完成草稿与缺项，给出人工创作/已有模块组合入口，不自行发明新工艺求解器。|
<!-- confirmed-choice-source:end -->

贡献包按独立ID创建子目录，包含manifest.json、README.md、示例与素材许可。公共约定见 ../../docs/03_架构与契约/模块设计.md。原manifest/project.design仍是设计输入；P4-01当前公共类型候选、Rust入口及真实消费者证据见下文。

三份独立模板：[机加工交付](machining-delivery/README.md)、[外购总装](purchased-assembly/README.md)、[机电并行制造](parallel-electromechanical/README.md)。各自包含完整工程声明，复用同一模块库；默认不复制已有工程库存与订单。主规格见[模板与接手](../../docs/01_业务需求/三类工厂模板与接手.md)，生成脚本只整理设计，不运行产品。

行业生成样例：[纸箱厂目标与场地种子](carton-factory/README.md)。generation_profile组织行业/产品、长宽高、工艺与九类角色填充，供外部Agent真实操作平台生成；种子未填参数、未运行，不是日产已验证模板。贡献者可提交新的行业生成配置，版本/许可/执行范围按共同规则处理。

## 现行目标生成与开放主线

通用离散3模板用于直接编辑；纸箱制造配置供SF-17行业/场地目标填充，采购纸板→选定模切/折糊成箱→检验/打包/接受；与把产品装入纸箱区分。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

DRF04字段级采用入口：[instantiation.design.json](instantiation.design.json)。精确版本、新ID/引用重写、实际资源复用和供料/订单种子分别确认；设计文件须显式转公共类型。三模板未给高度/占地/间距时保留未判定，不把XY布局当场地通过。模板生成脚本仅同步设计；本轮未运行它的设备/catalog写入。

C02的显式包装工位分支已接入生成源，见[接入说明](../../docs/04_开发与验收/特征细化/DRF04_C02生成接入.md)。默认0.1.0声明模板保持；仅calculated_nominal读取owned consumer-device@0.2.0，Job ID/revision齐全才写两个TypedValue，缺选择列缺项。隔离验证只抽取纯函数，未执行覆写生成器或自动采用/注册候选。

## P4-01 当前实现（2026-10-16）

四目录下的 `candidate/` 是当前1.0公共类型文档，与旧设计文件分开。`module.json` 声明精确模块版本；`template.json` 是 `Versioned<TemplateSubset>`；`example-project.json` 是没有库存/订单的 `Versioned<ProjectState>`，其中示例Product/BOM/Route必须另选；`definitions.json` 是实际声明行为、人员和设备容量定义。原设计、来源与未知许可保留，不把旧draft-3直接当运行工程。

Rust入口是 [module-kit](../../packages/module-kit/src/templates.rs)：`builtin_template` / `decode_template_candidate` → `prepare_template` → 现有Commands `TemplateInstantiation`；`prepare_template_seed` 单独构造新情景，供料、初始质量、需求和检验预设分别选择。未选择资源就保持缺项；复用资源不修改实际容量/日历；复制不继承库存、在制、订单、Claim、认可或旧Run。 当前SDK另有`command_data_with_preparation`，对齐共同API的可省略preparation：实际列出的来源Project、命名空间、明确新建/复用/未解决资源、物品复用、示例产品选择和场地。Host重新按实际档案生成元数据，不接收调用者自报来源或认可。


当前候选显式声明设备执行容量，原检验的rework/quarantine同库别名归为一个实际隔离分支。未限定质量的普通声明工序显式接受unchecked/released，包装内容仍要求released；包装载体只在完成时按声明放行。`max_completed_reworks`仍可编辑定义字段，当前Engine不消费同名实例整数覆盖，不提供一个无效实例控件。原生表现不拥有物料或时钟。

[真实消费者与复现](../../scenarios/模板与交接/runtime/README.md)保留四模板Host导入/采用/运行/修改/保存重开、原预期时序、缺料例、九类可创作公共类型样例及共享缺口。源模块仍是Design候选，专业模式只有declared_process；几何/专业成果/实际许可未知不变。纸箱声明例只验证纸板→模切→折糊→检验后的箱数，未启用印刷/打捆，不代表日产一万已达标。

本轮接收复验：原17+新增2组SDK测试及同源Engine6例通过；四模板旧/显式形状及新源版本共9条完整Host链通过，包含Undo/Redo、Save/搬移/Open/重跑及三组元数据精确相等；另有9个拒绝的原子性与未解决角色草稿验证。P401-HOST-01/02已实际复验关闭；完整UI/IX/NFR/整包最终接收保持独立。
