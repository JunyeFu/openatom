// Independent explicit readonly timeline contract: real frozen Host frame, real pointer events.
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import assert from 'node:assert/strict';
if(!process.argv.includes('--root-consumer-frozen'))throw Error('Root frozen cut required');
const root=new URL('../../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/repairs-01/',root);
const index=JSON.parse(await readFile(new URL('scenarios/Agent共建/runtime-public-render-host/actual-inputs-index.json',root),'utf8'));
const directory=index.captures[44].directory;
const task={frame:JSON.parse(await readFile(new URL(directory+'/frame.json',root),'utf8')),input:JSON.parse(await readFile(new URL(directory+'/input.json',root),'utf8'))};
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[],warnings=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
const stage=process.argv[2]??'green';const result={source:directory,expectation:'Readonly task timeline rejects drag changes, playback/edit/seek and localStorage operations; original Task/view remains unchanged.',checks:[],Native:false};
try{
 await page.route('**/__render_task_fixture',r=>r.fulfill({status:200,contentType:'application/json',body:JSON.stringify(task)}));
 await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/public-render-consumer/consumer.html');
 await page.waitForFunction(()=>window.__renderConsumer?.read().ready.length||window.__renderConsumer?.read().failures.length);
 const before=await page.evaluate(()=>window.__renderConsumer.read());assert.equal(before.ready.length,1);assert.deepEqual(before.failures,[]);
 const chart=await page.locator('.sf-tl-chart').first().boundingBox(),ruler=await page.locator('.sf-tl-ruler-axis').boundingBox();assert.ok(chart&&ruler);
 await page.mouse.move(chart.x+chart.width*.2,chart.y+chart.height*.65);await page.mouse.down();await page.mouse.move(chart.x+chart.width*.6,chart.y+chart.height*.65,{steps:8});await page.mouse.up();
 const rangeText=await page.locator('.sf-tl-status').innerText();
 await page.mouse.move(ruler.x+ruler.width*.3,ruler.y+8);await page.mouse.down();await page.mouse.move(ruler.x+ruler.width*.4,ruler.y+8,{steps:5});await page.mouse.up();
 const after=await page.evaluate(()=>window.__renderConsumer.read());
 result.observed={rangeText,ghosts:await page.locator('.sf-tl-ghost').count(),afterStatus:await page.locator('.sf-tl-status').innerText(),stored:after.stored,calls:after.calls};
 const record=(name,passed)=>{result.checks.push({name,status:passed?'passed':'failed'});if(!passed)process.exitCode=1;};
 record('Readonly timeline does not start a drag selection or ghost seek',!rangeText.includes('选段')&&result.observed.ghosts===0);
 record('Task/view/source and read-only business boundary remain unchanged',JSON.stringify(after.ready)===JSON.stringify(before.ready)&&after.stored.length===0&&after.calls.every(c=>['native_render_ready','native_render_failure'].includes(c.command))&&after.failures.length===0);
 assert.deepEqual(errors,[]);assert.deepEqual(warnings,[]);
 await page.screenshot({path:fileURLToPath(new URL('readonly-input-'+stage+'-timeline.png',out))});
}catch(error){result.error=String(error);process.exitCode=1;}finally{result.errors=errors;result.warnings=warnings;result.passed=result.checks.length===2&&result.checks.every(c=>c.status==='passed')&&!result.error;await page.close();await browser.close();await writeFile(new URL('readonly-input-'+stage+'.json',out),JSON.stringify(result,null,2));console.log(JSON.stringify({passed:result.passed,checks:result.checks,error:result.error}));}
