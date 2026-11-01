# DRF01 时间轴、双画布与设备创作完整定义

<!-- equipment-model-quality:start -->
## 设备建模共同基准

详细设备统一按[八设备同等级建模规范](../../02_交互设计/设备建模精度与表现验收规范.md)：完整背底、关键机构与装配连接、同等级材质细部、命名活动部件及实际阶段动作。设备详情和流水线详细档一致；简化/区域LOD、插图接入或方块教学样例不降低详细档要求。面数/节点数不是独立通过条件，资产认可和产品实测仍据真实结果。
<!-- equipment-model-quality:end -->

2026-10-14。本文件为现有设计消费者入口；原17SF/21P/60S保持，不改变编码准入或通过数。

[完整JSON](DRF01_完整定义.json) · [缺口与修复](DRF01_缺口与修复.md) · [时间轴TL01—12](../../02_交互设计/时间轴主轴与子轴完整定义.md) · [派发表](../重要特征细化统筹与派发.md)

## 可执行特征定义

|特征|输入／来源|规则与输出|异常／返回|
|---|---|---|---|
|双画布与中央详情|project_id/input_revision/run_id、Hall/Device稳定ID、相机/LOD与ReturnContext|两常态画布保持；设备/专业/比较详情替换中央工作面。Back恢复父画布与相机/滚动；切工程加载新context，不借旧ID重绑。 明确归属的中央工作面与可返回入口|脏表单先应用/丢弃/留在；RevisionConflict保留字段；对象删除提供原Run查看|
|主轴/组与设备子轴|冻结工程分组、RunResult与TC01/TC02、IntervalMs；毫秒整数|组唯一成员、B/W/O可重叠；分母保持全成员，筛选另示匹配数。子轴与主轴同Run/确认时点，局部窗保持源区间/ID；部分结果端帽不声称完成。 厂房→显示组→设备最多三层及同源细化|无法绑定列未归属/未知；无阶段历史不画名义工时；共享资源占用按真实resource ID一次计数|
|输入拖动、吸附与联动|同ScenarioInput字段、base_revision、Arrival/Order/ProductionTask释放时间；ms；6CSSpx吸附阈值|候选按屏幕距离→明确参考/业务边界/网格→稳定ID；Alt关闭吸附。联动同Δt，不改变真实等待/Run，交期仅明确纳入；pointercancel/Esc不提交。 预览旧/新值与一次PutObject ScenarioInput提交|RevisionConflict/InvalidInput保留预览无部分提交；实际条段没有输入编辑手柄|
|设备定义、实例与表现编辑|F01/F03/F10/F17；module_id/version、DeviceInstance.id、端口/资源绑定；部件局部坐标、图像归一化锚点|外观草稿不能获得产能；实例不复制库定义库存；采用新版本列端口/行为/包络/标签影响。图片/姿态重排只改Presentation；多阶段每phase内u∈[0,1]，等人/中断保持有效进度姿态。 可编辑定义/指定版本实例、明确升级差异与待补项|删除引用列依赖并改绑/删除/保留待补；不存在能力显示未支持；素材失败保留草稿|
|运行查看、标签与等待解释|get_run ObjectsAt at_ms匹配回复、PhaseWorkInterval、WaitInterval与真实holder；quantity有物品单位，ratio有效进度，duration ms|数据标签只读，完成/移交/订单接受各自口径；没当前物品显示无当前物品；零时长phase沿完成事件取终姿态。迟到旧Run回复不得提交。计算与回看控制分离。 同Run/时点的3D、时间轴和描述；当前需要/持有者/下一已知边界|未知/未计算不查询超界或借final_items；Task取消等真实终态|
|dock、键盘与原型交付|visible flags、previous size/window/scroll/selected_event_ref；CSS px|描述左/时间轴右；单dock铺满、双隐藏释放中央；选事件不自动重开描述。Enter执行聚焦按钮，不无条件开描述；显式重开恢复布局和最新选择。 可预测显隐与可访问的选择/返回|偏好保存失败会话保留并标未保存；未得可操作原型不能填写已通过|

## 六类视图

