import { useState } from 'react';
import type { JobState } from '../../../../../packages/commands/generated/typescript/JobState';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import { WorkbenchModal } from './WorkbenchModal';

export interface ProcessSampleChoice {
  run_id: string;
  definition_ref: ModuleRef;
  draft_revision: number;
  jobs: JobState[];
  select(job_id: string): Promise<void>;
}
const states: Record<JobState['state'], string> = {
  ready: '就绪', working: '加工中', paused: '已暂停', waiting_resource: '等待资源',
  waiting_output: '等待输出接收', completed: '已完成', cancelled: '已取消',
};

export function ProcessSampleChooser({ choice, onDismiss }: { choice: ProcessSampleChoice; onDismiss(): void }) {
  const [jobId, setJobId] = useState(''), [busy, setBusy] = useState(false), [error, setError] = useState('');
  async function submit() {
    if (!jobId || busy) return;
    setBusy(true); setError('');
    try { await choice.select(jobId); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setBusy(false); }
  }
  return <WorkbenchModal labelId="process-sample-choice-title" busy={busy} onDismiss={onDismiss}>
    <form className="modal" onSubmit={event => { event.preventDefault(); void submit(); }}>
      <h2 id="process-sample-choice-title">选择样例作业</h2>
      <p>Run {choice.run_id}</p>
      <p>{choice.definition_ref.id}@{choice.definition_ref.version} · 草稿 r{choice.draft_revision}</p>
      <fieldset disabled={busy} style={{ minWidth: 0 }}><legend>此精确来源有多个实际作业，请明确选择观看对象。</legend>
        {choice.jobs.map(job => <label className="check-label" key={job.id}>
          <input type="radio" name="process-sample-job" value={job.id} checked={jobId === job.id} onChange={() => { setJobId(job.id); setError(''); }} />
          <span style={{ overflowWrap: 'anywhere' }}><strong>{job.id}</strong><br />任务 {job.task_ref} · {states[job.state]}<br />
            工艺 {job.operation_ref.id}@{job.operation_ref.version}<br />输入物品 {job.input_refs.join('、') || '此刻未持有输入物品'}</span>
        </label>)}
      </fieldset>
      {error && <p role="alert" className="error-message">{error}</p>}
      <p className="form-hint">选择只打开此 Run 的实际过程，不重新运行或改变库存。取消后原 Run 仍保留。</p>
      <footer><button type="button" autoFocus disabled={busy} onClick={onDismiss}>取消选择</button>
        <button type="submit" disabled={!jobId || busy}>{busy ? '核对来源…' : '查看所选作业'}</button></footer>
    </form>
  </WorkbenchModal>;
}
