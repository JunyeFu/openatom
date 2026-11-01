import { authoringRequest } from './model';
import type { AuthoringIO } from './types';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { RunView } from '../../../../../packages/commands/generated/typescript/RunView';
import type { RunInputRef } from '../../../../../packages/commands/generated/typescript/RunInputRef';

export interface DefinitionSampleContext { project_id: string | null; session_id: string; draft_id: string | null }
export function sameSampleContext(left: DefinitionSampleContext, right: DefinitionSampleContext) {
  return left.project_id === right.project_id && left.session_id === right.session_id && left.draft_id === right.draft_id;
}
export function sampleHasRun(task: TaskState) { return task.task.state === 'completed' && task.result?.kind === 'run'; }
function sameInput(left: RunInputRef, right: RunInputRef) {
  return left.project_id === right.project_id && left.project_revision === right.project_revision
    && left.scenario_id === right.scenario_id && left.scenario_revision === right.scenario_revision
    && left.frozen_project_file.kind === right.frozen_project_file.kind && left.frozen_project_file.path === right.frozen_project_file.path
    && left.frozen_scenario_file.kind === right.frozen_scenario_file.kind && left.frozen_scenario_file.path === right.frozen_scenario_file.path;
}
/** A read of the original completed Task's private Run, through the existing SDK. */
export async function readDefinitionSampleRun(io: AuthoringIO, projectId: string, task: TaskState, isCurrent: () => boolean): Promise<RunResult | null> {
  if (!isCurrent() || task.task.state !== 'completed' || task.result?.kind !== 'run') return null;
  const scope = task.task.scope;
  if (scope.kind !== 'edit' || scope.data.kind !== 'definition_draft' || scope.data.data.revision !== task.task.input_revision) throw new Error('此任务未保留同一精确定义草稿输入，不能改写为工程 Scope 读取结果。');
  const summary = task.result.data;
  const view = await io.dispatch<RunView>(authoringRequest(projectId, { kind: 'definition_draft', data: scope.data.data.id }, 'get_run', { run_id: summary.run_id, query: { kind: 'full_result' } }));
  if (!isCurrent()) return null;
  if (view.kind !== 'full' || view.data.id !== summary.run_id || !sameInput(view.data.input_ref, summary.input)) throw new Error('Host 未返回此任务的完整冻结 Run 与原输入引用；不会用摘要或另一个 Run 代替。');
  // DraftR and Run.input_ref.project_revision are different revision spaces.
  return view.data;
}
