import assert from 'node:assert/strict';
import {test} from 'node:test';
import {writeFile} from 'node:fs/promises';
import {Vector3} from 'three';
import {root,out,json,bytes} from './cu-consumer.mjs';
import './pixel-decoder.mjs';
const {projectFactoryPresentation}=await import(new URL('factoryPresentation.mjs',out));
const {readProcessItems,createProcessItemLayer}=await import(new URL('processItems.mjs',out));
const {resolveMedia}=await import(new URL('load.mjs',out));
const {mediaAsset}=await import(new URL('imageSurface.mjs',out));
const {bindScene,decodeGlb,activeClip,projectPhase,assetKey}=await import(new URL('index.mjs',out));
const labels=['phase-interrupted','l03-robot-output-wait','l03-pallet-output-wait','l03-complete','l03-missing-pcb','phase-resource-wait'];
const reports=[],reads=[],unknown=[],waiting=[];
let checked=0,rendered=0,nested=0,observations=0;
const key=ref=>ref.id+'@'+ref.version;
function recordedPorts(data,observation){
 const modules=new Map(data.modules.map(p=>[key(p.definition),p]));
 return {
  async dispatch(request){
   assert.equal(request.operation,'get_object','the media path sends no production mutation');
   assert.equal(request.project_id,data.project_id);assert.equal(request.scope.kind,'project');
   if(request.params.object_id.kind==='project'){
    assert.equal(request.params.source.kind,'run_snapshot');assert.equal(request.params.source.data.run_id,data.run.id);
    assert.equal(request.params.source.data.at_ms,observation.at_ms);return {kind:'project',data:structuredClone(data.project)};
   }
   assert.equal(request.params.object_id.kind,'module');
   const source=request.params.source;assert.equal(source.kind,'run_module_version');assert.equal(source.data.run_id,data.run.id);assert.equal(source.data.at_ms,observation.at_ms);
   const pkg=modules.get(key(source.data.module_ref));assert.ok(pkg,'actual fresh Host exact module '+key(source.data.module_ref));
   return {kind:'module',data:structuredClone(pkg)};
  },
  async readObservation(request){assert.equal(request.run_id,data.run.id);assert.equal(request.at_ms,observation.at_ms);assert.equal(request.scope_revision,data.scope_revision);return structuredClone(observation);},
  async readAsset(request){
   assert.equal(request.source.kind,'run_module_version');assert.equal(request.source.data.run_id,data.run.id);assert.equal(request.source.data.at_ms,observation.at_ms);assert.equal(request.scope_revision,data.scope_revision);
   const ref=request.source.data.module_ref;reads.push({case:data.label,at_ms:observation.at_ms,module_ref:ref,asset:request.asset});
   return bytes(`.work/cu-delivery/host-media/${ref.id}/${ref.version}/${request.asset.path}`);
  },
 };
}
async function factory(data,observation,ports){
 const modules=new Map(data.modules.map(p=>[key(p.definition),p]));
 return projectFactoryPresentation({session_id:'captured-real-UI',project_id:data.project_id,scope:{kind:'project'},scope_revision:data.scope_revision,project:data.project,view:{run_id:data.run.id,at_ms:observation.at_ms}},
  {project:data.project,run:data.run,observation,jobs:observation.objects.flatMap(o=>o.kind==='job'?[o.data]:[]),items:observation.objects.flatMap(o=>o.kind==='item'?[o.data]:[]),raw:{job_pages:[],item_pages:[]}},
  {module:async ref=>structuredClone(modules.get(key(ref))),readAsset:ports.readAsset},()=>{});
}
test('actual fresh Host observations drive detail attachments through holding, completion, transfer, waits and reverse seek',async()=>{
 for(const label of labels){
  const data=await json(`.work/cu-delivery/host/${label}.json`),before=JSON.stringify(data);
  assert.equal(data.passed,true);assert.equal(data.external_registration,false);
  const points=[...data.points].sort((a,b)=>a.at_ms-b.at_ms);observations+=points.length;
  // Every complete observation is real Host data; geometry checks concentrate on
  // the frozen acceptance boundaries and all short wait/interruption records.
  const boundary=new Set([0,42999,43000,48999,49000,82999,83000,83999,84000,87999,88000,89999,90000,119999,120000,330000]);
  const selected=points.length<=35?points:points.filter((p,i)=>boundary.has(p.at_ms)||i===points.length-1);
  const sequence=[...selected,...selected.toReversed(),selected[0]];
  for(const observation of sequence){
   const ports=recordedPorts(data,observation),source=await factory(data,observation,ports);
   assert.equal(source.items.length,new Set(source.items.map(x=>x.item.id)).size);
   for(const device of source.devices.filter(x=>x.frame&&x.media)){
    const frame=device.frame,point=projectPhase(frame);assert.equal(point.known,true);
    const input=await readProcessItems(frame,device.media.binding,()=>true,ports);
    assert.equal(input.device_id,device.device_id);assert.equal(input.unavailable,null);
    for(const lod of ['detailed','simplified']){
     const resolved=await resolveMedia({...device.media,lod,mode:'actual_run',frame});
     const asset=mediaAsset(resolved,activeClip(resolved.presentation,point));
     const decoded=asset?.kind==='glb'?await decodeGlb(await resolved.read(asset)):undefined;
     const rig=await bindScene(resolved.presentation,resolved.read,decoded?{assetKey:assetKey(asset),decoded}:undefined);
     rig.apply(point,activeClip(resolved.presentation,point));
     const ownedNodes=new Map([...rig.nodes].map(([id,node])=>[id,node.children.length]));
     let wake=null;const layer=createProcessItemLayer(input,rig,resolved.presentation,lod,()=>wake?.());
     layer.reconcile();while(layer.renderState().pending)await new Promise(resolve=>{wake=resolve;});wake=null;
     const state=layer.renderState();assert.deepEqual(state.failures,[]);assert.equal(state.pending,0);
     const ids=state.objects.map(x=>x.ref.id);assert.equal(ids.length,new Set(ids).size);
     rig.root.updateMatrixWorld(true);
     for(const object of state.objects){
      const item=input.source.items.find(x=>x.item.id===object.ref.id);assert.ok(item?.attachment);assert.equal(item.anchor_ref.id,device.device_id);
      assert.ok(!['transformed','scrapped'].includes(item.item.lifecycle));
      const {owner_ref,slot}=item.attachment;
      const owner=owner_ref.kind==='device'?{nodes:rig.nodes,presentation:resolved.presentation}:null;
      if(owner){const anchor=owner.presentation.anchor_roles[slot.anchor_role];assert.equal(anchor.kind,'scene_node');let parent=object.root;while(parent&&parent!==owner.nodes.get(anchor.data))parent=parent.parent;assert.equal(parent,owner.nodes.get(anchor.data));}
      else {assert.equal(owner_ref.kind,'item');const container=input.source.items.find(x=>x.item.id===owner_ref.id);assert.ok(container.item.contents.includes(item.item.id));assert.equal(item.item.holder.kind,'container');assert.equal(item.item.holder.id,container.item.id);const renderedParent=state.objects.find(x=>x.ref.id===container.item.id);assert.ok(renderedParent);let parent=object.root;while(parent&&parent!==renderedParent.root)parent=parent.parent;assert.equal(parent,renderedParent.root);nested++;}
      const position=object.root.getWorldPosition(new Vector3()).toArray();assert.ok(position.every(Number.isFinite));
      rendered++;
     }
     for(const item of input.source.items.filter(x=>x.unavailable||x.anchor_ref?.id!==device.device_id))assert.equal(ids.includes(item.item.id),false);
     reports.push({case:label,at_ms:observation.at_ms,device:device.device_id,job:frame.job_id,lod,state:point.job.state,progress:point.progress,rendered:ids,unavailable:state.unavailable,statuses:layer.statuses()});
     if(['waiting_resource','waiting_output'].includes(point.job.state))waiting.push(reports.at(-1));
     layer.dispose();layer.dispose();for(const [id,count]of ownedNodes)assert.equal(rig.nodes.get(id).children.length,count,'all old descendants detached before the next seek');
     rig.dispose();checked++;
    }
   }
  }
  assert.equal(JSON.stringify(data),before,'viewing, replacement and reverse seek preserve all real business records');
 }
 assert.ok(rendered>0);assert.ok(nested>0);assert.ok(waiting.some(x=>x.state==='waiting_output'));assert.ok(waiting.some(x=>x.state==='waiting_resource'));
 await writeFile(new URL('.work/cu-delivery/evidence/detail-attachments.json',root),JSON.stringify({passed:true,checked,rendered,nested,actual_Host_observations:observations,reports,reads,waiting},null,2));
});
test('stale/foreign Run and DefinitionCycle sources never acquire actual device attachments',async()=>{
 const data=await json('.work/cu-delivery/host/l03-complete.json'),observation=data.points.find(p=>p.at_ms===43000),ports=recordedPorts(data,observation),source=await factory(data,observation,ports),device=source.devices.find(d=>d.frame&&d.media);
 assert.ok(device);
 await assert.rejects(readProcessItems(device.frame,device.media.binding,()=>false,ports),/来源|变化/);
 const foreign=structuredClone(device.media.binding);foreign.request.source.data.run_id='foreign-run';
 await assert.rejects(readProcessItems(device.frame,foreign,()=>true,ports),/Run/);
 const draft=structuredClone(device.media.binding);draft.request.scope={kind:'definition_draft',data:'actual-draft'};
 await assert.rejects(readProcessItems(device.frame,draft,()=>true,ports),/DefinitionCycle/);
});

