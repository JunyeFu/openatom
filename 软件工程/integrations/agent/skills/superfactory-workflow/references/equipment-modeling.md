# Equipment modeling with external support

Read this for detailed equipment creation, replacement and quality review. Use the [canonical quality specification](../../../../../docs/02_交互设计/设备建模精度与表现验收规范.md) and its JSON for acceptance; do not invent another quality grade. This workflow supplies authoring guidance, not additional product tools.

## Explain the tool requirement once

The platform's native geometry supports boxes, cylinders, planes and imported GLB nodes. For curved mechanical housings, meaningful bevels, articulated assemblies and transferable materials, use an external modeling tool or a geometry program capable of the required result. Blender with Python/bpy is the default suggested offline tool; an existing Three.js generator is equally valid when its actual exported result meets the reference. Tool choice is not acceptance.

Before starting, inspect the available executable/version, export support and a usable local output folder. A configured but disconnected Blender MCP does not mean Blender is available. No paid service is necessary. State missing support concretely; do not downgrade the model in silence or claim a connector exists. Reuse authorized installed tools. Installation, paid services, reference uploads and redistribution retain the user's applicable authorization; this instruction does not grant new permission for them.

Example user-facing explanation:

> 这个详细设备需要外部网格、材质和刚性动画制作支持。平台负责接入、编辑绑定和运行显示。我将使用已确认可用的本地建模工具，交付可编辑源、GLB和同版本多视角检查图；最终以导入平台后的画面验收。若本机缺工具，我会说明缺什么及可行替代，不把方块简模当成最终设备。

## Build a family with one complete acceptance loop

New batches use canonical `simulation-efficient-v1`, revised by the user's 2026-10-27 efficiency instruction. Preserve L03 silhouette, materials, support, visible working mechanisms and the eight MQ dimensions. Do not turn business fidelity into manufacturing reconstruction. There is no default micrometer, mesh-volume error or animation sample-count gate.

1. Reuse a family and author differences only. Mature mechanisms may cover roughly 6–10 devices per batch; new mechanisms start with 1–2 representatives. Keep every definition independent. Reuse installed tools, materials, exporter and Host/Native fixtures. Do not refactor usable old batches or build a new fixture platform as a prerequisite.
2. Show coarse load→process→unload→blocked output→recovery before fine detail. Inspect the finished workpiece, support and distinct private staging/public buffer. Aim for a first preview in about 10–15 minutes with the installed chain; this is a method-change reminder, not permission to pass a defect.
3. Model what explains the visible process. Use one main moving assembly per visible action, rigid panels, a few bend segments or before/after meshes. Multi-ply paper can use one thickness plus edge lines; honeycomb interiors can use a cell texture and representative exposed sections; adhesive uses decals/thin sheets/state visibility. Do not derive every ply bend, coat each cell wall, reconstruct hidden mechanisms or infer business quantities from mesh volume. Use linear endpoints, grab/release boundaries and enough rotational poses for the intended path. After one unproductive noncritical repair, change representation instead of refining it again. Hole walls, staple clinches, repeated wire loops and enclosed drives may use whole meshes, repetition, textures or before/after states; do not reconstruct drilling, threading or wire forming continuously. Keep the main head and finished product recognizable, with countable major features consistent with definitions. A visibly simplified noncritical detail is allowed, not an open defect. Obvious workpiece penetration, unsupported main tools and wrong products still require correction. Stop detailing when the line view and complete-device view explain the machine.
4. Freeze and accept the batch once. Review all devices' MQ dimensions and applicable Host/Native states. Keep automatic seven-direction views for both LODs, but default to one detailed mechanism close-up; add another only for a concrete unclear feature. Keep actual Native normal and output-wait for every device, plus every special condition. Only missing, phase-wait and interrupted may use a named family representative when execution/wait drivers, identity rules and phase/holder/anchor bindings are shared. New mechanisms/bindings/routes or observed differences need their own waits. Keep full Host condition checks for every device; do not report shared Native waits as individually executed. Remove extra simplified-processing and finished-product close-ups. Component captures cover both LODs and product details, while output-wait shows the actual finished item. Add Native views only for an observed LOD difference, obscured product or rendering problem. One ordinary UI save and one new-process reopen cover all current assets and the same selected Native condition scope; source re-export once proves editability. After a local repair, re-export changed packages only, retaining unchanged item/device and business results. For fixed example paths, hand-author/bake sparse key poses rather than implement general IK, precise telescoping stages or real joint acceleration. Match pickup/release and tool parenting; smooth intermediate motion may be approximate. Normal business quantities, origins, holders, quality routes, clocks, waits and persistence stay exact.
5. Recheck only what changed and publish once. Keep actual observations and one batch review; do not add reports or counters during each repair. Camera edits need views; one-device geometry edits need affected views/contacts/current saved asset; business edits need affected Host/Native/persistence. Respect frozen Run bindings and rerun the affected scenario when required. No cancelled v1 stress matrix is revived by a modeling batch.

Use the shared capture entry rather than copying batch-number-only wrappers:

```text
node tools/3d-proof/native500/capture-batch.mjs <installed Playwright module> B02-Q07 coarse
node tools/3d-proof/native500/capture-batch.mjs <installed Playwright module> B02-Q07 views,feet
node tools/3d-proof/native500/capture-batch.mjs <installed Playwright module> B02-Q07 views corner-board-former
```

