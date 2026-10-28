# 超级工厂——开源工业生产设计、仿真与云端执行平台

简称：超级工厂。



可编辑的工业方案彩排桌面平台。用户搭建流水线和厂房，配置物料、人员、工艺、物流与订单，用同一仿真内核计算真实事件，再查看物品经历、资源等待、方案差异和交付结果。



当前产品阶段为 **v2.0 开发与交付收尾**，本轮候选为 **2.0.1**，核心云闭环、差异验收和独立运行包验证已完成，当前整理代码仓库提交。v1已按正常使用范围验收并存档；v2新增真实云服务、独立软件设备、数据查询及回执驱动生产。当前包、实际结果和剩余动作取[v2三层控制计划](docs/06_v2.0设计与开发控制/开发控制计划.json)，不以旧21包压力矩阵或累计测试数推算当前完成度。最终候选、材料、公开发布和人工认可分别记录。



纸包装92/92种通过：83种核心设备及9种输送搬运配套均已接收。最后4种六轴、SCARA、龙门夹爪及真空托盘槽位取放通过20工况/24作业、20帧实际Native和13份当前GLB保存新进程重开；同一箱件身份、数量、属性和质量保持。普通Native首启实际登记92定义/184媒体。全目录92/500通过、408待审。 [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)。纸包装目标范围完成后停止，不代表整个项目或500种候选全部完成。



原创代码沿用已发布v2.0.0的[MIT许可证](LICENSE)，第三方依赖和素材按[NOTICE](NOTICE)保留原许可。

当前支持本地1.7.1纸包装整厂和四设备双工序云示范；93端点整厂云扩展延期未通过。见[当前版本快速开始](docs/05_开源与参赛/release-preparation/v2/当前版本快速开始.md)。

## 工作方式



- 中央大画布呈现流水线或多厂房，左右停靠面板可折叠和调整；时间轴按组或设备细化，共用实际Run时钟。

- 物品、设备、资源、任务、工艺、物流、厂房组合、专业计算和表现素材使用九类模块定义。采用模板不自动增加库存、人员或旧认可。

- 人工和Agent共用Commands与Host。编辑产生新修订，已有Run保留冻结输入；观看动画和回放不推进第二套生产时钟。

- 评估报告从同一对象生成PDF、离线HTML、CSV和JSON；云执行报告与原方案报告各保留自己的固定来源和表结构。缺价格、未完成结果与未知状态保持明确缺值。

- 纸包装提供受限结构计算、三态结果和制造文件；专业结果、人工认可与工位绑定分别处理。

- 云端生产在同一面板提供设备、数据、执行三个页签；四个独立软件设备经真实HTTPS参与双工序并返回执行结果，只有Host/Engine提交业务效果。完整筛选数据导出不受界面首屏条数限制。



## 开发环境与启动



首版目标为Windows11 x64，需要MSVC Windows构建工具及WebView2。Rust版本按根Cargo.toml，Node/npm按锁文件和工程准备记录；最低运行配置尚未完成实测。



在本目录执行：



```powershell

npm ci --ignore-scripts

npm run typecheck

npm run start

```



`start`使用Tauri开发入口及127.0.0.1:8922。启动前核对现有进程与端口，避免重复实例；开发服务不作为离线发行证据。



构建内嵌前端的Native开发候选：



```powershell

npm run build --workspace @superfactory/planner

```



该命令实际调用`tauri build --debug --no-bundle --features custom-protocol -- --locked`，由beforeBuildCommand构建网页资源，主程序名为`superfactory.exe`。当前不是安装器；主程序与tauri.conf.json声明的资源需要按实际目录闭包交付，不能只复制exe就宣称可离线运行。最终候选及公开仓库版本尚在核对。



基础检查：



```powershell

npm run typecheck

npm test

cargo test -p sf-simulation --tests --locked

```



具体Host、UI、专业计算和端到端检查取各包回执中的实际输入与命令。部分开发期集成测试消费本机冻结工程，应先按交接取得对应输入；它们尚不构成无附加输入的公开测试套件。本轮主线14项编辑器检查、6项Host检查和完整planner类型检查已通过，不以这些局部检查代替全产品验收。



源码交付方法见 [源码交付与复现](docs/05_开源与参赛/release-preparation/源码交付与复现.md) ；下面保留 2026-10-22 r3 历史打包命令，仅作调用示例，不代表当前主线或已接收 C12 包的身份。新制包应使用新的候选标识与输出路径，避免覆盖旧产物：



```powershell

python -B -X utf8 tools/release-preparation/source_delivery.py --root . --candidate-id superfactory-0.1.0-source-20261022-r3 --output .work/source-delivery/superfactory-0.1.0-source-20261022-r3.zip --extract-to .work/source-delivery/clean-source

```



制包包含未跟踪源码和必需 Native 资源，并核对解包字节与资源闭包；具体排除结果以实际制包清单和校验为准，不能由脚本意图推断所有历史包无缓存。恢复依赖需现有缓存或网络。[三项材料清单](docs/05_开源与参赛/release-preparation/材料版本对应.json)保留历史版本记录，当前主线的最终材料对应仍须按实际接收更新。



