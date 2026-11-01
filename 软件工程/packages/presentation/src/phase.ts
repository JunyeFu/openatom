import type { ActualMediaFrame, PhasePoint } from './types';
type Run = ActualMediaFrame['run'];
type Snapshot = ActualMediaFrame['snapshot'];
// Run results and returned snapshots are immutable; index their actual records once.
const histories = new WeakMap<Run, {
  waits: Map<string, Run['waits']>; claims: Map<string, Run['resource_claims']>;
  work: Map<string, Run['phase_work_intervals']>; completed: Map<string, Run['events']>;
}>();
const snapshots = new WeakMap<Snapshot, { jobs: Map<string, PhasePoint['job']>; items: PhasePoint['items'] }>();
function history(run: Run) {
  let value = histories.get(run);
  if (!value) {
    value = { waits: new Map(), claims: new Map(), work: new Map(), completed: new Map() };
    for (const row of run.waits) if (row.object_ref.kind === 'job') { const list = value.waits.get(row.object_ref.id) ?? []; list.push(row); value.waits.set(row.object_ref.id, list); }
    for (const row of run.resource_claims) { const list = value.claims.get(row.job_id) ?? []; list.push(row); value.claims.set(row.job_id, list); }
    for (const row of run.phase_work_intervals) { const list = value.work.get(row.job_id) ?? []; list.push(row); value.work.set(row.job_id, list); }
    for (const row of run.events) if (row.subject.kind === 'job' && row.payload.kind === 'phase_completed') { const list = value.completed.get(row.subject.id) ?? []; list.push(row); value.completed.set(row.subject.id, list); }
    histories.set(run, value);
  }
  return value;
}
function snapshotRecords(snapshot: Snapshot) {
  let value = snapshots.get(snapshot);
  if (!value) {
    value = { jobs: new Map(), items: [] };
    for (const object of snapshot.objects) {
      if (object.kind === 'job' && !value.jobs.has(object.data.id)) value.jobs.set(object.data.id, object.data);
      else if (object.kind === 'item') value.items.push(object.data);
    }
    snapshots.set(snapshot, value);
  }
  return value;
}
/** A read-only view of authoritative snapshots and recorded effective-work intervals. */
export function projectPhase(frame: ActualMediaFrame): PhasePoint {
  const { run, snapshot, job_id, operation } = frame, at = snapshot.at_ms;
  const point: PhasePoint = { known: false, at_ms: at, job: null, phase_id: null, display_phase_id: null, effective_ms: null, duration_ms: null, progress: null, terminal_pose: false, paused: false, waits: [], claims: [], items: [], issues: [] };
  if (snapshot.run_id !== run.id || snapshot.next_index !== null) { point.issues.push('实际快照不属于这个 Run，或对象分页尚未完整返回。'); return point; }
  if (run.computed_until_ms == null || at < run.window.start_ms || at > run.computed_until_ms || at > run.window.end_ms) { point.issues.push('所选时点超出已知计算边界；当前位置未知，不补为空闲。'); return point; }
  const records = snapshotRecords(snapshot), job = records.jobs.get(job_id);
  if (!job) { point.issues.push('实际快照没有返回所选作业，不能从最终状态反推此刻。'); return point; }
  const indexed = history(run); point.job = job; point.known = true; point.phase_id = job.phase_ref ?? null;
  point.paused = run.computation_status === 'paused' && at === run.computed_until_ms;
  const frontier = at === run.computed_until_ms;
  const waiting = job.state === 'waiting_resource' || job.state === 'waiting_output';
  point.waits = (indexed.waits.get(job.id) ?? []).filter(wait => wait.interval.start_ms <= at && (at < wait.interval.end_ms || frontier && waiting && at === wait.interval.end_ms && ['computed_boundary','observation_end'].includes(wait.end_cause) && (wait.phase_ref == null || wait.phase_ref === job.phase_ref)));
  const held = new Set([...job.job_resources,...job.phase_resources]);
  point.claims = (indexed.claims.get(job.id) ?? []).filter(claim => claim.intervals.some((interval,index) => interval.start_ms <= at && (at < interval.end_ms || frontier && held.has(claim.resource_id) && at === interval.end_ms && ['computed_boundary','observation_end'].includes(claim.interval_end_causes[index]))));
  point.items = [...records.items];
  const work = (indexed.work.get(job.id) ?? []).filter(value => value.interval.start_ms <= at);
  // Zero-work phases have completion events but deliberately no positive work interval.
  const completed = (indexed.completed.get(job.id) ?? []).filter(event => event.at_ms <= at).at(-1);
  const priorPhase = completed?.payload.kind === 'phase_completed' ? completed.payload.data : null;
  const prior = priorPhase ? work.find(value => value.phase_id === priorPhase && value.end_cause === 'phase_completed') : null;
  let phase = point.phase_id, progress = job.effective_progress_ms;
  const startedCurrent = phase && work.some(value => value.phase_id === phase);
  if (priorPhase && (!phase || ((!startedCurrent && job.effective_progress_ms === 0) && job.state !== 'working'))) {
    phase = priorPhase; progress = prior?.effective_progress_end_ms ?? 0; point.terminal_pose = true;
  }
  if (!phase && !priorPhase && job.started_at_ms == null) phase = operation.phases[0]?.id ?? (operation.duration ? 'cycle' : null);
  point.display_phase_id = phase; point.effective_ms = progress;
  if (job.state === 'waiting_output' || job.state === 'completed') point.terminal_pose = true;
  if (!phase) { point.issues.push('当前实际阶段未给出；不使用整项作业的总时长冒充阶段。'); return point; }
  const declared = operation.phases.find(value => value.id === phase)?.duration ?? (operation.phases.length === 0 ? operation.duration : null);
  const complete = work.find(value => value.phase_id === phase && value.end_cause === 'phase_completed');
  const duration = complete?.effective_progress_end_ms ?? (declared && declared.basis !== 'unknown' ? declared.value : null);
  point.duration_ms = duration;
  if (duration == null) point.issues.push('当前阶段总有效工时未知；不按整项时长缩放片段。');
  else point.progress = duration === 0 ? (point.terminal_pose ? 1 : 0) : Math.min(1, Math.max(0, progress / duration));
  return point;
}
