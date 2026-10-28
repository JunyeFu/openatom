import * as host from './projectHost';
import { readDraft } from '../features/modules/model';
import type { WorkspaceSnapshot } from './projectHost';
import type { AuthoringDraft, AuthoringIO, AuthoringSamples, AuthoringTemplates } from '../features/modules/types';
import type { ApiRequest } from '../../../../packages/commands/generated/typescript/ApiRequest';
import type { CapabilityCatalog } from '../../../../packages/commands/generated/typescript/CapabilityCatalog';
import type { ProjectContext } from '../../../../packages/commands/generated/typescript/ProjectContext';
import type { TaskHandle } from '../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../packages/commands/generated/typescript/TaskState';
import type { ScenarioInput } from '../../../../packages/commands/generated/typescript/ScenarioInput';
import type { EditCommit } from '../../../../packages/commands/generated/typescript/EditCommit';
import type { DomainObject } from '../../../../packages/commands/generated/typescript/DomainObject';
import type { TemplateComposition } from '../../../../packages/commands/generated/typescript/TemplateComposition';
import type { TemplatePreview } from '../../../../packages/commands/generated/typescript/TemplatePreview';
import type { TemplatePreparationChoices } from '../../../../packages/commands/generated/typescript/TemplatePreparationChoices';
import type { TemplateSeedSelection } from '../../../../packages/commands/generated/typescript/TemplateSeedSelection';

/** Root keeps the authoritative snapshot; this adapter owns no Project or Run. */
export type AuthoringWorkspaceReader = () => WorkspaceSnapshot | null;

function binding(current: AuthoringWorkspaceReader) {
  const initial = current();
  if (!initial) throw new Error('尚未取得实际 Host 上下文，请先连接工程。');
  const sessionId = initial.context.host.session_id;
  const projectId = initial.project?.id ?? null;
  return {
    projectId,
    assertCurrent() {
      const next = current();
      if (!next || next.context.host.session_id !== sessionId || (next.project?.id ?? null) !== projectId) {
        throw new Error('Host 会话或工程已切换，请在当前工程重新打开定义创作；停止展示旧上下文结果，本地输入仍保留。');
      }
    },
    assertContext(context: ProjectContext) {
      if (context.host.session_id !== sessionId || (context.project?.id ?? null) !== projectId) {
        throw new Error('Host 返回的会话或工程已切换，请刷新当前上下文后继续。');
      }
    },
    assertTask(task: TaskHandle) {
      if ((task.scope.kind === 'workspace' && task.scope.data !== sessionId)
        || (task.scope.kind === 'edit' && task.scope.data.kind === 'project' && task.scope.data.data !== projectId)
        || (task.scope.kind === 'edit' && task.scope.data.kind === 'definition_draft' && projectId === null)) {
        throw new Error('任务不属于此创作会话或实际工程，不能改写其 Scope 后继续观察。');
      }
    },
  };
}

function sameTaskScope(a: TaskHandle['scope'], b: TaskHandle['scope']) {
  if (a.kind !== b.kind) return false;
  if (a.kind === 'workspace') return b.kind === 'workspace' && a.data === b.data;
  if (b.kind !== 'edit' || a.data.kind !== b.data.kind) return false;
  if (a.data.kind === 'project') return b.data.kind === 'project' && a.data.data === b.data.data;
  return b.data.kind === 'definition_draft' && a.data.data.id === b.data.data.id
    && a.data.data.revision === b.data.data.revision;
}

/** Reuses the native SDK. Package pickers select real ZIP files, not folders. */
export function createModuleAuthoringIO(current: AuthoringWorkspaceReader): AuthoringIO {
  const bound = binding(current);
  return {
    async dispatch<T>(request: ApiRequest): Promise<T> {
      bound.assertCurrent();
      if (request.scope.kind !== 'workspace' && request.project_id !== bound.projectId) {
        throw new Error('此定义请求属于其他工程，请刷新后重新选择。');
      }
      const result = await host.dispatch<T>(request);
      bound.assertCurrent();
      if (request.operation === 'get_context') bound.assertContext(result as ProjectContext);
      return result;
    },
    async awaitTask(task, onState) {
      bound.assertCurrent();
      bound.assertTask(task);
      const result = await host.awaitTask(task, state => {
        bound.assertCurrent();
        if (state.task.id !== task.id || !sameTaskScope(state.task.scope, task.scope)
          || state.task.input_revision !== task.input_revision) {
          throw new Error('Host 返回了不同任务、Scope 或输入修订；停止观察，原任务保持不变。');
        }
        onState?.(state);
      }, undefined, bound.projectId ?? undefined);
      bound.assertCurrent();
      return result;
    },
    async pickDirectory(purpose) {
      bound.assertCurrent();
      try {
        const selected = purpose === 'import_package' || purpose === 'export_package'
          ? await host.selectFile(purpose)
          : await host.selectDirectory(purpose);
        bound.assertCurrent();
        return selected;
      } catch (failure) {
        bound.assertCurrent();
        if (failure instanceof host.HostFailure && failure.detail.code === 'Cancelled') return null;
        throw failure;
      }
    },
  };
}

