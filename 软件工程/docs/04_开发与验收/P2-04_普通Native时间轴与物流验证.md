# P2-04 普通 Native 时间轴与物流验证

2026-10-18，W00。结论：当前公共 Host 双 Run、普通 Native 对比读取、运输持有者详情修复与另存保留取得限定证据。原 P2-04 三步骤、Q 与全部验收要求继续保留，整包尚未完成。

## 输入与真实生产结果

使用原 P2-02 IX01 baseline/delayed 固定输入：一个厂房、5 台设备、16 份精确定义。它不是原双厂房完整布局；定义未提供详细设备表现素材或运输空间轨迹，本轮不评价建模精度、行走动画或路径显示。

[实际公共 Host 记录](../../.work/single-window-20261018/p204-native31/Host-output/Host-validation.json)通过现行 ProjectHost 导入、Command 修改、运行、分页读取与保存，无直接 Engine 调用。原 IX01 和物料断言通过：基线 Run `whole-w5-host:run:1:2` 在 440 秒接收完 6 件；延迟 Run `whole-w5-host:run:1:3` 在 570 秒接收完 6 件。读取时点前后完整 Project 和 Run 相等。

普通 candidate31 核对基线 376 事件/32 作业、延迟 380 事件/32 作业。各有 1 条接收记录，该记录的数量为 6，不能表述为 6 条接收事件。同屏比较显示共同条件一致、`pcb-batch2` 到料 A240 秒/B420 秒；“读取 B 时点”实际切换到延迟 Run。证据见 [对比截图](../../.work/single-window-20261018/p204-native31/Native31/30-comparison-visible.png)及[跨 Run 读取](../../.work/single-window-20261018/p204-native31/Native31/32-B-primary-visible.png)。

## 实测缺陷与修复

370 秒选择真实货物 `output:33:job:10:TASK-LOCAL1:510:step-shell6:output1:0`，持有者为 `transport/trip:6:SUPPLY1:5`。点击“定位持有者”后，属性栏错误显示“选择一个对象”。见[原失败截图](../../.work/single-window-20261018/p204-native31/Native31/44-holder-location-result.png)。

修改 Workbench 的实际运输选择与详情、scene/runtime 的时点阶段解释和对应回归。详情直接读取当前冻结 Run 的运输记录，显示批次、供货输入、承运资源与四个事件边界；提供资源和货物入口。时点阶段按真实边界确定，不将终态 `returned` 用作历史时点阶段。空间轨迹未声明时明确显示未知，不按经过时间推算位置。没有修改公共 API、Schema、生产计算或独立批次源码。

运行 `node --test tests/planner/runtime.test.mjs tests/planner/timeline/timeline.test.mjs`：27/27 通过。新行为的修复前回归失败、修复后通过；[原始记录](../../.work/single-window-20261018/p204-native31/runtime-and-timeline-after.log)。candidate32 前端类型检查/构建及 Native/Bridge 构建均退出 0，见[构建元数据](../../.work/whole-packages/p001-20261017/portable-candidate-32-nfr/portable-build.json)。

## 普通 candidate32 复验

普通无参数进程 PID28040/window39390050，打开上述真实 Host 保存目录，不在 Native 重新计算生产。

| 观察时点 | 运输阶段 | 同一货物实际持有者 |
|---|---|---|
| 370 秒 | 等待目的接收 | transport / trip:6:SUPPLY1:5 |
| 419.999 秒 | 等待目的接收 | transport / trip:6:SUPPLY1:5 |
| 420 秒 | 返程中 | location / shell-buffer |
| 425 秒 | 已返库 | location / shell-buffer |

四条运输记录为发出 360 秒、到达 370 秒、接收 420 秒、返库 425 秒。资源入口显示实际 T1 定义、vehicle/carrier、1 vehicle、ALL 日历及声明 home `cnc-out`；home 不代表运输中的当前位置。货物入口返回同一身份、370 秒真实持有者和未定位说明。见[运输等待详情](../../.work/single-window-20261018/p204-native31/Native32/28-holder-370-visible.png)、[接收边界](../../.work/single-window-20261018/p204-native31/Native32/34-holder-420-visible.png)、[返库边界](../../.work/single-window-20261018/p204-native31/Native32/37-holder-425-visible.png)及[货物返回](../../.work/single-window-20261018/p204-native31/Native32/46-cargo-return-visible.png)。

通过 Native 保存到独立 `Native32/saved-r2`：30 文档，当前 Project 和两份 Scenario 完整相等；16 模块、两 Run 的 8 个冻结文件和 1 个支撑文件共 25 个文件逐字节相同。既有 `project_store::retained_paths` 另保留被冻结 Project 引用的两份旧代 Scenario，角色为 supporting_file，字节相同；当前输入索引仍只有原两方案。保存检查最初未区分这些合法旧代支撑项而失败，修正检查器以核对明确角色及完整预期集合后通过，产品保存代码未变。见[保存核对](../../.work/single-window-20261018/p204-native31/Native32/save-validation.json)。

