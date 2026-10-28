/// <reference types="vite/client" />
import {useEffect,useRef,useState} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import * as THREE from 'three';
import {FactoryScene} from '../../../apps/planner/src/scene/FactoryScene';
import {createFactoryPresentation,type FactoryPresentationContext,type FactoryPresentationSource,type FactoryPresentationPorts,type FactoryItemAttachment} from '../../../apps/planner/src/services/factoryPresentation';
import {dispatch,HostFailure} from '../../../apps/planner/src/services/projectHost';
import {readMediaAsset} from '../../../apps/planner/src/services/mediaAssets';
import {readRunPortOutput} from '../../../apps/planner/src/services/runPortOutput';
import {mediaDataFields} from '../../../packages/presentation/src';
import type {ModulePackage} from '../../../packages/commands/generated/typescript/ModulePackage';
import type {RuntimeItem} from '../../../packages/commands/generated/typescript/RuntimeItem';
import type {JobState} from '../../../packages/commands/generated/typescript/JobState';
import type {ApiRequest} from '../../../packages/commands/generated/typescript/ApiRequest';
import assemblyJson from '../../../modules/media/p302-line-grade/installable/assembly/module.json';
import boxJson from '../../../modules/media/p302-line-grade/installable/control-box/module.json';
import cartonJson from '../../../modules/media/p302-line-grade/installable/carton/module.json';
import assemblyUrl from '../../../modules/media/p302-line-grade/installable/assembly/assembly.glb?url';
import boxUrl from '../../../modules/media/p302-line-grade/installable/control-box/control-box.glb?url';
import cartonUrl from '../../../modules/media/p302-line-grade/installable/carton/carton.glb?url';

