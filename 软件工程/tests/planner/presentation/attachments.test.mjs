import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {Group,Vector3} from 'three';
import {root,out} from './frozen-consumer.mjs';
import {json,bytes} from './runtime.mjs';
import './pixel-decoder.mjs';
const {projectFactoryPresentation}=await import(new URL('factoryPresentation.mjs',out));
const {createFactoryMedia}=await import(new URL('factoryMedia.mjs',out));
const {createFactoryItemLayer}=await import(new URL('factoryItems.mjs',out));
const {poseMatrix}=await import(new URL('layout.mjs',out));
const reports=[];
async function packagesAt(directory){
 const manifest=(await json(directory+'/project.json')).data,result=new Map();
 for(const doc of manifest.documents)if(doc.role==='module_definition'){
  const pkg=(await json(directory+'/'+doc.file_ref.path)).data;result.set(pkg.definition.id+'@'+pkg.definition.version,pkg);
 }
 return result;
}
async function itemCases(){
 const path='.work/delivery/items-reception/artifacts/',cases=[];
 for(const name of await readdir(new URL(path,root))){
  if(!name.startsWith('attempt-'))continue;
  for(const label of ['l03-complete','l03-missing-pcb','phase-resource-wait']){
   const directory=path+name+'/'+label;
   try{const result=await json(directory+'/case-result.json');if(result.passed)cases.push({label,directory,moved:result.moved_project});}catch(e){if(e.code!=='ENOENT')throw e;}
  }
 }
 const residual=await json('.work/delivery/items-residual/result.json');assert.equal(residual.passed,true);
 const main=await json((await json('.work/delivery/host-runtime/delivery/latest-runtime-result.json')).result);
 for(const c of residual.cases)if(!c.reused_actual_complete_case)cases.push({label:c.label,directory:'.work/delivery/items-residual/'+c.label,moved:main.cases.find(x=>x.label===c.label).moved_project});
 return cases;
}
test('frozen production factory projection and real GLB attachment layer retain actual holder/contents in both LODs',async()=>{
 for(const entry of await itemCases()){
  const runs=await json(entry.directory+'/runs.json'),projects=await json(entry.directory+'/frozen-projects.json'),points=await json(entry.directory+'/timepoints.json'),mapping=await json(entry.directory+'/media-mapping.json');
  const modules=await packagesAt(entry.moved);
  for(const row of mapping.new_types)modules.set(row.items_package.definition.id+'@'+row.items_package.definition.version,row.items_package);
  const run=runs.new,project=projects.new,before=JSON.stringify({run,project,points});
  const selected=entry.label==='l03-complete'?[42999,43000,48999,49000,82999,84000,89999,90000,330000]:entry.label==='l03-missing-pcb'?[0,119999,120000]:[points.points.at(-1).at_ms];
  for(const t of selected){
   const actual=points.points.find(p=>p.at_ms===t);assert.ok(actual,entry.label+'/'+t);
   const items=actual.objects.filter(o=>o.kind==='item').map(o=>o.data),jobs=actual.objects.filter(o=>o.kind==='job').map(o=>o.data);
   const context={session_id:'captured-host-consumer',project_id:project.id,scope:{kind:'project'},scope_revision:points.current_revision,project,view:{run_id:run.id,at_ms:t}};
   const reads=[];
   const source=await projectFactoryPresentation(context,{project,run,observation:{run_id:run.id,at_ms:t,objects:actual.objects,next_index:null},jobs,items,raw:actual},{
    async module(ref){const p=modules.get(ref.id+'@'+ref.version);assert.ok(p,'exact saved Host module '+ref.id+'@'+ref.version);return structuredClone(p);},
    async readAsset(request){assert.equal(request.source.kind,'run_module_version');assert.equal(request.source.data.run_id,run.id);assert.equal(request.source.data.at_ms,t);reads.push(request);
     const ref=request.source.data.module_ref;return bytes('.work/delivery/host-media/'+ref.id+'/'+ref.version+'/'+request.asset.path);
    },
   },()=>{});
   assert.equal(source.items.length,items.length);assert.equal(new Set(source.items.map(x=>x.item.id)).size,items.length);
   for(const lod of ['detailed','simplified']){
    const devices=new Map(),locations=new Map(),world=new Group();
    for(const location of project.locations){const node=new Group();node.applyMatrix4(poseMatrix(location.position));locations.set(location.id,node);world.add(node);}
    for(const device of project.devices){
     const input=source.devices.find(v=>v.device_id===device.id);if(!input.media)continue;
     const rig=await createFactoryMedia(device,input,lod),placement=new Group(),line=project.lines.find(l=>l.id===device.line_id);
     if(line)placement.applyMatrix4(poseMatrix(line.transform));placement.applyMatrix4(poseMatrix(device.pose));world.add(placement);placement.add(rig.root);devices.set(device.id,rig);
    }
    let wake=null;
    const layer=createFactoryItemLayer(source.items,lod,id=>devices.get(id)??null,id=>locations.get(id)??null,()=>wake?.());layer.reconcile();
    while(layer.renderState(true).pending)await new Promise(resolve=>{wake=resolve;});
    wake=null;
    const state=layer.renderState(true);assert.equal(state.pending,0);assert.deepEqual(state.failures,[]);
    const ids=new Set(state.objects.map(o=>o.ref.id));assert.equal(ids.size,state.objects.length);
    world.updateMatrixWorld(true);
    for(const rendered of state.objects){
     const item=source.items.find(x=>x.item.id===rendered.ref.id);assert.ok(item?.attachment);assert.ok(!['transformed','scrapped'].includes(item.item.lifecycle));
     const owner=item.attachment.owner_ref,slot=item.attachment.slot;
     const parent=owner.kind==='device'?devices.get(owner.id)?.nodes.get(devices.get(owner.id).presentation.anchor_roles[slot.anchor_role].data):owner.kind==='location'?locations.get(owner.id):state.objects.find(o=>o.ref.id===owner.id)?.root;
     assert.ok(parent,'declared actual owner exists');let current=rendered.root;while(current&&current!==parent)current=current.parent;assert.equal(current,parent,'attached under actual owner, not guessed world position');
     if(owner.kind==='item'){const container=items.find(x=>x.id===owner.id);assert.ok(container.contents.includes(item.item.id));assert.equal(item.item.holder.kind,'container');assert.equal(item.item.holder.id,owner.id);}
     const position=rendered.root.getWorldPosition(new Vector3());assert.ok(position.toArray().every(Number.isFinite));
    }
    for(const item of source.items.filter(x=>x.unavailable))assert.equal(ids.has(item.item.id),false,'unmapped/transformed identities have no invented geometry');
    reports.push({case:entry.label,at_ms:t,lod,actual_identities:items.length,rendered_ids:[...ids],unavailable:state.unavailable,source_reads:reads.length});
    layer.dispose();for(const rig of devices.values())rig.dispose();
   }
  }
  assert.equal(JSON.stringify({run,project,points}),before,'projection/loop never mutates production');
 }
 await mkdir(new URL('.work/delivery/consumer-results/',root),{recursive:true});
 await writeFile(new URL('.work/delivery/consumer-results/attachments.json',root),JSON.stringify({passed:true,reports,consumer:'actual frozen projectFactoryPresentation/createFactoryMedia/createFactoryItemLayer',pixels:'Pillow headless adapter',GUI:false},null,2));
});
