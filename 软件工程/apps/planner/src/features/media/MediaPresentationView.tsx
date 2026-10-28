import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import type {DataTagBinding} from '../../../../../packages/commands/generated/typescript/DataTagBinding';
import type {JobStatus} from '../../../../../packages/commands/generated/typescript/JobStatus';
import type {WaitReasonKind} from '../../../../../packages/commands/generated/typescript/WaitReasonKind';
import {activeClip,assetKey,bindScene,decodeGlb,projectPhase,declaredProgress,imageTransitionPlan,resolveMediaData,type MediaDataContext,type VisualProgress,type ClipPoint,type PresentationScene} from '../../../../../packages/presentation/src';
import type {MediaPresentationProps} from './types';
import {resolveMedia,type ResolvedMedia} from './load';
import {ModelViewport} from './viewport';
import {createImageSurface,mediaAsset} from './imageSurface';
import type {Surface} from './surface';
import {domPainted,type ReadonlyVisualCapture} from '../render-surface/visualCapture';
import {readProcessItems,createProcessItemLayer} from './processItems';
import './media.css';
import {factoryResourceTitle} from '../../scene/factoryLabels';

const jobLabels:Record<JobStatus,string>={ready:'等待开始',working:'加工中',paused:'已暂停',waiting_resource:'等待资源',waiting_output:'等待输出接收',completed:'已完成',cancelled:'已取消'};
const waitLabels:Record<WaitReasonKind,string>={missing_material:'缺少物料',resource_busy:'资源占用',calendar:'等待工作时段',preparation:'等待准备完成',quality:'等待质量检查',output_capacity:'输出空间不足',receiver_capacity:'接收空间不足'};
function DataValue({binding,context}:{binding:DataTagBinding;context:MediaDataContext}) {
  const value=resolveMediaData(binding,context);
  return <span data-field-status={value.status} title={value.status==='unavailable'?value.reason:binding.source_field}>{value.status==='available'?value.formatted:`未提供 · ${value.message}`}</span>;
}
export function MediaPresentationView(props:MediaPresentationProps) {
  const stamp=JSON.stringify([props.mode,props.binding,props.presentation,props.lod,props.rights,props.nodeOwner]);
  const [resolvedState,setResolved]=useState<{stamp:string;value:ResolvedMedia}|null>(null),[error,setError]=useState<unknown>(null),[loaded,setLoaded]=useState(false),[frameLabel,setFrameLabel]=useState(''),[anchorPositions,setAnchorPositions]=useState<Record<string,{x:number;y:number}|null>>({});
  const resolved=resolvedState?.stamp===stamp?resolvedState.value:null;
  const host=useRef<HTMLDivElement>(null),surface=useRef<Surface|null>(null),latest=useRef(props);latest.current=props;
  const model=useRef<{rig:PresentationScene;viewport:ModelViewport;presentation:ResolvedMedia['presentation']}|null>(null);
  const itemLayer=useRef<ReturnType<typeof createProcessItemLayer>|null>(null);
  const [itemStatuses,setItemStatuses]=useState<Record<string,string>>({});
  const measure=useMemo(()=>{const ctx=document.createElement('canvas').getContext('2d')!;ctx.font='12px "Microsoft YaHei",sans-serif';return ctx;},[]);
  const actualFrame=props.mode==='actual_run'?props.frame:null;
  const itemFrameKey=JSON.stringify([stamp,actualFrame?.run.id,actualFrame?.job_id,actualFrame?.snapshot.at_ms]);
  const itemCapture=useRef<{key:string;pending:boolean;limitations:string[];failure?:string}>({key:itemFrameKey,pending:!!actualFrame,limitations:[]});
  if(itemCapture.current.key!==itemFrameKey)itemCapture.current={key:itemFrameKey,pending:!!actualFrame,limitations:[]};
  const point=useMemo(()=>actualFrame?projectPhase(actualFrame):null,[actualFrame]);
  let visualPoint:VisualProgress|null=point;
  let clip:ClipPoint|null=null,projectionError:unknown=null,previousAssetKey:string|null=null;
  try {
    if(props.mode==='declared_preview')visualPoint=declaredProgress(props.preview);
    if(resolved&&visualPoint?.known){
      clip=activeClip(resolved.presentation,visualPoint);
      const prior=imageTransitionPlan(resolved.presentation,clip).previous_clip,asset=prior?mediaAsset(resolved,prior):null;
      previousAssetKey=asset?assetKey(asset):null;
    }
  } catch(e){projectionError=e;}
  const current=useRef({point:visualPoint,clip});current.current={point:visualPoint,clip};
  const currentStamp=useRef(stamp);currentStamp.current=stamp;
  useEffect(()=>{
    let live=true;setResolved(null);setError(null);setLoaded(false);
    resolveMedia(props).then(value=>{if(live)setResolved({stamp,value});},e=>{if(live)setError(e);});
    return()=>{live=false;};
  },[stamp,props.readAsset,props.resolveModule]);
  const asset=resolved?mediaAsset(resolved,clip):null;
  const selectedKey=JSON.stringify([resolved?.locator?.kind==='image_sequence'?resolved.locator.data:null,asset?assetKey(asset):resolved?.locator?.kind,previousAssetKey]);
  useEffect(()=>{
    if(!resolved||!host.current||props.lod==='region')return;
    let live=true,instance:Surface|null=null;
    const localHost=host.current;setLoaded(false);setError(null);
    const capture:ReadonlyVisualCapture|undefined=props.capture?{...props.capture,onFrame(frame){
      const items=itemCapture.current;
      if(!live||items.pending)return;
      if(items.failure)props.capture!.onFailure(items.failure);
      else props.capture!.onFrame({...frame,limitations:[...frame.limitations,...items.limitations]});
    }}:undefined;
    const projectAnchors=()=>{if(!live)return;const values:Record<string,{x:number;y:number}|null>={};for(const tag of resolved.presentation.tags)values[tag.id]=instance?.anchor(tag.anchor)??null;for(const [name,ref]of Object.entries({...resolved.presentation.anchor_roles,...resolved.presentation.port_anchors}))values[`binding:${name}`]=instance?.anchor(ref)??null;setAnchorPositions(values);};
    async function create() {
      const p=resolved!.presentation;
      if(asset?.kind==='glb'||resolved!.locator?.kind==='native_scene') {
        const decoded=asset?await decodeGlb(await resolved!.read(asset)):undefined;
        const rig=await bindScene(p,resolved!.read,decoded&&asset?{assetKey:assetKey(asset),decoded}:undefined);
        rig.root.userData.mqSource={asset,source:resolved!.source,lod:props.lod};
        if(!live){rig.dispose();return;}
        let viewport:ModelViewport;
        try{viewport=new ModelViewport(localHost,rig,index=>{
          if(!live||currentStamp.current!==stamp||!latest.current.onSelectNode)return;
          if(!resolved!.owner){setError(new Error('此画面尚未提供精确节点父拥有者，不能猜用字节来源。'));return;}
          latest.current.onSelectNode({owner:resolved!.owner,node_index:index,rendered_node_id:p.nodes[index]?.id,lod:props.lod,asset:asset??null,byte_source:resolved!.source});
        },projectAnchors,capture,id=>latest.current.onInspectItem?.(id),()=>new Map(itemLayer.current?.renderState().objects.map(item=>[item.root,item.ref.id])??[]));}catch(error){rig.dispose();throw error;}
        model.current={rig,viewport,presentation:p};
        instance={draw(point,clip){if(props.capture?.poseName)rig.applyNamedPose(props.capture.poseName);else rig.apply(point,clip);viewport.draw();setFrameLabel('三维模型');},dispose:()=>viewport.dispose(),anchor:ref=>ref.kind==='viewport'?{x:ref.data.x*100,y:ref.data.y*100}:viewport.project(ref),view:props.capture?undefined:kind=>viewport.view(kind)};
        localHost.dataset.mediaKind='model';
      } else {
        instance=await createImageSurface(localHost,resolved!,asset,current.current.clip,setFrameLabel,projectAnchors,capture);
        if(!live){instance.dispose();return;}
        localHost.dataset.mediaKind=asset?.kind==='gif'?'gif':resolved!.locator?.kind==='image_sequence'?'image_sequence':'image';
      }
      if(!instance)return;surface.current=instance;
      if(current.current.point?.known!==false)await instance.draw(current.current.point,current.current.clip);
      if(live){setLoaded(true);projectAnchors();}
    }
    create().catch(e=>{if(live)setError(e);});
    return()=>{live=false;itemLayer.current?.dispose();itemLayer.current=null;model.current=null;instance?.dispose();if(surface.current===instance)surface.current=null;};
  },[resolved,selectedKey,props.lod,props.capture]);
  useEffect(()=>{
    const instance=surface.current;if(!instance||!loaded||visualPoint?.known===false||projectionError)return;
    let live=true;Promise.resolve(instance.draw(visualPoint,clip)).catch(e=>{if(live)setError(e);});return()=>{live=false;};
  },[props.mode,props.preview?.phase_ref,props.preview?.view_u,props.frame,clip?.clip.id,loaded]);
  const visibleError=projectionError??error;
  useLayoutEffect(()=>{
    itemCapture.current.pending=!!actualFrame;
    itemLayer.current?.dispose();itemLayer.current=null;
    const ids=point?.items.map(item=>item.id)??[];
    const status=(message:string)=>setItemStatuses(Object.fromEntries(ids.map(id=>[id,message])));
    const unavailable=(message:string,failure?:string)=>{itemCapture.current={key:itemFrameKey,pending:false,limitations:message?[message]:[],failure};status(message);};
    if(!actualFrame){unavailable('');return;}
    if(!point?.known){unavailable('实际附件尚未取得完整可呈现来源。');return;}
    if(!loaded||visibleError){status('实际附件尚未取得完整可呈现来源。');return;}
    if(point.progress==null){unavailable('当前阶段姿态未确定；保留实际身份，不推测支承位置。');return;}
    if(props.lod==='region'){unavailable('区域档不绘制物品几何；实际身份保留。');return;}
    if(!model.current){unavailable('当前图片/GIF没有三维支承节点；实际身份保留，不推测位置。');return;}
    let live=true,owned:ReturnType<typeof createProcessItemLayer>|null=null;
    const owner=model.current;
    status('正在读取此时点的布局与物品所在位置…');
    (props.readItems??readProcessItems)(actualFrame,props.binding,()=>live&&currentStamp.current===stamp).then(input=>{
      if(!live||model.current!==owner)return;
      const changed=()=>{if(live){
        const state=owned!.renderState();
        itemCapture.current={key:itemFrameKey,pending:state.pending>0,
          limitations:[...(input.unavailable?[input.unavailable]:[]),...state.unavailable.map(value=>`${value.ref.id}：${value.message}`)],
          failure:state.failures.length?state.failures.map(value=>`${value.ref.id}：${value.message}`).join('；'):undefined};
        setItemStatuses(owned!.statuses());owner.viewport.draw();
      }};
      owned=createProcessItemLayer(input,owner.rig,owner.presentation,props.lod,changed);
      itemLayer.current=owned;owned.reconcile();changed();
    },failure=>{if(live){const message=`无法读取 · ${failure instanceof Error?failure.message:String(failure)}`;unavailable(message,message);owner.viewport.draw();}});
    return()=>{live=false;itemCapture.current.pending=true;owned?.dispose();if(itemLayer.current===owned)itemLayer.current=null;if(model.current===owner)owner.viewport.draw();};
  },[actualFrame,point?.known,loaded,resolved,props.lod,stamp,itemFrameKey,visibleError,props.readItems]);
  useEffect(()=>{if(visibleError&&props.capture)props.capture.onFailure(visibleError instanceof Error?visibleError.message:String(visibleError));else if(props.lod==='region'&&props.capture){let live=true;domPainted(()=>live).then(done=>{if(done)props.capture?.onFrame({visible_objects:[],limitations:['区域档仅显示声明区域规则，不伪造设备几何；此二维规则面板不适用三维相机。']});});return()=>{live=false;};}},[visibleError,props.capture,props.lod]);
  const p=resolved?.presentation??props.presentation,playback=props.playback??{playing:false,rate:1,loop:false};
  const changePlayback=(value:Partial<typeof playback>)=>props.onPlaybackChange?.({...playback,...value});
  const claimIds=new Set(point?.claims.map(c=>c.resource_id));
  const heldIds=[...(point?.claims.map(c=>c.resource_id)??[]),...[...new Set([...(point?.job?.job_resources??[]),...(point?.job?.phase_resources??[])])].filter(id=>!claimIds.has(id))];
  const heldLabels=heldIds.map((id,index)=>props.sourceProject?factoryResourceTitle(props.sourceProject,id):actualFrame?.snapshot.objects.flatMap(object=>object.kind==='resource'&&object.data.id===id?[object.data.title]:[])[0]??`资源 ${index+1}`);
  const waitLabel=point?.waits.flatMap(wait=>wait.reasons.map(r=>waitLabels[r.kind])).join('、')||(point?.job?.state==='waiting_output'?'等待输出接收（此刻暂无完整原因区间）':point?.job?.state==='waiting_resource'?'等待资源（此刻暂无完整原因区间）':'无活动等待记录');
  const dataContext:MediaDataContext=actualFrame&&point?{mode:'actual_run',frame:actualFrame,point}:{mode:props.mode==='declared_preview'?'declared_preview':'asset_inspection'};
  const workers=actualFrame?.snapshot.objects.flatMap(object=>object.kind==='resource'&&object.data.kind==='worker'?[object.data]:[])??[];
  const tags=p.tags.filter(tag=>!tag.progress_range||visualPoint?.progress!=null&&visualPoint.progress>=tag.progress_range[0]&&(visualPoint.progress<tag.progress_range[1]||visualPoint.progress===1&&tag.progress_range[1]===1));
  const projected:typeof anchorPositions={};const occupied:{left:number;right:number;top:number;bottom:number}[]=[];
  for(const tag of tags){
    const pos=anchorPositions[tag.id],w=host.current?.clientWidth??0,h=host.current?.clientHeight??0;
    if(!pos||!w||!h){projected[tag.id]=null;continue;}
    const width=Math.min(160,measure.measureText(tag.label).width+16),x=pos.x*w/100,y=pos.y*h/100,box={left:x-width/2,right:x+width/2,top:y-26,bottom:y};
    if(box.left>=0&&box.right<=w&&box.top>=0&&box.bottom<=h&&!occupied.some(o=>box.left<o.right+4&&box.right>o.left-4&&box.top<o.bottom+4&&box.bottom>o.top-4)){projected[tag.id]=pos;occupied.push(box);}else projected[tag.id]=null;
  }
  return <section className="sf-media" data-media-mode={props.mode} aria-label={props.mode==='declared_preview'?'声明阶段媒体预览':'真实状态媒体呈现'}>
    <div className="sf-media-toolbar"><strong>设备表现</strong><span>{props.mode==='declared_preview'?'过程预览':props.mode==='actual_run'?'运行记录':'静态检查'}</span><span>{props.lod==='detailed'?'详细':props.lod==='simplified'?'简化':'区域'}档</span><div className="sf-media-camera">{(['iso','front','back','left','right','top','bottom','fit'] as const).map((kind,i)=><button key={kind} disabled={!loaded||!surface.current?.view||!!visibleError} onClick={()=>surface.current?.view?.(kind)}>{['斜视','正面','背面','左侧','右侧','顶部','底部','适配'][i]}</button>)}</div></div>
    <div className="sf-media-body"><div className="sf-media-stage">
      <div className="sf-media-canvas" ref={host} aria-label="媒体画面" />
      {props.lod==='region'?<div className="sf-media-region">区域规则：{p.lod.region}<p>区域汇总由产线/区域视图呈现。</p></div>:null}
      {visibleError?<div className="sf-media-message" role="alert">{visibleError instanceof Error?`${visibleError.name}: ${visibleError.message}`:String(visibleError)}</div>:!loaded&&props.lod!=='region'?<div className="sf-media-message" role="status">正在读取精确版本素材…</div>:null}
      {loaded&&!visibleError&&point?.known!==false&&props.lod!=='region'?<div className="sf-media-anchors">{tags.map(tag=>projected[tag.id]?<span key={tag.id} style={{left:`${projected[tag.id]!.x}%`,top:`${projected[tag.id]!.y}%`}}>{tag.label}</span>:null)}{Object.entries(anchorPositions).filter(([key,pos])=>key.startsWith('binding:')&&pos).map(([key,pos])=><i key={key} title={key.slice(8)} style={{left:`${pos!.x}%`,top:`${pos!.y}%`}} />)}</div>:null}
      <div className="sf-media-caption">{props.mode==='declared_preview'?`过程预览 · ${(props.preview.view_u*100).toFixed(1)}% · 无实际运行数据`:actualFrame?point?.known?`${point.paused?'计算已暂停 · ':''}${point.job?jobLabels[point.job.state]:'作业状态未知'} · 有效加工 ${point.effective_ms??'未知'} 毫秒`:'此时点状态未知':props.inspectionLabel??'静态外观检查 · 未连接运行记录'}{loaded?` · ${frameLabel}`:''}</div>
    </div><aside className="sf-media-inspector"><h3>{props.mode==='actual_run'?'阶段与持有':props.inspectionLabel?(props.inspectionTitle??'冻结静态声明'):'声明与预览'}</h3>
      {point?.issues.map(issue=><p key={issue}>{issue}</p>)}
      {props.mode==='actual_run'?<dl><dt>作业状态</dt><dd>{point?.job?jobLabels[point.job.state]:'未知'}</dd><dt>实际阶段</dt><dd>{point?.phase_id==='cycle'?'完整周期':point?.phase_id?'已定位当前阶段':'无当前阶段'}</dd><dt>画面进度</dt><dd>{point?.progress==null?'未知':`${(point.progress*100).toFixed(1)}%`}</dd><dt>等待</dt><dd>{waitLabel}</dd><dt>占用资源</dt><dd>{heldLabels.join('、')||'无'}</dd></dl>:<dl><dt>预览阶段</dt><dd>{props.preview?.phase_ref==='cycle'?'完整周期':props.preview?.phase_ref?'已选择':'未选阶段'}</dd><dt>观看进度</dt><dd>{props.preview?`${(props.preview.view_u*100).toFixed(1)}%`:'—'}</dd><dt>实际数据</dt><dd>{props.inspectionLabel?'静态外观不提供实际作业、阶段进度或产品产量。':'尚未连接运行记录。'}</dd></dl>}
      <details><summary>作业与阶段详情</summary><p>作业编号：{point?.job?.id??'未提供'}；阶段编号：{point?.phase_id??props.preview?.phase_ref??'未提供'}；资源编号：{heldIds.join('、')||'无'}。</p></details>

      <h3>设备标签</h3>{tags.length?tags.map(tag=><div className="sf-media-tag" key={tag.id}><strong>{tag.label}</strong>{tag.payload.kind==='annotation'?<details><summary>标签说明</summary><p>{tag.payload.data}</p></details>:<span>{tag.payload.kind==='data'?<DataValue binding={tag.payload.data} context={dataContext}/>:tag.payload.kind==='parameter'?`参数 ${tag.payload.data}（只读）`:null}</span>}{tag.payload.kind==='action'?<button disabled={!props.onAction} onClick={()=>tag.payload.kind==='action'&&props.onAction?.(tag.payload.data)}>{{configure:'配置',inspect:'检查',preview_cycle:'周期预览'}[tag.payload.data]}</button>:null}</div>):<p>未声明标签</p>}
      <h3>人员与角色</h3>{workers.length?workers.map(worker=><p key={worker.id}>{props.sourceProject?factoryResourceTitle(props.sourceProject,worker.id):worker.title}<small>{point?.known?claimIds.has(worker.id)||point.job?.job_resources.includes(worker.id)||point.job?.phase_resources.includes(worker.id)?'本作业占用':'未被本作业占用':'占用未知'} · 登记驻点：{worker.home_location??'未声明'}；移动位置未声明</small></p>):<p>{actualFrame?'当前快照未返回人员资源':'声明预览不提供实际人员资源'}</p>}
      <h3>物品</h3>{point?.items.length?point.items.map((item,index)=><div key={item.id}><button className="sf-media-item" disabled={!props.onInspectItem} onClick={()=>props.onInspectItem?.(item.id)}>查看物品 {index+1}<small>{itemStatuses[item.id]??'正在读取物品画面。'}</small></button><details><summary>物品身份与位置详情</summary><p>{item.id}</p><pre>{JSON.stringify(item.holder,null,2)}</pre></details></div>):<p>{actualFrame?'当前快照未返回物品':'未连接实际物品或库存'}</p>}
      <details><summary>查看素材许可与来源</summary>{(resolved?.rights??props.rights).map(right=><p className="sf-media-right" key={right.asset_ref}>{right.asset_ref} · {right.license??'许可未给出'}<small>{right.source??right.missing_reason??'来源未给出'} · {right.redistribution_confirmed?'已声明允许再分发':'再分发未确认'}</small></p>)}</details>
    </aside></div>
    <div className="sf-media-playback"><button disabled={!props.onPlaybackChange||props.mode==='asset_inspection'} onClick={()=>changePlayback({playing:!playback.playing})}>{playback.playing?'暂停观看':'播放观看'}</button><label>倍率 <select value={playback.rate} disabled={!props.onPlaybackChange||props.mode==='asset_inspection'} onChange={e=>changePlayback({rate:Number(e.target.value)})}>{[0.25,0.5,1,2,4].map(rate=><option key={rate} value={rate}>{rate}×</option>)}</select></label><label><input type="checkbox" checked={playback.loop} disabled={!props.onPlaybackChange||props.mode==='asset_inspection'} onChange={e=>changePlayback({loop:e.target.checked})}/>循环观看</label><label className="sf-media-seek">{props.mode==='declared_preview'?'声明进度':'呈现时点'} <input type="range" min={actualFrame?.run.window.start_ms??0} max={props.mode==='declared_preview'?1:actualFrame?.run.computed_until_ms??0} step={props.mode==='declared_preview'?0.001:1} value={props.preview?.view_u??actualFrame?.snapshot.at_ms??0} disabled={props.mode==='declared_preview'?!props.onPreviewSeek:!actualFrame||!props.onSeek} onChange={e=>{if(props.mode==='declared_preview')props.onPreviewSeek?.({...props.preview,view_u:Number(e.target.value)});else if(props.mode==='actual_run')props.onSeek?.(Number(e.target.value));}}/></label><output>{props.preview?`${props.preview.view_u.toFixed(3)} u`:actualFrame?`${actualFrame.snapshot.at_ms} 毫秒`:'—'}</output></div>

    <p className="sf-media-help">单击选择 · 右键旋转 · 中键平移 · 滚轮缩放。观看控制只改变呈现时点，不改变生产、库存或资源。</p>
  </section>;
}
