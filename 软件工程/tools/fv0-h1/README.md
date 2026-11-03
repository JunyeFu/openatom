# B 的 H1 长归档复现入口

这是 FV0 首轮诊断工具，调用仓库现有的 Rust 归档类型和 ProjectHost，离线完成长历史读取、保存及新进程重开。无需云账号、数据库、原作者目录或正在运行的超级工厂。

## 1. 下载与准备

用已获授权的 GitHub 账号打开[私有材料发布页](https://github.com/JunyeFu/openatom/releases/tag/fv0-inputs-20261101)，下载两个文件：

- `SuperFactory-FV0-Inputs-20261101.zip`：真实历史输入，约 110 MB。**保持 ZIP，不用手动解压。**
- `SuperFactory-FV0-H1-Repro-20261101.zip`：本工具。完整解压到短路径，例如 `D:\sf-h1\tool`；里面应直接看到 `run_h1.py`、`fv0_h1.exe`、`actual-prepared-R93.json` 和本说明。

要求 Windows x64、Python 3.10 或以上；先运行 `python --version`。如果本机使用 Python Launcher，将下面命令的 `python` 换成 `py -3`。预编译工具不要求安装 Rust、Node、数据库或额外 Python 包。建议预留 8 GB 磁盘及 8 GB 可用内存；不要同时运行全厂演示或其他大内存任务。正式运行包无需替换。

exe 依赖 Windows 的 Visual C++ v14 x64 运行库。若启动提示缺少 `VCRUNTIME140.dll`，按[微软官方说明](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170)安装 x64 Redistributable，再执行本命令；不需要安装整套 Visual Studio。

## 2. 一条命令运行

在 PowerShell 中执行，按实际下载位置修改两个路径：

```powershell
python D:\sf-h1\tool\run_h1.py --inputs D:\sf-h1\SuperFactory-FV0-Inputs-20261101.zip --output D:\sf-h1\result-01
```

输出目录必须尚不存在，防止覆盖已有结果。程序自动解出输入、构造夹具，再依次启动三个独立进程进行 codec、save、reopen；控制台每30秒显示进程和内存进展，debug大工程可能需要数分钟。需要中止时按Ctrl+C，仅结束本工具创建的子进程，保留部分输出并记失败，不算正常保存。成功时最后显示 `PASS: ...\report.json`，进程退出码为 0。失败时查看 `report.json` 的 `error` 和相应 `*.stderr.txt`；保留失败结果，修正原因后换一个新的输出目录。缺少输入、权限不足、内存不足均按实际原因回报，不能标通过。

## 3. 它实际核对了什么

- 原始 63,584 条观察记录完整、有序；R15904 快照、93 台设备、冻结工程/场景/995 个模块及模型资源由生产归档读入代码校验。
- codec 单独测量类型解码、JSON 序列化和压缩；这是定位阶段耗时的拆分测量，不能相加后冒充界面保存耗时。
- save 通过真实 ProjectHost 执行导入、采纳和保存；reopen 在另一个操作系统进程中重新导入保存结果。
- 两个 Host 进程都要求最新快照与原始 R15904 逐字段相同，并通过实际历史分页接口读完全部 63,584 条记录；归档不恢复生产时钟或设备控制权限。
- 保存后的模型、模块、原本地 Run 和执行归档文件逐字节比较，确认持久化未丢失这些输入。

输出中 `fixture/` 是重建夹具，`saved-project/` 是本次 Host 实际保存的工程；`report.json`、`*-metrics.json`、`*.jsonl` 为测量结果。保留两份工程供失败定位，本轮不再复制整个输出。

## 4. 必须保留的测量边界

原执行没有成功保存完整长归档。工具使用原始历史及冻结输入，保留真实 R93 准备态和 R15904 最终帧；R0 的对象状态由最初 626 条真实记录重建，时间零元数据取自真实 R93 准备态。**R0 是明确标记的派生基线；没有伪称找回原 R0。** 生产读取器会验证记录是否准确重建这些帧。

这是三帧、全量观察历史的只读复现夹具，不含原执行所有中间快照，不能据其通过宣称原先 23.8 GB 峰值已重现或消失、原会话已正常停止、全厂订单已完成。它足以让 B 定位当前真实载荷的类型解码、保存和重开成本。

预编译程序是 **Rust 1.97 / debug** 构建，使用当前规划分支继承的产品代码。`peak_working_set_bytes` 是 Windows 工作集峰值；`peak_commit_bytes` 是 Windows 进程提交量峰值；`sampled_peak_private_bytes` 是每 50 ms 采样的单个 helper 进程 Private Bytes 峰值。这些值不能替代 Native 应用及 WebView2 进程树峰值、FPS、端到端通信延迟或 10 分钟持续运行数据。Python 准备夹具的内存未计入 helper 指标。不同 profile/机器的数字须分开列。

## 5. B 的首轮回报

在 [Issue #2](https://github.com/JunyeFu/openatom/issues/2) 提交：

1. 实际机器、Python/系统版本、运行命令，以及本机是否具备源码构建环境；下载权限和缺项也写明。
2. 附 `report.json` 和三份 `*-metrics.json`，无需再次上传数 GB 工程。列出各阶段时间、内存口径、运行成功或失败；对照[本次交接验证](https://github.com/JunyeFu/openatom/blob/codex/full-version-planning-20261031/软件工程/tools/fv0-h1/verification.json)只比较相同范围。
3. 结合 `execution_archive.rs` 的 decode/load/capture，给出有测量支持的热点或“当前夹具未复现”，不要看到 `clone` 就认定根因。停止排空仍按历史证据定位 `execution_session.rs`；通信读取输入包中的 `sdk-send-attempts.csv`，不将 HTTP 平均数当端到端 p95。
4. 提交三类问题的结论、最小修改建议和未测范围，由 A 冻结后续预算。工具完成不自动关闭 #2，也不替代 B 本机执行和 Native 体验测量。

如需在正式应用中辅助查阅：使用“打开工程”选择 `result-01\saved-project`，按[现有操作指南](https://github.com/JunyeFu/superfactory-releases/blob/main/USER_GUIDE.md)查看本地历史与云执行归档；不要连接原云会话或继续生产。正式应用的界面可用性由其实际操作结果单列，命令行成功不自动记作 Native 通过。

## 6. B 修改测量代码后如何重建

在规划分支仓库的 `软件工程` 目录执行：

```powershell
cargo build --locked --manifest-path tools/v2-demo/Cargo.toml --bin fv0_h1
```

Rust 及 MSVC 环境按仓库 README 准备。默认产物位于 `tools/v2-demo/target/debug/fv0_h1.exe`；若设置了 `CARGO_TARGET_DIR`，在该目录的 `debug` 下取文件。仅替换自己的工具副本中的 exe，并在回报写明构建提交、profile 和改动。Python 入口在 `tools/fv0-h1/run_h1.py`，Rust 入口在 `tools/v2-demo/src/bin/fv0_h1.rs`。本工具不修改产品源码、正式安装包或已提交的公开下载页。

FV1 保存失败验证仅在一次性 H1 存档副本运行。先用 save 生成独立输出目录，再用 `fv0_h1.exe check-failed-replace 副本目录 actual-PG-snapshot.json` 锁住副本根清单并尝试保存，核对失败及旧根字节。最后以独立进程运行 `fv0_h1.exe reopen 副本目录 actual-PG-snapshot.json` 检查原快照与全部历史。该命令会写入副本目录，保留原始输入。
