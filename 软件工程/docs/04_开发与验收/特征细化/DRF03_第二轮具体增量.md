# DRF03-C01—C04 第二轮具体增量

本文件保留第二轮当时的证据快照；当前C02生成/C04内层进展及剩余以[第三轮增量](DRF03_第三轮具体增量.md)为准。原预期与输出未反写。
2026-10-14。按[current_design_reassignment](../本机并行窗口计划.json)执行：公共Rust/API/Schema/TS及进度唯一写入为复审窗口1，S04与交互源由窗口2维护。本窗口只改原授权专业/模块源、适配场景和DRF03材料，不提升通过数，不启动产品编码。

## 已完成的拥有者定义与实际试做

|原请求|本轮落地内容|证据与现状|
|---|---|---|
|C01 九类采用/升级|[九类精确效果](DRF03_C01_逐类原子采用.json)，区分Project.module_refs登记与先前独立库提交，明确全部校核后一次提交|A升级/B及旧Run不变；单位错/修订冲突全批回退；三份现有API请求形状通过|
|C02 专业绑定|[十步完整推导](DRF03_C02_专业绑定推导.json)及[现有类型的参数片段](DRF03_C02_消费者参数片段.json)，不再只有“比版本/范围”的原则|当前工位独立选择Job，不能由artifact反推期望；完整制造要求/可信人认可/同产品换材料均有反例|
|C03 过程身份|[Rust最小片段](DRF03_C03_过程样本Rust片段.rs)、[Schema选定片段](DRF03_C03_过程样本Schema片段.json)、[八条语义](DRF03_C03_过程样本语义.json)|step/progress/panel_id-Pose明确；重排不变，未知步骤/缺板/重板/错误单位/旧NamedPose拒绝；尚未冒充公共已合入|
|C04 旧包适配|[实际适配脚本](../../../scenarios/九类模块采用/DRF03_旧清单适配试做.py)、[61项转换规则](../../../scenarios/九类模块采用/DRF03_适配字段规则.json)、[实际结果](../../../scenarios/九类模块采用/DRF03_适配试做结果.json)|九外层候选Schema通过；八类有定义的载荷转换；模板内层保持阻塞，原12库位/18连接完整保留|

本轮[29项独立预期](../../../scenarios/九类模块采用/DRF03_第二轮独立预期.json)均与[隔离输出](../../../scenarios/九类模块采用/DRF03_第二轮核验结果.json)匹配。实际读取旧manifest和原生素材，未用正式夹具替换旧载荷；原15采用、第一轮22项和S06-04原数值/结论不变。有限参考和请求形状不是实际Host/Engine执行。

## C01：Task和Transport不再停在逻辑实例名

任务沿`ProductionTask.routing_ref → RouteStep.operation_ref → ModulePayload::Tasks`，按`device_id`和显式资源/目标绑定编译为共同准备Job/Phase，工时30000ms来自实际旧任务。准备完成仅为该目标/选定配置提供marker，不生产物料；已有marker满足同配置的重复触发，不伪造第二份completed Job。排他设备与人员仍走共同claim/日历，维修/建设仍后置。

运输沿`ScenarioInput.supplies[].transport_ref/resources/source/destination → TransportDefinition`。采用/升级修改整个Scenario副本，再以原PutObject(Scenario)提交，禁止构造不存在的PutObject(Supply)变体。原10/30/10/30秒意味着无额外等待时50000ms接受、80000ms返程释放，同货物C1身份保持。

只有Project/Scenario副本上的全部校核通过才交换修订；失败不留下Project.module_refs或一半目标。已经在另一独立事务成功提交到库的版本不回滚，不混合Definition Scope和Project Scope。

## C02：当前期望不取自待验证成果

完整纸包装名义计算绑定先从Device.configuration的两个已声明TypedValue键`packaging_job_id`、`packaging_job_revision`解析不可变的期望Job；缺选择明确缺项，不取最新Job或artifact自己的Job代替。

由该Job的subject_ref及当前Project产品/路线验证目标工位和精确pack工艺，推导expected_subject_revision。完整profile固定要求形成、开合、直线取物、净空间、制造及可编辑Job；requested_scope不能少勾制造降级。材料/厚度/姿态/过程/容差/预算全部来自选定冻结Job；artifact/result的job_ref、输入文件与此一致，才能继续验证真实内容、全部必需Pass与既有可信人工用途认可。

两个键沿现有参数/配置类型，不增加第19操作；其声明及公共消费者仍需窗口1合入。declared_process保留原显式声明分支，不冒充真实纸包装；software_in_loop仍后置。旧Run继续冻结其旧Job选择。

## C03：可以直接合入的最小改变

