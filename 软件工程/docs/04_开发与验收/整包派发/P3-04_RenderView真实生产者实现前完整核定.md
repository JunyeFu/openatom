# P3-04 RenderView真实生产者实现前完整核定

继续 [P3-04原完整执行包](P3-04_整包执行.json)及其原任务/公开契约；原全部步骤、Q、SF/IX/NFR/release保留。18操作里的 `render_view` 当前没有真实Host生产者，不可用既有MCP回包schema/PNG协议夹具冒充已实现。窗口2正在独占另一个 `check_factory_draft` 新生产者，勿改其源或全包JSON。

本轮完整核定这个真实缺口的最小可执行方案，先读公开操作契约/RenderViewSpec、RenderOutput、RenderedSceneRef、RenderDelivery，现有Host Task/I/O/取消/授权及MCP managed asset lease，再读实际Planner九类视图和Native主窗口接口。九类为line_canvas、multi_hall_canvas、device_detail、module_preview、packaging_workbench、packaging_manufacturing、device_cycle_preview、timeline_snapshot、comparison。精确适用条件来自原文，不能重新发明简易视图来满足截图字段。

目标必须是真实外部授权请求 → 同一ProjectHost捕获原Scope/R/Run时点/精确对象 → 同一产品渲染实现 → 实际PNG或JPEG及对应RenderedSceneRef/可见对象/缺项 → 原Task完成与managed只读结果；取消、工程切换、陈旧来源和权限仍按原边界。CaptureOnly不改可见观看状态，ShowAndCapture仅在授权后切换。实际错误不能补默认图；外部调用者不提供截图字节、任意路径、伪造caller或新公开operation。

选一个可以落到现有产品的最小方案，指出确切Host/Native/前端接点和单写拆分；优先复用已安装依赖和现有同源组件。先核定Native可提供的真实捕获路径，若需查API仅用官方文档，不安装依赖。此前CDP远程调试启动被自动审批拒绝，禁止重试或换代理绕开；本轮是产品功能实现核定，不能把调试远程控制作为功能或GUI验收手段。也不能用Sky/外部屏幕截图工具在Host中充当产品renderer。

写范围仅 `scenarios/Agent共建/runtime-render-view-source-review/` 的完整核定、自审和最小验证建议；Host/API/domain/Native/UI/桥接/原JSON/资产不改，不运行Cargo、Native或新增监听。给Root一个选定方案、涉及文件、必要内部协议字段（若确有）、各九类真实组件可复用性、当前缺口和最小真实red→green顺序。避免另建一套renderer/状态库/模拟时钟，明确哪些必须等待现有消费者冻源。

完整自审后一次交Root；不得将核定或协议往返当真实GPU/Native/industrial/NFR/release通过。Root据实际方案分派并接线，原整包完成仍开放。
