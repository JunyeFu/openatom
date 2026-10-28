import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { compileReactFeature } from './load-react-source.mjs';
import { rootUrl } from './load-source.mjs';
const { DeviceConnectionImpact, AuthoringFailure, ConnectionEditor, ModuleAuthoringWorkspace, fieldExamples, categories, refKey, AdoptionPreparation, TemplateCompositionEditor, emptyComposition, adoptionDifferences, roleRequirements, ModulePresentationPreview, saveTemplateComposition, TemplateCompositionSaveFailure, readAuthoringTemplatePreview, createAuthoringTemplates, captureCompleteSeed, clearCompositionSeed, compositionInput, compositionFromPreview, DefinitionSampleResult, HostFailure, HostFailureDetails, DeclaredParameters } = await compileReactFeature();
const fixture = async name => JSON.parse(await readFile(new URL(`tools/type-contract-proof/fixtures/${name}.json`, rootUrl), 'utf8')).data;
const project = await fixture('project'), scenario = await fixture('scenario');
const workspace = { project, context: {host:{session_id:'finite-render-session'}}, capabilities: { implemented_operations: [{ operation: 'apply_commands' }], providers: [] }, modules: new Map(Object.values(fieldExamples).map(module => [refKey(module.definition), module])), scenarios: [scenario] };
const forbiddenIO = () => { throw Error('SSR must never call native/Host IO'); };
const output = new URL('scenarios/自定义模块/runtime-authoring/ssr/', rootUrl);

test('actual adoption port options distinguish copied device identities and exact module sources', () => {
  const actual = structuredClone(project), module = structuredClone(fieldExamples.devices);
  const reference = { id: module.definition.id, version: module.definition.version };
  actual.devices = ['CNC01', 'copy:CNC01'].map(id => ({ ...structuredClone(project.devices[0]), id, title: 'Same CNC', module_ref: reference }));
  const view = { ...workspace, project: actual, modules: new Map([[refKey(reference), module]]) };
  const html = renderToStaticMarkup(React.createElement(AdoptionPreparation, { workspace: view, module, busy: false, onCancel() {}, onApply: forbiddenIO }));
  for (const id of ['CNC01', 'copy:CNC01']) for (const port of module.definition.payload.data.ports) {
    assert.ok(html.includes(`Same CNC · ${id} / ${port.id} · ${reference.id}@${reference.version}`), `${id} must have an explicit, distinguishable port label`);
  }
});

test('device adoption includes the exact independent rule and phase resource roles', () => {
  const device = structuredClone(fieldExamples.devices), operation = structuredClone(fieldExamples.operations);
  operation.definition.id = 'adoption.exact-rule'; operation.definition.version = '0.1.0';
  operation.definition.payload.data.resource_roles = [{ role: 'machine', quantity: { value: { coefficient: 1, decimal_places: 0 }, unit: 'slot' }, qualifications: [] }];
  operation.definition.payload.data.phases = [{ id: 'load', duration: { value: 10000, unit: 'millisecond', basis: 'user_provided', source_ref: null }, interruption_policy: 'non_preemptive', work_kind: 'loading', phase_resources: [{ role: 'worker', quantity: { value: { coefficient: 1, decimal_places: 0 }, unit: 'person' }, qualifications: [] }] }];
  device.definition.payload.data.behavior = { selected: 'operation_ref', inline: null, operation_ref: { id: operation.definition.id, version: operation.definition.version } };
  const newer = structuredClone(operation); newer.definition.version = '0.2.0'; newer.definition.payload.data.resource_roles[0].role = 'other-version-machine'; newer.definition.payload.data.phases = [];
  const modules = new Map([[refKey(newer.definition), newer], [refKey(operation.definition), operation], [refKey(device.definition), device]]);
  assert.deepEqual([...roleRequirements(device, modules).resource].sort(), ['execution_capacity', 'machine', 'worker']);
  const html = renderToStaticMarkup(React.createElement(AdoptionPreparation, { workspace: { ...workspace, modules }, module: device, busy: false, onCancel() {}, onApply: forbiddenIO }));
  assert.match(html, /资源角色 machine/); assert.match(html, /资源角色 worker/); assert.doesNotMatch(html, /other-version-machine/);
  modules.delete(refKey(operation.definition));
  assert.deepEqual(roleRequirements(device, modules).resource, ['execution_capacity']);
  device.definition.payload.data.behavior = { selected: 'inline', operation_ref: null, inline: operation.definition.payload.data };
  assert.deepEqual([...roleRequirements(device, modules).resource].sort(), ['execution_capacity', 'machine', 'worker']);
});
test('actual operation form distinguishes released quality from released definition and explains empty admission',()=>{
  const definition=structuredClone(fieldExamples.operations.definition);definition.definition_state='released';definition.payload.data.inputs[0].required_quality=['unchecked','released'];
  const html=renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace,{workspace,busy:false,initialDraft:{definition,summary:{draft_id:'quality-label-regression',revision:1,definition_ref:{id:definition.id,version:definition.version},issues:[]},history:null},io:{dispatch:forbiddenIO,awaitTask:forbiddenIO,pickDirectory:forbiddenIO},samples:{status:'unavailable',reason:'no IO'},templates:{status:'unavailable',reason:'no IO'},onDraftChanged(){},onCommitted:forbiddenIO,onPreviewChange(){},onConsumerRequested(){},onClose(){}}));
  assert.match(html,/value="&quot;released&quot;" selected="">已放行<\/option>/);
  assert.match(html,/value="&quot;released&quot;" selected="">已发布版本<\/option>/);
  assert.match(html,/空列表不会允许所有质量/);assert.match(html,/实际作业会等待质量条件/);
});
test('composition authoring offers explicit actual resource capture and whole-group selection without adopting or calling IO', () => {
  const subset=emptyComposition(),resource=project.resources[0];
  subset.devices=[{...project.devices[0],role_bindings:{item_types:{},resources:{capacity:resource.id},targets:{},ports:{}}}];
  const html=renderToStaticMarkup(React.createElement(TemplateCompositionEditor,{workspace,value:subset,baseRevision:project.revision,onChange:forbiddenIO,disabled:false}));
  assert.match(html,/按已选组合及声明种子重建资源需求（1 项）/);assert.ok(html.includes(resource.id));
  assert.match(html,/选择全部连接/);assert.match(html,/采用时仍须分别选择新建或复用资源/);
  assert.match(html,/不复制实例标识、驻留位置、当前占用或 Run/);
  assert.deepEqual(subset.resource_requirements,[]);assert.equal(subset.seed,null);
});

