import type { RunSummary } from '../../../../packages/commands/generated/typescript/RunSummary';
import type { ScenarioInput } from '../../../../packages/commands/generated/typescript/ScenarioInput';

export const runStatusLabel: Record<RunSummary['computation_status'], string> = {
  queued: '等待运行', running: '运行中', paused: '已暂停', cancel_requested: '正在停止',
  completed: '运行完成', failed: '运行失败', cancelled: '已取消',
};
export const taskStateLabel: Record<RunSummary['computation_status'], string> = {
  queued: '等待处理', running: '处理中', paused: '已暂停', cancel_requested: '正在取消',
  completed: '已完成', failed: '失败', cancelled: '已取消',
};
export const scenarioName = (scenario: ScenarioInput, index = 0) => scenario.title.trim() || `方案 ${index + 1}`;

export function runRecordLabel(summary: RunSummary, index: number, scenarios: readonly ScenarioInput[], frozen?: ScenarioInput | null): string {
  const matches = (scenario: ScenarioInput) => scenario.id === summary.input.scenario_id && scenario.revision === summary.input.scenario_revision;
  const scenario = frozen && matches(frozen) ? frozen : scenarios.find(matches);
  const name = scenario ? scenarioName(scenario) : '历史方案';
  const time = summary.computed_until_ms === null ? '尚无结果' : `至 ${summary.computed_until_ms / 1000} 秒`;
  return `记录 ${index + 1} · ${name} · ${time} · ${runStatusLabel[summary.computation_status]}`;
}

export function historyBoundaryLabel(reason: string): string {
  const labels: Record<string, string> = {
    'No active project history': '当前尚无工程编辑历史。',
    'No reversible draft edits in this session': '本次打开工程后，还没有可撤销的草稿编辑。',
    'Professional facts, human acknowledgment and explicit binding are outside ordinary undo': '专业计算事实、人工确认和明确绑定不能通过草稿撤销更改。',
  };
  return labels[reason] ?? (/[\u3400-\u9fff]/.test(reason) ? reason : '当前更改超出可撤销范围，原始原因见历史详情。');
}
