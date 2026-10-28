# 本地Agent桥

当前已实现Rust stdio MCP进程，经实际Windows当前用户管道连接同一个桌面Host及Engine；命令、Scope和结果复用packages/commands，不保存第二份项目/运行库存。实际进程已验证共享编辑、授权拒绝、真实Run、保存取消、完整大结果及冻结的`superfactory-result://<id>`资源URI；原独立消费者13组、桥8项和本窗原生44项有限检查已通过。完整执行记录见[P3-04整包JSON](../../docs/04_开发与验收/整包派发/P3-04_整包执行.json)。

共同Host会话更新/撤权及新会话显式旧Task授权的三个私有Rust接口已实现并通过185组commands回归，Native消费者接续中。实际Codex客户端、渲染反馈、检查/报告与完整GF/用户/release仍待实际接收；受控原生进程不充当这些结论。[接入设计](../../docs/03_架构与契约/通用Agent接入与专业软件连接器.md)为权威入口。未修改用户全局MCP设置，不提供公网/多用户服务。

## 现行目标生成与开放主线

18项计划操作由桥连接同一桌面宿主；query_module_library和check_factory_draft用于SF-17，skills驱动填参/运行/修正/导出，不在桥内另建引擎。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

S02-01将本目录及packages/commands、integrations/agent纳入P3-04的release_paths；本平台实际stdio桥为首版必做，共享宿主、不自建Engine。外部专业软件连接器仍后置。范围见[冻结清单](../../docs/00_项目基线/首版交付基线.md)。

## S05-04具体技术选择

首版桥采用官方rmcp3.5.1 Rust原生exe；stdio→当前会话ACL管道→同一Host。具体版本/Windows部署取技术栈设计；无宿主不自起另一Engine。既有原生SDK/管道隔离fixture作为历史证据引用，当前实际产品桥/Host已接通，真实Codex验收仍独立待办。 [技术栈/部署](../../docs/03_架构与契约/首版技术栈与部署设计.md)。

以下DRF04、RR与S07条目保留其设计/试做时点的原始证据，旧“待实现”不覆盖本页开头及当前P3-04产品回执。

## DRF04 启动、身份、任务与受管画面

唯一连接器语义源：[stdio-connector.design.json](../../integrations/agent/stdio-connector.design.json)。本轮公共Host/API写入归复审窗口1/W00，S04及docs/02归窗口2。产品启动参数选为`--host-pipe <Host给出的当前会话管道> --host-instance-id <真实Host实例>`；这是待产品实现的CLI契约。Codex配置审阅稿见[design.toml](../../integrations/agent/clients/codex-stdio.design.toml)，未安装/启用，不自动改全局配置。

stdio客户端启动子进程后进行MCP协商/发现/调用；管道Host由显式当前登录SID的DACL和远程拒绝保护。真实客户端PID与登录会话从Windows取得，JSON不能声明UI来源或授权。IpcHello核对Host实例，IpcWelcome给连接会话；每帧核对同Host/会话/版本/原operation，再向同Commands注入HostCallContext。现行私有HostEditGrant已由窗口1合入并原生编译，不进入请求Schema/TS；实际运行校核仍待Host实现。

7长操作只返回TaskHandle，get_task取得匹配的TaskOutput。接收成功不等于工程/运行/文件完成。取消已接收任务用cancel_task(action=cancel)再读终态；完成先胜仍Completed。失联时已接收工作归Host，最后一次接收不明就保留本地提案/最后确认轮次并停止自动工作；request_id是关联ID，不是自动重发的幂等承诺。重连须实际新授权，旧文件引用不恢复。

画面回传选Host打开受管结果并向经OS验证的桥进程复制**只读文件句柄**。桥消费匹配Task/Asset/Scope/请求及scene修订的字节，再返回MCP image内容和原类型化Response；不按外来AssetRef.path开文件。[附件delta](../../docs/04_开发与验收/特征细化/DRF04_受管画面IPC合入.delta.rs)已由窗口1合入并通过原生/标准Schema/411TS及8个形状案例，实际Host/Renderer接线未执行，业务操作仍18项。旧R捕获保留原任务归属，不盖R+1；只关闭已认证连接实际拥有的句柄，不拿外来handle数字当读取/关闭权限。

