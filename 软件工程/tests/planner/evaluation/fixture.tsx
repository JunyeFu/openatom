// Finite consumer of the actual product components. No Host, native bridge or network.
import React from 'react';
import { createRoot } from 'react-dom/client';
import { DeliveryWorkspace } from '../../../apps/planner/src/features/reporting/DeliveryWorkspace';
import { EvaluationView } from '../../../apps/planner/src/features/reporting/ReportPanel';
import type { DeliveryServices, RunEvidence } from '../../../apps/planner/src/features/comparison/types';
import type { WorkspaceSnapshot } from '../../../apps/planner/src/services/projectHost';
import type { EvaluationReport } from '../../../packages/commands/generated/typescript/EvaluationReport';
import type { TaskHandle } from '../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../packages/commands/generated/typescript/TaskState';
import type { TaskOutput } from '../../../packages/commands/generated/typescript/TaskOutput';
import type { RunComparison } from '../../../packages/commands/generated/typescript/RunComparison';
import type { ExportEvaluationParams } from '../../../packages/commands/generated/typescript/ExportEvaluationParams';
import type { ExportPackageParams } from '../../../packages/commands/generated/typescript/ExportPackageParams';
import '../../../apps/planner/src/styles/workbench.css';

declare global { interface Window { deliveryFixture: {
  evidence: RunEvidence[]; report: EvaluationReport; mode: 'success' | 'hold' | 'fail' | 'picker_cancel' | 'compare_fail' | 'read_fail_once' | 'empty_result' | 'incomparable';
  calls: { method: string; params: unknown }[]; reset(): void;
} } }
const f = window.deliveryFixture;
const current = f.evidence[f.evidence.length - 1];
const summaries = f.evidence.map(e => ({ run_id: e.run.id, input: e.run.input_ref,
  computation_status: e.run.computation_status, window: e.run.window, computed_until_ms: e.run.computed_until_ms ?? null,
  whole_window_computed: e.run.whole_window_computed, goal_met: f.report.run_evaluations.find(v => v.run_id === e.run.id)?.goal_met ?? null,
  accepted: f.report.run_evaluations.find(v => v.run_id === e.run.id)?.order_metrics[0]?.accepted ?? null, issues: e.run.limitations }));
const workspace = { project: current.project, scenarios: f.evidence.map(e => e.scenario),
  context: { project: { id: current.project.id, revision: current.project.revision, title: current.project.title, dirty: true }, run_summaries: summaries },
  modules: new Map(), moduleIssues: [], scenarioIssues: [], capabilities: {} } as unknown as WorkspaceSnapshot;
