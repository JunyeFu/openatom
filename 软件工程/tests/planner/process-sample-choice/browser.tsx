// Actual React/dialog controls; fixed select port exercises source rejection.
// Actual two-job Host/Run behavior is checked separately by host.test.mjs.
import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ProcessSampleChooser, type ProcessSampleChoice } from '../../../apps/planner/src/features/factory/ProcessSampleChooser';
import '../../../apps/planner/src/styles/workbench.css';
const checks: string[] = [], selected: string[] = [];
let reject = true, waiting: (() => void) | null = null, dismissals = 0;
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition: unknown, message: string) => { if (!condition) throw new Error(message); };
async function until(test: () => unknown) { for (let i = 0; i < 200; i++) { if (test()) return; await delay(20); } throw new Error('Timeout: ' + document.body.innerText); }
const first = { id: 'first-completed', task_ref: 'TASK-A', operation_ref: { id: 'op', version: '0.1.0' },
  execution_ref: { kind: 'project_device' as const, data: { device_id: 'ASSEMBLY' } }, input_refs: ['shell-1', 'pcb-1'],
  job_resources: [], phase_resources: [], effective_progress_ms: 30000, state: 'completed' as const, pending_outputs: [] };
function App() {
  const [open, setOpen] = useState(false);
  const choice: ProcessSampleChoice = { run_id: 'immutable-two-job-run', definition_ref: { id: 'assembly', version: '0.1.0' }, draft_revision: 7,
    jobs: [first, { ...first, id: 'second-waiting', task_ref: 'TASK-B', input_refs: ['shell-2'], state: 'waiting_resource' }],
    async select(id) {
      selected.push(id);
      if (reject) throw new Error('实际样例已产生，但Host中的原定义草稿修订已变化');
      await new Promise<void>(resolve => { waiting = resolve; }); setOpen(false);
    } };
  return <><button id="opener" onClick={() => setOpen(true)}>打开样例选择</button>{open && <ProcessSampleChooser choice={choice} onDismiss={() => { dismissals++; setOpen(false); }} />}</>;
}
createRoot(document.getElementById('root')!).render(<App />);
async function main() {
  await until(() => document.getElementById('opener'));
  const opener = document.getElementById('opener') as HTMLButtonElement; opener.focus(); opener.click();
  await until(() => document.querySelector('dialog[open]'));
  let radios = [...document.querySelectorAll<HTMLInputElement>('input[type="radio"]')];
  let submit = document.querySelector<HTMLButtonElement>('button[type="submit"]')!;
  assert(radios.length === 2 && radios.every(radio => !radio.checked) && submit.disabled, 'No implicit first Job selection');
  checks.push('Two matching Jobs remain unselected; confirmation is disabled until explicit choice');
  radios[1].click(); await until(() => !submit.disabled);
  submit.focus(); const tab = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }); submit.dispatchEvent(tab);
  assert(tab.defaultPrevented && document.activeElement === radios[0], 'Tab at last control stays inside the actual modal');
  submit.click(); await until(() => document.querySelector('[role="alert"]'));
  assert(document.querySelector('dialog[open]') && radios[1].checked && selected[0] === 'second-waiting', 'Real error keeps deliberate second selection for review');
  checks.push('Modal contains focus; source-revision rejection stays visible and retains the chosen Job');
  reject = false; submit.click(); await until(() => waiting !== null);
  assert(submit.disabled && document.querySelector('fieldset')?.disabled, 'No overlapping selection submission');
  document.querySelector('dialog')!.dispatchEvent(new Event('cancel', { cancelable: true }));
  assert(dismissals === 0 && document.querySelector('dialog[open]'), 'In-flight source check cannot be dismissed by Escape');
  waiting!(); await until(() => !document.querySelector('dialog'));
  assert(selected.length === 2 && selected.every(id => id === 'second-waiting'), 'Chosen second Job reaches the select port without defaulting to first');
  checks.push('Successful explicit second selection closes the dialog; in-flight duplicate/dismiss actions are disabled');
  opener.focus(); opener.click(); await until(() => document.querySelector('dialog[open]'));
  document.querySelector('dialog')!.dispatchEvent(new Event('cancel', { cancelable: true }));
  await until(() => !document.querySelector('dialog'));
  assert(dismissals === 1 && selected.length === 2 && document.activeElement === opener, 'Cancel preserves Run, submits no selection and restores opener focus');
  checks.push('Cancel makes no selection and restores focus to its opener');
  return { status: 'pass', checks, ordinaryNative: false };
}
main().then(result, error => result({ status: 'fail', checks, error: String(error), stack: error.stack }));
function result(value: unknown) { const pre = document.createElement('pre'); pre.id = 'result'; pre.textContent = JSON.stringify(value); document.body.append(pre); }