function sampleScopeListed(catalog: CapabilityCatalog, draftId: string) {
  return catalog.implemented_operations.some(operation => operation.operation === 'run_scenario'
    && operation.allowed_scopes.some(scope => scope.kind === 'definition_draft' && scope.data === draftId));
}

function unavailableReason(draft: AuthoringDraft | null): string {
  if (!draft) return '请先保存并读取实际定义草稿，再选择适用样例。';
  const payload = draft.definition.payload;
  switch (payload.category) {
    case 'items': return '物品定义使用规格、单位与跟踪检查；当前没有该草稿的实际生产周期映射。';
    case 'resources': return '资源定义检查容量、资格与日历；实际运行消费明确采用后的资源实例，不隐式创建人员或覆盖实例。';
    case 'templates': return '模板先保存实际组合并经真实准备/明确采用；供应和订单单独确认，本入口不隐式复制。';
    case 'professional': return '纸包装使用实际专业输入与 compute_packaging；其他提供者只按 Host 的真实能力显示。';
    case 'media': return '表现使用部件、锚点与动作预览；外观不会创建生产 Run 或库存。';
    case 'tasks':
      if (payload.data.kind === 'deferred_maintenance' || payload.data.kind === 'deferred_construction') {
        return '维修和施工任务保持执行后置，可保存与检查定义，当前没有可调用周期。';
      }
  }
  return '当前 Host 未为这个精确 DefinitionDraft Scope 提供可调用样例；请明确准备真实实例/角色/库位映射和方案输入。';
}

function request<O extends ApiRequest['operation']>(projectId: string | null,
  scope: ApiRequest['scope'], operation: O,
  params: Extract<ApiRequest, { operation: O }>['params']): Extract<ApiRequest, { operation: O }> {
  return { api_version: '0.1', request_id: `authoring-service-${crypto.randomUUID()}`,
    project_id: projectId, scope, operation, params } as Extract<ApiRequest, { operation: O }>;
}

/** Status consumes a real describe_capabilities snapshot; run rechecks it. */
export function createAuthoringSamples(current: AuthoringWorkspaceReader,
  draft: AuthoringDraft | null, io?: AuthoringIO): AuthoringSamples {
  const snapshot = current();
  if (!snapshot?.project) return { status: 'unavailable', reason: '请先打开实际工程，样例需要明确的工程实例、资源和库位映射。' };
  if (!draft || !sampleScopeListed(snapshot.capabilities, draft.summary.draft_id)) {
    return { status: 'unavailable', reason: unavailableReason(draft) };
  }
  const bound = binding(current);
  const actualIO = io ?? createModuleAuthoringIO(current);
  const projectId = snapshot.project.id;
  const draftId = draft.summary.draft_id;
  return {
    status: 'available',
    async run(selected: AuthoringDraft, scenario: ScenarioInput): Promise<TaskState> {
      bound.assertCurrent();
      if (selected.summary.draft_id !== draftId) throw new Error('所选定义草稿已变化，请刷新样例入口后继续。');
      const catalog = await actualIO.dispatch<CapabilityCatalog>(request(null, { kind: 'workspace' }, 'describe_capabilities', {}));
      if (!sampleScopeListed(catalog, draftId)) throw new Error(unavailableReason(selected));
      const task = await actualIO.dispatch<TaskHandle>(request(projectId, { kind: 'definition_draft', data: draftId }, 'run_scenario', {
        scenario_id: scenario.id,
        input_revision: selected.summary.revision,
        observation_window: scenario.window,
        intent: { kind: 'definition_cycle', data: { definition_revision: selected.summary.revision, test_input: scenario } },
      }));
      if (task.scope.kind !== 'edit' || task.scope.data.kind !== 'definition_draft'
        || task.scope.data.data.id !== draftId || task.scope.data.data.revision !== selected.summary.revision
        || task.input_revision !== selected.summary.revision) {
        throw new Error('Host 未返回所选定义修订的真实样例任务；不改写 Task Scope 或输入修订。');
      }
      const observed: { terminal: TaskState | null } = { terminal: null };
      const observe = (state: TaskState) => {
        if (state.task.state === 'completed' || state.task.state === 'failed' || state.task.state === 'cancelled') observed.terminal = state;
      };
      try {
        await actualIO.awaitTask(task, observe);
      } catch (failure) {
        bound.assertCurrent();
        // Failed/cancelled are actual terminal TaskStates, not transport errors
        // or a manufactured success. Keep the exact original result/failure.
        if (observed.terminal?.task.state === 'failed' || observed.terminal?.task.state === 'cancelled') return observed.terminal;
        throw failure;
      }
      bound.assertCurrent();
      if (!observed.terminal) throw new Error('真实任务观察器没有返回终态；不推定样例已完成。');
      return observed.terminal;
    },
  };
}

