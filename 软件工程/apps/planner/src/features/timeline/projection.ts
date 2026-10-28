import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { RuntimeItem } from '../../../../../packages/commands/generated/typescript/RuntimeItem';
import type { Quantity } from '../../../../../packages/commands/generated/typescript/Quantity';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { SimulationEvent } from '../../../../../packages/commands/generated/typescript/SimulationEvent';
import type { HolderRef } from '../../../../../packages/commands/generated/typescript/HolderRef';
import type { JobState } from '../../../../../packages/commands/generated/typescript/JobState';
import type { Props, Primitive, TimeWindow, TimelineSelection, TimelineViewPreferencesV1 } from './types';
import { clipInterval, pointVisible, timeText } from './time';

export interface Track { id: string; title: string; kind: 'hall' | 'group' | 'device' | 'resource' | 'phase' | 'wait' | 'events' | 'input' | 'item'; devices: string[]; depth: number; object?: ObjectRef; warning?: string }
export interface Group extends Track { hall_id: string; manual: boolean }
export function jobPredecessors(project: ProjectState, scenario: ScenarioInput | null | undefined, run: RunResult, job: JobState) {
  const none = { jobs: [] as JobState[], declared_steps: [] as string[], route: null as ProjectState['routes'][number] | null };
  if (!scenario || scenario.id !== run.input_ref.scenario_id || scenario.revision !== run.input_ref.scenario_revision || scenario.project_revision !== run.input_ref.project_revision || job.execution_ref.kind !== 'project_device') return none;
  const task = scenario.production_tasks.find(t => t.id === job.task_ref);
  if (!task) return none;
  const route = project.routes.find(r => r.id === task.routing_ref.id && r.revision === task.routing_ref.revision);
  const stepId = job.execution_ref.data.route_step_id, predecessors = route?.steps.find(s => s.id === stepId)?.predecessors ?? [];
  if (!route || !predecessors.length) return none;
  const inputs = new Set(job.input_refs), origins = new Map<string, string>(), jobs = new Map(run.jobs.map(j => [j.id, j])), actual = new Map<string, JobState>();
  // Match opaque item/job identities in the real transfer sequence. The first
  // same-step job may belong to a different production cycle.
  for (const event of uniqueEvents(run)) {
    if (event.payload.kind !== 'item_transferred' || !inputs.has(event.payload.data.item_ref)) continue;
    const transfer = event.payload.data;
    if (transfer.from_holder.kind === 'job') origins.set(transfer.item_ref, transfer.from_holder.id);
    if (transfer.to_holder.kind !== 'job' || transfer.to_holder.id !== job.id) continue;
    const previous = jobs.get(origins.get(transfer.item_ref) ?? '');
    const step = previous?.execution_ref.kind === 'project_device' ? previous.execution_ref.data.route_step_id : null;
    if (previous && previous.id !== job.id && previous.task_ref === job.task_ref && step && predecessors.includes(step)) actual.set(previous.id, previous);
  }
  const covered = new Set([...actual.values()].map(j => j.execution_ref.kind === 'project_device' ? j.execution_ref.data.route_step_id : null));
  return { jobs: [...actual.values()], declared_steps: predecessors.filter(id => !covered.has(id)), route };
}
export const reasonNames = { missing_material: '缺料', resource_busy: '资源占用', calendar: '日历条件未满足', preparation: '准备条件', quality: '质量待处理', output_capacity: '输出空间不足', receiver_capacity: '接收空间不足' };
export const eventNames = { job_started: '作业开始', phase_completed: '阶段完成', item_transferred: '物品移交', arrival_accepted: '来料接受', quality_changed: '质量处置', resource_released: '资源释放', order_accepted: '订单接受', transport_changed: '运输变化', wait_changed: '等待变化' };
export const transportNames: Record<string, string> = { reserved: '已预留', loading: '装载', outbound: '去程', waiting_receive: '等待接收', unloading: '卸载', returning: '返程', returned: '已返库' };
export function quantityText(q: Quantity) {
  const { coefficient, decimal_places } = q.value;
  const digits = String(coefficient).padStart(decimal_places + 1, '0');
  return `${decimal_places ? `${digits.slice(0, -decimal_places)}.${digits.slice(-decimal_places)}` : digits} ${q.unit}`;
}
export function quantityTotals(items: RuntimeItem[]) {
  const sums = new Map<string, { value: bigint; places: number }>();
  for (const item of items.filter(v => v.lifecycle !== 'transformed')) {
    const q = item.quantity, old = sums.get(q.unit) ?? { value: 0n, places: 0 }, places = Math.max(old.places, q.value.decimal_places);
    sums.set(q.unit, { value: old.value * 10n ** BigInt(places - old.places) + BigInt(q.value.coefficient) * 10n ** BigInt(places - q.value.decimal_places), places });
  }
  return [...sums].map(([unit, v]) => {
    const digits = String(v.value).padStart(v.places + 1, '0');
    return `${v.places ? `${digits.slice(0, -v.places)}.${digits.slice(-v.places)}` : digits} ${unit}`;
  });
}
export const executionDevice = (job: RunResult['jobs'][number]) => job.execution_ref.kind === 'project_device' ? job.execution_ref.data.device_id : null;
const objectKey = (o: ObjectRef) => `${o.kind}:${o.id}`;
export const holderObject = (holder: HolderRef): ObjectRef => ({ kind: holder.kind === 'container' ? 'item' : holder.kind, id: holder.id });
export function uniqueEvents(run: RunResult) { return [...new Map(run.events.map(e => [e.sequence, e])).values()].sort((a, b) => a.sequence - b.sequence); }
export function frozenProjectMatches(project: ProjectState | null, run: RunResult | null) {
  return !run || !!project && project.id === run.input_ref.project_id && project.revision === run.input_ref.project_revision;
}

