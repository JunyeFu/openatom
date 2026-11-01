import { useEffect, useId, useState } from 'react';
import { fieldLabel, optionLabel } from './labels';
import { initialValue, resolve, tagOf, variants } from './model';
import type { FormSchema } from './formSchema';
import type { Quantity } from '../../../../../packages/commands/generated/typescript/Quantity';

interface Props {
  schema: FormSchema;
  value: unknown;
  onChange(value: unknown): void;
  name: string;
  disabled?: boolean;
  required?: boolean;
  omit?: string[];
}
export function SchemaField({ schema: input, value, onChange, name, disabled = false, required = false, omit = [] }: Props) {
  const id = useId(); const schema = resolve(input); const choices = variants(schema);
  const title = fieldLabel(name);
  if (input.$ref === '#/$defs/Quantity') return <QuantityField value={value as Quantity} onChange={onChange} title={title} disabled={disabled} />;
  if (choices.length) {
    const nullable = choices.find(item => item.type === 'null');
    const nonnull = choices.filter(item => item.type !== 'null');
    if (nullable) return <div className="authoring-optional"><div className="authoring-field-heading"><span>{title}</span><button type="button" disabled={disabled} onClick={() => onChange(value == null ? initialValue(nonnull[0]) : null)}>{value == null ? '填写' : '留空'}</button></div>{value != null && <SchemaField schema={nonnull.length === 1 ? nonnull[0] : { oneOf: nonnull }} value={value} onChange={onChange} name={name} disabled={disabled} />}{value == null && <small>未提供</small>}</div>;
    const tags = choices.map(tagOf); const current = value && typeof value === 'object' ? value as Record<string, unknown> : {};
    const selected = choices.findIndex((_, index) => tags[index] && current[tags[index]!.field] === tags[index]!.value);
    const index = selected < 0 ? 0 : selected;
    if (tags.every(Boolean)) return <fieldset className="authoring-group"><legend>{title}</legend><label htmlFor={id}>类型<select id={id} disabled={disabled} value={index} onChange={event => onChange(initialValue(choices[Number(event.target.value)]))}>{tags.map((tag, position) => <option key={position} value={position}>{optionLabel(tag!.value)}</option>)}</select></label><SchemaField schema={choices[index]} value={value} onChange={onChange} name={name} disabled={disabled} omit={[tags[index]!.field]} /></fieldset>;
    return <SchemaField schema={choices[0]} value={value} onChange={onChange} name={name} disabled={disabled} />;
  }
  const types = Array.isArray(schema.type) ? schema.type : [schema.type];
  if (types.includes('null')) return <div className="authoring-optional"><div className="authoring-field-heading"><span>{title}</span><button type="button" disabled={disabled} onClick={() => onChange(value == null ? initialValue({ ...schema, type: types.find(type => type !== 'null') }) : null)}>{value == null ? '填写' : '留空'}</button></div>{value != null && <SchemaField schema={{ ...schema, type: types.find(type => type !== 'null') }} value={value} onChange={onChange} name={name} disabled={disabled} />}</div>;
  if (schema.type === 'object') {
    const record = (value ?? {}) as Record<string, unknown>;
    if (!schema.properties) return <MapField schema={schema} value={record} onChange={onChange} name={name} disabled={disabled} />;
    return <fieldset className="authoring-group"><legend>{title}</legend><div className="authoring-field-grid">{Object.entries(schema.properties).filter(([key]) => !omit.includes(key)).map(([key, child]) => <SchemaField key={key} schema={child} value={record[key]} onChange={next => onChange({ ...record, [key]: next })} name={key} disabled={disabled} required={schema.required?.includes(key)} />)}</div></fieldset>;
  }
  if (schema.type === 'array') {
    const rows = (value ?? []) as unknown[];
    if (schema.prefixItems) return <fieldset className="authoring-group"><legend>{title}</legend><div className="authoring-field-grid">{schema.prefixItems.map((child, index) => <SchemaField key={index} schema={child} value={rows[index]} onChange={next => onChange(rows.map((row, position) => position === index ? next : row))} name={['红色','绿色','蓝色','透明度'][index] ?? `${index + 1}`} disabled={disabled} required />)}</div></fieldset>;
    return <fieldset className="authoring-group"><legend>{title} · {rows.length} 项</legend>{rows.map((row, index) => <div className="authoring-array-row" key={index}><div className="authoring-field-heading"><strong>{title} {index + 1}</strong><button type="button" disabled={disabled || rows.length <= (schema.minItems ?? 0)} onClick={() => onChange(rows.filter((_, position) => position !== index))}>移除</button></div><SchemaField schema={schema.items!} value={row} onChange={next => onChange(rows.map((item, position) => position === index ? next : item))} name={name} disabled={disabled} /></div>)}<button type="button" disabled={disabled || (schema.maxItems !== undefined && rows.length >= schema.maxItems)} onClick={() => onChange([...rows, initialValue(schema.items!)])}>添加{title}</button></fieldset>;
  }
  const options = schema.enum ?? (schema.const !== undefined ? [schema.const] : null);
  if (options) return <label className="authoring-field" htmlFor={id}><span>{title}{required && <span aria-label="必填"> *</span>}</span><select id={id} disabled={disabled || options.length === 1} value={JSON.stringify(value)} onChange={event => onChange(JSON.parse(event.target.value))}>{!options.some(option => option === value) && <option value={JSON.stringify(value)}>请选择</option>}{options.map((option, index) => <option key={index} value={JSON.stringify(option)}>{input.$ref === '#/$defs/QualityState' && option === 'released' ? '已放行' : optionLabel(option)}</option>)}</select></label>;
  if (schema.type === 'boolean') return <label className="authoring-check"><input type="checkbox" checked={value === true} disabled={disabled} onChange={event => onChange(event.target.checked)} />{title}</label>;
  const numeric = schema.type === 'number' || schema.type === 'integer';
  return <label className="authoring-field" htmlFor={id}><span>{title}{required && <span aria-label="必填"> *</span>}</span><input id={id} type={numeric ? 'number' : 'text'} value={value == null ? '' : String(value)} disabled={disabled} required={required && (schema.minLength !== undefined || numeric)} minLength={schema.minLength} min={schema.minimum} max={schema.maximum} step={schema.type === 'integer' ? 1 : 'any'} onChange={event => onChange(numeric ? (event.target.value === '' ? null : event.target.valueAsNumber) : event.target.value)} /></label>;
}

