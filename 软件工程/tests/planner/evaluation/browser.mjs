// Offline finite consumer. Actual components, fixed service ports, no listener/Host/native execution.
import {createRequire} from 'node:module';
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/方案比较/runtime-delivery-ui/',root),build=new URL('fixture-build/',out);
const data=await readFile(new URL('data.json',build),'utf8'),js=await readFile(new URL('fixture.js',build),'utf8');
let css='';for(const file of await readdir(build))if(file.endsWith('.css'))css+=await readFile(new URL(file,build),'utf8');
const html=`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>超级工厂 · 交付组件有限消费者</title><style>${css}\n.finite-fixture-banner{height:30px;background:#fff1ce;color:#674715;padding:3px 12px;font-size:12px}</style></head><body><div id="root"></div><script>window.deliveryFixture=${data.replace(/</g,'\\u003c')}</script><script>${js.replace(/<\/script/gi,'<\\/script')}</script></body></html>`;
await writeFile(new URL('finite-component-consumer.html',out),html);
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const page=await browser.newPage({viewport:{width:1440,height:900},locale:'zh-CN'});
const errors=[],results=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
await page.route('**/*',route=>route.abort());
async function check(name,work){try{await work();results.push({name,passed:true});}catch(e){results.push({name,passed:false,error:String(e)});await page.screenshot({path:fileURLToPath(new URL('failure.png',out))});}}
async function reset(mode='success') {await page.evaluate(mode=>{window.deliveryFixture.mode=mode;window.deliveryFixture.calls=[];window.deliveryFixture.reset();},mode);await page.getByRole('heading',{name:'方案评估与交接',exact:true}).waitFor();}
const tab=name=>page.getByRole('navigation',{name:'评估与交付任务'}).getByRole('button',{name,exact:true}).click();
async function selectRuns(n){for(let i=0;i<n;i++)await page.locator('.delivery-selection .delivery-run-choice input').nth(i).check();await page.waitForFunction(()=>!document.querySelector('.delivery-selection [role=status]'));}
async function completed(){await page.getByText('宿主任务完成，已读取实际结果。',{exact:true}).waitFor();}
const screenshot=name=>page.screenshot({path:fileURLToPath(new URL(name,out)),fullPage:false});
try {
 await page.setContent(html,{waitUntil:'load'});
 await page.getByRole('heading',{name:'方案评估与交接',exact:true}).waitFor();
 await check('actual_component_nonblank_offline_and_no_invented_initial_runs',async()=>{
  assert.equal(await page.title(),'超级工厂 · 交付组件有限消费者');
  assert.equal(await page.getByRole('button',{name:'比较所选两次运行',exact:true}).isDisabled(),true);
  assert.ok((await page.locator('body').innerText()).includes('人工固定服务响应'));
  assert.equal(await page.locator('vite-error-overlay').count(),0);
 });
 await check('two_Run_comparison_uses_canonical_service_and_real_recorded_changes_metrics_and_locate_event',async()=>{
  await selectRuns(2);await page.getByRole('button',{name:'比较所选两次运行',exact:true}).click();
  await page.getByRole('heading',{name:'宿主计算的同口径指标'}).waitFor();
  const calls=await page.evaluate(()=>window.deliveryFixture.calls);const call=calls.find(x=>x.method==='compare');
  assert.equal(call.params.run_ids.length,2);assert.deepEqual(call.params.metric_scope.keys,['accepted_quantity','completion_time','waiting_time']);
  await page.getByRole('button',{name:'候选定位',exact:true}).first().click();
  assert.equal((await page.evaluate(()=>window.deliveryFixture.calls)).at(-1).params.run,call.params.run_ids[1]);
  await screenshot('comparison-desktop.png');
 });
 await check('frozen_initial_scenario_request_keeps_old_Run_and_user_specified_new_identity',async()=>{
  await page.getByLabel('新方案ID',{exact:true}).fill('UI-NEW');await page.getByLabel('新方案标题',{exact:true}).fill('供料修正候选');
  await page.getByRole('button',{name:'创建方案草稿',exact:true}).click();
  await page.getByText(/已创建方案 UI-NEW/).waitFor();
  const c=(await page.evaluate(()=>window.deliveryFixture.calls)).find(x=>x.method==='clone');
  assert.equal(c.params.new_scenario_id,'UI-NEW');assert.equal(c.params.source_run_id,(await page.evaluate(()=>window.deliveryFixture.evidence))[0].run.id);
 });
 await check('comparison_errors_keep_field_detail_and_never_attach_to_changed_Run_selection',async()=>{
  await reset('compare_fail');await selectRuns(2);await page.getByRole('button',{name:'比较所选两次运行',exact:true}).click();
  await page.getByRole('alert').filter({hasText:'有限服务比较错误'}).waitFor();
  await page.getByText('比较服务原始错误与字段定位',{exact:true}).click();
  assert.ok((await page.locator('.delivery-main').innerText()).includes('/metric_scope'));
  await page.locator('.delivery-selection .delivery-run-choice input').nth(1).uncheck();
  await page.waitForFunction(()=>!document.querySelector('.delivery-selection [role=status]'));
  assert.equal(await page.getByRole('alert').filter({hasText:'有限服务比较错误'}).count(),0);
 });
 await check('incomparable_pair_never_displays_an_improvement_percentage',async()=>{
  await reset('incomparable');await selectRuns(2);await page.getByRole('button',{name:'比较所选两次运行',exact:true}).click();
  await page.getByRole('heading',{name:'可比性：仅并列，不作改善结论',exact:true}).waitFor();
  assert.ok((await page.locator('.delivery-main').innerText()).includes('不可比，不显示改善量'));
  assert.ok(!(await page.locator('.delivery-main').innerText()).includes('相对变化 7%'));
 });
 await check('single_Run_report_all_four_formats_waits_real_port_terminal_and_tab_fields_persist',async()=>{
  await reset();await selectRuns(1);await tab('评估报告');
  await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();await completed();
  const c=(await page.evaluate(()=>window.deliveryFixture.calls)).find(x=>x.method==='evaluation');
  assert.equal(c.params.run_refs.length,1);assert.equal(c.params.report_mode,'simulation_evaluation');assert.deepEqual(c.params.formats,['pdf','html','csv','json']);
  assert.equal(await page.getByText('finite/report.pdf',{exact:true}).count(),1);
  const waits=page.locator('table').filter({has:page.locator('caption',{hasText:'等待与对象定位'})});
  if(await waits.count()){
   await waits.first().getByRole('button').first().click();
   const location=(await page.evaluate(()=>window.deliveryFixture.calls)).at(-1);
   assert.equal(location.method,'locate');assert.equal(location.params.run,c.params.run_refs[0]);
  }
  await screenshot('report-single.png');
  await page.getByRole('radio',{name:'尚未运行的设计说明',exact:true}).check();await tab('方案比较');await tab('评估报告');
  assert.equal(await page.getByRole('radio',{name:'尚未运行的设计说明',exact:true}).isChecked(),true);
 });
 await check('no_Run_design_export_transmits_no_Run_and_does_not_claim_actual_production',async()=>{
  await reset();await tab('评估报告');await page.getByRole('radio',{name:'尚未运行的设计说明',exact:true}).check();
  await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();await completed();
  const c=(await page.evaluate(()=>window.deliveryFixture.calls)).find(x=>x.method==='evaluation');assert.deepEqual(c.params.run_refs,[]);assert.equal(c.params.report_mode,'design_description');
  assert.ok((await page.getByRole('region',{name:'实际评估结果'}).innerText()).includes('仅设计说明'));
  await screenshot('report-design.png');
 });
 await check('accepted_cancellation_waits_cancelled_terminal_without_file_success',async()=>{
  await reset('hold');await selectRuns(1);await tab('评估报告');
  await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();await page.getByRole('button',{name:'请求取消',exact:true}).click();
  await page.getByText('宿主确认任务已取消；不表示文件已导出。',{exact:true}).waitFor();
  assert.equal(await page.getByText('上次任务返回的实际文件',{exact:true}).count(),0);
  await screenshot('task-cancelled.png');
 });
 await check('picker_cancellation_and_failed_task_remain_distinct_from_success',async()=>{
  await reset('picker_cancel');await selectRuns(1);await tab('评估报告');await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();
  await page.getByText('未选择目标，没有启动导出或保存任务。',{exact:true}).waitFor();
  await reset('fail');await selectRuns(1);await tab('评估报告');await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();
  await page.getByRole('alert').filter({hasText:'缺少许可允许的附件'}).first().waitFor();
  assert.equal(await page.getByText('上次任务返回的实际文件',{exact:true}).count(),0);
 });
 await check('read_failure_retries_original_task_without_duplicate_export_or_losing_error_detail',async()=>{
  await reset('read_fail_once');await selectRuns(1);await tab('评估报告');await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();
  await page.getByRole('alert').filter({hasText:'有限服务读取中断'}).waitFor();
  await page.getByText('服务原始错误与字段定位',{exact:true}).click();assert.ok((await page.locator('.delivery-task').innerText()).includes('IoFailure'));
  await page.getByRole('button',{name:'重新读取原任务',exact:true}).click();await completed();
  assert.equal((await page.evaluate(()=>window.deliveryFixture.calls)).filter(x=>x.method==='evaluation').length,1);
 });
 await check('completed_task_without_actual_file_list_is_not_export_success',async()=>{
  await reset('empty_result');await selectRuns(1);await tab('评估报告');await page.getByRole('button',{name:'选择目录并生成报告',exact:true}).click();
  await page.getByRole('alert').filter({hasText:'未返回对应的实际文件结果'}).waitFor();assert.equal(await page.getByText('上次任务返回的实际文件',{exact:true}).count(),0);
 });
 await check('directory_save_displays_saved_revision_and_remaining_dirty_state',async()=>{
  await reset();await tab('可编辑交接与贡献');await page.getByRole('button',{name:'另选目录保存工程',exact:true}).click();await completed();
  await page.getByRole('region',{name:'实际保存回执',exact:true}).waitFor();assert.ok((await page.getByRole('region',{name:'实际保存回执',exact:true}).innerText()).includes('工程仍未全部保存'));
 });
 await check('project_ZIP_selection_keeps_canonical_purpose_references_and_missing_rights',async()=>{
  await reset();await tab('可编辑交接与贡献');await page.getByRole('button',{name:'选择位置并导出ZIP包',exact:true}).click();await completed();
  const c=(await page.evaluate(()=>window.deliveryFixture.calls)).find(x=>x.method==='package');assert.equal(c.params.package_scope.purpose,'handoff_project');assert.equal(c.params.package_scope.include_redistributable_assets,false);
  assert.ok(c.params.package_scope.module_refs.length>0);assert.deepEqual(c.params.package_scope.run_ids,[]);
  await page.getByText('finite/project.zip',{exact:true}).waitFor();await page.getByText(/素材许可未知/).waitFor();
  await page.getByRole('button',{name:'在另一位置打开并检查',exact:true}).click();assert.equal((await page.evaluate(()=>window.deliveryFixture.calls)).at(-1).method,'open');
  await page.locator('.delivery-select select').selectOption('community_module');
  const modules=page.locator('.delivery-module input');for(let i=1;i<await modules.count();i++)await modules.nth(i).uncheck();
  assert.equal(await page.getByRole('button',{name:'选择位置并导出ZIP包',exact:true}).isDisabled(),true);
  await page.getByRole('alert').filter({hasText:'作者、代码许可或实际定义'}).waitFor();
  await screenshot('handoff-rights.png');
 });
 await check('compact_desktop_keeps_controls_readable_and_body_without_horizontal_overflow',async()=>{
  await reset();await page.setViewportSize({width:960,height:720});await selectRuns(2);await page.getByRole('button',{name:'比较所选两次运行',exact:true}).click();await page.getByRole('heading',{name:'宿主计算的同口径指标'}).waitFor();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await screenshot('comparison-compact.png');
 });
 assert.deepEqual(errors,[]);
} finally {
 await writeFile(new URL('browser-results.json',out),JSON.stringify({kind:'actual_product_components_with_finite_service_ports',no_Host_or_Native:true,actual_product_Host:false,new_listener_started:false,network_requests_allowed:false,results,console_errors:errors},null,2));
 await browser.close();
}
console.log(JSON.stringify(results));if(results.some(r=>!r.passed)||errors.length)process.exitCode=1;
