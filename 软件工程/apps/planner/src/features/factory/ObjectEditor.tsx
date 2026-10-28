import { useEffect, useState } from 'react';
import { Vector3 } from 'three';
import type { DeviceInstance } from '../../../../../packages/commands/generated/typescript/DeviceInstance';
import type { HallDesign } from '../../../../../packages/commands/generated/typescript/HallDesign';
import type { LineGroup } from '../../../../../packages/commands/generated/typescript/LineGroup';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { EditableProjectObject } from '../../../../../packages/commands/generated/typescript/EditableProjectObject';
import type { EditCommand, ProjectState, ModulePackage } from '../../services/projectHost';
import { HostFailure, moduleKey } from '../../services/projectHost';
import { deviceHallPosition, devicePlacementCommands, identityPose, rotatePoseZ } from '../../scene/layout';

export type EditorTarget = { kind: 'hall'; value?: HallDesign } | { kind: 'device'; value?: DeviceInstance; moduleRef?: ModuleRef } | { kind: 'line'; value?: LineGroup };
interface Props {
  project: ProjectState;
  modules: Map<string, ModulePackage>;
  target: EditorTarget;
  busy: boolean;
  onApply: (commands: EditCommand[]) => Promise<void>;
  onReload: () => Promise<void>;
  onClose: () => void;
  onDirtyChange?: (dirty: boolean) => void;
}

