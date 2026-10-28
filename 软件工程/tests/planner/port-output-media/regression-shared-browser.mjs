import {createRequire} from 'node:module';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';

const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const {chromium}=require('playwright'),root=new URL('../../../',import.meta.url),out=new URL('scenarios/图片时间轴/runtime-port-output/old-regressions/',root);
const expected=JSON.parse(await readFile(new URL('shared-expectations.json',out),'utf8'));
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const checks=[],evidence=[];
async function open(fixture){
 const child=spawn(fileURLToPath(new URL('core_consumer.exe',out)),[fileURLToPath(root),`scenarios/图片时间轴/runtime/${fixture}.input.json`],{windowsHide:true,stdio:['pipe','pipe','pipe']}),queue=[];
 let accept,reject,stderr='',serial=Promise.resolve();const first=new Promise((a,b)=>{accept=a;reject=b;});
 child.stderr.on('data',s=>stderr+=s);child.on('exit',code=>{if(code){const error=Error(stderr);reject(error);for(const q of queue.splice(0))q.reject(error);}});
 createInterface({input:child.stdout}).on('line',line=>{const r=JSON.parse(line);if(r.ready)accept(r);else queue.shift()?.resolve(r);});
 const ready=await first,calls=[],errors=[],warnings=[];
 function rpc(q){const next=serial.then(()=>new Promise((resolve,reject)=>{queue.push({resolve:r=>{calls.push({request:q,response:r});resolve(r);},reject});child.stdin.write(JSON.stringify(q)+'\n');}));serial=next;return next;}
 const page=await browser.newPage({viewport:{width:1500,height:1150}});page.setDefaultTimeout(15000);
 page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
 await page.route('**/__p303_core*',async route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(route.request().method()==='GET'?ready:await rpc(route.request().postDataJSON()))}));
 await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/scenarios/图片时间轴/runtime/preview.html');
 const editor=page.getByRole('region',{name:'图片时间轴编辑'}),surface=editor.locator('.sf-media');await editor.waitFor();await page.waitForFunction(()=>window.__p303.read().actual!==null);
 const read=()=>page.evaluate(()=>window.__p303.read());
 async function tab(name){await editor.getByRole('navigation',{name:'表现编辑分类'}).getByRole('button',{name,exact:true}).click();}
 async function u(value,transition='hold',alpha=null){await editor.getByRole('slider',{name:'观看u',exact:true}).fill(String(value));await page.waitForFunction(({transition,alpha})=>{const c=document.querySelector('.sf-media-canvas canvas');return c?.dataset.transition===transition&&(alpha==null||Math.abs(Number(c.dataset.currentAlpha)-alpha)<1e-10);},{transition,alpha});}
 async function pixel(x,y){return page.evaluate(({x,y})=>{const c=document.querySelector('.sf-media-canvas canvas'),s=Math.min(c.width/800,c.height/450);return [...c.getContext('2d').getImageData((c.width-800*s)/2+x*s,(c.height-450*s)/2+y*s,1,1).data];},{x,y});}
 async function at(value){await page.evaluate(t=>window.__p303.seekActual(t),value);await page.waitForFunction(t=>window.__p303.read().actual?.frame.snapshot.at_ms===t,value);await surface.locator('canvas').waitFor();await surface.locator('.sf-media-caption').filter({hasText:/(working|waiting|completed)/}).waitFor();}
 async function check(name,fn){await fn();checks.push({fixture,name,status:'passed'});console.log('PASS '+fixture+' '+name);}
 async function close(){await page.screenshot({path:fileURLToPath(new URL(`shared-${fixture}-final.png`,out)),fullPage:true});evidence.push({fixture,errors,warnings,calls,view:await read(),ui:await editor.innerText(),Native:false,Host_or_Engine_mock:false,renderer_test_double:false});await page.close();child.stdin.end();}
 return {page,editor,surface,ready,rpc,read,tab,u,pixel,at,check,close,errors,warnings};
}
let active;
try{
 active=await open('fixed');let s=active;
 await s.check('real shared hold boundaries and original four source colors',async()=>{
  for(const row of expected.asset_pixels){await s.u(row.view_u);await s.page.waitForFunction(({x,y,rgba})=>{const c=document.querySelector('.sf-media-canvas canvas');if(!c)return false;const z=Math.min(c.width/800,c.height/450);return [...c.getContext('2d').getImageData((c.width-800*z)/2+x*z,(c.height-450*z)/2+y*z,1,1).data].every((v,i)=>v===rgba[i]);},{x:row.source_xy[0],y:row.source_xy[1],rgba:row.rgba});}
  await s.u(.199);assert.deepEqual(await s.pixel(320,130),[247,250,252,255]);await s.u(.2);await s.page.waitForFunction(()=>document.querySelector('.sf-media-caption')?.textContent.includes('静态原图'));assert.deepEqual(await s.pixel(320,130),[49,116,157,255]);
 });
 await s.check('same real shared crossfade pixels and reverse seek with exact own images',async()=>{
  await s.tab('图片与区间');await s.editor.getByRole('button',{name:'时间轴片段 frame-b',exact:true}).click();await s.editor.getByRole('combobox',{name:'图片过渡',exact:true}).selectOption('crossfade');const e=expected.crossfade,alpha=(e.view_u-.2)/.35/.1;
  await s.u(e.view_u,'crossfade',alpha);const actual=await s.pixel(...e.source_xy);assert.ok(actual.every((v,i)=>Math.abs(v-e.rgba[i])<=e.byte_rounding_tolerance),JSON.stringify(actual));evidence.push({crossfade:{actual,expected:e.rgba,view_u:e.view_u,source_xy:e.source_xy,byte_rounding_tolerance:e.byte_rounding_tolerance}});
  await s.u(.25,'crossfade',1);await s.u(e.view_u,'crossfade',alpha);assert.deepEqual(await s.pixel(...e.source_xy),actual);
 });
 await s.check('canonical pan and zoom actually move original SVG pixels',async()=>{
  const values=[];
  for(const transition of ['pan','zoom']){await s.editor.getByRole('combobox',{name:'图片过渡',exact:true}).selectOption(transition);await s.u(.2,transition);const before=await s.pixel(310,130);await s.u(.235,transition);const after=await s.pixel(310,130);assert.notDeepEqual(before,after,transition);values.push({transition,before,after});}
  evidence.push({pixels:values});
 });
 await s.check('local edited image transition never changes frozen formal presentation or work',async()=>{
  await s.editor.getByRole('combobox',{name:'查看来源',exact:true}).selectOption('actual_run');await s.at(30000);assert.equal((await s.read()).actual.presentation.timeline[1].transition,'hold');assert.equal((await s.read()).actual.operation.duration.value,60000);
  await s.page.waitForFunction(()=>document.querySelector('.sf-media-canvas canvas')?.dataset.transition==='hold');assert.deepEqual((await s.read()).run,s.ready.formal_run);assert.ok((await s.editor.locator('header').innerText()).includes('本地有修改'));
 });
 assert.deepEqual(s.errors,[]);await s.close();active=null;
 active=await open('phase');s=active;
 await s.check('real catalog binds five typed fields; declared preview shows unavailable without zero',async()=>{
  await s.tab('标签与热点');for(let i=0;i<5;i++){await s.editor.getByRole('button',{name:'新增说明标签',exact:true}).click();await s.editor.getByRole('combobox',{name:'绑定实际字段',exact:true}).selectOption(String(i));}
  await s.editor.getByRole('button',{name:'保存到当前草稿',exact:true}).click();await s.page.waitForFunction(()=>document.querySelector('.sf-p303 header')?.textContent.includes('已同步'));
  assert.equal(await s.surface.locator('[data-field-status="unavailable"]').count(),5);assert.equal(await s.surface.locator('[data-field-status="available"]').count(),0);
  const previous=(await s.read()).run.id;await s.editor.getByRole('button',{name:'明确运行已保存草稿样例',exact:true}).click();await s.page.waitForFunction(id=>window.__p303.read().run.id!==id,previous);await s.page.waitForFunction(()=>window.__p303.read().actual!==null);assert.equal((await s.read()).actual.operation.phases[1].duration.value,45000);
 });
 await s.check('actual new Host JobsAt keeps auto final image at55/60 with raw unload0 fields',async()=>{
  const rows=[];for(const t of [55000,60000]){await s.at(t);const value=(await s.read()).actual;assert.equal(value.job.state,'waiting_resource');assert.equal(value.job.phase_ref,'unload');assert.equal(value.job.effective_progress_ms,0);await s.surface.locator('.sf-media-caption').filter({hasText:'auto · 45000 ms'}).waitFor();rows.push(await s.pixel(320,160));const fields=await s.surface.locator('[data-field-status]').allTextContents();assert.deepEqual(fields.slice(0,3),['waiting_resource','unload','0 ms']);assert.equal(value.raw.job_pages[0].objects[0].kind,'job');if(t===55000)await s.page.screenshot({path:fileURLToPath(new URL('shared-phase-actual-55000.png',out)),fullPage:true});}
  assert.deepEqual(rows,[[49,116,157,255],[49,116,157,255]]);
 });
 await s.check('actual arbitrary72ms unload work and75/80 output hold precede85 acceptance',async()=>{
  await s.at(72000);assert.equal((await s.read()).actual.job.effective_progress_ms,2000);await s.surface.getByText('40.0%',{exact:true}).waitFor();
  for(const t of [75000,80000]){await s.at(t);const a=(await s.read()).actual;assert.equal(a.job.state,'waiting_output');assert.equal(a.job.phase_ref,null);assert.equal(a.job.effective_progress_ms,0);assert.equal(a.job.pending_outputs.length,1);const output=a.frame.snapshot.objects.find(o=>o.kind==='item'&&o.data.id===a.job.pending_outputs[0]).data;assert.equal(output.holder.kind,'job');assert.equal(output.quantity.unit,'piece');assert.equal(output.quantity.value.coefficient,1);await s.surface.locator('.sf-media-caption').filter({hasText:'unload · 5000 ms'}).waitFor();assert.equal((await s.surface.locator('[data-field-status]').allTextContents())[2],'0 ms');if(t===75000)await s.page.screenshot({path:fileURLToPath(new URL('shared-phase-actual-75000.png',out)),fullPage:true});}
  await s.at(85000);const a=(await s.read()).actual;assert.equal(a.job.state,'completed');assert.equal(a.run.acceptance_events[0].at_ms,85000);assert.equal(a.run.acceptance_events[0].quantity.unit,'piece');assert.equal(a.run.acceptance_events[0].quantity.value.coefficient,1);await s.surface.getByText('100.0%',{exact:true}).waitFor();
 });
 await s.check('actual frame and frozen source evidence remain unchanged by declared loop/seek/rate',async()=>{
  const before=await s.read(),count=await s.page.evaluate(()=>window.__p303.evidence.requests.length);await s.editor.getByRole('combobox',{name:'查看来源',exact:true}).selectOption('isolated_view');const controls=s.editor.locator('.sf-p303-view > aside');await controls.getByRole('checkbox',{name:'循环观看',exact:true}).check();await controls.getByRole('combobox',{name:'观看倍率',exact:true}).selectOption('4');await controls.getByRole('button',{name:'播放观看',exact:true}).click();await s.page.waitForFunction(()=>Number(document.querySelector('[aria-label="观看进度"]').textContent)>.1);await controls.getByRole('button',{name:'暂停观看',exact:true}).click();await s.u(.85);assert.equal(await s.page.evaluate(()=>window.__p303.evidence.requests.length),count);assert.deepEqual((await s.read()).run,before.run);assert.equal(await s.surface.locator('[data-field-status="unavailable"]').count(),5);
 });
 await s.tab('图片与区间');await s.page.screenshot({path:fileURLToPath(new URL('shared-phase-editor-desktop.png',out)),fullPage:true});await s.page.setViewportSize({width:390,height:844});await s.page.screenshot({path:fileURLToPath(new URL('shared-phase-editor-mobile.png',out)),fullPage:true});evidence.push({mobile:await s.page.evaluate(()=>({width:innerWidth,scroll:document.documentElement.scrollWidth}))});
 assert.deepEqual(s.errors,[]);await s.close();active=null;
}catch(error){checks.push({name:'shared consumer execution',status:'failed',error:String(error),stack:error.stack});process.exitCode=1;if(active)await active.close().catch(()=>{});}
finally{await writeFile(new URL('shared-browser-result.json',out),JSON.stringify({checks,evidence,original_expectations:expected,real_shared_renderer:true,actual_Host_frames:true,Native:false},null,2));await browser.close();console.log(JSON.stringify(checks,null,2));}
