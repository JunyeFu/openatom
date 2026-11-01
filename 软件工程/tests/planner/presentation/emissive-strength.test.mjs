import assert from 'node:assert/strict';
import {test} from 'node:test';
import {out} from './runtime.mjs';
const {inspectGlb,decodeGlb}=await import(new URL('glb.mjs',out));
const {bindScene}=await import(new URL('scene.mjs',out));
const extension='KHR_materials_emissive_strength';

function triangle({strength,required=false,used=true,declare=true}={}) {
 const material={emissiveFactor:[.2,.4,.6]};
 if(declare)material.extensions={[extension]:strength===undefined?{}:{emissiveStrength:strength}};
 const doc={asset:{version:'2.0'},buffers:[{byteLength:36}],bufferViews:[{buffer:0,byteOffset:0,byteLength:36}],accessors:[{bufferView:0,componentType:5126,count:3,type:'VEC3',min:[0,0,0],max:[1,1,0]}],materials:[material],meshes:[{primitives:[{attributes:{POSITION:0},material:0}]}],nodes:[{mesh:0}],scenes:[{nodes:[0]}],scene:0};
 if(used)doc.extensionsUsed=[extension];
 if(required)doc.extensionsRequired=[extension];
 const raw=Buffer.from(JSON.stringify(doc)),json=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),bin=Buffer.alloc(36),b=Buffer.alloc(28+json.length+bin.length);
 [0,0,0,1,0,0,0,1,0].forEach((n,i)=>bin.writeFloatLE(n,i*4));
 b.writeUInt32LE(0x46546c67);b.writeUInt32LE(2,4);b.writeUInt32LE(b.length,8);b.writeUInt32LE(json.length,12);b.writeUInt32LE(0x4e4f534a,16);json.copy(b,20);b.writeUInt32LE(bin.length,20+json.length);b.writeUInt32LE(0x004e4942,24+json.length);bin.copy(b,28+json.length);
 return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);
}
const source={id:'emissive-regression',path:'triangle.glb',kind:'glb'};
const pose={translation:{x:0,y:0,z:0},rotation:{x:0,y:0,z:0,w:1},scale:{x:1,y:1,z:1},unit:'meter',coordinate_frame:'right_handed_y_up'};
const presentation={unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:[{name:'emissive triangle',parent_index:null,transform:pose,geometry:{kind:'imported_node',data:{asset_ref:source,node_index:0,unsupported_extensions:[]}}}],poses:[],actions:[],timeline:[]};

for(const required of [false,true])test(`emissive strength ${required?'required':'used'} survives inspect, decode and canonical bind`,async()=>{
 const bytes=triangle({strength:.5,required});
 assert.equal(inspectGlb(bytes).extensionsUsed[0],extension);
 const decoded=await decodeGlb(bytes);
 const rig=await bindScene(presentation,()=>{throw Error('unexpected asset read');},{assetKey:JSON.stringify([source.id,source.path,source.kind]),decoded});
 try {
  const material=rig.nodes.get(0).material;
  assert.deepEqual(material.emissive.toArray(),[.2,.4,.6]);
  assert.equal(material.emissiveIntensity,.5);
  assert.deepEqual(material.emissive.clone().multiplyScalar(material.emissiveIntensity).toArray(),[.1,.2,.3]);
  rig.apply(null,null);
  assert.equal(rig.nodes.get(0).material,material);
  assert.equal(material.emissiveIntensity,.5);
 }finally{rig.dispose();}
});

test('emissive intensity keeps the extension default and core material default',async()=>{
 for(const args of [{},{declare:false,used:false}]){
  const decoded=await decodeGlb(triangle(args));
  try{assert.equal(decoded.nodes.get(0).material.emissiveIntensity,1);assert.deepEqual(decoded.nodes.get(0).material.emissive.toArray(),[.2,.4,.6]);}finally{decoded.dispose();}
 }
});

test('support remains specific: an unknown used or required extension still fails inspection',()=>{
 for(const field of ['extensionsUsed','extensionsRequired']){
  const bytes=triangle(),v=new DataView(bytes),doc=JSON.parse(new TextDecoder().decode(new Uint8Array(bytes,20,v.getUint32(12,true))));
  doc[field]=[extension,'KHR_draco_mesh_compression'];
  const raw=Buffer.from(JSON.stringify(doc)),json=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),b=Buffer.alloc(20+json.length);
  b.writeUInt32LE(0x46546c67);b.writeUInt32LE(2,4);b.writeUInt32LE(b.length,8);b.writeUInt32LE(json.length,12);b.writeUInt32LE(0x4e4f534a,16);json.copy(b,20);
  assert.throws(()=>inspectGlb(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength)),/KHR_draco_mesh_compression/);
 }
});
