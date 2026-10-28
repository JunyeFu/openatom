import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createRequire} from 'node:module';
import {createInterface} from 'node:readline';
import {mkdir,readFile,writeFile,cp} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const {chromium}=require('playwright');
const root=fileURLToPath(new URL('../../../',import.meta.url)),stage=process.argv[2]??'red';
const out=path.join(root,'scenarios/自定义模块/runtime-root-navigation');
await mkdir(out,{recursive:true});
const results=[],browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
async function check(name,kind,action){
 const sandbox=path.join(out,stage+'-'+kind+'-'+results.length);
 const fixed='scenarios/图片时间轴/runtime/fixed.input.json';
 await mkdir(path.join(sandbox,path.dirname(fixed)),{recursive:true});
 await cp(path.join(root,fixed),path.join(sandbox,fixed));
 const modules='modules/devices/manual-assembly/presentation/p303-owned';
 await mkdir(path.join(sandbox,path.dirname(modules)),{recursive:true});
 await cp(path.join(root,modules),path.join(sandbox,modules),{recursive:true});
 const child=spawn(path.join(root,'scenarios/图片时间轴/runtime/core_consumer.exe'),[sandbox],{windowsHide:true,stdio:['pipe','pipe','pipe']});
 const queue=[],calls=[],errors=[];let stderr='',serial=Promise.resolve(),readyResolve,readyReject;
 const started=new Promise((resolve,reject)=>{readyResolve=resolve;readyReject=reject;});
 child.stderr.on('data',chunk=>stderr+=chunk);
 child.on('exit',code=>{if(code){const error=new Error(stderr);readyReject(error);for(const q of queue.splice(0))q.reject(error);}});
 createInterface({input:child.stdout}).on('line',line=>{const value=JSON.parse(line);if(value.ready)readyResolve(value);else queue.shift()?.resolve(value);});
 const rpc=query=>{const next=serial.then(()=>new Promise((resolve,reject)=>{queue.push({resolve:value=>{calls.push({query,value});resolve(value);},reject});child.stdin.write(JSON.stringify(query)+'\n');}));serial=next;return next;};
 const ready=await started;
 let draft=ready.draft;
 if(kind==='professional'){
  const pkg=JSON.parse(await readFile(path.join(root,'modules/devices/template-support/professional/module.json'),'utf8'));
  const reply=await rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'root-navigation-new-professional',project_id:ready.snapshot.project.id,scope:{kind:'project'},operation:'apply_commands',params:{base_revision:ready.snapshot.project.revision,commands:[{kind:'create_definition_draft',data:pkg.data.definition}]}}});
  assert.equal(reply.outcome.status,'ok',JSON.stringify(reply));
  draft={summary:{draft_id:reply.outcome.data.scope.data.id},definition:pkg.data.definition};
 }
 const page=await browser.newPage({viewport:{width:1700,height:1100}});page.setDefaultTimeout(12000);page.on('pageerror',error=>errors.push(String(error)));
 await page.route('**/__root_navigation_core',async route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(await rpc(route.request().postDataJSON()))}));
 try{
  await page.goto('http://127.0.0.1:8922/@fs/'+root.replaceAll('\\','/')+'/tests/planner/root-authoring-navigation/harness.html');
  const modulesMenu=page.locator('details').filter({has:page.locator('summary').filter({hasText:/^模块$/})});
  await modulesMenu.locator('summary').click();
  await modulesMenu.getByRole('button',{name:'继续定义草稿 · '+draft.summary.draft_id,exact:true}).click();
  const authoring=page.getByRole('region',{name:'九类定义创作'});await authoring.waitFor();
  const title=authoring.getByRole('textbox',{name:'中文名称',exact:true});
  assert.equal(await title.inputValue(),draft.definition.title);
  await title.fill('Root 未保存的精确输入 '+kind);
  const menu=page.locator('details').filter({has:page.locator('summary').filter({hasText:kind==='professional'?/^专业工程$/:/^设备过程$/})});
  await menu.locator('summary').click();
  await menu.getByRole('button',{name:kind==='professional'?'在当前专业定义草稿中计算…':'当前定义的图片过程与标签…',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'未保存输入',exact:true});
  await dialog.waitFor();
  await action({page,authoring,title,dialog,ready,draft,rpc});
  const original=await rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'root-old-run-check',project_id:ready.snapshot.project.id,scope:{kind:'project'},operation:'get_run',params:{run_id:ready.formal_run.id,query:{kind:'full_result'}}}});
  assert.equal(original.outcome.status,'ok');assert.deepEqual(original.outcome.data.data,ready.formal_run);
  assert.deepEqual(errors,[]);results.push({name,status:'passed',kind,calls,errors,stderr});console.log('PASS '+name);
 }catch(error){results.push({name,status:'failed',kind,error:String(error),stack:error.stack,calls,errors,stderr});process.exitCode=1;console.log('FAIL '+name+': '+error);}
 finally{await page.screenshot({path:path.join(out,stage+'-'+results.length+'.png'),fullPage:true});await page.close();child.stdin.end();}
}
try{
 await check('Root device-process menu must preserve unsaved authoring on cancel and explicitly discard only local input','device',async({page,authoring,title,dialog})=>{
  await dialog.getByRole('button',{name:'继续编辑',exact:true}).click();assert.equal(await title.inputValue(),'Root 未保存的精确输入 device');assert.equal(await authoring.count(),1);
  await page.locator('details').filter({has:page.locator('summary').filter({hasText:/^设备过程$/})}).locator('summary').click();
  await page.getByRole('button',{name:'当前定义的图片过程与标签…',exact:true}).click();
  await dialog.getByRole('button',{name:'放弃未保存输入后继续',exact:true}).click();
  await page.getByRole('region',{name:'设备过程工作区'}).waitFor();assert.equal(await authoring.count(),0);
 });
 await check('Root professional-Draft menu uses the same actual authoring departure confirmation','professional',async({page,title,dialog})=>{
  await dialog.getByRole('button',{name:'继续编辑',exact:true}).click();assert.equal(await title.inputValue(),'Root 未保存的精确输入 professional');
  await page.locator('details').filter({has:page.locator('summary').filter({hasText:/^专业工程$/})}).locator('summary').click();
  await page.getByRole('button',{name:'在当前专业定义草稿中计算…',exact:true}).click();
  await dialog.getByRole('button',{name:'放弃未保存输入后继续',exact:true}).click();
  await page.getByRole('heading',{name:'纸包装 · 结构与过程',exact:true}).waitFor();
 });
 await check('Root device-process save-and-continue writes one actual definition revision before navigation','device',async({page,dialog,rpc,ready,draft})=>{
  await dialog.getByRole('button',{name:'保存后继续',exact:true}).click();await page.getByRole('region',{name:'设备过程工作区'}).waitFor();
  const result=await rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'root-saved-draft-check',project_id:ready.snapshot.project.id,scope:{kind:'definition_draft',data:draft.summary.draft_id},operation:'get_object',params:{object_id:{kind:'definition_draft',id:draft.summary.draft_id},source:{kind:'draft'},node_index:null}}});
  assert.equal(result.outcome.status,'ok');assert.equal(result.outcome.data.data.title,'Root 未保存的精确输入 device');
 });
}finally{await browser.close();await writeFile(path.join(out,stage+'.json'),JSON.stringify({stage,checks:results,actual_Workbench:true,actual_ModuleAuthoringWorkspace:true,actual_original_Core:true,Native_IPC:false,transport:'private HTTP to preexisting frozen Core binary in isolated copied fixture root'},null,2));}