let task: TaskHandle | null = null, count = 0, cancelled = false, output: TaskOutput | null = null;
const start = (kind: 'evaluation' | 'package' | 'save', params: unknown): TaskHandle | null => {
  f.calls.push({ method: kind, params });
  if (f.mode === 'picker_cancel') return null;
  count = 0; cancelled = false;
  task = { id: `finite-${kind}-${f.calls.length}`, scope: { kind: 'edit', data: { kind: 'project', data: current.project.id } }, kind: 'exporting', state: 'queued', input_revision: current.project.revision, progress: null, result_ref: null, failure: null };
  if (kind === 'evaluation') {
    const selection = params as Omit<ExportEvaluationParams, 'selected_destination_ref'>;
    const report = structuredClone(f.report);
    if (selection.report_mode === 'design_description') { report.mode = 'design_description'; report.run_refs = []; report.run_evaluations = []; report.scenario_comparison = null; report.conclusions = ['人工固定Task结果：仅设计说明，不是产品Host运行。']; }
    else if (selection.run_refs.length === 1) { report.run_refs = selection.run_refs; report.run_evaluations = report.run_evaluations.filter(r => selection.run_refs.includes(r.run_id)); report.scenario_comparison = null; }
    output = { kind, data: { destination: { selection_id: 'finite-selected-directory', purpose: 'export_evaluation' }, context: report,
      files: f.mode === 'empty_result' ? [] : selection.formats.map(v => ({ path: `finite/report.${v}`, kind: 'evaluation' })), limitations: [] } };
  } else if (kind === 'package') output = { kind, data: { destination: { selection_id: 'finite-selected-file', purpose: 'export_package' }, package: { path: 'finite/project.zip', kind: 'project' }, included_files: [{ path: 'project.json', kind: 'project' }], missing: [{ code: 'finite.unknown_rights', message: '人工固定缺项：素材许可未知，不能声称全部已带入。', blocks: [] }] } };
  else output = { kind, data: { project_id: current.project.id, destination: { selection_id: 'finite-saved-directory', purpose: 'save_project' }, saved_revision: current.project.revision, current_revision: current.project.revision + 1, still_dirty: true, manifest: { path: 'project.json', kind: 'project' } } };
  return task;
};
const services: DeliveryServices = {
  readEvidence: async id => { f.calls.push({ method: 'readEvidence', params: id }); return f.evidence.find(e => e.run.id === id)!; },
  compare: async params => {
    f.calls.push({ method: 'compare', params });
    if (f.mode === 'compare_fail') throw Object.assign(new Error('有限服务比较错误'), { detail: { code: 'InvalidInput', field_errors: [{ path: '/metric_scope', message: '有限字段缺项' }] } });
    const value = structuredClone(f.report.scenario_comparison as RunComparison);
    if (f.mode === 'incomparable') { value.pairs.forEach(p => { p.status = 'side_by_side_only'; }); value.metrics.forEach(m => m.pair_differences.forEach(d => { d.comparable = false; d.change_percent = 7; })); }
    return value;
  },
  startEvaluation: async params => start('evaluation', params),
  startPackage: async params => start('package', params),
  startSave: async params => start('save', params),
  readTask: async handle => {
    count++;
    if (f.mode === 'read_fail_once' && count === 1) throw Object.assign(new Error('有限服务读取中断'), { detail: { code: 'IoFailure', field_errors: [] } });
    const status = cancelled ? 'cancelled' : f.mode === 'fail' && count > 1 ? 'failed' : f.mode !== 'hold' && count > 2 ? 'completed' : 'running';
    const state: TaskState = { task: { ...handle, state: status, progress: { phase: 'finite_component_fixture', processed: Math.min(count, 3), total: 3 } }, originating_operation: 'export_evaluation', allowed_controls: status === 'running' ? ['cancel'] : [], result: status === 'completed' ? output : null, failure: status === 'failed' ? { code: 'InvalidInput', message: '人工固定失败：缺少许可允许的附件', field_errors: [], object_refs: [], expected_revision: null, actual_revision: null } : null };
    return state;
  },
  cancelTask: async handle => { f.calls.push({ method: 'cancel', params: handle.id }); cancelled = true; return { task: handle, requested_action: 'cancel', action_accepted: true, terminal_won: false, issues: [] }; },
  cloneScenario: async params => { f.calls.push({ method: 'clone', params }); return { scenario_id: params.new_scenario_id, revision: 1 }; },
};
let key = 0; const root = createRoot(document.getElementById('root')!);
function render() { root.render(<><div className="finite-fixture-banner">有限组件测试：人工固定服务响应，不是产品Host、真实导出或实际Native验收。</div><div style={{height:'calc(100vh - 30px)'}}><DeliveryWorkspace key={key} workspace={workspace} services={services}
  onClose={() => f.calls.push({ method: 'close', params: null })} onOpenPackage={() => f.calls.push({ method: 'open', params: null })}
  onInspectRun={id => f.calls.push({ method: 'inspect', params: id })} onLocate={(run,object) => f.calls.push({ method: 'locate', params: {run,object} })} /></div></>); }
f.reset = () => { key++; render(); };
render();
// Dedicated finite zero/null value view uses the exact product renderer.
export { EvaluationView };
