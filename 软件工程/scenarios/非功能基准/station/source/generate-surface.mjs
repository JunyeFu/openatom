// B60 1.0.5: preserve the benchmark mechanism and Engine rules, improve the
// observed sharp-edged/flat surfaces with self-contained core glTF materials.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import * as T from 'three';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {bindScene} from '../../../../packages/presentation/src/scene.ts';

const here=path.resolve(process.argv[3]??path.dirname(fileURLToPath(import.meta.url)));
const home=path.resolve(here,'..');
const output=path.resolve(process.argv[2]??path.join(home,'candidate-1.0.5'));
const version='1.0.5';
const read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const document=await read(path.join(home,'candidate-1.0.5-native/station.module-document.json'));
const p=document.data.definition.payload.data.presentation;
const old=structuredClone(p);
const rig=await bindScene(p,async()=>{throw Error('The preserved native input has no external assets.');});
rig.root.name='B60Station';
const materials=new Map(),geometries=new Map();
const geometry=(key,build)=>{if(!geometries.has(key))geometries.set(key,build());return geometries.get(key);};
function material(name,color,metalness,roughness,opacity=1){
 const key=JSON.stringify([name,color,metalness,roughness,opacity]);
 if(!materials.has(key))materials.set(key,new T.MeshStandardMaterial({name,color:new T.Color(...color),metalness,roughness,opacity,transparent:opacity<1,depthWrite:opacity===1}));
 return materials.get(key);
}
const steel=material('Brushed stainless',[.52,.58,.62],.78,.32);
const profile=material('Anodised aluminium',[.42,.49,.54],.72,.38);
const rubber=material('Rubber black',[.025,.035,.04],.02,.82);
const dark=material('Cast iron graphite',[.09,.12,.14],.38,.44);
const label=material('B60_NAMEPLATE',[1,1,1],0,.7);
const screen=material('B60_SCREEN',[1,1,1],0,.48);

// Extruded sections contain a real centre bore and longitudinal T grooves.
function extrusion(size){
 const axis=size.indexOf(Math.max(...size)),cross=size.filter((_,i)=>i!==axis),w=Math.min(...cross),h=w/2,a=w*.09,b=w*.19;
 const s=new T.Shape(),points=[[-h,-h],[-a,-h],[-a,-h+b],[a,-h+b],[a,-h],[h,-h],[h,-a],[h-b,-a],[h-b,a],[h,a],[h,h],[a,h],[a,h-b],[-a,h-b],[-a,h],[-h,h],[-h,a],[-h+b,a],[-h+b,-a],[-h,-a]];
 s.moveTo(...points[0]);points.slice(1).forEach(v=>s.lineTo(...v));s.closePath();
 const hole=new T.Path();hole.absarc(0,0,w*.1,0,Math.PI*2,true);s.holes.push(hole);
 const g=new T.ExtrudeGeometry(s,{depth:size[axis],bevelEnabled:false,curveSegments:8});g.translate(0,0,-size[axis]/2);
 g.applyQuaternion(new T.Quaternion().setFromUnitVectors(new T.Vector3(0,0,1),new T.Vector3(...[0,1,2].map(i=>i===axis?1:0))));
 return g;
}
for(let i=0;i<old.nodes.length;i++){
 const n=old.nodes[i],mesh=rig.nodes.get(i),m=n.material,[r,g,b,a]=m.color_rgba;
 mesh.material=material(a<1?'Polycarbonate guard':b>r*1.18?'Powder coat blue':m.metallic>.6?'Brushed stainless':m.metallic>.2?'Cast iron graphite':Math.max(r,g,b)<.2?'Rubber black':'Ceramic white shell',[r,g,b],m.metallic,m.roughness,a);
 if(n.geometry.kind==='box'){
  const d=n.geometry.data,size=[d.x,d.z,d.y];
  const slotted=/^(corner_column|gantry_vertical|front_guard_post|upper_length_rail|upper_cross_rail|side_guard_rail|front_guard_crossmember)/.test(n.name);
  mesh.geometry=geometry((slotted?'profile':'rounded')+JSON.stringify(size),()=>slotted?extrusion(size):new RoundedBoxGeometry(...size,1,Math.min(.004,...size.map(v=>v*.16))));
  if(slotted)mesh.material=profile;
 }
}

