// Exercise the production view controls on actual frozen Host observations.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
const [proof,output,port='8922']=process.argv.slice(2),require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const frames=JSON.parse(await fs.readFile(path.join(proof,'actual-observation-frames.json'),'utf8'));
const itemsAt=i=>frames[i].objects.filter(o=>o.kind==='item').map(o=>o.data);
const item=itemsAt(4).find(i=>i.item_type_ref.id==='PF-packed-case'&&i.holder.kind==='transport');assert.ok(item);
await fs.mkdir(output,{recursive:false});
const browser=await chromium.launch({channel:'msedge',headless:true,args:['--enable-unsafe-swiftshader']});
const page=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1}),errors=[],observations=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const diagnostics=()=>page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());
async function settled(index){
 const at=frames[index].at_ms;
 await page.waitForFunction(at=>window.factoryProof?.observation?.at_ms===at&&document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===at,at,{timeout:90000});
 await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:90000});
}
async function seek(index){await page.getByLabel('运行时点',{exact:true}).selectOption(String(index));await settled(index);}
try{
 const url=`http://127.0.0.1:${port}/factory-proof`;
 await page.goto(url,{waitUntil:'domcontentloaded'});await settled(0);
 assert.equal(await page.title(),'超级工厂 · 实际整厂渲染检查');assert.ok((await page.locator('body').innerText()).includes('纸包装整厂'));
 assert.equal(await page.locator('vite-error-overlay').count(),0);
 let d=await diagnostics();assert.equal(d.asset_loading.devices.length,92);assert.ok(d.asset_loading.devices.every(i=>i.loaded));
 const zones=await page.locator('[data-scene-label-key^="region:"]').allTextContents();assert.equal(zones.length,12);
 assert.equal(await page.locator('[data-scene-label-key^="device:"]').count(),0);
 await page.screenshot({path:path.join(output,'factory-overview.png')});
 await seek(4);
 const stock=page.getByLabel('实际物品与共享标签',{exact:true});await stock.locator('summary').first().click();
 const chosen=stock.locator('[data-item-geometry-status]').locator('xpath=..').locator('xpath=..');
 const card=await chosen.evaluateAll((nodes,id)=>nodes.findIndex(n=>n.querySelector('[data-item-geometry-status]')?.getAttribute('data-item-geometry-status')===id),item.id);
 assert.ok(card>=0);await chosen.nth(card).getByRole('button').first().click();await stock.locator('summary').first().click();
 await page.getByRole('button',{name:'跟随选中物品',exact:true}).click();
 for(const index of [4,7,9,12,9,4]){
  if(index!==4||observations.length)await seek(index);
  await page.waitForFunction(id=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();const b=d?.item_bounds[id];return d?.following_item===id&&b&&d.camera.target.every((v,i)=>Math.abs(v-(b.min[i]+b.max[i])/2)<1e-5);},item.id,{timeout:90000});
  d=await diagnostics();assert.equal(d.asset_loading.devices.length,92);assert.ok(d.asset_loading.devices.every(i=>i.loaded));assert.deepEqual(d.asset_loading.items.failures,[]);
  const actual=itemsAt(index).find(i=>i.id===item.id);assert.ok(actual);assert.equal(d.observation_ms,frames[index].at_ms);
  observations.push({index,at_ms:d.observation_ms,item_id:item.id,holder:actual.holder,target:d.camera.target,bounds:d.item_bounds[item.id],camera:d.camera});
  await fs.writeFile(path.join(output,'follow-observations.json'),JSON.stringify(observations,null,2));
  if(index===9&&observations.length===3)await page.screenshot({path:path.join(output,'follow-in-transit.png')});
  if(index===12)await page.screenshot({path:path.join(output,'follow-shipped-case.png')});
 }
 assert.ok(new Set(observations.map(o=>o.holder.kind+':'+o.holder.id)).size>=4);
 assert.ok(Math.hypot(...observations[0].target.map((v,i)=>v-observations[3].target[i]))>5);
 await seek(0);await page.waitForFunction(()=>document.querySelector('canvas')?.readFactoryDiagnostics?.().following_item===null);
 assert.ok((await page.getByRole('status').allTextContents()).some(s=>s.includes('尚无该物品')));
 await page.getByRole('button',{name:'全厂适配',exact:true}).click();
 await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return Math.hypot(...d.camera.eye.map((v,i)=>v-d.camera.target[i]))>35;});
 await page.setViewportSize({width:1366,height:768});await settled(0);
 await page.screenshot({path:path.join(output,'factory-1366.png')});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 assert.deepEqual(errors,[]);
 await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify({source:proof,url,viewports:[[1920,1080],[1366,768]],browser:'Headless Edge through project Playwright; Browser plugin not available',devices:92,zones:12,follow_seeks:observations.length,absent_item_stops_follow:true,whole_factory_accepted:false,ordinary_Native_verified:false,errors},null,2));
 console.log('Factory overview and actual item follow passed:',output);
}catch(error){await page.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});await fs.writeFile(path.join(output,'failure.json'),JSON.stringify({error:String(error),errors,body:await page.locator('body').innerText(),diagnostics:await diagnostics()},null,2));throw error;}
finally{await browser.close();}