本批实跑[rmcp/管道fixture](../../docs/04_开发与验收/特征细化/DRF04_stdio-proof/output/native-trial-result.json)及[只读句柄跨进程试做](../../docs/04_开发与验收/特征细化/DRF04_stdio-proof/output/managed-asset-result.json)。后者匹配修订实际读出自建PNG字节，写入被Windows以错误5拒绝，错修订不返回字节。fixture没有产品Renderer/18工具或Codex，原管道fixture用按行JSON，也不能冒充规范u32长度IPC握手已验证。SC01—05给出产品Host、授权编译、规范附件/渲染、断线取消和Codex/GF的可复现剩余接点。

[Windows管道权限原文](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)、[客户端PID](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid)、[DuplicateHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)支持上述设计机制；实际权限、任务范围与媒体许可仍由共同Commands/Host决定。

新增[14个附件身份/生命周期案例](../../docs/04_开发与验收/特征细化/DRF04_stdio-proof/output/attachment-lifecycle-result.json)使用现行canonical frame Schema、真实fixture进程PID和实际复制句柄。Task/Asset/Scope/请求/场景修订、会话/Host、错PID/句柄/字节长度均在读前核对，错误不返回字节；消费/拒绝/真实stdin EOF后通过GetHandleInformation确认句柄已失效。业务仍运行、零重提/无第二Engine是有限模型，未执行真正Host业务。旧native/只读40例等证据只引用未重跑，fixture handle12/字节123未当OS能力。

S05工具、最小输入、独立预期、失败退出及监督前置见[准备包](../../docs/04_开发与验收/特征细化/DRF04_S05接点准备.json)。本轮不启动Tauri/产品，不安装或用缓存旧版本代替冻结版本。

后续新增[真实Windows管道对端身份试做](../../docs/04_开发与验收/特征细化/DRF04_管道对端身份试做.md)3例通过。真实server与Hidden bridge双向调用GetNamedPipeClient/ServerProcessId、GetNamedPipeClient/ServerSessionId，读取API所得进程的实际用户/登录SID和TokenSessionId。正文伪造身份未用于授权，错误预期实例被拒绝，实际EOF及已知自有进程/句柄清理通过。它只补足OS原语与隔离实例判断，产品注册表/授权注入/完整canonical握手仍待实现；旧试做未重跑。S04前置已按用户授权窗口2代行走查更新，等待实际证据及窗口1同步。

## RR 23/24/25 的当前消费者接点

共同[大结果合同](../../docs/03_架构与契约/大结果传输设计.md)已选择ApiOutcome.Managed与managed_results。桥先用唯一匹配的已认证只读句柄，完整解包原Response<R>，校核schema/请求/上下文/Run输入和前沿，再独立关联原渲染附件。MCP保留小structuredContent及完整application/json EmbeddedResource，不新增resources/read。原响应为PermissionDenied等错误时，MCP isError仍为true；外层managed不是业务成功。传输取消只释放已知自有句柄，不伪造业务Cancelled或重跑。

本轮[9项实际OS消费者试做](../../tools/design-quality-audit/report-linked/window4/output/managed-consumer-result.json)读取1,319,773字节类型试样、核对EOF和禁止写入，错误类型/内外身份/嵌套managed/Run/长度/句柄均不返回资源；还验证受管大错误保持isError=true。数据仍是synthetic_type_fixture，MCP内容为隔离消费者的本地序列化，未执行产品Engine或Codex业务。

本机Codex0.162已通过[禁用配置命令级读回](../../tools/design-quality-audit/report-linked/window4/output/codex-recipe-result.json)，没有启动server/model或改全局配置。真实产品桥Cargo/CLI、Host Commands和simulation Engine仍须原P0/P1/P3-04接线及S07-05实际派发；具体缺产物/前置和本轮结论见[RR窗口4](../../docs/04_开发与验收/特征细化/RR_窗口4_报告关联核验.json)。

AQ-01：超过1MiB的原完整Response由managed_results只读句柄交给桥，按原操作Schema统一解包。MCP以application/json EmbeddedResource携带完整原响应，不能仅返回不可读取引用。句柄/Scope/失败清理见[共同合同](../../docs/03_架构与契约/大结果传输设计.md)；不新增业务操作或结果资源服务器。

## S07实际工程入口

S07时建立了可编译骨架及`--version`入口。当前P3-04已实现实际MCP到同一宿主业务桥，启动需要真实Host给出的管道和实例参数；缺宿主不自建Engine。全局客户端配置和真实Codex调用接收尚未完成。
