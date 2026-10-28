// Real React/process/media components; only the Native IPC boundary is a fixed
// recorded-history fixture. This is not ordinary Native or FPS acceptance.
import { createRoot } from 'react-dom/client';
import { DeviceProcessWorkspace } from '../../../apps/planner/src/features/factory/DeviceProcessWorkspace';
import input from '../../../scenarios/媒体表现/runtime/calendar.input.json';
import frames from '../../../scenarios/媒体表现/runtime/actual-frames.json';
import image from '../../../modules/media/p302-controlled/image.module.json';
import png from '../../../modules/media/p302-controlled/station.png?inline';
import pngEnd from '../../../modules/media/p302-controlled/station-end.png?inline';

const checks: string[] = [], reads: unknown[] = [], actions: any[] = [];
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition: unknown, message: string) => { if (!condition) throw new Error(message); };
async function until(test: () => unknown) {
  for (let i = 0; i < 200; i++) { if (test()) return; await delay(20); }
  throw new Error('Timed out: ' + document.body.innerText.slice(-1600));
}
const project: any = structuredClone(input.project), packages: any[] = structuredClone(input.definitions)
  .map(definition => ({ definition, assets: [], files: [], dependencies: [], rights: [], issues: [] }));
const base: any = frames.find(v => v.label === 'calendar@35000')!.frame;
const job = base.snapshot.objects.find((v: any) => v.kind === 'job').data;
const device = project.devices.find((v: any) => v.id === job.execution_ref.data.device_id);
const pkg = packages.find(v => v.definition.id === device.module_ref.id);
pkg.definition.payload.data.presentation = structuredClone(image.definition.payload.data);
pkg.definition.payload.data.operations = ['inspect'];
pkg.assets = image.assets; pkg.files = image.files; pkg.rights = image.rights;
const original = JSON.stringify(base.run), pngBytes = new Map([
  ['station.png', readPng(png)], ['station-end.png', readPng(pngEnd)],
]);
async function readPng(source: string) { return (await fetch(source)).arrayBuffer(); }
let held: (() => void) | null = null, blockAt: number | null = null, observedRequest: number | null = null;
const context: any = { host: { session_id: 'process-playback-fixed-ipc' }, project: { id: project.id, revision: 9 }, definition: null };
(window as any).__TAURI_INTERNALS__ = { async invoke(command: string, args: any) {
  const q = args.request;
  if (command === 'project_read_media_asset') {
    reads.push(structuredClone(q)); const bytes = pngBytes.get(q.asset.path);
    assert(bytes, 'Only the exact declared PNG bytes may be consumed'); return (await bytes!).slice(0);
  }
  if (command === 'project_read_run_port_output') return { ...q, item_type_ref: null,
    completed_output: { value: 0, unit: 'piece' }, accepted_output: { value: 0, unit: 'piece' } };
  assert(command === 'project_api', 'Unexpected Native command ' + command);
  let data: any;
  if (q.operation === 'get_context') data = context;
  else if (q.operation === 'get_run') {
    const query = q.params.query;
    if (query.kind === 'full_result') data = { kind: 'full', data: { ...base.run, id: q.params.run_id } };
    else {
      const at = query.data.at_ms;
      if (blockAt === at) { blockAt = null; observedRequest = at; await new Promise<void>(resolve => { held = resolve; }); }
      const frame: any = frames.find(v => v.label === `calendar@${at}`)?.frame;
      assert(frame, 'Only declared recorded times are permitted');
      data = { kind: 'objects', data: { run_id: q.params.run_id, at_ms: at, next_index: null,
        objects: frame.snapshot.objects.filter((v: any) => query.data.objects.length ? v.kind === 'job' && v.data.id === job.id : v.kind === 'item') } };
    }
  } else if (q.operation === 'get_object') {
    data = q.params.object_id.kind === 'project' ? { kind: 'project', data: project }
      : { kind: 'module', data: packages.find(v => v.definition.id === q.params.source.data.module_ref.id && v.definition.version === q.params.source.data.module_ref.version) };
    assert(data.data, 'Exact frozen module must exist');
  } else throw new Error('Unexpected production write or operation ' + q.operation);
  return { outcome: { status: 'ok', data } };
} };
const root = createRoot(document.getElementById('root')!);
const props: any = { workspace: { context, project, capabilities: {}, modules: new Map(), scenarios: [], moduleIssues: [], scenarioIssues: [] },
  scope: { kind: 'project' }, scopeRevision: 9, actions: { inspect: async (request: unknown) => { actions.push(request); return { message: 'Exact current frame inspection accepted' }; } }, onClose() {},
  selection: { kind: 'actual_run', run_id: base.run.id, job_id: job.id, at_ms: 35000 } };
