import {createRequire} from 'node:module';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const stage=process.argv[2];if(!['red','green'].includes(stage))throw new Error('Explicit red or green required');
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const {chromium}=require('playwright'),root=new URL('../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/regression/old-factory/',root);await mkdir(out,{recursive:true});
const url='http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/factory-media/harness.html';
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const checks=[];
async function settle(page, models=true){await page.waitForFunction(()=>document.querySelector('.factory-scene canvas'));if(models&&stage==='green')await page.waitForFunction(()=>window.__factory.capture()?.meshes.length>60);await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));}
async function update(page,patch,models=true){await page.evaluate(p=>window.__factory.set(p),patch);await settle(page,models);}
const capture=page=>page.evaluate(()=>window.__factory.capture());
const read=page=>page.evaluate(()=>window.__factory.read());
const near=(a,b,tolerance=1e-5)=>assert.ok(a.length===b.length&&a.every((n,i)=>Math.abs(n-b[i])<tolerance),`expected ${JSON.stringify(b)}, actual ${JSON.stringify(a)}`);
async function check(name,execute){const page=await browser.newPage({viewport:{width:1300,height:900}}),errors=[],warnings=[],evidence=[];page.setDefaultTimeout(10000);page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});const row={name,evidence,errors,warnings};checks.push(row);
try{await page.goto(url);await settle(page);await execute(page,evidence);assert.equal((await read(page)).runUnchanged,true);assert.deepEqual(errors,[]);row.status='passed';console.log('PASS '+name);}catch(e){row.status='failed';row.error=String(e);row.stack=e.stack;console.log('FAIL '+name+' '+String(e));}finally{await page.screenshot({path:fileURLToPath(new URL(`${stage}-${checks.length}.png`,out)),fullPage:true});await page.close();}}
try{
await check('real FactoryScene detailed GLB geometry, two transformed instances and one WebGL',async(page,evidence)=>{
 const c=await capture(page);evidence.push(c);assert.ok(c.meshes.length>=100,'original detailed canvas must consume actual GLB geometry');assert.equal(c.canvases,1);assert.ok(c.colorful>100);
 const a=c.devices.find(d=>d.id==='CNC'),b=c.devices.find(d=>d.id==='B');near(a.origin,[6,0,-20]);near(b.origin,[3,0,-20]);
 // Same GLB, a declared 1.5 scale and quarter-turn; no fit-to-footprint normalization.
 const meshA=c.meshes.find(m=>m.ref.id==='CNC'&&m.name==='Base_weldment'),meshB=c.meshes.find(m=>m.ref.id==='B'&&m.name==='Base_weldment');
 assert.ok(meshA&&meshB,'exact imported GLB base mesh names');near([...meshA.size].sort((a,b)=>a-b).map(v=>v*1.5),[...meshB.size].sort((a,b)=>a-b));
 near(meshA.size,[1.72,.16,1.1]);near(meshB.size,[1.65,.24,2.58]);
 assert.ok(meshA.world[1]>.5&&meshA.world[1]<1,'GLB Y-up remains actual physical height');
});
await check('image uses controlled decoded real pixels on the same physical scene',async(page,evidence)=>{
 await page.waitForFunction(()=>window.__factory.capture().textures.length===1);const c=await capture(page);evidence.push(c);const t=c.textures[0];assert.equal(t.id,'IMAGE');near(t.glRgba,t.rgba,2);assert.notDeepEqual(t.rgba,[232,237,242,255]);assert.equal(c.canvases,1);
});
await check('real LOD source, region group behavior, selection and camera persistence',async(page,evidence)=>{
 const before=await capture(page),a=before.devices.find(d=>d.id==='CNC'),box=await page.locator('.factory-scene').boundingBox();await page.mouse.click(box.x+a.screen[0],box.y+a.screen[1]);
 if((await read(page)).selection?.kind!=='device')await page.getByRole('button').filter({hasText:/^CNC/}).click();assert.equal((await read(page)).selection.id,'CNC');
 await update(page,{lod:'simplified'});await page.waitForFunction(()=>window.__factory.read().reads.some(r=>r.request.asset.path==='station-simple.glb'&&r.state==='returned'));const simple=await capture(page);evidence.push({before,simple});near(simple.camera,before.camera);assert.ok(simple.meshes.length<before.meshes.length);assert.ok(simple.meshes.length>30);
 await update(page,{lod:'region'},false);const region=await capture(page);evidence.push(region);assert.equal(region.meshes.length,0);assert.equal(region.devices.find(d=>d.id==='CNC').visible,false);assert.equal(region.devices.find(d=>d.id==='IMAGE').visible,true);
 await update(page,{lod:'detailed'});near((await capture(page)).camera,before.camera);
 await update(page,{auto:true});const auto=await capture(page);assert.ok(auto.meshes.length<before.meshes.length);
 const box2=await page.locator('.factory-scene').boundingBox();await page.mouse.move(box2.x+box2.width/2,box2.y+box2.height/2);await page.mouse.wheel(0,5000);await page.waitForFunction(()=>document.querySelector('.scene-caption').textContent.includes('区域'));assert.equal((await capture(page)).meshes.length,0);assert.equal((await read(page)).selection.id,'CNC');
 await update(page,{auto:false,reset:1});const reset=await capture(page);near(reset.camera,before.camera);evidence.push({auto,reset});
});
await check('original Engine stage, wait hold, reverse seek and complete Run immutability',async(page,evidence)=>{
 const positions=[];for(const at of [20000,35000,40000,45000,35000]){await update(page,{actual:true,at});const c=await capture(page),r=await read(page);evidence.push({at,c,r});positions.push(c.meshes.filter(m=>m.ref.id==='CNC').map(m=>({name:m.name,local:m.local})));assert.ok(r.reads.filter(r=>r.request.source.kind==='run_module_version').every(r=>r.request.source.data.module_ref.version==='0.1.0'));}
 assert.deepEqual(positions[1],positions[4],'reverse seek returns exact prior pose');assert.notDeepEqual(positions[0],positions[1],'actual auto effective work advances');
 const waiting=[];for(const at of [35000,40000]){await update(page,{family:'busy',at});const c=await capture(page);evidence.push({family:'busy',at,c});waiting.push(c.meshes.filter(m=>m.ref.id==='CNC').map(m=>({name:m.name,local:m.local})));}assert.deepEqual(waiting[0],waiting[1],'two original actual waiting frames hold exact pose');
});
await check('held byte load cannot attach after LOD/time/Run/project/unmount switches',async(page,evidence)=>{
 await page.evaluate(()=>window.__factory.hold());await update(page,{lod:'simplified'},false);await page.waitForFunction(()=>window.__factory.read().held>0);const held=await read(page);
 await update(page,{lod:'region'},false);await page.evaluate(()=>window.__factory.release());await page.waitForFunction(()=>window.__factory.read().held===0);await settle(page,false);assert.equal((await capture(page)).meshes.length,0);evidence.push({held,after:await read(page)});
 await page.evaluate(()=>window.__factory.hold());await update(page,{lod:'detailed',actual:true,at:35000},false);await page.waitForFunction(()=>window.__factory.read().held>0);
 await update(page,{actual:false,project:'P2',lod:'region'},false);await page.evaluate(()=>window.__factory.release());await settle(page,false);assert.equal((await capture(page)).meshes.length,0);
 await update(page,{project:'P1',actual:true,at:20000,lod:'detailed'});await page.evaluate(()=>window.__factory.hold());await update(page,{at:40000},false);await page.waitForFunction(()=>window.__factory.read().held>0);await update(page,{at:35000,lod:'region'},false);await page.evaluate(()=>window.__factory.release());await settle(page,false);assert.equal((await capture(page)).meshes.length,0);
 await update(page,{actual:false,lod:'detailed'});const before=await read(page);await page.evaluate(()=>window.__factory.set({mounted:false}));await page.waitForFunction(()=>!document.querySelector('.factory-scene canvas'));const after=await read(page);evidence.push({before,after});assert.equal(after.offscreen,0);assert.ok(after.disposed.geometry>before.disposed.geometry);assert.ok(after.disposed.texture>before.disposed.texture);assert.ok(after.disposed.bitmap>before.disposed.bitmap);
 // Same context, actual held bytes: decoder/rig may finish after the renderer
 // is gone, and the scene owner must dispose that completed late instance.
 await page.evaluate(()=>window.__factory.hold());await page.evaluate(()=>window.__factory.set({mounted:true,reset:1}));await settle(page,false);await page.waitForFunction(()=>window.__factory.read().held>0);
 await page.evaluate(()=>window.__factory.set({mounted:false}));await page.waitForFunction(()=>!document.querySelector('.factory-scene canvas'));const atUnmount=await read(page);await page.evaluate(()=>window.__factory.release());await page.waitForFunction(count=>window.__factory.read().disposed.geometry>count,atUnmount.disposed.geometry);await page.waitForFunction(()=>window.__factory.read().offscreen===0);evidence.push({atUnmount,lateDisposed:await read(page)});
});
await check('controlled GIF real frame changes from actual phases in the same scene',async(page,evidence)=>{
 const colors=[];for(const at of [0,35000,45000]){await update(page,{actual:true,at,imageKind:'gif'});await page.waitForFunction(()=>window.__factory.capture().textures.length===2);const c=await capture(page),texture=c.textures.find(t=>t.id==='CNC');evidence.push({at,c});colors.push(texture.probes);assert.ok(texture.glRgba[3]===255);assert.equal(c.canvases,1);}assert.notDeepEqual(colors[0],colors[2]);
});
await check('sequential real Engine JobsAt boundaries select current execution over queued and completed histories',async(page,evidence)=>{
 const selections=[];
 for(const at of [0,40000,80000]){
  await update(page,{family:'sequential',actual:true,at});
  await page.waitForFunction(at=>window.__factory.read().devices?.some(d=>d.device_id==='root-native-context:Assembly01')&&window.__factory.read().devices.every(d=>!d.frame||d.frame.at_ms===at),at);
  const r=await read(page),c=await capture(page),device=r.devices.find(d=>d.device_id==='root-native-context:Assembly01');evidence.push({at,r,c});assert.equal(device.unavailable,null);assert.ok(c.meshes.length>=150);assert.equal(c.canvases,1);selections.push(device.frame?.job_id??null);
  assert.equal(device.projection,at===80000?'static':'active');
 }
 assert.deepEqual(selections,['job:24:ROOT-NATIVE-CONTEXT:task1:035:root-native-context:step-Assembly01','job:24:ROOT-NATIVE-CONTEXT:task1:135:root-native-context:step-Assembly01',null]);
});
}finally{await browser.close();}
await writeFile(new URL(`browser-${stage}.json`,out),JSON.stringify({stage,checks},null,2)+'\n');if(checks.some(check=>check.status!=='passed'))process.exitCode=1;
