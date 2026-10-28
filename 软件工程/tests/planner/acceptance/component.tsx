// Limited actual-component consumer. There is no Tauri/Host substitute here.
import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import * as THREE from 'three';
import {OrbitControls} from 'three/addons/controls/OrbitControls.js';
import {FactoryScene} from '../../../apps/planner/src/scene/FactoryScene';
import {RunTimeline} from '../../../apps/planner/src/features/timeline/RunTimeline';
import '../../../apps/planner/src/styles/workbench.css';
import input from './presentation-input.json';
import actual from '../../../packages/commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json';

let scene:THREE.Scene|null=null;
let controls:OrbitControls|null=null;
// Passive observation of real Three objects; constructors, control behavior and
// producer rendering are unchanged. This is never evidence of the Native UI.
const add=THREE.Object3D.prototype.add;
THREE.Object3D.prototype.add=function(...objects){const result=add.apply(this,objects);if(this instanceof THREE.Scene)scene=this;return result;};
const listen=OrbitControls.prototype.addEventListener;
OrbitControls.prototype.addEventListener=function(...args){controls=this;return listen.apply(this,args);};
function mesh(id:string){let result:THREE.Mesh|undefined;scene?.traverse(o=>{if(o instanceof THREE.Mesh&&o.userData.ref?.id===id)result=o;});if(!result)throw Error('Actual rendered mesh missing '+id);return result;}
function freeze(value:any):any{if(value&&typeof value==='object'){Object.values(value).forEach(freeze);Object.freeze(value);}return value;}
const initial=freeze(input.project);
let current=initial;
let selected:any=null;
function read(){
 scene?.updateMatrixWorld(true);
 const items:any[]=[];
 scene?.traverse(o=>{if(o instanceof THREE.Mesh&&o.userData.ref){o.geometry.computeBoundingBox();items.push({ref:o.userData.ref,visible:o.visible,world:new THREE.Vector3().setFromMatrixPosition(o.matrixWorld).toArray(),box:o.geometry.boundingBox?.getSize(new THREE.Vector3()).toArray(),geometry:o.geometry.type});}});
 return {camera:controls?.object.position.toArray(),target:controls?.target.toArray(),items,selected,input:JSON.stringify(current)};
}
function area(id:string){const m=mesh(id);const positions=m.geometry.getAttribute('position');const index=m.geometry.index;let total=0;const count=index?.count??positions.count;for(let i=0;i<count;i+=3){const n=[0,1,2].map(j=>index?index.getX(i+j):i+j);const p=n.map(j=>new THREE.Vector2(positions.getX(j),positions.getY(j)));total+=Math.abs((p[1].x-p[0].x)*(p[2].y-p[0].y)-(p[1].y-p[0].y)*(p[2].x-p[0].x))/2;}return total;}
function screenPoint(id:string){scene?.updateMatrixWorld(true);const m=mesh(id);const p=m.geometry.getAttribute('position');const ix=m.geometry.index;const point=new THREE.Vector3();for(let j=0;j<3;j++){const n=ix?ix.getX(j):j;point.add(new THREE.Vector3(p.getX(n),p.getY(n),p.getZ(n)));}point.divideScalar(3).applyMatrix4(m.matrixWorld).project(controls!.object);const rect=document.querySelector('canvas')!.getBoundingClientRect();return {x:rect.left+(point.x+1)*rect.width/2,y:rect.top+(1-point.y)*rect.height/2};}
function App(){
 const [project,setProject]=useState(initial);
 const [canvas,setCanvas]=useState<'line_canvas'|'multi_hall_canvas'>('line_canvas');
 const [lod,setLod]=useState<'detailed'|'simplified'|'region'>('detailed');
 const [reset,setReset]=useState(0);
 const [selection,setSelection]=useState(null);
 const partial=new URLSearchParams(location.search).get('mode')==='timeline';
 const [at,setAt]=useState<number|null>(null);
 current=project;
 Object.assign(window,{__acceptance:{read,area,screenPoint,initial:JSON.stringify(initial),moveGroup(){const next=structuredClone(initial);next.lines[0].transform.translation={x:18,y:16,z:3};setProject(freeze(next));}}});
 return <main style={{height:'100dvh',display:'flex',flexDirection:'column'}}><header style={{padding:8,display:'flex',gap:8}}><strong>实际组件有限消费者</strong><button onClick={()=>setCanvas('line_canvas')}>线内</button><button onClick={()=>setCanvas('multi_hall_canvas')}>跨厂</button>{(['detailed','simplified','region'] as const).map(x=><button key={x} onClick={()=>setLod(x)}>{x}</button>)}<button onClick={()=>setReset(x=>x+1)}>复位</button><output data-testid="selection">{JSON.stringify(selected)}</output></header>
 {partial?<div style={{height:300}}><RunTimeline project={actual.quality_partial[0] as any} run={actual.quality_partial[3] as any} atMs={at} onTime={setAt} onSelect={value=>{selected=value;setSelection(value as any);}}/></div>:<div style={{position:'relative',flex:1,minHeight:0}}><FactoryScene project={project as any} canvas={canvas} hallId={project.halls[0].id} selection={selection} onSelect={value=>{selected=value;setSelection(value as any);}} lod={lod} autoLod={false} resetKey={reset}/></div>}
 </main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
