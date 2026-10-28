import { readCloud, readExecution, readLocalView, identityKey, EXECUTION_CONTRACT, CloudProductionError } from './cloudProduction';
import type { CloudContext, ExecutionIdentity, ExecutionReadEnvelope } from './cloudProduction';
import { projectFactoryPresentation, pointOnPath, type FactoryTransport, type FactoryHolderJob, type FactoryPresentationSource } from './factoryPresentation';
import { ProcessUnavailable } from './presentationHost';
import { executionDevicePhases } from './executionPhase';
import type { ExecutionFrozenInputs } from '../../../../packages/commands/generated/typescript/ExecutionFrozenInputs';
import type { ExecutionObservedObject } from '../../../../packages/commands/generated/typescript/ExecutionObservedObject';
import type { ExecutionObservationRecord } from '../../../../packages/commands/generated/typescript/ExecutionObservationRecord';
import type { ExecutionObservationPage } from '../../../../packages/commands/generated/typescript/ExecutionObservationPage';
import type { ModuleRef } from '../../../../packages/commands/generated/typescript/ModuleRef';
import type { AssetRef } from '../../../../packages/commands/generated/typescript/AssetRef';
import type { MediaAssetReadRequest } from './mediaAssets';
import type { CloudReadSelection } from '../../../../packages/commands/generated/typescript/CloudReadSelection';

export type ExecutionReadOrigin = 'host' | 'cloud';
export interface ExecutionObservationSelection {
  read_origin: ExecutionReadOrigin; identity: ExecutionIdentity; execution_revision: number; observed_until_host_ms: number;
}
export interface ExecutionAssetRequest {
  identity: ExecutionIdentity; execution_revision: number; module_ref: ModuleRef; asset: AssetRef;
}
/** Root supplies the independently authorized Native execution-archive byte reader. */
export type ExecutionAssetPort = (request: ExecutionAssetRequest) => Promise<ArrayBuffer>;
export interface ExecutionFrozenSource {
  read_origin: ExecutionReadOrigin; context: CloudContext; identity: ExecutionIdentity; revision: number;
  inputs: ExecutionFrozenInputs; envelope: ExecutionReadEnvelope;
}
export interface CompleteExecutionObservation {
  identity: ExecutionIdentity; execution_revision: number; observation_frontier: number;
  at_host_ms: number; clock_started: boolean; objects: ExecutionObservedObject[]; pages: number;
}
export interface CompleteExecutionHistory {
  identity: ExecutionIdentity; execution_revision: number; observation_frontier: number;
  host_from_ms: number; host_until_ms: number; records: ExecutionObservationRecord[]; pages: number;
}
export interface ReadGuard { signal?: AbortSignal; assertCurrent?: () => void }
export const executionBaseKey = (c: CloudContext, nativeSessionId: string, i: ExecutionIdentity, r: number, origin: ExecutionReadOrigin) =>
  JSON.stringify([origin, c.hostContextId, nativeSessionId, c.projectId, c.projectRevision, identityKey(i), r]);
