import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {Scene,PerspectiveCamera,Object3D,Quaternion,Vector3} from 'three';
const root=new URL('../../../',import.meta.url),out=new URL('.work/p201-closeout/',root);await mkdir(out,{recursive:true});
await writeFile(new URL('layoutControls.mjs',out),stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/layoutControls.ts',root),'utf8')));
const {attachLayoutControls}=await import(new URL('layoutControls.mjs',out));
class Surface {
 constructor(){this.style={};this.listeners=new Map();this.captures=new Set();this.ownerDocument=this;this.pointerLockElement=null;}
 addEventListener(k,f,options){let a=this.listeners.get(k)??[];if(!a.some(x=>x.f===f))a.push({f,capture:options===true});this.listeners.set(k,a);}
 removeEventListener(k,f){this.listeners.set(k,(this.listeners.get(k)??[]).filter(x=>x.f!==f));}
 getRootNode(){return this;}getBoundingClientRect(){return {left:0,top:0,width:800,height:600};}
 setPointerCapture(id){this.captures.add(id);}hasPointerCapture(id){return this.captures.has(id);}releasePointerCapture(id){this.captures.delete(id);}
 fire(type,extra={}){let stopped=false;const e={type,button:0,pointerId:7,clientX:400,clientY:300,preventDefault(){},stopPropagation(){},stopImmediatePropagation(){stopped=true;},...extra};for(const {f} of [...(this.listeners.get(type)??[])].sort((a,b)=>Number(b.capture)-Number(a.capture))){f(e);if(stopped)break;}}
}
export function setup(commit=async()=>{},initial=null) {
 const canvas=new Surface(),scene=new Scene(),camera=new PerspectiveCamera(42,4/3,.01,1000),object=new Object3D(),orbit={enabled:true},calls=[];
 globalThis.document=canvas;
 object.position.set(2,3,4);object.quaternion.setFromAxisAngle(new Vector3(0,0,1),.2);object.updateMatrix();scene.add(object);camera.position.set(8,9,12);camera.lookAt(object.position);scene.updateMatrixWorld(true);camera.updateMatrixWorld(true);
 const original={translation:{x:2,y:3,z:4},rotation:{x:object.quaternion.x,y:object.quaternion.y,z:object.quaternion.z,w:object.quaternion.w},scale:{x:1,y:1,z:1}};
 if(initial){Object.assign(original,structuredClone(initial));object.position.set(...Object.values(initial.translation));object.quaternion.set(...Object.values(initial.rotation));object.scale.set(...Object.values(initial.scale));object.updateMatrix();}
 const handle=attachLayoutControls(scene,camera,canvas,orbit,object,original,{mode:'translate',snap:true,step_m:.25},()=>{},async pose=>{calls.push(pose);await commit(pose);},()=>{});
 const controls=scene.children.find(o=>o.isTransformControlsRoot).controls;
 function begin(){canvas.fire('pointerdown');controls.axis='X';controls._positionStart.copy(object.position);controls._quaternionStart.copy(object.quaternion);controls._scaleStart.copy(object.scale);controls.dragging=true;controls.dispatchEvent({type:'mouseDown',mode:'translate'});object.position.x+=5;object.quaternion.setFromAxisAngle(new Vector3(0,0,1),.6);controls.dispatchEvent({type:'objectChange'});}
 return {canvas,scene,camera,object,orbit,calls,controls,handle,original,begin};
}
