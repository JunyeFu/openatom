import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { EditableProjectObject } from '../../../../../packages/commands/generated/typescript/EditableProjectObject';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { DeviceSetupPolicy } from '../../../../../packages/commands/generated/typescript/DeviceSetupPolicy';
import type { BusinessCommitProposal, BusinessSnapshot } from './types';

export function equal(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((value, index) => equal(value, b[index]));
  if (Array.isArray(b) || a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
  const left = a as Record<string, unknown>, right = b as Record<string, unknown>, keys = Object.keys(left);
  return keys.length === Object.keys(right).length && keys.every(key => Object.hasOwn(right, key) && equal(left[key], right[key]));
}
export function setupPoliciesEqual(requested: readonly DeviceSetupPolicy[], actual: readonly DeviceSetupPolicy[]): boolean {
  // These four Rust Option fields deserialize omission as None and serialize
  // None as null. Keep this DTO-specific meaning out of general draft equality.
  const canonical = (policy: DeviceSetupPolicy) => ({ ...policy,
    initial_family: policy.initial_family ?? null,
    first_article: policy.first_article ?? null,
    changeovers: policy.changeovers.map(rule => ({ ...rule,
      from_family: rule.from_family ?? null,
      duration: { ...rule.duration, source_ref: rule.duration.source_ref ?? null }
    }))
  });
  return Array.isArray(requested) && Array.isArray(actual) && equal(requested.map(canonical), actual.map(canonical));
}
export function getPath(object: unknown, path: (string | number)[]): unknown { return path.reduce<unknown>((value, key) => value && typeof value === 'object' ? (value as Record<string, unknown>)[key] : undefined, object); }
export function setPath<T>(value: T, path: (string | number)[], next: unknown): T {
  if (!path.length) return next as T;
  const [key, ...tail] = path;
  const copy = Array.isArray(value) ? [...value] : { ...(value as object) };
  (copy as Record<string | number, unknown>)[key] = setPath((value as Record<string | number, unknown>)[key], tail, next);
  return copy as T;
}
export const collectionKinds = { item_types: 'item_type', locations: 'location', resources: 'resource', calendars: 'calendar', products: 'product', boms: 'bom', routes: 'route', devices: 'device' } as const;
export function removeInstanceOverride(snapshot: BusinessSnapshot, deviceId: string, parameterId: string): BusinessSnapshot {
  return { ...snapshot, project: { ...snapshot.project, devices: snapshot.project.devices.map(device => {
    if (device.id !== deviceId) return device;
    const configuration = { ...device.configuration }; delete configuration[parameterId];
    return { ...device, configuration };
  }) } };
}
export function commitProposal(base: BusinessSnapshot, next: BusinessSnapshot, canEditSetupPolicies: boolean): BusinessCommitProposal {
  if (base.project.id !== next.project.id) throw new Error('当前表单所属工程已变化');
  const commands: EditCommand[] = [], scenario_revisions: Record<string, number> = {};
  for (const [collection, kind] of Object.entries(collectionKinds) as [keyof typeof collectionKinds, typeof collectionKinds[keyof typeof collectionKinds]][]) {
    const old = base.project[collection], current = next.project[collection];
    for (const value of current) if (!equal(old.find(v => v.id === value.id), value)) commands.push({ kind: 'put_object', data: { kind, data: value } as EditableProjectObject });
    for (const value of old) if (!current.some(v => v.id === value.id)) commands.push({ kind: 'remove_object', data: { kind, id: value.id } });
  }
  for (const scenario of next.scenarios) {
    const previous = base.scenarios.find(s => s.id === scenario.id);
    if (equal(previous, scenario)) continue;
    if (previous) scenario_revisions[scenario.id] = previous.revision;
    const data: ScenarioInput = { ...scenario, revision: previous?.revision ?? 1, project_revision: base.project.revision };
    commands.push({ kind: 'put_object', data: { kind: 'scenario', data } });
  }
  for (const previous of base.scenarios) if (!next.scenarios.some(s => s.id === previous.id)) {
    scenario_revisions[previous.id] = previous.revision; commands.push({ kind: 'remove_object', data: { kind: 'scenario', id: previous.id } });
  }
  const settings = (value: BusinessSnapshot['project']) => ({ title: value.title, time_reference: value.time_reference, generation_context: value.generation_context ?? null, sources: value.sources });
  const setupChanged = !equal(base.project.setup_policies, next.project.setup_policies);
  if (setupChanged && !canEditSetupPolicies) throw new Error('当前公共 Commands 尚未开放换单矩阵写入；预览保留，不能宣称已保存。');
  if (!equal(settings(base.project), settings(next.project)) || setupChanged) {
    const data = settings(next.project);
    if (canEditSetupPolicies) Object.assign(data, { setup_policies: next.project.setup_policies });
    commands.push({ kind: 'set_project_settings', data });
  }
  return { project_id: base.project.id, base_project_revision: base.project.revision, scenario_revisions, label: `业务输入 ${commands.length} 项`, commands };
}
export interface MergeConflict { path: string; base: unknown; mine: unknown; current: unknown }
const segment = (key: string) => key.replaceAll('~', '~0').replaceAll('/', '~1');
type ParameterCommand = Extract<EditCommand, {kind:'set_declared_parameter'}>;
export function mergeParameterCommands(base: BusinessSnapshot, commands: Record<string, ParameterCommand>, current: BusinessSnapshot, choices: Record<string, 'mine' | 'current'> = {}) {
  const merged: Record<string, ParameterCommand> = {}, conflicts: MergeConflict[] = [];
  function stored(snapshot: BusinessSnapshot, command: ParameterCommand) {
    const device = snapshot.project.devices.find(d => d.id === command.data.target.id);
    if (!device) return undefined;
    const source = snapshot.project.parameter_sources?.find(s => s.target.kind === 'device' && s.target.id === device.id && s.field_path === `/configuration/${segment(command.data.parameter_id)}/value`);
    return { value: device.configuration[command.data.parameter_id], basis: source?.basis ?? 'unknown' };
  }
  for (const [key, command] of Object.entries(commands)) {
    const latest = stored(current, command), mine = {value: command.data.value, basis: command.data.basis};
    const result = mergeDraft(stored(base, command), mine, latest, choices, `/declared_parameters/${segment(key)}`);
    conflicts.push(...result.conflicts);
    if (result.value && !equal(result.value, latest)) merged[key] = {kind:'set_declared_parameter', data:{...command.data, ...result.value as typeof mine}};
  }
  return { commands: merged, conflicts };
}
export function mergeDraft(base: unknown, mine: unknown, current: unknown, choices: Record<string, 'mine' | 'current'> = {}, path = ''): { value: unknown; conflicts: MergeConflict[] } {
  if (equal(base, mine)) return { value: current, conflicts: [] };
  if (equal(base, current) || equal(mine, current)) return { value: mine, conflicts: [] };
  if (choices[path]) return { value: choices[path] === 'mine' ? mine : current, conflicts: [] };
  const objects = [base, mine, current].every(v => v && typeof v === 'object' && !Array.isArray(v));
  if (objects) {
    const a = base as Record<string, unknown>, b = mine as Record<string, unknown>, c = current as Record<string, unknown>, value: Record<string, unknown> = {}, conflicts: MergeConflict[] = [];
    for (const key of new Set([...Object.keys(a), ...Object.keys(b), ...Object.keys(c)])) {
      const merged = mergeDraft(a[key], b[key], c[key], choices, `${path}/${segment(key)}`); if (merged.value !== undefined) value[key] = merged.value; conflicts.push(...merged.conflicts);
    }
    return { value, conflicts };
  }
  const arrays = [base, mine, current].every(v => Array.isArray(v) && v.every(x => x && typeof x === 'object' && typeof x.id === 'string'));
  if (arrays) {
    const a = base as { id: string }[], b = mine as { id: string }[], c = current as { id: string }[], value: unknown[] = [], conflicts: MergeConflict[] = [];
    const retained = a.map(v => v.id).filter(id => b.some(v => v.id === id) && c.some(v => v.id === id));
    const order = (rows: { id: string }[]) => rows.map(v => v.id).filter(id => retained.includes(id));
    const mineOrder = order(b), currentOrder = order(c), mineReordered = !equal(retained, mineOrder), currentReordered = !equal(retained, currentOrder);
    if (mineReordered && currentReordered && !equal(mineOrder, currentOrder)) return { value: current, conflicts: [{ path, base, mine, current }] };
    const first = mineReordered ? b : c, second = mineReordered ? c : b;
    for (const id of new Set([...first.map(v => v.id), ...second.map(v => v.id), ...a.map(v => v.id)])) {
      const merged = mergeDraft(a.find(v => v.id === id), b.find(v => v.id === id), c.find(v => v.id === id), choices, `${path}/@${segment(id)}`);
      if (merged.value !== undefined) value.push(merged.value); conflicts.push(...merged.conflicts);
    }
    return { value, conflicts };
  }
  return { value: current, conflicts: [{ path, base, mine, current }] };
}
// These canonical schemas describe references to existing objects/assets,
// unlike owning records such as OrderLine, ArrivalPlan or RouteDefinition.
// SchemaForm supplies the actual $ref name; do not infer this from an ID value
// or from a general "Ref" suffix.
const externalIdentitySchemas = new Set(['ModuleRef', 'SubjectRevisionRef', 'ObjectRef', 'AssetRef']);
export function copyRecord(value: unknown, schemaName: string): unknown {
  const copy = structuredClone(value) as Record<string, unknown>, fresh = (id: string) => `${id || 'row'}-copy-${crypto.randomUUID().slice(0, 8)}`;
  if (!copy || typeof copy !== 'object' || Array.isArray(copy)) return copy;
  if (typeof copy.id === 'string' && !externalIdentitySchemas.has(schemaName)) copy.id = fresh(copy.id);
  if (schemaName === 'ArrivalPlan' && Array.isArray(copy.items)) {
    const items = copy.items as { id: string; components: string[]; contents: string[]; holder: { kind: string; id: string } }[];
    const ids = new Map(items.map(i => [i.id, fresh(i.id)]));
    items.forEach(i => { i.id = ids.get(i.id)!; i.components = i.components.map(id => ids.get(id) ?? id); i.contents = i.contents.map(id => ids.get(id) ?? id); if (i.holder.kind === 'container' && ids.has(i.holder.id)) i.holder.id = ids.get(i.holder.id)!; });
  }
  if (schemaName === 'RouteDefinition' && Array.isArray(copy.steps)) {
    const steps = copy.steps as { id: string; predecessors: string[] }[], ids = new Map(steps.map(s => [s.id, fresh(s.id)]));
    steps.forEach(s => { s.id = ids.get(s.id)!; s.predecessors = s.predecessors.map(id => ids.get(id) ?? id); });
  }
  return copy;
}