test('actual composition UI counts supply resources only for an explicitly attached exact source and keeps editable declarations folded', () => {
  const actual=structuredClone(project),resource=actual.resources[0],carrier={...structuredClone(resource),id:'captured-carrier',title:'Captured carrier',kind:'vehicle',role:'carrier'};
  actual.resources.push(carrier);
  const source={...structuredClone(scenario),id:'source-with-supply',revision:12,supplies:[{resources:{carrier:carrier.id}}]},subset=emptyComposition();
  subset.devices=[{...actual.devices[0],role_bindings:{item_types:{},resources:{capacity:resource.id},targets:{},ports:{}}}];
  const view={...workspace,project:actual,scenarios:[source]},captured=captureCompleteSeed(subset,source);
  const render=(value,sourceScenario,scenarios=view.scenarios)=>renderToStaticMarkup(React.createElement(TemplateCompositionEditor,{workspace:{...view,scenarios},value,sourceScenario,baseRevision:actual.revision,onChange:forbiddenIO,disabled:false}));
  assert.match(render(subset,captured.source_scenario),/按已选组合及声明种子重建资源需求（1 项）/);
  const html=render(captured.subset,captured.source_scenario);
  assert.match(html,/按已选组合及声明种子重建资源需求（2 项）/);assert.match(html,/Captured carrier/);
  assert.match(html,/<details><summary>精确模块引用 · 0 项 · 查看与编辑/);
  assert.match(html,/<details><summary>资源需求正文 · 0 项 · 查看与编辑/);
  assert.match(html,/仅选择方案候选不加入/);
  const drifted=render(captured.subset,captured.source_scenario,[{...source,revision:13}]);
  assert.match(drifted,/按已选组合及声明种子重建资源需求（1 项）/);assert.match(drifted,/当前返回的来源方案为 r13/);
});

test('real Host failure detail keeps every admission reason and affected object, without invented reasons', () => {
  const detail={code:'InvalidInput',message:'Frozen run inputs did not pass execution admission',field_errors:[{pointer:'/devices/4/configuration/cycle_seconds',reason:'inactive_parameter_binding: The original inline branch is inactive'},{pointer:'/routes/0/steps/4/operation_ref',reason:'operation_binding_mismatch: Select an exact operation'}],object_refs:[{kind:'device',id:'Assembly01'},{kind:'route',id:'reviewed-route'}],expected_revision:null,actual_revision:null};
  const html=renderToStaticMarkup(React.createElement(HostFailureDetails,{detail,onLocate:forbiddenIO}));
  for(const field of detail.field_errors){assert.ok(html.includes(field.pointer));assert.ok(html.includes(field.reason));}
  for(const object of detail.object_refs)assert.ok(html.includes(object.id));
  assert.match(html,/<details/);assert.match(html,/查看具体原因与相关对象/);assert.doesNotMatch(html,/disabled/);
  const locked=renderToStaticMarkup(React.createElement(HostFailureDetails,{detail}));assert.equal((locked.match(/disabled=""/g)??[]).length,2);
  assert.equal(renderToStaticMarkup(React.createElement(HostFailureDetails,{detail:{...detail,field_errors:[],object_refs:[]}})), '');
});

