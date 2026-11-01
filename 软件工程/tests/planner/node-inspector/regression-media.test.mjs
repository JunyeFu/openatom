import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {test} from 'node:test';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-inspector/old-media-pure/media/',root);
await mkdir(out,{recursive:true});
for(const file of ['types','phase','clips','bytes','glb','scene']){
 const raw=await readFile(new URL(`packages/presentation/src/${file}.ts`,root),'utf8');
 const code=stripTypeScriptTypes(raw,{mode:'transform'}).replace(/from '\.\/(\w+)'/g,"from './$1.mjs'");await writeFile(new URL(`${file}.mjs`,out),code);
}
const {projectPhase}=await import(new URL('phase.mjs',out));
const {activeClip,lodSource}=await import(new URL('clips.mjs',out));
const {inspectGif,gifFrameAt}=await import(new URL('bytes.mjs',out));
const {decodeGlb,inspectGlb}=await import(new URL('glb.mjs',out));
const {bindScene,sampledPose}=await import(new URL('scene.mjs',out));
const {Object3D}=await import('three');
const {applyPose}=await import(new URL('scene.mjs',out));
const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
const bytes=async path=>{const b=await readFile(new URL('modules/media/p302-controlled/'+path,root));return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);};
const frames=await json('scenarios/媒体表现/runtime/actual-frames.json');
const p=(await json('modules/media/p302-controlled/model.module.json')).definition.payload.data;
const get=(name,at)=>frames.find(x=>x.case===name&&x.frame.snapshot.at_ms===at).frame;
test('actual Engine stage-local boundary, calendar interruption, output hold and pause',()=>{
 let point=projectPhase(get('calendar',35000));assert.equal(point.display_phase_id,'auto');assert.equal(point.progress,1);assert.equal(point.terminal_pose,true);assert.equal(point.waits[0].reasons[0].kind,'calendar');
 point=projectPhase(get('calendar',40000));assert.equal(point.display_phase_id,'unload');assert.equal(point.progress,0);assert.equal(point.job.state,'working');
 point=projectPhase(get('interrupt',20000));assert.equal(point.display_phase_id,'auto');assert.equal(point.effective_ms,15000);assert.equal(point.progress,.5);assert.equal(point.terminal_pose,false);
 point=projectPhase(get('output',100000));assert.equal(point.display_phase_id,'unload');assert.equal(point.progress,1);assert.equal(point.terminal_pose,true);assert.equal(point.job.state,'waiting_output');
 assert.ok(point.waits.some(w=>w.reasons.some(r=>r.kind==='output_capacity')));assert.ok(point.claims.some(c=>c.resource_id==='CAP-CNC'));
 const paused=frames.find(x=>x.label==='interrupt-paused@20000').frame;assert.equal(projectPhase(paused).paused,true);assert.equal(projectPhase(paused).progress,.5);
});
test('unknown frontier, wrong Run, missing actual job and partial page remain unknown',()=>{
 for(const mutate of [f=>f.snapshot.at_ms=100001,f=>f.snapshot.run_id='other',f=>f.snapshot.objects=[],f=>f.snapshot.next_index=1]){const f=structuredClone(get('calendar',35000));mutate(f);assert.equal(projectPhase(f).known,false);}
});
test('real single-duration Engine uses cycle and exact duration, with a matching declared clip',()=>{
 const point=projectPhase(get('cycle',40000));assert.equal(point.display_phase_id,'cycle');assert.equal(point.duration_ms,40000);assert.equal(point.progress,1);assert.equal(activeClip(p,point).clip.phase_ref,'cycle');
});
test('real zero-duration same-tick phases select the final completion event without inventing work',()=>{
 const f=get('zero',0);assert.equal(f.run.phase_work_intervals.length,0);const point=projectPhase(f);assert.equal(point.display_phase_id,'unload');assert.equal(point.progress,1);assert.equal(point.terminal_pose,true);assert.equal(activeClip(p,point).clip.id,'clip-unload');
});
test('exact clips use phase-local half-open boundaries; no idle/other-phase fallback',()=>{
 const point=projectPhase(get('calendar',35000));assert.equal(activeClip(p,point).clip.id,'clip-auto');assert.equal(activeClip(p,point).progress,1);
 const custom=structuredClone(p);custom.timeline=[{...custom.timeline[1],id:'a',start_progress:0,end_progress:.5},{...custom.timeline[1],id:'b',start_progress:.5,end_progress:1}];assert.equal(activeClip(custom,{...point,progress:.5,effective_ms:15000}).clip.id,'b');
 assert.throws(()=>activeClip({...p,timeline:p.timeline.filter(c=>c.phase_ref!=='auto')},point),/唯一/);
 assert.throws(()=>lodSource({...p,lod:{...p.lod,simplified:null}},'simplified'),/未声明/);assert.equal(lodSource(p,'region'),null);
});
test('actual encoded GIF timing supports exact frames, encoded loop, terminal clamp',async()=>{
 const info=inspectGif(await bytes('station.gif'));assert.deepEqual(info.durations,[200,200,200,200,200,200,200,200]);assert.equal(info.width,960);assert.equal(info.duration_ms,1600);assert.equal(info.repeat,0);
 assert.equal(gifFrameAt(info,0),0);assert.equal(gifFrameAt(info,199),0);assert.equal(gifFrameAt(info,200),1);assert.equal(gifFrameAt(info,1600),0);assert.equal(gifFrameAt(info,1600,true),7);assert.equal(gifFrameAt(info,800),4);
 assert.throws(()=>inspectGif(new ArrayBuffer(10)),/签名/);
});
function makeGlb(doc){const raw=Buffer.from(JSON.stringify(doc)),pad=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),b=Buffer.alloc(20+pad.length);b.writeUInt32LE(0x46546c67,0);b.writeUInt32LE(2,4);b.writeUInt32LE(b.length,8);b.writeUInt32LE(pad.length,12);b.writeUInt32LE(0x4e4f534a,16);pad.copy(b,20);return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);}
test('self-contained GLB rejects URI/extensions/skin/truncation before loader',async()=>{
 const data=await bytes('station.glb');assert.equal(inspectGlb(data).nodes.length,66);
 for(const doc of [{asset:{version:'2.0'},buffers:[{uri:'https://example.org/x.bin'}]},{asset:{version:'2.0'},images:[{uri:'file:///C:/secret.png'}]},{asset:{version:'2.0'},extensionsUsed:['KHR_draco_mesh_compression']},{asset:{version:'2.0'},nodes:[{skin:0}]}])assert.throws(()=>inspectGlb(makeGlb(doc)));
 assert.throws(()=>inspectGlb(data.slice(0,100)));
});
test('real GLB geometry + animation uses actual snapshots, reversible seek, no input mutation',async()=>{
 const decoded=await decodeGlb(await bytes('station.glb'));let meshes=0,vertices=0;decoded.gltf.scene.traverse(o=>{if(o.isMesh){meshes++;vertices+=o.geometry.attributes.position.count;}});assert.equal(meshes,66);assert.equal(vertices,66*24);
 const rig=await bindScene(p,()=>{throw Error('unexpected extra asset');},{assetKey:JSON.stringify([p.source.data.id,p.source.data.path,p.source.data.kind]),decoded});
 const head=p.nodes.findIndex(n=>n.name==='Contact head');
 const midway=get('interrupt',20000),before=JSON.stringify(midway),point=projectPhase(midway);rig.apply(point,activeClip(p,point));const y=rig.nodes.get(head).position.y;assert.ok(Math.abs(y-1.38)<.00001);
 let endPoint=projectPhase(get('calendar',35000));rig.apply(endPoint,activeClip(p,endPoint));assert.ok(Math.abs(rig.nodes.get(head).position.y-1.5)<.00001);
 rig.apply(point,activeClip(p,point));assert.ok(Math.abs(rig.nodes.get(head).position.y-y)<.00001);assert.equal(JSON.stringify(midway),before);
 rig.dispose();
});
test('explicit Y-up/meter and Z-up/mm conversion; quaternion interpolation',()=>{
 const pose={translation:{x:1000,y:2000,z:3000},rotation:{x:0,y:0,z:0,w:1},scale:{x:1,y:2,z:3},unit:'millimeter',coordinate_frame:'right_handed_z_up'},o=new Object3D();applyPose(o,pose);assert.ok(Math.abs(o.position.x-1)<1e-8&&Math.abs(o.position.y-3)<1e-8&&Math.abs(o.position.z+2)<1e-8);assert.deepEqual(o.scale.toArray(),[1,3,2]);
 const sampled=sampledPose([{at_progress:0,pose},{at_progress:1,pose:{...pose,translation:{x:2000,y:2000,z:3000}}}],.5,true);assert.equal(sampled.translation.x,1500);
});
