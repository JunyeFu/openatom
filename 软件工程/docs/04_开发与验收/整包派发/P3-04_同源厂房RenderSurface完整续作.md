# P3-04 同源厂房 RenderSurface 完整续作

继续[P3-04原完整包](P3-04_整包执行.json)及[九视图真实生产者核定](../../../scenarios/Agent共建/runtime-render-view-source-review/source-review.md)。完整九视图、Host Task、权限、取消、managed图像、GF/IX/NFR/release均保留；本续作完成厂房两画布共用原产品渲染器的只读捕获表面与实际就绪/相机/可见对象接点，交给Root原生捕获接线。不得把本次浏览器证据称作CapturePreview或其余七视图完成。

## 完整目标与现行来源

使用原唯一FactoryScene、factoryMedia、factoryItems与冻结FactoryPresentationSource。新增只读FactoryRenderSurface接收明确的原RenderViewSpec及同一Scope/R/Run/时点的真实source，消费原精确模块、最终LOD、实际设备姿态、物品展示槽与缺项，不另建viewer、媒体loader、时钟或业务数据。line_canvas/multi_hall_canvas分别核对kind/canvas/hall及工程修订；原视图合同缺少或矛盾要显式拒绝，不悄悄换当前工程/默认厂房/相机。

相机必须消费原CameraState的单位、坐标系、eye/target/zoom，转换一次到实际Three坐标；不能沿用bounds fit或用户上次视角覆盖所请求相机。任务只读模式禁相机交互、选择编辑、自动LOD及主工作台偏好写入，普通FactoryScene的现有交互/默认取景/缓存行为保持。所请求详细/简化/区域均使用原语义，工业模型质量不因截图要求降低。

## 实际就绪与输出

在真实场景尺寸非零、指定相机/LOD已应用、精确模型/图片/物品资源装载已完成或具体缺项已终结、该源实际GPU提交及DOM标签布局/字体就绪后，才通知onFrameReady。不能用固定sleep、两个盲rAF或只有组件挂载代替资源/帧完成。迟到素材、切Scope/R/Run/相机及卸载的旧回调不能宣告新任务就绪。真实初始化/解码失败走明确onFailure，不生成默认图。

FrameReady返回同一source key、实际scene/project revision及所用view、当前实际像素尺寸、具体limitations和真正可见ObjectRef集合。可见对象从最终可绘制、可见层级、所请求相机视锥及实际物品层推导，不能把Project所有ID或列表中可查身份直接当可见。side list和未知物品不构成3D几何可见；正常缺项标记若计入，说明其是哪个实际对象的标记，不能假称完整设备已加载。

回调只提供就绪/来源/可见对象元数据，不从前端提交PNG或用户文件路径。Root后继使用可信真实WebviewWindow调用已编译的native_render_capture；Task的取消/提交、hidden HWND/controller/焦点及managed字节由Root原生接线验证。

## 独占写入与编译

本包明确授权产品前端。允许新`apps/planner/src/features/render-surface/`、新`tests/planner/render-surface/`、新`scenarios/Agent共建/runtime-factory-render-surface/`；`scene/FactoryScene.tsx`仅新增所需受控相机/只读/实际帧就绪/可见对象hook，保留Root既有标签避让及所有普通入口行为。具体Props在源中固定后一次性交Root，Root保留main.tsx/Workbench/Native/Host/API/生成器/配置锁及原设计任务JSON。

actual_device_process正在同一FactoryScene的原接点联验持久化物品；其services/factoryPresentation.ts与scene/factoryItems.ts/factoryMedia.ts只读，若确缺其资源就绪接点，先报告准确接口并由Root协调，不覆盖其源。Cargo现租给definition_cycle_host节点读取；本包不跑Cargo、不改Native配置。前端验证使用既有工具，无安装。真实Host输入可使用已固定布局SDK代理，但不修改其生成源/选择其他活动rlib。

## 验证、冻源与交接

真实浏览器验证原FactoryScene两画布、两组明确相机产生不同且来源正确画面、坐标/单位/zoom、两个LOD/实际物品、非零尺寸、真实素材延迟及资源失败、当前源切换/卸载/晚回调、可见对象随相机变化和实际缺项。至少一个正常产品FactoryScene旧行为检查，保留原既有7画布/5标签组/实际slot消费者接线，不能修改旧业务数字或Run结果使测试通过。既有探针若只有内部Props资格变化，报告Root，不自行删其守门期待。

完成Planner TS及必要浏览器/原回归后完整自审，冻源一次性交handoff.md、self-review.json、真实命令/退出码、前后源切片及图片、明确Root最小接线，最后释放FactoryScene与全部本轮源租约。编译/浏览器通过不能关闭真实Native捕获、其余七视图或P3-04整包。