// Display membership is private UI state. Ambiguous business membership gets an
// explicit row instead of silently choosing one of the conflicting lines.
export function displayGroups(project: ProjectState, prefs: TimelineViewPreferencesV1): Group[] {
  const groups: Group[] = [], assigned = new Set<string>(), independent = new Set(prefs.independent_device_ids);
  for (const manual of prefs.group_overrides) {
    const devices = [...new Set(manual.members)].filter(id => project.devices.some(d => d.id === id && d.hall_id === manual.hall_id) && !assigned.has(id) && !independent.has(id));
    devices.forEach(id => assigned.add(id));
    if (devices.length) groups.push({ id: manual.id, title: manual.title, hall_id: manual.hall_id, kind: 'group', devices, depth: 1, manual: true });
  }
  for (const device of project.devices) {
    if (assigned.has(device.id) || independent.has(device.id)) continue;
    const refs = project.lines.filter(l => l.members.includes(device.id) || device.line_id === l.id);
    const invalid = refs.length > 1 || refs.some(l => l.hall_id !== device.hall_id) || !!device.line_id && !project.lines.some(l => l.id === device.line_id);
    const id = invalid ? `ambiguous:${device.hall_id}` : refs[0] ? `line:${refs[0].id}` : `ungrouped:${device.hall_id}`;
    let group = groups.find(g => g.id === id);
    if (!group) {
      group = { id, title: invalid ? '分组待确认' : refs[0]?.title ?? '未编组设备', hall_id: device.hall_id, kind: 'group', devices: [], depth: 1, manual: false,
        object: refs[0] && !invalid ? { kind: 'line', id: refs[0].id } : undefined, warning: invalid ? '成员列表、设备线组或厂房归属冲突；请在工程中确认。' : undefined };
      groups.push(group);
    }
    group.devices.push(device.id); assigned.add(device.id);
  }
  const order = new Map(prefs.row_order.map((id, i) => [id, i]));
  return groups.sort((a, b) => (order.get(a.id) ?? Infinity) - (order.get(b.id) ?? Infinity));
}

