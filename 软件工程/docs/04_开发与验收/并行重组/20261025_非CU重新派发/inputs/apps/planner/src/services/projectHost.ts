import { invoke } from '@tauri-apps/api/core';
import type { ApiRequest } from '../../../../packages/commands/generated/typescript/ApiRequest';
import type { ApiError } from '../../../../packages/commands/generated/typescript/ApiError';
import type { Response } from '../../../../packages/commands/generated/typescript/Response';
import type { CapabilityCatalog } from '../../../../packages/commands/generated/typescript/CapabilityCatalog';
import type { ProjectContext } from '../../../../packages/commands/generated/typescript/ProjectContext';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { DomainObject } from '../../../../packages/commands/generated/typescript/DomainObject';
import type { ModulePackage } from '../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleRef } from '../../../../packages/commands/generated/typescript/ModuleRef';
import type { ScenarioInput } from '../../../../packages/commands/generated/typescript/ScenarioInput';
import type { EditCommand } from '../../../../packages/commands/generated/typescript/EditCommand';
import type { EditCommit } from '../../../../packages/commands/generated/typescript/EditCommit';
import type { DepartureDecision } from '../../../../packages/commands/generated/typescript/DepartureDecision';
import type { SelectedFileRef } from '../../../../packages/commands/generated/typescript/SelectedFileRef';
import type { TaskHandle } from '../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../packages/commands/generated/typescript/TaskState';
import type { TaskOutput } from '../../../../packages/commands/generated/typescript/TaskOutput';
import type { ImportDraft } from '../../../../packages/commands/generated/typescript/ImportDraft';
import type { RunView } from '../../../../packages/commands/generated/typescript/RunView';
import type { RunQuery } from '../../../../packages/commands/generated/typescript/RunQuery';
import type { CancellationState } from '../../../../packages/commands/generated/typescript/CancellationState';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { ModuleTemplateCandidates } from '../../../../packages/commands/generated/typescript/ModuleTemplateCandidates';
import type { ApiCategory } from '../../../../packages/commands/generated/typescript/ApiCategory';
import type { LibrarySourceScope } from '../../../../packages/commands/generated/typescript/LibrarySourceScope';
import type { TemplatePreview } from '../../../../packages/commands/generated/typescript/TemplatePreview';
import type { TemplatePreparationChoices } from '../../../../packages/commands/generated/typescript/TemplatePreparationChoices';
import type { TemplateSeedSelection } from '../../../../packages/commands/generated/typescript/TemplateSeedSelection';
import type { FilePurpose } from '../../../../packages/commands/generated/typescript/FilePurpose';
import type { MetricScope } from '../../../../packages/commands/generated/typescript/MetricScope';
import type { RunComparison } from '../../../../packages/commands/generated/typescript/RunComparison';
import type { ExportEvaluationParams } from '../../../../packages/commands/generated/typescript/ExportEvaluationParams';
import type { ExportPackageParams } from '../../../../packages/commands/generated/typescript/ExportPackageParams';

export type { ProjectState, ScenarioInput, ModulePackage, EditCommand, DepartureDecision, TaskHandle, ImportDraft };
export const nativeHostAvailable = () => '__TAURI_INTERNALS__' in window;

export class HostFailure extends Error {
  constructor(readonly detail: ApiError) {
    super(detail.code === 'RevisionConflict' && detail.expected_revision !== null && detail.actual_revision !== null
      ? `修订已变化：此表单基于 r${detail.expected_revision}，当前为 r${detail.actual_revision}。输入仍保留，请刷新对应草稿后核对并重新编辑。`
      : detail.message);
    this.name = detail.code;
  }
}

// Only the native UI adapter establishes the UI session and its authority.
// Browser preview has no in-memory substitute Host or business-state simulator.
export async function dispatch<T>(request: ApiRequest): Promise<T> {
  if (!nativeHostAvailable()) throw new Error('请在超级工厂桌面应用中连接工程。');
  const response = await invoke<Response<T>>('project_api', { request });
  if (response.outcome.status === 'error') throw new HostFailure(response.outcome.data);
  if (response.outcome.status === 'managed') {
    throw new Error('此桌面读取返回了托管结果，当前适配尚不能读取该结果；原结果未改写。');
  }
  return response.outcome.data;
}

