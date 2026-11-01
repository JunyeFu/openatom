# P2-01 普通Native创建与布局连接验证

状态：P2-01继续in_progress，整包、IX、NFR与真人接收均未代填。全部操作由本窗口完成，四批隔离源码只读。

## candidate23：空工程到三设备分支

普通无参数进程PID24664从空工程r1创建30×20×8m厂房，只采纳CNC精确定义1.0.0，再声明工件类型与缓冲库位、线组及设备。根节点X+3m后设备局部(3,4,0)不变、厂房坐标为(6,4,0)；撤销/重做分别形成r9/r10。首条物料连接及其撤销/重做到r13已保存。

继续从CNC-A复制为独立ID P201-CNC-B/C，实际表单确认两个副本的物品角色为空、尚未归组。随后明确重绑input/output物品并归入原线组，厂房位置分别保持(12,6,0)/(12,2,0)，局部位置为(9,6,0)/(9,2,0)。物料交接为CNC-A output→共享缓冲→CNC-B/C input，三条连接均包含精确模块引用、实际物品及piece单位。缓冲归入实际厂房、允许普通领用；容量仍未声明。

实际保存r20共3文档，原A及首条连接不变，精确模块及原始支持文件逐字节保留。没有自动创建Scenario、Run或初始库存。见[空工程到首条连接](../../.work/single-window-20261018/p201-public-native23/empty-create-edit-validation.json)及[三设备分支实际数据核对](../../.work/single-window-20261018/p201-public-native23/three-device-branch-validation.json)。r20保存后正常关闭，OS确认PID已退出。

## 发现并修复的画布缺口

r20三条连接在Host及对象树中存在，普通Native画布未画方向。冻结设计要求可读的物品方向和连接，源码原本缺少该渲染。公共前端增加基于实际对象定位和精确端口定义的物料逻辑箭头；直接点击箭头或画布标签中的连接可打开原业务详情。合批几何保持一条逻辑连接与原ID对应；不生成运输时长、生产状态或新的连接对象。缺少精确版本、端口、角色/单位、当前画布端点或重合位置时不猜测连线，显示具体未绘制原因。旧Run使用已读取的冻结定义，不借用当前版本。

6项定向检查、22项已有来源/冻结消费者回归及类型检查通过；这些是源码验证。candidate24显式NFR候选构建通过；普通无参数新进程PID33756实际重开r20，直接点击箭头定位原C连接、标签定位原B连接，两画布×三LOD均显示3/3连线且保留同一修订。另存3文档后完整Project相同，模块与原始支持文件逐字节不变，没有新Scenario/Run/库存。见[实际Native验证](../../.work/single-window-20261018/p201-public-native23/connection-drawing-repair/Native-validation.json)。真实原Run P201-core-164896:run:1:2随后在同一candidate24读取：H1画布40/120、多厂房120/120条逻辑连线，1200秒与3000实际物品身份保持；末端L6-D10-output详情仍为原Run来源、编辑禁用。另存13文档后完整Project/Scenario相同，唯一原Run及8份不可变文件逐字节不变。见[冻结Run实际验证](../../.work/single-window-20261018/p201-public-native23/connection-drawing-repair/frozen-Run-Native-validation.json)及[原始Sky截图](../../.work/single-window-20261018/p201-public-native23/connection-drawing-repair/candidate24-frozen-connection-readonly.jpg)；UIA大树截断的指定字段由原始截图核对，没有据此计算正式时延或FPS。本轮不当完整包或正式性能通过。原始证据及进度见[修复记录](../../.work/single-window-20261018/p201-public-native23/connection-drawing-repair/observed-and-repair.json)。

## 保留的完整验收

当前搭建验证不等于生产运行或静态检查通过。详细设备素材、缓冲容量、实际资源/工艺、产品目标及方案仍须按原任务补齐；声明未知保持未知，不制造库存。完整P2-01步骤、修订冲突/redo分支/工程切换、布局冲突/未判定、原IX/NFR及真人门槛保留。正式Native性能仍需原DPR1/100%环境，不以功能截图当性能数据。只读捕获的连线身份/缺项问题已在下文candidate27批次限定修复并取得实际公共MCP图片，完整捕获包验收不据此提升。

## H1静态检查与离开确认缺口

candidate24在balanced原草稿r2限定H1、显式选择L1-PRODUCT@r1/L1-SINK、0—1200000ms后，实际Host返回269项检查。20台设备的边界、占地重叠、高度各20项通过；净距及静态区域共40项、日历及需求2项保持无法确定，没有补造条件或推定生产成功。完整原始UI及分类计数见[基线](../../.work/single-window-20261018/p201-public-native23/static-layout-check/baseline-observed.json)。

