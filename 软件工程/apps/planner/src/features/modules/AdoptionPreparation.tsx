import { useState } from 'react';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleAdoption } from '../../../../../packages/commands/generated/typescript/ModuleAdoption';
import type { EditableProjectObject } from '../../../../../packages/commands/generated/typescript/EditableProjectObject';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { PortRef } from '../../../../../packages/commands/generated/typescript/PortRef';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import { SchemaField } from './SchemaField';
import { formSchemas } from './formSchema';
import { brief, copy, differences, initialValue, refKey } from './model';
import { fieldLabel, optionLabel } from './labels';
import { identityPose } from '../device-authoring/parts';
import { AuthoringFailure } from './DefinitionSampleResult';

export function DeviceConnectionImpact({ project, reference, targets }: { project: ProjectState; reference: ModuleRef; targets: ObjectRef[] }) {
  const affected: { id: string; side: string; port: PortRef }[] = [];
  for (const connection of project.connections) for (const side of ['source', 'destination'] as const) {
    let port: PortRef | null;
    if (connection.kind === 'material') { const endpoint = connection.data[side]; port = endpoint.kind === 'port' ? endpoint.data : null; }
    else port = connection.data[side];
    if (port && targets.some(target => target.kind === 'device' && target.id === port.instance_id) && refKey(port.module_ref) !== refKey(reference)) affected.push({ id: connection.data.id, side: side === 'source' ? '源端口' : '目的端口', port });
  }
  return affected.length ? <section className="authoring-warning" aria-label="采用后需人工重接的连接"><h3>需要人工核对的连接 · {affected.length}</h3><p>所选设备将使用 {reference.id}@{reference.version}。以下连接仍指向其他精确版本，采用不会自动按名称重接。</p>{affected.map(({ id, side, port }) => <p key={`${id}/${side}`}>{id} · {side}：{port.instance_id} / {port.port_id} · {port.module_ref.id}@{port.module_ref.version}</p>)}<p>采用后，在对象树核对对应连接；物料连接进入“编辑物料连接”明确选择新端点并应用。连接条件完整后才能运行。</p></section> : null;
}

export function roleRequirements(module: ModulePackage, modules?: ReadonlyMap<string, ModulePackage>) {
  const item = new Set<string>(), resource = new Set<string>(), target = new Set<string>(), port = new Set<string>();
  function visit(value: unknown) {
    if (Array.isArray(value)) { value.forEach(visit); return; }
    if (!value || typeof value !== 'object') return;
    for (const [key, child] of Object.entries(value)) {
      if (['item_role', 'output_role'].includes(key) && typeof child === 'string') item.add(child);
      if (['item_roles', 'content_roles', 'container_material_roles'].includes(key) && Array.isArray(child)) child.filter((role): role is string => typeof role === 'string').forEach(role => item.add(role));
      if (['target_role', 'pending_location_role', 'isolation_location_role'].includes(key) && typeof child === 'string') target.add(child);
      if (key === 'resource_roles' || key === 'resources' || key.endsWith('_resources')) if (Array.isArray(child)) child.forEach(row => { if (row && typeof row.role === 'string') resource.add(row.role); });
      if (key === 'ports' && Array.isArray(child)) child.forEach(row => { if (row && typeof row.id === 'string') port.add(row.id); });
      if (key === 'port_anchors' && child && typeof child === 'object') Object.keys(child).forEach(role => port.add(role));
      visit(child);
    }
  }
  visit(module.definition.payload.data);
  if (module.definition.payload.category === 'devices') {
    const behavior = module.definition.payload.data.behavior;
    if (behavior.selected === 'operation_ref' && behavior.operation_ref) {
      const operation = modules?.get(refKey(behavior.operation_ref));
      if (operation?.definition.payload.category === 'operations') visit(operation.definition.payload.data);
    }
    resource.add('execution_capacity');
  }
  if (module.definition.payload.category === 'resources' && module.definition.payload.data.role) resource.add(module.definition.payload.data.role);
  return { item: [...item], resource: [...resource], target: [...target], port: [...port] };
}

