import assert from 'node:assert/strict';
import {test} from 'node:test';
import {out} from './runtime.mjs';
const {inspectGlb}=await import(new URL('glb.mjs',out));
const encode=doc=>{
 const json=Buffer.from(JSON.stringify(doc)),pad=Buffer.concat([json,Buffer.alloc((4-json.length%4)%4,32)]),b=Buffer.alloc(20+pad.length);
 b.writeUInt32LE(0x46546c67);b.writeUInt32LE(2,4);b.writeUInt32LE(b.length,8);b.writeUInt32LE(pad.length,12);b.writeUInt32LE(0x4e4f534a,16);pad.copy(b,20);
 return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);
};
test('unknown GLB interpolation cannot silently fall back to linear',()=>{
 assert.throws(()=>inspectGlb(encode({asset:{version:'2.0'},animations:[{samplers:[{input:0,output:1,interpolation:'BEZIER'}],channels:[]}]})),/插值/);
});
test('rigid-only GLB rejects weight channels before loader',()=>{
 assert.throws(()=>inspectGlb(encode({asset:{version:'2.0'},animations:[{samplers:[],channels:[{sampler:0,target:{node:0,path:'weights'}}]}]})),/刚性|形变/);
});
