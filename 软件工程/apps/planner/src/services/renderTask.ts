import { invoke } from '@tauri-apps/api/core';
import type { ApiScope } from '../../../../packages/commands/generated/typescript/ApiScope';
import type { AssetRef } from '../../../../packages/commands/generated/typescript/AssetRef';
import type { ApiError } from '../../../../packages/commands/generated/typescript/ApiError';
import type { ArtifactRecord } from '../../../../packages/commands/generated/typescript/ArtifactRecord';
import type { DefinitionScopeIdentity } from '../../../../packages/commands/generated/typescript/DefinitionScopeIdentity';
import type { DomainObject } from '../../../../packages/commands/generated/typescript/DomainObject';
import type { EditScope } from '../../../../packages/commands/generated/typescript/EditScope';
import type { FileRef } from '../../../../packages/commands/generated/typescript/FileRef';
import type { Issue } from '../../../../packages/commands/generated/typescript/Issue';
import type { JobExecutionRef } from '../../../../packages/commands/generated/typescript/JobExecutionRef';
import type { ModuleDefinition } from '../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { ModulePackage } from '../../../../packages/commands/generated/typescript/ModulePackage';
import type { ObjectSource } from '../../../../packages/commands/generated/typescript/ObjectSource';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { PackagingProcessSample } from '../../../../packages/commands/generated/typescript/PackagingProcessSample';
import type { PackagingResult } from '../../../../packages/commands/generated/typescript/PackagingResult';
import type { PackageStructureJob } from '../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { PortRef } from '../../../../packages/commands/generated/typescript/PortRef';
import type { Quantity } from '../../../../packages/commands/generated/typescript/Quantity';
import type { RenderViewParams } from '../../../../packages/commands/generated/typescript/RenderViewParams';
import type { RunComparison } from '../../../../packages/commands/generated/typescript/RunComparison';
import type { RunInputRef } from '../../../../packages/commands/generated/typescript/RunInputRef';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { ScenarioInput } from '../../../../packages/commands/generated/typescript/ScenarioInput';
import type { SubjectRevisionRef } from '../../../../packages/commands/generated/typescript/SubjectRevisionRef';
import type { TemplateSubset } from '../../../../packages/commands/generated/typescript/TemplateSubset';
import type { FactoryRenderFrame } from '../features/render-surface/FactoryRenderSurface';
import type { RunPortOutput } from './runPortOutput';

/** Private Host/Native structs from host_render.rs; not another public caller or Host. */
export type RenderInputSource = { kind: 'project' } | { kind: 'definition_draft'; data: DefinitionScopeIdentity }
  | { kind: 'run'; data: { input: RunInputRef; definition: DefinitionScopeIdentity | null } };
export interface RenderFrame {
  task_id: string; epoch: number; project_id: string; scope: EditScope; scope_revision: number;
  source: RenderInputSource; request: RenderViewParams;
}
export type FrozenPortOutputRecord = [number, string, JobExecutionRef, string, SubjectRevisionRef, Quantity, boolean, string];
export interface FrozenRenderInput {
  project: ProjectState; scenarios: ScenarioInput[]; modules: ModulePackage[];
  definition: ModuleDefinition | null; template_subset: TemplateSubset | null;
  run: RunResult | null; active_job_id: string | null; objects_at: DomainObject[];
  port_outputs: FrozenPortOutputRecord[];
  port_output_summaries: RunPortOutput[];
  port_output_failures?: [PortRef, ApiError][];
  packaging: { artifact: ArtifactRecord; result: PackagingResult; job: PackageStructureJob; sample: PackagingProcessSample | null } | null;
  comparison: RunComparison | null;
  comparison_runs: { project: ProjectState; scenario: ScenarioInput; run: RunResult }[];
  assets: { source: ObjectSource; asset: AssetRef }[]; files: { file: FileRef }[]; issues: Issue[];
}
export interface NativeRenderInput { frame: RenderFrame; input: FrozenRenderInput }
export type RenderTaskReady = FactoryRenderFrame;
export interface RenderTaskFailure { stage: string; message: string; object_ref: ObjectRef | null }
export interface RenderTaskPorts {
  input(): Promise<NativeRenderInput | null>;
  asset(request: { source: ObjectSource; asset: AssetRef }): Promise<ArrayBuffer>;
  file(request: { file: FileRef }): Promise<ArrayBuffer>;
  ready(request: { frame: RenderFrame; ready: RenderTaskReady }): Promise<void>;
  failure(request: { frame: RenderFrame } & RenderTaskFailure): Promise<void>;
}
export const nativeRenderPorts: RenderTaskPorts = {
  input: () => invoke('native_render_input'),
  asset: request => invoke('native_render_asset', { request }),
  file: request => invoke('native_render_file', { request }),
  ready: request => invoke('native_render_ready', { request }),
  failure: request => invoke('native_render_failure', { request }),
};
/** Main returns null; query navigation never manufactures task authority. */
export const readNativeRenderInput = () => nativeRenderPorts.input();
export function renderTaskScope(frame: RenderFrame): Exclude<ApiScope, { kind: 'workspace' }> {
  return frame.scope.kind === 'project' ? { kind: 'project' } : { kind: 'definition_draft', data: frame.scope.data.id };
}
export function renderTaskSourceKey(frame: RenderFrame): string {
  const view = frame.request.view;
  return JSON.stringify([frame.task_id, frame.project_id, renderTaskScope(frame), frame.scope_revision, frame.project_id,
    view.project_revision, view.run_id == null ? null : { run_id: view.run_id, at_ms: view.view_time_ms }]);
}
export const renderTaskKey = (frame: RenderFrame) => JSON.stringify([renderTaskSourceKey(frame), frame.scope_revision, frame.request.view]);
