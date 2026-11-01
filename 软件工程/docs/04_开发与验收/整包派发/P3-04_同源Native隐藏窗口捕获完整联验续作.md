# P3-04 同源 Native 隐藏窗口捕获完整联验续作

继续[P3-04原完整包](P3-04_整包执行.json)、[捕获基础层](P3-04_原生产品捕获基础层完整续作.md)与[同源厂房RenderSurface](P3-04_同源厂房RenderSurface完整续作.md)。原完整九视图、Host Task/权限/取消/managed资产、GF/IX/NFR/release全部保留。本续作完成真实原生捕获生产者接线、当前真实Native构建，并由Root实际启动验证隐藏窗口/两相机产品PNG的第一硬门；不能把smoke私有接线当public render_view、其余七视图或完整Task链通过。

## 实际同源输入与正常产品边界

使用已冻源FactoryRenderSurface、原FactoryScene和native_render_capture::capture_png。实际源经同一Native ProjectApiState的原ImportPackage/OpenProject及原只读presentationHost/factoryPresentation/资产端口获得；不引用浏览器独立Host结果替代Native，不用测试JSON预填FactoryPresentationSource，不创建另一个引擎。可复用实际保存并搬移的`scenarios/媒体表现/runtime-item-layout-consumer/moved-73580`归档，或Root明确指定的同等已验证归档。保留原目录，输出仅Root本轮确切新建`.work/whole-packages/`目录。

为第一技术硬门允许沿项目既有私有CLI smoke模式新增`--p3-04-capture-smoke`，由Root本机CLI明确传入capture-input/output/views文件；不是公开文件选择/Render API，不接受前端路径、PNG或任意窗口名。正常模式返回无probe计划并照常挂Workbench，不显示/切换用户工程。使用`project_api`内现有register_directory、wait_for_task、smoke_request/business_adoption等原路径导入到本轮新Host；准备阶段无Host mutex跨await。缺文件/导入失败明确退出，不改权利、不补素材或业务输入。

Root指定两个完整原RenderViewSpec（同一实际工程/冻结源，不同相机），前端只消费Native实际计划和同一Host当前Scope/R/项目/可空Run与时点。同源FactoryRenderSurface就绪回调必须对应当前计划step及原view/ScopeR/ProjectR，真实资源/GPU/字体/标签就绪后才调用Native。图片字节只由Native CapturePreview产生，前端不交图、不截图canvas、不调用CDP或外部截图。

## 原生窗口、字节与真实接收

本轮Root拥有的smoke WebviewWindow可用原main标签及其原UI会话，整个窗口hidden、非最小化、非零viewport；正常main-only私有getter门不能为任意新label放宽。若WebView2 controller需要IsVisible=true可在该可信窗口UI STA明确设置并读回/记录；不show/focus窗口、不更改其他窗口或先借主视图再恢复。以实际API状态与原PNG证明hidden HWND/controller组合可绘制，不能用API存在或编译通过作结论。

Native等待实际FrameReady后调用同一捕获叶，保存原PNG及计划/实际frame/窗口visible/focus/inner_size/实际PNG尺寸、成功或原API错误。每步单次在途，迟到/重复frame不覆盖新step；await不持Host mutex，保留窗口直到COM完成，真实失败终结本轮并保存错误，不重试/默认图/显示窗口兜底。完整公共Task取消/managed发布仍是后续原P3-04责任，probe不冒认。

Root实际普通启动后检查：两个PNG非空白、能看见实际已加载设备/素材并与source/view对应；相机变化真实改变画面/可见对象；相机不改Project/旧Run/库存，隐藏窗口始终不显示聚焦。本轮证明范围是此受控独立Native烟测窗口，不伪称已经证明与用户活动main共存的全部CaptureOnly生命周期。若隐藏绘制失败，保留实际失败并回Root决定真实可行实现，不绕过约束。

## 精确租约与编译顺序

本包交W00内部p0_01_host_core。允许新`apps/planner/src-tauri/src/project_api/native_capture_probe.rs`、`project_api.rs`仅私有模块登记、`lib.rs`仅state/handler/CLI初始化与隐藏窗口控制的必要接点，新`apps/planner/src/features/native-capture-probe/`、`main.tsx`仅正常Workbench/probe明确分流，新专属测试/`scenarios/Agent共建/runtime-native-product-capture/`。捕获叶与已冻源RenderSurface只在实际观测失败且本包必要时最小修复；Workbench/Host/API/domain/配置锁/资产/公共生成器/原任务JSON保持Root。

Cargo当前Root在执行冻结实例后端全套，先只写上述Native/新UI源，不跑Cargo；Root明确交回后才编译。前端共享media/load/types/过程目前仍为Root冻结状态，Node消费者只写新独立源；本包先完成`npm run build:web`形成实际`apps/planner/dist`快照，立即告知Root，Root随后释放Node共享源。之后Native编译固定使用这份dist，不再混入Node开发中的前端。严格TS/必要原生单元及实际Tauri `--no-bundle --features custom-protocol -- --locked --offline`构建，无安装、升级、网络或新端口。

Root唯一拥有Native生命周期：你不启动Native/Chrome/CDP、不开窗口，不自行清理其他PID/目录。代码及构建冻源后一次交Root：完整变更/源自审、精确CLI/输入/输出/views文件及实际exe/资源位置、真实red/green/退出码、Root最小启动命令与验证方法，并释放Cargo/Native/入口租约。Root随后实际启动并把真实PNG/状态交你核查，必要缺陷继续原完整包闭环。最后真实同源原生PNG通过后，仍须回原九视图/Task/权限/managed完整实现，不把本阶段当终点。
