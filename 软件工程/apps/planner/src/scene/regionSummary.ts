import type { LineGroup } from '../../../../packages/commands/generated/typescript/LineGroup';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { JobStatus } from '../../../../packages/commands/generated/typescript/JobStatus';
import type { FactoryPresentationSource } from '../services/factoryPresentation';
import { decimalText } from './runtime';

/** Aggregate only the explicit region members and already-read same-time records. */
export function regionSummary(project: ProjectState, line: LineGroup, source: FactoryPresentationSource | null, actualPending = false) {
  const members = project.devices.filter(device => device.line_id === line.id || line.members.includes(device.id));
  const ids = new Set(members.map(device => device.id));
  const boundary = project.connections.flatMap(connection => {
    if (connection.kind !== 'material') return [];
    const value = connection.data;
    const from = value.source.kind === 'port' && ids.has(value.source.data.instance_id);
    const to = value.destination.kind === 'port' && ids.has(value.destination.data.instance_id);
    return from === to ? [] : [{ id: value.id, label: (to ? '流入' : '流出') + ' · ' + value.item_type + ' / ' + value.unit }];
  });
  const actual = source?.run && source.observation ? source : null;
  const objects = actual?.raw.job_pages.length ? actual.raw.job_pages.flatMap(page => page.objects) : actual?.observation?.objects ?? [];
  const jobs = objects.flatMap(object => object.kind === 'job' && object.data.execution_ref.kind === 'project_device'
    && ids.has(object.data.execution_ref.data.device_id) ? [object.data] : []);
  const states = new Map<JobStatus, number>();
  for (const job of jobs) states.set(job.state, (states.get(job.state) ?? 0) + 1);
  const labels: Record<JobStatus, string> = { ready: '待执行', working: '执行', paused: '暂停', waiting_resource: '等资源', waiting_output: '等移交', completed: '完成', cancelled: '取消' };
  const at = actual?.observation?.at_ms;
  const jobIds = new Set(jobs.map(job => job.id));
  const resourceIds = new Set(actual?.run?.resource_claims.filter(claim => jobIds.has(claim.job_id)
    && at != null && claim.intervals.some(interval => interval.start_ms <= at && at < interval.end_ms)).map(claim => claim.resource_id) ?? []);
  const resources = project.resources.filter(resource => resourceIds.has(resource.id)).map(resource => ({ id: resource.id, title: resource.title }));
  const ports = line.boundary_ports.map(port => {
    const output = actual?.devices.find(device => device.device_id === port.instance_id)?.port_outputs?.ports.find(value =>
      value.port.port_id === port.port_id && value.port.module_ref.id === port.module_ref.id && value.port.module_ref.version === port.module_ref.version);
    return { port, label: output?.status === 'available'
      ? '做完 ' + decimalText(output.completed_output.value) + ' ' + output.completed_output.unit + ' · 移交 ' + decimalText(output.accepted_output.value) + ' ' + output.accepted_output.unit
      : output?.status === 'unavailable' ? output.message : '未读到此精确边界口的实际数量' };
  });
  return { members, boundary, resources, ports, actual: !!actual,
    state: actual ? '已读作业 ' + jobs.length + (states.size ? ' · ' + [...states].map(([state, count]) => labels[state] + count).join(' / ') : '')
      : actualPending ? '正在读取当前时点实际作业 · 暂不汇总' : '声明区域 · 无实际运行数据' };
}
