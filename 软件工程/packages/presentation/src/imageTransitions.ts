import type {Presentation} from '../../commands/generated/typescript/Presentation';
import type {ImageTransition} from '../../commands/generated/typescript/ImageTransition';
import type {ClipPoint} from './clips';
import {UnsupportedMedia} from './types';

export const IMAGE_TRANSITION_RULES={entry_fraction:0.1,pan_scale:1.08,pan_extent:0.04,zoom_end_scale:1.08} as const;
export interface ImageViewport {scale:number;offset_x:number;offset_y:number}
export interface ImageTransitionPlan {kind:ImageTransition;entry_u:number;current:ImageViewport;previous:ImageViewport;previous_clip:ClipPoint|null;current_alpha:number;previous_alpha:number}
const fixedViewport=(kind:ImageTransition,t:number):ImageViewport=>kind==='pan'?{scale:IMAGE_TRANSITION_RULES.pan_scale,offset_x:(2*t-1)*IMAGE_TRANSITION_RULES.pan_extent,offset_y:0}:kind==='zoom'?{scale:1+(IMAGE_TRANSITION_RULES.zoom_end_scale-1)*t,offset_x:0,offset_y:0}:{scale:1,offset_x:0,offset_y:0};
export function imageTransitionPlan(p:Presentation,point:ClipPoint|null):ImageTransitionPlan {
  const kind=point?.clip.transition??'hold',entry_u=point?Math.min(1,point.progress/IMAGE_TRANSITION_RULES.entry_fraction):1;
  const phase=(ref:string|null|undefined)=>ref??'cycle';
  const prior=point&&kind==='crossfade'?p.timeline.find(clip=>phase(clip.phase_ref)===phase(point.clip.phase_ref)&&clip.end_progress===point.clip.start_progress):undefined;
  if(prior&&!['image_asset','gif_segment'].includes(prior.source.kind))throw new UnsupportedMedia('图片crossfade的前段须为image_asset或gif_segment；canonical未声明跨3D/图片的合成相机与视口。');
  return {kind,entry_u,current:fixedViewport(kind,entry_u),previous:fixedViewport(prior?.transition??'hold',1),previous_clip:prior?{clip:prior,progress:1,effective_ms:null}:null,current_alpha:prior?entry_u:1,previous_alpha:prior?1-entry_u:0};
}
export interface ImageRect {x:number;y:number;width:number;height:number}
export function containedImageRect(width:number,height:number,imageWidth:number,imageHeight:number):ImageRect {
  const scale=Math.min(width/imageWidth,height/imageHeight);return{x:(width-imageWidth*scale)/2,y:(height-imageHeight*scale)/2,width:imageWidth*scale,height:imageHeight*scale};
}
export function transformedImageRect(base:ImageRect,viewport:ImageViewport):ImageRect {
  const width=base.width*viewport.scale,height=base.height*viewport.scale;
  return {x:base.x+(base.width-width)/2+viewport.offset_x*base.width,y:base.y+(base.height-height)/2+viewport.offset_y*base.height,width,height};
}
