import type { RuntimeItem } from '../../../../packages/commands/generated/typescript/RuntimeItem';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { DecimalAmount } from '../../../../packages/commands/generated/typescript/DecimalAmount';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { TransportState } from '../../../../packages/commands/generated/typescript/TransportState';

export function transportPhaseAt(trip: TransportState, atMs: number | null): string {
  if (atMs == null) return '未知时点';
  if (trip.departed_ms == null || atMs < trip.departed_ms) return '尚未发出';
  if (trip.arrived_ms == null || atMs < trip.arrived_ms) return '在途';
  if (trip.received_ms == null || atMs < trip.received_ms) return '等待目的接收';
  if (trip.returned_ms == null || atMs < trip.returned_ms) return '返程中';
  return '已返库';
}

export function decimalText(value: DecimalAmount) {
  const digits = String(value.coefficient).padStart(value.decimal_places + 1, '0');
  return value.decimal_places === 0 ? digits : `${digits.slice(0, -value.decimal_places)}.${digits.slice(-value.decimal_places)}`;
}

// Logical holders are actual Engine state. A transport with no recorded world
// position stays unlocated; elapsed time never synthesizes a path or velocity.
export function itemAnchor(item: RuntimeItem, items: RuntimeItem[], run: RunResult): ObjectRef | null {
  const visited = new Set<string>();
  let current: RuntimeItem | undefined = item;
  while (current) {
    if (visited.has(current.id)) return null;
    visited.add(current.id);
    switch (current.holder.kind) {
      case 'location': return { kind: 'location', id: current.holder.id };
      case 'transport': return null;
      case 'job': {
        const execution = run.jobs.find(job => job.id === current!.holder.id)?.execution_ref;
        return execution?.kind === 'project_device' ? { kind: 'device', id: execution.data.device_id } : null;
      }
      case 'container': current = items.find(value => value.id === current!.holder.id); break;
    }
  }
  return null;
}

export function objectHall(project: ProjectState, object: ObjectRef, items: RuntimeItem[], run: RunResult | null): string | null {
  let anchor: ObjectRef | null = object;
  if (object.kind === 'item') {
    const item = items.find(value => value.id === object.id);
    anchor = item && run ? itemAnchor(item, items, run) : null;
  } else if (object.kind === 'job') {
    const execution = run?.jobs.find(job => job.id === object.id)?.execution_ref;
    anchor = execution?.kind === 'project_device' ? { kind: 'device', id: execution.data.device_id } : null;
  }
  if (anchor?.kind === 'hall') return project.halls.find(hall => hall.id === anchor.id)?.id ?? null;
  if (anchor?.kind === 'line') return project.lines.find(line => line.id === anchor.id)?.hall_id ?? null;
  if (anchor?.kind === 'device') return project.devices.find(device => device.id === anchor.id)?.hall_id ?? null;
  if (anchor?.kind === 'location') return project.locations.find(location => location.id === anchor.id)?.hall_id ?? null;
  return null;
}

const activityHistories = new WeakMap<RunResult, { jobs: Map<string, Set<string>>; work: Map<string, RunResult['phase_work_intervals']>; waits: Map<string, RunResult['waits']> }>();
export function deviceActivity(run: RunResult, atMs: number, deviceId: string) {
  let index = activityHistories.get(run);
  if (!index) {
    index = { jobs: new Map(), work: new Map(), waits: new Map() };
    for (const job of run.jobs) if (job.execution_ref.kind === 'project_device') { const id = job.execution_ref.data.device_id, ids = index.jobs.get(id) ?? new Set(); ids.add(job.id); index.jobs.set(id, ids); }
    for (const row of run.phase_work_intervals) { const rows = index.work.get(row.job_id) ?? []; rows.push(row); index.work.set(row.job_id, rows); }
    for (const row of run.waits) if (row.execution_ref?.kind === 'project_device') { const id = row.execution_ref.data.device_id, rows = index.waits.get(id) ?? []; rows.push(row); index.waits.set(id, rows); }
    activityHistories.set(run, index);
  }
  const ids = index.jobs.get(deviceId) ?? new Set<string>();
  const work = new Set([...ids].flatMap(id => index.work.get(id) ?? []).filter(interval => interval.interval.start_ms <= atMs && atMs < interval.interval.end_ms).map(interval => interval.job_id));
  const waits = (index.waits.get(deviceId) ?? []).filter(wait => wait.interval.start_ms <= atMs && atMs < wait.interval.end_ms);
  return { work: work.size, waits: waits.length };
}
