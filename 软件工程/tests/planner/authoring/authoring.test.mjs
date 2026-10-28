import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { Matrix4, Quaternion, Vector3 } from 'three';
import { source, rootUrl } from './load-source.mjs';
await source('modules','formSchema');
const model = await source('modules','model');
const examples = (await source('modules','fieldExamples')).fieldExamples;
const parts = await source('device-authoring','parts');
const templates = await source('modules','templateComposition');
const imports = await source('modules','importAssets');
test('source text distinguishes a saved unread-source Draft, a selected unread version, a real old source and a fresh definition', () => {
  const definition=examples.devices.definition;
  const draft={summary:{draft_id:'real-existing-draft',revision:3,definition_ref:{id:definition.id,version:definition.version},issues:[]},definition,history:null};
  assert.equal(model.definitionSourceText(draft,null,null),'已读取定义草稿；库中来源尚未读取');
  assert.match(model.definitionSourceText(null,null,{id:'exact-selected',version:'1.2.3'}),/exact-selected @ 1\.2\.3.*尚未读取/);
  const old={...structuredClone(definition),version:'0.9.0'};
  assert.equal(model.definitionSourceText(draft,old,null),`${old.id} @ 0.9.0`);
  assert.equal(model.definitionSourceText(null,null,null),'新建定义（尚未保存到 Host）');
  assert.equal(draft.history,null);assert.equal(draft.summary.revision,3);
});

test('all nine canonical payloads have independent forms and examples without formal runtime state', () => {
  assert.equal(model.categories.length,9);
  for (const category of model.categories) {
    const definition=examples[category.id].definition;
    assert.equal(definition.payload.category,category.id);
    assert.deepEqual(model.definitionIssues(definition),[],category.id);
    assert.ok(model.payloadSchema(category.id));
    assert.equal(model.newDefinition(category.id).payload.category,category.id);
    for (const key of ['current_stock','current_occupation','run_result','final_items']) assert.equal(Object.hasOwn(definition.payload.data,key),false);
  }
});
test('changing pose length unit preserves physical translation without changing geometry scale or rotation', () => {
  const pose={...parts.identityPose('millimeter','right_handed_z_up'),translation:{x:500,y:-250,z:1000}};
  const meters=parts.convertPoseUnit(pose,'meter');
  assert.deepEqual(meters.translation,{x:.5,y:-.25,z:1});
  close(transform(pose),transform(meters));
  assert.deepEqual(parts.convertPoseUnit(meters,'millimeter'),pose);
});
test('template composition copies only selected declarations and optional supply/orders are independent', async () => {
  const project=JSON.parse(await readFile(new URL('tools/type-contract-proof/fixtures/project.json',rootUrl),'utf8')).data;
  const scenario=JSON.parse(await readFile(new URL('tools/type-contract-proof/fixtures/scenario.json',rootUrl),'utf8')).data;
  const subset=templates.selectCompositionObjects(templates.emptyComposition(),project,'devices',[project.devices[0].id]);
  assert.equal(subset.devices.length,1); assert.ok(subset.module_refs.some(ref=>model.refKey(ref)===model.refKey(project.devices[0].module_ref)));
  assert.equal(subset.seed,null);assert.equal('acknowledgments' in subset,false);assert.equal('views' in subset,false);
  subset.devices[0].title='changed draft'; assert.notEqual(project.devices[0].title,'changed draft');
  assert.equal(templates.declaredSeed(scenario,false,false),null);
  const orders=templates.declaredSeed(scenario,false,true); assert.deepEqual(orders.initial_conditions.items,[]);assert.deepEqual(orders.arrivals,[]);assert.deepEqual(orders.orders,scenario.orders);
  const supply=templates.declaredSeed(scenario,true,false);assert.deepEqual(supply.orders,[]);assert.deepEqual(supply.initial_conditions,scenario.initial_conditions);
});
test('editing a read-back template never resubmits Host-generated adoption claims, including a partially created Draft retry', () => {
  const subset=templates.emptyComposition();
  subset.adoption={source_archive:{path:'actual-committed-source.json',kind:'project'},source_project_id:'P1',source_project_revision:12};
  const original=structuredClone(subset),state={subset,base_revision:12,source_scenario:null};
  for(const draft of [null,{summary:{draft_id:'actual-partial-draft',revision:1},definition:examples.templates.definition}]) {
    const request=templates.compositionInput('P1',state,examples.templates.definition,draft);
    assert.equal(request.subset.adoption??null,null);assert.deepEqual(request.subset,{...original,adoption:null});
    assert.equal(request.base_revision,12);assert.equal(request.draft,draft);assert.equal(Object.hasOwn(request,'source_scenario'),false);
  }
  assert.deepEqual(subset,original);
});

