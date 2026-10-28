# P3-04 隐藏 Native 首回执缺口完整续作

继续[同源Native隐藏窗口捕获完整任务](P3-04_同源Native隐藏窗口捕获完整联验续作.md)全部要求与原九视图、Host Task/权限/取消/managed及release责任。本包是该任务实际失败后的闭环，终点仍为 Root 真正接收同一Native Host、隐藏非激活窗口、原产品场景与双相机实际PNG；观测或重新编译本身不算完成。

## 已观测事实

Root唯一启动固定portable-01，PID80584、exec session87907。原Import/Open实际完成，plan.json来自真实Project R11、三份已存Run与Root原两相机请求。进程Live/Responding，约3分30秒无window-initial、首FrameReady、PNG或failure；stdout/stderr空，Sky只返回原ChatGPT窗口。Root在核验精确exe路径和StartTime后显式终结该本轮拥有进程，原生exit=-1。原output01、观测/停止/退出码完整保留；这是可观测链缺口，不是已证明Host死锁或隐藏渲染不可能，也不能称Native成功。

## 精确修复范围

仅本包原owner p0_01_host_core 可写已有新 `project_api/native_capture_probe.rs`、`lib.rs` 的probe必要接点，以及本人runtime-native-product-capture证据。不改Host/API/domain/配置锁/共享前端/Workbench/main.tsx/原封存dist，不启动Native、Chrome、CDP或服务。

在capture_smoke成功创建可信main窗口后直接安排原with_webview UI STA的controller IsVisible读→true→读，记录实际过程；解除对首个前端计划invoke的顺序依赖，HWND仍hidden/unfocused/not-minimized，不show/focus或使用别的窗口。真实Window/Controller错误依原失败结束，不retry/default。

仅probe新增必要区分观测：window-built；实际on_page_load Started/Finished；实际本地HTML/JS资源request/status/content-type；计划command入口、UI闭包排队/进入/COM读回/oneshot返回。仅probe初始化脚本可监听真实DOMContentLoaded/error/unhandledrejection，通过main-only、deny_unknown_fields的最小阶段DTO回传；不接图像、任意文件/窗口/HTML，不开放公共操作。观测输出仅本轮明确output，由Native持有路径，真实消息原样保留；不靠sleep、超时、重试或吞错制造终态。

保持原FactoryRenderSurface/共享FactoryScene、精确source/view/ScopeR/ProjectR、非零viewport/字体/资源/GPU ready、一次在途/迟到门、CapturePreview STA、Host不跨await锁、原Project及三份完整旧Run不变、create_new输出等条件。首回执出现后仍须完成实际两PNG内容/相机差异/隐藏性，必要实际缺陷继续同包修复；不以只补日志交差。

## 构建与Root实际接收

现授本包Native窄源及Cargo独占直到完整构建交接；其它agent仍不得Cargo。继续同一绝对dist-frozen、beforeBuildCommand=null的既有私有override，custom-protocol release no-bundle locked/offline，检查必要Native单元/真实编译与原警告，不重新取活动前端。新portable-02单独保存，portable-01/原失败源/原raw不覆盖；正式37资源映射及28line-grade不变，Fit4仍不打包。

Root下一轮指定同一原input与views，新的output `.work/whole-packages/native-hidden-capture-root-actual-02`（不得预建）。你不启动进程；完整handback给出准确exe/资源/源diff、命令/实际退出码/raw、初始化及观测各接点、自审和剩余边界，释放Cargo/源。Root核对并唯一启动；未收到真实结果时仍保持pending。实际双PNG技术门后，继续原公开九视图完整任务，不标整包done或填人验。
