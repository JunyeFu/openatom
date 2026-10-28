import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';
const root=process.cwd(),home=path.join(root,process.argv[2]??'modules/media/native-equipment-500/reviews/B02-Q01');
const read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const inventory=await read(path.join(home,'inventory.json')),registry=await read(path.join(home,'registration.json'));
const refs=new Set(registry.register_order.map(e=>e.id+'@'+e.version)),results=[];
for(const entry of registry.register_order){const pkg=(await read(path.join(home,entry.directory,'module.json'))).data;
 assert.equal(pkg.definition.id,entry.id);assert.equal(pkg.definition.version,entry.version);
 for(const f of pkg.files)assert.ok((await fs.stat(path.join(home,entry.directory,f.path))).isFile());
 for(const dep of pkg.dependencies)assert.ok(refs.has(dep.id+'@'+dep.version));
}
for(const e of inventory.entries){
 const dir=path.join(home,e.directory),bytes=await fs.readFile(path.join(dir,e.asset.path));
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),bin=bytes.subarray(28+bytes.readUInt32LE(12));
 assert.equal(bytes.readUInt32LE(8),bytes.length);assert.equal(new Set(j.nodes.map(n=>n.name)).size,j.nodes.length);
 for(const a of j.accessors.filter(a=>a.componentType===5126)){const view=j.bufferViews[a.bufferView],components={SCALAR:1,VEC2:2,VEC3:3,VEC4:4,MAT4:16}[a.type],stride=view.byteStride??components*4;
  for(let n=0;n<a.count;n++)for(let c=0;c<components;c++)assert.ok(Number.isFinite(bin.readFloatLE((view.byteOffset??0)+(a.byteOffset??0)+n*stride+c*4)));}
 for(const a of j.animations){const channels=new Set();for(const c of a.channels){const key=c.target.node+':'+c.target.path;assert.ok(!channels.has(key));channels.add(key);}}
 const scratch=path.join(root,'.work/native500-quality-20261025/rebuild',path.basename(home),e.id+'-'+e.suffix);await fs.mkdir(scratch,{recursive:true});
 await fs.cp(path.join(dir,'source'),path.join(scratch,'source'),{recursive:true});
 for(const f of ['LICENSE.txt','nameplate.png'])await fs.copyFile(path.join(dir,f),path.join(scratch,f));
 const run=spawnSync(process.execPath,['source/export.mjs'],{cwd:scratch,windowsHide:true,encoding:'utf8'});assert.equal(run.status,0,run.stderr);
 assert.ok((await fs.readFile(path.join(scratch,e.asset.path))).equals(bytes),e.id+' standalone source rebuild differs');
 results.push({id:e.id,lod:e.suffix,bytes:bytes.length,finite_geometry:true,unique_channels:true,own_source_rebuild_same_bytes:true});
}
await fs.writeFile(path.join(home,'source-checks.json'),JSON.stringify({passed:true,models:results.length,exact_packages:registry.register_order.length,checks:results},null,2)+'\n');
console.log(results.length+' GLBs: independent package-source re-export, finite geometry, unique channels and exact package closure PASS.');
