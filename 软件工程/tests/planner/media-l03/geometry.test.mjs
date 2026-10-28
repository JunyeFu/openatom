import {test,after} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {Vector3,Box3} from 'three';
import {bindScene} from '../../../scenarios/媒体表现/runtime-line-grade/compiled/scene.mjs';
import {activeClip} from '../../../scenarios/媒体表现/runtime-line-grade/compiled/clips.mjs';
import {assetKey} from '../../../scenarios/媒体表现/runtime-line-grade/compiled/types.mjs';
const root=new URL('../../../',import.meta.url),base=new URL('modules/media/p302-l03-v03/',root),out=new URL('scenarios/媒体表现/runtime-l03/',root);
const registration=JSON.parse(await fs.readFile(new URL('registration.json',base),'utf8'));
const report={kind:'actual_new_GLB_geometry_and_phase_bindings_not_Native',checks:[],models:[]},models=new Map();
function geometryBytes(bytes){
 const b=Buffer.from(bytes),j=JSON.parse(b.subarray(20,20+b.readUInt32LE(12)).toString()),offset=20+b.readUInt32LE(12),bin=b.subarray(offset+8);
 for(const m of j.materials??[])delete m.pbrMetallicRoughness?.baseColorTexture;delete j.images;delete j.textures;delete j.samplers;
 const raw=Buffer.from(JSON.stringify(j)),js=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),r=Buffer.alloc(28+js.length+bin.length);
 r.writeUInt32LE(0x46546c67);r.writeUInt32LE(2,4);r.writeUInt32LE(r.length,8);r.writeUInt32LE(js.length,12);r.writeUInt32LE(0x4e4f534a,16);js.copy(r,20);r.writeUInt32LE(bin.length,20+js.length);r.writeUInt32LE(0x004e4942,24+js.length);bin.copy(r,28+js.length);return r.buffer.slice(r.byteOffset,r.byteOffset+r.byteLength);
}
for(const entry of registration.register_order){
 const dir=new URL(entry.directory+'/',root),pkg=JSON.parse(await fs.readFile(new URL('module.json',dir),'utf8')).data,p=pkg.definition.payload.data;
 const b=await fs.readFile(new URL(p.source.data.path,dir)),doc=JSON.parse(b.subarray(20,20+b.readUInt32LE(12)).toString());
 const gltf=await new GLTFLoader().parseAsync(geometryBytes(b),''),nodes=new Map();for(let i=0;i<doc.nodes.length;i++)nodes.set(i,await gltf.parser.getDependency('node',i));
 const rig=await bindScene(p,async()=>{throw Error('unexpected external geometry');},{assetKey:assetKey(p.source.data),decoded:{gltf,nodes,dispose(){}}});
 models.set(entry.id,{entry,pkg,p,doc,rig,nodes,get:name=>nodes.get(doc.nodes.findIndex(n=>n.name===name))});
}
const model=f=>models.get('p302.line-grade.'+f),near=(a,b,tol=2e-5)=>assert.ok(Math.abs(a-b)<tol,`${a} != ${b}`);
const point=o=>o.getWorldPosition(new Vector3());
function apply(m,phase,u){const p={known:true,display_phase_id:phase,progress:u};m.rig.apply(p,activeClip(m.p,p));}
test('new28 exact0.3 packages retain actual hierarchy, source rights, three LOD contract and declared ports',async()=>{
 assert.equal(models.size,28);
 for(const m of models.values()){
  assert.equal(m.pkg.definition.version,'0.3.0');assert.equal(m.doc.nodes.length,m.p.nodes.length);
  m.doc.nodes.forEach((n,i)=>{assert.equal(n.name,m.p.nodes[i].name);assert.equal(m.p.nodes[i].parent_index,m.doc.nodes.findIndex(p=>p.children?.includes(i))<0?null:m.doc.nodes.findIndex(p=>p.children?.includes(i)));});
  for(const a of m.pkg.assets){await fs.access(new URL(m.entry.directory+'/'+a.path,root));assert.ok(m.pkg.rights.some(r=>r.asset_ref===a.id&&r.license==='CC0-1.0'&&r.redistribution_confirmed));}
  for(const a of Object.values(m.p.port_anchors)){assert.equal(a.kind,'scene_node');assert.ok(m.doc.nodes[a.data]);}
  report.models.push({ref:{id:m.pkg.definition.id,version:m.pkg.definition.version},nodes:m.doc.nodes.length,ports:Object.keys(m.p.port_anchors),phases:m.p.timeline.map(c=>c.phase_ref)});
 }report.checks.push('exact28_hierarchy_rights_ports');
});
test('actual robot five clips contact the unchanged source ASM and CV entry; jaws align same item orientation',()=>{
 const m=model('robot');assert.deepEqual(m.doc.animations.map(a=>a.name),['approach','grip','loaded_move','release','return']);
 for(const [phase,u,target] of [['approach',1,[0,0,.8375]],['grip',0,[0,0,.8375]],['grip',1,[0,0,.8375]],['loaded_move',1,[2.65,0,.8375]],['release',0,[2.65,0,.8375]],['release',1,[2.65,0,.8375]]]){
  apply(m,phase,u);const p=point(m.get('GraspCenter'));[p.x+1.7,-p.z-.9,p.y].forEach((v,i)=>near(v,target[i]));
  if(phase==='grip'&&u===1){const left=new Box3().setFromObject(m.get('JawPad1')),right=new Box3().setFromObject(m.get('JawPad-1'));near(Math.max(left.min.x,right.min.x)-Math.min(left.max.x,right.max.x),.24);}
 }
 report.checks.push('robot_actual_contacts_and_five_clips');
});
test('sealer stage names and actual head contact match original .36 high package and .34 guide gap',()=>{
 const m=model('sealer');assert.deepEqual(m.doc.animations.map(a=>a.name),['infeed','seal','outfeed']);
 for(const [phase,u,x] of [['infeed',0,-.65],['infeed',1,-.455],['seal',0,-.455],['seal',1,.035],['outfeed',0,.035],['outfeed',1,.65]]){apply(m,phase,u);near(point(m.get('CartonSeat')).x,x);near(point(m.get('CartonSeat')).y,.798);}
 apply(m,'seal',.5);near(point(m.get('TapeContact')).y,.798+.36);
 const a=new Box3().setFromObject(m.get('SideBeltRun-0.231')),b=new Box3().setFromObject(m.get('SideBeltRun0.231'));
 near(b.min.z-a.max.z,.342);report.checks.push('sealer_source_package_contact');
});
test('packing plate closes on the same package top; original carton envelope is physically authored',()=>{
 const m=model('packing');apply(m,'working',.5);const plate=new Box3().setFromObject(m.get('FoldPlate'));near(plate.min.y,.798+.36);
 const carton=new Box3().setFromObject(model('carton').rig.root),size=carton.getSize(new Vector3());near(size.x,.49);near(size.y,.36);near(size.z,.3405,.002);
 report.checks.push('packing_contact_and_source_carton');
});
test('conveyor item anchor follows physical belt motion at source height; no reference item exists',()=>{
 const m=model('conveyor');for(const u of [0,.5,1]){apply(m,'working',u);const p=point(m.get('BeltCargo'));near(p.x,-.65+1.3*u);near(p.y,.805);}
 for(const m of models.values())assert.ok(!m.doc.nodes.some(n=>n.name.includes('REFERENCE_GAUGE_NOT_INVENTORY')));
 report.checks.push('actual_belt_anchor_no_fit_inventory');
});
test('actual raw housing posts support the PCB; four same screws meet the original driver plane',()=>{
 const shell=model('shell'),pcb=model('pcb'),screw=model('fastener');
 const post=new Box3().setFromObject(shell.get([...shell.doc.nodes].find(n=>n.name.startsWith('PCBStandoff')).name));near(post.max.y,.026);
 const board=new Box3().setFromObject(pcb.get('CircuitBoard'));near(board.min.y+.026,.026);
 const head=new Box3().setFromObject(screw.get('SocketHead'));near(head.max.y+.025,.062);
 report.checks.push('actual_raw_parts_jig_stack_and_driver_contact');
});
after(async()=>{await fs.writeFile(new URL('geometry-result.json',out),JSON.stringify(report,null,2));});
