// Actual browser preview and actual-component evidence, never a substitute Host.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url);
const {chromium}=require(process.argv[2]);
const root=new URL('../../../',import.meta.url);
const out=new URL('.work/whole-packages/window5-planner-acceptance/browser/',root);await mkdir(out,{recursive:true});
const groups=[],errors=[];
async function check(name,work){try{await work();groups.push({name,passed:true});}catch(error){groups.push({name,passed:false,failure:String(error)});}}
const browser=await chromium.launch({headless:true});
const context=await browser.newContext({viewport:{width:1440,height:900},locale:'zh-CN'});
const page=await context.newPage();page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const base='http://127.0.0.1:8922';
const snapshot=async()=>{await writeFile(new URL('preview-dom.txt',out),await page.locator('body').innerText());};
const menu=async(name,button)=>{await page.getByText(name,{exact:true}).click();await page.locator('nav details[open]').getByRole('button',{name:button}).click();};
const caption=async text=>page.waitForFunction(value=>document.querySelector('.scene-caption')?.textContent.includes(value),text);
const close=(a,b)=>{assert.equal(a.length,b.length);a.forEach((x,i)=>assert.ok(Math.abs(x-b[i])<1e-7,`${a} != ${b}`));};
const vecDistance=(a,b)=>Math.hypot(...a.map((v,i)=>v-b[i]));
try{
 await page.goto(base);await page.getByRole('main').waitFor();await snapshot();
 await check('actual_preview_identity_nonblank_no_overlay_and_real_canvas',async()=>{
  assert.equal(await page.title(),'超级工厂 · 工业工作台');assert.equal(page.url(),base+'/');
  assert.ok((await page.locator('body').innerText()).includes('浏览器预览'));assert.equal(await page.locator('vite-error-overlay').count(),0);
  await page.locator('canvas').waitFor();assert.equal(await page.getByRole('alert').count(),0);
 });
 await check('actual_preview_has_no_native_substitute_and_sdk_refuses_business_dispatch',async()=>{
  assert.equal(await page.evaluate(()=>('__TAURI_INTERNALS__' in window)),false);
  await page.getByText('文件',{exact:true}).click();assert.equal(await page.getByRole('button',{name:'新建工程…',exact:true}).isDisabled(),true);await page.getByText('文件',{exact:true}).click();
  const failure=await page.evaluate(async()=>{const host=await import('/src/services/projectHost.ts');try{await host.dispatch({api_version:'0.1',request_id:'preview-native-boundary',project_id:null,scope:{kind:'workspace'},operation:'get_context',params:{selection:[]}});return null;}catch(error){return String(error);}});
  assert.match(failure,/桌面应用/);
 });
 await check('actual_preview_two_canvases_and_three_manual_lods_change_only_view',async()=>{
  await page.getByRole('button',{name:'多厂房画布',exact:true}).click();assert.match(await page.locator('.scene-caption').innerText(),/多厂房画布/);
  await menu('视图',/^简化$/);await caption('简化');assert.match(await page.locator('.scene-caption').innerText(),/简化/);
  await menu('视图',/^区域$/);await caption('区域');assert.match(await page.locator('.scene-caption').innerText(),/区域/);
  await menu('视图',/^详细$/);await page.getByRole('button',{name:'流水线画布',exact:true}).click();
  assert.match(await page.locator('.scene-caption').innerText(),/流水线画布/);
 });
 await check('actual_preview_independent_docks_collapse_resize_and_restore_160px',async()=>{
  assert.equal(Math.round(await page.locator('.bottom-dock').evaluate(n=>n.getBoundingClientRect().height)),160);
  await menu('窗口',/事件描述$/);await page.locator('.description').waitFor({state:'detached'});assert.equal(await page.locator('.description').count(),0);assert.equal(await page.getByRole('region',{name:'全厂时间轴'}).count(),1);
  assert.equal(await page.locator('.timeline').count(),1);await menu('窗口',/时间轴$/);assert.equal(await page.locator('.bottom-dock').count(),0);
  await menu('窗口',/恢复默认布局/);assert.equal(await page.locator('.description').count(),1);assert.equal(await page.locator('.timeline').count(),1);
  const handle=page.getByRole('separator',{name:'对象树宽度'});const box=await handle.boundingBox();const before=await page.locator('.object-tree').evaluate(n=>n.getBoundingClientRect().width);
  await page.mouse.move(box.x+2,box.y+40);await page.mouse.down();await page.mouse.move(box.x+42,box.y+40);await page.mouse.up();
  assert.ok(await page.locator('.object-tree').evaluate(n=>n.getBoundingClientRect().width)>before+20);
  await menu('窗口',/恢复默认布局/);assert.equal(Math.round(await page.locator('.bottom-dock').evaluate(n=>n.getBoundingClientRect().height)),160);
 });
 await page.screenshot({path:fileURLToPath(new URL('preview-desktop.png',out))});
 await check('actual_preview_mobile_visible_controls_and_no_document_overflow',async()=>{
  await page.setViewportSize({width:390,height:844});await page.reload();await page.getByRole('main').waitFor();
  assert.equal(await page.getByRole('button',{name:'流水线画布',exact:true}).isVisible(),true);
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
  await page.screenshot({path:fileURLToPath(new URL('preview-mobile.png',out))});
 });
 await page.setViewportSize({width:1440,height:900});
 const componentPath=fileURLToPath(new URL('tests/planner/acceptance/component.html',root)).replaceAll('\\','/');
 const componentURL=base+'/@fs/'+componentPath;
 await page.goto(componentURL);await page.locator('canvas').waitFor();await page.waitForFunction(()=>window.__acceptance?.read().items.length>0);
 await writeFile(new URL('component-dom.txt',out),await page.locator('body').innerText());
 const read=()=>page.evaluate(()=>window.__acceptance.read());
 const item=(state,id)=>state.items.find(x=>x.ref.id===id);
 await check('actual_scene_line_then_hall_then_world_transform_matches_independent_coordinates',async()=>{
  close(item(await read(),'Assembly01').world,[7,5,-22]);await page.getByRole('button',{name:'跨厂',exact:true}).click();
  await page.waitForFunction(()=>Math.abs(window.__acceptance.read().items.find(x=>x.ref.id==='Assembly01').world[0]-78)<1e-7);
  close(item(await read(),'Assembly01').world,[78,10,-207]);await page.getByRole('button',{name:'线内',exact:true}).click();
 });
 await check('actual_scene_keeps_concave_geometry_and_unknown_height_flat',async()=>{
  const state=await read();close(item(state,'Assembly01').box,[6,6,0]);close(item(state,'Inspection01').box,[6,6,2]);
  assert.equal(await page.evaluate(()=>window.__acceptance.area('Assembly01')),20);
 });
 await check('actual_scene_group_root_move_applies_once_and_keeps_member_local_pose',async()=>{
  const before=JSON.parse((await read()).input).devices[0].pose;
  await page.evaluate(()=>window.__acceptance.moveGroup());await page.waitForFunction(()=>Math.abs(window.__acceptance.read().items.find(x=>x.ref.id==='Assembly01').world[0]-15)<1e-7);
  const state=await read();close(item(state,'Assembly01').world,[15,7,-18]);assert.deepEqual(JSON.parse(state.input).devices[0].pose,before);
 });
 async function drag(button){const b=await page.locator('canvas').boundingBox();await page.mouse.move(b.x+b.width*.55,b.y+b.height*.5);await page.mouse.down({button});await page.mouse.move(b.x+b.width*.55+90,b.y+b.height*.5+25,{steps:8});await page.mouse.up({button});}
 await check('actual_scene_right_drag_orbits_as_frozen_Q07_requires',async()=>{
  const before=await read();await drag('right');const after=await read();
  await writeFile(new URL('right-drag-state.json',out),JSON.stringify({before,after},null,2));
  assert.ok(vecDistance(before.camera,after.camera)>1e-5);assert.ok(vecDistance(before.target,after.target)<1e-7,'right drag changed target: actual pan, expected orbit');
 });
 await check('actual_scene_middle_drag_pans',async()=>{
  const before=await read();await drag('middle');const after=await read();assert.ok(vecDistance(before.target,after.target)>1e-5);
  close(after.camera.map((v,i)=>v-before.camera[i]),after.target.map((v,i)=>v-before.target[i]));
 });
 await check('actual_scene_wheel_manual_lod_and_reset_preserve_input_graph',async()=>{
  const before=await read();const b=await page.locator('canvas').boundingBox();await page.mouse.move(b.x+b.width/2,b.y+b.height/2);await page.mouse.wheel(0,-250);
  await page.waitForFunction(previous=>Math.abs(Math.hypot(...window.__acceptance.read().camera)-previous)>1e-4,Math.hypot(...before.camera));
  for(const name of ['region','simplified','detailed']){await page.getByRole('button',{name,exact:true}).click();await page.waitForFunction(()=>window.__acceptance.read().items.length>0);assert.equal((await read()).input,before.input);}
  await page.getByRole('button',{name:'复位',exact:true}).click();assert.equal((await read()).input,before.input);
 });
 await check('actual_scene_click_reports_stable_device_identity',async()=>{
  await page.waitForFunction(()=>window.__acceptance?.read().items.find(x=>x.ref.id==='Assembly01')?.visible);
  const p=await page.evaluate(()=>window.__acceptance.screenPoint('Assembly01'));await page.mouse.click(p.x,p.y);
  await page.waitForFunction(()=>window.__acceptance.read().selected?.id==='Assembly01');
  assert.deepEqual((await read()).selected,{kind:'device',id:'Assembly01'});
 });
 await page.screenshot({path:fileURLToPath(new URL('actual-component-detailed.png',out))});
 await page.goto(componentURL+'?mode=timeline');await page.getByRole('slider',{name:'观察时点'}).waitFor();
 await check('actual_timeline_uses_real_cancelled_prefix_frontier_and_never_full_window',async()=>{
  const fixture=JSON.parse(await readFile(new URL('packages/commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json',root),'utf8')).quality_partial[3];
  const slider=page.getByRole('slider',{name:'观察时点'});assert.equal(Number(await slider.getAttribute('max')),fixture.computed_until_ms);assert.ok(fixture.computed_until_ms<fixture.window.end_ms);
  assert.ok(await page.locator('.phase-bar').count()>0);
 });
 await page.screenshot({path:fileURLToPath(new URL('actual-component-prefix-timeline.png',out))});
 await check('actual_browser_console_health_no_framework_overlay',async()=>{assert.deepEqual(errors,[]);assert.equal(await page.locator('vite-error-overlay').count(),0);});
}catch(error){groups.push({name:'remaining_browser_sequence',passed:false,failure:String(error)});}
finally{
 await writeFile(new URL('result.json',out),JSON.stringify({url:base,viewports:[[1440,900],[390,844]],browser_plugin:'not available',runtime:'installed Playwright1.61.0 Chromium1228',scope:'actual browser preview plus isolated production components with passive observation; no mock Tauri/Host and no native UI claim',groups,errors,whole_task_done:false,human_acceptance:false},null,2));
 await browser.close();
}
for(const group of groups)console.log(JSON.stringify(group));
process.exitCode=groups.every(x=>x.passed)?0:1;
