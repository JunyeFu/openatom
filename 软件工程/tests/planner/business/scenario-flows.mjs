// Original quality and actual declared-phase inputs through the real Core.
import {createRequire} from 'node:module';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
const root=new URL('../../../',import.meta.url),output=new URL('scenarios/真实业务设计/runtime-ui/',root);
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const url='http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/scenarios/真实业务设计/runtime-ui/preview.html';
const all=[];
async function flow(name,fixture,action){
 const child=spawn(fileURLToPath(new URL('core_consumer.exe',output)),[fileURLToPath(root),fixture],{windowsHide:true,stdio:['pipe','pipe','pipe']}),queue=[];
 let readyResolve,readyReject,stderr='';const initial=new Promise((a,b)=>{readyResolve=a;readyReject=b;});
 child.stderr.on('data',c=>stderr+=c);child.on('exit',c=>{if(c){const error=new Error(stderr||`Core exited ${c}`);readyReject(error);for(const waiting of queue.splice(0))waiting.reject(error);}});
 createInterface({input:child.stdout}).on('line',line=>{const v=JSON.parse(line);if(v.ready)readyResolve(v);else queue.shift()?.resolve(v);});
 const ready=await initial,requests=[],checks=[],errors=[],warnings=[];let serial=Promise.resolve();
 function rpc(v){requests.push(v);const p=serial.then(()=>new Promise((resolve,reject)=>{queue.push({resolve,reject});child.stdin.write(JSON.stringify(v)+'\n');}));serial=p;return p;}
 const page=await browser.newPage({viewport:{width:1500,height:1000}});
 page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
 await page.route('**/__p205_core*',async route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(route.request().method()==='GET'?ready:await rpc(route.request().postDataJSON()))}));
 const editor=page.getByRole('region',{name:'业务输入'}),read=()=>page.evaluate(()=>window.__businessRead()),evidence=()=>page.evaluate(()=>window.__businessEvidence);
 async function tab(n){await editor.getByRole('navigation',{name:'业务面板分类'}).getByRole('button',{name:n,exact:true}).click();}
 async function apply(){await editor.getByRole('button',{name:'检查变更',exact:true}).click();const d=editor.getByRole('dialog',{name:'业务变更预览'});await d.waitFor();await d.getByRole('button',{name:'确认应用草稿',exact:true}).click();await d.waitFor({state:'detached'});await page.waitForFunction(()=>document.querySelector('.sf-business header')?.textContent.includes('已同步'));}
 async function run(){const old=(await read()).run.id;await editor.getByRole('button',{name:'从初始输入重跑',exact:true}).click();await page.waitForFunction(old=>window.__businessRead().run.id!==old,old);return (await read()).run;}
 async function check(n,a){await a();checks.push({name:n,status:'passed'});console.log(`PASS ${name}: ${n}`);}
 try{await page.goto(url+(['shared-input','declared-phase'].includes(name)?'?shared-timeline=1':''));await editor.waitFor();await editor.getByRole('button',{name:'从初始输入重跑',exact:true}).waitFor();await action({page,editor,ready,read,evidence,rpc,tab,apply,run,check});assert.deepEqual(errors,[]);assert.deepEqual(warnings,[]);}
 catch(e){checks.push({name:'scenario flow',status:'failed',error:String(e),stack:e.stack});await page.screenshot({path:fileURLToPath(new URL(`${name}-failure.png`,output)),fullPage:true});process.exitCode=1;}
 finally{await page.screenshot({path:fileURLToPath(new URL(`${name}-desktop.png`,output)),fullPage:true});const record={name,fixture,checks,errors,warnings,evidence:await evidence().catch(()=>null),actual_Core_requests:requests,actual_Core_stderr:stderr,actual_ProjectHost:true,Host_or_Engine_mock:false,Native_Workbench_acceptance:false};all.push(record);await writeFile(new URL(`${name}-browser-result.json`,output),JSON.stringify(record,null,2));await page.close();child.stdin.end();}
}
try{
 await flow('shared-input','packages/commands/tests/fixtures/production/p104-A-first.json',async({page,editor,ready,read,evidence,tab,apply,check})=>{
  const timeline=page.getByRole('region',{name:'全厂时间轴'}),baseline=ready.run;
  await check('business due save appears in real same-Scenario timeline; timeline exact edit appears back in business form',async()=>{
   await editor.getByRole('textbox',{name:'交期（ms，可未知）',exact:true}).fill('180000');await apply();const key=`input:${ready.state.snapshot.scenarios[0].id}:order_due:A`;await timeline.getByRole('combobox',{name:'选择输入标记',exact:true}).selectOption(key);assert.equal(await timeline.getByLabel('输入精确毫秒',{exact:true}).inputValue(),'180000');await timeline.getByLabel('输入精确毫秒',{exact:true}).fill('190000');await timeline.getByRole('button',{name:'预览精确时点',exact:true}).click();await timeline.getByRole('button',{name:'确认应用一次',exact:true}).click();await page.waitForFunction(()=>window.__businessRead().state.snapshot.scenarios[0].orders[0].due_at_ms===190000);assert.equal(await editor.getByRole('textbox',{name:'交期（ms，可未知）',exact:true}).inputValue(),'190000');
  });
  await check('timeline priority/calendar entries edit canonical fields through existing business form, no auto-run or separate schedule',async()=>{
   await timeline.getByRole('button',{name:'编辑优先序',exact:true}).click();await editor.getByRole('textbox',{name:'优先序（数值越大越优先）',exact:true}).fill('7');await apply();assert.equal((await read()).state.snapshot.scenarios[0].production_tasks[0].priority,7);assert.equal((await read()).state.snapshot.scenarios[0].orders[0].priority,ready.state.snapshot.scenarios[0].orders[0].priority);await timeline.getByRole('button',{name:'编辑班次',exact:true}).click();await editor.getByRole('textbox',{name:'名称',exact:true}).fill('同源时间轴打开的既有日历');await apply();assert.equal((await read()).state.snapshot.project.calendars[0].title,'同源时间轴打开的既有日历');await timeline.getByRole('button',{name:'编辑阶段工时',exact:true}).click();await editor.getByRole('heading',{name:'设备阶段与声明参数',exact:true}).waitFor();assert.equal((await read()).run.id,baseline.id);assert.equal((await evidence()).runs.length,0);assert.ok((await evidence()).replies.filter(r=>r.ok).every(r=>r.old_Run_unchanged));
  });
 });
 await flow('quality','packages/commands/tests/fixtures/quality/quality-original.json',async({page,editor,ready,read,evidence,rpc,tab,apply,run,check})=>{
  const baseline=ready.run;
  await check('original frozen quality input accepts at40/100 seconds, target unmet, scrap physically retained',async()=>{
   assert.deepEqual(baseline.acceptance_events.map(e=>e.at_ms),[40000,100000]);assert.equal(baseline.goal_met,false);assert.equal(baseline.final_items.filter(i=>i.lifecycle==='scrapped').length,1);
  });
  await check('explicit scheduled release changed90 to110 seconds saves input only, rerun accepts40/120, old Run immutable',async()=>{
   await tab('声明质量决定');await editor.getByRole('combobox',{name:'业务记录'}).selectOption({label:'manual-release-3'});await editor.getByRole('textbox',{name:'决定生效（ms）',exact:true}).fill('110000');await apply();assert.equal((await read()).run.id,baseline.id);assert.equal((await evidence()).runs.length,0);const next=await run();assert.deepEqual(next.acceptance_events.map(e=>e.at_ms),[40000,120000]);assert.deepEqual(await rpc({kind:'read_run',run_id:baseline.id}),baseline);
  });
  await check('unknown quality explicitly typed PASS gives40/80 seconds, stale manual release is a real unapplied decision with unknown conclusion',async()=>{
   await tab('质量结果预设');await editor.getByRole('combobox',{name:'业务记录'}).selectOption({label:'QC-3'});const enabled=editor.getByRole('checkbox',{name:'填写按尝试顺序的预设结果第1行',exact:true});await enabled.check();assert.equal(await enabled.isChecked(),true);const type=editor.getByRole('combobox',{name:'按尝试顺序的预设结果第1行类型',exact:true});await type.selectOption({label:'文字'});await editor.getByRole('textbox',{name:'数值',exact:true}).fill('PASS');await apply();const next=await run();assert.deepEqual(next.acceptance_events.map(e=>e.at_ms),[40000,80000]);assert.equal(next.goal_met,null);assert.ok(next.limitations.some(i=>i.code==='simulation.quality_decision_unapplied'));assert.deepEqual(await rpc({kind:'read_run',run_id:baseline.id}),baseline);await tab('实际接受与等待');assert.ok((await editor.innerText()).includes('未评价'));assert.ok((await editor.innerText()).includes('not currently available'));
  });
  await check('explicitly remove superseded declaration, next run evaluates unmet target and retains physical scrap',async()=>{
   await tab('声明质量决定');await editor.getByRole('combobox',{name:'业务记录'}).selectOption({label:'manual-release-3'});await editor.getByRole('button',{name:'移除记录',exact:true}).click();await apply();const next=await run();assert.deepEqual(next.acceptance_events.map(e=>e.at_ms),[40000,80000]);assert.equal(next.goal_met,false);assert.equal(next.final_items.filter(i=>i.lifecycle==='scrapped').length,1);assert.equal(next.limitations.length,0);assert.deepEqual(await rpc({kind:'read_run',run_id:baseline.id}),baseline);
  });
  await tab('实际接受与等待');assert.ok((await editor.innerText()).includes('未达到'));
 });
 await flow('declared-phase','scenarios/真实业务设计/runtime-ui/declared-phase.input.json',async({page,editor,ready,read,evidence,rpc,tab,apply,run,check})=>{
  const baseline=ready.run;let intermediate;
  await check('actual original5/30/5 with20–40 closure completes45 seconds',async()=>{assert.equal(baseline.jobs[0].finished_at_ms,45000);});
  await check('declared exact unload binding5→10 sends canonical set_declared_parameter, same Engine completes50 seconds',async()=>{
   await page.getByRole('region',{name:'全厂时间轴'}).getByRole('button',{name:'编辑阶段工时',exact:true}).click();await editor.getByRole('textbox',{name:'数值',exact:true}).fill('10000');await apply();const e=await evidence();assert.equal(e.commits.at(-1).commands.length,1);assert.equal(e.commits.at(-1).commands[0].kind,'set_declared_parameter');assert.equal((await read()).run.id,baseline.id);intermediate=await run();assert.equal(intermediate.jobs[0].finished_at_ms,50000);assert.deepEqual(await rpc({kind:'read_run',run_id:baseline.id}),baseline);
  });
  await check('explicit worker calendar opening40→45 reruns from initial input to55 seconds, both old results stay frozen',async()=>{
   await page.getByRole('region',{name:'全厂时间轴'}).getByRole('button',{name:'编辑班次',exact:true}).click();await editor.getByRole('combobox',{name:'业务记录'}).selectOption({label:'declared non-continuous worker calendar'});await editor.getByRole('combobox',{name:'选择可用区间行',exact:true}).selectOption('1');await editor.getByRole('textbox',{name:'开始（ms，含）',exact:true}).fill('45000');await apply();assert.equal((await read()).run.id,intermediate.id);const next=await run();assert.equal(next.jobs[0].finished_at_ms,55000);assert.deepEqual(await rpc({kind:'read_run',run_id:baseline.id}),baseline);assert.deepEqual(await rpc({kind:'read_run',run_id:intermediate.id}),intermediate);
  });
  await tab('实际接受与等待');assert.ok((await editor.innerText()).includes('真实资源持有'));
  await check('concurrent declared parameter is not silently reapplied after refresh; explicit current choice removes stale command',async()=>{
   await tab('设备阶段与参数');await editor.getByRole('textbox',{name:'数值',exact:true}).fill('12000');const current=await rpc({kind:'concurrent_parameter',device_id:ready.state.snapshot.project.devices[0].id});assert.equal(current.snapshot.project.devices.find(d=>d.id===ready.state.snapshot.project.devices[0].id).configuration.unload_ms.value,9000);await editor.getByRole('button',{name:'检查变更',exact:true}).click();const preview=editor.getByRole('dialog',{name:'业务变更预览'});await preview.getByRole('button',{name:'确认应用草稿',exact:true}).click();await preview.waitFor({state:'detached'});assert.ok((await editor.innerText()).includes('RevisionConflict'));await editor.getByRole('button',{name:'读取当前工程',exact:true}).click();await editor.getByRole('button',{name:'基于当前输入重新预览',exact:true}).click();const merge=editor.getByRole('dialog',{name:'修订冲突字段核对'});assert.ok((await merge.innerText()).includes('declared_parameters'));assert.equal(await merge.getByRole('button',{name:'接受核对并重新检查',exact:true}).isDisabled(),true);await merge.getByRole('button',{name:'保留当前此字段',exact:true}).click();await merge.getByRole('button',{name:'接受核对并重新检查',exact:true}).click();await editor.getByRole('button',{name:'检查变更',exact:true}).click();assert.equal(await editor.getByRole('textbox',{name:'数值',exact:true}).inputValue(),'9000');assert.equal(await editor.getByRole('dialog',{name:'业务变更预览'}).count(),0);assert.ok((await editor.innerText()).includes('没有需要提交'));assert.equal((await read()).run.id,(await evidence()).replies.filter(r=>r.run).at(-1).run.id);
  });
 });
}finally{await browser.close();await writeFile(new URL('scenario-flows-result.json',output),JSON.stringify({flows:all,Host_or_Engine_mock:false,Native_Workbench_acceptance:false},null,2));console.log(JSON.stringify(all.map(v=>({name:v.name,checks:v.checks,errors:v.errors,warnings:v.warnings})),null,2));}