const render = (selection: any) => { props.selection = selection; root.render(<DeviceProcessWorkspace {...props} />); };
const result = (value: unknown) => { const p = document.createElement('pre'); p.id = 'result'; p.textContent = JSON.stringify(value); document.body.append(p); };
async function main() {
  render(props.selection);
  await until(() => document.querySelector('.sf-media canvas') && !document.querySelector('.sf-media-message'));
  const canvas = document.querySelector('.sf-media canvas'), initialReads = reads.length;
  assert(initialReads > 0, 'The real image surface must consume Native bytes');
  assert((canvas as HTMLCanvasElement).width > 0 && (canvas as HTMLCanvasElement).height > 0, 'Actual production CSS must give the surface nonzero dimensions');
  blockAt = 40000;
  render({ ...props.selection, at_ms: 40000 });
  await until(() => observedRequest === 40000);
  assert(document.querySelector('.sf-media canvas') === canvas, 'Same Run time read must retain the last complete canvas while IPC is pending');
  assert(document.querySelector('[aria-label="实际计算边界"]')?.textContent?.includes('35000 ms'), 'Pending IPC must retain its actual completed frame time');
  assert(document.querySelector('[aria-label="过程帧更新"]')?.textContent?.includes('40000 ms'), 'Pending requested time must be explicit');
  assert((document.querySelector('.sf-media-tag button') as HTMLButtonElement).disabled, 'Actions must not mix a pending time with the previous displayed frame');
  checks.push('Same Run pending time keeps visible canvas and exact previous completed time');
  held!(); held = null;
  await until(() => document.querySelector('[aria-label="实际计算边界"]')?.textContent?.includes('40000 ms'));
  assert(document.querySelector('.sf-media canvas') === canvas, 'Same Run time update must preserve the actual surface');
  assert(reads.length === initialReads, 'Immutable frozen image must not be reread for each time update');
  checks.push('Recorded 40000ms frame updates without rebuilding or rereading the frozen image');
  (document.querySelector('.sf-media-tag button') as HTMLButtonElement).click();
  await until(() => actions.length === 1);
  assert(actions[0].at_ms === 40000 && actions[0].binding.request.source.data.at_ms === 40000, 'An actual action must use the newly displayed frame, not the retained immutable byte-reading time');
  checks.push('Actions are disabled while pending and retain exact current-frame provenance after completion');
  blockAt = 45000;
  render({ ...props.selection, run_id: 'different-fixed-run', at_ms: 45000 });
  await until(() => observedRequest === 45000);
  assert(!document.querySelector('.sf-media canvas'), 'Switching Run must hide the old source immediately');
  checks.push('Different Run never retains a previous Run canvas');
  held!(); held = null;
  await until(() => document.querySelector('.sf-media canvas') && !document.querySelector('.sf-media-message'));
  assert(document.querySelector('.sf-media canvas') !== canvas, 'Different Run requires its own resolved source');
  assert(JSON.stringify(base.run) === original, 'Viewing must leave the complete original Run unchanged');
  checks.push('New Run resolves independently; complete original Run is unchanged');
  result({ status: 'pass', checks, initialReads, finalReads: reads.length, ordinaryNative: false });
}
main().catch(error => result({ status: 'fail', checks, error: String(error), stack: error.stack }));
