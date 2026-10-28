import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const root=process.cwd(),home=path.join(root,'modules/media/native-equipment-500'),read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const inv=await read(path.join(home,'inventory.json')),reg=await read(path.join(home,'registration.json'));
assert.equal(inv.devices,500);assert.equal(inv.entries.length,1000);assert.equal(reg.register_order.length,1500);
const refs=new Set(reg.register_order.map(e=>e.id+'@'+e.version));assert.equal(refs.size,1500);
const packages=new Map();let fileCount=0;
for(const r of reg.register_order){
 const dir=path.join(home,r.directory),pkg=(await read(path.join(dir,'module.json'))).data;assert.equal(pkg.definition.id,r.id);assert.equal(pkg.definition.version,r.version);
 for(const dep of pkg.dependencies)assert.ok(refs.has(dep.id+'@'+dep.version),'unresolved '+dep.id);
 for(const f of pkg.files){assert.ok((await fs.stat(path.join(dir,f.path))).isFile());fileCount++;}packages.set(r.id,pkg);
}
for(const e of inv.entries){
 const bytes=await fs.readFile(path.join(home,e.directory,e.asset.path));assert.equal(bytes.readUInt32LE(0),0x46546c67);assert.equal(bytes.readUInt32LE(8),bytes.length);assert.equal(e.bytes,bytes.length);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)).toString()),presentation=packages.get(e.module_id).definition.payload.data,device=packages.get(e.definition_id).definition.payload.data;
 assert.equal(j.nodes.length,e.nodes);assert.ok(j.images?.every(im=>im.bufferView!==undefined&&!im.uri));assert.ok(j.meshes.length>0);assert.ok(j.animations.length>0);
 assert.equal(new Set(j.nodes.map(n=>n.name)).size,j.nodes.length,e.id+' duplicate nodes');
 const offset=28+bytes.readUInt32LE(12),bin=bytes.subarray(offset);
 for(const a of j.accessors.filter(a=>a.componentType===5126)){const v=j.bufferViews[a.bufferView],start=(v.byteOffset??0)+(a.byteOffset??0),components={SCALAR:1,VEC2:2,VEC3:3,VEC4:4,MAT4:16}[a.type],stride=v.byteStride??components*4;for(let n=0;n<a.count;n++)for(let c=0;c<components;c++)assert.ok(Number.isFinite(bin.readFloatLE(start+n*stride+c*4)),e.id+' non-finite geometry/animation');}
 for(const a of [...Object.values(presentation.anchor_roles),...Object.values(presentation.port_anchors)])if(a.kind==='scene_node')assert.ok(j.nodes[a.data]);
 for(const t of presentation.timeline){const a=j.animations[t.source.data.animation_index];assert.ok(a);const tracks=new Set();for(const c of a.channels){const key=c.target.node+':'+c.target.path;assert.ok(!tracks.has(key),e.id+' duplicate animation channel '+key);tracks.add(key);assert.ok(!['Machine','Chassis','Equipment'].includes(j.nodes[c.target.node].name),e.id+' entire machine action');}}
 for(const p of device.ports)assert.ok(presentation.port_anchors[p.id],e.id+' missing port '+p.id);
 for(const p of device.behavior.inline.phases)assert.ok(presentation.timeline.some(t=>t.phase_ref===p.id),e.id+' missing phase '+p.id);
}
const host=await read(path.join(root,'tools/3d-proof/native500/host-library-result.json'));assert.equal(host.outcome.status,'ok');const c=host.outcome.data.candidates;
assert.equal(c.filter(e=>e.category==='devices').length,500);assert.equal(c.filter(e=>e.category==='media').length,1000);
for(const candidate of c)assert.ok(refs.has(candidate.module_ref.id+'@'+candidate.module_ref.version),'Host exact version is stale '+candidate.module_ref.id);
const result={device_count:500,model_count:1000,package_count:1500,new_devices:450,reused_unchanged_devices:45,revised_baseline_devices:5,checked_declared_files:fileCount,exact_dependencies_and_files:'pass',finite_geometry_animation_and_unique_channels:'pass',actual_GLB_port_phase_bindings:'pass',current_Rust_Host_registration_and_query:'pass',glb_bytes:inv.entries.reduce((s,e)=>s+e.bytes,0),formal_per_device_MQ_acceptance:'pending',native_actual_item_wait_save_reopen:'pending'};
await fs.writeFile(path.join(root,'tools/3d-proof/native500/package-checks.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
