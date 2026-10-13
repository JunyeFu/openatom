# 本地Agent桥

首版预定Rust stdio MCP进程，经当前用户宿主IPC连接已打开桌面工程；不保存第二份项目/运行库存。命令、范围和结果复用packages/commands，当前尚无可运行进程。

实际客户端配置、握手、渲染反馈和调用验收由实现阶段完成。[接入设计](../../docs/03_架构与契约/通用Agent接入与专业软件连接器.md)为权威入口。此设计不修改用户全局MCP设置，不提供公网/多用户服务。

## 现行目标生成与开放主线

18项计划操作由桥连接同一桌面宿主；query_module_library和check_factory_draft用于SF-17，skills驱动填参/运行/修正/导出，不在桥内另建引擎。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

S02-01将本目录及packages/commands、integrations/agent纳入P3-04的release_paths；本平台实际stdio桥为首版必做，共享宿主、不自建Engine。外部专业软件连接器仍后置。范围见[冻结清单](../../docs/00_项目基线/首版交付基线.md)。
