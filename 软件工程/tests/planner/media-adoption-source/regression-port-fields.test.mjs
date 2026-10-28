import {test} from 'node:test';import assert from 'node:assert/strict';
import {fields,phase,setup,json} from './regression-port-source.mjs';
const original=['job.state','job.phase_ref','job.effective_progress_ms','run.computed_until_ms','run.computation_status'];
async function frame(){const s=setup(),actual=await s.reader.read(s.context),point=phase.projectPhase(actual.frame);return {s,frame:actual.frame,point,context:{mode:'actual_run',frame:actual.frame,point}};}
test('five original fields remain exact and dynamic presets use only the real frozen output port and original unit',async()=>{
 const value=await frame(),catalog=fields.mediaDataFields(value.context);assert.deepEqual(fields.MEDIA_DATA_FIELD_CATALOG.map(f=>f.binding.source_field),original);assert.equal(fields.MEDIA_DATA_FIELD_CATALOG.length,5);assert.equal(catalog.length,7);assert.deepEqual(catalog.slice(0,5),fields.MEDIA_DATA_FIELD_CATALOG);
 assert.deepEqual(catalog.slice(5).map(f=>f.binding.source_field),['device.completed_output','device.accepted_output']);for(const field of catalog.slice(5)){assert.equal(field.binding.value_type,'quantity');assert.equal(field.binding.unit,'piece');assert.deepEqual(field.binding.port_ref,value.s.portCalls[0].port);assert.deepEqual(field.binding.object_ref,{kind:'device',id:'CNC'});assert.equal(field.unavailable_reason,undefined);}
});
test('75/80 work-finished1 transfer0 and85 transfer1 use quantity objects; declared previews never show zero',async()=>{
 for(const at of [75000,80000,85000]){const s=setup({at}),value=await s.reader.read(s.context),context={mode:'actual_run',frame:value.frame,point:phase.projectPhase(value.frame)},catalog=fields.mediaDataFields(context).slice(5);assert.deepEqual(catalog.map(f=>fields.resolveMediaData(f.binding,context).formatted),['1 piece',at===85000?'1 piece':'0 piece']);assert.equal(fields.resolveMediaData(catalog[0].binding,context).value.value.coefficient,1);for(const field of catalog)assert.deepEqual(fields.resolveMediaData(field.binding,{mode:'declared_preview'}).status,'unavailable');}
});
test('canonical DecimalAmount formats safe-integer maximum and9-place amounts without float division, conversion or rounding',async()=>{
 const value=await frame(),row=value.frame.port_outputs.ports[0],binding=fields.mediaDataFields(value.context)[5].binding,expected=await json('scenarios/媒体表现/runtime-media-adoption-consumer/old-port/expected.json');for(const [coefficient,decimal_places,text]of expected.decimal_display_cases){row.completed_output.value={coefficient:Number(coefficient),decimal_places};assert.equal(fields.resolveMediaData(binding,value.context).formatted,text+' piece');}
});
test('quantity fields reject missing/wrong exact port, object, type and unit; no transferred alias',async()=>{
 const value=await frame(),binding=fields.mediaDataFields(value.context)[5].binding;
 for(const change of [b=>b.port_ref=null,b=>b.port_ref.port_id='input',b=>b.port_ref.module_ref.version='9.9.9',b=>b.port_ref.instance_id='BLOCKER',b=>b.object_ref={kind:'device',id:'BLOCKER'},b=>b.value_type='number',b=>b.unit=null,b=>b.unit='kg']){const b=structuredClone(binding);change(b);assert.equal(fields.resolveMediaData(b,value.context).status,'unavailable');}
 assert.equal(fields.resolveMediaData({...binding,source_field:'device.transferred_output'},value.context).reason,'unsupported_field');
 value.frame.port_outputs.ports[0].completed_output.unit='kg';assert.equal(fields.resolveMediaData(binding,value.context).reason,'wrong_unit');
});
test('unknown/old frames, missing wiring/history and DefinitionCycle remain named unavailable rather than invented zero',async()=>{
 const value=await frame(),binding=fields.mediaDataFields(value.context)[5].binding;
 for(const mutate of [v=>delete v.frame.port_outputs,v=>v.frame.port_outputs.run_id='old',v=>v.frame.port_outputs.at_ms=85000,v=>v.frame.port_outputs.device=null,v=>v.point.known=false]){const v=structuredClone(value.context);mutate(v);assert.equal(fields.resolveMediaData(binding,v).status,'unavailable');assert.equal(fields.mediaDataFields(v).length,5);}
 for(const reason of ['not_wired','unsupported_capability']){const v=structuredClone(value.context),old=v.frame.port_outputs.ports[0];v.frame.port_outputs.ports[0]={status:'unavailable',port:old.port,unit:old.unit,reason,message:reason==='not_wired'?'No getter wired':'Old history or incompatible item types unavailable'};assert.equal(fields.resolveMediaData(binding,v).reason,'port_unavailable');assert.ok(fields.mediaDataFields(v)[5].unavailable_reason);}
 const v=structuredClone(value.context);v.point.job.execution_ref={kind:'definition_cycle',data:{module_ref:v.frame.port_outputs.device.module_ref}};assert.equal(fields.mediaDataFields(v).length,5);assert.equal(fields.resolveMediaData(binding,v).status,'unavailable');
});
