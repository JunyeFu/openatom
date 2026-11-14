import type { RunSummary } from '../../../../../packages/commands/generated/typescript/RunSummary';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import { dispatch } from '../../services/projectHost';

/** Read the recorded Scenario directly; a display name never needs the full Run or module catalogue. */
export async function readFrozenRunScenario(projectId: string, summary: RunSummary): Promise<ScenarioInput> {
  const object = await dispatch<DomainObject>({ api_version: '0.1', request_id: crypto.randomUUID(), project_id: projectId, scope: { kind: 'project' }, operation: 'get_object', params: {
    object_id: { kind: 'scenario', id: summary.input.scenario_id }, source: { kind: 'run_snapshot', data: { run_id: summary.run_id, at_ms: summary.computed_until_ms ?? summary.window.start_ms } }, node_index: null,
  } });
  if (object.kind !== 'scenario' || object.data.id !== summary.input.scenario_id || object.data.revision !== summary.input.scenario_revision || object.data.project_revision !== summary.input.project_revision) throw new Error('运行记录名称的冻结方案来源不一致。');
  return object.data;
}
