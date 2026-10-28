import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-factory-items/old-process/compiled/',root);
await mkdir(out,{recursive:true});
const source=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/presentationHost.ts',root),'utf8'),{mode:'transform'})
  .replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('Native not invoked by protocol tests')};")
  .replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('Native bytes not invoked by protocol tests')};");
await writeFile(new URL('presentationHost.mjs',out),source);
const {createPresentationHost,ProcessSourceChanged,ProcessUnavailable}=await import(new URL('presentationHost.mjs',out));
await writeFile(new URL('phase.mjs',out),stripTypeScriptTypes(await readFile(new URL('packages/presentation/src/phase.ts',root),'utf8'),{mode:'transform'}));
const {projectPhase}=await import(new URL('phase.mjs',out));
const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
const inputs=await json('scenarios/媒体表现/runtime/calendar.input.json');
const frames=await json('scenarios/媒体表现/runtime/actual-frames.json');
const base=frames.find(v=>v.label==='calendar@35000').frame;
const image=await json('modules/media/p302-controlled/image.module.json');
const ref=d=>({id:d.id,version:d.version});
function setup(change={}) {
  const frame=structuredClone(change.frame??base),project=structuredClone(inputs.project);
  const packages=structuredClone(inputs.definitions).map(definition=>({definition,assets:[],files:[],dependencies:[],rights:[],issues:[]}));
  const job=frame.snapshot.objects.find(o=>o.kind==='job').data;
  const device=project.devices.find(d=>d.id===job.execution_ref.data.device_id);
  const pkg=packages.find(p=>p.definition.id===device.module_ref.id);
  pkg.definition.payload.data.presentation=structuredClone(image.definition.payload.data);
  pkg.assets=structuredClone(image.assets);pkg.files=structuredClone(image.files);pkg.rights=structuredClone(image.rights);
  const context={session_id:'protocol-session',project_id:project.id,scope:{kind:'project'},scope_revision:9,
    selection:{kind:'actual_run',run_id:frame.run.id,job_id:job.id,at_ms:frame.snapshot.at_ms}};
  let active=context;
  const calls=[],reads=[];
  const items=frame.snapshot.objects.filter(o=>o.kind==='item');
  const ports={async dispatch(request) {
    calls.push(structuredClone(request));
    if(change.dispatch){const value=await change.dispatch(request,{frame,project,packages,job,items});if(value!==undefined)return value;}
    if(request.operation==='get_context')return {host:{session_id:context.session_id},project:{id:project.id},definition:{draft_id:context.scope.data,revision:context.scope_revision,definition_ref:ref(pkg.definition),issues:[]}};
    if(request.operation==='get_run'){
      const q=request.params.query;if(q.kind==='full_result')return {kind:'full',data:frame.run};
      const objects=q.data.objects.length?[{kind:'job',data:job}]:items;
      // Intentionally split the original Item read over two SDK pages.
      return {kind:'objects',data:{run_id:frame.run.id,at_ms:frame.snapshot.at_ms,objects:q.data.objects.length?objects:q.data.page.after_index===null?objects.slice(0,1):objects.slice(1),next_index:q.data.objects.length||q.data.page.after_index!==null?null:1}};
    }
    if(request.params.object_id.kind==='project')return {kind:'project',data:project};
    if(request.params.object_id.kind==='definition_draft')return {kind:'definition',data:pkg.definition};
    if(request.scope.kind==='definition_draft'&&request.params.source.kind==='module_version')throw new Error('Actual Host requires Project scope for ModuleVersion metadata');
    const wanted=request.params.source.data.module_ref;
    const module=packages.find(p=>p.definition.id===wanted.id&&p.definition.version===wanted.version);
    assert.ok(module,'protocol fixture must explicitly contain this exact module');return {kind:'module',data:module};
  },async readAsset(request){reads.push(request);return change.bytes?change.bytes(request):new Uint8Array([1,2,3]).buffer;}};
  return {frame,project,packages,job,pkg,context,calls,reads,host:createPresentationHost(()=>active,ports),switchTo:next=>active=next};
}

