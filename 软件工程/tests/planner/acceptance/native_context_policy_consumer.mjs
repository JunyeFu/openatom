// Explicit direct-Native consumer of original p104-B-first declarations.
// Same coordinator Native only. This is not GUI or an alternative Host/Engine.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
const require=createRequire(import.meta.url),{chromium}=require(process.argv[2]);
const root=new URL('../../../',import.meta.url);
const out=new URL('.work/whole-packages/window5-timeline-evaluation/native-context-72052/policy/',root);
await mkdir(out,{recursive:true});
const read=async url=>JSON.parse(await readFile(url,'utf8'));
const save=(name,value)=>writeFile(new URL(name,out),JSON.stringify(value,null,2));
const browser=await chromium.connectOverCDP(process.argv[3]);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().startsWith('devtools:'));
assert.equal((await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'))).host_instance_id,process.argv[4]);
const requestLog=[],checks=[];
async function api(operation,params,scope={kind:'project'},projectId=pid) {
 const request={api_version:'0.1',request_id:'w5-policy-'+crypto.randomUUID(),project_id:projectId,scope,operation,params};
 const response=await page.evaluate(request=>window.__TAURI_INTERNALS__.invoke('project_api',{request}),request);
 requestLog.push({request,response});
 if(response.outcome.status!=='ok')throw new Error(JSON.stringify(response));
 return response.outcome.data;
}
const initial=await api('get_context',{selection:[]},{kind:'workspace'},null),pid=initial.project.id;
assert.equal(pid,'desktop-host-72052:project:2');assert.equal(initial.project.revision,14);
const object=async(kind,id,source={kind:'draft'},scope={kind:'project'})=>(await api('get_object',{object_id:{kind,id},source,node_index:null},scope)).data;
const project=()=>object('project',pid);
const full=async id=>(await api('get_run',{run_id:id,query:{kind:'full_result'}})).data;
const apply=(base_revision,commands,scope={kind:'project'})=>api('apply_commands',{base_revision,commands},scope);
const oldRuns=await Promise.all(initial.run_summaries.map(r=>full(r.run_id)));
const fixture=await read(new URL('packages/commands/tests/fixtures/production/p104-B-first.json',root));
fixture.scenario.orders=fixture.scenario.orders.filter(o=>o.id==='B');
fixture.scenario.production_tasks=fixture.scenario.production_tasks.filter(t=>t.id==='TASK-B');
fixture.project.setup_policies[0].first_article={route_ref:{id:'ROUTE-B',revision:'r1'},family:'B',inspection_module:{id:'w5.op.inspect',version:'0.1.0'},gate_id:'B-FIRST-ARTICLE'};
await save('original-input-and-expectations.json',{fixture,expected:{
 source_and_declared_inspection_starts_ms:[15000,55000],declared_inspection_acceptance_ms:[145000],
 declared_inspection_accepted_quantity:2,without_inspection_starts_ms:[15000],
 without_inspection_acceptance_ms:[],without_inspection_accepted_quantity:0,
 scope:'Original whole-Project empty scope retained; no fabricated PASS, price or goal.'
}});
const task=async(handle,scope={kind:'project'})=>{
 for(;;){const value=await api('get_task',{task_id:handle.id},scope);if(['completed','failed','cancelled'].includes(value.task.state))return value;await new Promise(r=>setTimeout(r,50));}
};
async function run(id) {
 const p=await project(),s=await object('scenario',id);
 const handle=await api('run_scenario',{scenario_id:id,input_revision:p.revision,observation_window:s.window,intent:{kind:'project_scenario'}});
 const terminal=await task(handle);assert.equal(terminal.task.state,'completed',JSON.stringify(terminal.failure));
 assert.equal(terminal.result.kind,'run');return {task:terminal,run:await full(terminal.result.data.run_id)};
}
const starts=run=>run.jobs.filter(j=>j.operation_ref.id==='w5.op.assembly-B'&&j.input_refs.length).map(j=>j.started_at_ms);
async function preserveOld(){for(const run of oldRuns)assert.deepEqual(await full(run.id),run);}
try {
 // Preserve existing target objects/policies. Register exact fixture definitions
 // without creating a Host or substituting definitions in an existing Run.
 for(const definition of fixture.definitions) {
  const created=await apply(14,[{kind:'create_definition_draft',data:definition}]);
  assert.equal(created.scope.kind,'definition_draft');
  await apply(created.new_revision,[{kind:'commit_definition_version',data:{version:definition.version}}],{kind:'definition_draft',data:created.scope.data.id});
 }
 const before=await project();assert.equal(before.revision,14);
 const commands=[{kind:'set_project_settings',data:{title:before.title,time_reference:fixture.project.time_reference,
  generation_context:before.generation_context,sources:before.sources,
  setup_policies:[...before.setup_policies,...fixture.project.setup_policies]}}];
 for(const ref of fixture.project.module_refs)commands.push({kind:'adopt_module',data:{module_ref:ref,targets:[],bindings:{item_types:{},resources:{},targets:{},ports:{}},new_instances:[],explicit_seed:null}});
 const groups=[['item_types','item_type'],['locations','location'],['halls','hall'],['lines','line'],['devices','device'],['resources','resource'],['calendars','calendar'],['products','product'],['boms','bom'],['routes','route'],['connections','connection']];
 for(const [field,kind] of groups)for(const data of fixture.project[field])commands.push({kind:'put_object',data:{kind,data}});
 fixture.scenario.project_revision=before.revision;
 commands.push({kind:'put_object',data:{kind:'scenario',data:fixture.scenario}});
 await save('explicit-fixture-load-commands.json',{scope:'direct Native on own existing Project',before_time_reference:before.time_reference,fixture_time_reference:fixture.project.time_reference,commands});
 await apply(before.revision,commands);
 const sourceProject=await project(),source=await object('scenario',fixture.scenario.id);
 assert.equal(sourceProject.revision,15);assert.deepEqual(source.scope,fixture.scenario.scope);
 assert.deepEqual(source.cost_inputs,fixture.scenario.cost_inputs);
 assert.deepEqual(source.parameter_overrides,fixture.scenario.parameter_overrides);
 assert.deepEqual(sourceProject.setup_policies.slice(0,before.setup_policies.length),before.setup_policies);
 await preserveOld();
 const original=await run(source.id);assert.deepEqual(starts(original.run),[15000,55000]);
 assert.deepEqual(original.run.acceptance_events.map(e=>e.at_ms),[145000]);
 await save('source-actual-Run.json',original);
 checks.push({name:'original_p104_B_first_source_on_same_actual_Native',passed:true,starts_ms:starts(original.run),accepted_ms:[145000]});
 const definition={id:'w5.policy.context.72052',version:'1.0.0',title:'W5 original policy complete context',definition_state:'design',contribution:{author:null,license:null,assets:[]},parameters:[],attribute_schema:[],payload:{category:'templates',data:{project:'template/project.subset.json',generation_profile:null}}};
 const created=await apply(15,[{kind:'create_definition_draft',data:definition}]);
 const draftScope={kind:'definition_draft',data:created.scope.data.id};
 const subset={layout_scope:'region',halls:sourceProject.halls,lines:sourceProject.lines,devices:sourceProject.devices,connections:sourceProject.connections,module_refs:sourceProject.module_refs,
  resource_requirements:sourceProject.resources.map(r=>({kind:r.kind,role:r.role,capacity:r.capacity,qualifications:r.qualifications,default_calendar_ms:[],presentation:null})),
  item_types:sourceProject.item_types,role_requirements:[],suggested_role_bindings:{item_types:{},resources:{},targets:{},ports:{}},
  seed:{initial_conditions:source.initial_conditions,arrivals:source.arrivals,orders:source.orders},locations:sourceProject.locations,adoption:null};
 await apply(1,[{kind:'put_template_composition',data:{project_revision:15,definition,subset,source_scenario:{scenario_id:source.id,revision:source.revision}}}],draftScope);
 const committed=await apply(2,[{kind:'commit_definition_version',data:{version:'1.0.0'}}],draftScope);
 await apply(15,[{kind:'adopt_module',data:{module_ref:{id:definition.id,version:'1.0.0'},targets:[],bindings:{item_types:{},resources:{},targets:{},ports:{}},new_instances:[],explicit_seed:null}}]);
 const ref={id:definition.id,version:'1.0.0'};
 for(const [namespace,scenarioId,inspection,expectedStarts,expectedAccepted] of [
  ['W5-POLICY-UNKNOWN','W5-POLICY-UNKNOWN-SEED',false,[15000],[]],
  ['W5-POLICY-KNOWN','W5-POLICY-KNOWN-SEED',true,[15000,55000],[145000]]
 ]) {
  const p=await project();
  const request=(choices,seed)=>({object_id:{kind:'module',id:ref.id},source:{kind:'template_preview',data:{module_ref:ref,base_revision:p.revision,choices,seed}},node_index:null});
  const preview=(await api('get_object',request(null,null))).data;
  assert.equal(preview.source_scenario.revision,source.revision);
  const resources=Object.fromEntries(preview.resources.map(r=>[r.id,r.id]));
  const quality=Object.fromEntries([...preview.subset.seed.initial_conditions.items,...preview.subset.seed.arrivals.flatMap(a=>a.items)].map(i=>[i.id,i.quality]));
  const prepared=(await api('get_object',request({source_project:preview.source_project,namespace,resources,reuse_items:{},include_example_product:true,site_envelope:null},
   {scenario_id:scenarioId,supply:true,demand:true,initial_quality:quality,adopt_declared_inspection_results:inspection}))).data;
  assert.ok(prepared.command);
  assert.deepEqual(prepared.command.project_setup_policies.slice(0,p.setup_policies.length),p.setup_policies);
  await apply(p.revision,[{kind:'instantiate_template',data:prepared.command}]);
  const input=await object('scenario',scenarioId);
  assert.equal(input.quality_presets.length,inspection?4:0);
  assert.deepEqual(input.scope,source.scope);
  assert.deepEqual(input.cost_inputs,source.cost_inputs);
  assert.deepEqual(input.parameter_overrides,source.parameter_overrides);
  const actual=await run(scenarioId);
  assert.deepEqual(starts(actual.run),expectedStarts);
  assert.deepEqual(actual.run.acceptance_events.map(e=>e.at_ms),expectedAccepted);
  assert.equal(actual.run.acceptance_events.reduce((sum,e)=>sum+e.quantity.value.coefficient,0),inspection?2:0);
  assert.deepEqual(await full(original.run.id),original.run);
  await preserveOld();
  await save(namespace+'.json',{preview,prepared,input,result:actual});
  checks.push({name:namespace,passed:true,starts_ms:expectedStarts,acceptance_ms:expectedAccepted,
   declared_inspection_adopted:inspection,original_quality_presets:4,scope_preserved:true,old_Runs_unchanged:true});
 }
 const last=await project();
 await save('final-Native-context.json',await api('get_context',{selection:[]},{kind:'workspace'},null));
 await save('final-Project.json',last);
} catch(error) {checks.push({name:'actual_native_policy_failure',passed:false,failure:String(error)});}
await save('public-Native-request-response-log.json',requestLog);
await save('result.json',{mode:'direct actual Native; not GUI',initial,checks,
 old_independent_Run_count:oldRuns.length,no_new_Host:true,no_picker_token_fabricated:true,
 original_source_fixture_and_numbers_preserved:true,whole_task_done:false,human_acceptance:false});
for(const row of checks)console.log(JSON.stringify(row));
process.exit(checks.length===3&&checks.every(c=>c.passed)?0:1);
