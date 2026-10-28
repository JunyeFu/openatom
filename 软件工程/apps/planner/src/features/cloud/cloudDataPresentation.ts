import type { DaaSQueryResult } from '../../../../../packages/commands/generated/typescript/DaaSQueryResult';

export type CloudDataRow = DaaSQueryResult['rows'][number];
export type Cell = string | number | boolean | null;
export interface DataColumn { path: string; label: string; unit: string }
export interface MinuteSeries { deviceId: string; sourceMode: string; points: { bucket: string; count: number }[] }

const fields: Record<string, [string, string]> = {
  kind: ['记录类型', '—'], ingress_id: ['云接收序号', '—'], row_index: ['结果行位置', '—'],
  dataset_id: ['数据集', '—'], cloud_received_at: ['云接收时间', 'UTC'], bucket_start: ['分钟起点', 'UTC'],
  device_id: ['设备标识', '—'], device_ref: ['设备标识', '—'], source_mode: ['数据来源', '—'],
  source_purpose: ['数据用途', '—'], device_timestamp: ['设备自报时间', '原设备表示'],
  device_sequence: ['设备上报序号', '—'], device_boot_id: ['设备启动标识', '—'],
  session_id: ['执行会话', '—'], execution_epoch_id: ['执行轮次', '—'], execution_revision: ['执行修订', '—'],
  accept_new_work: ['允许接收新工作', '—'], device_state: ['设备状态', '—'], active_command_id: ['当前命令', '—'],
  observation_probe_id: ['观测请求', '—'], contract_version: ['契约版本', '—'],
  count: ['记录数量', '条'], quantity: ['数量', '值内附单位'], observed_at: ['观测时间', 'UTC'],
  metric: ['指标', '—'], value: ['指标值', '依指标定义'], reason: ['等待原因', '—'],
  known_host_ms: ['已知等待时长', 'ms'], missing_conditions: ['缺失条件', '—'],
  logical_job_id: ['逻辑作业', '—'], state: ['状态', '—'], destination: ['接受去向', '—'],
  acceptance_stage: ['接受阶段', '—'], project_id: ['工程标识', '—'], host_context_session_id: ['Host会话', '—'],
};
const sourceModes: Readonly<Record<string, string>> = { software_in_loop: '软件在环', physical_device: '物理设备', generated_load: '生成负载' };
export const sourceModeText = (mode: string) => sourceModes[mode] ?? mode;
const rowKinds: Record<CloudDataRow['kind'], string> = { telemetry: '设备观测', generated_load: '生成负载事件', count_by_device_minute: '设备分钟计数', execution_attempt: '执行尝试', acceptance: '接受事件', accepted_quantity: '接受量', command_count: '命令数量', waiting_time: '等待时长' };
export const rowKindText = (kind: CloudDataRow['kind']) => rowKinds[kind];

/** Presentation only. Original rows, order, units and the fixed export selection are unchanged. */
export function rowCells(row: CloudDataRow): Record<string, Cell> {
  const cells: Record<string, Cell> = {};
  function visit(value: unknown, path: string) {
    if (value === null || typeof value !== 'object') { cells[path] = value as Cell; return; }
    if (Array.isArray(value)) { cells[path] = JSON.stringify(value); return; }
    const object = value as Record<string, unknown>;
    const decimal = object.value as { coefficient: number; decimal_places: number } | undefined;
    if (typeof object.unit === 'string' && decimal && typeof decimal.coefficient === 'number' && typeof decimal.decimal_places === 'number') {
      const digits = String(decimal.coefficient).padStart(decimal.decimal_places + 1, '0');
      cells[path] = `${decimal.decimal_places ? digits.slice(0, -decimal.decimal_places) + '.' + digits.slice(-decimal.decimal_places) : digits} ${object.unit}`;
      return;
    }
    for (const [key, child] of Object.entries(object)) visit(child, path ? `${path}.${key}` : key);
  }
  visit(row, '');
  return cells;
}
export function dataColumns(rows: readonly CloudDataRow[]): DataColumn[] {
  const paths = new Set(rows.flatMap(row => Object.keys(rowCells(row))));
  const preferred = ['kind', 'device_id', 'observation.device_id', 'event.device_ref', 'cloud_received_at', 'bucket_start', 'observation.device_state', 'source_mode', 'quantity', 'event.quantity', 'known_host_ms', 'reason', 'count'];
  const order = (path: string) => preferred.includes(path) ? preferred.indexOf(path) : preferred.length;
  return [...paths].sort((a, b) => order(a) - order(b)).map(path => {
    const leaf = path.split('.').at(-1)!;
    const definition = fields[leaf];
    return { path, label: definition?.[0] ?? leaf, unit: definition?.[1] ?? (leaf.endsWith('_ms') ? 'ms' : '—') };
  });
}
export function displayCell(cell: Cell | undefined): string {
  if (cell === undefined) return '—';
  if (cell === null) return '未知';
  if (typeof cell === 'boolean') return cell ? '是' : '否';
  return String(cell);
}
export function minuteSeries(rows: readonly CloudDataRow[]): MinuteSeries[] {
  const groups = new Map<string, MinuteSeries>();
  for (const row of rows) {
    if (row.kind !== 'count_by_device_minute') continue;
    const key = JSON.stringify([row.device_id, row.source_mode]);
    let group = groups.get(key);
    if (!group) { group = { deviceId: row.device_id, sourceMode: row.source_mode, points: [] }; groups.set(key, group); }
    group.points.push({ bucket: row.bucket_start, count: row.count });
  }
  for (const group of groups.values()) group.points.sort((a, b) => a.bucket.localeCompare(b.bucket));
  return [...groups.values()];
}
