import {spawn} from 'node:child_process';import {writeFile} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-inspector/',root),results={};
async function run(name,args){const result=await new Promise(resolve=>{const child=spawn(process.execPath,args,{cwd:fileURLToPath(root),windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',v=>stdout+=v);child.stderr.on('data',v=>stderr+=v);child.on('exit',exit_code=>resolve({exit_code,stdout,stderr}));});await writeFile(new URL(name+'.stdout.txt',out),result.stdout);await writeFile(new URL(name+'.stderr.txt',out),result.stderr);results[name]={command:[process.execPath,...args],exit_code:result.exit_code};console.log(name+': '+result.exit_code);if(result.exit_code!==0)process.exitCode=1;}
await run('pure-and-actual-reader',['--test','--test-concurrency=1','tests/planner/node-inspector/model.test.mjs','tests/planner/node-inspector/reader-actual.test.mjs','tests/planner/node-inspector/seed-actual.test.mjs']);
await run('normal-product',['tests/planner/node-inspector/product.mjs']);
await run('normal-product-boundaries',['tests/planner/node-inspector/product-boundaries.mjs']);
await run('normal-product-cycle',['tests/planner/node-inspector/product-cycle.mjs']);
await run('old-protocol',['--test','--test-concurrency=1',...['regression-factory.test.mjs','regression-process.test.mjs','regression-port-fields.test.mjs','regression-port-host.test.mjs'].map(n=>'tests/planner/node-inspector/'+n)]);
await run('old-media-browser',['tests/planner/node-inspector/regression-media-browser.mjs','green']);
await run('old-port-ui',['tests/planner/node-inspector/regression-port-browser.mjs']);
await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','apps/planner/tsconfig.json']);
await run('harness-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','tests/planner/node-inspector/tsconfig.json']);
await run('presentation-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','packages/presentation/tsconfig.json']);
await run('build',['tests/planner/node-inspector/build.mjs']);
await writeFile(new URL('verification-results.json',out),JSON.stringify({at:new Date().toISOString(),results,new_fixed_sdk:'scenarios/媒体表现/runtime-node-inspector/core_consumer.exe',Native:false,release_closed:false},null,2));
