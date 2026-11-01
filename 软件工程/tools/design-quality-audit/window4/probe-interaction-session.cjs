// Read-only independent execution of original prototype code. Presentation only is stubbed.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '../../..');
const output = path.join(__dirname, 'output');
const sourceRef = 'docs/02_交互设计/S04_可操作设计原型/workbench.js';
const sourcePath = path.join(root, sourceRef);
const stat = fs.statSync(sourcePath);
const source = fs.readFileSync(sourcePath, 'utf8');
const dataSource = fs.readFileSync(path.join(root, 'docs/02_交互设计/S04_可操作设计原型/prototype.data.js'), 'utf8');
const startup = source.lastIndexOf('\ngo(S.frame,true);view.load()');
assert.ok(startup > 0, 'Harness needs a known bootstrap boundary; do not infer a product defect');
const executable = source.slice(0, startup).replace(/^import\s+.*;\r?$/mg, '');
const clone = structuredClone;
const normalize = value => JSON.parse(JSON.stringify(value));
const lineOf = marker => source.slice(0, source.indexOf(marker)).split('\n').length;

function harness() {
  const elements = new Map(), timers = new Map(), eventListeners = new Map(), storage = new Map(), blobs = new Map();
  const graphics = [], downloads = [];
  let timerId = 0, selectedCallback;
  function element(id) {
    if (!elements.has(id)) elements.set(id, {
      id, innerHTML: '', textContent: '', value: '', dataset: {}, checked: false, open: false, clicks: 0,
      listeners: new Map(), style: { setProperty() {} }, classList: { add() {}, remove() {}, toggle() {}, contains() { return false; } },
      addEventListener(type, callback) { if (!this.listeners.has(type)) this.listeners.set(type, []); this.listeners.get(type).push(callback); },
      showModal() { this.open = true; }, close() { this.open = false; },
      click() { this.clicks++; if (this.download) downloads.push({ name: this.download, blob: blobs.get(this.href) }); },
      querySelector() { return null; }, querySelectorAll() { return []; }, remove() {}, getBoundingClientRect() { return { width: 640 }; }
    });
    return elements.get(id);
  }
  class SceneView {
    constructor(canvas, callbacks) { selectedCallback = callbacks.select; }
    setProject(p) { graphics.push({ method: 'setProject', project: p.title }); }
    setVisualU(u) { graphics.push({ method: 'setVisualU', u }); }
    setTime(t, data, unit) { graphics.push({ method: 'setTime', t, unit }); }
    setTool() {} setLOD() {} inspect() {} resetCamera() {} load() { return Promise.resolve(); }
  }
  const document = {
    getElementById: element, querySelector() { return null; }, querySelectorAll() { return []; },
    createElement: tag => element('created-' + tag + '-' + elements.size), body: element('body'), documentElement: element('html'),
    addEventListener(type, callback) { if (!eventListeners.has(type)) eventListeners.set(type, []); eventListeners.get(type).push(callback); },
    removeEventListener() {}
  };
  const context = vm.createContext({
    window: {}, document, SceneView, structuredClone, URLSearchParams, Blob, console,
    location: { search: '?frame=VP03' }, history: { replaceState() {} },
    URL: { createObjectURL(blob) { const id = 'blob:probe/' + blobs.size; blobs.set(id, blob); return id; }, revokeObjectURL() {} },
    localStorage: { setItem(k, v) { storage.set(k, v); }, getItem: k => storage.get(k) ?? null },
    setTimeout(fn, ms) { const id = ++timerId; timers.set(id, { fn, ms }); return id; }, clearTimeout(id) { timers.delete(id); },
    setInterval() { return ++timerId; }, clearInterval() {},
  });
  vm.runInContext(dataSource, context, { filename: 'original-prototype.data.js' });
  vm.runInContext(executable + `\n
    render=()=>{};renderProperties=()=>{};renderWorkspace=()=>{};renderTimeline=()=>{};renderTools=()=>{};
    globalThis.review={S,seed,edit,apply,commit,action,selectDevice,bindSelectedInstance,snapshot,seek,loadSchedule,
      image:()=>imageAsset};`, context, { filename: sourceRef });
  const p = context.review;
  p.S.project.title = 'A-original';
  p.S.project.instanceState.A01.config.duration = 31;
  p.S.project.instanceState.A01.media.imageAsset = 'A01-owned-image';
  p.S.project.instanceState.ROB01.config.duration = 61;
  p.S.project.instanceState.ROB01.media.imageAsset = 'ROB01-owned-image';
  p.bindSelectedInstance();
  function state() {
    const s = p.S;
    return normalize({ project: s.project, revision: s.revision, savedRevision: s.savedRevision, selected: s.selected,
      form: s.form, undo: s.undo, redo: s.redo, run: s.run, compute: s.compute, packResult: s.packResult,
      paperProcessSample: s.paperProcessSample, sampleRuns: s.sampleRuns, play: s.play, visualLoop: s.visualLoop,
      inputPreview: s.inputPreview, drag: s.drag, source: s.source, axis: s.axis, selectedEvent: s.selectedEvent,
      pendingSeek: s.pendingSeek, viewTime: s.viewTime, agent: s.agent, imageAsset: p.image(), selectedUnit: s.selectedUnit,
      definitionDraft: s.definitionDraft });
  }
  const buttons = () => [...element('dialog-content').innerHTML.matchAll(/<button\b[^>]*data-action="([^"]+)"[^>]*>([\s\S]*?)<\/button>/g)]
    .map(m => ({ action: m[1], label: m[2].replace(/<[^>]*>/g, '') }));
  function choose(labelPattern) {
    const found = buttons().find(b => labelPattern.test(b.label));
    assert.ok(found, `Original dialog has no ${labelPattern}; actual ${JSON.stringify(buttons())}`);
    p.action(found.action);
    return found;
  }
  async function open(doc, failure) {
    p.action('open-project');
    assert.equal(element('project-file').clicks, 1, 'Native open action must reach file chooser in this current consumer');
    const callbacks = element('project-file').listeners.get('change');
    assert.ok(callbacks?.length, 'Original file change callback missing');
    const file = { name: 'C-independent.json', text: async () => {
      if (failure === 'read') throw new Error('InjectedFileReadFailure');
      return failure === 'json' ? '{not JSON' : JSON.stringify(doc);
    } };
    const target = element('project-file'); target.files = [file]; target.value = 'C-independent.json';
    for (const cb of callbacks) await cb({ target });
    return buttons();
  }
  function dirtyForm(x = 8.6) { p.edit('pose'); p.S.form.data.x = x; p.S.form.dirty = true; }
  function history() {
    p.action('rotate'); p.action('rotate'); p.action('undo');
    assert.ok(p.S.undo.length && p.S.redo.length, 'Setup must use native actions to obtain both histories');
  }
  function imported() {
    const project = p.seed(); project.title = 'C-imported';
    project.devices = [{ ...project.devices[0], id: 'C_ONLY', name: 'C device', x: 3 }];
    project.lineMembers = ['C_ONLY']; project.connections = [];
    const config = clone(project.device), media = clone(project.media); config.duration = 91; media.imageAsset = 'C-owned-image';
    project.instanceState = { C_ONLY: { config, media } }; project.device = config; project.media = media;
    project.paper.ack = false; project.paper.bound = false;
    return { document_kind: 'S04_interaction_prototype_project_state', project, revision: 30,
      view: { frame: 'VP03', canvas: 'line', lod: 'detailed', selected: 'C_ONLY', viewTime: 0 } };
  }
  function oldSession() {
    const s = p.S;
    p.loadSchedule();
    assert.ok(s.run.data.units.some(u => u.id === 'package005'), 'Old A selection must reference an actual source-labelled sample unit');
    for (const callback of eventListeners.get('change') ?? []) callback({ target: { id: 'unit-picker', value: '005', dataset: {} } });
    s.compute = { id: 'A-old-task', state: 'completed' };
    s.packResult = { id: 'A-old-professional', outcome: 'pass' }; s.paperProcessSample = { id: 'A-old-process' };
    s.project.paper.ack = true; s.project.paper.bound = true; s.play = true; s.visualLoop = true;
    s.agent = { preview: { title: 'A-old-proposal' }, built: true, rounds: 2, scenario: 'three', stop: 'old', accepted: 6 };
    s.copyPreview = { old: 'A-copy' }; s.adoptionPreview = { old: 'A-adoption' }; s.connectSource = 'A01';
    s.inputPreview = [{ id: 'ARR01', before: 10, after: 50 }]; s.drag = { old: 'A-drag' };
    s.source = 'actual'; s.axis = 'device'; s.selectedEvent = 'A-event'; s.pendingSeek = 25;
    s.pendingFrame = 'VP18'; s.pendingSelection = 'A01'; s.pendingAction = 'A-old-action';
    s.category = 1;
    for (const [field, value] of [['id', 'A_ONLY_DEFINITION'], ['version', '1.0.0']]) {
      for (const callback of eventListeners.get('input') ?? []) callback({ target: { value, dataset: { field, scope: 'definition' } } });
    }
  }
  return { p, state, element, buttons, choose, open, dirtyForm, history, imported, oldSession, timers, graphics, storage, downloads,
    select: id => selectedCallback(id),
    confirmDefinition() {
      const button = { dataset: { action: 'definition-confirm' } };
      const target = { closest(selector) { return selector === '[data-action]' || selector.includes('definition-confirm') ? button : null; } };
      for (const callback of eventListeners.get('click') ?? []) callback({ target });
    } };
}
const cases = [];
async function test(id, finding, purpose, body) {
  const h = harness(); let observation;
  try {
    observation = await body(h);
    cases.push({ id, finding, purpose, status: 'passed', actual: observation });
  } catch (error) {
    cases.push({ id, finding, purpose, status: 'failed', failure: error.message, actual: observation ?? h.state(), original_dialog: h.buttons() });
  }
}
function unchanged(h, before) { assert.deepEqual(h.state(), before); }
function isC(h) {
  assert.equal(h.p.S.project.title, 'C-imported'); assert.equal(h.p.S.selected, 'C_ONLY');
  assert.equal(h.p.S.undo.length, 0); assert.equal(h.p.S.redo.length, 0);
}
async function main() {
  await test('D03-01', 'DQ-03', 'Actual SceneView select callback offers apply/discard/stay and retains original identity before choice', h => {
    h.dirtyForm(); const before = h.state(); h.select('ROB01'); unchanged(h, before);
    const b = h.buttons(); for (const re of [/应用/, /丢弃/, /留在/]) assert.ok(b.some(x => re.test(x.label)));
    return { selected: h.p.S.selected, form: clone(h.p.S.form), actual_buttons: b };
  });
  await test('D03-02', 'DQ-03', 'Apply modifies A01 only, then binds ROB01 own configuration/media', h => {
    h.dirtyForm(); const rob = clone(h.p.S.project.devices.find(d => d.id === 'ROB01'));
    h.select('ROB01'); h.choose(/应用/);
    assert.equal(h.p.S.selected, 'ROB01'); assert.equal(h.p.S.project.devices.find(d => d.id === 'A01').x, 8.6);
    assert.deepEqual(normalize(h.p.S.project.devices.find(d => d.id === 'ROB01')), normalize(rob));
    assert.equal(h.p.S.project.device.duration, 61); assert.equal(h.p.image(), 'ROB01-owned-image');
    return { selected: h.p.S.selected, old_A01_x: h.p.S.project.devices[0].x, current_duration: h.p.S.project.device.duration, current_image: h.p.image() };
  });
  await test('D03-03', 'DQ-03', 'Discard switches with no commit and correct selected-instance identity', h => {
    h.dirtyForm(); const rev = h.p.S.revision; h.select('ROB01'); h.choose(/丢弃/);
    assert.equal(h.p.S.selected, 'ROB01'); assert.equal(h.p.S.project.devices[0].x, 8); assert.equal(h.p.S.revision, rev);
    assert.equal(h.p.S.form, null); assert.equal(h.p.S.project.device.duration, 61); assert.equal(h.p.image(), 'ROB01-owned-image');
    return { selected: h.p.S.selected, revision: rev, old_A01_x: h.p.S.project.devices[0].x, current_image: h.p.image() };
  });
  await test('D03-04', 'DQ-03', 'Stay keeps pending fields/base revision and current A identity', h => {
    h.dirtyForm(); const before = h.state(); h.select('ROB01'); h.choose(/留在/); unchanged(h, before); return h.state();
  });
  for (const [id, mode] of [['D03-05', 'missing'], ['D03-06', 'conflict'], ['D03-07', 'bounds']]) {
    await test(id, 'DQ-03', `Actual apply failure ${mode} retains fields, base, selected identity, revision and histories`, h => {
      h.history(); h.dirtyForm(mode === 'missing' ? '' : mode === 'bounds' ? 1000 : 8.6);
      if (mode === 'conflict') h.p.S.revision++;
      const before = h.state(); h.select('ROB01'); h.choose(/应用/); unchanged(h, before);
      return { selected: h.p.S.selected, form: clone(h.p.S.form), revision: h.p.S.revision, notice: h.element('toast').textContent };
    });
  }
  await test('D04-01', 'DQ-04', 'Unapplied A opening C: first stay retains A/form and both histories', async h => {
    h.history(); h.dirtyForm(); const before = h.state(); await h.open(h.imported()); h.choose(/留在/); unchanged(h, before); return h.state();
  });
  await test('D04-02', 'DQ-04', 'Unapplied A apply then save/open C saves A applied fields before replacement', async h => {
    h.history(); h.dirtyForm(); await h.open(h.imported()); h.choose(/应用/);
    assert.equal(h.p.S.project.title, 'A-original'); assert.equal(h.p.S.project.devices[0].x, 8.6);
    h.choose(/保存/); isC(h);
    const saved = JSON.parse(h.storage.get('superfactory.S04.prototype.snapshot'));
    assert.equal(saved.project.title, 'A-original'); assert.equal(saved.project.devices[0].x, 8.6);
    return { saved_A_x: saved.project.devices[0].x, saved_title: saved.project.title, opened_title: h.p.S.project.title, downloads: h.downloads.map(d => d.name) };
  });
  await test('D04-03', 'DQ-04', 'Discard unapplied fields then discard unsaved A opens C without writing A', async h => {
    h.history(); h.dirtyForm(); await h.open(h.imported()); h.choose(/丢弃/);
    assert.equal(h.p.S.project.title, 'A-original'); assert.equal(h.p.S.project.devices[0].x, 8);
    h.choose(/丢弃/); isC(h); assert.equal(h.storage.size, 0); return h.state();
  });
  await test('D04-04', 'DQ-04', 'Already-applied dirty A save choice writes A, then opens C', async h => {
    h.history(); await h.open(h.imported()); h.choose(/保存/); isC(h);
    const saved = JSON.parse(h.storage.get('superfactory.S04.prototype.snapshot'));
    assert.equal(saved.project.title, 'A-original'); return { saved_A_revision: saved.revision, C_revision: h.p.S.revision, saved_A_title: saved.project.title };
  });
  await test('D04-05', 'DQ-04', 'Already-applied dirty A discard choice opens C without saving', async h => {
    h.history(); await h.open(h.imported()); h.choose(/丢弃/); isC(h); assert.equal(h.storage.size, 0); return h.state();
  });
  await test('D04-06', 'DQ-04', 'Already-applied dirty A stay preserves both histories and old runtime/view state', async h => {
    h.history(); h.oldSession(); const before = h.state(); await h.open(h.imported()); h.choose(/留在/); unchanged(h, before); return h.state();
  });
  for (const [id, failure] of [['D04-07', 'read'], ['D04-08', 'json'], ['D04-09', 'kind']]) {
    await test(id, 'DQ-04', `Original file callback ${failure} failure preserves A, unapplied fields, undo/redo and session`, async h => {
      h.history(); h.oldSession(); h.dirtyForm(); const before = h.state(); const doc = h.imported();
      if (failure === 'kind') doc.document_kind = 'foreign-format';
      await h.open(doc, failure); unchanged(h, before);
      assert.ok(/打开失败/.test(h.element('toast').textContent));
      h.p.S.form = null; h.p.action('undo'); assert.equal(h.p.S.project.title, 'A-original');
      h.p.action('redo'); assert.equal(h.p.S.project.title, 'A-original');
      return { retained_form: before.form, retained_undo: before.undo.length, retained_redo: before.redo.length, notice: h.element('toast').textContent };
    });
  }
  await test('D04-10', 'DQ-04', 'Unapplied open-apply conflict retains A/form/history and does not replace', async h => {
    h.history(); h.dirtyForm(); h.p.S.revision++; const before = h.state(); await h.open(h.imported()); h.choose(/应用/); unchanged(h, before); return h.state();
  });
  await test('D04-11', 'DQ-04', 'Successful C open isolates undo/redo, including C subsequent edit/undo/redo', async h => {
    h.history(); await h.open(h.imported()); h.choose(/丢弃/); isC(h);
    h.p.action('undo'); h.p.action('redo'); assert.equal(h.p.S.project.title, 'C-imported');
    h.dirtyForm(4.5); assert.equal(h.p.apply(), true); assert.equal(h.p.S.project.devices[0].x, 4.5);
    h.p.action('undo'); assert.equal(h.p.S.project.title, 'C-imported'); assert.equal(h.p.S.project.devices[0].x, 3);
    h.p.action('redo'); assert.equal(h.p.S.project.title, 'C-imported'); assert.equal(h.p.S.project.devices[0].x, 4.5);
    return { C_title: h.p.S.project.title, C_selected: h.p.S.selected, C_x: h.p.S.project.devices[0].x, C_undo_titles: h.p.S.undo.map(p => p.title) };
  });
  await test('D04-12', 'DQ-04', 'C success clears old Run, professional, proposal, gesture and primary selection state', async h => {
    h.history(); h.oldSession(); await h.open(h.imported()); h.choose(/丢弃/); isC(h);
    for (const key of ['run', 'compute', 'packResult', 'paperProcessSample', 'inputPreview', 'drag', 'copyPreview', 'adoptionPreview', 'connectSource', 'selectedEvent', 'pendingSeek', 'pendingSelection', 'pendingFrame', 'pendingAction']) assert.equal(h.p.S[key], null, key);
    assert.equal(h.p.S.sampleRuns.length, 0); assert.equal(h.p.S.play, false); assert.equal(h.p.S.visualLoop, false);
    assert.equal(h.p.S.agent.preview, null); assert.equal(h.p.S.agent.built, false); assert.equal(h.p.S.agent.rounds, 0);
    assert.equal(h.p.image(), 'C-owned-image'); assert.equal(h.p.S.project.paper.ack, false); assert.equal(h.p.S.project.paper.bound, false);
    return { selected: h.p.S.selected, run: h.p.S.run, professional: h.p.S.packResult, image: h.p.image(), agent: clone(h.p.S.agent) };
  });
  await test('D04-13', 'DQ-04', 'C success must not retain A frozen-Run unit selection or definition-authoring draft', async h => {
    h.history(); h.oldSession(); await h.open(h.imported()); h.choose(/丢弃/); isC(h);
    const residue = { selectedUnit: h.p.S.selectedUnit, definitionDraft: clone(h.p.S.definitionDraft) };
    assert.notEqual(residue.selectedUnit, '005', 'Old A Run package005 selection remains in C session');
    assert.ok(!residue.definitionDraft || residue.definitionDraft.id !== 'A_ONLY_DEFINITION', 'Old A definition authoring draft remains in C session');
    return residue;
  });
  await test('D04-14', 'DQ-04', 'Late actual A seek/completion callbacks do not reinsert A into C', async h => {
    h.history(); h.oldSession(); h.p.seek(42); h.p.action('compute-race');
    const oldCallbacks = [...h.timers.values()].filter(t => t.ms === 70 || t.ms === 250);
    assert.equal(oldCallbacks.length, 2);
    await h.open(h.imported()); h.choose(/丢弃/); isC(h);
    for (const t of oldCallbacks) t.fn();
    assert.equal(h.p.S.project.title, 'C-imported'); assert.equal(h.p.S.run, null); assert.equal(h.p.S.compute, null); assert.equal(h.p.S.viewTime, 0);
    assert.ok(!h.graphics.some(g => g.method === 'setTime' && g.t === 42));
    return { late_callbacks_executed: oldCallbacks.length, C_title: h.p.S.project.title, C_viewTime: h.p.S.viewTime, C_run: h.p.S.run, C_compute: h.p.S.compute };
  });
  await test('D04-15', 'DQ-04', 'Original definition-confirm click must not publish A authoring draft inside C', async h => {
    h.history(); h.oldSession(); await h.open(h.imported()); h.choose(/丢弃/); isC(h);
    h.confirmDefinition();
    const oldDraftCommitted = h.p.S.project.definitions.some(d => d.id === 'A_ONLY_DEFINITION');
    assert.equal(oldDraftCommitted, false, 'Actual definition-confirm callback committed stale A definition into C');
    return { C_title: h.p.S.project.title, definitions: clone(h.p.S.project.definitions), C_selected: h.p.S.selected };
  });
  fs.mkdirSync(output, { recursive: true });
  fs.writeFileSync(path.join(output, 'consumed-workbench.js'), source, 'utf8');
  const failed = cases.filter(c => c.status === 'failed');
  const result = {
    document_kind: 'DQ03_DQ04_independent_actual_prototype_function_and_file_callback_execution',
    observed_at: new Date().toISOString(), source_ref: sourceRef, source_last_modified: stat.mtime.toISOString(),
    source_changed_during_execution: fs.statSync(sourcePath).mtimeMs !== stat.mtimeMs,
    original_source_evidence: 'tools/design-quality-audit/window4/output/consumed-workbench.js',
    command: 'node tools/design-quality-audit/window4/probe-interaction-session.cjs',
    implementation_functions_replaced: [],
    presentation_stubs: ['SceneView graphics and DOM elements/dialogs', 'render, renderProperties, renderWorkspace, renderTimeline, renderTools', 'timer scheduling captured; two original late callbacks invoked explicitly', 'localStorage is a Map; file.text is an in-memory success or observed rejection; browser download is a Blob observation'],
    source_bootstrap_omitted: ['one SceneView ES import supplied by stub', 'last go()/view.load() startup; no browser graphics'],
    findings: ['DQ-03', 'DQ-04'].map(id => ({ id, status: cases.filter(c => c.finding === id).some(c => c.status === 'failed') ? 'blocker' : 'closed_in_isolated_prototype_sequence_scope' })),
    cases_passed: cases.length - failed.length, cases_total: cases.length, cases,
    blockers: failed.map(c => ({ finding: c.finding, case_id: c.id, failure: c.failure,
      source_ref: sourceRef, source_line: lineOf(c.finding === 'DQ-03' ? 'function selectDevice(' : 'function replaceSnapshot('), actual: c.actual })),
    actual_browser_execution: false, actual_product_runtime: null, service_started: false, Figma_operated: false,
    other_windows_source_or_public_status_modified: false,
  };
  fs.writeFileSync(path.join(output, 'probe-result.json'), JSON.stringify(result, null, 2) + '\n', 'utf8');
  console.log(JSON.stringify({ cases_passed: result.cases_passed, cases_total: result.cases_total, findings: result.findings, blockers: result.blockers.map(b => ({ case_id: b.case_id, source_line: b.source_line, failure: b.failure })) }, null, 2));
}
main().catch(error => { console.error(error); process.exitCode = 1; });
