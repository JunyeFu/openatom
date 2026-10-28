import { dispatch, nativeHostAvailable, HostFailure } from './projectHost';
import { invoke } from '@tauri-apps/api/core';
import type { ApiRequest } from '../../../../packages/commands/generated/typescript/ApiRequest';
import type { CapabilityCatalog } from '../../../../packages/commands/generated/typescript/CapabilityCatalog';
import type { HostStartRequest } from '../../../../packages/commands/generated/typescript/HostStartRequest';
import type { HostReadRequest } from '../../../../packages/commands/generated/typescript/HostReadRequest';
import type { HostControlRequest } from '../../../../packages/commands/generated/typescript/HostControlRequest';
import type { HostCloudReadRequest } from '../../../../packages/commands/generated/typescript/HostCloudReadRequest';
import type { HostExecutionViewRequest } from '../../../../packages/commands/generated/typescript/HostExecutionViewRequest';
import type { HostExportExecutionReportRequest } from '../../../../packages/commands/generated/typescript/HostExportExecutionReportRequest';
import type { HostExportCloudDataRequest } from '../../../../packages/commands/generated/typescript/HostExportCloudDataRequest';
import type { CloudReadResult } from '../../../../packages/commands/generated/typescript/CloudReadResult';
import type { CloudReadSelection } from '../../../../packages/commands/generated/typescript/CloudReadSelection';
import type { CloudReadTaskHandle } from '../../../../packages/commands/generated/typescript/CloudReadTaskHandle';
import type { CloudReadTaskState } from '../../../../packages/commands/generated/typescript/CloudReadTaskState';
import type { StartupTaskHandle } from '../../../../packages/commands/generated/typescript/StartupTaskHandle';
import type { ExportingTaskHandle } from '../../../../packages/commands/generated/typescript/ExportingTaskHandle';
import type { ExecutionPreparationTaskState } from '../../../../packages/commands/generated/typescript/ExecutionPreparationTaskState';
import type { ExecutionReportTaskState } from '../../../../packages/commands/generated/typescript/ExecutionReportTaskState';
import type { CloudDataExportTaskState } from '../../../../packages/commands/generated/typescript/CloudDataExportTaskState';
import type { ControlAck } from '../../../../packages/commands/generated/typescript/ControlAck';
import type { ExecutionViewRead } from '../../../../packages/commands/generated/typescript/ExecutionViewRead';
import type { ExecutionReadEnvelope } from '../../../../packages/commands/generated/typescript/ExecutionReadEnvelope';
import type { ExecutionIdentity } from '../../../../packages/commands/generated/typescript/ExecutionIdentity';

export async function requestSimulatedDeviceMaintenance(identity: ExecutionIdentity, deviceId: string, maintenance: boolean): Promise<void> {
  await invoke('project_set_simulated_device_maintenance', { request: { identity, device_id: deviceId, maintenance } });
}
import type { SnapshotSelection } from '../../../../packages/commands/generated/typescript/SnapshotSelection';
import type { ControlAction } from '../../../../packages/commands/generated/typescript/ControlAction';
import type { DaaSQuery } from '../../../../packages/commands/generated/typescript/DaaSQuery';
import type { DaaSQueryResult } from '../../../../packages/commands/generated/typescript/DaaSQueryResult';
import type { DatasetDescriptor } from '../../../../packages/commands/generated/typescript/DatasetDescriptor';
import type { CloudDataExportSelection } from '../../../../packages/commands/generated/typescript/CloudDataExportSelection';
import type { HostExportQuery } from '../../../../packages/commands/generated/typescript/HostExportQuery';
import type { ExecutionReportDestination } from '../../../../packages/commands/generated/typescript/ExecutionReportDestination';
import type { CloudDataExportDestination } from '../../../../packages/commands/generated/typescript/CloudDataExportDestination';
import type { Quantity } from '../../../../packages/commands/generated/typescript/Quantity';

export type { ExecutionIdentity, ExecutionReadEnvelope, ControlAction, DaaSQuery, DaaSQueryResult,
  CloudDataExportSelection, DatasetDescriptor, ExecutionReportDestination, CloudDataExportDestination };