/** Actual partial success: only creation committed, composition did not. */
export class TemplateCompositionSaveFailure extends Error {
  constructor(cause: unknown, readonly createdDraft: AuthoringDraft, readonly creationCommit: EditCommit) {
    super(`已创建实际定义草稿 ${createdDraft.summary.draft_id} r${createdDraft.summary.revision}，组合尚未保存；本地定义与组合输入仍应保留。${cause instanceof Error ? cause.message : String(cause)}`, { cause });
    this.name = 'TemplateCompositionSaveFailure';
  }
}

/** Use when the currently connected Host lacks the actual composition command. */
export function unavailableAuthoringTemplates(): Extract<AuthoringTemplates, { status: 'unavailable' }> {
  return { status: 'unavailable', reason: '当前 Host 未提供实际模板组合保存命令；可保留组合输入，连接支持此能力的 Host 后继续。' };
}

export function createAuthoringTemplates(current: AuthoringWorkspaceReader, io?: AuthoringIO): AuthoringTemplates {
  const snapshot = current();
  if (!snapshot?.project || !snapshot.capabilities.available_command_kinds.includes('put_template_composition')) {
    return unavailableAuthoringTemplates();
  }
  const bound = binding(current);
  const actualIO = io ?? createModuleAuthoringIO(current);
  const projectId = snapshot.project.id;
  return {
    status: 'available',
    async save(input) {
      bound.assertCurrent();
      if (input.project_id !== projectId) throw new Error('组合来源工程已变化，请在原工程刷新后重新选择；不会改写为当前工程。');
      const catalog = await actualIO.dispatch<CapabilityCatalog>(request(null, { kind: 'workspace' }, 'describe_capabilities', {}));
      if (!catalog.available_command_kinds.includes('put_template_composition')) throw new Error(unavailableAuthoringTemplates().reason);
      let active = input.draft;
      let creation: EditCommit | null = null;
      if (!active) {
        creation = await actualIO.dispatch<EditCommit>(request(projectId, { kind: 'project' }, 'apply_commands', {
          base_revision: input.base_revision,
          commands: [{ kind: 'create_definition_draft', data: input.definition }],
        }));
        if (creation.scope.kind !== 'definition_draft') throw new Error('Host 未返回实际新建定义草稿 Scope。');
        active = await readDraft(actualIO, projectId, creation.scope.data.id, creation.history, creation.new_revision);
      }
      const composition: TemplateComposition = {
        project_revision: input.base_revision,
        definition: input.definition,
        subset: input.subset,
        // This is the explicitly captured source, not the currently selected
        // Scenario. Preserve omission/null and let Host report exact conflicts.
        ...(input.source_scenario === undefined ? {} : { source_scenario: input.source_scenario }),
      };
      let commit: EditCommit;
      try {
        commit = await actualIO.dispatch<EditCommit>(request(projectId, { kind: 'definition_draft', data: active.summary.draft_id }, 'apply_commands', {
          base_revision: active.summary.revision,
          commands: [{ kind: 'put_template_composition', data: composition }],
        }));
      } catch (failure) {
        bound.assertCurrent();
        if (creation && failure instanceof host.HostFailure) throw new TemplateCompositionSaveFailure(failure, active, creation);
        throw failure;
      }
      if (commit.scope.kind !== 'definition_draft' || commit.scope.data.id !== active.summary.draft_id) {
        throw new Error('Host 未返回所选模板定义的实际组合提交 Scope。');
      }
      const saved = await readDraft(actualIO, projectId, active.summary.draft_id, commit.history, commit.new_revision);
      bound.assertCurrent();
      return { draft: saved, commit };
    },
  };
}

/** Draft source revision and returned Project preparation revision stay distinct. */
export async function readAuthoringTemplatePreview(current: AuthoringWorkspaceReader,
  draft: AuthoringDraft, choices: TemplatePreparationChoices | null = null,
  seed: TemplateSeedSelection | null = null, io?: AuthoringIO): Promise<TemplatePreview> {
  const bound = binding(current);
  if (!bound.projectId) throw new Error('模板准备需要实际工程与读取权限。');
  const actualIO = io ?? createModuleAuthoringIO(current);
  const reference = draft.summary.definition_ref;
  const object = await actualIO.dispatch<DomainObject>(request(bound.projectId, { kind: 'definition_draft', data: draft.summary.draft_id }, 'get_object', {
    object_id: { kind: 'module', id: reference.id },
    source: { kind: 'template_preview', data: { module_ref: reference, base_revision: draft.summary.revision, choices, seed } },
    node_index: null,
  }));
  bound.assertCurrent();
  if (object.kind !== 'template_preview' || object.data.project_id !== bound.projectId
    || object.data.module_ref.id !== reference.id || object.data.module_ref.version !== reference.version) {
    throw new Error('Host 未返回此精确定义草稿的真实模板准备预览。');
  }
  return object.data;
}
