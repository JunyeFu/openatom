/// <reference types="vite/client" />
import {useEffect,useRef,useState} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import * as THREE from 'three';
import {FactoryScene} from '../../../apps/planner/src/scene/FactoryScene';
import type {FactoryPresentationContext,FactoryPresentationSource} from '../../../apps/planner/src/services/factoryPresentation';
import type {ProcessReadContext,ProcessScope} from '../../../apps/planner/src/services/presentationHost';
import {dispatch,type WorkspaceSnapshot} from '../../../apps/planner/src/services/projectHost';
import {readMediaAsset} from '../../../apps/planner/src/services/mediaAssets';
import type {ModuleRef} from '../../../packages/commands/generated/typescript/ModuleRef';
import type {ModulePackage} from '../../../packages/commands/generated/typescript/ModulePackage';
import type {ProjectContext} from '../../../packages/commands/generated/typescript/ProjectContext';
import type {ApiRequest} from '../../../packages/commands/generated/typescript/ApiRequest';

const red=new URLSearchParams(location.search).get('stage')==='red';
const {DeviceProcessWorkspace}=await (red?import('./DeviceProcessWorkspace.before'):import('../../../apps/planner/src/features/factory/DeviceProcessWorkspace'));
const {createFactoryPresentation}=await (red?import('./factoryPresentation.before'):import('../../../apps/planner/src/services/factoryPresentation'));
const {createPresentationHost}=await (red?import('./presentationHost.before'):import('../../../apps/planner/src/services/presentationHost'));

