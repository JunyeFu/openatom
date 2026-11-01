import {createRequire} from 'node:module';import {writeFile,mkdir} from 'node:fs/promises';import {fileURLToPath} from 'node:url';import assert from 'node:assert/strict';
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-l03/browser/quality/',root);await mkdir(out,{recursive:true});
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'}),page=await browser.newPage({viewport:{width:1500,height:1000}}),errors=[],results=[];page.on('pageerror',e=>errors.push(String(e)));
try{await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/')+'tests/planner/media-l03/harness.html');await page.waitForFunction(()=>window.__l03?.read().ready);
const devices=process.argv.includes('--first')?['ASM01']:['ASM01','R01','CV01','V01','RW01','CV02','PK01','CASE01'];
for(const id of devices)for(const angle of (process.argv.includes('--first')?['tool']:['iso','front','back','bottom','tool'])){
 await page.evaluate(p=>window.__l03.set(p),{mode:'detail',at:43000,focus:id,angle,lod:'detailed'});await page.waitForFunction(()=>window.__l03.read().ready||window.__l03.read().failure);
 const r=await page.evaluate(()=>window.__l03.read());assert.equal(r.failure,null);assert.equal(r.unchanged,true);assert.ok(r.meshes.length>50);
 await page.locator('.sf-media-canvas').screenshot({path:fileURLToPath(new URL(id+'-'+angle+'.png',out))});results.push({id,angle,meshCount:r.meshes.length,visible:r.ready.visible_objects});console.log('ACTUAL '+id+' '+angle);
}
await writeFile(new URL('views.actual.json',out),JSON.stringify({kind:'actual_GLTF_details_same_exact_saved_Host_modules_not_human_or_industrial_acceptance',results,errors},null,2));assert.deepEqual(errors,[]);
}finally{await browser.close();}
