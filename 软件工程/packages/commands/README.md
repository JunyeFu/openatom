# 共同Rust操作层

本目录已有正式API类型、18个具体服务签名及单工程Host。当前13项操作在明确范围实现并复审，覆盖共同草稿/定义、修订/授权/原子提交、会话历史、目录保存重开、已加载库查询、唯一Engine运行/历史和真实专业计算/认可/采用。库存和时钟只属于同一simulation Engine；能力发现仍按真实会话、Scope、当前状态与提供者路径过滤。

[类型与操作](../../docs/03_架构与契约/首版类型格式与操作契约.md) · [Agent链路](../../docs/03_架构与契约/通用Agent接入与专业软件连接器.md) · [设计命令目录](command-catalog.design.json)。实际能力只在实现后注册，不从planned目录自动推导可用性。

## 现行目标生成与开放主线

SF-17使用库查询、共同编辑、草案检查、渲染、模拟/比较和评估导出共18项计划操作；登记设计不等于可调用实现。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

编码依据：[公开契约](../../docs/03_架构与契约/公开操作与宿主调用契约.md)、[唯一API源](api.design.rs)；领域字段消费domain，TS/Schema由同源派生。Workspace及Project／定义草稿的身份／修订分开，TaskScope文件任务不预造工程；IO完成结果不等于初次TaskHandle。

AQ-01：[大结果传输设计](../../docs/03_架构与契约/大结果传输设计.md)在共同传输层增加ApiOutcome.Managed及受管JSON只读附件；业务签名不变，调用方统一恢复原Response<R>。实际隔离原生/Schema/Windows句柄证据见tools/type-contract-proof/output/aq01-transport-result.json。

## S08正式类型边界

[src/api.rs](src/api.rs)公开冻结的有类型请求/响应、15分支EditCommand及18方法CommandsService声明，消费正式sf-domain；[project_host.rs](src/project_host.rs)实现本轮最小Host。`npm run generate:contracts`从正式Rust生成[TS请求入口](generated/typescript/ApiRequest.ts)及[工程Schema](generated/schemas/project.schema.json)。原步骤1的17组Rust测试与32个Schema案例保留为[类型切片报告](../../docs/04_开发与验收/P0-01_工程类型与Command边界切片.md)，当前真实Host范围见[Host切片报告](../../docs/04_开发与验收/P0-01_Host最小创建读取编辑切片.md)。

当前[限定集成切片](../../docs/04_开发与验收/S08_共同Host与Engine专业联通切片.md)已独立接收：单次348项Rust、40份Schema/71案例、TypeScript、桌面构建与同一managed Host143项检查通过，旧216项Rust/104项原生名称保留。十包执行、十一包待启动；原业务UI、真实外部MCP、渲染/比较/导出、完整库/模板采用、订单物流联合与IX/NFR/release按原包继续。模板采用记录/来源F06正在下一共享批次修复，不在本片接收范围。

当前P0-02原步骤1接收范围、接口及剩余项见docs/04_开发与验收/P0-02_业务输入与引用切片.json。
