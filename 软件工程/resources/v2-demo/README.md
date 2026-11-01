# Task8 · 两设备六件演示工程

当前物品可视布局示例请打开 **`geometry-v4/project/`**，并使用`geometry-v4/`同目录候选与profile映射。工程/方案r4已补齐六物品展示槽、精确端口和完整详细/简化LOD闭包；旧r2和全部旧Run原样保留。详见`geometry-v4/README.md`。下文仍记录原r2基线事实，不将旧冻结内容改写成新修订。

打开 `resources/v2-demo/project/`（包含project.json），使用Native原工程目录打开流程。它是实际Host导入、声明参数编辑、run_scenario及save_project生成的ProjectArchive，含真实终态预测Run；不是Engine测试JSON。`source/`仅供重复生成，初始2秒默认值会由Host命令改成下列10/15秒。技术版本保持0.1.0，文档schema为1.0。

|对象|精确ID／配置|
|---|---|
|Project|`Task8-two-device-six-units`，已保存修订2|
|Scenario|`Task8-six-units`，修订2，窗口0—120000ms|
|厂房／产线|`H1`／`L1`，一条产线|
|设备A／B|`L1-D01`／`L1-D02`，10000／15000ms|
|绑定A／B|`binding-L1-D01`／`binding-L1-D02`|
|原料／成品地点|`L1-SOURCE`／`L1-SINK`，两个候选共用|
|端口／角色|`input`／`output`；`workpiece`，单位piece|
|独立执行容量|`L1-D01-CAP`／`L1-D02-CAP`，各1slot|
|工序／阶段|`L1-D01-step`／`cycle`，同一单工序、一个不可中断阶段|
|投入／目标|`blank-0`—`blank-5`六个真实身份，各1piece；计划量及H01目标均6piece|

名义工时由原声明参数 `processing_sim_seconds` 配置。字段名沿用旧名，但原声明unit是millisecond；10000/15000就是10/15秒。两项值均通过Host `set_declared_parameter` 写入，并同步实际 `parameter_sources`（user_provided），未改旧模块版本的规则内容。

## Run与固定绑定事实

实际预测Run：`task8-build-host-92236:run:1:2`。由原公开Project `run_scenario`/Task/get_run生成，并与原冻结工程、方案和历史随包保存；不是外部执行快照。computation_status=completed，whole_window_computed=true，6件接受，接受时刻10/20/30/40/50/60秒。v1路由默认固定设备A，六个实际Job都在A，未人为凑成云端两候选40秒。

计划任务量是6件；H01使用 `profile-bindings.json` 中的 `target_quantity=6piece`。预测Run原`goal_met`保留为 `null`，未把未声明的日产量目标伪填为通过。Root可用已保存Run作T8-A07来源比较。

## 候选、模型与资产

`candidate-bindings.json` 是实际读回Project派生的原typed CandidateBinding数组；`profile-bindings.json` 将Native原 `processor-a/b` 分别映射到A/B，10/15秒与现有Python profiles一致。真实H01才生成session/epoch；示例不含URL、凭据、连接串或boot journal。

模型精确版本为 `p201.b60.station@1.0.5`、`p201.b60.simple@1.0.5`、`p201.b60.capacity@1.0.0`、`p201.b60.workpiece@1.0.1`。全部取自原v1 Native保存工程，模块及GLB字节未变。唯一外部素材 `assets/b60-station-1.0.5.glb` 随工程保存，含196节点及6幅内嵌PNG；无外部纹理URI。详细、简化和试件场景均通过实际Host read_media_asset闭包及生产presentation解析。来源与原权利声明见NOTICE.md和original-source-notice.json；未补造缺失许可证。

## 本批验证边界

原Host保存后，新进程Host重开验证11个required文档、四个精确模块、完整资产、原Run与冻结引用一致。由实际读回数据构造EngineInput并new_external，两候选10/15秒准入通过；错误B=10秒反例拒绝。prepared保持6逻辑工作、6投入、0Job/事件/认可/资源占用。

未启动共享Native、CU、桌面、端口或云。Native打开/普通保存重开、真正H01软件在环及T8-A07整机比较由Root接线验收，本批不把无界面解析当GUI或云验收。

重复生成见 `tools/v2-demo/README.md`；实际回执为 `.work/v2-implementation-20261026/demo-package/receipt.json`。不要覆盖本目录或旧Run。
