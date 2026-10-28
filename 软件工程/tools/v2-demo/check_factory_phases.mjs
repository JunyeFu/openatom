// Full real-Run active-phase sampling through the production renderer.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
const [proof,output,port='8922']=process.argv.slice(2),require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const read=async name=>JSON.parse(await fs.readFile(path.join(proof,name),'utf8'));
const [plan,frames,project]=await Promise.all(['sampling-plan.json','actual-observation-frames.json','actual-frozen-project.json'].map(read));
await fs.mkdir(output,{recursive:false});
const browser=await chromium.launch({channel:'msedge',headless:true,args:['--enable-unsafe-swiftshader']});
const page=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1}),errors=[],observations=[],waits=new Map();
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
async function seek(index){
 await page.getByLabel('运行时点',{exact:true}).selectOption(String(index));
 await page.waitForFunction(at=>window.factoryProof?.observation?.at_ms===at&&document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===at,frames[index].at_ms,{timeout:90000});
 await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:90000});
}
const checks=[];
try{
 await page.goto(`http://127.0.0.1:${port}/factory-proof`,{waitUntil:'domcontentloaded'});
 await page.getByRole('checkbox',{name:'物料连线',exact:true}).waitFor();
 assert.equal(await page.getByRole('checkbox',{name:'物料连线',exact:true}).isChecked(),false);
 await seek(0);
 const lines=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics().render.lines);
 await page.getByRole('checkbox',{name:'物料连线',exact:true}).check();
 await page.waitForFunction(lines=>document.querySelector('canvas')?.readFactoryDiagnostics?.().render.lines>lines,lines);
 await page.getByRole('checkbox',{name:'物料连线',exact:true}).uncheck();
 const indexes=[...frames.keys(),frames.length-2,Math.floor(frames.length/2),1,0,frames.length-1];
 for(const index of indexes){
  await seek(index);
  const state=await page.evaluate(()=>({diagnostics:document.querySelector('canvas').readFactoryDiagnostics(),jobs:window.factoryProof.observation.objects.filter(o=>o.kind==='job').map(o=>o.data),devices:window.factoryProof.devices.map(d=>({id:d.device_id,unavailable:d.unavailable,projection:d.projection,job_id:d.frame?.job_id}))}));
  const d=state.diagnostics;assert.equal(d.asset_loading.devices.length,92);assert.ok(d.asset_loading.devices.every(x=>x.loaded));
  assert.deepEqual(state.devices.filter(x=>x.unavailable),[]);assert.deepEqual(d.asset_loading.items.failures,[]);
  const samples=plan.samples.filter(s=>s.at_ms===frames[index].at_ms);
  for(const sample of samples){
   const job=state.jobs.find(j=>j.id===sample.job_id);assert.ok(job);
   const device=state.devices.find(x=>x.id===sample.device_id),pose=d.asset_loading.devices.find(x=>x.id===sample.device_id);
   assert.equal(device.job_id,sample.job_id,JSON.stringify({sample,device,job}));
   if(sample.kind==='working'){
    assert.equal(job.state,'working');assert.equal(job.phase_ref,sample.phase_id);assert.equal(job.effective_progress_ms,sample.effective_ms);
    assert.equal(pose.phase,sample.phase_id);assert.equal(pose.effective_ms,sample.effective_ms);assert.ok(pose.duration_ms>0);
    assert.ok(Math.abs(pose.progress-sample.effective_ms/pose.duration_ms)<1e-8);
   }else{
    assert.ok(['waiting_resource','waiting_output'].includes(job.state),JSON.stringify({sample,job}));
    const key=JSON.stringify([sample.job_id,sample.reasons]),values=waits.get(key)??[];values.push({at:sample.at_ms,effective:job.effective_progress_ms,pose_effective:pose.effective_ms,pose_phase:pose.phase});waits.set(key,values);
   }
   checks.push({at_ms:sample.at_ms,device_id:sample.device_id,job_id:sample.job_id,kind:sample.kind,phase:pose.phase,effective_ms:pose.effective_ms,progress:pose.progress});
  }
  observations.push({at_ms:d.observation_ms,manufacturing_jobs:state.jobs.filter(j=>j.execution_ref.kind==='project_device').length,active_devices:state.devices.filter(x=>x.projection==='active').length,render:d.render});
  await fs.writeFile(path.join(output,'checks.json'),JSON.stringify(checks,null,2));
  if(observations.length%10===0)console.log('verified actual frames',observations.length,'/',indexes.length);
 }
 for(const values of waits.values()){assert.ok(values.length>=2);assert.equal(new Set(values.map(v=>v.effective)).size,1);assert.equal(new Set(values.map(v=>JSON.stringify([v.pose_effective,v.pose_phase]))).size,1);}
 const closeDevices=['PF-paper-bag-tuber',plan.samples.find(s=>s.operation_ref.id==='sf.paperfactory.bag.operation.attach').device_id,'PF-paper-tube-winder','PF-label-die-cutter'];
 const captures=[];
 for(const deviceId of closeDevices){
  const sample=plan.samples.find(s=>s.kind==='working'&&s.device_id===deviceId);assert.ok(sample,deviceId);
  const index=frames.findIndex(f=>f.at_ms===sample.at_ms);await seek(index);
  await page.getByLabel('近景设备',{exact:true}).selectOption(deviceId);await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();
  await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
  const capture=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());
  await page.screenshot({path:path.join(output,deviceId+'.png')});captures.push({device_id:deviceId,at_ms:sample.at_ms,capture});
  await page.getByRole('button',{name:'全厂适配',exact:true}).click();
 }
 assert.equal(new Set(checks.filter(c=>c.kind==='working').map(c=>c.device_id)).size,92);assert.deepEqual(errors,[]);
 await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify({source:proof,renderer:'Production FactoryScene; actual Host objects at every sample',actual_Run:plan.run_id,devices:92,work_samples:plan.work_samples,wait_samples:plan.wait_samples,seeks:observations.length,observations,captures,errors,ordinary_Native_verified:false,cloud_phase_verified:false,whole_factory_accepted:false},null,2));
 console.log('All92 factory devices active-phase render passed:',output);
}catch(error){await page.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});await fs.writeFile(path.join(output,'failure.json'),JSON.stringify({error:String(error),errors,last_checks:checks.slice(-5),body:await page.locator('body').innerText(),diagnostics:await page.evaluate(()=>document.querySelector('canvas')?.readFactoryDiagnostics?.())},null,2));throw error;}
finally{await browser.close();}
