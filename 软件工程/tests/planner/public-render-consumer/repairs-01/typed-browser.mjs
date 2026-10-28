// Independent controlled-browser consumer. Run only after Root freezes the source.
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import assert from 'node:assert/strict';
const stage=process.argv[2];
if(!stage||!process.argv.includes('--root-consumer-frozen'))throw Error('Explicit Root-frozen consumer cut and evidence stage required');
const root=new URL('../../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/repairs-01/',root);
const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
const indices=await Promise.all(['scenarios/Agent共建/runtime-public-render-host/actual-inputs-index.json','scenarios/Agent共建/runtime-public-render-host/actual-visual-supplement/complete-index.json'].map(json));
const normalize=value=>Array.isArray(value)?value.map(normalize):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(k=>[k,normalize(value[k])])):value;
const same=(a,b)=>JSON.stringify(normalize(a))===JSON.stringify(normalize(b));
const reverse=value=>Array.isArray(value)?value.map(reverse):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).reverse().map(k=>[k,reverse(value[k])])):value;
const dataset=async (group,n)=>{const c=indices[group].captures[n-1],directory=c.directory;return {group,n,directory,task:{frame:await json(directory+'/frame.json'),input:await json(directory+'/input.json')},bytes:await json(directory+'/bytes.json')};};
const selected=[23,28,36,41];
const apparatusCorrections=process.argv.includes('--apparatus-corrections');
const cases=apparatusCorrections?await Promise.all([7,13,37,38,45].map(id=>dataset(0,id))):[...await Promise.all(selected.map(id=>dataset(0,id))),...await Promise.all([4,8,10].map(i=>dataset(1,i)))];
const req=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const {chromium}=req('playwright');
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const url='http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/public-render-consumer/consumer.html';
const checks=[],gifFrames=[];
const realRefs=input=>{
 const refs=new Set(),add=(kind,id)=>{if(id)refs.add(kind+':'+id);};
 add('project',input.project.id);
 for(const container of [input.project,input.template_subset].filter(Boolean))
  for(const [field,kind] of Object.entries({halls:'hall',lines:'line',devices:'device',locations:'location',resources:'resource',item_types:'item_type',products:'product',boms:'bom',routes:'route',orders:'order',connections:'connection'}))
   for(const value of container[field]??[])add(kind,value.id??value.data?.id);
 for(const module of input.modules)add('module',module.definition.id);
 for(const value of input.scenarios){add('scenario',value.id);for(const order of value.orders)add('order',order.id);for(const task of value.production_tasks)add('task',task.id);}
 if(input.run)add('run',input.run.id);
 for(const value of input.comparison_runs)add('run',value.run.id);
 for(const value of input.objects_at)add(value.kind,value.data.id);
 for(const value of input.assets)add('asset',value.asset.id);
 if(input.packaging){add('artifact',input.packaging.artifact.id);add('packaging_job',input.packaging.job.id);}
 return refs;
};
async function setup(data,variant){
 const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[],warnings=[],served=[];
 page.setDefaultTimeout(15000);
 page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
 const task=variant?variant(data.task):data.task;
 await page.route('**/__render_task_fixture',r=>r.fulfill({status:200,contentType:'application/json',body:JSON.stringify(task)}));
 await page.route('**/__render_task_bytes',async r=>{
  const request=r.request().postDataJSON(),q=request.args.request;
  const list=request.command==='native_render_asset'?data.bytes.assets:data.bytes.files;
  const entry=list.find(value=>q.asset?same(value.source,q.source)&&same(value.asset,q.asset):same(value.file,q.file));
  if(!entry){served.push({request,status:'outside frozen closure'});await r.fulfill({status:422,body:'Independent exact frozen source gate rejected reference'});return;}
  const bytes=await readFile(new URL(data.directory+'/'+entry.bytes_file,root));served.push({request,bytes:bytes.length});
  await r.fulfill({status:200,contentType:'application/octet-stream',body:bytes});
 });
 return {page,errors,warnings,served,task};
}
async function run(data,name,variant){
 const state=await setup(data,variant),{page,task}=state;
 const row={name,source:data.directory,kind:task.frame.request.view.kind,controlled_variant:variant?'JSON property ordering only; typed values unchanged':null};checks.push(row);
 try{
  await page.goto(url);assert.equal(await page.title(),'九视图实际冻结消费者');
  await page.waitForFunction(()=>window.__renderConsumer?.read().ready.length||window.__renderConsumer?.read().failures.length);
  const actual=await page.evaluate(()=>window.__renderConsumer.read());row.actual=actual;
  assert.deepEqual(actual.failures,[]);assert.equal(actual.ready.length,1);
  const ready=actual.ready[0].ready;
  assert.deepEqual(actual.ready[0].frame,task.frame);assert.deepEqual(ready.view,task.frame.request.view);
  assert.equal(ready.scene_revision,task.frame.scope_revision);assert.equal(ready.project_revision,task.frame.request.view.project_revision);
  assert.deepEqual(actual.stored,[]);
  assert.ok(actual.calls.every(c=>['native_render_asset','native_render_file','native_render_ready','native_render_failure'].includes(c.command)));
  const refs=realRefs(task.input);if(task.frame.scope.kind==='definition_draft')refs.add('definition_draft:'+task.frame.scope.data.id);
  if(task.frame.request.object_id)refs.add(task.frame.request.object_id.kind+':'+task.frame.request.object_id.id);
  for(const ref of ready.visible_objects)assert.ok(refs.has(ref.kind+':'+ref.id),'Invented visible ObjectRef '+JSON.stringify(ref));
  const kind=task.frame.request.view.kind;
  if(['timeline_snapshot','comparison','packaging_manufacturing'].includes(kind))assert.ok(ready.limitations.some(v=>v.message.includes('二维')),'2D camera limitation missing');
  if(kind==='packaging_manufacturing'){
   const files=task.input.packaging.result.manufacturing_files;
   assert.equal(await page.locator('.manufacturing-preview img').count(),files.length);
   assert.equal(actual.calls.filter(c=>c.command==='native_render_file').length,files.length);
   for(const q of state.served)assert.ok(files.some(f=>same(f.file,q.request.args.request.file)));
   assert.ok(ready.visible_objects.every(ref=>ref.kind==='artifact'&&ref.id===task.input.packaging.artifact.id));
  }
  if(kind==='comparison')for(const {run}of task.input.comparison_runs)assert.ok((await page.locator('body').innerText()).includes(run.id));
  if(kind==='timeline_snapshot')assert.ok(await page.locator('.sf-timeline button').evaluateAll(values=>values.every(v=>v.matches(':disabled'))));
  if(data.group===1&&[6,7].includes(data.n)){
   const expected=task.frame.request.view.camera.eye;
   assert.ok(actual.scenes.some(scene=>scene.eye.every((v,i)=>Math.abs(v-[expected.x,expected.y,expected.z][i])<1e-9)),'Actual rendered camera was clamped although Ready echoed request');
  }
  if(data.group===1&&data.n===2){
   const definition=task.input.modules.find(value=>value.definition.id===task.frame.request.module_ref.id).definition;
   const p=definition.payload.data.presentation,pose=p.poses.find(value=>value.id===task.frame.request.preview.pose_name);
   assert.ok(pose);assert.equal(task.input.run,null);assert.equal(task.input.active_job_id,null);
   const geometry=await page.evaluate(()=>window.__renderConsumer.geometry());row.geometry=geometry;
   for(const entry of pose.node_transforms){
    const node=geometry.flat().find(value=>value.name===p.nodes[entry.node_index].name);assert.ok(node);
    const {translation,unit,coordinate_frame}=entry.pose,factor=unit==='millimeter'?.001:1;
    const expected=(coordinate_frame==='right_handed_z_up'?[translation.x,translation.z,-translation.y]:[translation.x,translation.y,translation.z]).map(v=>v*factor);
    assert.ok(node.position.every((v,i)=>Math.abs(v-expected[i])<1e-9),'Declared pose not applied to actual original node');
   }
  }
  if(data.group===1&&[3,5].includes(data.n)&&!variant){
   const canvas=page.locator('.sf-media-canvas canvas'),first=await canvas.screenshot(),frameIndex=await canvas.getAttribute('data-frame-index');
   await page.evaluate(()=>new Promise(resolve=>{let paints=20;function next(){if(--paints)requestAnimationFrame(next);else resolve();}requestAnimationFrame(next);}));
   assert.equal(await canvas.getAttribute('data-frame-index'),frameIndex);assert.ok(first.equals(await canvas.screenshot()),'GIF advanced on wall clock after requested frame');
   gifFrames.push({id:data.n,frameIndex,bytes:first});row.fixed_gif_frame={frameIndex,paint_observations:20};
  }
  if(data.group===1&&data.n===8)assert.deepEqual(ready.view.object_ref,task.frame.request.object_id);
  if(data.group===1&&data.n===10)assert.ok(ready.visible_objects.some(ref=>ref.kind==='device'),'Explicit facing Line camera should show actual selected devices');
  assert.deepEqual(state.errors,[]);assert.deepEqual(state.warnings,[]);
  row.status='passed';
 }catch(error){row.status='failed';row.error=String(error);row.stack=error.stack;row.body=(await page.locator('body').innerText()).slice(0,6500);row.actual=await page.evaluate(()=>window.__renderConsumer?.read()).catch(()=>null);process.exitCode=1;}
 finally{
  row.errors=state.errors;row.warnings=state.warnings;row.served=state.served;
  row.screenshot=`${stage}-${checks.length}.png`;
  await page.screenshot({path:fileURLToPath(new URL(row.screenshot,out)),fullPage:false});await page.close();
  console.log(row.status.toUpperCase(),name,row.error??'');
 }
}
try{
 for(const data of cases)await run(data,`${data.group?'supplement':'original'}-${data.n} ${data.task.frame.request.view.kind}`);
 if(!apparatusCorrections){for(const [group,n]of [[0,23],[0,28],[0,36],[1,4]]){
  const data=await dataset(group,n);
  await run(data,`typed identical reordered asset/source ${group?'supplement':'original'}-${n}`,task=>reverse(task));
 }
 const line=await dataset(1,8);
 await run(line,'typed identical reordered Line target only',task=>{const copy=structuredClone(task);copy.frame.request.view.object_ref=reverse(copy.frame.request.view.object_ref);return copy;});
 if(gifFrames.length){try{assert.equal(gifFrames.length,2);assert.notEqual(gifFrames[0].frameIndex,gifFrames[1].frameIndex);assert.ok(!gifFrames[0].bytes.equals(gifFrames[1].bytes),'Distinct declared GIF progress must select distinct actual frames');checks.push({name:'Two explicit GIF progress samples select distinct original frames',status:'passed'});}
 catch(error){checks.push({name:'Two explicit GIF progress samples select distinct original frames',status:'failed',error:String(error)});process.exitCode=1;}}}
}finally{
 await browser.close();
 await writeFile(new URL(`${stage}.json`,out),JSON.stringify({evidence:'Independent controlled browser over actual other-owner product components and exact Host-produced input/resources. Not Native/CapturePreview/MCP.',source_indices:['session-82352','session-84240'],Root_consumer_freeze_required:true,checks,gif:gifFrames.map(({bytes,...v})=>v),passed:checks.every(c=>c.status==='passed'),Native:false},null,2));
}
