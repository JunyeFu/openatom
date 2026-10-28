import * as T from 'three';
import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
export const V=a=>new T.Vector3(...a);
const material=(name,color,metalness,roughness,extra={})=>new T.MeshStandardMaterial({name,color,metalness,roughness,...extra});
export function geometry(simple=false){
 const m={steel:material('Brushed stainless','#afbcc5',.78,.32),al:material('Anodised aluminium','#c6cdd2',.72,.35),blue:material('Powder coat blue','#306d8c',.32,.42),dark:material('Graphite cast iron','#243542',.50,.44),rubber:material('Rubber','#17272e',.02,.83),belt:material('Polyurethane belt','#328976',.08,.64),amber:material('Safety ochre','#e6ad45',.22,.42),red:material('Emergency red','#ce514b',.05,.40),green:material('Status green','#4eae90',.08,.4),lens:material('Optical glass','#2c536c',.55,.18),glass:material('Guard window','#809eab',.15,.22,{transparent:true,opacity:.25}),label:material('LABEL_ATLAS','#ffffff',0,.75),paper:material('Uncoated corrugated board','#bba078',0,.88),edge:material('Cardboard scored edge','#92764d',0,.9),glue:material('Applied hot melt','#d7ad56',0,.52),tape:material('Pressure sensitive kraft tape','#bb8d54',0,.58)};
 const cache=new Map(),get=(key,fn)=>{if(!cache.has(key))cache.set(key,fn());return cache.get(key);};
 const group=(p,n,pos=[0,0,0])=>{const o=new T.Group();o.name=n;o.position.fromArray(pos);p.add(o);return o;};
 const mesh=(p,n,g,pos,mat=m.steel)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(pos);o.castShadow=o.receiveShadow=true;p.add(o);return o;};
 const box=(p,n,d,pos,mat=m.steel,round=true)=>mesh(p,n,get('b'+[...d,round],()=>simple||!round?new T.BoxGeometry(...d):new RoundedBoxGeometry(...d,2,Math.min(.003,...d.map(v=>v*.14)))),pos,mat);
 const cyl=(p,n,r,h,pos,mat=m.steel,axis='y',r2=r)=>{const o=mesh(p,n,get('c'+[r,h,r2],()=>new T.CylinderGeometry(r2,r,h,simple?12:24)),pos,mat);if(axis==='x')o.rotation.z=Math.PI/2;if(axis==='z')o.rotation.x=Math.PI/2;return o;};
 const rod=(p,n,a,b,r=.006,mat=m.steel)=>{const d=V(b).sub(V(a)),o=cyl(p,n,r,d.length(),V(a).add(V(b)).multiplyScalar(.5).toArray(),mat);o.quaternion.setFromUnitVectors(V([0,1,0]),d.normalize());return o;};
 const profile=(p,n,len,at,axis='y',w=.06)=>{const o=box(p,n,[w,len,w],at,m.al,false);if(axis==='x')o.rotation.z=Math.PI/2;if(axis==='z')o.rotation.x=Math.PI/2;if(!simple){for(const s of[-1,1]){const line=box(o,n+'Slot'+s,[.007,len,.001],[0,0,s*(w/2+.0001)],m.dark,false);line.castShadow=false;}}return o;};
 const cable=(p,n,points,r=.006)=>mesh(p,n,new T.TubeGeometry(new T.CatmullRomCurve3(points.map(V)),simple?8:16,r,simple?5:8,false),[0,0,0],m.rubber);
 const bolt=(p,n,at,axis='y')=>{if(simple)return;cyl(p,n+'Washer',.009,.002,at,m.al,axis);const pos=[...at];pos[{x:0,y:1,z:2}[axis]]+=.004;cyl(p,n+'Head',.006,.006,pos,m.dark,axis);};
 function motor(p,n,at,axis='z') {const g=group(p,n,at);if(axis==='x')g.rotation.y=Math.PI/2;cyl(g,n+'Body',.052,.14,[0,0,0],m.blue,'z');cyl(g,n+'Gear',.066,.05,[0,0,.095],m.al,'z');cyl(g,n+'Shaft',.014,.05,[0,0,.145],m.steel,'z');box(g,n+'Foot',[.14,.018,.16],[0,-.061,0],m.dark);box(g,n+'Terminal',[.05,.035,.06],[0,.063,0],m.dark);if(!simple)for(let i=-2;i<=2;i++)box(g,n+'Fin'+i,[.10,.004,.12],[0,i*.014,-.005],m.blue,false);return g;}
 return{m,group,mesh,box,cyl,rod,profile,cable,bolt,motor};
}
export function clip(name,tracks){
 // Keep boundary holds. Only merge times which actually alias in Float32.
 for(const t of tracks){const size=t.getValueSize(),times=[],values=[];for(let i=0;i<t.times.length;i++){const time=Math.fround(t.times[i]);if(times.at(-1)===time){values.splice(-size,size,...t.values.slice(i*size,(i+1)*size));}else{times.push(time);values.push(...t.values.slice(i*size,(i+1)*size));}}t.times=new Float32Array(times);t.values=new Float32Array(values);}
 return new T.AnimationClip(name,1,tracks);
}
