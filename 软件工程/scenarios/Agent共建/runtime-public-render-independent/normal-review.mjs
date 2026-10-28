// Independent actual Core + normal Root Workbench consumer; no Native launch.
import{readFile,writeFile,readdir,access}from'node:fs/promises';
import{fileURLToPath}from'node:url';import{createRequire}from'node:module';import assert from'node:assert/strict';
import{openCore,root}from'../../../tests/planner/node-inspector/core.mjs';
if(!process.argv.includes('--root-consumer-frozen'))throw Error('Root frozen-consumer notice required');
const out=new URL('./',import.meta.url),coreOut=new URL('scenarios/媒体表现/runtime-node-inspector/',root);
const existing=new Set(await readdir(coreOut));
const checks=[],errors=[],warnings=[],privateCalls=[];let core,browser,page,pid,processId,evidence;
const scope={kind:'project'},files=new Map();
const request=async(operation,params)=>{const reply=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'independent-normal-'+crypto.randomUUID(),project_id:pid,scope,operation,params}});assert.equal(reply.outcome.status,'ok',JSON.stringify(reply));return reply.outcome.data;};
const snapshot=()=>core.rpc({kind:'snapshot'});
const edit=async commands=>request('apply_commands',{base_revision:(await snapshot()).project.revision,commands});
const menu=async name=>{const d=page.locator('nav details').filter({has:page.locator('summary',{hasText:name})}).first();if(!await d.evaluate(n=>n.open))await d.locator('summary').click();};
const open=async()=>{await menu('专业工程');await page.getByRole('button',{name:'纸包装 · 结构与过程…',exact:true}).click();const select=page.locator('.packaging-workspace label').filter({hasText:/^当前 Scope 保存的 Artifact/}).locator('select');await select.waitFor();await select.selectOption(evidence.artifact.id);};
const close=async()=>{await page.locator('.packaging-workspace').getByRole('button',{name:'关闭',exact:true}).click();const dialog=page.getByRole('dialog',{name:'未保存的专业输入'});if(await dialog.count())await dialog.getByRole('button',{name:'明确放弃本地输入并关闭',exact:true}).click();await page.locator('.packaging-workspace').waitFor({state:'detached'});};
const load=async()=>{await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/public-render-consumer/normal.html');await page.getByRole('button',{name:'输入草稿',exact:true}).waitFor();};
async function check(name,fn){const row={name};checks.push(row);try{row.actual=await fn();row.status='passed';console.log('PASS',name);}catch(e){row.status='failed';row.error=String(e);row.body=await page.locator('body').innerText();process.exitCode=1;throw e;}finally{await page.screenshot({path:fileURLToPath(new URL('normal-'+checks.length+'.png',out)),fullPage:true});}}
try{
 core=await openCore();
 pid=core.ready.snapshot.project.id;
 const identity=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'independent-normal-identity',project_id:pid,scope,operation:'get_context',params:{selection:[]}}});
 assert.equal(identity.outcome.status,'ok');processId=/^P303-core-(\d+)$/.exec(identity.context.host_instance_id)?.[1];assert.ok(processId,'Actual response envelope must expose the genuine PID-specific Host identity');
 for(const name of ['core-session-','saved-','moved-'])assert.ok(!existing.has(name+processId),'PID fixture existed before this independent process: '+name+processId);
 const project=(await snapshot()).project;
 const job=JSON.parse(await readFile(new URL('modules/professional/paper-packaging-ui/sliding-sleeve-explicit-candidate.job.json',root),'utf8')).data;
 job.subject_ref={id:project.products[0].id,revision:project.products[0].revision};if(job.conditions.product_envelope)job.conditions.product_envelope.subject=job.subject_ref;
 await edit([{kind:'put_packaging_job',data:job}]);
 const admitted=await request('compute_packaging',{packaging_job:job,input_revision:(await snapshot()).project.revision,computation_budget:job.budget});
 const complete=(await core.rpc({kind:'await_task',scope,task_id:admitted.id})).outcome.data;assert.equal(complete.task.state,'completed');
 const result=complete.result.data,artifact=(await request('get_object',{object_id:{kind:'artifact',id:result.artifact_id},source:{kind:'draft'},node_index:null})).data;evidence={artifact,result,input:job};
 await assert.rejects(access(new URL(`saved-${processId}/`,coreOut)),{code:'ENOENT'});
 const selected=await core.rpc({kind:'select_save'});assert.ok(selected.selection_id.startsWith(`P303-core-${processId}:`));
 const saved=await request('save_project',{revision:(await snapshot()).project.revision,selected_directory_ref:selected});assert.equal((await core.rpc({kind:'await_task',scope,task_id:saved.id})).outcome.data.task.state,'completed');
 for(const file of result.manufacturing_files)files.set(file.file.path,await readFile(new URL(`saved-${processId}/`+file.file.path,coreOut)));assert.ok(files.size);
 const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
 browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
 page=await browser.newPage({viewport:{width:1560,height:1100}});page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
 await page.route('**/__normal_core',async r=>r.fulfill({status:200,contentType:'application/json',body:JSON.stringify(await core.rpc(r.request().postDataJSON()))}));
 await page.route('**/__normal_private',async r=>{const q=r.request().postDataJSON();privateCalls.push(q);
  if(q.command==='project_list_stored_packaging')return r.fulfill({status:200,contentType:'application/json',body:JSON.stringify((await snapshot()).project.artifact_refs)});
  if(q.command==='project_read_stored_packaging')return r.fulfill({status:200,contentType:'application/json',body:JSON.stringify(evidence)});
  if(q.command==='project_read_stored_manufacturing_file'){assert.equal(q.request.artifact_id,artifact.id);assert.ok(files.has(q.request.file.path));return r.fulfill({status:200,contentType:'application/octet-stream',body:files.get(q.request.file.path)});}
  return r.fulfill({status:422,body:'No controlled leaf exists: '+q.command});
 });
 await load();
 await check('Normal Root Workbench menu renders every real saved SVG with no UI business mutation',async()=>{const before=await snapshot(),at=core.records.length;await open();await page.locator('.manufacturing-preview img').first().waitFor();await page.waitForFunction(()=>[...document.querySelectorAll('.manufacturing-preview img')].every(n=>n.complete&&n.naturalWidth>0));assert.equal(await page.locator('.manufacturing-preview img').count(),files.size);assert.deepEqual(await snapshot(),before);assert.ok(!core.records.slice(at).some(r=>r.request.kind==='dispatch'&&['apply_commands','compute_packaging','export_package'].includes(r.request.request.operation)));return{artifact:artifact.id,files:[...files.keys()],actual_Core:true,Native_file_leaf:'controlled over exact real saved SVGs'};});
 await close();
 await check('Normal manufacture rejects old held bytes after actual Host R changes while props remain old',async()=>{await page.evaluate(()=>window.__normalRender.hold());await open();await page.waitForFunction(()=>window.__normalRender.read().held>0);const old=(await snapshot()).project,location=structuredClone(old.locations[0]);location.title+=' independent actual ScopeR race';await edit([{kind:'put_object',data:{kind:'location',data:location}}]);await page.evaluate(()=>window.__normalRender.release());await page.locator('.manufacturing-preview [role="alert"]').waitFor();assert.equal(await page.locator('.manufacturing-preview img').count(),0);const message=await page.locator('.manufacturing-preview [role="alert"]').innerText();assert.match(message,/修订|Scope|变化/);return{old_revision:old.revision,new_revision:(await snapshot()).project.revision,message};});
 await close();await load();
 await check('Closing normal packaging during held bytes never attaches SVG after returning to Workbench',async()=>{await page.evaluate(()=>window.__normalRender.hold());await open();await page.waitForFunction(()=>window.__normalRender.read().held>0);await close();await page.evaluate(()=>window.__normalRender.release());await page.locator('.factory-scene canvas').waitFor();await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));assert.equal(await page.locator('.manufacturing-preview').count(),0);return{closed:true,stale_images:0};});
 assert.deepEqual(errors,[]);assert.deepEqual(warnings,[]);
}catch(e){if(!checks.some(r=>r.status==='failed'))checks.push({name:'Independent setup',status:'failed',error:String(e),stack:e.stack});process.exitCode=1;console.log('FAIL',String(e));}
finally{await browser?.close();const coreExit=await core?.close();await writeFile(new URL('normal-review.json',out),JSON.stringify({evidence:'Independent normal Root product browser over existing real isolated Core, actual compute/save and real original SVG bytes. Private Native binary invoke is controlled, not a Native app.',checks,errors,warnings,privateCalls,processId,allowed_new_PID_fixture_names:['core-session-'+processId,'saved-'+processId],core_exit:coreExit,records:core?.records,passed:checks.length===3&&checks.every(r=>r.status==='passed')&&errors.length===0&&warnings.length===0&&coreExit===0,Native:false},null,2));}
