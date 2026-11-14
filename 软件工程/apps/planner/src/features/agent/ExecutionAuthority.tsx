import { useEffect, useState } from 'react';
import type { ExecutionReadAccessAllowedSessionControlsItem as SessionControl } from '../../../../../packages/commands/generated/typescript/ExecutionReadAccessAllowedSessionControlsItem';
import type { ExecutionSourceSummary } from '../../../../../packages/commands/generated/typescript/ExecutionSourceSummary';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import { readExecution, readLocalView, type ExecutionReadEnvelope } from '../../services/cloudProduction';
import { grantAgentExecution } from '../../services/agentConnection';

const controls: [SessionControl, string][] = [['run', '开始生产'], ['pause_dispatch', '暂停派工'],
  ['resume_dispatch', '恢复派工'], ['stop_dispatch', '停止派工'], ['request_reconciliation', '核对原指令']];
const states: Record<string, string> = { prepared: '待开始', running: '生产中', dispatch_paused: '暂停派工',
  draining: '收尾中', stopped: '已停止', completed: '已完成', needs_resolution: '需要处理', failed: '失败', cancelled: '已取消' };

export function ExecutionAuthority({ workspace, hostInstanceId, sessionId, enabled, busy, action, notice, onDirtyChange }: {
  workspace: WorkspaceSnapshot; hostInstanceId: string | null; sessionId: string; enabled: boolean; busy: boolean;
  action: (work: (isCurrent: () => boolean) => Promise<void>) => Promise<void>;
  notice: (message: string) => void; onDirtyChange: (dirty: boolean) => void;
}) {
  const [entries, setEntries] = useState<ExecutionSourceSummary[]>([]);
  const [selected, setSelected] = useState('');
  const [source, setSource] = useState<ExecutionReadEnvelope | null>(null);
  const [actions, setActions] = useState<SessionControl[]>([]);
  const [devices, setDevices] = useState<string[]>([]);
  const [accepted, setAccepted] = useState('[[],[]]');
  const pending = JSON.stringify([actions, devices]) !== accepted;
  useEffect(() => { onDirtyChange(pending); return () => onDirtyChange(false); }, [pending, onDirtyChange]);
  useEffect(() => {
    setEntries([]); setSelected(''); setSource(null); setActions([]); setDevices([]); setAccepted('[[],[]]');
  }, [sessionId, workspace.project?.id, hostInstanceId]);
  const context = workspace.project && hostInstanceId ? { projectId: workspace.project.id, projectRevision: workspace.project.revision,
    hostContextId: hostInstanceId } : null;
  const allowedSource = source?.access.ownership === 'current_host_owner'
    && source.snapshot.identity.host_context_session_id === context?.hostContextId;
  function resetSelection() { setSource(null); setActions([]); setDevices([]); setAccepted('[[],[]]'); }
  return <fieldset disabled={busy || !enabled || !context}><legend>5 · 本次生产记录的执行权限</legend>
    <p>先在操作范围中允许“控制生产派工”，再明确选择记录、动作和设备。默认不授权执行；此处不会开始或停止生产。</p>
    <button onClick={() => void action(async current => {
      if (!context) return;
      const result = await readLocalView(context, { kind: 'catalog' });
      if (!current() || result.kind !== 'catalog') return;
      setEntries(result.data.entries); setSelected(''); resetSelection();
      notice('已读取当前工程的实际生产记录；历史记录保持只读。');
    })}>读取可授权的生产记录</button>
    <label>选择生产记录<select value={selected} onChange={event => {
      const value = event.target.value; setSelected(value); resetSelection();
      const entry = entries.find(item => item.identity.session_id === value);
      if (!context || !entry?.active_owner) return;
      void action(async current => {
        const actual = await readExecution(context, entry.identity, { kind: 'latest' });
        if (current()) setSource(actual);
      });
    }}><option value="">明确选择一条记录</option>{entries.map((entry, index) =>
      <option key={entry.identity.session_id} value={entry.identity.session_id}
        disabled={!entry.active_owner || entry.identity.host_context_session_id !== context?.hostContextId}>
        记录 {index + 1} · {states[entry.lifecycle_state] ?? '状态待核对'} · {entry.active_owner ? '当前桌面' : '历史只读'}
      </option>)}</select></label>
    {source && <>
      <p>所选记录：方案修订 {source.snapshot.project_input_ref.scenario_revision}，生产修订 {source.snapshot.execution_revision}。动作仍受实际执行状态限制。</p>
      <div className="agent-operations">{controls.map(([kind, label]) => <label key={kind}>
        <input type="checkbox" checked={actions.includes(kind)} disabled={!allowedSource}
          onChange={() => setActions(old => old.includes(kind) ? old.filter(value => value !== kind) : [...old, kind])} />{label}
      </label>)}</div>
      <p>允许改变下列设备的接单开关（不等于设备自身已恢复）：</p>
      {source.snapshot.devices.map((device, index) => {
        const id = device.binding.device_id;
        const sameInput = workspace.project?.revision === source.snapshot.project_input_ref.project_revision;
        const title = sameInput ? workspace.project?.devices.find(item => item.id === id)?.title
          ?? workspace.project?.resources.find(item => item.id === id)?.title ?? id : id;
        return <label key={id}><input type="checkbox" checked={devices.includes(id)} disabled={!allowedSource}
          onChange={() => setDevices(old => old.includes(id) ? old.filter(value => value !== id) : [...old, id])} />{title || `设备 ${index + 1}`}</label>;
      })}
      <button disabled={!allowedSource} onClick={() => void action(async current => {
        await grantAgentExecution(sessionId, { identity: source.snapshot.identity, session_controls: actions, device_acceptance_control_ids: devices });
        if (current()) { setAccepted(JSON.stringify([actions, devices])); notice('已向所选连接提交此记录的明确执行权限；Agent仍须读取实际权限后操作。'); }
      })}>确认此记录的执行权限</button>
      <button disabled={!allowedSource} onClick={() => void action(async current => {
        await grantAgentExecution(sessionId, { identity: source.snapshot.identity, session_controls: [], device_acceptance_control_ids: [] });
        if (current()) { setActions([]); setDevices([]); setAccepted('[[],[]]'); notice('已撤回所选连接在此记录的全部执行权限。'); }
      })}>撤回此记录的执行权限</button>
    </>}
  </fieldset>;
}
