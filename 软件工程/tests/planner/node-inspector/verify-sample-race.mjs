import {spawn} from 'node:child_process';import {readdir,readFile,writeFile,mkdir,copyFile,cp,stat} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-inspector/',root),prior=new URL('before-root-sample-race/',out);
await mkdir(prior,{recursive:true});
if((await readdir(prior)).length)throw new Error('Prior frozen evidence already preserved; do not overwrite it');
for(const file of await readdir(out))if(/\.(?:json|md|png|stdout\.txt|stderr\.txt)$/.test(file))await copyFile(new URL(file,out),new URL(file,prior));
await cp(new URL('final-source/',out),new URL('final-source/',prior),{recursive:true,errorOnExist:true,force:false});
for(const [from,to] of [['web/tests/planner/node-inspector/harness.html','previous-harness.html'],['planner-web/index.html','previous-planner-index.html']])await copyFile(new URL(from,out),new URL(to,prior));
const previous=JSON.parse(await readFile(new URL('source-cut.json',out),'utf8'));
for(const file of previous.files){const actual=await readFile(new URL(file.path,root)),frozen=await readFile(new URL(file.snapshot,out));if(!actual.equals(frozen))throw new Error('Frozen consumer source changed: '+file.path);}
const results={};
async function run(name,args){const result=await new Promise(resolve=>{const child=spawn(process.execPath,args,{cwd:fileURLToPath(root),windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',v=>stdout+=v);child.stderr.on('data',v=>stderr+=v);child.on('exit',exit_code=>resolve({exit_code,stdout,stderr}));});await writeFile(new URL('sample-race-'+name+'.stdout.txt',out),result.stdout);await writeFile(new URL('sample-race-'+name+'.stderr.txt',out),result.stderr);results[name]={command:[process.execPath,...args],exit_code:result.exit_code};console.log(name+': '+result.exit_code);if(result.exit_code!==0)process.exitCode=1;}
await run('normal-product-cycle',['tests/planner/node-inspector/product-cycle.mjs']);
await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','apps/planner/tsconfig.json']);
await run('harness-typecheck',['node_modules/typescript/bin/tsc','--noEmit','-p','tests/planner/node-inspector/tsconfig.json']);
await run('build',['tests/planner/node-inspector/build.mjs']);
const cycle=JSON.parse(await readFile(new URL('product-cycle.json',out),'utf8'));
if(cycle.checks.length!==2||cycle.checks.some(c=>c.status!=='passed')||cycle.errors.length||cycle.warnings.length)throw new Error('Actual product cycle did not retain 2 clean passing checks');
const path='apps/planner/src/features/factory/Workbench.tsx',s=await stat(new URL(path,root));
await writeFile(new URL('sample-race-verification.json',out),JSON.stringify({at:new Date().toISOString(),results,previous_complete_cut:'before-root-sample-race/',consumer_sources_unchanged:previous.files.map(f=>f.path),Root_changed_source:{path,bytes:s.size,modified_at:s.mtime.toISOString()},normal_UI_checks:2,errors:cycle.errors,warnings:cycle.warnings,unchanged_checks_not_repeated:['normal product9','persistence/Scope4','all original regression groups'],new_fixed_sdk:'scenarios/媒体表现/runtime-node-inspector/core_consumer.exe',Cargo_or_relink:false,Native:false,release_closed:false},null,2));