test('actual parameter editor exposes explicit removal and disables an inactive inline binding', async () => {
  const module=structuredClone(fieldExamples.devices),snapshot={project:structuredClone(project),scenarios:[scenario]},device=snapshot.project.devices[0];
  module.definition.id=device.module_ref.id;module.definition.version=device.module_ref.version;
  module.definition.parameters=[{id:'cycle_seconds',value_type:'duration',binding:'/payload/data/behavior/inline/duration',label:'原工时',unit:'millisecond',integer:true,options:[],basis:'module_declared'}];
  device.operation_binding=fieldExamples.operations.definition;device.configuration={cycle_seconds:{kind:'duration',value:{value:40000,unit:'millisecond',basis:'module_declared'}}};
  const catalog=JSON.parse(await readFile(new URL('packages/commands/generated/schemas/get_object.response.schema.json',rootUrl),'utf8'));
  const props={selectedDeviceId:device.id,snapshot,modules:new Map([[JSON.stringify([device.module_ref.id,device.module_ref.version]),module]]),catalog,commands:{},onChange:forbiddenIO,onRemove:forbiddenIO,onLocate:forbiddenIO};
  const html=renderToStaticMarkup(React.createElement(DeclaredParameters,props));
  assert.match(html,/当前已选择独立工艺/);assert.match(html,/移除此实例覆盖 · 原工时/);assert.match(html,/disabled=""/);
  device.configuration={};const unset=renderToStaticMarkup(React.createElement(DeclaredParameters,props));assert.match(unset,/未设置实例覆盖/);assert.doesNotMatch(unset,/移除此实例覆盖/);
});
await mkdir(output, { recursive: true });
for (const category of categories) test(`actual ${category.id} component renders its canonical structured fields`, async () => {
  const definition = structuredClone(fieldExamples[category.id].definition);
  const markup = renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace, {
    workspace, busy: false, initialDraft: { definition, summary: { draft_id: `finite-${category.id}`, revision: 7, definition_ref: { id: definition.id, version: definition.version }, issues: [] }, history: { can_undo: true, can_redo: false, boundary_reason: null } },
    io: { dispatch: forbiddenIO, awaitTask: forbiddenIO, pickDirectory: forbiddenIO }, samples: { status: 'unavailable', reason: '没有实际样例执行者' }, templates: { status: 'unavailable', reason: '没有实际组合文件写入者' }, onDraftChanged() {}, onCommitted: forbiddenIO, onPreviewChange() {}, onConsumerRequested() {}, onClose() {},
  }));
  assert.match(markup, new RegExp(category.heading));
  assert.match(markup, /共同身份与版本/); assert.match(markup, /采用/); assert.match(markup, /贡献包/);
  assert.doesNotMatch(markup, /<textarea|type="file"/);
  assert.equal((markup.match(/aria-current="page"/g) ?? []).length, 1);
  if (category.id === 'devices') { assert.match(markup, /内联/); assert.match(markup, /独立工艺/); }
  if (category.id === 'tasks') { assert.match(markup, /<legend>任务定义<\/legend>/); assert.doesNotMatch(markup, /<legend>tasks<\/legend>/); }
  if (category.id === 'transport') { assert.match(markup, /<legend>运输定义<\/legend>/); assert.doesNotMatch(markup, /<legend>transport<\/legend>/); }
  if (category.id === 'templates') { assert.match(markup, /<legend>模板定义<\/legend>/); assert.doesNotMatch(markup, /<legend>templates<\/legend>/); }
  if (category.id === 'professional') { assert.doesNotMatch(markup, />立即计算</); assert.match(markup, /<legend>专业定义<\/legend>/); assert.doesNotMatch(markup, /<legend>professional<\/legend>/); }
  await writeFile(new URL(`${category.id}.html`, output), markup);
});
test('actual adoption confirmation exposes explicit reuse/seed controls and sends no IO during render', () => {
  const markup = renderToStaticMarkup(React.createElement(AdoptionPreparation, { workspace, module: fieldExamples.devices, busy: false, onCancel() {}, onApply: forbiddenIO }));
  assert.match(markup, /role="dialog"/); assert.match(markup, /供料/); assert.match(markup, /订单/); assert.match(markup, /取消/);
  assert.doesNotMatch(markup, /type="checkbox" checked=""/);
});

test('resource adoption explains preserved instance inputs and explicit default application', () => {
  const props={workspace,busy:false,onCancel(){},onApply:forbiddenIO};
  const html=renderToStaticMarkup(React.createElement(AdoptionPreparation,{...props,module:fieldExamples.resources}));
  assert.match(html,/升级已有资源只更新精确定义引用/);
  assert.match(html,/保留实例容量、角色、资格和日历/);
  assert.match(html,/明确修改并检查应用/);
  assert.match(html,/日历仍须明确选择/);
  const device=renderToStaticMarkup(React.createElement(AdoptionPreparation,{...props,module:fieldExamples.devices}));
  assert.doesNotMatch(device,/升级已有资源只更新精确定义引用/);
});