## 已接收的 C12 材料（独立历史候选）



以下仅记录固定 C12 的交付事实，不将当前主线候选身份改为 C12：Root 已完成 C12 Windows 目录包的普通 Native 启动、两份随包原 Run 回看、正常退出及 25 文件保留核对；C12 R3 十页作品介绍 PDF 与可编辑源已接收；49 秒真实流程视频已在本地完成。包含 C12 的最终仓库 commit 尚未发布，公开视频 URL 尚未提供，本轮正式上传或提交未办理，最终三材料对应仍待完成。



已接收的源码打包修正版为 `superfactory-source-C12-cachefix.zip`，须与 `superfactory-source-C12-cachefix.manifest.json` 一同交付。修正版精确移除 160 项已知 npm/WebView2 缓存并替换制包脚本，其余 29,359 个 ZIP entry 解压字节不变。ZIP 内 `SOURCE_DELIVERY.json` 保留原字节作为历史清单，修正版文件列表、字节数、排除项和候选身份以同名外部 manifest 为准；原 C12 ZIP 保留，但不能称为无缓存包。



C12 源码声明 Node 24.15.0 / npm 12.0.1；其实际检查环境为 Node 24.19.0 / npm 11.12.1，Rust/Cargo 为 1.97.0。声明和实测分别保留，冷缓存依赖恢复未验证；源码不提供完整 npm/Cargo 离线依赖镜像。这些限定结果不代替当前主线或全产品验收。





## 目录



|目录|职责|

|---|---|

|apps/planner|Tauri/React工作台、两类画布、时间轴、表单及交付界面|

|apps/mcp-bridge|独立stdio桥与同一实际Host的调用边界|

|apps/cloud-service|真实云HTTP服务、PostgreSQL持久化和DaaS查询|

|resources/v2-devices、resources/v2-demo|可信设备运行时、脚本及原两设备演示工程|

|packages/domain、commands|共同工程类型、操作、权限、修订、历史和持久化|

|packages/simulation|物料、作业、阶段、资源、物流和质量状态计算|

|packages/module-kit、presentation|九类采用、版本与表现状态消费者|

|packages/packaging、reporting|纸包装专业计算及同源报告输出|

|modules、scenarios|实际模块、场景、冻结输入与独立预期|

|docs、community、tests、tools|设计和执行控制、贡献规则、测试与本地工具|



## 当前入口



- v2：[可读开发控制](docs/06_v2.0设计与开发控制/开发控制设计.md) → [原V2-00至11](docs/06_v2.0设计与开发控制/开发控制计划.json) → [执行参考索引](docs/04_开发与验收/执行参考索引.md)。实际接点和回执按当前包消费；已过未改结果复用。

- [v2运行与演示说明](docs/05_开源与参赛/release-preparation/v2/运行与演示说明.md)：原r4示例、正常生产/停止接单改派、全量导出、只读回看及交付工具。

- v1历史：[逐包收口计划](docs/04_开发与验收/逐包收口计划.md)与[完整任务包](docs/04_开发与验收/整包派发)保留原任务和失败记录，不恢复已取消的正式压力前置。

- [v1实施主线记录](docs/04_开发与验收/本窗口实施主线.md)保留历史接收；当前v2统筹与写边界取v2控制计划，不据旧窗口号恢复派发。

- [需求与验收](docs/00_项目基线/首版需求与验收.md)、[类型与操作契约](docs/03_架构与契约/首版类型格式与操作契约.md)：功能、单位、权限及数据边界。

- [运行与交付指南](docs/05_开源与参赛/release-preparation/运行与交付指南.md)、[历史作品介绍候选](docs/05_开源与参赛/release-preparation/output/pdf/超级工厂_作品介绍_开发候选.pdf)、[真实录屏准备](docs/05_开源与参赛/release-preparation/真实录屏与接手.md)：材料与最终版本对齐。

- [贡献规则](community/CONTRIBUTING.md)、[九类实际入口](docs/05_开源与参赛/release-preparation/九类贡献实际入口.md)：类别样例、版本、来源及显式采用。



## 交付与许可状态



官网三项提交物为完整可运行代码仓库链接、作品介绍PDF、真实核心功能与流程视频链接。第二机、指定真实客户端授权、第三人完整验收及逐素材确认不再作为材料齐备前置；原技术任务继续收口，既有权限和数据完整性边界保留。详见[当前交付口径](docs/04_开发与验收/官方提交物交付口径.md)。



原创代码目前仍为MIT候选，根标准LICENSE尚未落定；保留已有第三方许可、NOTICE和素材来源，不把未知声明改成已授权。研究下载原文、机器配置、临时输出和私人数据不随产品发布。正式公开发布、报名、发送及人工认可均按实际授权和结果单列，不由开发候选冒充。
