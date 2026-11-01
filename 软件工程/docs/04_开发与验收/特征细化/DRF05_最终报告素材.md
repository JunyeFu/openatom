# DRF05 最终选题报告素材

2026-10-14，复审窗口5。素材已由S06-05冻结后的最终报告消费；本文件保留有限试做证据层，未重做旧报告。机器稿见[素材JSON](DRF05_最终报告素材.json)。

## 可直接采用的正文

### 整体报告与交接定位

超级工厂把方案输入、生产结果与评估交付连接在同一工程中。设计中的评估报告携带目标、输入身份、接受范围、等待与资源区间、成本缺项和方案比较；可编辑工程另行交付，供接手者修改新方案后从初始输入运行。

### 四格式同源

已用一个人工构造的EvaluationReport实际写出并读回JSON、CSV、HTML及四页PDF。69行共享投影一致，PDF中文与分页已图像检查；HTML在实际浏览器核对窄窗口、滚动、放大及比较尾行，并修复单位断行与表头丢失。该试做验证文件与呈现，不产生生产运行或工业改善结论。

### 冻结输入、单位与未知

报告逐Run保存输入身份、计算范围、冻结费率和缺项。导出时工程修订为R8，TF-A/B/C仍分别属于输入修订3/4/5；36/72的人工费率不被当前方案覆写。CNY与USD分列，未知值保持null，明确零价保持0。TF-C只计算到50000ms，前缀接受量不替代全窗结果；这些数值均是类型样例。

### 比较限制

比较先说明两方案的输入变化与可比范围。样例TF-A→TF-B可显示绝对数量差2piece，基值为0时百分比为null；TF-A→TF-C的接受阶段及计算范围不同，只并列展示，差值和百分比都为空，不生成改善判断。

### 保存真实文件接点

另在自建临时工程验证两个Windows文件接点：R8文件先准备，仍按R7→R8实际替换根清单并重开，最终为R8；不分享DELETE的真实句柄占用造成替换错误32，旧R7根完整保留。六项逻辑参考和既有十二项I/O实试分别记录，不合并成产品Save/Open验收。

### 工程与社区交接

设计将评估报告、可编辑工程及模块贡献分为三个交付入口。工程以Versioned<PackageManifest>根清单明确列出活动工程、精确模块、相对资源及所选旧Run的完整冻结输入，导入先形成私有草稿再明确采用；接手后新运行从新方案初始输入开始。许可和版本采用按现行契约处理。十六个软件交接情景尚待产品执行。

## 图片与四格式文件

图片均须保留证据说明。DRF05的69行/4页样例、S05-05的48行/2页Rust样例和S04手工L03原型是三个数据集合，不能拼成同一次运行。S04原型截图已实际读取核对，窗口2的证据只读引用。

|素材|可用图及图注|证据层|
|---|---|---|
|DRF05-IMG01|[default-prefix-C.jpg](DRF05_四格式试做/html-browser/default-prefix-C.jpg)：实际浏览器中的人工类型样例：TF-C只覆盖50000ms前缀，whole_window与接受范围分列。|人工类型样例实际文件呈现|
|DRF05-IMG02|[default-comparison.jpg](DRF05_四格式试做/html-browser/default-comparison.jpg)：实际浏览器中的人工类型样例：零基数百分比为空，接受范围不一致的两Run仅并列。|人工类型样例实际文件呈现|
|DRF05-IMG03|[fixed-narrow-values-units.jpg](DRF05_四格式试做/html-browser/fixed-narrow-values-units.jpg)：640px窗口实际滚动截图：row_id/Run保持可见，数值、person_ms和CNY/USD不拆行。|人工类型样例实际文件呈现|
|DRF05-IMG04|[page-1.png](DRF05_四格式试做/output/preview/page-1.png)：人工类型样例PDF第1页，实际PDFium渲染；可用于说明中文报告文件版式。|人工类型样例实际文件呈现|
|DRF05-UI01|[VP22.png](../../02_交互设计/S04_可操作设计原型/output/playwright/VP22.png)：S04可操作设计原型的四格式预览，画面使用手工L03样例，产品Engine/Host/Agent尚未实现。|浏览器设计原型|
|DRF05-UI02|[VP23.png](../../02_交互设计/S04_可操作设计原型/output/playwright/VP23.png)：S04可操作设计原型的可编辑原型快照交接，快照是原型状态JSON，正式ProjectState/工程包产品接点待实现。|浏览器设计原型|
|DRF05-UI03|[VP24.png](../../02_交互设计/S04_可操作设计原型/output/playwright/VP24.png)：S04可操作设计原型的冲突、撤销和未保存离开画面；保存失败是原型分支，不能当Windows故障实测。|浏览器设计原型|

