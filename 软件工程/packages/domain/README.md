# packages/domain

物品、固定设备、资源、工艺、厂房与供应线共同定义。P0实现。

已补[工程模型与彩排状态](../../docs/03_架构与契约/工程模型与彩排状态.md)：库位现有/可用量、预留、在制、在途、装配组成与包装内容、方案和结果各有身份。参考完整两厂房声明样例，不能仅由画面布局推断库存或人员。

S03-02业务规则以[business-objects.design.json](business-objects.design.json)及[订单/用料正文](../../docs/01_业务需求/订单工艺与物料分配.md)为源：预留只属分配、订单接受与物理接收分开、业务取消与计算取消分别处理。[异常出口](../../docs/01_业务需求/换单班次与质量处置.md#s03-02-异常与恢复出口)规定缺项/等待/返工/取消/失败后的保留和恢复；这是业务设计，不是运行schema。

## 现行目标生成与开放主线

保存目标、场地包络、参数来源、模板生成上下文；目标与RunResult分开。L/W/H映射width_m/depth_m/height_m，产品包装以mm；目标不得绑定接受量。

[主线与范围](../../docs/00_项目基线/项目主线.json) · [完整生成链](../../docs/03_架构与契约/目标驱动厂房生成.md)。

S02-01按[冻结范围](../../docs/00_项目基线/首版交付基线.md)落实：P0-01共同格式、P0-02业务/独立归属，P0-03只含专业成果/认可/绑定/结果，建设投用/公用服务字段与求解不进入本版。

## S06-02具体类型与格式

唯一字段源[types.design.rs](types.design.rs)，[完整规范](../../docs/03_架构与契约/全层类型与工程格式.md)与[JSON](../../docs/03_架构与契约/全层类型与工程格式.json)规定同源TS／Schema、精确数量／单位／坐标／版本、九类61字段及保存清单。根project.json是PackageManifest，实际ProjectState与冻结Run输入按角色另列。原design稿需明确适配，不冒充运行格式；18操作与专业算法按S06-03/04冻结，其运行实现仍归对应后续切片。

H64-01：FixedPose使用canonical `FixedPoseMotion { part_id, pose }`；H64-02坐标/接触/连接域和F19-05/Agent消费规则见[类型规范](../../docs/03_架构与契约/全层类型与工程格式.json)。[独立公共契约输入](../../tools/type-contract-proof/fixed-pose-cases.design.json)与实际派生/类型核验不等于专业算法或产品通过。

## S08首个后端切片

运行中的基础值类型在src/value_types.rs与src/ids.rs，由src/lib.rs公开；20个类型对应冻结设计中的7个数值/路径/版本类型与13个不透明ID。Rust测试同时导出generated/value-types.ts并用真实TypeScript消费者验证。运行`cargo test -p sf-domain --locked`。完整ProjectState、九类Module、Scenario及传递数据类型现已提升至[src/models.rs](src/models.rs)，与已实现基础类型共用。正式Commands边界见sf-commands；实际Host、目录持久化和会话历史已接入sf-commands。详见[类型切片](../../docs/04_开发与验收/P0-01_工程类型与Command边界切片.md)。

当前[限定集成切片](../../docs/04_开发与验收/S08_共同Host与Engine专业联通切片.md)已独立接收：单次348项Rust、40份Schema/71案例与同一managed Host143项检查通过；共同类型由唯一Engine、真实专业链和保存重开消费者实际使用。13项Host操作只在该片具体范围可用，十包执行、十一包待启动；完整UI/外部MCP、库/模板、订单物流联合及IX/NFR/release继续按原包接收。

当前P0-02原步骤1接收范围、接口及剩余项见docs/04_开发与验收/P0-02_业务输入与引用切片.json。
