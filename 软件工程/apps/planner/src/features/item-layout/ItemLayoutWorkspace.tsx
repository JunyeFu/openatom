import { useEffect, useMemo, useRef, useState } from 'react';
import type { WorkspaceSnapshot, EditCommand } from '../../services/projectHost';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import { createPresentationHost, type ProcessReadContext } from '../../services/presentationHost';
import { resolveMedia } from '../media/load';
import { blankSlot, layoutInput, layoutObject, objectLayout, ownerKey, parseLayout, typeKey, withLayout, type ItemLayoutOwner, type LayoutInput, type SlotInput } from './model';
import './item-layout.css';
import { factoryItemTypeTitle } from '../../scene/factoryLabels';

const ownerLabels = { device: '设备', location: '库位', item_type: '物品类型' };
const vectorLabels = { translation: '位置偏移', rotation: '朝向（四元数）', scale: '缩放比例' };

export interface ItemLayoutWorkspaceProps {
  workspace: WorkspaceSnapshot;
  initialOwner?: ItemLayoutOwner;
  busy?: boolean;
  /** Original Project apply_commands + shared history + refresh; return the actual refreshed Project. */
  onApply(commands: EditCommand[], base: ProjectState): Promise<ProjectState>;
  onClose(): void;
  onDirtyChange?(dirty: boolean): void;
}
export function ItemLayoutWorkspace(props: ItemLayoutWorkspaceProps) {
  const project = props.workspace.project;
  const [owner, setOwner] = useState<ItemLayoutOwner | null>(props.initialOwner ?? null);
  const [base, setBase] = useState(project), [session] = useState(props.workspace.context.host.session_id);
  const [input, setInput] = useState<LayoutInput>(() => { const object = project && owner ? layoutObject(project, owner) : null; return layoutInput(object ? objectLayout(object) : null); });
  const [initial, setInitial] = useState(JSON.stringify(input));
  const [saving, setSaving] = useState(false), [error, setError] = useState(''), [message, setMessage] = useState('');
  const [confirm, setConfirm] = useState<'close' | ItemLayoutOwner | null>(null), [lod, setLod] = useState<'detailed' | 'simplified'>('detailed');
  const [roles, setRoles] = useState<{ key: string; names: string[]; message: string }>({ key: '', names: [], message: '' });
  const dirty = JSON.stringify(input) !== initial, object = base && owner ? layoutObject(base, owner) : null;
  const identity = props.workspace.context.host;
  const sameProject = project?.id === base?.id && identity.session_id === session && identity.active_project_id === base?.id
    && identity.selected_scope?.kind !== 'definition_draft' && (!identity.selected_scope || identity.selected_scope.data === base?.id);
  const live = useRef({ sameProject, projectId: project?.id, session: props.workspace.context.host.session_id }); live.current = { sameProject, projectId: project?.id, session: props.workspace.context.host.session_id };
  const ref = object?.kind === 'device' ? object.data.module_ref : object?.kind === 'item_type' ? object.data.module_ref : null;
  const context: ProcessReadContext | null = base && ref && sameProject ? { session_id: session, project_id: base.id, scope: { kind: 'project' }, scope_revision: base.revision,
    selection: { kind: 'definition', module_ref: ref, ...(object?.kind === 'device' ? { device_id: object.data.id } : {}) } } : null;
  const current = useRef(context); current.current = context;
  const host = useMemo(() => createPresentationHost(() => current.current), []);
  const rolesKey = JSON.stringify([context, lod]);
  useEffect(() => { props.onDirtyChange?.(dirty); }, [dirty, props.onDirtyChange]);
  function reset(nextProject: ProjectState, nextOwner: ItemLayoutOwner) {
    const next = layoutObject(nextProject, nextOwner), value = layoutInput(next ? objectLayout(next) : null);
    setBase(nextProject); setOwner(nextOwner); setInput(value); setInitial(JSON.stringify(value)); setError(''); setConfirm(null);
  }
  useEffect(() => { if (!dirty && !saving && sameProject && project && owner && project.revision !== base?.revision) reset(project, owner); }, [project?.revision, project?.id, dirty, saving, sameProject]);
  useEffect(() => {
    let active = true; setRoles({ key: rolesKey, names: [], message: object?.kind === 'location' ? '摆放位置以库位坐标为起点，不使用模型支承点。' : !ref ? '尚未提供可读取的模型，支承点暂不可选，原输入仍保留。' : '正在读取当前模型的支承点…' });
    if (!context) return;
    host.read(context).then(async value => {
      const reader = host.media(context, value.binding), resolved = await resolveMedia({ ...reader, presentation: value.presentation, binding: value.binding, rights: value.rights, lod, mode: 'asset_inspection' });
      const names = Object.entries(resolved.presentation.anchor_roles).filter(([, anchor]) => anchor.kind === 'scene_node' && resolved.presentation.nodes[anchor.data] != null).map(([name]) => name);
      if (active) setRoles({ key: rolesKey, names, message: names.length ? '下列支承点来自当前模型，选择物品应放置的位置。' : '当前模型未提供可用支承点；保留缺项时，这部分物品不会显示在画面中。' });
    }, reason => { if (active) setRoles({ key: rolesKey, names: [], message: String(reason) }); }).catch(reason => { if (active) setRoles({ key: rolesKey, names: [], message: String(reason) }); });
    return () => { active = false; };
  }, [host, rolesKey]);
  function choose(next: ItemLayoutOwner) { if (dirty) setConfirm(next); else if (project) reset(project, next); }
  function slot(index: number, value: SlotInput) { setInput(v => ({ ...v, slots: v.slots.map((s, i) => i === index ? value : s) })); }
  async function apply() {
    if (!base || !owner || !object || saving || !sameProject) return;
    setSaving(true); setError(''); setMessage('');
    try {
      const layout = parseLayout(input, owner), committed = await props.onApply([{ kind: 'put_object', data: withLayout(object, layout) }], base);
      if (!live.current.sameProject || live.current.projectId !== base.id || live.current.session !== session) { setMessage('原工程提交已返回；当前工程已切换，原编辑输入仍保留。'); return; }
      reset(committed, owner); setMessage('摆放布局已保存。库存、业务容量和旧运行记录保持不变。');
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setSaving(false); }
  }
  const availableRoles = roles.key === rolesKey ? roles.names : [];
  const owners: ItemLayoutOwner[] = project ? [...project.devices.map(v => ({ kind: 'device' as const, id: v.id })), ...project.locations.map(v => ({ kind: 'location' as const, id: v.id })), ...project.item_types.map(v => ({ kind: 'item_type' as const, id: v.id }))] : [];
  const ownerName = (value: ItemLayoutOwner) => {
    const target = project && layoutObject(project, value);
    const title = target?.kind === 'item_type' ? factoryItemTypeTitle(project, target.data) : target?.data.title;
    return `${ownerLabels[value.kind]} · ${title || '未命名对象'}`;
  };
  return <section className="sf-item-layout" aria-label="物品展示槽创作">
    <header><h2>物品展示槽</h2><button disabled={saving || props.busy} onClick={() => dirty ? setConfirm('close') : props.onClose()}>关闭展示槽</button></header>
    {confirm && <div role="alertdialog" aria-label="保留展示槽输入"><p>展示槽有未应用输入。保留编辑，或明确放弃后{confirm === 'close' ? '关闭' : '切换拥有者'}。</p><button onClick={() => setConfirm(null)}>保留编辑</button><button onClick={() => { if (confirm === 'close') props.onClose(); else if (project) reset(project, confirm); }}>放弃输入并{confirm === 'close' ? '关闭' : '切换'}</button></div>}
    <p>为设备、库位或容器中的物品设置画面上的摆放位置。同一位置内的同类物品按编号顺序摆放。这里的槽位数量不会改变库存或业务容量；未声明的位置不绘制物品。</p>
    <label>摆放对象 <select aria-label="展示槽拥有者" disabled={saving || props.busy || !sameProject} value={owner ? ownerKey(owner) : ''} onChange={e => { if (e.target.value) { const [kind,id] = JSON.parse(e.target.value); choose({ kind, id }); } }}><option value="">选择设备、库位或物品类型</option>{owners.map(v => <option key={ownerKey(v)} value={ownerKey(v)}>{ownerName(v)}</option>)}</select></label>
    {!sameProject && <p role="alert">工程或Host会话已切换；原输入保留，不能提交到新工程。</p>}
    {base && owner && <details><summary>对象编号与工程来源</summary><p>工程：{base.title}；编辑版本：{base.revision}；工程编号：{base.id}；对象编号：{owner.kind} / {owner.id}。</p>{owner.kind === 'location' && object?.kind === 'location' && <p>业务容量：{object.data.capacity ? JSON.stringify(object.data.capacity) : '未声明'}</p>}</details>}
    {owner && <><fieldset disabled={saving || props.busy}><legend>布局声明</legend><label><input type="radio" name="layout-presence" checked={!input.declared} onChange={() => setInput(v => ({ ...v, declared: false }))}/>未声明布局</label><label><input type="radio" name="layout-presence" checked={input.declared} onChange={() => setInput(v => ({ ...v, declared: true }))}/>明确声明布局（可为空）</label>
      {input.declared && <><label>分配方式 <select aria-label="展示槽分配方式" value={input.assignment} onChange={e => setInput(v => ({ ...v, assignment: e.target.value as LayoutInput['assignment'] }))}><option value="">选择分配规则</option><option value="item_id_order">按物品编号排序</option></select></label><button onClick={() => setInput(v => ({ ...v, slots: [...v.slots, blankSlot()] }))}>添加摆放点</button></>}
    </fieldset>
    {input.declared && <><label>模型细节 <select aria-label="角色核对档位" value={lod} onChange={e => setLod(e.target.value as typeof lod)}><option value="detailed">详细</option><option value="simplified">简化</option></select></label><p role="status">{roles.key === rolesKey ? roles.message : '正在读取模型支承点…'}</p>
      {input.slots.map((s, i) => <fieldset key={i} disabled={saving || props.busy}><legend>槽{i + 1}</legend>
        <label>物品类型及版本 <select aria-label={`槽${i + 1}子类型`} value={s.item} onChange={e => slot(i, { ...s, item: e.target.value })}><option value="">选择物品类型</option>{s.item && !base?.item_types.some(t => typeKey({ id: t.id, revision: t.revision }) === s.item) && <option value={s.item}>原引用的物品类型已缺失</option>}{base?.item_types.map(t => <option key={t.id + t.revision} value={typeKey({ id: t.id, revision: t.revision })}>{factoryItemTypeTitle(base,t)} · 版本 {t.revision}</option>)}</select></label>
        {owner.kind === 'location' ? <p>偏移以库位坐标为起点，不使用模型支承点。</p> : <label>模型支承点 <select aria-label={`槽${i + 1}角色`} value={s.anchor} onChange={e => slot(i, { ...s, anchor: e.target.value })}><option value="">暂未指定支承点</option>{s.anchor && !availableRoles.includes(s.anchor) && <option value={s.anchor}>当前模型缺少原支承点</option>}{availableRoles.map(name => <option key={name}>{name}</option>)}</select></label>}
        <details><summary>物品与支承点引用详情</summary><p>物品引用：{s.item || '未选择'}；支承点：{s.anchor || '未选择'}。</p></details>
        <label>单位 <select aria-label={`槽${i + 1}单位`} value={s.unit} onChange={e => slot(i, { ...s, unit: e.target.value as SlotInput['unit'] })}><option value="">选择单位</option><option value="meter">米（m）</option><option value="millimeter">毫米（mm）</option></select></label>
        <label>坐标系 <select aria-label={`槽${i + 1}坐标系`} value={s.coordinate_frame} onChange={e => slot(i, { ...s, coordinate_frame: e.target.value as SlotInput['coordinate_frame'] })}><option value="">选择坐标系</option><option value="right_handed_z_up">右手坐标系 · Z轴向上</option><option value="right_handed_y_up">右手坐标系 · Y轴向上</option></select></label>
        {(['translation','rotation','scale'] as const).map(group => <div className="sf-item-layout-vector" key={group}><strong>{vectorLabels[group]}</strong>{Object.entries(s[group]).map(([axis,value]) => <label key={axis}>{axis.toUpperCase()}<input type="number" step="any" aria-label={`槽${i + 1} ${vectorLabels[group]} ${axis.toUpperCase()}`} value={value} onChange={e => slot(i, { ...s, [group]: { ...s[group], [axis]: e.target.value } })}/></label>)}</div>)}
        <button onClick={() => setInput(v => ({ ...v, slots: v.slots.filter((_, index) => index !== i) }))}>删除槽{i + 1}</button><button disabled={i === 0} onClick={() => setInput(v => { const slots = [...v.slots]; [slots[i - 1],slots[i]] = [slots[i],slots[i - 1]]; return { ...v, slots }; })}>槽{i + 1}上移</button>
      </fieldset>)}
    </>}
    <button disabled={!dirty || !sameProject || saving || props.busy || !object} onClick={() => { void apply(); }}>应用展示槽</button><button disabled={!dirty || saving || !project} onClick={() => project && reset(project, owner)}>放弃本地修改并重读</button></>}
    {error && <p role="alert">{error} · 本地输入保留。</p>}{message && <p role="status">{message}</p>}
  </section>;
}
