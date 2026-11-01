# FV0：角色A首卡交付

2026-11-01 · [Issue #1](https://github.com/JunyeFu/openatom/issues/1) · V2-00/FV01。A已完成可独立落实的版本、输入和控制入口整理；等待B复核与本机基线，首卡仍开放。此次没有修改产品实现、运行新生产、部署云服务或替换正式发布。

## 1. 共同起点与分工

|对象|冻结值与用途|
|---|---|
|下一轮共享源码基点|`4ff2fc17cf678071f4b3bd93674dafdb8b90695b`，包含现有产品源码、协作材料和H1工具；产品目录与`19d789b9520251ed2e9854e35c93b53f90227282`一致|
|集成分支|[`codex/full-version-integration-20261101`](https://github.com/JunyeFu/openatom/tree/codex/full-version-integration-20261101)，从上述完整SHA建立；A文档经PR交B复核，后续按任务分支提交|
|协作说明分支|`codex/full-version-planning-20261031`，本次只增加A交付；它会继续更新，复现基点始终使用上面的完整SHA|
|产品实现 / 已交付源码|`580d829b297228b87b7fe8389693a2a4cd363f1b` / `19d789b9520251ed2e9854e35c93b53f90227282`；源码与展示文案版本分别记录|
|正式发行|[v2.0.1](https://github.com/JunyeFu/superfactory-releases/releases/tag/v2.0.1)：应用137,252,162字节、示例310,750,775字节、HTML指南26,639字节，仍使用原链接|

A负责本卡和集成；B复核版本/输入并在[#2](https://github.com/JunyeFu/openatom/issues/2)交环境与测量；C继续在[#3](https://github.com/JunyeFu/openatom/issues/3)使用正式包查验。用户已确认B为`wwl-ww`、C为`Jacky-z-q-wan`，分别负责#2和#3；两人已有write权限。三人投入时段及日历目标尚未确认。

### 已有工作区增量的处理决定

以冻结基点比较：产品`apps/packages/integrations`没有未合入源码；24个已有跟踪文件差异均留在原工作区，未并入本卡。A负责后续接收，实际作者未凭时间戳或文件名猜测。

|既存差异|数量|归属与本次处理|
|---|---:|---|
|`设计预览/`的多层视图、当前交互规格、设计ZIP、阅读说明|4|设计增量，原作者待确认；后续由A接收、C查设计差异，本次不合入|
|`软件工程/AGENTS.md`|1|本地执行约束，保留本地，不作为产品交付|
|收口计划、开发控制设计、开发控制计划JSON|3|已有控制增量；本卡只提交基点上的A入口和状态修改，不夹带整份本地内容|
|`演示视频/`制作适配、叙事、脚本、运镜的MD/JSON|8|原材料作者待确认；不在本窗口制作、派发或提交|
|`tools/3d-proof/output/`六个GLB和两个测量JSON|8|模型候选与测量增量，原作者待确认；需要具体设计任务接收，不自动替换模型|

160,940个未跟踪文件是本次盘点范围，不能据“未跟踪”判为无用：场景/运行工程109,439、一般output 46,479、tools输出1,803、非视频docs 1,562、测试输出682、构建缓存608、Playwright181、模块研究137、v1存档25、视频23、局部说明1。保留原地；历史与人工作品不能整体加入提交或删除。已发布的H1输入包和工具通过现有链接复用，不再打包全部工作区。

## 2. L1/C1输入与逐产品预期

输入来自[私有输入Release](https://github.com/JunyeFu/openatom/releases/tag/fv0-inputs-20261101)的`SuperFactory-FV0-Inputs-20261101.zip`：`H1/source-project`是执行前工程，`H1/actual-frozen-inputs.json`含实际工程、场景、模块和绑定定义。静态核对两者的ProjectState、场景内容相同；原始文件保持原样。

- **L1**：使用该工程和场景，模板`community.template-paper-packaging-factory@1.7.1`；92台设备、1个厂房、12条产线、266连接、395位置、995模块引用、71产品/BOM、102工艺路线，69订单、71条场景production_tasks。162是历史实际制造作业数，不能混写为任务定义数。正式包可从模板开始体验；比较性能与数量时用这份冻结工程，不自行修改订单。
- **C1**：同一工程和场景加原始`candidate_bindings`，944绑定、93个唯一端点；多出的`演示工厂:PF-CARRIERS`是共享转运端点。这里只冻结输入定义；新的实际云会话、隔离连接和脚本适配在对应链路/联验卡交付。打开本地模板不等于完成云接线。
- **数量预期**：[FV0冻结输入.json](FV0冻结输入.json)逐项保留69个订单的产品及版本、精确数量/单位、目的地、验收阶段和工艺引用。所有记录的存储单位均为`piece`，但含不同产品/处理项，167只是不具有统一产品含义的算术合计。最终按每个订单核对到达并放行的产物，不可多计/重复计数；允许部分接收不等于整个订单已完成。
- **参考量**：历史本地69订单、162制造作业、273运输、38,849等待；无扰动1.7.1参考1578阶段命令，只作对照。维护、故障与改派会改变轨迹和命令数；不以固定命令总数强判成功，也不混用1.6的962命令参考。

输入JSON是可消费的验收预期清单，不是新执行通过回执。原冻结执行身份和历史回执仅用于H1重放；C1新执行必须建立新的会话/Run，不复用旧云执行身份。

## 3. B/C现在就能取用的材料

|目的|获取及使用|边界|
|---|---|---|
|B：H1归档与失败基线|同一私有Release另下`SuperFactory-FV0-H1-Repro-20261101.zip`，按[工具说明](../../tools/fv0-h1/README.md)执行输入包＋工具包；从空结果目录开始，向#2交工具报告和机器环境|可直接运行预编译工具，无需Rust、数据库或云凭据；Native是单独测量|
|B/C：大报告|正式示例`纸包装产线演示`中`documents/run-00000001/result.json`；原Run `desktop-host-113016-1793178970914358700:run:1:1`，38,849等待；按指南从展示工厂打开历史运行/报告|不能用全新空工程代替这个负载|
|B/C：小云历史比较|同一正式示例`documents/execution-00000002/source.json.gz`；通过应用打开历史执行|R410有限案例：1产物、3运输、15命令；不作为93端点全订单完成证据|
|C：独立体验|[正式包、示例、指南](https://github.com/JunyeFu/superfactory-releases/releases/tag/v2.0.1)，体验历史和报告需同时下载示例|首次配置按自己实际账户记录，换解压目录不等于清空用户配置|

现有[H1交接验证](../../tools/fv0-h1/verification.json)已从GitHub下载两包完成Host保存/新进程重开：63,584条历史、93设备、R15904读回；保存158.312秒，Host打开74.216秒，单helper采样Private Bytes峰值约4.52 GiB。这是交接机的debug离线测量，**不是B本机基线、Native可操作时间、PF06进程树或PF07通过**。原长执行仍是保存失败、0最终产出且2作业待出料的历史；工具重建R0并保留R93/R15904三帧，不声称复现原会话全部中间快照密度。

## 4. 源码、模型和构建资源入口

C不需要克隆。B已取得源码时只fetch所需分支；新克隆使用空的短路径。以下命令在Windows PowerShell执行，GitHub需登录有私库权限的账号：

```powershell
$env:GIT_LFS_SKIP_SMUDGE = '1'
git clone --filter=blob:none --no-checkout --single-branch --branch codex/full-version-integration-20261101 https://github.com/JunyeFu/openatom.git D:/sf/openatom
git -C D:/sf/openatom config core.longpaths true
git -C D:/sf/openatom switch -c codex/b-fv0-baseline 4ff2fc17cf678071f4b3bd93674dafdb8b90695b
Remove-Item Env:GIT_LFS_SKIP_SMUDGE
git -C D:/sf/openatom lfs install --local
git -C D:/sf/openatom lfs pull --include="软件工程/modules/**,软件工程/resources/v2-devices/**,软件工程/tools/architecture-proof/assets/**" --exclude=""
```

需要已安装Git LFS；缺项在#2说明后按项目依赖准备。不要仅用GitHub源码ZIP中的LFS指针当模型或Python ZIP。现有工作区有修改时不要直接切分支覆盖它们。

|资源|基点内入口|
|---|---|
|应用构建、Rust/前端环境|[根README](../../README.md)、`rust-toolchain.toml`、`Cargo.toml`、`package.json`及锁文件；命令目录为`软件工程/`|
|打包实际资源映射|[tauri.conf.json](../../apps/planner/src-tauri/tauri.conf.json)的resources，154条声明均在冻结Git树中存在|
|设备模型、媒体、模板|`modules/devices/`、`modules/media/`、`modules/templates/`，版本按各模块清单；勿复制本机未合入模型output覆盖它们|
|内置Python与设备资源|`resources/v2-devices/`及其runtime.json；`python/python314.zip`属于LFS|
|报告字体|`tools/architecture-proof/assets/`，跟随Tauri资源映射|
|云服务/设备SDK|[云服务说明](../../apps/cloud-service/README.md)、[设备说明](../../integrations/devices/README.md)；云Cargo是独立工作区|
|制包|[package_v2.py](../../tools/release-preparation/package_v2.py)，消费已编译exe；本脚本不会编译exe|

2026-11-01核对：上述映射涉及570个LFS路径、518个唯一对象，总938,625,862字节；GitHub LFS batch为全部对象返回下载动作，无缺失。实际下载4,156,546字节的Python标准库ZIP，563条目CRC通过。其余模型只检查远端可取元数据，未重下载全部模型，也未执行新源码构建。B交回本人clone/LFS结果及Rust/MSVC、Node/npm、Python、WebView2版本和可用性，不能把本记录当B机器已就绪。

## 5. 本卡已定范围与待接收项

|项目|A决定 / 后续接收|
|---|---|
|构建profile|先沿用debug；FV3与FV4使用同一候选/profile。变更由A/B明确后重新解释性能差异|
|测试权限|FV0离线H1和本地报告可做，不需要云资源；实际云环境改动交对应链路卡，服务/schema变化使用隔离候选，不覆盖原sf-cloud或数据库|
|云预算|15代币是历史批准额度，当前余额/剩余额度未确认；本卡没有新增资源或费用。不因此阻止离线测量；实际云卡开工前由A确认可用额度、规格和访问|
|指标|PF06—PF09及基础20/进阶30分钟保持[方案](入围后完整版制作与验收方案.md)中的候选目标，收到B本机基线及C首次使用反馈后才冻结，不自行放宽已观察到的不达标|
|B接收|在#2回报完整提交号、输入是否可读、环境/缺项、测量范围和结果；A据此更新#1第二检查点。截止按实际可投入时段约定|
|任务流转|仍为五阶段15卡；具备自身输入和复现的FV1卡可单独确认开工。本次仅执行A首卡，未自动启动下一张；FV0整体未关闭|

## 6. 赛事要求与日历

2026-11-01检索到的[赛事官网内容](https://www.oschina.net/oa2026/)列出：初评材料截至10月31日、入围名单11月6日、决赛11月13日，入围团队现场路演。网页索引列明技术创新30%、场景落地30%、开源治理20%、长期发展20%；完整可运行源码、介绍材料和演示入口属于已有要求。本次直读页面未返回可解析正文，以上依据官网的检索索引；个人新通知、是否入围与具体现场安排未收到，不据此宣称已入围或已完成正式提交。

原11—22净工作日是完整版工量估计，不是本届11月13日前全部完成的承诺。五阶段与质量目标保留；A收到人员投入、B/C结果及新通知后确定赛前可交付范围和完整版日期，不能靠压缩验证或提前打勾凑日历。

源码仓库目前私有，现有两位协作者可写；公开运行包不能替代评委源码访问。评委账号/授权方式待A确认，本卡不擅自公开源码或发出邀请。已提交的公开仓库、正式tag、发行资产及页面保持原状。本窗口不制作或安排PDF和视频。

## 7. 本卡验收状态

A当前任务已更新：[协作入口中的A队列](GitHub协作入口.md)；#1是当前主任务，先收口FV0；随后FV1处理停止排空，再到FV2的[#5前端卡](https://github.com/JunyeFu/openatom/issues/5)。#5当前待启动。已交付输入不重做，任务更新不代表产品已开始修改。

已交付：冻结源码与集成分支、既存增量分类、L1/C1逐订单预期、H1和报告入口、LFS资源取用说明、当前控制入口纠正、赛事索引与待定项。静态核对不算产品运行验收。

待接收：B对本交付的复核及#2环境/基线；C已交自动化查验，真人操作复测及体验时间待补；A确认投入/交期、必要云资源额度与评委源码访问。第二检查点完成后再关闭#1；不因缺外部决定而扣住已可使用的输入。
