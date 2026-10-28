# 50种原生设备建模与定义包

2026-10-21按最新首版基线、纸箱厂候选定义、现行Presentation/ModulePackage和设备建模规范扩充。共50个独立设备定义：33种纸包装及配套设备、17种常用设备。每种有详细/简化两档，共100个被选择的GLB、100个精确媒体包；加上设备定义共150个包。托盘、物品、LOD与同一型号的多实例均不计作新增设备。

[完整设备、物流、阶段与定义ID清单](设备清单.md) · [机器可读清单](catalog.json) · [实际图册](../../../scenarios/媒体表现/native50-20261021/gallery.html) · [50种实模总览](../../../scenarios/媒体表现/native50-20261021/50种设备实际模型总览.jpg)

## 本轮实现

- 首版纸箱厂的可选印刷、DieCut01模切、FoldGlue01折糊、Inspection01检验及纸箱捆扎/接受路线均有对应设备模型。其余纸包装配套包括送纸、升降上料、上光、固化、覆膜/裱纸、分切、压痕、清废、成型/开箱、贴窗、钉箱、涂胶、压合、计数、热收缩、装箱、封箱、贴标与托盘缠绕。
- 常用设备覆盖直线/弯道/辊筒输送、顶升移载、垂直升降、六轴/SCARA/龙门搬运与码垛、装配/返工/检验、铣床、车床、钻床、压力机与检重。采用现有Engine的transform/assemble/inspect_and_route/rework/pack；没有新增库存、运行时钟或专业求解器。
- 7个成熟L03机型保留当前0.4.0/0.4.1媒体精确版本、完整网格和可重建源；43种新增参数化建模。未覆盖旧目录或修改已打开工程采用的精确版本。外观可共享，设备ID、物料角色、阶段、周期与动作绑定分别保存。
- 新模型具有完整机架/脚垫、型材或壳体、传动/执行机构、控制面板/急停、传感器与支撑、接线盒、原创铭牌及分材质表面。活动件有独立命名层级；辊筒绕各自轴旋转，机器人保留其五个阶段，封箱机保留入箱/封口/出箱阶段。没有用整机位移冒充工序。

全模型单位为m、右手Y-up、原点落地；领域坐标仍由现有产品转换。尺寸、周期和默认用料是声明示例，不是厂家实测产能；辅助耗材默认一个预配用料单元（piece），工程采用时明确绑定物品、单位与数量。装配台明确领用壳体1、PCB1、紧固件4；搬运/返工保持身份，装箱建立包含原物品的新包件，检验等待显式质量输入。

## 实际检查与适用范围

[包与端口/阶段检查](../../../tools/3d-proof/native50/package-checks.json)读取实际100个GLB，并核对150包精确版本、依赖、文件、端口锚点与工序阶段；真实当前Rust Host注册后，`query_module_library`实际返回50个设备、100个媒体条目，见[完整Host返回](../../../tools/3d-proof/native50/host-library-result.json)。该Host在独立进程/独立target运行，不操作正在使用的Native工程。

[实际呈现检查](../../../scenarios/媒体表现/native50-20261021/render-checks.json)调用当前`packages/presentation`的`decodeGlb`和`bindScene`，逐个重开100个选定文件，检查各声明阶段0%/50%/100%和反向seek。450张画面覆盖双LOD斜视/背面/底部，以及详细档机构近景关键姿态。此审阅器不生成生产结果。

独立复制单包源重建结果见[源码重建检查](../../../tools/3d-proof/native50/source-rebuild-checks.json)。新43种使用Three.js参数化网格与原创2K微表面，成熟7种沿用原可编辑源；没有Blender调用、付费生成或厂家模型下载。截图/图册、导入成功及重建检查不代签MQ-07/08的Native真实持有、四类等待、保存重开、性能或人工认可；这些逐设备联合接收仍为pending。

## 注册与独立重建

[registration.json](registration.json)是精确ID/版本/目录序列；[register-directories.txt](register-directories.txt)按简化媒体→详细媒体→设备排列，供共同Native启动入口逐项注册。每个设备只依赖自己的两档媒体，采用一台设备不会拉入其余49台。共同Host入口变更另交拥有者；[最小接线补丁](../../../tools/3d-proof/native50/native-host-integration.patch)已经准备，是否合入以该文件的当前拥有权与用户授权为准。

从软件工程根运行，无需安装新依赖：

```powershell
# 使用已有带Pillow的Python运行时，或设置SF_NATIVE50_PYTHON为该运行时。
node modules/media/native-equipment-50/source/generate.mjs
node tools/3d-proof/native50/server.mjs
cargo run --offline --locked --manifest-path tools/3d-proof/native50/host-check/Cargo.toml --target-dir .work/native50-20261021/target -- "$PWD"
node tools/3d-proof/native50/check.mjs
```

审阅器：[http://127.0.0.1:19290/tools/3d-proof/native50/index.html](http://127.0.0.1:19290/tools/3d-proof/native50/index.html)。选择型号和LOD，可自由旋转、查看背底和按实际阶段拖动。此独立模型工具不更改产品前端布局。

单包复制后从包内执行`node source/generate.mjs`重建；新包的`source/rebuild.json`选定型号/LOD，成熟包沿用其原`source/parameters.json`。所有声明资产、重建代码、参数和许可随各包列入`module.json`；依赖使用项目现有锁文件，不分发本机可执行程序。

## 设计依据与许可

工艺及设备类别参考[BOBST纸盒加工说明](https://media.bobst.com/en/news/detail/article/1768493402-the-essentials-of-carton-converting/)、[Kongsberg切割/压痕工作台](https://www.kongsbergsystems.com/en/digital-cutting/cad-table)、[Endoline开箱机](https://endoline-automation.com/erectors/)与[Robopac转台/膜架设备](https://www.robopac.com/en/business-units/robopac-machinery/vertical-stretch-wrapping)。只用于区分类别与工序，不复制厂家几何、商标或标定参数。

新增几何、微表面、铭牌和参数为原创CC0-1.0；成熟模型保留原包许可与来源。Three.js为现有MIT依赖。新模型质量以[现行MQ规范](../../../docs/02_交互设计/设备建模精度与表现验收规范.json)及实际审阅接收，不用面数、文件大小或资产数量替代验收。
