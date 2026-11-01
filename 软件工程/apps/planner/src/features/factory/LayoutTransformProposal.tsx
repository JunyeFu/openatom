import { useEffect, useState } from 'react';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { Pose } from '../../../../../packages/commands/generated/typescript/Pose';
import type { ProjectState } from '../../services/projectHost';

export interface TransformProposal { object: ObjectRef; pose: Pose; base: ProjectState; error: string }
export function LayoutTransformProposal({ proposal, busy, onApply, onReload, onClose, onDirtyChange }: {
  proposal: TransformProposal; busy: boolean;
  onApply(object: ObjectRef, pose: Pose, base: ProjectState): Promise<void>;
  onReload(): Promise<ProjectState>; onClose(): void; onDirtyChange(dirty: boolean): void;
}) {
  const [base, setBase] = useState(proposal.base), [error, setError] = useState(proposal.error);
  const [leaving, setLeaving] = useState(false), [loading, setLoading] = useState(false);
  useEffect(() => { onDirtyChange(true); return () => onDirtyChange(false); }, [onDirtyChange]);
  async function reload() {
    setLoading(true);
    try {
      const actual = await onReload();
      if (actual.id !== base.id) throw new Error('工程已切换，原变换仍保留，不能应用到其他工程。');
      const parent = (project: ProjectState) => proposal.object.kind === 'device'
        ? project.devices.filter(value => value.id === proposal.object.id).map(value => [value.hall_id, value.line_id])
        : proposal.object.kind === 'line' ? project.lines.filter(value => value.id === proposal.object.id).map(value => value.hall_id)
        : project.halls.filter(value => value.id === proposal.object.id).map(value => value.id);
      if (JSON.stringify(parent(proposal.base)) !== JSON.stringify(parent(actual))) throw new Error('对象父坐标已变化，请保留此姿态供核对并在当前画布重新定位。');
      setBase(actual); setError('');
    }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setLoading(false); }
  }
  async function apply() {
    try { await onApply(proposal.object, proposal.pose, base); onClose(); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }
  return <section className="object-editor" aria-label="保留的布局变换">
    <header><h2>保留的布局变换</h2><button disabled={busy || loading} onClick={() => setLeaving(true)}>返回画布</button></header>
    <p>{proposal.object.kind} / {proposal.object.id} · 提交基础 r{base.revision}</p>
    <p>变换尚未应用。核对当前修订后可重读基础，保留同一父坐标下的完整姿态再次提交。</p>
    <pre>{JSON.stringify(proposal.pose, null, 2)}</pre>
    {error && <p role="alert" className="error-text">{error} 变换输入保留。</p>}
    <div className="form-actions"><button disabled={busy || loading} onClick={() => void reload()}>重读基础并保留变换</button><button disabled={busy || loading} onClick={() => void apply()}>应用保留的变换</button></div>
    {leaving && <div role="dialog" aria-label="放弃布局变换"><p>此变换尚未应用。</p><button onClick={() => setLeaving(false)}>返回保留变换</button><button onClick={onClose}>明确放弃变换</button></div>}
  </section>;
}