export const EXECUTION_CONTRACT = 'sf.execution/2.0-draft11' as const;
export interface CloudContext { readonly projectId: string; readonly projectRevision: number; readonly hostContextId: string }
type AsyncOperation = 'read_cloud_production' | 'start_execution_session' | 'export_execution_report' | 'export_cloud_data';
type AsyncState = CloudReadTaskState | ExecutionPreparationTaskState | ExecutionReportTaskState | CloudDataExportTaskState;
type AsyncHandle = CloudReadTaskHandle | StartupTaskHandle | ExportingTaskHandle;
export interface CloudTaskRecord {
  context: CloudContext; operation: AsyncOperation; request_id: string;
  task: AsyncHandle | null; status: string;
}
export interface CloudObserveOptions { signal?: AbortSignal; onTask?: (record: CloudTaskRecord) => void }
/** Only provide adapters after Native really registers these NEW directory purposes.
 * References come from the actual current caller and matching Native purpose. */
export interface CloudExportDirectories {
  report?: () => Promise<ExecutionReportDestination | null>;
  data?: () => Promise<CloudDataExportDestination | null>;
}
export const exportDirectoryUnavailable = '当前环境暂未提供可用的导出目录选择器。';
export class CloudProductionError extends Error {
  constructor(readonly code: string, message: string) { super(message); this.name = 'CloudProductionError'; }
}
export function cloudErrorCode(error: unknown) {
  return error instanceof HostFailure ? error.detail.code : error instanceof CloudProductionError ? error.code : null;
}
function same(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
  if (Array.isArray(a) || Array.isArray(b)) {
    return Array.isArray(a) && Array.isArray(b) && a.length === b.length
      && a.every((value, index) => same(value, b[index]));
  }
  const left = a as Record<string, unknown>, right = b as Record<string, unknown>;
  const keys = Object.keys(left);
  return keys.length === Object.keys(right).length
    && keys.every(key => Object.prototype.hasOwnProperty.call(right, key) && same(left[key], right[key]));
}
export const identityKey = (i: ExecutionIdentity) => JSON.stringify([i.project_id, i.host_context_session_id, i.session_id, i.execution_epoch_id]);
export function cloudContextKey(c: CloudContext) { return JSON.stringify([c.hostContextId, c.projectId, c.projectRevision]); }
export function hasCloudOperation(catalog: CapabilityCatalog, operation: ApiRequest['operation']) {
  return nativeHostAvailable() && catalog.implemented_operations.some(c => c.operation === operation && c.allowed_scopes.some(s => s.kind === 'project'));
}
function header(c: CloudContext) {
  return { api_version: '0.1' as const, request_id: `cloud-ui-${crypto.randomUUID()}`, project_id: c.projectId, scope: { kind: 'project' as const } };
}
function checkIdentity(context: CloudContext, expected: ExecutionIdentity, actual: ExecutionIdentity) {
  if (expected.project_id !== context.projectId || identityKey(expected) !== identityKey(actual)) {
    throw new CloudProductionError('source_mismatch', 'Host结果与原工程/session/epoch不一致，未替换当前来源。');
  }
}
function assertTask(record: CloudTaskRecord, task: AsyncHandle) {
  const kind = record.operation === 'read_cloud_production' ? 'cloud_read'
    : record.operation === 'start_execution_session' ? 'execution_preparation' : 'exporting';
  if (task.kind !== kind || task.scope.kind !== 'edit' || task.scope.data.kind !== 'project'
    || task.scope.data.data !== record.context.projectId || task.input_revision !== record.context.projectRevision
    || (record.task !== null && task.id !== record.task.id)) {
    throw new CloudProductionError('task_mismatch', '任务不属于原工程、类型或修订，未消费该结果。');
  }
}
async function taskState(record: CloudTaskRecord): Promise<AsyncState> {
  if (!record.task) throw new CloudProductionError('task_missing', '原Task尚未确认，请按原请求查询，不要重复副作用。');
  const state = await dispatch<AsyncState>({ ...header(record.context), operation: 'get_task', params: { task_id: record.task.id } });
  assertTask(record, state.task);
  if (state.originating_operation !== record.operation || state.originating_request_id !== record.request_id) {
    throw new CloudProductionError('task_origin_mismatch', '任务origin/request不匹配，未消费结果。');
  }
  return state;
}
async function cancelOnlyRead(record: CloudTaskRecord) {
  if (record.operation !== 'read_cloud_production' || !record.task) return;
  const state = await taskState(record);
  if (state.allowed_controls.includes('cancel') && !['completed', 'failed', 'cancelled'].includes(state.task.state)) {
    await dispatch<unknown>({ ...header(record.context), operation: 'cancel_task', params: { task_id: record.task.id, action: 'cancel' } });
  }
}
function abortableDelay(signal?: AbortSignal) {
  return new Promise<void>((resolve, reject) => {
    signal?.throwIfAborted();
    const done = () => { signal?.removeEventListener('abort', abort); resolve(); };
    const timer = window.setTimeout(done, 120);
    const abort = () => { window.clearTimeout(timer); signal?.removeEventListener('abort', abort); reject(signal?.reason ?? new DOMException('停止观察', 'AbortError')); };
    signal?.addEventListener('abort', abort, { once: true });
  });
}
async function awaitOutput(record: CloudTaskRecord, options: CloudObserveOptions) {
  try {
    while (true) {
      options.signal?.throwIfAborted();
      const state = await taskState(record);
      options.signal?.throwIfAborted();
      options.onTask?.({ ...record, task: state.task, status: state.task.state });
      if (state.task.state === 'failed') throw new CloudProductionError(state.failure?.code ?? 'task_failed', state.failure?.message ?? 'Host任务失败，未取得交付结果。');
      if (state.task.state === 'cancelled') throw new CloudProductionError('task_cancelled', '该任务已取消。');
      if (state.task.state === 'completed') {
        const result = state.result;
        const expected = record.operation === 'read_cloud_production' ? 'cloud_read' : record.operation === 'start_execution_session' ? 'execution_session'
          : record.operation === 'export_execution_report' ? 'execution_report' : 'cloud_data_export';
        if (!result || result.kind !== expected) throw new CloudProductionError('task_output_mismatch', '任务完成但结果kind与原操作不一致。');
        return result;
      }
      await abortableDelay(options.signal);
    }
  } catch (error) {
    if (options.signal?.aborted && record.operation === 'read_cloud_production') await cancelOnlyRead(record);
    throw error;
  }
}
async function registered<T extends AsyncHandle>(context: CloudContext, request: ApiRequest & { operation: AsyncOperation }, options: CloudObserveOptions) {
  options.signal?.throwIfAborted();
  const record: CloudTaskRecord = { context: { ...context }, operation: request.operation, request_id: request.request_id, task: null, status: 'requesting' };
  options.onTask?.(record); // Retain original request even when registration response becomes unknown.
  let task: T;
  try { task = await dispatch<T>(request); }
  catch (error) {
    // A typed Host rejection is definite. Transport/managed-result failures remain unknown.
    if (error instanceof HostFailure) options.onTask?.({ ...record, status: 'rejected' });
    throw error;
  }
  assertTask(record, task);
  record.task = task; record.status = task.state;
  options.onTask?.({ ...record }); // Preparation/export obligations survive view departure.
  return awaitOutput(record, options);
}
export function observeRegisteredTask(record: CloudTaskRecord, options: CloudObserveOptions = {}) {
  return awaitOutput(record, options); // Observe original identity only; no repeat prepare/export.
}
/** Recover only the original caller request. This never resubmits a side effect. */
export async function recoverRegisteredCloudTask(record: CloudTaskRecord, options: CloudObserveOptions = {}) {
  options.signal?.throwIfAborted();
  const result = await readLocalView(record.context, { kind: 'request', originating_request_id: record.request_id });
  options.signal?.throwIfAborted();
  if (result.kind !== 'request') throw new CloudProductionError('request_mismatch', 'Host未返回原请求登记，未消费其他来源。');
  if (!result.data.task) {
    options.onTask?.({ ...record, status: 'rejected' });
    return null;
  }
  if (result.data.operation !== record.operation) throw new CloudProductionError('request_mismatch', '原请求登记为另一操作，未消费或重复提交。');
  assertTask(record, result.data.task);
  const registered: CloudTaskRecord = { ...record, task: result.data.task, status: result.data.task.state };
  options.onTask?.(registered);
  return observeRegisteredTask(registered, options);
}

