import React,{useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {FactoryScene} from '../../apps/planner/src/scene/FactoryScene';
import {projectFactoryPresentation,type FactoryPresentationSource} from '../../apps/planner/src/services/factoryPresentation';
import {deviceHallPosition} from '../../apps/planner/src/scene/layout';
import '../../apps/planner/src/styles/workbench.css';
const read=async(name:string)=>{const r=await fetch('/proof/'+name);if(!r.ok)throw new Error(await r.text());return r.json();};
const [project,scenario,run,frames,packages]=await Promise.all(['actual-frozen-project.json','actual-scenario.json','actual-connected-run.json','actual-observation-frames.json','actual-frozen-modules.json'].map(read));
const modules=new Map(packages.map((p:any)=>[JSON.stringify([p.definition.id,p.definition.version]),p]));
const caseBack=new URLSearchParams(window.location.search).get('case-side')==='back';
function App(){
 const [index,setIndex]=useState(0),[source,setSource]=useState<FactoryPresentationSource|null>(null),[error,setError]=useState(''),[selection,setSelection]=useState<any>(null),[lod,setLod]=useState<'detailed'|'simplified'>('simplified'),[focus,setFocus]=useState<'transport'|'case'|'bag'|'device'|null>(null),[focusDevice,setFocusDevice]=useState('');
 const page=frames[index];
 const loading=!source||source.observation?.at_ms!==page.at_ms;
 useEffect(()=>{let live=true;setError('');
  const context={session_id:'actual-host-export',project_id:project.id,scope:{kind:'project' as const},scope_revision:project.revision,project,view:{run_id:run.id,at_ms:page.at_ms}};
  projectFactoryPresentation(context,{project,scenario,run,observation:page,jobs:page.objects.filter((o:any)=>o.kind==='job').map((o:any)=>o.data),items:page.objects.filter((o:any)=>o.kind==='item').map((o:any)=>o.data),raw:{job_pages:[page],item_pages:[page]}},{
   async module(ref){const value=modules.get(JSON.stringify([ref.id,ref.version]));if(!value)throw new Error('Missing actual frozen module '+ref.id);return value as any;},
   async readAsset(request){const ref=request.source.data.module_ref;const r=await fetch('/proof-asset?'+new URLSearchParams({id:ref.id,version:ref.version,path:request.asset.path}));if(!r.ok)throw new Error(await r.text());return r.arrayBuffer();},
  },()=>{if(!live)throw new Error('Frame changed');}).then(s=>{if(live){setSource(s);(window as any).factoryProof=s;}},e=>{if(live)setError(String(e));});
  return()=>{live=false;};
 },[index]);
 const box=source?.items.find(i=>i.item.item_type_ref.id==='PF-packed-case'&&!['transformed','scrapped'].includes(i.item.lifecycle));
 const owner=box?.attachment?.owner_ref;
 const boxPoint=owner?.kind==='location'?project.locations.find((l:any)=>l.id===owner.id)?.position.translation
  :owner?.kind==='transport'?source?.transports?.find(t=>t.trip.id===owner.id)?.pose?.translation
  :owner?.kind==='device'?deviceHallPosition(project,project.devices.find((d:any)=>d.id===owner.id)):null;
 const transportPoint=source?.transports?.find(t=>t.trip.phase==='outbound'&&t.pose)?.pose?.translation??source?.transports?.find(t=>t.pose)?.pose?.translation;
 const bag=source?.items.find(i=>i.item.item_type_ref.id==='PF-BAG-finished-bag'&&i.item.lifecycle==='available');
 const bagLocation=bag?.attachment?.owner_ref.kind==='location'?project.locations.find((l:any)=>l.id===bag.attachment!.owner_ref.id):null;
 const bagPoint=bagLocation&&bag?.attachment?{x:bagLocation.position.translation.x+bag.attachment.slot.offset.translation.x,y:bagLocation.position.translation.y+bag.attachment.slot.offset.translation.y,z:bagLocation.position.translation.z+bag.attachment.slot.offset.translation.z+.15}:null;
 const device=project.devices.find((d:any)=>d.id===focusDevice),deviceOrigin=device&&deviceHallPosition(project,device);
 const devicePoint=deviceOrigin?{x:deviceOrigin.x,y:deviceOrigin.y,z:deviceOrigin.z+(device.height_m??0)/2}:null;
 const deviceSpan=device?Math.max(device.height_m??0,...device.footprint_m.map((p:any)=>Math.hypot(p.x,p.y)),1):1;
 const point=focus==='device'?devicePoint:focus==='bag'?bagPoint:focus==='case'?boxPoint:focus==='transport'?transportPoint:null;
 const close=()=>{(window as any).factoryCapture=null;(window as any).factoryCaptureFailure=null;};
 return <div style={{height:'100vh',display:'flex',flexDirection:'column'}}>
  <header style={{padding:'12px 20px',display:'flex',gap:16,alignItems:'center',background:'#fff',borderBottom:'1px solid #d8e0e8'}}><strong>纸包装整厂 · 92台设备</strong><label>运行时点 <select aria-label="运行时点" value={index} onChange={e=>{close();setFocus(null);setIndex(Number(e.target.value));}}>{frames.map((f:any,i:number)=><option key={i} value={i}>{(f.at_ms/1000).toFixed(1)}秒</option>)}</select></label><label>模型 <select aria-label="模型精度" value={lod} onChange={e=>setLod(e.target.value as any)}><option value="simplified">简化</option><option value="detailed">详细</option></select></label><button disabled={loading||!transportPoint} onClick={()=>{close();setFocus(focus==='transport'?null:'transport');}}>{focus==='transport'?'工厂全景':'运输近景'}</button><button disabled={loading||!boxPoint} onClick={()=>{close();setFocus(focus==='case'?null:'case');}}>{focus==='case'?'工厂全景':'箱件近景'}</button><span data-proof-status>{error||loading?'读取中':`在途 ${source.transports?.filter(t=>t.pose).length??0} · 实际物品 ${source.items.length}`}</span></header>
  <p style={{margin:0,padding:'6px 20px',fontSize:12,color:'#536776'}}>真实Host归档的只读渲染检查；端口计数未接入此检查页面。<button disabled={loading||!bagPoint} onClick={()=>{close();setFocus(focus==='bag'?null:'bag');}}>纸袋近景</button><select aria-label="近景设备" value={focusDevice} onChange={e=>{close();setFocus(null);setFocusDevice(e.target.value);setSelection(e.target.value?{kind:'device',id:e.target.value}:null);}}><option value="">选择设备</option>{project.devices.map((d:any)=><option key={d.id} value={d.id}>{d.title}</option>)}</select><button disabled={loading||!devicePoint} onClick={()=>{close();setFocus(focus==='device'?null:'device');}}>设备近景</button></p>
  {error&&<p role="alert">{error}</p>}
  <div style={{flex:1,minHeight:0,position:'relative'}}>{source&&<FactoryScene project={project} canvas="line_canvas" hallId="PF-H1" selection={selection} onSelect={setSelection} lod={lod} autoLod={false} resetKey={0} mediaSource={source} capture={point?{
   key:focus+'-close-'+(focus==='device'?focusDevice+'-':caseBack?'back-':'')+page.at_ms,camera:{coordinate_frame:'right_handed_z_up',unit:'meter',eye:focus==='device'?{x:point.x+deviceSpan*.9,y:point.y-deviceSpan,z:point.z+deviceSpan*.7}:focus==='bag'?{x:point.x+.45,y:point.y-.55,z:point.z+.35}:focus==='case'?{x:point.x+1.1,y:point.y+(caseBack?1.2:-1.2),z:point.z+.9}:{x:point.x+7,y:point.y-8,z:point.z+5},target:point,zoom:1},
   onFrameReady:frame=>{(window as any).factoryCapture=frame;},onFailure:failure=>{(window as any).factoryCaptureFailure=failure;},
  }:undefined}/>}</div>
 </div>;
}
createRoot(document.getElementById('root')!).render(<App/>);
