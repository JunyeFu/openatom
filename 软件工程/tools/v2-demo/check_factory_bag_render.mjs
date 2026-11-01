// Production FactoryScene reading actual frozen Host outputs, no production IPC mock.
import {createRequire} from 'node:module';
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const [proof,output,port='8922']=process.argv.slice(2),require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const frames=JSON.parse(await fs.readFile(path.join(proof,'actual-observation-frames.json'),'utf8'));
await fs.mkdir(output,{recursive:false});
const browser=await chromium.launch({channel:'msedge',headless:true,args:['--enable-unsafe-swiftshader']});
const page=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1}),errors=[],observations=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
try{
 await page.goto(`http://127.0.0.1:${port}/factory-proof`,{waitUntil:'domcontentloaded'});
 for(const index of [0,frames.length-1,0,frames.length-1]){
  const at=frames[index].at_ms;await page.getByLabel('运行时点',{exact:true}).selectOption(String(index));
  await page.waitForFunction(at=>window.factoryProof?.observation?.at_ms===at&&document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===at,at,{timeout:90000});
  await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:90000});
  const actual=await page.evaluate(()=>({at_ms:window.factoryProof.observation.at_ms,items:window.factoryProof.items.map(i=>({id:i.item.id,type:i.item.item_type_ref.id,quality:i.item.quality,lifecycle:i.item.lifecycle,holder:i.item.holder,attachment:i.attachment,unavailable:i.unavailable})),devices:window.factoryProof.devices.map(d=>({id:d.device_id,unavailable:d.unavailable})),diagnostics:document.querySelector('canvas').readFactoryDiagnostics()}));
  assert.equal(actual.devices.length,92);assert.ok(actual.devices.every(d=>!d.unavailable));assert.deepEqual(actual.diagnostics.asset_loading.items.failures,[]);
  const bags=actual.items.filter(i=>i.type==='PF-BAG-finished-bag'&&i.lifecycle==='available');
  if(index===frames.length-1){assert.equal(bags.length,3);assert.ok(bags.every(i=>!i.unavailable&&i.holder.id==='PF-SHIPPING'));assert.equal(actual.items.filter(i=>i.holder.kind==='location'&&i.holder.id==='PF-SHIPPING').length,33);}
  assert.deepEqual(actual.diagnostics.asset_loading.items.unavailable.filter(x=>{const item=actual.items.find(i=>i.id===x.ref.id);return item?.type.startsWith('PF-BAG-')&&!['transformed','scrapped'].includes(item.lifecycle);}),[]);
  const deviceTitles=actual.diagnostics.asset_loading.devices;
  assert.ok(deviceTitles.every(d=>d.loaded));
  assert.equal(deviceTitles.length,92);assert.ok(deviceTitles.every(d=>/实际结束姿态$|实际阶段$|静态素材（无活动执行）$|静态素材$/.test(d.title)),JSON.stringify(deviceTitles.filter(d=>!/实际结束姿态$|实际阶段$|静态素材（无活动执行）$|静态素材$/.test(d.title))));
  actual.device_titles=deviceTitles;
  observations.push(actual);await fs.writeFile(path.join(output,'observations.json'),JSON.stringify(observations,null,2));
 }
 await page.getByRole('button',{name:'纸袋近景',exact:true}).click();
 await page.waitForFunction(()=>window.factoryCapture||window.factoryCaptureFailure,{},{timeout:120000});
 assert.equal(await page.evaluate(()=>window.factoryCaptureFailure),null);
 const capture=await page.evaluate(()=>window.factoryCapture);assert.equal(capture.key,'bag-close-'+frames.at(-1).at_ms);
 await page.screenshot({path:path.join(output,'finished-bags.png')});
 assert.deepEqual(errors,[]);
 await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify({source:proof,renderer:'Production FactoryScene and actual frozen Host observations',seeks:observations.length,devices:92,shipping_items:33,bags:3,capture,errors,ordinary_Native_verified:false,whole_factory_accepted:false},null,2));
 console.log('Actual bag material/override FactoryScene rendered:',output);
}catch(error){await page.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});await fs.writeFile(path.join(output,'failure.json'),JSON.stringify({error:String(error),errors,body:await page.locator('body').innerText(),diagnostics:await page.evaluate(()=>document.querySelector('canvas')?.readFactoryDiagnostics?.())},null,2));throw error;}
finally{await browser.close();}
