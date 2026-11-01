# DRF05 完整定义：比较、评估、保存交接与贡献

2026-10-14，DRF05-r3。P2-03/P2-06/P5-01；关联S03-05、S06-02/03/05、S11-01/03。原17SF/21P/60S/Q01—Q64范围不变。**r2共同载荷与表单定义已合入，拥有者消费已对齐；r3已合入并由当前消费者复核，实际产品/Windows恢复/跨机仍未执行。**

机器可读入口：[完整定义JSON](DRF05_完整定义.json)；缺口与请求：[缺口JSON](DRF05_缺口与修复.json)。已有业务/工程/社区定义直接作为拥有者源，本页不重复整份合同。

|适用视图|精确实现入口与确定内容|
|---|---|
|任务与交互|[指标与决策](../../01_业务需求/指标成本与决策口径.md)：共同初始起点、受控变更、严格相同/不可比、两方案差值；[报告](../../03_架构与契约/评估报告与数据导出.md)：选择、冻结、生成、失败返回。|
|结构与数据|[reporting契约](../../../packages/reporting/evaluation-report.contract.design.json)的public_field_mapping完整覆盖现行22字段；format_version_mapping定位Versioned.schema_version与data.id；csv_projection列15分表。公共r2已合入；r3最小字段及快照增量见公共delta，禁止另立运行类型。|
|状态与行为|Engine观察窗接受点[start,end]、区间积分[start,end)；部分计算/开放等待/真实阶段/裁窗身份；同源成本和容量。保存R/编辑R+1、清单提交、取消竞争、打开私有草稿后采用，见[工程交接](../../03_架构与契约/工程交接与打开.md)。|
|接口与协作|get_run、compare_runs、export_evaluation、save_project、export_package、import_package、get_task、cancel_task及apply_commands沿现有18操作。打开完成用AdoptImportedDraft；文件输出沿TaskHandle→TaskOutput，不创造open_project或导出专用Handle。|
|界面与表现|表单消费定义已接入；本轮docs02/实际母版、可点击/本人走查归窗口2；下方工具、默认160px时间轴、窗口菜单控制显隐、独立描述dock。|
|部署与验证|沿已选Rust/printpdf0.12.8/csv1.4.0/Source Han Sans CN2.005R和Windows11 x64，不安装/改锁。15分表、PDF中文跨页、离线HTML、本机保存和跨机修改重跑各有真实验证入口，未实测项仍待取得。|

## 必须按此执行的数值与归属

- 观察窗末刻是Engine实际完整闭包，不被图形半开裁窗漏计。Events interval:null保留授权范围的完整时间流；报告总量消费FullResult/EvaluationReport，不能用已经加载的可见页计算。H01给公共精确出处与末刻例。
- 同起点是从初始输入重跑。两厂房B新增一片PCB是受控变化，严格初始相同应为false；完整列before/after、产品/需求/观察窗/接受位置/阶段/单位，再决定指标是否可比。不同接受阶段分列，不给改善百分比。
- 容量2的资源不能以忙区间并集替代容量积分；等待多原因不重复加时，跨对象累计注明对象·毫秒。实际加工取TC02，设备/阶段/等待归属取TC01；缺实际历史保留未知。
- 成本冻结费率/计量单位/币种/包含项；paid_calendar与occupied_time同对象同费项互斥。已知小计不是完整总成本，未知总额及单位成本保持null；展示舍入不反写基础值。
- 当前导出工程R8与旧Run输入R3/R4必须同时保留。多Run报告已按H02逐Run分组，不能把同order_id/resource_id混为一条。当前r2结构已由公共源统一定义；本轮r3剩余字段以复核结果逐项说明，真实输出未宣称已实现。
- 文件任务成功只在全部选定文件提交后成立。保存R7期间编辑到R8依旧dirty；ReplaceFileW失败按真实后置位置恢复，恢复失败也不得声称旧根完好。跨机重开不携带作者机器的私有时间轴偏好、当前库存或凭据。
- 本地贡献候选可显式导入/采用；精确版本固定。技术审阅、合并、用户授权指定版本发布、实际发布分别据实；逐文件许可和Host实际确认不能由外来布尔声明代替。

## 有限独立预期与证据等级

[独立预期JSON](DRF05_独立预期.json)的E01—E12覆盖末刻/前缀、C与迟延、容量、等待、阶段、条件成本、既有模板比较、供料变更、多Run/null编码、接受不可比和无Run。预期先按需求和现有有限案例写定，再以独立算式检查，不从产品输出反写。

[模板接手预期](../../../scenarios/模板与交接/expected-template-handoff.design.json)保留原10例，drf05_handoff_cases含原14例及追加HFC15/16，共16例，覆盖并发编辑保存、失败/取消、相对资源、精确版本冲突、未知格式、缺行为/权利、旧Run/新输入、私有偏好、跨机、贡献与ZIP边界。HFC是设计操作预期，本轮未实际打开/保存/跨机。

验证命令：`python docs/04_开发与验收/特征细化/DRF05_核对独立预期.py`。实际输出记[验证结果](DRF05_验证结果.json)。静态/有限数学通过只证明本轮定义及预期的对应关系，不是PDF导出、产品运行、实际原型、本人或第三人接受。所有actual_result保持null。

## 进入下一步的条件

窗口1已合入r2端点/报告/比较/容量/成本及共同保存定义，H07表单已接入，生成器已保留新增情景；本轮r3的CostEvaluation两个字段与InputChangeValue四个快照变体已由窗口1同步。S06-05仍须原五链全层联合冻结；S07取得实际入口命令后，按用户派发才能编码。P2-03/P2-06/P5-01的实际四格式、保存恢复、另一机器修改重跑和九类贡献采用由真实软件及目标使用者取得证据。本轮未Figma、未产品编码、未推送发布。

## 本轮r3精确增量与复核

[公共delta JSON](../../../packages/reporting/public-delta.design.json)与[直接采用的Rust声明片段](DRF05_公共增量.design.rs)明确字段/嵌套/Option/单位/序列化；[增量说明](DRF05_公共增量.md)给出迁移操作及CostBasis判断。[15组独立正反例](DRF05_增量正反例.json)覆盖旧Run归属、有类型变化、负差/零基线、缺日历、零价/缺价/单位不匹配、完整额/币种未定/前缀、重复计价与提交恢复。

[消费者复核](DRF05_消费者复核.json)只读比较当前Rust、标准Schema和TS；当前全部项标merged，未同步项为空。Wait结束只消费condition_changed/resolved/observation_end/computed_boundary，Claim按平行区间序消费released/observation_end/computed_boundary；旧别名仅为显式旧设计转换，既有数值不变。CostEvaluation.inputs为所选旧Run冻结费率快照，missing_conditions涵盖所要求但未支持费用；本轮隔离投影实际读回四种文件；产品导出仍未执行。


本轮收口：r3当前公共同步剩余0；[四格式与保存试做](DRF05_四格式与保存试做.md)记录type_fixture四格式69行读回、4页PDF检查及新授权受限HTTP的HTML实际浏览器呈现/截图。窄窗口单位断行和表头丢失已修复，临时27855已释放，窗口2的8921保持监听。新增两项隔离Windows文件接点：实际准备8→7、替换提交7→8；文件占用错误32且旧R7根保留。与6逻辑参考/原12 I/O实试分开；Host目录键、1176/1177恢复、产品/16软件交接/跨机与本人证据仍待取得。
