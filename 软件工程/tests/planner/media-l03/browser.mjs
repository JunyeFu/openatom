import {createRequire} from 'node:module';
import {writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const {chromium}=require('playwright');
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-l03/browser/',root);await mkdir(out,{recursive:true});
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const page=await browser.newPage({viewport:{width:1540,height:960}}),errors=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const base='http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/')+'tests/planner/media-l03/harness.html';
const checks=[];
async function ready(){await page.waitForFunction(()=>window.__l03?.read().ready||window.__l03?.read().failure,{timeout:20000});const r=await page.evaluate(()=>window.__l03.read());if(r.failure)throw Error(JSON.stringify(r.failure));return r;}
async function set(patch){await page.evaluate(p=>window.__l03.set(p),patch);return ready();}
async function check(name,fn){try{const detail=await fn();checks.push({name,passed:true,detail});console.log('PASS '+name);}catch(error){checks.push({name,passed:false,error:String(error),stack:error.stack});console.log('FAIL '+name+' '+error);}}
const assert=(await import('node:assert/strict')).default;
const near=(a,b)=>assert.ok(Math.abs(a-b)<.0001,`${a} != ${b}`);
try{
await page.goto(base);await ready();
await check('real ASM shell-PCB-four fasteners occupy the supported jig with actual worker claim',async()=>{
 const r=await set({at:43000}),parts=r.items.filter(i=>i.holder.kind==='job'&&i.holder.id.includes('step-ASM01')&&!i.unavailable);
 assert.equal(parts.filter(i=>i.type==='fastener').length,4);
 const pcb=parts.find(i=>i.type==='pcb'),board=r.meshes.find(m=>m.ref?.id===pcb?.id&&m.name==='CircuitBoard');assert.ok(board);near(board.position[0],2);near(board.position[1],.832);
 const heads=parts.filter(i=>i.type==='fastener').map(i=>r.meshes.find(m=>m.ref?.id===i.id&&m.name==='SocketHead'));assert.ok(heads.every(Boolean));
 for(const head of heads){near(Math.abs(head.position[0]-2),.103);near(Math.abs(head.position[2]+4),.073);near(head.position[1],.8655);}
 const worker=r.resources.find(v=>v.id==='W-ASSEMBLER-01');assert.equal(worker.status.kind,'claimed');assert.equal(worker.status.job_ids.length,1);
 assert.ok(r.ready.visible_objects.some(v=>v.kind==='resource'&&v.id==='W-ASSEMBLER-01'));return {board:board.position,heads:heads.map(h=>h.position),worker};
});
await check('same original cargo actual grip-move-deliver and reverse seek in one scene',async()=>{
 const points=[];
 for(const [at,x] of [[43000,0],[48000,2.65],[48999,2.65],[49000,2.65],[43000,0]]){
  const r=await set({at}),item=r.items.find(i=>i.type==='assembly'&&i.id.includes('L03-TASK1:0'));
  const mesh=r.meshes.find(m=>m.ref?.id===item?.id&&m.name==='GaugeBase');assert.ok(mesh);near(mesh.position[0],x+2);near(mesh.position[1],.809);
  assert.equal(r.canvases,1);assert.equal(r.unchanged,true);points.push({at,id:item.id,holder:item.holder,base:mesh.position});
 }return points;
});
await check('same package closure follows actual seal boundary and inside item identity',async()=>{
 const points=[];
 for(const [at,sealed] of [[87999,false],[88000,true],[89999,true],[90000,true],[87999,false]]){
  const r=await set({at}),item=r.items.find(i=>i.type==='package'&&i.id.includes('L03-TASK1:0'));
  assert.ok(item);assert.equal(item.attributes.closure.value,sealed?'sealed':'open');
  const tape=r.meshes.find(m=>m.ref?.id===item.id&&m.name==='ClosureTape');assert.ok(tape);near(tape.scale[0],sealed?1:.000001);
  const contained=r.items.find(i=>i.holder.kind==='container'&&i.holder.id===item.id);assert.ok(contained);assert.equal(contained.unavailable,null);
  const child=r.meshes.find(m=>m.ref?.id===contained.id&&m.name==='GaugeBase');assert.ok(child);
  points.push({at,id:item.id,state:item.attributes.closure.value,holder:item.holder,contained:contained.id,tapeSize:tape.size});
 }return points;
});
await check('explicit three LOD preserve full same Run and all available actual identities',async()=>{
 const points=[];
 for(const lod of ['detailed','simplified','region','detailed']){
  const r=await set({at:330000,lod});assert.equal(r.unchanged,true);assert.equal(r.canvases,1);assert.equal(r.devices.length,8);
  const bad=r.items.filter(i=>i.unavailable&&!i.unavailable.includes('已转化'));assert.deepEqual(bad,[]);
  assert.ok(r.resources.some(v=>v.id==='PAL01'&&!v.unavailable));assert.equal(r.resources.filter(v=>v.status).length,4);
  if(lod!=='region')assert.ok(r.ready.visible_objects.some(v=>v.kind==='resource'&&v.id==='PAL01'));
  await page.screenshot({path:fileURLToPath(new URL('whole-'+lod+'.png',out)),fullPage:true});points.push({lod,refs:r.ready.visible_objects,render:r.render});
 }return points;
});
await check('public line RenderSurface retains actual connected Locations, PAL and package ancestry',async()=>{
 const r=await set({at:330000,lod:'detailed',mode:'public'});assert.equal(r.canvases,1);assert.equal(r.unchanged,true);
 assert.ok(r.ready.visible_objects.some(v=>v.kind==='resource'&&v.id==='PAL01'));
 const packages=r.items.filter(i=>i.type==='package');assert.equal(packages.length,6);
 for(const item of packages){assert.ok(r.meshes.some(m=>m.ref?.id===item.id&&m.name==='CartonBottom'));const child=r.items.find(i=>i.holder.kind==='container'&&i.holder.id===item.id);assert.ok(child);assert.ok(r.meshes.some(m=>m.ref?.id===child.id&&m.name==='GaugeBase'));}
 await page.screenshot({path:fileURLToPath(new URL('public-line-retains-locations.png',out)),fullPage:true});return r.ready;
});
for(const [name,at,device] of [['robot-output-wait',48000,'R01'],['sealer-output-wait',90000,'CASE01'],['quality-pending',183000,'V01']]){
 await check('actual distinct '+name+' held pose and identity',async()=>{
  await page.goto(base+'?case='+name);await ready();const r=await set({at,focus:device});assert.equal(r.unchanged,true);assert.equal(r.devices.find(d=>d.id===device).projection,'active');
  assert.deepEqual(r.items.filter(i=>i.unavailable&&!i.unavailable.includes('已转化')),[]);
  await page.screenshot({path:fileURLToPath(new URL(name+'.png',out)),fullPage:true});return {state:r.state,refs:r.ready.visible_objects,items:r.items.filter(i=>i.holder.kind==='job')};
 });
}
for(const kind of ['image','gif']){
 await check('actual same full L03 with '+kind+' loader and no business mutation',async()=>{
  await page.goto(base+'?case='+kind);await ready();const r=await set({at:43000,focus:'ASM01'});assert.equal(r.unchanged,true);assert.equal(r.canvases,1);
  assert.ok(r.ready.visible_objects.some(v=>v.kind==='device'&&v.id==='ASM01'));
  await page.screenshot({path:fileURLToPath(new URL('same-behavior-'+kind+'.png',out)),fullPage:true});return {state:r.state,render:r.render,limitations:r.ready.limitations};
 });
}
await writeFile(new URL('matrix.actual.json',out),JSON.stringify({kind:'actual_Chromium_product_scene_readonly_projection_of_actual_fresh_Host_Run_not_Native_or_NFR',errors,checks},null,2));
console.log(JSON.stringify({passed:checks.filter(c=>c.passed).length,total:checks.length,errors}));
if(checks.some(c=>!c.passed)||errors.length)process.exitCode=1;
}finally{await browser.close();}