实际点击返回后，确认被排在269项结果底部，当前视口不可见；滚到底部才可明确放弃临时输入。公共前端复用既有居中modal，焦点先落在保留输入；同时纠正复选框分行和产品修订rr1显示。candidate25前端/类型/Native构建通过，普通Native新进程已复验居中确认、键盘保留输入及明确放弃路径；后续实际冲突与历史结果见下文。见[缺口与修复状态](../../.work/single-window-20261018/p201-public-native23/static-layout-check/observed-and-repair.json)。

## candidate25实际布局冲突、历史与保存

普通candidate25/PID35304限定H1，以原产品L1-PRODUCT@r1和L1-SINK、0—1200000ms检查；不补净距/日历/需求。D01的X60m/高度10m在r3产生4条边界与2条高度失败；点击失败条目准确定位原D01及当前值。r4撤销恢复原X4/高度3，r5重做、r6撤销均形成新修订；r7新编辑到D02原位置X7,Y5,Z0，旧redo禁用，实际返回两条重叠失败并能定位原D02。r8撤销恢复原几何后另存14文档；完整Project仅revision变化，Scenario仅revision/project_revision从2增至8，全部业务输入相同。唯一旧Run及8份不可变文件逐字节保留，没有生产重跑。

切换前旧菜单坐标缓存误命中一次redo（r9），实测识别后撤销为r10；已保存r8未改写。明确放弃这两个临时修订后打开另一独立工程r20，撤销/重做均禁用，前工程历史未串入。该输入事件作为自动操作记录保留，不伪作产品缺陷或漏记修改。见[Native实测](../../.work/single-window-20261018/p201-public-native23/static-layout-check/Native-validation.json)、[保存核对](../../.work/single-window-20261018/p201-public-native23/static-layout-check/saved-invariance.json)、[居中确认](../../.work/single-window-20261018/p201-public-native23/static-layout-check/candidate25-centered-confirmation.jpg)。原修订冲突、已声明净距/静态通道、捕获限制契约及完整P/IX/NFR/真人门继续开放。

## candidate27公共Native捕获的连接身份与限制

按冻结visible_objects约定核对时，发现只读捕获隐藏侧栏后漏报未绘制连线原因、所选Line仍带其他产线连接，以及已绘制箭头没有回原Connection身份。先按契约预期复现两项几何身份失败，再用同一合批箭头的实际线段、祖先变换、材质/图层、绘制范围及请求相机视锥识别原ID；不从DOM列表补身份。Line捕获只保留实际成员及关联物料连接，未绘制原因作为原连接的限制回执。公共wire/Scope、模块版本、模拟及库存均未改变。

10项定向、2项原可见性回归、类型检查和candidate27构建通过。复用原受控公共MCP Native模式，两独立进程实际生成5张2160×1350 PNG：原r20整画布/精确Line均回3个原连接；独立内存r21把B移到缓冲库位的实际厂房位置(10,5,0)m并将C归第二产线后，原Line回1个可见连接+B重合限制，第二Line回C连接且无B限制，多厂房回2个可见连接+B限制。所有ID与运行前预期完全相同，两个完整源快照前后一致；这属于真实Native公共MCP受控测试，不代真人或真实Codex使用接受。另1项实际ProjectHost陈旧修订拒绝测试通过；普通Native并发编辑反馈仍待实测。

candidate25正常退出后，candidate27普通无参数新进程PID37020/window61278876实际重开原手工r20，三连接/干净状态保留。见[实际验证](../../.work/single-window-20261018/p201-public-native23/capture-connections/Native-validation.json)、[原生PNG](../../.work/single-window-20261018/p201-public-native23/capture-connections/Native27-valid/case-01.png)和[受控Line PNG](../../.work/single-window-20261018/p201-public-native23/capture-connections/Native27-controlled/case-01.png)。原明确净距/静态通道、Native并发冲突反馈、P/IX/NFR/真人完整门继续开放。

## candidate28普通Native并发反馈与显式恢复

candidate28已修复并复验普通Native并发后的显式重读入口及Agent长面板离开确认：实际MCP仅改工程标题r20→r21，旧设备表单被拒且保留输入；确认默认保留，明确重读后读取原对象/r21，重新核对编辑r22、Native撤销r23、标题恢复r24及独立另存全部通过。保存完整工程仅较原r20增加修订号，两份精确定义文件保持原字节；原保存目录未改写。普通candidate28/PID38136/window9768104当前r24干净；原明确净距/静态通道与完整P/IX/NFR/真人门继续开放。

