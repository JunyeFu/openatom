# DRF04 原生接点隔离试做

使用现有依赖/已构建exe和Python，不安装、不改Cargo清单/锁或共享源。输出仅在本目录output。

1. 在本目录执行现有`tools/architecture-proof/target/debug/superfactory-architecture-proof.exe`绝对路径。真实官方rmcp3.5.1客户端启动子进程stdio，发现lab_context/lab_set_duration，经当前登录SID+DACL Windows管道访问fixture；同修订两调用仅一个提交，实际后台工作取消与完成先胜分别测得。输出[native-trial-result.json](output/native-trial-result.json)。fixture用按行管道消息，**未**验证产品规范u32长度/IpcHello/18操作或Codex。
2. 使用现有Python执行[managed_asset_probe.py](managed_asset_probe.py)。父进程打开本目录自建PNG副本并向隐藏子进程DuplicateHandle只读句柄；匹配scope/修订实际读出字节，WriteFile返回AccessDenied=5，错修订不返回字节。输出[managed-asset-result.json](output/managed-asset-result.json)。此scope判断是fixture期望，不是产品Host结果注册表或实际Renderer。

产品附件选择/接口及可复现剩余接点见[连接器](../../../../integrations/agent/stdio-connector.design.json)。真实Host授权、规范IPC附件、Renderer、Codex、GF仍待产品接线；S04由窗口2按用户明确授权代行走查，实际结果及窗口1准入同步待回收。上述子集不写actual_product_runtime或产品actual_result。

3. 新增[pipe_peer_probe.ps1](pipe_peer_probe.ps1)通过现有pwsh/Add-Type执行[PipePeerNative.cs](PipePeerNative.cs)，创建显式当前登录SID DACL管道及三个Hidden bridge子进程。双方实际查询管道对端PID/session和实际进程令牌，3例覆盖正文伪造字段、错误预期实例拒绝、实际断开EOF及已知自有句柄关闭。输出[pipe-peer-identity-result.json](output/pipe-peer-identity-result.json)，完整解释见[试做记录](../DRF04_管道对端身份试做.md)。它使用u32长度帧，但不是产品完整握手/注册表；没有重跑上面两项或旧有限模型。
