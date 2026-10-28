// Real ordinary Native main IPC. No replacement Host or mocked runtime.
import {createRequire} from 'node:module';import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';
const [playwrightPath,pass='ordinary-01',mode='inspect',attempt='attempt-05',planPath]=process.argv.slice(2),root=process.cwd(),proof=path.join(root,'.work/native500-quality-20261025');
const read=async p=>JSON.parse((await fs.readFile(p,'utf8')).replace(/^\uFEFF/,''));
const launch=await read(path.join(proof,pass+'-launch.json'));
const {chromium}=createRequire(import.meta.url)(playwrightPath),browser=await chromium.connectOverCDP(launch.cdp);
let page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');
// WebView2 can advertise the CDP target before Playwright receives its page.
if(!page){const context=browser.contexts()[0];page=context.pages()[0]??await context.waitForEvent('page');await page.waitForURL('http://tauri.localhost/');}
const identity=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'));assert.equal(identity.host_instance_id,'desktop-host-'+launch.pid);
if(mode.startsWith('ui-open')){
 await page.getByText('文件',{exact:true}).click();
 await page.getByRole('button',{name:'打开工程目录…',exact:true}).click();
 await page.getByRole('button',{name:'确认打开',exact:true}).click();
 const expectedProject=(await read(path.join(proof,attempt,'host-receipt.json'))).project;
 await page.getByText(expectedProject.title,{exact:true}).first().waitFor();
}
const api=async(operation,params,project_id='P1')=>{const response=await page.evaluate(({operation,params,project_id})=>window.__TAURI_INTERNALS__.invoke('project_api',{request:{api_version:'0.1',request_id:'paper-native-'+crypto.randomUUID(),project_id,scope:{kind:project_id?'project':'workspace'},operation,params}}),{operation,params,project_id});assert.equal(response.outcome.status,'ok',JSON.stringify(response));return response.outcome.data;};
if(mode==='library'){
 const library=await api('query_module_library',{category_filters:[],industry:null,product_family:null,site_envelope:null,source_scope:['bundled']},null);
 const registry=await read(path.join(root,'modules/media/native-equipment-500/accepted-registration.json'));
 const candidates=registry.register_order.map(e=>{const c=library.candidates.find(c=>c.module_ref.id===e.id&&c.module_ref.version===e.version);assert.ok(c,e.id);return c;});
 const progress=await read(path.join(root,'modules/media/native-equipment-500/quality-review-progress.json')),withdrawals=[...progress.withdrawn_rolling_directions,...progress.withdrawn_lift_joints,...progress.withdrawn_hydraulic_strokes],withdrawnAbsent=[];
 for(const w of withdrawals){const spec=(await read(path.join(root,'modules/media/native-equipment-500/reviews',w.batch,'catalog.json'))).equipment.find(e=>e.id===w.id);for(const id of [spec.definition_id,spec.media_id,spec.media_id+'.simple']){assert.ok(!library.candidates.some(c=>c.module_ref.id===id&&c.module_ref.version===w.version),'withdrawn version still registered: '+id);withdrawnAbsent.push({id,version:w.version});}}
 await fs.writeFile(path.join(proof,'public-registration-checks.json'),JSON.stringify({passed:true,identity,ordinary_Native_startup:true,device_types:registry.device_count,exact_registered_modules:candidates.length,candidates,withdrawn_exact_versions_absent:withdrawnAbsent},null,2));
 const expected=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));assert.equal(expected.project_id,null);assert.equal(expected.unsaved,false);
 try{await page.evaluate(expected=>window.__TAURI_INTERNALS__.invoke('project_finish_close',{expected,discardConfirmed:false}),expected);}catch(e){if(!/closed|destroy|Target/i.test(String(e)))throw e;}
 await browser.close();console.log(`Ordinary Native startup: ${registry.device_count} accepted definitions and ${registry.media_count} exact media versions registered.`);process.exit(0);
}
let close=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));
if(mode==='close'){
 assert.equal(close.project_id,'P1');assert.equal(close.unsaved,false);assert.equal(close.active_tasks.length,0);
 await fs.writeFile(path.join(proof,pass+'-close.json'),JSON.stringify({identity,close,normal_close:true,discard_confirmed:false},null,2));
 try{await page.evaluate(expected=>window.__TAURI_INTERNALS__.invoke('project_finish_close',{expected,discardConfirmed:false}),close);}catch(e){if(!/closed|destroy|Target/i.test(String(e)))throw e;}
}else{
 const project=(await api('get_object',{object_id:{kind:'project',id:'P1'},source:{kind:'draft'},node_index:null})).data;
 const expected=await read(path.join(proof,attempt,'PAPER-QA.run.json'));
 const full=await api('get_run',{run_id:expected.id,query:{kind:'full_result'}});assert.deepEqual(full.data,expected);
 const checks=await read(path.join(proof,attempt,'behavior-checks.json')),samples=[],itemsAt=new Map();
 const selected=planPath?new Set((await read(path.resolve(root,planPath))).cases.map(c=>c.name.split(' / ')[0])):null;
 for(const row of checks.checks){for(const [condition,value] of Object.entries(row.conditions)){
  if(selected&&!selected.has(row.id+'--'+condition))continue;
  const times=value.wait?[value.wait[1]+1,value.wait[2]-1,value.finished_ms]:[1,value.finished_ms];
  for(const at_ms of times){
   const job=await api('get_run',{run_id:expected.id,query:{kind:'objects_at',data:{at_ms,objects:[{kind:'job',id:value.job_id}],page:{after_index:null,limit:1000}}}});
   // This process observes one immutable Run; several devices share the same time.
   if(!itemsAt.has(at_ms))itemsAt.set(at_ms,await api('get_run',{run_id:expected.id,query:{kind:'objects_at',data:{at_ms,objects:[],page:{after_index:null,limit:1000}}}}));
   const items=itemsAt.get(at_ms);
   samples.push({device:row.id,condition,at_ms,job:job.data.objects,items:items.data.objects});
  }
 }}
 assert.deepEqual((await api('get_run',{run_id:expected.id,query:{kind:'full_result'}})).data,expected);
 if(mode==='ui-open-save'||mode==='ui-save'){
  await page.getByText('文件',{exact:true}).click();await page.getByRole('button',{name:'保存工程…',exact:true}).click();
  let save;while(!save){const context=await api('get_context',{selection:[]});save=context.tasks.find(t=>t.kind==='exporting');if(!save)await new Promise(r=>setTimeout(r,20));}
  let state;do{state=await api('get_task',{task_id:save.id});if(state.task.state!=='completed')await new Promise(r=>setTimeout(r,20));}while(!['completed','failed','cancelled'].includes(state.task.state));assert.equal(state.task.state,'completed');
  assert.equal(state.originating_operation,'save_project');
  close=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));assert.equal(close.unsaved,false);
 }
 const taskContext=await api('get_context',{selection:[]});
 await fs.writeFile(path.join(proof,pass+'-proof.json'),JSON.stringify({passed:true,identity,attempt,mode:'ordinary_Native_main_UI_open',project_revision:project.revision,exact_Run_after_UI_open:true,close_state:close,project,samples,tasks:taskContext.tasks,viewing_preserves_Run:true},null,2));
 await page.screenshot({path:path.join(proof,pass+'-main.png')});
 console.log(JSON.stringify({passed:true,identity,samples:samples.length,exact_Run:true,revision:project.revision}));
}
await browser.close();
