# DRF03 九类采用、版本与专业成果闭环完整定义

2026-10-14。原任务P0-03（仅首版专业绑定部分）、P3-01、P4-02；关联S03-03/S05-03/S06-04/S06-05。**本窗口定义与实际适配已完成；C01—C04公共增量和正式消费者均已复核，真实输入与产品运行关口保留。** 不增加17SF、21P、60S或Q01—Q64身份，不恢复Figma或启动产品编码。

[完整机器定义](DRF03_完整定义.json) · [缺口及修正](DRF03_缺口与修复.md) · [精确公共交接](DRF03_公共消费者交接.json) · [实际静态/有限参考](../../../scenarios/九类模块采用/DRF03_静态与有限参考结果.json)

## 唯一字段来源与九类采用

[原机器协议](../../../packages/module-kit/category-contracts.design.json)保留61个载荷字段和10个共同字段，逐项补充canonical载体、单位、引用、验证、实例化、原设计样例、正式形状样例和版本影响。具体Rust字段从[原映射](../../../packages/domain/module-field-bindings.design.json)取得；不复制另一份公共Schema。

正式文件为`Versioned<ModulePackage>`：分类在`/data/definition/payload/category`，类别数据在`/data/definition/payload/data`。旧`modules/**/manifest.json`是平铺设计例，不能仅改后缀就当正式模块。旧秒单位、dot/bracket参数路径和spec_ref字符串通过明确PREP适配；运行参数位置按RFC6901，且只能写已经声明的公开叶字段。

|类别|采用/实例化的实际含义|关键验证与预览|版本/导出|
|---|---|---|---|
|items|建立/明确复用ItemType；初始物品另由显式ScenarioInput给出|unit、tracking、规格精确版和属性schema一致；预览规格/外观|类型不是库存；不导出当前持有者|
|devices|显式DeviceInstance，冻结module_ref、pose、configuration、role_bindings|物品/信号/artifact端口分开；inline或operation_ref只选一处；预览读同一规则与表现|只升级所选实例，保留覆盖；旧Run不重绑|
|resources|显式ResourceInstance/Calendar；选择实际实例供角色复用|人数/车辆数/装载量分开；资格、容量、半开日历明确|定义不带claim/当前占用；服务执行仍后置|
|tasks|采用精确任务及目标/资源映射，接入原就绪/准备消费者|每步作用、资源、整数ms、shared/exclusive；准备不产料|逻辑instantiate_as不等于新增公共对象，公共接线见C01|
|operations|DeviceInstance.operation_binding或RouteStep.operation_ref|输入输出角色/单位/阶段/质量/身份规则；共同Engine执行|配方、阶段或端口变化列采用差异，不能双执行|
|transport|绑定实际运输设备、载具、源目的和货物|装载/出发/卸载/接收/返回分别定义；动画不提交接受|保留货物ID和旧Run；固定时间首版，交通求解后置|
|templates|InstantiateTemplate提交对象及映射；内外引用分别处理|内部ID连同端口/连接重映射；显式复用外部资源|既有工程默认不带库存/需求；新方案种子单独确认|
|professional|定义采用、实际计算、人工认可、消费者绑定四次明确动作|实际available_modes来自Host；检查真实输入/输出/范围/单位|首版真实求解仅纸包装；软件在环及外部连接器后置|
|media|绑定默认表现或实例presentation_override|支持格式/授权文件、节点/端口锚点、动作/标签及LOD|替换外观不改变工时、库存、规划包络或专业几何|

每类可独立创作、检查、预览、采用和导出；没有生产含义的类别不伪造“运行一周期”。九个设计样例与九个正式类型夹具各自保留身份，夹具通过不表示旧包已能在产品中导入。

## 单位、引用和实例覆盖

- 类型/物品数量沿Quantity单位；piece与kg不能自动换算。载具装载量不等于车辆数。只有实际消费者使用的属性才影响运行，其余只保存/展示。
- 时间进入现行DurationValue时为整数ms：1.25秒明确转换为1250ms；0.0001秒形成0.1ms，不能静默取整。日历、运输和工艺分别解释，不把专业计算墙钟耗时当生产工时。
- 规划厂房用m，纸包装用mm/Z-up；GLB源为米/Y-up，须显式坐标变换。例如绕X轴+90°后，源(1,2,3)m为(1000,−3000,2000)mm。显示scale=2不把实际100mm产品改成200mm。
- 定义精确版本、DefinitionScope修订、Project修订、产品revision、Job修订和RunId互不替代。实例覆盖只改所选实例，不回写库默认；同一已冻结(id,version)的不同内容拒绝覆盖。
- 引用按身份/版本/类型解析，不按显示名匹配。资产只在用户授权包/目录内解析；上传声明不取得脚本执行权限。

