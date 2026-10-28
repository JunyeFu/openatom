# S06-02／03隔离类型、格式与API契约试做

公共字段仅定义在[types.design.rs](../../packages/domain/types.design.rs)。本目录无Tauri／React产品入口，不构建前端，不执行18项产品API。Cargo锁和npm锁仅锁本试做；Python库／node_modules／原生target不提交产品。

执行顺序：

1. 在软件工程根执行`cargo run --locked --offline --manifest-path tools/type-contract-proof/Cargo.toml --target-dir tools/architecture-proof/target`，编译实际Rust类型，派生TS／Schema并执行20份类型样例往返。
2. 执行`python tools/type-contract-proof/check_schemas.py`，按Draft2020-12验证13领域Schema及额外API根Schema、20份样例、约束反例与本地引用；这一步不调用TS编译器。
3. 执行`python tools/type-contract-proof/check_persistence.py`，对自建样例实际文件／Windows ReplaceFileW／ZIP重开与失败保留做12例验证。
4. 执行`python tools/检查类型与格式.py`，核对61分类字段／10公共字段、全部公共类型的TS覆盖、S03-04测量记录绑定，并用TypeScript7.0.2实际检查派生声明。

Python标准Schema验证使用本目录隔离`python-libs`；精确依赖在[validator-requirements.lock.txt](validator-requirements.lock.txt)。TypeScript为同目录private包，安装禁用hooks并锁精确版本；不修改全局或产品依赖。已取得的输出见[类型试做](output/type-proof-result.json)、[Schema](output/schema-validation-result.json)、[TS编译](output/typescript-check-result.json)、[保存试做](output/persistence-proof-result.json)。

类型样例的结果标记type_fixture／未完成，不是生产或专业Pass。保存试做只证明选定的格式和原语，不能代实际软件打开／修改／接手、NFR性能或用户走查。ts-rs不识别部分Serde属性的告警保留；TS声明不执行Rust数值／路径约束，Schema与入口业务检查职责分开。

S06-03：`python tools/检查公开操作契约.py`核对18服务签名、36操作Schema／真实Rust往返、28冻结请求形状（Native与标准Schema一致）、26有限授权模型和5实际字节帧案例。有限模型信任预置输入，不是Windows真实认证或产品API执行；所有处理器仍未实现。API字段仅在packages/commands/api.design.rs，不复制领域字段。

H64-01增加[六项独立公共输入](fixed-pose-cases.design.json)、原生Job/compute/PutPackagingJob的18项实际形状与往返，[标准Schema与静态矩形参考](check_fixed_pose.py)以及[专业表单/Agent DTO消费者](fixed-pose-consumer.ts)。执行`python tools/type-contract-proof/check_fixed_pose.py`；再由原TS命令实际编译消费者。有限参考只覆盖冻结的静止矩形、明确目标、保持/位姿不符、碰撞和初态生效连接，不是完整连续专业内核。原20领域/36API样例和纸包装有限几何预期不改。

TC01/TC02原13个独立正反例保持，新增TCF14—TCF20端点正反例，共20例，见[timeline合同输入](timeline-contract-cases.design.json)。Rust实际解析/往返/有限消费者输出见[结果](output/timeline-contract-proof-result.json)，[标准Schema与FullResult一致性](check_timeline_contract.py)使用同一Rust派生根；[TS消费者](timeline-consumer.ts)实际参加原TypeScript检查。执行原cargo命令后运行`python tools/type-contract-proof/check_timeline_contract.py`。工作与等待区间、执行设备/容量、blocked历史阶段及计算截断分别核对；仍不实现真实Engine/Host或前端。

## AQ-01 大结果传输补充

原cargo命令重建公共类型/36操作Schema后，执行`python -B -X utf8 tools/type-contract-proof/check_managed_result.py`（使用前述3.12 runtime/python-libs）。原生AQ输入为3000物品/3000事件，文件包含完整Response<RunView>。脚本校验当前get_run操作及IPC泛型Schema，并在Windows子进程实际复制只读句柄、读回字节、测试短读/取消/断连/解包失败清理；结果为output/aq01-native-result.json和aq01-transport-result.json。API/IPC版本不变，不实现产品Host或Codex客户端。
