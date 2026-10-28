import { useEffect, useState } from 'react';
import type { ProjectState, EditCommand } from '../../services/projectHost';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import { copyLayoutSelection, groupLayoutSelection, layoutSelection } from '../../scene/layoutEditing';

export interface LayoutSelectionEditorProps {
  project: ProjectState;
  selection: ObjectRef[];
  mode: 'group' | 'copy';
  busy: boolean;
  onApply(commands: EditCommand[], base: ProjectState): Promise<void>;
  onReload(): Promise<ProjectState>;
  onClose(): void;
  onDirtyChange?(dirty: boolean): void;
}

export function LayoutSelectionEditor({ project, selection, mode, busy, onApply, onReload, onClose, onDirtyChange }: LayoutSelectionEditorProps) {
  const [base, setBase] = useState(project);
  const [groupId, setGroupId] = useState(() => 'line-' + crypto.randomUUID());
  const [title, setTitle] = useState('新线组');
  const chosen = layoutSelection(base, selection);
  const [ids, setIds] = useState<Record<string, string>>(() => Object.fromEntries([...chosen.lines, ...chosen.devices].map(value => [value.id, value.id + '-copy'])));
  const [x, setX] = useState('2'), [y, setY] = useState('2'), [z, setZ] = useState('0');
  const [error, setError] = useState(''), [leaving, setLeaving] = useState(false), [loading, setLoading] = useState(false), [attempted, setAttempted] = useState(false);
  const value = JSON.stringify({ groupId, title, ids, x, y, z });
  const [initial] = useState(value);
  const dirty = attempted || value !== initial;
  useEffect(() => { onDirtyChange?.(dirty); }, [dirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  const blocked = busy || loading;
  function close() { if (dirty) setLeaving(true); else onClose(); }
  async function submit(event: React.FormEvent) {
    event.preventDefault(); setError(''); setAttempted(true);
    try {
      if ([x, y, z].some(text => !text.trim() || !Number.isFinite(Number(text)))) throw new Error('请填写有限的X、Y、Z位移，单位为米。');
      const commands = mode === 'group' ? groupLayoutSelection(base, selection, groupId.trim(), title)
        : copyLayoutSelection(base, selection, ids, { x: Number(x), y: Number(y), z: Number(z) });
      await onApply(commands, base);
    } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }
  async function reload() {
    setLoading(true); setError('');
    try {
      const actual = await onReload();
      if (actual.id !== base.id) throw new Error('工程已切换，本地输入保留，不能移交到另一个工程。');
      setBase(actual);
    } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setLoading(false); }
  }
  return <form className="object-editor p201-layout-editor" aria-label={mode === 'group' ? '编组所选设备' : '复制布局为新ID'} onSubmit={submit}>
    <header><h2>{mode === 'group' ? '编组所选设备' : '复制布局为新ID'}</h2><button type="button" disabled={blocked} onClick={close}>返回画布</button></header>
    <p>当前基础 r{base.revision} · {chosen.lines.length}个线组 / {chosen.devices.length}台设备。所有修改通过共同工程命令原子提交，可在本会话撤销。</p>
    <fieldset disabled={blocked}>
      {mode === 'group' ? <><label>新线组ID<input value={groupId} onChange={event => setGroupId(event.target.value)} required /></label>
        <label>线组名称<input value={title} onChange={event => setTitle(event.target.value)} required /></label>
        <p>保留设备在厂房中的位置和完整朝向；原线组其余成员保持。线组原点取所选设备中心，区域仅按已有占地形成显示边界。</p></> : <>
        <p>请核对每个新ID。复制保留设备参数和局部布局，角色、边界端口及连接需要重新绑定；不会复制物料供给、在制、订单或专业认可。</p>
        {[...chosen.lines, ...chosen.devices].map(object => <label key={object.id}>{object.title} · {object.id} → 新ID<input value={ids[object.id] ?? ''} onChange={event => setIds({ ...ids, [object.id]: event.target.value })} required /></label>)}
        <div className="numeric-row"><label>X位移 / m<input type="number" step="any" value={x} onChange={event => setX(event.target.value)} required /></label>
          <label>Y位移 / m<input type="number" step="any" value={y} onChange={event => setY(event.target.value)} required /></label>
          <label>Z位移 / m<input type="number" step="any" value={z} onChange={event => setZ(event.target.value)} required /></label></div>
      </>}
      <ul>{chosen.devices.map(device => <li key={device.id}>{device.title} · {device.id} · {device.hall_id}</li>)}</ul>
      <div className="form-actions"><button type="button" onClick={() => void reload()}>重读基础并保留输入</button><button type="submit">{mode === 'group' ? '应用编组' : '复制为新ID'}</button></div>
    </fieldset>
    {error && <p role="alert" className="error-text">{error} 本地输入保留。</p>}
    {leaving && <div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-label="放弃布局输入"><h2>放弃布局输入</h2><p>本次布局输入尚未应用。</p><footer><button type="button" autoFocus onClick={() => setLeaving(false)}>返回保留输入</button><button type="button" onClick={onClose}>明确放弃本地输入</button></footer></section></div>}
  </form>;
}
