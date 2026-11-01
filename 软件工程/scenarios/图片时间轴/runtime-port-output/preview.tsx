import React,{useState} from 'react';import {createRoot} from 'react-dom/client';
import {DeviceProcessWorkspace} from '../../../apps/planner/src/features/factory/DeviceProcessWorkspace';
import {dispatch,HostFailure} from '../../../apps/planner/src/services/projectHost';
import {createAuthoringSamples} from '../../../apps/planner/src/services/moduleAuthoring';
import type {AuthoringIO,AuthoringDraft} from '../../../apps/planner/src/features/modules/types';
const ready=await fetch('/__p303_port_core?bootstrap').then(r=>r.json()),calls:any[]=[],commits:any[]=[];
const rpc=(query:unknown)=>fetch('/__p303_port_core',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(query)}).then(r=>r.json());
let holdNext=false,release:(()=>void)|null=null,held=false;
// Private browser support transport to the real same ProjectHost. This is not Native IPC.
(window as any).__TAURI_INTERNALS__={async invoke(command:string,args:{request:any}){
 const request=args.request;calls.push({command,request});
 if(command==='project_api')return rpc({kind:'dispatch',request});
 if(command==='project_read_media_asset'){const reply=await rpc({kind:'media',...request});if(!reply.ok)throw reply.error;return Uint8Array.from(reply.bytes).buffer;}
 if(command==='project_read_run_port_output'){const reply=await rpc({kind:'port_output',...request});if(holdNext){holdNext=false;held=true;await new Promise<void>(resolve=>{release=resolve;});}if(!reply.ok)throw reply.error;return reply.data;}
 throw new Error(`Private browser transport has no ${command}`);
}};
const io:AuthoringIO={dispatch,async awaitTask(task,onState){const scope=task.scope.kind==='edit'&&task.scope.data.kind==='definition_draft'?{kind:'definition_draft',data:task.scope.data.data.id}:{kind:'project'};const reply=await rpc({kind:'await_task',scope,task_id:task.id});if(reply.outcome.status!=='ok')throw new HostFailure(reply.outcome.data);const state=reply.outcome.data;onState?.(state);if(state.task.state!=='completed')throw new Error(state.failure?.message??state.task.state);return state.result;},async pickDirectory(){throw new Error('Native directory picker is not operated in this browser check');}};
function App(){
 const [draft,setDraft]=useState<AuthoringDraft>(ready.draft),[run,setRun]=useState(ready.formal_run),[at,setAt]=useState(75000),[scope,setScope]=useState<{kind:'project'}|{kind:'definition_draft';data:string}>({kind:'project'}),[visible,setVisible]=useState(true);
 const workspace={context:ready.context,capabilities:ready.capabilities,...ready.snapshot,modules:new Map([[JSON.stringify(ready.draft.summary.definition_ref),ready.package]]),moduleIssues:[],scenarioIssues:[]};
 const job=run.jobs.find((job:any)=>job.task_ref==='TASK-LOCAL')??run.jobs.find((job:any)=>job.execution_ref.kind==='definition_cycle');
 const samples=createAuthoringSamples(()=>workspace,draft,io);
 (window as any).__port={read:()=>({draft,run,scope,at,held,calls,commits,visible}),seek:setAt,hold(){holdNext=true;held=false;},release(){release?.();release=null;},formal(){setRun(ready.formal_run);setScope({kind:'project'});setAt(75000);}};
 return <main style={{height:'100vh'}}><p style={{margin:0}}>实际同Host getter / 真实组件 / 私有浏览器传输，未操作Native</p>{visible&&<DeviceProcessWorkspace workspace={workspace} scope={scope} scopeRevision={scope.kind==='project'?workspace.project.revision:draft.summary.revision} selection={{kind:'actual_run',run_id:run.id,job_id:job.id,at_ms:at}} onSelectionChanged={selection=>{if(selection.kind==='actual_run')setAt(selection.at_ms);}} actions={{}} editor={{draft,source:ready.package,io,samples,scenario:workspace.scenarios[0],actions:{},onDraftChanged:setDraft,onCommitted:commit=>{commits.push(commit);},onSampleRun:value=>{setRun(value);setScope({kind:'definition_draft',data:draft.summary.draft_id});setAt(0);}}} onClose={()=>setVisible(false)}/>}</main>;
}
const target=window as any;target.__portRoot??=createRoot(document.getElementById('root')!);target.__portRoot.render(<React.StrictMode><App/></React.StrictMode>);
