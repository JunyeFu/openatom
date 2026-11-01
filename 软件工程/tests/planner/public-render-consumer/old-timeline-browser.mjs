import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright');
const root = fileURLToPath(new URL('../../../', import.meta.url)), out = new URL('scenarios/Agent共建/runtime-public-render-consumer/regression/old-timeline-browser/', new URL('../../../', import.meta.url));
const child = spawn(fileURLToPath(new URL('scenarios/从零搭建控制盒厂房/runtime-timeline/consume_engine.exe', new URL('../../../',import.meta.url))), [root], { windowsHide: true, stdio:['pipe','pipe','pipe'] });
const lines = createInterface({input:child.stdout}), queue=[];let ready;
const initial = new Promise(resolve=>ready=resolve);
lines.on('line',line=>{const value=JSON.parse(line);if(value.ready)ready(value);else queue.shift()(value);});
let serial=Promise.resolve();
function query(value){const p=serial.then(()=>new Promise(resolve=>{queue.push(resolve);child.stdin.write(JSON.stringify(value)+'\n');}));serial=p;return p;}
const producer=await initial;
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
const page=await browser.newPage({viewport:{width:1440,height:950}}), errors=[], warnings=[], results=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',msg=>{if(msg.type()==='error')errors.push(msg.text());if(msg.type()==='warning')warnings.push(msg.text());});
await page.route('**/__p204_engine_observation?**',async route=>{
  const url=new URL(route.request().url()),request={run_id:url.searchParams.get('run_id'),at_ms:Number(url.searchParams.get('at_ms'))};
  const value=await query(request);await route.fulfill({status:value.objects?200:422,contentType:'application/json',body:JSON.stringify(value)});
});
const url='http://127.0.0.1:8922/@fs/'+root.replaceAll('\\','/').replace(/\/$/,'')+'/scenarios/从零搭建控制盒厂房/runtime-timeline/preview.html';
async function checkpoint(name,action){await action();results.push({name,status:'passed'});console.log('PASS '+name);}
try {
  await page.goto(url);await page.getByRole('region',{name:'全厂时间轴'}).waitFor();
  assert.equal(await page.title(),'P2-04 真实时间轴消费者');assert.ok((await page.locator('body').innerText()).includes('P204-IX01-delayed-pcb'));assert.equal(await page.locator('vite-error-overlay').count(),0);
  results.push({name:'identity / meaningful DOM / no framework overlay',status:'passed',url:page.url(),title:await page.title()});
  const timeline=page.getByRole('region',{name:'全厂时间轴'}), clock=page.getByTestId('at');
  await checkpoint('real ObjectsAt seek 370000 keeps production totals',async()=>{await timeline.getByRole('spinbutton',{name:'观察毫秒'}).fill('370000');await timeline.getByRole('button',{name:'读取时点',exact:true}).click();await page.waitForFunction(()=>document.querySelector('[data-testid=at]').textContent==='370000');assert.ok((await page.locator('body').innerText()).includes('真实接受终点 570000'));});
  await checkpoint('ruler drag previews without requests until pointerup',async()=>{const before=await page.evaluate(()=>window.__timelineEvidence.seeks.length),box=await page.locator('.sf-tl-ruler-axis').boundingBox();await page.mouse.move(box.x+box.width*.2,box.y+14);await page.mouse.down();await page.mouse.move(box.x+box.width*.3,box.y+14);assert.equal(await page.evaluate(()=>window.__timelineEvidence.seeks.length),before);assert.equal(await clock.innerText(),'370000');await page.mouse.up();await page.waitForFunction(n=>window.__timelineEvidence.seeks.length===n+1,before);await page.waitForFunction(()=>!document.querySelector('.sf-tl-ghost'));});
  await checkpoint('actual selection retained while description hidden',async()=>{await page.locator('button[aria-label^="展开 "]').first().click();await page.locator('.sf-tl-bar-work').first().click();await page.waitForFunction(()=>window.__timelineEvidence.selected!==null);assert.equal(await page.getByRole('complementary',{name:'测试共享描述'}).count(),0);await page.getByRole('button',{name:'显示 / 隐藏描述',exact:true}).click();assert.ok((await page.getByRole('complementary',{name:'测试共享描述'}).innerText()).includes('阶段'));});
  await checkpoint('display collapse/expand does not add production or create Run',async()=>{const run=await page.getByTestId('run').innerText();await page.locator('button[aria-label^="折叠 "]').first().click();await page.locator('button[aria-label^="展开 "]').first().click();assert.equal(await page.getByTestId('run').innerText(),run);});

  await checkpoint('actual cart wait 370-420 plus return425 remains selectable on shared resource',async()=>{
    await timeline.getByRole('button',{name:'共享资源',exact:true}).click();
    const view=timeline.locator('.sf-tl-viewport');await view.evaluate(e=>e.scrollTop=340);
    const wait=timeline.locator('[data-row="resource:T1"] .sf-tl-bar-wait[data-true-start="370000"][data-true-end="420000"]');await wait.waitFor();await wait.click();assert.equal(await page.evaluate(()=>window.__timelineEvidence.selected.wait.interval.start_ms),370000);assert.equal(await page.evaluate(()=>window.__timelineEvidence.selected.wait.interval.end_ms),420000);
    assert.ok((await page.getByRole('complementary',{name:'测试共享描述'}).innerText()).includes('接收'));
    await page.screenshot({path:fileURLToPath(new URL('actual-cart-wait.png',out)),fullPage:true});await view.evaluate(e=>e.scrollTop=0);await timeline.getByRole('button',{name:'共享资源',exact:true}).click();
  });
  await page.screenshot({path:fileURLToPath(new URL('desktop-actual.png',out)),fullPage:true});

  await checkpoint('W5-P204-01 item follow to input leaves ordinary cancel and Escape reachable',async()=>{
    const commits=await page.evaluate(()=>window.__timelineEvidence.commits.length);
    await timeline.getByRole('button',{name:'物品跟随',exact:true}).click();
    const itemPanel=timeline.getByLabel('物品跟随面板');await itemPanel.waitFor();
    const firstItem=await itemPanel.getByRole('combobox',{name:'跟随物品'}).locator('option').nth(1).getAttribute('value');
    await itemPanel.getByRole('combobox',{name:'跟随物品'}).selectOption(firstItem);
    await timeline.getByRole('button',{name:'输入',exact:true}).click();assert.equal(await itemPanel.count(),0);
    const select=timeline.getByRole('combobox',{name:'选择输入标记'}),marker=await select.locator('option').filter({hasText:'pcb-batch2'}).getAttribute('value');await select.selectOption(marker);
    await timeline.getByRole('spinbutton',{name:'输入精确毫秒'}).fill('510000');await timeline.getByRole('button',{name:'预览精确时点'}).click();
    const preview=timeline.getByLabel('输入编辑预览');await preview.waitFor();await page.screenshot({path:fileURLToPath(new URL('item-follow-input-cancel.png',out)),fullPage:true});
    await preview.getByRole('button',{name:'取消预览',exact:true}).click({timeout:2000});await preview.waitFor({state:'detached'});
    await timeline.getByRole('button',{name:'预览精确时点'}).click();await preview.waitFor();await page.keyboard.press('Escape');await preview.waitFor({state:'detached'});
    assert.equal(await page.evaluate(()=>window.__timelineEvidence.commits.length),commits);
    await timeline.getByRole('button',{name:'实际',exact:true}).click();await timeline.getByRole('button',{name:'物品跟随',exact:true}).click();
    await itemPanel.waitFor();assert.equal(await itemPanel.getByRole('combobox',{name:'跟随物品'}).inputValue(),firstItem);await itemPanel.getByRole('combobox',{name:'跟随物品'}).focus();await page.keyboard.press('Escape');await itemPanel.waitFor({state:'detached'});
  });
  await checkpoint('input linked numeric preview, cancel and explicit one commit',async()=>{await timeline.getByRole('button',{name:'输入',exact:true}).click();const options=await timeline.getByRole('combobox',{name:'选择输入标记'}).locator('option').evaluateAll(es=>es.slice(1).map(e=>({value:e.value,text:e.textContent})));assert.ok(options.length);
    await timeline.getByRole('combobox',{name:'选择输入标记'}).selectOption(options[0].value);await timeline.getByRole('spinbutton',{name:'输入精确毫秒'}).fill('5000');await timeline.getByRole('button',{name:'预览精确时点'}).click();assert.equal(await page.evaluate(()=>window.__timelineEvidence.commits.length),0);await timeline.getByRole('button',{name:'取消预览'}).click();assert.equal(await page.evaluate(()=>window.__timelineEvidence.commits.length),0);
    await timeline.getByRole('spinbutton',{name:'输入精确毫秒'}).fill('6000');await timeline.getByRole('button',{name:'预览精确时点'}).click();await timeline.getByRole('button',{name:'确认应用一次'}).click();await page.waitForFunction(()=>window.__timelineEvidence.commits.length===1);assert.equal(await timeline.getByLabel('输入编辑预览').count(),0);
  });
  await checkpoint('canonical conflict response preserves captured proposal and preview',async()=>{await page.getByRole('checkbox',{name:'测试输入冲突响应'}).check();await timeline.getByRole('spinbutton',{name:'输入精确毫秒'}).fill('7000');await timeline.getByRole('button',{name:'预览精确时点'}).click();await timeline.getByRole('button',{name:'确认应用一次'}).click();await timeline.getByRole('alert').filter({hasText:'RevisionConflict'}).waitFor();assert.equal(await page.evaluate(()=>window.__timelineEvidence.commits.length),2);await timeline.getByRole('button',{name:'取消预览'}).click();});
  await checkpoint('pointercancel/Escape leaves no input command',async()=>{const handle=page.locator('.sf-tl-point-input').first(),box=await handle.boundingBox(),before=await page.evaluate(()=>window.__timelineEvidence.commits.length);await page.mouse.move(box.x+4,box.y+8);await page.mouse.down();await page.mouse.move(box.x+34,box.y+8);await page.keyboard.press('Escape');await page.mouse.up();assert.equal(await page.evaluate(()=>window.__timelineEvidence.commits.length),before);assert.equal(await timeline.getByLabel('输入编辑预览').count(),0);});
  await checkpoint('actual comparison uses two real frozen RunIds',async()=>{await timeline.getByRole('button',{name:'对比',exact:true}).click();assert.equal(await page.locator('[data-comparable=true]').count(),1);assert.ok(await page.locator('.sf-tl-summary-b').count()>0);await timeline.getByRole('button',{name:'读取 B 时点'}).click();await page.waitForFunction(()=>document.querySelector('[data-testid=run]').textContent==='P204-IX01-baseline');assert.ok((await page.locator('body').innerText()).includes('真实接受终点 440000'));});
  await checkpoint('same exact item identity via complete Engine ObjectsAt',async()=>{await timeline.getByRole('button',{name:'物品跟随',exact:true}).click();await timeline.getByRole('combobox',{name:'跟随物品'}).locator('option').nth(1).waitFor({state:'attached'});const id=await timeline.getByRole('combobox',{name:'跟随物品'}).locator('option').nth(1).getAttribute('value');await timeline.getByRole('combobox',{name:'跟随物品'}).selectOption(id);assert.ok((await timeline.getByLabel('物品跟随面板').innerText()).includes(id));await timeline.getByLabel('物品跟随面板').getByRole('button',{name:'关闭',exact:true}).click();});
  await checkpoint('hide/reopen preferences retain expanded rows and shared selection',async()=>{const selected=await page.evaluate(()=>window.__timelineEvidence.selected?.key);await page.getByRole('button',{name:'显示 / 隐藏时间轴',exact:true}).click();await page.getByRole('button',{name:'显示 / 隐藏时间轴',exact:true}).click();assert.equal(await page.evaluate(()=>window.__timelineEvidence.selected?.key),selected);await timeline.waitFor();});

  await checkpoint('child detail shares absolute time and Back preserves deliberate seek',async()=>{
    await timeline.locator('.sf-tl-viewport').evaluate(e=>e.scrollTop=0);
    await timeline.locator('.sf-tl-summary').first().click();await timeline.getByRole('button',{name:'细化选段',exact:true}).click();await timeline.getByRole('button',{name:'返回主轴',exact:true}).waitFor();
    const start=Number(await timeline.getAttribute('data-window-start')),end=Number(await timeline.getAttribute('data-window-end'));assert.ok(end>start);const at=Math.floor((start+end)/2);
    await timeline.getByRole('spinbutton',{name:'观察毫秒'}).fill(String(at));await timeline.getByRole('button',{name:'读取时点',exact:true}).click();await page.waitForFunction(t=>document.querySelector('[data-testid=at]').textContent===String(t),at);await timeline.getByRole('button',{name:'返回主轴',exact:true}).click();assert.equal(await clock.innerText(),String(at));assert.equal(Number(await timeline.getAttribute('data-window-end')),600000);
  });
  await checkpoint('Ctrl wheel zoom and Shift pan change only absolute view window',async()=>{
    const view=timeline.locator('.sf-tl-viewport'),box=await view.boundingBox(),at=await clock.innerText();await page.mouse.move(box.x+box.width*.7,box.y+40);await page.keyboard.down('Control');await page.mouse.wheel(0,-200);await page.keyboard.up('Control');await page.waitForFunction(()=>Number(document.querySelector('.sf-timeline').dataset.windowEnd)-Number(document.querySelector('.sf-timeline').dataset.windowStart)<600000);assert.equal(await clock.innerText(),at);await timeline.getByRole('button',{name:'全窗口',exact:true}).click();
  });
  await checkpoint('private manual display group and independent row never edit Project',async()=>{
    const count=await page.evaluate(()=>window.__timelineEvidence.commits.length);await timeline.getByRole('button',{name:'显示编组',exact:true}).click();const fly=timeline.getByLabel('显示编组编辑');await fly.getByRole('checkbox').first().check();await fly.getByRole('textbox',{name:'显示组名称'}).fill('核验显示组');await fly.getByRole('button',{name:'加入 / 改名'}).click();await fly.getByRole('button',{name:'关闭',exact:true}).click();await timeline.locator('.sf-tl-viewport').evaluate(e=>e.scrollTop=0);assert.ok((await timeline.innerText()).includes('核验显示组'));assert.equal(await page.evaluate(()=>window.__timelineEvidence.commits.length),count);
  });
  await checkpoint('actual computed prefix leaves unknown tail and rejects future seek without query',async()=>{
    await page.getByRole('button',{name:'读取真实计算前缀',exact:true}).click();await timeline.locator('.sf-tl-unknown').first().waitFor();const before=await page.evaluate(()=>window.__timelineEvidence.seeks.length);await timeline.getByRole('spinbutton',{name:'观察毫秒'}).fill('590000');await timeline.getByRole('button',{name:'读取时点',exact:true}).click();assert.equal(await page.evaluate(()=>window.__timelineEvidence.seeks.length),before);assert.ok((await timeline.innerText()).includes('尚未计算'));
  });
  await page.screenshot({path:fileURLToPath(new URL('desktop-baseline.png',out)),fullPage:true});
  await page.setViewportSize({width:540,height:850});await page.getByRole('button',{name:'显示 / 隐藏描述',exact:true}).click();await page.getByRole('combobox',{name:'测试面板高度'}).selectOption('160');await page.screenshot({path:fileURLToPath(new URL('compact-160.png',out)),fullPage:true});
  assert.ok(await timeline.isVisible());assert.equal(errors.length,0,JSON.stringify(errors));
} catch(error) {results.push({name:'browser flow',status:'failed',error:String(error),stack:error.stack});await page.screenshot({path:fileURLToPath(new URL('browser-failure.png',out)),fullPage:true});process.exitCode=1;}
finally {
  const evidence=await page.evaluate(()=>window.__timelineEvidence??null).catch(()=>null);
  const output={browser_availability:'Browser plugin not available; existing bundled Playwright used without installation',flow:'isolated actual Engine records -> RunTimeline -> seek/selection/input preview/one commit/conflict/comparison/item follow',producer,results,errors,warnings,evidence,Host_commit_test_double:true,Native_Workbench_acceptance:false};
  await writeFile(new URL('browser-result.json',out),JSON.stringify(output,null,2));console.log(JSON.stringify({results,errors,warnings},null,2));await browser.close();child.stdin.end();
}
