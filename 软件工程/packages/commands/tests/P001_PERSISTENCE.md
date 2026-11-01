# P0-01 专项持久化验证

`p001_persistence_closeout` 是 Windows 专项测试，默认显式忽略，避免普通测试隐含要求证据目录、SID 和故障注入特性。它覆盖真实保存失败 F01—F04、同修订的两种提交顺序、原子命令批、保存期间继续编辑、撤销重做及异常导入。

运行前将 `SF_P001_EVIDENCE` 指向全新独立目录，`SF_P001_SID` 设为当前用户 SID，TEMP/TMP 和 Cargo target 指向自有目录。执行：

```powershell
cargo test --manifest-path Cargo.toml -p sf-commands --test p001_persistence_closeout --features nfr06-save-faults --locked --offline --target-dir <独立target> -j 1 -- --ignored --test-threads=1 --nocapture
```

F01 仅对测试创建的数据目录临时拒绝创建子目录，退出该分支时恢复；F02 使用真实 Windows 共享锁；F03/F04 消费既有测试故障特性。不要针对用户工程运行，也不要将故障特性用于普通发布包。

五个顶层测试包含四个实质组合及一个子进程入口；入口在父进程无子任务参数时返回，不单独计作业务覆盖。七个新 OS 子进程分别读回、编辑、运行和另存，并比较旧 Run。F04 原故障目标仍不健康，只证明保留副本能恢复，不能报告原目标已修复。

此测试不证明普通 Native、真实断网、外部 MCP 客户端或正式性能验收。

`p001_professional_reopen` 补专业组合的新OS接手：复用r12-publication实际保存夹具中的2份成果、2条测试用途记录、2条绑定、2份旧Run及3份SVG，由真实Host另存并搬移，再由子进程读回、编辑、另存。导入的用途/绑定不恢复原生产者的运行授权；该拒绝为预期。它不重算几何或仿真，不代表真人认可或普通Native。

将`SF_P001_PROFESSIONAL_EVIDENCE`设为父目录已存在、目标目录尚不存在的新路径，然后单独执行：

```powershell
cargo test -p sf-commands --test p001_professional_reopen --locked --offline -- --ignored --exact p001_professional_saved_archive_survives_move_and_new_os_process --nocapture --test-threads=1
```

父测试与子入口合计一个实质场景；子入口不另外计覆盖。2026-10-24的Root接收使用当前Native依赖组的既有rlib直接链接测试，无新增Cargo缓存，实际回执见`.work/noncu-closeout-20261024/p001-professional-process/receipt.json`。


## 非空CC0素材贡献ZIP与新OS接手

`p001_asset_contribution_reopen.rs`复用项目现有assembly详细/简化0.4.1 CC0素材及其精确依赖，由生产Host导出community_module ZIP、真正搬移，再由新OS进程导入并目录另存。新Host没有注册素材库；测试单独读取原文件仅作字节预期。核对4份GLB/PNG、全部36份声明文件（含纹理/源码/2份LICENSE）、定义/依赖/权利完整，原工程及旧Run不变，贡献包不带场景/实例/结果。原许可证已明确，测试Host确认不代表新增真实权利人签认。

设置`SF_P001_ASSET_EVIDENCE`为尚不存在的新证据目录（父目录已存在），执行：

```text
cargo test -p sf-commands --test p001_asset_contribution_reopen --locked --offline -- --ignored --exact p001_nonempty_cc0_contribution_zip_survives_move_and_new_os --nocapture --test-threads=1
```

只有一个实质父场景；子入口不另计通过数。该结果是Host/程序证据，不替代Native素材导入/渲染质量、真实断网或正式NFR采样。本轮实际回执：`.work/noncu-closeout-20261024/p001-asset-contribution/receipt.json`。

默认执行的`cc0_media_exact_version_matches_after_archive_roundtrip`覆盖同一素材的目录保存重开、贡献ZIP回收和已注册相同内置库的精确版本查询；再由共同Command实际采纳包内依赖，不创建设备、库存或方案。改动声明源文件后，目录报冲突，真实采纳也必须拒绝且工程不变。贡献包不必携带库内部未声明的`original-module.json`；模块元数据与声明源文件仍须一致。此回归不需要证据目录环境变量；精确版本元数据和声明文件冲突的拒绝由`full_gf_applicability`相关用例共同验证。
