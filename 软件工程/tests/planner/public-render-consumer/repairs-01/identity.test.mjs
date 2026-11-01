import {readFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import test from 'node:test';
import assert from 'node:assert/strict';
import {prepareRenderTask} from '../../../../scenarios/Agent共建/runtime-public-render-consumer/repairs-01/compiled/source.mjs';
const root=new URL('../../../../',import.meta.url),json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
const raw=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/renderTaskSource.ts',root),'utf8'));
// Execute the actual private predicate, rather than assert its spelling or export test-only API.
const equal=Function(raw.slice(raw.indexOf('const same='),raw.indexOf('const sameRef='))+';return same;')();
const reverse=value=>Array.isArray(value)?value.map(reverse):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).reverse().map(key=>[key,reverse(value[key])])):value;
test('actual frozen-value predicate ignores nested object member order, retaining every array position and field',()=>{
 const value={kind:'test',items:[{kind:'asset',id:'a'},{id:'b',values:[1,null,false,'x']}],source:{run:'r1',at_ms:75000}};
 assert.equal(equal(value,reverse(value)),true);
 for(const next of [{...value,items:[...value.items].reverse()},{...value,items:[...value.items,3]},{...value,source:{run:'r1'}},{...value,source:{...value.source,extra:null}},{...value,source:{...value.source,at_ms:'75000'}},{...value,source:{...value.source,at_ms:75001}},null,[]])assert.equal(equal(value,next),false);
 assert.equal(equal([1,2],{'0':1,'1':2}),false);assert.equal(equal({a:undefined},{}),false);assert.equal(equal(null,{}),false);
});
async function actual(id){const index=await json('scenarios/Agent共建/runtime-public-render-host/actual-inputs-index.json'),directory=index.captures[id-1].directory,task={frame:await json(directory+'/frame.json'),input:await json(directory+'/input.json')},calls=[];
 const ports={asset:async request=>{calls.push(request);throw Error('negative identity test must not request any bytes');}};
 return {task,calls,value:await prepareRenderTask(task,ports,()=>{})};
}
test('actual private Draft full source guard rejects real value changes before Native while reordered exact dependencies succeed',async()=>{
 const {task,value,calls}=await actual(36),media=value.media,request={...media.binding.request,asset:task.input.assets.find(entry=>entry.asset.kind==='glb').asset};
 for(const patch of [{scope:{kind:'project'}},{scope_revision:99},{project_id:'other'},{source:{...request.source,data:{...request.source.data,module_ref:{...request.source.data.module_ref,version:'9.9.9'}}}}])await assert.rejects(media.readAsset({...request,...patch}));
 for(const asset of [{...request.asset,id:'other'},{...request.asset,path:'other.glb'},{...request.asset,kind:'png'},{...request.asset,extra:null}])await assert.rejects(media.readAsset({...request,asset}));
 const dependency=task.input.assets.find(entry=>entry.source.data.module_ref.id!=='inspector.glb').source;
 const module=await media.resolveModule(reverse(dependency));assert.equal(module.definition.id,dependency.data.module_ref.id);assert.equal(module.definition.version,dependency.data.module_ref.version);assert.deepEqual(calls,[]);
});
test('actual old Run asset reader rejects altered Run id or observation time instead of following current metadata',async()=>{
 const {task,value,calls}=await actual(28),media=value.media,asset=task.input.assets.find(entry=>entry.asset.kind==='glb').asset,source=media.binding.request.source;
 for(const patch of [{run_id:'other-run'},{at_ms:source.data.at_ms+1}])await assert.rejects(media.readAsset({...media.binding.request,source:{...source,data:{...source.data,...patch}},asset}));
 assert.deepEqual(calls,[]);
});