## 实际调用与事务出口

|动作|沿用的真实契约|返回与失败处理|
|---|---|---|
|能力/选择|describe_capabilities、query_module_library、get_context|只取实际能力、精确候选和scope修订；空能力显示未支持|
|导入|import_package→get_task→ImportDraft；明确AdoptImportedDraft|暂存不改工程；取消/失败保留缺项，不先建半份实例|
|创作|apply_commands中的CreateDefinitionDraft/PutDefinition/SetDeclaredParameter；get_object；适用render_view|EditCommit/issues；没有第19个check_definition操作；草稿可保留缺值|
|采用/升级|同一apply_commands批次AdoptModule/InstantiateTemplate及所选PutObject|一个原子Project修订；角色/单位错或RevisionConflict不部分提交，保留预览供刷新后显式重用|
|专业计算|PutPackagingJob后compute_packaging→get_task|冻结Job与预算；Fail/Undetermined是正常专业结果；取消不写完整Pass|
|人工用途认可|可信人工通道RecordArtifactAcknowledgment|只认可指定artifact/产品/用途；Agent不能靠提交HumanStatement冒充人工|
|绑定|adopt_artifact→BindingCommit|校核实际来源、版本、当前条件、用途、内容和现有人工记录；绑定不产库存|
|运行/查看|run_scenario→get_task/get_run|同一冻结RunInputRef；旧Run不读当前库latest；未计算区域不补画|
|导出/重开|export_package→get_task；save_project沿原入口|真实文件完成才称导出；另一目录重开、权利与依赖须实际检查，不自动发布|

原API/编辑命令有载体不等于Host已经实现。AdoptModule.targets的逐类效果、旧实例升级原子语义由C01交W00冻结；当前窗口不擅改共同Commands。

## 专业成果的四个角色和版本影响

[生命周期机器规则](../../../packages/packaging/adoption-lifecycle.design.json)直接消费现行Rust。ArtifactRecord保存真实输入/结果/产品版本/单位/范围；ArtifactAcknowledgment保存人的指定用途认可；ConsumerBinding指定工位/工艺及采用范围。首版实际专业输出是PackagingResult，工厂输出是RunResult；旧逻辑execution_result不新增成一个首版公共API对象。

|变更|成果/认可|当前消费者|旧Run|
|---|---|---|---|
|仅实例标题、媒体/LOD|原几何成果不变，不自动新增认可|业务条件未变则绑定保留；新表现需明确版本|保留原输入/表现|
|产品r1→r2|r1成果/认可保留，不改写为r2|仅不匹配r2用途的绑定需复核；r1旧订单可继续使用|仍消费r1|
|材料/厚度/板形/折带/路径/净空间|新Job及新artifact，禁止复制旧认可|即使产品仍r1，也比较当前所需输入条件|旧结果及旧认可不回填|
|预算/数值口径|新冻结计算输入，不能推断新Pass|实际选择旧相符成果或新结果，不能自动用latest|原预算/未知原因保留|
|工艺版本/所需内容/用途|不自动扩认可范围|重算required_payload/expected_subject_revision，按精确用途集合匹配|原工艺仍冻结|
|工位/目标配置|原成果不自动改变|若认可限定工位/姿态等条件，只复核受影响consumer|旧目标/绑定保留|
|定义升级|不改旧成果|只更新明确所选实例；单位/角色不相容整批不提交|不重绑新库版本|

绑定可用性为当前上下文的派生结论，不在ArtifactRecord捏造`accepted`、`stale`或`needs_review`布尔。认可A2不能授权A1，r1的Pass不能授权r2，只有相同产品版本也不足以忽略厚度等条件变化。当前工位通过两个已声明configuration键独立选定期望Job，精确推导见第二轮C02，不能从成果反推期望。当前AdoptArtifactParams没有required_payload字段，不能由客户端补一个字段绕过；由目标工艺推导的共同规则明确交C02。

## 纸包装全过程与三态

原[过程算法](../../03_架构与契约/纸包装结构与过程算法.md)、[数值/制造规则](../../03_架构与契约/纸包装数值与制造规则.md)及[S06-04证据](../S06-04_纸包装算法与数值方法复审.json)继续是依据。受限容器、外盒—内衬、罩/滑套以及空白受限结构入口保留；产品功能、材料认可、部件关系/明确重复三变化保留。

执行链为明确输入/能力→结构与字段验证→成型/连接→净空间→开合→固定姿态直线取物→制造/可编辑输入→实际成果→人认可→指定工位绑定→Engine实际包装。几何通过、认可和绑定均不创造包材、产品或工时。

