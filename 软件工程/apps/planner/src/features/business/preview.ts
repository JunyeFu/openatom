import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { BusinessSnapshot } from './types';
import { collectionKinds, equal } from './draft';
import { labels } from './SchemaForm';
import { sections } from './sections';
import { decimalText } from './schema';
import { factoryItemTypeTitle, factoryUnitLabel } from '../../scene/factoryLabels';

export interface PreviewChange { field: string; before: string; after: string }
export interface PreviewGroup { title: string; changes: PreviewChange[] }

const objectKinds: Record<string, string> = Object.fromEntries(
  sections.filter(section => section.owner === 'project' && section.path[0] in collectionKinds)
    .map(section => [collectionKinds[section.path[0] as keyof typeof collectionKinds], section.title])
);

const fieldLabel = (key: string) => key === 'orders' ? '订单' : labels[key] ?? sections.find(section => section.path.at(-1) === key)?.title ?? key;

type DecimalValue = { coefficient: number; decimal_places: number };
type QuantityValue = { value: DecimalValue; unit: string };
function isDecimal(value: unknown): value is DecimalValue {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  return Object.keys(record).length === 2 && typeof record.coefficient === 'number'
    && Number.isSafeInteger(record.coefficient)
    && typeof record.decimal_places === 'number' && Number.isSafeInteger(record.decimal_places)
    && record.decimal_places >= 0 && record.decimal_places <= 9;
}
function isQuantity(value: unknown): value is QuantityValue {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  return Object.keys(record).length === 2 && isDecimal(record.value) && typeof record.unit === 'string';
}
function atomicKind(value: unknown): 'quantity' | 'decimal' | null {
  return isQuantity(value) ? 'quantity' : isDecimal(value) ? 'decimal' : null;
}
function previewDecimalText(value: DecimalValue): string {
  return value.coefficient < 0
    ? `-${decimalText({coefficient: Math.abs(value.coefficient), decimal_places: value.decimal_places})}`
    : decimalText(value);
}

function format(value: unknown): string {
  if (value === undefined || value === null || value === '') return '未填写';
  if (typeof value === 'boolean') return value ? '是' : '否';
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  if (Array.isArray(value)) return value.length ? value.map(format).join('、') : '无';
  if (isQuantity(value)) return `${previewDecimalText(value.value)} ${factoryUnitLabel(value.unit)}`;
  if (isDecimal(value)) return previewDecimalText(value);
  const record = value as Record<string, unknown>;
  if (typeof record.title === 'string') return record.title;
  if (typeof record.label === 'string') return record.label;
  if (typeof record.id === 'string') return record.id;
  return JSON.stringify(value);
}

function name(value: unknown, fallback: string): string {
  if (value && typeof value === 'object') {
    const record = value as Record<string, unknown>;
    if (typeof record.title === 'string' && record.title.trim()) return record.title;
  }
  return fallback;
}

function orderName(snapshot: BusinessSnapshot, order: unknown, index: number): string {
  if (!order || typeof order !== 'object') return `订单 ${index + 1}`;
  const row = order as { id?: string; product_ref?: { id: string; revision: string } };
  const ref = row.product_ref;
  const product = ref && snapshot.project.products.find(p => p.id === ref.id && p.revision === ref.revision);
  const productName = product ? factoryItemTypeTitle(snapshot.project, product.item_type_ref) : null;
  return `订单 ${index + 1}${productName ? ` · ${productName}` : ''}${row.id ? `（${row.id}）` : ''}`;
}

function describeArrayRow(path: string[], value: unknown, index: number, snapshot: BusinessSnapshot): string {
  if (path.at(-1) === 'orders') return orderName(snapshot, value, index);
  const section = sections.find(candidate => candidate.path.at(-1) === path.at(-1));
  return `${section?.title ?? labels[path.at(-1) ?? ''] ?? path.at(-1) ?? '记录'} ${index + 1}${name(value, '') ? ` · ${name(value, '')}` : ''}`;
}

