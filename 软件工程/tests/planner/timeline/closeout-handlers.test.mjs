import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),out=new URL('.work/p204-closeout-20261022/delivery/',root);
const {mount,event}=await import(new URL('staged-tests/noncu-component.mjs',out));
const {source}=await import(new URL('staged-tests/load-source.mjs',out));
const projection=await source('projection');
const record=async name=>JSON.parse(await readFile(new URL(`producer-root/.work/delivery/host/${name}.actual.json`,out),'utf8'));
const a=await record('baseline'),b=await record('comparison-phase'),observations=[];
function inputView(){
 const scenario=structuredClone(a.scenario);scenario.arrivals=[{...scenario.arrivals[0],id:'A',at_ms:10000},{...scenario.arrivals[0],id:'B',at_ms:20000}];scenario.window={start_ms:0,end_ms:70000};
 const calls=[],v=mount({project:a.project,scenario,run:null,atMs:null,onTime(){},onSelect(){},onCommitInput:async p=>{calls.push(p);return {ok:true,scenario};}});
 return {v,calls};
}
const marker=v=>v.find('到料 A 10秒');
function drag(v,altKey=false){let m=marker(v);m.props.onPointerDown(event(m,{clientX:200}));v.render();m=marker(v);m.props.onPointerMove(event(m,{clientX:297.5,altKey}));v.render();}
async function release(v,altKey){const m=marker(v);m.props.onPointerUp(event(m,{clientX:297.5,altKey}));await new Promise(r=>setImmediate(r));v.render();}
test('secondary-button motion cannot capture or commit a business input',async()=>{
 const {v,calls}=inputView();let m=marker(v);m.props.onPointerDown(event(m,{button:2,clientX:200}));v.render();m=marker(v);m.props.onPointerMove(event(m,{button:2,clientX:297.5,altKey:true}));await release(v,true);
 observations.push({case:'secondary_button',commands:calls.length,captures:v.captures.size});assert.equal(calls.length,0);assert.equal(v.captures.size,0);
});
test('pressing Alt before release recomputes the final unsnapped integer time',async()=>{
 const {v,calls}=inputView();drag(v,false);await release(v,true);observations.push({case:'Alt_pressed_on_release',actual:calls[0]?.changes[0].after_ms});assert.equal(calls[0].changes[0].after_ms,19750);
});
test('releasing Alt before pointer release restores the current snap target',async()=>{
 const {v,calls}=inputView();drag(v,true);await release(v,false);observations.push({case:'Alt_released_on_release',actual:calls[0]?.changes[0].after_ms});assert.equal(calls[0].changes[0].after_ms,20000);
});
test('lost input capture cancels its preview and cannot later commit an abandoned drag',async()=>{
 const {v,calls}=inputView();drag(v);const m=marker(v);observations.push({case:'lost_input_capture',handler_present:typeof m.props.onLostPointerCapture==='function'});
 assert.equal(typeof m.props.onLostPointerCapture,'function');m.releasePointerCapture(9);m.props.onLostPointerCapture(event(m));v.render();assert.equal(v.find('输入编辑预览'),undefined);
 await release(v,false);assert.equal(calls.length,0);assert.equal(v.captures.size,0);
});
test('a selected B job reads actual resources from B frozen Run instead of A',()=>{
 assert.notEqual(a.run.id,b.run.id);
 const selected=projection.actualProjection(b.project,b.run).find(p=>p.kind==='job').selection;
 const before=JSON.stringify({a,b}),v=mount({project:a.project,run:a.run,runScenario:a.scenario,comparison:{project:b.project,run:b.run,scenario:b.scenario},selected,atMs:0,onSelect(){},onTime(){}});
 const resources=b.run.resource_claims.filter(c=>c.job_id===selected.job.id).map(c=>c.resource_id);observations.push({case:'B_resources',expected:resources,relation_text:v.text().split('关系 ·').at(-1)});
 assert.ok(resources.length);for(const id of resources)assert.ok(v.text().includes(`实际资源 ${id}`),id);assert.equal(JSON.stringify({a,b}),before);
});
test('later-cycle predecessor links follow actual transferred input identity, not the first same-step job',()=>{
 const selected=projection.actualProjection(a.project,a.run).filter(p=>p.kind==='job'&&p.selection.job.execution_ref.kind==='project_device'&&p.selection.job.execution_ref.data.route_step_id==='step-assembly')[1].selection;
 const incoming=new Set(selected.job.input_refs),transfer=a.run.events.find(e=>e.payload.kind==='item_transferred'&&incoming.has(e.payload.data.item_ref)&&e.payload.data.from_holder.kind==='job');
 assert.ok(transfer);const actual=transfer.payload.data.from_holder.id;
 const v=mount({project:a.project,run:a.run,runScenario:a.scenario,selected,atMs:selected.job.started_at_ms,onSelect(){},onTime(){}});
 observations.push({case:'later_cycle_predecessor',selected:selected.job.id,expected:actual,relation_text:v.text().split('关系 ·').at(-1)});
 assert.ok(v.text().includes(`实际前置 ${actual}`),actual);
});
test('a newer same-ID Scenario cannot reinterpret an old Run route dependency',()=>{
 const job=a.run.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.route_step_id==='step-assembly');
 const latest={...a.scenario,revision:a.scenario.revision+1};
 const before=JSON.stringify({a,latest}),result=projection.jobPredecessors(a.project,latest,a.run,job);
 assert.deepEqual(result.jobs,[]);assert.deepEqual(result.declared_steps,[]);assert.equal(result.route,null);assert.equal(JSON.stringify({a,latest}),before);
});
test.after(async()=>writeFile(new URL('handler-counterexamples.json',out),JSON.stringify(observations,null,2)));

const nativeDelayed=await record('native-delayed');
for(const owner of ['A','B'])test('actual predecessor navigation inspects and seeks its own '+owner+' Run',()=>{
 const data=owner==='A'?a:nativeDelayed,parts=projection.actualProjection(data.project,data.run);
 const selected=parts.filter(p=>p.kind==='job'&&p.selection.job.execution_ref.kind==='project_device'&&p.selection.job.execution_ref.data.route_step_id==='step-assembly')[1].selection;
 const target=projection.jobPredecessors(data.project,data.scenario,data.run,selected.job).jobs[0];assert.ok(target);
 const inspected=[],seeks=[],objects=[],before=JSON.stringify({a,b});
 const v=mount({project:a.project,run:a.run,runScenario:a.scenario,comparison:{project:nativeDelayed.project,run:nativeDelayed.run,scenario:nativeDelayed.scenario},selected,atMs:selected.job.started_at_ms,onSelect:value=>objects.push(value),onInspect:value=>inspected.push(value),onSeek:value=>seeks.push(value),onTime(){}});
 const link=v.button('实际前置 '+target.id);assert.ok(link);link.props.onClick();
 assert.equal(inspected.length,1);assert.equal(inspected[0].run_id,data.run.id);assert.equal(inspected[0].job.id,target.id);
 assert.deepEqual(JSON.parse(JSON.stringify(seeks)),[{run_id:data.run.id,at_ms:target.started_at_ms}]);assert.deepEqual(JSON.parse(JSON.stringify(objects.at(-1))),{kind:'job',id:target.id});assert.equal(JSON.stringify({a,b}),before);
});
