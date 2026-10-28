import test from 'node:test';
import assert from 'node:assert/strict';
import {Quaternion} from 'three';
import {setup} from './closeout-layout-harness.mjs';
for(const kind of ['Escape','pointercancel','lostpointercapture'])test(`${kind} discards a live real-Three transform and subsequent release cannot commit`,async()=>{
 const t=setup();try{t.begin();assert.equal(t.orbit.enabled,false);assert.equal(t.object.position.x,7);
 if(kind==='Escape')t.canvas.fire('keydown',{key:'Escape'});else {if(kind==='lostpointercapture')t.canvas.releasePointerCapture(7);t.canvas.fire(kind);}
 assert.deepEqual(t.object.position.toArray(),[2,3,4]);assert.ok(t.object.quaternion.angleTo(new Quaternion(...Object.values(t.original.rotation)))<1e-7);assert.equal(t.orbit.enabled,true);assert.equal(t.controls.dragging,false);
 t.canvas.fire('pointerup');await new Promise(r=>setImmediate(r));assert.equal(t.calls.length,0);
 }finally{t.handle.dispose();}
});
test('normal release produces one pose proposal; following capture loss cannot undo it',async()=>{
 const t=setup();try{t.begin();t.canvas.fire('pointerup');t.canvas.fire('lostpointercapture');await new Promise(r=>setImmediate(r));assert.equal(t.calls.length,1);assert.equal(t.calls[0].translation.x,7);assert.equal(t.object.position.x,7);assert.equal(t.orbit.enabled,true);}finally{t.handle.dispose();}
});
test('unrelated pointer loss cannot cancel the active pointer and dispose releases it',()=>{
 const t=setup();t.begin();t.canvas.fire('lostpointercapture',{pointerId:8});assert.equal(t.object.position.x,7);assert.equal(t.controls.dragging,true);t.handle.dispose();assert.equal(t.canvas.captures.size,0);assert.equal(t.calls.length,0);
});
