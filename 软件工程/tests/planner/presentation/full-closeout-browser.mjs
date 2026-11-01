import * as T from 'three';
import {RoomEnvironment} from 'three/addons/environments/RoomEnvironment.js';
import {projectFactoryPresentation} from '/.work/full-closeout-20261021/delivery/consumer/factoryPresentation.mjs';
import {createFactoryMedia} from '/.work/full-closeout-20261021/delivery/consumer/factoryMedia.mjs';
import {createFactoryItemLayer} from '/.work/full-closeout-20261021/delivery/consumer/factoryItems.mjs';
import {createFactoryResourceLayer} from '/.work/full-closeout-20261021/delivery/consumer/factoryResources.mjs';
import {poseMatrix} from '/.work/full-closeout-20261021/delivery/consumer/layout.mjs';
import {ControlledGif,projectPhase,decodeGlb,bindScene,activeClip,assetKey} from '/.work/full-closeout-20261021/delivery/consumer/index.mjs';
const base='/.work/full-closeout-20261021/delivery/',data=await fetch(base+'render/input.json').then(r=>r.json());
const output=document.querySelector('#output'),ctx=output.getContext('2d'),status=document.querySelector('#status');
const check=(v,message)=>{if(!v)throw Error(message);},url=p=>'/'+p.replaceAll('\\','/').split('/').map(encodeURIComponent).join('/');
const renderer=new T.WebGLRenderer({antialias:true,preserveDrawingBuffer:true});
renderer.setSize(1920,900);renderer.setPixelRatio(1);renderer.outputColorSpace=T.SRGBColorSpace;
renderer.toneMapping=T.ACESFilmicToneMapping;renderer.toneMappingExposure=1;
const pmrem=new T.PMREMGenerator(renderer),environment=pmrem.fromScene(new RoomEnvironment(),.04).texture;
const frames=[],reads=[],errors=[];
const png=()=>new Promise(resolve=>output.toBlob(async blob=>resolve(new Uint8Array(await blob.arrayBuffer())),'image/png'));
async function save(name,value){
 const response=await fetch('/p302-evidence/'+name,{method:'POST',body:value instanceof Uint8Array?value:JSON.stringify(value,null,2)});
 if(!response.ok)throw Error(await response.text());
}
function fit(camera,box,aspect){
 camera.up.set(0,0,1);camera.aspect=aspect;const center=box.getCenter(new T.Vector3()),size=box.getSize(new T.Vector3());
 const radius=Math.max(size.length()/2,.3),distance=radius/Math.sin(camera.fov*Math.PI/360)*1.12;
 camera.position.copy(center).addScaledVector(new T.Vector3(1,-1,.9).normalize(),distance);camera.lookAt(center);
 camera.near=Math.max(.001,distance/30);camera.far=distance+radius*5;camera.updateProjectionMatrix();
}
async function sceneFrame(c,point,lod){
 const before=JSON.stringify({run:c.run,project:c.project,point}),modules=new Map(c.modules.map(p=>[p.definition.id+'@'+p.definition.version,p]));
 const items=point.objects.filter(o=>o.kind==='item').map(o=>o.data),jobs=point.objects.filter(o=>o.kind==='job').map(o=>o.data);
 const context={session_id:'independent-hidden-browser',project_id:c.project.id,scope:{kind:'project'},scope_revision:c.scope_revision,project:c.project,view:{run_id:c.run.id,at_ms:point.at_ms}};
 const source=await projectFactoryPresentation(context,{project:c.project,run:c.run,observation:{run_id:c.run.id,at_ms:point.at_ms,objects:point.objects,next_index:null},jobs,items,raw:point},{
  async module(ref){const p=modules.get(ref.id+'@'+ref.version);check(p,'Missing saved exact module '+ref.id+'@'+ref.version);return structuredClone(p);},
  async readAsset(request){
   check(request.source.kind==='run_module_version'&&request.source.data.run_id===c.run.id&&request.source.data.at_ms===point.at_ms,'Wrong exact Run asset source');
   const ref=request.source.data.module_ref,path='.work/delivery/host-media/'+ref.id+'/'+ref.version+'/'+request.asset.path;
   const response=await fetch(url(path));check(response.ok,'Missing exact Host asset '+path);reads.push({case:c.label,t:point.at_ms,ref,asset:request.asset});
   return response.arrayBuffer();
  }
 },()=>{});
 const scene=new T.Scene();scene.background=new T.Color('#e5ebef');scene.environment=environment;
 scene.add(new T.HemisphereLight(0xffffff,0x71808a,2));const sun=new T.DirectionalLight(0xffffff,2.5);sun.position.set(3,-5,10);scene.add(sun);
 const world=new T.Group(),locations=new Map(),devices=new Map(),roots=[];scene.add(world);
 for(const location of c.project.locations){const node=new T.Group();node.applyMatrix4(poseMatrix(location.position));locations.set(location.id,node);world.add(node);}
 for(const device of c.project.devices){
  const input=source.devices.find(x=>x.device_id===device.id);if(!input?.media)continue;
  const rig=await createFactoryMedia(device,input,lod),placement=new T.Group(),line=c.project.lines.find(x=>x.id===device.line_id);
  if(line)placement.applyMatrix4(poseMatrix(line.transform));placement.applyMatrix4(poseMatrix(device.pose));world.add(placement);placement.add(rig.root);devices.set(device.id,rig);roots.push(rig.root);
 }
 let wake=null;const changed=()=>wake?.(),resources=createFactoryResourceLayer(source.resources??[],lod,id=>locations.get(id)??null,changed);
 resources.reconcile();const layer=createFactoryItemLayer(source.items,lod,id=>devices.get(id)??null,id=>locations.get(id)??null,changed);layer.reconcile();
 while(layer.renderState(true).pending||resources.renderState().pending)await new Promise(resolve=>{wake=resolve;});
 wake=null;const state=layer.renderState(true),resourceState=resources.renderState();
 check(state.failures.length===0,'Actual item media decode failure '+JSON.stringify(state.failures));
 check(resourceState.failures.length===0,'Actual resource media decode failure '+JSON.stringify(resourceState.failures));
 world.updateMatrixWorld(true);const box=new T.Box3();for(const r of roots)box.union(new T.Box3().setFromObject(r));
 for(const object of state.objects){const item=items.find(i=>i.id===object.ref.id);check(item,'Invented displayed identity');
  check(!['transformed','scrapped'].includes(item.lifecycle),'Destroyed item displayed');
  check(object.root.getWorldPosition(new T.Vector3()).toArray().every(Number.isFinite),'Invalid actual attachment position');
 }
 const focusId=c.label==='l03-robot-output-wait'?'R01':c.label==='l03-pallet-output-wait'?'CASE01':c.label.startsWith('phase-')?'CNC':'R01';
 const camera=new T.PerspectiveCamera(40),detail=new T.PerspectiveCamera(40);fit(camera,box,1280/900);
 fit(detail,new T.Box3().setFromObject(devices.get(focusId)?.root??world),640/900);
 renderer.setScissorTest(true);renderer.setViewport(0,0,1280,900);renderer.setScissor(0,0,1280,900);renderer.render(scene,camera);
 renderer.setViewport(1280,0,640,900);renderer.setScissor(1280,0,640,900);renderer.render(scene,detail);
 ctx.fillStyle='#f3f6f8';ctx.fillRect(0,0,1920,1080);ctx.drawImage(renderer.domElement,0,125);
 ctx.fillStyle='#213a4c';ctx.font='bold 28px Microsoft YaHei';ctx.fillText(c.label+' · '+lod+' · '+point.at_ms+' ms',24,38);
 ctx.font='20px Microsoft YaHei';ctx.fillText('实际Host历史输入 → 冻结生产投影 → 原媒体/附件消费者 → WebGL；非Native界面',24,73);
 const active=source.devices.filter(d=>d.frame).map(d=>({id:d.device_id,point:projectPhase(d.frame)}));
 const focus=active.find(x=>x.id===focusId)?.point;
 ctx.fillText('实际身份 '+items.length+'；'+focusId+'：阶段 '+(focus?.display_phase_id??'未知')+'；有效进度 '+(focus?.progress??'未知')+'；状态 '+(focus?.job?.state??'未开工')+'；持终态 '+Boolean(focus?.terminal_pose),24,110);
 ctx.font='18px Microsoft YaHei';ctx.fillText('显示物品 '+state.objects.length+'；未映射/不适用 '+state.unavailable.length+'；精确素材读取 '+reads.length+'；观看不产生生产事件',24,1060);
 const name=c.label+'-'+point.at_ms+'-'+lod+'.png';await save('render/'+name,await png());
 const localNodes=[];for(const [device,rig]of devices)for(const [index,node]of rig.nodes)localNodes.push({device,index,parent:node.parent?.name,translation:node.position.toArray(),rotation:node.quaternion.toArray(),scale:node.scale.toArray()});
 const region=createFactoryItemLayer(source.items,'region',()=>null,()=>null,()=>{});
 region.reconcile();check(region.renderState(true).objects.length===0,'Region must not invent individual item meshes');
 const regionSummary={actual_ids:source.items.map(x=>x.item.id),individual_meshes:0};region.dispose();
 frames.push({case:c.label,at_ms:point.at_ms,lod,image:'render/'+name,run_id:c.run.id,actual_items:items.length,rendered_ids:state.objects.map(o=>o.ref.id),unavailable:state.unavailable,active,localNodes,regionSummary});
 check(JSON.stringify({run:c.run,project:c.project,point})===before,'Viewing mutated production');
 layer.dispose();resources.dispose();for(const rig of devices.values())rig.dispose();renderer.renderLists.dispose();
}
async function gifFrames(){
 check(typeof ImageDecoder==='function','Native browser ImageDecoder unavailable');
 const encoded=await fetch(url(data.gif.file)).then(r=>r.arrayBuffer()),gif=new ControlledGif(encoded),samples=[];
 for(const [t,terminal,expected]of [[0,false,0],[199,false,0],[200,false,1],[800,false,4],[1600,false,0],[1600,true,7],[800,false,4]]){
  const bitmap=await gif.frame(t,terminal);ctx.fillStyle='#f3f6f8';ctx.fillRect(0,0,1920,1080);ctx.fillStyle='#213a4c';ctx.font='28px Microsoft YaHei';
  ctx.fillText('实际Host GIF · 浏览器原生 ImageDecoder · '+t+'ms · terminal='+terminal+' · 应为实帧'+expected,24,48);
  const raw=document.createElement('canvas');raw.width=bitmap.width;raw.height=bitmap.height;
  const rawctx=raw.getContext('2d');rawctx.drawImage(bitmap,0,0);
  const rawPixels=rawctx.getImageData(0,0,raw.width,raw.height).data;
  await save('render/gif-'+t+'-'+terminal+'.rgba',new Uint8Array(rawPixels.buffer));
  ctx.drawImage(bitmap,24,90,1728,972);bitmap.close();await save('render/gif-'+t+'-'+terminal+'.png',await png());
  samples.push({t,terminal,expected,width:raw.width,height:raw.height,raw:'render/gif-'+t+'-'+terminal+'.rgba'});
 }
 gif.close();return {native_ImageDecoder:true,actual_Host_source:data.gif.source,samples};
}
document.querySelector('#run').onclick=async()=>{
 document.querySelector('#run').disabled=true;
 try{
  const gif=await gifFrames();
  for(const c of data.cases)for(const point of c.points)for(const lod of ['detailed','simplified']){
   status.textContent='实际渲染 '+c.label+' / '+point.at_ms+' ms / '+lod;await sceneFrame(c,point,lod);
  }
  await save('render/result.json',{passed:true,frames,gif,asset_reads:reads.length,renderer:renderer.getContext().getParameter(renderer.getContext().VERSION),
   original_data_unchanged:true,Native:false,NFR:false,human_acceptance:null});
  status.textContent='已完成 '+frames.length+' 张实际历史WebGL画面及原生GIF实帧；未替代Native与性能验收。';
 }catch(error){errors.push(String(error));status.textContent='失败：'+error;await save('render/error.json',{errors,completed_frames:frames.length});}
};
status.textContent='已载入六组实际Host历史。开始后只写本窗口取证目录，不调用生产写操作。';
const pauseButton=document.createElement('button');pauseButton.textContent='补验实际暂停帧';document.querySelector('header').append(pauseButton);
pauseButton.onclick=async()=>{
 pauseButton.disabled=true;
 try{
  const rows=await fetch('/.work/delivery/legacy-engine/actual-frames.json').then(r=>r.json());
  const frame=rows.find(x=>x.frame.run.computation_status==='paused').frame,before=JSON.stringify(frame),point=projectPhase(frame);
  check(point.paused&&point.at_ms===20000&&point.progress===.5,'Actual Engine paused-prefix expectation changed');
  const records=[];
  for(const suffix of ['','.simple']){
   const folder='.work/delivery/host-media/p302.line-grade.assembly'+suffix+'/0.4.1/';
   const pack=await fetch(url(folder+'module.json')).then(r=>r.json()),p=pack.definition.payload.data,asset=p.source.data;
   const read=async a=>fetch(url(folder+a.path)).then(r=>{check(r.ok,'Exact paused asset');return r.arrayBuffer();});
   const decoded=await decodeGlb(await read(asset)),rig=await bindScene(p,read,{assetKey:assetKey(asset),decoded});
   rig.apply(point,activeClip(p,point));const transforms=()=>JSON.stringify([...rig.nodes].map(([i,n])=>[i,n.position.toArray(),n.quaternion.toArray(),n.scale.toArray()]));
   const frozen=transforms();for(let i=0;i<5;i++)await new Promise(resolve=>requestAnimationFrame(resolve));
   check(transforms()===frozen,'Browser viewing advanced paused pose');
   const scene=new T.Scene();scene.background=new T.Color('#e5ebef');scene.environment=environment;
   scene.add(new T.HemisphereLight(0xffffff,0x71808a,2));const sun=new T.DirectionalLight(0xffffff,2);sun.position.set(3,-5,10);scene.add(sun);
   const world=new T.Group();world.rotation.x=Math.PI/2;world.add(rig.root);scene.add(world);world.updateMatrixWorld(true);
   const camera=new T.PerspectiveCamera(40);fit(camera,new T.Box3().setFromObject(world),1920/900);
   renderer.setScissorTest(false);renderer.setViewport(0,0,1920,900);renderer.render(scene,camera);
   ctx.fillStyle='#f3f6f8';ctx.fillRect(0,0,1920,1080);ctx.fillStyle='#213a4c';ctx.font='28px Microsoft YaHei';
   ctx.fillText('实际Engine暂停 · 20000ms · auto有效15000ms / 30000ms = 0.5 · '+(suffix?'simplified':'detailed'),24,46);
   ctx.drawImage(renderer.domElement,0,125);await save('render/paused'+(suffix?'-simple':'-detailed')+'.png',await png());
   records.push({module_ref:{id:pack.definition.id,version:pack.definition.version},point,view_frames_without_advance:5,canonical_nodes:rig.nodes.size});rig.dispose();
  }
  check(JSON.stringify(frame)===before,'Pause read mutated Engine prefix');
  await save('render/paused-result.json',{passed:true,source:'.work/delivery/legacy-engine/actual-frames.json',records,Native:false});
  status.textContent='实际暂停帧两LOD已渲染；浏览器帧不推进阶段有效工时。';
 }catch(error){status.textContent='暂停补验失败：'+error;await save('render/paused-error.json',{error:String(error)});}
};
