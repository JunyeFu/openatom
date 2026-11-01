import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { readFile,writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { root,out } from './current-core.mjs';
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
const fixture=JSON.parse(await readFile(new URL('tests/planner/p201-whole/fixtures/b60/balanced.input.json',root),'utf8'));
const result={kind:'controlled_checker_saved_scope_fields_regression',actual_Host:false,Native_app:false,cases:[],errors:[]};
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
try {
  for(const single of [false,true]) {
    const page=await browser.newPage({viewport:{width:1280,height:900}});page.on('pageerror',error=>result.errors.push(String(error)));
    await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/p201-whole/checker-scopes.html'+(single?'?single':''));
    const form=page.getByRole('form',{name:'工程草稿检查',exact:true});await form.waitFor();const scopeView=form.getByLabel('已保存的附加接受范围');
    if(single) assert.equal(await scopeView.count(),0);
    else {assert.equal(await scopeView.locator('p').count(),5);const text=await scopeView.innerText();for(const scope of fixture.scenario.acceptance_goal.additional_acceptance_scopes){assert.ok(text.includes(scope.product_ref.id+'@'+scope.product_ref.revision));assert.ok(text.includes(scope.unit));assert.ok(text.includes(scope.destination));assert.ok(text.includes('工厂放行'));}}
    await form.getByRole('button',{name:'执行只读检查',exact:true}).click();await form.getByLabel('实际草稿检查结果').waitFor();
    const requests=await page.evaluate(()=>window.__p201GoalRequests);assert.equal(requests.length,1);const submitted=requests[0].params.generation_goal;
    if(single) assert.equal('additional_acceptance_scopes' in submitted,false);else assert.deepEqual(submitted.additional_acceptance_scopes,fixture.scenario.acceptance_goal.additional_acceptance_scopes);
    assert.deepEqual(submitted.acceptance_scope,fixture.scenario.acceptance_goal.acceptance_scope);
    result.cases.push({single,status:'passed',submitted_goal:submitted});await page.screenshot({path:fileURLToPath(new URL('checker-scopes-'+(single?'single':'multi')+'.png',out))});await page.close();
  }
  assert.equal(result.errors.length,0,result.errors.join('\n'));result.status='passed';
}catch(error){result.status='failed';result.error=String(error);process.exitCode=1;}
finally{await writeFile(new URL('checker-scopes.json',out),JSON.stringify(result,null,2));await browser.close();}
console.log(JSON.stringify({status:result.status,cases:result.cases.length,error:result.error,actual_Host:false,Native_app:false}));
