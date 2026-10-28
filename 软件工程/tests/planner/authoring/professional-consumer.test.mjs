import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {source,rootUrl} from './load-source.mjs';
await source('modules','formSchema');
const model=await source('modules','model');
const workbench=await readFile(new URL('apps/planner/src/features/factory/Workbench.tsx',rootUrl),'utf8');
const plan=JSON.parse(await readFile(new URL('modules/devices/template-support/professional/module.json',rootUrl),'utf8')).data;
const paper=JSON.parse(await readFile(new URL('modules/professional/paper-packaging/module.json',rootUrl),'utf8')).data;
const ref=pkg=>({id:pkg.definition.id,version:pkg.definition.version});
function actualFunction(name,end,dependencies){
  const begin=workbench.indexOf(`  async function ${name}(`),stop=workbench.indexOf(end,begin);
  assert.ok(begin>=0&&stop>begin);
  const code=stripTypeScriptTypes(workbench.slice(begin,stop),{mode:'transform'}),names=Object.keys(dependencies);
  return new Function(...names,code+`\nreturn ${name};`)(...names.map(name=>dependencies[name]));
}
function consumer(pkg,adopted=true){
  const calls=[],project={id:'consumer-project'},next={project,modules:new Map(adopted?[[model.refKey(ref(pkg)),pkg]]:[])},deps={
    project,refreshLatest:{current:async()=>next},host:{moduleKey:model.refKey},assertPackagingDefinition:model.assertPackagingDefinition,
    authoringIO:{dispatch:async request=>{calls.push(request);return {kind:'module',data:pkg};}},
    openPackaging:async()=>calls.push('paper-workspace'),closeAuthoring(){},setLibraryModule(){},setLibraryOpen(){},setEditor(){},setBusinessEditIntent(){},setBusinessOpen(){},setMessage(){},setAuthoringPresentation(){},
  };
  return {calls,invoke:actualFunction('openAuthoringConsumer','\n  async function action(',deps)};
}
test('planned machining exact definition cannot open the unrelated paper packaging consumer',async()=>{
  const value=consumer(plan);
  await assert.rejects(value.invoke('professional',ref(plan)),/尚无对应.*执行消费者/);
  assert.ok(!value.calls.includes('paper-workspace'));
});
test('an imported professional definition still requires explicit adoption into this Project',async()=>{
  const value=consumer(paper,false);
  await assert.rejects(value.invoke('professional',ref(paper)),/尚未采用到工程/);
  assert.ok(!value.calls.includes('paper-workspace'));
});
test('the adopted exact paper definition opens the real paper input consumer without executing it',async()=>{
  const value=consumer(paper);
  await value.invoke('professional',ref(paper));
  assert.equal(value.calls.filter(x=>x==='paper-workspace').length,1);
  const read=value.calls.find(x=>x.operation==='get_object');
  assert.deepEqual(read.params.source,{kind:'module_version',data:{module_ref:ref(paper)}});
  assert.equal(read.params.object_id.id,paper.definition.id);
});
function packaging(definition){
  const calls=[],project={id:'consumer-project'},deps={refreshLatest:{current:async()=>({project})},authoringIO:{},readDraft:async()=>({definition}),assertPackagingDefinition:model.assertPackagingDefinition,
    readPackagingContext:async()=>{calls.push('context');return {};},closeAuthoring:()=>calls.push('close-authoring'),setDeliveryTab(){},setLibraryOpen(){},setPackagingContext(){},setPackagingScope:scope=>calls.push(scope),setSource(){}};
  return {calls,invoke:actualFunction('openPackaging','\n  async function refreshPackaging',deps)};
}
test('the professional Draft menu cannot route a planned definition into paper calculation',async()=>{
  const value=packaging(plan.definition);
  await assert.rejects(value.invoke({kind:'definition_draft',data:'planned-draft'}),/尚无对应.*执行消费者/);
  assert.deepEqual(value.calls,[]);
});
test('paper DefinitionDraft keeps its exact scope and direct Project paper input remains available',async()=>{
  for(const scope of [{kind:'definition_draft',data:'paper-draft'},{kind:'project'}]){
    const value=packaging(paper.definition);await value.invoke(scope);
    assert.ok(value.calls.includes('context'));assert.deepEqual(value.calls.at(-1),scope);
  }
});