test('in-flight source changes, historical Jobs, absent anchors and region LOD retain explicit unknown attachments',async()=>{
 const data=await json('.work/cu-delivery/host/l03-complete.json');let found=null;
 for(const observation of [...data.points].sort((a,b)=>a.at_ms-b.at_ms)){
  const jobs=observation.objects.flatMap(o=>o.kind==='job'?[o.data]:[]);
  const pair=jobs.flatMap(active=>active.state==='working'&&active.execution_ref.kind==='project_device'?
   jobs.filter(old=>old.state==='completed'&&old.execution_ref.kind==='project_device'&&old.execution_ref.data.device_id===active.execution_ref.data.device_id).map(old=>({active,old})):[])[0];
  if(!pair)continue;const ports=recordedPorts(data,observation),source=await factory(data,observation,ports),device=source.devices.find(d=>d.frame?.job_id===pair.active.id);
  if(device&&source.items.some(i=>i.anchor_ref?.id===device.device_id&&i.attachment&&!i.unavailable)){found={observation,ports,device,pair};break;}
 }
 assert.ok(found,'actual repeated device has a completed Job and another actual owner');
 const {ports,device,pair}=found;
 let current=true;await assert.rejects(readProcessItems(device.frame,device.media.binding,()=>current,{...ports,async dispatch(request){const value=await ports.dispatch(request);current=false;return value;}}),/来源|变化/);
 const historical=await readProcessItems({...device.frame,job_id:pair.old.id},device.media.binding,()=>true,ports);
 assert.ok(historical.unavailable);
 const input=await readProcessItems(device.frame,device.media.binding,()=>true,ports),point=projectPhase(device.frame);
 const resolved=await resolveMedia({...device.media,mode:'actual_run',frame:device.frame,lod:'simplified'}),asset=mediaAsset(resolved,activeClip(resolved.presentation,point)),decoded=await decodeGlb(await resolved.read(asset));
 const rig=await bindScene(resolved.presentation,resolved.read,{assetKey:assetKey(asset),decoded});rig.apply(point,activeClip(resolved.presentation,point));
 const old=createProcessItemLayer(historical,rig,resolved.presentation,'simplified',()=>{});old.reconcile();assert.equal(old.renderState().objects.length,0);assert.equal(old.renderState().pending,0);old.dispose();
 const absent=createProcessItemLayer(input,rig,{...resolved.presentation,anchor_roles:{}},'simplified',()=>{});absent.reconcile();assert.equal(absent.renderState().objects.length,0);assert.ok(absent.renderState().unavailable.some(x=>/角色/.test(x.message)));absent.dispose();
 const region=createProcessItemLayer(input,rig,resolved.presentation,'region',()=>{});region.reconcile();assert.equal(region.renderState().objects.length,0);assert.equal(region.renderState().pending,0);region.dispose();rig.dispose();
 await writeFile(new URL('.work/cu-delivery/evidence/attachment-boundaries.json',root),JSON.stringify({passed:true,actual_time:found.observation.at_ms,device:device.device_id,active_job:pair.active.id,historical_job:pair.old.id,in_flight_change_rejected:true,historical_pose_not_retargeted:true,missing_anchor_unavailable:true,region_has_no_geometry:true},null,2));
});