export async function readCloud(context: CloudContext, selection: CloudReadSelection, options: CloudObserveOptions = {}): Promise<CloudReadResult> {
  const request: HostCloudReadRequest = { ...header(context), operation: 'read_cloud_production', params: { execution_contract_version: EXECUTION_CONTRACT, selection } };
  const result = await registered<CloudReadTaskHandle>(context, request, options);
  if (result.kind !== 'cloud_read') throw new CloudProductionError('task_output_mismatch', '不是只读任务结果。');
  const expected = selection.kind === 'data_query' ? 'query_result' : selection.kind;
  if (result.data.kind !== expected) throw new CloudProductionError('selection_mismatch', '只读结果与原选择不一致。');
  return result.data;
}
export async function prepareExecution(context: CloudContext, params: HostStartRequest['params'], options: CloudObserveOptions = {}) {
  const request: HostStartRequest = { ...header(context), operation: 'start_execution_session', params };
  const result = await registered<StartupTaskHandle>(context, request, options);
  if (result.kind !== 'execution_session' || result.data.state !== 'prepared'
    || result.data.identity.project_id !== context.projectId || result.data.identity.host_context_session_id !== context.hostContextId) {
    throw new CloudProductionError('preparation_mismatch', '准备结果不属于原Host/工程或不是prepared。');
  }
  return result.data; // Never sends run.
}
export async function readExecution(context: CloudContext, identity: ExecutionIdentity, selection: SnapshotSelection) {
  const request: HostReadRequest = { ...header(context), operation: 'get_execution_snapshot', params: { execution_contract_version: EXECUTION_CONTRACT, identity, selection } };
  const result = await dispatch<ExecutionReadEnvelope>(request);
  if (!result?.snapshot || !result.delivery || !result.access || result.delivery.read_origin !== 'host')
    throw new CloudProductionError('snapshot_mismatch', 'Host未返回本机执行快照。');
  checkIdentity(context, identity, result.snapshot.identity);
  if (selection.kind === 'revision' && result.snapshot.execution_revision !== selection.execution_revision) throw new CloudProductionError('revision_mismatch', '快照不是原固定修订。');
  return result;
}
export function canControl(envelope: ExecutionReadEnvelope | null, action: ControlAction, currentLatest: boolean) {
  if (!currentLatest || !envelope || envelope.delivery.read_origin !== 'host' || envelope.access.ownership !== 'current_host_owner') return false;
  return action.kind === 'set_device_acceptance'
    ? envelope.access.device_acceptance_control_ids.includes(action.device_id)
    : envelope.access.allowed_session_controls.includes(action.kind);
}
export async function controlExecution(context: CloudContext, envelope: ExecutionReadEnvelope, action: ControlAction, currentLatest: boolean) {
  if (!canControl(envelope, action, currentLatest)) throw new CloudProductionError('read_only', '此来源没有该控制权限；历史与云来源保持只读。');
  const current = await readExecution(context, envelope.snapshot.identity, { kind: 'latest' });
  if (!canControl(current, action, true)) throw new CloudProductionError('read_only', '当前执行来源已不允许此控制，请刷新后核对。');
  const request: HostControlRequest = { ...header(context), operation: 'control_execution_session', params: {
    execution_contract_version: EXECUTION_CONTRACT, identity: current.snapshot.identity, expected_execution_revision: current.snapshot.execution_revision, action,
  } };
  const result = await dispatch<ControlAck>(request);
  if (!result?.identity || !same(result.action, action)) throw new CloudProductionError('control_mismatch', '控制确认与原动作不符。');
  checkIdentity(context, envelope.snapshot.identity, result.identity);
  return result;
}
export async function readLocalView(context: CloudContext, selection: HostExecutionViewRequest['params']['selection']) {
  const request: HostExecutionViewRequest = { ...header(context), operation: 'read_execution_view', params: { execution_contract_version: EXECUTION_CONTRACT, selection } };
  const result = await dispatch<ExecutionViewRead>(request);
  if (result.kind !== selection.kind) throw new CloudProductionError('selection_mismatch', '本机读取不是原请求的类型。');
  if (result.kind === 'catalog' && result.data.project_id !== context.projectId) throw new CloudProductionError('source_mismatch', '执行目录属于其他工程。');
  if (selection.kind === 'request' && result.kind === 'request'
    && (result.data.originating_request_id !== selection.originating_request_id || result.data.host_context_session_id !== context.hostContextId)) {
    throw new CloudProductionError('request_mismatch', '原请求索引不属于当前Host与请求。');
  }
  return result;
}
export async function exportExecutionReport(context: CloudContext, params: HostExportExecutionReportRequest['params'], options: CloudObserveOptions = {}) {
  if (params.selected_destination_ref.purpose !== 'export_execution_report') throw new CloudProductionError('directory_purpose', exportDirectoryUnavailable);
  const request: HostExportExecutionReportRequest = { ...header(context), operation: 'export_execution_report', params };
  const result = await registered<ExportingTaskHandle>(context, request, options);
  if (result.kind !== 'execution_report' || !same(result.data.destination, params.selected_destination_ref)) throw new CloudProductionError('export_mismatch', '报告Task结果与原目录不一致，未声称已落盘。');
  const sources = result.data.sources.map(s => ({ read_origin: s.read_origin, identity: s.identity, execution_revision: s.execution_revision }));
  if (!same(sources, params.sources) || !same(result.data.prediction, params.prediction)) throw new CloudProductionError('export_source_mismatch', '报告结果不是原固定来源/预测选择。');
  return result.data;
}
export async function exportCloudData(context: CloudContext, params: HostExportCloudDataRequest['params'], options: CloudObserveOptions = {}) {
  if (params.selected_destination_ref.purpose !== 'export_cloud_data') throw new CloudProductionError('directory_purpose', exportDirectoryUnavailable);
  // Rust UtcMicros returns six fractional digits even for a millisecond calendar selection.
  const query = params.query.dataset === 'telemetry' ? { ...params.query,
    received_from: utcMicrosecondText(params.query.received_from), received_until: utcMicrosecondText(params.query.received_until) } : params.query;
  const request: HostExportCloudDataRequest = { ...header(context), operation: 'export_cloud_data', params: { ...params, query } };
  const result = await registered<ExportingTaskHandle>(context, request, options);
  if (result.kind !== 'cloud_data_export' || !same(result.data.selection, params.selection) || !same(result.data.destination, params.selected_destination_ref) || !same(result.data.query, query)) throw new CloudProductionError('export_mismatch', '数据导出结果与原固定筛选/来源/目录不符。');
  return result.data;
}

