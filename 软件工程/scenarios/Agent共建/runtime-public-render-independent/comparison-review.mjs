import{readFile,writeFile,readdir}from'node:fs/promises';import{fileURLToPath}from'node:url';import{createRequire}from'node:module';import assert from'node:assert/strict';
import{openCore,root}from'../../../tests/planner/node-inspector/core.mjs';
if(!process.argv.includes('--root-consumer-frozen'))throw Error('Root final consumer cut required');
const out=new URL('./',import.meta.url),beforeNames=new Set(await readdir(new URL('scenarios/媒体表现/runtime-node-inspector/',root))),checks=[],errors=[],warnings=[];let core,browser,page,processId,pid;
const request=async(operation,params)=>{const r=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'independent-comparison-'+crypto.randomUUID(),project_id:pid,scope:{kind:'project'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r));return r.outcome.data;};
try{
 core=await openCore();pid=core.ready.snapshot.project.id;
 const identity=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'independent-comparison-identity',project_id:pid,scope:{kind:'project'},operation:'get_context',params:{selection:[]}}});
 processId=/^P303-core-(\d+)$/.exec(identity.context.host_instance_id)?.[1];assert.ok(processId);assert.ok(!beforeNames.has('core-session-'+processId));
 const beforeRun=await core.rpc({kind:'snapshot'}),scenario=beforeRun.scenarios[0];
 const task=await request('run_scenario',{scenario_id:scenario.id,input_revision:beforeRun.project.revision,observation_window:scenario.window,intent:{kind:'project_scenario'}});
 const completed=(await core.rpc({kind:'await_task',scope:{kind:'project'},task_id:task.id})).outcome.data;assert.equal(completed.task.state,'completed');
 const ids=[core.ready.formal_run.id,completed.result.data.run_id];
 const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
 browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});page=await browser.newPage({viewport:{width:1560,height:1100}});page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
 await page.route('**/__normal_core',async r=>r.fulfill({status:200,contentType:'application/json',body:JSON.stringify(await core.rpc(r.request().postDataJSON()))}));
 await page.route('**/__normal_private',r=>r.fulfill({status:422,body:'Independent comparison does not grant any manufacturing/private source leaf'}));
 await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/public-render-consumer/normal.html');await page.getByRole('button',{name:'输入草稿',exact:true}).waitFor();
 const before=await core.rpc({kind:'snapshot'}),start=core.records.length;
 const menu=page.locator('nav details').filter({has:page.locator('summary',{hasText:'评审与交付'})}).first();await menu.locator('summary').click();await page.locator('nav').getByRole('button',{name:'方案比较',exact:true}).click();
 const workspace=page.getByRole('region',{name:'方案评估与交接工作区'});await workspace.waitFor();
 for(const id of ids)await workspace.locator('.delivery-run-choice').filter({hasText:id}).getByRole('checkbox').check();
 await workspace.getByRole('button',{name:'比较所选两次运行',exact:true}).click();await workspace.getByRole('region',{name:'实际方案比较结果'}).waitFor();
 const calls=core.records.slice(start).filter(r=>r.request.kind==='dispatch'),comparison=calls.find(r=>r.request.request.operation==='compare_runs');assert.ok(comparison);assert.deepEqual(comparison.request.request.params.run_ids,ids);assert.equal(comparison.reply.outcome.status,'ok');assert.equal(await workspace.locator('.comparison-result table').count(),2);
 for(const id of ids)assert.ok((await workspace.locator('.comparison-result').innerText()).includes(id));
 assert.deepEqual(await core.rpc({kind:'snapshot'}),before);assert.ok(!calls.some(r=>['apply_commands','run_scenario','compute_packaging','adopt_artifact'].includes(r.request.request.operation)));assert.deepEqual(errors,[]);assert.deepEqual(warnings,[]);
 checks.push({name:'Actual Root normal comparison uses actual producer and same extracted result without mutation',status:'passed',run_ids:ids,producer:comparison.reply.outcome.data});console.log('PASS actual normal comparison');
}catch(e){checks.push({name:'Actual normal comparison',status:'failed',error:String(e),stack:e.stack});process.exitCode=1;console.log('FAIL',String(e));}
finally{if(page)await page.screenshot({path:fileURLToPath(new URL('normal-comparison.png',out)),fullPage:true});await browser?.close();const exit=await core?.close();await writeFile(new URL('comparison-review.json',out),JSON.stringify({checks,errors,warnings,processId,core_exit:exit,records:core?.records,passed:checks.length===1&&checks[0].status==='passed'&&exit===0,evidence:'Independent normal Workbench over existing actual Core/Engine/compare_runs. Controlled browser dispatch transport, no Native app.',Native:false},null,2));}
