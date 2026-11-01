// Regression for the observed cross-operation material animation, using the
// actual Host observation and the original production phase projection.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {stripTypeScriptTypes} from 'node:module';import {pathToFileURL} from 'node:url';
const [proof,output]=process.argv.slice(2),root=process.cwd();await fs.mkdir(output,{recursive:true});
const read=async name=>JSON.parse(await fs.readFile(path.join(proof,name),'utf8'));
await fs.writeFile(path.join(output,'phase.mjs'),stripTypeScriptTypes(await fs.readFile(path.join(root,'packages/presentation/src/phase.ts'),'utf8'),{mode:'transform'}));
const source=await fs.readFile(path.join(root,'apps/planner/src/scene/factoryMedia.ts'),'utf8'),body=source.slice(source.indexOf('export function factoryItemPoint'),source.indexOf('async function resolveInput'));
await fs.writeFile(path.join(output,'item.mjs'),"import {projectPhase} from './phase.mjs';class UnsupportedMedia extends Error{}\n"+stripTypeScriptTypes(body,{mode:'transform'}));
const {factoryItemPoint}=await import(pathToFileURL(path.join(output,'item.mjs')).href);
const [run,frames,modules]=await Promise.all(['actual-connected-run.json','actual-observation-frames.json','actual-frozen-modules.json'].map(read));
const page=frames.find(f=>f.at_ms===100000),jobs=page.objects.filter(o=>o.kind==='job').map(o=>o.data),item=page.objects.filter(o=>o.kind==='item').map(o=>o.data).find(i=>i.item_type_ref.id==='PF-packed-case'&&i.holder.kind==='job');
const job=jobs.find(j=>j.id===item.holder.id),definition=modules.find(m=>m.definition.id===job.operation_ref.id&&m.definition.version===job.operation_ref.version).definition;
const operation=definition.payload.category==='devices'?definition.payload.data.behavior.inline:definition.payload.data,frame={run,snapshot:page,job_id:job.id,operation},input={item,holder_frame:frame},state=item.attributes.presentation_state.value;
assert.equal(job.execution_ref.data.device_id,'PF-manual-packing');assert.equal(item.attributes.seal_state.value,'open');
const stable={phase_ref:state},scoped=`${state}:${job.operation_ref.id}:${job.operation_ref.version}:unload`,legacy=`${state}:unload`;
const before=JSON.stringify({run,page,item,operation}),checks=[];
function check(name,body){try{body();checks.push({name,passed:true});}catch(error){checks.push({name,passed:false,error:String(error)});}}
check('The exact actual holder operation and version may drive its explicitly scoped clip',()=>{
 const point=factoryItemPoint(input,{timeline:[stable,{phase_ref:scoped}]});assert.equal(point.display_phase_id,scoped);assert.equal(point.progress,1);
});
check('Manual packing output-wait cannot drive tape-case-sealer material clips',()=>{
 const clip=`${state}:sf.native50.device.tape-case-sealer:1.1.0:unload`,point=factoryItemPoint(input,{timeline:[stable,{phase_ref:clip}]});assert.equal(point.display_phase_id,state);assert.equal(point.effective_ms,null);
});
check('A different exact version cannot borrow the holder clock',()=>{
 const clip=`${state}:${job.operation_ref.id}:99.0.0:unload`;assert.equal(factoryItemPoint(input,{timeline:[stable,{phase_ref:clip}]}).display_phase_id,state);
});
check('Existing state:phase authoring keeps the same actual phase clock',()=>{
 const point=factoryItemPoint(input,{timeline:[stable,{phase_ref:legacy}]});assert.equal(point.display_phase_id,legacy);assert.equal(point.progress,1);
});
check('A location or transport item has a finite-state pose without a fabricated Job',()=>{
 const point=factoryItemPoint({item},{timeline:[stable,{phase_ref:scoped}]});assert.equal(point.display_phase_id,state);assert.equal(point.effective_ms,null);
});
check('An incomplete actual snapshot cannot produce a scoped phase',()=>{
 assert.throws(()=>factoryItemPoint({...input,holder_frame:{...frame,snapshot:{...page,next_index:200}}},{timeline:[stable,{phase_ref:scoped}]}),/未知/);
});
const mediaPackage=modules.find(m=>m.definition.id==='sf.paperfactory.media.packed-case'),media=mediaPackage.definition.payload.data;
const raw=await fs.readFile(path.join(proof,'project',mediaPackage.assets[0].path)),jsonLength=raw.readUInt32LE(12),gltf=JSON.parse(raw.subarray(20,20+jsonLength).toString()),binStart=20+jsonLength+8;
function terminalScale(point,nodeName){
 const clip=media.timeline.find(c=>c.phase_ref===point.display_phase_id),animation=gltf.animations[clip.source.data.animation_index],node=gltf.nodes.findIndex(n=>n.name===nodeName),channel=animation.channels.find(c=>c.target.node===node&&c.target.path==='scale');
 const accessor=gltf.accessors[animation.samplers[channel.sampler].output],view=gltf.bufferViews[accessor.bufferView],offset=binStart+(view.byteOffset??0)+(accessor.byteOffset??0)+(accessor.count-1)*12;
 return [0,4,8].map(i=>raw.readFloatLE(offset+i));
}
check('Actual manual-packing output-wait keeps the exact carton tape mesh hidden',()=>{
 const point=factoryItemPoint(input,media);assert.equal(point.display_phase_id,state);assert.deepEqual(terminalScale(point,'TopTape'),[0,0,0]);
});
check('Actual final released carton retains its tape, label and both bands',()=>{
 const last=frames.at(-1),carton=last.objects.filter(o=>o.kind==='item').map(o=>o.data).find(i=>i.item_type_ref.id==='PF-packed-case'&&i.holder.kind==='location');
 assert.equal(carton.attributes.seal_state.value,'taped');assert.equal(carton.attributes.label_state.value,'labelled');assert.equal(carton.attributes.band_state.value,'strapped');
 const point=factoryItemPoint({item:carton},media);for(const name of ['TopTape','AppliedShippingLabel','AppliedBand_-0.09','AppliedBand_0.09'])assert.deepEqual(terminalScale(point,name),[1,1,1]);
});
check('Actual sheet-feeder holder cannot drive the creaser material clip',()=>{
 const flat=modules.find(m=>m.definition.id==='sf.paperfactory.media.box-blank').definition.payload.data;
 let found=null;
 for(const page of frames){const jobs=page.objects.filter(o=>o.kind==='job').map(o=>o.data);for(const o of page.objects){if(o.kind!=='item'||o.data.item_type_ref.id!=='PF-flat-box-blank'||o.data.holder.kind!=='job')continue;const job=jobs.find(j=>j.id===o.data.holder.id);if(job?.execution_ref.kind==='project_device'&&job.execution_ref.data.device_id==='PF-sheet-feeder'){found={page,item:o.data,job};break;}}if(found)break;}
 assert.ok(found,'Use a real captured feeder owner, not an invented Job');const d=modules.find(m=>m.definition.id===found.job.operation_ref.id&&m.definition.version===found.job.operation_ref.version).definition;
 const point=factoryItemPoint({item:found.item,holder_frame:{run,snapshot:found.page,job_id:found.job.id,operation:d.payload.data.behavior.inline}},flat);assert.equal(point.display_phase_id,found.item.attributes.presentation_state.value);assert.equal(point.effective_ms,null);
});
assert.equal(JSON.stringify({run,page,item,operation}),before);
const result={source:'Actual100000ms Host observation; real production factoryItemPoint and projectPhase; explicit media authoring fixtures only',run_id:run.id,item_id:item.id,operation_ref:job.operation_ref,checks,immutable_inputs:true};
await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(checks));if(checks.some(c=>!c.passed))process.exitCode=1;
