// Isolated consumers of canonical generated DTOs; no frontend or production Engine.
import type { RunResult } from './output/typescript/RunResult';
import type { RunView } from './output/typescript/RunView';
import type { JobExecutionRef } from './output/typescript/JobExecutionRef';

export function fullResult(view: RunView): RunResult | null {
  return view.kind === 'full' ? view.data : null;
}

export function executionDevice(execution: JobExecutionRef): string | null {
  return execution.kind === 'project_device' ? execution.data.device_id : null;
}

export function capacityClaims(run: RunResult, jobId: string) {
  const job = run.jobs.find(job => job.id === jobId);
  if (!job || job.execution_ref.kind !== 'project_device') return null;
  const capacity = job.execution_ref.data.execution_capacity_ref;
  return run.resource_claims.filter(claim => claim.job_id === jobId && claim.resource_id === capacity.id);
}

export function phaseBands(run: RunResult, jobId: string, phaseId: string) {
  return run.phase_work_intervals.filter(part => part.job_id === jobId && part.phase_id === phaseId);
}

export function effectiveProgressAt(run: RunResult, jobId: string, phaseId: string, atMs: number): number | null {
  const frontier = run.computed_until_ms;
  if (frontier == null || atMs > frontier) return null;
  const parts = phaseBands(run, jobId, phaseId);
  const active = parts.find(part => part.interval.start_ms <= atMs && atMs < part.interval.end_ms);
  if (active) return active.effective_progress_start_ms + atMs - active.interval.start_ms;
  const prior = parts.filter(part => part.interval.end_ms <= atMs).sort((a, b) => b.interval.end_ms - a.interval.end_ms)[0];
  return prior ? prior.effective_progress_end_ms : null;
}

export function waitPhase(run: RunResult, index: number) {
  const wait = run.waits[index];
  return { execution: wait.execution_ref, blockedPhase: wait.phase_ref };
}