## Native32 输入、真实冲突与新进程回看

当前基线方案的到料标记从240秒预览到300秒，实际公共Host读回仍为r2/240秒，预览未自动提交。通过Native面板明确绑定实际Bridge，仅授权P1的title字段；真实标题编辑将工程提升到r3。旧预览提交收到RevisionConflict，候选保留、到料仍为240秒。显式刷新后重新预览捕获工程/方案r3，Native确认应用一次得到r4/300秒；测试标题恢复后工程r5。完整Project只增加修订号，基线仅改该到料时点，延迟方案业务不变。方案独立计数，基线r5、延迟r4，所属工程引用均r5。见[冲突保留](../../.work/single-window-20261018/p204-native31/Native32-input/44-conflict-visible.png)、[重新预览](../../.work/single-window-20261018/p204-native31/Native32-input/48-current-preview-visible.png)及[完整状态核对](../../.work/single-window-20261018/p204-native31/Native32-input/state-validation.json)。

Agent面板里的临时Bridge路径保留，Ctrl+S未执行保存，没有丢弃本地输入。通过Native对话框授予新目录单用途令牌，再由同一Native Host公共MCP保存r5：task:3 completed、saved5/current5/still_dirty=false。归档32文档，当前输入与完整Host相等，原27个非当前文档和2份旧代方案字节相同；两Run完整读取与最初实际结果相等。首个测试客户端曾因只读请求字段错误而正常退出，没有修改工程，不算产品缺陷；正确会话重新获得明确Native授权。两次Bridge均退出0且已不存在。

同目录candidate32二进制副本入口启动普通无参数新进程PID35764/window67702390，读回300秒输入，历史仍仅原两Run；A显示冻结r1，同屏条件仍是A240秒/B420秒，没有用当前草稿反填。从439秒回看，观察到453.2秒，暂停于465.147秒，两次间隔28.785秒的观察一致；这不是NFR或帧率测量。经普通Native Ctrl+S独立另存，全部32文档字节相同，读取进程正常关闭、未强杀。见[冻结比较](../../.work/single-window-20261018/p204-native31/Native32-input/readback/21-frozen-compare-after-input-edit.png)及[回看保存核对](../../.work/single-window-20261018/p204-native31/Native32-input/readback/validation.json)。

## 后续实测缺陷：保存完成后状态漏刷

原Native32 Host保存任务已确认completed/still_dirty=false，Agent工作台仍显示有未保存修订，显式刷新后才清除该提示，临时输入保持。见[刷新前](../../.work/single-window-20261018/p204-native31/Native32-input/67-producer-active-after-save.png)和[刷新后](../../.work/single-window-20261018/p204-native31/Native32-input/69-producer-explicit-refresh-settled.png)。后台窗口未激活时的一张截图与UIA不符，已排除该图片。

AgentWorkspace.tsx既有观察未比较dirty，且连接结束后不再读取工程；已将dirty纳入变化判断，并在已观察过授权连接的同一工作台继续读取，覆盖保存完成及紧接着断连。没有新增服务、重试、依赖或契约。candidate33类型/前端构建已通过，Native/Bridge构建继续，普通Native自动刷新复验尚未完成。见[修复状态](../../.work/single-window-20261018/p204-native31/Native32-input/save-observer-repair.json)。输入及回看限定通过与此待复验项分别登记。

## 继续与保留项

- 下一步先复验candidate33保存状态自动刷新，再继续P2-04多输入联动、吸附及等待段与两画布定位；原双厂房IX01完整对象、等待、物品和人员呈现仍需单独验证。
- 正式 NFR04/NFR09、目标机与缩放条件、完整 IX/SF/release 及真人验收继续开放，本轮不代填。
- P2-01 candidate31/PID29436 的未提交测试输入和副本解组保留待动作前确认；本轮没有放弃输入或解组。另一个同路径空进程 PID33444 保留，未采纳工程或编辑；相同应用身份造成的工具截图/输入歧义不计产品证据。
- 本轮只由 W00 执行；四个独立批次的 simulation、packaging、reporting、module-kit 与 templates 生产源未写入。所有逐步 UI 记录、原截图和限定结果统一见[验证 JSON](../../.work/single-window-20261018/p204-native31/Native-validation.json)。


候选33保存提示修复已由普通Native新进程PID34732/window22088092复验。r5仅标题改为r6后恢复r7，同一实际Host保存任务task2完成saved7/current7/still_dirty=false；无需手动刷新即清除“有未保存修订”，Bridge正常断开后保持正确，临时本地Agent输入保留。32文档中29份非当前文档、两个原Run完整字节保留，当前两Scenario业务输入不变，仅修订引用推进。保存完成先于断连，未另测断连先于保存完成的竞态。见[实际复验](../../.work/single-window-20261018/p204-native31/Native32-input/Native33-save-observer/validation.json)。下一步继续原多输入联动/吸附及等待定位；整包、IX、NFR和真人门均未代填。


