# Sealos 北京 A 云服务交付准备

2026-10-27：已在批准的15代币内完成真实Sealos部署。当前云源码镜像已推送私有Harbor并按digest部署，PG16.4单实例、20Gi盘及schema1迁移成功；公网HTTPS、授权/匿名访问和真实PG TLS共10项检查通过，见[实际托管结果](../../../.work/v2-implementation-20261026/sealos-live-deploy/hosted-checks.json)。Native09真实两设备正常六件53.202秒、H10四格式22文件和NativeH11全110行已核对；Native10首次Started后禁止A新单也已完成六件97.092秒、A1/B5，原A任务正常完成且无新A尝试，见[正常组](../../../.work/v2-implementation-20261026/sealos-live-deploy/positive-export-verification.json)与[对照组](../../../.work/v2-implementation-20261026/sealos-live-deploy/ban-A-verification.json)。首次4/6及原CAS被拒结果保留；实际托管百万数据、持续采集和其余交付仍在进行，尚未记整体产品通过。旧Render方案不用于本次平台配置。

本次dry-run通过Job、Deployment、Service、Ingress四类资源，补齐实际服务器提示的非root、禁止提权、丢弃capabilities及RuntimeDefault seccomp声明后无警告；前后四个目标均不存在。输入使用明确的夹具镜像、域名、class和证书名，只证明结构和租户准入，不证明镜像可拉取、真实域名/证书或PG可用。按[官方平台规范](https://github.com/labring/sealos-skills/blob/main/plugins/sealos/skills/use-sealos/references/platform.md)统一应用名/容器名/分组标签，客户端模板使用2048Mi，容量仍为2GiB。

同一已有镜像补验9项通过：实际迁移、UID10001、cap-drop ALL、禁止提权、Docker内置seccomp、1核/2GiB限制、204就绪、PG实际TLS1.3、reader目录查询及匿名401。迁移与服务均退出0，原组件PG恢复停止并保留数据；未接触百万行数据库。两次启动器失败分别漏传原SSL_CERT_FILE变量、错误预期Docker返回的安全选项格式，原失败和修正后结果均保留；不记为产品源码缺陷。

## 构建和有限运行验证

在软件工程根运行，使用现有 Python 3.11+、Docker Linux engine。验证脚本另需现有 `cryptography`；本轮使用随 Codex 提供的 Python 3.12.14 / cryptography 50.0.2，没有安装依赖。

```powershell
python -B tools/cloud/sealos/build.py --output C:/Users/fujunye/AppData/Local/Temp/sf-sealos-container-20261027/build-NEW
python -B tools/cloud/sealos/verify_image.py --image sf-cloud-service:sealos-20261027
```

`build.py` 只复制云 crate 清单/锁/src/迁移、domain 清单/src 和 Dockerfile，在新 C 盘上下文生成一个只注册 domain 的最小根工作区。继承字段及 domain 的五个依赖逐项取当前根 Cargo；云独立 `[workspace]` 和锁原样保留。Dockerfile 在 Rust 1.97.0 bookworm 中 `cargo build --release --locked`，运行层使用 Debian bookworm-slim、libssl3、系统 CA 和 curl，UID/GID 10001，直接执行云二进制。构建上下文不含 `.work`、凭据、journal、Host、前端、模型或存档。新 cache 位于 `--output/build-cache`，没有搬动或清理旧缓存。

实际配置入口为 `serve` / `migrate`，就绪入口为 **GET `/healthz`**；任务书 `/health` 是简写，本批沿用源码及 OP01。Docker HEALTHCHECK 必须收到 204；Kubernetes 只配置真实 readinessProbe，数据库故障时摘流，不用它反复重启应用。HTTP 在容器内监听 `0.0.0.0:8080`，云数据全部持久化到 PG，应用无需数据盘。

`verify_image.py` 为每次运行新建带批次标签的容器、PG 数据卷及网络；仅发布 Docker 自动分配的 `127.0.0.1` 端口。使用真实 PG、新组件库和三天有效的测试 CA/主机证书，**不使用现有 sfcloud_scale 或百万数据**。测试内容包括缺配置退出、迁移前503/后204、16表、reader认证、真实 TLS 会话、拒绝不可信 CA/错误 hostname、迁移重放、Docker探针和 SIGTERM。即便连接串写 `sslmode=disable`，实际服务仍强制 Require；本轮记录到 TLS1.3 / TLS_AES_256_GCM_SHA384。测试凭据和私钥仅写 ACL 限制的私有目录，不输出内容。结束正常停止并保留本批容器/卷/网络和日志；测试 CA 不用于云部署。

## 公共部署模板和私有配置

`application.template.json` 包含 Deployment、ClusterIP Service 和 HTTPS Ingress；`migration.template.json` 是独立的一次迁移 Job。它们是 Kubernetes 原生 JSON，也可交给 `kubectl -f`。模板仅引用名称，不携带秘密值；本次已在实际工作空间创建下列配置对象：

|资源|内容与责任|
|---|---|
|Secret `sf-cloud-runtime`|`DATABASE_URL`、`SF_CLOUD_AUTH_TOKENS_JSON` 两键；平台私有配置，由实际托管 PG 和权限目录生成|
|Secret `sf-cloud-pg-trust`|`ca.pem`：PG 提供方可信 CA/完整信任 bundle；必须从可信平台来源取得|
|Secret `sf-cloud-registry-pull`|`kubernetes.io/dockerconfigjson`；目标 registry 拉取权限，不给 Native/设备|
|ConfigMap `sf-cloud-bindings`|`demo-bindings.json`；用现行 `tools/cloud/make_bindings.py` 从实际 Host 冻结输入生成|
|平台 HTTPS Secret|Root 从北京 A 应用创建/导出配置取得实际名称，填 `--https-tls-secret`|

服务身份 JSON 使用已实现的严格 `token/principal_id/role/project_ids/device_ids/dataset_ids` 六字段。`host_gateway` 只给 Host 网关；`device` 每设备独立 token、单设备且匹配固定 principal；`scoped_reader` 仅获准读范围；`operator` 独立权限；`generated_load_producer` 只绑定一个 generated_load dataset。完整目录仅在服务 Secret 中，Native/设备各拿自己的受管私有配置，不能将全部 token 放入工程、Renderer、报告或分发 ZIP。配置格式以 `apps/cloud-service/README.md` 的当前消费者为准。

PG 必须提供支持 TLS 的 **DNS hostname**、端口、数据库、账户及可信 CA，证书 SAN 匹配该 hostname。服务和 Job 都挂同一信任 bundle 并使用 `SSL_CERT_FILE`；此机制已在本镜像的真实 PG 正例/反例中验证。不得设置 `SF_CLOUD_LOCAL_NO_TLS`，不得用 IP 替代证书 DNS 或关闭证书验证。北京 A 实际 PG 若尚不支持这一连接条件，先由 Root 配好 TLS 入口/CA，再执行迁移，不能以明文接通作为交付。

模板应用为单实例、Recreate发布、30秒退出期，实际服务最多20秒drain。应用limits为1核/2048Mi，requests按平台规范为100m/204Mi；迁移limits为1核/1024Mi、requests为100m/102Mi，单Job最多1800秒，对应预算方案的半小时迁移窗口。原数据库和Host epoch/命令身份继续保留；模板不创建或删除PG/PVC，不削减数据量、查询或持久化功能。未知schema由现有迁移拒绝。

## Root 部署顺序

目标为 `https://bja.sealos.run/` 的 My Workspace，已确认namespace为`ns-61j010p3`。仅使用原有私有登录，不改默认kubeconfig/context；实际认证信息仅保留在用户私有目录或平台Secret，未输出或写入工程。

1. 当前实际PG16.4已运行，单实例1CPU/2Gi/20Gi；`postgres.template.json`含Cluster及基于上游Patroni/lorry实际需要的命名空间Role，不授予Secret或RBAC通配权限。TLS使用内置CA和SAN匹配的Pod DNS；`DoNotTerminate`保留数据。实际预算、独立业务库和各角色Secret/ConfigMap已准备完成。
2. 确定可推送且 Sealos 可拉取的实际 registry、项目路径、仓库和权限，推送经本机验证的镜像。官方公有云部署说明支持国内外镜像仓库和私有镜像认证。[镜像部署](https://sealos.run/docs/getting-started/deploy-docker-image)、[应用管理](https://sealos.run/docs/guides/app-management)
3. 取得推送后的 registry digest（**不以本机 image ID 代替 registry digest**）、平台分配的 HTTPS hostname、证书 Secret 和 ingress class，渲染私有部署文件：

```powershell
python -B tools/cloud/sealos/render.py --namespace <实际namespace> --image-digest-ref <registry/project/sf-cloud-service@sha256:推送后digest> --public-host <平台实际host> --https-tls-secret <平台证书Secret> --ingress-class <实际class> --output .work/v2-implementation-20261026/sealos-container/deploy
```

4. Root 审阅实际目标/费用后先应用 `deploy/migration.json`，等 `sf-cloud-schema1` Job Complete，确认原日志 `schema version 1 ready`。失败即停，不部署假就绪服务。然后应用 `deploy/application.json`，等 Deployment 可用、HTTPS `/healthz` 为204，真实 bearer 访问 C12；再做原 Host/Native/设备及规模验收的实际集成。迁移 Job 保留；需要重试时先查原失败，另用新 Job 名，不覆盖证据。
5. 核对云端 PG 真实 TLS 会话、重启持久化、原授权和查询行为。现有109项组件及百万/60查询结果可复用；本镜像有限验证不替代真实托管环境的集成/性能出口。

部署/Service/Ingress 的结构参考[官方模板](https://sealos.run/docs/guides/appstore/code-templets)；Ingress 使用原路径和 HTTPS 跳转。实际 hostname/证书不能从示例推测，未在本轮创建公网入口。

### Registry 认证及推送方式

[官方通用镜像仓库文档](https://sealos.run/docs/guides/image-registry)说明先登录、推送，再由应用引用。[官方私有云镜像服务](https://sealos.run/docs/private-cloud/deployment/image-registry)描述 `hub.<域名>/<项目>/<仓库>:<tag>` 和访问密钥流程，**它属于私有云文档，不能据此认定北京 A 公有云存在 `hub.bja.sealos.run` 或账户已获写权限**。实际地址/项目目录必须取 Root 现有平台或选定的 registry 配置；没有核实前不填写可上线地址。

Root 确认发布权限后，可在其受管私有目录执行下列标准 Docker 流程。`SF_REGISTRY_*` 仅是本地操作变量，不传给应用。`SF_REGISTRY_TOKEN_FILE` 是私有文件；`SF_REGISTRY_AUTH_DIR` 是独立受限目录，避免改默认 Docker 登录配置。Registry 账户有目标仓库 push 权限，平台 pull Secret 使用对应拉取权限。

```powershell
Get-Content -Raw -LiteralPath $env:SF_REGISTRY_TOKEN_FILE | docker --config $env:SF_REGISTRY_AUTH_DIR login $env:SF_REGISTRY_HOST --username $env:SF_REGISTRY_USER --password-stdin
docker tag sf-cloud-service:sealos-20261027 "$env:SF_IMAGE_REPOSITORY`:sealos-20261027"
docker --config $env:SF_REGISTRY_AUTH_DIR push "$env:SF_IMAGE_REPOSITORY`:sealos-20261027"
docker image inspect "$env:SF_IMAGE_REPOSITORY`:sealos-20261027" --format '{{json .RepoDigests}}'
```

`SF_IMAGE_REPOSITORY` 为实际 `registry/项目/sf-cloud-service`；部署使用 push 返回/仓库核对的 digest。私有 registry 登录后不输出 token、docker auth 内容、数据库 URL 或 auth catalog。认证参数流程依据 [Docker 官方 login](https://docs.docker.com/reference/cli/docker/login/)。

## 版本、资源与费用核对（2026-10-27）

[官方创建数据库说明](https://sealos.run/docs/getting-started/create-database)提供操作背景；当前版本和TLS结论来自本次实际PG运行、证书及后端会话，而非旧Render或本机组件结果。当前源码镜像另在本机PG16.4上通过19项有限检查。

当前应用与PG均为单实例，各 **1 CPU/2Gi**，PG持久盘 **20Gi**，与实际Pod资源一致。迁移Job短时1CPU/1Gi，已正常退出。服务无本地数据盘。

[官方帮助](https://sealos.run/docs)和[计费说明](https://sealos.run/docs/billing)已核对；正式估算使用本次认证账户/区域API的原生货币数值及1000000显示尺度，无人民币或美元假定换算。余额15、查询当时扣费0；48小时基础费用 **3.8048110848代币**，加规划迁移、1500Mi出网及额外24小时保留盘等参考 **5.576456974代币**。详见[计算与实价](../../../.work/v2-implementation-20261026/sealos-price-query/actual-budget.json)。资源已创建，后续以实际账单为准，费用上限仍为15。

[官方配额说明](https://sealos.run/docs/billing/resource-quotas)区分空间资源上限与账单；既有设计记录的16核/64Gi/20Pod/100Gi配额不是钱包余额，本轮没有再次查询或把它记成已购容量。暂停计算不等于保留磁盘不收费；数据库数据和证据由 Root 按实际交付安排保留，本批不清理。

当前registry为`harbor.bja.sealos.run/ns-61j010p3/sf-cloud-service`，部署digest为`sha256:672a29466d3074f5aa49e718db834a73b7545dbd6ff403eb1cc7ae917183dc3a`。平台实际应用域为`sealosbja.site`、TLS Secret引用`wildcard-cert`，Ingress采用官方模板的nginx类别。真实HTTPS证书校验已通过。后续重点为原Native完整执行、H11、十设备持续采集及规模/时延验收。
