import {spawnSync} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-factory-items/',root),results={};
await mkdir(out,{recursive:true});
async function run(name,args,executable=process.execPath){
 const r=spawnSync(executable,args,{cwd:fileURLToPath(root),encoding:'utf8',windowsHide:true});
 await writeFile(new URL(name+'.stdout.txt',out),r.stdout??'');await writeFile(new URL(name+'.stderr.txt',out),r.stderr??'');
 results[name]={exit_code:r.status,...(r.error?{error:String(r.error)}:{})};process.stdout.write(`${name}: ${r.status}\n`);return r.status===0;
}
const built=await run('core-build-runner',['tests/planner/factory-actual-items/build-core.py'],'python');
if(built)await run('prepare-regressions',['tests/planner/factory-actual-items/prepare-regressions.mjs']);
await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','apps/planner/tsconfig.json']);
await run('harness-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','tests/planner/factory-actual-items/tsconfig.json']);
await run('build',['tests/planner/factory-actual-items/build.mjs']);
await run('new-protocol',['--test','tests/planner/factory-actual-items/source.test.mjs']);
if(built){
 await run('old-protocol',['--test','--test-concurrency=1',...['regression-factory-source.test.mjs','regression-port-fields.test.mjs','regression-port-host.test.mjs','regression-process-source.test.mjs'].map(n=>'tests/planner/factory-actual-items/'+n)]);
 await run('browser',['tests/planner/factory-actual-items/browser.mjs']);
 await run('old-factory-browser',['tests/planner/factory-actual-items/regression-factory-browser.mjs','green']);
 await run('old-port-ui',['tests/planner/factory-actual-items/regression-port-browser.mjs']);
}
await writeFile(new URL('verification-results.json',out),JSON.stringify({results,new_protocol_checks:7,new_browser_checks:{actual_host_source:4,explicit_internal_attachment_probe:2},regression_checks:{factory_protocol:13,process_protocol:13,port_protocol:12,factory_browser:7,port_browser:6,original_complete_runs:9},evidence_not_additive:true,Native_IPC:false,actual_spatial_binding_producer:false,whole_task_done:false,human_acceptance:null},null,2)+'\n');
if(Object.values(results).some(r=>r.exit_code!==0))process.exitCode=1;
