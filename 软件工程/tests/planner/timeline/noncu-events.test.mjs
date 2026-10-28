import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {mount,event} from './noncu-component.mjs';import {source} from './load-source.mjs';
const {resourceUsage}=await source('projection');
const fixture=JSON.parse(readFileSync(new URL('../../../packages/commands/tests/fixtures/production/p202-ix01-baseline.json',import.meta.url)));
function inputView(options={}){
 const scenario=structuredClone(fixture.scenario);scenario.arrivals=[{...scenario.arrivals[0],id:'A',at_ms:10000},{...scenario.arrivals[0],id:'B',at_ms:20000}];scenario.window={start_ms:0,end_ms:70000};
 const calls=[];const props={project:fixture.project,scenario,run:null,atMs:null,onTime(){},onSelect(){},onCommitInput:async p=>{calls.push(p);return {ok:true,scenario};}};
 return {view:mount(props,options),scenario,calls,props};
}
function handle(v,label='到料 A 10秒'){return v.find(label);}
function drag(v,{altKey=false,dx=97.5}={}){let h=handle(v);assert.ok(h);h.props.onPointerDown(event(h,{clientX:200}));v.render();h=handle(v);h.props.onPointerMove(event(h,{clientX:200+dx,altKey}));v.render();}
async function apply(v){if(v.captures.size){const [id,target]=v.captures.entries().next().value;target.props.onPointerUp(event(target,{pointerId:id}));}else v.button('确认应用一次').props.onClick();await new Promise(r=>setImmediate(r));v.render();}
test('captured drag previews only, snaps within six pixels; release retains current metadata',async()=>{
 const {view:v,calls,scenario}=inputView();drag(v);assert.equal(calls.length,0);assert.match(v.text(),/锚点 20000 ms/);
 await apply(v);assert.equal(calls.length,1);assert.equal(calls[0].command.data.data.arrivals[0].at_ms,20000);assert.equal(calls[0].base_scenario_revision,scenario.revision);
 assert.equal(v.captures.size,0);assert.equal(scenario.arrivals[0].at_ms,10000);
});
test('Alt production pointer handler bypasses snap and retains exact integer milliseconds',async()=>{
 const {view:v,calls}=inputView();drag(v,{altKey:true});await apply(v);assert.equal(calls[0].changes[0].after_ms,19750);
});
test('Ctrl toggles same-type selected input in the real marker handler',()=>{
 const {view:v}=inputView();const h=handle(v);h.props.onPointerDown(event(h,{ctrlKey:true}));v.render();handle(v).props.onPointerUp(event(handle(v),{ctrlKey:true}));handle(v).props.onClick(event(handle(v),{ctrlKey:true}));v.render();assert.match(v.text(),/添加联动输入（1）/);
 const other=handle(v,'到料 B 20秒');other.props.onPointerDown(event(other,{ctrlKey:true}));v.render();handle(v,'到料 B 20秒').props.onClick(event(handle(v,'到料 B 20秒'),{ctrlKey:true}));v.render();assert.match(v.text(),/添加联动输入（2）/);
 handle(v,'到料 B 20秒').props.onClick(event(handle(v,'到料 B 20秒'),{ctrlKey:true}));v.render();assert.match(v.text(),/添加联动输入（1）/);
});
test('linked delta excludes unselected due/calendar/stock; cancel releases capture and sends no command',()=>{
 const {view:v,calls}=inputView();const select=v.find('选择输入标记');
 select.props.onChange({target:{value:`input:${fixture.scenario.id}:arrival:B`}});v.render();
 let h=handle(v,'到料 B 20秒');h.props.onPointerDown(event(h));v.render();h=handle(v,'到料 B 20秒');h.props.onPointerMove(event(h,{clientX:150,altKey:true}));v.render();
 assert.match(v.text(),/尚未应用/);h=handle(v,'到料 B 20秒');h.props.onPointerCancel(event(h));v.render();assert.equal(v.find('输入编辑预览'),undefined);assert.equal(v.captures.size,0);assert.equal(calls.length,0);
});
test('CSS-scaled ruler uses the same inverse mapping for seek and input delta',async()=>{
 const {view:v,calls}=inputView({rectWidth:1400});drag(v,{altKey:true,dx:20});await apply(v);assert.equal(calls[0].changes[0].after_ms,11000);
});
test('RevisionConflict preserves frozen proposal; refreshed re-preview uses new revisions',async()=>{
 const {view:v,props,calls}=inputView();v.render({onCommitInput:async p=>{calls.push(p);return {ok:false,error:{code:'RevisionConflict',message:'changed'}};}});drag(v);await apply(v);assert.match(v.text(),/RevisionConflict/);assert.ok(v.find('输入编辑预览'));
 const project={...props.project,revision:2},scenario={...props.scenario,revision:2,project_revision:2};v.render({project,scenario});v.button('按已刷新输入重新预览').props.onClick();v.render();
 await apply(v);assert.equal(calls[0].base_project_revision,1);assert.equal(calls[1].base_project_revision,2);
});
test('switch Scenario drops the old preview and explicit reference bindings',()=>{
 const {view:v,scenario}=inputView();drag(v);assert.ok(v.find('输入编辑预览'));v.render({scenario:{...scenario,id:'another-scenario'}});assert.equal(v.find('输入编辑预览'),undefined);
});
test('prefix utilization excludes uncomputed future calendar closure',()=>{
 const p={resources:[{id:'R',calendar_id:'C',capacity:{value:{coefficient:1,decimal_places:0},unit:'slot'}}],calendars:[{id:'C',intervals:[{start_ms:0,end_ms:50}]}]};
 const r={computed_until_ms:100,resource_claims:[{resource_id:'R',units:1,intervals:[{start_ms:0,end_ms:50}]}]};
 assert.equal(resourceUsage(p,r,['R'],{start_ms:0,end_ms:1000})[0].available,50);
});
function actualView(){
 const r={id:'event-entry-sample',input_ref:{project_id:'P1',project_revision:1},window:{start_ms:0,end_ms:70000},computed_until_ms:40000,
   jobs:[],waits:[],phase_work_intervals:[],resource_claims:[],transport:[],events:[],acceptance_events:[],scope:[],limitations:[]};
 const queries=[];return {queries,view:mount({project:fixture.project,run:r,atMs:0,onSelect(){},onTime(){},onSeek:q=>queries.push(q)})};
}
test('Escape releases range capture and discards uncommitted selection without querying production',()=>{
 const {view:v,queries}=actualView();let chart=v.class('sf-tl-chart');chart.props.onPointerDown(event(chart));v.render();
 chart=v.class('sf-tl-chart');chart.props.onPointerMove(event(chart,{clientX:150}));v.render();assert.equal(v.captures.size,1);
 v.root.props.onKeyDown(event(v.root,{key:'Escape'}));v.render();assert.equal(v.captures.size,0);assert.doesNotMatch(v.text(),/选段 0秒/);assert.equal(queries.length,0);
});
test('Escape releases shared ruler capture; unknown future produces no ObjectsAt request',()=>{
 const {view:v,queries}=actualView();let ruler=v.class('sf-tl-ruler-axis');ruler.props.onPointerDown(event(ruler));v.render();
 v.root.props.onKeyDown(event(v.root,{key:'Escape'}));v.render();assert.equal(v.captures.size,0);
 v.find('观察毫秒').props.onChange({target:{value:'50000'}});v.render();v.button('读取时点').props.onClick();v.render();assert.equal(queries.length,0);assert.match(v.text(),/尚未计算/);
});
test('detail and Back keep the shared active time; display windows retain absolute coordinates',()=>{
 const {view:v,queries}=actualView();const chart=v.nodes().find(n=>n.props['data-row']?.startsWith('group:')||n.props['data-row']?.startsWith('line:')||n.props['data-row']?.startsWith('ungrouped:'));
 assert.ok(chart);chart.props.onPointerDown(event(chart,{clientX:200}));v.render();
 chart.props.onPointerMove(event(chart,{clientX:400}));v.render();chart.props.onPointerUp(event(chart,{clientX:400}));v.render();
 assert.equal(v.button('细化选段').props.disabled,false);v.button('细化选段').props.onClick();v.render();assert.equal(v.root.props['data-window-start'],10000);assert.equal(v.root.props['data-window-end'],30000);
 v.find('观察毫秒').props.onChange({target:{value:'20000'}});v.render();v.button('读取时点').props.onClick();v.render({atMs:20000});
 v.find('返回主轴').props.onClick();v.render();assert.equal(v.root.props['data-view-time'],20000);assert.equal(v.root.props['data-window-start'],0);assert.equal(queries.length,1);
});
test('switching to B requests B RunId and switching the comparison clears its temporary bindings',()=>{
 const {view:v,queries}=actualView();const comparison={project:fixture.project,scenario:fixture.scenario,run:{id:'B',input_ref:{project_id:'P1',project_revision:1},window:{start_ms:0,end_ms:70000},computed_until_ms:30000,jobs:[],waits:[],phase_work_intervals:[],resource_claims:[],transport:[],events:[],acceptance_events:[],scope:[],limitations:[]}};
 v.render({comparison,runScenario:fixture.scenario});v.button('对比').props.onClick();v.render();v.button('读取 B 时点').props.onClick();assert.deepEqual(JSON.parse(JSON.stringify(queries)),[{run_id:'B',at_ms:30000}]);
 const key=v.key;v.render({comparison:{...comparison,run:{...comparison.run,id:'C'}}});assert.notEqual(v.key,key);assert.doesNotMatch(v.text(),/候选 30秒/);
});
test('same-Scenario linked selection preserves spacing; explicit due moves and clearing removes all links',async()=>{
 const {view:v,calls,scenario}=inputView();let select=v.find('选择输入标记');
 for(const kind of ['arrival:A','arrival:B','order_due:ORDER-01']){select.props.onChange({target:{value:`input:${scenario.id}:${kind}`}});v.render();select=v.find('选择输入标记');}
 v.find('输入精确毫秒').props.onChange({target:{value:'15000'}});v.render();v.button('预览精确时点').props.onClick();v.render();await apply(v);
 assert.deepEqual(calls[0].changes.map(c=>c.after_ms-c.before_ms),[5000,5000,5000]);assert.equal(calls[0].command.data.data.orders[0].release_at_ms,scenario.orders[0].release_at_ms);
 assert.deepEqual(calls[0].command.data.data.initial_conditions,scenario.initial_conditions);
 v.button('清除联动').props.onClick();v.render();assert.match(v.text(),/添加联动输入（0）/);
});
test('calendar opening boundary is a visible business snap candidate without editing calendar data',async()=>{
 const {view:v,calls,props}=inputView();const project=structuredClone(props.project);project.calendars[0].intervals=[{start_ms:14000,end_ms:21000}];v.render({project});
 drag(v,{dx:41});assert.match(v.text(),/班次开始/);await apply(v);assert.equal(calls[0].changes[0].after_ms,14000);assert.deepEqual(project.calendars[0].intervals,[{start_ms:14000,end_ms:21000}]);
});
test('linked negative time is rejected without partial commit; pointer identity remains bound',()=>{
 const {view:v,calls}=inputView();let h=handle(v);h.props.onPointerDown(event(h));v.render();h=handle(v);h.props.onPointerMove(event(h,{pointerId:99,clientX:150}));v.render();assert.equal(v.find('输入编辑预览'),undefined);
 h=handle(v);h.props.onPointerMove(event(h,{clientX:-1000,altKey:true}));v.render();assert.match(v.text(),/非负/);assert.equal(calls.length,0);v.root.props.onKeyDown(event(v.root,{key:'Escape'}));v.render();assert.equal(v.captures.size,0);
});
test('InvalidInput retains the typed field error and the complete proposal for review',async()=>{
 const {view:v}=inputView();v.render({onCommitInput:async()=>({ok:false,error:{code:'InvalidInput',message:'/arrivals/0/at_ms: invalid'}})});drag(v);await apply(v);assert.match(v.text(),/InvalidInput.*arrivals/);assert.ok(v.find('输入编辑预览'));
});
test('declared material, signal and artifact dependency links use exact selected-device endpoints',()=>{
 const a=JSON.parse(readFileSync(new URL('../../../.work/delivery/host/baseline.actual.json',import.meta.url)));
 const job=a.run.jobs.find(j=>j.execution_ref.kind==='project_device'),device=job.execution_ref.data.device_id,project=structuredClone(a.project);
 const port={module_ref:project.devices.find(d=>d.id===device).module_ref,instance_id:device,port_id:'controlled-out'};
 project.connections.push({kind:'signal',data:{id:'signal-link',source:port,destination:{...port,instance_id:'other'},value_type:'boolean',effect_target:null}},
   {kind:'artifact',data:{id:'artifact-link',source:port,destination:{...port,instance_id:'other'},binding_ref:'controlled-binding'}});
 const selected={key:'controlled-selected-job',source:'actual',run_id:a.run.id,object:{kind:'job',id:job.id},label:job.id,fields:[],related_objects:[],job};
 const v=mount({project,run:a.run,runScenario:a.scenario,atMs:0,selected,onTime(){},onSelect(){}});
 assert.match(v.text(),/逻辑连接 signal-link/);assert.match(v.text(),/产物连接 artifact-link/);
});
test('phase/resource/wait rows appear only after explicit single-device detail, Back restores main grouping',()=>{
 const a=JSON.parse(readFileSync(new URL('../../../.work/delivery/host/phase-calendar.actual.json',import.meta.url))),v=mount({project:a.project,run:a.run,runScenario:a.scenario,atMs:0,onSelect(){},onTime(){}});
 assert.equal(v.nodes().filter(n=>n.props['data-row']?.includes(':work:')).length,0);
 let chart=v.nodes().find(n=>n.props['data-row']?.startsWith('device:'));assert.ok(chart);
 chart.props.onPointerDown(event(chart));v.render();chart.props.onPointerMove(event(chart,{clientX:450}));v.render();chart.props.onPointerUp(event(chart,{clientX:450}));v.render();
 v.button('细化选段').props.onClick();v.render();
 // Rows are virtualized: scroll through their same-axis local inspection.
 const all=new Set();for(const top of [0,100,200,300]){v.class('sf-tl-viewport').props.onScroll({currentTarget:{scrollTop:top}});v.render();for(const n of v.nodes())if(n.props['data-row'])all.add(n.props['data-row']);}
 assert.ok([...all].some(r=>r.includes(':work:')));assert.ok([...all].some(r=>r.includes(':wait:')));assert.ok([...all].some(r=>r.includes(':resource:')));
 v.find('返回主轴').props.onClick();v.render();assert.equal(v.nodes().filter(n=>n.props['data-row']?.startsWith('detail:')).length,0);assert.equal(v.root.props['data-run-id'],a.run.id);
});
test('TL06 pointer release submits its frozen preview exactly once, cancellation before release submits nothing',async()=>{
 const {view:v,calls}=inputView();let h=handle(v);h.props.onPointerDown(event(h));v.render();h=handle(v);h.props.onPointerMove(event(h,{clientX:150,altKey:true}));v.render();assert.equal(calls.length,0);
 h=handle(v);h.props.onPointerUp(event(h,{clientX:150}));h.props.onPointerUp(event(h,{clientX:150}));await new Promise(r=>setImmediate(r));v.render();
 assert.equal(calls.length,1);assert.equal(calls[0].changes[0].after_ms,15000);assert.equal(calls[0].base_project_revision,1);assert.equal(v.captures.size,0);
});
test('clearing linked inputs discards an uncommitted numeric preview instead of retaining stale propagation',()=>{
 const {view:v,calls,scenario}=inputView();v.find('选择输入标记').props.onChange({target:{value:`input:${scenario.id}:arrival:A`}});v.render();v.find('输入精确毫秒').props.onChange({target:{value:'15000'}});v.render();v.button('预览精确时点').props.onClick();v.render();assert.ok(v.find('输入编辑预览'));
 v.button('清除联动').props.onClick();v.render();assert.equal(v.find('输入编辑预览'),undefined);assert.equal(calls.length,0);
});
