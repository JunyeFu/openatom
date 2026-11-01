import type {AssetRef} from '../../../../../packages/commands/generated/typescript/AssetRef';
import {ControlledGif,decodeStatic,gifFrameAt,imageTransitionPlan,containedImageRect,transformedImageRect,UnsupportedMedia,type VisualProgress,type ClipPoint,type AssetByteReader,type ImageRect,type ImageViewport} from '../../../../../packages/presentation/src';
import type {ResolvedMedia} from './load';
import type {Surface} from './surface';
import {domPainted,type ReadonlyVisualCapture} from '../render-surface/visualCapture';

export function mediaAsset(resolved:ResolvedMedia,clip:ClipPoint|null):AssetRef|null {
  if(clip?.clip.source.kind==='image_asset')return clip.clip.source.data;
  if(clip?.clip.source.kind==='gif_segment'||clip?.clip.source.kind==='model_animation')return clip.clip.source.data.asset_ref;
  return resolved.locator?.kind==='asset'?resolved.locator.data:null;
}
interface ImageFrame {bitmap:ImageBitmap;index:number;label:string;release:()=>void}
interface ImageLayer {frame:(point:VisualProgress|null,clip:ClipPoint|null)=>Promise<ImageFrame>;dispose:()=>void}
async function loadLayer(asset:AssetRef|null,sequence:AssetRef[]|null,read:AssetByteReader):Promise<ImageLayer> {
  if(!asset&&!sequence?.length)throw new UnsupportedMedia('当前来源没有可显示的图片或序列。');
  if(asset?.kind==='gif'){
    const gif=new ControlledGif(await read(asset));
    return {async frame(point,clip){
      let time=point?.effective_ms??(point?.progress??0)*gif.info.duration_ms,terminal=point?.terminal_pose??false;
      if(clip?.clip.source.kind==='gif_segment'){
        const segment=clip.clip.source.data;if(segment.end_ms<=segment.start_ms||segment.end_ms>gif.info.duration_ms)throw new UnsupportedMedia('GIF片段超出真实编码时长。');
        time=Math.min(segment.start_ms+(segment.end_ms-segment.start_ms)*clip.progress,segment.end_ms-0.001);terminal=true;
      }
      const bitmap=await gif.frame(time,terminal),index=gifFrameAt(gif.info,time,terminal);
      return {bitmap,index,label:`GIF 实帧 ${index+1}/${gif.info.durations.length}`,release:()=>bitmap.close()};
    },dispose:()=>gif.close()};
  }
  const bitmaps:ImageBitmap[]=[];
  try {for(const image of sequence??[asset!])bitmaps.push(await decodeStatic(image,await read(image)));}
  catch(error){for(const bitmap of bitmaps)bitmap.close();throw error;}
  return {async frame(point,clip){const index=sequence?Math.min(bitmaps.length-1,Math.floor((clip?.progress??point?.progress??0)*bitmaps.length)):0;return {bitmap:bitmaps[index],index,label:sequence?`图片序列 ${index+1}/${bitmaps.length}`:'静态原图',release:()=>{}};},dispose(){for(const bitmap of bitmaps)bitmap.close();}};
}
/** Both modes use these exact bytes, frame decode, viewport and blend operations. */
export async function createImageSurface(host:HTMLElement,resolved:ResolvedMedia,asset:AssetRef|null,initialClip:ClipPoint|null,onLabel:(label:string)=>void,onProject:()=>void,capture?:ReadonlyVisualCapture):Promise<Surface> {
  const sequence=!asset&&resolved.locator?.kind==='image_sequence'?resolved.locator.data:null;
  const initialPlan=imageTransitionPlan(resolved.presentation,initialClip);
  const current=await loadLayer(asset,sequence,resolved.read);
  let previous:ImageLayer|null=null;
  try {if(initialPlan.previous_clip)previous=await loadLayer(mediaAsset(resolved,initialPlan.previous_clip),null,resolved.read);}
  catch(error){current.dispose();throw error;}
  const canvas=document.createElement('canvas'),ctx=canvas.getContext('2d')!,blend=document.createElement('canvas'),blendCtx=blend.getContext('2d')!;
  host.append(canvas);
  let serial=0,disposed=false,frames:ImageFrame[]=[],plan=initialPlan,currentRect:ImageRect={x:0,y:0,width:0,height:0},baseRect=currentRect;
  const drawLayer=(target:CanvasRenderingContext2D,frame:ImageFrame,viewport:ImageViewport,alpha:number)=>{
    const base=containedImageRect(host.clientWidth,host.clientHeight,frame.bitmap.width,frame.bitmap.height),rect=transformedImageRect(base,viewport);
    target.save();target.beginPath();target.rect(base.x,base.y,base.width,base.height);target.clip();target.globalAlpha=alpha;target.drawImage(frame.bitmap,rect.x,rect.y,rect.width,rect.height);target.restore();return{base,rect};
  };
  const paint=()=>{
    const w=host.clientWidth,h=host.clientHeight,dpr=devicePixelRatio;canvas.width=w*dpr;canvas.height=h*dpr;ctx.setTransform(dpr,0,0,dpr,0,0);ctx.fillStyle='#eaf1f7';ctx.fillRect(0,0,w,h);
    if(frames.length){
      let result;
      if(frames[1]&&plan.previous_alpha>0){
        blend.width=canvas.width;blend.height=canvas.height;blendCtx.setTransform(dpr,0,0,dpr,0,0);
        drawLayer(blendCtx,frames[1],plan.previous,plan.previous_alpha);blendCtx.globalCompositeOperation='lighter';result=drawLayer(blendCtx,frames[0],plan.current,plan.current_alpha);ctx.drawImage(blend,0,0,w,h);
      }else result=drawLayer(ctx,frames[0],plan.current,1);
      currentRect=result.rect;baseRect=result.base;
    }
    onProject();
    if(capture&&frames.length){const ticket=serial;domPainted(()=>!disposed&&ticket===serial).then(done=>{if(done)capture.onFrame({visible_objects:w>0&&h>0?[capture.object]:[],limitations:['二维图片画面不适用三维相机；保留请求相机声明。']});});}
  };
  const observer=new ResizeObserver(paint);observer.observe(host);
  return {async draw(point,clip){
    const ticket=++serial,nextPlan=imageTransitionPlan(resolved.presentation,clip);
    const results=await Promise.allSettled([current.frame(point,clip),...(previous&&nextPlan.previous_clip?[previous.frame({...point!,known:true,progress:1,terminal_pose:true,effective_ms:null,duration_ms:null,display_phase_id:nextPlan.previous_clip.clip.phase_ref??'cycle'},nextPlan.previous_clip)]:[])]);
    const next=results.flatMap(result=>result.status==='fulfilled'?[result.value]:[]),failed=results.find(result=>result.status==='rejected');
    if(disposed||ticket!==serial||failed){for(const frame of next)frame.release();if(failed?.status==='rejected')throw failed.reason;return;}
    const old=frames;frames=next;plan=nextPlan;
    canvas.dataset.frameIndex=String(frames[0].index);canvas.dataset.transition=plan.kind;canvas.dataset.currentAlpha=String(plan.current_alpha);canvas.dataset.previousAlpha=String(plan.previous_alpha);
    onLabel(`${frames[0].label}${plan.kind==='hold'?'':` · ${plan.kind}`}`);paint();for(const frame of old)frame.release();
  },dispose(){disposed=true;serial++;observer.disconnect();for(const frame of frames)frame.release();current.dispose();previous?.dispose();canvas.remove();},anchor(ref){
    if(ref.kind==='viewport')return{x:ref.data.x*100,y:ref.data.y*100};
    if(ref.kind!=='image_normalized'||!host.clientWidth||!host.clientHeight)return null;
    const x=currentRect.x+currentRect.width*ref.data.x,y=currentRect.y+currentRect.height*ref.data.y;
    return x<baseRect.x||x>baseRect.x+baseRect.width||y<baseRect.y||y>baseRect.y+baseRect.height?null:{x:x/host.clientWidth*100,y:y/host.clientHeight*100};
  }};
}
