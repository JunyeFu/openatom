# Agent工作台交互验证

产品入口为普通工业工作台的“Agent → 连接、权限与任务…”。组件复用当前 `WorkspaceSnapshot`，Native调用均指向已经运行的 `AgentPipeState`；不创建Host或Engine。组件接入的身份为当前UI会话与工程ID，真实字段变化/忙碌状态进入同一主窗关闭保护。

`node tests/delivery/mcp/agent-ui/browser.mjs` 使用现有8922开发服务和已有Playwright运行时，检查真实产品组件。当前10项通过：初态清洁、字段改回清洁、配置仅复制、默认只读授权、错误JSON保留、权限拒绝保留、授权不清掉未保存目标、目录用途与连接绑定、真实工程修订观察触发刷新且保留输入，以及卸载后的晚到响应。证据在 `tests/delivery/mcp/output/full-package-20261017/agent-ui/`。

这是固定Native命令返回值的受控UI检查，不代替实际Windows连接、Codex、Host业务和四格式交付；这些分别由P3-04实际进程与完整工作流接收。页面顶端保留该标识。
