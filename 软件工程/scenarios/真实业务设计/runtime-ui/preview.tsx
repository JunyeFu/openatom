// Actual Core consumer over a test-only browser route; no Host/Engine doubles.
import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import {RunTimeline} from '../../../apps/planner/src/features/timeline/RunTimeline';
import type {BusinessEditIntent} from '../../../apps/planner/src/features/business/types';
import {BusinessEditor} from '../../../apps/planner/src/features/business/BusinessEditor';
import type {BusinessCommitProposal,BusinessRunRequest} from '../../../apps/planner/src/features/business/types';
const bootstrap=await fetch('/__p205_core?bootstrap=1').then(r=>r.json());
const evidence={commits:[] as BusinessCommitProposal[],runs:[] as BusinessRunRequest[],locates:[] as object[],dirty:[] as object[],replies:[] as object[],oldRun:bootstrap.run,initial:bootstrap.state};
(window as unknown as {__businessEvidence:typeof evidence}).__businessEvidence=evidence;
const rpc=(value:unknown)=>fetch('/__p205_core',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(value)}).then(r=>r.json());
function App(){
 const [state,setState]=useState(bootstrap.state),[run,setRun]=useState(bootstrap.run),[frozenProject,setFrozenProject]=useState(bootstrap.frozenProject),[frozenScenario,setFrozenScenario]=useState(bootstrap.frozenScenario),[scenario,setScenario]=useState(bootstrap.state.snapshot.scenarios[0]?.id??null),[observation,setObservation]=useState(null),[visible,setVisible]=useState(true);
 (window as unknown as {__businessRead:()=>unknown}).__businessRead=()=>({state,run,scenario});
 const [editIntent,setEditIntent]=useState<BusinessEditIntent>();
 (window as unknown as {__businessOpenIntent:(request:BusinessEditIntent)=>void}).__businessOpenIntent=request=>{setVisible(true);setEditIntent(request);};
 async function commit(p:BusinessCommitProposal){evidence.commits.push(p);const reply=await rpc({kind:'commit',proposal:p});evidence.replies.push(reply);if(reply.ok)setState({snapshot:reply.snapshot,modules:reply.modules,history:reply.history});return reply;}
 const modules=new Map(state.modules.map((m:any)=>[JSON.stringify([m.definition.id,m.definition.version]),m]));
 return <main style={{height:'100vh',display:'flex',flexDirection:'column',font:'14px Segoe UI,Microsoft YaHei,sans-serif',background:'#eef4f8'}}><header style={{padding:8,borderBottom:'1px solid #c0d2dd'}}><strong>P2-05 · 实际ProjectHost / Commands / Engine消费者</strong> · 独立组件核验，不是Native接收 <span data-testid="revision">r{state.snapshot.project.revision}</span> <span data-testid="run-id">{run?.id??'无实际结果'}</span><button onClick={async()=>{const next=await rpc({kind:'concurrent_edit',scenario_id:scenario});evidence.replies.push(next);setState(next);}}>制造真实并发编辑</button><button onClick={async()=>{const next=await rpc({kind:"concurrent_edit",field:"due",scenario_id:scenario});evidence.replies.push(next);setState(next);}}>制造同字段并发编辑</button><button onClick={async()=>{const next=await rpc({kind:"new_project"});evidence.replies.push(next);setState(next);setScenario(null);setRun(null);setFrozenProject(null);setFrozenScenario(null);setObservation(null);}}>测试明确切换新工程</button><button onClick={()=>setVisible(true)}>重新打开业务</button></header>
 {visible&&<BusinessEditor editIntent={editIntent} project={state.snapshot.project} scenarios={state.snapshot.scenarios} modules={modules as any} scenarioId={scenario} history={state.history} run={run} frozenProject={frozenProject} frozenScenario={frozenScenario} atMs={run?.computed_until_ms??null} observation={observation}
 canEditSetupPolicies={bootstrap.setup_policy_write_supported===true}
 onCommit={commit}
 onRefresh={async()=>{setState(await rpc({kind:'refresh'}));}}
 onRun={async request=>{evidence.runs.push(request);const reply=await rpc({kind:'run',request});evidence.replies.push(reply);setRun(reply.run);setFrozenProject(reply.frozenProject);setFrozenScenario(reply.frozenScenario);setState(reply.state);setObservation(await rpc({kind:'observe',run_id:reply.run.id,at_ms:reply.run.computed_until_ms}));}}
 onScenarioChange={setScenario} onLocate={value=>evidence.locates.push(value)} onDirtyChange={value=>evidence.dirty.push(value)} onClose={()=>setVisible(false)}/>}
 {new URLSearchParams(location.search).has('shared-timeline')&&<div style={{height:270,flex:'none'}}><RunTimeline project={state.snapshot.project} draftProject={state.snapshot.project} scenario={state.snapshot.scenarios.find((s:any)=>s.id===scenario)??null} run={null} atMs={null} onTime={()=>{}} onSelect={object=>evidence.locates.push({object,source:'draft'})}
 onEditBusiness={request=>{setVisible(true);setEditIntent({...request,id:crypto.randomUUID()});}}
 onCommitInput={async proposal=>{const reply=await commit({project_id:proposal.project_id,base_project_revision:proposal.base_project_revision,scenario_revisions:{[proposal.scenario_id]:proposal.base_scenario_revision},label:'同源时间轴编辑',commands:[proposal.command]});return reply.ok?{ok:true,scenario:reply.snapshot.scenarios.find((s:any)=>s.id===proposal.scenario_id)}:reply;}}/></div>}
 </main>;
}
const host=window as unknown as {__p205Root?:ReturnType<typeof createRoot>};host.__p205Root??=createRoot(document.getElementById('root')!);host.__p205Root.render(<App/>);