- **任务与交互**：单击详情、手柄拖动、相机修饰键与Ctrl多选优先级固定；详情在中央替换工作面，返回恢复两画布上下文；缩放只变相机/LOD。dock统一由窗口菜单显隐。 来源：navigation#/task_routes and pointer_policy; forms F03/F08/F10/F11/F16/F17/F28；消费者：factory_ui/workbench and ui_host。
- **结构与数据**：主轴/子轴使用同Run和毫秒绝对时点；局部时间窗仅裁显示；phase、事件、作业身份保持。一个shared selected_event_ref，AxisView只投影；表现分段取phase_id和有效进度。 来源：timeline#/private_view_records; forms; frozen RunInputRef and DeviceInstance；消费者：TimelinePanel/TimelineInspector/TimelineDescription/DeviceWorkbench。
- **状态与行为**：输入草稿、临时预览、待提交、冲突、实际跟随/回看分开；隐藏描述选择不自动重开；换Run清空临时预览、旧选择和旧定位归属。 来源：timeline#/ui_transitions; navigation#/editing_policy；消费者：ui_host and Commands/Host。
- **接口与协作**：输入编辑一次apply_commands并校验base_revision；运行时点通过get_run ObjectsAt取得匹配同Run/at_ms后提交；计算只取get_task.allowed_controls。configure/inspect/preview_cycle是设备入口，不能绕过Commands。 来源：18 existing operations; merged canonical timeline_runtime_contract；消费者：same Rust Host via existing UI invoke/MCP adapters。
- **界面与表现**：工业浅灰外壳、顶菜单、底工具/状态、可伸缩左右面板、中央工作面；描述左/时间轴右，两dock独立显隐；全厂组/设备轨道与设备三条表现编辑轨分开。 来源：master and TL03/TL05/TL09; F10/F16/F17；消费者：S04-02 frames and S04-04 clickable prototype。
- **部署与验证**：沿既有React/TS/Three与SVG/HTML选择，未新增依赖；静态与DTC01—03数学已有检查，DTC04—11与E01—E08等待真实可操作证据；本人走查不由AI代填。 来源：TL11/TL12; S04-04/S04-05/S05-05/S06-05；消费者：W00 readiness and later admitted implementation。

## 已合入公共类型与剩余实际证据

TC01/TC02公共设计类型和13项有限契约试做已合入，[唯一公共规范](../../03_架构与契约/全层类型与工程格式.json)为timeline_runtime_contract。实际Engine/Host生产及UI消费仍未实现，不把母版缺项改成真实运行。续算以新快照替换旧临时截断段，DTC11预期不重复累计；Summary/FullResult须为同一冻结输入与边界。

71项九类字段规则逐项映射原111字段组，8个DRF06时间轴实例按manual_expected_not_Run接入F16/F28，均无真实Run/实际结果。比较/导出同源展示消费DRF05-H07，仍使用既有两种报告mode。

现行母版20页，160px默认/320px展开、描述300px；M16为DTC02手工预期，M17缺历史，M18为E04预览，M19三厂供货，M20独立u表现轴。24帧五链、可点击/可输入/可返回原型及本人走查分别验收，Figma继续暂停操作。

## 公共交接当前状态

|交接|当前状态|内容／消费者|
|---|---|---|
|H-DRF01-01|已合入公共设计／有限证据，实际运行待验|按TC01/TC02实际合入公共运行字段并统一Rust/Schema/TS/API、阶段算法与消费者；合入证据回写DRF02，不因派发改变原准入数。|
|H-DRF01-02|待拥有者整合|新24帧/母版落实window.timeline/window.timeline_description、描述左/轨道右及hidden selection；同步SXX生成、共享视图状态、缺口清单中的旧“视图菜单”文字；保留真实证据缺口。|
|H-DRF01-03|待拥有者整合|device-editor-capabilities.design.json顶层timeline.clock/duration_source须明确按binding_modes选择，不能把多阶段一律写成behavior.duration；phase_local每阶段分段、零时长用PhaseCompleted、等候保持有效进度。现binding_modes已具基础语义，完善顶层避免读取歧义。|
|H-DRF01-04|已合入公共设计／有限证据，实际运行待验|公开调用文档明确既有Events interval:null不加时间过滤、仍受scope与分页权限限制；持续IntervalMs半开和观察窗末刻事件闭包分别解释，报告指标同源。无需新增公开操作。|
|H-DRF01-05|已合入原验收引用，实际执行待验|按业务责任接入既有17个AC，不增身份、不改实际结果；输入与预期都添加精确引用，并补对应procedure/expected。保持检查断言，不以跳过案例恢复通过。|

H-DRF01-01/04的当前字段及Events语义只取唯一公共规范；H-DRF01-05在原17个AC中已核对B01—B18输入/预期覆盖，实际结果槽仍为空。H02/H03等尚未标完成的公共消费事项继续由拥有者处理。

DTC01—03数学核对通过；DTC04—11和E01—E08的原型/运行/本人证据仍需实际执行。静态、有限契约、实际原型、产品运行和本人结果分开记录。
