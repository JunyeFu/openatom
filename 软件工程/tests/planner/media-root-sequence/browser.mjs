import {createRequire} from 'node:module';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const stage=process.argv[2];
if(!['red','green'].includes(stage))throw new Error('Pass the explicit evidence stage red or green');
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const {chromium}=require('playwright'),root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-root-sequence/',root);
await mkdir(out,{recursive:true});
const url='http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/media-root-sequence/harness.html';
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const checks=[];
const red=[255,0,0,255],blue=[0,0,255,255],purple=[128,0,128,255];
async function settled(page){
  await page.waitForFunction(()=>document.querySelector('.sf-media-canvas canvas')&&!document.querySelector('.sf-media-message'));
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
}
async function seek(page,u){await page.getByRole('slider',{name:'声明u',exact:true}).fill(String(u));await page.waitForFunction(u=>window.__sequence.read().state.u===u,u);await settled(page);}
async function color(page,samples,expected,label,tolerance=0){
  await settled(page);
  const actual=await page.evaluate(()=>{const c=document.querySelector('.sf-media-canvas canvas');return {rgba:[...c.getContext('2d').getImageData(Math.floor(c.width/2),Math.floor(c.height/2),1,1).data],canvases:document.querySelectorAll('.sf-media-canvas canvas').length,transition:c.dataset.transition,alpha:Number(c.dataset.currentAlpha),caption:document.querySelector('.sf-media-caption').textContent};});
  samples.push({label,expected,tolerance,...actual});
  assert.ok(actual.rgba.every((value,i)=>Math.abs(value-expected[i])<=tolerance),`${label}: expected ${expected}, actual ${actual.rgba}`);
  assert.equal(actual.canvases,1,`${label}: stale surfaces must be disposed`);
}
async function check(name,execute){
  const page=await browser.newPage({viewport:{width:1420,height:900}}),errors=[],warnings=[],samples=[];
  page.setDefaultTimeout(5000);page.on('pageerror',error=>errors.push(String(error)));page.on('console',message=>{if(message.type()==='error')errors.push(message.text());if(message.type()==='warning')warnings.push(message.text());});
  const row={name,status:'running',samples,errors,warnings};checks.push(row);
  try{
    await page.goto(url);await settled(page);
    await execute(page,samples);
    assert.deepEqual(errors,[]);assert.equal(await page.evaluate(()=>window.__sequence.read().runsUnchanged),true);
    row.status='passed';console.log('PASS '+name);
  }catch(error){row.status='failed';row.error=String(error);row.stack=error.stack;console.log('FAIL '+name+' '+String(error));}
  finally{
    row.observed=await page.evaluate(()=>{const value=window.__sequence?.read();return value?{state:value.state,requests:value.requests,held:value.held,runsUnchanged:value.runsUnchanged,frame:{run_id:value.frame.run.id,at_ms:value.frame.snapshot.at_ms,job:value.frame.snapshot.objects.find(o=>o.kind==='job')?.data}}:null;});
    row.screenshot=`${stage}-${name}.png`;await page.screenshot({path:fileURLToPath(new URL(row.screenshot,out)),fullPage:true});await page.close();
  }
}
try{
  await check('explicit-hold-boundary-and-reverse',async(page,samples)=>{
    for(const [u,expected]of [[.25,red],[.499,red],[.5,blue],[.75,blue],[1,blue],[.25,red],[.75,blue]]){await seek(page,u);await color(page,samples,expected,'u='+u);}
    const refs=await page.evaluate(()=>window.__sequence.read().requests.map(value=>value.request.asset));
    for(const [id,path]of [['root-sequence.red','red.png'],['root-sequence.blue','blue.png']])assert.ok(refs.some(ref=>ref.id===id&&ref.path===path&&ref.kind==='png'));
  });
  await check('pure-sequence-identity-retained',async(page,samples)=>{
    await page.getByRole('checkbox',{name:'纯sequence无显式片段',exact:true}).check();
    for(const [u,expected]of [[.25,red],[.5,blue],[1,blue],[.25,red]]){await seek(page,u);await color(page,samples,expected,'sequence u='+u);}
  });
  await check('crossfade-entry-and-reverse',async(page,samples)=>{
    await page.getByRole('combobox',{name:'过渡',exact:true}).selectOption('crossfade');
    for(const [u,expected,tolerance]of [[.499,red,0],[.5,red,0],[.525,purple,1],[.55,blue,0],[.525,purple,1],[.49,red,0]]){await seek(page,u);await color(page,samples,expected,'crossfade u='+u,tolerance);}
  });
  await check('phase-switch-keeps-active-exact-asset',async(page,samples)=>{
    await seek(page,.75);await color(page,samples,blue,'auto B');
    await page.getByRole('combobox',{name:'阶段',exact:true}).selectOption('unload');await seek(page,.25);await color(page,samples,red,'unload A');
    await page.getByRole('combobox',{name:'阶段',exact:true}).selectOption('auto');await seek(page,.75);await color(page,samples,blue,'reverse auto B');
  });
  await check('late-crossfade-read-disposed-after-phase-change',async(page,samples)=>{
    await page.getByRole('combobox',{name:'过渡',exact:true}).selectOption('crossfade');await seek(page,.25);
    await page.evaluate(()=>window.__sequence.holdBlue());await page.getByRole('slider',{name:'声明u',exact:true}).fill('0.75');
    await page.waitForFunction(()=>window.__sequence.read().held>0);
    await page.getByRole('combobox',{name:'阶段',exact:true}).selectOption('unload');await seek(page,.25);await color(page,samples,red,'new phase while old blue bytes held');
    await page.evaluate(()=>window.__sequence.release());await page.waitForFunction(()=>window.__sequence.read().requests.every(value=>value.state==='returned'));
    await color(page,samples,red,'late blue discarded, one red canvas');
  });
  await check('actual-engine-frame-mode-and-business-immutable',async(page,samples)=>{
    const before=await page.evaluate(()=>JSON.stringify(window.__sequence.read().frame.run));
    await page.getByRole('combobox',{name:'模式',exact:true}).selectOption('actual_run');
    for(const [label,expected]of [['calendar@0',red],['calendar@20000',blue],['calendar@35000',blue],['calendar@40000',red],['calendar@45000',blue]]){
      await page.getByRole('combobox',{name:'实际帧',exact:true}).selectOption(label);await settled(page);await color(page,samples,expected,label);
      const observed=await page.evaluate(()=>window.__sequence.read());assert.ok(observed.requests.filter(r=>r.request.source.kind==='run_module_version').every(r=>r.request.source.data.run_id===observed.frame.run.id));
    }
    assert.equal(await page.evaluate(()=>JSON.stringify(window.__sequence.read().frame.run)),before);
    await page.getByRole('combobox',{name:'模式',exact:true}).selectOption('declared_preview');await seek(page,.75);await color(page,samples,blue,'back to declaration');
  });
  await check('late-image-read-disposed-after-mode-switch',async(page,samples)=>{
    await page.evaluate(()=>window.__sequence.holdBlue());await page.getByRole('slider',{name:'声明u',exact:true}).fill('0.75');
    await page.waitForFunction(()=>window.__sequence.read().held>0);
    await page.getByRole('combobox',{name:'模式',exact:true}).selectOption('actual_run');await color(page,samples,red,'actual load A while declaration B held');
    await page.evaluate(()=>window.__sequence.release());await page.waitForFunction(()=>window.__sequence.read().requests.every(value=>value.state==='returned'));
    await color(page,samples,red,'late declaration B cannot replace actual A');
  });
}finally{
  await browser.close();
  const result={stage,url,checks,real_shared_React_component:true,real_PNG_decode_and_Canvas_pixels:true,
    bytes_port:'test-only exact AssetRef HTTP provider',actual_mode:'existing real Engine-derived frames; not Host or Native',
    original_21_Runs_or_expected_values_modified:false,Host_or_listener_started:false,Native_operated:false};
  await writeFile(new URL(`browser-${stage}.json`,out),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify(checks.map(({name,status,error})=>({name,status,error})),null,2));
  if(checks.some(value=>value.status!=='passed'))process.exitCode=1;
}
