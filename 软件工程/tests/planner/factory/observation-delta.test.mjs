import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {decodeObservationFrame} from '../../../apps/planner/src/services/runObservationDelta.ts';

const source=JSON.parse(await readFile(new URL('../../../packages/commands/tests/fixtures/comparison/actual-frozen-inputs.json',import.meta.url),'utf8'));
const hall=title=>({kind:'hall',data:{...structuredClone(source[0][0].halls[0]),title}});
const page=(at_ms,objects)=>({run_id:'R1',at_ms,objects,next_index:null});

test('initial admitted full observation preserves the actual complete page',()=>{
  const value={frame:1,page:page(0,[hall('A')])};
  assert.equal(decodeObservationFrame(null,{kind:'full',data:value}),value);
});
test('delta restores complete object order and time without mutating the prior observation',()=>{
  const original=page(6000,[hall('A'),hall('B')]);const before=structuredClone(original);
  Object.freeze(original.objects);Object.freeze(original);
  const updated=hall('C');
  const value=decodeObservationFrame({frame:1,page:original},{kind:'delta',data:{frame:2,base:1,run_id:'R1',at_ms:0,objects_len:3,updates:[[1,updated],[2,hall('D')]]}});
  assert.deepEqual(value,{frame:2,page:page(0,[hall('A'),hall('C'),hall('D')])});
  assert.deepEqual(original,before);
});
test('actual truncation and unchanged objects reconstruct the exact complete page',()=>{
  const original=page(0,[hall('A'),hall('B')]);
  const value=decodeObservationFrame({frame:4,page:original},{kind:'delta',data:{frame:5,base:4,run_id:'R1',at_ms:1200000,objects_len:1,updates:[]}});
  assert.deepEqual(value,{frame:5,page:page(1200000,[hall('A')])});assert.equal(original.objects.length,2);
});
test('a delta cannot use an absent or different actual base frame',()=>{
  const delta={kind:'delta',data:{frame:3,base:2,run_id:'R1',at_ms:1,objects_len:0,updates:[]}};
  for(const base of [null,{frame:1,page:page(0,[])}])assert.throws(()=>decodeObservationFrame(base,delta),/未匹配此请求的原帧/);
});