export function formatQuantity(q: Quantity) {
  const places = q.value.decimal_places; const digits = String(q.value.coefficient).padStart(places + 1, '0');
  return `${places ? `${digits.slice(0, -places)}.${digits.slice(-places)}` : digits} ${q.unit}`;
}
export function parseQuantity(text: string, unit: string): Quantity {
  if (!/^\d+(\.\d{1,9})?$/.test(text) || !unit) throw new CloudProductionError('target', '目标需为非负十进制数量（最多9位小数）及真实单位。');
  const [whole, fraction = ''] = text.split('.'); const value = BigInt(whole + fraction);
  if (value > BigInt(Number.MAX_SAFE_INTEGER)) throw new CloudProductionError('target', '目标超出精确安全整数范围。');
  return { value: { coefficient: Number(value), decimal_places: fraction.length }, unit };
}
function utcMicrosecondText(s: string) {
  const [base, fraction = ''] = s.slice(0, -1).split('.');
  return `${base}.${fraction.padEnd(6, '0')}Z`;
}
export function checkUtcRange(from: string, until: string) {
  const pattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,6})?Z$/;
  if (!pattern.test(from) || !pattern.test(until) || utcMicrosecondText(from) > utcMicrosecondText(until)) throw new CloudProductionError('utc_range', '请填写有效UTC起止字符串（Z，最多6位小数）；起点不得晚于终点。');
}
function positions(result: DaaSQueryResult) { return result.rows.map(row => 'ingress_id' in row ? row.ingress_id : row.row_index); }
export function validateQueryPage(query: DaaSQuery, result: DaaSQueryResult, previous?: {
  selection: CloudDataExportSelection; lastPosition: number | null;
}, descriptor?: DatasetDescriptor): { selection: CloudDataExportSelection; lastPosition: number | null } {
  if (query.dataset !== result.dataset) throw new CloudProductionError('query_mismatch', '数据页类型与原查询不符。');
  let selection: CloudDataExportSelection;
  if (query.dataset === 'telemetry' && result.dataset === 'telemetry') {
    if (query.dataset_id !== result.dataset_id) throw new CloudProductionError('query_mismatch', '数据集发生变化。');
    selection = { dataset: 'telemetry', cloud_project_id: query.cloud_project_id, dataset_id: result.dataset_id,
      selected_visible_frontier: result.selected_visible_frontier, selected_device_ids: [...result.selected_device_ids] };
    if (query.device_ids.length && !same([...query.device_ids].sort(), [...result.selected_device_ids].sort())) throw new CloudProductionError('device_set_mismatch', '实际设备集合与明确筛选不符。');
    for (const row of result.rows) {
      if ((query.aggregation === 'rows') === (row.kind === 'count_by_device_minute')) throw new CloudProductionError('row_kind', '聚合行与原始行不能混用。');
      if (row.kind !== 'count_by_device_minute' && row.dataset_id !== result.dataset_id) throw new CloudProductionError('row_source', '行来源不是原数据集。');
      if (descriptor && ((row.kind === 'generated_load' && descriptor.purpose !== 'generated_load')
        || (row.kind === 'telemetry' && row.observation.source_purpose !== descriptor.purpose))) throw new CloudProductionError('row_source', '行用途与真实数据集描述不一致。');
    }
  } else if (query.dataset === 'execution' && result.dataset === 'execution') {
    if (query.selection.kind !== 'revision' || result.selected_execution_revision !== query.selection.execution_revision
      || result.selected_epoch !== query.identity.execution_epoch_id) throw new CloudProductionError('query_revision', '执行查询必须保持精确epoch/revision。');
    selection = { dataset: 'execution', identity: query.identity, execution_revision: result.selected_execution_revision };
    for (const row of result.rows) {
      if (identityKey(row.identity) !== identityKey(query.identity) || row.execution_revision !== result.selected_execution_revision) throw new CloudProductionError('row_source', '执行行不属于该固定来源。');
      if (query.aggregation !== 'rows' && row.kind !== ({ accepted_quantity: 'accepted_quantity', command_counts: 'command_count', waiting_time: 'waiting_time' } as const)[query.aggregation]) throw new CloudProductionError('row_kind', '执行聚合行类型不符。');
      if (query.aggregation === 'rows' && row.kind !== 'execution_attempt' && row.kind !== 'acceptance') throw new CloudProductionError('row_kind', '执行原始行类型不符。');
    }
  } else throw new CloudProductionError('query_mismatch', '查询来源不符。');
  if (previous && !same(selection, previous.selection)) throw new CloudProductionError('frontier_mismatch', '分页前沿/设备集合变化；原有效页保留，未合并此页。');
  const p = positions(result); const before = previous?.lastPosition ?? null;
  if (p.some((v, i) => i ? v <= p[i - 1] : before !== null && v <= before)
    || (query.page.cursor !== null && result.next_cursor === query.page.cursor)) throw new CloudProductionError('page_not_progressing', '数据页重复或未前进，未并入原分页链。');
  return { selection, lastPosition: p.length ? p[p.length - 1] : before };
}
export function exportQuery(query: DaaSQuery, selection: CloudDataExportSelection): HostExportQuery {
  if (query.dataset === 'telemetry' && selection.dataset === 'telemetry') return { ...query, device_ids: selection.selected_device_ids, page: { limit: 200, cursor: null } };
  if (query.dataset === 'execution' && selection.dataset === 'execution') return { ...query, selection: { kind: 'revision', execution_revision: selection.execution_revision }, page: { limit: 200, cursor: null } };
  throw new CloudProductionError('export_source', '没有对应的实际查询来源。');
}
