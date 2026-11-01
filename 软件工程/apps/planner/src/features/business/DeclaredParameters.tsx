import { useEffect, useState } from 'react';
import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { TypedValue } from '../../../../../packages/commands/generated/typescript/TypedValue';
import type { SourceBasis } from '../../../../../packages/commands/generated/typescript/SourceBasis';
import type { BusinessSnapshot, BusinessEditorProps } from './types';
import type { SchemaCatalog } from './schema';
import { emptyValue, fieldSchema, resolve } from './schema';
import { SchemaField, term } from './SchemaForm';
type Command=Extract<EditCommand,{kind:'set_declared_parameter'}>;
interface Props { selectedDeviceId?:string; snapshot:BusinessSnapshot; modules:ReadonlyMap<string,ModulePackage>; catalog:SchemaCatalog; commands:Record<string,Command>; onChange:(commands:Record<string,Command>)=>void; onRemove:(deviceId:string,parameterId:string)=>void; onLocate:BusinessEditorProps['onLocate'] }
export function DeclaredParameters(props:Props){
  const [id,setId]=useState(props.selectedDeviceId??props.snapshot.project.devices[0]?.id??'');
  useEffect(()=>{if(props.selectedDeviceId)setId(props.selectedDeviceId);},[props.selectedDeviceId]);
  const device=props.snapshot.project.devices.find(d=>d.id===id),definition=device?props.modules.get(JSON.stringify([device.module_ref.id,device.module_ref.version]))?.definition:null;
  const payload=definition?.payload.category==='devices'?definition.payload.data:null;
  const selectedRef=device?.operation_binding??(payload?.behavior.selected==='operation_ref'?payload.behavior.operation_ref:null);
  const independent=selectedRef?props.modules.get(JSON.stringify([selectedRef.id,selectedRef.version]))?.definition:null;
  const operation=selectedRef?(independent?.payload.category==='operations'?independent.payload.data:null):payload?.behavior.selected==='inline'?payload.behavior.inline:null;
  return <><h2>设备阶段与声明参数</h2><p>只修改当前精确版本已经公开的实例参数，不改库中不可变定义。未开放的工序/阶段请在模块编辑创建新版本后明确采用。</p><label>选择执行设备<select aria-label="参数设备" value={id} onChange={e=>setId(e.target.value)}><option value="" disabled>请选择</option>{props.snapshot.project.devices.map(d=><option key={d.id} value={d.id}>{d.title} · {d.module_ref.id}@{d.module_ref.version}</option>)}</select></label>
    {id&&!device&&<p role="status">未找到指定设备 {id}，请选择当前工程中的设备。</p>}
    {device&&<><p>设备定义：{device.module_ref.id} @ {device.module_ref.version}</p>{!definition&&<p role="alert">缺少这一精确版本，不能猜测参数或阶段。</p>}
      {operation&&<div className="sf-business-readonly"><h3>冻结声明的阶段与资源</h3><p>家族：{operation.family??'未声明'}；工艺来源 {selectedRef?`${selectedRef.id}@${selectedRef.version}`:'设备内联行为'}</p>{operation.phases.map(phase=><p key={phase.id}><strong>{phase.id}</strong> · {phase.duration?`${phase.duration.value} ms · ${term(phase.duration.basis)}`:'持续时间未声明'} · {term(phase.interruption_policy)} · {phase.phase_resources.map(r=>`${r.role} → ${device.role_bindings.resources[r.role]??'未绑定'}`).join('；')}</p>)}</div>}
      {!operation&&<p>当前未解析到可执行工艺，不把外观或未知字段当作工时。</p>}
      {definition?.parameters.map(parameter=>{
        const key=JSON.stringify([device.id,parameter.id]),stored=props.commands[key],existing=device.configuration[parameter.id];
        const variant=resolve(fieldSchema('TypedValue',props.catalog),props.catalog).oneOf?.find(s=>s.properties?.kind?.const===parameter.value_type);
        if(!variant)return <p key={key}>{parameter.label??parameter.id}：当前字段类型不受此编辑器支持。</p>;
        const inactive=!!selectedRef&&parameter.binding.startsWith('/payload/data/behavior/inline/');
        const value=stored?.data.value??existing??emptyValue(variant,props.catalog),basis=stored?.data.basis??(existing?props.snapshot.project.parameter_sources?.find(s=>s.target.kind==='device'&&s.target.id===device.id&&s.field_path===`/configuration/${parameter.id.replaceAll('~','~0').replaceAll('/','~1')}/value`)?.basis:undefined)??parameter.basis;
        function update(next:unknown,source?:SourceBasis){let value=next as TypedValue;const actualSource=source??(value?.kind==='duration'?value.value?.basis:basis)??'unknown';if(value?.kind==='duration'&&source)value={...value,value:{...value.value,basis:source}};props.onChange({...props.commands,[key]:{kind:'set_declared_parameter',data:{target:{kind:'device',id:device!.id},parameter_id:parameter.id,value,basis:actualSource}}});}
        return <fieldset key={key}><legend>{parameter.label??parameter.id} · {parameter.unit??'未声明单位'}</legend><p>精确绑定：{parameter.binding} · 范围 {parameter.minimum??'不限'}—{parameter.maximum??'不限'}</p>{inactive&&<p>此参数绑定原内联工艺；当前已选择独立工艺，不能将此覆盖套用到新工艺。</p>}{existing===undefined&&!stored&&<p>未设置实例覆盖；执行使用当前所选精确工艺的声明。</p>}<SchemaField schema={variant} catalog={props.catalog} value={value} snapshot={props.snapshot} label={parameter.label??parameter.id} path={`/parameters/${parameter.id}`} readOnly={inactive} onChange={next=>update(next)}/><label>来源性质<select aria-label={`${parameter.id}来源`} disabled={inactive} value={basis} onChange={e=>update(value,e.target.value as SourceBasis)}>{['user_provided','module_declared','template_declared_example','unknown'].map(v=><option key={v} value={v}>{term(v)}</option>)}</select></label>{stored&&<button onClick={()=>{const next={...props.commands};delete next[key];props.onChange(next);}}>撤回此参数改动</button>}{existing!==undefined&&<button onClick={()=>props.onRemove(device.id,parameter.id)}>移除此实例覆盖 · {parameter.label??parameter.id}</button>}</fieldset>;
      })}
      {definition&&!definition.parameters.length&&<p>此精确设备版本没有公开实例参数；没有可直接编辑的阶段别名。</p>}
      <SchemaField schema={resolve(fieldSchema('DeviceInstance',props.catalog),props.catalog).properties!.role_bindings} catalog={props.catalog} value={device.role_bindings} snapshot={props.snapshot} label="实际实例角色绑定（在对应记录编辑）" readOnly onChange={()=>{}}/>
      <button onClick={()=>props.onLocate({object:{kind:'device',id:device.id},source:'draft'})}>定位设备与编辑入口</button>
    </>}
  </>;
}