以下汇总针对一个确定候选/已选择结构；多候选搜索仍沿S06-04，找到完整Pass候选可交付它，一个候选Fail不否定其他候选，未覆盖域不能宣布全局无解。

1. 所请求硬条件出现已证违反即Fail；保留其他未完成项，不因预算耗尽抹掉反例。
2. 没有已证Fail但缺必需目的覆盖、输入、精度、预算、制造内容或可编辑源时Undetermined。空checks/少几个Pass不能依空集all()放行。
3. 只有所有明确所需目的均被覆盖并通过、实际内容齐全，才有对应范围Pass。用途与Job必需条件、目标工艺共同决定要求，不能由请求者少勾制造来假称完整工程通过。
4. 物理几何不满足是专业Fail；畸形引用/单位/域错误沿原InvalidInput；取消/未执行不写完整结果。人工认可不能把Fail/Undetermined改成Pass。

制造按显式part形成cut外边、实际折带fold和生效双面共同区域glue；没有折带/胶接的结构可明确零需求，有胶接却无局部胶区必须未知。纸材面积mm²、显式份数，不从订单数量放大。ArtifactRecord.input_ref对应冻结可编辑Job与实际依赖，SVG不能代替结构/过程源。

原32项几何/制造预期、6项保持预期、16数值原语、6构造Job及SVG有限证据不重写。六Job Schema不等于三机制完整几何；cover样例不等于滑套闭缝已验证；工业材料性能、完整Rust求解内核及实际工厂绑定仍待。

## 时间轴、视觉状态和返回

严格消费[主轴/子轴定义](../../02_交互设计/时间轴主轴与子轴完整定义.json)。工厂轴为同一Run整数ms与共享view_time；设备媒体按真实阶段有效工作进度；纸包装模型预览用冻结Job的step_id/u。三者不混用。

有限例：有效工作段[10000,20000)、[30000,50000)，总工时30000ms，在Run时点40000ms完成20000ms有效工作，进度2/3；等待不推进动作。纸包装duration_hint_ms=12000不能改工位30000ms工时或共享view_time。时间轴TC01/TC02、computed_until_ms及Events末刻语义已经窗口1合入，本窗口只读timeline_runtime_contract，不复制阶段历史或重复计pending。

第一轮记录的process_poses身份缺口C03已由窗口1合入实际PackagingProcessSample/PackagingPanelPose，本窗口读取当前Schema完成正反复核。模型预览及实际返回样本均按step/progress/panel ID解释；NamedPose仅属表现，有限样本仍不能给连续碰撞Pass。

F01—10/F19需呈现来源/版本、单位、缺项、库默认/实例覆盖、新建/复用映射、升级差异、三个专业身份及证据层。空态返回库选择；缺项保留草稿；加载可取消回原选择；不支持明确缺能力；采用预览取消零提交；冲突保留差异并显式刷新；非Pass回冻结输入的新草稿；旧Run只读定位同Run。71项字段表单映射已接入，具体控件现由窗口2维护，当前窗口不复制UI源。

母版沿窗口菜单控制dock、下方工具、默认160px时间轴和独立描述dock，不恢复旧顶部工具排/固定250px。真实原型、本人走查、桌面接点和产品运行分别保留原门，均未由本定义取得。

## 实际核验和剩余

[22项独立预期](../../../scenarios/九类模块采用/DRF03_有限独立预期.json)全部与[有限参考输出](../../../scenarios/九类模块采用/DRF03_静态与有限参考结果.json)匹配；九类正式夹具通过当前Schema，九份平铺设计清单直接解析被拒绝，证明适配边界存在。71字段与实际生成类型字段对应；原21登记/15采用预期保留，actual_result为null。

运行入口为`scenarios/九类模块采用/DRF03_核对细化.py`，用现有CPython3.12及只读validator依赖。只写本目录DRF03结果，没有产品加载器/Host/前端调用、依赖安装或共享类型修改。

C01升级/目标语义和C02完整推导已登记到公共操作合同，C03公共载体已实际生成并复核；C02真实生成分支与任务输入投影已复核；C04已完成强类型采用材料和输出质量效果的公共合入，并对真实canonical载体及19项预期复核，详[当前交接](DRF03_C04_r4_交接.md)。71字段UI已接入，由窗口2维护，不重复计缺口。它们不新建P/S任务；原S06-05、S07及真实产品验收继续控制后续实施。

规范核读：[SemVer2](https://semver.org/spec/v2.0.0.html)第1—4/10条、[RFC6901](https://www.rfc-editor.org/rfc/rfc6901.html)第3—4节、[glTF2](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)坐标/单位与name/资源边界。项目的精确版本固定、允许字段和支持子集是项目决定，不能归因于标准自动保证。
