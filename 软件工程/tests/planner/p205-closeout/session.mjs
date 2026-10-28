import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createInterface} from 'node:readline';
import {readFile,writeFile,appendFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {root,out,runtime,event} from './component.mjs';
export const clone=v=>structuredClone(v),plain=v=>JSON.parse(JSON.stringify(v));
export const read=async relative=>JSON.parse(await readFile(new URL(relative,root),'utf8'));
export const ok=reply=>{assert.equal(reply.outcome.status,'ok',JSON.stringify(reply));return reply.outcome.data;};
export async function bridge(args){
 await mkdir(new URL('temp/',out),{recursive:true});
 const child=spawn(fileURLToPath(new URL('host.exe',out)),args,{cwd:fileURLToPath(root),windowsHide:true,stdio:['pipe','pipe','pipe'],env:{...process.env,TEMP:fileURLToPath(new URL('temp/',out)),TMP:fileURLToPath(new URL('temp/',out))}});
 const pending=[],ended=once(child,'exit');let transcript=Promise.resolve(),closed=false;
 child.stderr.on('data',data=>{transcript=transcript.then(()=>appendFile(new URL(`logs/host-${child.pid}.stderr.log`,out),data));});
 createInterface({input:child.stdout}).on('line',line=>{const item=pending.shift();assert.ok(item);const response=JSON.parse(line);transcript=transcript.then(()=>appendFile(new URL(`host-${child.pid}.ndjson`,out),JSON.stringify({request:item.value,response})+'\n'));item.resolve(response);});
 ended.then(([code])=>{for(const item of pending)item.reject(new Error(`actual Host exited ${code}`));});
 const api={pid:child.pid,send(value){return new Promise((resolve,reject)=>{pending.push({value,resolve,reject});child.stdin.write(JSON.stringify(value)+'\n');});},
  async close(){if(closed)return;closed=true;child.stdin.end();const[code]=await ended;await transcript;assert.equal(code,0);},
  call(operation,params){return api.send({op:'call',operation,params});},snapshot(){return api.send({op:'snapshot'});},full(run){return api.send({op:'full',run:typeof run==='string'?run:run.id});},
 };return api;
}
export function moduleMap(definitions){return new Map(definitions.map(definition=>[JSON.stringify([definition.id,definition.version]),{definition,assets:[],files:[],dependencies:[],rights:[],issues:[]} ]));}
export async function form(host,fixture){
 const rt=runtime(),{BusinessEditor}=rt.source('apps/planner/src/features/business/BusinessEditor.tsx'),{RunTimeline}=rt.source('apps/planner/src/features/timeline/RunTimeline.tsx');
 let currentHost=host,state=await host.snapshot(),history=null,run=null,frozen=null,view,timeline,selected=fixture.scenario.id,commitGate=null;
 const commits=[],runRecords=[],locations=[],runRequests=[],dirty=[],proposals=[];
 const business=s=>({project:s.project,scenarios:s.scenarios});
 function props(){return {...business(state),modules:moduleMap(fixture.definitions),scenarioId:selected,history,run,frozenProject:frozen?.project??null,frozenScenario:frozen?.scenario??null,atMs:0,canEditSetupPolicies:true,
  onDirtyChange:value=>dirty.push(plain(value)),onScenarioChange:id=>{selected=id;},onLocate:value=>locations.push(plain(value)),onClose:()=>locations.push({closed:true}),
  async onCommit(proposal){proposals.push(plain(proposal));assert.equal(proposal.project_id,state.project.id);
   const gate=commitGate;commitGate=null;if(gate)await gate;
   const reply=await currentHost.call('apply_commands',{base_revision:proposal.base_project_revision,commands:plain(proposal.commands)});
   if(reply.outcome.status!=='ok')return {ok:false,error:reply.outcome.data};
   const commit=reply.outcome.data;state=await currentHost.snapshot();history=commit.history;commits.push(plain(commit));publish();
   return {ok:true,commit,snapshot:business(state)};
  },async onRefresh(){state=await currentHost.snapshot();publish();},
  async onRun(request){runRequests.push(plain(request));assert.equal(request.project_id,state.project.id);assert.equal(request.base_project_revision,state.project.revision);
   const scenario=state.scenarios.find(s=>s.id===request.scenario_id);assert.equal(request.base_scenario_revision,scenario.revision);
   frozen={project:clone(state.project),scenario:clone(scenario)};run=await currentHost.send({op:'run',scenario:scenario.id});runRecords.push(clone(run));publish();
  }};}
 function timelineProps(){return {project:frozen?.project??state.project,draftProject:state.project,scenario:state.scenarios.find(s=>s.id===selected),run,runScenario:frozen?.scenario??null,atMs:0,onTime(){},onSelect(){},
  onEditBusiness:intent=>{view.render({editIntent:{...intent,id:`intent-${proposals.length}-${intent.section}`}});},
  async onCommitInput(proposal){const result=await props().onCommit({project_id:proposal.project_id,base_project_revision:proposal.base_project_revision,scenario_revisions:{[proposal.scenario_id]:proposal.base_scenario_revision},commands:[plain(proposal.command)],label:'actual timeline handler'});
   return result.ok?{ok:true,scenario:state.scenarios.find(s=>s.id===proposal.scenario_id)}:result;
  }};}
 function publish(){view?.render(props());timeline?.render(timelineProps());}
 view=rt.mount(BusinessEditor,props());await view.flush();
 const api={view,commits,runRecords,locations,runRequests,dirty,proposals,get state(){return state;},get run(){return run;},get history(){return history;},get host(){return currentHost;},
  async refresh(){state=await currentHost.snapshot();publish();await view.flush();},
  mountTimeline(){timeline=rt.mount(RunTimeline,timelineProps());return timeline;},
  holdNextCommit(){let release;commitGate=new Promise(resolve=>{release=resolve;});return release;},
  async attach(next,nextFixture){currentHost=next;fixture=nextFixture;state=await next.snapshot();selected=fixture.scenario.id;history=null;run=null;frozen=null;publish();await view.flush();},
  unmount(){view.unmount();timeline?.unmount();},
 };return api;
}
export async function tab(view,label){const node=view.button(label);assert.ok(node,`tab ${label}`);return view.act(()=>node.props.onClick());}
export async function fill(view,label,value){const input=view.find(label);assert.ok(input,`field ${label}`);return view.act(()=>input.props.onChange(event(input,{target:{value}})));}
export async function select(view,label,match){const input=view.find(label);assert.ok(input,`select ${label}`);const values=input.props.children.flat(Infinity).filter(v=>v?.type==='option');const option=values.find(o=>typeof match==='function'?match(view.textOf(o)):view.textOf(o)===match);assert.ok(option,`option ${match}`);return view.act(()=>input.props.onChange(event(input,{target:{value:String(option.props.value)}})));}
export async function apply(view){await tab(view,'检查变更');assert.ok(view.find('业务变更预览'),view.text());await tab(view,'确认应用草稿');assert.ok(!view.find('业务变更预览'));}
export async function runSaved(session){await tab(session.view,'从初始输入重跑');assert.ok(session.run);return session.run;}
export async function artifact(name,value){await writeFile(new URL(name,out),JSON.stringify(value,null,2));}
