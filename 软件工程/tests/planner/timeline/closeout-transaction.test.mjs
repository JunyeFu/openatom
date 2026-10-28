import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createInterface} from 'node:readline';
import {readFile,writeFile,appendFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('.work/p204-closeout-20261022/delivery/',root);
const {mount,event}=await import(new URL('staged-tests/noncu-component.mjs',out));
const {source}=await import(new URL('staged-tests/load-source.mjs',out));
const {inputProjection}=await source('projection'),{timeText}=await source('time');
const a=JSON.parse(await readFile(new URL('producer-root/.work/delivery/host/baseline.actual.json',out),'utf8'));
const ok=reply=>{assert.equal(reply.outcome.status,'ok',JSON.stringify(reply));return reply.outcome.data;};

test('real timeline pointer commit, conflict, undo/redo and historical seek share the actual Host and keep both Runs frozen',async()=>{
 await mkdir(new URL('temp/',out),{recursive:true});
 const child=spawn(fileURLToPath(new URL('transaction-host.exe',out)),[],{cwd:fileURLToPath(root),windowsHide:true,stdio:['pipe','pipe','pipe'],env:{...process.env,TEMP:fileURLToPath(new URL('temp/',out)),TMP:fileURLToPath(new URL('temp/',out))}});
 const ended=once(child,'exit'),pending=[],transcript=new URL(`transaction-${child.pid}.ndjson`,out);let flush=Promise.resolve();
 child.stderr.on('data',data=>{flush=flush.then(()=>appendFile(new URL(`transaction-${child.pid}.stderr.log`,out),data));});
 createInterface({input:child.stdout}).on('line',line=>{const request=pending.shift();assert.ok(request);const value=JSON.parse(line);flush=flush.then(()=>appendFile(transcript,JSON.stringify({request:request.value,response:value})+'\n'));request.resolve(value);});
 ended.then(([code])=>{for(const request of pending)request.reject(new Error(`Host exited ${code}`));});
 const send=value=>new Promise((resolve,reject)=>{pending.push({value,resolve,reject});child.stdin.write(JSON.stringify(value)+'\n');});
 const call=(operation,params)=>send({op:'call',operation,params});
 const object=(kind,id)=>send({op:'object',kind,id});
 const queued=async revision=>ok(await call('run_scenario',{scenario_id:a.scenario.id,input_revision:revision,observation_window:a.scenario.window,intent:{kind:'project_scenario'}}));
 const commits=[],reads=[];let view;
 const commit=async proposal=>{
  commits.push(proposal);const response=await call('apply_commands',{base_revision:proposal.base_project_revision,commands:[proposal.command]});
  if(response.outcome.status==='error')return {ok:false,error:response.outcome.data};
  ok(response);return {ok:true,scenario:await object('scenario',proposal.scenario_id)};
 };
 const seek=request=>{reads.push(call('get_run',{run_id:request.run_id,query:{kind:'objects_at',data:{at_ms:request.at_ms,objects:[],page:{after_index:null,limit:1000}}}}).then(reply=>{const result=ok(reply);assert.equal(result.kind,'objects');assert.equal(result.data.next_index,null);view.render({atMs:request.at_ms,observation:result.data});}));};
 async function drag(){
  const selected=inputProjection(a.scenario),arrival=selected.find(p=>p.input_field?.kind==='arrival'),task=selected.find(p=>p.input_field?.kind==='task_release');
  for(const p of [arrival,task]){view.find('选择输入标记').props.onChange({target:{value:p.key}});view.render();}
  const label=`${arrival.label} ${timeText(arrival.point_ms)}`,span=a.scenario.window.end_ms-a.scenario.window.start_ms,position=200+9750*700/span;
  let marker=view.find(label);assert.ok(marker);marker.props.onPointerDown(event(marker,{clientX:200}));view.render();marker=view.find(label);marker.props.onPointerMove(event(marker,{clientX:position,altKey:false}));view.render();
  marker=view.find(label);marker.props.onPointerUp(event(marker,{clientX:position,altKey:true}));await new Promise(r=>setImmediate(r));
  // The real Host round-trip is pending, so await its actual scenario response.
  await commitFinished();await new Promise(r=>setImmediate(r));view.render();
 }
 let latestCommit=Promise.resolve();
 const actualCommit=proposal=>{const promise=commit(proposal);latestCommit=promise;return promise;};
 const commitFinished=()=>latestCommit;
 try{
  assert.deepEqual(await object('project','P1'),a.project);
  const accepted=await queued(1);assert.equal(accepted.state,'queued');
  view=mount({project:a.project,scenario:a.scenario,run:null,atMs:null,onSelect(){},onTime(){},onSeek:seek,onCommitInput:actualCommit});
  await drag();assert.equal(commits.length,1);assert.equal(commits[0].changes.length,2);assert.ok(commits[0].changes.every(c=>c.after_ms-c.before_ms===9750));
  const edited=await object('scenario',a.scenario.id);assert.equal(edited.revision,2);assert.deepEqual(edited.orders,a.scenario.orders);assert.deepEqual(edited.initial_conditions,a.scenario.initial_conditions);
  const old=await send({op:'finish_run',task:accepted});assert.equal(old.input_ref.project_revision,1);assert.equal(Math.max(...old.acceptance_events.map(e=>e.at_ms)),440000);
  const next=await send({op:'finish_run',task:await queued(2)});assert.notEqual(next.id,old.id);assert.equal(next.input_ref.scenario_revision,2);
  assert.equal(next.orders[0].accepted.value.coefficient,6);assert.equal(next.orders[0].accepted.value.decimal_places,0);
  assert.equal(new Set(next.acceptance_events.flatMap(e=>e.item_refs)).size,6);
  const undo=ok(await call('apply_commands',{base_revision:2,commands:[{kind:'undo'}]}));assert.equal(undo.new_revision,3);
  const restored=await object('scenario',a.scenario.id);assert.deepEqual(restored.arrivals,a.scenario.arrivals);assert.deepEqual(restored.production_tasks,a.scenario.production_tasks);
  const redo=ok(await call('apply_commands',{base_revision:3,commands:[{kind:'redo'}]}));assert.equal(redo.new_revision,4);
  const reapplied=await object('scenario',a.scenario.id);assert.deepEqual(reapplied.arrivals,edited.arrivals);assert.deepEqual(reapplied.production_tasks,edited.production_tasks);
  const context=await send({op:'context'}),draft=await object('project','P1');
  view.render({project:a.project,run:old,runScenario:a.scenario,draftProject:draft,scenario:reapplied,atMs:0});
  for(const at of [370000,419999,425000,0]){view.find('观察毫秒').props.onChange({target:{value:String(at)}});view.render();view.button('读取时点').props.onClick();await reads.at(-1);assert.equal(view.root.props['data-view-time'],at);}
  assert.deepEqual(await send({op:'context'}),context);assert.deepEqual(await send({op:'full',run:old.id}),old);assert.deepEqual(await send({op:'full',run:next.id}),next);
  // A stale form is a real failed Host commit, with the component retaining its preview.
  view.render({project:a.project,draftProject:null,run:null,scenario:a.scenario,runScenario:null});await drag();assert.equal(commits.length,2);assert.ok(view.find('输入编辑预览'));assert.match(view.text(),/RevisionConflict/);
  assert.deepEqual(await send({op:'context'}),context);
  await writeFile(new URL('actual-handler-transaction.json',out),JSON.stringify({status:'passed',producer:'production RunTimeline pointer handlers -> actual public Host/unique Engine',commands:commits,original_run:old.id,changed_run:next.id,old_completion_ms:440000,undo_revision:3,redo_revision:4,view_reads:[370000,419999,425000,0],both_Runs_unchanged:true,stale_conflict_retained:true,transcript:fileURLToPath(transcript)},null,2));
 }finally{child.stdin.end();const[code]=await ended;await flush;assert.equal(code,0);}
});
