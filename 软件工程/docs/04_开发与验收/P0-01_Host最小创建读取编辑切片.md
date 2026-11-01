# P0-01原步骤2：实际Host最小创建、读取和编辑

2026-10-15。按用户明确授权，由本聊天统筹，现有实现窗口2—6分别完成适配、独立测试及只读复审。[机器记录](P0-01_Host最小创建读取编辑切片.json)保存具体派发、文件边界、实际命令、原始结果和复审状态；其他P任务没有因此启动。

**原步骤2已实现、验证及独立复审通过；三项P2全部关闭，12项跨文件同步检查通过。P0-01整项仍为`in_progress`。**

实际代码：[单活动工程Host](../../packages/commands/src/project_host.rs)、[Tauri边界](../../apps/planner/src-tauri/src/project_api.rs)、[桌面入口](../../apps/planner/src-tauri/src/lib.rs)。Host保存工程和已登记会话；界面请求只能通过真实main窗口使用托管UI身份，报文不能注入caller、grant或替换会话。

| 操作 | 本片实际范围 |
| --- | --- |
| `describe_capabilities` | 返回实际四项能力；外部编辑能力按当前获准目标、叶字段、删除权或创建预算过滤，类型声明不自动变成能力 |
| `get_context` | Workspace空工程或当前获准Project的真实上下文、修订与dirty状态 |
| `get_object` | 当前Project或ItemType的Draft数据；未实现对象/RunSnapshot/DefinitionDraft明确拒绝 |
| `apply_commands` | 真实UI的Workspace创建；Project设置及ItemType增删改；其他命令明确不支持 |

创建生成真实工程ID及修订1；每个编辑批次只递增一次共享修订。Host先检查真实会话、目标Scope及操作权，再检查已知base_revision，完整核验候选影响的授权后才处理引用/支持范围；最后一次提交工程、修订及创建账本。任一失败均保留原状态。精确RFC6901叶字段不能以父对象、通配符或数字数组索引替代，删除不退还累计创建额度。

脏工程上的Stay和伪称AlreadySaved不会丢弃数据。明确Discard后新工程获得新身份，旧外部会话撤销，旧对象不会混入。没有虚构长任务、生产库存、运行时钟或可撤销历史。

| 必要验证 | 实际结果 |
| --- | --- |
| `npm run test:rust` | 49组通过：原17组类型边界、4组Host核心、独立6组Scope/修订、13组授权/原子提交、5组创建/切换、4组Tauri适配；无失败 |
| `npm run generate:contracts`及标准Schema消费者 | 正式Rust派生11份Schema，34个正反例通过；TS字段仍由Rust派生 |
| `npm run typecheck`、受影响Rust格式检查 | 通过 |
| 实际桌面构建及`superfactory.exe --p0-01-host-smoke` | 真实桌面进程中复用app.manage登记的同一Host；10项创建、读取、编辑、冲突、伪造身份及错误边界核验全部通过 |

可复跑入口为[verify.ps1](../../tools/p0-01-host/verify.ps1)，原始命令、退出码和日志见机器记录的actual_result.verification_ref。该原生验证直接消费托管Rust入口，没有声称业务WebView交互、外部MCP进程认证或用户整机验收。

复审中发现并修复了未解析DefinitionDraft错误上下文回填、UI未实现操作的错误码、空授权被能力目录标成可编辑，共享接线的Tauri AppHandle调用，以及未知API版本/未知操作的错误分类。最终复核还补齐已记账对象删除后的同ID恢复能力：恢复A不消耗第二个distinct-ID额度，新建B仍拒绝，账本不重置。已观测的授权容器替换和能力过滤问题有真实回归测试。独立窗口6的最终发现、关闭及实现接收状态以[独立复审记录](P0-01_Host切片_窗口6.json)为准；首次失败证据在各窗口JSON保留。

原步骤1/2已有实际交付；P0-01整项仍为`in_progress`。下一推荐范围是原步骤3：目录保存、异目录重开后修改及失败保留旧有效工程；原步骤4的会话撤销/重做仍未实现。四项操作仅在上述最小范围可用，不代表完整18项业务能力或S08阶段退出。


2026-10-16 后续接收：以上49/10项及步骤3/4未实现等描述是原步骤2当时范围；当前后续整包实现与证据取[整包集成验证](整包派发/整包集成验证.json)，不改写历史切片预期。本轮另据单独授权P2-01实际产品前端，在隔离Tauri WebView2/同一Native Host完成创建、读取、编辑、撤销重做及冲突保值4组GUI，并补验中文冲突r6/r7、表单22m保留/真实20m不被覆盖。Root适配纠正了准备脚本误读不存在的公开身份字段，原红记录与原独立源保留；该次Root执行尚待窗口5独立接收。实际OS文件选择器、完整Run/专业/IX/NFR及用户整包验收仍保留。


2026-10-16 后续独立真实桌面接收：W5在隔离实际desktop-host-67428上完成16项不同检查，包含本步骤的创建/读取/编辑、撤销重做与冲突保值，并覆盖真实模板显式采纳、同Host运行、时间轴输入原子提交、两份旧Run冻结与比较、冲突后显式刷新和本地停靠偏好。实际证据见[独立16项合并结果](../../.work/whole-packages/window5-timeline-evaluation/native-combined-result.json)及[共同接收](整包派发/整包集成验证.json)；不与旧49/10或其他批次重复累加。该进程已按精确路径关闭，端口释放；完整P0-01和S08退出仍取原release/IX/NFR/用户结果，未据本次GUI局部结果放行整包。
