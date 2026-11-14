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
  if (selected.kind === 'frozen_device' && context.scope.kind !== 'project') return fail('该次运行的设备不属于当前工程，请重新选择来源。');
  if (selected.kind === 'frozen_definition' && context.scope.kind !== 'definition_draft') return fail('请选择该次运行使用的定义草稿。');
  async function checkContext() {
    const actual = await reader.request<ProjectContext>({ operation: 'get_context', params: { selection: [] } });
    if (actual.host.session_id !== context.session_id || actual.project?.id !== context.project_id
      || (context.scope.kind === 'project' ? actual.project.revision !== context.scope_revision
        : actual.definition?.draft_id !== context.scope.data || actual.definition.revision !== context.scope_revision)) return fail('工程连接或版本已变化，请重新读取原记录。');
  }
  await checkContext();
  const result = await reader.request<RunView>({ operation: 'get_run', params: { run_id: selected.run_id, query: { kind: 'full_result' } } });
  if (result.kind !== 'full' || result.data.id !== selected.run_id || result.data.input_ref.project_id !== context.project_id) return fail('未能完整读取所选运行结果，请重新加载。');
  const run = result.data, at = selected.at_ms;
  if (!Number.isSafeInteger(at) || run.computed_until_ms == null || at < run.window.start_ms || at > run.window.end_ms || at > run.computed_until_ms) return fail('所选时间点尚未计算，请选择已计算的时间。');
  const source = { kind: 'run_snapshot' as const, data: { run_id: run.id, at_ms: at } };
  const project = await reader.request<DomainObject>({ operation: 'get_object', params: { object_id: { kind: 'project', id: context.project_id }, source, node_index: null } });
  if (project.kind !== 'project' || project.data.id !== context.project_id || project.data.revision !== run.input_ref.project_revision) return fail('未能读取该次运行使用的工程版本。');
  const object_id = selected.kind === 'frozen_device' ? { kind: 'device' as const, id: selected.device_id }
    : context.scope.kind === 'definition_draft' ? { kind: 'definition_draft' as const, id: context.scope.data } : fail('该次运行的定义缺少草稿编号。');
  const owner = await reader.request<DomainObject>({ operation: 'get_object', params: { object_id, source, node_index: null } });
  let reference: ModuleRef, presentation: Presentation | null, device: ProcessDefinition['device'];
  if (selected.kind === 'frozen_device') {
    const frozen = project.data.devices.find(value => value.id === selected.device_id);
    if (!frozen || owner.kind !== 'device' || owner.data.id !== frozen.id || owner.data.module_ref.id !== frozen.module_ref.id || owner.data.module_ref.version !== frozen.module_ref.version) return fail('所选运行记录中没有匹配的设备版本。');
    reference = frozen.module_ref; device = owner.data; presentation = owner.data.presentation_override ?? null;
  } else {
    if (owner.kind !== 'definition') return fail('未能读取该次运行保存的定义。');
    reference = { id: owner.data.id, version: owner.data.version }; device = null; presentation = presentationOf(owner.data);
  }
  const module = await reader.module(reference);
  if (selected.kind === 'frozen_device' && module.definition.payload.category !== 'devices') return fail('该次运行使用的模块不是设备定义。');
  presentation ??= presentationOf(module.definition);
  if (!presentation) return fail('此精确冻结来源没有表现声明。');
  await checkContext(); reader.assertCurrent();
  return { kind: 'frozen_static', module, definition: module.definition, rights: module.rights, module_ref: reference, presentation,
    binding: reader.binding(reference), device, run,
    node_owner: selected.kind === 'frozen_definition' || device?.presentation_override ? { object_id, source }
      : { object_id: { kind: 'module', id: reference.id }, source: reader.source(reference) },
    raw_static: { run, frozen_project: project.data, frozen_owner: owner } };
}