test('actual authoring view explains uncommitted drafts and the immutable history boundary in Chinese', () => {
  const definition = structuredClone(fieldExamples.items.definition);
  const markup = renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace, {
    workspace, busy: false,
    initialDraft: { definition, summary: { draft_id: 'observed-native-draft', revision: 2, definition_ref: { id: definition.id, version: definition.version },
      issues: [{ code: 'definition.unsaved_changes', message: 'This editable definition is not a committed local version; commit or explicitly discard it before switching project', blocks: [] },
        { code: 'new.host.issue', message: '保留实际 Host 新检查项', blocks: [] }] },
      history: { can_undo: false, can_redo: false, boundary_reason: 'A committed immutable version/professional input is a history boundary' } },
    io: { dispatch: forbiddenIO, awaitTask: forbiddenIO, pickDirectory: forbiddenIO },
    samples: { status: 'unavailable', reason: '没有实际样例执行者' }, templates: { status: 'unavailable', reason: '没有实际组合文件写入者' },
    onDraftChanged() {}, onCommitted: forbiddenIO, onPreviewChange() {}, onConsumerRequested() {}, onClose() {},
  }));
  assert.match(markup, /此定义尚未提交为本地精确版本；切换工程前须提交，或明确放弃此草稿。/);
  assert.match(markup, /已提交的不可变版本或专业输入是编辑历史边界，不能通过草稿撤销回退。/);
  assert.match(markup, /保留实际 Host 新检查项/);
  assert.doesNotMatch(markup, /This editable definition|A committed immutable version/);
});
test('actual composition editor selects project declarations and keeps explicit seed and role requirements', () => {
  const markup = renderToStaticMarkup(React.createElement(TemplateCompositionEditor, { workspace, value: emptyComposition(), baseRevision: project.revision, onChange: forbiddenIO, disabled: false }));
  assert.match(markup, /组合/); assert.match(markup, new RegExp(project.devices[0].id)); assert.match(markup, /角色/); assert.match(markup, /供料/); assert.match(markup, /订单/);
  assert.doesNotMatch(markup, /type="checkbox" checked=""/);
});

test('adoption compares actual device presentation or inline rules, and missing originals remain unknown', () => {
  const device=structuredClone(fieldExamples.devices.definition);
  const media=structuredClone(fieldExamples.media.definition);
  media.payload.data=structuredClone(device.payload.data.presentation);
  assert.deepEqual(adoptionDifferences(device,media),[]);
  media.payload.data.scale.x+=1;
  const changes=adoptionDifferences(device,media);
  assert.ok(changes.some(change=>change.path==='/scale/x'));
  assert.ok(changes.every(change=>!change.path.includes('ports')&&!change.path.includes('behavior')));
  assert.equal(adoptionDifferences(undefined,media,null),null);
  assert.deepEqual(adoptionDifferences(undefined,media,media.payload.data),[]);
  device.payload.data.behavior={selected:'inline',inline:structuredClone(fieldExamples.operations.definition.payload.data),operation_ref:null};
  assert.deepEqual(adoptionDifferences(device,fieldExamples.operations.definition),[]);
  device.payload.data.behavior={selected:'operation_ref',operation_ref:{id:'not-read',version:'1.0.0'},inline:null};
  assert.equal(adoptionDifferences(device,fieldExamples.operations.definition),null);
});
test('role discovery preserves business mappings without treating presentation anchors as objects', () => {
  const module=structuredClone(fieldExamples.operations);
  module.definition.payload.data={item_roles:['goods'],inputs:[{item_role:'input'}],resource_roles:[{role:'worker'}],quality_and_routes:{pending_location_role:'pending-store',isolation_location_role:'isolation-store'},ports:[{id:'material-out'}],anchor_roles:{machine:{kind:'scene_node',data:5}}};
  assert.deepEqual(roleRequirements(module),{item:['goods','input'],resource:['worker'],target:['pending-store','isolation-store'],port:['material-out']});
});
test('actual native media adoption keeps its local scene anchors without requesting business object bindings', async () => {
  const module=JSON.parse(await readFile(new URL('modules/devices/template-support/media/module.json',rootUrl),'utf8')).data;
  const original=structuredClone(module);
  assert.deepEqual(roleRequirements(module),{item:[],resource:[],target:[],port:[]});
  const html=renderToStaticMarkup(React.createElement(AdoptionPreparation,{workspace,module,busy:false,onCancel(){},onApply:forbiddenIO}));
  assert.doesNotMatch(html,/对象角色 input|对象角色 output/);
  assert.match(html,/受影响的已有实例/);
  assert.deepEqual(module,original);
});