function changedFields(before: unknown, after: unknown, snapshot: BusinessSnapshot): PreviewChange[] {
  const changes: PreviewChange[] = [];
  function visit(oldValue: unknown, newValue: unknown, path: string[]) {
    if (equal(oldValue, newValue)) return;
    const oldKind = atomicKind(oldValue), newKind = atomicKind(newValue);
    if (oldKind && (oldKind === newKind || newValue == null) || newKind && oldValue == null) {
      changes.push({field:path.map(fieldLabel).join(' / ') || '内容', before:format(oldValue), after:format(newValue)});
      return;
    }
    if (Array.isArray(oldValue) || Array.isArray(newValue)) {
      const oldRows = Array.isArray(oldValue) ? oldValue : [];
      const newRows = Array.isArray(newValue) ? newValue : [];
      const identified = [...oldRows, ...newRows].every(value => value && typeof value === 'object' && typeof value.id === 'string');
      if (identified) {
        const oldIds = new Set(oldRows.map(value => value.id as string));
        const seen = new Set<string>();
        for (const [index, row] of newRows.entries()) {
          const id = row.id as string;
          const previous = oldRows.find(value => value.id === id);
          seen.add(id);
          visit(previous, row, [...path.slice(0, -1), describeArrayRow(path, row, index, snapshot)]);
        }
        for (const [index, row] of oldRows.entries()) if (!seen.has(row.id as string))
          visit(row, undefined, [...path.slice(0, -1), describeArrayRow(path, row, index, snapshot)]);
        if (oldRows.length === newRows.length && newRows.every(row => oldIds.has(row.id as string)) &&
            oldRows.some((row, index) => row.id !== newRows[index]?.id))
          changes.push({field:[...path.map(fieldLabel), '顺序'].join(' / '), before:'原顺序', after:'新顺序'});
        return;
      }
      for (let index = 0; index < Math.max(oldRows.length, newRows.length); index++)
        visit(oldRows[index], newRows[index], [...path.slice(0, -1), describeArrayRow(path, newRows[index] ?? oldRows[index], index, snapshot)]);
      return;
    }
    const oldObject = oldValue && typeof oldValue === 'object' ? oldValue as Record<string, unknown> : null;
    const newObject = newValue && typeof newValue === 'object' ? newValue as Record<string, unknown> : null;
    if (oldObject || newObject) {
      const keys = new Set([...Object.keys(oldObject ?? {}), ...Object.keys(newObject ?? {})]);
      for (const key of keys) {
        visit(oldObject?.[key], newObject?.[key], [...path, key]);
      }
      return;
    }
    changes.push({field:path.map(fieldLabel).join(' / ') || '内容', before:format(oldValue), after:format(newValue)});
  }
  visit(before, after, []);
  return changes;
}

export function previewCommands(base: BusinessSnapshot, draft: BusinessSnapshot, commands: readonly EditCommand[]): PreviewGroup[] {
  return commands.map(command => {
    if (command.kind === 'put_object' || command.kind === 'remove_object') {
      const { kind } = command.data;
      const id = command.kind === 'put_object' ? ('id' in command.data.data ? String(command.data.data.id) : '') : command.data.id;
      const oldValue = kind === 'scenario' ? base.scenarios.find(s => s.id === id) :
        Object.entries(collectionKinds).flatMap(([collection, candidate]) => candidate === kind ?
          [(base.project[collection as keyof typeof collectionKinds] as {id:string}[]).find(row => row.id === id)] : [])[0];
      const newValue = command.kind === 'put_object' ? command.data.data : undefined;
      const readable = name(newValue ?? oldValue, id);
      const exactName = readable === id || !id ? readable : `${readable}（${id}）`;
      const title = kind === 'scenario' ? `方案 · ${exactName}` : `${objectKinds[kind] ?? kind} · ${exactName}`;
      return {title: `${command.kind === 'remove_object' ? '移除 ' : oldValue ? '修改 ' : '新增 '}${title}`,
        changes: changedFields(oldValue, newValue, draft)};
    }
    if (command.kind === 'set_project_settings') {
      const before = Object.fromEntries(Object.keys(command.data).map(key => [key, base.project[key as keyof typeof base.project]]));
      return {title: `工程设置 · ${base.project.title}`, changes: changedFields(before, command.data, draft)};
    }
    if (command.kind === 'set_declared_parameter') {
      const device = base.project.devices.find(row => row.id === command.data.target.id);
      return {title: `设备参数 · ${name(device, command.data.target.id)} · ${command.data.parameter_id}`,
        changes: [{field:'声明值', before:format(device?.configuration[command.data.parameter_id]), after:format(command.data.value)}]};
    }
    return {title:command.kind, changes:[]};
  });
}
