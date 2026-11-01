import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { openCore, out } from './current-core.mjs';
const fixtureUrl=new URL(process.argv[2]??'b60-scaling-10.inline-allowlist-fixed.input.json',out),fixture=JSON.parse(await readFile(fixtureUrl,'utf8'));
const result={kind:'actual_post_scope_Host_B60_input_static_check',full_B60:false,Native_app:false};let core;
try{
  core=await openCore(fileURLToPath(fixtureUrl),'post-scopes-core.exe');const before=(await core.rpc({kind:'snapshot'})).project;
  const reply=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'P201-B60-explicit-scopes-check',project_id:before.id,scope:{kind:'project'},operation:'check_factory_draft',params:{revision:before.revision,generation_goal:fixture.scenario.acceptance_goal,check_scope:{objects:[{kind:'project',id:before.id}],check_references:true,check_business:true,check_static_layout:true}}}});
  assert.equal(reply.outcome.status,'ok');result.report=reply.outcome.data;assert.ok(result.report.checks.length>0);
  const failures=result.report.checks.filter(value=>value.outcome==='fail');assert.deepEqual(failures,[],'Prepared B60 references, ports and declared layout must have no known failure; absent goal site/clearance stays undetermined');
  assert.ok(result.report.checks.some(value=>value.outcome==='undetermined'));
  assert.deepEqual((await core.rpc({kind:'snapshot'})).project,before);
  const actual=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'P201-B60-old-Run-check',project_id:before.id,scope:{kind:'project'},operation:'get_run',params:{run_id:core.ready.formal_run.id,query:{kind:'full_result'}}}});
  assert.equal(actual.outcome.status,'ok');assert.deepEqual(actual.outcome.data.data,core.ready.formal_run);
  result.status='passed';result.revision_unchanged=before.revision;result.all_six_explicit_scopes=fixture.scenario.acceptance_goal;
}catch(error){result.status='failed';result.error=String(error);process.exitCode=1;}
finally{await writeFile(new URL('b60-checker-small.json',out),JSON.stringify(result,null,2));await core?.close();}
console.log(JSON.stringify({status:result.status,error:result.error?.slice(0,1200),checks:result.report?.checks.length,failure_count:result.report?.checks.filter(value=>value.outcome==='fail').length,Native_app:false,full_B60:false}));
