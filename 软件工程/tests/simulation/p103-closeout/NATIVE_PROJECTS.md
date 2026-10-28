# P1-03 真实 Host 工程与新进程读回

`native_projects` 生成七个真实工程，覆盖三阶段、可中断恢复、辅助资源等待、输出阻塞及原 IX-01 基线/延迟。固定业务预期位于 `inputs/native-phases/expectations.json`。

此入口写出工程，因此默认忽略；不是普通 `cargo test` 已通过的业务覆盖。执行时将 `SF_P103_DELIVERY` 指向全新独立目录，并使用自有 TEMP/TMP 与 Cargo target，然后显式执行：

```powershell
cargo test --manifest-path tests/simulation/p103-closeout/Cargo.toml --test native_projects --locked --offline --target-dir <独立target> -j 1 -- --ignored --exact phase_projects_are_actual_host_runs_saved_and_reopened_in_new_os_processes --nocapture --test-threads=1
```

父测试逐一调用七个新 OS 子进程。不要单独运行子入口，也不要直接使用不带 `--exact` 的 `--ignored`。输出中的 `host-result.json`、每例 `readback.json` 和 `projects` 是实际结果；工程可供 Native 回看，但程序结果不代表普通 Native、正式性能或整包验收。
