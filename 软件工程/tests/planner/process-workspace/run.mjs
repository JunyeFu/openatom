import {spawnSync} from 'node:child_process';
import {writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-root-consumer/',root);
await mkdir(out,{recursive:true});
const results={};
async function run(name,args){
  const value=spawnSync(process.execPath,args,{cwd:fileURLToPath(root),encoding:'utf8'});
  await writeFile(new URL(`${name}.stdout.txt`,out),value.stdout??'');
  await writeFile(new URL(`${name}.stderr.txt`,out),value.stderr??'');
  if(value.error)throw value.error;
  results[name]={exit_code:value.status};
  process.stdout.write(`${name}: ${value.status}\n`);
  return value.status===0;
}
await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','--project','apps/planner/tsconfig.json']);
await run('service-protocol',['--test','tests/planner/process-workspace/presentationHost.test.mjs']);
if(await run('real-source-ssr-build',['tests/planner/process-workspace/build.mjs'])){
  await run('react-and-adapter-contract',['--test','tests/planner/process-workspace/root-contract.test.mjs']);
}else results['react-and-adapter-contract']={skipped:'SSR build failed; no stale output consumed'};
await writeFile(new URL('verification-results.json',out),JSON.stringify({
  evidence_level:'protocol_fixtures_real_shared_source_static_React_no_UI_Host_Native',
  service_protocol_tests:13,static_react_tests:3,real_W02_adapter_element_contracts:2,
  original_engine_tests_modified:false,whole_task_done:false,results,
},null,2)+'\n');
if(Object.values(results).some(value=>value.exit_code!==0))process.exitCode=1;
