# P3-04 公共RenderView原生任务窗口与managed交付完整续作

接续[P3-04原完整包](P3-04_整包执行.json)、[Host生产者完整续作](P3-04_公共RenderView完整Host任务生产者续作.md)及[原生实际技术接收](../../../scenarios/Agent共建/runtime-native-product-capture/final-native-technical-reception.md)。实施规则以公开操作与宿主调用契约MD/JSON的render_view_execution_contract为准，原九kind/全部Q/GF/IX/NFR/人验/release保持。actual04只是私有CLI同源PNG技术门，不是本公共调用。

## 完成标准

将Host已准入的真实Rendering Task通过可信Rust RenderBackend交唯一Native产品专用窗口，读本Task精确冻结输入/字节闭包，等真实typed FrameReady后原CapturePreview返回实际bytes，原Host提交实际RenderOutput；原agent_pipe发布只读图像lease并被MCP按原Task/source消费。完整完成本Native/交付层的实现、自审、必要反例与Root实际接收，保持全九kind后继同源视图责任与原包未完成项，不以协议fixture代真实PNG。

## 写权与共同接口

本负责人独占新 apps/planner/src-tauri/src/native_render.rs 和专属测试/runtime-public-native-render；允许为该层最小接线修改project_api.rs的RenderBackend注入/Task窗口专用命令、lib.rs相应mod/setup/handler登记、agent_pipe.rs真实managed_assets与实际UI-only授权映射，以及apps/mcp-bridge/src/decode.rs中真实scene_revision与lease ScopeR一致门及专属定向测试。不得改Root Host/API/domain/公开契约/原JSON、当前Node静态消费者、tools/3d-proof。Root刚补的AuthorityChoice省略false/显式UI值及当前普通Workbench/probe行为保留，不能将它们算入旧portable04证据。

共享main.tsx/九kind前端仍Root/共享消费者单写，不能假设旧sealed JS有新路由。Host生产者现已冻源并由Root接收最终14份源/138项检查，准确接口见其完整交接；Root也已接main可信Task分流，当前Planner类型检查通过，浏览器仍在收尾。Cargo已由Host明确释放并由Root正式交给本负责人，执行本轮完整Native/bridge验证及新公共smoke入口准备；固定-j1 --locked --offline，保留此前真实并行编译内存失败。Host补充输入测试只准备不抢Cargo、不改生产依赖。不自行改依赖/安装、构建活动共享前端或启动Native。实际Native启动、show/焦点/共存验证由Root唯一执行。

Root已追加授权正常制造预览的最小main-only字节转发：project_api.rs/lib.rs登记project_read_stored_manufacturing_file，精确请求为project_id/scope/scope_revision/artifact_id/file。Native注入真实UI session，Host校验当前ScopeR、实际Artifact/Result/Job三元组及manufacturing_files中的精确FileRef，返回实际已存字节；五个Task端口不变，Task仍仅读自己的native_render_file冻结闭包。

Root也已授权专属project_api/public_render_smoke.rs与本runtime/process_render_client.py及相应最小登记，提供新公共Task的实际后继入口。仅显式新smoke配置及真实main的native_render_input调用后在后台执行，事件循环必须活跃；复用原实际UI授权函数、同一Host、真实OS peer与独立stdio bridge，不在setup同步阻塞，不伪caller，不复用旧private capture probe。CLI资源/视图/bridge/output路径由Root明确指定，结果标为受控授权联验而非真人或真实Codex。实现者只编译与准备，Native生命周期仍Root。

## 必须落实的实际边界

Commands不依赖Tauri，沿唯一ProjectApiState/FIFO；Native不持Host锁跨await，不创建第二Host/Engine。专用任务窗口固定产品地址，窗口身份由真实Tauri WebviewWindow及可信Task注册表获取，不信客户端label/path/caller/PNG。冻结input只对该Task原窗口可读；assets/files精确匹配闭包，不放开原main-only getter或UI身份，不向external外送源DTO/字节。FrameReady来源/view/ScopeR/ProjectR/Run/time/camera/LOD/尺寸按完整typed类型匹配，保留此前整数浮点修复；缺项/失败不补图。支持kind只列实际同源前端已接的能力，全九kind缺口逐一保留，不能永远Unsupported抵验收。

CaptureOnly保持隐藏未聚焦、不改主Workbench选择/相机/播放/偏好；控制器可绘制与真实窗口隐藏分开处理。ShowAndCapture需真实UI独立授权allow_view_control并在实际显示前检查Host原动态权限/epoch门，失效明确失败而非降级。保原已准入Task断连不自动取消、旧epoch结果归原Task不写新工程；取消/窗口关闭/COM回调安全完成和资源释放遵原竞争结果，不把丢future当取消成功。

原管道已认证peer/只读publish_readonly lease复用：实际completed Rendering get_task按原owner/Scope/Task/Asset取Host真正捕获图，同时保大JSON和image两种leases；错误只关闭自己已接收句柄。MCP现decode尚缺scene.scene_revision==attachment.scope_revision门，本包补真反例及最小修复，不据此冒称公开调用已实际跑通。报告actual_render_missing仍由Root后继匹配同源实际render成果再处理，不靠CLI文件路径清除。

## 验证与交接

保留原实际01/02/03红、actual04原两PNG与25sourcecut，尤其退出1412 stderr不清零。当前Native新权限/公共Task源需新的编译和实际运行证据。先真实Host控制层接线，再由Root按固定新候选实际完成授权pipe→Rendering Task→原生PNG→Host结果→managed→MCP；测CaptureOnly与正在使用的主窗口共存、显式Show权限、错窗/错Task/错source/旧R/时点/取消/撤权/切工程/旧Task授权，以及同次大JSON+图像、MIME/Asset/sceneR/长度错误。编译、callback fixture与真Native图区分记录。

完成完整diff、协议与Root前端接点、命令/退出码/raw、source cut/自审/Windows实际启动步骤后集中回Root并冻源；若尚待前端或Root生命周期联验，准确列等待门并继续可独立完成的本包工作，不把旧probe或部分kind置为整个P3-04完成。详细设备仍按统一八设备精度规范，技术捕获不构成工业资产质量或人工认可。
