import * as THREE from './vendor/three.module.js';
const invoke = window.__TAURI__.core.invoke;
const report = { status: 'running', user_agent: navigator.userAgent, three_revision: THREE.REVISION, steps: [] };
function requireFact(value, message) { if (!value) throw new Error(message); }
async function reject(command, args, expected) {
  try { await invoke(command, args); } catch (error) {
    requireFact(String(error).includes(expected), `Unexpected rejection: ${error}`);
    return { command, expected, actual_error: String(error) };
  }
  throw new Error(`Unexpected acceptance: ${command} ${expected}`);
}
function makeView(canvas) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, preserveDrawingBuffer: true });
  renderer.setPixelRatio(1);
  renderer.setSize(640, 360);
  renderer.setClearColor(0xffffff, 1);
  const scene = new THREE.Scene();
  const box = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial({ color: 0x006dff }));
  scene.add(box);
  const camera = new THREE.OrthographicCamera(-5, 5, 2.8125, -2.8125, 0.1, 100);
  camera.position.set(5, 0, 10);
  camera.lookAt(5, 0, 0);
  return {
    renderer,
    capture(x) {
      box.position.set(x, 0, 0);
      renderer.render(scene, camera);
      return renderer.domElement.toDataURL('image/png');
    }
  };
}
const editorView = makeView(document.getElementById('scene'));
const frozenView = makeView(document.createElement('canvas'));
const renderer = editorView.renderer;
const gl = renderer.getContext();
const debug = gl.getExtension('WEBGL_debug_renderer_info');
report.actual_webgl = { version: gl.getParameter(gl.VERSION), vendor: gl.getParameter(gl.VENDOR), renderer: gl.getParameter(gl.RENDERER), unmasked_renderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null };
try {
  const initial = await invoke('get_context');
  requireFact(initial.revision === 7 && initial.x_m === 2, 'Initial R7 pose mismatch');
  const frozen = await invoke('freeze_render', { scope: 'P1', revision: 7 });
  report.steps.push({ action: 'freeze_real_Rust_R7_before_edits', initial, frozen });
  await invoke('start_agent');
  const uiEdit = await invoke('ui_edit', { baseRevision: 7, xM: 8 });
  const current = await invoke('get_context');
  requireFact(current.revision === 8 && current.x_m === 8, 'Shared edit race did not leave R8 x=8m');
  report.steps.push({ action: 'actual_UI_and_MCP_same_base_revision_race', ui_result: uiEdit, current });
  const currentBefore = editorView.capture(current.x_m);
  const frozenPng = frozenView.capture(frozen.x_m);
  const packet = { task_id: frozen.task_id, scope: frozen.scope, requested_revision: 7, scene_revision: 7, png_data_url: frozenPng };
  report.steps.push(await reject('receive_capture', { packet: { ...packet, scope: 'foreign' } }, 'ScopeMismatch'));
  report.steps.push(await reject('receive_capture', { packet: { ...packet, scene_revision: 8 } }, 'FrozenRevisionMismatch'));
  const returned = await invoke('receive_capture', { packet });
  requireFact(returned.scene_revision === 7 && returned.current_revision === 8, 'Frozen output relabelled');
  report.steps.push({ action: 'actual_WebView2_Three_frozen_R7_png_to_Rust', returned });
  const currentAfter = renderer.domElement.toDataURL('image/png');
  requireFact(currentAfter === currentBefore, 'Frozen capture overwrote current R8 canvas');
  report.steps.push({ action: 'separate_capture_canvas_preserves_actual_current_R8_pixels', current_revision: current.revision, actual_current_before_after_identical: true });
  await invoke('save_current', { pngDataUrl: currentAfter });
  const cancelId = await invoke('start_work', { mode: 'cancel' });
  const progress = await invoke('wait_work', { taskId: cancelId, firstProgress: true });
  const cancelRequested = await invoke('cancel_work', { taskId: cancelId });
  const cancelled = await invoke('wait_work', { taskId: cancelId, firstProgress: false });
  requireFact(progress.completed_steps > 0 && cancelled.status === 'Cancelled', 'Actual running work cancellation failed');
  const failId = await invoke('start_work', { mode: 'failure' });
  const failed = await invoke('wait_work', { taskId: failId, firstProgress: false });
  requireFact(failed.status === 'Failed' && failed.completed_steps === 1, 'Actual worker failure result mismatch');
  const preserved = await invoke('get_context');
  requireFact(preserved.revision === 8 && preserved.x_m === 8, 'Cancel/failure changed last confirmed draft');
  report.steps.push({ action: 'cancel_and_worker_failure_preserve_confirmed_draft', progress, cancelRequested, cancelled, failed, preserved });
  report.status = 'passed';
} catch (error) {
  report.status = 'failed';
  report.error = String(error);
}
document.getElementById('result').textContent = JSON.stringify(report, null, 2);
await invoke('finish_probe', { report });