四格式实际文件：

- [JSON](DRF05_四格式试做/output/evaluation.type_fixture.json)，绝对路径 `D:\Agent\ICNSD-KEL\大作业\02_竞赛作品\超级工厂\软件工程\docs\04_开发与验收\特征细化\DRF05_四格式试做\output\evaluation.type_fixture.json`。
- [CSV](DRF05_四格式试做/output/evaluation.type_fixture.csv)，绝对路径 `D:\Agent\ICNSD-KEL\大作业\02_竞赛作品\超级工厂\软件工程\docs\04_开发与验收\特征细化\DRF05_四格式试做\output\evaluation.type_fixture.csv`。
- [HTML](DRF05_四格式试做/output/evaluation.type_fixture.html)，绝对路径 `D:\Agent\ICNSD-KEL\大作业\02_竞赛作品\超级工厂\软件工程\docs\04_开发与验收\特征细化\DRF05_四格式试做\output\evaluation.type_fixture.html`。
- [PDF](DRF05_四格式试做/output/pdf/evaluation.type_fixture.pdf)，绝对路径 `D:\Agent\ICNSD-KEL\大作业\02_竞赛作品\超级工厂\软件工程\docs\04_开发与验收\特征细化\DRF05_四格式试做\output\pdf\evaluation.type_fixture.pdf`。

截图和文件的绝对路径同时列在素材JSON的assets/actual_four_format_outputs中。PDF为既有ReportLab参考，未因素材整理重生；选定printpdf产品路径的真实实现仍另验。

## 当前设计证据与后续入口

现行视觉基线为draw.io原生母版及字段规范，Figma保持暂停。窗口2的S04-04/05已正式归集用户授权AI实际浏览器走查通过、五条连续链和八组失败检查；S05-05实际桌面接点也已公共合入，公共设计现30/30，完整设计已冻结。公共准入JSON的design_freeze已通过；本窗口依明确冻结放行实际生成最终DOCX/PDF并完成29页QA，不修改公共通过数、S06出口或MS04。

本轮HTML的操作者是AI，S04授权代走查也据实际操作者记AI；真人actual_user_walkthrough保持空。这不否定已经取得的AI设计证据，独立目标用户试用属于S10。产品Engine/Host/实际Agent、正式工程包保存/恢复、16软件交接与跨机修改重跑仍未由本次素材证明。

报告区分三种交付：最终选题说明表达设计；PDF/HTML/CSV/JSON评估表达所引用结果；可编辑工程/模块包支持接手与采用。不要将报告截图或S04原型快照写成完整可编辑产品工程。

当前完整报告：[当前Markdown](../../../../模块研究/输出/报告/超级工厂_最终选题报告.md)；正文、12幅图和7项核读原始来源已按用户精简为25页报告，56书签/13项目录跳转及逐页QA通过；作者傅钧烨。原29页导出属于此前历史试做记录。

最终交付：[可编辑DOCX](../../../../模块研究/输出/报告/超级工厂_最终选题报告.docx) · [PDF阅读版](../../../../模块研究/输出/报告/超级工厂_最终选题报告.pdf) · [逐页视觉QA](../../../../模块研究/输出/报告/超级工厂_最终选题报告_配图/逐页视觉QA.json)。产品与跨机验收证据不由报告交付替代。
