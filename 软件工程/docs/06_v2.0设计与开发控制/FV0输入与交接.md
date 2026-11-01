# FV0输入与交接

本页供A/B/C接手首批任务。日期：2026-10-31。历史结论来自现有验收和控制记录，本轮没有重新测试产品。当前任务与角色见[协作入口](GitHub协作入口.md)。

## 1. 可以直接取得的输入

|材料|入口|用途|
|---|---|---|
|正式运行包/可选示例/离线指南|[v2.0.1正式下载](https://github.com/JunyeFu/superfactory-releases/releases/tag/v2.0.1)|C基础和进阶实走；B对照已有可用范围|
|操作指南|[在线指南](https://github.com/JunyeFu/superfactory-releases/blob/main/USER_GUIDE.md)|不用开发者口头讲解的操作输入|
|源码与设计|[本规划分支的软件工程目录](https://github.com/JunyeFu/openatom/tree/codex/full-version-planning-20261031/软件工程)；源码继承`19d789b9520251ed2e9854e35c93b53f90227282`|B静态定位与既有测试阅读；A固定下一轮基点|
|H1原始输入与诊断载荷|[FV0内部输入下载](https://github.com/JunyeFu/openatom/releases/tag/fv0-inputs-20261101)|登录已有仓库协作者账号后下载；约110 MB，解压约1.23 GB；包内README逐项说明|
|执行归档实现|[execution_archive.rs](../../packages/commands/src/execution_archive.rs)|decode、load、capture和共享字节处理|
|会话与外部执行|[execution_session.rs](../../packages/commands/src/execution_session.rs)、[external.rs](../../packages/simulation/src/external.rs)|停止、阶段派工、排空、资源和业务效果|
|设备HTTP实现|[virtual_device.py](../../integrations/devices/virtual_device.py)|HttpClient、发送、观测和请求身份|
|归档现有测试|[host_execution_archive_tests.rs](../../packages/commands/src/host_execution_archive_tests.rs)|已有持久化预期和测试夹具；不是H1长历史本身|
|指南实走记录|[guide-walkthrough-20261029.json](../../.work/v2-full-target-20261028/guide-walkthrough-20261029.json)|已修报告问题、最终包重开等已有证据|
|正式包文案与压缩接收|[presentation-copy-20261029.json](../../.work/v2-full-target-20261028/presentation-copy-20261029.json)|区分代码、资源与文案版本|

不需要所有人下载源码：C使用正式应用和指南即可；B按需要拉取本规划分支，工作目录为仓库内`软件工程`。已有本地仓库可fetch分支，不必另复制整套依赖和模型。

2026-11-01已核对：21个首轮核心源码/环境/控制文件及四份协作文档的相对链接均在规划提交中存在；所查资源路径无Git符号链接或子模块。GitHub已有`wwl-ww`、`Jacky-z-q-wan`两位write协作者，尚未替负责人决定谁是B/C。第三方AI仍须使用有该私有仓库访问权的连接。

|角色|文件可用性|关闭任务还需要什么|
|---|---|---|
|A / #1|方案、控制入口、代码基点及H1下载均可定位|清点本人工作区未提交增量、确认测试环境/预算、接收B的环境和基线；未提交源码不能因本检查直接上传|
|B / #2|代码、原输入工程、R15904快照、63,584条历史、93设备计时和报告样例已可取得|先接通H1只读复现/测量入口，再完成规定实测；原长执行没有成功的Native存档，不能直接打开恢复|
|C / #3|正式应用、包内/在线指南、用于本卡历史回看的示例均可直接下载|本卡需要应用包和示例包；基础操作可先只用应用包。按本人机器独立实走并报告，不代填体验通过|

Windows新检出建议用短路径并只设置本仓库的长路径支持，避免第三方复核已发生的检出失败：

```powershell
git clone --filter=blob:none --no-checkout --single-branch --branch codex/full-version-planning-20261031 https://github.com/JunyeFu/openatom.git D:/sf/openatom
git -C D:/sf/openatom config core.longpaths true
git -C D:/sf/openatom checkout codex/full-version-planning-20261031
```

`D:/sf/openatom`应为未占用的新目录；已有仓库按原分支流程fetch，不重复克隆。C无需执行这些命令。

## 2. 三种输入的可用性

- **L1本地工厂：可直接使用。** 正式程序内置92设备模板，从模板开始创建；参考本地记录为69订单、162制造作业、273运输。逐产品结果和单位仍以冻结输入为准。
- **C1整厂云输入：源码设计和相关生成工具可读取，实际运行配置由A交接。** 93端点、身份绑定、云连接和精确工艺版本不能从“打开本地模板”推断已经就绪；FV0不要求C发起实时生产。
- **H1原始输入与历史载荷：已上传私有仓库。** 从上表内部输入下载页取得`SuperFactory-FV0-Inputs-20261101.zip`。其中`H1/source-project`为云执行前的原工程；`actual-frozen-inputs.json`、`actual-PG-snapshot.json`和`actual-PG-history.jsonl`为同一真实执行的冻结输入、R15904快照和63,584条原始观察记录。另附SDK发送计时CSV与输入摘要，不包含私有连接或SDK请求/响应载荷。

H1原长历史保存失败，已核对其目标目录为空；不能承诺这里有“保存成功的长执行工程”。B可做原始证据诊断、源码定位和输入工程测量；完整长历史序列化/保存/恢复须在本卡允许的测量夹具中接入原始载荷后实测。未接通的步骤仍待复现，短流程不能替代H1通过。C1实际最终历史是FV3输出，不是FV0/FV1前置。

**报告样例无需另传：** 正式示例`纸箱产运演示`内的`documents/run-00000001/result.json`，运行ID为`desktop-host-113016-1793178970914358700:run:1:1`，已核对含38,849条等待、69个订单和273趟运输。按指南“展示任务→确认打开→历史结果→评估报告”读取即可。该示例另有`documents/execution-00000002/source.json.gz`，用于小范围云产运历史回看；它不是93端点全量历史。比较任务按指南复制方案并生成第二次运行，不要求下载另一套相同模型。

内部包已提供目录与打开方式、历史身份、记录数及保存失败边界。压缩包共6,665个文件条目，ZIP读检通过，上传后的GitHub大小与摘要相符；仅验证材料传递，没有执行新的Native复现。A交下载链接，B仍需报告实际取得和读取结果。大载荷放私有Release，Issue只放入口和结论。

交接顺序为：A在#1交输入链接和测试环境说明→B确认取得文件、接通必要复现入口并在#2实测→A根据基线冻结预算。B实测不等待#1关闭。输入上传不等于B测量完成，FV0整体仍保持开放。

环境说明同时要列测试云服务/数据库的隔离方式、预算与操作授权；仅逻辑项目不同不能隔离服务代码或数据库schema变更。候选镜像、部署/恢复由B的链路卡交付，不能覆盖已有对外服务。

### 开发者的环境和资源交接

- 从[根README](../../README.md#开发环境与启动)读取构建方法；命令工作目录统一为`软件工程/`。当前声明为Windows 11 x64、MSVC构建工具、WebView2、Rust 1.97、Node 24.15.0、npm 12.0.1；实际使用以冻结提交的`rust-toolchain.toml`、`Cargo.toml`、`package.json`和锁文件为准。这是源码声明，尚未验证B机器已具备环境。
- B先报告已有工具版本和缺项；安装依赖按现有锁文件及README，不升级全局工具来迁就任务卡。Python设备/制包入口见[integrations/devices说明](../../integrations/devices/README.md)和现有脚本。
- 环境报告由B在双方约定的首个工作时段提交#2，A在#1引用接收，字段为`工具/实际版本或可用性/缺项/影响的步骤/处理人`。FV0关闭前确认基线测量环境可用；未确认投入时间不硬设1日承诺，不把安装版本清单当实际测试通过。
- 云服务是独立Cargo工作区，检查命令为`cargo test --locked --manifest-path apps/cloud-service/Cargo.toml --lib`；配置含义见[云服务说明](../../apps/cloud-service/README.md)。本文不要求执行其中的生产部署命令。
- A交出候选打包所需模型/运行资源/示例的已有取得位置和适用版本；B先确认资源可得，FV2再构建。打包脚本`tools/release-preparation/package_v2.py`不会编译exe，且需要显式输入；不能只交一个打包命令而把资源准备留空。
- C用自己的Windows账户和新工程查验。首次使用状态须实际确认，单独解压应用不能隔离Tauri用户配置；已有配置则注明复测，不清除原连接或个人数据。

### FV3脚本启动准备

下表仅核对`add556d0`中现存脚本的参数，尚不是可直接执行的FV3交接。脚本都会连接实际Native并可能操作生产/保存；没有`--help`处理。A在V2-10/FV02的FV2启动准备检查点完成必要适配和短链接线，完整负载仍在FV3运行。

|脚本（均在`tools/v2-demo/`）|当前位置参数|实际依赖与待处理项|
|---|---|---|
|`check_factory_cloud_native.mjs`|`base attempt [resume]`|读取`base/attempt/launch.json`、`base/st03-template-02/flow/actual-run.json`，另有写死的旧F盘receipt；Playwright/PowerShell指向原操作者目录|
|`check_factory_cloud_drain_native.mjs`|`base attempt [resume]`|同样依赖旧路径；约32行把目标填成`167`、单位选`piece`，不能代表混合单位的C1逐产品预期|
|`check_factory_cloud_visual_native.mjs`|`base [attempt=native-01] [resume]`|读取现用launch文件；Playwright位置及视图/时点选择需核对当前候选|
|`save_factory_cloud_native.mjs`|`base attempt [production=factory-live] [output=save] [resume]`|读取对应生产目录的`terminal.json`，使用`base/owned-dialog.ps1`；不能与不同输出名的生产记录混用|
|`reopen_factory_cloud_native.mjs`|`base attempt previous [resume]`|读取新旧launch、旧`save/receipt.json`和`factory-live/terminal.json`及固定模板记录；新进程与输入对应关系必须按本轮候选核对|

接线要求：复用现有脚本，替换已观察到的不适用路径和旧场景常量，按C1各产品/单位核对，不新增通用自动化平台。A交当前准确调用、输入/输出映射、有效launch文件、停止条件和短链结果；B消费运行标识采样，C只需可理解的观察步骤。云连接仍走应用已有私有配置，不虚构这些脚本拥有端点/令牌命令行参数。仅参数说明完成不等于脚本已适配或整厂已通过。

## 3. 需要调查的历史现象

|问题|已有事实|当前边界|
|---|---|---|
|整厂停止与排空|历史整厂运行存在待出料工作，正常停止/关闭未完成|短路线正常停止已验；整厂全量不因此通过。B定位具体等待/持有/移交条件|
|长历史保存|发生分配失败、保存未完成；历史记录的Native内存提交峰值至少23,800,000,000字节|共享归档字节和压缩/比较处理已有修复；短会话已验，H1完整范围仍需测。不能把该历史峰值当当前候选刚测得结果|
|93端点HTTP观测|历史SDK发送日志记录的平均HTTP耗时范围601—731 ms|仅平均HTTP现象；不等于p95、不等于设备到画面延迟，也不是实体设备产能|

来源：A现行本地`开发控制计划.json/full_factory_cloud_extension`的`archive_failure`、`long_archive_issue`、`performance_observation`及已上传H1输入。发送计时CSV来自93份只读SDK日志，不能用于推断Host到UI延迟；本页不是新的性能测量。

## 4. 已验结果摘要

ST05核心范围已有真实云服务与虚拟设备进程的四设备双工序案例：24条命令、12接受量，维护→替代承接→恢复、Host/云状态对应、保存与正常关闭有记录。最终另一个有限产运案例完成1件产出、3趟运输、15条命令，保存后新进程精确恢复。两者是不同案例，不能拼成93端点整厂全部订单通过。

报告曾因38,849条等待记录及大量文件/JSON展示卡住；已改为100条分页和按需展开，并有33项相关报告检查及最终包重开记录。运输历史白屏已补transport分支并验收。B/C先回归它们，有当前失败再新开缺陷，不按完全未实现处理。

## 5. 版本与交付边界

- 产品代码提交：`580d829b297228b87b7fe8389693a2a4cd363f1b`。
- 源码及后续展示文案提交：`19d789b9520251ed2e9854e35c93b53f90227282`。
- 正式运行包：`SuperFactory-2.0.1-Windows-x64.zip`，137,252,162字节；示例310,750,775字节；独立HTML指南26,639字节。
- 本规划分支只加入协作材料，不更新上述发布资产、不修改已提交链接、不恢复云生产或旧自动化。

输入缺失的报告格式：`缺哪项输入 → 哪些步骤仍可开展 → 哪个验证因此不能完成 → 由角色A交接`。未测留未测，不把缺输入写成产品通过或无问题。