const ready=await fetch('/__adoption_core?ready').then(r=>r.json());
const deviceRef:ModuleRef={id:ready.package.definition.id,version:ready.package.definition.version};
const rpc=(q:unknown)=>fetch('/__adoption_core',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(q)}).then(r=>r.json());
const calls:unknown[]=[],releases:(()=>void)[]=[];let holdMeta=false,holdBytes=false;
(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={async invoke(command:string,args:{request:ApiRequest}){
 const request=args.request;calls.push({command,request});
 if(command==='project_api'){
  const reply=await rpc({kind:'dispatch',request});
  if(holdMeta&&request.operation==='get_object'&&request.params.object_id.kind==='module'&&request.params.object_id.id.startsWith('consumer.')){holdMeta=false;await new Promise<void>(r=>releases.push(r));}
  return reply;
 }
 const reply=await rpc({kind:command==='project_read_media_asset'?'media':'port_output',...request});
 if(holdBytes&&command==='project_read_media_asset'){holdBytes=false;await new Promise<void>(r=>releases.push(r));}
 if(!reply.ok)throw reply.error;return command==='project_read_media_asset'?Uint8Array.from(reply.bytes).buffer:reply.data;
}};
let scene:THREE.Scene|null=null,draw:{renderer:THREE.WebGLRenderer;camera:THREE.Camera}|null=null;const old=THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender=function(renderer,current,camera,geometry,material,group){scene=current as THREE.Scene;draw={renderer,camera};return old.call(this,renderer,current,camera,geometry,material,group);};
type State={scope:'project'|'draft';view:'process'|'factory';actual:boolean;at:number;lod:'detailed'|'simplified'|'region';deviceId?:string};
declare global {interface Window{__adoption:{set:(s:Partial<State>)=>void;read:()=>unknown;adopt:(version:string,image?:boolean)=>Promise<void>;sources:()=>Promise<unknown>;denials:()=>Promise<unknown>;currentInstanceProof:()=>Promise<unknown>;hold:(kind:'metadata'|'bytes')=>void;release:()=>void;request:(request:ApiRequest)=>Promise<unknown>;capture:()=>unknown}}}
function App(){
 const [state,setState]=useState<State>({scope:'project',view:'process',actual:true,at:0,lod:'detailed'}),[revision,setRevision]=useState(0),[snapshot,setSnapshot]=useState(ready.snapshot),[factory,setFactory]=useState<FactoryPresentationSource|null>(null),[error,setError]=useState('');
 const current=useRef<FactoryPresentationContext|null>(null),scope:ProcessScope['scope']=state.scope==='project'?{kind:'project'}:{kind:'definition_draft',data:ready.draft.summary.draft_id};
 const scopeRevision=state.scope==='project'?snapshot.project.revision:ready.draft.summary.revision;
 const workspace:WorkspaceSnapshot={context:ready.project_context,capabilities:ready.capabilities,project:snapshot.project,modules:new Map(),moduleIssues:[],scenarios:snapshot.scenarios,scenarioIssues:[]};
 const context:ProcessReadContext={session_id:ready.context.host.session_id,project_id:snapshot.project.id,scope,scope_revision:scopeRevision,selection:state.actual?{kind:'actual_run',run_id:ready.formal_run.id,job_id:ready.formal_run.jobs[0].id,at_ms:state.at}:{kind:'definition',module_ref:deviceRef,...(scope.kind==='project'?{device_id:state.deviceId??snapshot.project.devices[0].id}:{})}};
 useEffect(()=>{
  current.current={...context,project:snapshot.project,view:state.actual?{run_id:ready.formal_run.id,at_ms:state.at}:null};setFactory(null);setError('');
  if(state.view!=='factory')return;
  let live=true;createFactoryPresentation(()=>current.current).read(current.current).then(v=>{if(live)setFactory(v);},e=>{if(live)setError(String(e));});return()=>{live=false;current.current=null;};
 },[state.scope,state.actual,state.at,state.view,snapshot,revision]);
 const request=(request:ApiRequest)=>dispatch(request);
 window.__adoption={set:p=>setState(s=>({...s,...p})),read:()=>({state,snapshot,factory,error,calls,held:releases.length,ready}),request,
  async adopt(version,image=false){await request({api_version:'0.1',request_id:'actual-media-adopt',project_id:snapshot.project.id,scope:{kind:'project'},operation:'apply_commands',params:{base_revision:snapshot.project.revision,commands:[{kind:'adopt_module',data:{module_ref:{id:image?'consumer.adopted.image':'consumer.adopted.media',version},targets:[{kind:'device',id:snapshot.project.devices[0].id}],bindings:{item_types:{},resources:{},targets:{},ports:{}},new_instances:[],explicit_seed:null}}]}});setSnapshot(await rpc({kind:'snapshot'}));setRevision(r=>r+1);},
  async sources(){const active={...context,scope:{kind:'definition_draft' as const,data:ready.draft.summary.draft_id},scope_revision:ready.draft.summary.revision,selection:{kind:'definition' as const,module_ref:deviceRef}};const h=createPresentationHost(()=>active),v=await h.read(active),reader=h.media(active,v.binding);const refs:ModuleRef[]=[deviceRef,{id:'consumer.adopted.media',version:'1.0.0'},{id:'consumer.media.bridge',version:'1.0.0'},{id:'consumer.media.sequence',version:'1.0.0'}];const found:unknown[]=[];for(const module_ref of refs){const p=await reader.resolveModule({kind:'module_version',data:{module_ref}});let bytes=0;if(p.assets.length)bytes=(await reader.readAsset({...v.binding.request,source:{kind:'module_version',data:{module_ref}},asset:p.assets[0]})).byteLength;found.push({ref:module_ref,definition:p.definition,bytes,rights:p.rights});}return{found,scope:active.scope,scopeRevision:active.scope_revision};},
  async currentInstanceProof(){
   const selected={kind:'definition' as const,module_ref:deviceRef,device_id:snapshot.project.devices[0].id},base:ProcessReadContext={...context,scope:{kind:'project'},scope_revision:snapshot.project.revision,selection:selected};
   const guards:unknown[]=[];
   for(const candidate of [{...base,scope:{kind:'definition_draft' as const,data:ready.draft.summary.draft_id}},{...base,selection:{...selected,device_id:'missing-real-device'}},{...base,selection:{...selected,module_ref:{...deviceRef,version:'9.0.0'}}},{...base,scope_revision:base.scope_revision+1}])try{await createPresentationHost(()=>candidate).read(candidate);guards.push('unexpected_success');}catch(e){guards.push({name:(e as Error).name,message:(e as Error).message,detail:(e as {detail?:unknown}).detail});}
   let live=base,entered!:()=>void,release!:()=>void;const entry=new Promise<void>(r=>entered=r),gate=new Promise<void>(r=>release=r);
   const h=createPresentationHost(()=>live,{async dispatch<T>(q:ApiRequest){const v=await dispatch<T>(q);if(q.operation==='get_object'&&q.params.object_id.kind==='device'){entered();await gate;}return v;},readAsset:readMediaAsset});
   const pending=h.read(base);await entry;live={...base,selection:{...selected,device_id:'other-instance-same-module'}};release();try{await pending;guards.push('unexpected_success');}catch(e){guards.push({name:(e as Error).name,message:(e as Error).message});}
   // Change the actual mutable instance head after the first real context read,
   // before get_object(device). The reader must reject the new override at old R
   // even before a GLB/image/native-scene byte path can be involved.
   let changed=false;
   const changing=createPresentationHost(()=>base,{async dispatch<T>(q:ApiRequest){if(!changed&&q.operation==='get_object'&&q.params.object_id.kind==='device'){changed=true;await request({api_version:'0.1',request_id:'actual-midread-adoption',project_id:snapshot.project.id,scope:{kind:'project'},operation:'apply_commands',params:{base_revision:base.scope_revision,commands:[{kind:'adopt_module',data:{module_ref:{id:'consumer.adopted.media',version:'1.0.0'},targets:[{kind:'device',id:selected.device_id}],bindings:{item_types:{},resources:{},targets:{},ports:{}},new_instances:[],explicit_seed:null}}]}});}return dispatch<T>(q);},readAsset:readMediaAsset});
   try{await changing.read(base);guards.push('unexpected_success');}catch(e){guards.push({name:(e as Error).name,message:(e as Error).message});}setSnapshot(await rpc({kind:'snapshot'}));
   return{guards,exact_device_id:selected.device_id,module_ref:deviceRef};
  },async denials(){const scope={kind:'definition_draft' as const,data:ready.draft.summary.draft_id},source={kind:'module_version' as const,data:{module_ref:{id:'consumer.adopted.media',version:'1.0.0'}}},base={api_version:'0.1' as const,request_id:'exact-draft-denied',project_id:snapshot.project.id,scope};const errors:unknown[]=[];
   for(const [id,version]of [['consumer.adopted.media','2.0.0'],['wrong-object','1.0.0']])try{await request({...base,operation:'get_object',params:{object_id:{kind:'module',id},source:{kind:'module_version',data:{module_ref:{id:'consumer.adopted.media',version}}},node_index:null}});errors.push('unexpected_success');}catch(e){errors.push((e as {detail?:unknown}).detail??String(e));}
   const pkg=(await request({...base,operation:'get_object',params:{object_id:{kind:'module',id:'consumer.adopted.media'},source,node_index:null}})) as {kind:'module';data:ModulePackage};
   try{await readMediaAsset({project_id:snapshot.project.id,scope,scope_revision:ready.draft.summary.revision+1,source,asset:pkg.data.assets[0]});errors.push('unexpected_success');}catch(e){errors.push((e as {detail?:unknown}).detail??String(e));}return errors;
  },hold:kind=>{if(kind==='metadata')holdMeta=true;else holdBytes=true;},release:()=>{holdMeta=false;holdBytes=false;for(const r of releases.splice(0))r();},capture:()=>{let meshes=0;const names:string[]=[],textures:unknown[]=[];if(scene&&draw)draw.renderer.render(scene,draw.camera);scene?.traverse(o=>{if(o instanceof THREE.Mesh){meshes++;names.push(o.name);const material=Array.isArray(o.material)?o.material[0]:o.material,map=(material as THREE.MeshBasicMaterial).map,image=map?.image as HTMLCanvasElement|undefined;if(image?.getContext&&draw){const c=image.getContext('2d'),rgba=c?.getImageData(image.width/2,image.height/2,1,1).data,point=o.getWorldPosition(new THREE.Vector3()).project(draw.camera),size=draw.renderer.getDrawingBufferSize(new THREE.Vector2()),gl=draw.renderer.getContext(),gpu=new Uint8Array(4);gl.readPixels(Math.round((point.x+1)/2*size.x),Math.round((1+point.y)/2*size.y),1,1,gl.RGBA,gl.UNSIGNED_BYTE,gpu);textures.push({id:o.userData.ref?.id,rgba:rgba?[...rgba]:null,gpu:[...gpu]});}}});return{meshes,names,textures,canvases:document.querySelectorAll('canvas').length};}};
 return <><p>同一实际 ProjectHost；从原 Device.module_ref + 实际 adopt override 起步。测试传输不是 Native IPC。{error}</p>{state.view==='process'?<DeviceProcessWorkspace key={state.scope+state.actual+revision} workspace={workspace} scope={scope} scopeRevision={scopeRevision} selection={context.selection} actions={{}} onClose={()=>{}}/>:factory?<FactoryScene project={snapshot.project} mediaSource={factory} canvas="multi_hall_canvas" hallId={null} selection={null} onSelect={()=>{}} lod={state.lod} autoLod={false} resetKey={0}/>:<p>读取主画布</p>}</>;
}
const container=document.getElementById('root') as HTMLElement&{adoptionRoot?:Root};const root=container.adoptionRoot??(container.adoptionRoot=createRoot(container));root.render(<App/>);
if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();if(container.adoptionRoot===root)delete container.adoptionRoot;});
