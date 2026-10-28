// Execute unchanged original assertions, redirecting every output under the
// authorized P205 directory so frozen P204/W5 evidence remains read-only.
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/真实业务设计/runtime-ui/readback-repair/p204-regression/',root),scope=new URL('engine-scope/',out),engineOut=new URL('scenarios/从零搭建控制盒厂房/runtime-timeline/',scope),original=new URL('scenarios/从零搭建控制盒厂房/runtime-timeline/',root);
await mkdir(engineOut,{recursive:true});await mkdir(new URL('packages/commands/tests/fixtures/production/',scope),{recursive:true});
for(const name of ['p202-ix01-baseline.json','p202-ix01-delayed-pcb.json','p1-phase-calendar.json'])await copyFile(new URL('packages/commands/tests/fixtures/production/'+name,root),new URL('packages/commands/tests/fixtures/production/'+name,scope));
await copyFile(new URL('consume_engine.exe',original),new URL('consume_engine.exe',engineOut));
function replace(text,from,to){assert.ok(text.includes(from),`original runner path changed: ${from}`);return text.replace(from,to);}
const loader=await readFile(new URL('tests/planner/timeline/load-source.mjs',root),'utf8');
await writeFile(new URL('load-source.mjs',out),replace(replace(loader,"new URL('../../../', import.meta.url)",`new URL(${JSON.stringify(root.href)})`),"new URL('scenarios/从零搭建控制盒厂房/runtime-timeline/compiled/', root)",`new URL(${JSON.stringify(new URL('compiled/',out).href)})`));
await copyFile(new URL('tests/planner/timeline/timeline.test.mjs',root),new URL('original-timeline.test.mjs',out));
let browser=await readFile(new URL('tests/planner/timeline/browser.mjs',root),'utf8');
browser=replace(browser,"const root = fileURLToPath(new URL('../../../', import.meta.url)), out = new URL('scenarios/从零搭建控制盒厂房/runtime-timeline/', new URL('../../../', import.meta.url));",`const root = fileURLToPath(new URL(${JSON.stringify(root.href)})), out = new URL(${JSON.stringify(engineOut.href)});`);
browser=replace(browser,'[root], { windowsHide:',`[${JSON.stringify(fileURLToPath(scope))}], { windowsHide:`);
await writeFile(new URL('original-browser.mjs',out),browser);
const runs=[];
for(const[name,args]of [['original21',['--test',fileURLToPath(new URL('original-timeline.test.mjs',out))]],['original17',[fileURLToPath(new URL('original-browser.mjs',out))]]]){
 const result=spawnSync(process.execPath,args,{encoding:'utf8',windowsHide:true,cwd:fileURLToPath(root)});await writeFile(new URL(`${name}.stdout.txt`,out),result.stdout);await writeFile(new URL(`${name}.stderr.txt`,out),result.stderr);runs.push({name,command:[process.execPath,...args],exit_code:result.status});console.log(`${name} exit ${result.status}\n${result.stdout.slice(-600)}`);
}
// The original preview still reads its frozen records. Confirm the isolated
// producer generated exactly those records before accepting this regression.
for(const name of ['baseline.actual.json','delayed-pcb.actual.json','phase-calendar.actual.json','baseline-prefix.actual.json'])assert.deepEqual(JSON.parse(await readFile(new URL(name,engineOut),'utf8')),JSON.parse(await readFile(new URL(name,original),'utf8')));
await writeFile(new URL('result.json',out),JSON.stringify({original_assertions_changed:false,original_sources_or_outputs_written:false,path_redirects_only:true,isolated_Engine_records_equal_original_frozen_records:true,Host_commit_test_double:'Original P204 browser callback remains its explicitly labelled test double; no Native claim',runs},null,2));
if(runs.some(r=>r.exit_code!==0))process.exitCode=1;
