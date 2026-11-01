import {useEffect,useRef,useState} from 'react';import {createRoot} from 'react-dom/client';import * as THREE from 'three';
import {Workbench} from '../../../apps/planner/src/features/factory/Workbench';import {FactoryScene} from '../../../apps/planner/src/scene/FactoryScene';
import {createFactoryPresentation,type FactoryPresentationSource,type FactoryPresentationContext} from '../../../apps/planner/src/services/factoryPresentation';
import {createFactoryItemLayer} from '../../../apps/planner/src/scene/factoryItems';import {createFactoryMedia} from '../../../apps/planner/src/scene/factoryMedia';
import * as host from '../../../apps/planner/src/services/projectHost';import type {ApiRequest} from '../../../packages/commands/generated/typescript/ApiRequest';
import type {MediaAssetReadRequest} from '../../../apps/planner/src/services/mediaAssets';
import '../../../apps/planner/src/styles/workbench.css';
const ready=await fetch('/__layout_core').then(r=>r.json()),rpc=(q:unknown)=>fetch('/__layout_core',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(q)}).then(r=>r.json());
const calls:unknown[]=[],releases:(()=>void)[]=[];let hold=false,holdMeta=false;
(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={async invoke(command:string,args:{request:ApiRequest}){
 if(command==='project_select_directory')return rpc({kind:'select_save'});
 const request=args.request;calls.push({command,request});
 if(command==='project_api'){const value=await (request.operation==='get_task'?rpc({kind:'await_task',scope:request.scope,task_id:request.params.task_id}):rpc({kind:'dispatch',request}));if(holdMeta&&request.operation==='get_object'&&request.params.object_id.kind==='module'&&request.params.object_id.id==='layout.native.item'){holdMeta=false;await new Promise<void>(r=>releases.push(r));}return value;}
 const reply=await rpc({kind:command==='project_read_media_asset'?'media':'port_output',...request});
 if(hold&&command==='project_read_media_asset'){hold=false;await new Promise<void>(r=>releases.push(r));}
 if(!reply.ok)throw reply.error;return command==='project_read_media_asset'?Uint8Array.from(reply.bytes).buffer:reply.data;
}};
let latest:{scene:THREE.Scene;renderer:THREE.WebGLRenderer;camera:THREE.Camera}|null=null;
const old=THREE.Scene.prototype.onBeforeRender;THREE.Scene.prototype.onBeforeRender=function(renderer,scene,camera,geometry,material,group){latest={scene:scene as THREE.Scene,renderer,camera};return old.call(this,renderer,scene,camera,geometry,material,group);};
type State={mode:'workbench'|'scene'|'none';run:string;at:number;lod:'detailed'|'simplified'|'region'};
declare global{interface Window{__layout:any}}
function App(){
 const [state,setState]=useState<State>({mode:'workbench',run:ready.formal_run.id,at:0,lod:'detailed'}),[source,setSource]=useState<FactoryPresentationSource|null>(null),[error,setError]=useState('');
 const current=useRef<FactoryPresentationContext|null>(null);
 useEffect(()=>{let live=true;setSource(null);setError('');if(state.mode!=='scene')return;
 (async()=>{const snapshot=await host.readWorkspace();current.current={session_id:snapshot.context.host.session_id,project_id:snapshot.project!.id,scope:{kind:'project'},scope_revision:snapshot.project!.revision,project:snapshot.project!,view:{run_id:state.run,at_ms:state.at}};const value=await createFactoryPresentation(()=>current.current).read(current.current);if(live)setSource(value);})().catch(e=>{if(live)setError(String(e));});return()=>{live=false;current.current=null;};
 },[state.mode,state.run,state.at]);
 window.__layout={set:(p:Partial<State>)=>setState(v=>({...v,...p})),read:()=>({state,source,error,calls,held:releases.length}),ready,host,rpc,hold:(kind='bytes')=>{if(kind==='metadata')holdMeta=true;else hold=true;},release:()=>{hold=false;holdMeta=false;for(const r of releases.splice(0))r();},async layerProof(){
 if(!source)throw new Error('Requires actual frozen source');
 const plain=(layer:ReturnType<typeof createFactoryItemLayer>,settled=false)=>{const r=layer.renderState(settled);return{pending:r.pending,objects:r.objects.map(v=>v.ref),failures:r.failures,unavailable:r.unavailable};},settle=async(check:()=>boolean)=>{for(let i=0;i<200&&!check();i++)await new Promise(r=>setTimeout(r,5));if(!check())throw new Error('layer probe failed to settle');};
 const item=source.items.find(i=>i.attachment?.owner_ref.kind==='device'&&i.media);if(!item)throw new Error('Requires actual device-holder item');const device=source.project.devices.find(d=>d.id===item.attachment!.owner_ref.id)!,input=source.devices.find(d=>d.device_id===device.id)!;
 let owner:Awaited<ReturnType<typeof createFactoryMedia>>|null=null,changed=0;const layer=createFactoryItemLayer([item],'detailed',()=>owner,()=>null,()=>changed++);layer.reconcile();const waiting=plain(layer),missing=plain(layer,true);owner=await createFactoryMedia(device,input,'detailed');layer.reconcile();const loading=plain(layer,true);await settle(()=>layer.renderState(true).objects.length===1);const attached=plain(layer,true);layer.dispose();const disposed=plain(layer,true);owner.dispose();
 let release!:()=>void,entered=false;const gate=new Promise<void>(r=>release=r);const held={...item,media:{...item.media!,resolveModule:async(request:MediaAssetReadRequest['source'])=>{entered=true;await gate;return item.media!.resolveModule!(request);}}};
 owner=await createFactoryMedia(device,input,'detailed');const late=createFactoryItemLayer([held],'detailed',()=>owner,()=>null,()=>changed++);late.reconcile();await settle(()=>entered);const latePending=plain(late,true);late.dispose();release();await new Promise(r=>setTimeout(r,20));const lateDisposed=plain(late,true);owner.dispose();
 const failed={...item,media:{...item.media!,resolveModule:async()=>{throw new Error('explicit actual-metadata port rejection');}}};owner=await createFactoryMedia(device,input,'detailed');const failure=createFactoryItemLayer([failed],'detailed',()=>owner,()=>null,()=>changed++);failure.reconcile();await settle(()=>failure.renderState(true).failures.length===1);const rejected=plain(failure,true);failure.dispose();owner.dispose();
 const result={waiting,missing,loading,attached,disposed,latePending,lateDisposed,rejected,changed};return result;
 },async parentFailureProof(){
 if(!source)throw new Error('Requires actual source');const actual=source.items.filter(i=>i.attachment&&(i.attachment.owner_ref.kind==='location'||i.attachment.owner_ref.kind==='item'));
 const inputs=actual.map(i=>i.attachment!.owner_ref.kind==='location'?{...i,media:{...i.media!,resolveModule:async()=>{throw new Error('explicit parent decoder/metadata rejection');}}}:i),location=new THREE.Group();let changed=0;
 const layer=createFactoryItemLayer(inputs,'simplified',()=>null,()=>location,()=>changed++);layer.reconcile();for(let i=0;i<200&&layer.renderState(true).pending;i++)await new Promise(r=>setTimeout(r,5));const state=layer.renderState(true);const result={pending:state.pending,objects:state.objects.map(o=>o.ref),failures:state.failures,unavailable:state.unavailable,changed};layer.dispose();return result;
 },capture:()=>{
 if(!latest)return null;latest.scene.updateMatrixWorld(true);const meshes:unknown[]=[];latest.scene.traverse(o=>{if(o instanceof THREE.Mesh){const id=o.userData.ref;meshes.push({ref:id,name:o.name,world:o.getWorldPosition(new THREE.Vector3()).toArray(),size:new THREE.Box3().setFromObject(o).getSize(new THREE.Vector3()).toArray(),parent:o.parent?.name,chain:(()=>{const a=[];let p:THREE.Object3D|null=o;while(p){a.push({name:p.name,position:p.position.toArray(),scale:p.scale.toArray()});p=p.parent;}return a;})()});}});return{meshes,canvases:document.querySelectorAll('.factory-scene canvas').length};
 },pixels:(itemId:string)=>{
 if(!latest)return null;const {scene,renderer,camera}=latest,objects:THREE.Object3D[]=[];scene.traverse(o=>{if(o.userData.ref?.kind==='item'&&o.userData.ref.id===itemId&&o instanceof THREE.Mesh)objects.push(o);});if(!objects.length)return null;
 const aim=objects[0].getWorldPosition(new THREE.Vector3());camera.position.copy(aim).add(new THREE.Vector3(.8,.6,.8));camera.lookAt(aim);camera.updateMatrixWorld(true);const gl=renderer.getContext(),size=renderer.getDrawingBufferSize(new THREE.Vector2());const pixels=()=>{renderer.render(scene,camera);const a=new Uint8Array(size.x*size.y*4);gl.readPixels(0,0,size.x,size.y,gl.RGBA,gl.UNSIGNED_BYTE,a);return a;};const full=pixels();objects.forEach(o=>o.visible=false);const hidden=pixels();objects.forEach(o=>o.visible=true);let difference=0;for(let i=0;i<full.length;i+=4)if(full[i]!==hidden[i]||full[i+1]!==hidden[i+1]||full[i+2]!==hidden[i+2])difference++;renderer.render(scene,camera);return{difference,geometry_scaled:false};
 }};
 return state.mode==='workbench'?<Workbench/>:state.mode==='scene'?<><p>同一真实Host、原唯一FactoryScene。{error}</p>{source?<FactoryScene project={source.project} mediaSource={source} run={source.run} observation={source.observation} canvas="multi_hall_canvas" hallId={null} selection={null} onSelect={()=>{}} lod={state.lod} autoLod={false} resetKey={0}/>:<p>读取冻结展示槽</p>}</>:<p>已卸载</p>;
}
const container=document.getElementById('root') as HTMLElement&{layoutRoot?:ReturnType<typeof createRoot>};const root=container.layoutRoot??(container.layoutRoot=createRoot(container));root.render(<App/>);
if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();if(container.layoutRoot===root)delete container.layoutRoot;});
