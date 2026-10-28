import { useEffect, useRef, useState } from 'react';
import { cloudContextKey, recoverRegisteredCloudTask, type CloudContext, type CloudTaskRecord } from '../../services/cloudProduction';

interface Props {
  context: CloudContext;
  records: readonly CloudTaskRecord[];
  onTaskRegistered: (record: CloudTaskRecord) => void;
}

/** Historical input revision stays attached to its original request and Task. */
export function RetainedCloudTasks(props: Props) {
  const key = cloudContextKey(props.context);
  const pending = useRef<AbortController | null>(null);
  const [reading, setReading] = useState<string | null>(null);
  const [error, setError] = useState('');
  const [result, setResult] = useState<{ request: string; text: string } | null>(null);
  useEffect(() => {
    setReading(null); setError(''); setResult(null);
    return () => { pending.current?.abort(); pending.current = null; };
  }, [key]);
  const records = props.records.filter(record => record.operation !== 'read_cloud_production'
    && record.context.hostContextId === props.context.hostContextId
    && record.context.projectId === props.context.projectId
    && record.context.projectRevision !== props.context.projectRevision);
  if (!records.length) return null;

  async function recover(record: CloudTaskRecord) {
    pending.current?.abort();
    const controller = new AbortController(); pending.current = controller;
    setReading(record.request_id); setError(''); setResult(null);
    try {
      const output = await recoverRegisteredCloudTask(record, { signal: controller.signal, onTask: props.onTaskRegistered });
      if (!controller.signal.aborted) setResult({ request: record.request_id, text: output ? JSON.stringify(output, null, 2) : '原请求未登记；没有重复提交。' });
    } catch (e) {
      if (!controller.signal.aborted) setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (pending.current === controller) { pending.current = null; setReading(null); }
    }
  }

  return <aside className="cloud-retained-tasks" aria-label="原修订任务">
    <details><summary>原修订的准备与导出任务（{records.length}）</summary>
      <p>按原输入修订查询已有任务。停止观察或返回画布不会停止生产，也不会重复准备或导出。</p>
      {records.map(record => <div className="cloud-record" key={record.request_id}>
        <strong>工程 r{record.context.projectRevision}</strong> · {record.operation} · {record.status}
        <small>{record.request_id} · {record.task?.id ?? '登记回执尚未确认'}</small>
        <button type="button" disabled={reading !== null} onClick={() => void recover(record)}>查询此原请求</button>
      </div>)}
      {reading && <button type="button" onClick={() => { pending.current?.abort(); pending.current = null; setReading(null); }}>停止观察</button>}
      {error && <p role="alert">{error}</p>}
      {result && <details className="cloud-json" open><summary>原请求 {result.request} 的读取结果</summary><pre>{result.text}</pre></details>}
    </details>
  </aside>;
}
