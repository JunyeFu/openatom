import {test,after} from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {openCore} from '../../node-inspector/core.mjs';
import {root,out,model,authoring,createNodeInspectorReader,workbenchEditor,consumerEditNode} from './load-source.mjs';
const core=await openCore('scenarios/图片时间轴/runtime/phase.input.json'),scope={kind:'project'},pid=core.ready.snapshot.project.id,evidence=[];
for(const directory of JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/registry.json',root),'utf8')))await core.rpc({kind:'register',directory});
const snapshot=()=>core.rpc({kind:'snapshot'}),ports={async dispatch(request){const reply=await core.rpc({kind:'dispatch',request});if(reply.outcome.status!=='ok')throw Object.assign(new Error(reply.outcome.data.message),{name:reply.outcome.data.code,detail:reply.outcome.data});return reply.outcome.data;}},request=(operation,params,sourceScope=scope)=>ports.dispatch({api_version:'0.1',request_id:'independent-'+crypto.randomUUID(),project_id:pid,scope:sourceScope,operation,params}),apply=async commands=>request('apply_commands',{base_revision:(await snapshot()).project.revision,commands});
after(async()=>{await writeFile(new URL('actual-workbench.json',out),JSON.stringify({evidence,records:core.records,stderr:core.stderr(),fixed_sdk:'scenarios/媒体表现/runtime-node-inspector/core_consumer.exe',Native:false,browser:false,normal_UI:false},null,2));await core.close();});
async function selection(owner={object_id:{kind:'module',id:'inspector.native'},source:{kind:'module_version',data:{module_ref:{id:'inspector.native',version:'1.0.0'}}}}){const p=(await snapshot()).project;return {context:{session_id:core.ready.context.host.session_id,project_id:pid,scope,scope_revision:p.revision,selection:{kind:'definition',device_id:'CNC',module_ref:p.devices.find(d=>d.id==='CNC').module_ref}},owner,node_index:1,lod:'detailed',asset:null,byte_source:{kind:'module_version',data:{module_ref:{id:'inspector.native',version:'1.0.0'}}}};}
async function editorFor(s,{afterCreate,afterDispatch,refreshFailure,readDraft=authoring.readDraft}={}){
  const state={messages:[],draft:null,authoring:null,commits:[]},workspaceRef={current:{project:(await snapshot()).project,context:{host:{session_id:s.context.session_id}}}},deviceProcessRef={current:{id:'independent-process-origin',scope:s.context.scope,selection:s.context.selection}},actionCount={current:0};
  if(s.context.scope.kind==='definition_draft')deviceProcessRef.current.draft=await authoring.readDraft(ports,pid,s.context.scope.data);
  const io={async dispatch(q){const value=await ports.dispatch(q);if(q.operation==='apply_commands'&&q.params.commands.some(c=>c.kind==='create_definition_draft')){state.commits.push(value);if(afterCreate)await afterCreate({value,state,workspaceRef,deviceProcessRef});}if(afterDispatch)await afterDispatch({q,value,state});return value;}};
  const deps={workspaceRef,deviceProcessRef,authoringIO:io,actionCount,authoringRequest:authoring.authoringRequest,readDraft,
    setBusy:()=>{},setFailure:v=>{state.failure=v;},setMessage:v=>state.messages.push(v),rememberCommit:c=>{state.remembered=c;},
    refreshLatest:{current:async()=>{const p=await snapshot();workspaceRef.current={...workspaceRef.current,project:p.project};if(refreshFailure)throw refreshFailure;return p;}},
    setDeviceProcess:v=>{deviceProcessRef.current=v;},setDeliveryTab:()=>{},setLibraryOpen:()=>{},setSource:()=>{},setAuthoringDraft:v=>{state.draft=v;},setAuthoringPresentation:()=>{},setAuthoring:v=>{state.authoring=v;}};
  const invoke=await workbenchEditor(deps),key=model.nodeSelectionKey(s);state.currentSource=s;
  return {invoke:request=>invoke({...request,assertCurrent:request.assertCurrent??(()=>{
    if(!state.currentSource||model.nodeSelectionKey(state.currentSource)!==key)throw new Error('independent current source changed');
  })}),state,io,workspaceRef,deviceProcessRef};
}
test('actual Root Workbench published exact source creates real Draft and leaves current Project/old Run unchanged',async()=>{
  const s=await selection(),inspection=await createNodeInspectorReader(()=>s,ports).read(s),before=await snapshot(),old=await request('get_run',{run_id:core.ready.formal_run.id,query:{kind:'full_result'}}),editor=await editorFor(s);
  await editor.invoke({...inspection,target:model.nodeEditTarget(s)});assert.equal(editor.state.commits.length,1);assert.equal(editor.state.draft.summary.draft_id,editor.state.commits[0].scope.data.id);
  assert.equal(editor.state.authoring.draft.summary.draft_id,editor.state.draft.summary.draft_id);assert.equal(editor.state.draft.definition.id,'inspector.native');assert.deepEqual(await snapshot(),before);assert.deepEqual(await request('get_run',{run_id:core.ready.formal_run.id,query:{kind:'full_result'}}),old);
  evidence.push({name:'actual Workbench exact module seed',s,commit:editor.state.commits[0],draft:editor.state.draft,project_unchanged:true});
});
test('actual inline native model edits selected node only through original PutObject and actual RevisionConflict is preserved',async()=>{
  const p=(await snapshot()).project,parent=p.devices.find(d=>d.id==='CNC'),presentation=JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/inline-native.json',root),'utf8'));
  await apply([{kind:'put_object',data:{kind:'device',data:{...parent,presentation_override:presentation}}}]);
  const s=await selection({object_id:{kind:'device',id:'CNC'},source:{kind:'draft'}});s.byte_source={kind:'module_version',data:{module_ref:parent.module_ref}};
  const inspected=await createNodeInspectorReader(()=>s,ports).read(s),fields=model.inlineNodeFields(inspected.node);fields.name='independent selected node';fields.transform={...fields.transform,translation:{...fields.transform.translation,x:1.25}};
  const changed=model.withInlineNode(inspected.parent_device,s.node_index,fields),old=await request('get_run',{run_id:core.ready.formal_run.id,query:{kind:'full_result'}});
  const commit=await request('apply_commands',{base_revision:s.context.scope_revision,commands:[{kind:'put_object',data:{kind:'device',data:changed}}]});
  const reread=await request('get_object',{object_id:{kind:'device',id:'CNC'},source:{kind:'draft'},node_index:1});assert.equal(reread.data.name,fields.name);assert.equal(reread.data.transform.translation.x,1.25);assert.deepEqual(changed.presentation_override.nodes[0],presentation.nodes[0]);
  await assert.rejects(request('apply_commands',{base_revision:s.context.scope_revision,commands:[{kind:'put_object',data:{kind:'device',data:changed}}]}),e=>e.detail?.code==='RevisionConflict');
  assert.deepEqual(await request('get_run',{run_id:core.ready.formal_run.id,query:{kind:'full_result'}}),old);evidence.push({name:'actual inline PutObject conflict and old Run isolation',commit,reread,unsubmitted_fields_preserved:fields});
});
test('successful create followed by source switch must report actual created Draft identity, not only generic stale response',async()=>{
  const s=await selection(),inspection=await createNodeInspectorReader(()=>s,ports).read(s),editor=await editorFor(s,{afterCreate:({deviceProcessRef})=>{deviceProcessRef.current={...deviceProcessRef.current,id:'switched-entry-after-real-create'};}});
  let failure;try{await editor.invoke({...inspection,target:model.nodeEditTarget(s)});}catch(e){failure=e;}
  assert.equal(editor.state.commits.length,1,'real Host has already created one Draft');const created=editor.state.commits[0].scope.data.id,actual=await authoring.readDraft(ports,pid,created);
  evidence.push({name:'after-create process switch',actual_created_draft:created,real_commit:editor.state.commits[0],actual_readback:actual,error:failure?{message:failure.message,createdDraft:failure.createdDraft,creationCommit:failure.creationCommit}:null,visible_messages:editor.state.messages});
  assert.ok(failure,'must report change');
  assert.ok(failure.message.includes(created)||failure.createdDraft?.summary?.draft_id===created||failure.creationCommit?.scope?.data?.id===created,
    `Host actually created ${created}, but current UI error loses that identity: ${failure.message}`);
});
test('real Draft revision race during original readDraft preserves successful creation identity and original cause',async()=>{
  const s=await selection(),inspection=await createNodeInspectorReader(()=>s,ports).read(s);let changed=false;
  const editor=await editorFor(s,{afterDispatch:async({q,value,state})=>{if(!changed&&state.commits.length&&q.operation==='get_object'&&q.scope.kind==='definition_draft'&&value.kind==='definition'){
    changed=true;await request('apply_commands',{base_revision:1,commands:[{kind:'put_definition',data:{...value.data,title:value.data.title+' actual concurrent DraftR2'}}]},q.scope);
  }}});let failure;try{await editor.invoke({...inspection,target:model.nodeEditTarget(s)});}catch(e){failure=e;}
  assert.equal(editor.state.commits.length,1);const created=editor.state.commits[0].scope.data.id;
  assert.ok(changed);assert.ok(failure.message.includes(created));assert.match(failure.cause.message,/读取期间共同定义已变化/);assert.equal(editor.state.failure,failure);
  const actual=await authoring.readDraft(ports,pid,created);assert.equal(actual.summary.revision,2);assert.equal(editor.state.authoring,null);
  evidence.push({name:'real readDraft revision race after real creation',created,actual,error:failure.message,cause:failure.cause.message,automatic_retries:editor.state.commits.length-1});
});
test('refresh rejection after real create and successful readDraft retains created identity and same error cause',async()=>{
  const s=await selection(),inspection=await createNodeInspectorReader(()=>s,ports).read(s),cause=new Error('controlled consumer refresh rejection after actual SDK snapshot'),editor=await editorFor(s,{refreshFailure:cause});
  let failure;try{await editor.invoke({...inspection,target:model.nodeEditTarget(s)});}catch(e){failure=e;}
  assert.equal(editor.state.commits.length,1);const created=editor.state.commits[0].scope.data.id;
  assert.ok(failure.message.includes(created));assert.equal(failure.cause,cause);assert.equal(editor.state.failure,failure);assert.equal(editor.state.authoring,null);
  evidence.push({name:'controlled consumer refresh fault after actual create/readback',created,error:failure.message,cause:failure.cause.message,fixture_fault_not_Host_failure:true});
});
async function run(){const s=await snapshot(),task=await request('run_scenario',{scenario_id:s.scenarios[0].id,input_revision:s.project.revision,observation_window:s.scenarios[0].window,intent:{kind:'project_scenario'}}),done=(await core.rpc({kind:'await_task',scope,task_id:task.id})).outcome.data;assert.equal(done.task.state,'completed');return (await request('get_run',{run_id:done.result.data.run_id,query:{kind:'full_result'}})).data;}
const adopt=ref=>({kind:'adopt_module',data:{module_ref:ref,targets:[{kind:'device',id:'CNC'}],bindings:{item_types:{},resources:{},targets:{},ports:{}},new_instances:[],explicit_seed:null}});
test('actual Root frozen GLB source to original authoring put/commit and explicit adoption preserves GLB identity rights and old Run',async()=>{
  const ref={id:'inspector.glb',version:'1.0.0'};await apply([adopt(ref)]);const old=await run(),s=await selection(),job=old.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id==='CNC');
  s.context.selection={kind:'actual_run',run_id:old.id,job_id:job.id,at_ms:75000};s.owner=model.moduleNodeOwner({kind:'run_module_version',data:{module_ref:ref,run_id:old.id,at_ms:75000}});s.byte_source=s.owner.source;s.node_index=0;
  const parent=await request('get_object',{object_id:s.owner.object_id,source:s.owner.source,node_index:null});s.asset=parent.data.assets.find(a=>a.kind==='glb');
  const inspection=await createNodeInspectorReader(()=>s,ports).read(s);assert.equal(inspection.node.id,1053);assert.equal(inspection.node.geometry.data.node_index,53);
  const editor=await editorFor(s);await editor.invoke({...inspection,target:model.nodeEditTarget(s)});const draft=editor.state.draft;assert.deepEqual(draft.definition,parent.data.definition);
  const definition=structuredClone(draft.definition);definition.payload.data.nodes[0].name='independent frozen GLB edit';definition.payload.data.nodes[0].transform.translation.x=1.75;
  const draftScope={kind:'definition_draft',data:draft.summary.draft_id};await request('apply_commands',{base_revision:draft.summary.revision,commands:[{kind:'put_definition',data:definition}]},draftScope);
  await request('apply_commands',{base_revision:2,commands:[{kind:'commit_definition_version',data:{version:'9.1.0'}}]},draftScope);
  assert.equal((await snapshot()).project.devices.find(d=>d.id==='CNC').presentation_override.source.data.version,'1.0.0');
  await apply([adopt({...ref,version:'9.1.0'})]);const current=await request('get_object',{object_id:s.owner.object_id,source:{kind:'module_version',data:{module_ref:{...ref,version:'9.1.0'}}},node_index:0});
  assert.equal(current.data.name,definition.payload.data.nodes[0].name);assert.deepEqual(current.data.geometry,inspection.node.geometry);assert.deepEqual(current.data.material,inspection.node.material);assert.equal(current.data.parent_index,inspection.node.parent_index);
  const frozen=await request('get_object',{object_id:s.owner.object_id,source:s.owner.source,node_index:0});assert.deepEqual(frozen.data,inspection.node);assert.deepEqual((await request('get_run',{run_id:old.id,query:{kind:'full_result'}})).data,old);
  const draftRead=await authoring.readDraft(ports,pid,draft.summary.draft_id);assert.deepEqual(draftRead.definition.contribution,parent.data.definition.contribution);
  const bytes=await core.rpc({kind:'media',project_id:pid,scope:draftScope,scope_revision:3,source:{kind:'module_version',data:{module_ref:{...ref,version:'9.1.0'}}},asset:current.data.geometry.data.asset_ref});assert.equal(bytes.ok,true,JSON.stringify(bytes.error));assert.equal(new TextDecoder().decode(Uint8Array.from(bytes.bytes).slice(0,4)),'glTF');
  evidence.push({name:'actual Root frozen GLB complete explicit workflow',source:s,created:draft.summary,committed_ref:{...ref,version:'9.1.0'},old_node:frozen.data,current_node:current.data,bytes:bytes.bytes.length,old_run_unchanged:true});
});
test('actual Root frozen Device seed uses same Run base plus its exact override while current head differs',async()=>{
  const p=(await snapshot()).project,device=p.devices.find(d=>d.id==='CNC'),native=JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/inline-native.json',root),'utf8'));
  await apply([{kind:'put_object',data:{kind:'device',data:{...device,presentation_override:native}}}]);const old=await run(),head=(await snapshot()).project.devices.find(d=>d.id==='CNC'),changed=structuredClone(head);changed.presentation_override.nodes[1].transform.translation.x=8.5;await apply([{kind:'put_object',data:{kind:'device',data:changed}}]);
  const s=await selection(),job=old.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id==='CNC');s.context.selection={kind:'actual_run',run_id:old.id,job_id:job.id,at_ms:75000};s.owner={object_id:{kind:'device',id:'CNC'},source:{kind:'run_snapshot',data:{run_id:old.id,at_ms:75000}}};s.byte_source={kind:'run_module_version',data:{module_ref:device.module_ref,run_id:old.id,at_ms:75000}};
  const inspection=await createNodeInspectorReader(()=>s,ports).read(s),base=await request('get_object',{object_id:{kind:'module',id:device.module_ref.id},source:s.byte_source,node_index:null}),editor=await editorFor(s);
  await editor.invoke({...inspection,target:model.nodeEditTarget(s)});const draft=editor.state.draft;
  assert.deepEqual(draft.definition.payload.data.presentation,native);assert.deepEqual({...draft.definition.payload.data,presentation:null},{...base.data.definition.payload.data,presentation:null});assert.deepEqual(draft.definition.contribution,base.data.definition.contribution);
  assert.equal((await snapshot()).project.devices.find(d=>d.id==='CNC').presentation_override.nodes[1].transform.translation.x,8.5);assert.deepEqual((await request('get_run',{run_id:old.id,query:{kind:'full_result'}})).data,old);
  evidence.push({name:'actual Root frozen Device source seed',selection:s,created:draft.summary,presentation:draft.definition.payload.data.presentation,current_override_x:8.5,old_run_unchanged:true,adoption_not_automatic:true});
});
test('normal consumer node or LOD change during real source read cannot create or navigate from the old node',async()=>{
  for(const kind of ['clear_node_or_LOD','close_then_reopen_same_key']){
  const s=await selection(),inspection=await createNodeInspectorReader(()=>s,ports).read(s),currentNode={current:s},nodeGeneration={current:1};let invalidated=false;
  const editor=await editorFor(s,{afterDispatch:({q})=>{if(!invalidated&&q.operation==='get_object'&&q.params.node_index===null&&q.params.object_id.kind==='module'){
    invalidated=true;currentNode.current=kind==='clear_node_or_LOD'?null:structuredClone(s);nodeGeneration.current++; // navigation/close and reopen invalidates even an equal source key
  }}});
  const edit=await consumerEditNode({editing:null,currentNode,nodeGeneration,nodeSelectionKey:model.nodeSelectionKey,ProcessSourceChanged:class extends Error {},setInlineEdit:()=>{},props:{onEditNodeSource:editor.invoke}});
  let failure;try{await edit({...inspection,target:model.nodeEditTarget(s)});}catch(e){failure=e;}
  evidence.push({name:'consumer node/LOD clear during actual Root parent read',kind,invalidated,created:editor.state.commits,error:failure?.message,navigated_to:editor.state.authoring?.draft?.summary});
  assert.ok(invalidated);assert.equal(editor.state.commits.length,0,'old node/LOD was cleared before create but Host still received create_definition_draft');assert.equal(editor.state.authoring,null);
  }
});
test('actual DefinitionCycle frozen Draft offers an edit source only if the Root path can really create it',async()=>{
  const p=(await snapshot()).project,device=p.devices.find(d=>d.id==='CNC'),pkg=await request('get_object',{object_id:{kind:'module',id:device.module_ref.id},source:{kind:'module_version',data:{module_ref:device.module_ref}},node_index:null}),definition=structuredClone(pkg.data.definition);
  definition.payload.data.presentation=JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/inline-native.json',root),'utf8'));
  const texture=pkg.data.assets[0];assert.ok(texture,'actual Device package must supply its original owned image');
  definition.payload.data.presentation.nodes[1].material.texture=texture;
  const created=await apply([{kind:'create_definition_draft',data:definition}]),draftScope={kind:'definition_draft',data:created.scope.data.id},scenario=(await snapshot()).scenarios[0];
  const task=await request('run_scenario',{scenario_id:scenario.id,input_revision:1,observation_window:scenario.window,intent:{kind:'definition_cycle',data:{definition_revision:1,test_input:scenario}}},draftScope),done=(await core.rpc({kind:'await_task',scope:draftScope,task_id:task.id})).outcome.data;
  assert.equal(done.task.state,'completed');const run=(await request('get_run',{run_id:done.result.data.run_id,query:{kind:'full_result'}},draftScope)).data,job=run.jobs[0],s=await selection();
  s.context={...s.context,scope:draftScope,scope_revision:1,selection:{kind:'actual_run',run_id:run.id,job_id:job.id,at_ms:75000}};s.owner={object_id:{kind:'definition_draft',id:draftScope.data},source:{kind:'run_snapshot',data:{run_id:run.id,at_ms:75000}}};s.byte_source={kind:'run_module_version',data:{module_ref:device.module_ref,run_id:run.id,at_ms:75000}};
  const beforeBytes=await core.rpc({kind:'media',project_id:pid,scope:draftScope,scope_revision:1,source:{kind:'run_module_version',data:{module_ref:device.module_ref,run_id:run.id,at_ms:75000}},asset:texture});assert.equal(beforeBytes.ok,true,JSON.stringify(beforeBytes.error));
  const head=structuredClone(definition);head.title+=' current DraftR2 must not seed frozen R1';head.payload.data.presentation.nodes[1].name='current head differs';
  await request('apply_commands',{base_revision:1,commands:[{kind:'put_definition',data:head}]},draftScope);s.context.scope_revision=2;
  const inspection=await createNodeInspectorReader(()=>s,ports).read(s),target=model.nodeEditTarget(s);assert.deepEqual(inspection.node,definition.payload.data.presentation.nodes[1]);
  if(!target){evidence.push({name:'DefinitionCycle frozen Draft explicitly inspect-only',selection:s,node:inspection.node,no_failing_edit_button:true});return;}
  const editor=await editorFor(s);let failure;try{await editor.invoke({...inspection,target});}catch(e){failure=e;}
  evidence.push({name:'DefinitionCycle frozen Draft advertised edit source',selection:s,target,node:inspection.node,error:failure?.message,created:editor.state.commits});
  assert.equal(failure,undefined,'advertised frozen Draft edit must not be a guaranteed failing Root branch');assert.equal(editor.state.commits.length,1);assert.deepEqual(editor.state.draft.definition,definition);
  const bytes=await core.rpc({kind:'media',project_id:pid,scope:{kind:'definition_draft',data:editor.state.draft.summary.draft_id},scope_revision:1,source:{kind:'module_version',data:{module_ref:device.module_ref}},asset:texture});assert.equal(bytes.ok,true,JSON.stringify(bytes.error));assert.deepEqual(bytes.bytes,beforeBytes.bytes);
  evidence.push({name:'DefinitionCycle frozen owned texture byte closure',source_draft_revision:2,frozen_definition_revision:1,new_draft:editor.state.draft.summary,asset:texture,bytes_equal:true,byte_count:bytes.bytes.length,current_head_not_used:true,no_Native_texture_decode_claim:true});
});
test('actual private DefinitionCycle final GLB owner can seed a new Draft with the same real frozen asset bytes',async()=>{
  const p=(await snapshot()).project,device=p.devices.find(d=>d.id==='CNC'),base=await request('get_object',{object_id:{kind:'module',id:device.module_ref.id},source:{kind:'module_version',data:{module_ref:device.module_ref}},node_index:null}),definition=structuredClone(base.data.definition),mediaRef={id:'inspector.glb',version:'9.1.0'};
  const presentation=JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/inline-native.json',root),'utf8'));presentation.nodes=[];presentation.source={kind:'media_ref',data:mediaRef};presentation.lod.detailed=presentation.source;presentation.lod.simplified=presentation.source;definition.payload.data.presentation=presentation;
  const created=await apply([{kind:'create_definition_draft',data:definition}]),draftScope={kind:'definition_draft',data:created.scope.data.id},scenario=(await snapshot()).scenarios[0],task=await request('run_scenario',{scenario_id:scenario.id,input_revision:1,observation_window:scenario.window,intent:{kind:'definition_cycle',data:{definition_revision:1,test_input:scenario}}},draftScope),done=(await core.rpc({kind:'await_task',scope:draftScope,task_id:task.id})).outcome.data;assert.equal(done.task.state,'completed');
  const run=(await request('get_run',{run_id:done.result.data.run_id,query:{kind:'full_result'}},draftScope)).data,source={kind:'run_module_version',data:{module_ref:mediaRef,run_id:run.id,at_ms:75000}},parent=await request('get_object',{object_id:{kind:'module',id:mediaRef.id},source,node_index:null},draftScope),s=await selection();
  s.context={...s.context,scope:draftScope,scope_revision:1,selection:{kind:'actual_run',run_id:run.id,job_id:run.jobs[0].id,at_ms:75000}};s.owner=model.moduleNodeOwner(source);s.byte_source=source;s.node_index=0;s.asset=parent.data.assets.find(a=>a.kind==='glb');
  const inspection=await createNodeInspectorReader(()=>s,ports).read(s),frozen=await core.rpc({kind:'media',project_id:pid,scope:draftScope,scope_revision:1,source,asset:s.asset});assert.equal(frozen.ok,true,JSON.stringify(frozen.error));
  const editor=await editorFor(s);await editor.invoke({...inspection,target:model.nodeEditTarget(s)});assert.deepEqual(editor.state.draft.definition,parent.data.definition);
  const bytes=await core.rpc({kind:'media',project_id:pid,scope:{kind:'definition_draft',data:editor.state.draft.summary.draft_id},scope_revision:1,source:{kind:'module_version',data:{module_ref:mediaRef}},asset:s.asset});assert.equal(bytes.ok,true,JSON.stringify(bytes.error));assert.deepEqual(bytes.bytes,frozen.bytes);assert.equal(new TextDecoder().decode(Uint8Array.from(bytes.bytes).slice(0,4)),'glTF');
  evidence.push({name:'actual private DefinitionCycle GLB frozen source to real Draft byte closure',selection:s,draft:editor.state.draft.summary,asset:s.asset,bytes:bytes.bytes.length,GLB_bytes_exact:true,rights_preserved:true,Native:false});
});