// The old screen shared its front plane with the console (visible z fighting).
// Its static content explicitly describes declarations, not fabricated live data.
rig.root.getObjectByName('control_display').position.z=1.002;
rig.root.getObjectByName('control_display').material=screen;
for(const name of ['emergency_stop','reset_button']){
 const button=rig.root.getObjectByName(name);button.position.z=1.002;button.position.y=1.50;button.quaternion.setFromAxisAngle(new T.Vector3(1,0,0),Math.PI/2);
}

function mesh(name,g,pos,m,parent=rig.root){const v=new T.Mesh(g,m);v.name=name;v.position.set(pos[0],pos[2],-pos[1]);parent.add(v);return v;}
const box=(name,size,pos,m=steel)=>mesh(name,geometry('box'+JSON.stringify(size),()=>new RoundedBoxGeometry(size[0],size[2],size[1],1,Math.min(.003,...size.map(v=>v*.14)))),pos,m);
function cylinder(name,r,h,pos,m=steel,segments=24){return mesh(name,geometry('cylinder'+JSON.stringify([r,h,segments]),()=>new T.CylinderGeometry(r,r,h,segments)),pos,m);}
for(const side of [-.52,.52])for(const z of [.62,.74])box(`service_handle_mount_${side}_${z}`,[.045,.045,.03],[side+.25,-.927,z]);
for(const x of [-.79,.79])for(const y of [-.79,.79])cylinder(`foot_locknut_${x}_${y}`,.045,.018,[x,y,.236],dark,6);
for(const x of [-.105,.105])for(const y of [-.06,.06]){
 cylinder(`fixture_locking_washer_${x}_${y}`,.013,.004,[x,y,1.276]);
 cylinder(`fixture_locking_bolt_${x}_${y}`,.007,.008,[x,y,1.281],dark,6);
}
const stopCollar=mesh('Emergency_stop_mounting_collar',geometry('stopCollar',()=>new T.TorusGeometry(.036,.005,8,32)),[-.82,-1.003,1.50],material('Safety yellow',[.9,.55,.05],.05,.5));
const plate=mesh('B60_original_identification',geometry('nameplate',()=>new T.PlaneGeometry(.43,.14)),[-.55,-.924,.80],label);
function cable(name,points){
 const curve=new T.CatmullRomCurve3(points.map(a=>new T.Vector3(a[0],a[2],-a[1])));
 return mesh(name,new T.TubeGeometry(curve,32,.010,8,false),[0,0,0],rubber);
}
cable('Drive_power_to_enclosed_cable_trunk',[[.90,.1,.82],[.90,.25,.67],[.78,.55,.65],[-.75,.55,.65],[-.90,.55,.70]]);
cable('HMI_cable_and_post_route',[[-.73,-.90,1.49],[-.86,-.85,1.44],[-.86,-.82,1.1],[-.90,-.25,.70]]);
for(const y of [-.71,.71])box(`sensor_cable_retainer_${y}`,[.035,.035,.025],[-.69,y,1.18],dark);

globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
let binary=Buffer.from(await new GLTFExporter().parseAsync(rig.root,{binary:true,onlyVisible:true,trs:true}));
let json=JSON.parse(binary.subarray(20,20+binary.readUInt32LE(12)).toString());
const offset=20+binary.readUInt32LE(12);let bin=Buffer.from(binary.subarray(offset+8,offset+8+binary.readUInt32LE(offset)));
json.images??=[];json.textures??=[];json.samplers??=[];
const sampler=json.samplers.push({magFilter:9729,minFilter:9987,wrapS:10497,wrapT:10497})-1,images=new Map();
async function texture(name){
 if(images.has(name))return images.get(name);
 const bytes=await fs.readFile(path.join(here,'surfaces',name));
 bin=Buffer.concat([bin,Buffer.alloc((4-bin.length%4)%4)]);const at=bin.length;bin=Buffer.concat([bin,bytes]);
 const view=json.bufferViews.push({buffer:0,byteOffset:at,byteLength:bytes.length})-1;
 const image=json.images.push({bufferView:view,mimeType:'image/png',name})-1;
 const id=json.textures.push({source:image,sampler})-1;images.set(name,id);return id;
}
for(const m of json.materials){
 const pb=m.pbrMetallicRoughness??={};
 if(/Brushed stainless|Anodised aluminium/.test(m.name)){
  pb.baseColorTexture={index:await texture('metal-color.png')};pb.metallicRoughnessTexture={index:await texture('metal-orm.png')};m.normalTexture={index:await texture('metal-normal.png'),scale:.24};
 }else if(/Powder coat|Ceramic white shell|Cast iron graphite/.test(m.name))m.normalTexture={index:await texture('paint-normal.png'),scale:.10};
 else if(m.name==='B60_NAMEPLATE')pb.baseColorTexture={index:await texture('nameplate.png'),extensions:{KHR_texture_transform:{offset:[0,1],scale:[1,-1]}}};
 else if(m.name==='B60_SCREEN')pb.baseColorTexture={index:await texture('screen.png'),extensions:{KHR_texture_transform:{offset:[0,1],scale:[1,-1]}}};
}
// Raw PNG rows start at the top; the authored Three UVs put the top at v=1.
json.extensionsUsed=[...new Set([...(json.extensionsUsed??[]),'KHR_texture_transform'])];
bin=Buffer.concat([bin,Buffer.alloc((4-bin.length%4)%4)]);json.buffers[0].byteLength=bin.length;
const raw=Buffer.from(JSON.stringify(json)),chunk=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]);binary=Buffer.alloc(28+chunk.length+bin.length);
binary.writeUInt32LE(0x46546c67);binary.writeUInt32LE(2,4);binary.writeUInt32LE(binary.length,8);binary.writeUInt32LE(chunk.length,12);binary.writeUInt32LE(0x4e4f534a,16);chunk.copy(binary,20);binary.writeUInt32LE(bin.length,20+chunk.length);binary.writeUInt32LE(0x004e4942,24+chunk.length);bin.copy(binary,28+chunk.length);