`PackagingResult.process_poses`元素改用`PackagingProcessSample { step_id, progress, panel_poses }`；每个`PackagingPanelPose { panel_id, pose }`明确板片到Job世界的完整mm/Z-up单位姿态。父result.job_ref关联唯一冻结Job，不在每个样本复制第二份输入。

progress是步骤参数u，不是生产毫秒；样本按ID解析、覆盖该Job全部板片且不重复。有限样本只解释实际输出，不证明样本间碰撞安全；三态仍由原区间方法给出。NamedPose保留为表现命名姿态，旧节点索引不能猜专业步骤/板片。公共派生Schema/TS、生产者、get_task/artifact、预览、Agent及reporting合入后需复核。

## C04：真实转换揭示的模板内层阻塞

当前试做正确转换了真实旧数据中的秒→整数ms、数量DecimalAmount、端口方向/角色、准备作用、工艺组成身份、原生box/cylinder节点和姿态；原参数ID、配比、时长与业务结论保留。未确认作者/许可保留null及redistribution阻塞，机器轴向/字段类型采用明确试做选项并标未确认；没有注册available_modes或创建人认可。

模板不以删内容换Schema通过。其外层候选保留原project.design相对引用并带adoption/execution阻塞；内层尚非TemplateSubset。实际差异详[精确交接](DRF03_C04_适配试做与内层模板交接.json)：

- 源厂房高度为null，HallDesign.height_m却要求有限数值；不能填写假高度。未知尺寸须保留可编辑缺项，场地通过仍要求实际值。
- 原12库位需要完整保存；已有InstantiateTemplate.proposed_objects支持Location，但独立TemplateSubset文件没有locations，须明确同包typed补充或统一补字段。
- 原18连接含库位→设备端口；MaterialConnection只有PortRef，库位没有module_ref。公共端点需明确port/store或已有真实库位端口契约，不能捏造模块引用。
- 原footprint/clearance、准备/专业声明及新方案seed所需引用不齐继续保留ImportDraft缺项，不自动生成BOM/Route/认可来声称可执行。

这些是实际适配发现的共同格式问题，窗口1统一收口。本窗口已把可独立完成的转换和具体失败材料交出，尚未将模板内层标已转换或实际导入成功。

## 已合入项与仍待项

TC01/TC02、RunResult.computed_until_ms及Events末刻语义已经合入，消费者只读[timeline_runtime_contract](../../03_架构与契约/全层类型与工程格式.json)，不再算旧pending。71字段表单映射已经接入，由窗口2维护，不重复修。新wait/claim结束原因在公共实际合入前仍只记本轮选定设计，别名转换不改原时间数值/结论。

窗口1现已合入C01逐类原子效果、C02完整推导及C03公共DTO，本窗口已读取实际生成Schema复核29项预期及正式结果正反例；408份TS声明检查通过的记录只证明当前DTO/消费者。详[公共合入消费者复核](DRF03_公共合入消费者复核.json)。实际Host/Engine、产品原型、本人走查和公开通过数未由本轮代填。

## C02追补：两个键已经落成实际定义

[包装工位定义](../../../packages/packaging/consumer-device.design.json)是按当前ModulePackage Schema核验的`community.parameterized-pack@0.2.0`未注册设计候选，实际声明了String类型packaging_job_id和Integer类型packaging_job_revision，分别绑定`/payload/data/configuration/packaging_job_id/value`、`/payload/data/configuration/packaging_job_revision/value`。原product_item和15秒包装规则保留；旧0.1.0清单、21登记和冻结Run未升级。

未选时configuration为空，两个值都不默认填入；用户明确选择后，现有PutObject(Device)/采用事务创建已声明的TypedValue覆盖。缺声明时，即使客户端塞同名值也拒绝。默认叶缺失表示可编辑草稿，不授任意指针写权限。

[实际声明与生成接入](DRF03_C02_实际工位定义与生成接入.json)列出真实现存生成源`modules/templates/生成三类模板设计.py`的pack生成块/Packing01装配处，以及窗口1的`tools/type-contract-proof/src/main.rs`和原任务包引用投影。生成器读取新定义必须是显式calculated_nominal分支；原declared_process模板不种入Job，也不覆盖旧版本。当前本窗口没有写这些共享源。

[七项声明正反核验](../../../scenarios/九类模块采用/DRF03_C02_声明核验结果.json)验证实际定义与显式选择请求的Schema、缺声明、重复声明、错误类型/指针和未选择。示例选择LOCAL_CONTACT_JOB只是原有限接触子例，不能满足完整纸包装profile，不是合格绑定或实际人选择记录。C02不再只有“规范说应声明”，生成端实际接入仍按上述精确位置交窗口1。
