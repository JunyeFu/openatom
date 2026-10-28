# 外部Agent适配位置

首版实现本平台真实工具桥；不同客户端共享同一API。clients用于实际验证后的配置样例，skills用于可移植工作流指导。当前原生 stdio 桥已接入同一 Tauri managed ProjectApiState；只向实际主窗口授权后的 OS peer 提供当前 Host 能力。未安装全局 Codex 配置。

[共同操作](../../docs/03_架构与契约/首版类型格式与操作契约.md) · [接入与专业连接器](../../docs/03_架构与契约/通用Agent接入与专业软件连接器.md)。普通工程包不携带账号、密钥或当前机器配置。

## 现行目标生成与开放主线

一句话行业/目标/accepted_unit/接受窗/长宽高→原生/导入社区模板与九类库→可编辑对象/连接/排期→检查/渲染/实际模拟。初案输入齐全、检查通过且达标直接报告/工程交付；独立不足案例验证许可变量内有限修正/重跑比较或不能修正说明。按[S02-01冻结清单](../../docs/00_项目基线/首版交付基线.md)，本平台MCP桥必做，Unity/Blender/FreeCAD实际连接器后置，不从设计清单注册虚构能力。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

## 纸包装动作作者契约

外部Agent与专业表单共用[完整Job类型](../../docs/03_架构与契约/全层类型与工程格式.json)。`fixed_pose`的data是`{part_id, pose}`：先读当前Job的真实part ID，再提交保持姿态；不得猜首个part或把保持改成瞬移。沿既有apply_commands/compute_packaging提交，同样接受输入域及完整几何检查；这里不新增公开操作，实际可用能力以 Host catalog 和本批执行记录为准。

## DRF04：Codex stdio、Host任务和消费者

[完整定义/调用样例](../../docs/04_开发与验收/特征细化/DRF04_完整定义.json) · [公共交接](../../docs/04_开发与验收/特征细化/DRF04_公共交接.json) · [产品workflow](skills/superfactory-workflow/SKILL.md)。本机只读核对得到codex-cli 0.162.0-alpha.17.2和`codex mcp add --help`；未配置、连接或验证Super Factory服务。

首版选定Rust/rmcp3.5.1桥，Codex启动stdio子进程，桥按Host发出的当前会话管道/实例参数握手，Host注入身份和范围，实际工具目录调用同一Commands，不固定工具数量。stdio按行传JSON-RPC，stdout只输出协议；内部Windows IPC才使用u32小端长度＋UTF-8（上限1MiB），两种分帧不混用。工具名取`superfactory_<operation>`实际注册，输入/输出schema取同源API版本0.1；MCP协议版本协商独立于项目格式1.0和IPC版本1。

