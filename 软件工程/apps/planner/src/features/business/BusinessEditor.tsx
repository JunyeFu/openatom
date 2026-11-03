import { factoryItemTypeTitle, factoryUnitLabel } from '../../scene/factoryLabels';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { ApiError } from '../../../../../packages/commands/generated/typescript/ApiError';
import type { BusinessEditorProps, BusinessSnapshot, BusinessCommitProposal } from './types';
import { commitProposal, copyRecord, setupPoliciesEqual, getPath, mergeDraft, mergeParameterCommands, removeInstanceOverride, setPath } from './draft';
import type { MergeConflict } from './draft';
import { decimalText, emptyValue, fieldSchema, loadBusinessSchema, resolve, validateForm } from './schema';
import type { Schema, SchemaCatalog, FormIssue } from './schema';
import { SchemaField, labels } from './SchemaForm';
import { sections } from './sections';
import { BusinessRuntime } from './BusinessRuntime';
import { DeclaredParameters } from './DeclaredParameters';
import { previewCommands } from './preview';
import { historyBoundaryLabel, scenarioName } from '../../services/workbenchLabels';
import './business.css';

export function BusinessEditor(props: BusinessEditorProps) {
  if (!props.project) return <section className="sf-business" aria-label="业务输入"><h2>业务输入</h2><p>先创建或打开工程，再配置业务输入。</p><button onClick={props.onClose}>关闭面板</button></section>;
  return <BusinessSession key={props.project.id} {...props} project={props.project} />;
}
const typeForKind: Record<string,string>={scenario:'ScenarioInput',item_type:'ItemType',location:'StockLocation',resource:'ResourceInstance',calendar:'Calendar',product:'ProductRevision',bom:'BomDefinition',route:'RouteDefinition',device:'DeviceInstance'};
const labelForObject=(value:unknown)=>value&&typeof value==='object'?String((value as Record<string,unknown>).title??(value as Record<string,unknown>).id??'未命名行'):String(value??'未填写');
function BusinessSession(props: BusinessEditorProps & {project:NonNullable<BusinessEditorProps['project']>}) {
  const [base,setBase]=useState<BusinessSnapshot>({project:props.project,scenarios:[...props.scenarios]}),[draft,setDraft]=useState(base);
  const [dirty,setDirty]=useState(false),[catalog,setCatalog]=useState<SchemaCatalog|null>(null),[schemaError,setSchemaError]=useState('');
  const [section,setSection]=useState('orders'),[rowIndex,setRowIndex]=useState<Record<string,number>>({}),[query,setQuery]=useState('');
  const [selectedScenarioId,setSelectedScenarioId]=useState<string|null>(props.scenarioId??props.scenarios[0]?.id??null);
  const [proposal,setProposal]=useState<BusinessCommitProposal|null>(null),[issues,setIssues]=useState<FormIssue[]>([]),[failure,setFailure]=useState<ApiError|null>(null),[notice,setNotice]=useState('');
  const [pending,setPending]=useState(false),[closing,setClosing]=useState(false),[history,setHistory]=useState<EditCommit|null>(null);
  const [parameterCommands,setParameterCommands]=useState<Record<string,Extract<EditCommand,{kind:'set_declared_parameter'}>>>({});
  const [mergeChoices,setMergeChoices]=useState<Record<string,'mine'|'current'>>({}),[mergeOpen,setMergeOpen]=useState(false);
  const pendingRef=useRef(false),schemaRequest=useRef(0);
  const latest=useMemo<BusinessSnapshot>(()=>({project:props.project,scenarios:[...props.scenarios]}),[props.project,props.scenarios]);
  const stale=base.project.revision!==props.project.revision;
  const scenarioIndex=draft.scenarios.findIndex(s=>s.id===selectedScenarioId),activeScenario=draft.scenarios[scenarioIndex]??null;
  const spec=sections.find(s=>s.id===section);
  const sectionReadOnly=spec?.id==='overrides';
  // Actual snapshot paths have project/scenarios as their two canonical owners.
  const path=spec?(spec.owner==='project'?['project',...spec.path]:['scenarios',scenarioIndex,...spec.path]):[];
  const values=spec?(getPath(draft,path) as unknown[]??[]):[];
  const index=Math.min(rowIndex[section]??0,Math.max(0,values.length-1)),record=values[index];
  const availability=props.history??(history?.new_revision===props.project.revision?history.history:null);
  const merged=useMemo(()=>{
    const snapshot=mergeDraft(base,draft,latest,mergeChoices),parameters=mergeParameterCommands(base,parameterCommands,latest,mergeChoices);
    return {...snapshot,parameters:parameters.commands,conflicts:[...snapshot.conflicts,...parameters.conflicts]};
  },[base,draft,latest,mergeChoices,parameterCommands]);
  const busy=pending||props.busy;
  useEffect(()=>{
    const request=++schemaRequest.current;loadBusinessSchema().then(value=>{if(request===schemaRequest.current)setCatalog(value);},error=>{if(request===schemaRequest.current)setSchemaError((error as Error).message);});
    return()=>{schemaRequest.current++;};
  },[]);
  useEffect(()=>{if(!dirty&&!pending){setBase(latest);setDraft(latest);setProposal(null);setFailure(null);}},[latest]);
  useEffect(()=>{if(props.scenarioId!==null)setSelectedScenarioId(props.scenarioId);},[props.scenarioId]);
  useEffect(()=>{if(selectedScenarioId===null&&draft.scenarios[0])setSelectedScenarioId(draft.scenarios[0].id);},[selectedScenarioId,draft.scenarios]);
  useEffect(()=>{
    const intent=props.editIntent;if(!intent)return;setSection(intent.section);setQuery('');
    const section=sections.find(s=>s.id===intent.section);
    if(section){const input=!dirty&&!pending?latest:draft,selected=input.scenarios.findIndex(s=>s.id===(props.scenarioId??selectedScenarioId)),path=section.owner==='project'?['project',...section.path]:['scenarios',selected,...section.path],values=(getPath(input,path)??[]) as {id?:string}[];if(section.owner==='scenario'&&props.scenarioId!==null)setSelectedScenarioId(props.scenarioId);setRowIndex(current=>({...current,[intent.section]:intent.object_id?values.findIndex(v=>v.id===intent.object_id):Math.max(0,current[intent.section]??0)}));}
  },[props.editIntent?.id]);
  useEffect(()=>{props.onDirtyChange?.({project_id:props.project.id,dirty});return()=>props.onDirtyChange?.({project_id:props.project.id,dirty:false});},[dirty,props.project.id]);
  function change(next:BusinessSnapshot){setDraft(next);setDirty(true);setProposal(null);setIssues([]);setClosing(false);}
  function editRecord(next:unknown){change(setPath(draft,[...path,index],next));}
  function newRow(copy=false){if(!spec||!catalog||spec.owner==='scenario'&&!activeScenario)return;let value=copy?copyRecord(record,spec.type):emptyValue(fieldSchema(spec.type,catalog),catalog);if(value&&typeof value==='object'&&!Array.isArray(value)&&'id' in value&&!copy)value={...value,id:`${spec.id}-${crypto.randomUUID()}`};change(setPath(draft,path,[...values,value]));setRowIndex({...rowIndex,[section]:values.length});}
  function newScenario(copy=false){if(!catalog)return;const value=(copy&&activeScenario?structuredClone(activeScenario):emptyValue(fieldSchema('ScenarioInput',catalog),catalog)) as ScenarioInput;
    value.id=`scenario-${crypto.randomUUID()}`;value.revision=1;value.project_revision=base.project.revision;value.title=copy?`${activeScenario!.title}（副本）`:'新方案';value.time_reference=structuredClone(base.project.time_reference);if(!copy)value.window={start_ms:0,end_ms:null as unknown as number};
    change({...draft,scenarios:[...draft.scenarios,value]});setSelectedScenarioId(value.id);setSection('scenario');
  }
  function checkProposal(){
    if(!catalog||busy)return;setFailure(null);setNotice('');
    try{
      const next=commitProposal(base,draft,props.canEditSetupPolicies===true);next.commands.push(...Object.values(parameterCommands));next.label=`业务输入 ${next.commands.length} 项`;
      const problems=next.commands.flatMap((command,i)=>command.kind==='put_object'?validateForm(fieldSchema(typeForKind[command.data.kind],catalog),command.data.data,catalog,`/commands/${i}/${command.data.kind}`):command.kind==='set_declared_parameter'?validateForm(fieldSchema('TypedValue',catalog),command.data.value,catalog,`/commands/${i}/value`):[]);
      setIssues(problems);if(problems.length){setProposal(null);setNotice('字段尚未齐全，表单保留；请补齐下列输入。');return;}
      if(!next.commands.length){setNotice('没有需要提交的变更。');setDirty(false);setProposal(null);return;}
      setProposal(next);
    }catch(error){setNotice((error as Error).message);}
  }
  async function submit(request:BusinessCommitProposal){
    if(pendingRef.current)return;pendingRef.current=true;setPending(true);setFailure(null);setNotice('');
    try{
      const result=await props.onCommit(request);
      if(!result.ok){setFailure(result.error);setProposal(null);return;}
      if(result.snapshot.project.id!==base.project.id)throw new Error('提交结果属于另一工程，不能替换当前业务表单。');
      const policies=request.commands.flatMap(c=>c.kind==='set_project_settings'&&'setup_policies'in c.data?[c.data.setup_policies]:[])[0];
      if(policies!=null&&!setupPoliciesEqual(policies,result.snapshot.project.setup_policies))throw new Error('Host回读未包含请求的换单矩阵；表单保留，不能标记已保存。');
      setBase(result.snapshot);setDraft(result.snapshot);setHistory(result.commit);setDirty(false);setProposal(null);setParameterCommands({});setMergeOpen(false);setMergeChoices({});setIssues([]);
      if(activeScenario&&result.snapshot.scenarios.some(s=>s.id===activeScenario.id))props.onScenarioChange(activeScenario.id);
      setNotice('输入已应用到工程草稿。下一步运行方案，生成新的运行记录；原有记录保留。');
    }catch(error){setProposal(null);setNotice((error as Error).message);}finally{pendingRef.current=false;setPending(false);}
  }
  function historyAction(kind:'undo'|'redo'){
    if(dirty||busy)return;
    void submit({project_id:props.project.id,base_project_revision:props.project.revision,scenario_revisions:{},label:kind==='undo'?'撤销草稿编辑':'重做草稿编辑',commands:[{kind}]});
  }
  async function refresh(){try{await props.onRefresh();setNotice('已读取当前工程，表单输入仍保留；使用“基于当前输入重新预览”核对差异。');}catch(error){setNotice((error as Error).message);}}
  function rebase(){setMergeChoices({});setMergeOpen(true);}
  function acceptMerge(){if(merged.conflicts.length)return;setBase(latest);setDraft(merged.value as BusinessSnapshot);setParameterCommands(merged.parameters);setProposal(null);setFailure(null);setMergeOpen(false);setDirty(true);setNotice('已按所选字段形成新预览基础；请检查变更，再明确应用。');}
  function discard(){setBase(latest);setDraft(latest);setDirty(false);setParameterCommands({});setProposal(null);setFailure(null);setIssues([]);setNotice('');setMergeOpen(false);setClosing(false);}
  function close(){if(busy)return;if(dirty){setClosing(true);return;}props.onClose?.();}
  async function runSaved(){
    const saved=props.scenarios.find(s=>s.id===activeScenario?.id);if(!saved||dirty||busy)return;setPending(true);
    try{await props.onRun({project_id:props.project.id,base_project_revision:props.project.revision,scenario_id:saved.id,base_scenario_revision:saved.revision});setNotice('已生成新的运行记录，旧记录保留。');}catch(error){setNotice((error as Error).message);}finally{setPending(false);}
  }
  const recordLabel=(value:unknown,index:number)=>{
    if(spec?.id==='orders'){const order=value as ScenarioInput['orders'][number];const product=draft.project.products.find(p=>p.id===order.product_ref.id&&p.revision===order.product_ref.revision);return `订单 ${index+1} · ${product?factoryItemTypeTitle(draft.project,product.item_type_ref):'产品版本未找到'} · ${order.quantity?decimalText(order.quantity.value)+factoryUnitLabel(order.quantity.unit):'数量未填写'}`;}
    return labelForObject(value);
  };
  const visible=values.map((value,index)=>({value,index})).filter(v=>[recordLabel(v.value,v.index),labelForObject(v.value),v.value&&typeof v.value==='object'&&'id'in v.value?String(v.value.id):''].some(s=>s.toLowerCase().includes(query.toLowerCase()))).slice(0,100);
  const baseValues=spec?getPath(base,spec.owner==='project'?['project',...spec.path]:['scenarios',Math.max(0,base.scenarios.findIndex(s=>s.id===activeScenario?.id)),...spec.path]) as {id?:string}[]:[];
  const immutable=!!record&&typeof record==='object'&&'id' in record&&baseValues?.some(v=>v.id===(record as {id:string}).id);
  const scenarioShape=catalog?resolve(fieldSchema('ScenarioInput',catalog),catalog):null;
  const scenarioFields=['id','title','time_reference','window','scope','acceptance_goal'];
  const generalSchema:Schema={type:'object',properties:Object.fromEntries(scenarioFields.map(key=>[key,scenarioShape?.properties?.[key]??{}])),required:scenarioFields.filter(key=>scenarioShape?.required?.includes(key))};
  return <section className="sf-business" aria-label="业务输入" onKeyDown={event=>{
    if(busy||event.nativeEvent.isComposing)return;
    const text=event.target instanceof HTMLInputElement||event.target instanceof HTMLTextAreaElement||(event.target instanceof HTMLElement&&event.target.isContentEditable);
    if(event.key==='Escape'){event.stopPropagation();if(proposal)setProposal(null);else if(mergeOpen)setMergeOpen(false);else close();}
    if((event.ctrlKey||event.metaKey)&&event.key==='Enter'){event.preventDefault();if(proposal)void submit(proposal);else checkProposal();}
    if(!text&&(event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='z'){event.preventDefault();historyAction(event.shiftKey?'redo':'undo');}
    if(!text&&(event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='y'){event.preventDefault();historyAction('redo');}
  }}>
    <header><strong>业务输入与方案</strong><span>{props.project.title} · {dirty?'有待应用变更':'输入已同步'}</span><button type="button" disabled={busy} onClick={close}>返回画布</button></header>
    <div className="sf-business-toolbar"><label>当前方案<select aria-label="业务当前方案" disabled={busy} value={scenarioIndex>=0?scenarioIndex:''} onChange={event=>{const i=Number(event.target.value);setSelectedScenarioId(draft.scenarios[i].id);if(props.scenarios.some(s=>s.id===draft.scenarios[i]?.id))props.onScenarioChange(draft.scenarios[i].id);setQuery('');}}><option value="" disabled>{draft.scenarios.length?'未找到所选方案':'尚无方案'}</option>{draft.scenarios.map((s,i)=><option key={s.id} value={i}>{scenarioName(s,i)}</option>)}</select></label><button type="button" onClick={()=>newScenario() } disabled={!catalog||busy}>新建空白方案</button><button type="button" onClick={()=>newScenario(true)} disabled={!activeScenario||busy}>复制为新方案</button><button type="button" disabled={busy||dirty||!availability?.can_undo} onClick={()=>historyAction('undo')}>撤销草稿</button><button type="button" disabled={busy||dirty||!availability?.can_redo} onClick={()=>historyAction('redo')}>重做草稿</button><button type="button" disabled={busy} onClick={()=>void refresh()}>刷新当前数据</button><button type="button" disabled={busy||dirty||!props.scenarios.some(s=>s.id===activeScenario?.id)} onClick={()=>void runSaved()}>运行当前方案</button></div>
    {(stale||failure?.code==='RevisionConflict')&&<div role="alert" className="sf-business-warning">当前工程已变化；保留表单，不覆盖新状态。<button onClick={()=>void refresh()}>读取当前工程</button><button onClick={rebase}>基于当前输入重新预览</button></div>}
    {availability?.boundary_reason&&<div className="sf-business-note">{historyBoundaryLabel(availability.boundary_reason)}<details><summary>历史详情</summary><p>{availability.boundary_reason}</p></details></div>}{!availability&&<p className="sf-business-note">尚未读取到当前编辑历史。输入仍可编辑，应用变更后可查看撤销和重做状态。</p>}
    {schemaError&&<p role="alert">{schemaError}</p>}
    <fieldset className="sf-business-body" disabled={!!busy}><nav aria-label="业务面板分类">{[{id:'scenario',title:'方案与观察窗'},...sections,{id:'stages',title:'设备阶段与参数'},{id:'actual',title:'实际接受与等待'}].map(s=><button key={s.id} aria-pressed={section===s.id} onClick={()=>{setSection(s.id);setQuery('');}}>{s.title}</button>)}</nav>
      <main className="sf-business-content">
        {!catalog&&!schemaError&&<p>正在读取当前公共字段规范…</p>}
        {catalog&&section==='scenario'&&activeScenario&&<><h2>方案与观察窗</h2><p>方案 r{activeScenario.revision} · 共同时间基准 {draft.project.time_reference.label}；不会修改旧结果。</p><SchemaField schema={generalSchema} catalog={catalog} value={Object.fromEntries(scenarioFields.filter(key=>Object.hasOwn(activeScenario,key)).map(key=>[key,(activeScenario as unknown as Record<string,unknown>)[key]]))} snapshot={draft} immutableId={base.scenarios.some(s=>s.id===activeScenario.id)} label="方案输入" onChange={value=>change(setPath(draft,['scenarios',scenarioIndex],{...activeScenario,...value as object}))}/><SchemaField schema={resolve(fieldSchema('InitialConditions',catalog),catalog).properties!.initial_setup_families} catalog={catalog} value={activeScenario.initial_conditions.initial_setup_families} snapshot={draft} label="明确初始家族" path="/initial_setup_families" onChange={value=>change(setPath(draft,['scenarios',scenarioIndex,'initial_conditions','initial_setup_families'],value))}/></>}
        {catalog&&spec&&<><h2>{spec.title}</h2><p>{spec.note}</p>{spec.id==='setup'&&!props.canEditSetupPolicies&&<p role="status">当前环境只能查看换型规则，尚未提供修改入口。</p>}{spec.owner==='scenario'&&!activeScenario?<p>先新建或选择方案。</p>:<><div className="sf-business-row-tools"><input aria-label="查找业务记录" placeholder="按身份/名称查找" value={query} onChange={e=>setQuery(e.target.value)}/><select aria-label="业务记录" value={values.length&&index>=0?index:''} onChange={e=>setRowIndex({...rowIndex,[section]:Number(e.target.value)})}><option value="" disabled>{index<0?'未找到指定记录':'尚无记录'}</option>{visible.map(v=><option key={v.index} value={v.index}>{recordLabel(v.value,v.index)}</option>)}{values.length&&index>=0&&!visible.some(v=>v.index===index)?<option value={index}>{recordLabel(record,index)}</option>:null}</select><button disabled={busy||sectionReadOnly} onClick={()=>newRow()}>新增记录</button><button disabled={record===undefined||busy||sectionReadOnly} onClick={()=>newRow(true)}>复制记录</button><button disabled={record===undefined||busy||sectionReadOnly} onClick={()=>change(setPath(draft,path,values.filter((_,i)=>i!==index)))}>移除记录</button>{sectionReadOnly&&values.length>0&&<button disabled={busy} onClick={()=>change(setPath(draft,path,[]))}>清除不受支持的方案覆盖</button>}<span>共{values.length}行；完整ID可查找</span></div>{index<0&&<p role="status">当前分区未找到指定记录，请选择已有记录。</p>}{record!==undefined&&<SchemaField key={`${section}:${index}`} schema={fieldSchema(spec.type,catalog)} readOnly={sectionReadOnly} catalog={catalog} value={record} onChange={editRecord} snapshot={draft} immutableId={immutable} label={spec.title} hidden={spec.id==='device_bindings'?['configuration','pose','footprint_m','height_m','presentation_override','module_ref','hall_id','line_id']:undefined} path={`/${spec.id}/${index}`}/>}</>}</>}
        {catalog&&section==='stages'&&<DeclaredParameters selectedDeviceId={props.editIntent?.section==='stages'?props.editIntent.object_id:undefined} snapshot={draft} modules={props.modules} catalog={catalog} commands={parameterCommands} onChange={value=>{setParameterCommands(value);setDirty(true);setProposal(null);}} onRemove={(deviceId,parameterId)=>{const next={...parameterCommands};delete next[JSON.stringify([deviceId,parameterId])];setParameterCommands(next);change(removeInstanceOverride(draft,deviceId,parameterId));}} onLocate={props.onLocate}/>}
        {section==='actual'&&<BusinessRuntime {...props}/>}
        {catalog&&section==='scenario'&&!activeScenario&&<p>{draft.scenarios.length?'未找到所选方案，请选择当前工程中的方案。':'尚无方案。新建空白方案后明确观察窗，或从已有方案复制。'}</p>}
      </main>
    </fieldset>
    {issues.length>0&&<div role="alert" className="sf-business-errors">{issues.map((issue,i)=><p key={i}>{issue.path.split('/').map(s=>labels[s]??s).join(' / ')}：{issue.message}</p>)}</div>}
    {failure&&<div role="alert" className="sf-business-errors"><strong>{failure.code}</strong>：{failure.message}{failure.field_errors.map((f,i)=><p key={i}>{f.pointer}：{f.reason}</p>)}{failure.object_refs.map(o=><button key={`${o.kind}:${o.id}`} onClick={()=>props.onLocate({object:o,source:'draft'})}>定位 {o.id}</button>)}</div>}
    {notice&&<p role="status" className="sf-business-note">{notice}</p>}
    <footer><button disabled={busy||!dirty||!catalog} onClick={checkProposal}>检查变更</button><button disabled={busy||!dirty} onClick={discard}>放弃本地表单变更</button><span>Ctrl+Enter 检查/应用 · 文本框保留原生文字撤销</span></footer>
    {proposal&&<div className="sf-business-dialog" role="dialog" aria-label="业务变更预览"><h2>应用到同一工程草稿</h2><p>工程 r{proposal.base_project_revision} · {proposal.commands.length}条共同命令 · 不重跑或回滚当前Run</p><ol>{previewCommands(base,draft,proposal.commands).map((group,i)=><li key={i}><strong>{group.title}</strong>{group.changes.length>0?<ul>{group.changes.map((change,j)=><li key={j}>{change.field}：{change.before} → {change.after}</li>)}</ul>:<p>命令无可比较字段；请返回表单核对。</p>}</li>)}</ol><button disabled={busy} onClick={()=>void submit(proposal)}>确认应用草稿</button><button disabled={busy} onClick={()=>setProposal(null)}>返回继续编辑</button></div>}
    {mergeOpen&&<div className="sf-business-dialog" role="dialog" aria-label="修订冲突字段核对"><h2>按当前修订核对字段</h2><p>未同时修改的字段自动保留；下列冲突须明确选择。</p>{merged.conflicts.map((c:MergeConflict)=><fieldset key={c.path}><legend>{c.path}</legend><p>你的输入：{labelForObject(c.mine)}</p><p>当前工程：{labelForObject(c.current)}</p><details><summary>查看完整冲突值</summary><pre>{JSON.stringify({mine:c.mine,current:c.current},null,2)}</pre></details><button onClick={()=>setMergeChoices({...mergeChoices,[c.path]:'mine'})}>采用我的此字段</button><button onClick={()=>setMergeChoices({...mergeChoices,[c.path]:'current'})}>保留当前此字段</button></fieldset>)}<button disabled={!!merged.conflicts.length} onClick={acceptMerge}>接受核对并重新检查</button><button onClick={()=>setMergeOpen(false)}>保留原表单返回</button></div>}
    {closing&&<div className="sf-business-dialog" role="dialog" aria-label="保留未应用表单"><h2>还有未应用的业务输入</h2><button onClick={()=>setClosing(false)}>返回保留表单</button><button onClick={()=>{discard();props.onClose?.();}}>放弃这些本地输入并关闭</button></div>}
  </section>;
}
