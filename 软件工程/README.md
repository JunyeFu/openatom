# 超级工厂——开源工业生产设计、仿真与云端执行平台

简称：超级工厂。



可编辑的工业方案彩排桌面平台。用户搭建流水线和厂房，配置物料、人员、工艺、物流与订单，用同一仿真内核计算真实事件，再查看物品经历、资源等待、方案差异和交付结果。



当前正式版本为 **v2.0.1**，提供Windows x64运行包、纸包装示例工程和基础与进阶操作指南。新增云服务、独立软件设备、数据查询及回执驱动生产。[正式下载](https://github.com/JunyeFu/superfactory-releases/releases/tag/v2.0.1) · [操作指南](docs/05_开源与参赛/release-preparation/v2/软件操作指南.md)。开发任务和测试记录见[v2开发控制计划](docs/06_v2.0设计与开发控制/开发控制计划.json)。



纸包装设备库提供92种设备，包括83种核心设备和9种输送搬运配套，覆盖生产、检验、包装与物流环节。设备支持三维表现、作业状态和物品交接，可在同一厂房中组织产线并保存工程。



原创代码沿用已发布v2.0.0的[MIT许可证](LICENSE)，第三方依赖和素材按[NOTICE](NOTICE)保留原许可。

从纸包装92台设备模板开始搭建工厂，在全厂、流水线和物品跟随视图中查看生产与运送；通过云端页面查看设备状态、安排双工序作业和读取执行记录。见[当前版本快速开始](docs/05_开源与参赛/release-preparation/v2/当前版本快速开始.md)。

## 工作方式



- 中央大画布呈现流水线或多厂房，左右停靠面板可折叠和调整；时间轴按组或设备细化，共用实际Run时钟。

- 物品、设备、资源、任务、工艺、物流、厂房组合、专业计算和表现素材使用九类模块定义。采用模板不自动增加库存、人员或旧认可。

- 人工和Agent共用Commands与Host。编辑产生新修订，已有Run保留冻结输入；观看动画和回放不推进第二套生产时钟。

- 评估报告从同一对象生成PDF、离线HTML、CSV和JSON；云执行报告与原方案报告各保留自己的固定来源和表结构。缺价格、未完成结果与未知状态保持明确缺值。

- 纸包装提供受限结构计算、三态结果和制造文件；专业结果、人工认可与工位绑定分别处理。

- 云端生产在同一面板提供设备、数据、执行三个页签；四个独立软件设备经真实HTTPS参与双工序并返回执行结果，只有Host/Engine提交业务效果。完整筛选数据导出不受界面首屏条数限制。



## 开发环境与启动



开发环境为Windows 11 x64，需要MSVC Windows构建工具及WebView2。Rust版本按根Cargo.toml，Node/npm按锁文件和工程准备记录。



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



该命令调用`tauri build --debug --no-bundle --features custom-protocol -- --locked`，由beforeBuildCommand构建网页资源，主程序名为`superfactory.exe`。交付时，将主程序与tauri.conf.json声明的资源放在同一完整目录。



基础检查：



```powershell

npm run typecheck

npm test

cargo test -p sf-simulation --tests --locked

```



Host、UI、专业计算和端到端检查使用各包说明中的输入与命令。需要冻结工程的集成检查，应先准备对应工程和运行输入。



源码交付方法见[源码交付与复现](docs/05_开源与参赛/release-preparation/源码交付与复现.md)。以下为源码制包工具的调用示例；使用时填写本次版本标识和输出路径：



```powershell

python -B -X utf8 tools/release-preparation/source_delivery.py --root . --candidate-id superfactory-0.1.0-source-20261022-r3 --output .work/source-delivery/superfactory-0.1.0-source-20261022-r3.zip --extract-to .work/source-delivery/clean-source

```



制包工具收集源码与必需Native资源，生成文件清单并核对解包内容。恢复开发依赖需要网络或本机依赖缓存；版本与文件对应见生成的清单。正式用户运行包提供应用、模型、模板、操作指南及必要运行资源，内部测试记录和开发清单留在工作区。








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



正式下载入口提供Windows应用、纸包装示例工程和操作指南。作品介绍与流程演示围绕工厂搭建、生产安排、设备数据、运行回看和可编辑工程展开。



原创代码使用[MIT许可证](LICENSE)。第三方依赖、模型与字体按[NOTICE](NOTICE)及各资源许可文件使用。正式运行包提供应用、模型、模板与必要运行资源。
