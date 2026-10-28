import type { AuthoringIO } from '../modules/types';
import type { PackagingScope, PackagingEvidence, ManufacturingExportPort } from './types';
import type { ManufacturingFileDeclaration } from '../../services/manufacturingExport';
import type { ExportPackage } from '../../../../../packages/commands/generated/typescript/ExportPackage';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ApiRequest } from '../../../../../packages/commands/generated/typescript/ApiRequest';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { BindingCommit } from '../../../../../packages/commands/generated/typescript/BindingCommit';
import type { PackageStructureJob } from '../../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { ProductRevision } from '../../../../../packages/commands/generated/typescript/ProductRevision';
import type { CalculationBudget } from '../../../../../packages/commands/generated/typescript/CalculationBudget';
import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { ArtifactAcknowledgment } from '../../../../../packages/commands/generated/typescript/ArtifactAcknowledgment';
import type { AdoptArtifactParams } from '../../../../../packages/commands/generated/typescript/AdoptArtifactParams';
import type { CapabilityCatalog } from '../../../../../packages/commands/generated/typescript/CapabilityCatalog';
import type { OperationName } from '../../../../../packages/commands/generated/typescript/OperationName';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { PackagingResult } from '../../../../../packages/commands/generated/typescript/PackagingResult';
import type { JobInputRevisionRef } from '../../../../../packages/commands/generated/typescript/JobInputRevisionRef';
import type { CancellationState } from '../../../../../packages/commands/generated/typescript/CancellationState';
import type { ProjectContext } from '../../../../../packages/commands/generated/typescript/ProjectContext';

