// Controlled real component with frozen actual Engine data. No Tauri/Host mock.
import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import {RunTimeline} from '../../../apps/planner/src/features/timeline/RunTimeline';
import * as projection from '../../../apps/planner/src/features/timeline/projection';
import * as time from '../../../apps/planner/src/features/timeline/time';
import * as input from '../../../apps/planner/src/features/timeline/input';
import * as preferences from '../../../apps/planner/src/features/timeline/preferences';
import recorded from '../../../packages/reporting/tests/fixtures/evaluation-acceptance/actual-engine-inputs.json';
import '../../../apps/planner/src/styles/workbench.css';
const data:any=recorded;
function freeze(value:any):any{if(value&&typeof value==='object'){Object.values(value).forEach(freeze);Object.freeze(value);}return value;}
freeze(data);
const params=new URLSearchParams(location.search),caseName=params.get('case')??'ix_late';
const [project,scenario,,actual]=data.inputs[caseName];
const run=params.has('design')?null:actual;
const callbacks:any[]=[];
let selected:any=null;
function App(){
 const [at,setAt]=useState<number|null>(params.has('prefix')?null:370000);
 const [prefs,setPrefs]=useState(preferences.initialPreferences());
 const values=data.observations[caseName]?.[String(at)];
 const observation=run&&values&&!params.has('missing')?{run_id:run.id,at_ms:at,objects:values.map((value:any)=>({kind:'item',data:value})),next_index:null}:null;
 Object.assign(window,{__w5timeline:{data,project,scenario,run,projection,time,input,preferences,callbacks,read:()=>({selected,at,prefs,unchanged:JSON.stringify(data)})}});
 const compare=params.has('compare')?data.inputs.ix_base:null;
 return <main style={{height:'100dvh',display:'flex',flexDirection:'column'}}><header style={{padding:12}}><strong>实际时间轴组件 · 冻结 Engine 输入</strong><span> · 独立消费，未连接 Native Host</span></header><div style={{flex:1,minHeight:0}}>
 <RunTimeline project={project} draftProject={project} scenario={scenario} runScenario={scenario} run={run} atMs={at}
 observation={observation} preferences={prefs} onPreferencesChange={setPrefs}
 comparison={compare?{project:compare[0],scenario:compare[1],run:compare[3]}:undefined}
 onTime={value=>{callbacks.push({kind:'time',value});setAt(value);}}
 onSeek={request=>{callbacks.push({kind:'seek',...request});setAt(request.at_ms);}}
 onInspect={value=>{selected=value;callbacks.push({kind:'inspect',value});}}
 onSelect={value=>{callbacks.push({kind:'select',value});}}
 />
 </div></main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
