import { useEffect, useMemo, useState } from 'react';
import type { MaterialConnection } from '../../../../../packages/commands/generated/typescript/MaterialConnection';
import type { MaterialEndpoint } from '../../../../../packages/commands/generated/typescript/MaterialEndpoint';
import type { EditCommand, ModulePackage, ProjectState } from '../../services/projectHost';
import { moduleKey } from '../../services/projectHost';
import { WorkbenchModal } from './WorkbenchModal';

interface Props {
  project: ProjectState;
  modules: Map<string, ModulePackage>;
  value?: MaterialConnection;
  busy: boolean;
  onApply: (commands: EditCommand[]) => Promise<void>;
  onClose: () => void;
  onDirtyChange?: (dirty: boolean) => void;
}
interface EndpointChoice {
  endpoint: MaterialEndpoint;
  title: string;
  direction: 'input' | 'output' | 'bidirectional';
  typeId: string | null;
  unit: string | null;
  unbound: boolean;
}
const endpointKey = (endpoint: MaterialEndpoint) => endpoint.kind === 'location'
  ? JSON.stringify(['location', endpoint.data])
  : JSON.stringify(['port', endpoint.data.module_ref.id, endpoint.data.module_ref.version, endpoint.data.instance_id, endpoint.data.port_id]);