interface Props { workspace: WorkspaceSnapshot; module: ModulePackage; busy: boolean; onCancel(): void; onApply(adoption: ModuleAdoption, base: ProjectState): Promise<void> }
type NewModuleInstance = Extract<EditableProjectObject, { kind: 'device' | 'resource' | 'item_type' }>;
export function AdoptionPreparation({ workspace, module, busy, onCancel, onApply }: Props) {
  const [base] = useState(workspace.project!);
  const reference = { id: module.definition.id, version: module.definition.version };
  const [targets, setTargets] = useState<ObjectRef[]>([]);
  const [bindings, setBindings] = useState<ModuleAdoption['bindings']>({ item_types: {}, resources: {}, targets: {}, ports: {} });
  const [instances, setInstances] = useState<NewModuleInstance[]>([]);
  const [seedId, setSeedId] = useState(''); const [scenarioId, setScenarioId] = useState('');
  const [copySupply, setCopySupply] = useState(false); const [copyOrders, setCopyOrders] = useState(false);
  const [confirming, setConfirming] = useState(false); const [error, setError] = useState<unknown>(null);
  const category = module.definition.payload.category; const roles = roleRequirements(module, workspace.modules);
  const targetChoices: { ref: ObjectRef; title: string; previous?: { id: string; version: string }; previousPresentation?: Presentation | null }[] = category === 'items' ? base.item_types.map(item => ({ ref: { kind: 'item_type', id: item.id }, title: item.title })) : category === 'resources' ? base.resources.map(item => ({ ref: { kind: 'resource', id: item.id }, title: item.title, previous: item.definition_ref })) : ['devices', 'operations', 'media'].includes(category) ? base.devices.map(item => ({ ref: { kind: 'device', id: item.id }, title: item.title, previous: category === 'operations' ? item.operation_binding ?? item.module_ref : item.module_ref, previousPresentation: item.presentation_override })) : [];
  const newKind = category === 'devices' ? 'device' : category === 'resources' ? 'resource' : category === 'items' ? 'item_type' : null;
  const typeName = newKind === 'device' ? 'DeviceInstance' : newKind === 'resource' ? 'ResourceInstance' : 'ItemType';
  const itemChoices = [...base.item_types.map(item => ({ id: item.id, title: item.title })), ...instances.filter((item): item is Extract<EditableProjectObject, { kind: 'item_type' }> => item.kind === 'item_type').map(item => ({ id: item.data.id, title: `${item.data.title}（待新建）` }))].filter(item => item.id.trim());
  const resourceChoices = [...base.resources.map(item => ({ ...item })), ...instances.filter((item): item is Extract<EditableProjectObject, { kind: 'resource' }> => item.kind === 'resource').map(item => ({ ...item.data, title: `${item.data.title}（待新建）` }))].filter(item => item.id.trim());
  const objectChoices: { ref: ObjectRef; title: string }[] = [...base.devices.map(item => ({ ref: { kind: 'device' as const, id: item.id }, title: item.title })), ...base.locations.map(item => ({ ref: { kind: 'location' as const, id: item.id }, title: item.title })), ...base.halls.map(item => ({ ref: { kind: 'hall' as const, id: item.id }, title: item.title }))];
  for (const instance of instances) if (instance.kind === 'device' && instance.data.id.trim()) objectChoices.push({ ref: { kind: 'device', id: instance.data.id }, title: `${instance.data.title}（待新建）` });
  const ports: { ref: PortRef; title: string }[] = [];
  for (const device of [...base.devices, ...instances.filter((item): item is Extract<EditableProjectObject, { kind: 'device' }> => item.kind === 'device').map(item => item.data)]) {
    const effectiveReference = category === 'devices' && targets.some(target => target.kind === 'device' && target.id === device.id) ? reference : device.module_ref;
    const definition = refKey(effectiveReference) === refKey(reference) ? module.definition : workspace.modules.get(refKey(effectiveReference))?.definition;
    if (definition?.payload.category !== 'devices' || !device.id.trim()) continue;
    for (const entry of definition.payload.data.ports) ports.push({ ref: { module_ref: effectiveReference, instance_id: device.id, port_id: entry.id }, title: `${device.title} · ${device.id} / ${entry.id} · ${effectiveReference.id}@${effectiveReference.version} · ${optionLabel(entry.direction)}` });
  }
  function addInstance() {
    if (!newKind) return;
    const data = initialValue(formSchemas[typeName]) as Record<string, unknown>;
    data.id = ''; data.title = ''; if (newKind === 'device') { data.module_ref = reference; data.pose = identityPose('meter'); data.role_bindings = copy(bindings); if (module.definition.payload.category === 'devices') data.configuration = copy(module.definition.payload.data.configuration); }
    if (newKind === 'resource') {
      data.definition_ref = reference;
      if (module.definition.payload.category === 'resources') {
        const resource = module.definition.payload.data;
        for (const key of ['kind', 'role', 'capacity', 'qualifications', 'presentation'] as const) if (resource[key] != null) data[key] = copy(resource[key]);
      }
    }
    if (newKind === 'item_type' && module.definition.payload.category === 'items') { const item = module.definition.payload.data; Object.assign(data, { revision: 'r1', unit: item.unit ?? '', tracking: item.tracking ?? 'individual', spec_ref: item.spec_ref ?? null, attributes: copy(item.attributes), attribute_schema: copy(item.attribute_schema) }); }
    setInstances(previous => [...previous, { kind: newKind, data } as NewModuleInstance]);
  }
  function explicitSeed(): ScenarioInput | null {
    if (!copySupply && !copyOrders) return null;
    const source = workspace.scenarios.find(item => item.id === scenarioId);
    if (!source || !seedId.trim()) throw new Error('单独选择实际方案输入并填写新方案标识；不会从运行结果复制库存。');
    const seed = copy(source); seed.id = seedId.trim(); seed.revision = 1; seed.project_revision = base.revision; seed.title = `${module.definition.title} · 明确采用的输入`;
    if (!copySupply) { seed.initial_conditions = { items: [], initial_setup_families: {}, declared_quality_decisions: [], material_allocations: [] }; seed.arrivals = []; seed.supplies = []; }
    if (!copyOrders) { seed.orders = []; seed.production_tasks = []; delete seed.acceptance_goal; }
    return seed;
  }
  const request = (): ModuleAdoption => ({ module_ref: reference, targets, bindings, new_instances: instances.map(instance => instance.kind === 'device' ? { ...instance, data: { ...instance.data, role_bindings: copy(bindings) } } : instance), explicit_seed: explicitSeed() });
  function confirm(form: HTMLFormElement) { if (!form.reportValidity()) return; try { if (Object.values(bindings.ports).some(binding => !ports.some(port => JSON.stringify(port.ref) === JSON.stringify(binding)))) throw new Error('端口映射与当前目标版本不一致，请重新明确选择实际端口。'); request(); setError(''); setConfirming(true); } catch (failure) { setError(failure); } }
  return <div className="authoring-dialog-backdrop"><section role="dialog" aria-modal="true" aria-labelledby="adoption-title" className="authoring-dialog authoring-adoption"><header><div><h2 id="adoption-title">核对采用与实例升级</h2><p>{module.definition.title} · {reference.id} @ {reference.version} · 工程 r{base.revision}</p></div><button type="button" disabled={busy} onClick={onCancel}>取消，保留原实例</button></header>{!!error && <section role="alert"><AuthoringFailure value={error} /></section>}{category === 'resources' && <p className="authoring-help">升级已有资源只更新精确定义引用，保留实例容量、角色、资格和日历。要采用新默认值，进入“人员与共享资源”明确修改并检查应用；新建资源时会带入已提供的默认值，日历仍须明确选择。</p>}<form onSubmit={event => { event.preventDefault(); confirm(event.currentTarget); }}><fieldset className="authoring-group"><legend>受影响的已有实例</legend>{targetChoices.map(choice => { const checked = targets.some(ref => ref.kind === choice.ref.kind && ref.id === choice.ref.id); return <div key={JSON.stringify(choice.ref)}><label className="authoring-check"><input type="checkbox" disabled={busy || confirming} checked={checked} onChange={event => setTargets(previous => event.target.checked ? [...previous, choice.ref] : previous.filter(ref => ref.kind !== choice.ref.kind || ref.id !== choice.ref.id))} />{choice.title} · {choice.ref.id}{choice.previous && ` · 原版本 ${choice.previous.id}@${choice.previous.version}`}</label>{checked && choice.previous && <VersionChanges before={workspace.modules.get(refKey(choice.previous))?.definition} after={module.definition} previousPresentation={category === 'media' ? choice.previousPresentation : undefined} />}</div>; })}{targetChoices.length === 0 && <p>此类先采用精确定义，再进入对应消费者明确准备实例、排期或专业绑定；本页不会虚构该类运行对象。</p>}</fieldset>{category === 'devices' && <DeviceConnectionImpact project={base} reference={reference} targets={targets} />}<fieldset className="authoring-group"><legend>明确映射 · 不按同名自动匹配</legend><div className="authoring-field-grid">{roles.item.map(role => <label key={`item-${role}`}>物品角色 {role}<select disabled={busy || confirming} value={bindings.item_types[role] ?? ''} onChange={event => { const next = { ...bindings.item_types }; if (event.target.value) next[role] = event.target.value; else delete next[role]; setBindings({ ...bindings, item_types: next }); }}><option value="">待选择</option>{itemChoices.map(item => <option key={item.id} value={item.id}>{item.title} · {item.id}</option>)}</select></label>)}{roles.resource.map(role => <label key={`resource-${role}`}>资源角色 {role}<select required={role === 'execution_capacity' && category === 'devices' && (targets.length > 0 || instances.some(item => item.kind === 'device'))} disabled={busy || confirming} value={bindings.resources[role] ?? ''} onChange={event => { const next = { ...bindings.resources }; if (event.target.value) next[role] = event.target.value; else delete next[role]; setBindings({ ...bindings, resources: next }); }}><option value="">待选择</option>{resourceChoices.filter(item => role !== 'execution_capacity' || (item.kind === 'fixture' && item.capacity.value.decimal_places === 0 && item.capacity.value.coefficient > 0 && base.calendars.some(calendar => calendar.id === item.calendar_id))).map(item => <option key={item.id} value={item.id}>{item.title} · {item.id}</option>)}</select></label>)}{roles.target.map(role => <label key={`target-${role}`}>对象角色 {role}<select disabled={busy || confirming} value={bindings.targets[role] ? JSON.stringify(bindings.targets[role]) : ''} onChange={event => { const next = { ...bindings.targets }; if (event.target.value) next[role] = JSON.parse(event.target.value); else delete next[role]; setBindings({ ...bindings, targets: next }); }}><option value="">待选择</option>{objectChoices.map(item => <option key={JSON.stringify(item.ref)} value={JSON.stringify(item.ref)}>{item.title} · {item.ref.id}</option>)}</select></label>)}{roles.port.map(role => <label key={`port-${role}`}>端口 {role}<select disabled={busy || confirming} value={bindings.ports[role] ? JSON.stringify(bindings.ports[role]) : ''} onChange={event => { const next = { ...bindings.ports }; if (event.target.value) next[role] = JSON.parse(event.target.value); else delete next[role]; setBindings({ ...bindings, ports: next }); }}><option value="">待选择实际端口</option>{ports.map(item => <option key={JSON.stringify(item.ref)} value={JSON.stringify(item.ref)}>{item.title}</option>)}</select></label>)}</div></fieldset>{newKind && <fieldset className="authoring-group"><legend>明确新建实例</legend>{instances.map((instance, index) => <div className="authoring-array-row" key={index}><SchemaField schema={formSchemas[typeName]} value={instance.data} name={newKind} omit={['module_ref','definition_ref','role_bindings','hall_id','calendar_id']} disabled={busy || confirming} onChange={data => setInstances(previous => previous.map((item, position) => position === index ? { ...item, data } as NewModuleInstance : item))} />{instance.kind === 'device' && <label>实际所属厂房<select required disabled={busy || confirming} value={instance.data.hall_id} onChange={event => setInstances(previous => previous.map((item, position) => position === index && item.kind === 'device' ? { ...item, data: { ...item.data, hall_id: event.target.value } } : item))}><option value="">请选择工程中的厂房</option>{base.halls.map(hall => <option key={hall.id} value={hall.id}>{hall.title} · {hall.id}</option>)}</select></label>}{instance.kind === 'resource' && <label>实际工作日历<select required disabled={busy || confirming} value={instance.data.calendar_id} onChange={event => setInstances(previous => previous.map((item, position) => position === index && item.kind === 'resource' ? { ...item, data: { ...item.data, calendar_id: event.target.value } } : item))}><option value="">请选择工程中的日历</option>{base.calendars.map(calendar => <option key={calendar.id} value={calendar.id}>{calendar.id}</option>)}</select></label>}<button type="button" disabled={busy || confirming} onClick={() => setInstances(previous => previous.filter((_, position) => position !== index))}>移除此待新建实例</button></div>)}<button type="button" disabled={busy || confirming} onClick={addInstance}>添加待新建{optionLabel(newKind)}</button><p className="authoring-help">填写新 ID 和真实厂房、日历、占地等输入。新实例的精确模块版本锁定为本次选择，设备角色使用上面的明确映射。</p></fieldset>}<fieldset className="authoring-group"><legend>种子供料与订单 · 分别确认</legend><label className="authoring-check"><input type="checkbox" disabled={busy || confirming} checked={copySupply} onChange={event => setCopySupply(event.target.checked)} />复制所选方案的初始输入、到货与供料</label><label className="authoring-check"><input type="checkbox" disabled={busy || confirming} checked={copyOrders} onChange={event => setCopyOrders(event.target.checked)} />复制所选方案的订单、接受目标与排期</label>{(copySupply || copyOrders) && <div className="authoring-field-grid"><label>实际来源方案<select required disabled={busy || confirming} value={scenarioId} onChange={event => setScenarioId(event.target.value)}><option value="">请选择输入，不使用 Run 结果</option>{workspace.scenarios.map(scenario => <option key={scenario.id} value={scenario.id}>{scenario.title} · {scenario.id} r{scenario.revision}</option>)}</select></label><label>新方案稳定标识<input required disabled={busy || confirming} value={seedId} onChange={event => setSeedId(event.target.value)} /></label></div>}</fieldset>{confirming ? <section className="authoring-confirmation"><h3>确认后提交以下变化</h3><dl><dt>精确来源</dt><dd>{reference.id} @ {reference.version}</dd><dt>已有实例</dt><dd>{targets.map(ref => ref.id).join('、') || '无；只纳入定义'}</dd><dt>新实例 ID</dt><dd>{instances.map(item => item.data.id).join('、') || '无'}</dd><dt>资源复用</dt><dd>{Object.entries(bindings.resources).map(([role, id]) => `${role} → ${id}`).join('；') || '无'}</dd><dt>种子供料</dt><dd>{copySupply ? `来源 ${scenarioId}，写入新方案 ${seedId}` : '不复制'}</dd><dt>订单与排期</dt><dd>{copyOrders ? `来源 ${scenarioId}，写入新方案 ${seedId}` : '不复制'}</dd></dl><p>取消不会提交采用命令。当前 Run、库存、人工认可和磁盘文件不会由此回退。</p><div className="authoring-actions"><button type="button" disabled={busy} onClick={() => setConfirming(false)}>返回修改映射</button><button type="button" className="authoring-primary" disabled={busy || workspace.project?.id !== base.id} onClick={async () => { setError(''); try { await onApply(request(), base); } catch (failure) { setError(failure); } }}>确认采用到 r{base.revision} 的工程</button></div></section> : <button type="submit" className="authoring-primary" disabled={busy}>核对全部变化</button>}</form></section></div>;
}

