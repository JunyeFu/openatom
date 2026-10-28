import type { ModuleDefinition } from '../../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { TimelineClip } from '../../../../../packages/commands/generated/typescript/TimelineClip';
import type { AssetRef } from '../../../../../packages/commands/generated/typescript/AssetRef';
import type { OperationDefinition } from '../../../../../packages/commands/generated/typescript/OperationDefinition';
import type { DeviceTag } from '../../../../../packages/commands/generated/typescript/DeviceTag';
import { presentationOf } from '../modules/model';
export function same(a:unknown,b:unknown):boolean{
  if(a===b)return true;
  if(Array.isArray(a))return Array.isArray(b)&&a.length===b.length&&a.every((v,i)=>same(v,b[i]));
  if(Array.isArray(b)||!a||!b||typeof a!=='object'||typeof b!=='object')return false;
  const x=a as Record<string,unknown>,y=b as Record<string,unknown>,keys=Object.keys(x);return keys.length===Object.keys(y).length&&keys.every(k=>Object.hasOwn(y,k)&&same(x[k],y[k]));
}
export function withPresentation(definition:ModuleDefinition,value:Presentation):ModuleDefinition{
 const p=definition.payload;
 if(p.category==='media')return {...definition,payload:{category:p.category,data:value}};
 if(p.category==='devices'||p.category==='resources'||p.category==='transport')return {...definition,payload:{category:p.category,data:{...p.data,presentation:value}}} as ModuleDefinition;
 if(p.category==='items')return {...definition,payload:{category:p.category,data:{...p.data,presentation_or_media_ref:value}}};
 throw new Error('此类别没有可编辑表现字段。');
}
export function inlineOperation(definition:ModuleDefinition):OperationDefinition|null{
 const p=definition.payload;return p.category==='devices'&&p.data.behavior.selected==='inline'?p.data.behavior.inline??null:null;
}
export function withOperation(definition:ModuleDefinition,operation:OperationDefinition):ModuleDefinition{
 if(definition.payload.category!=='devices'||definition.payload.data.behavior.selected!=='inline')throw new Error('只修改当前草稿内联行为；独立工艺版本须在原工艺编辑器另存。');
 return {...definition,payload:{category:'devices',data:{...definition.payload.data,behavior:{selected:'inline',inline:operation,operation_ref:null}}}};
}
export function group(clips:readonly TimelineClip[],phase:string|null){return clips.filter(c=>(c.phase_ref??null)===phase);}
function replaceGroup(value:Presentation,phase:string|null,next:TimelineClip[]):Presentation{
 const result:TimelineClip[]=[];let inserted=false;
 for(const c of value.timeline){if((c.phase_ref??null)===phase){if(!inserted){result.push(...next);inserted=true;}}else result.push(c);}
 if(!inserted)result.push(...next);
 return {...value,timeline:result,production_effect:'none'};
}
function distribute(clips:TimelineClip[]):TimelineClip[]{
 const weights=clips.map(c=>c.end_progress-c.start_progress),total=weights.reduce((a,b)=>a+b,0);if(!clips.length)return [];
 if(weights.some(w=>!Number.isFinite(w)||w<=0)||!Number.isFinite(total))throw new Error('各片段需要明确的正持续比例。');
 let start=0;return clips.map((c,i)=>{const end=i===clips.length-1?1:start+weights[i]/total,result={...c,start_progress:start,end_progress:end};start=end;return result;});
}
export function reorder(value:Presentation,id:string,target:number):Presentation{
 const clip=value.timeline.find(c=>c.id===id);if(!clip)throw new Error('片段已不存在。');const phase=clip.phase_ref??null,rows=group(value.timeline,phase),from=rows.findIndex(c=>c.id===id);if(target<0||target>=rows.length)throw new Error('只能在当前绑定阶段内排序。');rows.splice(from,1);rows.splice(target,0,clip);return replaceGroup(value,phase,distribute(rows));
}
export function insertImage(value:Presentation,asset:AssetRef,phase:string|null,afterId?:string):Presentation{
 return insertImages(value,[asset],phase,afterId);
}
export function insertImages(value:Presentation,assets:readonly AssetRef[],phase:string|null,afterId?:string):Presentation{
 const clips=assets.map(asset=>{if(!['svg','png','jpeg'].includes(asset.kind))throw new Error('图片轨道仅插入已提供的静态图片；其他媒体保留原类型编辑。');return {id:`clip-${crypto.randomUUID()}`,phase_ref:phase,start_progress:0,end_progress:1,source:{kind:'image_asset' as const,data:structuredClone(asset)},transition:'hold' as const};});
 const rows=group(value.timeline,phase);
 if(afterId&&!rows.some(c=>c.id===afterId))throw new Error('插入位置的片段已不存在于当前阶段。');
 const at=afterId?rows.findIndex(c=>c.id===afterId)+1:rows.length;
 rows.splice(at,0,...clips);
 return replaceGroup(value,phase,distribute(rows));
}
export function duplicateClip(value:Presentation,id:string):Presentation{
 const clip=value.timeline.find(c=>c.id===id);if(!clip)throw new Error('片段已不存在。');const phase=clip.phase_ref??null,rows=group(value.timeline,phase),index=rows.findIndex(c=>c.id===id);rows.splice(index+1,0,{...structuredClone(clip),id:`clip-${crypto.randomUUID()}`});return replaceGroup(value,phase,distribute(rows));
}
export function removeClip(value:Presentation,id:string):Presentation{const c=value.timeline.find(v=>v.id===id);if(!c)return value;return replaceGroup(value,c.phase_ref??null,distribute(group(value.timeline,c.phase_ref??null).filter(v=>v.id!==id)));}
export function interval(value:Presentation,id:string,start:number,end:number):Presentation{
 const clip=value.timeline.find(c=>c.id===id);if(!clip)throw new Error('片段已不存在。');const phase=clip.phase_ref??null,rows=group(value.timeline,phase),index=rows.findIndex(c=>c.id===id);
 if(!Number.isFinite(start)||!Number.isFinite(end)||start<0||end>1||start>=end||index===0&&start!==0||index===rows.length-1&&end!==1||index>0&&start<=rows[index-1].start_progress||index<rows.length-1&&end>=rows[index+1].end_progress)throw new Error('区间须连续覆盖0—1；边界调整不能跨越相邻片段。');
 const next=rows.map((c,i)=>i===index?{...c,start_progress:start,end_progress:end}:i===index-1?{...c,end_progress:start}:i===index+1?{...c,start_progress:end}:c);return replaceGroup(value,phase,next);
}
export function newTag():DeviceTag{return {id:`tag-${crypto.randomUUID()}`,label:'说明',anchor:{kind:'image_normalized',data:{x:.1,y:.1}},progress_range:null,payload:{kind:'annotation',data:'请输入说明'}};}
export function presentationIssues(value:Presentation):string[]{
 const issues:string[]=[];if(value.production_effect!=='none')issues.push('表现不能声明生产效果。');
 if(new Set(value.timeline.map(c=>c.id)).size!==value.timeline.length)issues.push('片段ID重复。');
 for(const phase of new Set(value.timeline.map(c=>c.phase_ref??null))){const rows=group(value.timeline,phase);rows.forEach((c,i)=>{if(!Number.isFinite(c.start_progress)||!Number.isFinite(c.end_progress)||c.start_progress>=c.end_progress||c.start_progress!==(i?rows[i-1].end_progress:0)||i===rows.length-1&&c.end_progress!==1)issues.push(`${phase??'单阶段'} / ${c.id}：须按顺序连续覆盖u=0—1。`);});}
 for(const tag of value.tags){if(tag.anchor.kind!=='scene_node'&&(!Number.isFinite(tag.anchor.data.x)||!Number.isFinite(tag.anchor.data.y)||tag.anchor.data.x<0||tag.anchor.data.x>1||tag.anchor.data.y<0||tag.anchor.data.y>1))issues.push(`${tag.label}：锚点坐标须在0—1之间。`);if(tag.anchor.kind==='scene_node'&&!value.nodes[tag.anchor.data])issues.push(`${tag.label}：部件锚点不存在。`);}
 return issues;
}
/** Atomic editable slices: never silently overwrite a concurrently edited presentation/behavior. */
export function reconcile(base:ModuleDefinition,mine:ModuleDefinition,current:ModuleDefinition,choices:Record<string,'mine'|'current'>={}){
 let value=current;const conflicts:string[]=[];
 for(const key of ['presentation','behavior'] as const){const get=(d:ModuleDefinition)=>key==='presentation'?presentationOf(d):inlineOperation(d),a=get(base),b=get(mine),c=get(current);if(same(a,b))continue;
  if(!same(a,c)&&!same(b,c)&&!choices[key]){conflicts.push(key);continue;}
  if(choices[key]==='current')continue;
  if(key==='presentation'&&b)value=withPresentation(value,b as Presentation);else if(key==='behavior'&&b)value=withOperation(value,b as OperationDefinition);
 }
 return {value,conflicts};
}
