import { useEffect, useId, useRef, useState } from 'react';
import { FactoryScene } from '../../scene/FactoryScene';
import type { LodMode } from '../../scene/FactoryScene';
import { cloudErrorCode, formatQuantity } from '../../services/cloudProduction';
import { factoryDeviceTitle, factoryItemPlace, factoryItemTitle, factoryLifecycleLabel, factoryQualityLabel, factoryUnitLabel } from '../../scene/factoryLabels';
import type { CloudContext } from '../../services/cloudProduction';
import {
  createExecutionPresentation, executionBaseKey, executionObjectRef, executionSourceKey,
  readCompleteExecutionHistory, readCompleteExecutionObservation, readExecutionFrozenSource,
} from '../../services/executionPresentation';
import type { CompleteExecutionHistory, CompleteExecutionObservation, ExecutionAssetPort,
  ExecutionFrozenSource, ExecutionObservationSelection } from '../../services/executionPresentation';
import type { FactoryPresentationSource } from '../../services/factoryPresentation';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { ExecutionObservedObject } from '../../../../../packages/commands/generated/typescript/ExecutionObservedObject';
import type { ExecutionObservedObjectRef } from '../../../../../packages/commands/generated/typescript/ExecutionObservedObjectRef';
import type { LogicalWorkViewState } from '../../../../../packages/commands/generated/typescript/LogicalWorkViewState';
import type { BoundExecutionJobSnapshotState } from '../../../../../packages/commands/generated/typescript/BoundExecutionJobSnapshotState';
import type { DispatchExplanationReason } from '../../../../../packages/commands/generated/typescript/DispatchExplanationReason';
import './executionObservation.css';