const header = (projectId: string | null) => ({
  api_version: '0.1' as const,
  request_id: `ui-${crypto.randomUUID()}`,
  project_id: projectId,
  scope: projectId === null ? { kind: 'workspace' as const } : { kind: 'project' as const },
});

export const moduleKey = (ref: ModuleRef) => JSON.stringify([ref.id, ref.version]);

export interface WorkspaceSnapshot {
  context: ProjectContext;
  capabilities: CapabilityCatalog;
  project: ProjectState | null;
  modules: Map<string, ModulePackage>;
  moduleIssues: { ref: ModuleRef; error: ApiError }[];
  scenarios: ScenarioInput[];
  scenarioIssues: { id: string; error: ApiError }[];
}

export async function readWorkspace(): Promise<WorkspaceSnapshot> {
  const context = await dispatch<ProjectContext>({ ...header(null), operation: 'get_context', params: { selection: [] } });
  const projectId = context.project?.id ?? null;
  const capabilities = await dispatch<CapabilityCatalog>({ ...header(null), operation: 'describe_capabilities', params: {} });
  if (!context.project) return { context, capabilities, project: null, modules: new Map(), moduleIssues: [], scenarios: [], scenarioIssues: [] };
  const object = await dispatch<DomainObject>({ ...header(projectId), operation: 'get_object', params: {
    object_id: { kind: 'project', id: context.project.id }, source: { kind: 'draft' }, node_index: null,
  } });
  if (object.kind !== 'project') throw new Error('Host 未返回工程对象。');
  const project = object.data;
  if (project.revision !== context.project.revision) throw new Error('读取期间工程已变化，请刷新后继续。');
  const [moduleObjects, scenarioObjects] = await Promise.all([
    Promise.allSettled(project.module_refs.map(ref => dispatch<DomainObject>({ ...header(project.id), operation: 'get_object', params: {
      object_id: { kind: 'module', id: ref.id }, source: { kind: 'module_version', data: { module_ref: ref } }, node_index: null,
    } }))),
    Promise.allSettled(project.scenario_refs.map(id => dispatch<DomainObject>({ ...header(project.id), operation: 'get_object', params: {
      object_id: { kind: 'scenario', id }, source: { kind: 'draft' }, node_index: null,
    } }))),
  ]);
  const modules = new Map<string, ModulePackage>();
  const moduleIssues: WorkspaceSnapshot['moduleIssues'] = [];
  moduleObjects.forEach((result, i) => {
    if (result.status === 'rejected') {
      if (!(result.reason instanceof HostFailure) || result.reason.detail.code !== 'MissingReference') throw result.reason;
      moduleIssues.push({ ref: project.module_refs[i], error: result.reason.detail });
      return;
    }
    const value = result.value;
    if (value.kind !== 'module') throw new Error('Host 未返回精确版本模块。');
    modules.set(moduleKey(project.module_refs[i]), value.data);
  });
  const scenarios: ScenarioInput[] = [];
  const scenarioIssues: WorkspaceSnapshot['scenarioIssues'] = [];
  scenarioObjects.forEach((result, i) => {
    if (result.status === 'rejected') {
      if (!(result.reason instanceof HostFailure) || result.reason.detail.code !== 'MissingReference') throw result.reason;
      scenarioIssues.push({ id: project.scenario_refs[i], error: result.reason.detail });
      return;
    }
    const value = result.value;
    if (value.kind !== 'scenario') throw new Error('Host 未返回方案对象。');
    scenarios.push(value.data);
  });
  // A second context is a consistency boundary, never an automatic retry or overwrite.
  const current = await dispatch<ProjectContext>({ ...header(project.id), operation: 'get_context', params: { selection: [] } });
  if (current.project?.id !== project.id || current.project.revision !== project.revision) {
    throw new Error('读取期间工程已变化，请刷新后继续。');
  }
  return { context: current, capabilities, project, modules, moduleIssues, scenarios, scenarioIssues };
}

export function applyCommands(project: ProjectState, commands: EditCommand[]) {
  return dispatch<EditCommit>({ ...header(project.id), operation: 'apply_commands', params: {
    base_revision: project.revision, commands,
  } });
}