test('actual output-wait holders and nested contents keep identical world transforms across time and reverse seek',async()=>{
 const results=[];
 for(const [label,deviceId,times]of [['l03-robot-output-wait','R01',[48000,60000,48000]],['l03-pallet-output-wait','CASE01',[90000,119999,90000]]]){
  const data=await json(`.work/cu-delivery/host/${label}.json`);
  for(const lod of ['detailed','simplified']){
   let rig=null,resolved=null,reference=null;
   for(const time of times){
    const observation=data.points.find(p=>p.at_ms===time);assert.ok(observation);
    const ports=recordedPorts(data,observation),source=await factory(data,observation,ports),device=source.devices.find(d=>d.device_id===deviceId);
    assert.ok(device.frame&&device.media);const point=projectPhase(device.frame);assert.equal(point.job.state,'waiting_output');assert.equal(point.terminal_pose,true);
    if(!rig){resolved=await resolveMedia({...device.media,lod,mode:'actual_run',frame:device.frame});const asset=mediaAsset(resolved,activeClip(resolved.presentation,point)),decoded=await decodeGlb(await resolved.read(asset));rig=await bindScene(resolved.presentation,resolved.read,{assetKey:assetKey(asset),decoded});}
    rig.apply(point,activeClip(resolved.presentation,point));
    const input=await readProcessItems(device.frame,device.media.binding,()=>true,ports);
    let wake=null;const layer=createProcessItemLayer(input,rig,resolved.presentation,lod,()=>wake?.());layer.reconcile();while(layer.renderState().pending)await new Promise(resolve=>{wake=resolve;});wake=null;
    const state=layer.renderState();assert.deepEqual(state.failures,[]);assert.ok(state.objects.length>0);rig.root.updateMatrixWorld(true);
    const transforms=Object.fromEntries(state.objects.map(x=>[x.ref.id,x.root.matrixWorld.toArray()]));
    if(reference)assert.deepEqual(transforms,reference,'waiting holds the exact actual support pose and contents, including on reverse seek');else reference=transforms;
    results.push({label,lod,at_ms:time,holders:input.source.items.filter(i=>i.anchor_ref?.id===deviceId).map(i=>({id:i.item.id,holder:i.item.holder,contents:i.item.contents})),transforms});
    layer.dispose();
   }
   rig.dispose();
  }
 }
 await writeFile(new URL('.work/cu-delivery/evidence/wait-world-transforms.json',root),JSON.stringify({passed:true,checks:results.length,results},null,2));
});