const indices=new Map(json.nodes.map((n,i)=>[n.name,i]));
const mapped=i=>indices.get(old.nodes[i].name);
const remapAnchor=a=>a.kind==='scene_node'?{...a,data:mapped(a.data)}:a;
const vector=a=>({x:a[0],y:a[1],z:a[2]});
const pose=n=>({translation:vector(n.translation??[0,0,0]),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:vector(n.scale??[1,1,1]),unit:'meter',coordinate_frame:'right_handed_y_up'});
const asset={id:'p201.b60.station.surface.1.0.5',path:'assets/b60-station-1.0.5.glb',kind:'glb'};
Object.assign(p,{kind:'model',source:{kind:'asset',data:asset},coordinate_frame:'right_handed_y_up',
 nodes:json.nodes.map((n,i)=>({id:i,name:n.name,parent_index:json.nodes.findIndex(parent=>parent.children?.includes(i))<0?null:json.nodes.findIndex(parent=>parent.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.5,texture:null}})),
 actions:old.actions.map(a=>({...a,node_index:mapped(a.node_index)})),
 poses:old.poses.map(a=>({...a,node_transforms:a.node_transforms.map(v=>({...v,node_index:mapped(v.node_index)}))})),
 anchor_roles:Object.fromEntries(Object.entries(old.anchor_roles).map(([k,a])=>[k,remapAnchor(a)])),
 port_anchors:Object.fromEntries(Object.entries(old.port_anchors).map(([k,a])=>[k,remapAnchor(a)]))});
document.data.definition.version=version;document.data.dependencies[0].version=version;p.lod.simplified.data.version=version;
const right={asset_ref:asset.id,source:'Original B60 parametric geometry and original Super Factory micro-surfaces; illustrative benchmark dimensions, not manufacturer CAD.',license:'CC0-1.0',redistribution_confirmed:true,missing_reason:null};
document.data.assets=[asset];document.data.files=[{path:asset.path,kind:'asset'}];document.data.rights=[right];document.data.definition.contribution={author:'Super Factory B60 original modeling',license:'CC0-1.0',assets:[right]};
const simple=await read(path.join(home,'candidate-1.0.5-native/simple.module-document.json'));simple.data.definition.version=version;
await fs.mkdir(path.join(output,'assets'),{recursive:true});
await fs.writeFile(path.join(output,asset.path),binary);
await fs.writeFile(path.join(output,'station.module-document.json'),JSON.stringify(document,null,2)+'\n');
await fs.writeFile(path.join(output,'simple.module-document.json'),JSON.stringify(simple,null,2)+'\n');
const stats={version,asset,bytes:binary.length,nodes:json.nodes.length,triangles:json.meshes.reduce((n,m)=>n+m.primitives.reduce((s,p)=>s+(p.indices==null?json.accessors[p.attributes.POSITION].count:json.accessors[p.indices].count)/3,0),0),embedded_surfaces:[...images.keys()],formal_MQ_acceptance:false,Native_acceptance:false};
await fs.writeFile(path.join(output,'build.json'),JSON.stringify(stats,null,2)+'\n');
console.log(JSON.stringify(stats));
rig.dispose();for(const g of geometries.values())g.dispose();for(const m of materials.values())m.dispose();
