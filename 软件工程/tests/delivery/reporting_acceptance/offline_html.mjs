// Read actual exported HTML offline after moving the complete output directory.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,writeFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
const require=createRequire(import.meta.url);const {chromium}=require(process.argv[2]);
const root=new URL('../../../',import.meta.url);const out=new URL('.work/whole-packages/window5-timeline-evaluation/',root);
const hostOutput=process.argv.includes('--host');
const prior=JSON.parse(await readFile(new URL(hostOutput?'host-output-readback.json':'independent-output-result.json',out),'utf8'));
const browser=await chromium.launch({headless:true});const context=await browser.newContext({offline:true,viewport:{width:1440,height:1000}});
const results=[];
try{for(const name of hostOutput?['single','design']:['fractional','design']){
 const page=await context.newPage();const errors=[],network=[];page.on('pageerror',e=>errors.push(String(e)));page.on('request',r=>{if(/^https?:/.test(r.url()))network.push(r.url());});
 try{
  const directory=path.join(prior.moved_directory,name);await page.goto(pathToFileURL(path.join(directory,'index.html')).href);
  await page.getByRole('heading',{name:'超级工厂 · 工程评估报告',exact:true}).waitFor();
  const report=JSON.parse(await readFile(path.join(directory,'report.json'),'utf8'));
  assert.deepEqual(await page.evaluate(()=>JSON.parse(document.getElementById('report-data').textContent)),report);
  assert.equal(await page.evaluate(()=>window.__w5_injected),undefined);
  const before=await page.locator('.data-row:visible').count();await page.getByRole('searchbox').fill(name==='fractional'?'fractional-cartons':name==='single'?'frozen-carton':'__no_matching_saved_record__');
  const after=await page.locator('.data-row:visible').count();assert.ok(after<before);
  assert.deepEqual(await page.evaluate(()=>JSON.parse(document.getElementById('report-data').textContent)),report);
  assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
  await page.getByRole('searchbox').fill('');await page.screenshot({path:path.join(directory,'offline-html.png')});
  results.push({name:name+'_actual_offline_html_filter_same_data_no_script_injection',passed:true,url:page.url(),rows_before:before,rows_filtered:after});
 }catch(error){results.push({name,passed:false,failure:String(error),errors,network});}finally{await page.close();}
}}finally{await browser.close();}
await writeFile(new URL(hostOutput?'host-offline-browser-result.json':'offline-browser-result.json',out),JSON.stringify({runtime:'installed Playwright1.61.0/Chromium1228; actual file URLs; offline context',moved_directory:prior.moved_directory,results,whole_task_done:false,human_acceptance:false},null,2));
for(const result of results)console.log(JSON.stringify(result));process.exitCode=results.every(r=>r.passed)?0:1;
