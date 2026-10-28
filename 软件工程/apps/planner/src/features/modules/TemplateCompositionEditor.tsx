import { useState } from 'react';
import { SchemaField } from './SchemaField';
import { formSchemas } from './formSchema';
import { captureCompleteSeed, clearCompositionSeed, compositionGroups, compositionObjectId, compositionResources, selectCompositionObjects } from './templateComposition';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { TemplateSubset } from '../../../../../packages/commands/generated/typescript/TemplateSubset';
import type { ScenarioReference } from '../../../../../packages/commands/generated/typescript/ScenarioReference';
import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';

export function TemplateCompositionEditor({ workspace, value, sourceScenario = null, issues, onChange, disabled, baseRevision }: { workspace: WorkspaceSnapshot; value: TemplateSubset; sourceScenario?: ScenarioReference | null; issues?: Issue[]; onChange(value: TemplateSubset, source?: ScenarioReference | null): void; disabled: boolean; baseRevision: number }) {
  const [scenarioId, setScenarioId] = useState('');
  const project = workspace.project;
  const selected = workspace.scenarios.find(scenario => scenario.id === scenarioId);
  const sourceNow = sourceScenario && workspace.scenarios.find(scenario => scenario.id === sourceScenario.scenario_id);
  function attach() { if (selected) { const captured = captureCompleteSeed(value, selected); onChange(captured.subset, captured.source_scenario); } }
  function clear() { const cleared = clearCompositionSeed(value); onChange(cleared.subset, cleared.source_scenario); }
  if (!project) return <p className="authoring-warning">请先打开工程，再明确选择组合对象。</p>;
  const resources = compositionResources(value, project, sourceNow ?? null, sourceScenario);
  return <section aria-label="组合模板创作"><h3>从当前工程选择组合</h3><p>选择基于工程 r{baseRevision}。保留选定对象的声明和精确引用；采用时由模板消费者重新映射 ID、资源与端口。</p>{project.revision !== baseRevision && <p className="authoring-warning">工程修订已变化。保存会提交原修订，冲突时保留当前输入，不自动覆盖。</p>}{value.seed && <p className="authoring-status">组合中已保留独立声明种子：{value.seed.initial_conditions.items.length} 项初始输入、{value.seed.arrivals.length} 项到货、{value.seed.orders.length} 项订单。<button type="button" disabled={disabled} onClick={clear}>清除完整种子和来源</button></p>}
    <SchemaField schema={formSchemas.TemplateScope} value={value.layout_scope} onChange={layout_scope => onChange({ ...value, layout_scope: layout_scope as TemplateSubset['layout_scope'] })} name="layout_scope" disabled={disabled} />
    <div className="authoring-field-grid">{compositionGroups.map(({ key, title }) => <fieldset className="authoring-group" key={key}><legend>{title}</legend><button type="button" disabled={disabled || project[key].length === 0 || project[key].every(item => value[key].some(candidate => compositionObjectId(candidate) === compositionObjectId(item)))} onClick={() => onChange(selectCompositionObjects(value, project, key, project[key].map(compositionObjectId)))}>选择全部{title}</button>{project[key].map(item => { const id = compositionObjectId(item); const selected = value[key].some(candidate => compositionObjectId(candidate) === id); return <label className="authoring-check" key={id}><input type="checkbox" disabled={disabled} checked={selected} onChange={event => onChange(selectCompositionObjects(value, project, key, event.target.checked ? [...value[key].map(compositionObjectId), id] : value[key].map(compositionObjectId).filter(candidate => candidate !== id)))} />{'title' in item ? item.title : id} · {id}</label>; })}{project[key].length === 0 && <p>当前工程无此对象。</p>}</fieldset>)}</div>
    <details><summary>精确模块引用 · {value.module_refs.length} 项 · 查看与编辑</summary><SchemaField schema={{ type: 'array', items: formSchemas.ModuleRef }} value={value.module_refs} onChange={module_refs => onChange({ ...value, module_refs: module_refs as TemplateSubset['module_refs'] })} name="module_refs" disabled={disabled} /></details>
    <fieldset className="authoring-group"><legend>从实际对象附带资源声明</legend><p>所选组合及已捕获种子引用的实际资源：{resources.map(row => `${row.source.title} · ${row.source.id}`).join('；') || '无'}。</p><button type="button" disabled={disabled || resources.length === 0} onClick={() => onChange({ ...value, resource_requirements: resources.map(row => row.requirement) })}>按已选组合及声明种子重建资源需求（{resources.length} 项）</button><p className="authoring-help">明确附带种子后，同时包括精确来源方案的供料资源；仅选择方案候选不加入。明确替换下方资源需求，仅复制实际种类、角色、容量、资质、日历声明和表现；不复制实例标识、驻留位置、当前占用或 Run。采用时仍须分别选择新建或复用资源。</p></fieldset>
    <details><summary>资源需求正文 · {value.resource_requirements.length} 项 · 查看与编辑</summary><SchemaField schema={{ type: 'array', items: formSchemas.ResourceDefinition }} value={value.resource_requirements} onChange={resource_requirements => onChange({ ...value, resource_requirements: resource_requirements as TemplateSubset['resource_requirements'] })} name="resource_requirements" disabled={disabled} /></details>
    <SchemaField schema={{ type: 'array', items: formSchemas.RoleQuantity }} value={value.role_requirements} onChange={role_requirements => onChange({ ...value, role_requirements: role_requirements as TemplateSubset['role_requirements'] })} name="role_requirements" disabled={disabled} />
    <fieldset className="authoring-group"><legend>完整声明种子与精确来源</legend>
      {sourceScenario && <p className="authoring-status">保留的真实来源：{sourceScenario.scenario_id} · r{sourceScenario.revision}。保存提交这一引用，不按当前方案修订重写。</p>}
      {value.seed && !sourceScenario && <p className="authoring-warning">历史种子缺少实际来源引用。正文可查看，但保存前须明确重新附带真实方案；不会按正文猜测来源。</p>}
      {sourceScenario && (!sourceNow || sourceNow.revision !== sourceScenario.revision) && <p className="authoring-warning">{sourceNow ? `当前返回的来源方案为 r${sourceNow.revision}` : '当前上下文未返回这一来源方案'}；仍保留已捕获的 {sourceScenario.scenario_id} r{sourceScenario.revision} 和种子正文，由真实 Host 检查冲突。</p>}
      <label>实际来源方案候选<select value={scenarioId} disabled={disabled} onChange={event => setScenarioId(event.target.value)}><option value="">仅选择候选，不附带种子</option>{workspace.scenarios.map(scenario => <option key={scenario.id} value={scenario.id}>{scenario.title} · {scenario.id} r{scenario.revision}</option>)}</select></label>
      <button type="button" disabled={disabled || !selected} onClick={attach}>明确附带所选方案的完整声明种子</button>
      <p className="authoring-help">完整捕获该修订的初始条件、到货和订单，供采用时再分别选择供料与需求。仅选择候选不会产生种子或来源请求字段；不会取用 Run 末态、实际库存、占用或结果。</p>
      {value.seed && <details><summary>查看已捕获的声明正文（只读）</summary><SchemaField schema={formSchemas.TemplateSeed} value={value.seed} name="已捕获的声明正文" disabled onChange={() => undefined} /></details>}
    </fieldset>
    {issues !== undefined && <section aria-label="最近一次实际模板预览检查"><h4>最近一次实际模板预览检查</h4>{issues.map((issue, index) => <p key={index} className="authoring-warning"><strong>{issue.code}{issue.field_path ? ` · ${issue.field_path}` : ''}</strong>{issue.message}</p>)}{issues.length === 0 && <p>此次实际预览未返回检查项；不等于完整采用或运行通过。</p>}</section>}
    <p>组合共 {value.halls.length} 个厂房、{value.devices.length} 个设备、{value.connections.length} 条连接；保存后仍须在实际模板消费者中预览新 ID、复用资源和采用差异。</p>
  </section>;
}