test('protocol: explicit JobsAt, all original Item pages, frozen resource declarations remain separate',async()=>{
  const s=setup(),value=await s.host.read(s.context);
  assert.equal(value.job.state,s.job.state);assert.equal(value.job.effective_progress_ms,s.job.effective_progress_ms);
  assert.equal(value.raw.job_pages.length,1);assert.equal(value.raw.item_pages.length,2);
  assert.equal(value.raw.frozen_project,s.project);assert.equal(value.view_snapshot.next_index,null);
  assert.equal(value.frame.snapshot,value.view_snapshot);assert.notEqual(value.frame.snapshot,value.raw.job_pages[0]);
  assert.equal(value.view_snapshot.objects.filter(o=>o.kind==='resource').length,s.project.resources.length);
  const objects=s.calls.filter(c=>c.operation==='get_run'&&c.params.query.kind==='objects_at');
  assert.deepEqual(objects.map(c=>c.params.query.data.objects),[[{kind:'job',id:s.job.id}],[],[]]);
  assert.equal(objects[2].params.query.data.page.after_index,1);
  assert.ok(s.calls.filter(c=>c.operation==='get_object'&&c.params.object_id.kind==='module').every(c=>c.params.source.kind==='run_module_version'&&c.params.source.data.at_ms===35000));
  assert.equal(value.binding.request.scope_revision,9);assert.equal(value.run.input_ref.project_revision,1);
});
test('protocol: missing Job history stops actual rendering despite a final Run.jobs record',async()=>{
  const s=setup({dispatch:q=>q.operation==='get_run'&&q.params.query.kind==='objects_at'&&q.params.query.data.objects.length?{kind:'objects',data:{run_id:base.run.id,at_ms:35000,objects:[],next_index:null}}:undefined});
  assert.ok(s.frame.run.jobs.length);await assert.rejects(s.host.read(s.context),/没有唯一实际Job历史/);
  assert.equal(s.calls.some(c=>c.operation==='get_object'),false);
});
test('protocol: frontier, wrong Run/time, and non-advancing pages stop explicitly',async()=>{
  const s=setup();s.context.selection.at_ms=s.frame.run.computed_until_ms+1;await assert.rejects(s.host.read(s.context),/已知计算边界/);
  for(const mutate of [p=>p.run_id='other',p=>p.at_ms=35001,p=>p.next_index=0]){
    const a=setup({dispatch:q=>{if(q.operation==='get_run'&&q.params.query.kind==='objects_at'){const p={run_id:base.run.id,at_ms:35000,objects:[],next_index:0};mutate(p);return {kind:'objects',data:p};}}});
    await assert.rejects(a.host.read(a.context),ProcessUnavailable);
  }
});
test('protocol: frozen inline operation and exact operation_ref resolve independently of current library',async()=>{
  const s=setup();s.job.operation_ref=ref(s.pkg.definition);
  const exact=s.packages.find(p=>p.definition.payload.category==='operations').definition.payload.data;
  s.pkg.definition.payload.data.behavior={selected:'inline',inline:exact,operation_ref:null};
  const value=await s.host.read(s.context);assert.equal(value.operation,exact);assert.equal(value.operation_module,s.pkg);
  assert.equal(s.calls.filter(c=>c.operation==='get_object'&&c.params.object_id.kind==='module').length,1);
  const separate=setup(),other=await separate.host.read(separate.context);
  assert.equal(other.operation_module.definition.id,separate.job.operation_ref.id);
});
test('protocol: definition_cycle uses its frozen execution module and same Draft Scope',async()=>{
  const s=setup();s.job.execution_ref={kind:'definition_cycle',data:{module_ref:ref(s.pkg.definition)}};
  s.pkg.definition.payload.data.behavior={selected:'operation_ref',inline:null,operation_ref:s.job.operation_ref};
  s.context.scope={kind:'definition_draft',data:'actual-draft'};s.context.scope_revision=6;
  const value=await s.host.read(s.context);assert.equal(value.device,null);
  assert.equal(value.binding.request.scope.kind,'definition_draft');assert.equal(value.binding.request.scope_revision,6);
  assert.ok(s.calls.every(c=>c.scope.kind==='definition_draft'&&c.scope.data==='actual-draft'));
});
test('protocol: wrong frozen module, Project revision or undeclared operation is never substituted',async()=>{
  const s=setup();s.project.revision=2;await assert.rejects(s.host.read(s.context),/冻结输入修订/);
  const t=setup();t.pkg.definition.version='new';await assert.rejects(t.host.read(t.context));
  const a=setup();a.job.operation_ref={id:'unknown',version:'1'};await assert.rejects(a.host.read(a.context),/不匹配/);
});
test('protocol: source switches discard late SDK responses at project, Scope revision, Run and at boundaries',async()=>{
  for(const switcher of [c=>({...c,project_id:'other'}),c=>({...c,session_id:'other'}),c=>({...c,scope_revision:10}),c=>({...c,scope:{kind:'definition_draft',data:'other'}}),c=>({...c,selection:{...c.selection,run_id:'other'}}),c=>({...c,selection:{...c.selection,at_ms:40000}})]){
    let release;const deferred=new Promise(resolve=>release=resolve);
    const s=setup({dispatch:async q=>{if(q.operation==='get_run'&&q.params.query.kind==='full_result'){await deferred;return {kind:'full',data:base.run};}}});
    const pending=s.host.read(s.context);s.switchTo(switcher(s.context));release();await assert.rejects(pending,ProcessSourceChanged);
  }
});
test('protocol: exact dependency metadata and real declared ArrayBuffer bytes retain frozen provenance',async()=>{
  const bytes=await readFile(new URL('modules/media/p302-controlled/station.png',root));
  const s=setup({bytes:async()=>bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength)});
  const value=await s.host.read(s.context),reader=s.host.media(s.context,value.binding);
  const asset=value.module.assets.find(a=>a.kind==='png');assert.ok(asset);
  const result=await reader.readAsset({...value.binding.request,asset});assert.ok(result instanceof ArrayBuffer);assert.deepEqual([...new Uint8Array(result).slice(0,8)],[137,80,78,71,13,10,26,10]);
  const dependency=structuredClone(image);s.packages.push(dependency);s.pkg.dependencies.push(ref(dependency.definition));
  const dep={kind:'run_module_version',data:{...value.binding.request.source.data,module_ref:ref(dependency.definition)}};
  assert.equal((await reader.resolveModule(dep)).definition.id,dep.data.module_ref.id);
  await reader.readAsset({...value.binding.request,source:dep,asset});assert.equal(s.reads[1].source.data.module_ref.id,dep.data.module_ref.id);
  await assert.rejects(reader.resolveModule({kind:'module_version',data:{module_ref:dep.data.module_ref}}),/改变了Run/);
  await assert.rejects(reader.readAsset({...value.binding.request,source:{...dep,data:{...dep.data,at_ms:99}},asset}),/改变了已绑定/);
});
test('protocol: late bytes and failures do not cross the active source or become successful media',async()=>{
  let release;const deferred=new Promise(resolve=>release=resolve);
  const s=setup({bytes:()=>deferred}),value=await s.host.read(s.context),reader=s.host.media(s.context,value.binding);
  const pending=reader.readAsset({...value.binding.request,asset:value.module.assets[0]});s.switchTo({...s.context,scope_revision:10});release(new ArrayBuffer(1));await assert.rejects(pending,ProcessSourceChanged);
  const failure=new Error('actual byte failure'),a=setup({bytes:async()=>{throw failure;}}),definition=await a.host.read(a.context);
  await assert.rejects(a.host.media(a.context,definition.binding).readAsset({...definition.binding.request,asset:definition.module.assets[0]}),e=>e===failure);
});
test('protocol: declaration reads original Draft definition without fabricating a ModulePackage',async()=>{
  const s=setup();s.context.selection={kind:'definition',module_ref:ref(s.pkg.definition)};s.context.scope={kind:'definition_draft',data:'draft-1'};s.context.scope_revision=4;
  const value=await s.host.read(s.context);assert.equal('run' in value,false);assert.equal(value.binding.request.source.kind,'module_version');
  assert.equal(value.binding.request.scope_revision,4);assert.deepEqual(s.calls.map(c=>c.operation),['get_context','get_object','get_context']);
  assert.equal(s.calls[1].params.object_id.kind,'definition_draft');assert.equal(s.calls[1].params.source.kind,'draft');
  assert.equal(value.module,null);assert.equal(value.definition,s.pkg.definition);assert.equal(value.raw_definition.definition,s.pkg.definition);
  assert.equal(value.rights,s.pkg.definition.contribution.assets);
  const reader=s.host.media(s.context,value.binding);
  await assert.rejects(reader.resolveModule({kind:'module_version',data:{module_ref:ref(s.packages[0].definition)}}),/不支持DefinitionDraft/);
  await reader.readAsset({...value.binding.request,asset:s.pkg.assets[0]});assert.equal(s.reads[0].scope.kind,'definition_draft');
});
test('protocol: action tags call only declared Root ports with exact frozen identity',async()=>{
  const s=setup(),value=await s.host.read(s.context),called=[];
  s.pkg.definition.payload.data.operations=['configure','inspect','preview_cycle'];
  for(const action of ['configure','inspect','preview_cycle']){
    const result=await s.host.action(s.context,value,action,{[action]:async request=>{called.push(request);return {message:'actual Root port result'};}});
    assert.equal(result.message,'actual Root port result');
  }
  assert.deepEqual(called.map(v=>v.action),['configure','inspect','preview_cycle']);
  assert.ok(called.every(v=>v.project_id===s.project.id&&v.run_id===s.frame.run.id&&v.device_id==='CNC'&&v.job_id===s.job.id&&v.at_ms===35000));
  assert.ok(called.every(v=>v.binding.request.source.kind==='run_module_version'&&v.scope_revision===9));
  await assert.rejects(s.host.action(s.context,value,'inspect',{}),/unavailable/);
  s.pkg.definition.payload.data.operations=[];
  await assert.rejects(s.host.action(s.context,value,'inspect',{inspect:async()=>{throw new Error('must not execute');}}),/unavailable/);
});
test('protocol: action failure and late completion preserve the real result boundary',async()=>{
  const s=setup(),value=await s.host.read(s.context);s.pkg.definition.payload.data.operations=['configure'];
  const failure=new Error('Root form cancelled with local input retained');
  await assert.rejects(s.host.action(s.context,value,'configure',{configure:async()=>{throw failure;}}),error=>error===failure);
  let release;const delayed=new Promise(resolve=>release=resolve);
  const pending=s.host.action(s.context,value,'configure',{configure:()=>delayed});
  s.switchTo({...s.context,selection:{...s.context.selection,at_ms:40000}});release({message:'old result'});
  await assert.rejects(pending,ProcessSourceChanged);
});
test('protocol plus real shared projection: same stage/hold at actual Engine-derived boundaries, Run never mutates',async()=>{
  for(const [at,phase,progress] of [[35000,'auto',1],[40000,'unload',0],[45000,'unload',1]]){
    const frame=frames.find(v=>v.label===`calendar@${at}`).frame,s=setup({frame});
    const original=JSON.stringify(s.frame.run),value=await s.host.read(s.context),point=projectPhase(value.frame);
    assert.equal(point.known,true);assert.equal(point.display_phase_id,phase);assert.equal(point.progress,progress);
    assert.equal(JSON.stringify(s.frame.run),original);assert.equal(JSON.stringify(value.raw.run),original);
    assert.ok(s.calls.every(c=>['get_context','get_object','get_run'].includes(c.operation)));
  }
});