新增实际缺陷：candidate33普通Native联动选入到料300秒和任务0秒后，数值被第二项覆盖为0、预览却按首项锚定而报负时间；尚未提交任何命令。已修RunTimeline.tsx，加入后续项保留数值、拖动项成为明确锚点；22项投影检查和18项真实React组件浏览器流程通过，提交仍为测试替身，不当Host/Native通过。candidate34前端类型/构建已通过，Native构建及普通Native复验待完成。见[实际缺陷与修复状态](../../.work/single-window-20261018/p204-native31/Native32-input/Native33-multiinput/anchor-repair.json)。


联动锚点修复已完成candidate34普通Native复验。P2-04时间轴输入/物流继续。本轮candidate33实际公共Host保存提示自动刷新复验已通过。candidate33实测新增联动项覆盖数值锚点的缺陷已修RunTimeline.tsx；22项投影检查、18项实际React组件流程、candidate34前端/Native/Bridge构建及普通Native PID43564/window49286642复验通过。先选到料300秒后加入任务0秒保留300000毫秒；拖动任务成为锚点，15.422秒候选再数值预览完全一致；精调15秒后r7→8单次提交到料315秒/任务15秒，订单释放0/交期600秒未改；Native撤销r9/重做r10/再撤销r11准确恢复。相同141px拖动吸附开启取350000毫秒、关闭保留351084毫秒，均已取消；Alt、日历/明确参考/业务边界完整吸附未验。r8及r11分别Native另存各32文档，完整Project除修订相同、两Scenario仅明确输入/修订差异，29份非当前文档及原两Run文件字节保留。当前r11已保存；旧candidate33/candidate32临时Agent本地输入及P2-01旧PID29436待确认项均保留。继续原等待条件/两画布定位及其余吸附细节；完整双厂房IX/NFR/真人门仍开放。见[本轮证据](../../.work/single-window-20261018/p204-native31/Native32-input/Native34-multiinput/validation.json)。


候选34又已实际验证原延迟Run1:3/370秒的运输等待条件、shell-buffer阻断库位及下一边界未知保持；两张画布保留同一Run/时点/库位。运输等待的真实引用为job/trip，右栏仅匹配transport而空白；已改Workbench.tsx按当前Run.transport的精确ID复用原运输详情，未改Wait/Host/Engine。candidate35普通Native复验已通过：等待job精确打开同一冻结运输，370秒为等待目的接收，货物仍由该trip持有且空间未知；新目录保存32份文档全部字节未变。原等待前后事件、完整吸附与双厂房验收仍未完成。见[实际等待记录与待验修复](../../.work/single-window-20261018/p204-native31/Native32-input/Native34-multiinput/wait-inspector-repair.json)。


本轮范围校正：原IX-01的project.design.json明确H1一个厂房和5台设备，任务包要求同Run时点在两画布联动，不是双厂房输入。先前“完整双厂房IX”措辞混入了P2-01场景；不再作为IX-01额外前置。原P2-01多厂房要求及全部原IX/NFR/真人条件各自保留。

candidate36已补现有时间轴的等待时点保留资源/阻断资源持有者/同Run物品持有者与关联事件导航，以及只读班次开放行、可见班次边界吸附、对齐线/整数毫秒/Δt和显式参考取消。24项单元与19项真实组件流程通过；提交响应为明确测试替身，普通Native仍待复验。记录见[限定检查](../../.work/single-window-20261018/p204-native31/Native32-input/Native36-wait-snap/pre-Native.json)。


candidate36普通Native已通过等待持有/前后关联事件及419999/420000/425000边界、同Run两画布定位、冻结参考370000/班次600000/到料300000吸附的对齐线与毫秒Δt；全部预览取消，另存32份文档字节全同。继续原两Run联合核对，显式事件参考和Alt普通Native仍未验；原P2-01多厂房与完整IX/NFR/真人门另列。 前关联事件#285为该运输370秒变化；后关联事件#292为阻断库位在420秒的另一件物品移交，不能误称本运输货物自己的移交。输入提交仍仅作取消预览，未新增Command或Run。见[普通Native36结果](../../.work/single-window-20261018/p204-native31/Native32-input/Native36-wait-snap/validation.json)。


候选36追加实际事件#285参考吸附并取消，基线440秒序号375、延迟570秒序号379接受事件各6个实际身份可定位；两Run比较冻结到料240/420秒，未使用草稿300秒。见[追加联合核对](../../.work/single-window-20261018/p204-native31/Native32-input/Native36-wait-snap/joint-readback.json)。本包所有原完整门保持；本窗口按实际可用技术前置进入P3-01公共接入。