export function closedIntervals(open: TimeWindow[], domain: TimeWindow) {
  const result: TimeWindow[] = []; let cursor = domain.start_ms;
  for (const interval of [...open].sort((a, b) => a.start_ms - b.start_ms)) {
    if (interval.end_ms <= cursor || interval.start_ms >= domain.end_ms) continue;
    const start = Math.max(domain.start_ms, interval.start_ms), end = Math.min(domain.end_ms, interval.end_ms);
    if (start > cursor) result.push({ start_ms: cursor, end_ms: start });
    cursor = Math.max(cursor, end);
  }
  if (cursor < domain.end_ms) result.push({ start_ms: cursor, end_ms: domain.end_ms });
  return result;
}
export function eventRelated(event: SimulationEvent): ObjectRef[] {
  const p = event.payload;
  if (p.kind === 'item_transferred') return [{ kind: 'item', id: p.data.item_ref }, holderObject(p.data.from_holder), holderObject(p.data.to_holder)];
  if (p.kind === 'transport_changed') return [{ kind: 'transport', id: p.data.id }, { kind: 'resource', id: p.data.carrier_id }, ...p.data.cargo.map(id => ({ kind: 'item', id } as ObjectRef))];
  if (p.kind === 'order_accepted') return p.data.item_refs.map(id => ({ kind: 'item', id }));
  if (p.kind === 'wait_changed') return p.data.reasons.flatMap(r => r.blocking_object ? [r.blocking_object] : []);
  return [];
}
export function itemEvents(events: SimulationEvent[], id: string) {
  return events.filter(e => e.subject.id === id || eventRelated(e).some(o => o.id === id));
}

export function waitContext(selected: TimelineSelection | null, run: RunResult | null, at: number | null, items: RuntimeItem[] | null) {
  if (!selected?.wait || !run || selected.run_id !== run.id || at == null) return null;
  const wait = selected.wait, job = run.jobs.find(j => j.id === wait.object_ref.id), trip = run.transport.find(t => t.id === wait.object_ref.id);
  const cargoIds = new Set(trip?.cargo ?? [...(job?.input_refs ?? []), ...(job?.pending_outputs ?? [])]);
  const blockingResources = new Set(wait.reasons.flatMap(r => r.blocking_object?.kind === 'resource' ? [r.blocking_object.id] : []));
  // Sealed caps still hold at the boundary; natural releases stay half-open.
  const active = run.resource_claims.filter(c => c.intervals.some((i, index) => i.start_ms <= at &&
    (at < i.end_ms || at === i.end_ms && c.interval_end_causes[index] !== 'released')));
  const relatedIds = new Set([wait.object_ref.id, ...cargoIds, ...wait.reasons.flatMap(r => r.blocking_object ? [r.blocking_object.id] : [])]);
  const events = uniqueEvents(run).filter(e => relatedIds.has(e.subject.id) || eventRelated(e).some(o => relatedIds.has(o.id)));
  return { held_resources: active.filter(c => c.job_id === wait.object_ref.id),
    blocker_holders: active.filter(c => blockingResources.has(c.resource_id)),
    cargo: items === null ? null : items.filter(i => cargoIds.has(i.id)),
    previous: [...events].reverse().find(e => e.at_ms <= at), next: events.find(e => e.at_ms > at) };
}

