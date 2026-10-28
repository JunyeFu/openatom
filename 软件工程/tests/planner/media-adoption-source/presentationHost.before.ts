import { dispatch } from '../../../apps/planner/src/services/projectHost';
import { readMediaAsset, type MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import type { ApiRequest } from '../../../packages/commands/generated/typescript/ApiRequest';
import type { ApiScope } from '../../../packages/commands/generated/typescript/ApiScope';
import type { DomainObject } from '../../../packages/commands/generated/typescript/DomainObject';
import type { ModulePackage } from '../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleDefinition } from '../../../packages/commands/generated/typescript/ModuleDefinition';
import type { AssetRights } from '../../../packages/commands/generated/typescript/AssetRights';
import type { ProjectContext } from '../../../packages/commands/generated/typescript/ProjectContext';
import type { ModuleRef } from '../../../packages/commands/generated/typescript/ModuleRef';
import type { OperationDefinition } from '../../../packages/commands/generated/typescript/OperationDefinition';
import type { Presentation } from '../../../packages/commands/generated/typescript/Presentation';
import type { ProjectState } from '../../../packages/commands/generated/typescript/ProjectState';
import type { RunObjectsPage } from '../../../packages/commands/generated/typescript/RunObjectsPage';
import type { RunResult } from '../../../packages/commands/generated/typescript/RunResult';
import type { RunView } from '../../../packages/commands/generated/typescript/RunView';
import type { JobState } from '../../../packages/commands/generated/typescript/JobState';
import type { DeviceInstance } from '../../../packages/commands/generated/typescript/DeviceInstance';
import type { DeviceAction } from '../../../packages/commands/generated/typescript/DeviceAction';
import type { ActualMediaFrame, ActualPortOutputFrame } from '../../../packages/presentation/src';
import type { PortDefinition } from '../../../packages/commands/generated/typescript/PortDefinition';
import type { RunPortOutput, RunPortOutputReadRequest } from '../../../apps/planner/src/services/runPortOutput';
import type { ApiError } from '../../../packages/commands/generated/typescript/ApiError';

export interface ProcessScope {
  session_id: string;
  project_id: string;
  scope: Exclude<ApiScope, { kind: 'workspace' }>;
  /** Current authorized Project/Draft revision, distinct from Run.input_ref. */
  scope_revision: number;
}
export type DeviceProcessSelection =
  | { kind: 'actual_run'; run_id: string; job_id: string; at_ms: number }
  | { kind: 'definition'; module_ref: ModuleRef };
export interface ProcessReadContext extends ProcessScope { selection: DeviceProcessSelection }
export type ProcessAssetSource = MediaAssetReadRequest['source'];
export interface ProcessMediaBinding { session_id: string; request: Omit<MediaAssetReadRequest, 'asset'> }
export interface PresentationReadPorts {
  dispatch<T>(request: ApiRequest): Promise<T>;
  readAsset(request: MediaAssetReadRequest): Promise<ArrayBuffer>;
  /** Explicit test ports that omit this source must never invoke real Native. */
  readPortOutput?(request: RunPortOutputReadRequest): Promise<RunPortOutput>;
}
export interface ProcessDefinition {
  module: ModulePackage | null;
  definition: ModuleDefinition;
  rights: AssetRights[];
  module_ref: ModuleRef;
  presentation: Presentation;
  binding: ProcessMediaBinding;
  /** A real Draft definition is not a fabricated ModulePackage. */
  raw_definition?: { before_context: ProjectContext; definition: ModuleDefinition; after_context: ProjectContext };
}
export interface ProcessActionRequest extends ProcessScope {
  action: DeviceAction;
  binding: ProcessMediaBinding;
  module_ref: ModuleRef;
  device_id: string | null;
  run_id: string | null;
  job_id: string | null;
  at_ms: number | null;
}
export type ProcessActions = Partial<Record<DeviceAction, (request: ProcessActionRequest) => Promise<{ message: string }>>>;
export interface ActualProcessObservation extends ProcessDefinition {
  module: ModulePackage;
  run: RunResult;
  job: JobState;
  device: DeviceInstance | null;
  operation: OperationDefinition;
  operation_module: ModulePackage;
  frame: ActualMediaFrame;
  /** These are original SDK responses, retained separately from the view below. */
  raw: { run: RunResult; job_pages: RunObjectsPage[]; item_pages: RunObjectsPage[]; frozen_project: ProjectState };
  /** Local read-only composition. Resource objects are declarations, not occupancy. */
  view_snapshot: RunObjectsPage;
}

const scopeKey = (scope: ProcessScope['scope']) => scope.kind === 'project' ? 'project' : `draft:${scope.data}`;
export const processContextKey = (value: ProcessReadContext) => JSON.stringify([value.session_id, value.project_id,
  scopeKey(value.scope), value.scope_revision, value.selection.kind,
  value.selection.kind === 'actual_run' ? [value.selection.run_id, value.selection.job_id, value.selection.at_ms]
    : [value.selection.module_ref.id, value.selection.module_ref.version]]);
const sameRef = (a: ModuleRef, b: ModuleRef) => a.id === b.id && a.version === b.version;
const sourceRef = (source: ProcessAssetSource) => source.data.module_ref;
const sameSource = (a: ProcessAssetSource, b: ProcessAssetSource) => a.kind === b.kind && sameRef(sourceRef(a), sourceRef(b))
  && (a.kind !== 'run_module_version' || b.kind === 'run_module_version' && a.data.run_id === b.data.run_id && a.data.at_ms === b.data.at_ms);
const actualPorts: PresentationReadPorts = { dispatch, readAsset: readMediaAsset,
  readPortOutput: async request => (await import('../../../apps/planner/src/services/runPortOutput')).readRunPortOutput(request) };
type ReadOperation = 'get_context' | 'get_object' | 'get_run';
type ReadRequest = { [O in ReadOperation]: { operation: O; params: Extract<ApiRequest, { operation: O }>['params'] } }[ReadOperation];

export class ProcessSourceChanged extends Error {
  constructor() { super('工程、会话、Scope修订或观看来源已切换；旧响应已丢弃，本地编辑输入保留。'); this.name = 'ProcessSourceChanged'; }
}
export class ProcessUnavailable extends Error {
  constructor(message: string) { super(message); this.name = 'ProcessUnavailable'; }
}

/** Shared private getter projection for process detail and the one factory scene. */
export async function readActualPortOutputs(context: ProcessScope,
  actual: { run_id: string; at_ms: number; device: Pick<DeviceInstance, 'id' | 'module_ref'> | null; ports: PortDefinition[] },
  ports: Pick<PresentationReadPorts, 'readPortOutput'>, assertCurrent: () => void): Promise<ActualPortOutputFrame> {
  assertCurrent();
  const formalDevice = context.scope.kind === 'project' ? actual.device : null;
  const output: ActualPortOutputFrame = { run_id: actual.run_id, at_ms: actual.at_ms,
    device: formalDevice ? { id: formalDevice.id, module_ref: formalDevice.module_ref } : null, ports: [],
    ...(!formalDevice ? { not_applicable: 'DefinitionCycle没有正式Project设备实例端口，数量标签不适用。' } : {}) };
  if (formalDevice) for (const port of actual.ports) {
    if (port.payload.kind !== 'material' || !['output','bidirectional'].includes(port.direction)) continue;
    const reference = { module_ref: formalDevice.module_ref, instance_id: formalDevice.id, port_id: port.id }, unit = port.payload.data.unit;
    if (!ports.readPortOutput) { output.ports.push({ status: 'unavailable', port: reference, unit, reason: 'not_wired', message: '此显式读取端口未接入实际端口数量getter；不调用Native或补零。' }); continue; }
    try {
      const value = await ports.readPortOutput({ project_id: context.project_id, scope: context.scope,
        scope_revision: context.scope_revision, run_id: actual.run_id, at_ms: actual.at_ms, port: reference });
      assertCurrent();
      if (value.run_id !== actual.run_id || value.at_ms !== actual.at_ms || value.port.instance_id !== reference.instance_id
        || value.port.port_id !== reference.port_id || !sameRef(value.port.module_ref, reference.module_ref)) throw new ProcessUnavailable('实际端口数量回包改变了Run、时点或精确PortRef。');
      if (value.completed_output.unit !== unit || value.accepted_output.unit !== unit) throw new ProcessUnavailable('实际端口数量单位不匹配冻结物料输出端口。');
      output.ports.push({ ...value, status: 'available', unit });
    } catch (failure) {
      assertCurrent();
      const error = failure as { detail?: ApiError };
      if (error?.detail?.code !== 'UnsupportedCapability') throw failure;
      output.ports.push({ status: 'unavailable', port: reference, unit, reason: 'unsupported_capability', message: error.detail.message });
    }
  }
  assertCurrent(); return output;
}

function presentationOf(definition: ModuleDefinition): Presentation | null {
  const payload = definition.payload;
  if (payload.category === 'media') return payload.data;
  if (payload.category === 'devices' || payload.category === 'resources' || payload.category === 'transport') return payload.data.presentation ?? null;
  if (payload.category === 'items') return payload.data.presentation_or_media_ref ?? null;
  return null;
}

/** Same 18-operation SDK and private byte getter. No Host or business state is created. */
export function createPresentationHost(current: () => ProcessReadContext | null, ports: PresentationReadPorts = actualPorts) {
  function capture(context: ProcessReadContext) {
    const key = processContextKey(context);
    const assertCurrent = () => { const next = current(); if (!next || processContextKey(next) !== key) throw new ProcessSourceChanged(); };
    assertCurrent();
    const request = async <T>(input: ReadRequest): Promise<T> => {
      assertCurrent();
      const value = await ports.dispatch<T>({ api_version: '0.1', request_id: `process-${crypto.randomUUID()}`,
        project_id: context.project_id, scope: context.scope, ...input });
      assertCurrent();
      return value;
    };
    const source = (ref: ModuleRef): ProcessAssetSource => context.selection.kind === 'actual_run'
      ? { kind: 'run_module_version', data: { run_id: context.selection.run_id, at_ms: context.selection.at_ms, module_ref: ref } }
      : { kind: 'module_version', data: { module_ref: ref } };
    const module = async (ref: ModuleRef): Promise<ModulePackage> => {
      if (context.scope.kind === 'definition_draft' && context.selection.kind === 'definition') {
        throw new ProcessUnavailable('当前Host不支持DefinitionDraft Scope的ModuleVersion元数据读取；此精确媒体依赖尚不可解析，不切换Scope或借新库替代。');
      }
      const result = await request<DomainObject>({ operation: 'get_object', params: { object_id: { kind: 'module', id: ref.id }, source: source(ref), node_index: null } });
      if (result.kind !== 'module' || !sameRef(result.data.definition, ref)) throw new ProcessUnavailable('Host没有返回所选Scope中的精确模块版本。');
      return result.data;
    };
    const binding = (ref: ModuleRef): ProcessMediaBinding => ({ session_id: context.session_id, request: {
      project_id: context.project_id, scope: context.scope, scope_revision: context.scope_revision, source: source(ref),
    } });
    return { assertCurrent, request, source, module, binding };
  }

  async function read(context: ProcessReadContext): Promise<ProcessDefinition | ActualProcessObservation> {
    const bound = capture(context), selected = context.selection;
    if (selected.kind === 'definition') {
      if (context.scope.kind === 'definition_draft') {
        const before = await bound.request<ProjectContext>({ operation: 'get_context', params: { selection: [] } });
        const object = await bound.request<DomainObject>({ operation: 'get_object', params: { object_id: { kind: 'definition_draft', id: context.scope.data }, source: { kind: 'draft' }, node_index: null } });
        const after = await bound.request<ProjectContext>({ operation: 'get_context', params: { selection: [] } });
        if (object.kind !== 'definition' || !before.definition || !after.definition
          || before.host.session_id !== context.session_id || after.host.session_id !== context.session_id
          || before.project?.id !== context.project_id || after.project?.id !== context.project_id
          || before.definition.draft_id !== context.scope.data || after.definition.draft_id !== context.scope.data
          || before.definition.revision !== context.scope_revision || after.definition.revision !== context.scope_revision
          || !sameRef(after.definition.definition_ref, selected.module_ref) || !sameRef(object.data, selected.module_ref)) {
          throw new ProcessUnavailable('实际定义草稿或Scope修订已变化；请重读原Draft，本地输入保留。');
        }
        const presentation = presentationOf(object.data);
        if (!presentation) throw new ProcessUnavailable('此实际定义草稿没有过程表现声明。');
        return { module: null, definition: object.data, rights: object.data.contribution.assets,
          module_ref: selected.module_ref, presentation, binding: bound.binding(selected.module_ref),
          raw_definition: { before_context: before, definition: object.data, after_context: after } };
      }
      const module = await bound.module(selected.module_ref), presentation = presentationOf(module.definition);
      if (!presentation) throw new ProcessUnavailable('此精确版本没有可呈现的设备过程声明。');
      return { module, definition: module.definition, rights: module.rights, module_ref: selected.module_ref, presentation, binding: bound.binding(selected.module_ref) };
    }
    const result = await bound.request<RunView>({ operation: 'get_run', params: { run_id: selected.run_id, query: { kind: 'full_result' } } });
    if (result.kind !== 'full' || result.data.id !== selected.run_id || result.data.input_ref.project_id !== context.project_id) {
      throw new ProcessUnavailable('Host没有返回本工程所选Run的完整原结果。');
    }
    const run = result.data, at = selected.at_ms;
    if (run.computed_until_ms == null || at < run.window.start_ms || at > run.window.end_ms || at > run.computed_until_ms) {
      throw new ProcessUnavailable('此时点超出Run已知计算边界；停止实际呈现，不补为空闲。');
    }
    async function pages(objects: { kind: 'job'; id: string }[]): Promise<RunObjectsPage[]> {
      const output: RunObjectsPage[] = [];
      let after_index: number | null = null;
      do {
        const value: RunView = await bound.request<RunView>({ operation: 'get_run', params: { run_id: run.id, query: { kind: 'objects_at', data: { at_ms: at, objects, page: { after_index, limit: 200 } } } } });
        if (value.kind !== 'objects' || value.data.run_id !== run.id || value.data.at_ms !== at) throw new ProcessUnavailable('Host对象页不属于所选Run或时点。');
        if (value.data.next_index !== null && value.data.next_index <= (after_index ?? -1)) throw new ProcessUnavailable('Host分页游标没有前进，不能把不完整页当完整快照。');
        output.push(value.data); after_index = value.data.next_index;
      } while (after_index !== null);
      return output;
    }
    // Explicit job queries consume authoritative JobsAt history. Empty query retains
    // its original Item page meaning; Run.jobs is never used as a history fallback.
    const job_pages = await pages([{ kind: 'job', id: selected.job_id }]);
    const found = job_pages.flatMap(page => page.objects).filter(object => object.kind === 'job' && object.data.id === selected.job_id);
    if (found.length !== 1 || found[0].kind !== 'job') throw new ProcessUnavailable('所选时点没有唯一实际Job历史；不能从最终Run.jobs反推。');
    const job = found[0].data, item_pages = await pages([]);
    const projectObject = await bound.request<DomainObject>({ operation: 'get_object', params: {
      object_id: { kind: 'project', id: context.project_id }, source: { kind: 'run_snapshot', data: { run_id: run.id, at_ms: at } }, node_index: null,
    } });
    if (projectObject.kind !== 'project' || projectObject.data.id !== context.project_id || projectObject.data.revision !== run.input_ref.project_revision) {
      throw new ProcessUnavailable('Host没有返回此Run冻结输入修订的工程。');
    }
    const frozen = projectObject.data;
    const execution = job.execution_ref;
    const device = execution.kind === 'project_device'
      ? frozen.devices.find(device => device.id === execution.data.device_id) ?? null : null;
    if (job.execution_ref.kind === 'project_device' && !device) throw new ProcessUnavailable('此作业的实际设备不在Run冻结工程内。');
    const reference = device?.module_ref ?? (job.execution_ref.kind === 'definition_cycle' ? job.execution_ref.data.module_ref : null);
    if (!reference) throw new ProcessUnavailable('作业没有受支持的冻结执行来源。');
    const module = await bound.module(reference), payload = module.definition.payload;
    let operationModule: ModulePackage;
    let operation: OperationDefinition;
    if (sameRef(job.operation_ref, reference) && payload.category === 'devices' && payload.data.behavior.selected === 'inline' && payload.data.behavior.inline) {
      operationModule = module; operation = payload.data.behavior.inline;
    } else if (sameRef(job.operation_ref, reference) && payload.category === 'operations') {
      operationModule = module; operation = payload.data;
    } else {
      const declared = payload.category === 'devices' && payload.data.behavior.selected === 'operation_ref' ? payload.data.behavior.operation_ref : null;
      const allowed = [declared, device?.operation_binding, ...(device?.route_operation_refs ?? [])].some(ref => ref != null && sameRef(ref, job.operation_ref));
      if (!allowed) throw new ProcessUnavailable('Job的冻结工艺不匹配此设备已选inline/operation_ref或显式实例工艺绑定；不猜其它规则。');
      operationModule = await bound.module(job.operation_ref);
      if (operationModule.definition.payload.category !== 'operations') throw new ProcessUnavailable('此冻结工艺类别尚未接入设备过程呈现。');
      operation = operationModule.definition.payload.data;
    }
    const presentation = device?.presentation_override ?? presentationOf(module.definition);
    if (!presentation) throw new ProcessUnavailable('该冻结设备版本没有过程表现声明。');
    const view_snapshot: RunObjectsPage = { run_id: run.id, at_ms: at, next_index: null, objects: [
      { kind: 'job', data: job }, ...item_pages.flatMap(page => page.objects.filter(object => object.kind === 'item')),
      ...frozen.resources.map(data => ({ kind: 'resource' as const, data })),
    ] };
    // DefinitionCycle may retain fixture device declarations in its frozen test
    // project; Draft Scope does not make those formal adopted instances.
    const formalDevice = context.scope.kind === 'project' ? device : null;
    const port_outputs = await readActualPortOutputs(context, { run_id: run.id, at_ms: at, device: formalDevice,
      ports: payload.category === 'devices' ? payload.data.ports : [] }, ports, bound.assertCurrent);
    bound.assertCurrent();
    return { module, definition: module.definition, rights: module.rights, module_ref: reference, presentation, binding: bound.binding(reference), run, job, device: formalDevice, operation,
      operation_module: operationModule, view_snapshot, frame: { run, snapshot: view_snapshot, job_id: job.id, operation, port_outputs },
      raw: { run, job_pages, item_pages, frozen_project: frozen } };
  }

  function media(context: ProcessReadContext, binding: ProcessMediaBinding) {
    const bound = capture(context);
    const expected = bound.binding(sourceRef(binding.request.source));
    if (binding.session_id !== expected.session_id || binding.request.project_id !== expected.request.project_id
      || binding.request.scope_revision !== expected.request.scope_revision || scopeKey(binding.request.scope) !== scopeKey(expected.request.scope)
      || !sameSource(binding.request.source, expected.request.source)) throw new ProcessUnavailable('媒体请求不属于当前会话、Scope修订或精确来源。');
    return {
      async readAsset(request: MediaAssetReadRequest) {
        bound.assertCurrent();
        const requested = bound.binding(sourceRef(request.source)).request;
        if (request.project_id !== requested.project_id || request.scope_revision !== requested.scope_revision
          || scopeKey(request.scope) !== scopeKey(requested.scope) || !sameSource(request.source, requested.source)) throw new ProcessUnavailable('媒体字节请求改变了已绑定Scope或来源。');
        const bytes = await ports.readAsset(request); bound.assertCurrent(); return bytes;
      },
      async resolveModule(source: ProcessAssetSource) {
        const expectedSource = bound.source(sourceRef(source));
        if (!sameSource(source, expectedSource)) throw new ProcessUnavailable('媒体依赖改变了Run/时点或精确来源类型。');
        return bound.module(sourceRef(source));
      },
    };
  }
  async function action(context: ProcessReadContext, value: ProcessDefinition | ActualProcessObservation,
    selected: DeviceAction, actions: ProcessActions) {
    const bound = capture(context), payload = value.definition.payload, port = actions[selected];
    if (payload.category !== 'devices' || !payload.data.operations.includes(selected) || !port) {
      throw new ProcessUnavailable(`${selected} unavailable：当前精确声明与Root实际操作入口未同时具备。`);
    }
    // Validate the exact media provenance before crossing into Root's real ports.
    media(context, value.binding);
    const actual = 'frame' in value ? value as ActualProcessObservation : null;
    const result = await port({ ...context, action: selected, binding: value.binding, module_ref: value.module_ref,
      device_id: actual?.device?.id ?? null, run_id: actual?.run.id ?? null,
      job_id: actual?.job.id ?? null, at_ms: actual?.view_snapshot.at_ms ?? null });
    bound.assertCurrent(); return result;
  }
  return { read, media, action };
}
export type PresentationHost = ReturnType<typeof createPresentationHost>;
