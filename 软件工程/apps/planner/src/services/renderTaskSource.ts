import type {ModuleRef} from '../../../../packages/commands/generated/typescript/ModuleRef';
import type {ModuleDefinition} from '../../../../packages/commands/generated/typescript/ModuleDefinition';
import type {ModulePackage} from '../../../../packages/commands/generated/typescript/ModulePackage';
import type {ProjectState} from '../../../../packages/commands/generated/typescript/ProjectState';
import type {RunObjectsPage} from '../../../../packages/commands/generated/typescript/RunObjectsPage';
import type {ObjectRef} from '../../../../packages/commands/generated/typescript/ObjectRef';
import type {ObjectSource} from '../../../../packages/commands/generated/typescript/ObjectSource';
import type {MediaPresentationProps} from '../features/media/types';
import type {MediaAssetReadRequest} from './mediaAssets';
import {projectFactoryPresentation,type FactoryPresentationContext,type FactoryPresentationSource} from './factoryPresentation';
import {presentationOf,ProcessUnavailable,readActualPortOutputs,resolveProcessOperation} from './presentationHost';
import {renderTaskScope,type NativeRenderInput,type RenderTaskPorts} from './renderTask';

// Frozen DTO identity is structural; JSON object member order carries no identity.
const same=(a:unknown,b:unknown):boolean=>{
  if(a===b)return true;
  if(!a||!b||typeof a!=='object'||typeof b!=='object')return false;
  if(Array.isArray(a))return Array.isArray(b)&&a.length===b.length&&a.every((value,index)=>same(value,b[index]));
  if(Array.isArray(b))return false;
  const left=a as Record<string,unknown>,right=b as Record<string,unknown>,keys=Object.keys(left);
  return keys.length===Object.keys(right).length&&keys.every(key=>Object.hasOwn(right,key)&&same(left[key],right[key]));
};
const sameRef=(a:ModuleRef,b:ModuleRef)=>a.id===b.id&&a.version===b.version;
export function renderModuleSource(task:NativeRenderInput,reference:ModuleRef):MediaAssetReadRequest['source']{
  const view=task.frame.request.view;
  return view.run_id!=null?{kind:'run_module_version',data:{run_id:view.run_id,at_ms:view.view_time_ms!,module_ref:reference}}:{kind:'module_version',data:{module_ref:reference}};
}
export interface PreparedRenderTask {
  context:FactoryPresentationContext;
  factory?:FactoryPresentationSource;
  template?:FactoryPresentationSource;
  media?:MediaPresentationProps;
  definition?:ModuleDefinition;
  observation:RunObjectsPage|null;
  target:ObjectRef|null;
  limitations:string[];
}
/** Consume the frozen producer DTO directly. Never implements a dispatcher or reconstructs business history. */
export async function prepareRenderTask(task:NativeRenderInput,ports:RenderTaskPorts,assertCurrent:()=>void):Promise<PreparedRenderTask>{
  const {frame,input}=task,view=frame.request.view,scope=renderTaskScope(frame);
  const context:FactoryPresentationContext={session_id:frame.task_id,project_id:frame.project_id,scope,scope_revision:frame.scope_revision,project:input.project,view:view.run_id!=null?{run_id:view.run_id,at_ms:view.view_time_ms!}:null};
  const observation:RunObjectsPage|null=input.run&&context.view?{run_id:input.run.id,at_ms:context.view.at_ms,next_index:null,objects:[...input.objects_at,...input.project.resources.filter(resource=>!input.objects_at.some(object=>object.kind==='resource'&&object.data.id===resource.id)).map(data=>({kind:'resource' as const,data}))]}:null;
  const target=frame.request.object_id??view.object_ref??null;
  const output:PreparedRenderTask={context,observation,target,limitations:[]};
  assertCurrent();
  if(input.project.id!==frame.project_id||input.project.revision!==view.project_revision)throw new ProcessUnavailable('Task冻结工程与完整RenderFrame不一致。');
  if(input.run&&(view.run_id!==input.run.id||!context.view||input.run.input_ref.project_revision!==input.project.revision||input.run.computed_until_ms==null||context.view.at_ms<input.run.window.start_ms||context.view.at_ms>input.run.computed_until_ms||context.view.at_ms>input.run.window.end_ms))throw new ProcessUnavailable('Task实际Run/时点与冻结输入或计算前沿不一致。');
  const module=async(reference:ModuleRef):Promise<ModulePackage>=>{
    assertCurrent();const found=input.modules.filter(value=>sameRef(value.definition,reference));
    if(found.length!==1)throw new ProcessUnavailable('Task闭包中没有唯一精确模块版本。');return found[0];
  };
  const readAsset=async(request:MediaAssetReadRequest)=>{
    assertCurrent();if(request.project_id!==context.project_id||request.scope_revision!==context.scope_revision||!same(request.scope,scope)||!same(request.source,renderModuleSource(task,request.source.data.module_ref)))throw new ProcessUnavailable('Task媒体请求改变了工程/Scope修订/Run精确来源。');
    if(!input.assets.some(value=>same(value.source,request.source)&&same(value.asset,request.asset)))throw new ProcessUnavailable('Task素材不在真实冻结字节闭包。');
    const bytes=await ports.asset({source:request.source,asset:request.asset});assertCurrent();return bytes;
  };
  const readPortOutput=async(request:Parameters<NonNullable<import('./presentationHost').PresentationReadPorts['readPortOutput']>>[0])=>{
    assertCurrent();
    if(request.project_id!==context.project_id||request.scope_revision!==context.scope_revision||!same(request.scope,scope)||request.run_id!==input.run?.id||request.run_id!==context.view?.run_id||request.at_ms!==context.view?.at_ms)throw new ProcessUnavailable('Task端口请求改变了工程/Scope修订/Run或精确时点。');
    const value=input.port_output_summaries.find(value=>value.run_id===request.run_id&&value.at_ms===request.at_ms&&same(value.port,request.port));
    if(value)return value;
    const failure=input.port_output_failures?.find(([port])=>same(port,request.port));
    if(failure)throw {detail:failure[1]};
    const issue=input.issues.find(issue=>['render.port_output_not_applicable','render.missing_port_output_history'].includes(issue.code));
    if(issue)throw {detail:{code:'UnsupportedCapability',message:`${issue.code}: ${issue.message}`}};
    throw new ProcessUnavailable('真实Render输入没有此精确端口累计量，不把空列表当零。');
  };
  const factory=async(project:ProjectState)=>projectFactoryPresentation({...context,project},{project,run:input.run,scenario:input.scenarios.find(s=>s.id===input.run?.input_ref.scenario_id),observation,jobs:input.objects_at.flatMap(object=>object.kind==='job'?[object.data]:[]),items:input.objects_at.flatMap(object=>object.kind==='item'?[object.data]:[]),raw:{job_pages:[],item_pages:[],frozen_task:frame.task_id}},{module,readAsset,readPortOutput},assertCurrent);
  if(view.kind==='line_canvas'||view.kind==='multi_hall_canvas'){output.factory=await factory(input.project);return output;}
  if((view.kind==='packaging_workbench'||view.kind==='packaging_manufacturing')&&!input.packaging)throw new ProcessUnavailable('Task缺少实际Artifact/Result/Job三元组。');
  if(view.kind==='comparison'&&(!input.comparison||input.comparison_runs.length!==input.comparison.run_ids.length||!input.comparison.run_ids.every(id=>input.comparison_runs.some(value=>value.run.id===id))))throw new ProcessUnavailable('Task比较缺少原compare_runs及每个Run的冻结工程/方案输入。');
  if(view.kind!=='device_detail'&&view.kind!=='device_cycle_preview'&&view.kind!=='module_preview')return output;
  let definition:ModuleDefinition,reference:ModuleRef,pkg:ModulePackage|null=null;
  let device:ProjectState['devices'][number]|null=null;
  if(target?.kind==='device'){
    device=input.project.devices.find(device=>device.id===target.id)??null;
    if(!device)throw new ProcessUnavailable('精确目标设备不在Task冻结工程。');
    reference=device.module_ref;pkg=await module(reference);definition=pkg.definition;
  }else{
    if(target?.kind==='definition_draft'){
      const identity=frame.source.kind==='definition_draft'?frame.source.data:frame.source.kind==='run'?frame.source.data.definition:null;
      if(!identity||identity.id!==target.id||!input.definition)throw new ProcessUnavailable('预览不是原opaque Draft/Run的精确定义。');
      definition=input.definition;reference={id:definition.id,version:definition.version};
      if(view.kind==='module_preview'&&(!frame.request.module_ref||!sameRef(definition,frame.request.module_ref)))throw new ProcessUnavailable('模块预览缺少原Host规范化的精确ModuleRef。');
    }else{const ref=frame.request.module_ref;if(!ref)throw new ProcessUnavailable('模块预览缺少精确ModuleRef。');reference=ref;pkg=await module(ref);definition=pkg.definition;}
  }
  output.definition=definition;
  if(definition.payload.category==='templates'){
    if(!input.template_subset)throw new ProcessUnavailable('真实模板子集尚未包含在冻结输入中。');
    const subset=input.template_subset;
    const projected={...input.project,halls:subset.halls,lines:subset.lines,devices:subset.devices,locations:subset.locations,connections:subset.connections,module_refs:subset.module_refs,item_types:subset.item_types,resources:[]};
    output.template=await projectFactoryPresentation({...context,project:projected},{project:projected,run:null,observation:null,jobs:[],items:[],raw:{job_pages:[],item_pages:[],frozen_task:frame.task_id}},{module,readAsset,readPortOutput},assertCurrent);
    output.limitations.push('只读显示精确TemplateSubset；未实例化、采用角色、供料或库存；即使来源为旧Run，也不把该Run库存附着到模板。');return output;
  }
  const presentation=device?.presentation_override??presentationOf(definition);
  if(!presentation){output.limitations.push(`精确定义类别 ${definition.payload.category} 没有可显示的Presentation；仅展示真实声明。`);return output;}
  const source=renderModuleSource(task,reference),binding={session_id:context.session_id,request:{project_id:context.project_id,scope,scope_revision:context.scope_revision,source}};
  const nodeOwner={object_id:target?.kind==='definition_draft'?target:device?.presentation_override?{kind:'device' as const,id:device.id}:{kind:'module' as const,id:reference.id},source:device?.presentation_override?input.run?{kind:'run_snapshot' as const,data:{run_id:input.run.id,at_ms:context.view!.at_ms}}:{kind:'draft' as const}:target?.kind==='definition_draft'?input.run?{kind:'run_snapshot' as const,data:{run_id:input.run.id,at_ms:context.view!.at_ms}}:{kind:'draft' as const}:source};
  const common={presentation,binding,readAsset,resolveModule:async(next:ObjectSource)=>{
    if(next.kind!=='module_version'&&next.kind!=='run_module_version'||!same(next,renderModuleSource(task,next.data.module_ref)))throw new ProcessUnavailable('媒体依赖改变了Task精确来源。');return module(next.data.module_ref);
  },lod:view.lod,rights:pkg?.rights??definition.contribution.assets,nodeOwner};
  if(input.run&&(view.kind==='device_cycle_preview'||view.kind==='device_detail'&&input.active_job_id!=null)){
    const matches=input.objects_at.flatMap(object=>object.kind==='job'&&object.data.id===input.active_job_id?[object.data]:[]);
    if(matches.length!==1)throw new ProcessUnavailable('实际周期没有唯一活动JobsAt；不从末态Job补造阶段。');
    const job=matches[0];
    const execution=job.execution_ref;
    const frozenFixture=execution.kind==='project_device'?input.project.devices.find(value=>value.id===execution.data.device_id&&sameRef(value.module_ref,reference))??null:null;
    if(device?execution.kind!=='project_device'||execution.data.device_id!==device.id:execution.kind==='definition_cycle'?!sameRef(execution.data.module_ref,reference):!frozenFixture)throw new ProcessUnavailable('活动Job不属于精确周期目标。');
    const {operation}=await resolveProcessOperation(definition,reference,job,device??frozenFixture,module);
    const port_outputs=await readActualPortOutputs(context,{run_id:input.run.id,at_ms:context.view!.at_ms,device:scope.kind==='project'?device:null,ports:definition.payload.category==='devices'?definition.payload.data.ports:[]},{readPortOutput},assertCurrent);
    output.media={...common,mode:'actual_run',frame:{run:input.run,snapshot:observation!,job_id:job.id,operation,port_outputs},readItems:async()=>{
      if(execution.kind!=='project_device')throw new ProcessUnavailable('DefinitionCycle没有正式采用设备的空间holder；不绘制推测附件。');
      const source=await factory(input.project),device_id=execution.data.device_id;
      assertCurrent();
      return {source,device_id,unavailable:source.devices.find(value=>value.device_id===device_id)?.frame?.job_id===job.id?null:
        '当前设备没有由所选Job唯一持有的实际姿态；不把其他Job的物品挂到此历史动作。'};
    }};
  }else if(frame.request.preview?.progress!=null||frame.request.preview?.phase_id!=null){
    output.media={...common,mode:'declared_preview',preview:{phase_ref:frame.request.preview.phase_id??'cycle',view_u:frame.request.preview.progress??0}};
  }else output.media={...common,mode:'asset_inspection',inspectionLabel:frame.request.preview?.pose_name?`声明命名姿态 ${frame.request.preview.pose_name} · 无实际Run`:input.run?'Run冻结静态声明 · 非运行时世界姿态':'精确源声明静态检查'};
  assertCurrent();return output;
}
