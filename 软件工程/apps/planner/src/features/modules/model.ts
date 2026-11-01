import { formSchemas, type FormSchema } from './formSchema';
import type { ModuleDefinition } from '../../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { ModulePayload } from '../../../../../packages/commands/generated/typescript/ModulePayload';
import type { ApiRequest } from '../../../../../packages/commands/generated/typescript/ApiRequest';
import type { ApiScope } from '../../../../../packages/commands/generated/typescript/ApiScope';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';
import type { ProjectContext } from '../../../../../packages/commands/generated/typescript/ProjectContext';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import type { HistoryAvailability } from '../../../../../packages/commands/generated/typescript/HistoryAvailability';
import type { AuthoringDraft, AuthoringIO } from './types';

export type Category = ModulePayload['category'];
export const categories: { id: Category; title: string; heading: string; boundary: string }[] = [
  { id: 'items', title: '物品', heading: '规格、单位与属性', boundary: '定义物品规格；数量和当前位置属于工程输入与实例。' },
  { id: 'devices', title: '固定设备', heading: '设备、端口与行为', boundary: '只选择一处行为来源；外观与动作不生成产量。' },
  { id: 'resources', title: '资源', heading: '角色、容量与日历', boundary: '容量属于定义；当前占用由实际运行维护。' },
  { id: 'tasks', title: '任务', heading: '前置、步骤与声明效果', boundary: '准备任务改变就绪状态；制造产出交给工艺规则。' },
  { id: 'operations', title: '工艺规则', heading: '配比、阶段与资源', boundary: '明确工时、单位与来源；未提供的工时保持缺项。' },
  { id: 'transport', title: '物流与供应', heading: '批次、装卸与接受', boundary: '运输定义不带在途货物；采用时绑定实际起讫库位。' },
  { id: 'templates', title: '布局与工厂模板', heading: '组合文件与适用条件', boundary: '引用相对工程文件；种子供料和订单另行确认。' },
  { id: 'professional', title: '专业计算', heading: '输入输出与计算范围', boundary: '声明的模式不等于实际可用提供者；认可与绑定分别完成。' },
  { id: 'media', title: '表现素材', heading: '部件、锚点与命名动作', boundary: '画面只消费状态；不写库存、占用或实际结果。' },
];
const payloadTypes: Record<Category, string> = { items: 'ItemDefinition', devices: 'DeviceDefinition', resources: 'ResourceDefinition', tasks: 'TaskDefinition', operations: 'OperationDefinition', transport: 'TransportDefinition', templates: 'TemplateDefinition', professional: 'ProfessionalDefinition', media: 'Presentation' };
export const payloadSchema = (category: Category) => formSchemas[payloadTypes[category]];
export const refKey = (ref: ModuleRef) => JSON.stringify([ref.id, ref.version]);
export const copy = <T,>(value: T): T => structuredClone(value);
export function assertPackagingDefinition(definition: ModuleDefinition): void {
  if (definition.payload.category !== 'professional' || definition.payload.data.output_artifact_type !== 'paper_packaging') {
    throw new Error(`定义 ${definition.id} @ ${definition.version} 尚无对应的实际执行消费者；可继续保存、检查和贡献声明。纸包装计算只消费纸包装输入，不执行此提供者。`);
  }
}
export function definitionSourceText(draft: AuthoringDraft | null, source: ModuleDefinition | null, selected: ModuleRef | null): string {
  if (source) return `${source.id} @ ${source.version}`;
  if (draft) return '已读取定义草稿；库中来源尚未读取';
  if (selected) return `已选择 ${selected.id} @ ${selected.version}；精确库内容尚未读取`;
  return '新建定义（尚未保存到 Host）';
}
export function resolve(schema: FormSchema): FormSchema {
  return schema.$ref ? formSchemas[schema.$ref.slice('#/$defs/'.length)] : schema;
}
export function variants(schema: FormSchema): FormSchema[] { const node = resolve(schema); return (node.oneOf ?? node.anyOf ?? []).map(resolve); }
export function tagOf(schema: FormSchema): { field: string; value: string } | null {
  const node = resolve(schema);
  for (const [field, property] of Object.entries(node.properties ?? {})) {
    const data = resolve(property); const value = data.const ?? (data.enum?.length === 1 ? data.enum[0] : undefined);
    if (typeof value === 'string') return { field, value };
  }
  return null;
}
export function initialValue(input: FormSchema): unknown {
  const schema = resolve(input); const options = variants(schema);
  if (options.length) {
    const nullable = options.find(option => option.type === 'null');
    return initialValue(nullable ?? options[0]);
  }
  if (schema.const !== undefined) return schema.const;
  if (schema.enum) return schema.enum.includes('unknown') ? 'unknown' : schema.enum[0];
  const type = Array.isArray(schema.type) ? (schema.type.includes('null') ? 'null' : schema.type[0]) : schema.type;
  if (type === 'null') return null;
  if (type === 'object') return Object.fromEntries(Object.entries(schema.properties ?? {}).map(([name, value]) => [name, initialValue(value)]));
  if (type === 'array') return schema.prefixItems ? schema.prefixItems.map(initialValue) : [];
  if (type === 'boolean') return false;
  if (type === 'number' || type === 'integer') return schema.minimum ?? 0;
  return '';
}
export function newDefinition(category: Category): ModuleDefinition {
  const data = initialValue(payloadSchema(category));
  const result: ModuleDefinition = { id: '', version: '0.1.0', title: '', definition_state: 'design', contribution: { author: null, license: null, assets: [] }, parameters: [], attribute_schema: [], payload: { category, data } as ModulePayload };
  if (category === 'media') result.payload = { category, data: newPresentation() };
  return result;
}
export function newPresentation(): Presentation {
  const value = initialValue(formSchemas.Presentation) as Presentation;
  return { ...value, kind: 'native_scene', source: { kind: 'native_scene', data: 'local.scene' }, unit: 'millimeter', coordinate_frame: 'right_handed_z_up', scale: { x: 1, y: 1, z: 1 }, production_effect: 'none' };
}
export function presentationOf(definition: ModuleDefinition): Presentation | null {
  switch (definition.payload.category) {
    case 'media': return definition.payload.data;
    case 'devices': case 'resources': case 'transport': return definition.payload.data.presentation ?? null;
    case 'items': return definition.payload.data.presentation_or_media_ref ?? null;
    default: return null;
  }
}