test('selected composition resources retain actual capacity and calendar declarations without instance or runtime state', async () => {
  const project=JSON.parse(await readFile(new URL('tools/type-contract-proof/fixtures/project.json',rootUrl),'utf8')).data;
  const actual=project.resources[0],calendar=project.calendars.find(value=>value.id===actual.calendar_id);
  const subset=templates.emptyComposition();subset.devices=[{...project.devices[0],role_bindings:{item_types:{},resources:{first:actual.id,second:actual.id},targets:{},ports:{}}}];
  const before=structuredClone(project),rows=templates.compositionResources(subset,project);
  assert.equal(rows.length,1);assert.equal(rows[0].source.id,actual.id);
  assert.deepEqual(rows[0].requirement,{kind:actual.kind,role:actual.role,capacity:actual.capacity,qualifications:actual.qualifications,default_calendar_ms:calendar.intervals,presentation:actual.presentation});
  for(const key of ['id','definition_ref','calendar_id','home_location','current_occupation']) assert.equal(Object.hasOwn(rows[0].requirement,key),false);
  rows[0].requirement.capacity.value=99;rows[0].requirement.default_calendar_ms.length=0;
  assert.deepEqual(project,before);assert.deepEqual(subset.resource_requirements,[]);assert.equal(subset.seed,null);
  assert.deepEqual(templates.compositionResources(templates.emptyComposition(),project),[]);
});

test('explicit full-seed resource capture includes its exact source supply carrier, never a merely selected or newer scenario', async () => {
  const project=JSON.parse(await readFile(new URL('tools/type-contract-proof/fixtures/project.json',rootUrl),'utf8')).data;
  const scenario=JSON.parse(await readFile(new URL('tools/type-contract-proof/fixtures/scenario.json',rootUrl),'utf8')).data;
  const original=project.resources[0],carrier={...structuredClone(original),id:'seed-carrier',kind:'vehicle',role:'carrier'};
  project.resources.push(carrier);
  const source={...scenario,id:'captured-source',revision:12,supplies:[{resources:{carrier:carrier.id}}]},reference={scenario_id:source.id,revision:12};
  const subset=templates.emptyComposition();subset.devices=[{...project.devices[0],role_bindings:{item_types:{},resources:{capacity:original.id},targets:{},ports:{}}}];
  assert.deepEqual(templates.compositionResources(subset,project,source,reference).map(row=>row.source.id),[original.id]);
  const captured=templates.captureCompleteSeed(subset,source),before=structuredClone({project,source,captured});
  const rows=templates.compositionResources(captured.subset,project,source,captured.source_scenario);
  assert.deepEqual(rows.map(row=>row.source.id),[original.id,carrier.id]);
  assert.equal(rows[1].requirement.role,'carrier');assert.equal(Object.hasOwn(rows[1].requirement,'home_location'),false);
  assert.deepEqual(templates.compositionResources(captured.subset,project,{...source,revision:13},reference).map(row=>row.source.id),[original.id]);
  assert.deepEqual(templates.compositionResources(captured.subset,project,{...source,id:'other-source'},reference).map(row=>row.source.id),[original.id]);
  assert.deepEqual({project,source,captured},before);
});

