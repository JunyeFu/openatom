# Task8物品可视布局 · 当前r4示例

打开本目录`project/`。这是原Host公开导入、apply_commands、run_scenario、save_project保存并经新Host进程重开的实际ProjectArchive。原`resources/v2-demo/project/`r2及本轮中间r3仍保留，不覆盖旧Run、冻结输入或素材。

- Project `Task8-two-device-six-units` r4；Scenario `Task8-six-units` r4，ID及业务含义不变。
- 设备A/B仍为10/15秒、同L1-SOURCE/L1-SINK、同input/output/workpiece、target6，独立容量各1。
- 详细设备`p201.b60.station@1.0.6`、简化媒体`p201.b60.simple@1.0.6`、试件`p201.b60.workpiece@1.0.2`；容量保持1.0.0。原模型体和`b60-station-1.0.5.glb`字节未变，新旧精确版本都随包保存。
- 源/汇各声明六个`item_id_order`展示槽：0.45m间距、原库位坐标为基准；两设备各声明一个`workpiece`命名锚点槽。它们只表示布局，不增加库存、资源容量或加工进度。
- 四条物料连接的精确PortRef、设备/路由引用和简化LOD模块采用已同步。`candidate-bindings.json`与`profile-bindings.json`均在本目录，Root应成套消费，不混用旧根目录的r2候选版本。

本包三个真实已保存预测Run：

1. 原r2：`task8-build-host-92236:run:1:2`。
2. 中间r3：`task8-geometry-build-host-109760:run:1:2`，保留原冻结内容。
3. 当前r4：`task8-geometry-build-host-112120:run:1:2`。

三个Run均实际Host可读；原预测仍固定设备A，接受时刻10/20/30/40/50/60秒，6件，未伪造成两候选并行预测。当前r4完整采用简化LOD依赖；r3只作为本轮保留记录，不作为推荐入口。

新进程实际重开、旧Run/冻结字节保留、完整精确素材解码，以及实际External Engine/ExecutionProjector下六物品source→A/B实际job holder→sink的槽位与详细/简化三维附着均已检查。观察页面和字节传输使用有限组件夹具，尚不代表Native授权、真实云/设备或完整H01整机通过；物理连续进度仍未知。

原权利声明沿用上级NOTICE及原模块内contribution/rights。本轮仅补布局和确切版本闭包，不重新制作设备模型。复现入口见`tools/v2-demo/README.md`；实际证据与剩余Root工作见`.work/v2-implementation-20261026/demo-geometry/README.md`和receipt.json。
