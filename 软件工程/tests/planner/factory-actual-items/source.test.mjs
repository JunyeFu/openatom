import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {fields,host,setup as portSetup} from './regression-port-source.mjs';

const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-factory-items/compiled/',root);
await mkdir(out,{recursive:true});
let source=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/factoryPresentation.ts',root),'utf8'),{mode:'transform'})
 .replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('No Native in protocol fixture')};")
 .replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('No Native bytes in protocol fixture')};")
 .replace("from './presentationHost'","from '../old-port/compiled/presentationHost.mjs'");
await writeFile(new URL('factoryPresentation.mjs',out),source);
let fixtureSource=stripTypeScriptTypes(await readFile(new URL('tests/planner/factory-media/fixture.ts',root),'utf8'),{mode:'transform'});
for(const match of [...fixtureSource.matchAll(/import (\w+) from '([^']+\.json)';/g)])fixtureSource=fixtureSource.replace(match[0],`const ${match[1]}=${await readFile(new URL(match[2].replace('../../../',''),root),'utf8')};`);
await writeFile(new URL('fixture.mjs',out),fixtureSource);
const {createFactoryPresentation,factoryItemHolder}=await import(new URL('factoryPresentation.mjs',out));
const {fixture,fixturePorts,frames,pose}=await import(new URL('fixture.mjs',out));
function setup(intercept){
 const data=fixture(),frame=frames.find(v=>v.label==='calendar@35000').frame;
 const context={session_id:'factory-items-protocol',project_id:data.project.id,scope:{kind:'project'},scope_revision:9,project:structuredClone(data.project),view:{run_id:frame.run.id,at_ms:35000}};
 let current=context;
 const original=fixturePorts(data,context,async()=>new ArrayBuffer(0)),calls=[];
 const ports={...original.ports,async dispatch(request){calls.push(structuredClone(request));const value=await intercept?.(request,{data,frame});return value??original.ports.dispatch(request);}};
 return{data,frame,context,ports,calls,reader:createFactoryPresentation(()=>current,ports),switchTo:c=>current=c};
}
const materialPort=(s)=>s.data.pkg.definition.payload.data.ports.find(p=>p.payload.kind==='material'&&['output','bidirectional'].includes(p.direction));
function wirePort(s,read){s.ports.readPortOutput=async request=>read?read(request):({run_id:request.run_id,at_ms:request.at_ms,port:request.port,completed_output:{value:{coefficient:2,decimal_places:0},unit:materialPort(s).payload.data.unit},accepted_output:{value:{coefficient:1,decimal_places:0},unit:materialPort(s).payload.data.unit}});}
function adoptFixture(s){
 const item=s.frame.snapshot.objects.find(v=>v.kind==='item').data;
 const old={id:'fixture.exact.items',version:'1.0.0'},latest={id:old.id,version:'2.0.0'};
 const p=structuredClone(s.data.image),rights=[{redistribution_confirmed:false,license:'Unknown'}];
 p.definition.id=old.id;p.definition.version=old.version;p.definition.payload={category:'items',data:{presentation_or_media_ref:structuredClone(s.data.model.definition.payload.data)}};p.rights=rights;
 s.data.packages.push(p,{...structuredClone(p),definition:{...structuredClone(p.definition),version:latest.version}});
 const frozen=s.data.project.item_types.find(t=>t.id===item.item_type_ref.id);frozen.module_ref=old;frozen.revision=item.item_type_ref.revision;
 s.context.project.item_types.find(t=>t.id===frozen.id).module_ref=latest;
 return{item,old,latest,rights};
}
test('new protocol: shared actual_device counters do not require a Job; original Job fields stay unavailable',async()=>{
 const s=portSetup(),loaded=await s.reader.read(s.context),frame={run:loaded.run,snapshot:loaded.view_snapshot,device:loaded.frame.port_outputs.device,port_outputs:loaded.frame.port_outputs};
 const context={mode:'actual_device',frame},catalog=fields.mediaDataFields(context),completed=catalog.find(f=>f.binding.source_field==='device.completed_output');
 assert.equal(catalog.length,7);assert.equal(fields.resolveMediaData(completed.binding,context).formatted,'1 piece');
 for(const field of catalog.filter(f=>f.object_kind==='job'))assert.equal(fields.resolveMediaData(field.binding,context).reason,'no_actual_source');
 for(const patch of [{run_id:'other'},{at_ms:85000}]){const wrong={...frame,port_outputs:{...frame.port_outputs,...patch}};assert.equal(fields.resolveMediaData(completed.binding,{mode:'actual_device',frame:wrong}).status,'unavailable');}
 assert.equal(fields.resolveMediaData(completed.binding,{mode:'actual_device',frame:{...frame,device:{...frame.device,module_ref:{...frame.device.module_ref,version:'999'}}}}).status,'unavailable');
 assert.equal(fields.resolveMediaData(completed.binding,{mode:'actual_device',frame:{...frame,snapshot:{...frame.snapshot,next_index:1}}}).reason,'unknown_frame');
});
test('new protocol: no actual Job or ambiguous jobs cannot suppress independently sourced device outputs',async()=>{
 for(const mode of ['absent','multiple']){
  const s=setup((request,{frame})=>request.operation==='get_run'&&request.params.query.kind==='objects_at'&&request.params.query.data.objects.length?{kind:'objects',data:{run_id:frame.run.id,at_ms:35000,next_index:null,objects:mode==='absent'?[]:[...frame.snapshot.objects.filter(v=>v.kind==='job'),{kind:'job',data:{...frame.snapshot.objects.find(v=>v.kind==='job').data,id:'actual-parallel'}}]}}:undefined);wirePort(s);
  const source=await s.reader.read(s.context),device=source.devices.find(d=>d.device_id==='CNC');assert.equal(device.frame,null);assert.equal(device.port_outputs.ports[0].completed_output.value.coefficient,2);assert.equal(device.projection,mode==='absent'?'static':'unavailable');
 }
});
test('new protocol: shared getter keeps explicit missing port, Draft, exact identities and only UnsupportedCapability downgrade',async()=>{
 const s=setup(),device=s.data.project.devices[0],ports=s.data.pkg.definition.payload.data.ports,actual={run_id:s.frame.run.id,at_ms:35000,device,ports};
 assert.equal((await host.readActualPortOutputs(s.context,actual,{},()=>{})).ports[0].reason,'not_wired');
 let called=false;const draft=await host.readActualPortOutputs({...s.context,scope:{kind:'definition_draft',data:'private'}},actual,{readPortOutput:async()=>{called=true;}},()=>{});assert.equal(draft.device,null);assert.equal(called,false);
 const unsupported=Object.assign(new Error('old archive'),{detail:{code:'UnsupportedCapability',message:'no old port history'}});
 assert.equal((await host.readActualPortOutputs(s.context,actual,{readPortOutput:async()=>{throw unsupported;}},()=>{})).ports[0].reason,'unsupported_capability');
 const denied=Object.assign(new Error('revision denied'),{detail:{code:'RevisionConflict'}});await assert.rejects(host.readActualPortOutputs(s.context,actual,{readPortOutput:async()=>{throw denied;}},()=>{}),e=>e===denied);
 wirePort(s,request=>({run_id:'wrong',at_ms:request.at_ms,port:request.port,completed_output:{unit:'piece'},accepted_output:{unit:'piece'}}));await assert.rejects(host.readActualPortOutputs(s.context,actual,s.ports,()=>{}),/精确PortRef/);
});
test('new protocol: frozen ItemType precise module wins over current v2, with rights and exact media bytes source retained',async()=>{
 const s=setup(),a=adoptFixture(s),result=await s.reader.read(s.context),item=result.items.find(v=>v.item.id===a.item.id);assert.deepEqual(item.media.binding.request.source.data.module_ref,a.old);assert.deepEqual(item.media.rights,a.rights);assert.match(item.unavailable,/绑定/);assert.equal(item.attachment,null);
 assert.ok(s.calls.filter(q=>q.operation==='get_object'&&q.params.object_id.id===a.old.id).every(q=>q.params.source.kind==='run_module_version'&&q.params.source.data.module_ref.version==='1.0.0'));
 const missing=setup();missing.data.project.module_refs.push(a.old);missing.data.project.item_types[0].module_ref=null;const absent=await missing.reader.read(missing.context);assert.ok(absent.items.every(i=>!i.media&&/不从名称/.test(i.unavailable)));
});
test('new protocol: exact internal attachment accepts only actual ItemType/media/holder, never same-name or duplicate bindings',async()=>{
 for(const kind of ['valid','wrong_media','wrong_holder','duplicate']){
  const s=setup(),a=adoptFixture(s),device=s.data.project.devices[0];assert.equal(a.item.holder.kind,'job');
  const binding={item_id:a.item.id,item_type_ref:a.item.item_type_ref,holder:a.item.holder,media_ref:kind==='wrong_media'?a.latest:a.old,target:{kind:'device_node',device_id:device.id,device_module_ref:device.module_ref,media_ref:device.module_ref,asset_ref:s.data.model.assets.find(a=>a.kind==='glb'),node_index:0,offset:pose(0,0)}};
  if(kind==='wrong_holder')binding.holder={kind:'location',id:'unrelated'};
  s.ports.readItemAttachments=async request=>({...request,bindings:kind==='duplicate'?[binding,binding]:[binding]});
  const item=(await s.reader.read(s.context)).items.find(i=>i.item.id===a.item.id);assert.equal(Boolean(item.attachment),kind==='valid');if(kind!=='valid')assert.ok(item.unavailable);
 }
});
test('new protocol: pending spatial source cannot cross Run/time/revision and source identity mismatches reject',async()=>{
 for(const mode of ['late','wrong_response']){
  const s=setup();let release,entered;const gate=new Promise(resolve=>release=resolve),entry=new Promise(resolve=>entered=resolve);
  s.ports.readItemAttachments=async request=>{entered();await gate;return{...request,at_ms:mode==='wrong_response'?999:request.at_ms,bindings:[]};};
  const pending=s.reader.read(s.context);await entry;if(mode==='late')s.switchTo({...s.context,view:{...s.context.view,at_ms:45000}});release();await assert.rejects(pending,mode==='late'?/旧响应已丢弃/:/物品附着来源改变/);
 }
});
test('new protocol: container anchor requires actual reciprocal contents and actual historical Job mapping',()=>{
 const item={id:'child',holder:{kind:'container',id:'box'},contents:[]},box={id:'box',holder:{kind:'job',id:'actual-job'},contents:['child']};
 const job={id:'actual-job',execution_ref:{kind:'project_device',data:{device_id:'actual-device'}}};
 assert.deepEqual(factoryItemHolder(item,[item,box],[job]),{kind:'device',id:'actual-device'});assert.equal(factoryItemHolder(item,[item,{...box,contents:[]}],[job]),null);assert.equal(factoryItemHolder(item,[item,box],[]),null);
 assert.equal(factoryItemHolder(item,[item,{...box,holder:{kind:'container',id:'child'}}],[job]),null);assert.equal(factoryItemHolder({...item,holder:{kind:'transport',id:'moving'}},[item,box],[job]),null);
});
