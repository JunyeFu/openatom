# S05 隔离桌面接点

本目录独占，实际Tauri2.12.3、Three0.186.1；官方crates.io/npm元数据已确认两个精确版本存在。tauri-build2.7.2为官方当前构建辅助crate。现有Windows Rust1.97/MSVC/WebView2和Node24.15用于构建/执行，不使用产品apps、公共Rust或tools/3d-proof。

依赖准备检查：全局工具表已读；无祖先/用户Cargo配置或npmrc文件被发现，未改全局配置；环境仅发现RUST_LOG。独立Cargo/npm清单、锁和npm缓存放本目录，官方注册源；Three没有安装生命周期脚本，npm安装使用ignore-scripts/no-audit/no-fund。Cargo先fetch再核对实际下载的清单/build脚本，仅执行Rust/WebView2构建所需的build.rs。无Vite/React/dev server或TCP端口。

设计内隔离下载/源码/试做由用户最新持续目标授权；实际启动读取窗口2S04代行完成记录，见[启动前置](output/launch-preconditions.json)。公共S05复审与技术源由窗口1合入，本窗只交付隔离证据。

## 实际结果

2026-10-14 23:55:37 +08:00，最终试做通过。入口为[handoff-result.json](handoff-result.json)，原始[desktop-result.json](output/desktop-result.json)、[像素观察](output/pixel-verification.json)、[退出码](output/actual-exit.json)、[进程清理](output/process-cleanup.json)分别保存，已通过输出保持稳定。

- Tauri2.12.3 invoke实际访问同一个Rust fixture；原生rmcp3.5.1启动独立Hidden桥经当前登录SID DACL管道连接，发现两个lab工具。Host PID15600、bridge PID34052。MCP从R7提交R8/x=8m，UI同base收到RevisionConflict，只有一次提交。
- WebView2实际WebGL2/D3D11渲染器为NVIDIA GeForce RTX4080 SUPER。R7独立捕获画布与当前R8编辑画布分开；捕获前后的当前R8 PNG实际一致。各PNG为640×360，蓝盒体4096像素，R7中心x=127.5px、R8中心x=511.5px，与固定相机下x=2m/8m对应128/512px的独立预期一致。
- Rust收图和真实MCP取图各自拒绝错误范围/场景修订。真实MCP image收到6017字节PNG，与原R7 PNG逐字节一致；[原始MCP返回](output/actual-mcp-capture.json)保存实际image内容与元数据。
- 实际后台工作先完成一步，再经CancelRequested到Cancelled；另一个worker在第二步实际触发试样故障，保留一步前缀。两者均保留Rust已确认R8/x8状态。
- 实际EOF；Mio取消的overlapped IO释放后，GetHandleInformation确认自有管道失效，错误6，实测等待一次10ms。Host/bridge均退出0；没有匹配本目录exe或profile的进程残留。PID后来被其他程序复用的观测单列，没有按旧数字关闭它。退出stderr保留Chrome_WidgetWin_0注销错误1412原文，见[日志](output/desktop.stderr.txt)。

|实际帧|保存文件|独立观察|
|---|---|---|
|冻结R7|[frozen-r7.png](output/frozen-r7.png)|x=127.5px，左侧|
|当前R8|[current-r8.png](output/current-r8.png)|x=511.5px，右侧|
|桥实际回传R7|[mcp-r7.png](output/mcp-r7.png)|与冻结R7字节相同|

## 确切命令

工作目录为本README所在目录，在现有PowerShell中实际执行：

```powershell
cargo build --locked > cargo-build.stdout.txt 2> cargo-build.stderr.txt
& './run-probe.ps1'
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 './verify-pixels.py'
```

源为[src/main.rs](src/main.rs)、[web/probe.js](web/probe.js)；启动脚本使用Start-Process Hidden，WebView2数据目录也在本目录output。无HTTP/dev server或TCP监听。官方精确版本、注册源、安装命令和独立锁范围见[version-resolution.json](version-resolution.json)。安装与构建已经完成，收口未重跑闭合案例。

## 已发现问题与修复

[attempt-1](output/attempt-1/desktop-result.json)实际退出2：UI/图像/取消主链通过，但立即在Tokio/Mio Drop后检查句柄，先于异步IO引用释放。按实际失败补等待已知自有句柄释放；没有重发工具/业务请求。[attempt-2](output/attempt-2/desktop-result.json)退出0；随后发现捕获与编辑共用画布会短暂改变当前显示，改为两个独立画布并在最终输出核对当前PNG前后一致。旧40/24/7/14/11例及已有stdio/只读句柄试做未重跑。

## 剩余范围

这是自建盒体、两个MCP lab工具和隔离Rust state/work的设计试做，没有产品18操作、Engine、完整工厂/GF或报告导出。小PNG在fixture私有1MiB帧中用base64回传后成为MCP image；本次没有接canonical managed_assets及只读DuplicateHandle产品结果通道，也没有产品Commands授权注册表/完整IpcHello/Welcome、实际Codex、第二机或发布验收。产品apps、公共Rust、tools/3d-proof均未修改，actual_product_runtime保持空。