test('an old actual Run keeps its original exact item sources after newer media adoption',async()=>{
 const data=await json('.work/cu-delivery/host/l03-complete.json');
 // The Rust consumer separately asserted each old/new public ObjectsAt payload
 // equal before exporting the new private observation. Only the page's Run ID
 // is composed here; old business payloads and frozen Project are retained.
 const at=data.points.find(p=>p.at_ms===43000),observation={...structuredClone(at),run_id:data.old_run.id};
 const old={...data,run:data.old_run,project:data.old_project};
 const ports=recordedPorts(old,observation),source=await factory(old,observation,ports),device=source.devices.find(d=>d.device_id==='R01');
 assert.ok(device.frame&&device.media);
 const projected=await readProcessItems(device.frame,device.media.binding,()=>true,ports);
 assert.equal(projected.source.project.revision,data.old_run.input_ref.project_revision);
 assert.deepEqual(projected.source.project,data.old_project);
 const differences=[];
 for(const item of projected.source.items){
  const original=data.old_project.item_types.find(t=>t.id===item.item.item_type_ref.id&&t.revision===item.item.item_type_ref.revision);
  assert.ok(original);
  if(original.module_ref&&item.media){
   assert.deepEqual(item.media.binding.request.source.data.module_ref,original.module_ref);
   const current=data.current_project.item_types.find(t=>t.id===original.id&&t.revision===original.revision);
   if(JSON.stringify(current.module_ref)!==JSON.stringify(original.module_ref))differences.push({id:item.item.id,old:original.module_ref,current:current.module_ref});
  }else if(!original.module_ref)assert.ok(item.media===null&&item.unavailable.includes('精确Items'));
 }
 assert.ok(differences.length>0,'the frozen old and current adopted media versions really differ');
 await writeFile(new URL('.work/cu-delivery/evidence/old-run-item-sources.json',root),JSON.stringify({passed:true,old_run:data.old_run.id,old_revision:data.old_project.revision,current_revision:data.current_project.revision,exact_old_vs_current:differences,current_media_not_inferred:true,old_public_payloads_previously_verified_by_fresh_Host:true},null,2));
});