test('import asset choices come only from selected exact module versions and retain missing rights as unknown', () => {
  const module=structuredClone(examples.media);module.assets=[{id:'real-asset',path:'media/actual.glb',kind:'glb'}];module.rights=[];
  const draft={proposed_data:[{kind:'module',data:module}]};
  assert.deepEqual(imports.importAssetChoices(draft,[]),[]);
  assert.deepEqual(imports.importAssetChoices(draft,[{id:module.definition.id,version:'unselected'}]),[]);
  const choices=imports.importAssetChoices(draft,[module.definition]);assert.equal(choices[0].asset.id,'real-asset');assert.equal(choices[0].rights,undefined);
});
test('nullable branches cannot hide invalid form values, nonfinite numbers or missing IDs', () => {
  const definition=structuredClone(examples.devices.definition);
  definition.payload.data.presentation.scale.x=NaN;
  assert.ok(model.definitionIssues(definition).some(issue=>issue.field_path.endsWith('/scale/x')));
  const empty=model.newDefinition('items');
  assert.ok(model.definitionIssues(empty).some(issue=>issue.field_path==='/id'));
  assert.ok(model.shapeIssues({anyOf:[{type:'number'},{type:'null'}]},'not a number').length);
  assert.deepEqual(model.shapeIssues({anyOf:[{type:'number'},{type:'null'}]},null),[]);
});
test('device behavior cannot carry both inline and referenced rules', () => {
  const definition=structuredClone(examples.devices.definition);
  definition.payload.data.behavior={selected:'inline',inline:examples.operations.definition.payload.data,operation_ref:{id:'actual.operation',version:'1.0.0'}};
  assert.ok(model.definitionIssues(definition).some(issue=>issue.message.includes('一处')));
});
test('version review exposes changed ports and parameter leaves rather than equal array lengths', () => {
  const before=structuredClone(examples.devices.definition), after=structuredClone(before);
  after.payload.data.ports[0].direction='bidirectional';
  const changes=model.differences(before,after);
  assert.ok(changes.some(change=>change.path.endsWith('/direction')&&change.after==='bidirectional'));
  assert.deepEqual(before,examples.devices.definition);
});
function transform(pose) {const unit=pose.unit==='meter'?1:.001;return new Matrix4().compose(new Vector3(pose.translation.x*unit,pose.translation.y*unit,pose.translation.z*unit),new Quaternion(pose.rotation.x,pose.rotation.y,pose.rotation.z,pose.rotation.w),new Vector3(pose.scale.x,pose.scale.y,pose.scale.z));}
function close(left,right) {left.elements.forEach((value,index)=>assert.ok(Math.abs(value-right.elements[index])<1e-9,`${index}: ${value} vs ${right.elements[index]}`));}
test('group and ungroup preserve declared world pose; selected parent movement is applied once', () => {
  const presentation=model.newPresentation();const a=parts.newPart(presentation,'box'), b=parts.newPart({...presentation,nodes:[a]},'cylinder');
  a.transform.translation.x=100;b.transform.translation.x=350;b.transform.translation.y=200;
  const original=[a,b], grouped=parts.groupParts(original,[0,1]);
  close(transform(a.transform).multiply(transform(grouped[1].transform)),transform(b.transform));
  const ungrouped=parts.ungroupParts(grouped,0);close(transform(ungrouped[1].transform),transform(b.transform));
  const moved=parts.moveParts(grouped,[0,1],{x:50,y:0,z:0});assert.equal(moved[0].transform.translation.x,150);assert.equal(moved[1].transform.translation.x,grouped[1].transform.translation.x);
});
test('copy uses new declared IDs and internal parent links, without copying actions or bindings', () => {
  const presentation=model.newPresentation();const a=parts.newPart(presentation,'box');const b=parts.newPart({...presentation,nodes:[a]},'plane');b.parent_index=0;
  const copied=parts.copyParts([a,b],[0,1]);assert.equal(new Set(copied.map(node=>node.id)).size,4);assert.equal(copied[3].parent_index,2);assert.deepEqual(copied[2].transform,a.transform);
  presentation.nodes=[a,b];presentation.actions=[{id:'motion',node_index:1,keys:[],transition:'linear'}];assert.ok(parts.removalDependencies(presentation,0).some(use=>use.includes('motion')));
});
test('grouping refuses mixed coordinate declarations and nonrepresentable shear', () => {
  const presentation=model.newPresentation();const a=parts.newPart(presentation,'box'),b=parts.newPart({...presentation,nodes:[a]},'box');
  b.transform.unit='meter';assert.throws(()=>parts.groupParts([a,b],[0,1]),/单位/);b.transform.unit=a.transform.unit;
  a.transform.scale={x:2,y:1,z:1};const q=new Quaternion().setFromAxisAngle(new Vector3(0,0,1),Math.PI/4);b.transform.rotation={x:q.x,y:q.y,z:q.z,w:q.w};
  assert.throws(()=>parts.groupParts([a,b],[0,1]),/剪切/);
});
test('typed requests retain explicit project/definition scopes and caller fields never appear', () => {
  const request=model.authoringRequest('P1',{kind:'definition_draft',data:'D1'},'apply_commands',{base_revision:7,commands:[{kind:'undo'}]});
  assert.equal(request.params.base_revision,7);assert.deepEqual(request.scope,{kind:'definition_draft',data:'D1'});assert.equal(request.project_id,'P1');assert.equal('caller' in request,false);assert.equal('grants' in request,false);
});
test('actual definition read uses Host summaries and cannot fabricate a missing history version', async () => {
  const definition=examples.items.definition;const calls=[];
  const context={definition:{draft_id:'D1',revision:3,definition_ref:{id:definition.id,version:definition.version},issues:[]}};
  const io={dispatch:async request=>{calls.push(request);return request.operation==='get_object'?{kind:'definition',data:definition}:context;}};
  const value=await model.readDraft(io,'P1','D1');assert.equal(value.summary.revision,3);assert.equal(value.history,null);assert.equal('edit_history_version' in value,false);assert.equal(calls.length,3);
  const history={can_undo:true,can_redo:true,boundary_reason:null};assert.deepEqual((await model.readDraft(io,'P1','D1',history,3)).history,history);assert.equal((await model.readDraft(io,'P1','D1',history,2)).history,null);
  let count=0;await assert.rejects(model.readDraft({dispatch:async request=>request.operation==='get_object'?{kind:'definition',data:definition}:{definition:{...context.definition,revision:++count}}},'P1','D1'),/读取期间/);
});

