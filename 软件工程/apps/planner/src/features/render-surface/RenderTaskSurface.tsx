import {useCallback,useEffect,useMemo,useRef,useState} from 'react';
import {nativeRenderPorts,renderTaskKey,renderTaskScope,renderTaskSourceKey,type NativeRenderInput,type RenderTaskPorts,type RenderTaskReady} from '../../services/renderTask';
import {prepareRenderTask,type PreparedRenderTask} from '../../services/renderTaskSource';
import {FactoryRenderSurface} from './FactoryRenderSurface';
import {FactoryScene} from '../../scene/FactoryScene';
import {MediaPresentationView} from '../media/MediaPresentationView';
import {ProcessViewport} from '../packaging/ProcessViewport';
import {ManufacturingPreview} from '../packaging/ManufacturingPreview';
import type {PackagingEvidence} from '../packaging/types';
import {RunTimeline} from '../timeline/RunTimeline';
import {ComparisonResult} from '../comparison/ComparisonResult';
import type {ObjectRef} from '../../../../../packages/commands/generated/typescript/ObjectRef';
import {domPainted,type ReadonlyVisualCapture} from './visualCapture';
import type {FactorySceneCapture,FactorySceneFrame} from './factoryCapture';
import '../reporting/delivery.css';
import './render-task.css';

