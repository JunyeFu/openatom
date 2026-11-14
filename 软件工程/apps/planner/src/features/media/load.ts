import type { MediaLocator } from '../../../../../packages/commands/generated/typescript/MediaLocator';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { AssetByteReader } from '../../../../../packages/presentation/src';
import { lodSource,UnsupportedMedia } from '../../../../../packages/presentation/src';
import type { MediaPresentationProps } from './types';
import type { MediaNodeOwner } from '../node-inspector/types';

export interface ResolvedMedia {presentation:Presentation;locator:MediaLocator|null;read:AssetByteReader;rights:MediaPresentationProps['rights'];owner:MediaNodeOwner|null;source:MediaPresentationProps['binding']['request']['source']}
export async function resolveMedia(props:MediaPresentationProps):Promise<ResolvedMedia> {
  let presentation=props.presentation,locator=lodSource(presentation,props.lod),source=props.binding.request.source,rights=props.rights,owner=props.nodeOwner??null;
  const seen=new Set<string>();
  while(locator?.kind==='media_ref') {
    const key=JSON.stringify(locator.data);if(seen.has(key))throw new UnsupportedMedia('媒体依赖存在循环。');seen.add(key);
    if(!props.resolveModule)throw new UnsupportedMedia('当前无法读取此素材引用的模块版本。');
    source=source.kind==='run_module_version'?{kind:source.kind,data:{...source.data,module_ref:locator.data}}:{kind:source.kind,data:{module_ref:locator.data}};
    const pkg=await props.resolveModule(source);
    if(pkg.definition.id!==locator.data.id||pkg.definition.version!==locator.data.version||pkg.definition.payload.category!=='media')throw new UnsupportedMedia('返回的素材版本不匹配，请重新选择素材。');
    owner={object_id:{kind:'module',id:pkg.definition.id},source};
    presentation=pkg.definition.payload.data;rights=pkg.rights;locator=lodSource(presentation,props.lod);
  }
  const request={...props.binding.request,source};
  return {presentation,locator,rights,owner,source,read:asset=>props.readAsset({...request,asset})};
}
