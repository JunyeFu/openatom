import type { ApiRequest } from '../../../../packages/commands/generated/typescript/ApiRequest';
import type { DomainObject } from '../../../../packages/commands/generated/typescript/DomainObject';
import type { ModuleDefinition } from '../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { ModulePackage } from '../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleRef } from '../../../../packages/commands/generated/typescript/ModuleRef';
import type { Presentation } from '../../../../packages/commands/generated/typescript/Presentation';
import type { ProjectContext } from '../../../../packages/commands/generated/typescript/ProjectContext';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { RunView } from '../../../../packages/commands/generated/typescript/RunView';
import type { ProcessDefinition, ProcessReadContext, ProcessMediaBinding, ProcessAssetSource } from './presentationHost';
type Operation = 'get_context' | 'get_object' | 'get_run';
type Request = { [O in Operation]: { operation: O; params: Extract<ApiRequest, { operation: O }>['params'] } }[Operation];
interface Reader { request<T>(input: Request): Promise<T>; assertCurrent(): void; module(ref: ModuleRef): Promise<ModulePackage>; source(ref: ModuleRef): ProcessAssetSource; binding(ref: ModuleRef): ProcessMediaBinding }
export interface FrozenProcessObservation extends ProcessDefinition {
  kind: 'frozen_static';
  module: ModulePackage;
  run: RunResult;
  raw_static: { run: RunResult; frozen_project: ProjectState; frozen_owner: DomainObject };
}
/** Frozen declaration has no synthetic Job, phase, movement, port quantity or clock. */
export async function readFrozenProcess(context: ProcessReadContext, reader: Reader,
  presentationOf: (definition: ModuleDefinition) => Presentation | null, fail: (message: string) => never): Promise<FrozenProcessObservation> {
  const selected = context.selection;
  if (selected.kind !== 'frozen_device' && selected.kind !== 'frozen_definition') return fail('没有明确冻结静态来源。');
  if (selected.kind === 'frozen_device' && context.scope.kind !== 'project') return fail('冻结设备实例只属于原Project Scope；DefinitionCycle不能借同名设备。');
  if (selected.kind === 'frozen_definition' && context.scope.kind !== 'definition_draft') return fail('冻结定义声明必须保留原opaque DefinitionDraft Scope。');
  async function checkContext() {
    const actual = await reader.request<ProjectContext>({ operation: 'get_context', params: { selection: [] } });
    if (actual.host.session_id !== context.session_id || actual.project?.id !== context.project_id
      || (context.scope.kind === 'project' ? actual.project.revision !== context.scope_revision
        : actual.definition?.draft_id !== context.scope.data || actual.definition.revision !== context.scope_revision)) return fail('冻结静态检查的实际会话/工程/Scope修订已变化；请重读原来源。');
  }
  await checkContext();
  const result = await reader.request<RunView>({ operation: 'get_run', params: { run_id: selected.run_id, query: { kind: 'full_result' } } });
  if (result.kind !== 'full' || result.data.id !== selected.run_id || result.data.input_ref.project_id !== context.project_id) return fail('Host没有返回同工程所选Run的完整原结果。');
  const run = result.data, at = selected.at_ms;
  if (!Number.isSafeInteger(at) || run.computed_until_ms == null || at < run.window.start_ms || at > run.window.end_ms || at > run.computed_until_ms) return fail('静态检查时点超出Run已知计算边界；不借当前输入或补成空闲。');
  const source = { kind: 'run_snapshot' as const, data: { run_id: run.id, at_ms: at } };
  const project = await reader.request<DomainObject>({ operation: 'get_object', params: { object_id: { kind: 'project', id: context.project_id }, source, node_index: null } });
  if (project.kind !== 'project' || project.data.id !== context.project_id || project.data.revision !== run.input_ref.project_revision) return fail('Host没有返回此Run冻结输入修订的真实Project。');
  const object_id = selected.kind === 'frozen_device' ? { kind: 'device' as const, id: selected.device_id }
    : context.scope.kind === 'definition_draft' ? { kind: 'definition_draft' as const, id: context.scope.data } : fail('冻结定义缺少原opaque Draft身份。');
  const owner = await reader.request<DomainObject>({ operation: 'get_object', params: { object_id, source, node_index: null } });
  let reference: ModuleRef, presentation: Presentation | null, device: ProcessDefinition['device'];
  if (selected.kind === 'frozen_device') {
    const frozen = project.data.devices.find(value => value.id === selected.device_id);
    if (!frozen || owner.kind !== 'device' || owner.data.id !== frozen.id || owner.data.module_ref.id !== frozen.module_ref.id || owner.data.module_ref.version !== frozen.module_ref.version) return fail('该设备不在所选Run冻结Project内，或精确设备来源不匹配。');
    reference = frozen.module_ref; device = owner.data; presentation = owner.data.presentation_override ?? null;
  } else {
    if (owner.kind !== 'definition') return fail('Host没有返回该opaque Draft/Run的冻结定义；不能借当前Draft。');
    reference = { id: owner.data.id, version: owner.data.version }; device = null; presentation = presentationOf(owner.data);
  }
  const module = await reader.module(reference);
  if (selected.kind === 'frozen_device' && module.definition.payload.category !== 'devices') return fail('冻结设备的精确Module不是设备定义。');
  presentation ??= presentationOf(module.definition);
  if (!presentation) return fail('此精确冻结来源没有表现声明。');
  await checkContext(); reader.assertCurrent();
  return { kind: 'frozen_static', module, definition: module.definition, rights: module.rights, module_ref: reference, presentation,
    binding: reader.binding(reference), device, run,
    node_owner: selected.kind === 'frozen_definition' || device?.presentation_override ? { object_id, source }
      : { object_id: { kind: 'module', id: reference.id }, source: reader.source(reference) },
    raw_static: { run, frozen_project: project.data, frozen_owner: owner } };
}
