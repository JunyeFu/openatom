// Form metadata is read from the actual generated public schema. It is never
// a second domain model or a substitute for Host validation.
export interface Schema {
  $ref?: string; type?: string | string[]; title?: string; description?: string;
  const?: unknown; enum?: unknown[]; oneOf?: Schema[]; anyOf?: Schema[];
  properties?: Record<string, Schema>; patternProperties?: Record<string, Schema>; required?: string[]; items?: Schema;
  additionalProperties?: Schema | boolean; minimum?: number; maximum?: number;
  minLength?: number; maxLength?: number; minItems?: number; maxItems?: number; pattern?: string;
}
export interface SchemaCatalog { $defs: Record<string, Schema> }
export async function loadBusinessSchema(): Promise<SchemaCatalog> {
  const response = await fetch(new URL('../../../../../packages/commands/generated/schemas/get_object.response.schema.json', import.meta.url));
  if (!response.ok) throw new Error(`业务字段规范读取失败：${response.status}`);
  return response.json();
}
export const definitionName = (schema: Schema) => schema.$ref?.split('/').at(-1);
export function resolve(schema: Schema, catalog: SchemaCatalog): Schema {
  if (!schema.$ref) return schema;
  const found = catalog.$defs[definitionName(schema)!];
  if (!found) throw new Error(`当前公共规范缺少 ${definitionName(schema)}`);
  return { ...resolve(found, catalog), title: found.title ?? definitionName(schema) };
}
export function fieldSchema(name: string, catalog: SchemaCatalog): Schema {
  if (!catalog.$defs[name]) throw new Error(`当前规范未提供 ${name}`);
  return { $ref: `#/$defs/${name}` };
}
export function branches(schema: Schema, catalog: SchemaCatalog) {
  const value = resolve(schema, catalog);
  return (value.oneOf ?? value.anyOf ?? (Array.isArray(value.type) ? value.type.map(type => ({ ...value, type })) : [value])).map(s => resolve(s, catalog));
}
export function discriminator(schema: Schema) {
  return Object.entries(schema.properties ?? {}).find(([, v]) => v.const !== undefined);
}
export function branchFor(schema: Schema, value: unknown, catalog: SchemaCatalog): Schema | null {
  const candidates = branches(schema, catalog);
  return candidates.find(s => {
    const key = discriminator(s);
    if (key) return value !== null && typeof value === 'object' && (value as Record<string, unknown>)[key[0]] === key[1].const;
    if (s.type === 'null') return value === null;
    return s.type === 'array' ? Array.isArray(value) : s.type === 'object' ? value !== null && typeof value === 'object' && !Array.isArray(value) : s.type === 'integer' || s.type === 'number' ? typeof value === 'number' : typeof value === s.type;
  }) ?? null;
}
export function emptyValue(schema: Schema, catalog: SchemaCatalog): unknown {
  const s = resolve(schema, catalog);
  if (s.const !== undefined) return s.const;
  if (s.anyOf || s.oneOf || Array.isArray(s.type)) return branches(s, catalog).some(b => b.type === 'null') ? null : {};
  if (s.type === 'array') return [];
  if (s.type === 'object') return Object.fromEntries((s.required ?? []).map(key => [key, emptyValue(s.properties?.[key] ?? {}, catalog)]));
  if (s.type === 'boolean') return false;
  if (s.type === 'string') return '';
  return null; // Unknown numeric input stays blank, never an invented zero.
}
export interface FormIssue { path: string; message: string }
const escaped = (s: string) => s.replaceAll('~', '~0').replaceAll('/', '~1');
export function validateForm(schema: Schema, value: unknown, catalog: SchemaCatalog, path = ''): FormIssue[] {
  const s = resolve(schema, catalog), error = (message: string) => [{ path, message }];
  if (s.anyOf || s.oneOf || Array.isArray(s.type)) {
    const options = branches(s, catalog), matches = options.map(option => validateForm(option, value, catalog, path));
    const valid = matches.filter(issues => !issues.length);
    if (s.oneOf ? valid.length === 1 : valid.length > 0) return [];
    const selected = branchFor(s, value, catalog);
    return selected ? validateForm(selected, value, catalog, path) : error('请选择明确的类型并填写其字段');
  }
  if (s.const !== undefined && value !== s.const) return error('类型标记与当前字段不符');
  if (s.enum && !s.enum.includes(value)) return error('请从当前声明的选项中选择');
  if (s.type === 'null') return value === null ? [] : error('该选项须保持未知');
  if (s.type === 'number' || s.type === 'integer') {
    if (typeof value !== 'number' || !Number.isFinite(value) || s.type === 'integer' && !Number.isSafeInteger(value)) return error(s.type === 'integer' ? '须填写安全范围内的整数' : '须填写有限数值');
    if (s.minimum !== undefined && value < s.minimum || s.maximum !== undefined && value > s.maximum) return error(`须在 ${s.minimum ?? '不限'} 至 ${s.maximum ?? '不限'} 之间`);
  }
  if (s.type === 'string') {
    if (typeof value !== 'string') return error('须填写文本');
    if (s.minLength !== undefined && value.length < s.minLength || s.maxLength !== undefined && value.length > s.maxLength || s.pattern && !new RegExp(s.pattern).test(value)) return error('文本长度或格式不符合当前公开字段');
  }
  if (s.type === 'array') {
    if (!Array.isArray(value)) return error('须使用多行输入');
    if (s.minItems !== undefined && value.length < s.minItems || s.maxItems !== undefined && value.length > s.maxItems) return error('行数不符合当前字段要求');
    return value.flatMap((item, i) => validateForm(s.items ?? {}, item, catalog, `${path}/${i}`));
  }
  if (s.type === 'object') {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return error('须填写该字段组');
    const object = value as Record<string, unknown>, issues: FormIssue[] = [];
    for (const key of s.required ?? []) if (object[key] === undefined) issues.push({ path: `${path}/${escaped(key)}`, message: '必填项尚未填写' });
    for (const [key, item] of Object.entries(object)) {
      if (item === undefined) continue;
      const child = s.properties?.[key] ?? Object.entries(s.patternProperties ?? {}).find(([pattern]) => new RegExp(pattern).test(key))?.[1];
      if (child) issues.push(...validateForm(child, item, catalog, `${path}/${escaped(key)}`));
      else if (s.additionalProperties === false) issues.push({ path: `${path}/${escaped(key)}`, message: '当前公共版本不支持此字段' });
      else if (typeof s.additionalProperties === 'object') issues.push(...validateForm(s.additionalProperties, item, catalog, `${path}/${escaped(key)}`));
    }
    return issues;
  }
  return [];
}
export function parseDecimal(text: string) {
  if (!/^\d+(\.\d+)?$/.test(text)) throw new Error('填写非负十进制数，不使用科学计数法');
  const [whole, fraction = ''] = text.split('.'), coefficient = BigInt(whole + fraction);
  if (coefficient > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('精度超出公开十进制系数范围');
  return { coefficient: Number(coefficient), decimal_places: fraction.length };
}
export function decimalText(value: unknown): string {
  if (!value || typeof value !== 'object' || !('coefficient' in value) || !('decimal_places' in value) || !Number.isSafeInteger(value.coefficient) || !Number.isSafeInteger(value.decimal_places)) return '';
  const places = value.decimal_places as number, digits = String(value.coefficient).padStart(places + 1, '0');
  return places ? `${digits.slice(0, -places)}.${digits.slice(-places)}` : digits;
}
