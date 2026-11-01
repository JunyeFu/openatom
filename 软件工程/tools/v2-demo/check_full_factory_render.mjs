// Actual Host archive rendered through the production FactoryScene. No IPC mock.
import {createRequire} from 'node:module';
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const [proof,output,port='8922']=process.argv.slice(2);
const require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const frames=JSON.parse(await fs.readFile(path.join(proof,'actual-observation-frames.json'),'utf8'));
await fs.mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true,args:['--enable-unsafe-swiftshader']});
const page=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1});
let activePage=page;
const errors=[],warnings=[],observations=[],closeups=[];
function track(target){
 target.on('pageerror',e=>errors.push(String(e)));
 target.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
 target.on('requestfailed',r=>{if(r.failure()?.errorText!=='net::ERR_ABORTED')errors.push(r.url()+': '+r.failure()?.errorText);});
}
track(page);
async function observe(index,direction){
 const started=Date.now();
 const at=frames[index].at_ms;
 await page.getByLabel('运行时点',{exact:true}).selectOption(String(index));
 await page.waitForFunction(at=>window.factoryProof?.observation?.at_ms===at,at,{timeout:60000});
 await page.waitForFunction(at=>document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===at,at,{timeout:60000});
 await page.waitForFunction(()=>![...document.querySelectorAll('[data-scene-label-key^="device:"]')].some(e=>/读取素材中/.test(e.title)),{},{timeout:60000});
 await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:60000});
 await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 const state=await page.evaluate(()=>{
  const s=window.factoryProof;
  return {at_ms:s.observation.at_ms,devices:s.devices.map(d=>({id:d.device_id,unavailable:d.unavailable})),
   transports:s.transports.map(t=>({id:t.trip.id,phase:t.trip.phase,pose:t.pose,cargo:t.trip.cargo,unavailable:t.unavailable})),
   items:s.items.map(i=>({id:i.item.id,type:i.item.item_type_ref.id,holder:i.item.holder,lifecycle:i.item.lifecycle,attributes:i.item.attributes,attachment:i.attachment,unavailable:i.unavailable})),
   diagnostics:document.querySelector('canvas').readFactoryDiagnostics()};
 });
 assert.equal(state.devices.length,92);
 assert.equal(state.devices.filter(d=>d.unavailable).length,0);
 const cartons=state.items.filter(i=>i.type==='PF-packed-case'&&!['transformed','scrapped'].includes(i.lifecycle));
 if(cartons.length){
  const c=cartons[0];assert.equal(c.unavailable,null);
  assert.ok(state.diagnostics.visible_objects.some(o=>o.kind==='item'&&o.id===c.id),'Actual carton geometry must be in the factory viewport');
 }
 {
  const routeIssues=state.diagnostics.asset_loading.items.unavailable.filter(l=>{
   const item=state.items.find(i=>i.id===l.ref.id);
   return item?.type.startsWith('PF-')&&!['transformed','scrapped'].includes(item.lifecycle);
  });
  assert.deepEqual(routeIssues,[],'Route item geometry must have actual media and holder anchors');
  assert.deepEqual(state.diagnostics.asset_loading.items.failures,[]);
 }
 const sample={direction,index,elapsed_ms:Date.now()-started,...state};observations.push(sample);
 await fs.writeFile(path.join(output,'observations.json'),JSON.stringify(observations,null,2));
 if(direction==='forward'&&[0,12].includes(index))await page.screenshot({path:path.join(output,`${String(index).padStart(2,'0')}-${at}-overview.png`)});
 console.log(direction,index,at,'items',state.items.length,'cartons',cartons.length,'incremental viewport');
 return sample;
}
try{
 await page.goto(`http://127.0.0.1:${port}/factory-proof`,{waitUntil:'domcontentloaded'});
 await page.waitForFunction(()=>window.factoryProof?.observation?.at_ms===0,{},{timeout:60000});
 await page.getByLabel('模型精度',{exact:true}).selectOption('simplified');
 for(let i=0;i<frames.length;i++)await observe(i,'forward');
 for(let i=frames.length-2;i>=0;i--)await observe(i,'reverse');
 const last=observations.find(o=>o.at_ms===1800000),shipping=last.items.filter(i=>i.holder.kind==='location'&&i.holder.id==='PF-SHIPPING');
 assert.equal(shipping.length,6);
 assert.equal(new Set(shipping.map(i=>JSON.stringify(i.attachment.slot.offset.translation))).size,6,'Six finished products occupy distinct declared display slots');
 for(const index of [2,5,7,9,12]){
  const target=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1});activePage=target;track(target);
  const back=[7,9,12].includes(index),at=frames[index].at_ms;
  await target.goto(`http://127.0.0.1:${port}/factory-proof${back?'?case-side=back':''}`,{waitUntil:'domcontentloaded'});
  await target.getByLabel('运行时点',{exact:true}).selectOption(String(index));
  await target.waitForFunction(at=>window.factoryProof?.observation?.at_ms===at,at,{timeout:60000});
  await target.getByRole('button',{name:'箱件近景',exact:true}).click({timeout:60000});
  await target.waitForFunction(()=>window.factoryCapture||window.factoryCaptureFailure,{},{timeout:120000});
  assert.equal(await target.evaluate(()=>window.factoryCaptureFailure),null);
  const capture=await target.evaluate(()=>window.factoryCapture);
  assert.equal(capture.key,`case-close-${back?'back-':''}${at}`);
  const filename=`${String(index).padStart(2,'0')}-${at}-case${back?'-back':''}.png`;
  await target.screenshot({path:path.join(output,filename),timeout:30000});
  closeups.push({index,at_ms:at,back,screenshot:filename,capture});
  await fs.writeFile(path.join(output,'closeups.json'),JSON.stringify(closeups,null,2));
  console.log('close-up',index,at,back?'back':'front');await target.close();activePage=page;
 }
 assert.equal(errors.length,0,JSON.stringify(errors));
 const receipt={source:proof,renderer:'Production FactoryScene, actual frozen Host exports; same viewport source updates for seeks; separate settled close-up pages',ordinary_Native_verified:false,Browser_plugin:'unavailable; project Playwright fallback',frames:frames.length,observations:observations.length,forward_reverse:true,closeups:closeups.length,final_shipping_items:shipping.length,errors,warnings};
 await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2));console.log('Renderer matrix passed',output);
}catch(error){
 await activePage.screenshot({path:path.join(output,'failure.png'),timeout:10000}).catch(()=>{});
 await fs.writeFile(path.join(output,'failure.json'),JSON.stringify({error:String(error),errors,warnings,body:await activePage.locator('body').innerText().catch(()=>''),captureFailure:await activePage.evaluate(()=>window.factoryCaptureFailure).catch(()=>null),diagnostics:await activePage.evaluate(()=>document.querySelector('canvas')?.readFactoryDiagnostics?.()).catch(()=>null)},null,2));
 throw error;
}finally{await browser.close();}
