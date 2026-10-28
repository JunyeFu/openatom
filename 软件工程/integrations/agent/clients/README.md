# 客户端配置样例位置

2026-10-17：普通工作台已有“Agent → 连接、权限与任务…”入口，可读取实际Host地址、确认Windows验证的连接与操作/字段范围、撤销权限、选择目录用途和转授既有任务。字段输入与已授予权限分开；关闭面板不自动撤销连接。当前受控UI交互10项通过，实际Native/Codex业务接收仍单独记录。

仓库内提供[codex-session.py](codex-session.py)启动入口：显式传入实际桥程序、当前Host管道/实例、UTF-8任务说明文件及一个新的输出目录。它先确认已有ChatGPT登录，通过本次进程的`-c`设置MCP，不编辑全局配置，不使用API-key计费。`superfactory`标为必需服务，避免可选服务尚未进入初始工具目录时会话已结束；项目根以本产品`Cargo.toml`定位，避免嵌套副本继续向上继承项目说明。CLI会话日志保存在指定目录；普通桌面仍须确认对应Windows验证的连接和精确权限。程序存在及`--help`通过不等于实际Codex互通通过。

```powershell
python integrations/agent/clients/codex-session.py --bridge 'D:/实际发布包/superfactory-mcp-bridge.exe' --host-pipe '实际Host管道' --host-instance-id '实际Host实例' --prompt-file 'D:/实际任务.txt' --output 'D:/新的验收目录'
```

2026-10-21独立技术验证使用已有ChatGPT登录的`codex-cli 0.163.0`，实际完成`describe_capabilities`、`get_context`、`get_object`，同一隐藏Native managed Host的P1完整草稿前后不变。无编辑grant时，真实目录未暴露`apply_commands`，未发生写入；这不是已提交命令后的Host拒绝错误验证。仅授权P1 `/title`的独立案例发起一次写调用，但真实客户端返回`MCP tool call requires approval, but approval policy is never`，已停止且Host仍为原标题/revision 1，标题修改与revision增加未通过。该受控Case授权不是真人确认或普通GUI验收，亦不代表完整GF、九类创作或整包通过。

启动器在`client-receipt.json`分别记录实际CLI退出码和`launcher_exit_code`：CLI失败保留其退出码；CLI退出0但没有superfactory业务调用时返回2，出现失败的superfactory MCP调用时返回3。实际Host业务错误仍须读取完整Response及前后草稿判断，退出0不能替代业务验收。上述审批拒绝后仅用保留的真实事件验证退出码分类，没有重试写入或更改审批策略。配置仍需要当前Host的真实endpoint/instance ID和明确授权。

接口依赖和工作流见[接入设计](../../../docs/03_架构与契约/通用Agent接入与专业软件连接器.md)；客户端能使用哪些工具由实际tools/list决定。

## 现行目标生成与开放主线

实际客户端配置后仍需调用真实可用的工具及项目skill验证SF-17目标生成与反馈；上述只读接点和被审批阻止的单字段案例未验证完整生成、运行、渲染、报告或人工接手流程。

[主线与范围](../../../docs/00_项目基线/项目主线.json) · [完整生成链](../../../docs/03_架构与契约/目标驱动厂房生成.md)。

本批[Codex stdio配置审阅稿](codex-stdio.design.toml)仅消费官方command/args格式，保持未启用，使用时须填当前构建的 exe 绝对路径及当前桌面 Host 参数，不能把占位参数当已接入。它不是已验证客户端样例。连接、断线、取消和画面附件统一取[连接器定义](../stdio-connector.design.json)；原生 fixture 与真实 Host/Engine 进程证据分别记录，实际 Codex 互通仍空。实际复现入口见[接入说明](../README.md)，当前接收状态见 P3-04 整包执行记录。
