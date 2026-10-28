import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/regression/old-media-pure/shared/',root);
await mkdir(out,{recursive:true});
for(const file of ['types','phase','clips','bytes','glb','scene','dataFields','imageTransitions']){
 const code=stripTypeScriptTypes(await readFile(new URL(`packages/presentation/src/${file}.ts`,root),'utf8'),{mode:'transform'}).replace(/from '\.\/(\w+)'/g,"from './$1.mjs'");await writeFile(new URL(`${file}.mjs`,out),code);
}
const {declaredProgress,activeClip}=await import(new URL('clips.mjs',out));
const {projectPhase}=await import(new URL('phase.mjs',out));
const {MEDIA_DATA_FIELD_CATALOG,resolveMediaData}=await import(new URL('dataFields.mjs',out));
const {imageTransitionPlan,containedImageRect,transformedImageRect}=await import(new URL('imageTransitions.mjs',out));
const {decodeGlb}=await import(new URL('glb.mjs',out));
const {bindScene}=await import(new URL('scene.mjs',out));
const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
const pkg=await json('modules/media/p302-controlled/model.module.json'),p=pkg.definition.payload.data;
const prefix=await json('scenarios/媒体表现/runtime/shared-preview/actual-prefix.json');
const frames=await json('scenarios/媒体表现/runtime/actual-frames.json');
const point=projectPhase(prefix),context={mode:'actual_run',frame:prefix,point};
const near=(a,b)=>assert.ok(Math.abs(a-b)<1e-10,`${a} != ${b}`);
test('declaration is a unitless visual projection, never a Run/Job/source of live tags',()=>{
 const visual=declaredProgress({phase_ref:'auto',view_u:.525});assert.equal(visual.progress,.525);assert.equal(visual.duration_ms,null);assert.equal(visual.effective_ms,null);assert.equal('job' in visual,false);assert.equal('run' in visual,false);assert.equal(activeClip(p,visual).effective_ms,null);
 for(const view_u of [-.01,1.01,NaN,Infinity])assert.throws(()=>declaredProgress({phase_ref:'auto',view_u}));
 for(const field of MEDIA_DATA_FIELD_CATALOG)assert.equal(resolveMediaData(field.binding,{mode:'declared_preview'}).status,'unavailable');
});
test('declared u and actual finite-window Engine point drive the same real GLB pose; seek is reversible',async()=>{
 near(point.progress,.525);const bytes=await readFile(new URL('modules/media/p302-controlled/station.glb',root)),decoded=await decodeGlb(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength));
 const rig=await bindScene(p,()=>{throw Error('no extra source');},{assetKey:JSON.stringify([p.source.data.id,p.source.data.path,p.source.data.kind]),decoded});
 const before=JSON.stringify(prefix),poses=()=>[...rig.nodes].map(([index,node])=>[index,...node.position.toArray(),...node.quaternion.toArray(),...node.scale.toArray()]);
 rig.apply(point,activeClip(p,point));const actual=poses();const preview=declaredProgress({phase_ref:'auto',view_u:.525});rig.apply(preview,activeClip(p,preview));assert.deepEqual(poses(),actual);
 const terminal=declaredProgress({phase_ref:'auto',view_u:1});rig.apply(terminal,activeClip(p,terminal));rig.apply(preview,activeClip(p,preview));assert.deepEqual(poses(),actual);assert.equal(JSON.stringify(prefix),before);rig.dispose();
});
test('declared native pose and track use the same supported canonical nodes',async()=>{
 const pose=x=>({translation:{x,y:0,z:0},rotation:{x:0,y:0,z:0,w:1},scale:{x:1,y:1,z:1},unit:'meter',coordinate_frame:'right_handed_y_up'});
 const native={...p,kind:'native_scene',source:{kind:'native_scene',data:'declared-test'},nodes:[{id:0,name:'declared part',parent_index:null,transform:pose(0),geometry:{kind:'box',data:{x:1,y:1,z:1}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:1,texture:null}}],poses:[{id:'home',node_transforms:[{node_index:0,pose:pose(0)}]}],actions:[{id:'slide',node_index:0,transition:'linear',keys:[{at_progress:0,pose:pose(0)},{at_progress:1,pose:pose(2)}]}],timeline:[{id:'native',phase_ref:'auto',start_progress:0,end_progress:1,transition:'hold',source:{kind:'native_pose',data:'home'}}]};
 const rig=await bindScene(native,()=>{throw Error('no asset read');}),u=declaredProgress({phase_ref:'auto',view_u:.5});rig.apply(u,activeClip(native,u));assert.equal(rig.nodes.get(0).position.x,1);rig.dispose();
});
test('catalog and actual field source/type/unit/object rules agree, including boundary display vs Job',()=>{
 assert.equal(MEDIA_DATA_FIELD_CATALOG.length,5);assert.equal(new Set(MEDIA_DATA_FIELD_CATALOG.map(x=>x.binding.source_field)).size,5);
 for(const field of MEDIA_DATA_FIELD_CATALOG){assert.ok(field.value_types.includes(field.binding.value_type));assert.equal(resolveMediaData(field.binding,context).status,'available');}
 const elapsed=MEDIA_DATA_FIELD_CATALOG.find(x=>x.binding.source_field==='job.effective_progress_ms').binding;
 assert.equal(resolveMediaData(elapsed,context).value,15750);assert.equal(resolveMediaData({...elapsed,unit:'s'},context).value,15.75);
 for(const binding of [{...elapsed,value_type:'boolean'},{...elapsed,unit:'kg'},{...elapsed,object_ref:{kind:'run',id:prefix.run.id}},{...elapsed,source_field:'port.produced_quantity'}])assert.equal(resolveMediaData(binding,context).status,'unavailable');
 const boundary=frames.find(x=>x.label==='calendar@35000').frame,boundaryPoint=projectPhase(boundary);assert.equal(boundaryPoint.effective_ms,30000);assert.equal(resolveMediaData(elapsed,{mode:'actual_run',frame:boundary,point:boundaryPoint}).value,0);
});
const photo=(transition='crossfade')=>({...p,timeline:[{id:'before',phase_ref:'auto',start_progress:0,end_progress:.5,transition:'hold',source:{kind:'image_asset',data:{id:'red',path:'red.png',kind:'png'}}},{id:'after',phase_ref:'auto',start_progress:.5,end_progress:1,transition,source:{kind:'image_asset',data:{id:'blue',path:'blue.png',kind:'png'}}}]});
const plan=(presentation,u)=>imageTransitionPlan(presentation,activeClip(presentation,declaredProgress({phase_ref:'auto',view_u:u})));
test('hold remains discrete; crossfade is a fixed short part of the existing clip, exact seek boundaries',()=>{
 assert.equal(plan(photo('hold'),.5).previous_alpha,0);assert.equal(plan(photo('hold'),.5).current_alpha,1);
 assert.equal(plan(photo(),.5).previous_alpha,1);near(plan(photo(),.525).current_alpha,.5);near(plan(photo(),.55).current_alpha,1);assert.equal(plan(photo(),1).current_alpha,1);assert.equal(plan(photo(),.499).previous_clip,null);
 const actual=imageTransitionPlan(photo(),activeClip(photo(),point)),preview=plan(photo(),.525);assert.deepEqual(actual,preview);
});
test('pan/zoom use fixed 2D viewports and finish inside the current clip; phase binding stays isolated',()=>{
 const pan=photo('pan'),zoom=photo('zoom');near(plan(pan,.5).current.offset_x,-.04);near(plan(pan,.525).current.offset_x,0);near(plan(pan,.55).current.offset_x,.04);near(plan(zoom,.5).current.scale,1);near(plan(zoom,.525).current.scale,1.04);near(plan(zoom,1).current.scale,1.08);
 const base=containedImageRect(200,100,100,80);assert.deepEqual(base,{x:37.5,y:0,width:125,height:100});const transformed=transformedImageRect(base,plan(pan,.5).current);near(transformed.x,27.5);near(transformed.width,135);near(transformed.y,-4);
 const other=photo();other.timeline[0].phase_ref='load';assert.equal(plan(other,.525).previous_clip,null);
 const mixed=photo();mixed.timeline[0].source={kind:'model_animation',data:{asset_ref:p.source.data,animation_index:0}};assert.throws(()=>plan(mixed,.525),/跨3D/);
});
test('all five installable standard directories use canonical Versioned manifests and preserve exact payloads, declared files and dependencies',async()=>{
 const names=['image-simple','model-simple','image','gif','model'],keys=new Set(names.map(name=>`p302.controlled.${name}@0.1.0`));
 for(const name of names){
  const original=await readFile(new URL(`modules/media/p302-controlled/${name}.module.json`,root)),directory=new URL(`modules/media/p302-controlled/installable/${name}/`,root),manifest=JSON.parse(await readFile(new URL('module.json',directory)));
  assert.deepEqual(Object.keys(manifest).sort(),['data','schema_version']);assert.equal(manifest.schema_version,'1.0');assert.deepEqual(manifest.data,JSON.parse(original));
  const pkg=manifest.data,files=await readdir(directory);assert.deepEqual(files.sort(),['module.json',...pkg.files.map(x=>x.path)].sort());
  for(const ref of pkg.files)assert.deepEqual(await readFile(new URL(ref.path,directory)),await readFile(new URL('modules/media/p302-controlled/'+ref.path,root)));
  for(const dep of pkg.dependencies)assert.ok(keys.has(`${dep.id}@${dep.version}`));
  for(const right of pkg.rights)assert.equal(right.license,'CC0-1.0');
 }
});