test('native preview component exposes read-only static scope and keyboard reachable camera buttons', () => {
  const markup=renderToStaticMarkup(React.createElement(ModulePresentationPreview,{presentation:fieldExamples.media.definition.payload.data}));
  assert.match(markup,/静态未提交预览/);assert.match(markup,/适配全部/);assert.match(markup,/俯视/);assert.match(markup,/正视/);assert.match(markup,/不产生库存/);
  assert.doesNotMatch(markup,/<img|<video/);
});
test('only the actual partial-success class attaches the created draft and preserves all local composition inputs', async () => {
  const definition=structuredClone(fieldExamples.templates.definition), subset=emptyComposition();
  subset.role_requirements=[{role:'explicit-role',quantity:{value:{coefficient:2,decimal_places:0},unit:'person'}}];
  const input={project_id:project.id,base_revision:project.revision,definition,draft:null,subset}; const original=structuredClone(input);
  const created={summary:{draft_id:'actual-created',revision:1,definition_ref:{id:definition.id,version:definition.version},issues:[]},definition:structuredClone(definition),history:{can_undo:true,can_redo:false,boundary_reason:null}};
  const commit={scope:{kind:'definition_draft',data:{id:'actual-created',revision:1}},new_revision:1};
  const cause=new Error('RevisionConflict from composition');cause.detail={code:'RevisionConflict',details:{expected:3,actual:4}};
  const failure=new TemplateCompositionSaveFailure(cause,created,commit);const received=[];
  await assert.rejects(saveTemplateComposition({status:'available',save:async value=>{assert.equal(value.draft,null);throw failure;}},input,(draft,event)=>received.push([draft,event])),error=>error===failure&&error.cause===cause&&error.cause.detail.details.actual===4);
  assert.deepEqual(input,original);assert.equal(received.length,1);assert.equal(received[0][0],created);assert.equal(received[0][1],commit);
  const retry={...input,draft:received[0][0]};await saveTemplateComposition({status:'available',save:async value=>{assert.equal(value.draft.summary.draft_id,'actual-created');assert.deepEqual(value.subset,original.subset);return {draft:created,commit};}},retry,()=>assert.fail('retry must not create another draft'));
  const ordinary=new Error('ordinary failure');ordinary.name='TemplateCompositionSaveFailure';
  await assert.rejects(saveTemplateComposition({status:'available',save:async()=>{throw ordinary;}},input,()=>assert.fail('only exact class may attach a partial draft')),error=>error===ordinary);
});
test('partial-success refresh failure retains original Host cause rather than replacing the composition failure', async () => {
  const cause=new Error('original Host conflict');const failure=new TemplateCompositionSaveFailure(cause,{summary:{draft_id:'D-existing'}},{new_revision:1});const refresh=new Error('refresh failed');
  await assert.rejects(saveTemplateComposition({status:'available',save:async()=>{throw failure;}},{},()=>{throw refresh;}),error=>error instanceof AggregateError&&error.errors[0]===failure&&error.errors[0].cause===cause&&error.errors[1]===refresh);
});
test('template reopen reads the actual DefinitionDraft sidecar scope and exact revision, not the local edited subset', async () => {
  const definition=fieldExamples.templates.definition;const draft={summary:{draft_id:'D-template',revision:7,definition_ref:{id:definition.id,version:definition.version},issues:[]},definition,history:null};
  const current={...workspace,context:{host:{session_id:'same-session'}}};const calls=[];const stored=emptyComposition();stored.role_requirements=[{role:'stored-in-sidecar',quantity:{value:{coefficient:1,decimal_places:0},unit:'person'}}];
  const actual=await readAuthoringTemplatePreview(()=>current,draft,null,null,{dispatch:async request=>{calls.push(request);return {kind:'template_preview',data:{module_ref:draft.summary.definition_ref,project_id:project.id,base_revision:project.revision,subset:stored,issues:[]}};}});
  assert.equal(calls.length,1);assert.equal(calls[0].operation,'get_object');assert.deepEqual(calls[0].scope,{kind:'definition_draft',data:'D-template'});assert.equal(calls[0].params.source.data.base_revision,7);assert.equal(actual.subset.role_requirements[0].role,'stored-in-sidecar');assert.equal(actual.base_revision,project.revision);
});

