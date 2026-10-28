import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { TaskOutput } from '../../../../../packages/commands/generated/typescript/TaskOutput';
export const terminal = (state: string) => ['completed', 'failed', 'cancelled'].includes(state);
export function completedOutput(state: TaskState | null, expected: TaskOutput['kind']): TaskOutput | null {
  if (state?.task.state !== 'completed' || state.result?.kind !== expected) return null;
  const result = state.result;
  if (result.kind === 'evaluation' && !result.data.files.length) return null;
  if (result.kind === 'package' && !result.data.package.path) return null;
  if (result.kind === 'save' && !result.data.manifest.path) return null;
  return result;
}
/** Observe the original task; stopping observation is never business cancellation. */
export async function observeTask(handle: TaskHandle, read: (handle: TaskHandle) => Promise<TaskState>,
  publish: (state: TaskState) => void, signal: AbortSignal,
  pause = () => new Promise<void>(resolve => setTimeout(resolve, 100))): Promise<TaskState> {
  while (true) {
    signal.throwIfAborted();
    const state = await read(handle);
    signal.throwIfAborted();
    if (state.task.id !== handle.id || JSON.stringify(state.task.scope) !== JSON.stringify(handle.scope)) {
      throw new Error('宿主返回了另一任务或作用域，未采用其状态。');
    }
    publish(state);
    if (terminal(state.task.state)) return state;
    await pause();
  }
}
