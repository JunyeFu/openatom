const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const fs=require('node:fs'),path=require('node:path');
(async()=>{const b=await chromium.launch({channel:'chrome',headless:true}),p=await b.newPage({viewport:{width:1920,height:1080}}),out=path.join(__dirname,'output','delegated-walkthrough');fs.mkdirSync(out,{recursive:true});
const e=()=>p.evaluate(()=>window.S04_PROTOTYPE.evidence()),click=a=>p.locator(`[data-action="${a}"]:visible`).first().click();
await p.goto('http://127.0.0.1:8921/?frame=VP24');await p.waitForFunction(()=>window.S04_READY);await p.getByRole('textbox',{name:'X',exact:true}).fill('8.6');const before=await e();await click('rotate');const after=await e();const rotation={before:{x:before.form.data.x,dirty:before.formDirty},after:{x:after.project.devices[0].x,dirty:after.formDirty,form:after.form?.data.x},dialog:await p.locator('dialog').isVisible()};
await p.screenshot({path:path.join(out,'probe-dirty-rotation.png')});
await p.goto('http://127.0.0.1:8921/?frame=VP10');await p.waitForFunction(()=>window.S04_READY);await click('copy-line');await click('copy-confirm');await click('copy-line');await click('copy-confirm');const ids=(await e()).project.devices.map(x=>x.id);const copy={total:ids.length,unique:new Set(ids).size,duplicates:ids.filter((x,i)=>ids.indexOf(x)!==i)};
await p.screenshot({path:path.join(out,'probe-repeated-copy.png')});
fs.writeFileSync(path.join(out,'initial-probes.json'),JSON.stringify({operator:'Codex AI delegated by user',authorization:'代替我完成S04-04/S04-05',observed_at:new Date().toISOString(),rotation,copy},null,2));console.log(JSON.stringify({rotation,copy}));await b.close();})().catch(e=>{console.error(e);process.exitCode=1});
