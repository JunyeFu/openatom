import type { DeliveryTask } from './useDeliveryTask';
import { JsonDetails } from '../comparison/Shared';
export function TaskReceipt({ task }: { task: DeliveryTask }) {
  const value = task.state?.task ?? task.handle;
  const progress = value?.progress;
  return <section className="delivery-task" aria-label="交付任务状态" aria-live="polite">
    <strong>{value ? `交付任务 · ${{ queued: '排队中', running: '处理中', paused: '已暂停', cancel_requested: '请求取消中', completed: '已结束', failed: '失败', cancelled: '已取消' }[value.state]}` : '尚未启动交付任务'}</strong>
    {value && <JsonDetails label="任务身份、输入修订与进度来源" value={{ id: value.id, input_revision: value.input_revision, kind: value.kind, progress }} />}
    {progress && <div><progress aria-label="实际任务进度" value={progress.total != null && progress.total > 0 ? progress.processed : undefined} max={progress.total != null && progress.total > 0 ? progress.total : undefined} /><span>已处理 {progress.processed} / {progress.total ?? '总量未知'} {progress.message && /[\u3400-\u9fff]/u.test(progress.message) ? progress.message : ''}</span></div>}
    {task.notice && <p>{task.notice}</p>}{task.failure && <p role="alert" className="delivery-error">{task.failure}</p>}
    {task.handle && task.active && <div className="delivery-actions"><button disabled={task.cancelling || !task.state?.allowed_controls.includes('cancel')} onClick={() => void task.cancel()}>请求取消</button>{!task.observing && <button onClick={() => void task.resume()}>重新读取原任务</button>}</div>}
    {task.state?.failure && <JsonDetails label="宿主原始错误与字段定位" value={task.state.failure} />}
    {task.detail != null && task.detail !== task.state?.failure && <JsonDetails label="服务原始错误与字段定位" value={task.detail} />}
  </section>;
}