test('composition editor shows captured old source, real revision changes and exact unsupported preview issues without replacing them', () => {
  const captured=captureCompleteSeed(emptyComposition(),scenario);const old={scenario_id:scenario.id,revision:scenario.revision};
  const changed={...workspace,project:{...project,revision:project.revision+1},scenarios:[{...scenario,revision:scenario.revision+2}]};
  const markup=renderToStaticMarkup(React.createElement(TemplateCompositionEditor,{workspace:changed,value:captured.subset,sourceScenario:old,baseRevision:project.revision,disabled:false,onChange(){assert.fail('render cannot attach or replace source');},issues:[{code:'template.seed_context_unsupported',field_path:'/seed_context/quality_presets',message:'Actual missing inspection context',blocks:[]}]}));
  assert.match(markup,new RegExp(`${scenario.id}.*r${scenario.revision}`));assert.match(markup,/真实 Host 检查冲突/);assert.match(markup,/仅选择候选不会产生种子或来源请求字段/);assert.match(markup,/template.seed_context_unsupported/);assert.match(markup,/Actual missing inspection context/);assert.match(markup,/清除完整种子和来源/);
  assert.deepEqual(old,{scenario_id:scenario.id,revision:scenario.revision});assert.deepEqual(captured.subset.seed.orders,scenario.orders);
  const legacy=renderToStaticMarkup(React.createElement(TemplateCompositionEditor,{workspace,value:captured.subset,baseRevision:project.revision,disabled:false,onChange(){assert.fail('historical render cannot infer source');}}));
  assert.match(legacy,/历史种子缺少实际来源引用/);assert.match(legacy,/查看已捕获的声明正文（只读）/);assert.doesNotMatch(legacy,/保留的真实来源/);
});
test('partial composition failure retains captured source as well as definition/subset for same-Draft retry', async () => {
  const definition=structuredClone(fieldExamples.templates.definition),capture=captureCompleteSeed(emptyComposition(),scenario);
  const input=compositionInput(project.id,{...capture,base_revision:project.revision},definition,null),original=structuredClone(input);
  const actual={summary:{draft_id:'D-partial-seed',revision:1,definition_ref:{id:definition.id,version:definition.version},issues:[]},definition,history:null};
  const commit={scope:{kind:'definition_draft',data:{id:'D-partial-seed',revision:1}},new_revision:1};const failure=new TemplateCompositionSaveFailure(new Error('actual source revision conflict'),actual,commit);let saved;
  await assert.rejects(saveTemplateComposition({status:'available',save:async value=>{assert.deepEqual(value.source_scenario,original.source_scenario);throw failure;}},input,draft=>{saved=draft;}),error=>error===failure);
  assert.equal(saved,actual);assert.deepEqual(input,original);
  const retry=compositionInput(project.id,{...capture,base_revision:project.revision},definition,saved);assert.equal(retry.draft.summary.draft_id,'D-partial-seed');assert.deepEqual(retry.source_scenario,original.source_scenario);assert.deepEqual(retry.subset.seed,original.subset.seed);
});
test('cross-project source capture never sends a template request through the real bound Root service', async () => {
  const first={...workspace,context:{host:{session_id:'source-session'}},capabilities:{...workspace.capabilities,available_command_kinds:['put_template_composition']}};let current=first;let sent=0;
  const templates=createAuthoringTemplates(()=>current,{dispatch:async()=>{sent++;throw Error('must not be sent');}});assert.equal(templates.status,'available');
  const capture=captureCompleteSeed(emptyComposition(),scenario),input=compositionInput(project.id,{...capture,base_revision:project.revision},fieldExamples.templates.definition,null);
  current={...first,project:{...project,id:'different-project'}};
  await assert.rejects(templates.save(input),/会话或工程已切换/);assert.equal(sent,0);assert.deepEqual(input.source_scenario,{scenario_id:scenario.id,revision:scenario.revision});
});

const sampleReceipt=JSON.parse(await readFile(new URL('scenarios/自定义模块/runtime-authoring/sample-result-input.json',rootUrl),'utf8'));
const sampleDraft=revision=>({summary:{draft_id:sampleReceipt.task.task.scope.data.data.id,revision},definition:fieldExamples.devices.definition,history:null});
function sampleMarkup(task=sampleReceipt.task,run=sampleReceipt.run,extra={}) {return renderToStaticMarkup(React.createElement(DefinitionSampleResult,{task,run,readError:null,reading:false,currentDraft:sampleDraft(1),onRead(){assert.fail('SSR cannot start a read');},...extra}));}
test('actual sample business component displays existing Native receipt rows at70/110s, exact revisions, waits/resources and all real limits', async () => {
  const original=structuredClone(sampleReceipt),markup=sampleMarkup();
  assert.match(markup,/此定义样例结果/);assert.match(markup,/70000 ms/);assert.match(markup,/110000 ms/);assert.match(markup,/Task 输入修订<\/dt><dd>1/);assert.match(markup,/Run 冻结工程<\/dt><dd>[^<]+r2/);assert.match(markup,/ROOT-NATIVE-CYCLE/);
  assert.match(markup,/资源占用记录 12/);assert.match(markup,/等待记录 1/);assert.match(markup,/资源占用/);assert.match(markup,/下个已知边界 未知（未提供）/);assert.match(markup,/definition_cycle.frozen_draft_source/);assert.match(markup,/不属于正式工程运行、库存或人工认可/);
  assert.deepEqual(sampleReceipt,original);await writeFile(new URL('definition-sample-known.html',output),markup);
});
test('unknown calculation boundary/goal and incomplete tail are explicit even when original Task summary says goal_met=true', async () => {
  const partial=structuredClone(sampleReceipt.run);partial.goal_met=null;partial.computed_until_ms=null;partial.whole_window_computed=false;
  const markup=sampleMarkup(sampleReceipt.task,partial);
  assert.match(markup,/实际目标判定<\/dt><dd>未知（实际回包未给出）/);assert.match(markup,/实际计算边界<\/dt><dd>未知（未提供）/);assert.match(markup,/计算边界之后未知，不补全尾段/);assert.doesNotMatch(markup,/已达成（此样例实际回包）/);
  assert.equal(sampleReceipt.task.result.data.goal_met,true);assert.equal(partial.goal_met,null);await writeFile(new URL('definition-sample-unknown.html',output),markup);
});
test('failed/cancelled component retains complete original API and execution failure and never displays a leftover Run', () => {
  for (const state of ['failed','cancelled']) {
    const task=structuredClone(sampleReceipt.task);task.task.state=state;task.result=null;
    task.failure={code:'ComputationFailed',message:'actual task failure',object_refs:[{kind:'resource',id:'actual-resource'}],field_errors:[{pointer:'/actual/field',reason:'actual reason'}],expected_revision:1,actual_revision:2};
    task.task.failure={code:'actual_execution_failure',message:'actual execution detail',field_path:'/actual/execution',blocks:['complete_conclusion']};
    const markup=sampleMarkup(task,sampleReceipt.run);assert.match(markup,/ComputationFailed/);assert.match(markup,/actual task failure/);assert.match(markup,/actual-resource/);assert.match(markup,/actual reason/);assert.match(markup,/actual_execution_failure/);assert.doesNotMatch(markup,/实际样例 Run/);assert.doesNotMatch(markup,/重新实际读取/);
  }
});
test('readback failure shows original Host detail plus readonly retry, while Task completion and original frozen DraftR survive later edits', () => {
  const detail={code:'PermissionDenied',message:'actual readonly access failure',object_refs:[{kind:'definition_draft',id:'original-draft'}],field_errors:[{pointer:'/scope/data',reason:'actual permission detail'}],expected_revision:1,actual_revision:5};
  const markup=sampleMarkup(sampleReceipt.task,null,{readError:new HostFailure(detail),currentDraft:sampleDraft(5)});
  assert.match(markup,/任务状态<\/dt><dd>已完成/);assert.match(markup,/PermissionDenied/);assert.match(markup,/actual permission detail/);assert.match(markup,/重新实际读取此冻结样例/);assert.match(markup,/当前草稿已为 r5/);assert.match(markup,/仍属于冻结的 r1/);assert.doesNotMatch(markup,/已达成（此样例实际回包）/);
});
test('actual quantity precision and zero milliseconds survive formatting without summing or filling unknown values', () => {
  const run=structuredClone(sampleReceipt.run);run.acceptance_events[0].quantity={unit:'piece',value:{coefficient:1000000001,decimal_places:9}};run.orders[0].completion_ms=null;
  const markup=sampleMarkup(sampleReceipt.task,run);
  assert.match(markup,/1\.000000001 piece/);assert.match(markup,/0 piece/);assert.match(markup,/0 ms/);assert.match(markup,/未知（未提供）/);assert.equal(run.orders[0].completion_ms,null);
});

