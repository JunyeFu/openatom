# P3-01/P3-02 采用媒体与Draft精确来源消费者完整续作

继续 [P3-01原完整执行包](P3-01_整包执行.json)和 [P3-02原完整执行包](P3-02_整包执行.json)，保留全部原步骤/Q、九类定义创作与明确采用、同一行为三种媒体、三档、真实阶段/等待/回看、SF/IX/NFR/release。本轮是当前实际共享消费者的必要续作，不建立替代viewer或另一套业务/时钟。

生产者前置为 [共同Host完整修复包](P3-01_P3-02_媒体采用与精确读源完整续作.md)及 `scenarios/媒体表现/runtime-media-adoption-source/` 的最终冻源交接。Root确认生产者交回后方可接实际新Host，不把修复前的冻结rlib当新生产者。当前 `ItemType.module_ref`、实际ItemsAt/端口计数消费者已完成，勿重做其类型或覆盖其原证据。

已观测的实际接点有两个：`createPresentationHost` 的 module() 与 `createFactoryPresentation` 的 module() 仍写死 DefinitionDraft 的 ModuleVersion 元数据不可读；Host现已在原Scope内提供Draft自身及声明依赖闭包的精确版本。仅删除已失效的硬编码阻断，让请求继续经过原SDK、原Scope/当前Scope修订/精确版本校核。权限、Scope、current key、迟到回包及精确来源检查继续保留，不通过Project Scope或新库兜底。

实际Media采用如今以现有MediaRef壳保留精确源，source与详细/简化LOD均链接精确媒体版本；节点/动作保留Media包。完整核对共享 `resolveMedia` → 同Scope ModuleVersion/RunModuleVersion → 严格byte读取，确认普通设备定义、实际Run设备、主画布和Draft预览都消费同一链。详细/简化/静态图片/序列的声明路径应正确；新版本只有显式采用后才影响当前设备，旧Run和已冻结物品继续旧版。不能在测试里绕过原Device绑定直接喂Media源。

验证必须含真实Host实际采用、Draft自身和显式传递依赖metadata/bytes，实际共享组件或主画布看到合法GLB/图片；至少覆盖详细/简化切换、当前采用v2与旧Run v1、实际等待/反向seek、原Scope/R不变及迟到响应不覆盖新选择。保留Core原红；先保留前端旧阻断的真实失败，再最小修复并完成受影响原回归。若普通生产界面仍无法触达某条预览路径，指出确切入口，不将测试专用组件计作Native。

独占写范围：`apps/planner/src/services/presentationHost.ts` 与 `factoryPresentation.ts` 必要精确读源接点，`apps/planner/src/features/factory/DeviceProcessWorkspace.tsx` 的实际精确媒体声明相位接点，`tests/planner/media-adoption-source/`、`scenarios/媒体表现/runtime-media-adoption-consumer/`。Core已指出该Workspace在resolveMedia前从MediaRef壳的空timeline派生声明相位，应由同一实际精确Media解析结果提供，不能复制包内容或另猜进度。`features/media/load.ts` 仅在真实失败证明共享解析有缺陷时最小修正；其它GLB/模型资产、Native/Host/domain/API/Engine、Root Workbench不改。Core/Root可能在跑Cargo，不得运行Cargo；可直接链接已确认完整冻结的库组并明确cut，不能暗改旧原始结果。

本轮同时检查已观测一条Chromium immutable-texture警告：`runtime-factory-items/self-review.json`。如确定是现有资源清理/复用的实际缺陷，可最小修 `scene/factoryItems.ts` 或 `factoryMedia.ts` 并用原真实探针重现/复验；先向Root报告具体原因与文件，获得单写租约再改。不得隐藏console消息或将原警告从旧证据删除。

Root已逐行批准唯一旧协议资格演进：`tests/planner/process-workspace/presentationHost.test.mjs` 的setup一律拒Draft ModuleVersion及约135行“前端不支持DefinitionDraft”期待。保留原文件/原红，再仅允许真实本Draft自身或明确精确依赖的fixture读取，并将此期待改为同Scope精确元数据成功；其余权限、分页、历史与业务期待不改。该fixture演进不是实际Host证明，新真实Host联验单独保留。

资产详细等级统一引用 [设备建模精度与表现验收规范](../../02_交互设计/设备建模精度与表现验收规范.md)。不删机构、改方块或复制几何掩盖来源失败；无持久化空间绑定仍具体显示缺项，不把既有两项显式空间探针升级为正式生产者。完整handoff、自审、原业务/九份完整Run不变证明及TS/构建完成后一次冻源交Root。实际Native、工业质量、NFR与全部原release保持开放。