export function executionSourceKey(frozen: ExecutionFrozenSource, nativeSessionId: string, at: number) {
  return JSON.stringify([executionBaseKey(frozen.context, nativeSessionId, frozen.identity, frozen.revision, frozen.read_origin),
    frozen.inputs.project_input_ref, frozen.envelope.snapshot.observation_frontier, at]);
}
function guard(options: ReadGuard) { options.signal?.throwIfAborted(); options.assertCurrent?.(); }
function same(a: unknown, b: unknown) { return JSON.stringify(a) === JSON.stringify(b); }
function assertIdentity(a: ExecutionIdentity, b: ExecutionIdentity) {
  if (identityKey(a) !== identityKey(b)) throw new CloudProductionError('execution_source_mismatch', '执行身份/session/epoch不一致，未采用此页。');
}
function matchPage(f: ExecutionFrozenSource, page: Pick<ExecutionObservationPage, 'identity' | 'execution_revision' | 'observation_frontier'>) {
  assertIdentity(f.identity, page.identity);
  if (page.execution_revision !== f.revision || page.observation_frontier !== f.envelope.snapshot.observation_frontier)
    throw new CloudProductionError('execution_prefix_mismatch', '执行修订或观察前沿不一致，未拼接不完整场景。');
}
function validTime(f: ExecutionFrozenSource, at: number) {
  if (!Number.isSafeInteger(at) || at < 0 || at > f.envelope.snapshot.coverage.observed_until_host_ms)
    throw new CloudProductionError('time_not_observed', '时点超出该R的实际Host观察前沿。');
  if (!f.envelope.snapshot.coverage.clock_started && at !== 0)
    throw new CloudProductionError('clock_not_started', 'prepared尚未启动时钟，只能观察T=0初始化。');
}
export function executionObjectRef(object: ExecutionObservedObject): { kind: ExecutionObservedObject['kind']; id: string } {
  switch (object.kind) {
    case 'transport': return { kind: 'transport', id: object.data.id };
    case 'item': return { kind: 'item', id: object.data.id };
    case 'job': return { kind: 'job', id: object.data.job_id };
    case 'device': return { kind: 'device', id: object.data.binding.device_id };
    case 'logical_work': return { kind: 'logical_work', id: object.data.logical_job_id };
    case 'attempt': return { kind: 'attempt', id: object.data.attempt.command_id };
  }
}
/** Route only the explicitly selected origin; neither path falls back to the other. */
async function readSourceView(context: CloudContext, origin: ExecutionReadOrigin,
  selection: Extract<CloudReadSelection, { kind: 'snapshot' | 'inputs' | 'observations' | 'history' }>, options: ReadGuard) {
  guard(options);
  if (origin === 'cloud') return readCloud(context, selection, { signal: options.signal });
  if (selection.kind === 'snapshot') return { kind: 'snapshot' as const, data: await readExecution(context, selection.identity, selection.selection) };
  return readLocalView(context, selection);
}
export async function readExecutionFrozenSource(context: CloudContext, identity: ExecutionIdentity, revision: number,
  origin: ExecutionReadOrigin, options: ReadGuard = {}): Promise<ExecutionFrozenSource> {
  guard(options);
  if (origin !== 'host' && origin !== 'cloud') throw new CloudProductionError('execution_origin', '必须明确选择host或cloud来源；没有自动换源。');
  if (identity.project_id !== context.projectId || !Number.isSafeInteger(revision) || revision < 0)
    throw new CloudProductionError('execution_source', '工程身份或执行修订无效。');
  const [snapshot, result] = await Promise.all([
    readSourceView(context, origin, { kind: 'snapshot', identity, selection: { kind: 'revision', execution_revision: revision } }, options),
    readSourceView(context, origin, { kind: 'inputs', identity }, options),
  ]);
  guard(options);
  if (snapshot.kind !== 'snapshot') throw new CloudProductionError('snapshot_missing', '所选来源没有返回执行快照。');
  const envelope = snapshot.data;
  assertIdentity(identity, envelope.snapshot.identity);
  if (envelope.delivery.read_origin !== origin || envelope.snapshot.execution_revision !== revision)
    throw new CloudProductionError('execution_source_mismatch', '快照不是原来源或精确R；未切换来源补齐。');
  if (result.kind !== 'inputs') throw new CloudProductionError('inputs_missing', '没有取得所选来源的执行冻结输入。');
  const inputs = result.data; assertIdentity(identity, inputs.identity);
  if (!same(inputs.project_input_ref, envelope.snapshot.project_input_ref)
    || inputs.project.id !== identity.project_id || inputs.project.revision !== inputs.project_input_ref.project_revision
    || inputs.scenario.id !== inputs.project_input_ref.scenario_id || inputs.scenario.revision !== inputs.project_input_ref.scenario_revision)
    throw new CloudProductionError('frozen_input_mismatch', '冻结文档与同R输入引用不一致；不使用当前草稿补齐。');
  return { read_origin: origin, context: { ...context }, identity, revision, inputs, envelope };
}
export async function readCompleteExecutionObservation(frozen: ExecutionFrozenSource, at: number, options: ReadGuard = {}): Promise<CompleteExecutionObservation> {
  validTime(frozen, at); const objects: ExecutionObservedObject[] = [], keys = new Set<string>(), cursors = new Set<string>();
  let cursor: string | null = null, pages = 0;
  do {
    guard(options);
    const result = await readSourceView(frozen.context, frozen.read_origin, { kind: 'observations', query: { contract_version: EXECUTION_CONTRACT, identity: frozen.identity,
      execution_revision: frozen.revision, at_host_ms: at, objects: [], page: { limit: 200, cursor } } }, options);
    guard(options);
    if (result.kind !== 'observations') throw new CloudProductionError('observation_missing', 'Host未返回执行时点对象页。');
    const page = result.data; matchPage(frozen, page);
    if (page.at_host_ms !== at || page.clock_started !== frozen.envelope.snapshot.coverage.clock_started)
      throw new CloudProductionError('observation_time_mismatch', '对象页不属于原T或clock_started。');
    for (const object of page.objects) {
      const key = JSON.stringify(executionObjectRef(object));
      if (keys.has(key)) throw new CloudProductionError('duplicate_observation', '同一时点对象分页重复，未当作完整场景。');
      keys.add(key); objects.push(object);
    }
    pages++; cursor = page.next_cursor;
    if (cursor !== null && (cursors.has(cursor) || page.objects.length === 0)) throw new CloudProductionError('cursor_not_progressing', '对象分页未前进。');
    if (cursor !== null) cursors.add(cursor);
  } while (cursor !== null);
  return { identity: frozen.identity, execution_revision: frozen.revision, observation_frontier: frozen.envelope.snapshot.observation_frontier,
    at_host_ms: at, clock_started: frozen.envelope.snapshot.coverage.clock_started, objects, pages };
}
export async function readCompleteExecutionHistory(frozen: ExecutionFrozenSource, from: number, until: number, options: ReadGuard = {}): Promise<CompleteExecutionHistory> {
  validTime(frozen, from); validTime(frozen, until);
  if (from > until) throw new CloudProductionError('history_range', '历史闭区间的起点不能晚于终点。');
  const records: ExecutionObservationRecord[] = [], cursors = new Set<string>(); let cursor: string | null = null, pages = 0;
  let previousSequence = 0, previousTime = -1;
  do {
    guard(options);
    const result = await readSourceView(frozen.context, frozen.read_origin, { kind: 'history', query: { contract_version: EXECUTION_CONTRACT, identity: frozen.identity,
      execution_revision: frozen.revision, host_from_ms: from, host_until_ms: until, objects: [], page: { limit: 200, cursor } } }, options);
    guard(options);
    if (result.kind !== 'history') throw new CloudProductionError('history_missing', 'Host未返回执行历史页。');
    const page = result.data; matchPage(frozen, page);
    if (page.host_from_ms !== from || page.host_until_ms !== until) throw new CloudProductionError('history_range_mismatch', '历史页区间改变。');
    for (const record of page.records) {
      if ((previousSequence !== 0 && record.record_sequence !== previousSequence + 1) || record.record_sequence > page.observation_frontier || record.execution_revision > frozen.revision
        || record.at_host_ms < from || record.at_host_ms > until || record.at_host_ms < previousTime)
        throw new CloudProductionError('history_order', '原历史顺序/前沿/闭区间不一致，未排序修补或拼成完整历史。');
      previousSequence = record.record_sequence; previousTime = record.at_host_ms; records.push(record);
    }
    pages++; cursor = page.next_cursor;
    if (cursor !== null && (cursors.has(cursor) || page.records.length === 0)) throw new CloudProductionError('cursor_not_progressing', '历史分页未前进。');
    if (cursor !== null) cursors.add(cursor);
  } while (cursor !== null);
  if (from === 0 && until === frozen.envelope.snapshot.coverage.observed_until_host_ms && records.length !== frozen.envelope.snapshot.observation_frontier)
    throw new CloudProductionError('history_prefix_gap', '完整观察区间缺少原始记录；未声称历史完整。');
  return { identity: frozen.identity, execution_revision: frozen.revision, observation_frontier: frozen.envelope.snapshot.observation_frontier,
    host_from_ms: from, host_until_ms: until, records, pages };
}
/** No JobState is synthesized: only the actual holder-to-device identity pair is projected. */
export function executionHolderJobs(objects: readonly ExecutionObservedObject[]): FactoryHolderJob[] {
  return objects.flatMap(object => object.kind === 'job' ? [{ id: object.data.job_id, execution_ref: object.data.execution_ref }] : []);
}