export interface Difference { path: string; before: unknown; after: unknown }
/** Human review of supplied versions, never a Host commit or an audit ledger. */
export function differences(before: unknown, after: unknown, path = ''): Difference[] {
  if (Object.is(before, after)) return [];
  if (Array.isArray(before) && Array.isArray(after)) {
    const identified = [...before, ...after].every(item => item && typeof item === 'object' && typeof item.id === 'string');
    const left = Object.fromEntries(before.map((item, index) => [identified ? item.id : String(index), item]));
    const right = Object.fromEntries(after.map((item, index) => [identified ? item.id : String(index), item]));
    return [...new Set([...Object.keys(left), ...Object.keys(right)])].flatMap(key => differences(left[key], right[key], `${path}/${key}`));
  }
  if (before && after && typeof before === 'object' && typeof after === 'object' && !Array.isArray(before) && !Array.isArray(after)) {
    const left = before as Record<string, unknown>, right = after as Record<string, unknown>;
    return [...new Set([...Object.keys(left), ...Object.keys(right)])].flatMap(key => differences(left[key], right[key], `${path}/${key}`));
  }
  return JSON.stringify(before) === JSON.stringify(after) ? [] : [{ path, before, after }];
}
export function brief(value: unknown): string {
  if (value === null || value === undefined) return '未提供';
  if (Array.isArray(value)) return `${value.length} 项`;
  if (typeof value === 'object') return Object.entries(value).slice(0, 4).map(([key, item]) => `${key}: ${brief(item)}`).join('；');
  return String(value);
}