export function readModuleLibrary(projectId: string | null, category: ApiCategory | null, sources: LibrarySourceScope[]) {
  return dispatch<ModuleTemplateCandidates>({ ...header(projectId), operation: 'query_module_library', params: {
    category_filters: category ? [category] : [], industry: null, product_family: null, site_envelope: null,
    source_scope: sources,
  } });
}

export function adoptDefinition(project: ProjectState, moduleRef: ModuleRef) {
  return applyCommands(project, [{ kind: 'adopt_module', data: {
    module_ref: moduleRef, targets: [], bindings: { item_types: {}, resources: {}, targets: {}, ports: {} },
    new_instances: [], explicit_seed: null,
  } }]);
}

export async function previewTemplate(project: ProjectState, moduleRef: ModuleRef,
  choices: TemplatePreparationChoices | null = null, seed: TemplateSeedSelection | null = null) {
  const object = await dispatch<DomainObject>({ ...header(project.id), operation: 'get_object', params: {
    object_id: { kind: 'module', id: moduleRef.id }, source: { kind: 'template_preview', data: {
      module_ref: moduleRef, base_revision: project.revision, choices, seed,
    } }, node_index: null,
  } });
  if (object.kind !== 'template_preview') throw new Error('Host 未返回模板的真实准备预览。');
  return object.data;
}

export function applyTemplate(project: ProjectState, preview: TemplatePreview) {
  if (!preview.command) throw new Error('该预览没有可采纳的模板结果，请先补齐选择。');
  return applyCommands({ ...project, revision: preview.base_revision }, [{ kind: 'instantiate_template', data: preview.command }]);
}

export function createProject(title: string, departure: DepartureDecision) {
  return dispatch<EditCommit>({ ...header(null), operation: 'apply_commands', params: {
    base_revision: null, commands: [{ kind: 'create_project', data: {
      title, time_reference: { label: '工程相对时间' }, departure,
    } }],
  } });
}

