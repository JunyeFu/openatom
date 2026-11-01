import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,writeFile} from 'node:fs/promises';
import {pathToFileURL,fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url),{chromium}=require(process.argv[2]);
const out=new URL('../../../.work/whole-packages/window5-timeline-evaluation/native-repair-79676/',import.meta.url);
const prior=JSON.parse(await readFile(new URL('file-readback.json',out),'utf8'));
const report=JSON.parse(await readFile(new URL('moved-report/report.json',out),'utf8'));
const browser=await chromium.launch({headless:true});
const context=await browser.newContext({offline:true,viewport:{width:1440,height:1000}});
const page=await context.newPage(),errors=[],network=[];
page.on('pageerror',e=>errors.push(String(e)));
page.on('request',r=>{if(/^https?:/.test(r.url()))network.push(r.url());});
try {
 await page.goto(pathToFileURL(prior.moved_report_directory+'/index.html').href);
 await page.getByRole('heading',{name:'超级工厂 · 工程评估报告',exact:true}).waitFor();
 assert.deepEqual(await page.evaluate(()=>JSON.parse(document.getElementById('report-data').textContent)),report);
 const before=await page.locator('.data-row:visible').count();
 await page.getByRole('searchbox').fill('__no_matching_saved_record__');
 assert.equal(await page.locator('.data-row:visible').count(),0);
 assert.ok(before>0);
 assert.deepEqual(await page.evaluate(()=>JSON.parse(document.getElementById('report-data').textContent)),report);
 assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
 await page.getByRole('searchbox').fill('');
 await page.screenshot({path:fileURLToPath(new URL('offline-report.png',out))});
 await writeFile(new URL('offline-readback.json',out),JSON.stringify({
  passed:true,actual_moved_file_url:page.url(),offline:true,network_requests:network,
  browser_errors:errors,rows_before:before,rows_filtered:0,same_immutable_data:true,
  whole_task_done:false,human_acceptance:false
 },null,2));
 console.log(JSON.stringify({passed:true,rows_before:before,rows_filtered:0,offline_no_network:true}));
} finally {await browser.close();}