export interface ExecutionObservationViewProps {
  context: CloudContext;
  /** Actual caller Native UI session, not an execution epoch or fabricated Run ID. */
  nativeSessionId: string;
  selection: ExecutionObservationSelection;
  readAsset?: ExecutionAssetPort;
  onClose: () => void;
  onLocateObject: (reference: ObjectRef | ExecutionObservedObjectRef) => void;
  onSelectionChange?: (selection: ExecutionObservationSelection) => void;
}
const kinds = [['item', '物品'], ['job', '加工记录'], ['device', '设备'], ['logical_work', '生产任务'], ['attempt', '指令下发']] as const;
const label = (kind: ExecutionObservedObject['kind']) => kinds.find(k => k[0] === kind)![1];
const json = (value: unknown) => JSON.stringify(value, null, 2);
const workStates: Record<LogicalWorkViewState, string> = {
  queued: '等待安排', reserved: '已预留设备', published: '已发送指令', device_accepted: '设备已接单',
  device_started: '设备已开工', device_completed: '设备已报告完成', business_effect_committed: '生产结果已登记',
  output_wait: '等待出料', accepted: '结果已接收', reconciliation_required: '需要人工核对',
  cancelled_not_started: '开工前已取消', device_failed: '设备异常',
};
const jobStates: Record<BoundExecutionJobSnapshotState, string> = {
  paused: '已暂停', working: '加工中', waiting_output: '等待出料', completed: '加工完成', cancelled: '已取消', needs_resolution: '需要处理',
};
const dispatchReasons: Record<DispatchExplanationReason, string> = {
  earliest_estimated_completion: '预计完成更早', stable_binding_tiebreak: '同等条件下按固定顺序选择设备',
  waiting_for_eligible_binding: '等待可接单设备', reselect_after_confirmed_withdrawal: '原指令确认撤回后重新分派',
};
const seconds = (ms: number) => `${(ms / 1000).toLocaleString('zh-CN', { maximumFractionDigits: 3 })} 秒`;
const deviceTitle = (source: ExecutionFrozenSource | null, id: string) => {
  const device = source?.inputs.project.devices.find(d => d.id === id);
  return device ? factoryDeviceTitle(device.title) : '设备名称未记录';
};
function objectTitle(object: ExecutionObservedObject, source: ExecutionFrozenSource | null) {
  switch (object.kind) {
    case 'item': return factoryItemTitle(source?.inputs.project ?? null, object.data);
    case 'device': return deviceTitle(source, object.data.binding.device_id);
    case 'job': return object.data.execution_ref.kind === 'project_device'
      ? `${deviceTitle(source, object.data.execution_ref.data.device_id)}加工记录` : '独立加工记录';
    case 'attempt': return `${deviceTitle(source, object.data.device_id)} · 第 ${object.data.attempt.attempt_ordinal} 次下发`;
    case 'logical_work': {
      const task = source?.inputs.scenario.production_tasks.find(t => t.id === object.data.production_task_id);
      const products = task && source ? source.inputs.project.products.filter(p => p.routing_ref.id === task.routing_ref.id && p.routing_ref.revision === task.routing_ref.revision) : [];
      const names = [...new Set(products.map(p => factoryItemTitle(source!.inputs.project, p)))];
      return names.length === 1 ? `${names[0]}生产任务` : '生产任务';
    }
  }
}
function detail(object: ExecutionObservedObject, source: ExecutionFrozenSource | null) {
  switch (object.kind) {
    case 'item': return `${formatQuantity({ ...object.data.quantity, unit: factoryUnitLabel(object.data.quantity.unit) })} · ${factoryLifecycleLabel[object.data.lifecycle]} · ${factoryQualityLabel[object.data.quality]} · ${factoryItemPlace(source?.inputs.project ?? null, object.data)}`;
    case 'job': return `${jobStates[object.data.state]} · 开始于 ${seconds(object.data.started_at_host_ms)} · ${object.data.finished_at_host_ms === null ? '尚无完成记录' : `完成于 ${seconds(object.data.finished_at_host_ms)}`} · 关联 ${object.data.input_refs.length} 条来料记录`;
    case 'device': {
      const state = object.data.latest_observation?.observation.device_state;
      const reported = state ? { idle: '空闲', busy: '忙碌', failed: '异常' }[state] : '尚未上报';
      const fresh = { fresh: '状态报告有效', stale: '状态报告已过期', unknown: '尚无状态报告' }[object.data.freshness];
      const acceptance = object.data.effective_accept_new_work === null ? '能否接单尚未确认' : object.data.effective_accept_new_work ? '当前可接单' : '当前不可接单';
      return `${reported} · ${fresh} · 接单开关${object.data.operator_accept_new_work ? '已开启' : '已关闭'} · ${acceptance}`;
    }
    case 'logical_work': return `${workStates[object.data.state]} · ${object.data.latest_dispatch_decision ? dispatchReasons[object.data.latest_dispatch_decision.reason] : '尚无设备分派记录'}`;
    case 'attempt': return `${workStates[object.data.state]} · 已收到 ${object.data.receipt_ids.length} 条回执`;
  }
}
function readSeconds(raw: string) {
  const ms = Math.round(Number(raw) * 1000);
  if (!/^\d+(?:\.\d{1,3})?$/.test(raw) || !Number.isSafeInteger(ms)) throw new Error('请输入不小于 0 的秒数，最多保留三位小数。');
  return ms;
}

