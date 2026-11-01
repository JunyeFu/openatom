import React,{useEffect,useMemo,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {MediaPresentationView} from '../../../apps/planner/src/features/media/MediaPresentationView';
import {readProcessItems} from '../../../apps/planner/src/features/media/processItems';
import {projectFactoryPresentation} from '../../../apps/planner/src/services/factoryPresentation';
import {readMediaAsset} from '../../../apps/planner/src/services/mediaAssets';
import type {FactoryPresentationPorts,FactoryDeviceMedia} from '../../../apps/planner/src/services/factoryPresentation';
import type {MediaPresentationCommonProps} from '../../../apps/planner/src/features/media/types';
const labels=['l03-complete','phase-interrupted','phase-resource-wait','l03-robot-output-wait','l03-pallet-output-wait','l03-missing-pcb'];
function App(){
 const [label,setLabel]=useState(labels[0]),[time,setTime]=useState(43000),[device,setDevice]=useState('R01'),[lod,setLod]=useState<'detailed'|'simplified'|'region'>('detailed');
 const [input,setInput]=useState<any>(null),[source,setSource]=useState<FactoryDeviceMedia[]>([]),[error,setError]=useState(''),[selection,setSelection]=useState('');
 const [playback,setPlayback]=useState({playing:false,rate:1,loop:false});
 const [captureMode,setCaptureMode]=useState(false),[holdItems,setHoldItems]=useState(false),[captured,setCaptured]=useState<any[]>([]);
 const heldReads=useRef<Array<()=>void>>([]),holdCurrent=useRef(holdItems);holdCurrent.current=holdItems;
 const held=useRef<{key:string;media:NonNullable<FactoryDeviceMedia['media']>}|null>(null);
 const currentLabel=useRef(label);currentLabel.current=label;
 const ports=useMemo(()=>{
  async function read(method:string,request:unknown,binary=false){
   const response=await fetch('/__cu__/read',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({label,method,request})});
   if(!response.ok)throw Error(await response.text());return binary?response.arrayBuffer():response.json();
  }
  return {dispatch:request=>read('object',request),readObservation:request=>read('observation',request),readAsset:request=>read('bytes',request,true)} as FactoryPresentationPorts;
 },[label]);
 useEffect(()=>{
  let live=true;if(input?.label!==label)setSource([]);setError('');
  fetch('/__cu__/input?label='+label+'&at='+time).then(r=>r.json()).then(async data=>{
   const modules=new Map(data.modules.map((p:any)=>[JSON.stringify([p.definition.id,p.definition.version]),p]));
   const result=await projectFactoryPresentation({session_id:'captured-'+label,project_id:data.project.id,scope:{kind:'project'},scope_revision:data.scope_revision,
    project:data.project,view:{run_id:data.run.id,at_ms:data.observation.at_ms}},
    {project:data.project,run:data.run,observation:data.observation,jobs:data.observation.objects.flatMap((o:any)=>o.kind==='job'?[o.data]:[]),items:data.observation.objects.flatMap((o:any)=>o.kind==='item'?[o.data]:[]),raw:{job_pages:[],item_pages:[]}},
    {module:async ref=>modules.get(JSON.stringify([ref.id,ref.version])) as any,readAsset:ports.readAsset},()=>{if(currentLabel.current!==label)throw Error('captured source changed');});
   if(live){setInput({...data,label});setSource(result.devices);}
  }).catch(e=>{if(live)setError(String(e));});return()=>{live=false;};
 },[label,time,ports]);
 const selected=input?.label===label?source.find(x=>x.device_id===device&&x.media&&x.frame):undefined,key=label+'/'+device;
 if(selected?.media&&held.current?.key!==key)held.current={key,media:selected.media};
 const media=selected?.media&&held.current?.key===key?held.current.media:null;
 const readItems=useMemo<NonNullable<MediaPresentationCommonProps['readItems']>>(()=>
  async(frame,binding,current)=>{if(holdCurrent.current)await new Promise<void>(resolve=>heldReads.current.push(resolve));return readProcessItems(frame,binding,current,ports);},[ports]);
 const capture=useMemo(()=>captureMode?{key:`cu-${label}-${device}-${time}-${lod}`,object:{kind:'device' as const,id:device},
  camera:{eye:{x:3,y:2.5,z:3},target:{x:0,y:.8,z:0},zoom:1,unit:'meter' as const,coordinate_frame:'right_handed_y_up' as const},
  onFrame:(value:any)=>setCaptured(old=>[...old,value]),onFailure:(message:string)=>setCaptured(old=>[...old,{failure:message}])}:undefined,[captureMode,label,device,time,lod]);
 return <main><h1>P3-02 设备详情实际附件验证</h1><p>输入来自独立新进程ProjectHost读回；本页显式注入捕获的只读端口，未模拟Native、生产规则或库存。</p>
  <nav><label>情景 <select aria-label="情景" value={label} onChange={e=>{setLabel(e.target.value);setTime(0);setDevice(e.target.value.startsWith('phase-')?'CNC':'R01');setSelection('');}}>{labels.map(x=><option key={x}>{x}</option>)}</select></label>
   <label>实际时点 <select aria-label="实际时点" value={time} onChange={e=>setTime(Number(e.target.value))}>{(input?.times??[43000]).map((t:number)=><option key={t} value={t}>{t}</option>)}</select></label>
   <label>设备 <select aria-label="设备" value={device} onChange={e=>setDevice(e.target.value)}>{(input?.project.devices??[]).map((d:any)=><option key={d.id}>{d.id}</option>)}</select></label>
   <label>档位 <select aria-label="档位" value={lod} onChange={e=>setLod(e.target.value as typeof lod)}>{['detailed','simplified','region'].map(x=><option key={x}>{x}</option>)}</select></label>
   <label><input type="checkbox" aria-label="暂挂附件读取" checked={holdItems} onChange={e=>{setHoldItems(e.target.checked);if(!e.target.checked){const pending=heldReads.current;heldReads.current=[];pending.forEach(resolve=>resolve());}}}/>暂挂附件读取</label>
   <label><input type="checkbox" aria-label="实际帧完成回调" checked={captureMode} onChange={e=>{setCaptured([]);setCaptureMode(e.target.checked);}}/>实际帧完成回调</label></nav>
  {error?<p role="alert">{error}</p>:selected?.frame&&media?<MediaPresentationView {...media} mode="actual_run" frame={selected.frame} lod={lod} readItems={readItems}
   capture={capture} playback={playback} onPlaybackChange={setPlayback} onSeek={at=>{if(input.times.includes(at))setTime(at);}}
   onInspectItem={id=>setSelection(id)} onSelectNode={n=>setSelection('node '+n.node_index)}/>:<p role="status">此时点该设备没有唯一实际Job姿态，未推测附件。</p>}
  <output aria-label="已选实际身份">{selection||'未选'}</output><output aria-label="实际帧回调记录">{JSON.stringify(captured)}</output></main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
