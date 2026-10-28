import {MediaPresentationView,MEDIA_DATA_FIELD_CATALOG,resolveMediaData,type MediaPresentationCommonProps} from '../media';
import {projectPhase,mediaDataFields,type ActualMediaFrame} from '../../../../../packages/presentation/src';
import type {PresentationMediaPort} from './types';

type Source = Pick<MediaPresentationCommonProps,'binding'|'readAsset'|'resolveModule'|'rights'|'nodeOwner'>;
export interface ImageTimelineMediaOptions {
  declaration: Source;
  /** Exact presentation, bytes and operation frozen for this actual Run. */
  actual: (Source & Pick<MediaPresentationCommonProps,'presentation'> & {frame:ActualMediaFrame}) | null;
  unavailable_reason?: string;
  onActualSeek?(at_ms:number):void;
  onSelectNode?:MediaPresentationCommonProps['onSelectNode'];
  onInspectItem?:MediaPresentationCommonProps['onInspectItem'];
}

/** Wiring only: all decoding, transitions, fields and phase projection stay shared. */
export function createImageTimelineMedia(options:ImageTimelineMediaOptions):PresentationMediaPort {
  const point=options.actual?projectPhase(options.actual.frame):null;
  return {
    fields:options.actual&&point?mediaDataFields({mode:'actual_run',frame:options.actual.frame,point}):MEDIA_DATA_FIELD_CATALOG,
    resolveData(binding,mode){
      if(mode==='isolated_view')return resolveMediaData(binding,{mode:'declared_preview'});
      if(!options.actual||!point)return {status:'unavailable',reason:'unknown_frame',message:options.unavailable_reason??'尚未取得所选Run的实际快照'};
      return resolveMediaData(binding,{mode:'actual_run',frame:options.actual.frame,point});
    },
    render(input){
      const actual=input.mode==='actual_run'?options.actual:null;
      if(input.mode==='actual_run'&&!actual)return <p role="status">{options.unavailable_reason??'正在读取实际Run、历史作业和冻结表现…'}</p>;
      const presentation=actual?.presentation??input.presentation;
      const actionTags=presentation.tags.filter(tag=>tag.payload.kind==='action');
      const unavailable=actionTags.filter(tag=>!input.canAction(tag));
      const onAction=actionTags.some(tag=>input.canAction(tag))?(action:Parameters<NonNullable<MediaPresentationCommonProps['onAction']>>[0])=>{
        const tag=actionTags.find(tag=>tag.payload.kind==='action'&&tag.payload.data===action);if(tag)input.onAction(tag);
      }:undefined;
      const shared={presentation,lod:'detailed' as const,onAction,onSelectNode:options.onSelectNode,onInspectItem:options.onInspectItem};
      return <>{unavailable.length>0&&<p role="status">尚无当前执行入口：{unavailable.map(tag=>tag.label).join('、')}。绑定输入保留。</p>}{actual
        ?<MediaPresentationView {...options.actual!} {...shared} mode="actual_run" frame={actual.frame} onSeek={options.onActualSeek}/>
        :<MediaPresentationView {...options.declaration} {...shared} mode="declared_preview" preview={{phase_ref:input.phase_ref??'cycle',view_u:input.view_u}} onPreviewSeek={value=>input.onSeek(value.view_u)}/>}</>;
    },
  };
}