function entryProps(extra={}) {
  const calls=[];
  const props={workspace,busy:false,io:{dispatch:async request=>{calls.push(request);throw Error('entry render must not dispatch');},awaitTask:forbiddenIO,pickDirectory:forbiddenIO},samples:{status:'unavailable',reason:'No sample on entry'},templates:{status:'unavailable',reason:'No template mutation on entry'},onDraftChanged:value=>calls.push({callback:'draft',value}),onCommitted:value=>calls.push({callback:'commit',value}),onPreviewChange:value=>calls.push({callback:'preview',value}),onConsumerRequested:forbiddenIO,onClose:forbiddenIO,...extra};
  return {props,calls};
}
test('all nine real resumed Draft entries render original identity/body/revision with unknown library source and history, without render-time writes', () => {
  for (const category of categories) {
    const definition=structuredClone(fieldExamples[category.id].definition);definition.title=`原草稿正文-${category.id}`;
    const draft={summary:{draft_id:`original-${category.id}-draft`,revision:6,definition_ref:{id:definition.id,version:definition.version},issues:[]},definition,history:null};
    const original=structuredClone(draft),{props,calls}=entryProps({initialDraft:draft});
    const markup=renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace,props));
    assert.match(markup,/库中来源<\/dt><dd>已读取定义草稿；库中来源尚未读取<\/dd>/);
    assert.ok(markup.includes(`${draft.summary.draft_id} · r6`));assert.ok(markup.includes(`value="${definition.id}"`));assert.ok(markup.includes(`value="${definition.version}"`));assert.ok(markup.includes(`value="${definition.title}"`));
    assert.match(markup,/<button[^>]*disabled=""[^>]*>撤销草稿编辑<\/button>/);assert.match(markup,/<button[^>]*disabled=""[^>]*>重做草稿编辑<\/button>/);
    assert.doesNotMatch(markup,/端口、参数、素材与载荷变化/);assert.deepEqual(calls,[]);assert.deepEqual(draft,original);
  }
});
test('fresh and precise-version entries remain distinct, and initialDraft wins over a leftover initialModule reference', () => {
  const fresh=entryProps();const freshMarkup=renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace,fresh.props));
  assert.match(freshMarkup,/库中来源<\/dt><dd>新建定义（尚未保存到 Host）<\/dd>/);assert.match(freshMarkup,/<form/);assert.deepEqual(fresh.calls,[]);
  const reference={id:fieldExamples.devices.definition.id,version:fieldExamples.devices.definition.version};
  const pending=entryProps({initialModule:reference});const pendingMarkup=renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace,pending.props));
  assert.match(pendingMarkup,/待读取精确库版本/);assert.match(pendingMarkup,/精确库内容尚未读取/);assert.doesNotMatch(pendingMarkup,/<form/);assert.match(pendingMarkup,/<button[^>]*disabled=""[^>]*>保存并检查 · Ctrl\+S<\/button>/);assert.match(pendingMarkup,/<button[^>]*disabled=""[^>]*>进入本类实际消费者<\/button>/);assert.deepEqual(pending.calls,[]);
  const definition=structuredClone(fieldExamples.items.definition),draft={summary:{draft_id:'actual-resumed-items',revision:4,definition_ref:{id:definition.id,version:definition.version},issues:[]},definition,history:null};
  const both=entryProps({initialModule:reference,initialDraft:draft});const bothMarkup=renderToStaticMarkup(React.createElement(ModuleAuthoringWorkspace,both.props));
  assert.match(bothMarkup,/已读取定义草稿；库中来源尚未读取/);assert.match(bothMarkup,/actual-resumed-items · r4/);assert.ok(bothMarkup.includes(`value="${definition.id}"`));assert.match(bothMarkup,/当前精确库引用<\/dt><dd>尚未明确提交或选择<\/dd>/);assert.doesNotMatch(bothMarkup,/待读取精确库版本/);assert.match(bothMarkup,/<button[^>]*disabled=""[^>]*>选择实例并核对采用<\/button>/);assert.deepEqual(both.calls,[]);
});


