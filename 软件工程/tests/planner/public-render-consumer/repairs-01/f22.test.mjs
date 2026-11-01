import {readFile}from'node:fs/promises';import test from'node:test';import assert from'node:assert/strict';
import {actualGetter}from'./port-guard.test.mjs';
import {readActualPortOutputs}from'../../../../scenarios/Agent共建/runtime-public-render-consumer/repairs-01/compiled/source.mjs';
const root=new URL('../../../../',import.meta.url),index=JSON.parse(await readFile(new URL('scenarios/Agent共建/runtime-public-render-port-unavailable/producer/actual-inputs-index.json',root),'utf8'));
async function real(){const value=await actualGetter(index.captures[1].directory),device=value.task.input.project.devices.find(v=>v.id==='CNC'),definition=value.task.input.modules.find(v=>v.definition.id===device.module_ref.id&&v.definition.version===device.module_ref.version).definition;
 value.project=async(id='CNC')=>readActualPortOutputs({project_id:value.task.frame.project_id,scope:{kind:'project'},scope_revision:value.task.frame.scope_revision},{run_id:value.task.input.run.id,at_ms:value.task.frame.request.view.view_time_ms,device:value.task.input.project.devices.find(v=>v.id===id),ports:definition.payload.data.ports},{readPortOutput:value.read},()=>{});return value;}
test('real F22 exact PortRef forwards original full ApiError; shared projection downgrades only that quantity while other true quantities stay unchanged',async()=>{
 const value=await real(),[reference,error]=value.task.input.port_output_failures[0];await assert.rejects(value.read(value.request(reference)),actual=>actual.detail===error);
 const frame=await value.project(),row=frame.ports.find(v=>v.port.port_id==='output');assert.equal(row.status,'unavailable');assert.equal(row.reason,'unsupported_capability');assert.equal(row.message,error.message);assert.equal(row.unit,'piece');assert.ok(!('completed_output'in row));
 const normal=await value.project('CNC-NORMAL'),q=normal.ports.find(v=>v.port.port_id==='output'),original=value.task.input.port_output_summaries[0];assert.deepEqual(q.completed_output,original.completed_output);assert.deepEqual(q.accepted_output,original.accepted_output);
 for(const patch of [{scope_revision:99},{scope:{kind:'definition_draft',data:'wrong'}},{run_id:'wrong'},{at_ms:100001}])await assert.rejects(value.read({...value.request(reference),...patch}),actual=>!actual.detail);
});
test('missing exact failure/changed PortRef does not turn global unavailable issue into a fallback for another source',async()=>{
 for(const change of [v=>delete v.task.input.port_output_failures,v=>v.task.input.port_output_failures[0][0].module_ref.version='9.9.9',v=>v.task.input.port_output_failures[0][0].instance_id='CNC-NORMAL',v=>v.task.input.port_output_failures[0][0].port_id='other']){const value=await real();change(value);await assert.rejects(value.project(),actual=>!actual.detail);}
});
test('permission revision invalid-input failures and wrong quantity units keep original hard rejection instead of degrading media data',async()=>{
 for(const code of ['PermissionDenied','RevisionConflict','InvalidInput']){const value=await real();value.task.input.port_output_failures[0][1]={...value.task.input.port_output_failures[0][1],code};await assert.rejects(value.project(),actual=>actual.detail?.code===code);}
 const value=await real();value.task.input.port_output_summaries[0].completed_output.unit='kg';await assert.rejects(value.project('CNC-NORMAL'),/单位/);
});