export function definitionIssues(definition: ModuleDefinition): Issue[] {
  const issues: Issue[] = [];
  const add = (field: string, message: string) => issues.push({ code: 'authoring.input', object_ref: null, field_path: field, message, blocks: [] });
  if (!definition.id.trim()) add('/id', '请填写稳定模块标识。');
  if (!definition.version.trim()) add('/version', '请填写明确版本。');
  if (!definition.title.trim()) add('/title', '请填写中文名称。');
  if (definition.payload.category === 'devices') {
    const behavior = definition.payload.data.behavior;
    if (behavior.inline && behavior.operation_ref) add('/payload/data/behavior', '内联规则与独立规则引用只能保留一处。');
    if (behavior.selected === 'inline' && !behavior.inline) add('/payload/data/behavior/inline', '尚未填写选中的内联工艺规则。');
    if (behavior.selected === 'operation_ref' && !behavior.operation_ref) add('/payload/data/behavior/operation_ref', '尚未选择独立工艺规则的精确版本。');
  }
  for (const issue of shapeIssues(formSchemas.ModuleDefinition, definition)) add(issue.path, issue.message);
  return issues;
}
/** Form-input shape checks only; business references/capabilities stay Host-owned. */
export function shapeIssues(input: FormSchema, value: unknown, path = ''): { path: string; message: string }[] {
  const schema = resolve(input), alternatives = variants(schema);
  if (alternatives.length) {
    const tagged = alternatives.find(option => { const tag = tagOf(option); return tag && value && typeof value === 'object' && (value as Record<string, unknown>)[tag.field] === tag.value; });
    if (tagged) return shapeIssues(tagged, value, path);
    const results = alternatives.map(option => shapeIssues(option, value, path));
    return results.find(result => result.length === 0) ?? results.reduce((left, right) => left.length <= right.length ? left : right);
  }
  const error = (message: string) => [{ path: path || '/', message }];
  if (schema.const !== undefined && value !== schema.const) return error('请选择有效类型。');
  if (schema.enum && !schema.enum.includes(value as string)) return error('请选择声明中的有效选项。');
  const types = Array.isArray(schema.type) ? schema.type : [schema.type];
  if (value === null) return types.includes('null') || schema.enum?.includes(null) ? [] : error('此字段不能留空。');
  if (types.length === 1 && types[0] === 'null') return error('此分支只允许留空。');
  if (types.includes('object')) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return error('需要填写结构化字段。');
    const object = value as Record<string, unknown>;
    return [...(schema.required ?? []).filter(key => object[key] === undefined).map(key => ({ path: `${path}/${key}`, message: '必填字段尚未提供。' })), ...Object.entries(object).flatMap(([key, child]) => { const field = schema.properties?.[key] ?? (typeof schema.additionalProperties === 'object' ? schema.additionalProperties : null); return field ? shapeIssues(field, child, `${path}/${key}`) : []; })];
  }
  if (types.includes('array')) {
    if (!Array.isArray(value)) return error('需要填写有序条目。');
    if (value.length < (schema.minItems ?? 0) || value.length > (schema.maxItems ?? Infinity)) return error(`条目数量应在 ${schema.minItems ?? 0} 至 ${schema.maxItems ?? '不限'} 之间。`);
    return value.flatMap((child, index) => { const field = schema.prefixItems?.[index] ?? schema.items; return field ? shapeIssues(field, child, `${path}/${index}`) : []; });
  }
  if (types.includes('number') || types.includes('integer')) {
    if (typeof value !== 'number' || !Number.isFinite(value) || (types.includes('integer') && !Number.isInteger(value))) return error('请填写有限的有效数值。');
    if (value < (schema.minimum ?? -Infinity) || value > (schema.maximum ?? Infinity)) return error('数值超出声明范围。');
  } else if (types.includes('string')) {
    if (typeof value !== 'string' || value.length < (schema.minLength ?? 0)) return error('请填写有效文字或标识。');
  } else if (types.includes('boolean') && typeof value !== 'boolean') return error('请选择是或否。');
  return [];
}
let nextRequest = 0;
export function authoringRequest<O extends ApiRequest['operation']>(projectId: string | null, scope: ApiScope, operation: O, params: Extract<ApiRequest, { operation: O }>['params']): Extract<ApiRequest, { operation: O }> {
  return { api_version: '0.1', request_id: `authoring-${++nextRequest}`, project_id: projectId, scope, operation, params } as Extract<ApiRequest, { operation: O }>;
}
export async function readDraft(io: AuthoringIO, projectId: string, draftId: string, history: HistoryAvailability | null = null, historyRevision: number | null = null): Promise<AuthoringDraft> {
  const scope: ApiScope = { kind: 'definition_draft', data: draftId };
  const context = await io.dispatch<ProjectContext>(authoringRequest(projectId, scope, 'get_context', { selection: [] }));
  const object = await io.dispatch<DomainObject>(authoringRequest(projectId, scope, 'get_object', { object_id: { kind: 'definition_draft', id: draftId }, source: { kind: 'draft' }, node_index: null }));
  if (object.kind !== 'definition' || !context.definition) throw new Error('Host 未返回所选定义草稿。');
  const current = await io.dispatch<ProjectContext>(authoringRequest(projectId, scope, 'get_context', { selection: [] }));
  if (current.definition?.draft_id !== draftId || current.definition.revision !== context.definition.revision) throw new Error('读取期间共同定义已变化，请明确刷新后继续。');
  return { summary: current.definition, definition: object.data, history: current.definition.history ?? (current.definition.revision === historyRevision ? history : null) };
}