[Codex官方MCP说明](https://developers.openai.com/codex/mcp)确认支持命令启动的stdio，配置使用command/args/cwd；本地CLI帮助也确认`codex mcp add <NAME> -- <COMMAND>...`形式。执行阶段由Host提供准确连接参数、实际exe位置后才能形成最终配置。桥的拟议argv接点见DRF04公共交接；本轮不写全局配置，不在clients留下“已支持”样例。真实Codex客户端仍须后续现场验证。

七种长操作回TaskHandle，Agent以get_task读TaskState.originating_operation/result，完成输出分别为Render/Run/Packaging/Import/Save/Package/Evaluation。不将请求成功当目标达标、未完成当已保存。Run结果再用get_run Summary/Events/ObjectsAt/FullResult；同一scope/修订，接受量/等待/时间轴来自Engine。取消用cancel_task(action=cancel)，观察实际终态；终态先胜、部分范围和失联处理取公共契约。

[MCP工具规范](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)允许structuredContent与outputSchema；本项目以有类型Response作为业务结果。[协议取消](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/cancellation)针对仍在处理的请求；已回TaskHandle的Host任务由业务cancel_task管理。本版不要求MCP可选Tasks扩展。视图暂停不取消计算。[传输原文](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)核读于2026-10-14。

越界由Host同一Commands拒绝：整对象变化、模板新建/复用、文件SelectedFileRef.purpose、素材许可、人工认可都不能靠换工具绕过。Host私有授权与受管画面IPC附件已由窗口1合入，workflow按15分支和现行carrier消费；实际 Host 的可用操作由当前 catalog 决定；Renderer 尚未接线时不注册可用渲染能力。只读句柄原生子集、新增14个身份/生命周期案例及产品边界取[连接器源](stdio-connector.design.json)。

窗口2界面消费[32字段绑定](../../docs/04_开发与验收/特征细化/DRF04_H04-04界面绑定.json)。TC01/TC02/frontier及Events闭点末刻已公共合入；共享SKILL/YAML生成器已由窗口1修复并隔离验证。不得继续把这些字段写成旧pending，实际Agent/Host/Renderer/原型证据仍分别保留。

## 当前原生接入

2026-10-17普通产品入口：“Agent → 连接、权限与任务…”。工作台展示实际连接PID/Windows会话，默认只读，逐项确认操作、定义草稿和对象字段/新增额度；目录令牌由原生选择器按当前连接/用途授予。界面仅复制客户端配置与任务说明，不把文本粘贴称为Agent执行。实际Agent提交的工程变化由同一Host读取，面板真实未提交输入及忙碌状态进入主窗关闭保护。当前受控UI10/10与Planner类型检查通过；完整实际Native/Codex/GF和四格式接收仍取P3-04整包记录。

在桌面 main 窗口调用 `agent_connection_info` 取得当前进程的 endpoint 和 host_instance_id；启动 `superfactory-mcp-bridge --host-pipe <endpoint> --host-instance-id <host_instance_id>`。连接在真实 PID、用户 SID、登录 SID、Windows session 核验后等待授权。main 窗口通过 `agent_authorize_connection(session_id, choice)` 明确项目、操作、草稿以及对象字段/创建额度，随后才完成 IPC welcome。`choice` 是桌面管理入口的选择，不是 MCP 参数。`agent_update_connection_authority` 在同一 OS 连接上更新实际 Host 授权，保留相同 grant 的累计创建账本；`agent_revoke_connection` 同时撤销共享 Host 会话并关闭连接。EOF 也清理 Host 会话、文件权限和转授任务权限，不取消已受理任务。工程目录由 `agent_select_directory` 弹出原生选择器，SelectedFileRef 只属于当前连接。

`tools/list` 每次消费当前 Host 的能力，不注册未完成业务。`tools/call` 传完整冻结 ApiRequest，不提供 caller/grants/path。大结果附带完整原始 Response 的 application/json 资源正文；原始错误即使不带 context，也保留错误状态。pipe 损坏或协议取消使桥退出，Windows 回收已接收或尚未读完帧里的租约，不重投请求；业务任务继续由 Host 持有。

局部复现：`cargo test -p sf-mcp-bridge --all-targets --locked`；构建 desktop/bridge 后，在仓库根设 `SUPERFACTORY_TEST_PYTHON` 为已有 Python 绝对路径，运行 `target/debug/superfactory.exe --p3-04-mcp-smoke`。该启动模式使用真正的 Tauri managed 状态、Windows pipe、独立桥进程及 Python MCP 客户端；用例的路径选择通过受信任原生测试入口注入，不宣称人工目录选择或真实 Codex 接收。原始结果在 `tests/delivery/mcp/output/`。

主窗口可用 `agent_grant_task_access(session_id, task_id, allow_controls)` 给新 OS 会话授权一个确切的原 TaskId；默认只读，控制权限须明确选择，并继续受当前操作、Project/Definition Scope 与任务控制能力约束。它不恢复旧文件 token，不改 Task 身份或冻结输入，不重新提交工作。新连接猜中旧 ID 仍无权限。两个管理入口都拒绝副窗口。完整九类/媒体/渲染/GF/四格式及真实 Codex 的接收状态以该执行记录为准。

## v2执行消费者（draft11）

现行 `ExecutionWorkflow` 与[产品工作流](skills/superfactory-workflow/references/execution.md)消费真实 H08/H01/H09/H02/H03/H10/H11；不将预测 `run_factory/run_scenario` 用作生产执行。MCP schema继续由当前 Rust `ApiRequest` 生成，返回由原 `contract.rs` 的实际专用类型解码；额外核对原请求、Task、工程/修订、会话/epoch与精确R/T。普通Agent工具目录及调用入口均排除四个Gateway私有操作，实际Host仍独立检查原principal授权。

Python入口接收已经授权的同步 `send(canonical_request)`、实际 `Response.context` 和当前能力目录。`send` 必须把完整请求发给 `superfactory_<operation>`，返回真实MCP结果或已恢复的完整Response；不能注入调用者/grant或替换为本地模拟。先读H08的configuration和issues，再经明确授权 `prepare`；`advance(max_reads=1)`一次只读取有限Task，未终结就返回pending，不后台高速轮询。准备成功保持prepared。明确run/接单/暂停/停止各自提交原expectedR，冲突仅返回刷新结果。Host的生命周期、未结清命令/输出/预留原样保留；停止观察不发停止生产。

任何传输未知状态保留 `workflow.pending`（原完整请求、已知原Task与状态）；重连需重新获得当前Host/任务授权，不能继承旧文件令牌。`recover()`只用H09查询原请求，不重发；not_found继续未知。H03没有登记查询对应结果时也保持未知，不据新快照猜原控制是否已受理。`view_pages`固定原R/T和首个F、原历史顺序，预算不足返回partial及下一选择；`query_first_page/query_next_page`保存UTC微秒原文、首个F/设备或exactR，完整导出重新从原筛选起点由Host流式执行。

导出选择器回调只接收用途并返回**真实原生选择器授予当前连接**的 `{selection_id,purpose}`；两个用途分别 `export_execution_report`、`export_cloud_data`，不能传磁盘路径或自行创建ID。Native尚未提供对应目的时传入None，调用明确拒绝，不提交导出。报告固定来源/identity/R/origin及可选真实预测关联，四格式PDF/HTML/CSV/JSON；H11使用固定筛选与首个真实选集，包含所有页。此消费者实现不证明Native选择器、HTTPS生产或实际四格式落盘已验收。
