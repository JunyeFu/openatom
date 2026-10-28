import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const root=process.cwd(),home=path.join(root,'modules/media/native-equipment-50');
const read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const inventory=await read(path.join(home,'inventory.json')),catalog=await read(path.join(home,'catalog.json')),registration=await read(path.join(home,'registration.json'));
assert.equal(catalog.equipment.length,50);assert.equal(new Set(catalog.equipment.map(e=>e.definition_id)).size,50);assert.equal(inventory.entries.length,100);assert.equal(registration.register_order.length,150);
const expected=new Map(registration.register_order.map(e=>[e.id+'@'+e.version,e]));assert.equal(expected.size,150);
const packages=[];
for(const entry of registration.register_order){
 const pkg=(await read(path.join(root,entry.directory,'module.json'))).data;
 assert.equal(pkg.definition.id,entry.id);assert.equal(pkg.definition.version,entry.version);
 for(const dependency of pkg.dependencies)assert.ok(expected.has(dependency.id+'@'+dependency.version),'Missing exact dependency '+dependency.id);
 for(const file of pkg.files)assert.ok((await fs.stat(path.join(root,entry.directory,file.path))).isFile(),file.path);
 packages.push(pkg);
}
for(const entry of inventory.entries){
 const bytes=await fs.readFile(path.join(home,entry.directory,entry.asset.path));assert.equal(bytes.length,entry.bytes);assert.equal(bytes.readUInt32LE(0),0x46546c67);assert.equal(bytes.readUInt32LE(8),bytes.length);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)).toString());assert.ok(j.nodes.length>20);assert.ok(j.meshes.length>5);assert.equal(j.nodes.length,entry.nodes);assert.ok(j.images.every(i=>i.uri===undefined));
 const p=entry.presentation;
 for(const anchor of [...Object.values(p.anchor_roles),...Object.values(p.port_anchors)])if(anchor.kind==='scene_node')assert.ok(j.nodes[anchor.data]);
 for(const t of p.timeline){const a=j.animations[t.source.data.animation_index];assert.ok(a);for(const c of a.channels)assert.notEqual(j.nodes[c.target.node].name,'Equipment','Entire-machine motion is not a mechanism');}
 const device=packages.find(p=>p.definition.id===entry.definition_id).definition.payload.data;
 for(const port of device.ports)assert.ok(p.port_anchors[port.id],entry.id+' missing actual port '+port.id);
 for(const phase of device.behavior.inline.phases)assert.ok(p.timeline.some(t=>t.phase_ref===phase.id),entry.id+' missing phase '+phase.id);
}
const host=await read(path.join(root,'tools/3d-proof/native50/host-library-result.json'));
assert.equal(host.outcome.status,'ok');const candidates=host.outcome.data.candidates;
assert.equal(candidates.filter(c=>c.category==='devices').length,50);assert.equal(candidates.filter(c=>c.category==='media').length,100);
const result={devices:50,paper:catalog.paper_devices,common:catalog.common_devices,glb_models:100,new_geometry_types:43,reused_mature_geometry_types:7,canonical_host_devices:50,canonical_host_media:100,exact_dependencies_and_files:'pass',actual_glb_port_and_phase_bindings:'pass',glb_bytes:inventory.entries.reduce((n,e)=>n+e.bytes,0),scope:'model_packages_and_real_Host_registration_not_Native_run_acceptance'};
await fs.writeFile(path.join(root,'tools/3d-proof/native50/package-checks.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