export function adoptionDifferences(before: ModulePackage['definition'] | undefined, after: ModulePackage['definition'], previousPresentation?: Presentation | null) {
  if (after.payload.category === 'media' && (before?.payload.category === 'devices' || previousPresentation != null)) return differences(previousPresentation ?? (before?.payload.category === 'devices' ? before.payload.data.presentation ?? null : null), after.payload.data);
  if (before?.payload.category === 'devices' && after.payload.category === 'operations') {
    const behavior = before.payload.data.behavior;
    return behavior.selected === 'inline' && behavior.inline ? differences(behavior.inline, after.payload.data) : null;
  }
  return before ? differences(before, after) : null;
}
export function VersionChanges({ before, after, previousPresentation }: { before?: ModulePackage['definition']; after: ModulePackage['definition']; previousPresentation?: Presentation | null }) {
  const changes = adoptionDifferences(before, after, previousPresentation);
  if (!changes) return <p className="authoring-help">原精确版本或实际采用内容未读到，无法把缺失当作无变化；请先补齐原模块。</p>;
  return <details className="authoring-version-diff"><summary>端口、参数、素材与载荷变化 · {changes.length} 处</summary><table><thead><tr><th>字段</th><th>原值</th><th>新值</th></tr></thead><tbody>{changes.map((change, index) => <tr key={index}><th>{change.path.split('/').filter(Boolean).map(fieldLabel).join(' / ')}</th><td>{optionLabel(brief(change.before))}</td><td>{optionLabel(brief(change.after))}</td></tr>)}</tbody></table></details>;
}
