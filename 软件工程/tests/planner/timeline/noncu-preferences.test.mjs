import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';import {stripTypeScriptTypes} from 'node:module';import vm from 'node:vm';
import {source} from './load-source.mjs';await source('time');const {displayGroups}=await source('projection');
const raw=readFileSync(new URL('../../../apps/planner/src/features/timeline/preferences.ts',import.meta.url),'utf8');
const code=stripTypeScriptTypes(raw).replace(/^import .*;\r?\n/gm,'').replaceAll('export function','function')+'\nmodule.exports={useViewPreferences,initialPreferences,preferenceKey};';
const project=JSON.parse(readFileSync(new URL('../../../.work/delivery/host/baseline.actual.json',import.meta.url))).project;
function hook(storage,props={project,run:null}){
 const states=[];let index=0;const module={exports:{}};
 vm.runInNewContext(code,{displayGroups,localStorage:storage,module,useState(initial){const i=index++;if(!(i in states))states[i]=typeof initial==='function'?initial():initial;return [states[i],v=>{states[i]=typeof v==='function'?v(states[i]):v;}];}});
 return {render(){index=0;return module.exports.useViewPreferences(props,module.exports.preferenceKey(project.id,null));},api:module.exports};
}
const normalize=v=>JSON.parse(JSON.stringify(v));
test('default private preferences collapse groups rather than opening permanent per-device tracks',()=>{
 const h=hook({getItem:()=>null}),state=h.render();
 assert.deepEqual(normalize(state.value.collapsed_group_ids),displayGroups(project,normalize(state.value)).map(g=>g.id));
 assert.deepEqual(normalize(state.value.independent_device_ids),[]);
});
test('view settings save only the private V1 value, renders do not persist production time or inventory',()=>{
 const writes=[],storage={getItem:()=>null,setItem:(...v)=>writes.push(v)},h=hook(storage),before=JSON.stringify(project),state=h.render();
 state.update({...state.value,row_scroll:99,snap_enabled:false});h.render();h.render();
 assert.equal(writes.length,1);assert.equal(writes[0][0],'superfactory.timeline.view.v1:P1:draft');
 const saved=JSON.parse(writes[0][1]);assert.equal(saved.row_scroll,99);assert.equal(saved.snap_enabled,false);
 assert.equal(saved.run,undefined);assert.equal(saved.view_time_ms,undefined);assert.equal(saved.items,undefined);assert.equal(JSON.stringify(project),before);
});
test('storage failure retains session edits, declares not saved, and does not automatically retry',()=>{
 let attempts=0;const h=hook({getItem:()=>null,setItem(){attempts++;throw Error('controlled unavailable storage');}}),state=h.render();
 state.update({...state.value,row_scroll:123});const next=h.render();assert.equal(next.value.row_scroll,123);assert.equal(next.storageError,'视图偏好未保存');
 next.update({...next.value,row_scroll:456});const last=h.render();assert.equal(last.value.row_scroll,456);assert.equal(attempts,1);
});
test('readonly render tasks neither read nor write private storage; controlled owner callback is used directly',()=>{
 const forbidden={getItem(){throw Error('unexpected read');},setItem(){throw Error('unexpected write');}},h=hook(forbidden,{project,run:null,readonly:true}),state=h.render();state.update({...state.value,row_scroll:5});assert.equal(h.render().value.row_scroll,0);
 const calls=[],owner=hook(forbidden,{project,run:null,onPreferencesChange:v=>calls.push(v)});
 // Initial storage read is explicitly reported unavailable; controlled settings
 // still go to the existing owning callback without a storage retry.
 const value=owner.render();value.update({...value.value,row_scroll:5});assert.equal(calls.length,1);assert.equal(calls[0].row_scroll,5);
});
