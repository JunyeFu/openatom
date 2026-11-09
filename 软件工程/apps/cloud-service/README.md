# 超级工厂 v2 云服务

当前数据库迁移到 schema version 3：保留原表和数据，新增 `telemetry_query_cover` 部分覆盖索引，供固定数据集／设备／时间／前沿的原始查询和分钟聚合使用。`migrate` 在既有 advisory lock 与事务内升级空库、version 1 或 version 2；重复执行不重复建索引，未知版本拒绝。升级期间停止服务，先运行新版本 `migrate`，再启动同版本服务；健康检查要求完整版本序列 `[1,2,3]`。首次建立索引会扫描现存事件表并占用额外磁盘，不应把迁移耗时算入查询性能。游标、数据来源和聚合单位不变。

独立 Axum / PostgreSQL 组件，wire 为 `sf.execution/2.0-draft11`。使用 `sf_domain::cloud_execution` 的严格请求、响应和游标 DTO；不链接 simulation、commands 或 Native，不运行 Engine、不产生库存或 Host 接纳事实。

## 构建与启动

在软件工程根运行；本批保持独立 workspace 和 Cargo.lock，不修改根依赖。

```powershell
cargo build --locked --manifest-path apps/cloud-service/Cargo.toml --target-dir .work/v2-implementation-20261026/cloud/target
.work/v2-implementation-20261026/cloud/target/debug/sf-cloud-service.exe migrate
.work/v2-implementation-20261026/cloud/target/debug/sf-cloud-service.exe serve
```

先通过私有环境提供下列配置，凭据不得写入命令行、工程、日志或交付包。磁盘不足时可将 `--target-dir` 指向本批专用临时缓存；相应配置 `SF_CLOUD_TEST_EXE` 后仍可运行同一验收脚本。

|环境变量|要求|
|---|---|
|`DATABASE_URL`|PostgreSQL 连接串，私有环境提供|
|`SF_CLOUD_AUTH_TOKENS_JSON`|下述严格 principal 数组，私有环境提供|
|`SF_CLOUD_BINDINGS_FILE`|非秘密固定映射的绝对路径；部署路径建议 `/opt/superfactory/config/demo-bindings.json`|
|`PORT`|HTTP 端口，默认 23270|
|`SF_CLOUD_LISTEN_ADDRESS`|默认 `0.0.0.0`；本机验收必须显式 `127.0.0.1`|
|`SF_CLOUD_LOCAL_NO_TLS`|只有显式值 `1` 且数据库全部 host 为回环地址时才允许 NoTls|

远端数据库始终 `sslmode=Require`，使用系统信任库、证书和 hostname 校验、SNI，最低 TLS1.2；连接失败不会退回 NoTls。Linux 处理 SIGTERM / Ctrl+C，Windows 处理 Ctrl+C；停止接新请求、ready 置 false，最多等待 20 秒后关闭连接。未收到响应的原请求应查询或按原身份重发。

`migrate` 在事务内取得 advisory lock `53023001`。空库创建 15 张业务表、约束、索引、固定项目／设备／两个非执行数据集，并同事务登记 schema version1。version1 重跑核对原固定映射，不覆盖数据；未知版本拒绝。表及字段见 `migrations/001.sql`，`schema_migrations` 为第16表。

## 固定映射和身份

固定映射结构为 `projects`、`device_bindings`、`datasets` 三个数组，严格拒绝未知配置字段。项目包含 `cloud_project_id/local_project_id/owner_scope_ref`；设备绑定包含 `binding_id/cloud_project_id/device_id/principal_scope_ref/definition_ref/capability_map`。`definition_ref` 为精确 ModuleRef；`capability_map` 包含 `action=process_unit`、`operation_ref`、`execution_capacity_ref`。设备、精确模块、操作和能力必须与 Host 冻结候选一致。

两个固定非执行 dataset 分别为 `telemetry_only`（`software_in_loop` 或 `physical_device`）和 `generated_load`。execution dataset 在创建 session 时生成 `execution-{session_id}`，其 source_mode 取该云项目固定 telemetry_only 映射；该项目须有明确映射。执行数据集依 session 归属授权，两个非执行数据集使用显式 dataset scope。

从实际 Host 冻结输入生成映射，保留真实 v1 工程／设备／定义 ID：

```powershell
python tools/cloud/make_bindings.py <frozen-inputs.json> <demo-bindings.json> --cloud-project-id <cloud-id> --owner-scope-ref <gateway-principal-id> --telemetry-dataset-id <telemetry-id> --load-dataset-id <load-id> --source-mode software_in_loop
```

