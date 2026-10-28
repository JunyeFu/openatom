import type { AcceptanceScope } from '../../../../../packages/commands/generated/typescript/AcceptanceScope';
import type { Quantity } from '../../../../../packages/commands/generated/typescript/Quantity';
import type { MetricValue } from '../../../../../packages/commands/generated/typescript/MetricValue';
import type { NumericDifference } from '../../../../../packages/commands/generated/typescript/NumericDifference';
import type { MetricKey } from '../../../../../packages/commands/generated/typescript/MetricKey';
import type { ScenarioReference } from '../../../../../packages/commands/generated/typescript/ScenarioReference';
import type { DeliveryServices, RunEvidence } from './types';
import type { ApplyCommandsParams } from '../../../../../packages/commands/generated/typescript/ApplyCommandsParams';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';

export const metricLabels: Record<MetricKey, string> = { accepted_quantity: '实际接受量', completion_time: '完整接受时刻', waiting_time: '等待时间', resource_utilization: '资源利用率', unit_cost: '单位成本', rework_quantity: '返工数量' };
export function quantityText(q: Quantity): string {
  const raw = String(q.value.coefficient).padStart(q.value.decimal_places + 1, '0');
  const p = q.value.decimal_places;
  return `${p ? `${raw.slice(0, -p)}.${raw.slice(-p)}` : raw} ${q.unit}`;
}
export function metricText(v: MetricValue): string {
  switch (v.kind) {
    case 'quantity': return quantityText(v.data);
    case 'duration': return `${v.data} ms`;
    case 'scalar': return `${v.data.value} ${v.data.unit}`;
    case 'money_per_unit': return `${v.data.amount} ${v.data.currency}/${v.data.unit}`;
    case 'unknown': return `未知：${v.data.message}`;
  }
}
export function differenceText(v: NumericDifference | null | undefined): string {
  if (v == null) return '不适用／未知，见比较条件';
  switch (v.kind) {
    case 'duration': return `${v.data.negative ? '−' : '+'}${v.data.magnitude} ms`;
    case 'quantity': return `${v.data.negative ? '−' : '+'}${quantityText(v.data.magnitude)}`;
    case 'scalar': return `${v.data.value} ${v.data.unit}`;
    case 'money_per_unit': return `${v.data.amount} ${v.data.currency}/${v.data.unit}`;
  }
}
export const scopeKey = (s: AcceptanceScope) => JSON.stringify([s.product_ref.id, s.product_ref.revision, s.unit, s.destination, s.stage]);
export const scopeText = (s: AcceptanceScope) => `${s.product_ref.id}@${s.product_ref.revision} · ${s.unit} → ${s.destination} / ${s.stage}`;
export function acceptanceScopes(evidence: readonly RunEvidence[]): AcceptanceScope[] {
  const scopes = new Map<string, AcceptanceScope>();
  for (const { project, scenario } of evidence) {
    const goal = scenario.acceptance_goal ?? project.generation_context?.goal;
    const declared: AcceptanceScope[] = goal ? [goal.acceptance_scope] : scenario.orders.flatMap(order => order.quantity ? [{ product_ref: order.product_ref, unit: order.quantity.unit, destination: order.destination, stage: order.acceptance_stage }] : []);
    for (const s of declared) scopes.set(scopeKey(s), s);
  }
  return [...scopes.values()];
}
export function scenarioReferences(evidence: readonly RunEvidence[]): ScenarioReference[] {
  const refs = new Map<string, ScenarioReference>();
  for (const { run } of evidence) {
    const ref = { scenario_id: run.input_ref.scenario_id, revision: run.input_ref.scenario_revision };
    refs.set(JSON.stringify(ref), ref);
  }
  return [...refs.values()];
}
export function evidenceMatches(projectId: string, runIds: readonly string[], evidence: readonly RunEvidence[]): boolean {
  return evidence.length === runIds.length && runIds.every(id => evidence.some(e => e.run.id === id && e.run.evidence_origin === 'actual_software' && e.run.input_ref.project_id === projectId && e.project.id === projectId && e.project.revision === e.run.input_ref.project_revision && e.scenario.id === e.run.input_ref.scenario_id && e.scenario.revision === e.run.input_ref.scenario_revision));
}
export function errorText(error: unknown): string {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error);
}
export function errorDetail(error: unknown): unknown {
  return typeof error === 'object' && error !== null && 'detail' in error ? error.detail : null;
}

/** Prepare the existing edit operation; the caller must submit it and read the committed Scenario. */
export function prepareScenarioFork(project: ProjectState, evidence: RunEvidence,
  request: Parameters<NonNullable<DeliveryServices['cloneScenario']>>[0]): ApplyCommandsParams {
  if (request.source_run_id !== evidence.run.id || !evidenceMatches(project.id, [request.source_run_id], [evidence])) {
    throw new Error('创建方案必须使用同工程该Run的真实冻结输入。');
  }
  const id = request.new_scenario_id.trim(), title = request.title.trim();
  if (!id || !title) throw new Error('请填写新方案ID与标题。');
  if (project.scenario_refs.includes(id) || id === evidence.scenario.id) throw new Error('新方案ID已存在，不能覆盖已有方案。');
  const scenario = structuredClone(evidence.scenario);
  scenario.id = id;
  scenario.title = title;
  scenario.revision = 1;
  scenario.project_revision = project.revision;
  return { base_revision: project.revision, commands: [{ kind: 'put_object', data: { kind: 'scenario', data: scenario } }] };
}