见[实际验证](../../.work/single-window-20261018/p201-public-native23/native-concurrency/Native-validation.json)、[修订冲突](../../.work/single-window-20261018/p201-public-native23/native-concurrency/Native28/06-stale-rejection-input-preserved.png)、[明确重读](../../.work/single-window-20261018/p201-public-native23/native-concurrency/Native28/09-reloaded-actual-r21-original-object.png)及[Agent居中确认](../../.work/single-window-20261018/p201-public-native23/native-concurrency/Native28/03-agent-centered-confirmation.png)。此批修改仅涉及公共ObjectEditor、Workbench和AgentWorkspace，没有改动四组隔离生产源。


## candidate30 显式净距与逐点区域

candidate30普通Native已完成净距/逐点可用区、禁入区与静态通道入口及实际检查：留空/零净距区分、3m净距检出未选邻机、三类区域冲突/通过、多边形非包围矩形、旧结果失效、H1/H2输入隔离和离开保护均取得限定证据。原归档为3厂房/6线/60设备/120物料连接；仅检查L1-D01，不推定全厂或动态交通通过。独立另存r8后完整Project/Scenario与8份不可变文件一致，未重跑生产。普通PID22216/window32574288当前r8干净；下一项为原P2-01多选编组/布局编辑与历史，完整P/IX/NFR/真人门继续开放。模板侧实际只应用尺寸，本轮仅增加指向草稿检查的说明，没有提供填后不生效的区域输入。

实际H1/L1-D01条件为L1-PRODUCT@r1、L1-SINK、0—1200000ms。缺声明时12通过/4未定；明确0净距后12通过/3未定；3m时检出未选L1-D02，12通过/1失败/3未定。三角形(0,0),(60,0),(0,80)分别作为StaticLane/KeepOut时各12通过/1失败/2未定，作为Usable时13通过/0失败/2未定。将第三点Y改为6，原设备占地包围范围仍在矩形内，但真实三角边界失败；日历与需求缺项始终保留。

4项字段转换与13项实际Host回归、类型检查和candidate30前端/Native构建通过。普通Native证据与8类实际检查汇总见[验证JSON](../../.work/single-window-20261018/p201-public-native23/static-region-entry/Native-validation.json)；[保存不变性](../../.work/single-window-20261018/p201-public-native23/static-region-entry/saved-invariance.json)保留原精确模块、冻结Project/Scenario、原Run结果及物品历史。原始UIA存在滞后，结论取单独刷新后的实际状态和截图；不可用的下拉鼠标点击没有当成已执行，最终选择由键盘与刷新核对。candidate29为中间构建、未启动，不当最终证据。


## candidate30/31 多选编组、整体变换、复制与长表单离开保护

candidate30普通Native完成两设备多选编组r9、组根整体移动/90度旋转r10、撤销r11/重做r12与新ID复制r13；5次当前Host只读get_object核对完整状态，设备组内姿态/其余对象/120连接保持，副本不复制业务供给或连接。独立保存r13后Project完整等于Host，Scenario业务输入保留且修订引用对齐13，原Run及4份模块共8文件字节保留，原r8未改写。复制长表单离开确认以内联块显示的缺陷已修复；10项模型检查、前端类型/构建与candidate31 Native构建通过。普通candidate31/PID29436/window9308658全新打开r13（62设备/8组），十设备长复制表单居中确认、Enter保留、重复ID拒绝和显式重读保留已复验。当前只保留未提交测试输入/离开确认；明确放弃与测试副本解组待动作前确认，未记通过。完整P/IX/NFR/真人门继续开放。

实际证据：[本轮JSON](../../.work/single-window-20261018/p201-public-native23/multiselect-layout/Native-validation.json)、[完整状态核对](../../.work/single-window-20261018/p201-public-native23/multiselect-layout/layout-state-validation.json)及两候选UI原始记录。Native输入均从普通产品界面发起；只读MCP仅核对同一实际Host状态，不充当GUI输入或性能证据。group前后用原点及三个基向量核对完整厂房姿态，容差1e-9；r10只变组根，r11/r12分别完整恢复r9/r10（除新修订号）。复制后为3厂房/8组/62设备/120物料连接，新增1组2设备，原对象和其余Project字段完全保持。10项现有布局模型测试通过，仍与普通Native证据分别记录。

当前等待只针对测试输入明确放弃、Native30-pair-copy解组及撤销；未执行就不填通过。原正式机器/Frames条件、IX、完整包与真人接受不据此关闭。原21项/68步骤/7后置不变，四个隔离批次源码本轮没有修改。