export function actualProjection(project: ProjectState, run: RunResult): Primitive[] {
  if (!frozenProjectMatches(project, run) || run.computed_until_ms == null) return [];
  const frontier = run.computed_until_ms, all: Primitive[] = [], jobs = new Map(run.jobs.map(j => [j.id, j]));
  const base = (key: string, object: ObjectRef, label: string, fields: TimelineSelection['fields'] = [], related: ObjectRef[] = []): TimelineSelection =>
    ({ key: `${run.id}:${key}`, source: 'actual', run_id: run.id, input_ref: run.input_ref, object, label, fields, related_objects: related });
  const push = (p: Omit<Primitive, 'source' | 'run_id'>) => all.push({ ...p, source: 'actual', run_id: run.id });
  const deviceRow = (jobId: string) => { const job = jobs.get(jobId); return job && executionDevice(job) ? `device:${executionDevice(job)}` : 'events:unbound'; };
  for (const job of run.jobs) {
    if (job.started_at_ms == null || job.started_at_ms > frontier) continue;
    const interval = { start_ms: job.started_at_ms, end_ms: Math.min(job.finished_at_ms ?? frontier, frontier) }, key = `job:${job.id}`;
    const selection = { ...base(key, { kind: 'job', id: job.id }, `作业 ${job.id}`, [
      { label: '执行设备', value: executionDevice(job) ?? '定义周期' }, { label: '生产任务', value: job.task_ref },
      { label: '冻结操作', value: `${job.operation_ref.id} @ ${job.operation_ref.version}` }, { label: '实际开始', value: timeText(job.started_at_ms) },
      { label: '实际完成', value: job.finished_at_ms == null ? '尚未完成' : timeText(job.finished_at_ms) }, { label: '状态', value: job.state },
      { label: '有效进度', value: timeText(job.effective_progress_ms) }], [
        ...(executionDevice(job) ? [{ kind: 'device', id: executionDevice(job)! } as ObjectRef] : []), { kind: 'task', id: job.task_ref }, ...job.input_refs.map(id => ({ kind: 'item', id } as ObjectRef))]), job, true_interval: interval };
    push({ key: selection.key, row: deviceRow(job.id), kind: 'job', label: selection.label, object: selection.object, interval,
      point_ms: interval.start_ms === interval.end_ms ? interval.start_ms : undefined, known_end: job.finished_at_ms != null, selection });
  }
  for (const work of run.phase_work_intervals) {
    const job = jobs.get(work.job_id), key = `work:${work.job_id}:${work.phase_id}:${work.interval.start_ms}`;
    const selection = { ...base(key, { kind: 'job', id: work.job_id }, `阶段 ${work.phase_id}`, [
      { label: '阶段', value: work.phase_id }, { label: '实际有效工作', value: `${timeText(work.interval.start_ms)}—${timeText(work.interval.end_ms)}` },
      { label: '结束原因', value: work.end_cause }, { label: '累计有效进度', value: `${timeText(work.effective_progress_start_ms)}—${timeText(work.effective_progress_end_ms)}` }
    ], job?.input_refs.map(id => ({ kind: 'item', id })) ?? []), job, true_interval: work.interval };
    push({ key: selection.key, row: deviceRow(work.job_id), kind: 'work', label: work.phase_id, object: selection.object,
      phase: work.phase_id, known_end: work.end_cause !== 'computed_boundary', selection, interval: { ...work.interval } });
  }
  for (const wait of run.waits) {
    const device = wait.execution_ref?.kind === 'project_device' ? wait.execution_ref.data.device_id : null;
    const transport = run.transport.find(t => t.id === wait.object_ref.id);
    const key = `wait:${objectKey(wait.object_ref)}:${wait.phase_ref ?? ''}:${wait.interval.start_ms}`;
    const related = wait.reasons.flatMap(r => r.blocking_object ? [r.blocking_object] : []);
    const selection = { ...base(key, wait.object_ref, wait.reasons.map(r => transport && r.kind === 'output_capacity' ? '目的接收空间不足' : reasonNames[r.kind]).join('、'), wait.reasons.flatMap(r => [
      { label: '等待条件', value: r.required }, { label: '阻断对象', value: r.blocking_object ? objectKey(r.blocking_object) : '未给出' },
      { label: '下一已知边界', value: r.next_known_boundary_ms == null ? '未知' : timeText(r.next_known_boundary_ms) }
    ]).concat([{ label: '历史阶段', value: wait.phase_ref ?? '未绑定阶段' }, { label: '结束原因', value: wait.end_cause }]), related), wait, true_interval: wait.interval };
    push({ key: selection.key, row: device ? `device:${device}` : transport ? `resource:${transport.carrier_id}` : 'events:unbound', kind: 'wait',
      label: selection.label, object: selection.object, interval: wait.interval, point_ms: wait.interval.start_ms === wait.interval.end_ms ? wait.interval.start_ms : undefined,
      phase: wait.phase_ref ?? undefined, known_end: wait.end_cause === 'resolved' || wait.end_cause === 'condition_changed', selection });
  }
  for (const claim of run.resource_claims) claim.intervals.forEach((interval, index) => {
    const key = `claim:${claim.id}:${interval.start_ms}`, job = jobs.get(claim.job_id), cause = claim.interval_end_causes[index];
    const selection = { ...base(key, { kind: 'resource', id: claim.resource_id }, `资源占用 ${claim.resource_id}`, [
      { label: '持有作业', value: claim.job_id }, { label: '占用数量', value: String(claim.units) }, { label: '占用类型', value: claim.kind },
      { label: '历史阶段', value: claim.phase_ref ?? '作业全程' }, { label: '结束原因', value: cause }
    ], [{ kind: 'job', id: claim.job_id }]), job, true_interval: interval };
    const p = { key: selection.key, row: `resource:${claim.resource_id}`, kind: 'claim' as const, label: `${claim.units} · ${claim.job_id}`, object: selection.object,
      interval, known_end: cause === 'released', selection };
    push(p);
    // Only the frozen execution-capacity claim contributes to device occupancy.
    if (job?.execution_ref.kind === 'project_device' && job.execution_ref.data.execution_capacity_ref.id === claim.resource_id)
      push({ ...p, key: `${p.key}:capacity`, row: `device:${job.execution_ref.data.device_id}` });
  });
  for (const resource of project.resources) {
    const calendar = project.calendars.find(c => c.id === resource.calendar_id);
    if (!calendar) continue;
    for (const interval of closedIntervals(calendar.intervals, { start_ms: run.window.start_ms, end_ms: frontier })) {
      const key = `calendar:${resource.id}:${interval.start_ms}`, selection = { ...base(key, { kind: 'calendar', id: calendar.id }, `日历关闭 · ${calendar.title}`), true_interval: interval };
      push({ key: selection.key, row: `resource:${resource.id}`, kind: 'calendar', label: '日历关闭', object: selection.object, interval, known_end: interval.end_ms < frontier, selection });
    }
  }
  for (const trip of run.transport) {
    const key = `transport:${trip.id}`, fields = [
      { label: '运输阶段', value: transportNames[trip.phase] ?? trip.phase }, { label: '车辆', value: trip.carrier_id },
      { label: '真实货物', value: trip.cargo.join('、') || '无' }, ...([
        ['实际发出', trip.departed_ms], ['到达目的', trip.arrived_ms], ['接受货物', trip.received_ms], ['车辆返库', trip.returned_ms]
      ] as const).map(([label, ms]) => ({ label, value: ms == null ? '尚未发生' : timeText(ms) }))
    ];
    const selection = base(key, { kind: 'transport', id: trip.id }, `运输 ${trip.id}`, fields,
      [{ kind: 'resource', id: trip.carrier_id }, ...trip.cargo.map(id => ({ kind: 'item', id } as ObjectRef))]);
    if (trip.departed_ms != null) push({ key: selection.key, row: `resource:${trip.carrier_id}`, kind: 'transport', label: trip.id, object: selection.object,
      interval: { start_ms: trip.departed_ms, end_ms: trip.returned_ms ?? frontier }, known_end: trip.returned_ms != null, selection });
    for (const [name, ms] of [['发出', trip.departed_ms], ['到达', trip.arrived_ms], ['接受', trip.received_ms], ['返库', trip.returned_ms]] as const)
      if (ms != null) push({ key: `${selection.key}:${name}`, row: `resource:${trip.carrier_id}`, kind: 'transport', label: name, object: selection.object,
        point_ms: ms, known_end: true, selection: { ...selection, key: `${selection.key}:${name}`, label: `${name} · ${trip.id}`, at_ms: ms } });
    // Definition-cycle phase work belongs to its actual carrier, not a fake device.
    for (const p of all.filter(p => p.row === 'events:unbound' && p.selection.job?.id === trip.id)) p.row = `resource:${trip.carrier_id}`;
  }
  for (const event of uniqueEvents(run)) {
    const key = `event:${event.sequence}`, related = eventRelated(event);
    // An arrival batch has no Task object; its event belongs to the frozen Scenario.
    const object: ObjectRef = event.payload.kind === 'arrival_accepted'
      ? { kind: 'scenario', id: run.input_ref.scenario_id } : event.subject;
    const selection = { ...base(key, object, eventNames[event.payload.kind], [
      { label: '实际时点', value: timeText(event.at_ms) }, { label: '事件序号', value: String(event.sequence) }
    ], related), at_ms: event.at_ms, event };
    push({ key: selection.key, row: event.payload.kind === 'arrival_accepted' ? 'events:arrivals' : event.payload.kind === 'order_accepted' ? 'events:acceptance' : 'events:all',
      label: selection.label, kind: event.payload.kind === 'arrival_accepted' ? 'arrival' : event.payload.kind === 'order_accepted' ? 'acceptance' : 'event', object: selection.object,
      point_ms: event.at_ms, known_end: true, selection });
  }
  return all;
}

