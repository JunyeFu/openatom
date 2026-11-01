/// <reference types="vite/client" />
import {createRoot} from 'react-dom/client';import * as THREE from 'three';import {Workbench} from '../../../apps/planner/src/features/factory/Workbench';import type {ApiRequest} from '../../../packages/commands/generated/typescript/ApiRequest';import '../../../apps/planner/src/styles/workbench.css';
const ready=await fetch('/__p201_core').then(r=>r.json()),rpc=(q:unknown)=>fetch('/__p201_core',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(q)}).then(r=>r.json());
let openDirectory:string|null=null;
const calls:unknown[]=[],releases:(()=>void)[]=[];let hold=false;
(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>1,unregisterCallback:()=>{},async invoke(command:string,args:{request:ApiRequest}){
 if(command.startsWith('plugin:event|')) return 1;
 if(command==='project_select_directory'){if((args as unknown as {purpose:string}).purpose==='import_package'){if(!openDirectory)throw new Error('Test import directory was not selected');return rpc({kind:'select_open',directory:openDirectory});}return rpc({kind:'select_save'});}
 const request=args.request;calls.push({command,request});
 if(command==='project_api'){const reply=await (request.operation==='get_task'?rpc({kind:'await_task',scope:request.scope,task_id:request.params.task_id}):rpc({kind:'dispatch',request}));if(hold&&request.operation==='get_object'&&request.params.node_index!==null){hold=false;await new Promise<void>(r=>releases.push(r));}return reply;}
 const reply=await rpc({kind:command==='project_read_media_asset'?'media':'port_output',...request});if(!reply.ok)throw reply.error;return command==='project_read_media_asset'?Uint8Array.from(reply.bytes).buffer:reply.data;
}};
let latest:{scene:THREE.Scene;renderer:THREE.WebGLRenderer;camera:THREE.Camera}|null=null;const prior=THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender=function(renderer,scene,camera,geometry,material,group){latest={scene:scene as THREE.Scene,renderer,camera};return prior.call(this,renderer,scene,camera,geometry,material,group);};
declare global{interface Window{__p201:any}}
window.__p201={ready,rpc,setOpenDirectory:(value:string)=>{openDirectory=value;},scene:()=>latest,read:()=>({calls,held:releases.length}),hold:()=>{hold=true;},release:()=>{hold=false;for(const r of releases.splice(0))r();},point:(match='right')=>{
 if(!latest)throw new Error('No actual rendered scene');const {scene,renderer,camera}=latest,meshes:THREE.Mesh[]=[];scene.updateMatrixWorld(true);scene.traverse(o=>{if(o instanceof THREE.Mesh)meshes.push(o);});const object=meshes.find(o=>match.startsWith('name:')?o.name===match.slice(5):match==='right'?o.getWorldPosition(new THREE.Vector3()).x>0:o.getWorldPosition(new THREE.Vector3()).x<0);if(!object)throw new Error('Requested real mesh missing: '+JSON.stringify(meshes.map(m=>m.name)));const point=object.getWorldPosition(new THREE.Vector3()).project(camera),rect=renderer.domElement.getBoundingClientRect();return{x:rect.x+(point.x+1)/2*rect.width,y:rect.y+(1-point.y)/2*rect.height,name:object.name,world:object.getWorldPosition(new THREE.Vector3()).toArray()};
}};
const container=document.getElementById('root') as HTMLElement&{nodeRoot?:ReturnType<typeof createRoot>};const root=container.nodeRoot??(container.nodeRoot=createRoot(container));root.render(<Workbench/>);if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();if(container.nodeRoot===root)delete container.nodeRoot;});
