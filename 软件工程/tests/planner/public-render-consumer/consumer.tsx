import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import * as THREE from 'three';
import {RenderTaskSurface} from '../../../apps/planner/src/features/render-surface/RenderTaskSurface';
import type {NativeRenderInput,RenderTaskPorts} from '../../../apps/planner/src/services/renderTask';
import '../../../apps/planner/src/styles/workbench.css';
const initial=await fetch('/__render_task_fixture').then(r=>r.json()) as NativeRenderInput;
const calls:{command:string;args?:unknown;status:string}[]=[],ready:unknown[]=[],failures:unknown[]=[],stored:unknown[]=[],held:(()=>void)[]=[];
let hold=false,refuse=false,alive=true;
const scenes:{camera:THREE.Camera;scene:THREE.Scene}[]=[];
const original=THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender=function(renderer,scene,camera,geometry,material,group){if(!scenes.some(value=>value.scene===scene))scenes.push({scene,camera});Reflect.apply(original,this,[renderer,scene,camera,geometry,material,group]);};
const get=Storage.prototype.getItem,set=Storage.prototype.setItem;
Storage.prototype.getItem=function(key){stored.push({kind:'get',key});return Reflect.apply(get,this,[key]);};
Storage.prototype.setItem=function(key,value){stored.push({kind:'set',key});return Reflect.apply(set,this,[key,value]);};
async function binary(command:string,args:unknown){const call={command,args,status:'pending'};calls.push(call);if(hold)await new Promise<void>(resolve=>held.push(resolve));if(refuse){call.status='rejected';throw new Error('observed-test real resource refusal');}const response=await fetch('/__render_task_bytes',{method:'POST',body:JSON.stringify({command,args})});if(!response.ok){call.status='rejected';throw new Error(await response.text());}const bytes=await response.arrayBuffer();call.status='returned';return bytes;}
const ports:RenderTaskPorts={input:async()=>initial,asset:request=>binary('native_render_asset',{request}),file:request=>binary('native_render_file',{request}),ready:async request=>{calls.push({command:'native_render_ready',args:request,status:'returned'});ready.push(request);},failure:async request=>{calls.push({command:'native_render_failure',args:request,status:'returned'});failures.push(request);}};
function Harness(){const [task,setTask]=useState<NativeRenderInput|null>(initial);window.__renderConsumer={set(value){ready.length=0;failures.length=0;setTask(value);},hold(value){hold=value;},reject(value){refuse=value;},release(){hold=false;held.splice(0).forEach(resolve=>resolve());},geometry(){return scenes.map(value=>{const nodes:{name:string;position:number[];world:number[]}[]=[];value.scene.traverse(object=>{if(object instanceof THREE.Mesh)nodes.push({name:object.name,position:object.position.toArray(),world:object.getWorldPosition(new THREE.Vector3()).toArray()});});return nodes;});},read(){return {calls,ready,failures,stored,held:held.length,alive,scenes:scenes.map(value=>{const camera=value.camera as THREE.PerspectiveCamera;const refs:string[]=[];let meshes=0;value.scene.traverse(object=>{if(object instanceof THREE.Mesh){meshes++;if(object.userData.ref)refs.push(JSON.stringify(object.userData.ref));}});return {eye:camera.position.toArray(),zoom:camera.zoom,projection:camera.projectionMatrix.elements,meshes,refs};})};}};return task?<RenderTaskSurface task={task} ports={ports}/>:null;}
declare global {interface Window {__renderConsumer:{set(value:NativeRenderInput|null):void;hold(value:boolean):void;reject(value:boolean):void;release():void;read():unknown;geometry():unknown}}}
window.addEventListener('pagehide',()=>{alive=false;});
const container=document.getElementById('root') as HTMLElement&{renderConsumerRoot?:ReturnType<typeof createRoot>};
const reactRoot=container.renderConsumerRoot??(container.renderConsumerRoot=createRoot(container));reactRoot.render(<Harness/>);
if(import.meta.hot)import.meta.hot.dispose(()=>{reactRoot.unmount();if(container.renderConsumerRoot===reactRoot)delete container.renderConsumerRoot;THREE.Scene.prototype.onBeforeRender=original;Storage.prototype.getItem=get;Storage.prototype.setItem=set;});