export function ConnectionEditor({ project, modules, value, busy, onApply, onClose, onDirtyChange }: Props) {
  const [source, setSource] = useState(value ? endpointKey(value.source) : '');
  const [destination, setDestination] = useState(value ? endpointKey(value.destination) : '');
  const [typeId, setTypeId] = useState(value?.item_type ?? '');
  const [storeId, setStoreId] = useState(value?.destination_store ?? '');
  const [error, setError] = useState('');
  const [leaving, setLeaving] = useState(false);
  const [initial] = useState(JSON.stringify({ source, destination, typeId, storeId }));
  const currentInput = JSON.stringify({ source, destination, typeId, storeId });
  const changed = !value || initial !== currentInput;
  useEffect(() => {
    onDirtyChange?.(initial !== currentInput);
    return () => onDirtyChange?.(false);
  }, [initial, currentInput, onDirtyChange]);
  const endpoints = useMemo(() => {
    const choices: EndpointChoice[] = project.locations.map(location => ({
      endpoint: { kind: 'location', data: location.id }, title: `${location.title} · 库位 ${location.id}`,
      direction: 'bidirectional', typeId: null, unit: location.capacity?.unit ?? null, unbound: false,
    }));
    for (const device of project.devices) {
      const payload = modules.get(moduleKey(device.module_ref))?.definition.payload;
      if (payload?.category !== 'devices') continue;
      for (const port of payload.data.ports) {
        if (port.payload.kind !== 'material') continue;
        const bound = device.role_bindings.item_types[port.payload.data.item_role];
        choices.push({ endpoint: { kind: 'port', data: { module_ref: device.module_ref, instance_id: device.id, port_id: port.id } },
          title: `${device.title} · ${device.id} / ${port.id} · ${device.module_ref.id}@${device.module_ref.version}${bound ? '' : ' · 物品角色未绑定'}`, direction: port.direction,
          typeId: bound ?? null, unit: port.payload.data.unit, unbound: !bound });
      }
    }
    return choices;
  }, [project, modules]);
  const itemType = project.item_types.find(item => item.id === typeId);
  const choices = (direction: 'input' | 'output') => endpoints.filter(choice => choice.direction === direction || choice.direction === 'bidirectional');
  const incompatible = (choice: EndpointChoice) => choice.unbound || !!itemType && (
    choice.typeId !== null && choice.typeId !== itemType.id || choice.unit !== null && choice.unit !== itemType.unit);
  const staleOriginal = value ? [{ label: '源', endpoint: value.source, selected: source }, { label: '目的', endpoint: value.destination, selected: destination }].filter(({ endpoint, selected }) => selected === endpointKey(endpoint) && !endpoints.some(choice => endpointKey(choice.endpoint) === selected)) : [];

  async function submit(event: React.FormEvent) {
    event.preventDefault(); setError(''); setLeaving(false);
    const from = endpoints.find(choice => endpointKey(choice.endpoint) === source);
    const to = endpoints.find(choice => endpointKey(choice.endpoint) === destination);
    try {
      if (!itemType || !from || !to || !storeId) throw new Error('请选择实际物品、源端点、目的端点和接收库位。');
      if (source === destination) throw new Error('源端点与目的端点不能相同。');
      if (!choices('output').includes(from) || !choices('input').includes(to) || incompatible(from) || incompatible(to)) throw new Error('端口方向、物品绑定或单位不相容，请先修正设备绑定。');
      const store = project.locations.find(location => location.id === storeId);
      if (!store || !store.accepted_item_types.includes(itemType.id) || (store.capacity && store.capacity.unit !== itemType.unit)) throw new Error('接收库位须接受该实际物品和单位。');
      if (to.endpoint.kind === 'location' && to.endpoint.data !== store.id) throw new Error('目的端点为库位时，接收库位必须是同一实际库位。');
      await onApply([{ kind: 'put_object', data: { kind: 'connection', data: { kind: 'material', data: {
        ...(value ?? { id: `connection-${crypto.randomUUID()}`, quantity_rule: null, capacity_rule: null }),
        source: from.endpoint, destination: to.endpoint, item_type: itemType.id, unit: itemType.unit, destination_store: store.id,
      } } } }]);
    } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }

  function requestClose() {
    if (busy) return;
    if (changed) setLeaving(true);
    else onClose();
  }

  return <form className="object-editor" onSubmit={submit} onKeyDown={event => {
    if (event.key !== 'Escape' || event.defaultPrevented || event.nativeEvent.isComposing || leaving) return;
    event.preventDefault();
    requestClose();
  }}>
    <header><div><small>工程草稿 · r{project.revision}</small><h2>{value ? '编辑' : '新建'}物料连接</h2></div><button type="button" autoFocus disabled={busy} onClick={requestClose}>返回画布</button></header>
    <p className="form-hint">端口取设备的精确版本定义。连接表达物料交接；生产工艺、资源及运输时间仍使用方案中的实际声明。</p>
    {staleOriginal.map(({ label, endpoint }) => <p className="error-message" role="alert" key={label}>原{label}端点 {endpoint.kind === 'location' ? endpoint.data : `${endpoint.data.instance_id} / ${endpoint.data.port_id} · ${endpoint.data.module_ref.id}@${endpoint.data.module_ref.version}`} 已不在当前精确端点中。请明确重新选择；不会按名称自动接线。</p>)}
    <fieldset disabled={busy}>
      <label>物品类型<select value={typeId} onChange={event => setTypeId(event.target.value)} required><option value="">请选择</option>{project.item_types.map(item => <option key={item.id} value={item.id}>{item.title} · {item.id} · {item.unit}</option>)}</select></label>
      <label>源端点 · 输出<select value={source} onChange={event => setSource(event.target.value)} required><option value="">请选择</option>{choices('output').map(choice => <option key={endpointKey(choice.endpoint)} value={endpointKey(choice.endpoint)} disabled={incompatible(choice)}>{choice.title} · {choice.unit ?? '单位未声明'}</option>)}</select></label>
      <label>目的端点 · 输入<select value={destination} onChange={event => setDestination(event.target.value)} required><option value="">请选择</option>{choices('input').map(choice => <option key={endpointKey(choice.endpoint)} value={endpointKey(choice.endpoint)} disabled={incompatible(choice)}>{choice.title} · {choice.unit ?? '单位未声明'}</option>)}</select></label>
      <label>接收库位<select value={storeId} onChange={event => setStoreId(event.target.value)} required><option value="">请选择</option>{project.locations.map(location => <option key={location.id} value={location.id}>{location.title} · {location.id}</option>)}</select></label>
      <p className="muted">{itemType ? `共同单位：${itemType.unit}。` : '先选择实际物品。'}数量规则与容量沿用原声明；新连接尚未声明，不以零或无限容量补齐。</p>
    </fieldset>
    {error && <p className="error-message" role="alert">{error}</p>}
    <footer><button type="button" disabled={busy || !changed} onClick={onClose}>放弃更改</button><button className="primary" type="submit" disabled={busy || !changed}>应用连接</button></footer>
    {leaving && <WorkbenchModal labelId="unapplied-connection-title" busy={busy} onDismiss={() => setLeaving(false)}><section className="modal"><h2 id="unapplied-connection-title">尚有未应用连接</h2><footer><button type="button" autoFocus disabled={busy} onClick={() => setLeaving(false)}>留在当前</button><button type="button" disabled={busy} onClick={onClose}>丢弃并返回</button><button type="submit" className="primary" disabled={busy}>应用后返回</button></footer></section></WorkbenchModal>}
  </form>;
}
