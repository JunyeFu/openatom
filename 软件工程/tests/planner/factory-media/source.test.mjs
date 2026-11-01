import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-factory-canvas/compiled/',root);await mkdir(out,{recursive:true});
const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
let host=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/presentationHost.ts',root),'utf8'),{mode:'transform'})
 .replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('No Native protocol test')};")
 .replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('No Native bytes protocol test')};");
host=host.replace("from './frozenProcess'","from './frozenProcess.mjs'");
await writeFile(new URL('frozenProcess.mjs',out),stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/frozenProcess.ts',root),'utf8'),{mode:'transform'}));
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
test('protocol: a large Job catalog reads every actual state through bounded selectors and partial producer pages',async()=>{
 let actualJobs;
 const s=setup(true,(q,{frame})=>{
  if(q.operation!=='get_run')return undefined;
  const query=q.params.query;
  if(query.kind==='full_result'){
   const actual=frame.snapshot.objects.find(object=>object.kind==='job').data;
   actualJobs=[actual,...Array.from({length:400},(_,index)=>({...structuredClone(actual),id:`queued-${String(index).padStart(3,'0')}`,
    state:'ready',started_at_ms:null,finished_at_ms:null,job_resources:[],phase_resources:[]}))];
   return {kind:'full',data:{...frame.run,jobs:actualJobs.map(job=>({...structuredClone(job),state:'completed',finished_at_ms:35000}))}};
  }
  if(query.kind==='objects_at'&&query.data.objects.length){
   const ids=new Set(query.data.objects.map(ref=>ref.id)),selected=actualJobs.filter(job=>ids.has(job.id));
   const start=query.data.page.after_index===null?0:query.data.page.after_index+1;
   const end=Math.min(start+100,selected.length);
   return {kind:'objects',data:{run_id:frame.run.id,at_ms:35000,
    objects:selected.slice(start,end).map(data=>({kind:'job',data})),next_index:end<selected.length?end-1:null}};
  }
 });
 const value=await s.reader.read(s.context),observed=value.raw.job_pages.flatMap(page=>page.objects).map(object=>object.data);
 const queries=s.calls.filter(q=>q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length);
 assert.ok(queries.every(q=>q.params.query.data.objects.length<=200),'a Job page must not resend the complete large catalog');
 assert.deepEqual(observed,actualJobs);assert.equal(new Set(observed.map(job=>job.id)).size,401);
 assert.equal(observed.filter(job=>job.state==='ready').length,400,'actual JobsAt, not completed final states');
 assert.deepEqual(queries.map(q=>q.params.query.data.page.after_index),[null,99,null,99,null]);
 assert.equal(JSON.stringify(frames.map(v=>v.frame.run)),originalRuns);
});

test('desktop: already obtained frozen inputs and one complete actual observation equal the public paged source',async()=>{
 const s=setup(),expected=await s.reader.read(s.context),requests=[];
 s.context.preparedRun={run:s.frame.run,modules:new Map(s.data.packages.map(pkg=>[JSON.stringify([pkg.definition.id,pkg.definition.version]),pkg]))};
 s.ports.readObservation=async request=>{requests.push(structuredClone(request));return structuredClone(expected.observation);};
 s.calls.length=0;
 const actual=await s.reader.read(s.context);
 assert.equal(s.calls.length,0,'do not reread the same complete Run, frozen Project or exact modules');
 assert.equal(requests.length,1);assert.deepEqual(Object.keys(requests[0]).sort(),['at_ms','project_id','run_id','scope','scope_revision','session_id']);
 assert.deepEqual(actual.observation,expected.observation);
 assert.deepEqual(actual.devices.map(v=>[v.device_id,v.projection,v.frame?.job_id,v.unavailable]),expected.devices.map(v=>[v.device_id,v.projection,v.frame?.job_id,v.unavailable]));
 assert.deepEqual(actual.items.map(v=>v.item),expected.items.map(v=>v.item));
 assert.equal(JSON.stringify(frames.map(v=>v.frame.run)),originalRuns);
});

test('desktop: wrong or pending complete observation cannot cross a scene or pretend a complete page',async()=>{
 for(const patch of [{run_id:'other'},{at_ms:99999},{next_index:0}]){
  const s=setup(),expected=await s.reader.read(s.context);
  s.ports.readObservation=async()=>({...expected.observation,...patch});
  await assert.rejects(s.reader.read(s.context),/完整观察/);
 }
 let release;const hold=new Promise(resolve=>release=resolve),s=setup(),expected=await s.reader.read(s.context);
 s.ports.readObservation=()=>hold;const pending=s.reader.read(s.context);
 s.switchTo({...s.context,scope_revision:10});release(expected.observation);
 await assert.rejects(pending,/旧响应已丢弃/);
 const wrong=setup();wrong.context.preparedRun={run:wrong.frame.run,modules:new Map()};wrong.context.project.revision=99;
 await assert.rejects(wrong.reader.read(wrong.context),/冻结工程修订/);
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


test('original phase_calendar staff labels keep declared role/home and actual five-boundary claims distinct from final state',async()=>{
 const input=await json('packages/commands/tests/fixtures/production/p1-phase-calendar.json');
 const recorded=await json('scenarios/真实业务设计/runtime-job-history/phase-calendar.actual.json');
 const original=JSON.stringify(recorded),api=await import(new URL('factoryPresentation.mjs',out));
 // Isolate the staff projection from device media; the five JobsAt records are actual producer history.
 const project={...input.project,devices:[]},ports={module:async()=>{throw new Error('No media needed for declared worker marker')},readAsset:async()=>{throw new Error('No invented worker media')}};
 for(const [at_ms,claimed] of [[0,true],[5000,false],[35000,false],[40000,true],[45000,false]]){
  const job=recorded.job_history.find(([at])=>at===at_ms)[1];
  const observation={run_id:recorded.run.id,at_ms,next_index:null,objects:[{kind:'job',data:job}]};
  const context={session_id:'staff-projection',project_id:project.id,scope:{kind:'project'},scope_revision:1,project,view:{run_id:recorded.run.id,at_ms}};
  const value=await api.projectFactoryPresentation(context,{project,run:recorded.run,observation,jobs:[job],items:[],raw:{job_pages:[observation],item_pages:[]}},ports,()=>{});
  const worker=value.resources.find(r=>r.resource.id==='W-C');
  assert.deepEqual(worker.worker_status,{kind:claimed?'claimed':'available',job_ids:claimed?[job.id]:[]});
  const label=api.factoryWorkerLabel(worker);assert.match(label,/W-C/);assert.match(label,/人员角色 worker/);assert.match(label,/声明位置 raw-store/);
  if(claimed){assert.match(label,/占用作业/);assert.ok(label.includes(job.id));}else assert.match(label,/本时点未占用/);
  assert.equal(worker.media,null);assert.equal(worker.unavailable,null);
 }
 const context={session_id:'staff-draft',project_id:project.id,scope:{kind:'project'},scope_revision:1,project,view:null};
 const draft=await api.projectFactoryPresentation(context,{project,run:null,observation:null,jobs:[],items:[],raw:{job_pages:[],item_pages:[]}},ports,()=>{});
 assert.match(api.factoryWorkerLabel(draft.resources.find(r=>r.resource.id==='W-C')),/占用尚未读取/);
 assert.equal(JSON.stringify(recorded),original);
});

test('pending actual frame preserves declared worker identity and home with unknown occupation',async()=>{
 const {declaredFactoryWorkers,factoryWorkerLabel}=await import(new URL('factoryPresentation.mjs',out));
 const input=await json('packages/commands/tests/fixtures/production/p1-phase-calendar.json');
 const original=JSON.stringify(input.project),markers=declaredFactoryWorkers(input.project);
 assert.equal(markers.length,1);assert.equal(markers[0].resource.id,'W-C');
 assert.deepEqual(markers[0].worker_status,{kind:'unobserved',job_ids:[]});
 assert.equal(markers[0].media,null);assert.equal(markers[0].unavailable,null);
 assert.match(factoryWorkerLabel(markers[0]),/人员角色 worker · 声明位置 raw-store · 占用尚未读取/);
 assert.doesNotMatch(factoryWorkerLabel(markers[0]),/本时点未占用|占用作业/);
 assert.deepEqual(declaredFactoryWorkers({...input.project,locations:[]}),[]);
 assert.deepEqual(declaredFactoryWorkers(null),[]);
 assert.equal(JSON.stringify(input.project),original);
});
