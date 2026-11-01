import { useEffect, useRef, useState } from 'react';
import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { TaskOutput } from '../../../../../packages/commands/generated/typescript/TaskOutput';
import type { DeliveryServices } from '../comparison/types';
import { errorDetail, errorText } from '../comparison/model';
import { completedOutput, observeTask, terminal } from './task';

export function useDeliveryTask(services: DeliveryServices, projectId: string) {
  const [handle, setHandle] = useState<TaskHandle | null>(null);
  const [state, setState] = useState<TaskState | null>(null);
  const [expected, setExpected] = useState<TaskOutput['kind']>('evaluation');
  const [starting, setStarting] = useState(false); const [observing, setObserving] = useState(false);
  const [cancelling, setCancelling] = useState(false); const [notice, setNotice] = useState(''); const [failure, setFailure] = useState('');
  const [detail, setDetail] = useState<unknown>(null);
  const controller = useRef<AbortController | null>(null); const identity = useRef(projectId); identity.current = projectId;
  useEffect(() => { setHandle(null); setState(null); setNotice(''); setFailure(''); setStarting(false); setObserving(false);
    return () => controller.current?.abort();
  }, [projectId]);
  async function follow(task: TaskHandle, kind: TaskOutput['kind']) {
    controller.current?.abort(); const local = new AbortController(); controller.current = local;
    const project = projectId; setObserving(true); setFailure(''); setDetail(null);
    try {
      const last = await observeTask(task, services.readTask, s => { if (identity.current === project && !local.signal.aborted) setState(s); }, local.signal);
      if (identity.current !== project || local.signal.aborted) return;
      setNotice('');
      if (last.task.state === 'completed' && !completedOutput(last, kind)) setFailure('任务已结束，但未返回对应的实际文件结果；不能确认导出成功。');
      else if (last.task.state === 'failed') setFailure(last.failure ? `${last.failure.code}: ${last.failure.message}` : last.task.failure?.message ?? '宿主任务失败。');
      else if (last.task.state === 'cancelled') setNotice('宿主确认任务已取消；不表示文件已导出。');
      else setNotice('宿主任务完成，已读取实际结果。');
    } catch (e) { if (!local.signal.aborted && identity.current === project) { setFailure(errorText(e)); setDetail(errorDetail(e)); } }
    finally { if (!local.signal.aborted && identity.current === project) setObserving(false); }
  }
  async function start(work: () => Promise<TaskHandle | null>, kind: TaskOutput['kind']) {
    const project = projectId; setStarting(true); setFailure(''); setDetail(null); setNotice(''); setState(null); setHandle(null); setExpected(kind);
    try {
      const task = await work();
      if (identity.current !== project) return;
      if (!task) { setNotice('未选择目标，没有启动导出或保存任务。'); return; }
      setHandle(task); setNotice('请求已受理，正在读取原任务状态；尚未确认文件完成。');
      await follow(task, kind);
    } catch (e) { if (identity.current === project) { setFailure(errorText(e)); setDetail(errorDetail(e)); } }
    finally { if (identity.current === project) setStarting(false); }
  }
  async function cancel() {
    if (!handle) return;
    setCancelling(true); setFailure(''); setDetail(null);
    try {
      const result = await services.cancelTask(handle);
      if (identity.current !== projectId) return;
      if (result.task.id !== handle.id) throw new Error('取消响应不属于原任务。');
      setNotice(result.terminal_won ? '任务先到终态，继续读取最终结果。' : result.action_accepted ? '取消请求已接收，等待宿主确认终态。' : result.issues.map(i => i.message).join('；') || '宿主未接受取消请求。');
    } catch (e) { if (identity.current === projectId) { setFailure(errorText(e)); setDetail(errorDetail(e)); } }
    finally { if (identity.current === projectId) setCancelling(false); }
  }
  const active = starting || observing || !!handle && !terminal(state?.task.state ?? handle.state);
  return { handle, state, expected, active, observing, cancelling, notice, failure, detail, start, cancel,
    resume: () => handle && follow(handle, expected), result: completedOutput(state, expected) };
}
export type DeliveryTask = ReturnType<typeof useDeliveryTask>;
