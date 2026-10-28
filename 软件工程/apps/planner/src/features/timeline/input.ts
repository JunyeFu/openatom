import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { InputField, TimelineInputCommit } from './types';

export const fieldKey = (field: InputField) => `${field.kind}:${field.id}`;
export function fieldTime(scenario: ScenarioInput, field: InputField): number | null {
  if (field.kind === 'arrival') return scenario.arrivals.find(v => v.id === field.id)?.at_ms ?? null;
  if (field.kind === 'task_release') return scenario.production_tasks.find(v => v.id === field.id)?.release_at_ms ?? null;
  const order = scenario.orders.find(v => v.id === field.id);
  return (field.kind === 'order_due' ? order?.due_at_ms : order?.release_at_ms) ?? null;
}
export function inputProposal(project: ProjectState, scenario: ScenarioInput, fields: InputField[], delta: number): TimelineInputCommit {
  if (scenario.project_revision !== project.revision) throw new Error('方案与工程修订不同，请刷新输入后重新应用。');
  const unique = [...new Map(fields.map(f => [fieldKey(f), f])).values()];
  const changes = unique.map(field => {
    const before_ms = fieldTime(scenario, field);
    if (before_ms === null) throw new Error('选中的输入标记已经不存在，请刷新。');
    const after_ms = before_ms + delta;
    if (!Number.isSafeInteger(after_ms) || after_ms < 0) throw new Error('时间必须是非负的整数毫秒。');
    return { ...field, before_ms, after_ms };
  });
  const times = new Map(changes.map(c => [fieldKey(c), c.after_ms]));
  // Keep the potentially 3000-item initial state and unchanged arrival payloads
  // shared. Pointer moves project only the explicitly edited time fields.
  const next: ScenarioInput = { ...scenario,
    arrivals: scenario.arrivals.map(a => times.has(`arrival:${a.id}`) ? { ...a, at_ms: times.get(`arrival:${a.id}`)! } : a),
    production_tasks: scenario.production_tasks.map(t => times.has(`task_release:${t.id}`) ? { ...t, release_at_ms: times.get(`task_release:${t.id}`)! } : t),
    orders: scenario.orders.map(o => ({ ...o,
      release_at_ms: times.get(`order_release:${o.id}`) ?? o.release_at_ms,
      due_at_ms: times.get(`order_due:${o.id}`) ?? o.due_at_ms })) };
  // PutObject names the CURRENT Scenario/Project revision; the Host advances
  // revisions in its one atomic commit. Never pre-increment them in the UI.
  return { project_id: project.id, scenario_id: scenario.id, base_project_revision: project.revision,
    base_scenario_revision: scenario.revision, command: { kind: 'put_object', data: { kind: 'scenario', data: next } }, changes };
}