/** Position follows the device's reported phase progress and the frozen path. */
export function executionTransports(inputs: ExecutionFrozenInputs, observation: CompleteExecutionObservation): FactoryTransport[] {
  const phases = executionDevicePhases(observation);
  const labels = { reserved: '准备装货', loading: '装货', outbound: '运送', waiting_receive: '等待收货', unloading: '卸货', returning: '空车返回', returned: '已返回' };
  return observation.objects.flatMap(o => o.kind === 'transport' && o.data.phase !== 'returned' ? [o.data] : []).map(trip => {
    const path = inputs.scenario.supplies.find(s => s.id === trip.supply_ref)?.presentation_path;
    const row: FactoryTransport = { trip, hall_id: path?.hall_id ?? null, pose: null, cargo_layout: path?.cargo_layout ?? null,
      label: labels[trip.phase], unavailable: null };
    if (!path || path.waypoints_m.length < 2) return { ...row, unavailable: '此运输未声明显示路线。' };
    let progress = trip.phase === 'reserved' || trip.phase === 'loading' ? 0 : 1;
    if (trip.phase === 'outbound' || trip.phase === 'returning') {
      const phase = phases.get(trip.carrier_id);
      if (phase?.job_id !== trip.id || !phase.point?.known || phase.point.progress == null || phase.point.display_phase_id !== trip.phase)
        return { ...row, unavailable: '等待运输设备报告当前位置。' };
      progress = trip.phase === 'returning' ? 1 - phase.point.progress : phase.point.progress;
    }
    const { point, yaw } = pointOnPath(path.waypoints_m, progress), angle = yaw + (trip.phase === 'returning' ? Math.PI : 0);
    return { ...row, pose: { translation: point, rotation: { x: 0, y: 0, z: Math.sin(angle / 2), w: Math.cos(angle / 2) },
      scale: { x: 1, y: 1, z: 1 }, unit: 'meter', coordinate_frame: 'right_handed_z_up' } };
  });
}
export async function createExecutionPresentation(frozen: ExecutionFrozenSource, observation: CompleteExecutionObservation, nativeSessionId: string,
  readAsset: ExecutionAssetPort | undefined, options: ReadGuard = {}): Promise<FactoryPresentationSource> {
  guard(options); matchPage(frozen, observation); validTime(frozen, observation.at_host_ms);
  const modules = new Map<string, ExecutionFrozenInputs['modules'][number]>();
  for (const pkg of frozen.inputs.modules) {
    const key = JSON.stringify([pkg.definition.id, pkg.definition.version]);
    if (modules.has(key)) throw new CloudProductionError('duplicate_module', '冻结输入含重复精确模块。');
    modules.set(key, pkg);
  }
  const module = async (ref: ModuleRef) => {
    guard(options); const pkg = modules.get(JSON.stringify([ref.id, ref.version]));
    if (!pkg) throw new ProcessUnavailable(`冻结模块缺失：${ref.id}@${ref.version}；未读取草稿模块。`);
    return pkg;
  };
  const asset = async (request: MediaAssetReadRequest) => {
    guard(options);
    if (request.source.kind !== 'module_version' || request.project_id !== frozen.identity.project_id || request.scope.kind !== 'project'
      || request.scope_revision !== frozen.inputs.project.revision) throw new ProcessUnavailable('执行媒体描述符改变了冻结来源。');
    const reference = request.source.data.module_ref, pkg = await module(reference);
    if (!pkg.assets.some(a => same(a, request.asset)) || !frozen.inputs.asset_bindings.some(b => same(b.module_ref, reference) && same(b.asset, request.asset)))
      throw new ProcessUnavailable('素材不属于该执行冻结模块/AssetRef绑定。');
    if (!readAsset) throw new ProcessUnavailable('执行归档媒体字节端口尚未接入；未回退到当前模块或任意URL。');
    const bytes = await readAsset({ identity: frozen.identity, execution_revision: frozen.revision, module_ref: reference, asset: request.asset });
    guard(options); return bytes;
  };
  const source = await projectFactoryPresentation({ session_id: nativeSessionId, project_id: frozen.identity.project_id, scope: { kind: 'project' },
    scope_revision: frozen.inputs.project.revision, project: frozen.inputs.project, view: null }, {
      project: frozen.inputs.project, run: null, observation: null, jobs: [],
      transports: executionTransports(frozen.inputs, observation),
      holderJobs: executionHolderJobs(observation.objects), items: observation.objects.flatMap(o => o.kind === 'item' ? [o.data] : []),
      raw: { job_pages: [], item_pages: [] },
    }, { module, readAsset: asset }, () => guard(options));
  guard(options);
  const key = executionSourceKey(frozen, nativeSessionId, observation.at_host_ms);
  const phases = executionDevicePhases(observation);
  source.devices = source.devices.map(device => {
    const phase = phases.get(device.device_id);
    return phase ? { ...device, observed_phase: phase, projection: phase.point?.terminal_pose ? 'terminal' : 'active' } : device;
  });
  const holders = new Map([...phases.values()].map(phase => [phase.job_id, phase]));
  source.items = source.items.map(input => input.item.holder.kind === 'job'
    ? { ...input, holder_observed_phase: holders.get(input.item.holder.id) } : input);
  if (!readAsset) {
    const reason = '执行归档媒体字节端口未接入；仅显示冻结声明布局，物理进度未测。';
    source.devices = source.devices.map(d => ({ ...d, media: null, unavailable: reason, projection: 'unavailable' }));
    source.items = source.items.map(item => ({ ...item, media: null, unavailable: item.unavailable ?? reason }));
    source.resources = source.resources?.map(r => ({ ...r, media: null, unavailable: r.unavailable ?? reason }));
  }
  return { ...source, key, execution_source: { read_origin: frozen.read_origin, identity: frozen.identity, execution_revision: frozen.revision, at_host_ms: observation.at_host_ms } };
}