It uses the existing registered service on 19291; it does not install dependencies or start a server. `views` keeps seven directions per selected LOD plus `action-50`, checks phase/reverse seeking and writes a separate selected receipt for a device-only rerun. `coarse`, `feet` and optional `mechanisms` call existing batch hooks and still cover the whole batch. Historical full capture remains available in the original page without `capture=lean`. These captures are declared model previews, never Native production acceptance. New `acceptance-input.json` still explicitly selects `simulation-efficient-v1`; do not rewrite old acceptance or count unverified assets. Record known start/end times and accepted types only in the existing batch record; leave unknown times blank. Do not infer active work from file timestamps or claim an end-to-end speed multiplier from a reduction in captures.

### Existing batch receipts, with semantic coverage

Keep the existing review files. Keep `simulation-efficient-v1`; optional `acceptance-input.json.native_wait_reuse` maps each member to `{representative, basis}`. Basis names the actual shared execution/wait driver and phase/holder/anchor bindings; representatives do not chain and identity rules must match. No public module/API fields change. `contact-checks.json.critical_layout_checks` has one device row with `passed` and the four `stages`: `input`, `processing`, `output_wait`, `delivered` (pass, or a justified not-applicable). `behavior-checks.json` identifies each device and its applicable conditions. Native render checks name the actual device/condition and verify current Run/presentations. A separate finished-product Native close-up is optional; the shared review panel can use the missing-input state instead. Each selected persistence check identifies `device`, `condition`, `process`, `passed`, `holders_verified`; both real processes, every device's normal/output-wait/special cases, representative waits and every current saved media reference remain required. These are summaries of real observations, not manually filled acceptance substitutes.

After the Host writes its full `native-plan.json`, run `python tools/3d-proof/native500/acceptance_profiles.py <batch>` to write a separate `native-plan-selected.json`. Use that path with `launch-native.ps1 -PlanFile` and as the final optional argument to both ordinary-process `native-proof.mjs` calls. The persistence verifier and acceptance gate derive the same scope from the review config. Full Host plans/Runs remain intact; absent config preserves full per-device behavior. Record reuse in the batch receipt, not as member Native passes. Do not regenerate historical acceptance to adopt this policy.

Validate before changing registry/counters with `python tools/3d-proof/native500/accept-paper.py <batch> --validate-only`; publish only after the required real results and explicit visual review exist. The efficient branch checks actual semantic coverage instead of old per-batch picture/sample totals. Merge static detail that has no binding into shared geometry/materials while preserving moving parts, ports and holder anchors.

## Export for the actual loader

The product loader in [glb.ts](../../../../../packages/presentation/src/glb.ts) is the authority for compatibility. Currently require:

- GLB 2.0 with embedded buffers and PNG/JPEG images, triangle meshes; no external resource URI.
- Meter/Y-up export, declared origin, grounding, direction and envelope; account explicitly for the platform's Z-up world conversion. Keep consistent workpiece/fixture/port scale.
- Preserve named nodes, parent hierarchy, local pivots and rigid animation clips. No skinning, morph targets or non-triangle primitives. Export evaluated geometry from unsupported curves/modifiers; retain editable authoring sources separately.
- Use core metallic/roughness materials. Transfer necessary color, roughness/metalness and normal detail through supported image textures when required by appearance; names or flat colors do not substitute for a brushed surface. A simple part need not have textures merely to meet a count.
- The loader currently allows only KHR_materials_unlit and KHR_texture_transform extensions. Disable Draco, meshopt, KTX2/BasisU, GPU-instance and unsupported material-extension exports, or convert them to supported core data. Do not change the loader's supported list in a modeling task.
- Export actual rigid transform animation; material/light animation or a moving camera is not the equipment's working mechanism. Confirm exported clip order and node mapping from the GLB.

Primary exporter reference, checked 2026-10-19: [Khronos/Blender exporter documentation source](https://github.com/KhronosGroup/glTF-Blender-IO/blob/main/docs/blender_docs/scene_gltf2.rst). It documents triangle conversion, metallic/roughness materials, texture channels and animation. Blender's exporter supports more than this product loader: exporter support does not imply platform support. Read settings for the actual installed version rather than copying guessed version-specific flags.

## Per-model evidence and delivery

Register files under the selected module package, with editable source, parameters, dependency/version instructions, GLB, lawful material assets, node/clip/port mapping and a reopenable example. Generated images remain design references or previews, never substitutes for actual-model captures.

The shared ledger is [model-quality-ledger.json](../../../../../modules/media/model-quality-ledger.json). Run from the software-engineering root:

```powershell
python tools/check_model_quality.py --refresh
python tools/check_model_quality.py --check
python tools/check_model_quality.py --release-catalog modules/media/p302-l03-v03/registration.json
```

The last command checks the selected current module catalog, not every historical copy. Use the actual delivery catalog when the selection changes. Refresh never creates acceptance. File changes require review of affected results, retaining the previous review and reusing unaffected device/runtime evidence. Camera/text changes do not revoke unchanged model acceptance. A fixture or diagnostic asset cannot become an official detailed asset by changing its name.

Every selected model requires an explicit source/export review, visual review across all eight canonical dimensions, and product/reopen review. Required visual evidence covers isometric/front/back/left/right/top/bottom, task-specific mechanism/material close-ups and relevant motion/wait cases under the canonical per-device/family coverage above. Non-applicable items require a reason. Exact review fields are in the canonical JSON. Give qualitative, defect-specific conclusions; no weighted scores, triangle minimums or inferred acceptance from passing tests. Do not auto-fill pass or human approval.

For each handoff state which gates passed, name remaining defects and identify the exact model/module version. A source/export or visual candidate can be handed off for the next step, but the official detailed-model delivery gate remains closed until all required checks pass. Do not mutate another protected model source or a running Native window merely to complete this workflow; prepare the scoped asset change and its real remaining integration work.
