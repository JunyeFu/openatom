# DRF04 Windows 管道对端身份实际试做

2026-10-14，新增3例通过。[原始结果](DRF04_stdio-proof/output/pipe-peer-identity-result.json)保存实际进程、会话、用户SID、登录SID、请求、返回和关闭结果；旧40/24/7/14/11例与既有stdio/只读句柄试做均未重跑。

server为本次PowerShell进程43116；3个隐藏bridge子进程为51524、50576、3416。server实际调用GetNamedPipeClientProcessId/SessionId，bridge实际调用GetNamedPipeServerProcessId/SessionId。双方再打开由管道API所得PID的进程令牌，读取TokenUser、TokenGroups登录SID、TokenSessionId，并与当前实际令牌比较。观测会话均为1，实际登录SID一致；比较值来自API结果，没有预置same_PID或用正文PID打开进程。

|实际案例|输入与结果|
|---|---|
|spoofed_body|预期实例正确；正文客户端PID冒用server PID，server PID=1，SID与Host声明均伪造。隔离server只使用实际对端身份，正确预期实例获准；伪造声明未成为授权来源。|
|wrong_instance|预期实例为另一GUID，即使正文填写真实Host实例与server PID，仍返回ExpectedHostInstanceMismatch。|
|disconnect|bridge在实际身份查询/ready之后直接关闭，无请求正文；server读到实际EOF。前两例回复后也读到EOF。|

每个已知自有server/client管道句柄、查询令牌/进程句柄和Start-Process进程句柄均实际关闭；管道及子进程句柄在关闭后GetHandleInformation返回ERROR_INVALID_HANDLE=6。三个自建bridge均正常退出，没有终止其他进程。显式当前登录SID DACL、FIRST_PIPE_INSTANCE和REJECT_REMOTE实际用于创建，但本次没有执行跨登录/远程拒绝案例。

复现入口：[pipe_peer_probe.ps1](DRF04_stdio-proof/pipe_peer_probe.ps1)，客户端为[pipe_peer_client.ps1](DRF04_stdio-proof/pipe_peer_client.ps1)，P/Invoke源为[PipePeerNative.cs](DRF04_stdio-proof/PipePeerNative.cs)。在软件工程根目录用现有pwsh运行该脚本即可；它以Add-Type内存编译，无新依赖或锁文件，子进程Hidden，仅处理本次Start-Process对象。帧使用实际u32-LE长度及UTF-8 JSON。

本结果证明Windows身份/实例检查、管道EOF和自有句柄清理原语。握手载荷仅为隔离ready/expected-instance试样，完整canonical IpcHello/Welcome、产品Host连接/授权注册表、Commands、受管结果、Renderer、业务断线继续和Codex尚未执行。S05实际产品结果仍空。

S04最新[授权源](../完整设计与执行准入.json)已经核实用户“代替我完成S04-04/S04-05”，窗口2正在执行代行走查；本窗[32字段/6状态](DRF04_H04-04界面绑定.json)供其消费，等待实际证据及窗口1同步。S05缓存范围和版本缺项见[准备包](DRF04_S05接点准备.json)。
