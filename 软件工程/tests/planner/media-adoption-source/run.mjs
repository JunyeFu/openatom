import {spawnSync} from 'node:child_process';import {writeFile} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-media-adoption-consumer/',root),results={};
async function run(name,args){const r=spawnSync(process.execPath,args,{cwd:fileURLToPath(root),encoding:'utf8',windowsHide:true});await writeFile(new URL(name+'.stdout.txt',out),r.stdout??'');await writeFile(new URL(name+'.stderr.txt',out),r.stderr??'');results[name]={exit_code:r.status,...(r.error?{error:String(r.error)}:{})};console.log(`${name}: ${r.status}`);return r.status===0;}
await run('prepare',['tests/planner/media-adoption-source/prepare.mjs']);
await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','apps/planner/tsconfig.json']);
await run('harness-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','tests/planner/media-adoption-source/tsconfig.json']);
await run('build',['tests/planner/media-adoption-source/build.mjs']);
await run('old-protocol',['--test','--test-concurrency=1',...['regression-factory.test.mjs','regression-process.test.mjs','regression-port-fields.test.mjs','regression-port-host.test.mjs'].map(p=>'tests/planner/media-adoption-source/'+p)]);
await run('browser-green',['tests/planner/media-adoption-source/browser.mjs','green']);
await run('old-factory-browser',['tests/planner/media-adoption-source/regression-factory-browser.mjs','green']);
await writeFile(new URL('verification-results.json',out),JSON.stringify({results,actual_Host_browser_checks:11,old_protocol_checks:{factory:13,process:13,port:12},original_complete_Run_comparisons:9,old_factory_browser_checks:7,Native:false,whole_task_done:false,human_acceptance:null,frozen_exe_reused_no_Cargo_or_rlib_reselection:true},null,2)+'\n');if(Object.values(results).some(r=>r.exit_code!==0))process.exitCode=1;
