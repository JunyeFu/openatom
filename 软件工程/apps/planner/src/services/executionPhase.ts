import type { CompleteExecutionObservation } from './executionPresentation';
import type { FactoryObservedPhase } from './factoryPresentation';

/** Match the real bound Job, attempt, device boot and sampled command. No Host-time interpolation. */
export function executionDevicePhases(observation: CompleteExecutionObservation): Map<string, FactoryObservedPhase> {
  const jobs = observation.objects.flatMap(o => o.kind === 'job' ? [o.data] : []);
  const attempts = observation.objects.flatMap(o => o.kind === 'attempt' ? [o.data] : []);
  const result = new Map<string, FactoryObservedPhase>();
  for (const object of observation.objects) {
    if (object.kind !== 'device') continue;
    const device = object.data, id = device.binding.device_id;
    const mapped = jobs.filter(j => j.execution_ref.kind === 'project_device' ? j.execution_ref.data.device_id === id
      : attempts.some(a => a.attempt.logical_job_id === j.logical_job_id && a.device_id === id));
    const active = mapped.filter(j => j.state !== 'completed' && j.state !== 'cancelled');
    const job = active.length === 1 ? active[0] : active.length === 0 && mapped.length === 1 && mapped[0].state === 'completed' ? mapped[0] : null;
    if (!job) continue;
    const view: FactoryObservedPhase = { job_id: job.job_id, operation_ref: device.binding.operation_ref, label: '工序待确认 · 阶段进度未提供', point: null };
    result.set(id, view);
    const owned = attempts.filter(a => a.attempt.logical_job_id === job.logical_job_id && a.device_id === id);
    if (job.state === 'completed' || job.state === 'waiting_output') {
      const completed = owned.filter(a => a.completed_at_host_ms != null
        && a.completed_at_host_ms <= observation.at_host_ms && (job.finished_at_host_ms == null || a.completed_at_host_ms === job.finished_at_host_ms));
      if (completed.length !== 1) continue;
      view.label = job.state === 'waiting_output' ? '工序完成 · 等待出料' : '工序完成';
      view.point = { known: true, display_phase_id: completed[0].attempt.phase_id, progress: 1, terminal_pose: true, effective_ms: null, duration_ms: null };
      continue;
    }
    if (job.state === 'paused') { view.label = '已暂停 · 保留在制品'; continue; }
    if (job.state !== 'working') continue;
    view.label = '加工中 · 阶段进度未提供';
    const sample = device.latest_observation, report = sample?.observation.phase_progress;
    if (!sample || !report || sample.host_received_elapsed_ms > observation.at_host_ms || sample.host_received_elapsed_ms < job.started_at_host_ms) continue;
    const attempt = owned.filter(a => a.state === 'device_started' && a.attempt.command_id === sample.observation.active_command_id
      && a.target_device_boot_id === sample.observation.device_boot_id);
    if (attempt.length !== 1 || sample.observation.device_state !== 'busy' || report.phase_id !== attempt[0].attempt.phase_id || report.phase_id !== job.phase_id) continue;
    view.label = device.freshness === 'fresh' ? '加工中 · 设备报告阶段' : '加工中 · 保留最近阶段报告';
    view.point = { known: true, display_phase_id: report.phase_id, progress: report.effective_work_ms / report.duration_ms,
      terminal_pose: false, effective_ms: report.effective_work_ms, duration_ms: report.duration_ms };
  }
  return result;
}