/** Local device inspection references the same actual segments, never a sub-Run. */
export function deviceDetailProjection(project: ProjectState, run: RunResult, primitives: Primitive[], device: string, window: TimeWindow) {
  const rows = new Map<string, Track>(), projected: Primitive[] = [];
  const inWindow = (p: Primitive) => p.point_ms != null ? pointVisible(p.point_ms, window) : !!p.interval && !!clipInterval(p.interval, window);
  for (const p of primitives) {
    if (p.row !== `device:${device}` || !['work', 'wait'].includes(p.kind) || !inWindow(p)) { projected.push(p); continue; }
    const job = p.selection.job?.id ?? p.selection.wait?.object_ref.id;
    const id = `detail:${run.id}:${p.kind}:${job}:${p.phase ?? 'unbound'}`;
    rows.set(id, { id, title: `${p.kind === 'work' ? '阶段' : '等待'} · ${job} · ${p.phase ?? '输出 / 未绑定阶段'}`, kind: p.kind === 'work' ? 'phase' : 'wait', devices: [], depth: 3, object: p.object });
    projected.push({ ...p, row: id });
  }
  for (const p of primitives) {
    if (p.kind !== 'claim' || !p.row.startsWith('resource:') || !p.selection.job || executionDevice(p.selection.job) !== device || !inWindow(p)) continue;
    const resource = project.resources.find(r => r.id === p.object.id), id = `detail:${run.id}:resource:${p.selection.job.id}:${p.object.id}`;
    rows.set(id, { id, title: `资源引用 · ${resource?.title ?? p.object.id} · ${resource?.role ?? p.object.id}`, kind: 'resource', devices: [], depth: 3, object: p.object });
    projected.push({ ...p, key: `${p.key}:detail`, row: id });
  }
  return { rows: [...rows.values()], primitives: projected };
}

