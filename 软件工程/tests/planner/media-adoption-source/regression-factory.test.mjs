import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-media-adoption-consumer/old-factory/compiled/',root);await mkdir(out,{recursive:true});
const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
let host=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/presentationHost.ts',root),'utf8'),{mode:'transform'})
 .replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('No Native protocol test')};")
 .replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('No Native bytes protocol test')};");
await writeFile(new URL('presentationHost.mjs',out),host);
let service=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/factoryPresentation.ts',root),'utf8'),{mode:'transform'})
 .replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('No Native protocol test')};")
 .replace(/import \{ readMediaAsset \} from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('No Native bytes protocol test')};")
 .replace("from './presentationHost'","from './presentationHost.mjs'");
await writeFile(new URL('factoryPresentation.mjs',out),service);
let fixtureSource=stripTypeScriptTypes(await readFile(new URL('tests/planner/factory-media/fixture.ts',root),'utf8'),{mode:'transform'});
for(const match of [...fixtureSource.matchAll(/import (\w+) from '([^']+\.json)';/g)])fixtureSource=fixtureSource.replace(match[0],`const ${match[1]}=${JSON.stringify(await json(match[2].replace('../../../','')))};`);
await writeFile(new URL('fixture.mjs',out),fixtureSource);
const {createFactoryPresentation}=await import(new URL('factoryPresentation.mjs',out));
const {fixture,fixturePorts,frames,originalRuns,sequential,sequentialJobs,sequentialOriginal}=await import(new URL('fixture.mjs',out));
const bytes=await readFile(new URL('modules/media/p302-controlled/station.glb',root));
const buffer=()=>bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);
function setup(actual=true,intercept){
 const data=fixture(),frame=frames.find(x=>x.label==='calendar@35000').frame;
 const context={session_id:'protocol-session',project_id:data.project.id,scope:{kind:'project'},scope_revision:9,project:structuredClone(data.project),view:actual?{run_id:frame.run.id,at_ms:35000}:null};
 let active=context;const raw=fixturePorts(data,context,async()=>buffer()),calls=[];
 const ports={...raw.ports,async dispatch(q){calls.push(structuredClone(q));if(context.scope.kind==='definition_draft'&&q.operation==='get_object'&&q.params.object_id.kind==='module'){const wanted=q.params.source.data.module_ref;if(![{id:data.pkg.definition.id,version:data.pkg.definition.version},...data.pkg.dependencies].some(ref=>ref.id===wanted.id&&ref.version===wanted.version))throw new Error('Draft fixture only grants exact self or explicit dependencies');}if(intercept){const value=await intercept(q,{data,frame});if(value!==undefined)return value;}return raw.ports.dispatch(q);}};
 const reader=createFactoryPresentation(()=>active,ports);
 return {data,frame,context,ports,calls,reader,switchTo:c=>active=c};
}
test('protocol: one explicit JobsAt catalog + all original Item pages + frozen physical project',async()=>{
 const s=setup(),v=await s.reader.read(s.context);assert.equal(v.project,s.data.project);assert.equal(v.run,s.frame.run);assert.notEqual(v.observation,s.frame.snapshot);
 assert.equal(v.raw.job_pages.length,1);assert.equal(v.raw.item_pages.length,1);assert.equal(v.observation.next_index,null);
 assert.equal(v.devices.find(d=>d.device_id==='CNC').frame.snapshot,v.observation);assert.equal(v.devices.find(d=>d.device_id==='B').frame,null);
 const qs=s.calls.filter(q=>q.operation==='get_run'&&q.params.query.kind==='objects_at');assert.deepEqual(qs[0].params.query.data.objects,s.frame.run.jobs.map(job=>({kind:'job',id:job.id})));assert.ok(qs.slice(1).every(q=>q.params.query.data.objects.length===0));
 assert.ok(s.calls.every(q=>['get_run','get_object'].includes(q.operation)));assert.equal(JSON.stringify(frames.map(v=>v.frame.run)),originalRuns);
});
test('protocol: current revision/model/override cannot replace old Run dependencies or frozen instance pose',async()=>{
 const s=setup();s.context.project.revision=99;s.context.project.devices[0].module_ref.version='2.0.0';s.context.project.devices[0].pose.translation.x=999;
 const v=await s.reader.read(s.context),a=v.devices.find(d=>d.device_id==='CNC');assert.equal(v.project.revision,1);assert.equal(v.project.devices[0].pose.translation.x,1);assert.equal(a.media.binding.request.source.kind,'run_module_version');assert.equal(a.media.binding.request.source.data.module_ref.version,'0.1.0');
 assert.ok(s.calls.filter(q=>q.operation==='get_object'&&q.params.object_id.kind==='module').every(q=>q.params.source.kind==='run_module_version'&&q.params.source.data.at_ms===35000));
 const dep=await a.media.resolveModule({kind:'run_module_version',data:{...s.context.view,module_ref:{id:s.data.simple.definition.id,version:s.data.simple.definition.version}}});assert.equal(dep.definition.id,s.data.simple.definition.id);
 const result=await a.media.readAsset({...a.media.binding.request,asset:s.data.model.assets.find(a=>a.kind==='glb')});assert.ok(result instanceof ArrayBuffer);assert.equal(new DataView(result).getUint32(0,true),0x46546c67);
});
test('protocol: final Jobs never supply historical state, while absence of a real Job stays static',async()=>{
 const s=setup(true,(q,{frame})=>q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length?{kind:'objects',data:{run_id:frame.run.id,at_ms:35000,objects:[],next_index:null}}:undefined);
 assert.ok(s.frame.run.jobs.length);const v=await s.reader.read(s.context);assert.ok(v.devices.every(d=>d.frame===null));assert.ok(v.devices.every(d=>d.media));
});
test('protocol: original completed frame consumes every real Item page with its advancing cursor',async()=>{
 const s=setup();s.context.view.at_ms=45000;const v=await s.reader.read(s.context);assert.equal(v.raw.item_pages.length,2);
 assert.equal(v.observation.objects.filter(o=>o.kind==='item').length,2);
 const pages=s.calls.filter(q=>q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length===0);assert.deepEqual(pages.map(q=>q.params.query.data.page.after_index),[null,1]);
});
test('protocol: old archives without Job history remain unavailable, never final Run substitution',async()=>{
 const missing=new Error('UnsupportedCapability: stored Run has no actual Job history');const s=setup(true,q=>{if(q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length)throw missing;});await assert.rejects(s.reader.read(s.context),e=>e===missing);
});
test('protocol: two actual Job mappings are explicit unavailable, no first-job choice',async()=>{
 const s=setup(true,(q,{frame})=>q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length?{kind:'objects',data:{run_id:frame.run.id,at_ms:35000,next_index:null,objects:[...frame.snapshot.objects.filter(o=>o.kind==='job'),{kind:'job',data:{...frame.snapshot.objects.find(o=>o.kind==='job').data,id:'simultaneous-job'}}]}}:undefined);
 const v=await s.reader.read(s.context),a=v.devices.find(d=>d.device_id==='CNC');assert.equal(a.media,null);assert.match(a.unavailable,/多个实际活动Job/);assert.equal(a.frame,null);
});
test('protocol: exact selected inline or explicit route operation binding, no unrelated operation guess',async()=>{
 const s=setup(),definition=s.data.pkg.definition.payload.data,job=s.frame.snapshot.objects.find(o=>o.kind==='job').data;
 const operation=s.data.packages.find(pkg=>pkg.definition.payload.category==='operations').definition.payload.data;
 definition.behavior={selected:'inline',inline:operation,operation_ref:null};
 const inline=setup(true,(q,{frame})=>q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length?{kind:'objects',data:{run_id:frame.run.id,at_ms:35000,objects:[{kind:'job',data:{...job,operation_ref:s.data.project.devices[0].module_ref}}],next_index:null}}:undefined);
 inline.data.pkg.definition.payload.data.behavior=definition.behavior;const a=await inline.reader.read(inline.context);assert.deepEqual(a.devices[0].frame.operation,operation);
 const route=setup();route.data.pkg.definition.payload.data.behavior={selected:'inline',inline:null,operation_ref:null};route.data.project.devices[0].operation_binding=null;route.data.project.devices[0].route_operation_refs=[job.operation_ref];assert.ok((await route.reader.read(route.context)).devices[0].frame);
 route.data.project.devices[0].route_operation_refs=[];const absent=await route.reader.read(route.context);assert.equal(absent.devices[0].media,null);assert.match(absent.devices[0].unavailable,/operation不匹配/);
});
test('protocol: same-scope session/project/revision/Run/time changes reject genuinely pending SDK responses',async()=>{
 for(const change of [c=>({...c,session_id:'other'}),c=>({...c,project_id:'other'}),c=>({...c,scope_revision:10}),c=>({...c,scope:{kind:'definition_draft',data:'other'}}),c=>({...c,view:{...c.view,run_id:'other'}}),c=>({...c,view:{...c.view,at_ms:40000}})]){
  let release;const pending=new Promise(resolve=>release=resolve);const s=setup(true,async q=>{if(q.operation==='get_run'&&q.params.query.kind==='full_result')await pending;});const read=s.reader.read(s.context);s.switchTo(change(s.context));release();await assert.rejects(read,/旧响应已丢弃/);
 }
});
test('protocol: late exact bytes and altered Run/source bindings cannot cross scenes',async()=>{
 const s=setup(),v=await s.reader.read(s.context),a=v.devices[0];
 await assert.rejects(a.media.readAsset({...a.media.binding.request,source:{kind:'module_version',data:{module_ref:s.data.project.devices[0].module_ref}},asset:s.data.model.assets[3]}),/改变了已绑定/);
 let release;const wait=new Promise(resolve=>release=resolve);s.ports.readAsset=()=>wait;const next=await s.reader.read(s.context),media=next.devices[0].media;
 const p=media.readAsset({...media.binding.request,asset:s.data.model.assets[3]});s.switchTo({...s.context,view:{...s.context.view,at_ms:40000}});release(buffer());await assert.rejects(p,/旧响应已丢弃/);
});
test('protocol: editing takes exact current version and override, Draft metadata stays in exact self/dependency scope',async()=>{
 const s=setup(false);s.context.project.devices[0].presentation_override=s.data.image.definition.payload.data;
 const v=await s.reader.read(s.context);assert.equal(v.devices[0].media.presentation,s.context.project.devices[0].presentation_override);assert.equal(v.run,null);assert.equal(v.devices[0].media.binding.request.source.kind,'module_version');assert.equal(s.calls.some(q=>q.operation==='get_run'),false);
 const draft=setup(false);draft.context.scope={kind:'definition_draft',data:'draft'};const d=await draft.reader.read(draft.context);assert.ok(d.devices.every(device=>device.media));assert.equal(draft.calls.length,1);assert.equal(draft.calls[0].scope.kind,'definition_draft');assert.equal(draft.calls[0].scope.data,'draft');assert.equal(d.devices[0].media.binding.request.scope_revision,9);
});
test('protocol: wrong source revision, page identity and unknown frontier stop explicitly',async()=>{
 const wrong=setup();wrong.data.project.revision=2;await assert.rejects(wrong.reader.read(wrong.context),/冻结工程修订/);
 for(const patch of [{run_id:'wrong'},{at_ms:99999},{next_index:0}]){const s=setup(true,(q,{frame})=>q.operation==='get_run'&&q.params.query.kind==='objects_at'?{kind:'objects',data:{run_id:frame.run.id,at_ms:35000,objects:[],next_index:null,...patch}}:undefined);await assert.rejects(s.reader.read(s.context));}
 const future=setup(true,(q,{frame})=>q.operation==='get_run'&&q.params.query.kind==='full_result'?{kind:'full',data:frame.run}:undefined);future.context.view.at_ms=future.frame.run.computed_until_ms+1;await assert.rejects(future.reader.read(future.context),/实际Run前沿/);assert.equal(future.calls.length,1);
});
test('sequential actual recorded Jobs: completed and unstarted queued Jobs do not shadow the unique current execution',async()=>{
 const data=fixture('sequential');const result=[];
 for(const at of [0,40000,80000]){
  const context={session_id:'sequential-recorded-engine',project_id:data.project.id,scope:{kind:'project'},scope_revision:9,project:data.project,view:{run_id:sequential.run.id,at_ms:at}};
  const {ports,calls}=fixturePorts(data,context,async()=>buffer(),'sequential');const value=await createFactoryPresentation(()=>context,ports).read(context);
  const device=value.devices.find(d=>d.device_id==='root-native-context:Assembly01'),jobs=sequentialJobs(at).filter(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===device.device_id);
  result.push({at,jobs,selected:device.frame?.job_id??null,unavailable:device.unavailable,job_pages:value.raw.job_pages.length});
  await writeFile(new URL(`sequential-${process.env.FACTORY_EVIDENCE_STAGE??'green'}.json`,out),JSON.stringify(result,null,2)+'\n');
  assert.equal(device.unavailable,null);assert.equal(value.raw.job_pages.length,3);
  if(at===0)assert.equal(device.frame.job_id,'job:24:ROOT-NATIVE-CONTEXT:task1:035:root-native-context:step-Assembly01');
  if(at===40000)assert.equal(device.frame.job_id,'job:24:ROOT-NATIVE-CONTEXT:task1:135:root-native-context:step-Assembly01');
  if(at===80000)assert.equal(device.frame,null,'no active execution: multiple actual terminal histories use static geometry');
  assert.ok(calls.every(q=>['get_run','get_object'].includes(q.operation)));
 }
 assert.equal(JSON.stringify(sequential),sequentialOriginal);
 await writeFile(new URL(`sequential-${process.env.FACTORY_EVIDENCE_STAGE??'green'}.json`,out),JSON.stringify(result,null,2)+'\n');
});
test('nine original complete stored Runs still equal their fixed producer records',async()=>{
 const names=['root_original','root_sdk_mapped','w5_original_source','w5_original_target','existing_seed','internal-pass','external-pass','internal-pending'];
 for(const name of names){const history=await json(`scenarios/真实业务设计/runtime-job-history/${name}.actual.json`),baseline=await json(`scenarios/模板与交接/runtime/source-clock/${name}.actual.json`);assert.deepEqual(history.run,baseline);}
 const p303=await json('scenarios/真实业务设计/runtime-job-history/p303-actual-history.json'),formal=await json('scenarios/图片时间轴/runtime/phase.bootstrap.actual.json');assert.deepEqual(p303.run,formal.formal_run);
});
