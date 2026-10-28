import type {DataTagBinding} from '../../commands/generated/typescript/DataTagBinding';
import type {ValueKind} from '../../commands/generated/typescript/ValueKind';
import type {ActualMediaFrame,ActualPortOutputFrame,PhasePoint} from './types';
import type {PortRef} from '../../commands/generated/typescript/PortRef';
import type {Quantity} from '../../commands/generated/typescript/Quantity';
import type {ModuleRef} from '../../commands/generated/typescript/ModuleRef';

const durationUnits={ms:1,millisecond:1,s:1000,second:1000,min:60000,minute:60000,h:3600000,hour:3600000} as const;
export interface MediaDataField {
  label:string;
  binding:DataTagBinding;
  object_kind:'job'|'run'|'device';
  value_types:readonly ValueKind[];
  units:readonly string[];
  limitations:string;
  unavailable_reason?:string;
}
const textTypes=['enum','string'] as const,numericTypes=['duration','integer','number'] as const;
/** The original five static fields; real device port presets are built separately. */
export const MEDIA_DATA_FIELD_CATALOG:readonly MediaDataField[]=[
  {label:'作业状态',binding:{source_field:'job.state',value_type:'enum',object_ref:null,port_ref:null,unit:null},object_kind:'job',value_types:textTypes,units:[],limitations:'当前所选实际JobState.state；不由动画推断。'},
  {label:'作业当前阶段',binding:{source_field:'job.phase_ref',value_type:'enum',object_ref:null,port_ref:null,unit:null},object_kind:'job',value_types:textTypes,units:[],limitations:'当前实际JobState.phase_ref；无阶段时unavailable，不拿等待画面阶段替代。'},
  {label:'作业当前阶段有效工时',binding:{source_field:'job.effective_progress_ms',value_type:'duration',object_ref:null,port_ref:null,unit:'ms'},object_kind:'job',value_types:numericTypes,units:Object.keys(durationUnits),limitations:'当前实际JobState.effective_progress_ms；不拿上阶段保持姿态的有效工时冒充当前Job字段。'},
  {label:'Run已知计算边界',binding:{source_field:'run.computed_until_ms',value_type:'integer',object_ref:null,port_ref:null,unit:'ms'},object_kind:'run',value_types:numericTypes,units:Object.keys(durationUnits),limitations:'Run原始已知边界，非观看时点；null时unavailable。'},
  {label:'Run计算状态',binding:{source_field:'run.computation_status',value_type:'enum',object_ref:null,port_ref:null,unit:null},object_kind:'run',value_types:textTypes,units:[],limitations:'Run实际计算状态；观看暂停不改该值。'},
];
export interface ActualDeviceMediaFrame extends Pick<ActualMediaFrame,'run'|'snapshot'> {
  device:{id:string;module_ref:ModuleRef};
  port_outputs?:ActualPortOutputFrame;
}
export type MediaDataContext={mode:'actual_run';frame:ActualMediaFrame;point:PhasePoint}|{mode:'actual_device';frame:ActualDeviceMediaFrame}|{mode:'declared_preview'|'asset_inspection'};
export type MediaDataValue={status:'available';value:string|number|Quantity;unit:string|null;formatted:string}|{status:'unavailable';reason:'no_actual_source'|'unknown_frame'|'unsupported_field'|'unsupported_port'|'port_unavailable'|'wrong_object'|'wrong_type'|'wrong_unit'|'missing_value';message:string};
const unavailable=(reason:Extract<MediaDataValue,{status:'unavailable'}>['reason'],message:string):MediaDataValue=>({status:'unavailable',reason,message});
const portFields=[['device.completed_output','工序做完量'],['device.accepted_output','已移交量']] as const;
const samePort=(a:PortRef,b:PortRef)=>a.instance_id===b.instance_id&&a.port_id===b.port_id&&a.module_ref.id===b.module_ref.id&&a.module_ref.version===b.module_ref.version;
function knownSnapshot(frame:Pick<ActualMediaFrame,'run'|'snapshot'>){
  return frame.snapshot.run_id===frame.run.id&&frame.snapshot.next_index===null&&frame.run.computed_until_ms!=null
    &&frame.snapshot.at_ms>=frame.run.window.start_ms&&frame.snapshot.at_ms<=frame.run.window.end_ms&&frame.snapshot.at_ms<=frame.run.computed_until_ms;
}
function actualPorts(frame:ActualMediaFrame|ActualDeviceMediaFrame,point?:PhasePoint){
  const source=frame.port_outputs;
  if(!knownSnapshot(frame)||!source?.device||source.run_id!==frame.run.id||source.at_ms!==frame.snapshot.at_ms)return null;
  if('device'in frame){if(frame.device.id!==source.device.id||frame.device.module_ref.id!==source.device.module_ref.id||frame.device.module_ref.version!==source.device.module_ref.version)return null;}
  else {const execution=point?.job?.execution_ref;if(execution?.kind!=='project_device'||execution.data.device_id!==source.device.id)return null;}
  return source;
}
/** Only a same-Run frozen instance supplies selectable device quantity presets. */
export function mediaDataFields(context?:MediaDataContext):readonly MediaDataField[]{
  if(!context||context.mode!=='actual_run'&&context.mode!=='actual_device'||context.mode==='actual_run'&&!context.point.known)return MEDIA_DATA_FIELD_CATALOG;
  const source=actualPorts(context.frame,context.mode==='actual_run'?context.point:undefined);if(!source?.device)return MEDIA_DATA_FIELD_CATALOG;
  const fields:MediaDataField[]=[];
  for(const row of source.ports){
    if(row.port.instance_id!==source.device.id||row.port.module_ref.id!==source.device.module_ref.id||row.port.module_ref.version!==source.device.module_ref.version)continue;
    for(const [source_field,label]of portFields)fields.push({label:`${row.port.instance_id} / ${row.port.port_id} @ ${row.port.module_ref.version} · ${label}`,binding:{source_field,value_type:'quantity',unit:row.unit,object_ref:{kind:'device',id:row.port.instance_id},port_ref:structuredClone(row.port)},object_kind:'device',value_types:['quantity'],units:[row.unit],limitations:source_field==='device.completed_output'?'实际输出形成时累计，原物品类型与单位；不从库存推算。':'实际Job输出移入Location时累计；不是客户接受量或质量放行量。',...(row.status==='unavailable'?{unavailable_reason:row.message}:{})});
  }
  return [...MEDIA_DATA_FIELD_CATALOG,...fields];
}
function portValue(binding:DataTagBinding,frame:ActualMediaFrame|ActualDeviceMediaFrame,point?:PhasePoint):MediaDataValue{
  const source=actualPorts(frame,point);
  if(!source)return unavailable('no_actual_source',frame.port_outputs?.not_applicable??'当前帧没有同一Run/时点的正式设备端口数量源');
  const port=binding.port_ref;if(!port)return unavailable('unsupported_port','设备数量必须明确绑定精确物料输出端口');
  const row=source.ports.find(value=>samePort(value.port,port));
  if(!row||port.instance_id!==source.device!.id||port.module_ref.id!==source.device!.module_ref.id||port.module_ref.version!==source.device!.module_ref.version)return unavailable('unsupported_port','端口或精确模块版本不属于此Run的冻结设备');
  if(binding.object_ref&&(binding.object_ref.kind!=='device'||binding.object_ref.id!==port.instance_id))return unavailable('wrong_object','数量绑定对象必须匹配此实际设备实例');
  if(binding.value_type!=='quantity')return unavailable('wrong_type','端口数量只支持canonical quantity类型');
  if(row.status==='unavailable')return unavailable('port_unavailable',`${row.reason} · ${row.message}`);
  if(row.run_id!==frame.run.id||row.at_ms!==frame.snapshot.at_ms)return unavailable('unknown_frame','端口读数不属于当前Run或时点');
  const quantity=binding.source_field==='device.completed_output'?row.completed_output:row.accepted_output;
  if(binding.unit!==row.unit||quantity.unit!==row.unit)return unavailable('wrong_unit','数量必须保留真实端口的原单位，不换算或合计');
  const scale=quantity.value.decimal_places,digits=String(quantity.value.coefficient).padStart(scale+1,'0');
  const value=scale?`${digits.slice(0,-scale)}.${digits.slice(-scale)}`:digits;
  return {status:'available',value:quantity,unit:quantity.unit,formatted:`${value} ${quantity.unit}`};
}
export function resolveMediaData(binding:DataTagBinding,context:MediaDataContext):MediaDataValue {
  if(context.mode!=='actual_run'&&context.mode!=='actual_device')return unavailable('no_actual_source','声明预览没有实际运行数据');
  const {frame}=context,point=context.mode==='actual_run'?context.point:undefined;
  if(context.mode==='actual_run'? !point?.known||!point.job : !knownSnapshot(frame))return unavailable('unknown_frame','当前实际快照未知');
  if(portFields.some(([field])=>binding.source_field===field))return portValue(binding,frame,point);
  const field=MEDIA_DATA_FIELD_CATALOG.find(value=>value.binding.source_field===binding.source_field);
  if(!field)return unavailable('unsupported_field','未支持此字段；没有可读取的实际源');
  if(binding.port_ref!=null)return unavailable('unsupported_port','此目录未提供端口库存或数量源');
  if(field.object_kind==='job'&&!point?.job)return unavailable('no_actual_source','此实际设备时点没有绑定Job；不伪造作业或阶段字段');
  const object=binding.object_ref,id=field.object_kind==='job'?point!.job!.id:frame.run.id;
  if(object&&(object.kind!==field.object_kind||object.id!==id))return unavailable('wrong_object','绑定对象不是此字段的当前实际对象');
  if(!field.value_types.includes(binding.value_type))return unavailable('wrong_type','字段与声明的数据类型不匹配');
  const values:Record<string,string|number|null|undefined>={'job.state':point?.job?.state,'job.phase_ref':point?.job?.phase_ref,'job.effective_progress_ms':point?.job?.effective_progress_ms,'run.computed_until_ms':frame.run.computed_until_ms,'run.computation_status':frame.run.computation_status};
  let value=values[binding.source_field];if(value==null)return unavailable('missing_value','实际字段未提供');
  const unit=binding.unit??null;
  if(unit){if(!field.units.includes(unit)||typeof value!=='number')return unavailable('wrong_unit','字段不支持此单位');value/=durationUnits[unit as keyof typeof durationUnits];}
  return {status:'available',value,unit,formatted:`${value}${unit?` ${unit}`:''}`};
}
