// Records only the explicitly launched, owned Super Factory Native instance.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn, execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const exec=promisify(execFile),[root,mode='snapshot']=process.argv.slice(2);
const read=async file=>JSON.parse((await fs.readFile(file,'utf8')).replace(/^\uFEFF/,''));
const write=(name,value)=>fs.writeFile(path.join(root,name),JSON.stringify(value,null,2)+'\n');
const launch=await read(path.join(root,mode.startsWith('reopen')?'native-02/launch.json':'native-01/launch.json'));
if(!launch.owned_browser||launch.monitor.primary)throw Error('Expected owned Native on non-primary display.');
const browser=await chromium.connectOverCDP(launch.cdp);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');
if(!page)throw Error('Owned product page not found.');
page.setDefaultTimeout(30000);
const pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const ffmpeg='D:/MathModelingTools/envs/cumcm/Lib/site-packages/imageio_ffmpeg/binaries/ffmpeg-win-x86_64-v7.1.exe';
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function rect(){
 const r=await exec(pwsh,['-NoProfile','-File',path.join(root,'helpers/owned-rectangle.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});
 const v=JSON.parse(r.stdout);if(v.primary||v.width<100||v.height<100)throw Error('Capture geometry invalid.');return v;
}
async function capture(name,seconds,action){
 const area=await rect(),file=path.join(root,name+'.mp4'),log=path.join(root,name+'.ffmpeg.log');
 const logHandle=await fs.open(log,'wx');
 const args=['-hide_banner','-n','-f','gdigrab','-draw_mouse','1','-framerate','30','-offset_x',String(area.x),'-offset_y',String(area.y),'-video_size',`${area.width}x${area.height}`,'-i','desktop','-an','-vf','pad=ceil(iw/2)*2:ceil(ih/2)*2','-c:v','libx264','-preset','veryfast','-crf','18','-pix_fmt','yuv420p','-movflags','+faststart',file];
 const recorder=spawn(ffmpeg,args,{windowsHide:true,stdio:['pipe','ignore',logHandle.fd]});
 const ended=new Promise((resolve,reject)=>{recorder.once('error',reject);recorder.once('exit',(code)=>code===0?resolve():reject(Error(`FFmpeg ${name} failed ${code}; see ${log}`)));});
 const start=new Date().toISOString(),startedAt=Date.now();
 let actionError=null;
 try{await pause(900);try{await action();}catch(error){actionError=error;}await pause(Math.max(2000,seconds*1000-(Date.now()-startedAt)));recorder.stdin.write('q\n');await ended;}finally{await logHandle.close();}
 const record={file,candidate:launch.exe,pid:launch.pid,source_resolution:[area.width,area.height],fps:30,requested_seconds:seconds,started_at_utc:start,capture:'actual_owned_Native_desktop_region',monitor:area.selected_monitor,primary:false,actions_complete:!actionError,action_error:actionError?String(actionError):null,scope:'stable1.6_working_demonstration_not_whole_factory_acceptance'};
 await write(name+'.json',record);console.log(JSON.stringify(record));
 if(actionError)throw actionError;
}
const api=async(operation,params={},projectId=null)=>{
 const reply=await page.evaluate(({operation,params,projectId})=>window.__TAURI_INTERNALS__.invoke('project_api',{request:{api_version:'0.1',request_id:'video-'+crypto.randomUUID(),project_id:projectId,scope:{kind:projectId?'project':'workspace'},operation,params}}),{operation,params,projectId});
 if(reply.outcome.status!=='ok')throw Error(JSON.stringify(reply.outcome));return reply.outcome.data;
};
async function sceneReady(){await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d&&d.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});}
async function seek(at){await page.getByLabel('观察毫秒',{exact:true}).fill(String(at));await page.getByRole('button',{name:'读取时点',exact:true}).click();await page.waitForFunction(t=>document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===t,at,{timeout:120000});await sceneReady();}
async function selectDirectory(folder){
 await fs.mkdir(folder,{recursive:true});let dialog;
 const until=Date.now()+15000;
 do{
  const r=await exec(pwsh,['-NoProfile','-File',path.join(root,'helpers/owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});
  if(r.stdout.trim()){
   const value=JSON.parse(r.stdout);
   if(value.controls.some(c=>c.class==='Edit'&&c.id===1152)&&value.controls.some(c=>c.class==='Button'&&c.id===1)){dialog=value;break;}
  }
  await pause(100);
 }while(Date.now()<until);
 if(!dialog)throw Error('Owned folder picker not observed.');
 const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);
 await exec(pwsh,['-NoProfile','-File',path.join(root,'helpers/owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',folder,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
}
try{
 if(mode==='snapshot'){
  console.log((await page.locator('body').innerText()).slice(0,15000));
 }else if(mode==='open-existing'){
  const input=await read(path.join(root,'additional-input.json')),original=await read(input.source_run);
  await page.getByText('文件',{exact:true}).click();await page.getByRole('button',{name:'打开工程目录…',exact:true}).click();
  await selectDirectory(input.source_project);await page.getByRole('button',{name:'确认打开',exact:true}).click();
  await page.waitForFunction(()=>!document.querySelector('[role="dialog"]')&&document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
  const context=await api('get_context',{selection:[]});
  const restored=await api('get_run',{run_id:original.id,query:{kind:'full_result'}},context.project.id);
  if(JSON.stringify(restored.data)!==JSON.stringify(original))throw Error('Existing demo Run changed.');
  const object=await api('get_object',{object_id:{kind:'project',id:context.project.id},source:{kind:'draft'},node_index:null},context.project.id);
  await page.getByLabel('历史结果',{exact:true}).selectOption(original.id);
  await page.waitForFunction(id=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true'&&document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,original.id,{timeout:120000});await sceneReady();
  await write('recorded-context.json',context);await write('recorded-run.json',restored.data);await write('recorded-project.json',object.data);
  console.log('Existing saved demo and exact Run ready for supplemental recording.');
 }else if(mode==='setup-smoke'){
  await capture('00-capture-check',12,async()=>{
   await page.locator('#start-template').click();await pause(1200);
   await page.getByLabel('工程名称',{exact:true}).fill('超级工厂演示 · 纸包装工厂');await pause(1200);
   await page.getByRole('button',{name:'创建',exact:true}).click();await pause(1200);
   await page.getByRole('button',{name:/纸包装多产品工厂 · 92台设备/}).click();
   await page.getByRole('heading',{name:'配置模板工厂',exact:true}).waitFor();
   await page.getByRole('button',{name:'使用模板演示配置',exact:true}).click();
   await page.getByLabel('工厂前缀',{exact:true}).fill('演示工厂');
   await page.getByLabel('方案名称',{exact:true}).fill('纸包装演示生产');
  });
  await page.screenshot({path:path.join(root,'capture-check-end.png')});
 }else if(mode==='adopt-run'){
  await capture('01-template-and-production',55,async()=>{
   await pause(1400);await page.getByRole('button',{name:'预览创建对象',exact:true}).click();
   await page.getByRole('heading',{name:'创建预览',exact:true}).waitFor();await pause(2500);
   await page.getByRole('button',{name:'创建模板工厂',exact:true}).click();
   await page.locator('.module-library').waitFor({state:'hidden',timeout:120000});
   await sceneReady();await pause(4000);
   await page.getByLabel('当前方案',{exact:true}).selectOption('纸包装演示生产');
   await page.getByRole('button',{name:'运行方案',exact:true}).click();
   await page.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
   await sceneReady();await pause(2500);
   await page.getByRole('button',{name:'工厂全景模式',exact:true}).click();await pause(4500);
   await seek(43422);await pause(5000);await seek(100000);await pause(5000);
   await page.getByRole('button',{name:'返回编辑布局',exact:true}).click();
  });
  const context=await api('get_context',{selection:[]});const pid=context.project.id;
  const r=await api('get_run',{run_id:context.run_summaries.at(-1).run_id,query:{kind:'full_result'}},pid);
  const p=await api('get_object',{object_id:{kind:'project',id:pid},source:{kind:'draft'},node_index:null},pid);
  await write('recorded-context.json',context);await write('recorded-run.json',r.data);await write('recorded-project.json',p.data);
  await page.screenshot({path:path.join(root,'production-end.png')});
 }else if(mode==='overview'){
  await capture('02-factory-and-timeline',35,async()=>{
   await page.getByRole('button',{name:'全厂适配',exact:true}).click();await seek(100000);await pause(5000);
   await page.getByRole('button',{name:'工厂全景模式',exact:true}).click();await pause(6500);
   await seek(7200000);await pause(7000);
   await page.getByRole('button',{name:'返回编辑布局',exact:true}).click();await pause(4500);
  });
 }else if(mode==='device'){
  const p=await read(path.join(root,'recorded-project.json')),r=await read(path.join(root,'recorded-run.json'));
  const device=p.devices.find(d=>d.id.endsWith(':PF-paper-bag-tuber'));
  const job=r.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===device.id);
  const work=r.phase_work_intervals.filter(w=>w.job_id===job.id).sort((a,b)=>(b.interval.end_ms-b.interval.start_ms)-(a.interval.end_ms-a.interval.start_ms))[0];
  const at=work.interval.start_ms+1;
  await seek(at);
  await page.locator('.object-tree').getByRole('button',{name:'▣ '+device.title,exact:true}).click();
  await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();await sceneReady();
  await capture('03-device-production',25,async()=>{
   await pause(1800);await page.getByLabel('回看速度',{exact:true}).selectOption('1');
   const before=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());
   await page.getByRole('button',{name:'播放回看',exact:true}).click();await pause(13000);
   const after=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());
   await write('actual-continuous-playback.json',{device_id:device.id,phase_interval:work.interval,before_ms:before.observation_ms,after_ms:after.observation_ms,before_frame:before.render?.frame,after_frame:after.render?.frame,source_run:r.id,synthetic_animation:false});
   const stop=page.getByRole('button',{name:/^暂停/});
   if(await stop.count()===1)await stop.click();
   await pause(2500);await page.getByRole('button',{name:'全厂适配',exact:true}).click();await pause(2500);
  });
 }else if(mode==='professional'){
  await capture('06-professional-calculation',35,async()=>{
   const example=page.locator('.packaging-workspace').getByRole('combobox').filter({has:page.getByRole('option',{name:'受限折叠容器',exact:true})});
   await example.selectOption({label:'受限折叠容器'});await pause(2500);
   await page.getByRole('button',{name:'保存专业输入',exact:true}).click();
   await page.waitForFunction(()=>{const b=[...document.querySelectorAll('.packaging-workspace button')].find(e=>e.textContent==='计算已保存输入');return b&&!b.disabled;},{},{timeout:120000});
   await pause(1800);await page.getByRole('button',{name:'计算已保存输入',exact:true}).click();
   await page.waitForFunction(()=>/完成|失败|取消/.test(document.querySelector('.packaging-task strong')?.textContent??''),{},{timeout:120000});
   await page.locator('.packaging-results').scrollIntoViewIfNeeded();await pause(6500);
  });
  await write('actual-professional-ui.json',{text:await page.locator('.packaging-workspace').innerText(),context:await api('get_context',{selection:[]})});
  await page.screenshot({path:path.join(root,'professional-result.png')});
 }else if(mode==='professional-poses'){
  const sample=page.locator('.packaging-results').getByRole('combobox').filter({has:page.getByRole('option',{name:'BOX_FORM_FRONT · u=0.5',exact:true})});await sample.scrollIntoViewIfNeeded();
  await capture('06b-professional-poses',18,async()=>{
   for(const label of ['BOX_FORM_FRONT · u=0','BOX_FORM_FRONT · u=0.5','BOX_FORM_FRONT · u=1','BOX_FORM_BACK · u=1']){
    await sample.selectOption({label});await pause(2800);
   }
  });
  await page.screenshot({path:path.join(root,'professional-poses.png')});
 }else if(mode==='report'||mode==='report-resume'){
  const context=await api('get_context',{selection:[]});
  if(mode==='report-resume'){
   const id=(await read(path.join(root,'recorded-run.json'))).id;
   await page.getByLabel('历史结果',{exact:true}).selectOption(id);
   await page.waitForFunction(id=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true'&&document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,id,{timeout:120000});await sceneReady();
  }
  await capture(mode==='report'?'07-report-export':'07b-report-export',35,async()=>{
   if(await page.locator('.packaging-workspace').count())await page.locator('.packaging-workspace').getByRole('button',{name:'关闭',exact:true}).click();
   await page.getByRole('button',{name:'导出评估报告',exact:true}).click();
   await page.getByRole('heading',{name:'导出评估报告',exact:true}).waitFor();
   const options=page.locator('.delivery-run-choice');
   if(await options.count()!==1)throw Error('Expected only the recorded local Run in this demo.');
   await options.getByRole('checkbox').check();await pause(2200);
   await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();
   await selectDirectory(path.join(root,'report-export'));
   await page.getByRole('heading',{name:'上次任务返回的实际文件',exact:true}).waitFor({timeout:120000});await pause(4500);
  });
  await write('actual-report-ui.json',{text:await page.locator('.delivery-workspace').innerText(),project_id:context.project.id,run_id:context.run_summaries[0].run_id});
  await page.screenshot({path:path.join(root,'report-result.png')});
  await page.locator('.delivery-workspace > header').getByRole('button',{name:/返回|关闭/}).click();
 }else if(mode==='save'){
  await capture('04-save-project',28,async()=>{
   await pause(1500);await page.getByText('文件',{exact:true}).click();await pause(1000);
   await page.getByRole('button',{name:'保存工程…',exact:true}).click();
   await selectDirectory(path.join(root,'demo-project'));
   await page.getByRole('status').filter({hasText:'已保存'}).waitFor({timeout:120000});await pause(3500);
  });
  const c=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));
  if(c.unsaved||c.active_tasks.length||c.execution_blockers.length)throw Error('Saved demo is not ready for normal close.');
  await write('native-01-close-state.json',c);
  await exec(pwsh,['-NoProfile','-File',path.join(root,'helpers/close-release.ps1'),'-NativeDirectory',path.join(root,'native-01')],{windowsHide:true});
 }else if(mode==='reopen'){
  const original=await read(path.join(root,'recorded-run.json'));
  await capture('05-new-process-reopen',32,async()=>{
   await pause(1200);await page.getByText('文件',{exact:true}).click();
   await page.getByRole('button',{name:'打开工程目录…',exact:true}).click();
   await selectDirectory(path.join(root,'demo-project'));
   await page.getByRole('button',{name:'确认打开',exact:true}).click();
   await page.waitForFunction(()=>!document.querySelector('[role="dialog"]')&&document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
   await page.getByLabel('历史结果',{exact:true}).selectOption(original.id);
   await page.waitForFunction(id=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true'&&document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,original.id,{timeout:120000});
   await sceneReady();
   await seek(84730);await pause(4500);await seek(7200000);await pause(4000);
  });
  const context=await api('get_context',{selection:[]}),restored=await api('get_run',{run_id:original.id,query:{kind:'full_result'}},context.project.id);
  if(JSON.stringify(restored.data)!==JSON.stringify(original))throw Error('Reopened Run differs from actual recorded Run.');
  await write('actual-demo-reopened.json',{new_pid:launch.pid,run_id:original.id,run_exact:true,context});
  const close=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));
  if(close.unsaved||close.active_tasks.length||close.execution_blockers.length)throw Error('Reopened demo is not ready for normal close.');
  await write('native-02-close-state.json',close);
  await exec(pwsh,['-NoProfile','-File',path.join(root,'helpers/close-release.ps1'),'-NativeDirectory',path.join(root,'native-02')],{windowsHide:true});
 }else if(mode==='reopen-recover'){
  const original=await read(path.join(root,'recorded-run.json'));
  await page.waitForFunction(id=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true'&&document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,original.id,{timeout:120000});
  await sceneReady();
  await capture('05b-restored-run',25,async()=>{
   await seek(84730);await pause(6500);await seek(7200000);await pause(6500);
  });
  const context=await api('get_context',{selection:[]}),restored=await api('get_run',{run_id:original.id,query:{kind:'full_result'}},context.project.id);
  if(JSON.stringify(restored.data)!==JSON.stringify(original))throw Error('Reopened Run differs from actual recorded Run.');
  await write('actual-demo-reopened.json',{new_pid:launch.pid,previous_pid:(await read(path.join(root,'native-01/launch.json'))).pid,run_id:original.id,run_exact:true,context,initial_recorder_sequencing_failure_preserved:'failure-reopen.json',continued_same_process:true});
  const close=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));
  if(close.unsaved||close.active_tasks.length||close.execution_blockers.length)throw Error('Reopened demo is not ready for normal close.');
  await write('native-02-close-state.json',close);
  await exec(pwsh,['-NoProfile','-File',path.join(root,'helpers/close-release.ps1'),'-NativeDirectory',path.join(root,'native-02')],{windowsHide:true});
 }else throw Error('Unknown recording mode '+mode);
 process.exit(0); // Detach this helper without closing the owned Native or its drafts.
}catch(error){
 await write('failure-'+mode+'.json',{error:String(error),stack:error.stack});
 await page.screenshot({path:path.join(root,'failure-'+mode+'.png')}).catch(()=>{});
 console.error(error);process.exit(1);
}
