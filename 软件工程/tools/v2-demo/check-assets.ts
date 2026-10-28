import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
import {Box3} from 'three';
import {inspectGlb,decodeGlb} from '../../packages/presentation/src/glb';
import {bindScene} from '../../packages/presentation/src/scene';
import {assetKey} from '../../packages/presentation/src/types';

const inputDirectory=resolve(process.argv[2]),output=resolve(process.argv[3]);
await import(process.argv[4]);
const input=JSON.parse(await readFile(resolve(inputDirectory,'host-input.json'),'utf8'));
const results:any[]=[];
for(const module of input.modules){
  const d=module.definition;
  const p=d.payload.category==='devices'?d.payload.data.presentation:d.payload.category==='items'?d.payload.data.presentation_or_media_ref:d.payload.category==='media'?d.payload.data:null;
  if(!p)continue;
  let model:any=undefined,glb:any=null;
  if(p.source.kind==='asset'){
    const bytes=await readFile(resolve(inputDirectory,p.source.data.path));
    const buffer=bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);
    glb=inspectGlb(buffer);assert.equal(glb.asset.version,'2.0');
    const decoded=await decodeGlb(buffer);model={assetKey:assetKey(p.source.data),decoded};
  }
  const rig=await bindScene(p,async asset=>{const bytes=await readFile(resolve(inputDirectory,asset.path));return bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);},model);
  rig.apply(null,null);rig.root.updateMatrixWorld(true);
  const bounds=new Box3().setFromObject(rig.root);assert(!bounds.isEmpty());
  assert([...bounds.min.toArray(),...bounds.max.toArray()].every(Number.isFinite));
  assert.equal(rig.nodes.size,p.nodes.length);
  results.push({module_id:d.id,version:d.version,canonical_nodes:rig.nodes.size,glb_nodes:glb?.nodes?.length??0,embedded_images:glb?.images?.length??0,bounds:{min:bounds.min.toArray(),max:bounds.max.toArray()},decoded_by_production_presentation:true});rig.dispose();
}
assert(results.some(r=>r.module_id==='p201.b60.station'&&r.version==='1.0.5'&&r.embedded_images===6));
assert(results.some(r=>r.module_id==='p201.b60.workpiece'&&r.version==='1.0.1'));
await writeFile(output,JSON.stringify({passed:true,source:'actual new Host read_media_asset byte closure + original presentation decoder',modules:results,gpu_or_native_gui_verified:false},null,2));
console.log('Actual Host asset/presentation decode passed',results.length);
