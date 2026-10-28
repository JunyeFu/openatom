import { useRef, useState } from 'react';
import type { MetricScope } from '../../../../../packages/commands/generated/typescript/MetricScope';
import type { RunComparison } from '../../../../../packages/commands/generated/typescript/RunComparison';
import type { DeliveryServices, DeliveryWorkspaceProps, RunEvidence } from './types';
import { errorDetail, errorText } from './model';
import { IssueList, JsonDetails } from './Shared';
import {ComparisonResult} from './ComparisonResult';

interface Props {
  projectId: string; evidence: readonly RunEvidence[]; scope: MetricScope;
  services: DeliveryServices; busy: boolean;
  onInspectRun: DeliveryWorkspaceProps['onInspectRun']; onLocate: DeliveryWorkspaceProps['onLocate'];
}
export function ComparisonPanel({ projectId, evidence, scope, services, busy, onInspectRun, onLocate }: Props) {
  const [comparison, setComparison] = useState<{ key: string; value: RunComparison } | null>(null);
  const [pendingKey, setPendingKey] = useState<string | null>(null);
  const [failure, setFailure] = useState<{ key: string; text: string; detail: unknown } | null>(null);
  const [newId, setNewId] = useState(''); const [title, setTitle] = useState(''); const [created, setCreated] = useState<{ key: string; text: string } | null>(null);
  const runIds = evidence.map(e => e.run.id);
  const key = JSON.stringify([projectId, runIds, scope]); const current = useRef(key); current.current = key;
  const pending = pendingKey === key;
  const value = comparison?.key === key ? comparison.value : null;
  async function compare() {
    const identity = key; setPendingKey(key); setFailure(null);
    try {
      const value = await services.compare({ run_ids: runIds, metric_scope: scope });
      if (current.current !== identity) return;
      if (value.run_ids.length !== runIds.length || !runIds.every(id => value.run_ids.includes(id))) throw new Error('宿主比较结果与当前所选Run不一致。');
      setComparison({ key: identity, value });
    } catch (e) { if (current.current === identity) setFailure({ key: identity, text: errorText(e), detail: errorDetail(e) }); }
    finally { if (current.current === identity) setPendingKey(null); }
  }
  async function fork() {
    if (!services.cloneScenario || !evidence[0]) return;
    const identity = key; setPendingKey(key); setFailure(null); setCreated(null);
    try {
      const ref = await services.cloneScenario({ source_run_id: evidence[0].run.id, new_scenario_id: newId.trim(), title: title.trim() });
      if (identity === current.current) setCreated({ key: identity, text: `已创建方案 ${ref.scenario_id} r${ref.revision}；旧Run仍保留，后续从初始输入重跑。` });
    } catch (e) { if (identity === current.current) setFailure({ key: identity, text: errorText(e), detail: errorDetail(e) }); }
    finally { if (identity === current.current) setPendingKey(null); }
  }
  return <section className="delivery-panel" aria-label="方案比较">
    <header><div><h2>先核共同条件，再看实际差异</h2><p>主比较最多两个Run。修订、单位、目的接受口径或初始条件不一致时，由宿主给出可比性与限制。</p></div><button className="primary" disabled={busy || pending || evidence.length !== 2} onClick={() => void compare()}>比较所选两次运行</button></header>
    {evidence.length < 2 && <p className="delivery-hint">可以选一个Run直接查看和交付报告；需要比较时再选择第二个Run。</p>}
    <div className="delivery-run-pair">{evidence.map((e, n) => <article key={e.run.id}><strong>{n ? '候选' : '基准'} · {e.run.id}</strong><dl><dt>冻结工程</dt><dd>{e.project.id} r{e.project.revision}</dd><dt>冻结方案</dt><dd>{e.scenario.id} r{e.scenario.revision}</dd><dt>请求窗口</dt><dd>{e.run.window.start_ms}—{e.run.window.end_ms} ms</dd><dt>计算边界</dt><dd>{e.run.computed_until_ms ?? '未知'} ms · {e.run.whole_window_computed ? '完整窗口' : '仅已计算前缀'}</dd></dl><button onClick={() => onInspectRun(e.run.id)}>打开该Run回放</button><IssueList issues={e.run.limitations} /></article>)}</div>
    {failure?.key === key && <><p role="alert" className="delivery-error">{failure.text}</p>{failure.detail != null && <JsonDetails label="比较服务原始错误与字段定位" value={failure.detail} />}</>}
    {value && <ComparisonResult value={value} evidence={[]} onLocate={onLocate} embedded/>}
    <section><h3>从基准冻结输入创建可修改方案</h3><p>复制基准Run的方案初始输入，在当前工程新建方案；不覆盖旧Run，不使用运行末态。当前工程若已修改，后续比较须列出这些变化；此操作不恢复旧工程。改变供料或排期后可重新运行并回到此处比较。</p><div className="delivery-fields"><label>新方案ID<input value={newId} onChange={e => setNewId(e.target.value)} /></label><label>新方案标题<input value={title} onChange={e => setTitle(e.target.value)} /></label></div><button disabled={busy || pending || !services.cloneScenario || !evidence[0] || !newId.trim() || !title.trim()} onClick={() => void fork()}>创建方案草稿</button>{!services.cloneScenario && <p className="delivery-hint">宿主尚未接入冻结输入创建方案能力，此操作不可用。</p>}{created?.key === key && <p role="status">{created.text}</p>}</section>
  </section>;
}
