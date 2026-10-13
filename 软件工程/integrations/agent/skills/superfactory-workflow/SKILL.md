---
name: superfactory-workflow
description: Use connected Super Factory tools to generate editable factories from industry, capacity and site goals, author modules, inspect runs, and export evaluations.
---

# Super Factory workflows

The [first-release scope baseline](../../../../docs/00_项目基线/首版交付基线.md) defines required and deferred work; the selected task JSON defines the current release projection. The platform MCP bridge is required, while external CAD/control connectors and other professional kernels are deferred. Scope design does not make a connected capability available.

Repository skill design, not an installed integration. The platform runtime and tool bridge are not implemented yet. Discover the actual connected tools and capabilities before acting; if absent, report the missing runtime rather than fabricating execution.

Read the [API and ownership contract](../../../../docs/03_架构与契约/首版类型格式与操作契约.md) for data meaning and the [integration workflows](../../../../docs/03_架构与契约/通用Agent接入与专业软件连接器.md) for the relevant task. These files define intended operations; the connected tool catalog defines actual availability.

For module authoring, inspect the [nine category contracts](../../../../docs/03_架构与契约/九类模块清单与消费者.md). Items, devices, resources, tasks, operations, transport, halls/compositions, professional providers, and media have different payloads and sample checks. Edit definitions through shared commands, then explicitly adopt the requested versions into the project. A hall/composition uses the existing templates category; do not create a tenth category or treat every payload as a device. A saved provider definition is not an implemented solver.

For device work, inspect supported geometry, behavior, ports and labels; edit the same draft the human can edit. Obtain a real render before judging appearance, and run a supported isolated cycle separately before judging production behavior. Save or adopt only the requested draft/instances.

For factory work, inspect product/BOM, existing objects and scope; construct a draft through shared commands, bind material/resource roles explicitly, and run the selected scenario. Explain waiting with actual object IDs, intervals and required conditions. Propose a changed scenario when requested; do not call a visually smooth flow an optimal or calibrated factory.

For a one-sentence production goal, read the [goal-driven factory workflow](../../../../docs/03_架构与契约/目标驱动厂房生成.md). Use query_module_library to select an actual native/imported community industry/product template compatible with the site's length, width and height. Resolve the production-day calendar, product specification, cycle/quantity, resource, supply and dimensional inputs from the user or clearly labeled template/module assumptions. Instantiate and fill the nine-category roles through shared commands; never bind the requested daily quantity to an output result.

Check material/port/resource references and declared footprint, height and space allowances with check_factory_draft; request a render of that revision, then run the actual scenario. Read accepted output at the stated time/window and destination in the chosen accepted unit. If the initial draft has sufficient inputs, passes checks and actually meets the modeled goal, save and export it directly: no change, revised RunId or comparison is required. Only for a shortfall or failed check, repair a supported and authorized condition and compare a new run, or explain why inputs, space or capabilities prevent a repair. Verify repair capability with a separate shortfall case rather than forcing it into every successful task. Stop on meeting the modeled goal, missing evidence/needed human decisions, unsupported scope or the task's finite execution budget; preserve the editable draft either way. This is template-driven generation with feedback, not an unbounded global optimizer.

For carton manufacturing, confirm the product family and required route. The initial sample uses purchased board for die-cut/fold-glued cartons with supported optional printing, inspection and bundling. Packing another product into an existing carton is a different factory. Count accepted cartons, not bundles; forming/manufacturing artifacts do not supply guessed production cycle times.

Scheduling edits change release/due dates, priorities, arrivals, calendars, and supported phase durations. Actual start/completion follows dependencies and resources; moving a planned bar does not manufacture a result. Track a purchased or machined part through assembly and packaging using actual item/container relationships, not only the current camera view.

For packaging work, obtain product revision, material/connection and use conditions. Call actual packaging computation, preserve Pass/Fail/Undetermined and its supported scope, and return editable geometry/manufacturing outputs. Human acceptance and factory binding remain distinct; a rendered box or guessed cycle time is not a professional result.

For evaluation delivery, read the [report contract](../../../../docs/03_架构与契约/评估报告与数据导出.md) and discover whether export_evaluation is actually available. Select the requested scenarios/runs, metric scope, formats, and destination. PDF, offline HTML, CSV tables, and JSON must use the same result and unit/acceptance definitions. Without an actual run, export only a clearly marked design description. An evaluation report, editable project package, and community module package serve different purposes; prepare contribution files without publishing them automatically.

Keep edits on the project revision read from the host. On RevisionConflict, read the current context and reconcile the requested change; do not overwrite the user's edits. Running inventory, view playback and draft versions are separate. Report missing inputs or capabilities explicitly.

Use the user's existing authorization and selected project/file scope. This workflow does not authorize installing providers, configuring global clients, publishing, submitting entries, or sending files to other services. A private asset's availability does not grant redistribution rights.