type Bootstrap={snapshot:{project:FactoryPresentationContext['project']};formal_run:NonNullable<FactoryPresentationSource['run']>;context:{host:{session_id:string}}};
const ready=await fetch('/__factory_items_core?ready').then(r=>r.json()) as Bootstrap;
const rpc=(value:unknown)=>fetch('/__factory_items_core',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(value)}).then(r=>r.json());
const calls:unknown[]=[],itemsByTime=new Map<number,RuntimeItem[]>(),jobsByTime=new Map<number,JobState[]>();
let holdPort=false,holdBytes=false;const releases:(()=>void)[]=[];
const packages=[assemblyJson.data,boxJson.data,cartonJson.data] as unknown as ModulePackage[],assembly=packages[0],box=packages[1],carton=packages[2];
const assets=new Map([[assembly.assets.find(a=>a.kind==='glb')!.id,assemblyUrl],[box.assets.find(a=>a.kind==='glb')!.id,boxUrl],[carton.assets.find(a=>a.kind==='glb')!.id,cartonUrl]]);
(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={async invoke(command:string,args:{request:unknown}){
 const request=args.request;calls.push({command,request});
 if(command==='project_api')return rpc({kind:'dispatch',request});
 const reply=await rpc({kind:command==='project_read_media_asset'?'media':'port_output',...request as object});
 if(command==='project_read_run_port_output'&&holdPort){holdPort=false;await new Promise<void>(r=>releases.push(r));}
 if(!reply.ok)throw reply.error;
 return command==='project_read_media_asset'?Uint8Array.from(reply.bytes).buffer:reply.data;
}};
let latest:{scene:THREE.Scene;renderer:THREE.WebGLRenderer;camera:THREE.Camera}|null=null;
const before=THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender=function(renderer,scene,camera,geometry,material,group){latest={scene:scene as THREE.Scene,renderer,camera};return before.call(this,renderer,scene,camera,geometry,material,group);};
const originalRun=JSON.stringify(ready.formal_run);
const identity={unit:'meter' as const,coordinate_frame:'right_handed_z_up' as const,translation:{x:0,y:0,z:0},rotation:{x:0,y:0,z:0,w:1},scale:{x:1,y:1,z:1}};
declare global{interface Window{__items:{set:(value:Partial<State>)=>void;read:()=>unknown;capture:()=>unknown;pixelProbe:()=>unknown;holdPort:()=>void;holdBytes:()=>void;release:()=>void}}}
type State={at:number;probe:boolean;mounted:boolean;lod:'detailed'|'simplified'|'region'};
function App(){
 const [state,setState]=useState<State>({at:75000,probe:false,mounted:true,lod:'detailed'}),[source,setSource]=useState<FactoryPresentationSource|null>(null),[error,setError]=useState(''),[selected,setSelected]=useState<unknown>(null);
 const current=useRef<FactoryPresentationContext|null>(null);
 useEffect(()=>{
  const context:FactoryPresentationContext={session_id:ready.context.host.session_id,project_id:ready.snapshot.project.id,scope:{kind:'project'},scope_revision:ready.snapshot.project.revision,project:ready.snapshot.project,view:{run_id:ready.formal_run.id,at_ms:state.at}};
  current.current=context;setSource(null);setError('');
  const ports:FactoryPresentationPorts={async dispatch<T>(request:ApiRequest){
   if(state.probe&&request.operation==='get_object'&&request.params.object_id.kind==='module'&&request.params.source.kind==='run_module_version'){
    const wanted=request.params.source.data.module_ref,pkg=packages.find(p=>p.definition.id===wanted.id&&p.definition.version===wanted.version);if(pkg)return {kind:'module',data:pkg} as T;
   }
   const value=await dispatch<T>(request);
   if(request.operation==='get_run'&&request.params.query.kind==='objects_at'){
    const page=(value as unknown as {data:{at_ms:number;objects:{kind:string;data:RuntimeItem|JobState}[]}}).data;
    for(const object of page.objects){const map=object.kind==='item'?itemsByTime:object.kind==='job'?jobsByTime:null;if(map){const rows=map.get(page.at_ms)??[];if(!rows.some(row=>row.id===object.data.id))rows.push(object.data as never);map.set(page.at_ms,rows as never);}}
   }
   if(!state.probe)return value;
   const copied=structuredClone(value) as unknown as {kind:string;data:FactoryPresentationContext['project']&ModulePackage};
   if(request.operation==='get_object'&&copied.kind==='project'){
    const device=copied.data.devices.find(d=>d.id==='CNC');if(device&&assembly.definition.payload.category==='media'){const tags=device.presentation_override?.tags??[];device.presentation_override={...structuredClone(assembly.definition.payload.data),tags};}
    for(const type of copied.data.item_types)if(['raw','pcb','part'].includes(type.id))type.module_ref={id:'fixture.exact.item',version:'1.0.0'};
   }
   if(request.operation==='get_object'&&copied.kind==='module'&&copied.data.definition.id==='fixture.exact.item'&&copied.data.definition.payload.category==='items'&&box.definition.payload.category==='media')copied.data.definition.payload.data.presentation_or_media_ref={...structuredClone(box.definition.payload.data),source:{kind:'media_ref',data:{id:box.definition.id,version:box.definition.version}},lod:{...box.definition.payload.data.lod,detailed:null}};
   return copied as T;
  },async readAsset(request){
   if(state.probe&&assets.has(request.asset.id)){if(holdBytes)await new Promise<void>(r=>releases.push(r));const response=await fetch(assets.get(request.asset.id)!);return response.arrayBuffer();}
   return readMediaAsset(request);
  },readPortOutput:readRunPortOutput};
  if(state.probe)ports.readItemAttachments=async request=>{
   const items=(itemsByTime.get(request.at_ms)??[]).filter(item=>request.item_ids.includes(item.id)),jobs=jobsByTime.get(request.at_ms)??[];
   const bindings:FactoryItemAttachment[]=[];let index=0;
   for(const item of items){
    if(!['raw','pcb','part'].includes(item.item_type_ref.id)||item.lifecycle==='transformed')continue;
    if(item.holder.kind==='location'&&(item.item_type_ref.id!=='part'||item.composition_refs.length===0))continue;
    const offset=structuredClone(identity);offset.translation.x=item.item_type_ref.id==='part'?0:index++===0?-.25:.25;
    const actualJob=jobs.find(job=>job.id===item.holder.id&&job.execution_ref.kind==='project_device'&&job.execution_ref.data.device_id==='CNC');
    const target=item.holder.kind==='job'&&actualJob?{kind:'device_node' as const,device_id:'CNC',device_module_ref:ready.snapshot.project.devices.find(d=>d.id==='CNC')!.module_ref,media_ref:ready.snapshot.project.devices.find(d=>d.id==='CNC')!.module_ref,asset_ref:assembly.assets.find(a=>a.kind==='glb')!,node_index:352,offset}
     :item.holder.kind==='location'?{kind:'location' as const,location_id:item.holder.id,offset}:null;
    if(target)bindings.push({item_id:item.id,item_type_ref:item.item_type_ref,holder:item.holder,media_ref:{id:'fixture.exact.item',version:'1.0.0'},target});
   }
   return {...request,bindings};
  };
  let live=true;createFactoryPresentation(()=>current.current,ports).read(context).then(value=>{
   // Root multi-job source has no quantity declarations: add explicit fixture
   // labels to this read-only view clone; raw Host declarations stay untouched.
   if(!value.project.devices.some(d=>d.id==='CNC'))for(const device of value.devices)if(device.media&&device.port_outputs){const actual=value.project.devices.find(d=>d.id===device.device_id)!;
    const context={mode:'actual_device' as const,frame:{run:value.run!,snapshot:value.observation!,device:actual,port_outputs:device.port_outputs}};
    const tags=mediaDataFields(context).slice(5).map((field,i)=>({id:`explicit-port-${i}`,label:field.label,anchor:{kind:'viewport' as const,data:{x:.2,y:.1+i*.1}},payload:{kind:'data' as const,data:field.binding}}));device.media={...device.media,presentation:{...device.media.presentation,tags}};
   }
   if(live)setSource(value);
  },reason=>{if(live)setError(String(reason));});return()=>{live=false;current.current=null;};
 },[state.at,state.probe]);
 window.__items={set:patch=>setState(s=>({...s,...patch})),read:()=>({state,error,source,selected,calls,held:releases.length,runUnchanged:source?.run?originalRun===JSON.stringify(source.run):null}),holdPort:()=>{holdPort=true;},holdBytes:()=>{holdBytes=true;},release:()=>{holdPort=false;holdBytes=false;for(const r of releases.splice(0))r();},pixelProbe:()=>{
  if(!latest)return null;const {scene,renderer,camera}=latest,objects:THREE.Mesh[]=[];scene.traverse(o=>{if(o instanceof THREE.Mesh&&o.userData.ref?.kind==='item')objects.push(o);});
  if(!objects.length)return null;
  // Move only the test camera by declared meters; no geometry normalization.
  const aim=objects[0].getWorldPosition(new THREE.Vector3());camera.position.copy(aim).add(new THREE.Vector3(.9,.75,.9));camera.lookAt(aim);camera.updateMatrixWorld(true);
  const gl=renderer.getContext(),size=renderer.getDrawingBufferSize(new THREE.Vector2());
  const pixels=()=>{renderer.render(scene,camera);const data=new Uint8Array(size.x*size.y*4);gl.readPixels(0,0,size.x,size.y,gl.RGBA,gl.UNSIGNED_BYTE,data);return data;};
  const full=pixels(),differences:Record<string,number>={};
  for(const id of new Set(objects.map(o=>o.userData.ref.id as string))){const selected=objects.filter(o=>o.userData.ref.id===id);selected.forEach(o=>o.visible=false);const hidden=pixels();selected.forEach(o=>o.visible=true);let changed=0;for(let i=0;i<full.length;i+=4)if(full[i]!==hidden[i]||full[i+1]!==hidden[i+1]||full[i+2]!==hidden[i+2])changed++;differences[id]=changed;}
  renderer.render(scene,camera);return{differences,camera:camera.position.toArray(),geometry_scaled:false};
 },capture:()=>{
  if(!latest)return null;const meshes:unknown[]=[];latest.scene.updateMatrixWorld(true);latest.scene.traverse(object=>{if(object instanceof THREE.Mesh&&object.userData.ref?.kind==='item')meshes.push({id:object.userData.ref.id,name:object.name,world:object.getWorldPosition(new THREE.Vector3()).toArray(),size:new THREE.Box3().setFromObject(object).getSize(new THREE.Vector3()).toArray()});});
  latest.renderer.render(latest.scene,latest.camera);return{meshes,canvases:document.querySelectorAll('.factory-scene canvas').length,offscreen:document.querySelectorAll('body>div[aria-hidden="true"]').length};
 }};
 return <><p>真实同Host/默认Root端口。显式附件探针仅在测试内声明节点/媒体，不代表持久化空间生产者。{error}</p>{source&&state.mounted?<FactoryScene project={ready.snapshot.project} mediaSource={source} canvas="multi_hall_canvas" hallId={null} selection={null} onSelect={setSelected} lod={state.lod} autoLod={false} resetKey={0}/>:<p>读取实际来源</p>}</>;
}
const container=document.getElementById('root') as HTMLElement&{itemRoot?:Root};const root=container.itemRoot??(container.itemRoot=createRoot(container));root.render(<App/>);
if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();if(container.itemRoot===root)delete container.itemRoot;});