export async function selectDirectory(purpose: FilePurpose) {
  try { return await invoke<SelectedFileRef>('project_select_directory', { purpose }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}

export async function selectFile(purpose: 'import_package' | 'export_package') {
  try { return await invoke<SelectedFileRef>('project_select_file', { purpose }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}

function taskHeader(task: TaskHandle, definitionProjectId?: string) {
  if (task.scope.kind === 'workspace') return header(null);
  if (task.scope.data.kind === 'project') return header(task.scope.data.data);
  if (!definitionProjectId) throw new Error('定义任务需要其实际所属工程，不能借当前工程观察。');
  return { ...header(definitionProjectId), scope: { kind: 'definition_draft' as const, data: task.scope.data.data.id } };
}

export async function awaitTask(task: TaskHandle,
  onState?: (state: TaskState) => void, signal?: AbortSignal, definitionProjectId?: string): Promise<TaskOutput> {
  while (true) {
    signal?.throwIfAborted();
    const state = await getTaskState(task, definitionProjectId);
    onState?.(state);
    if (state.task.state === 'failed') {
      if (state.failure) throw new HostFailure(state.failure);
      throw new Error('计算失败，Host 未提供失败详情。');
    }
    if (state.task.state === 'cancelled') throw new Error('任务已取消。');
    if (state.task.state === 'completed') {
      if (!state.result) throw new Error('任务已终止，但尚无可读取结果。');
      return state.result;
    }
    // Poll only this actual Host task. Abort stops observation; it never claims cancellation.
    await new Promise<void>(resolve => window.setTimeout(resolve, 100));
  }
}

export async function saveProject(project: ProjectState, destination: SelectedFileRef,
  onState?: (state: TaskState) => void) {
  const task = await startSaveProject(project, destination);
  return awaitTask(task, onState);
}

export function startSaveProject(project: ProjectState, destination: SelectedFileRef) {
  return dispatch<TaskHandle>({ ...header(project.id), operation: 'save_project', params: {
    revision: project.revision, selected_directory_ref: destination,
  } });
}

export function getTaskState(task: TaskHandle, definitionProjectId?: string) {
  return dispatch<TaskState>({ ...taskHeader(task, definitionProjectId), operation: 'get_task', params: { task_id: task.id } });
}

export async function stageOpen(selection: SelectedFileRef, onState?: (state: TaskState) => void) {
  const task = await dispatch<TaskHandle>({ ...header(null), operation: 'import_package', params: {
    selected_file_ref: selection, import_policy: 'open_project', base_revision: null,
  } });
  const result = await awaitTask(task, onState);
  if (result.kind !== 'import') throw new Error('导入任务未返回待审工程。');
  return { task, draft: result.data };
}

export function adoptOpen(task: TaskHandle, draft: ImportDraft, departure: DepartureDecision, allowMissing: boolean) {
  return dispatch<EditCommit>({ ...header(null), operation: 'apply_commands', params: {
    base_revision: null, commands: [{ kind: 'adopt_imported_draft', data: { task_id: task.id, choices: {
      chosen_policy: 'open_project', objects_to_create: draft.proposed_objects,
      module_versions: draft.proposed_definitions, reused_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} }, selected_assets: [],
      accept_missing_as_draft: allowMissing, departure,
    } } }],
  } });
}

export async function runScenario(project: ProjectState, scenario: ScenarioInput, onState?: (state: TaskState) => void) {
  const task = await dispatch<TaskHandle>({ ...header(project.id), operation: 'run_scenario', params: {
    scenario_id: scenario.id, input_revision: project.revision,
    observation_window: scenario.window, intent: { kind: 'project_scenario' },
  } });
  return awaitTask(task, onState);
}

export function getRun(projectId: string, runId: string, query: RunQuery) {
  return dispatch<RunView>({ ...header(projectId), operation: 'get_run', params: { run_id: runId, query } });
}

export function cancelTask(task: TaskHandle, definitionProjectId?: string) {
  return dispatch<CancellationState>({ ...taskHeader(task, definitionProjectId), operation: 'cancel_task', params: { task_id: task.id, action: 'cancel' } });
}

export function compareActualRuns(projectId: string, runIds: string[], metricScope: MetricScope) {
  return dispatch<RunComparison>({ ...header(projectId), operation: 'compare_runs', params: {
    run_ids: runIds, metric_scope: metricScope,
  } });
}

export function startEvaluationExport(projectId: string, params: ExportEvaluationParams) {
  return dispatch<TaskHandle>({ ...header(projectId), operation: 'export_evaluation', params });
}

export function startPackageExport(projectId: string, params: ExportPackageParams) {
  return dispatch<TaskHandle>({ ...header(projectId), operation: 'export_package', params });
}

export async function readRunProject(projectId: string, runId: string, atMs: number) {
  const object = await dispatch<DomainObject>({ ...header(projectId), operation: 'get_object', params: {
    object_id: { kind: 'project', id: projectId }, source: { kind: 'run_snapshot', data: { run_id: runId, at_ms: atMs } }, node_index: null,
  } });
  if (object.kind !== 'project') throw new Error('Host 未返回该 Run 的冻结工程。');
  return object.data;
}

export async function readRunInputs(projectId: string, run: RunResult, atMs: number) {
  const project = await readRunProject(projectId, run.id, atMs);
  const scenarioObject = await dispatch<DomainObject>({ ...header(projectId), operation: 'get_object', params: {
    object_id: { kind: 'scenario', id: run.input_ref.scenario_id },
    source: { kind: 'run_snapshot', data: { run_id: run.id, at_ms: atMs } }, node_index: null,
  } });
  if (scenarioObject.kind !== 'scenario') throw new Error('Host 未返回该 Run 的冻结方案。');
  const moduleObjects = await Promise.allSettled(project.module_refs.map(ref => dispatch<DomainObject>({
    ...header(projectId), operation: 'get_object', params: {
      object_id: { kind: 'module', id: ref.id },
      source: { kind: 'run_module_version', data: { run_id: run.id, at_ms: atMs, module_ref: ref } }, node_index: null,
    },
  })));
  const modules = new Map<string, ModulePackage>();
  const moduleIssues: WorkspaceSnapshot['moduleIssues'] = [];
  moduleObjects.forEach((result, i) => {
    if (result.status === 'rejected') {
      if (!(result.reason instanceof HostFailure) || result.reason.detail.code !== 'MissingReference') throw result.reason;
      moduleIssues.push({ ref: project.module_refs[i], error: result.reason.detail });
      return;
    }
    if (result.value.kind !== 'module') throw new Error('Host 未返回该 Run 的精确版本模块。');
    modules.set(moduleKey(project.module_refs[i]), result.value.data);
  });
  return { project, scenario: scenarioObject.data, modules, moduleIssues };
}
