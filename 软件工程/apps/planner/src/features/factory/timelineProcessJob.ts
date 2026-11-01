import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { TimelineSelection } from '../timeline/types';

/** An explicit timeline Job may be queued while another Job uses its device. */
export function timelineProcessJob(selection: TimelineSelection | null, run: RunResult | null) {
  if (!selection || selection.source !== 'actual' || !run || selection.run_id !== run.id) return null;
  const id = selection.object.kind === 'job' ? selection.object.id : selection.job?.id;
  const job = run.jobs.find(value => value.id === id);
  return job?.execution_ref.kind === 'project_device' ? job : null;
}