脚本逐候选核对冻结工程设备、精确模块版本及操作引用。每个设备凭据 principal_id 使用生成的 `device:{实际设备ID}`，与 `principal_scope_ref` 一致。实际部署冻结输入及私有凭据由统筹集成，本批公共 `example` 工程仅用于组件验收。

整厂的物理设备仍各占一条 `device_bindings` 和一个有效boot；多个加工、换型或运输阶段放在 `capability_map.stage_bindings`，逐条保存完整CandidateBinding。指令选择冻结阶段，poll/收据序列仍归同一物理端点。单工位原映射继续可读。

完整模型的冻结输入约96 MB，C01支持gzip上传；认证后的会话/快照上传及解压内容上限为128 MiB，其余请求保持16 MiB。冻结输入在原 `execution_inputs` 行内使用gzip/base64封装，C16和原身份重试先还原精确内容；旧JSON行继续可读。高频会话行仅保存请求头部字段，避免每次设备轮询读取整套模型。不改变公共wire或数据库表结构。针对性实库检查：`tools/cloud/factory_acceptance.py <新的证据目录>`，沿用已有私有本机数据库配置。

`SF_CLOUD_AUTH_TOKENS_JSON` 是 JSON 数组，每项必须且仅可包含：

```json
{"token":"<私有配置注入>","principal_id":"<固定主体>","role":"host_gateway","project_ids":["<云项目ID>"],"device_ids":["<实际设备ID>"],"dataset_ids":["<非执行数据集ID>"]}
```

role 仅可为 `host_gateway/scoped_reader/operator/device/generated_load_producer`。token 和 principal_id 各自唯一；device 主体精确绑定一个已配置设备，generated_load_producer 精确绑定一个 generated_load 数据集。没有通配 scope，也不接收 caller、grant 或请求注入的权限。所有业务端点要求 `Authorization: Bearer ...`；`/healthz` 不暴露业务数据。

## HTTP

业务成功为 `CloudResponse.result {kind,data}`，错误为 `CloudResponse.error`；401 未认证、403 越权、409 身份／不可变内容／CAS／boot 冲突、422 输入或引用错误、503 I/O。事务失败回滚，不将冲突写成成功。错误不返回数据库连接串或数据库诊断细节。

|编号|方法及路径|请求／响应 kind|写主体|
|---|---|---|---|
|C01|PUT `/v2/projects/{project}/sessions/{session}`|CloudSessionCreate / session_stored|host_gateway|
|C02|PUT `/v2/sessions/{session}/commands/{command}`|Command / command_stored|host_gateway|
|C03|POST `/v2/devices/{device}/poll`|DevicePollRequest / device_poll|device|
|C04|POST `/v2/devices/{device}/events`|CloudReceiptRequest 或 TelemetryObservation / event_stored|device|
|C05|GET `/v2/sessions/{session}/events`|epoch、after_ingress_id、limit、visible_frontier / events_page|host_gateway|
|C06|PUT `/v2/sessions/{session}/snapshots/{revision}`|SnapshotPublication / snapshot_published|host_gateway|
|C07|GET `/v2/sessions/{session}/snapshots/{latest或revision或cutoff}`|epoch / snapshot|只读 scope|
|C08|POST `/v2/data/query`|DaaSQuery / query_result|只读 scope|
|C09|POST `/v2/commands/{command}/withdraw`|CloudWithdrawRequest / withdrawal|host_gateway|
|C10|POST `/v2/sessions/{session}/release-slot`|CloudReleaseRequest / slot_released|host_gateway|
|C11|POST `/v2/datasets/{dataset}/load-batches`|LoadBatch / load_ingested|generated_load_producer|
|C12|GET `/v2/projects/{project}/datasets`|dataset_catalog|只读 scope|
|C13|GET `/v2/projects/{project}/active-execution`|active_execution|只读 scope|
|C14|GET `/v2/commands/{command}`|command|只读 scope；device 仅自己命令|
|C15|GET `/v2/commands/{command}/receipts`|CommandReceiptsQuery / command_receipts|reader/gateway|
|C16|GET `/v2/sessions/{session}/inputs`|epoch / inputs|reader/gateway|
|C17|POST `/v2/sessions/{session}/observations/query`|ExecutionObservationQuery / observations|reader/gateway|
|C18|POST `/v2/sessions/{session}/history/query`|ExecutionHistoryQuery / history|reader/gateway|
|C19|POST `/v2/devices/{device}/boots/register`|DeviceBootRegisterRequest / device_boot|device|
|C20|POST `/v2/devices/{device}/boots/{boot}/retire`|DeviceBootRetireRequest / device_boot|host_gateway|
|OP01|GET `/healthz`|配置映射、真实 SELECT1、schema1 齐备为204空体，否则503空体|公开|