export interface PackagingTarget { projectId: string; scope: PackagingScope; revision: number }
export function sameInput(left: unknown, right: unknown) {
  // Object field order can change across Host serialization; array order is input.
  const orderedFields = (_key: string, value: unknown) => value !== null && typeof value === 'object' && !Array.isArray(value)
    ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)) : value;
  return JSON.stringify(left, orderedFields) === JSON.stringify(right, orderedFields);
}
export function assertPackagingContext(expected: string, current: string, live: boolean) {
  if (!live || current !== expected) throw new Error('工程、会话或专业 Scope 已切换，停止展示旧响应。');
}
export const targetHeader = (target: PackagingTarget) => ({ api_version: '0.1' as const, request_id: `packaging-${crypto.randomUUID()}`, project_id: target.projectId, scope: target.scope });
export function scopeAvailable(catalog: CapabilityCatalog, operation: OperationName, scope: PackagingScope) {
  return catalog.implemented_operations.some(entry => entry.operation === operation && entry.allowed_scopes.some(allowed =>
    allowed.kind === scope.kind && (allowed.kind !== 'definition_draft' || (scope.kind === 'definition_draft' && allowed.data === scope.data))));
}
export function saveJob(io: AuthoringIO, target: PackagingTarget, job: PackageStructureJob) {
  return io.dispatch<EditCommit>({ ...targetHeader(target), operation: 'apply_commands', params: { base_revision: target.revision, commands: [{ kind: 'put_packaging_job', data: job }] } });
}
export function saveProduct(io: AuthoringIO, target: PackagingTarget, product: ProductRevision) {
  if (target.scope.kind !== 'project') throw new Error('产品属于实际工程；请在 Project 工作区保存，定义草稿不能改工程产品。');
  return io.dispatch<EditCommit>({ ...targetHeader(target), operation: 'apply_commands', params: { base_revision: target.revision, commands: [{ kind: 'put_object', data: { kind: 'product', data: product } }] } });
}
export function readScopeContext(io: AuthoringIO, target: PackagingTarget) {
  return io.dispatch<ProjectContext>({ ...targetHeader(target), operation: 'get_context', params: { selection: [] } });
}
export function commitProfessionalDefinitionVersion(io: AuthoringIO, target: PackagingTarget, version: string) {
  if (target.scope.kind !== 'definition_draft' || !version.trim()) throw new Error('仅在明确的 DefinitionDraft 输入版本后提交。');
  return io.dispatch<EditCommit>({ ...targetHeader(target), operation: 'apply_commands', params: { base_revision: target.revision, commands: [{ kind: 'commit_definition_version', data: { version: version.trim() } }] } });
}
export function assertTaskTarget(task: TaskHandle, target: PackagingTarget) {
  const scope = task.scope;
  if (task.input_revision !== target.revision || scope.kind !== 'edit'
    || (target.scope.kind === 'project' ? scope.data.kind !== 'project' || scope.data.data !== target.projectId
      : scope.data.kind !== 'definition_draft' || scope.data.data.id !== target.scope.data || scope.data.data.revision !== target.revision)) {
    throw new Error('Host Task 的工程、Scope 或输入修订与此次请求不同，停止观察。');
  }
}
export function cancelPackagingTask(io: AuthoringIO, target: PackagingTarget, task: TaskHandle) {
  assertTaskTarget(task, { ...target, revision: task.input_revision });
  return io.dispatch<CancellationState>({ ...targetHeader(target), operation: 'cancel_task', params: { task_id: task.id, action: 'cancel' } });
}
export async function computeJob(io: AuthoringIO, target: PackagingTarget, job: PackageStructureJob, budget: CalculationBudget, onState: (state: TaskState) => void, onTerminal?: (state: TaskState) => void | Promise<void>): Promise<PackagingEvidence> {
  const task = await io.dispatch<TaskHandle>({ ...targetHeader(target), operation: 'compute_packaging', params: { packaging_job: job, input_revision: target.revision, computation_budget: budget } });
  assertTaskTarget(task, target);
  let terminalState: TaskState | null = null;
  try {
    const output = await io.awaitTask(task, state => {
      assertTaskTarget(state.task, target);
      if (state.task.id !== task.id) throw new Error('Host 返回另一任务，停止展示。');
      terminalState = ['completed', 'failed', 'cancelled'].includes(state.task.state) ? state : null;
      onState(state);
    });
    if (output.kind !== 'packaging') throw new Error('实际 Task 未返回 PackagingResult，不能用 Completed 代替计算结果。');
    return await readTaskEvidence(io, target, output.data);
  } finally {
    // A cancelled worker can retain partial evidence and advance the actual Scope revision.
    if (terminalState) await onTerminal?.(terminalState);
  }
}
export async function readTaskEvidence(io: AuthoringIO, target: PackagingTarget, result: PackagingResult): Promise<PackagingEvidence> {
  const object = await io.dispatch<DomainObject>({ ...targetHeader(target), operation: 'get_object', params: { object_id: { kind: 'artifact', id: result.artifact_id }, source: { kind: 'draft' }, node_index: null } });
  if (object.kind !== 'artifact' || object.data.id !== result.artifact_id || !sameInput(object.data.subject_ref, result.subject_ref)) throw new Error('Host 未返回此结果的精确 Artifact。');
  const input = await io.dispatch<DomainObject>({ ...targetHeader(target), operation: 'get_object', params: { object_id: { kind: 'packaging_job', id: result.job_ref.id }, source: { kind: 'draft' }, node_index: null } });
  const exact = input.kind === 'packaging_job' && input.data.id === result.job_ref.id && input.data.revision === result.job_ref.revision;
  return { result, artifact: object.data, input: exact && input.kind === 'packaging_job' ? input.data : null,
    inputReadIssue: exact ? null : `Host 当前读取未返回结果所用的 ${result.job_ref.id}@r${result.job_ref.revision}；保留成果，但不以其他修订绘制过程。` };
}
export function acknowledgmentCommand(evidence: PackagingEvidence, purposes: string[], statement: string): ArtifactAcknowledgment {
  if (!statement.trim() || !purposes.length || purposes.some(value => !evidence.artifact.supported_scope.includes(value))) throw new Error('请明确选择该成果支持的认可用途并填写人类认可说明。');
  return { id: `ack-${crypto.randomUUID()}`, artifact_ref: evidence.artifact.id, accepted_subject: evidence.artifact.subject_ref,
    accepted_scope: purposes, decision_source: { kind: 'human_statement', data: statement.trim() }, conditions: [] };
}
export function artifactAcknowledgments(project: ProjectState, evidence: PackagingEvidence) {
  return project.acknowledgments.filter(value => value.artifact_ref === evidence.artifact.id
    && value.accepted_subject.id === evidence.artifact.subject_ref.id
    && value.accepted_subject.revision === evidence.artifact.subject_ref.revision);
}
export function recordAcknowledgment(io: AuthoringIO, target: PackagingTarget, acknowledgment: ArtifactAcknowledgment) {
  if (target.scope.kind !== 'project') throw new Error('人工认可与包装工位绑定属于实际 Project；定义草稿成果须先经真实工程采用。');
  return io.dispatch<EditCommit>({ ...targetHeader(target), operation: 'apply_commands', params: { base_revision: target.revision, commands: [{ kind: 'record_artifact_acknowledgment', data: acknowledgment }] } });
}
export function adoptArtifact(io: AuthoringIO, target: PackagingTarget, params: Omit<AdoptArtifactParams, 'base_revision'>) {
  if (target.scope.kind !== 'project') throw new Error('仅实际 Project 可绑定包装工位。');
  return io.dispatch<BindingCommit>({ ...targetHeader(target), operation: 'adopt_artifact', params: { ...params, base_revision: target.revision } });
}
export function selectStationJob(io: AuthoringIO, target: PackagingTarget, deviceId: string, job: JobInputRevisionRef) {
  if (target.scope.kind !== 'project') throw new Error('仅实际 Project 可修改包装工位的显式 Job 选择。');
  return io.dispatch<EditCommit>({ ...targetHeader(target), operation: 'apply_commands', params: { base_revision: target.revision, commands: [
    { kind: 'set_declared_parameter', data: { target: { kind: 'device', id: deviceId }, parameter_id: 'packaging_job_id', value: { kind: 'string', value: job.id }, basis: 'user_provided' } },
    { kind: 'set_declared_parameter', data: { target: { kind: 'device', id: deviceId }, parameter_id: 'packaging_job_revision', value: { kind: 'integer', value: job.revision }, basis: 'user_provided' } },
  ] } });
}
export function packagingStations(project: ProjectState, modules: Map<string, import('../../../../../packages/commands/generated/typescript/ModulePackage').ModulePackage>) {
  return project.devices.flatMap(device => {
    const deviceDefinition = modules.get(JSON.stringify([device.module_ref.id, device.module_ref.version]))?.definition;
    if (deviceDefinition?.payload.category !== 'devices') return [];
    const behavior = deviceDefinition.payload.data.behavior;
    const independent = device.operation_binding ?? (behavior.selected === 'operation_ref' ? behavior.operation_ref : null);
    const external = independent ? modules.get(JSON.stringify([independent.id, independent.version]))?.definition.payload : null;
    const operation = independent ? external?.category === 'operations' ? external.data : null : behavior.selected === 'inline' ? behavior.inline : null;
    if (operation?.builtin !== 'pack') return [];
    const jobSelectionAvailable = deviceDefinition.parameters.some(parameter => parameter.id === 'packaging_job_id' && parameter.value_type === 'string' && parameter.binding === '/payload/data/configuration/packaging_job_id/value')
      && deviceDefinition.parameters.some(parameter => parameter.id === 'packaging_job_revision' && parameter.value_type === 'integer' && parameter.integer && parameter.binding === '/payload/data/configuration/packaging_job_revision/value');
    return [{ device, operation: independent ?? device.module_ref, jobSelectionAvailable }];
  });
}
export async function exportEvidence(io: AuthoringIO, target: PackagingTarget, evidence: PackagingEvidence, onState: (state: TaskState) => void) {
  const moduleRefs = [];
  if (target.scope.kind === 'definition_draft') {
    const context = await readScopeContext(io, target);
    if (context.definition?.draft_id !== target.scope.data) throw new Error('Host 未返回当前专业定义草稿，不能猜测要导出的模块版本。');
    if (context.definition.issues.some(issue => issue.code === 'definition.unsaved_changes')) throw new Error('实际专业定义仍有未提交变更；请先明确提交定义版本，再导出含成果的模块工程包。');
    moduleRefs.push(context.definition.definition_ref);
  }
  const selected = await io.pickDirectory('export_package');
  if (!selected) return null;
  const request: ApiRequest = { ...targetHeader(target), operation: 'export_package', params: { revision: target.revision, selected_destination_ref: selected,
    package_scope: { purpose: target.scope.kind === 'project' ? 'private_project' : 'private_module', scenarios: [], run_ids: [], artifact_ids: [evidence.artifact.id], module_refs: moduleRefs, include_redistributable_assets: false } } };
  const task = await io.dispatch<TaskHandle>(request);
  assertTaskTarget(task, target);
  const output = await io.awaitTask(task, onState);
  if (output.kind !== 'package') throw new Error('实际导出 Task 未返回工程包。');
  return output.data;
}

