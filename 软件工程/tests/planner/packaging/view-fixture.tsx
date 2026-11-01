// Finite rendering fixture: protocol/type_fixture input, no Host, Native or geometric computation.
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { PackagingWorkspace } from '../../../apps/planner/src/features/packaging/PackagingWorkspace';
export { loadStoredArtifactList } from '../../../apps/planner/src/features/packaging/PackagingWorkspace';
export { HostFailure } from '../../../apps/planner/src/services/projectHost';
export { startManufacturingPackageExport } from '../../../apps/planner/src/services/manufacturingExport';
import type { WorkspaceSnapshot } from '../../../apps/planner/src/services/projectHost';
import type { PackagingEvidence } from '../../../apps/planner/src/features/packaging/types';
import type { ProjectState } from '../../../packages/commands/generated/typescript/ProjectState';
import type { PackageStructureJob } from '../../../packages/commands/generated/typescript/PackageStructureJob';
import type { PackagingResult } from '../../../packages/commands/generated/typescript/PackagingResult';
import type { ArtifactRecord } from '../../../packages/commands/generated/typescript/ArtifactRecord';
import type { ManufacturingPackageExportRequest } from '../../../apps/planner/src/services/manufacturingExport';
import type { TaskHandle } from '../../../packages/commands/generated/typescript/TaskHandle';
export const manufacturingRequestFixture: ManufacturingPackageExportRequest = {
  project_id: 'P', scope: { kind: 'definition_draft', data: 'D' }, scope_revision: 3,
  params: { revision: 3, selected_destination_ref: { selection_id: 'finite-file-token', purpose: 'export_package' }, package_scope: { purpose: 'private_module', scenarios: [], run_ids: [], artifact_ids: ['A'], module_refs: [{ id: 'M', version: '2.0.0' }], include_redistributable_assets: true } },
  declarations: [{ artifact_id: 'A', file: { path: 'manufacturing/exact.svg', kind: 'asset' }, source: 'Explicit finite fixture source', license: 'Explicit finite fixture delivery permission' }], confirm_redistribution: true,
};
export const manufacturingTaskFixture: TaskHandle = { id: 'private-task', kind: 'exporting', state: 'queued', input_revision: 3, scope: { kind: 'edit', data: { kind: 'definition_draft', data: { id: 'D', revision: 3 } } } };
export function renderFixture(mode: 'project' | 'draft' | 'no_exact_input' | 'manufacturing' | 'manufacturing_available' | 'available_list', data: {
  project: ProjectState; input: PackageStructureJob;
  docs: { jobs: PackageStructureJob[]; results: PackagingResult[]; artifacts: ArtifactRecord[] };
}) {
  const { project, docs, input } = data;
  const result = structuredClone(docs.results[0]);
  if (mode === 'manufacturing' || mode === 'manufacturing_available') {
    // Explicit consumer protocol fixture for content display; not generated or accepted manufacturing.
    result.manufacturing_files = [{ part_id: 'PROTOCOL-PART', file: { path: 'protocol-fixture.svg', kind: 'asset' }, layers: ['cut', 'fold', 'glue'], unit: 'millimeter' }];
    result.material_counts = [{ material_id: 'PROTOCOL-MATERIAL', part_id: 'PROTOCOL-PART', quantity: 3, area_mm2: 12.5 }];
  }
  const evidence: PackagingEvidence = { result, artifact: docs.artifacts.find(item => item.id === result.artifact_id)!,
    input: mode === 'no_exact_input' ? null : docs.jobs.find(item => item.id === result.job_ref.id)!, inputReadIssue: mode === 'no_exact_input' ? '实际精确旧 Job 不可读取，禁止用 latest 替代。' : null };
  const workspace: WorkspaceSnapshot = { project, modules: new Map(), moduleIssues: [], scenarios: [], scenarioIssues: [],
    context: { host: { session_id: 'finite-protocol-session', active_project_id: project.id, active_project_revision: project.revision, selected_scope: { kind: 'project', data: project.id } },
      project: { id: project.id, title: project.title, revision: project.revision, dirty: false }, selected_objects: [], view_state: null, definition: null, tasks: [], authorized_operations: [], issues: [] },
    capabilities: { api_version: '0.1', implemented_operations: [], available_command_kinds: [], providers: [], category_ids: [], media_kinds: [], native_primitives: [], max_ipc_frame_bytes: 1048576, limitations: [] } };
  return renderToStaticMarkup(<PackagingWorkspace workspace={workspace} scope={mode === 'draft' ? { kind: 'definition_draft', data: 'PROTOCOL-DRAFT' } : { kind: 'project' }} scopeRevision={project.revision}
    io={{ dispatch: async () => { throw new Error('SSR must not dispatch'); }, awaitTask: async () => { throw new Error('SSR must not observe'); }, pickDirectory: async () => null }} initialJob={input} initialEvidence={evidence}
    manufacturingExport={mode === 'manufacturing_available' ? { status: 'available', start: async () => { throw new Error('SSR must not invoke a private export'); } } : undefined}
    storedArtifacts={mode === 'available_list' ? { status: 'available', list: async () => ['DRAFT-ONLY-F18'], read: async () => { throw new Error('SSR must not read files'); } } : { status: 'unavailable', reason: 'Protocol fixture: no actual persisted-result producer' }} onCommitted={() => {}} onRefresh={() => {}} onClose={() => {}} />);
}