C05/C07/C16 GET 需 `execution_epoch_id=<epoch>`，也可传 URL 编码 JSON `identity`。C15 传 `query=<URL编码的完整CommandReceiptsQuery JSON>`，或分别将 `identity/page` 编码为 JSON query 字段。

C05 的既有 body 没有 F 字段；服务通过 `X-SF-Visible-Frontier` 返回首次固定 F，后续页必须将该值放入 `visible_frontier` query。body 保持原 CloudEventsPage。Host 应核对该 header 并沿同一 F 读至 `next_ingress_id=null`。

所有 POST 查询用 `page:{limit:1..200,cursor:null或原游标}`。游标为无签名 base64url JSON，仅描述位置，不授予权限。Telemetry/ExecutionCursor 的 `cursor_version` 按冻结 DTO 是字符串 `"1"`，CommandReceipts/Observation/HistoryCursor 为整数 `1`。每页重新认证并绑定原筛选、设备集合、F/R 和 limit；显式越权设备403，权限变化不能静默缩小原页集。

telemetry 按 DB `cloud_received_at` 的 UTC 微秒闭开区间 `[from,until)` 过滤，原始行按 ingress 排序；分钟聚合只含 telemetry/generated_load，排除 receipt，按 UTC minute/device/source 排序，row_index 从0开始。空和末页仍返回实际 selected_device_ids、dataset_id、F。

execution 查询固定同一 Host 快照，range 为 Host 毫秒闭开区间。rows 取该快照中的 attempt／acceptance，accepted_quantity 按单位用整数十进制精确求和，command_counts 取 attempt 状态；business_effect_committed/output_wait 归 device_completed，不将它们算为 accepted。waiting_time 保留未知 known_host_ms=null，并保留快照及相关 WorkTiming 的缺项原因，不用云接收时间或设备报告时间臆算 Host 等待。C17 是 T 时点每对象最后状态；C18 的历史时点范围按设计为闭区间 `[from,until]`，包括相等上下界，分页保留原观察前缀。

## 关键事务行为

同工程 slot 不按超时释放。session、command、poll、boot/sequence、load batch 都比较规范 typed 内容；空 poll 也不可变，退休 boot 仅可重放原结果。accept_commands=false 不 claim，仍原样返回已发布 Host probe；云端不延长 Host 期限。queued 撤回提供未投递事实；claimed 仅记录 cancel_requested，不代表已取消。

新消息及批次在 dataset 行锁后只取一次 DB clock_timestamp，以同事务 counter 分配连续 ingress；批次重放保留原时间，跨 batch 重复设备序列整批冲突。接纳、观察前缀、快照、head、cutoff 同事务发布。终态命令须匹配 Host-applied 终态 receipt；未投递撤回可凭原云端证明结清。slot 释放还须所有 boot 已退休及预留、产出、处置均结清。

## 本机独立验证

先核对23270/23271为空闲且属于本批。下面入口只使用精确命名的 `sf-v2-cloud-db-20261026`、专属持久卷和回环端口；凭据写入 ACL 限制的 `.work/v2-implementation-20261026/cloud/private/`。保留数据库、原始导出和失败证据。

```powershell
python tools/cloud/local.py prepare
python tools/cloud/local.py db-start
python tools/cloud/local.py migrate
python tools/cloud/local.py verify
python tools/cloud/local.py scale
```

verify 使用新组件数据库，真实启动 Axum 和 PG，包含并发事务、重启持久性、授权、boot、快照、固定 F/R、telemetry_only、微秒及迁移失败反例；结束停止自己的 HTTP 进程。scale 使用专属 `sfcloud_scale` 数据库，按冻结公式写入或精确重放1000批百万行，以 PostgreSQL COPY 原始行及独立 Python bitset／公式／分组作 oracle；先冻结查询清单，再 warmup，按 R00,A00…R29,A29 完整分页计时。两组各保留全部30结果，p95 为升序第29项，门槛2000ms。回执在本批 `.work/.../cloud/`。

当前独立验证不代替真实 Host gateway、10设备1800秒、Native及 Sealos 部署集成；这些出口由统筹接收后完成。