export function inputProjection(scenario: ScenarioInput): Primitive[] {
  const all: Primitive[] = [];
  const add = (kind: 'arrival' | 'order_release' | 'task_release' | 'order_due', id: string, at: number, title: string, related: ObjectRef[] = []) => {
    const object: ObjectRef = { kind: kind === 'arrival' ? 'scenario' : kind === 'task_release' ? 'task' : 'order', id: kind === 'arrival' ? scenario.id : id };
    const key = `input:${scenario.id}:${kind}:${id}`, selection: TimelineSelection = { key, source: 'input', run_id: null, object, label: title, at_ms: at,
      fields: [{ label: '输入时点', value: timeText(at) }, { label: '方案', value: `${scenario.title} r${scenario.revision}` }, { label: '含义', value: '计划输入；实际开始由计算结果决定' }], related_objects: related };
    all.push({ key, row: kind === 'arrival' ? 'input:arrival' : kind === 'task_release' ? 'input:task' : 'input:order', label: title,
      kind: 'input', source: 'input', run_id: null, object, point_ms: at, known_end: true, input_field: { kind, id }, selection });
  };
  scenario.arrivals.forEach(a => add('arrival', a.id, a.at_ms, `到料 ${a.id}`, [{ kind: 'location', id: a.destination }]));
  scenario.production_tasks.forEach(t => add('task_release', t.id, t.release_at_ms, `释放 ${t.id}`, [{ kind: 'route', id: t.routing_ref.id }]));
  scenario.orders.forEach(o => { add('order_release', o.id, o.release_at_ms, `释放 ${o.id}`, [{ kind: 'location', id: o.destination }]);
    if (o.due_at_ms != null) add('order_due', o.id, o.due_at_ms, `交期 ${o.id}`); });
  return all;
}

