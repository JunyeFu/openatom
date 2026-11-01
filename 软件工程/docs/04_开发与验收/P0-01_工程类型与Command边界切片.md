# P0-01共同工程类型与Command边界切片

2026-10-15。按用户本轮明确派发，由当前W00聊天接续首个基础类型切片实施；没有启动其他窗口。[机器记录](P0-01_工程类型与Command边界切片.json)保存本轮范围、实际命令和复审结果，[首次切片](P0-01_首个后端切片.json)保留历史。

**原步骤1“共同字段/schema与命令边界、同源前端类型”已完成。P0-01整项保持`in_progress`。** 这次交付的是正式产品库中的有类型边界；实际Host创建、读取、编辑处理器属于原步骤2。

完整`ProjectState`、`ScenarioInput`、九类`ModulePackage`及其传递数据类型提升至[sf-domain](../../packages/domain/src/models.rs)，复用首片的20个基础类型。工程、目标、场地、单位、模块精确版本和对象引用沿用冻结字段，没有复制库存或生产时钟。

[sf-commands](../../packages/commands/src/api.rs)公开冻结的Request/Response、15分支`EditCommand`、18操作及`CommandsService`方法声明，包含创建工程、对象编辑、工程设置、Scope、修订、结构化错误和历史可用性结构。`HostCallContext`、授权grant及ResolvedCallScope仍是只供宿主注入的Rust类型，不进入报文、TS或Schema。类型声明不提供实际处理器或操作注册。

`npm run generate:contracts`执行[正式生成器](../../packages/commands/src/bin/export-contracts.rs)，产出423份[TS声明入口](../../packages/commands/generated/typescript/ApiRequest.ts)和9份Schema，包括[工程](../../packages/commands/generated/schemas/project.schema.json)与[编辑请求](../../packages/commands/generated/schemas/apply_commands.request.schema.json)。字段由实际Rust派生，业务前端尚未接入。

| 必要验证 | 实际结果 |
| --- | --- |
| `cargo test -p sf-domain -p sf-commands --locked` | 17组通过，新增8组，原9组保留；无失败 |
| `npm run test:rust` | Rust整个工作区编译接入通过，同样17组测试通过，不重复计数 |
| 正式类型/报文消费 | 完整工程、方案、九类模块往返无损；创建/读取/编辑请求、响应、15个Command分支Schema与修订冲突响应结构通过 |
| 真实TS编译消费 | 生成声明、创建/读取/编辑正例及缺失工程ID、错修订、残缺工程、伪造授权等负例通过 |
| `python -B -X utf8 tools/检查工程与Command边界.py` | 使用既有隔离jsonschema依赖；9份实际Schema、32个正反例通过 |
| `npm run typecheck`、`cargo fmt -p sf-domain -p sf-commands -- --check` | 通过 |

实施后由本窗口只读复审：提升后的模型及API与冻结源逐体比较一致，确认缺陷0。此次为本窗自审，不声称独立外部复审。ts-rs对原有serde转换属性仍有工具警告，实际Rust JSON、Schema及TS消费者已通过，没有隐藏该诊断。

剩余工作是原步骤2的真实Host最小创建/读取/编辑及Scope、修订冲突、授权、原子提交行为，原步骤3的保存及异目录重开，以及原步骤4的会话撤销/重做。请求解析测试不证明这些行为已实现。18个业务操作处理器可用数仍为0；S08-01/05继续部分完成、pending，S08未退出，P0-01未标done。
