import {readFile}from'node:fs/promises';import{stripTypeScriptTypes}from'node:module';import test from'node:test';import assert from'node:assert/strict';
const root=new URL('../../../../',import.meta.url),json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
const source=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/renderTaskSource.ts',root),'utf8'));
const predicate=source.slice(source.indexOf('const same='),source.indexOf('const sameRef=')),reader=source.slice(source.indexOf('const readPortOutput='),source.indexOf('const factory='));
const create=Function('context','input','scope','assertCurrent','ProcessUnavailable',predicate+';'+reader+';return readPortOutput;');
class ProcessUnavailable extends Error{}
const reverse=v=>Array.isArray(v)?v.map(reverse):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).reverse().map(k=>[k,reverse(v[k])])):v;
export async function actualGetter(directory){const task={frame:await json(directory+'/frame.json'),input:await json(directory+'/input.json')},scope=task.frame.scope.kind==='project'?{kind:'project'}:{kind:'definition_draft',data:task.frame.scope.data.id},context={project_id:task.frame.project_id,scope,scope_revision:task.frame.scope_revision,view:{run_id:task.frame.request.view.run_id,at_ms:task.frame.request.view.view_time_ms}};
 let live=true;const read=create(context,task.input,scope,()=>{if(!live)throw Error('actual context closed');},ProcessUnavailable);
 const request=port=>({project_id:context.project_id,scope,scope_revision:context.scope_revision,run_id:context.view.run_id,at_ms:context.view.at_ms,port});return {task,read,request,close:()=>live=false};
}
test('actual frozen port reader checks complete Scope/Run/time before returning a valid unchanged Rust summary',async()=>{
 const index=await json('scenarios/Agent共建/runtime-public-render-host/actual-inputs-index.json'),value=await actualGetter(index.captures[40].directory),summary=value.task.input.port_output_summaries[0],request=value.request(summary.port);
 assert.equal(await value.read(reverse(request)),summary);
 for(const patch of [{project_id:'other'},{scope_revision:request.scope_revision+1},{scope:{kind:'definition_draft',data:'other'}},{run_id:'other-run'},{at_ms:request.at_ms+1}])await assert.rejects(value.read({...request,...patch}),/工程|Scope/);
 await assert.rejects(value.read({...request,port:{...request.port,module_ref:{...request.port.module_ref,version:'9.9.9'}}}),/没有此精确/);
 value.close();await assert.rejects(value.read(request),/context closed/);
});
