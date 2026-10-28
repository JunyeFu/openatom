/// <reference types="vite/client" />
// Test-only HTTP byte provider and isolated u clock; never imported by the product.
import {useEffect,useMemo,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {MediaPresentationView,MEDIA_DATA_FIELD_CATALOG,type MediaPresentationProps,type MediaPresentationCommonProps} from './index';
import type {ActualMediaFrame} from '../../../../../packages/presentation/src';
import type {ModulePackage} from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type {ImageTransition} from '../../../../../packages/commands/generated/typescript/ImageTransition';
import imageJson from '../../../../../modules/media/p302-controlled/image.module.json';
import modelJson from '../../../../../modules/media/p302-controlled/model.module.json';
import gifJson from '../../../../../modules/media/p302-controlled/gif.module.json';
import prefixJson from '../../../../../scenarios/媒体表现/runtime/shared-preview/actual-prefix.json';
import framesJson from '../../../../../scenarios/媒体表现/runtime/actual-frames.json';
import redUrl from '../../../../../tests/planner/media/fixtures/red.png?url';
import blueUrl from '../../../../../tests/planner/media/fixtures/blue.png?url';

const packages={image:imageJson,model:modelJson,gif:gifJson} as unknown as Record<'image'|'model'|'gif',ModulePackage>;
const red={id:'p302.test.red',path:'fixtures/red.png',kind:'png'} as const,blue={id:'p302.test.blue',path:'fixtures/blue.png',kind:'png'} as const;
const urls=import.meta.glob('../../../../../modules/media/p302-controlled/*.{png,jpg,gif,glb,svg}',{query:'?url',import:'default',eager:true}) as Record<string,string>;
const readAsset:MediaPresentationProps['readAsset']=async request=>{
  const fixture=request.asset.id===red.id&&request.asset.path===red.path&&request.asset.kind===red.kind?redUrl:request.asset.id===blue.id&&request.asset.path===blue.path&&request.asset.kind===blue.kind?blueUrl:null;
  const exact=imageJson.assets.find(a=>a.id===request.asset.id&&a.path===request.asset.path&&a.kind===request.asset.kind);
  const url=fixture??(exact?Object.entries(urls).find(([path])=>path.endsWith('/'+exact.path))?.[1]:null);
  if(!url)throw new Error('MissingReference: exact controlled byte fixture missing');
  const response=await fetch(url);if(!response.ok)throw new Error(`controlled HTTP bytes ${response.status}`);return response.arrayBuffer();
};
const actualFrames=[{label:'独立Engine前缀@20750 (u=0.525)',frame:prefixJson as unknown as ActualMediaFrame},...(framesJson as unknown as {label:string;frame:ActualMediaFrame}[]).filter(x=>['calendar@35000','calendar@40000','interrupt@20000','interrupt-paused@20000','output@100000','zero@0'].includes(x.label))];
function Harness(){
  const [mode,setMode]=useState<'declared_preview'|'actual_run'>('declared_preview'),[kind,setKind]=useState<'colors'|'photos'|'model'|'gif'>('colors'),[transition,setTransition]=useState<ImageTransition>('crossfade'),[phase,setPhase]=useState('auto'),[u,setU]=useState(.525),[index,setIndex]=useState(0),[pixel,setPixel]=useState('未读取'),[action,setAction]=useState('无'),[playback,setPlayback]=useState({playing:false,rate:1,loop:false});
  const progress=useRef(u);progress.current=u;
  useEffect(()=>setPixel('未读取当前画布'),[mode,kind,transition,phase,u,index]);
  useEffect(()=>{
    if(mode!=='declared_preview'||!playback.playing)return;
    let last=performance.now(),handle=0;
    const tick=(now:number)=>{const next=progress.current+(now-last)*playback.rate/2000;last=now;if(next>=1&&!playback.loop){setU(1);setPlayback(v=>({...v,playing:false}));return;}setU(playback.loop?next%1:next);handle=requestAnimationFrame(tick);};
    handle=requestAnimationFrame(tick);return()=>cancelAnimationFrame(handle);
  },[mode,playback.playing,playback.rate,playback.loop]);
  const pkg=packages[kind==='model'?'model':kind==='gif'?'gif':'image'];
  const presentation=useMemo(()=>{
    if(pkg.definition.payload.category!=='media')throw Error('media fixture required');
    const value=structuredClone(pkg.definition.payload.data);
    if(kind==='photos'||kind==='colors'){
      const first=kind==='colors'?red:packages.image.assets.find(a=>a.path==='station.png')!,last=kind==='colors'?blue:packages.image.assets.find(a=>a.path==='station-end.png')!;
      value.timeline=['load','auto','unload','cycle'].flatMap(phase_ref=>[{id:`${phase_ref}-before`,phase_ref,start_progress:0,end_progress:.5,source:{kind:'image_asset' as const,data:first},transition:'hold' as const},{id:`${phase_ref}-after`,phase_ref,start_progress:.5,end_progress:1,source:{kind:'image_asset' as const,data:last},transition}]);
    }
    value.tags=[...value.tags.filter(tag=>tag.payload.kind!=='data'),...MEDIA_DATA_FIELD_CATALOG.map((field,i)=>({id:`field-${i}`,label:field.label,anchor:{kind:'viewport' as const,data:{x:.18,y:.12+i*.1}},payload:{kind:'data' as const,data:{...field.binding}}}))];
    return value;
  },[pkg,kind,transition]);
  const frame=actualFrames[index].frame;
  const common:MediaPresentationCommonProps={presentation,readAsset,rights:kind==='colors'?[red,blue].map(asset=>({asset_ref:asset.id,source:'Original solid-color pixel probes; tests/planner/media/fixtures/LICENSE.txt',license:'CC0-1.0',redistribution_confirmed:true})):pkg.rights,lod:'detailed',binding:{session_id:'p302-shared-test',request:{project_id:'P1',scope:{kind:'project'},scope_revision:1,source:{kind:'module_version',data:{module_ref:{id:pkg.definition.id,version:pkg.definition.version}}}}},onAction:action=>setAction(action)};
  const input:MediaPresentationProps=mode==='declared_preview'?{...common,mode,preview:{phase_ref:phase,view_u:u},onPreviewSeek:value=>{setPhase(value.phase_ref);setU(value.view_u);},playback,onPlaybackChange:setPlayback}:{...common,mode,frame,onSeek:at=>setAction(`实际Host任意时点读取尚待Root；测试只提供捕获边界：${at}ms`)};
  return <main className="sf-shared-harness"><h1>共享媒体 · 声明预览与实际状态</h1><p>独立验证页。声明循环为2秒／阶段，只改变观看u；实际状态来自唯一Engine有限窗口，媒体为显式测试声明，均不冒称Host/Native接收。</p><div className="sf-shared-controls"><label>模式 <select aria-label="呈现模式" value={mode} onChange={e=>{setMode(e.target.value as typeof mode);setPlayback(v=>({...v,playing:false}));}}><option value="declared_preview">声明预览</option><option value="actual_run">实际Engine状态</option></select></label><label>素材 <select aria-label="共享素材" value={kind} onChange={e=>setKind(e.target.value as typeof kind)}>{['colors','photos','gif','model'].map(k=><option key={k}>{k}</option>)}</select></label><label>图片过渡 <select aria-label="图片过渡" value={transition} disabled={kind==='model'||kind==='gif'} onChange={e=>setTransition(e.target.value as ImageTransition)}>{['hold','crossfade','pan','zoom'].map(t=><option key={t}>{t}</option>)}</select></label>{mode==='declared_preview'?<label>阶段 <select aria-label="声明阶段" value={phase} onChange={e=>setPhase(e.target.value)}>{['load','auto','unload','cycle'].map(p=><option key={p}>{p}</option>)}</select></label>:<label>实际源 <select aria-label="实际边界" value={index} onChange={e=>setIndex(Number(e.target.value))}>{actualFrames.map((entry,i)=><option key={i} value={i}>{entry.label}</option>)}</select></label>}{[0,.499,.5,.525,.55,1].map(value=><button key={value} disabled={mode!=='declared_preview'} onClick={()=>setU(value)}>u={value}</button>)}</div>
    <MediaPresentationView {...input}/>
    <div className="sf-shared-controls"><button disabled={kind==='model'} onClick={()=>{const canvas=document.querySelector('.sf-media-canvas canvas') as HTMLCanvasElement;const rgba=canvas.getContext('2d')!.getImageData(Math.floor(canvas.width/2),Math.floor(canvas.height/2),1,1).data;setPixel(JSON.stringify(Array.from(rgba)));}}>读取画布中心像素</button><output aria-label="中心像素">{pixel}</output><span>显式动作：{action}</span></div>
    <h2>共享字段目录 · 实际能力</h2><p className="sf-shared-catalog-hint">窄屏可横向滚动字段表，查看单位与限制。</p><div className="sf-shared-catalog" tabIndex={0} aria-label="共享字段目录，可横向滚动"><table><thead><tr><th>字段</th><th>默认类型／单位</th><th>限制</th></tr></thead><tbody>{MEDIA_DATA_FIELD_CATALOG.map(field=><tr key={field.binding.source_field}><td>{field.label}<br/><code>{field.binding.source_field}</code></td><td>{field.binding.value_type}／{field.binding.unit??'无'}</td><td>{field.limitations}</td></tr>)}</tbody></table></div>
  </main>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Harness/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
