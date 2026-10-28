/// <reference types="vite/client" />
// Independent controlled-byte harness only. Never imported by the product entry.
import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {MediaPresentationView} from './MediaPresentationView';
import type {MediaPresentationProps} from './types';
import framesJson from '../../../../../scenarios/媒体表现/runtime/actual-frames.json';
import image from '../../../../../modules/media/p302-controlled/image.module.json';
import gif from '../../../../../modules/media/p302-controlled/gif.module.json';
import model from '../../../../../modules/media/p302-controlled/model.module.json';
import simple from '../../../../../modules/media/p302-controlled/model-simple.module.json';
import imageSimple from '../../../../../modules/media/p302-controlled/image-simple.module.json';
const packageMap:Record<string,any>={image,gif,model,'model-simple':simple,'image-simple':imageSimple};
for(const [kind,file]of [['jpeg','station.jpg'],['svg','anchor.svg']] as const){const pkg=structuredClone(image);const asset=pkg.assets.find(a=>a.path===file)!;pkg.definition.payload.data.source={kind:'asset',data:asset};for(const clip of pkg.definition.payload.data.timeline)clip.source={kind:'image_asset',data:asset};packageMap[kind]=pkg;}
const sequence=structuredClone(image);sequence.definition.payload.data.source={kind:'image_sequence',data:sequence.assets.filter(a=>['station.png','station-end.png'].includes(a.path))} as any;sequence.definition.payload.data.timeline=[];packageMap.sequence=sequence;
const textured=structuredClone(image) as any;const native=textured.definition.payload.data;native.kind='native_scene';native.source={kind:'native_scene',data:'controlled-texture-panel'};native.timeline=[];native.nodes=[{id:0,name:'精确 PNG 纹理检查面板',parent_index:null,transform:{translation:{x:0,y:0,z:0},rotation:{x:0,y:0,z:0,w:1},scale:{x:1,y:1,z:1},unit:'meter',coordinate_frame:'right_handed_y_up'},geometry:{kind:'box',data:{x:1.6,y:1.2,z:.12}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:1,texture:image.assets.find(a=>a.path==='station.png')}}];native.tags[0].anchor={kind:'scene_node',data:0};packageMap.texture=textured;
const missing=structuredClone(model) as any;missing.definition.payload.data.timeline=[];missing.definition.payload.data.source.data={id:'missing',path:'missing.glb',kind:'glb'};packageMap.missing=missing;
const urls=import.meta.glob('../../../../../modules/media/p302-controlled/*.{png,jpg,gif,glb,svg}',{query:'?url',import:'default',eager:true}) as Record<string,string>;
const frames=framesJson as unknown as {label:string;case:string;frame:NonNullable<MediaPresentationProps['frame']>}[];
let delayNext=false,releaseOld:(()=>void)|null=null;
const readAsset:MediaPresentationProps['readAsset']=async request=>{
  const expected=image.assets.find(a=>a.id===request.asset.id&&a.path===request.asset.path&&a.kind===request.asset.kind);
  if(!expected)throw new Error('MissingReference: controlled exact AssetRef not found');
  const url=Object.entries(urls).find(([path])=>path.endsWith('/'+expected.path))?.[1];if(!url)throw new Error('MissingReference: controlled byte fixture not found');
  const response=await fetch(url);if(!response.ok)throw new Error(`controlled HTTP bytes ${response.status}`);const bytes=await response.arrayBuffer();if(delayNext){delayNext=false;await new Promise<void>(resolve=>{releaseOld=resolve;});}return bytes;
};
const resolveModule:NonNullable<MediaPresentationProps['resolveModule']>=async source=>{
 const key=source.data.module_ref.id.replace('p302.controlled.',''),pkg=packageMap[key];if(!pkg||pkg.definition.version!==source.data.module_ref.version)throw new Error('MissingReference: precise controlled module');return pkg;
};
function Harness(){
 const [kind,setKind]=useState('model'),[index,setIndex]=useState(frames.findIndex(f=>f.case==='interrupt'&&f.frame.snapshot.at_ms===20000)),[lod,setLod]=useState<MediaPresentationProps['lod']>('detailed'),[selection,setSelection]=useState('无'),[action,setAction]=useState('无'),[playback,setPlayback]=useState({playing:false,rate:1,loop:false}),[context,setContext]=useState(1);
 const pkg=packageMap[kind],frame=frames[index].frame;
 return <main style={{maxWidth:1480,margin:'0 auto',padding:20,font:'14px Microsoft YaHei',color:'#172e48'}}><h1 style={{fontSize:20,margin:'0 0 12px'}}>媒体表现 · 独立浏览器验证</h1><p>自建 CC0 候选模型；实际 Engine 快照；此页不是 Native 接收或工业认可。</p><div style={{display:'flex',gap:12,flexWrap:'wrap',marginBottom:12}}><label>素材 <select aria-label="素材类型" value={kind} onChange={e=>setKind(e.target.value)}>{['model','image','gif','jpeg','svg','sequence','texture','missing'].map(v=><option key={v}>{v}</option>)}</select></label><label>真实时点 <select aria-label="真实时点" value={index} onChange={e=>setIndex(Number(e.target.value))}>{frames.map((f,i)=><option key={i} value={i}>{f.label}</option>)}</select></label><label>LOD <select aria-label="LOD" value={lod} onChange={e=>setLod(e.target.value as typeof lod)}>{['detailed','simplified','region'].map(v=><option key={v}>{v}</option>)}</select></label><button onClick={()=>setContext(c=>c+1)}>切换独立会话</button><button onClick={()=>{delayNext=true;setKind('model');setContext(c=>c+1);}}>延迟下一次读取</button><button onClick={()=>{releaseOld?.();releaseOld=null;}}>释放旧读取</button></div>
 <MediaPresentationView mode="actual_run" presentation={pkg.definition.payload.data} rights={pkg.rights} lod={lod} frame={frame} binding={{session_id:`controlled-${context}`,request:{project_id:'P1',scope:{kind:'project'},scope_revision:1,source:{kind:'run_module_version',data:{run_id:frame.run.id,at_ms:frame.snapshot.at_ms,module_ref:{id:pkg.definition.id,version:pkg.definition.version}}}}}} readAsset={readAsset} resolveModule={resolveModule} nodeOwner={{object_id:{kind:'module',id:pkg.definition.id},source:{kind:'run_module_version',data:{run_id:frame.run.id,at_ms:frame.snapshot.at_ms,module_ref:{id:pkg.definition.id,version:pkg.definition.version}}}}} onSelectNode={node=>setSelection(`${node.node_index}: ${node.owner.object_id.id} / id ${node.rendered_node_id}`)} onAction={setAction} onInspectItem={id=>setSelection(`物品 ${id} · ${frame.run.events.filter(e=>e.subject.id===id).length} 条实际事件`)} playback={playback} onPlaybackChange={setPlayback} onSeek={at=>{const known=frames.findIndex(f=>f.case===frames[index].case&&f.frame.snapshot.at_ms===at);if(known>=0)setIndex(known);else setAction(`独立页只有捕获的实际事件边界；${at} ms 需 Root objects_at 实读`);}}/>
 <p role="status">选择：{selection}；显式动作：{action}；观看倍率：{playback.rate}</p></main>;
}
const harnessRoot=createRoot(document.getElementById('root')!);harnessRoot.render(<Harness/>);
if(import.meta.hot)import.meta.hot.dispose(()=>harnessRoot.unmount());
