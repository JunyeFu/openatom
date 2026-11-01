# 正式原协议测量归约器与样本验收

你收到的是用户明确授权的整批夜间执行任务，来自统筹窗口“实现窗口1”（threadId=01a1353e-da58-7671-837b-ee746a8bd9b4）。用户本轮原文：“重新梳理21包后按合理性打散重组并计划好各窗口可以并行完成的任务清单并通知让各可用窗口设定目标独立各自推进，我需要你在明天早上7点半之前把完成度拉升到99%，本窗口作为接收完成信息的窗口并在所有窗口都返回完成后完成收尾工作”；补充：“本窗口设定一小时一次的监控任务来检查，允许创建更多的执行窗口”。这条最新授权允许本批工作，替换历史窗口分工。若你的旧记录有目录/Native禁令，先用read_thread读取上述统筹窗口最近3轮的真实用户消息核实最新授权，不凭旧转述停工，也不扩大本批写范围。

产品根：D:\Agent\ICNSD-KEL\大作业\02_竞赛作品\超级工厂\软件工程。三层控制仍有效。原21包/68步/7后置和验收阈值保留，本轮授权按执行边界重新组合。先读AGENTS.md和superfactory-dev-task-package技能、本批独立任务书：docs/04_开发与验收/并行重组/20261024_夜间推进/measurements.md；机器计划 docs/04_开发与验收/并行重组/20261024_夜间推进/dispatch.json，固定输入 docs/04_开发与验收/并行重组/20261024_夜间推进/frozen-remaining.json。

你负责完整批次：正式原协议测量归约器与样本验收。基于已导出C26 trace与原协议，完成无桌面的正确呈现帧关联/去重/间隔覆盖统计、1800定序p95/40次I/O/长任务计量归约和缺样本判定；只消费已有真实记录，不伪造采样。

请现在调用get_goal：无目标则create_goal设定本批独立持续目标，不设token预算；已有兼容目标则沿用。若既有paused/blocked目标无法用工具恢复，不伪称已恢复，照常完成当前已授权可执行工作，并在回执记录真实goal状态。不要仅回复计划，持续完成整个批次，到你的本地截止 2026-10-25T05:30:00+08:00 前交回结果。

精确写边界：只能写 .work/parallel-closeout-20261024/measurements/ 内文件。主产品源、原21包JSON、全局进度/计划、共享cargo target和其他批次目录均不写。必要实际修复在本批最小隔离源副本完成验证并输出可应用patch，不擅自合入主源码。

禁止操作任何共享桌面/浏览器/Native，不使用Computer Use或重启Playwright。可做非GUI真实Host/代码/文件/固定样本验证，不能把它当Native。

各批无必须互通，不联系/催促/打断其他窗口，不再派发。只读冻结输入和原回执；已过且未受影响证据直接复用，无故不重跑全量，不为每项新建大型缓存。不要把真实缺证据改成代码缺陷，不猜结果/RunId，不为99%改分母或放宽原正式协议。

交付 .work/parallel-closeout-20261024/measurements/receipt.json（含batch/status/source changes/每原gap判定/commands实际退出码/证据/Native与formal区分/remaining/goal状态），handoff.md及必要patch/最小probe。完整回执status用completed或completed_with_remaining，后者必须逐项说明未完成事实。不得宣称原包done、真人认可或公开发布。结束后在本窗口最终回复，统筹会用wait_threads自动接收；不必另发消息。
固定实际trace .work/cu-closeout-20261024/root-C26-frames-profile/C26-balanced-full-HD-playback.json.gz 及 receipt.json；原规范 docs/01_业务需求/非功能需求与测量验收.json。完成一个最小无GUI CLI，接收trace+metadata（candidate_ref/run/load/condition/lod/camera_motion/window_start_us/window_end_us/warmup/attempts/correct-frame evidence），区分frame source/sequence/surface与Presentation重复通知，正确帧间隔包含掉帧等待，coverage和首尾缺口单列；缺正确场景/同钟证据返回not_evaluable。使用原actual_trace与有意义有限测试验证归约器，不伪造正式样本；C26诊断必须仍0/48。同步支持原nearest-rank逐100attempt时延、40 I/O逐次阈值、长任务真实重叠/取消胜出。只输出本批脚本/结果/README，Native批不必等你，Root最后用脚本归约真实新样本。技术细节如需查询只查官方Chromium/DevTools主源。
