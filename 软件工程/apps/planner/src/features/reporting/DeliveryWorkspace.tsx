import { runRecordLabel, runStatusLabel } from '../../services/workbenchLabels';
import { useEffect, useState } from 'react';
import type { MetricKey } from '../../../../../packages/commands/generated/typescript/MetricKey';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { DeliveryWorkspaceProps, RunEvidence } from '../comparison/types';
import { acceptanceScopes, errorDetail, errorText, evidenceMatches, metricLabels, quantityText, scopeKey, scopeText } from '../comparison/model';
import { ComparisonPanel } from '../comparison/ComparisonPanel';
import { JsonDetails } from '../comparison/Shared';
import { ReportPanel } from './ReportPanel';
import { HandoffPanel } from '../handoff/HandoffPanel';
import { useDeliveryTask } from './useDeliveryTask';
import { TaskReceipt } from './TaskReceipt';
import './delivery.css';

export function DeliveryWorkspace({ workspace, services, initialTab = 'comparison', onClose, onInspectRun, onLocate, onOpenPackage, onEditModule }: DeliveryWorkspaceProps) {
  const project = workspace.project;
  const [tab, setTab] = useState(initialTab);
  const [runIds, setRunIds] = useState<string[]>([]);
  const [evidence, setEvidence] = useState<RunEvidence[]>([]);
  const [loading, setLoading] = useState(false); const [failure, setFailure] = useState('');
  const [failureDetail, setFailureDetail] = useState<unknown>(null);
  const [keys, setKeys] = useState<MetricKey[]>(['accepted_quantity', 'completion_time', 'waiting_time']);
  const [scopeIds, setScopeIds] = useState<string[]>([]);
  const [objects, setObjects] = useState<ObjectRef[]>([]);
  const task = useDeliveryTask(services, project?.id ?? '');
  const selectionKey = JSON.stringify(runIds);
  useEffect(() => {
    let live = true; setEvidence([]); setFailure(''); setFailureDetail(null);
    if (!project || !runIds.length) { setLoading(false); return () => { live = false; }; }
    setLoading(true);
    void Promise.all(runIds.map(id => services.readEvidence(id))).then(values => {
      if (!live) return;
      if (!evidenceMatches(project.id, runIds, values)) throw new Error('所读冻结工程／方案／Run身份不一致；未借用当前草稿替换。');
      setEvidence(values);
    }).catch(error => { if (live) { setFailure(errorText(error)); setFailureDetail(errorDetail(error)); } }).finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [project?.id, selectionKey, services.readEvidence]);
  if (!project) return <section className="delivery-workspace"><h2>请先创建或打开工程</h2><button onClick={onClose}>返回工作台</button></section>;
  const runs = (workspace.context.run_summaries ?? []).filter(r => r.input.project_id === project.id);
  const ready = !loading && evidenceMatches(project.id, runIds, evidence);
  const scopes = acceptanceScopes(ready ? evidence : []);
  const objectChoices = [...new Map(evidence.flatMap(e => [...e.project.devices.map(o => ({ kind: 'device' as const, id: o.id })), ...e.project.resources.map(o => ({ kind: 'resource' as const, id: o.id }))]).map(o => [JSON.stringify(o), o])).values()];
  const scope = { keys, objects: objects.filter(o => objectChoices.some(c => c.id === o.id && c.kind === o.kind)), acceptance: scopes.filter(s => scopeIds.includes(scopeKey(s))) };
  return <section className="delivery-workspace" aria-label="方案评估与交接工作区">
    <header className="delivery-title"><div><small>{project.title}</small><details><summary>工程信息</summary><p>工程编号：{project.id}；工程版本：{project.revision}。</p></details><h1>方案评估与交接</h1></div><button disabled={task.active} onClick={onClose}>返回流水线工作台</button></header>
    <nav className="delivery-tabs" aria-label="评估与交付任务">{(['comparison', 'reporting', 'handoff'] as const).map(t => <button key={t} aria-pressed={tab === t} onClick={() => setTab(t)}>{{ comparison: '方案比较', reporting: '评估报告', handoff: '可编辑交接与贡献' }[t]}</button>)}</nav>
    <div className="delivery-content"><aside className="delivery-selection"><h2>选择运行记录</h2><p>选择一次运行导出报告；需要比较时可选两次。</p>{runs.map((r,index) => <label className="delivery-run-choice" key={r.run_id}><input type="checkbox" checked={runIds.includes(r.run_id)} disabled={task.active || !runIds.includes(r.run_id) && runIds.length >= 2} onChange={e => setRunIds(e.target.checked ? [...runIds, r.run_id] : runIds.filter(id => id !== r.run_id))} /><span><strong>{runRecordLabel(r,index,workspace.scenarios,evidence.find(e=>e.run.id===r.run_id)?.scenario)}</strong><small>{runStatusLabel[r.computation_status]} · {r.whole_window_computed ? '完整窗口' : '前缀'} · 接受量 {r.accepted ? quantityText(r.accepted) : '未知'}</small></span></label>)}{!runs.length && <p>{workspace.context.run_summaries === undefined ? '宿主未提供运行列表。' : '尚无实际运行。'}</p>}
      {loading && <p role="status">正在读取所选运行记录…</p>}{failure && <p role="alert" className="delivery-error">{failure}</p>}{failureDetail != null && <JsonDetails label="冻结输入读取错误与字段定位" value={failureDetail} />}
      <fieldset disabled={task.active}><legend>指标</legend>{(Object.keys(metricLabels) as MetricKey[]).map(k => <label key={k}><input type="checkbox" checked={keys.includes(k)} onChange={e => setKeys(e.target.checked ? [...keys, k] : keys.filter(v => v !== k))} />{metricLabels[k]}</label>)}</fieldset>
      <fieldset disabled={task.active}><legend>接受口径</legend><p>不选时按宿主已声明范围分别列出；多个产品不合成一个单位成本。</p>{scopes.map(s => <label key={scopeKey(s)}><input type="checkbox" checked={scopeIds.includes(scopeKey(s))} onChange={e => setScopeIds(e.target.checked ? [...scopeIds, scopeKey(s)] : scopeIds.filter(v => v !== scopeKey(s)))} />{scopeText(s)}</label>)}</fieldset>
      <details><summary>限定设备／资源范围</summary><fieldset disabled={task.active}>{objectChoices.map(o => <label key={JSON.stringify(o)}><input type="checkbox" checked={objects.some(s => s.id === o.id && s.kind === o.kind)} onChange={e => setObjects(e.target.checked ? [...objects, o] : objects.filter(s => s.id !== o.id || s.kind !== o.kind))} />{o.kind}:{o.id}</label>)}</fieldset></details>
    </aside><div className="delivery-main">
      <TaskReceipt task={task} />
      <div hidden={tab !== 'comparison'}><ComparisonPanel projectId={project.id} evidence={ready ? evidence : []} scope={scope} services={services} busy={task.active || loading || !!failure} onInspectRun={onInspectRun} onLocate={onLocate} /></div>
      <div hidden={tab !== 'reporting'}><ReportPanel workspace={workspace} services={services} evidence={ready ? evidence : []} scope={scope} ready={ready} task={task} onLocate={onLocate} /></div>
      <div hidden={tab !== 'handoff'}><HandoffPanel workspace={workspace} services={services} task={task} onOpenPackage={onOpenPackage} onEditModule={onEditModule} /></div>
    </div></div>
    <footer className="delivery-footer">原运行记录和输入版本保持不变 · 报告不等于可编辑工程 · {task.active ? '任务仍在处理中，等待真实终态' : '数据由当前宿主提供'}</footer>
  </section>;
}