export interface RenderTaskSurfaceProps {task:NativeRenderInput;ports?:RenderTaskPorts}
/** One trusted immutable task. All nine kinds share the product's actual view components. */
export function RenderTaskSurface({task,ports=nativeRenderPorts}:RenderTaskSurfaceProps){
  const {frame,input}=task,view=frame.request.view,key=renderTaskKey(frame);
  const root=useRef<HTMLElement>(null),liveKey=useRef(key),complete=useRef(false),frameId=useRef(0);liveKey.current=key;
  const [loadedSource,setPrepared]=useState<{key:string;value:PreparedRenderTask}|null>(null),[error,setError]=useState('');
  const prepared=loadedSource?.key===key?loadedSource.value:null;
  const fail=useCallback((stage:string,message:string,object_ref:ObjectRef|null=null)=>{
    if(liveKey.current!==key||complete.current)return;complete.current=true;setError(message);
    void ports.failure({frame,stage,message,object_ref}).catch(reason=>{if(liveKey.current===key)setError(`${message}；失败回执拒绝：${String(reason)}`);});
  },[key,ports,frame]);
  useEffect(()=>{
    let live=true;liveKey.current=key;complete.current=false;setPrepared(null);setError('');
    const assertCurrent=()=>{if(!live||liveKey.current!==key)throw new Error('Task已销毁或精确帧已切换；丢弃旧响应。');};
    prepareRenderTask(task,ports,assertCurrent).then(value=>{if(live)setPrepared({key,value});},reason=>{if(live)fail('input',reason instanceof Error?reason.message:String(reason));});
    return()=>{live=false;if(liveKey.current===key)liveKey.current='';};
  },[key,task,ports,fail]);
  const ready=useCallback(async(value:{visible_objects:ObjectRef[];limitations:string[]}|FactorySceneFrame)=>{
    if(complete.current||liveKey.current!==key)return;
    const current=()=>liveKey.current===key&&!complete.current;
    if(!await domPainted(current)||!root.current)return;
    const bounds=root.current.getBoundingClientRect(),ratio=devicePixelRatio;
    if(!(bounds.width>0&&bounds.height>0))return;
    const physical={width:Math.round(bounds.width*ratio),height:Math.round(bounds.height*ratio),css_width:bounds.width,css_height:bounds.height,pixel_ratio:ratio};
    const limitations=[...input.issues.map(issue=>({object_ref:issue.object_ref??null,message:`${issue.code}: ${issue.message}`})),...(prepared?.limitations??[]).map(message=>({object_ref:null,message})),...value.limitations.map(value=>typeof value==='string'?{object_ref:null,message:value}:value)];
    const output:RenderTaskReady={key,frame_id:++frameId.current,camera:view.camera,lod:view.lod,pixels:physical,visible_objects:value.visible_objects,limitations,source_key:renderTaskSourceKey(frame),project_id:frame.project_id,scope:renderTaskScope(frame),scene_revision:frame.scope_revision,project_revision:view.project_revision,view};
    complete.current=true;
    try{await ports.ready({frame,ready:output});}catch(reason){if(liveKey.current!==key)return;complete.current=false;fail('ready',reason instanceof Error?reason.message:String(reason));}
  },[key,frame,view,input,prepared,ports,fail]);
  const target=prepared?.target??frame.request.object_id??view.object_ref??null;
  const capture=useMemo<ReadonlyVisualCapture|undefined>(()=>target?{key,camera:view.camera,object:target,poseName:frame.request.preview?.pose_name??undefined,onFrame:value=>{void ready(value);},onFailure:message=>fail('resource',message,target)}:undefined,[key,view.camera,target?.kind,target?.id,frame.request.preview?.pose_name,ready,fail]);
  const factoryCapture=useMemo<FactorySceneCapture>(()=>({key,camera:view.camera,onFrameReady:value=>{void ready(value);},onFailure:failure=>fail(failure.stage,failure.message,failure.object_ref??null)}),[key,view.camera,ready,fail]);
  const packaging=input.packaging;
  const evidence=useMemo<PackagingEvidence|null>(()=>packaging?{artifact:packaging.artifact,result:packaging.result,input:packaging.job,inputReadIssue:null}:null,[packaging]);
  const readFile=useCallback(async(value:PackagingEvidence,file:Parameters<RenderTaskPorts['file']>[0]['file'])=>{
    if(liveKey.current!==key||value.artifact.id!==input.packaging?.artifact.id||!input.files.some(entry=>entry.file.path===file.path&&entry.file.kind===file.kind))throw new Error('制造文件不属于当前Task的真实Artifact闭包。');
    const bytes=await ports.file({file});if(liveKey.current!==key)throw new Error('制造Task已切换；丢弃迟到字节。');return bytes;
  },[key,input,ports]);
  const staticContent=prepared&&(!prepared.media&&!prepared.factory&&!prepared.template)&&(view.kind==='module_preview'||view.kind==='device_detail'||view.kind==='device_cycle_preview'||view.kind==='comparison');
  useEffect(()=>{
    if(!staticContent)return;let live=true;
    domPainted(()=>live).then(done=>{if(done)void ready({visible_objects:view.kind==='comparison'?input.comparison_runs.map(value=>({kind:'run' as const,id:value.run.id})):target?[target]:[],limitations:view.kind==='comparison'?['二维比较表不适用三维相机；结论/可比性/限制均来自原compare_runs。']:['无可视Presentation，只显示真实声明；此二维声明面板不适用三维相机，未生成任何替代几何。']});});return()=>{live=false;};
  },[staticContent,ready,target?.kind,target?.id,input.comparison_runs,view.kind]);
  let content;
  if(prepared){
    if(prepared.factory)content=<FactoryRenderSurface context={prepared.context} source={prepared.factory} view={view} target={target} onFrameReady={value=>{void ready(value);}} onFailure={failure=>fail(failure.stage,failure.message,failure.object_ref??null)}/>;
    else if(prepared.template)content=<><FactoryScene project={prepared.template.project} mediaSource={prepared.template} canvas={view.hall_id?'line_canvas':'multi_hall_canvas'} hallId={view.hall_id??null} selection={null} onSelect={()=>{}} lod={view.lod} autoLod={false} resetKey={0} capture={factoryCapture}/><p className="render-task-template-note">精确TemplateSubset只读预览 · {view.hall_id?'请求厂房':'模板全部厂房'} · 未采用到工程</p></>;
    else if(prepared.media)content=<MediaPresentationView {...prepared.media} capture={capture}/>;
    else if((view.kind==='packaging_workbench'||view.kind==='packaging_manufacturing')&&packaging&&evidence&&capture)content=<section className="render-task-packaging"><header><h1>实际包装成果 · {packaging.result.outcome}</h1><p>Artifact {packaging.artifact.id} · Job {packaging.job.id}@{packaging.job.revision} · {packaging.sample?`${packaging.sample.step_id} / ${packaging.sample.progress}`:'明确平面输入状态'} · 未认可或绑定</p></header>{view.kind==='packaging_manufacturing'?<ManufacturingPreview evidence={evidence} readFile={readFile} capture={capture}/>:<><ProcessViewport job={packaging.job} sample={packaging.sample} capture={capture}/><div className="render-task-checks">{packaging.result.checks.map((check,index)=><p key={index}>{check.purpose} · {check.outcome} · 查询 {check.queries_used}{check.witnesses.map((w,i)=><span key={i}> · {w.message} [{w.interval.join('—')}]</span>)}{check.unresolved.map((issue,i)=><span key={i}> · {issue.code}: {issue.message}</span>)}</p>)}</div></>}</section>;
    else if(view.kind==='timeline_snapshot'){
      const scenario=input.run?input.scenarios.find(value=>value.id===input.run!.input_ref.scenario_id&&value.revision===input.run!.input_ref.scenario_revision):input.scenarios.find(value=>target?.kind==='scenario'&&value.id===target.id);
      content=<RunTimeline project={input.project} draftProject={input.run?null:input.project} run={input.run} atMs={view.view_time_ms??null} scenario={scenario} runScenario={input.run?scenario:null} observation={prepared.observation} readonly onFrameReady={value=>{void ready(value);}} onTime={()=>{throw new Error('只读Task不寻址Run。');}} onSelect={()=>{throw new Error('只读Task不导航产品选择。');}}/>;
    }else if(view.kind==='comparison'&&input.comparison)content=<ComparisonResult value={input.comparison} evidence={input.comparison_runs}/>;
    else if(prepared.definition)content=<section className="render-task-declaration" data-render-declaration="missing-presentation"><h1>{prepared.definition.title}</h1><p>{prepared.definition.id}@{prepared.definition.version} · {prepared.definition.payload.category}</p>{prepared.limitations.map((message,index)=><p key={index}>{message}</p>)}{input.run?<p>冻结 Run {input.run.id} · {view.view_time_ms} ms · {input.active_job_id?'以下仅为原JobsAt返回的实际身份与字段，未绘制运动姿态':'冻结静态声明；未提供唯一实际活动Job，不生成阶段或进度'}</p>:<p>声明检查；未连接实际 Run</p>}{input.objects_at.flatMap(object=>object.kind==='job'&&object.data.id===input.active_job_id?[<p key={object.data.id} data-real-job-id={object.data.id}>实际 Job {object.data.id} · {object.data.state} · phase {object.data.phase_ref??'无当前阶段'} · 有效累计 {object.data.effective_progress_ms} ms</p>]:[])}<pre>{JSON.stringify(prepared.definition.payload,null,2)}</pre></section>;
    else content=<p role="alert">冻结输入缺少此视图的精确消费来源。</p>;
  }
  return <main className={view.kind==='comparison'?'render-task-surface delivery-workspace':'render-task-surface'} ref={root} data-render-kind={view.kind} data-render-task={frame.task_id} data-render-key={key} aria-label="九视图同源只读渲染">{error?<p role="alert">{error}</p>:prepared?content:<p role="status">正在接收精确冻结渲染来源…</p>}</main>;
}
