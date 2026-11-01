/// <reference types="vite/client" />
// Independent fixture harness. Never imported into the Root product graph.
import {useMemo,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {MediaPresentationView,type MediaPresentationProps,type MediaPresentationCommonProps} from './index';
import type {ActualMediaFrame} from '../../../../../packages/presentation/src';
import type {ModulePackage} from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type {Versioned} from '../../../../../packages/commands/generated/typescript/Versioned';
import framesJson from '../../../../../scenarios/媒体表现/runtime/actual-frames.json';
import prefixJson from '../../../../../scenarios/媒体表现/runtime/shared-preview/actual-prefix.json';
import './line-grade-harness.css';
const packageDocuments=import.meta.glob('../../../../../modules/media/p302-line-grade/installable/*/module.json',{eager:true,import:'default'}) as Record<string,Versioned<ModulePackage>>;
const packageFiles=Object.fromEntries(Object.entries(packageDocuments).map(([path,document])=>[path,document.data]));
const bytes=import.meta.glob('../../../../../modules/media/p302-line-grade/installable/*/*.{glb,png}',{eager:true,query:'?url',import:'default'}) as Record<string,string>;
const packages=Object.values(packageFiles);
const readAsset:MediaPresentationProps['readAsset']=async request=>{
  const ref=request.source.data.module_ref,pkg=packages.find(p=>p.definition.id===ref.id&&p.definition.version===ref.version);
  const asset=pkg?.assets.find(a=>a.id===request.asset.id&&a.path===request.asset.path&&a.kind===request.asset.kind);
  const entry=Object.entries(packageFiles).find(([,p])=>p===pkg);
  if(!entry||!asset?.path)throw Error('MissingReference: exact line-grade package asset not declared');
  const key=entry[0].replace(/module.json$/,'')+asset.path,url=bytes[key];if(!url)throw Error('MissingReference: exact line-grade fixture bytes missing');
  const response=await fetch(url);if(!response.ok)throw Error(`line-grade fixture HTTP ${response.status}`);return response.arrayBuffer();
};
const resolveModule:NonNullable<MediaPresentationProps['resolveModule']>=async source=>{const r=source.data.module_ref,p=packages.find(p=>p.definition.id===r.id&&p.definition.version===r.version);if(!p)throw Error('exact line-grade module not registered in fixture');return p;};
const frames=[{label:'独立 Engine 前缀 @20750 / .525',frame:prefixJson as unknown as ActualMediaFrame},...(framesJson as unknown as {label:string;frame:ActualMediaFrame}[]).filter(e=>['calendar@35000','calendar@40000','interrupt@20000','interrupt-paused@20000','output@100000','missing@100000','zero@0'].includes(e.label))];
const families=['assembly','rework','packing','robot','conveyor','inspection','sealer','pallet','control-box','carton'];
function Harness(){
  const [family,setFamily]=useState('assembly'),[assetKind,setAssetKind]=useState('model'),[lod,setLod]=useState<MediaPresentationCommonProps['lod']>('detailed'),[mode,setMode]=useState<MediaPresentationProps['mode']>('declared_preview'),[phase,setPhase]=useState('auto'),[u,setU]=useState(0),[fi,setFi]=useState(0),[selection,setSelection]=useState('无'),[clean,setClean]=useState(false);
  const pkg=packages.find(p=>p.definition.id===`p302.line-grade.${family}${assetKind==='image'?'.image':assetKind==='fit'?'.fit':''}`)!;
  const presentation=useMemo(()=>{if(pkg.definition.payload.category!=='media')throw Error('media package');return pkg.definition.payload.data;},[pkg]);
  const common:MediaPresentationCommonProps={presentation,readAsset,resolveModule,rights:pkg.rights,lod,binding:{session_id:'p302-line-grade-only',request:{project_id:'P1',scope:{kind:'project'},scope_revision:1,source:{kind:'module_version',data:{module_ref:{id:pkg.definition.id,version:pkg.definition.version}}}}},nodeOwner:{object_id:{kind:'module',id:pkg.definition.id},source:{kind:'module_version',data:{module_ref:{id:pkg.definition.id,version:pkg.definition.version}}}},onSelectNode:node=>setSelection(`canonical 节点 ${node.node_index} / ${node.owner.object_id.id} / id ${node.rendered_node_id}`),onInspectItem:id=>setSelection('实际物品 '+id),onAction:action=>setSelection(action)};
  const props:MediaPresentationProps=mode==='actual_run'?{...common,mode,frame:frames[fi].frame}:mode==='declared_preview'?{...common,mode,preview:{phase_ref:phase,view_u:u},onPreviewSeek:p=>{setPhase(p.phase_ref);setU(p.view_u);}}:{...common,mode};
  return <main className={'p302-line-grade'+(clean?' p302-clean-capture':'')}><header><span>超级工厂 / P302</span><h1>八设备同等级资产 · 独立复核</h1><p>自建参数化设计 · 米 / Y-up · 共享 MediaPresentationView · 不接管产品 Native</p></header>
    <nav aria-label="资产复核控制"><label>构造 <select aria-label="构造" value={family} onChange={e=>{setFamily(e.target.value);setAssetKind("model");}}>{families.map(v=><option key={v}>{v}</option>)}</select></label><label>素材 <select aria-label="素材" value={assetKind} onChange={e=>{setAssetKind(e.target.value);if(e.target.value==="fit"){setMode("declared_preview");setLod("detailed");}}}><option value="model">GLB</option><option value="fit" disabled={!packages.some(p=>p.definition.id===`p302.line-grade.${family}.fit`)}>尺寸规检查（非库存）</option><option value="image" disabled={!packages.some(p=>p.definition.id===`p302.line-grade.${family}.image`)}>同模型静帧</option></select></label><label>档位 <select aria-label="档位" disabled={assetKind==="fit"} value={lod} onChange={e=>setLod(e.target.value as typeof lod)}>{['detailed','simplified','region'].map(v=><option key={v}>{v}</option>)}</select></label><label>数据模式 <select aria-label="数据模式" value={mode} onChange={e=>setMode(e.target.value as typeof mode)}><option value="declared_preview">声明预览</option><option value="asset_inspection">静态检查</option><option value="actual_run" disabled={assetKind==="fit"}>真实 Engine 状态投影</option></select></label>{mode==='declared_preview'?<label>阶段 <select aria-label="声明阶段" value={phase} onChange={e=>setPhase(e.target.value)}>{['load','auto','unload','cycle','working'].map(v=><option key={v}>{v}</option>)}</select></label>:mode==='actual_run'?<label>真实源 <select aria-label="真实源" value={fi} onChange={e=>setFi(Number(e.target.value))}>{frames.map((f,i)=><option key={i} value={i}>{f.label}</option>)}</select></label>:null}</nav>
    <div className="p302-keypoints">{[0,.125,.25,.375,.5,.625,.75,.875,1].map(v=><button key={v} disabled={mode!=='declared_preview'} onClick={()=>setU(v)}>u={v}</button>)}<label><input type="checkbox" checked={clean} onChange={e=>setClean(e.target.checked)}/>静帧隐藏标注</label><output aria-label="选择反馈">{selection}</output></div>
    <MediaPresentationView {...props}/>
    <footer><strong>{pkg.definition.id}@{pkg.definition.version}</strong><span>ASM01 / RW01 / PK01 共台架独立工装；CV01 / CV02 独立设备身份；PAL01 为资源。</span><p>{mode==='actual_run'?'当前真实源是既有 CNC 分阶段 Engine 用例，验证换外观不改变业务及等待/暂停投影；不冒称已经取得 L03 八设备的新 Native 输入。':'声明预览无实际作业、人员、物品或质量结论。参考工件只是独立尺寸规，不是库存。'} Root 尚需提供实际物品几何/持有锚点绑定及区域聚合；本页不构造替代业务状态。</p><p>右键旋转、中键平移、滚轮近看；斜视与正面、背面、左侧、右侧、顶部、底部均有独立相机入口，工具近景在同一视口复核。</p></footer>
  </main>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Harness/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());


