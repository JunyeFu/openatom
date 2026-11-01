// Finite, read-only execution of actual S04 prototype functions.
// Run with Node from any directory. No browser, service, Engine or Host is used.
// DOM/rendering/SceneView are stubs; business functions are sliced from the source.
// Output: tools/design-quality-audit/output/probe-s04-result.json only.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');

const root = path.resolve(__dirname, '../..');
const sourceRef = 'docs/02_交互设计/S04_可操作设计原型/workbench.js';
const source = fs.readFileSync(path.join(root, sourceRef), 'utf8');
const extracted = [];

function originalBlock(from, until) {
  const start = source.indexOf(from);
  assert.ok(start >= 0, 'Missing source marker: ' + from);
  const end = source.indexOf(until, start + from.length);
  assert.ok(end > start, 'Missing ending marker: ' + until);
  extracted.push({
    source_ref: sourceRef,
    from_marker: from,
    until_marker_exclusive: until,
    start_line: source.slice(0, start).split('\n').length,
    end_line: source.slice(0, end).split('\n').length - 1
  });
  return source.slice(start, end);
}

const initialPaper = {
  width: 160, depth: 100, height: 80, thickness: 0.5,
  productRevision: '1.0.0', ack: true, bound: true
};

// Test fixtures supply only state needed by the extracted functions.
function project(title) {
  return {
    title, devices: [{ id: 'A01' }], dimensions: [24, 16, 4],
    media: {}, definitions: [], lineMembers: ['A01'],
    paper: structuredClone(initialPaper)
  };
}

async function main() {
  const S = {
    project: project('A-current-project'), revision: 2, selected: 'A01',
    undo: [], redo: [], tool: 'select', form: null, taskLog: []
  };
  let dialog = '';
  let onOpen;
  const context = vm.createContext({
    S, copy: structuredClone, imageAsset: null,
    render() {}, renderTools() {}, renderTimeline() {}, renderWorkspace() {},
    notice() {}, log() {},
    view: { setProject() {}, setVisualU() {} },
    showDialog(title, html) { dialog = html; },
    msg: value => value,
    btn: (label, action) => `<button data-action="${action}">${label}</button>`,
    document: { querySelector() { return null; } },
    extraAction() { return false; },
    $: id => ({
      addEventListener(type, handler) {
        if (id === 'project-file' && type === 'change') onOpen = handler;
      },
      close() {}
    })
  });

  vm.runInContext(
    originalBlock('function commit(', 'function discard(') + '\n' +
    originalBlock('function selectDevice(', 'function moveDevice('),
    context
  );

  const selectionInput = {
    selected: 'A01', next_selection: 'ROB01',
    form: { dirty: true, kind: 'pose', data: { x: 8.6 }, base: 2 }
  };
  S.form = structuredClone(selectionInput.form);
  vm.runInContext("selectDevice('ROB01')", context);
  const selectionActual = {
    actions: [...dialog.matchAll(/data-action="([^"]+)"/g)].map(match => match[1]),
    pending: S.pendingSelection, selected: S.selected
  };
  assert.equal(selectionActual.selected, 'A01');
  assert.equal(selectionActual.pending, 'ROB01');

  const packagingInput = {
    paper: structuredClone(initialPaper),
    packResult: {
      outcome: 'pass', productRevision: '1.0.0', source_kind: 'manual_expected'
    },
    edit: { kind: 'paper', base: 2, dirty: true,
      data: { ...initialPaper, width: 200 } }
  };
  S.form = structuredClone(packagingInput.edit);
  S.packResult = structuredClone(packagingInput.packResult);
  const applied = vm.runInContext('apply()', context);
  const packagingActual = {
    applied, paper: structuredClone(S.project.paper),
    packResult: structuredClone(S.packResult)
  };
  assert.equal(packagingActual.applied, true);
  assert.equal(packagingActual.paper.width, 200);

  S.project = project('B-before-open');
  S.undo = [project('A-previous-project')];
  S.redo = [];
  S.form = null;
  const importInput = {
    document_kind: 'S04_interaction_prototype_project_state',
    project: project('C-imported-project'), revision: 10,
    view: { selected: 'A01' }
  };
  vm.runInContext(
    originalBlock("$('project-file').addEventListener", "$('media-file').addEventListener"),
    context
  );
  assert.equal(typeof onOpen, 'function');
  await onOpen({ target: { files: [{
    name: 'C.json', text: async () => JSON.stringify(importInput)
  }] } });
  const importActual = {
    opened: S.project.title, undo_depth_after_open: S.undo.length
  };
  assert.equal(importActual.opened, 'C-imported-project');
  vm.runInContext(
    originalBlock('function action(a)', "document.addEventListener('click'"),
    context
  );
  vm.runInContext("action('undo')", context);
  importActual.after_undo = S.project.title;

  const cases = [
    {
      id: 'S04-DQ-01', input: selectionInput, actual: selectionActual,
      contract_ref: 'docs/02_交互设计/交互流程与视图导航.json#/editing_policy/close_dirty_form',
      expected: 'Offer apply successfully then switch, discard then switch, and stay.',
      contract_holds: selectionActual.actions.some(action => /apply/.test(action))
    },
    {
      id: 'S04-DQ-02',
      input: { previous_undo_project: 'A-previous-project', imported_snapshot: importInput },
      actual: importActual,
      contract_ref: 'docs/02_交互设计/交互流程与视图导航.json#/editing_policy/undo',
      expected: 'Opening another project clears previous project undo/redo history.',
      contract_holds: importActual.undo_depth_after_open === 0 &&
        importActual.after_undo === 'C-imported-project'
    },
    {
      id: 'S04-DQ-03', input: packagingInput, actual: packagingActual,
      contract_ref: 'packages/packaging/adoption-lifecycle.design.json#/professional_version_effects/3',
      expected: 'Changing geometry cannot leave the old result effectively acknowledged and bound to the changed shape.',
      contract_holds: !(packagingActual.paper.ack && packagingActual.paper.bound &&
        packagingActual.packResult?.outcome === 'pass')
    }
  ];
  const result = {
    document_kind: 'S04_actual_source_finite_state_sequence_probe',
    source_ref: sourceRef, extracted_blocks: extracted,
    execution_kind: 'Node VM executing source functions with presentation stubs',
    stubs: [
      'DOM lookup/event registration and dialog markup; no browser layout or event dispatch.',
      'render/renderTools/renderTimeline/renderWorkspace and SceneView methods do nothing.',
      'notice/log do nothing; extraAction returns false for the tested undo action.',
      'Snapshot file is an in-memory object exposing the same file.text() interface.'
    ],
    cases,
    violated_contract_ids: cases.filter(item => !item.contract_holds).map(item => item.id),
    actual_browser_execution: false, actual_product_runtime: null,
    scope: 'Only the stated prototype state transitions; not product correctness, rendering, industrial validation or a rerun of the prior browser suites.'
  };
  const output = path.join(__dirname, 'output/probe-s04-result.json');
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, JSON.stringify(result, null, 2) + '\n', 'utf8');
  console.log(JSON.stringify({
    output, violated_contract_ids: result.violated_contract_ids,
    actual_browser_execution: result.actual_browser_execution
  }, null, 2));
}

main().catch(error => { console.error(error); process.exitCode = 1; });