test('reopening a definition uses history from the current Host summary instead of an old commit', async () => {
  const definition=examples.items.definition;
  const history={can_undo:true,can_redo:false,boundary_reason:null};
  const context={definition:{draft_id:'D1',revision:3,definition_ref:{id:definition.id,version:definition.version},issues:[],history}};
  const io={dispatch:async request=>request.operation==='get_object'?{kind:'definition',data:definition}:context};
  assert.deepEqual((await model.readDraft(io,'P1','D1')).history,history);
  assert.deepEqual((await model.readDraft(io,'P1','D1',{can_undo:false,can_redo:true,boundary_reason:null},2)).history,history);
});
test('original package and confirmed four steps remain intact in the execution receipt', async()=>{
 const receipt=JSON.parse(await readFile(new URL('docs/04_开发与验收/整包派发/P3-01_整包执行.json',rootUrl),'utf8'));
 assert.equal(receipt.source_task_snapshot.execution_plan.steps.length,4);assert.equal(receipt.whole_task_done,false);assert.equal(receipt.human_acceptance,null);assert.equal(receipt.scope.business_frontend_changes,true);
});

test('overall movement is shared in world coordinates under differently rotated and scaled parents', () => {
  const presentation=model.newPresentation();
  const nodes=[parts.newPart(presentation,'box'),parts.newPart(presentation,'box'),parts.newPart(presentation,'box'),parts.newPart(presentation,'box')];
  nodes.forEach((node,index)=>{node.id=index+1;node.transform=parts.identityPose('millimeter','right_handed_z_up');});
  const quarter=new Quaternion().setFromAxisAngle(new Vector3(0,0,1),Math.PI/2);
  nodes[0].transform.rotation={x:quarter.x,y:quarter.y,z:quarter.z,w:quarter.w};nodes[0].transform.scale={x:2,y:3,z:1};
  nodes[1].parent_index=0;nodes[1].transform.translation={x:100,y:200,z:0};
  nodes[3].parent_index=2;nodes[3].transform.translation={x:100,y:200,z:0};
  const moved=parts.moveParts(nodes,[1,3],{x:250,y:-100,z:50});
  for(const index of [1,3]) {
    const parent=nodes[index].parent_index;
    const before=new Vector3().setFromMatrixPosition(transform(nodes[parent].transform).multiply(transform(nodes[index].transform)));
    const after=new Vector3().setFromMatrixPosition(transform(moved[parent].transform).multiply(transform(moved[index].transform)));
    assert.ok(after.sub(before).distanceTo(new Vector3(.25,-.1,.05))<1e-9);
  }
  assert.deepEqual(moved[0],nodes[0]);assert.deepEqual(moved[2],nodes[2]);
});