export function ExecutionObservationView(props: ExecutionObservationViewProps) {
  const { context, nativeSessionId, selection } = props;
  const baseKey = executionBaseKey(context, nativeSessionId, selection.identity, selection.execution_revision, selection.read_origin);
  const activeKey = useRef(baseKey); activeKey.current = baseKey;
  const propsRef = useRef(props); propsRef.current = props;
  const inputTicket = useRef<AbortController | null>(null), objectTicket = useRef<AbortController | null>(null), historyTicket = useRef<AbortController | null>(null);
  const [frozen, setFrozen] = useState<{ key: string; value: ExecutionFrozenSource } | null>(null);
  const [at, setAt] = useState(selection.observed_until_host_ms), [timeDraft, setTimeDraft] = useState(String(at / 1000));
  const [observation, setObservation] = useState<{ key: string; base_key: string; value: CompleteExecutionObservation; scene: FactoryPresentationSource } | null>(null);
  const [history, setHistory] = useState<{ key: string; value: CompleteExecutionHistory } | null>(null);
  const [range, setRange] = useState<{ from: number; until: number } | null>(null), [rangeFrom, setRangeFrom] = useState('0'), [rangeUntil, setRangeUntil] = useState('');
  const [error, setError] = useState(''), [loading, setLoading] = useState<string[]>([]);
  const [mediaNotice, setMediaNotice] = useState('');
  const [canvas, setCanvas] = useState<'line_canvas' | 'multi_hall_canvas'>('line_canvas');
  const [hallId, setHallId] = useState(''), [lod, setLod] = useState<LodMode | 'auto'>('auto');
  const [sceneSelection, setSceneSelection] = useState<ObjectRef | null>(null), [reset, setReset] = useState(0);
  const [kind, setKind] = useState('all'), [objectId, setObjectId] = useState('');
  const id = useId();
  const fixed = frozen?.key === baseKey ? frozen.value : null;
  const targetKey = fixed ? executionSourceKey(fixed, nativeSessionId, at) : '';
  const targetRef = useRef(targetKey); targetRef.current = targetKey;
  const current = observation?.key === targetKey ? observation : null;
  const sceneObservation = observation?.base_key === baseKey ? observation : null;
  const historyKey = range && fixed ? JSON.stringify([executionSourceKey(fixed, nativeSessionId, range.until), range.from, range.until]) : '';
  const rangeRef = useRef(historyKey); rangeRef.current = historyKey;
  const currentHistory = history?.key === historyKey ? history.value : null;
  const front = fixed?.envelope.snapshot.coverage.observed_until_host_ms ?? 0;

  function busy(name: string, value: boolean) { setLoading(old => value ? [...old.filter(v => v !== name), name] : old.filter(v => v !== name)); }
  function showError(reason: unknown, stillCurrent: () => boolean) {
    if (!stillCurrent()) return;
    const code = cloudErrorCode(reason);
    if (code === 'PermissionDenied' || code === 'NoActiveProject') {
      inputTicket.current?.abort(); objectTicket.current?.abort(); historyTicket.current?.abort();
      setFrozen(null); setObservation(null); setHistory(null); setLoading([]); setMediaNotice('');
    }
    setError(reason instanceof Error ? reason.message : String(reason));
  }
  useEffect(() => {
    const controller = new AbortController(); inputTicket.current = controller;
    const live = () => activeKey.current === baseKey && !controller.signal.aborted;
    setFrozen(null); setObservation(null); setHistory(null); setRange(null); setSceneSelection(null); setError(''); setMediaNotice(''); setLoading(['工厂与方案']);
    setAt(selection.observed_until_host_ms); setTimeDraft(String(selection.observed_until_host_ms / 1000)); setKind('all'); setObjectId('');
    void readExecutionFrozenSource(context, selection.identity, selection.execution_revision, selection.read_origin, { signal: controller.signal }).then(value => {
      if (!live()) return;
      setFrozen({ key: baseKey, value }); setHallId(value.inputs.project.halls[0]?.id ?? '');
      const until = value.envelope.snapshot.coverage.observed_until_host_ms;
      setRange({ from: 0, until }); setRangeFrom('0'); setRangeUntil(String(until / 1000));
    }).catch(e => showError(e, live)).finally(() => { if (live()) busy('工厂与方案', false); });
    return () => { controller.abort(); objectTicket.current?.abort(); historyTicket.current?.abort(); };
  }, [baseKey]);
  useEffect(() => {
    setAt(selection.observed_until_host_ms); setTimeDraft(String(selection.observed_until_host_ms / 1000));
  }, [baseKey, selection.observed_until_host_ms]);
  useEffect(() => {
    if (!fixed) return;
    const controller = new AbortController(); objectTicket.current = controller;
    const expected = targetKey;
    const live = () => activeKey.current === baseKey && targetRef.current === expected && !controller.signal.aborted;
    const options = { signal: controller.signal, assertCurrent: () => { if (!live()) throw new DOMException('选择已变更', 'AbortError'); } };
    busy('所选时点', true); setError(''); setMediaNotice('');
    void (async () => {
      const value = await readCompleteExecutionObservation(fixed, at, options);
      const port = propsRef.current.readAsset;
      const protectedPort: ExecutionAssetPort | undefined = port ? async request => {
        try { return await port(request); }
        catch (reason) {
          if (cloudErrorCode(reason) === 'MissingReference') {
            if (live()) setMediaNotice(`所选来源的冻结模型在本机不可用；已读取的对象事实和历史仍可查看。${reason instanceof Error ? reason.message : String(reason)}`);
          } else showError(reason, live);
          throw reason;
        }
      } : undefined;
      const scene = await createExecutionPresentation(fixed, value, nativeSessionId, protectedPort, options);
      if (live()) setObservation({ key: expected, base_key: baseKey, value, scene });
    })().catch(e => showError(e, live)).finally(() => { if (live()) busy('所选时点', false); });
    return () => controller.abort();
  }, [baseKey, targetKey, !!props.readAsset]);
  useEffect(() => {
    if (!fixed || !range) return;
    const controller = new AbortController(); historyTicket.current = controller;
    const expected = historyKey;
    const live = () => activeKey.current === baseKey && rangeRef.current === expected && !controller.signal.aborted;
    setHistory(null); busy('过程记录', true);
    void readCompleteExecutionHistory(fixed, range.from, range.until, { signal: controller.signal }).then(value => {
      if (live()) setHistory({ key: expected, value });
    }).catch(e => showError(e, live)).finally(() => { if (live()) busy('过程记录', false); });
    return () => controller.abort();
  }, [baseKey, fixed, historyKey]);
  function seek(next: number) {
    if (!fixed || next > front || next < 0) { setError('这个时间尚无记录，请选择可回看的时间范围。'); return; }
    setError(''); setAt(next); setTimeDraft(String(next / 1000)); props.onSelectionChange?.({ ...selection, observed_until_host_ms: next });
  }
  function locate(reference: ObjectRef | ExecutionObservedObjectRef) {
    if (reference.kind === 'item' || reference.kind === 'device' || reference.kind === 'job') setSceneSelection({ kind: reference.kind, id: reference.id });
    props.onLocateObject(reference);
  }
  const accepts = (object: ExecutionObservedObject) => (kind === 'all' || object.kind === kind) && (!objectId || executionObjectRef(object).id === objectId);
  const objects = current?.value.objects.filter(accepts) ?? [], records = currentHistory?.records.filter(r => accepts(r.object)) ?? [];
  const readOnlyReason = !fixed ? '正在确认记录来源和查看权限。'
    : selection.read_origin === 'cloud' ? '这里读取云端存档，只能回看。生产控制由持有会话的一方负责。'
    : fixed.envelope.access.ownership === 'current_host_owner' ? '记录由当前本机会话维护。本页仅供回看，生产操作请返回“云端生产”。'
    : fixed.envelope.access.ownership === 'other_host' ? '当前本机会话不是这条记录的控制方，只能回看。'
    : '尚未确认控制权限，本页只显示已读取的记录。';
  return <section className="execution-observation" aria-label="执行只读观察工作区">
    <header className="execution-view-heading">
      <div><h2>执行观察</h2><span>{fixed ? `${fixed.envelope.snapshot.provenance.mode === 'physical_device' ? '设备上报数据' : '软件设备演示'} · 方案：${fixed.inputs.scenario.title}` : '正在读取生产方案'}</span></div>
      <button type="button" onClick={() => { inputTicket.current?.abort(); objectTicket.current?.abort(); historyTicket.current?.abort(); props.onClose(); }}>返回云端生产</button>
    </header>
    <div className="execution-view-source" aria-label="执行观察来源">
      <dl className="execution-source-facts">
        <div><dt>数据来源</dt><dd>{selection.read_origin === 'cloud' ? '云端保存的执行记录' : '本机会话的执行记录'}</dd></div>
        <div><dt>生产记录</dt><dd>第 {selection.execution_revision} 版</dd></div>
        <div><dt>观察时间</dt><dd>{seconds(at)}</dd></div>
        <div><dt>可回看至</dt><dd>{fixed ? seconds(front) : '正在读取'}</dd></div>
      </dl>
      <p className="execution-read-only"><strong>仅供回看</strong>{readOnlyReason}</p>
      <details className="execution-source-technical"><summary>技术详情</summary>
        <pre>{json({ read_origin: selection.read_origin, native_host_instance_id: context.hostContextId,
          native_ui_session_id: nativeSessionId, ...selection.identity, execution_revision: selection.execution_revision,
          observation_frontier: fixed?.envelope.snapshot.observation_frontier ?? null, at_host_ms: at,
          clock_started: fixed?.envelope.snapshot.coverage.clock_started ?? null,
          observed_until_host_ms: fixed ? front : null, access: fixed?.envelope.access ?? null,
          delivery: fixed?.envelope.delivery ?? null, input_ref: fixed?.inputs.project_input_ref ?? null,
          scenario_title: fixed?.inputs.scenario.title ?? null,
          object_pages: current?.value.pages ?? null, history_pages: currentHistory?.pages ?? null })}</pre>
        <button type="button" onClick={() => { void navigator.clipboard.writeText(json({ read_origin: selection.read_origin, identity: selection.identity, execution_revision: selection.execution_revision, observed_until_host_ms: at })).catch(e => setError(String(e))); }}>复制完整来源</button>
      </details>
    </div>
    {error && <p role="alert" className="execution-view-error">无法完整读取所选记录：{error}</p>}
    {loading.length > 0 && <p role="status" className="execution-view-loading">正在读取{loading.join('、')}，本页保持所选记录版本。</p>}
    <div className="execution-view-controls">
      <label>视角<select aria-label="观察视角" value={canvas} onChange={e => setCanvas(e.target.value as typeof canvas)}><option value="line_canvas">单厂房 / 流水线</option><option value="multi_hall_canvas">多厂房</option></select></label>
      <label>厂房<select aria-label="观察厂房" value={hallId} onChange={e => setHallId(e.target.value)}>{fixed?.inputs.project.halls.map(h => <option key={h.id} value={h.id}>{h.title}</option>)}</select></label>
      <label>模型精度<select aria-label="观察模型精度" value={lod} onChange={e => setLod(e.target.value as typeof lod)}><option value="auto">自动切换</option><option value="detailed">详细模型</option><option value="simplified">简化模型</option><option value="region">区域布局</option></select></label>
      <button type="button" disabled={!current} onClick={() => setReset(v => v + 1)}>重置视角</button>
      <form onSubmit={e => { e.preventDefault(); try { seek(readSeconds(timeDraft)); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } }}>
        <label>观察时间（秒）<input aria-label="观察时间（秒）" inputMode="decimal" value={timeDraft} onChange={e => setTimeDraft(e.target.value)}/></label>
        <button type="submit" disabled={!fixed}>读取时点</button>
      </form>
    </div>
    <div className="execution-view-clock">
      <input aria-label="执行观察时间游标" type="range" min={0} max={front} step={1} disabled={!fixed || !fixed.envelope.snapshot.coverage.clock_started} value={Math.min(at, front)} onChange={e => seek(Number(e.target.value))}/>
      <span>{fixed && !fixed.envelope.snapshot.coverage.clock_started ? '尚未开始生产，当前显示初始化记录。' : `观察时间 ${seconds(at)} · 拖动只回看记录`}</span>
    </div>
    {!props.readAsset && <p className="execution-view-notice">本机尚未接入这条记录的设备模型，可查看已保存的布局、对象状态和过程记录。</p>}
    {mediaNotice && <p className="execution-view-notice" role="status">{mediaNotice}</p>}
    <div className="execution-view-scene" aria-busy={!current}>{sceneObservation ? <>
      <FactoryScene key={baseKey} project={fixed!.inputs.project} canvas={canvas} hallId={hallId || null} selection={sceneSelection} onSelect={ref => { setSceneSelection(ref); if (ref) props.onLocateObject(ref); }} lod={lod === 'auto' ? 'simplified' : lod} autoLod={lod === 'auto'} resetKey={reset} mediaSource={sceneObservation.scene} run={null} observation={null}/>
      {!current && <p className="execution-view-loading execution-scene-pending">正在读取所选时点 · 当前画面 {seconds(sceneObservation.value.at_host_ms)}</p>}
    </> : <p className="execution-view-empty">{error ? '暂时无法显示所选记录的三维画面。' : '正在读取生产画面。'}</p>}</div>
    {current && <p className="execution-view-complete" role="status">当前时点共 {current.value.objects.length} 条对象记录。设备动作按上报阶段显示，物品位置按实际持有者显示；没有进度报告的设备保持静态。</p>}
    <div className="execution-view-filter">
      <label>对象类别<select aria-label="观察对象类别" value={kind} onChange={e => setKind(e.target.value)}><option value="all">全部对象</option>{kinds.map(([value, title]) => <option key={value} value={value}>{title}</option>)}</select></label>
      <details><summary>按编号精确查找</summary><label>对象编号<input aria-label="观察对象编号" value={objectId} onChange={e => setObjectId(e.target.value)}/></label></details>
      <button type="button" onClick={() => { setKind('all'); setObjectId(''); }}>清除筛选</button>
    </div>
    <div className="execution-view-table execution-object-table"><table>
      <caption>{seconds(at)} 时的对象状态</caption><thead><tr><th>对象</th><th>已记录的状态</th><th>详情</th></tr></thead>
      <tbody>{objects.map(object => { const ref = executionObjectRef(object); return <tr key={`${ref.kind}:${ref.id}`}>
        <td><button type="button" onClick={() => locate(ref)}>{objectTitle(object, fixed)}</button><small>{label(object.kind)}</small></td>
        <td>{detail(object, fixed)}</td><td><details><summary>技术详情</summary><pre>{json(object.data)}</pre></details></td>
      </tr>; })}</tbody>
    </table>{current && objects.length === 0 && <p>这个时间和筛选条件下没有对象记录。</p>}</div>
    <div className="execution-history-heading"><h3>生产过程记录</h3>
      <form onSubmit={e => { e.preventDefault(); try { const from = readSeconds(rangeFrom), until = readSeconds(rangeUntil); if (from > until || until > front) throw new Error('起点不能晚于终点，终点不能超过可回看的时间。'); setError(''); setRange({ from, until }); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } }}>
        <label>起点（秒）<input aria-label="历史起点（秒）" inputMode="decimal" value={rangeFrom} onChange={e => setRangeFrom(e.target.value)}/></label>
        <label>终点（秒）<input aria-label="历史终点（秒）" inputMode="decimal" value={rangeUntil} onChange={e => setRangeUntil(e.target.value)}/></label>
        <button type="submit" disabled={!fixed}>读取过程记录</button>
      </form>
    </div>
    <div className="execution-history-axis" aria-label="生产过程时间轴"><span>{seconds(range?.from ?? 0)}</span><div>{records.map(r => <button type="button" key={r.record_sequence} style={{ left: `${range && range.until !== range.from ? 100 * (r.at_host_ms - range.from) / (range.until - range.from) : 0}%` }} title={`第 ${r.record_sequence} 条 · ${seconds(r.at_host_ms)} · ${objectTitle(r.object, fixed)}`} aria-label={`跳到记录${r.record_sequence}`} onClick={() => seek(r.at_host_ms)}/>)}</div><span>{seconds(range?.until ?? 0)}</span></div>
    {currentHistory && <p className="execution-view-complete" role="status">已读取 {seconds(currentHistory.host_from_ms)} 至 {seconds(currentHistory.host_until_ms)} 的全部 {currentHistory.records.length} 条记录。同一时刻按原记录顺序排列，晚于当前观察时间的条目已标出。</p>}
    <div className="execution-view-table"><table id={`${id}-history`}><thead><tr><th>记录顺序</th><th>生产时间</th><th>对象与当时状态</th></tr></thead>
      <tbody>{records.map(r => { const ref = executionObjectRef(r.object); return <tr key={r.record_sequence} data-record-sequence={r.record_sequence} className={r.at_host_ms > at ? 'execution-history-future' : ''}>
        <td>第 {r.record_sequence} 条</td><td><button type="button" onClick={() => seek(r.at_host_ms)}>{seconds(r.at_host_ms)}</button>{r.at_host_ms > at && <small>晚于当前时点</small>}</td>
        <td><button type="button" onClick={() => locate(ref)}>{objectTitle(r.object, fixed)}</button><p>{detail(r.object, fixed)}</p><details><summary>技术详情</summary><pre>{json(r)}</pre></details></td>
      </tr>; })}</tbody>
    </table>{currentHistory && records.length === 0 && <p>这个时间范围和筛选条件下没有过程记录。</p>}{!currentHistory && <p>尚未读完整个时间范围的过程记录。</p>}</div>
  </section>;
}