function quantityText(value: Quantity): string { const digits = String(value.value.coefficient).padStart(value.value.decimal_places + 1, '0'); return value.value.decimal_places ? `${digits.slice(0, -value.value.decimal_places)}.${digits.slice(-value.value.decimal_places)}` : digits; }
function QuantityField({ value, onChange, title, disabled }: { value: Quantity; onChange(value: unknown): void; title: string; disabled: boolean }) {
  const [text, setText] = useState(() => quantityText(value));
  useEffect(() => { if (Number.isFinite(value.value.coefficient)) setText(quantityText(value)); }, [value.value.coefficient, value.value.decimal_places]);
  return <fieldset className="authoring-group"><legend>{title}</legend><div className="authoring-field-grid"><label>数量（十进制）<input inputMode="decimal" required value={text} disabled={disabled} onChange={event => { const next = event.target.value; setText(next); const pieces = /^(\d+)(?:\.(\d+))?$/.exec(next); const coefficient = pieces ? Number(pieces[1] + (pieces[2] ?? '')) : NaN; const places = pieces?.[2]?.length ?? 0; const valid = Number.isSafeInteger(coefficient) && places <= 9; event.target.setCustomValidity(valid ? '' : '请填写非负十进制数量，最多9位小数，并保持安全整数精度。'); onChange({ ...value, value: { coefficient: valid ? coefficient : NaN, decimal_places: places } }); }} /></label><label>单位代码<input required value={value.unit} disabled={disabled} onChange={event => onChange({ ...value, unit: event.target.value })} /></label></div><small>单位必须与采用目标明确一致，不做隐式换算。</small></fieldset>;
}

function MapField({ schema, value, onChange, name, disabled }: { schema: FormSchema; value: Record<string, unknown>; onChange(value: unknown): void; name: string; disabled: boolean }) {
  const [key, setKey] = useState('');
  const child = (key: string): FormSchema => Object.entries(schema.patternProperties ?? {}).find(([pattern]) => new RegExp(pattern).test(key))?.[1] ?? (typeof schema.additionalProperties === 'object' ? schema.additionalProperties : { type: 'string' });
  return <fieldset className="authoring-group"><legend>{fieldLabel(name)}</legend>{Object.entries(value).map(([entry, item]) => <div className="authoring-map-row" key={entry}><SchemaField schema={child(entry)} value={item} name={entry} onChange={next => onChange({ ...value, [entry]: next })} disabled={disabled} /><button type="button" disabled={disabled} onClick={() => { const next = { ...value }; delete next[entry]; onChange(next); }}>移除</button></div>)}<div className="authoring-add-row"><input aria-label={`${fieldLabel(name)}的新名称`} value={key} disabled={disabled} onChange={event => setKey(event.target.value)} /><button type="button" disabled={disabled || !key.trim() || Object.hasOwn(value, key.trim())} onClick={() => { onChange({ ...value, [key.trim()]: initialValue(child(key.trim())) }); setKey(''); }}>添加</button></div>{Object.hasOwn(value, key.trim()) && <small>此名称已存在，请编辑现有项。</small>}</fieldset>;
}
