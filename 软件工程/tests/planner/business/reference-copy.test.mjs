import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {root,source} from './closeout-source.mjs';
const draft=await source('draft'),schema=await source('schema');
const catalog=JSON.parse(await readFile(new URL('packages/commands/generated/schemas/get_object.response.schema.json',root),'utf8'));
const records=[];
test('copying the actual Device route_operation_refs item preserves exact ModuleRef identity and version',()=>{
 const field=catalog.$defs.DeviceInstance.properties.route_operation_refs.items;
 assert.equal(schema.definitionName(field),'ModuleRef');
 const input={id:'w5.op.phased',version:'1.0.0'},copied=draft.copyRecord(input,schema.definitionName(field));
 records.push({input,copied,schema:field.$ref});
 assert.deepEqual(copied,input);assert.notEqual(copied,input);assert.equal(input.id,'w5.op.phased');
});
test('canonical exact reference records retain all values instead of creating nonexistent objects',()=>{
 for(const [type,input]of Object.entries({SubjectRevisionRef:{id:'PRODUCT',revision:'r1'},ObjectRef:{kind:'device',id:'D1'},AssetRef:{id:'asset',path:'media/model.glb',kind:'glb'}})){
  assert.ok(catalog.$defs[type].properties.id,'the actual schema has the referenced ID');
  assert.deepEqual(draft.copyRecord(input,type),input,type);
 }
});
test('owning ArrivalPlan and RouteDefinition identities still remap only internal edges',()=>{
 const arrival={id:'arrival',at_ms:1000,destination:'raw',items:[{id:'box',components:[],contents:['part'],holder:{kind:'location',id:'raw'},item_type_ref:{id:'BOX',revision:'r1'}},{id:'part',components:['external-component'],contents:[],holder:{kind:'container',id:'box'},item_type_ref:{id:'PART',revision:'r2'}}]};
 const a=draft.copyRecord(arrival,'ArrivalPlan');assert.notEqual(a.id,arrival.id);assert.notEqual(a.items[0].id,'box');assert.equal(a.items[0].contents[0],a.items[1].id);assert.equal(a.items[1].holder.id,a.items[0].id);assert.deepEqual(a.items[1].components,['external-component']);assert.deepEqual(a.items[1].item_type_ref,arrival.items[1].item_type_ref);
 const route={id:'route',revision:'r1',steps:[{id:'s1',predecessors:[],operation_ref:{id:'op',version:'1.0.0'},device_id:'D1'},{id:'s2',predecessors:['s1'],operation_ref:{id:'op2',version:'1.0.0'},device_id:'D2'}]};
 const r=draft.copyRecord(route,'RouteDefinition');assert.notEqual(r.id,'route');assert.notEqual(r.steps[0].id,'s1');assert.equal(r.steps[1].predecessors[0],r.steps[0].id);assert.deepEqual(r.steps[0].operation_ref,route.steps[0].operation_ref);assert.equal(r.steps[0].device_id,'D1');
});
test.after(async()=>{await writeFile(new URL('.work/business-input-closeout/delivery/reference-copy-observations.json',root),JSON.stringify(records,null,2));});