export function calendarInputProjection(project: ProjectState, scenario: ScenarioInput): Primitive[] {
  return project.calendars.flatMap(calendar => calendar.intervals.map((interval, i) => {
    const key = `input:${scenario.id}:calendar:${calendar.id}:${i}`, object: ObjectRef = { kind: 'calendar', id: calendar.id };
    return { key, row: `input:calendar:${calendar.id}`, label: calendar.title, kind: 'calendar' as const, source: 'input' as const,
      run_id: null, object, interval, known_end: true, selection: { key, source: 'input' as const, run_id: null, object,
        label: `班次 · ${calendar.title}`, true_interval: interval,
        fields: [{ label: '开放区间', value: `${timeText(interval.start_ms)}—${timeText(interval.end_ms)}` }, { label: '编辑', value: '通过编辑班次表单修改；拖动输入不会移动日历' }], related_objects: [] } };
  }));
}
export interface SummaryBand { interval: TimeWindow; busy: number; waiting: number; occupied: number; total: number }
export function summarizeGroup(primitives: Primitive[], devices: string[]): SummaryBand[] {
  const members = new Set(devices.map(id => `device:${id}`));
  const source = primitives.filter(p => members.has(p.row) && p.interval && ['work', 'wait', 'claim'].includes(p.kind));
  const changes = new Map<number, { row: string; kind: 'work' | 'wait' | 'claim'; delta: number }[]>();
  for (const p of source) for (const [time, delta] of [[p.interval!.start_ms, 1], [p.interval!.end_ms, -1]]) {
    const list = changes.get(time) ?? []; list.push({ row: p.row, kind: p.kind as 'work' | 'wait' | 'claim', delta }); changes.set(time, list);
  }
  const times = [...changes.keys()].sort((a, b) => a - b), state = { work: new Map<string, number>(), wait: new Map<string, number>(), claim: new Map<string, number>() }, result: SummaryBand[] = [];
  times.forEach((at, i) => {
    for (const change of changes.get(at)!) { const counts = state[change.kind]; counts.set(change.row, (counts.get(change.row) ?? 0) + change.delta); }
    const end = times[i + 1]; if (end == null || end <= at) return;
    const count = (map: Map<string, number>) => [...map.values()].filter(v => v > 0).length;
    result.push({ interval: { start_ms: at, end_ms: end }, busy: count(state.work), waiting: count(state.wait), occupied: count(state.claim), total: devices.length });
  });
  return result;
}
export function resourceUsage(project: ProjectState, run: RunResult, ids: string[], range: TimeWindow) {
  const units = new Map<string, { occupied: number; available: number }>();
  for (const id of new Set(ids)) {
    const resource = project.resources.find(r => r.id === id), calendar = project.calendars.find(c => c.id === resource?.calendar_id);
    if (!resource || !calendar) continue;
    const q = resource.capacity, capacity = q.value.coefficient / 10 ** q.value.decimal_places;
    const overlap = (i: TimeWindow) => Math.max(0, Math.min(i.end_ms, range.end_ms, run.computed_until_ms ?? range.start_ms) - Math.max(i.start_ms, range.start_ms));
    // Merge opening intervals before capacity integration, never average percentages.
    const knownRange = { start_ms: range.start_ms, end_ms: Math.max(range.start_ms, Math.min(range.end_ms, run.computed_until_ms ?? range.start_ms)) };
    const unavailable = closedIntervals(calendar.intervals, knownRange).reduce((s, i) => s + i.end_ms - i.start_ms, 0);
    const available = (knownRange.end_ms - knownRange.start_ms - unavailable) * capacity;
    const occupied = run.resource_claims.filter(c => c.resource_id === id).reduce((sum, c) => sum + c.units * c.intervals.reduce((s, i) => s + overlap(i), 0), 0);
    const old = units.get(q.unit) ?? { occupied: 0, available: 0 };
    units.set(q.unit, { occupied: old.occupied + occupied, available: old.available + available });
  }
  return [...units].map(([unit, v]) => ({ unit, ...v, ratio: v.available > 0 ? v.occupied / v.available : null }));
}
const canonical = (v: unknown): string => Array.isArray(v) ? `[${v.map(canonical).join(',')}]` : v && typeof v === 'object' ? `{${Object.entries(v).sort(([a], [b]) => a.localeCompare(b)).map(([k, value]) => `${k}:${canonical(value)}`).join(',')}}` : JSON.stringify(v);
export function comparisonConditions(a: ScenarioInput | null | undefined, b: ScenarioInput | null | undefined, projectA?: ProjectState | null, projectB?: ProjectState | null) {
  if (!a || !b) return { comparable: false, differences: ['缺少两个结果的精确冻结方案'] };
  const differences: string[] = [];
  for (const [label, left, right] of [
    ['时间原点', a.time_reference, b.time_reference], ['初始条件', a.initial_conditions, b.initial_conditions], ['观察窗口', a.window, b.window],
    ['接受目标', a.acceptance_goal, b.acceptance_goal], ['订单接受口径', a.orders.map(o => ({ id: o.id, destination: o.destination, stage: o.acceptance_stage, quantity: o.quantity })), b.orders.map(o => ({ id: o.id, destination: o.destination, stage: o.acceptance_stage, quantity: o.quantity }))]
  ] as const) if (canonical(left) !== canonical(right)) differences.push(`${label}不同`);
  if (projectA && projectB) {
    const membership = (p: ProjectState) => p.devices.map(d => ({ id: d.id, hall: d.hall_id, line: d.line_id })).sort((x, y) => x.id.localeCompare(y.id));
    if (projectA.id !== projectB.id || canonical(membership(projectA)) !== canonical(membership(projectB)) || canonical(projectA.lines.map(l => ({ id: l.id, members: [...l.members].sort() })).sort((x,y)=>x.id.localeCompare(y.id))) !== canonical(projectB.lines.map(l => ({ id: l.id, members: [...l.members].sort() })).sort((x,y)=>x.id.localeCompare(y.id)))) differences.push('设备身份或分组成员不同');
  }
  return { comparable: !differences.length, differences };
}
export function comparisonInputs(a: ScenarioInput | null | undefined, b: ScenarioInput | null | undefined) {
  if (!a || !b) return [];
  const left = inputProjection(a), right = inputProjection(b), keys = new Set([...left, ...right].map(p => `${p.input_field!.kind}:${p.input_field!.id}`));
  return [...keys].flatMap(key => {
    const x = left.find(p => `${p.input_field!.kind}:${p.input_field!.id}` === key), y = right.find(p => `${p.input_field!.kind}:${p.input_field!.id}` === key);
    return x?.point_ms === y?.point_ms ? [] : [`${key}：A ${x ? timeText(x.point_ms!) : '无'} → B ${y ? timeText(y.point_ms!) : '无'}`];
  });
}
export function observedItems(observation: Props['observation'], run: RunResult | null, at: number | null): RuntimeItem[] | null {
  if (!run || at == null || !observation || observation.run_id !== run.id || observation.at_ms !== at || observation.next_index !== null) return null;
  return observation.objects.flatMap(o => o.kind === 'item' ? [o.data] : []);
}
/** The item-follow inspection is a current observation, unlike an explicitly selected historical event. */
export function resolveFollowedItemSelection(selected: TimelineSelection | null, observation: Props['observation'], run: RunResult | null, at: number | null): TimelineSelection | null {
  if (!selected || selected.source !== 'actual' || selected.object.kind !== 'item' || selected.event || selected.key !== `${selected.run_id}:item:${selected.object.id}`) return selected;
  if (!run || selected.run_id !== run.id) return null;
  const items = observedItems(observation, run, at), item = items?.find(item => item.id === selected.object.id);
  return { ...selected, at_ms: at ?? undefined,
    fields: item ? [
      { label: '持有者', value: `${item.holder.kind} ${item.holder.id}` },
      { label: '生命周期', value: item.lifecycle }, { label: '质量', value: item.quality },
      { label: '数量', value: quantityText(item.quantity) },
    ] : [{ label: '物品状态', value: items === null ? '正在读取同Run时点物品；暂不显示持有者。' : '此时点无该身份；不沿用其他时点库存。' }],
    related_objects: item ? [holderObject(item.holder)] : [],
  };
}