export function ObjectEditor({ project, modules, target, busy, onApply, onReload, onClose, onDirtyChange }: Props) {
  const value = target.value;
  const devices = [...modules.values()].filter(module => module.definition.payload.category === 'devices');
  const [title, setTitle] = useState(value?.title ?? '');
  const [hallId, setHallId] = useState(target.kind === 'hall' ? '' : target.value?.hall_id ?? project.halls[0]?.id ?? '');
  const [module, setModule] = useState(target.kind === 'device' && (target.value?.module_ref ?? target.moduleRef) ? moduleKey((target.value?.module_ref ?? target.moduleRef)!) : '');
  const [lineId, setLineId] = useState(target.kind === 'device' ? target.value?.line_id ?? '' : '');
  const [itemBindings, setItemBindings] = useState(target.kind === 'device' ? { ...target.value?.role_bindings.item_types } : {});
  const [resourceBindings, setResourceBindings] = useState(target.kind === 'device' ? { ...target.value?.role_bindings.resources } : {});
  const deviceDefinition = devices.find(item => moduleKey({ id: item.definition.id, version: item.definition.version }) === module)?.definition;
  const devicePayload = deviceDefinition?.payload.category === 'devices' ? deviceDefinition.payload.data : null;
  const operationPayload = devicePayload?.behavior.operation_ref ? modules.get(moduleKey(devicePayload.behavior.operation_ref))?.definition.payload : null;
  const behavior = devicePayload?.behavior.selected === 'inline' ? devicePayload.behavior.inline :
    devicePayload?.behavior.selected === 'operation_ref' && operationPayload?.category === 'operations' ? operationPayload.data : null;
  const itemRoles = [...new Set([...(behavior?.item_roles ?? []), ...(devicePayload?.ports.flatMap(port => port.payload.kind === 'material' ? [port.payload.data.item_role] : []) ?? [])])];
  const resourceRoles = [...new Set([...(behavior?.resource_roles.map(requirement => requirement.role) ?? []), ...(behavior?.phases.flatMap(phase => phase.phase_resources.map(requirement => requirement.role)) ?? [])])];
  const position = target.kind === 'device' && target.value ? deviceHallPosition(project, target.value) :
    target.kind === 'line' && target.value ? target.value.transform.translation : { x: 0, y: 0, z: 0 };
  const [x, setX] = useState(position ? String(position.x) : '');
  const [y, setY] = useState(position ? String(position.y) : '');
  const [z, setZ] = useState(position ? String(position.z) : '');
  const [angle, setAngle] = useState('0');
  const [width, setWidth] = useState(target.kind === 'hall' && target.value ? String(target.value.width_m) : '');
  const [depth, setDepth] = useState(target.kind === 'hall' && target.value ? String(target.value.depth_m) : '');
  const [height, setHeight] = useState(target.kind === 'line' ? '' : target.value?.height_m == null ? '' : String(target.value.height_m));
  const [error, setError] = useState('');
  const [leaving, setLeaving] = useState(false);
  const [revisionConflict, setRevisionConflict] = useState(false);
  const [confirmReload, setConfirmReload] = useState(false);
  const [reloading, setReloading] = useState(false);
  const original = JSON.stringify({ title, hallId, module, lineId, itemBindings, resourceBindings, x, y, z, angle, width, depth, height });
  const [initial] = useState(original);
  const changed = original !== initial || !value;
  useEffect(() => {
    onDirtyChange?.(original !== initial);
    return () => onDirtyChange?.(false);
  }, [original, initial, onDirtyChange]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault(); setError(''); setLeaving(false);
    const number = (text: string, label: string, positive = false) => {
      if (text.trim() === '' || !Number.isFinite(Number(text)) || (positive && Number(text) <= 0)) throw new Error(`${label}须填写${positive ? '大于零的' : '有限'}数值。`);
      return Number(text);
    };
    try {
      if (!title.trim()) throw new Error('请填写名称。');
      let object: EditableProjectObject;
      if (target.kind === 'hall') {
        object = { kind: 'hall', data: { ...(target.value ?? {
          id: `hall-${crypto.randomUUID()}`, origin: identityPose(), input_locations: [], output_locations: [],
        }), title: title.trim(), width_m: number(width, '宽度', true), depth_m: number(depth, '进深', true),
          height_m: height === '' ? null : number(height, '净高', true) } };
      } else if (target.kind === 'line') {
        if (!hallId) throw new Error('请选择所属厂房。');
        object = { kind: 'line', data: { ...(target.value ?? {
          id: `line-${crypto.randomUUID()}`, members: [], boundary_ports: [], region_polygon_m: [],
        }), title: title.trim(), hall_id: hallId, transform: rotatePoseZ({ ...(target.value?.transform ?? identityPose()),
          translation: { x: number(x, 'X'), y: number(y, 'Y'), z: number(z, 'Z') } }, number(angle, '旋转增量')) } };
      } else {
        const source = devices.find(item => moduleKey({ id: item.definition.id, version: item.definition.version }) === module);
        if (!hallId || (!target.value && !source)) throw new Error('请选择厂房和实际已加载的精确设备版本。');
        let device: DeviceInstance = target.value ?? {
          id: `device-${crypto.randomUUID()}`, title: title.trim(), hall_id: hallId, line_id: null,
          module_ref: { id: source!.definition.id, version: source!.definition.version },
          pose: identityPose(), footprint_m: [
            { x: 0, y: 0 }, { x: number(width, '占地宽度', true), y: 0 },
            { x: number(width, '占地宽度', true), y: number(depth, '占地进深', true) }, { x: 0, y: number(depth, '占地进深', true) },
          ], configuration: {}, role_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} },
        };
        device = { ...device, title: title.trim(), height_m: height === '' ? null : number(height, '设备高度', true),
          pose: rotatePoseZ(device.pose, number(angle, '旋转增量')),
          role_bindings: { ...device.role_bindings, item_types: itemBindings, resources: resourceBindings } };
        await onApply(devicePlacementCommands(project, device, lineId || null, new Vector3(number(x, 'X'), number(y, 'Y'), number(z, 'Z'))));
        return;
      }
      await onApply([{ kind: 'put_object', data: object }]);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
      setRevisionConflict(failure instanceof HostFailure && failure.detail.code === 'RevisionConflict');
    }
  };
  async function reload() {
    setReloading(true);
    try { await onReload(); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setReloading(false); setConfirmReload(false); }
  }

  return <form className="object-editor" onSubmit={submit}>
    <header><div><small>工程草稿 · r{project.revision}</small><h2>{value ? '编辑' : '新建'}{({ hall: '厂房', line: '流水线组', device: '设备' })[target.kind]}</h2></div><button type="button" onClick={() => changed ? setLeaving(true) : onClose()} disabled={busy}>返回画布</button></header>
    <p className="form-hint">应用会形成工程新修订。尚未应用的字段只保留在此表单中。</p>
    <fieldset disabled={busy || reloading}>
      <label>名称<input value={title} onChange={event => setTitle(event.target.value)} required /></label>
      {target.kind !== 'hall' && <label>所属厂房<select value={hallId} onChange={event => setHallId(event.target.value)} disabled={!!value} required><option value="">请选择</option>{project.halls.map(hall => <option key={hall.id} value={hall.id}>{hall.title} · {hall.id}</option>)}</select></label>}
      {target.kind === 'device' && <label>设备定义 · 精确版本<select value={module} onChange={event => setModule(event.target.value)} disabled={!!value} required><option value="">请选择</option>{target.value && !modules.has(moduleKey(target.value.module_ref)) && <option value={moduleKey(target.value.module_ref)}>{target.value.module_ref.id} @ {target.value.module_ref.version} · 缺少定义</option>}{devices.map(item => <option key={moduleKey({ id: item.definition.id, version: item.definition.version })} value={moduleKey({ id: item.definition.id, version: item.definition.version })}>{item.definition.title} · {item.definition.version}</option>)}</select></label>}
      {target.kind === 'device' && <label>所属线组<select value={lineId} onChange={event => setLineId(event.target.value)}><option value="">不编组 · 厂房坐标</option>{project.lines.filter(line => line.hall_id === hallId).map(line => <option key={line.id} value={line.id}>{line.title}</option>)}</select></label>}
      {target.kind === 'device' && itemRoles.length > 0 && <section><h3>物品角色绑定</h3>{itemRoles.map(role => <label key={role}>{role}<select value={itemBindings[role] ?? ''} onChange={event => setItemBindings(previous => { const next = { ...previous }; if (event.target.value) next[role] = event.target.value; else delete next[role]; return next; })}><option value="">未绑定 · 保留为草稿缺项</option>{project.item_types.map(item => <option key={item.id} value={item.id}>{item.title} · {item.id} · {item.unit}</option>)}</select></label>)}</section>}
      {target.kind === 'device' && resourceRoles.length > 0 && <section><h3>实际资源绑定</h3>{resourceRoles.map(role => <label key={role}>{role}<select value={resourceBindings[role] ?? ''} onChange={event => setResourceBindings(previous => { const next = { ...previous }; if (event.target.value) next[role] = event.target.value; else delete next[role]; return next; })}><option value="">未绑定 · 保留为草稿缺项</option>{project.resources.map(resource => <option key={resource.id} value={resource.id}>{resource.title} · {resource.id} · {resource.capacity.unit}</option>)}</select></label>)}</section>}
      {target.kind !== 'hall' && <div className="field-grid">{[['X', x, setX], ['Y', y, setY], ['Z', z, setZ]] .map(([label, current, update]) => <label key={String(label)}>{String(label)} · 厂房坐标 m<input type="number" step="any" value={current as string} onChange={event => (update as (value: string) => void)(event.target.value)} required /></label>)}</div>}
      {target.kind !== 'hall' && <label>绕 Z 旋转增量 · °<input type="number" step="any" value={angle} onChange={event => setAngle(event.target.value)} required /></label>}
      {(target.kind === 'hall' || (target.kind === 'device' && !value)) && <div className="field-grid"><label>{target.kind === 'hall' ? '宽度' : '矩形占地宽度'} · m<input type="number" step="any" value={width} onChange={event => setWidth(event.target.value)} required /></label><label>进深 · m<input type="number" step="any" value={depth} onChange={event => setDepth(event.target.value)} required /></label></div>}
      {target.kind !== 'line' && <label>{target.kind === 'hall' ? '厂房净高' : '设备包络高度'} · m<input type="number" step="any" value={height} onChange={event => setHeight(event.target.value)} placeholder="未填写" /></label>}
    </fieldset>
    {error && <p className="error-message" role="alert">{error}</p>}
    <footer><button type="submit" className="primary" disabled={busy || reloading || revisionConflict || !changed}>应用到工程</button><button type="button" disabled={busy || reloading || !changed} onClick={onClose}>放弃更改</button>{revisionConflict && <button type="button" disabled={busy || reloading} onClick={() => setConfirmReload(true)}>重新读取当前对象</button>}</footer>
    {confirmReload && <div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-label="重新读取对象"><h2>重新读取当前对象</h2><p>读取最新草稿将丢弃本表单中未应用的字段。请先保留需要重新填写的内容；不会覆盖已提交的工程更改。</p><footer><button type="button" autoFocus disabled={reloading} onClick={() => setConfirmReload(false)}>返回保留输入</button><button type="button" disabled={reloading} onClick={() => void reload()}>丢弃本地输入并重新读取</button></footer></section></div>}
    {leaving && <div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-label="未应用字段"><h2>尚有未应用字段</h2><p>选择应用后返回，或明确放弃本次表单更改。</p><footer><button type="button" onClick={() => setLeaving(false)}>留在当前</button><button type="button" onClick={onClose}>丢弃并返回</button><button type="submit" className="primary" disabled={revisionConflict}>应用后返回</button></footer></section></div>}
  </form>;
}
