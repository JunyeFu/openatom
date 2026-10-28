import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { TemplateSubset } from '../../../../../packages/commands/generated/typescript/TemplateSubset';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { ScenarioReference } from '../../../../../packages/commands/generated/typescript/ScenarioReference';
import type { ModuleDefinition } from '../../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { TemplatePreview } from '../../../../../packages/commands/generated/typescript/TemplatePreview';
import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';
import type { ResourceDefinition } from '../../../../../packages/commands/generated/typescript/ResourceDefinition';
import type { AuthoringDraft, TemplateCompositionInput } from './types';
import { copy, refKey } from './model';

export interface TemplateCompositionState { subset: TemplateSubset; base_revision: number; source_scenario: ScenarioReference | null; issues?: Issue[] }
export function compositionFromPreview(preview: TemplatePreview): TemplateCompositionState {
  return { subset: copy(preview.subset), base_revision: preview.base_revision, source_scenario: preview.source_scenario ? copy(preview.source_scenario) : null, issues: copy(preview.issues) };
}
/** Explicit attachment freezes all three SDK-matched bodies and the exact source revision. */
export function captureCompleteSeed(subset: TemplateSubset, scenario: ScenarioInput) {
  return { subset: { ...subset, seed: { initial_conditions: copy(scenario.initial_conditions), arrivals: copy(scenario.arrivals), orders: copy(scenario.orders) } }, source_scenario: { scenario_id: scenario.id, revision: scenario.revision } };
}
export function clearCompositionSeed(subset: TemplateSubset) { return { subset: { ...subset, seed: null }, source_scenario: null }; }
export function compositionInput(projectId: string, value: TemplateCompositionState, definition: ModuleDefinition, draft: AuthoringDraft | null): TemplateCompositionInput {
  if (value.subset.seed && !value.source_scenario) throw new Error('此历史种子没有实际来源引用。请明确选择真实方案并重新附带完整声明种子，或清除种子后再保存；不会按正文猜测来源。');
  return { project_id: projectId, base_revision: value.base_revision, definition, draft, subset: { ...value.subset, adoption: null }, ...(value.subset.seed ? { source_scenario: value.source_scenario } : {}) };
}

export type CompositionGroup = 'halls' | 'lines' | 'devices' | 'connections' | 'item_types' | 'locations';
export const compositionGroups: { key: CompositionGroup; title: string }[] = [
  { key: 'halls', title: '厂房' }, { key: 'lines', title: '产线' }, { key: 'devices', title: '设备' },
  { key: 'connections', title: '连接' }, { key: 'item_types', title: '物品类型' }, { key: 'locations', title: '库位' },
];
export function emptyComposition(): TemplateSubset {
  return { layout_scope: 'region', halls: [], lines: [], devices: [], connections: [], module_refs: [], resource_requirements: [], item_types: [], role_requirements: [], suggested_role_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} }, locations: [], seed: null };
}
export function compositionObjectId(value: ProjectState[CompositionGroup][number]): string {
  return 'kind' in value && 'data' in value ? value.data.id : value.id;
}
export function selectCompositionObjects(subset: TemplateSubset, project: ProjectState, group: CompositionGroup, ids: string[]): TemplateSubset {
  const next = copy({ ...subset, [group]: project[group].filter(item => ids.includes(compositionObjectId(item))) });
  // Declared exact references travel with selected devices; no instance or runtime is adopted here.
  const refs = [...next.module_refs, ...next.devices.map(device => device.module_ref)];
  next.module_refs = [...new Map(refs.map(reference => [refKey(reference), reference])).values()];
  return next;
}
export function compositionResources(subset: TemplateSubset, project: ProjectState, source: ScenarioInput | null = null, reference: ScenarioReference | null = null) {
  const ids = new Set([...subset.devices.flatMap(device => Object.values(device.role_bindings.resources)), ...Object.values(subset.suggested_role_bindings.resources)]);
  if (subset.seed && source && reference && source.id === reference.scenario_id && source.revision === reference.revision) {
    for (const supply of source.supplies) for (const id of Object.values(supply.resources)) ids.add(id);
  }
  return project.resources.filter(resource => ids.has(resource.id)).map(source => {
    const requirement: ResourceDefinition = copy({ kind: source.kind, role: source.role, capacity: source.capacity, qualifications: source.qualifications, default_calendar_ms: project.calendars.filter(calendar => calendar.id === source.calendar_id).flatMap(calendar => calendar.intervals), presentation: source.presentation });
    return { source, requirement };
  });
}
/** Independent supply/demand projection retained for adoption; creation uses captureCompleteSeed. */
export function declaredSeed(scenario: ScenarioInput, supply: boolean, orders: boolean): TemplateSubset['seed'] {
  if (!supply && !orders) return null;
  return {
    initial_conditions: supply ? copy(scenario.initial_conditions) : { items: [], initial_setup_families: {}, declared_quality_decisions: [], material_allocations: [] },
    arrivals: supply ? copy(scenario.arrivals) : [],
    orders: orders ? copy(scenario.orders) : [],
  };
}
