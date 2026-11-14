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
  if (!project) return <p className="authoring-warning">请先打开工程，再选择组合对象。</p>;
  const resources = compositionResources(value, project, sourceNow ?? null, sourceScenario);
  return <section aria-label="组合模板创作"><h3>从当前工程选择组合</h3><p>基于工程第{baseRevision}版选择对象。应用模板时将设置新的编号、资源和端口。</p>{project.revision !== baseRevision && <p className="authoring-warning">工程版本已变化。保存遇到冲突时会保留输入，请核对后处理。</p>}{value.seed && <p className="authoring-status">已附带方案输入：{value.seed.initial_conditions.items.length} 项初始输入、{value.seed.arrivals.length} 项到货、{value.seed.orders.length} 项订单。<button type="button" disabled={disabled} onClick={clear}>清除方案输入及来源</button></p>}
    <SchemaField schema={formSchemas.TemplateScope} value={value.layout_scope} onChange={layout_scope => onChange({ ...value, layout_scope: layout_scope as TemplateSubset['layout_scope'] })} name="layout_scope" disabled={disabled} />
    <div className="authoring-field-grid">{compositionGroups.map(({ key, title }) => <fieldset className="authoring-group" key={key}><legend>{title}</legend><button type="button" disabled={disabled || project[key].length === 0 || project[key].every(item => value[key].some(candidate => compositionObjectId(candidate) === compositionObjectId(item)))} onClick={() => onChange(selectCompositionObjects(value, project, key, project[key].map(compositionObjectId)))}>选择全部{title}</button>{project[key].map(item => { const id = compositionObjectId(item); const selected = value[key].some(candidate => compositionObjectId(candidate) === id); return <label className="authoring-check" key={id}><input type="checkbox" disabled={disabled} checked={selected} onChange={event => onChange(selectCompositionObjects(value, project, key, event.target.checked ? [...value[key].map(compositionObjectId), id] : value[key].map(compositionObjectId).filter(candidate => candidate !== id)))} />{'title' in item ? item.title : id} · {id}</label>; })}{project[key].length === 0 && <p>当前工程无此对象。</p>}</fieldset>)}</div>
    <details><summary>模块与版本 · {value.module_refs.length} 项 · 查看与编辑</summary><SchemaField schema={{ type: 'array', items: formSchemas.ModuleRef }} value={value.module_refs} onChange={module_refs => onChange({ ...value, module_refs: module_refs as TemplateSubset['module_refs'] })} name="module_refs" disabled={disabled} /></details>
    <fieldset className="authoring-group"><legend>附带所选对象的资源配置</legend><p>所选组合及已捕获种子引用的实际资源：{resources.map(row => `${row.source.title} · ${row.source.id}`).join('；') || '无'}。</p><button type="button" disabled={disabled || resources.length === 0} onClick={() => onChange({ ...value, resource_requirements: resources.map(row => row.requirement) })}>更新资源需求（{resources.length} 项）</button><p className="authoring-help">附带方案输入后可重建资源需求，包含资源配置，不复制编号、位置、当前占用或运行结果。应用时再选择新建或复用资源。</p></fieldset>
    <details><summary>资源需求正文 · {value.resource_requirements.length} 项 · 查看与编辑</summary><SchemaField schema={{ type: 'array', items: formSchemas.ResourceDefinition }} value={value.resource_requirements} onChange={resource_requirements => onChange({ ...value, resource_requirements: resource_requirements as TemplateSubset['resource_requirements'] })} name="resource_requirements" disabled={disabled} /></details>
    <SchemaField schema={{ type: 'array', items: formSchemas.RoleQuantity }} value={value.role_requirements} onChange={role_requirements => onChange({ ...value, role_requirements: role_requirements as TemplateSubset['role_requirements'] })} name="role_requirements" disabled={disabled} />
    <fieldset className="authoring-group"><legend>方案输入与来源</legend>
      {sourceScenario && <p className="authoring-status">已附带的来源：{sourceScenario.scenario_id} · r{sourceScenario.revision}。保存时保留此来源版本。</p>}
      {value.seed && !sourceScenario && <p className="authoring-warning">缺少方案输入的来源。保存前请重新选择并附带来源方案。</p>}
      {sourceScenario && (!sourceNow || sourceNow.revision !== sourceScenario.revision) && <p className="authoring-warning">{sourceNow ? `当前返回的来源方案为 r${sourceNow.revision}` : '当前上下文未返回这一来源方案'}；仍保留已捕获的 {sourceScenario.scenario_id} r{sourceScenario.revision} 和种子正文，提交时将检查版本冲突。</p>}
      <label>来源方案<select value={scenarioId} disabled={disabled} onChange={event => setScenarioId(event.target.value)}><option value="">请选择来源方案</option>{workspace.scenarios.map(scenario => <option key={scenario.id} value={scenario.id}>{scenario.title} · {scenario.id} r{scenario.revision}</option>)}</select></label>
      <button type="button" disabled={disabled || !selected} onClick={attach}>附带所选方案输入</button>
      <p className="authoring-help">附带所选方案的初始条件、到料和订单，不包含运行结束时的库存或结果。仅选择方案还不会附带输入。</p>
      {value.seed && <details><summary>查看已附带输入（只读）</summary><SchemaField schema={formSchemas.TemplateSeed} value={value.seed} name="已附带输入" disabled onChange={() => undefined} /></details>}
    </fieldset>
    {issues !== undefined && <section aria-label="最近一次模板检查"><h4>最近一次模板检查</h4>{issues.map((issue, index) => <p key={index} className="authoring-warning"><strong>{issue.code}{issue.field_path ? ` · ${issue.field_path}` : ''}</strong>{issue.message}</p>)}{issues.length === 0 && <p>预览未发现问题，应用后仍需运行检查。</p>}</section>}
    <p>组合共 {value.halls.length} 个厂房、{value.devices.length} 个设备、{value.connections.length} 条连接；保存后请预览应用，核对新编号和资源。</p>
  </section>;
}