test('device connection impact lists only selected instances with mismatched exact endpoint references', () => {
  const actual = structuredClone(project), reference = { id: 'review.device', version: '0.1.1' };
  const port = { module_ref: { id: 'review.device', version: '0.1.0' }, instance_id: 'CNC01', port_id: 'input' };
  const connection = { kind: 'material', data: { id: 'CNC01-material-input', source: { kind: 'location', data: actual.locations[0].id }, destination: { kind: 'port', data: port }, item_type: actual.item_types[0].id, unit: actual.item_types[0].unit, destination_store: actual.locations[0].id, quantity_rule: null, capacity_rule: null } };
  const other = structuredClone(connection); other.data.id = 'copy-material-input'; other.data.destination.data.instance_id = 'copy:CNC01';
  actual.connections = [connection, other]; const original = JSON.stringify(actual);
  const html = renderToStaticMarkup(React.createElement(DeviceConnectionImpact, { project: actual, reference, targets: [{ kind: 'device', id: 'CNC01' }] }));
  assert.match(html, /CNC01-material-input/); assert.match(html, /review.device@0.1.0/); assert.match(html, /review.device@0.1.1/); assert.match(html, /采用不会自动按名称重接/); assert.doesNotMatch(html, /copy-material-input/);
  assert.equal(JSON.stringify(actual), original);
  assert.equal(renderToStaticMarkup(React.createElement(DeviceConnectionImpact, { project: actual, reference: port.module_ref, targets: [{ kind: 'device', id: 'CNC01' }] })), '');
});

test('connection editor retains an obsolete exact endpoint and requires explicit rebinding with full identities', () => {
  const actual = structuredClone(project), module = structuredClone(fieldExamples.devices), reference = { id: module.definition.id, version: module.definition.version };
  actual.devices = ['CNC01', 'copy:CNC01'].map(id => ({ ...structuredClone(project.devices[0]), id, title: 'Same CNC', module_ref: reference }));
  const value = { id: 'CNC01-material-input', source: { kind: 'location', data: actual.locations[0].id }, destination: { kind: 'port', data: { module_ref: { id: 'old.device', version: '0.0.1' }, instance_id: 'CNC01', port_id: 'input' } }, item_type: actual.item_types[0].id, unit: actual.item_types[0].unit, destination_store: actual.locations[0].id, quantity_rule: null, capacity_rule: null };
  const original = JSON.stringify(value);
  const html = renderToStaticMarkup(React.createElement(ConnectionEditor, { project: actual, modules: new Map([[refKey(reference), module]]), value, busy: false, onApply: forbiddenIO, onClose: forbiddenIO }));
  assert.match(html, /原目的端点 CNC01 \/ input · old.device@0.0.1/); assert.match(html, /请明确重新选择/);
  for (const id of ['CNC01', 'copy:CNC01']) for (const port of module.definition.payload.data.ports) assert.ok(html.includes(`Same CNC · ${id} / ${port.id} · ${reference.id}@${reference.version}`));
  assert.equal(JSON.stringify(value), original);
});

test('authoring admission failure keeps complete Host field reasons, object identities and revisions before a Task exists', () => {
  const detail = { code: 'InvalidInput', message: 'Frozen run inputs did not pass execution admission', field_errors: [{ pointer: '/inputs/raw/input', reason: 'simulation.material_location: Exactly one explicit material connection is required.' }, { pointer: '/rows/0/quantity', reason: 'simulation.bom_operation_quantity: Explicit material ratio required.' }], object_refs: [{ kind: 'task', id: 'TASK-LOCAL' }, { kind: 'bom', id: 'BOM-LOCAL' }], expected_revision: null, actual_revision: null };
  const html = renderToStaticMarkup(React.createElement(AuthoringFailure, { value: new HostFailure(detail) }));
  for (const field of detail.field_errors) { assert.ok(html.includes(field.pointer)); assert.ok(html.includes(field.reason)); }
  for (const object of detail.object_refs) assert.ok(html.includes(object.id));
  assert.match(html, /InvalidInput/); assert.match(html, /未提供/); assert.doesNotMatch(html, /实际样例 Run|已完成/);
  assert.match(renderToStaticMarkup(React.createElement(AuthoringFailure, { value: new Error('Actual local error') })), /Actual local error/);
});
