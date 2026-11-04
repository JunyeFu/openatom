import { invoke } from '@tauri-apps/api/core';
import { CloudDataExplorer } from './CloudDataExplorer';
import { useEffect, useId, useRef, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { CapabilityCatalog } from '../../../../../packages/commands/generated/typescript/CapabilityCatalog';
import type { CloudProductionConfiguration } from '../../../../../packages/commands/generated/typescript/CloudProductionConfiguration';
import type { ExecutionSourceSummary } from '../../../../../packages/commands/generated/typescript/ExecutionSourceSummary';
import type { PredictionRunReference } from '../../../../../packages/commands/generated/typescript/PredictionRunReference';
import type { CommandReadEnvelope } from '../../../../../packages/commands/generated/typescript/CommandReadEnvelope';
import type { CommandReceiptsPage } from '../../../../../packages/commands/generated/typescript/CommandReceiptsPage';
import type { SnapshotSelection } from '../../../../../packages/commands/generated/typescript/SnapshotSelection';
import type { ApiRequest } from '../../../../../packages/commands/generated/typescript/ApiRequest';
import { factoryDeviceTitle, factoryUnitLabel } from '../../scene/factoryLabels';
import { cloudTimeLabel, executionQueryEnd, localInputToUtc, localTimeZone, millisecondsToSeconds, secondsToMilliseconds, utcToLocalInput } from '../../services/cloudFormValues';
import { readExecutionFrozenSource } from '../../services/executionPresentation';
import {
  EXECUTION_CONTRACT, CloudProductionError, canControl, checkUtcRange, cloudContextKey, cloudErrorCode, controlExecution,
  exportCloudData, exportDirectoryUnavailable, exportExecutionReport, exportQuery, formatQuantity,
  hasCloudOperation, identityKey, recoverRegisteredCloudTask, parseQuantity, prepareExecution, readCloud,
  readExecution, readLocalView, validateQueryPage, requestSimulatedDeviceMaintenance,
} from '../../services/cloudProduction';
import type { CloudContext, CloudExportDirectories, CloudTaskRecord, ControlAction, DatasetDescriptor,
  DaaSQuery, DaaSQueryResult, CloudDataExportSelection, ExecutionIdentity, ExecutionReadEnvelope } from '../../services/cloudProduction';
import './cloudProduction.css';
import type { ExecutionObservationSelection } from '../../services/executionPresentation';

export type ExecutionViewSelection = ExecutionObservationSelection;
export interface CloudProductionPanelProps {
  project: ProjectState | null;
  scenario: ScenarioInput | null;
  capabilities: CapabilityCatalog;
  hostContextId: string;
  onClose: () => void;
  onLocateDevice: (deviceId: string) => void;
  onInspectExecution: (selection: ExecutionViewSelection) => void;
  /** Actual terminal Run references from Workbench, never manufactured by this panel. */
  predictionRuns?: readonly PredictionRunReference[];
  directories?: CloudExportDirectories;
  /** Root can return a retained request after unmount; this only resumes observation. */
  retainedTask?: CloudTaskRecord | null;
  onTaskRegistered?: (record: CloudTaskRecord) => void;
  onUnappliedInputChanged?: (dirty: boolean) => void;
  onRefreshProject?: () => void | Promise<void>;
}
type Tab = 'devices' | 'data' | 'execution';
type Lane = 'configuration' | 'datasets' | 'catalog' | 'source' | 'query' | 'command' | 'receipts' | 'mutation' | 'request';
interface Source { identity: ExecutionIdentity; origin: 'host' | 'cloud'; selection: SnapshotSelection }
interface QueryPage {
  query: DaaSQuery; result: DaaSQueryResult; pin: CloudDataExportSelection;
  lastPosition: number | null; number: number; seenRows: number;
}
const tabs: readonly [Tab, string][] = [['devices', '设备'], ['data', '数据'], ['execution', '执行']];
const labels: Record<string, string> = {
  prepared: '已准备，尚未启动', running: '运行中', dispatch_paused: '新派工已暂停', draining: '等待已准入工作收尾',
  needs_resolution: '待核对', completed: '已完成收尾', stopped: '已停止', failed: '失败',
  fresh: '状态有效', stale: '已过期', unknown: '尚未确认', satisfied: '已达到目标', not_satisfied: '未达到目标', unobserved: '尚无观察',
  idle: '空闲', busy: '忙碌', queued: '等待安排', reserved: '已预留设备', published: '已发送指令',
  device_accepted: '设备已接单', device_started: '设备已开工', device_completed: '设备已报告完成',
  business_effect_committed: '生产结果已登记', output_wait: '等待出料', accepted: '结果已接收',
  reconciliation_required: '需要人工核对', cancelled_not_started: '开工前已取消', device_failed: '设备异常',
};
const text = (v: unknown) => v === null || v === undefined ? '未知' : String(v);
const rawErrorText = (e: unknown) => e instanceof Error ? e.message : String(e);
const errorText = (e: unknown) => {
  const raw = rawErrorText(e);
  if (raw.startsWith('external.target:')) return '目标数量与方案的最终产出数量不一致。请核对投产任务和最后一道工序，中间工序不能重复计入目标。';
  if (raw.startsWith('external.policy:')) return '生产目标须为大于 0 的整数，截止时间也须大于 0；请检查目标、单位和生产方案。';
  if (raw.startsWith('external.unsupported:')) return '所选工艺还没有对应的外部执行支持，请检查本次工序和设备绑定。完整技术原因可在下方查看。';
  return raw;
};
const quantityText = (quantity: Parameters<typeof formatQuantity>[0]) => formatQuantity({ ...quantity, unit: factoryUnitLabel(quantity.unit) });
const elapsed = (ms: number) => `${millisecondsToSeconds(ms)} 秒`;
const sourceModeLabel = { software_in_loop: '软件设备演示', physical_device: '实物设备数据', generated_load: '生成测试数据' };
const purposeLabel = { execution: '生产设备上报', telemetry_only: '独立状态采集', generated_load: '生成测试数据' };
const blockingLabel: Record<string, string> = {
  operator_disabled: '调度已暂停新任务', fresh_observation_required: '尚无有效状态报告', device_boot_not_registered: '尚未确认设备启动',
  device_disabled_acceptance: '设备暂停接单',
  device_not_idle: '设备尚未空闲', original_attempt_unsettled: '原指令尚未收尾',
};
function Json({ value, label = '完整原始字段' }: { value: unknown; label?: string }) {
  return <details className="cloud-json"><summary>{label}</summary><pre>{JSON.stringify(value, null, 2)}</pre></details>;
}
function safeInteger(raw: string, label: string, positive = false) {
  if (!/^\d+$/.test(raw) || !Number.isSafeInteger(Number(raw)) || (positive && Number(raw) === 0)) throw new CloudProductionError('input', `${label}需为${positive ? '正' : '非负'}安全整数。`);
  return Number(raw);
}
function Notice({ children }: { children: ReactNode }) { return <p className="cloud-notice">{children}</p>; }

function QueryFactsTable({ page, deviceTitle, locationTitle }: {
  page: QueryPage; deviceTitle: (id: string, execution: boolean) => string; locationTitle: (id: string) => string;
}) {
  const names = { telemetry: '设备状态报告', generated_load: '生成测试记录', count_by_device_minute: '每分钟记录数',
    execution_attempt: '指令执行记录', acceptance: '产出接收', accepted_quantity: '接收数量合计', command_count: '指令状态计数', waiting_time: '已记录等待时长' };
  return <div className="cloud-table-scroll cloud-query-table"><table><thead><tr>
    <th>设备或对象</th><th>记录类型</th><th>时间</th><th>状态或数值</th><th>补充说明</th><th>详情</th>
  </tr></thead><tbody>{page.result.rows.map(row => {
    let object = '', when = '', fact = '', note = '';
    switch (row.kind) {
      case 'telemetry': {
        object = deviceTitle(row.observation.device_id, false); when = cloudTimeLabel(row.cloud_received_at);
        fact = labels[row.observation.device_state];
        note = row.observation.source_purpose === 'telemetry_only' ? '仅采集状态，不参与生产调度' : row.observation.accept_new_work ? '设备自报允许接新任务' : '设备自报暂停接新任务';
        break;
      }
      case 'generated_load': object = deviceTitle(row.event.device_ref, false); when = cloudTimeLabel(row.cloud_received_at);
        fact = String(row.event.value); note = '生成的状态样本；未提供单位，不计作生产产出'; break;
      case 'count_by_device_minute': object = deviceTitle(row.device_id, false); when = cloudTimeLabel(row.bucket_start);
        fact = `${row.count} 条`; note = sourceModeLabel[row.source_mode]; break;
      case 'execution_attempt': object = deviceTitle(row.attempt.device_id, true);
        when = row.attempt.started_at_host_ms === null ? '尚未开工' : `开始于 ${elapsed(row.attempt.started_at_host_ms)}`;
        fact = labels[row.attempt.state]; note = row.attempt.completed_at_host_ms === null ? '尚无完成回执' : `设备报告完成于 ${elapsed(row.attempt.completed_at_host_ms)}`; break;
      case 'acceptance': object = locationTitle(row.event.destination); when = elapsed(row.event.at_ms);
        fact = quantityText(row.event.quantity); note = `关联 ${row.event.item_refs.length} 条物品记录`; break;
      case 'accepted_quantity': object = '所选生产记录'; when = `${elapsed(row.host_from_ms)} 至 ${elapsed(row.host_until_ms)}（终点不含）`;
        fact = quantityText(row.quantity); note = '按这段时间内的接收事件统计'; break;
      case 'command_count': object = '所选生产记录'; when = '所选时间范围';
        fact = `${row.count} 条`; note = labels[row.state]; break;
      case 'waiting_time': object = '生产任务'; when = '所选时间范围';
        fact = row.known_host_ms === null ? '尚无可核算时长' : elapsed(row.known_host_ms);
        note = labels[row.reason] ?? '等待原因与缺失信息见技术详情'; break;
    }
    const position = 'ingress_id' in row ? row.ingress_id : row.row_index;
    return <tr key={`${row.kind}:${position}`}><td>{object}</td><td>{names[row.kind]}</td><td>{when}</td>
      <td><strong>{fact}</strong></td><td>{note}</td><td><Json value={row} label="技术详情"/></td></tr>;
  })}</tbody></table></div>;
}

/** Central workspace only; Root retains canvas, Host and Native ownership. */
export function CloudProductionPanel(props: CloudProductionPanelProps) {
  const { project, scenario, capabilities, hostContextId } = props;
  const context: CloudContext | null = project ? { projectId: project.id, projectRevision: project.revision, hostContextId } : null;
  const key = `${context ? cloudContextKey(context) : 'none'}:${scenario?.id ?? ''}:${scenario?.revision ?? ''}`;
  const liveKey = useRef(key); liveKey.current = key;
  const propsRef = useRef(props); propsRef.current = props;
  const lanes = useRef(new Map<Lane, AbortController>());
  const mounted = useRef(true);
  const selectedSource = useRef<Source | null>(null);
  const sourceChoiceMade = useRef(false);
  const sourceGeneration = useRef(0);
  const previousFormOwner = useRef('');
  const sourcePollingPaused = useRef(false);
  const viewTimeEdited = useRef(false);
  const [tab, setTab] = useState<Tab>('devices');
  const [config, setConfig] = useState<CloudProductionConfiguration | null>(null);
  const [configStatus, setConfigStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [datasets, setDatasets] = useState<DatasetDescriptor[]>([]);
  const [catalog, setCatalog] = useState<ExecutionSourceSummary[]>([]);
  const [catalogStatus, setCatalogStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [envelope, setEnvelope] = useState<ExecutionReadEnvelope | null>(null);
  const [source, setSource] = useState<Source | null>(null);
  const [busy, setBusy] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [errorDetail, setErrorDetail] = useState('');
  const [frozenNames, setFrozenNames] = useState<{ key: string; project: ProjectState; scenario: ScenarioInput } | null>(null);
  const [message, setMessage] = useState('');
  const [taskRecord, setTaskRecord] = useState<CloudTaskRecord | null>(null);
  const [target, setTarget] = useState(''); const [unit, setUnit] = useState('');
  const [cutoffSeconds, setCutoffSeconds] = useState(''); const [maxAgeSeconds, setMaxAgeSeconds] = useState('');
  const [bindings, setBindings] = useState<string[]>([]);
  const [dirty, setDirty] = useState(false);
  const [dataSet, setDataSet] = useState(''); const [deviceFilter, setDeviceFilter] = useState<string[]>([]);
  const [knownDeviceSet, setKnownDeviceSet] = useState<{ dataset: string; ids: string[] } | null>(null);
  const [utcFrom, setUtcFrom] = useState(''); const [utcUntil, setUtcUntil] = useState('');
  const [fromSeconds, setFromSeconds] = useState('0'); const [untilSeconds, setUntilSeconds] = useState('');
  const [aggregation, setAggregation] = useState('rows'); const [page, setPage] = useState<QueryPage | null>(null);
  const [command, setCommand] = useState<CommandReadEnvelope | null>(null);
  const [receipts, setReceipts] = useState<CommandReceiptsPage | null>(null);
  const [receiptPages, setReceiptPages] = useState(0);
  const [viewSeconds, setViewSeconds] = useState(''); const [prediction, setPrediction] = useState('');
  const [revisionDraft, setRevisionDraft] = useState('');
  const id = useId();
  const available = (op: ApiRequest['operation']) => hasCloudOperation(capabilities, op);
  const currentLatest = source?.origin === 'host' && source.selection.kind === 'latest' && envelope?.access.ownership === 'current_host_owner';
  const historyReadOnly = !!source && (source.origin !== 'host' || source.selection.kind !== 'latest'
    || (!!envelope && envelope.access.ownership !== 'current_host_owner'));
  const canPresentProduction = !source || currentLatest;
  const snapshot = envelope?.snapshot ?? null;
  const frozenNameKey = snapshot ? JSON.stringify([identityKey(snapshot.identity), snapshot.project_input_ref]) : '';
  const currentInputsMatch = !snapshot || project?.revision === snapshot.project_input_ref.project_revision
    && scenario?.id === snapshot.project_input_ref.scenario_id && scenario.revision === snapshot.project_input_ref.scenario_revision;
  const executionProject = currentInputsMatch ? project : frozenNames?.key === frozenNameKey ? frozenNames.project : null;
  const executionScenario = currentInputsMatch ? scenario : frozenNames?.key === frozenNameKey ? frozenNames.scenario : null;
  const deviceName = (id: string, execution = false) => {
    const source = execution ? executionProject : project;
    const device = source?.devices.find(d => d.id === id);
    return device ? factoryDeviceTitle(device.title) : source?.resources.find(r => r.id === id)?.title ?? '设备名称未记录';
  };
  const locationName = (id: string) => executionProject?.locations.find(l => l.id === id)?.title ?? '位置名称未记录';
  const mutationBusy = busy.includes('mutation');
  const unresolvedTask = !!taskRecord && !['completed', 'failed', 'cancelled', 'rejected'].includes(taskRecord.status);

  function showError(reason: unknown) { const value = errorText(reason); setError(value); setErrorDetail(value === rawErrorText(reason) ? '' : rawErrorText(reason)); }

  function begin(lane: Lane) {
    lanes.current.get(lane)?.abort();
    const controller = new AbortController(); lanes.current.set(lane, controller);
    const captured = key;
    const live = () => mounted.current && liveKey.current === captured && lanes.current.get(lane) === controller && !controller.signal.aborted;
    setBusy(old => old.includes(lane) ? old : [...old, lane]);
    return { controller, live, context, lane };
  }
  function finish(lease: ReturnType<typeof begin>) {
    if (lease.live()) { lanes.current.delete(lease.lane); setBusy(old => old.filter(v => v !== lease.lane)); }
  }
  function fail(lease: ReturnType<typeof begin>, e: unknown) {
    if (!lease.live()) return;
    if (lease.lane === 'source') sourcePollingPaused.current = true;
    if (lease.lane === 'configuration') setConfigStatus('error');
    if (lease.lane === 'catalog') setCatalogStatus('error');
    const code = cloudErrorCode(e);
    if (code === 'PermissionDenied' || code === 'NoActiveProject') {
      setConfig(null); setDatasets([]); setCatalog([]); setEnvelope(null); setSource(null); setPage(null); setCommand(null); setReceipts(null); setFrozenNames(null); setKnownDeviceSet(null); setDeviceFilter([]);
      setConfigStatus('error'); setCatalogStatus('error');
      selectedSource.current = null; sourceGeneration.current += 1;
      // Permission rejection clears protected views rather than disguising it as an empty query.
      for (const [lane, c] of lanes.current) if (lane !== 'mutation') c.abort();
      setBusy(old => old.filter(lane => lane === 'mutation'));
    }
    showError(e);
  }
  function abortReads() {
    for (const [lane, controller] of lanes.current) if (lane !== 'mutation') { controller.abort(); lanes.current.delete(lane); }
    setBusy(old => old.filter(v => v === 'mutation'));
  }
  function tracked(lease: ReturnType<typeof begin>) {
    return { signal: lease.controller.signal, onTask: (record: CloudTaskRecord) => {
      // Original registrations survive view departure. No stop/cancel mutation is sent here.
      propsRef.current.onTaskRegistered?.(record);
      if (lease.live() && record.operation !== 'read_cloud_production') setTaskRecord(record);
    } };
  }
  function changeForm(apply: () => void) { apply(); setDirty(true); props.onUnappliedInputChanged?.(true); }
  function invalidateQuery(apply: () => void) {
    lanes.current.get('query')?.abort(); lanes.current.delete('query');
    setBusy(old => old.filter(v => v !== 'query')); setPage(null); setMessage(''); apply();
  }
  async function configuration() {
    if (!context || !scenario || !available('read_cloud_production')) return;
    const lease = begin('configuration');
    setConfigStatus('loading'); setConfig(null); setDatasets([]);
    try {
      const r = await readCloud(context, { kind: 'configuration', base_revision: context.projectRevision, scenario_id: scenario.id, scenario_revision: scenario.revision }, tracked(lease));
      if (lease.live()) {
        if (r.kind !== 'configuration') throw new CloudProductionError('configuration_mismatch', '未取得当前工程的连接配置。');
        if (r.data.project_revision !== context.projectRevision || r.data.scenario_id !== scenario.id || r.data.scenario_revision !== scenario.revision) throw new CloudProductionError('configuration_mismatch', '配置不属于所选工程/方案修订。');
        setConfig(r.data); setConfigStatus('ready'); setBindings(old => old.filter(b => r.data.candidate_bindings.some(c => c.binding_id === b)));
        if (r.data.cloud_connection_ref && r.data.cloud_project_id) void datasetCatalog(r.data);
        else setDatasets([]);
      }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  async function datasetCatalog(configuration = config) {
    if (!context || !available('read_cloud_production') || !configuration?.cloud_connection_ref || !configuration.cloud_project_id) return;
    const lease = begin('datasets');
    try {
      const r = await readCloud(context, { kind: 'dataset_catalog' }, tracked(lease));
      if (lease.live() && r.kind === 'dataset_catalog') setDatasets(r.data.datasets);
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  async function localCatalog() {
    if (!context || !available('read_execution_view')) return;
    const lease = begin('catalog');
    setCatalogStatus('loading'); setCatalog([]);
    try {
      const r = await readLocalView(context, { kind: 'catalog' });
      if (lease.live()) {
        if (r.kind !== 'catalog') throw new CloudProductionError('catalog_mismatch', '未取得当前工程的生产记录列表。');
        setCatalogStatus('ready');
        setCatalog(r.data.entries);
        const first = r.data.entries.find(e => e.active_owner) ?? r.data.entries[0];
        if (!selectedSource.current && !sourceChoiceMade.current && first) await chooseSource({ identity: first.identity, origin: 'host', selection: first.active_owner ? { kind: 'latest' } : { kind: 'revision', execution_revision: first.execution_revision } });
      }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  async function chooseSource(next: Source) {
    sourcePollingPaused.current = false; viewTimeEdited.current = false;
    selectedSource.current = next; sourceChoiceMade.current = true; sourceGeneration.current += 1;
    setSource(next); setEnvelope(null); setCommand(null); setReceipts(null); setReceiptPages(0); setPage(null);
    for (const lane of ['query', 'command', 'receipts'] as const) { lanes.current.get(lane)?.abort(); lanes.current.delete(lane); }
    setBusy(old => old.filter(v => !['query', 'command', 'receipts'].includes(v)));
    await refreshSource(next);
  }
  function clearSource() {
    selectedSource.current = null; sourceChoiceMade.current = true; sourceGeneration.current += 1;
    sourcePollingPaused.current = false; viewTimeEdited.current = false;
    for (const lane of ['source', 'query', 'command', 'receipts'] as const) { lanes.current.get(lane)?.abort(); lanes.current.delete(lane); }
    setBusy(old => old.filter(v => !['source', 'query', 'command', 'receipts'].includes(v)));
    setSource(null); setEnvelope(null); setFrozenNames(null); setCommand(null); setReceipts(null); setReceiptPages(0); setPage(null);
    setDataSet(old => old === '@execution' ? '' : old);
  }
  async function refreshSource(next = selectedSource.current, background = false) {
    if (!context || !next) return;
    if (background && sourcePollingPaused.current) return;
    if (!background) sourcePollingPaused.current = false;
    if (next.origin === 'host' && !available('get_execution_snapshot')) return;
    if (next.origin === 'cloud' && !available('read_cloud_production')) return;
    const lease = begin('source');
    try {
      let value: ExecutionReadEnvelope;
      if (next.origin === 'host') value = await readExecution(context, next.identity, next.selection);
      else {
        const r = await readCloud(context, { kind: 'snapshot', identity: next.identity, selection: next.selection }, tracked(lease));
        if (r.kind !== 'snapshot') throw new CloudProductionError('source_mismatch', '云端未返回所选快照。');
        value = r.data;
      }
      if (project?.revision !== value.snapshot.project_input_ref.project_revision
        || scenario?.id !== value.snapshot.project_input_ref.scenario_id || scenario?.revision !== value.snapshot.project_input_ref.scenario_revision) {
        const frozen = await readExecutionFrozenSource(context, next.identity, value.snapshot.execution_revision, next.origin, {
          signal: lease.controller.signal, assertCurrent: () => { if (!lease.live()) throw new DOMException('选择已变更', 'AbortError'); },
        });
        if (lease.live()) setFrozenNames({ key: JSON.stringify([identityKey(value.snapshot.identity), value.snapshot.project_input_ref]),
          project: frozen.inputs.project, scenario: frozen.inputs.scenario });
      }
      if (lease.live()) {
        if (identityKey(value.snapshot.identity) !== identityKey(next.identity) || value.delivery.read_origin !== next.origin
          || (next.selection.kind === 'revision' && value.snapshot.execution_revision !== next.selection.execution_revision)) throw new CloudProductionError('source_mismatch', '读取结果与当前固定来源不符。');
        setEnvelope(value);
        if (!viewTimeEdited.current) setViewSeconds(millisecondsToSeconds(value.snapshot.coverage.observed_until_host_ms));
        setUntilSeconds(old => old || millisecondsToSeconds(executionQueryEnd(value.snapshot.coverage.observed_until_host_ms)));
      }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  useEffect(() => {
    mounted.current = true;
    sourcePollingPaused.current = false; viewTimeEdited.current = false;
    setError(''); setErrorDetail(''); setMessage(''); setConfig(null); setConfigStatus('loading'); setDatasets([]); setCatalog([]); setCatalogStatus('loading'); setEnvelope(null); setSource(null); setPage(null); setTaskRecord(null); setFrozenNames(null); setKnownDeviceSet(null);
    setCommand(null); setReceipts(null); setBusy([]); selectedSource.current = null; sourceChoiceMade.current = false; sourceGeneration.current += 1;
    const owner = `${hostContextId}:${project?.id ?? ''}:${scenario?.id ?? ''}`;
    if (previousFormOwner.current !== owner) {
      previousFormOwner.current = owner; setTarget(''); setUnit(''); setCutoffSeconds(''); setMaxAgeSeconds(''); setBindings([]); setDirty(false);
      propsRef.current.onUnappliedInputChanged?.(false); setDataSet(''); setDeviceFilter([]); setUtcFrom(''); setUtcUntil(''); setFromSeconds('0'); setUntilSeconds(''); setPrediction('');
    }
    void configuration(); void localCatalog();
    return () => { mounted.current = false; for (const c of lanes.current.values()) c.abort(); lanes.current.clear(); };
    // Same-owner revision changes reload sources but preserve conflict form input.
  }, [key]);
  useEffect(() => {
    if (context && props.retainedTask && cloudContextKey(props.retainedTask.context) === cloudContextKey(context)) setTaskRecord(props.retainedTask);
  }, [key, props.retainedTask]);
  useEffect(() => {
    if (!source || !currentLatest || tab === 'data') return;
    if (envelope && ['completed', 'stopped', 'failed'].includes(envelope.snapshot.lifecycle.state) && envelope.delivery.relation === 'synced') return;
    const timer = window.setInterval(() => {
      const current = selectedSource.current;
      if (current?.selection.kind === 'latest' && !lanes.current.has('source') && !lanes.current.has('mutation') && !sourcePollingPaused.current) {
        void refreshSource(current, true);
      }
    }, 500);
    return () => window.clearInterval(timer);
    // Only latest device/execution views follow. Query pins and historical R/T stay fixed.
  }, [key, source, currentLatest, tab, envelope?.snapshot.lifecycle.state, envelope?.delivery.relation]);

  async function prepare() {
    if (!context || !scenario || !config || mutationBusy || unresolvedTask || (source && !currentLatest)) return;
    const lease = begin('mutation'); const generation = sourceGeneration.current; setError(''); setMessage('');
    try {
      const chosen = config.candidate_bindings.filter(b => bindings.includes(b.binding_id));
      if (!config.cloud_connection_ref || !config.cloud_project_id) throw new CloudProductionError('not_configured', '需有效本机云配置。');
      if (chosen.length < 2) throw new CloudProductionError('input', '请选择覆盖生产与运输阶段的候选设备。');
      const result = await prepareExecution(context, { execution_contract_version: EXECUTION_CONTRACT, base_revision: context.projectRevision,
        scenario_id: scenario.id, scenario_revision: scenario.revision, candidate_bindings: chosen,
        policy: { clock_mode: 'host_monotonic', report_cutoff_ms: secondsToMilliseconds(cutoffSeconds, '停止新派工时间', true), stop_new_dispatch_on_cutoff: true,
          finish_policy: 'drain_admitted_then_finish', target_quantity: parseQuantity(target, unit), source_prediction_window: { start_ms: 0, end_ms: scenario.window.end_ms },
          observation_max_age_ms: secondsToMilliseconds(maxAgeSeconds, '状态有效期', true) } }, tracked(lease));
      if (lease.live()) {
        setDirty(false); propsRef.current.onUnappliedInputChanged?.(false); setMessage('生产已准备好，点击“启动”后才会开始分派任务。');
        await propsRef.current.onRefreshProject?.();
        if (sourceGeneration.current === generation) await chooseSource({ identity: result.identity, origin: 'host', selection: { kind: 'latest' } });
        void localCatalog();
      }
    } catch (e) {
      fail(lease, e);
      if (lease.live() && cloudErrorCode(e) === 'RevisionConflict') {
        void refreshSource(); void configuration(); void propsRef.current.onRefreshProject?.();
      }
    } finally { finish(lease); }
  }
  async function control(action: ControlAction) {
    if (!context || !envelope || mutationBusy || !currentLatest) return;
    const lease = begin('mutation'); const original = envelope; const generation = sourceGeneration.current; setError('');
    try {
      await controlExecution(context, original, action, currentLatest);
      if (lease.live()) { setMessage(({ run: '已启动生产。', pause_dispatch: '已暂停新任务分派，正在加工的任务继续收尾。', resume_dispatch: '已恢复新任务分派。', stop_dispatch: '已停止新任务分派，正在等待已准入任务收尾。', request_reconciliation: '已请求核对原指令。', set_device_acceptance: '已更新设备接单设置。' })[action.kind]); if (generation === sourceGeneration.current) await refreshSource(); }
    } catch (e) { fail(lease, e); if (lease.live() && generation === sourceGeneration.current) void refreshSource(); }
    finally { finish(lease); }
  }
  async function deviceMaintenance(deviceId: string, maintenance: boolean) {
    if (!envelope || mutationBusy || !canControl(envelope, { kind:'set_device_acceptance', device_id:deviceId, accept_new_work:true }, currentLatest)) return;
    const lease = begin('mutation');setError('');
    try {
      await requestSimulatedDeviceMaintenance(envelope.snapshot.identity,deviceId,maintenance);
      if (lease.live()) setMessage(`已请求${deviceName(deviceId, true)}${maintenance ? '进入维护' : '恢复接单'}。调度开关保持原设置，设备状态上报后才会影响分派。`);
    } catch (e) { fail(lease,e); }
    finally { finish(lease); }
  }
  function makeQuery(): DaaSQuery {
    if (dataSet === '@execution' && snapshot) {
      const from = secondsToMilliseconds(fromSeconds, '开始时间'), until = secondsToMilliseconds(untilSeconds, '结束时间');
      if (from > until || until > executionQueryEnd(snapshot.coverage.observed_until_host_ms)) throw new CloudProductionError('range', '开始时间不能晚于结束时间。结束时间不包含该时点；请点击“整段记录”覆盖这条记录的完整范围。');
      if (!['rows', 'accepted_quantity', 'command_counts', 'waiting_time'].includes(aggregation)) throw new CloudProductionError('aggregation', '请选择执行聚合。');
      return { dataset: 'execution', contract_version: EXECUTION_CONTRACT, identity: snapshot.identity, selection: { kind: 'revision', execution_revision: snapshot.execution_revision },
        host_from_ms: from, host_until_ms: until, aggregation: aggregation as 'rows' | 'accepted_quantity' | 'command_counts' | 'waiting_time', page: { limit: 50, cursor: null } };
    }
    const descriptor = datasets.find(d => d.dataset_id === dataSet);
    if (!descriptor) throw new CloudProductionError('dataset', '请选择实际数据集。');
    if (!utcFrom || !utcUntil) throw new CloudProductionError('range', '请先选择查询开始和结束时间，或点击“最近24小时”。');
    checkUtcRange(utcFrom, utcUntil);
    if (aggregation !== 'rows' && aggregation !== 'count_by_device_minute') throw new CloudProductionError('aggregation', '请选择遥测聚合。');
    return { dataset: 'telemetry', contract_version: EXECUTION_CONTRACT, cloud_project_id: descriptor.cloud_project_id, dataset_id: descriptor.dataset_id,
      device_ids: [...deviceFilter], received_from: utcFrom, received_until: utcUntil,
      aggregation, page: { limit: 50, cursor: null } };
  }
  async function queryData(nextPage = false) {
    if (!context) return;
    const lease = begin('query'); setError(''); setMessage('');
    try {
      const previous = nextPage ? page : null;
      const query = previous ? { ...previous.query, page: { ...previous.query.page, cursor: previous.result.next_cursor } } : makeQuery();
      if (previous && previous.result.next_cursor === null) return;
      const r = await readCloud(context, { kind: 'data_query', query }, tracked(lease));
      if (lease.live() && r.kind === 'query_result') {
        const pinned = validateQueryPage(query, r.data, previous ? { selection: previous.pin, lastPosition: previous.lastPosition } : undefined,
          query.dataset === 'telemetry' ? datasets.find(d => d.dataset_id === query.dataset_id) : undefined);
        if (query.dataset === 'telemetry' && r.data.dataset === 'telemetry' && query.device_ids.length === 0)
          setKnownDeviceSet({ dataset: query.dataset_id, ids: [...r.data.selected_device_ids] });
        setPage({ query, result: r.data, pin: pinned.selection, lastPosition: pinned.lastPosition, number: (previous?.number ?? 0) + 1, seenRows: (previous?.seenRows ?? 0) + r.data.rows.length });
      }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  async function inspectCommand(commandId: string, nextPage = false) {
    if (!context || !snapshot || !available('read_cloud_production')) return;
    const original = snapshot.identity;
    if (!nextPage) {
      setCommand(null); setReceipts(null); setReceiptPages(0);
      const lease = begin('command');
      try {
        const r = await readCloud(context, { kind: 'command', identity: original, command_id: commandId }, tracked(lease));
        if (!lease.live()) return;
        if (lease.live() && r.kind === 'command') {
          if (identityKey(r.data.command.identity) !== identityKey(original) || r.data.command.attempt.command_id !== commandId) throw new CloudProductionError('command_mismatch', '命令不属于所选来源。');
          setCommand(r.data);
        }
      } catch (e) { fail(lease, e); if (!lease.live()) return; } finally { finish(lease); }
    }
    const lease = begin('receipts');
    try {
      const r = await readCloud(context, { kind: 'command_receipts', query: { contract_version: EXECUTION_CONTRACT, identity: original, command_id: commandId,
        page: { limit: 50, cursor: nextPage ? receipts?.next_cursor ?? null : null } } }, tracked(lease));
      if (lease.live() && r.kind === 'command_receipts') {
        if (identityKey(r.data.identity) !== identityKey(original) || r.data.command_id !== commandId
          || (nextPage && receipts && (r.data.dataset_id !== receipts.dataset_id || r.data.visible_frontier !== receipts.visible_frontier))) throw new CloudProductionError('receipt_mismatch', '回执分页来源/前沿变化，未替换原页。');
        setReceipts(r.data); setReceiptPages(n => nextPage ? n + 1 : 1);
      }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  async function exportResult(kind: 'report' | 'data') {
    if (!context || mutationBusy || unresolvedTask) return;
    const lease = begin('mutation'); setError(''); setMessage('');
    try {
      if (kind === 'data') {
        if (!page || !props.directories?.data) throw new CloudProductionError('directory', exportDirectoryUnavailable);
        const fixed = page;
        const destination = await props.directories.data(); lease.controller.signal.throwIfAborted(); if (!destination || !lease.live()) return;
        const result = await exportCloudData(context, { execution_contract_version: EXECUTION_CONTRACT, revision: context.projectRevision,
          query: exportQuery(fixed.query, fixed.pin), selection: fixed.pin, selected_destination_ref: destination }, tracked(lease));
        if (lease.live()) setMessage(`筛选结果已导出：共 ${result.row_count} 条、${result.page_count} 页。请在刚才选择的目录中查看。`);
      } else {
        if (!snapshot || !envelope || !props.directories?.report) throw new CloudProductionError('directory', exportDirectoryUnavailable);
        const fixed = { read_origin: envelope.delivery.read_origin, identity: snapshot.identity, execution_revision: snapshot.execution_revision };
        const chosenPrediction = props.predictionRuns?.find(r => r.run_id === prediction) ?? null;
        const destination = await props.directories.report(); lease.controller.signal.throwIfAborted(); if (!destination || !lease.live()) return;
        const result = await exportExecutionReport(context, { execution_contract_version: EXECUTION_CONTRACT, revision: context.projectRevision,
          sources: [fixed], formats: ['pdf', 'html', 'csv', 'json'], selected_destination_ref: destination,
          prediction: chosenPrediction ? { prediction: chosenPrediction, execution: fixed } : null }, tracked(lease));
        if (lease.live()) setMessage(`生产报告已导出，共 ${result.files.length} 个文件。请在刚才选择的目录中查看。`);
      }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  async function recoverRequest() {
    if (!context || !taskRecord || !available('read_execution_view')) return;
    const original = taskRecord;
    if (cloudContextKey(original.context) !== cloudContextKey(context)) return;
    const lease = begin('request'); setError('');
    try {
      const result = await recoverRegisteredCloudTask(original, tracked(lease));
      if (!lease.live()) return;
      if (!result) { setMessage('原请求未登记，保持输入；未自动重复准备或导出。'); return; }
      if (lease.live()) { setMessage(`原Task读取完成：${result.kind}。未重复提交。`); if (result.kind === 'execution_session') void localCatalog(); }
    } catch (e) { fail(lease, e); } finally { finish(lease); }
  }
  function switchTab(next: Tab) {
    // The selected source is shared by all three tabs; only tab-specific reads stop here.
    for (const lane of ['query', 'command', 'receipts'] as const) { lanes.current.get(lane)?.abort(); lanes.current.delete(lane); }
    setBusy(old => old.filter(lane => !['query', 'command', 'receipts'].includes(lane)));
    setTab(next);
    if (next === 'data' && !datasets.length && !lanes.current.has('datasets')) void datasetCatalog();
  }
  function tabKey(e: KeyboardEvent<HTMLButtonElement>, index: number) {
    const next = e.key === 'Home' ? 0 : e.key === 'End' ? 2 : e.key === 'ArrowRight' ? (index + 1) % 3 : e.key === 'ArrowLeft' ? (index + 2) % 3 : null;
    if (next === null) return; e.preventDefault(); switchTab(tabs[next][0]); document.getElementById(`${id}-${tabs[next][0]}`)?.focus();
  }
  function inspectView() {
    if (!snapshot || !source) return;
    try { const t = secondsToMilliseconds(viewSeconds, '回看时间'); if (t > snapshot.coverage.observed_until_host_ms) throw new CloudProductionError('time', '回看时间超过这条记录的可回看范围。');
      props.onInspectExecution({ read_origin: source.origin, identity: snapshot.identity, execution_revision: snapshot.execution_revision, observed_until_host_ms: t });
    } catch (e) { showError(e); }
  }
  const hasObligations = !!snapshot && (snapshot.lifecycle.unresolved_command_ids.length > 0 || snapshot.pending_reservations.length > 0 || snapshot.business_facts.pending_output_refs.length > 0);
  const routeRef = executionScenario?.production_tasks[0]?.routing_ref;
  const route = executionProject?.routes.find(route => route.id === routeRef?.id && route.revision === routeRef.revision);
  const routeProgress = snapshot && executionProject && route && route.steps.length > 1
    ? route.steps.map(step => {
      const work = snapshot.logical_work.filter(work => work.route_step_id === step.id);
      return { step, total: work.length, completed: work.filter(work => work.state === 'business_effect_committed' || work.state === 'accepted').length };
    }) : [];
  const configured = configStatus === 'ready' && !!config?.cloud_connection_ref && !!config.cloud_project_id;
  const selectedActiveOwner = source && catalog.find(entry => entry.active_owner && identityKey(entry.identity) === identityKey(source.identity));
  const connectionStatus = !available('read_cloud_production') ? '当前程序没有云端读取能力'
    : !scenario ? '尚未选择方案，未读取连接配置'
    : configStatus === 'loading' ? '正在读取连接配置'
    : configStatus === 'error' ? '连接配置读取失败，状态未确认'
    : configured ? '本机已配置云连接' : '本机尚未配置云连接';
  const dataExportReason = !available('export_cloud_data') ? '当前程序尚未提供筛选结果导出。' : !props.directories?.data ? exportDirectoryUnavailable : !page ? '请先查询一次，确定要导出的数据范围。' : '';
  const reportReason = !available('export_execution_report') ? '当前程序尚未提供生产报告导出。' : !props.directories?.report ? exportDirectoryUnavailable : !snapshot ? '请先选择生产记录。' : '';
  const descriptor = datasets.find(d => d.dataset_id === dataSet);
  const configuredDeviceIds = [...new Set(config?.candidate_bindings.map(b => b.device_id) ?? [])];
  const bindingGroups = configuredDeviceIds.map(deviceId => ({deviceId, items: config!.candidate_bindings.filter(b => b.device_id === deviceId)}));
  const queryDeviceIds = knownDeviceSet?.dataset === dataSet ? knownDeviceSet.ids : descriptor?.purpose === 'generated_load' ? [] : configuredDeviceIds;
  const targetUnits = [...new Set([...(scenario?.production_tasks.map(t => t.planned_quantity.unit) ?? []),
    ...(scenario?.orders.flatMap(o => o.quantity ? [o.quantity.unit] : []) ?? []), ...(project?.item_types.map(t => t.unit) ?? []), ...(unit ? [unit] : [])])];
  function bindingCaption(binding: CloudProductionConfiguration['candidate_bindings'][number], fixed = false) {
    const inputProject = fixed ? executionProject : project, inputScenario = fixed ? executionScenario : scenario;
    const steps = new Set<number>();
    for (const task of inputScenario?.production_tasks ?? []) {
      const route = inputProject?.routes.find(r => r.id === task.routing_ref.id && r.revision === task.routing_ref.revision);
      route?.steps.forEach((step, index) => { if (step.operation_ref.id === binding.operation_ref.id && step.operation_ref.version === binding.operation_ref.version) steps.add(index + 1); });
    }
    return deviceName(binding.device_id, fixed) + (steps.size ? ` · 适用工序 ${[...steps].sort((a,b) => a-b).join('、')}` : '');
  }
  function datasetTitle(dataset: DatasetDescriptor, index: number) {
    const session = catalog.findIndex(c => c.identity.session_id === dataset.session_id);
    return `${purposeLabel[dataset.purpose]} · ${sourceModeLabel[dataset.source_mode]}${session >= 0 ? ` · 生产记录 ${session + 1}` : ''} · 数据集 ${index + 1}`;
  }
  function queryDeviceName(deviceId: string) {
    const title = deviceName(deviceId);
    const index = queryDeviceIds.indexOf(deviceId);
    return title === '设备名称未记录' && index >= 0 ? `${descriptor?.purpose === 'generated_load' ? '测试设备' : '未命名设备'} ${index + 1}` : title;
  }
  function localQueryTime(which: 'from' | 'until', raw: string) {
    try { const utc = localInputToUtc(raw); setError(''); invalidateQuery(() => which === 'from' ? setUtcFrom(utc) : setUtcUntil(utc)); }
    catch (reason) { showError(reason); }
  }
  return <section className="cloud-production" aria-label="云端生产工作区">
    <header className="cloud-heading"><div><h2>云端生产</h2><span>{snapshot ? sourceModeLabel[snapshot.provenance.mode] : config ? sourceModeLabel[config.source_mode] : '连接设备、查询数据并管理生产'}</span></div><button type="button" onClick={() => { abortReads(); props.onClose(); }}>返回画布</button></header>
    <div className="cloud-source" aria-label="当前来源">
      <span>工厂：<strong>{project?.title ?? '尚未打开工厂'}</strong></span>
      <span>云连接：{connectionStatus}</span>
      <span>{snapshot ? `${source?.origin === 'cloud' ? '云端存档（只读）' : currentLatest ? '当前本机会话' : source?.selection.kind === 'latest' ? '其他本机会话（只读）' : '本机历史记录（只读）'} · 快照修订 R${snapshot.execution_revision}` : source ? '正在读取所选生产记录' : '尚未选择生产记录'}</span>
      <span>{snapshot ? `观察记录前沿 F${snapshot.observation_frontier} · 可回看至 ${elapsed(snapshot.coverage.observed_until_host_ms)}` : '选择记录后可查看生产状态'}</span>
      {envelope && <span>{({ synced: '已同步云端', publication_pending: '正在等待云端同步', unknown: '云端同步状态未确认' })[envelope.delivery.relation]}</span>}
      {snapshot && <details><summary>来源技术详情</summary><dl>{Object.entries(snapshot.identity).map(([k, v]) => <div key={k}><dt>{k}</dt><dd>{v}</dd></div>)}</dl>
        <Json value={{ project_input_ref: snapshot.project_input_ref, execution_revision: snapshot.execution_revision, delivery: envelope?.delivery, access: envelope?.access }} label="版本与权限字段"/>
        <button type="button" onClick={() => { void navigator.clipboard.writeText(JSON.stringify(snapshot.identity)).then(() => setMessage('已复制完整执行身份。'), e => showError(e)); }}>复制完整来源</button>
      </details>}
    </div>
    {error && <div className="cloud-alert" role="alert">{error}{errorDetail && <Json value={errorDetail} label="技术详情"/>}<button type="button" onClick={() => setError('')}>收起提示</button></div>}
    {message && <div className="cloud-message" role="status">{message}</div>}
    {tab !== 'data' && routeProgress.length > 0 && <div className="cloud-route-progress" aria-label="多工序进度">
      {routeProgress.map(({ step, total, completed }, index) => <div key={step.id}>
        <span>工序 {index + 1} · {executionProject?.devices.filter(device => device.route_operation_refs?.some(ref => ref.id === step.operation_ref.id && ref.version === step.operation_ref.version)).map(device => device.title).join(' / ')}</span>
        <strong>完成 {completed} / {total}</strong>
      </div>)}
      <small>各道工序分别计数；最终认可量以成品出口为准。设备端维护由新观测触发改派，恢复后重新参与候选。</small>
    </div>}
    {busy.length > 0 && <p className="cloud-loading" role="status">正在处理：{busy.map(lane => ({ configuration: '读取连接配置', datasets: '读取数据集', catalog: '读取生产记录', source: '读取所选记录', query: '查询数据', command: '读取指令', receipts: '读取回执', mutation: '提交操作', request: '查询操作状态' })[lane]).join('、')}。切换页面不会停止生产。</p>}
    <div role="tablist" aria-label="云端生产页签" className="cloud-tabs">{tabs.map(([value, label], index) => <button type="button" role="tab" id={`${id}-${value}`} key={value} aria-selected={tab === value} aria-controls={`${id}-panel-${value}`} tabIndex={tab === value ? 0 : -1} onKeyDown={e => tabKey(e, index)} onClick={() => switchTab(value)}>{label}</button>)}</div>
    {!project ? <Notice>请先打开工程。面板没有创建演示数据或生产会话。</Notice> : <>
      <div className="cloud-source-picker"><label>生产记录<select aria-label="生产记录" value={source ? identityKey(source.identity) : ''} onChange={e => { if (!e.target.value) { clearSource(); return; } const entry = catalog.find(c => identityKey(c.identity) === e.target.value); if (entry) void chooseSource({ identity: entry.identity, origin: 'host', selection: entry.active_owner ? { kind: 'latest' } : { kind: 'revision', execution_revision: entry.execution_revision } }); }}>
        <option value="">不选择记录 · 可准备新会话</option>{catalog.map((c, index) => <option key={identityKey(c.identity)} value={identityKey(c.identity)}>{c.active_owner ? '当前会话' : `已保存记录 ${catalog.slice(0, index + 1).filter(entry => !entry.active_owner).length}`} · 快照 R{c.execution_revision} · {labels[c.lifecycle_state]} · {elapsed(c.observed_until_host_ms)} · {labels[c.goal_at_snapshot]}</option>)}
      </select></label>
        {historyReadOnly && selectedActiveOwner && <button type="button" onClick={() => void chooseSource({ identity: selectedActiveOwner.identity, origin: 'host', selection: { kind: 'latest' } })}>返回当前本机会话</button>}
        {historyReadOnly && <button type="button" onClick={clearSource}>退出只读回看，准备新会话</button>}
        <button type="button" disabled={!available('read_execution_view')} onClick={() => void localCatalog()}>刷新记录列表</button>
        <button type="button" disabled={!source} onClick={() => void refreshSource()}>刷新所选记录</button>
        <button type="button" disabled={!source || !snapshot || !configured || !available('read_cloud_production')} onClick={() => { if (source && snapshot) void chooseSource({ identity: source.identity, origin: 'cloud', selection: { kind: 'revision', execution_revision: snapshot.execution_revision } }); }}>云端存档</button>
        <details className="cloud-version-options"><summary>版本选项</summary><div>
          <button type="button" disabled={!source || !snapshot} onClick={() => { if (source && snapshot) void chooseSource({ identity: source.identity, origin: source.origin, selection: { kind: 'revision', execution_revision: snapshot.execution_revision } }); }}>固定当前版本</button>
          <button type="button" disabled={!selectedActiveOwner || !available('get_execution_snapshot')} onClick={() => { if (selectedActiveOwner) void chooseSource({ identity: selectedActiveOwner.identity, origin: 'host', selection: { kind: 'latest' } }); }}>跟随本机会话</button>
          <label className="cloud-revision">快照修订 R<input aria-label="快照修订" inputMode="numeric" value={revisionDraft} onChange={e => setRevisionDraft(e.target.value)}/></label>
          <button type="button" disabled={!source} onClick={() => { if (!source) return; try { const r = safeInteger(revisionDraft, '记录版本'); void chooseSource({ identity: source.identity, origin: source.origin, selection: { kind: 'revision', execution_revision: r } }); } catch (e) { showError(e); } }}>读取这个版本</button>
        </div></details>
      </div>
      {!available('read_execution_view') && <Notice>当前程序没有本机生产记录读取能力。</Notice>}
      {available('read_execution_view') && catalogStatus === 'loading' && <Notice>正在读取本机生产记录列表。</Notice>}
      {available('read_execution_view') && catalogStatus === 'error' && <Notice>生产记录列表未能读取；请查看上方错误并刷新记录列表。</Notice>}
      {available('read_execution_view') && catalogStatus === 'ready' && catalog.length === 0 && <Notice>当前工程尚无本机生产记录；可在“执行”页准备新会话。</Notice>}
      <div role="tabpanel" id={`${id}-panel-${tab}`} aria-labelledby={`${id}-${tab}`} tabIndex={0} className="cloud-tab-content">
      {tab === 'devices' && <>
        <div className="cloud-section-heading"><h3>设备与接单状态</h3><button type="button" disabled={mutationBusy} onClick={() => void (async () => { const lease=begin('mutation'); setError(''); try { const count=await invoke<number|null>('project_import_cloud_connection'); if(lease.live() && count!==null)setMessage(`已校验并保存 ${count} 台设备的连接。请保存工程、退出并重新打开应用后使用新连接；当前生产连接未切换。`); } catch(e) { if(lease.live())showError(typeof e==='object' && e && 'message' in e ? new Error(String(e.message)) : e); } finally { finish(lease); } })()}>导入连接文件…</button><button type="button" disabled={!available('read_cloud_production')} onClick={() => void configuration()}>刷新连接配置</button></div>
        {!available('read_cloud_production') && <Notice>当前程序尚未提供云端读取功能；已保存的本机生产记录仍可选择回看。</Notice>}
        {available('read_cloud_production') && !scenario && <Notice>请选择当前方案后读取连接配置。已保存记录可以独立回看。</Notice>}
        {available('read_cloud_production') && scenario && configStatus === 'loading' && <Notice>正在读取本机连接配置。已保存记录可以独立回看。</Notice>}
        {available('read_cloud_production') && configStatus === 'error' && <Notice>连接配置未能读取；请查看上方错误并刷新配置。不能据此判断本机是否已配置。</Notice>}
        {available('read_cloud_production') && configStatus === 'ready' && !configured && <Notice>本机尚未配置云连接。可导入连接文件，保存工程并重启应用；已保存记录仍可只读回看。</Notice>}
        <div className="cloud-table-scroll"><table><thead><tr><th>设备与工序</th><th>调度设置</th><th>设备上报</th><th>报告状态</th><th>当前能否接单</th><th>操作</th></tr></thead><tbody>
          {[...new Map((snapshot?.devices ?? []).map(d => [d.binding.device_id,d])).values()].map(d => <tr key={d.binding.device_id}>
            <td><strong>{bindingCaption(d.binding, true)}</strong><Json value={d.binding} label="绑定技术详情"/></td>
            <td>{d.operator_accept_new_work ? '允许新任务' : '暂停新任务'}</td>
            <td>{d.latest_observation ? <>{labels[d.latest_observation.observation.device_state]}<small>设备{d.latest_observation.observation.accept_new_work ? '允许' : '暂停'}接单 · 接收于 {elapsed(d.latest_observation.host_received_elapsed_ms)}</small></> : '尚未收到报告'}</td>
            <td>{labels[d.freshness]}</td><td>{d.effective_accept_new_work === null ? '尚未确认' : d.effective_accept_new_work ? '可以接新任务' : '暂不能接新任务'}
              {d.blocking_reasons.length > 0 && <small>{d.blocking_reasons.map(reason => blockingLabel[reason] ?? '限制说明见详情').join('；')}</small>}<Json value={d} label="状态技术详情"/></td>
            <td><button type="button" disabled={project.revision !== snapshot!.project_input_ref.project_revision || !project.devices.some(device => device.id === d.binding.device_id)} onClick={() => props.onLocateDevice(d.binding.device_id)}>在工厂中定位</button>
              {!historyReadOnly && <button type="button" disabled={mutationBusy || !canControl(envelope, { kind: 'set_device_acceptance', device_id: d.binding.device_id, accept_new_work: !d.operator_accept_new_work }, currentLatest)} onClick={() => void control({ kind: 'set_device_acceptance', device_id: d.binding.device_id, accept_new_work: !d.operator_accept_new_work })}>{d.operator_accept_new_work ? '暂停接新任务' : '允许接新任务'}</button>}
              {!historyReadOnly && snapshot?.provenance.mode === 'software_in_loop' && <><button type="button" disabled={mutationBusy || !canControl(envelope,{kind:'set_device_acceptance',device_id:d.binding.device_id,accept_new_work:true},currentLatest)} onClick={() => void deviceMaintenance(d.binding.device_id,true)}>模拟设备维护</button><button type="button" disabled={mutationBusy || !canControl(envelope,{kind:'set_device_acceptance',device_id:d.binding.device_id,accept_new_work:true},currentLatest)} onClick={() => void deviceMaintenance(d.binding.device_id,false)}>恢复模拟设备</button></>}
            </td>
          </tr>)}
          {!snapshot?.devices.length && bindingGroups.map(({items}) => items[0]).map(b => <tr key={b.device_id}><td><strong>{bindingCaption(b)}</strong><Json value={b} label="绑定技术详情"/></td><td colSpan={4}>已配置为候选设备；选择生产记录后才能查看上报状态与接单资格。</td><td><button type="button" onClick={() => props.onLocateDevice(b.device_id)}>在工厂中定位</button></td></tr>)}
        </tbody></table></div>
        {!snapshot?.devices.length && !config?.candidate_bindings.length && <Notice>尚未配置可用于生产的设备。</Notice>}
        <Json value={config} label="连接配置技术详情"/>
      </>}
      {tab === 'data' && <>
        <div className="cloud-section-heading"><h3>查询生产与设备数据</h3><button type="button" disabled={!configured || !available('read_cloud_production')} onClick={() => void datasetCatalog()}>刷新数据集</button></div>
        <form className="cloud-filter" onSubmit={e => { e.preventDefault(); void queryData(); }}>
          <label>数据集<select aria-label="数据集" value={dataSet} onChange={e => invalidateQuery(() => { setDataSet(e.target.value); setAggregation('rows'); setDeviceFilter([]); setKnownDeviceSet(null); })}>
            <option value="">请选择数据集</option>{datasets.map((d,index) => <option key={`${d.cloud_project_id}:${d.dataset_id}`} value={d.dataset_id}>{datasetTitle(d,index)}</option>)}{snapshot && <option value="@execution">所选生产记录 · 快照 R{snapshot.execution_revision}</option>}
          </select></label>
          <label>查看内容<select aria-label="查看内容" value={aggregation} onChange={e => invalidateQuery(() => setAggregation(e.target.value))}><option value="rows">逐条记录</option>{dataSet === '@execution' ? <><option value="accepted_quantity">已接收数量</option><option value="command_counts">指令状态数量</option><option value="waiting_time">已记录等待时长</option></> : <option value="count_by_device_minute">每台设备每分钟的记录数</option>}</select></label>
          {dataSet === '@execution' ? <>
            <label>开始时间（秒，含）<input aria-label="生产查询起点（秒）" inputMode="decimal" value={fromSeconds} onChange={e => invalidateQuery(() => setFromSeconds(e.target.value))}/></label>
            <label>结束时间（秒，不含）<input aria-label="生产查询终点（秒）" inputMode="decimal" value={untilSeconds} onChange={e => invalidateQuery(() => setUntilSeconds(e.target.value))}/></label>
            <div className="cloud-time-shortcuts"><span>记录截至 {snapshot ? elapsed(snapshot.coverage.observed_until_host_ms) : '所选记录末尾'}，按准备生产后的经过时间计算。</span><button type="button" disabled={!snapshot || snapshot.coverage.observed_until_host_ms === Number.MAX_SAFE_INTEGER} onClick={() => snapshot && invalidateQuery(() => {setFromSeconds('0');setUntilSeconds(millisecondsToSeconds(executionQueryEnd(snapshot.coverage.observed_until_host_ms)));})}>整段记录</button><span>包括末尾毫秒内的事件；结束时间本身不计入。</span></div>
          </> : <>
            <label>接收开始时间（含）<input type="datetime-local" step="0.001" aria-label="接收开始时间" value={utcToLocalInput(utcFrom)} onChange={e => localQueryTime('from',e.target.value)}/></label>
            <label>接收结束时间（不含）<input type="datetime-local" step="0.001" aria-label="接收结束时间" value={utcToLocalInput(utcUntil)} onChange={e => localQueryTime('until',e.target.value)}/></label>
            <div className="cloud-time-shortcuts"><span>按本机时区 {localTimeZone} 显示</span>{([['最近1小时',3600000],['最近24小时',86400000]] as const).map(([label,span]) => <button type="button" key={label} onClick={() => invalidateQuery(() => { const now = new Date(); setUtcUntil(now.toISOString()); setUtcFrom(new Date(now.getTime()-span).toISOString()); })}>{label}</button>)}</div>
            <fieldset className="cloud-device-filter"><legend>设备（不勾选则查询当前连接允许的全部设备）</legend><div>{queryDeviceIds.map(deviceId => <label key={deviceId}><input type="checkbox" checked={deviceFilter.includes(deviceId)} onChange={e => invalidateQuery(() => setDeviceFilter(old => e.target.checked ? [...old,deviceId] : old.filter(id => id !== deviceId)))}/>{queryDeviceName(deviceId)}</label>)}</div>
              {queryDeviceIds.length === 0 && <p>首次查询后可读取这个数据集的设备清单，再按名称筛选。</p>}
              <button type="button" disabled={deviceFilter.length === 0} onClick={() => invalidateQuery(() => setDeviceFilter([]))}>清除设备筛选</button>
            </fieldset>
            <details className="cloud-precise-time"><summary>精确 UTC 时间</summary><div>
              <label>UTC 起点（含）<input aria-label="精确UTC起点" value={utcFrom} onChange={e => invalidateQuery(() => setUtcFrom(e.target.value))}/></label>
              <label>UTC 终点（不含）<input aria-label="精确UTC终点" value={utcUntil} onChange={e => invalidateQuery(() => setUtcUntil(e.target.value))}/></label>
            </div><p>可保留六位小数的微秒精度；日历控件显示到毫秒，查看日历不会改写这里的精确值。</p></details>
            {[utcFrom,utcUntil].some(value => /\.\d{4,6}Z$/.test(value)) && <p className="cloud-filter-hint">当前筛选保留微秒精度，精确值见“精确 UTC 时间”；日历显示到毫秒。</p>}
          </>}
          <button type="submit" disabled={!dataSet || !available('read_cloud_production') || busy.includes('query')}>查询并固定结果</button>
        </form>
        {descriptor && <Json value={descriptor} label="数据集技术详情"/>}
        {!page ? <Notice>选择数据集和时间范围后查询。修改筛选会清除上一批结果，需要重新查询后才能导出。</Notice> : <>
          <div className="cloud-page-status">第 {page.number} 页 · 本页 {page.result.rows.length} 条 · 累计读取 {page.seenRows} 条 · {page.result.next_cursor === null ? '已到末页' : '还有后续页'}</div>
          <Json value={{ query: page.query, selection: page.pin }} label="完整筛选与固定来源"/>
          <QueryFactsTable page={page} deviceTitle={(id,execution) => execution ? deviceName(id,true) : queryDeviceName(id)} locationTitle={locationName}/>
          <details className="cloud-data-technical"><summary>字段、显示列与当前页趋势</summary><CloudDataExplorer result={page.result} pageNumber={page.number}/></details>
          {page.result.rows.length === 0 && <Notice>这个筛选范围内没有记录。空结果仍有明确来源，可以导出。</Notice>}
          {page.result.data_quality_issues.length > 0 && <Json value={page.result.data_quality_issues} label={`数据质量说明（${page.result.data_quality_issues.length} 项）`}/>}
          <button type="button" disabled={page.result.next_cursor === null || busy.includes('query')} onClick={() => void queryData(true)}>下一页记录</button>
        </>}
        <div className="cloud-export"><button type="button" title={dataExportReason} disabled={!!dataExportReason || mutationBusy || unresolvedTask} onClick={() => void exportResult('data')}>导出全部筛选结果</button><span>{dataExportReason || '导出全部固定筛选结果，包含尚未翻阅的分页。'}</span></div>
      </>}
      {tab === 'execution' && <>
        <div className="cloud-section-heading"><h3>{historyReadOnly ? '只读生产记录' : '准备与运行生产'}</h3><span>{historyReadOnly ? '只读回看' : dirty ? '表单有待提交输入' : '准备完成后仍需手动启动'}</span></div>
        {historyReadOnly && <Notice>所选记录不是当前可控制的本机会话，仅供查看；这里不发送准备、启动或设备控制命令。需要新建生产会话时，先退出只读回看。</Notice>}
        {source && !envelope && !historyReadOnly && <Notice>正在确认所选会话的来源与操作权限；读取完成后再提供生产操作。</Notice>}
        {canPresentProduction && scenario && <form className="cloud-prepare" onSubmit={e => { e.preventDefault(); void prepare(); }}>
          <fieldset disabled={mutationBusy}><legend>新建生产会话 · {scenario.title}</legend>
            <div className="cloud-field-grid"><label>最终目标数量（整数）<input aria-label="最终目标数量" inputMode="numeric" value={target} onChange={e => changeForm(() => setTarget(e.target.value))}/></label>
              <label>目标单位<select aria-label="目标单位" value={unit} onChange={e => changeForm(() => setUnit(e.target.value))}><option value="">请选择单位</option>{targetUnits.map(value => <option key={value} value={value}>{factoryUnitLabel(value)}</option>)}</select></label>
              <label>停止新派工时间（秒）<input aria-label="停止新派工时间（秒）" inputMode="decimal" value={cutoffSeconds} onChange={e => changeForm(() => setCutoffSeconds(e.target.value))}/></label>
              <label>设备状态有效期（秒）<input aria-label="设备状态有效期（秒）" inputMode="decimal" value={maxAgeSeconds} onChange={e => changeForm(() => setMaxAgeSeconds(e.target.value))}/></label>
            </div>
            <p>最终目标按最后一道工序的产出计数，中间工序不重复计入。到达停止时间后不再派发新任务，已准入任务继续收尾。</p>
            <p>方案预测范围：0 至 {elapsed(scenario.window.end_ms)}。{scenario.production_tasks.length === 1 && `本方案投产数量：${quantityText(scenario.production_tasks[0].planned_quantity)}。`}</p>
            <details><summary>查看方案投产任务（{scenario.production_tasks.length} 项）</summary><ul>{scenario.production_tasks.map((task,index) => <li key={task.id}>任务 {index+1}：{quantityText(task.planned_quantity)}</li>)}</ul></details>
            <div className="cloud-bindings">
              {bindingGroups.some(g => g.items.length > 1) && <p>整厂软件演示设备按名义工时的 1/30 执行，每阶段至少 1 秒。生产时间、阶段进度与产出以真实回执为准；此模式不代表实体设备产能。</p>}
              <button type="button" disabled={!config?.candidate_bindings.length} onClick={() => changeForm(() => setBindings(config?.candidate_bindings.map(b => b.binding_id) ?? []))}>选择全部设备</button>
              <details open={bindingGroups.length <= 4 ? true : undefined}><summary>设备与阶段（已选 {bindingGroups.filter(g => g.items.every(b => bindings.includes(b.binding_id))).length} / {bindingGroups.length}）</summary>
              {bindingGroups.map(({deviceId,items}) => <div className="cloud-binding-choice" key={deviceId}><label><input type="checkbox" checked={items.every(b => bindings.includes(b.binding_id))} ref={el => {if(el) el.indeterminate = items.some(b => bindings.includes(b.binding_id)) && !items.every(b => bindings.includes(b.binding_id));}} onChange={e => changeForm(() => setBindings(old => e.target.checked ? [...new Set([...old,...items.map(b => b.binding_id)])] : old.filter(id => !items.some(b => b.binding_id === id))))}/><span>{deviceName(deviceId)}<small>{items.length} 个加工或运输阶段</small></span></label><details><summary>查看阶段与参考工时</summary>{items.map(b => <div key={b.binding_id}>{bindingCaption(b)} · {elapsed(b.nominal_work_ms)}<Json value={b} label="技术详情"/></div>)}</details></div>)}
              </details>
            </div>
            <button type="submit" disabled={!configured || !available('start_execution_session') || unresolvedTask}>准备生产</button>
            {!available('start_execution_session') && <small>{configured ? '当前连接未提供生产准备功能。' : '请先在“设备”页导入连接文件，保存工程并重启应用后准备生产。'}</small>}
          </fieldset>
        </form>}
        {currentLatest && <div className="cloud-controls">{([['run', '启动'], ['pause_dispatch', '暂停新派工'], ['resume_dispatch', '继续新派工'], ['stop_dispatch', '停止新派工'], ['request_reconciliation', '请求原命令核对']] as const).map(([kind, label]) => <button type="button" key={kind} disabled={mutationBusy || !available('control_execution_session') || !canControl(envelope, { kind }, currentLatest)} onClick={() => void control({ kind })}>{label}</button>)}</div>}
        {!snapshot ? <Notice>尚未选择生产记录。准备新的会话或选择已保存记录后，可查看状态与结果。</Notice> : <>
          <div className="cloud-facts"><div><span>生产状态</span><strong>{labels[snapshot.lifecycle.state]}</strong></div><div><span>最终目标 {quantityText(snapshot.policy.target_quantity)}</span><strong>{labels[snapshot.goal_at_snapshot]}</strong><small>停止新派工时间 {elapsed(snapshot.policy.report_cutoff_ms)}：{labels[snapshot.goal_at_cutoff]}</small></div><div><span>待核对指令 / 等待出料 / 已预留</span><strong>{snapshot.lifecycle.unresolved_command_ids.length} / {snapshot.business_facts.pending_output_refs.length} / {snapshot.pending_reservations.length}</strong></div></div>
          {snapshot.lifecycle.state === 'draining' && snapshot.lifecycle.reason && <Notice>{snapshot.lifecycle.reason}</Notice>}
          <details className="cloud-execution-details"><summary>执行明细与诊断</summary>
          <Json value={snapshot.business_facts} label="完整物品、占用与接收事实"/>
          <h4>同R接受事件（{snapshot.business_facts.acceptance_events.length}）</h4><div className="cloud-table-scroll"><table><thead><tr><th>Host时间</th><th>接受量</th><th>去向</th><th>阶段</th></tr></thead><tbody>{snapshot.business_facts.acceptance_events.map((a, i) => <tr key={i}><td>{a.at_ms}ms</td><td>{formatQuantity(a.quantity)}</td><td>{a.destination}</td><td>{a.acceptance_stage}</td></tr>)}</tbody></table></div>
          <h4>逻辑工作与派工理由（{snapshot.logical_work.length}）</h4>{snapshot.logical_work.map(w => <div className="cloud-record" key={w.logical_job_id}><strong>{w.logical_job_id}</strong> · {w.state} · 工单 {w.production_task_id} / 周期 {w.cycle}<Json value={w.latest_dispatch_decision} label="Host派工理由与候选"/></div>)}
          <h4>尝试与命令（{snapshot.attempts.length}）</h4>{snapshot.attempts.map(a => <div className="cloud-record" key={a.attempt.command_id}><strong>{a.attempt.command_id}</strong> · {a.device_id} · {a.state}<small>{a.attempt.logical_job_id} / 第{a.attempt.attempt_ordinal}次尝试</small><Json value={a} label="尝试及调度事实"/><button type="button" disabled={!available('read_cloud_production')} onClick={() => void inspectCommand(a.attempt.command_id)}>查看命令与云原始回执</button></div>)}
          <Json value={snapshot.receipt_admissions} label={`同R Host接纳决定（${snapshot.receipt_admissions.length}）；与云原始回执分列`}/>
          <h4>同R工时（缺测保持未知）</h4><div className="cloud-table-scroll"><table><thead><tr><th>工单 / 命令</th><th>名义ms</th><th>Host预留ms</th><th>Host占用ms</th><th>设备自报ms</th><th>物理有效ms</th></tr></thead><tbody>{snapshot.timings.map(t => <tr key={`${t.job_id}:${t.command_id}`}><td>{t.job_id}<small>{t.command_id}</small><Json value={t.missing_conditions} label="缺失条件"/></td><td>{text(t.nominal_work_ms)}</td><td>{text(t.host_confirmed_reservation_ms)}</td><td>{text(t.host_confirmed_occupation_ms)}</td><td>{text(t.device_reported_duration_ms)}</td><td>{text(t.physical_effective_work_ms)}</td></tr>)}</tbody></table></div>
          {command && <Json value={command} label={`原始命令与投递/Host确认：${command.command.attempt.command_id}`}/>}
          {receipts && <div className="cloud-record"><h4>云原始回执 · F{receipts.visible_frontier} · 第{receiptPages}页</h4><p>这是独立云流前缀，非同R接纳结果；{receipts.next_cursor ? '还有后续页' : '该前缀已到末页'}。</p><Json value={receipts.rows} label={`当前页${receipts.rows.length}条原始回执`}/><button type="button" disabled={!receipts.next_cursor || busy.includes('receipts')} onClick={() => void inspectCommand(receipts.command_id, true)}>下一页原始回执</button></div>}
          </details>
          <div className="cloud-view-action"><label>回看时间（秒）<input aria-label="回看时间（秒）" inputMode="decimal" value={viewSeconds} onChange={e => { viewTimeEdited.current = true; setViewSeconds(e.target.value); }}/></label><button type="button" onClick={inspectView}>回看执行画布与时间轴</button></div>
          <Json value={snapshot.coverage.data_quality_issues} label="执行覆盖与质量问题"/>
        </>}
        <div className="cloud-export"><label>对比预测方案（可不选）<select aria-label="对比预测方案" value={prediction} onChange={e => setPrediction(e.target.value)}><option value="">不与预测对比</option>{props.predictionRuns?.map((r,index) => <option key={r.run_id} value={r.run_id}>预测记录 {index+1} · 工厂第 {r.input_ref.project_revision} 版 / 方案第 {r.input_ref.scenario_revision} 版</option>)}</select></label><button type="button" title={reportReason} disabled={!!reportReason || mutationBusy || unresolvedTask} onClick={() => void exportResult('report')}>导出生产报告</button><span>{reportReason || '导出所选生产记录的完整报告。'}</span></div>
      </>}
      </div>
    </>}
    {taskRecord && <footer className="cloud-task"><strong>操作进度</strong><span>{({ start_execution_session: '准备生产', export_execution_report: '导出生产报告', export_cloud_data: '导出筛选结果', read_cloud_production: '读取云端数据' })[taskRecord.operation]} · {({ completed: '已完成', failed: '失败', cancelled: '已取消', rejected: '未受理', running: '处理中', queued: '等待处理', pending: '等待处理', registration_pending: '正在登记' } as Record<string,string>)[taskRecord.status] ?? '状态见技术详情'}</span><button type="button" disabled={!available('read_execution_view') || busy.includes('request') || mutationBusy} onClick={() => void recoverRequest()}>刷新操作状态</button><Json value={taskRecord} label="操作技术详情"/><small>{unresolvedTask ? '这次操作尚未确认结果，请先刷新状态再准备或导出。' : ''}准备和导出会继续处理，离开页面不会停止生产。</small></footer>}
  </section>;
}