export function manufacturingExportIdentity(session: string, target: PackagingTarget, evidence: PackagingEvidence | null) {
  return JSON.stringify([session, target, evidence?.artifact.id, evidence?.artifact.input_ref, evidence?.artifact.result_ref,
    evidence?.result.artifact_id, evidence?.result.job_ref, evidence?.result.manufacturing_files]);
}
export function emptyManufacturingDeclarations(evidence: PackagingEvidence): ManufacturingFileDeclaration[] {
  return evidence.result.manufacturing_files.map(({ file }) => ({ artifact_id: evidence.artifact.id, file: structuredClone(file), source: '', license: '' }));
}
export function manufacturingDeclarationsReady(evidence: PackagingEvidence, declarations: ManufacturingFileDeclaration[], confirmed: boolean) {
  const files = evidence.result.manufacturing_files.map(value => value.file);
  return confirmed && evidence.artifact.id === evidence.result.artifact_id && files.length > 0
    && declarations.length === files.length
    && new Set(declarations.map(value => JSON.stringify(value.file))).size === files.length
    && declarations.every(value => value.artifact_id === evidence.artifact.id && value.source.trim() && value.license.trim()
      && files.some(file => sameInput(file, value.file)));
}
/** One explicitly confirmed export candidate; source Project rights and professional acknowledgment stay unchanged. */
export async function exportDeclaredManufacturingEvidence(
  io: AuthoringIO, target: PackagingTarget, evidence: PackagingEvidence,
  declarations: ManufacturingFileDeclaration[], confirmed: boolean, producer: ManufacturingExportPort | undefined,
  onState: (state: TaskState) => void, guard: () => void,
): Promise<ExportPackage | null> {
  guard();
  if (!producer || producer.status !== 'available') throw new Error(producer?.reason ?? '制造文件声明导出尚未接入真实宿主。');
  const frozen = structuredClone({ target, evidence, declarations });
  if (!manufacturingDeclarationsReady(frozen.evidence, frozen.declarations, confirmed)) throw new Error('请逐文件填写当前成果的来源和许可，并明确确认本次携带；不能使用其他成果、缺项或重复文件的声明。');
  const moduleRefs: ModuleRef[] = [];
  if (frozen.target.scope.kind === 'definition_draft') {
    const context = await readScopeContext(io, frozen.target); guard();
    if (context.definition?.draft_id !== frozen.target.scope.data || context.definition.revision !== frozen.target.revision) throw new Error('专业 Scope 或修订已变化，请刷新后重新核对当前成果声明。');
    if (context.definition.issues.some(issue => issue.code === 'definition.unsaved_changes')) throw new Error('请先明确提交定义版本，再导出该草稿的专业成果。');
    moduleRefs.push(context.definition.definition_ref);
  }
  const selected = await io.pickDirectory('export_package'); guard();
  if (!selected) return null;
  const task = await producer.start({ project_id: frozen.target.projectId, scope: frozen.target.scope, scope_revision: frozen.target.revision,
    params: { revision: frozen.target.revision, selected_destination_ref: selected, package_scope: {
      purpose: frozen.target.scope.kind === 'project' ? 'private_project' : 'private_module', scenarios: [], run_ids: [],
      artifact_ids: [frozen.evidence.artifact.id], module_refs: moduleRefs, include_redistributable_assets: true,
    } }, declarations: frozen.declarations, confirm_redistribution: true });
  guard(); assertTaskTarget(task, frozen.target);
  const output = await io.awaitTask(task, state => {
    guard(); assertTaskTarget(state.task, frozen.target);
    if (state.task.id !== task.id || state.originating_operation !== 'export_package') throw new Error('Host 返回另一导出任务，停止展示旧响应。');
    onState(state);
  });
  guard();
  if (output.kind !== 'package') throw new Error('实际导出 Task 未返回工程包。');
  return output.data;
}
