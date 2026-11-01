import type { LayoutRegionKind } from '../../../../../packages/commands/generated/typescript/LayoutRegionKind';
import type { SiteConstraintsDraft, SiteRegionDraft } from './siteConstraints';

const kinds: [LayoutRegionKind, string][] = [['usable', '可用区'], ['keep_out', '禁入区'], ['static_lane', '静态通道']];

export function SiteConstraintsEditor({ value, onChange }: {
  value: SiteConstraintsDraft; onChange(value: SiteConstraintsDraft): void;
}) {
  function update(index: number, next: SiteRegionDraft) {
    onChange({ ...value, regions: value.regions.map((region, i) => i === index ? next : region) });
  }
  function add(kind: LayoutRegionKind) {
    onChange({ ...value, regions: [...value.regions, { id: `region-${crypto.randomUUID()}`, kind,
      clearance: '', points: Array.from({ length: 3 }, () => ({ x: '', y: '' })) }] });
  }
  return <section className="site-constraints" aria-label="净距与静态区域声明">
    <h3>净距与静态区域</h3>
    <label>设备间最小净距 / m<input type="number" min="0" step="any" value={value.clearance}
      onChange={event => onChange({ ...value, clearance: event.target.value })} placeholder="未提供" /></label>
    <p className="form-hint">坐标为所选场地的厂房 X/Y，单位米。顶点按边界顺序填写，末点自动连回首点。净距留空表示未提供，填写 0 才明确声明零净距。</p>
    {!value.regions.length && <p>尚未声明可用区、禁入区或静态通道；检查将保留对应缺项。</p>}
    {value.regions.map((region, index) => <fieldset key={index} aria-label={`静态区域 ${index + 1}`}>
      <legend>区域 {index + 1}</legend>
      <div className="field-grid"><label>区域 {index + 1} 标识<input value={region.id} required
        onChange={event => update(index, { ...region, id: event.target.value })} /></label>
        <label>区域 {index + 1} 用途<select value={region.kind} onChange={event => update(index, { ...region, kind: event.target.value as LayoutRegionKind })}>
          {kinds.map(([kind, title]) => <option key={kind} value={kind}>{title}</option>)}
        </select></label>
        <label>区域 {index + 1} 预留净距 / m<input type="number" min="0" step="any" value={region.clearance}
          onChange={event => update(index, { ...region, clearance: event.target.value })} placeholder="未提供" /></label></div>
      {region.points.map((point, pointIndex) => <div className="field-grid" key={pointIndex}>
        {(['x', 'y'] as const).map(axis => <label key={axis}>区域 {index + 1} 顶点 {pointIndex + 1} {axis.toUpperCase()} / m<input type="number" step="any" value={point[axis]} required
          onChange={event => update(index, { ...region, points: region.points.map((current, i) => i === pointIndex ? { ...current, [axis]: event.target.value } : current) })} /></label>)}
        <button type="button" disabled={region.points.length <= 3} onClick={() => update(index, { ...region, points: region.points.filter((_, i) => i !== pointIndex) })}>移除顶点 {pointIndex + 1}</button>
      </div>)}
      <button type="button" onClick={() => update(index, { ...region, points: [...region.points, { x: '', y: '' }] })}>区域 {index + 1} 添加顶点</button>
      <button type="button" onClick={() => onChange({ ...value, regions: value.regions.filter((_, i) => i !== index) })}>移除区域 {index + 1}</button>
    </fieldset>)}
    <div>{kinds.map(([kind, title]) => <button type="button" key={kind} onClick={() => add(kind)}>新增{title}</button>)}</div>
    <p className="form-hint">这些区域用于静态场地检查；动态交通仍由相应运行规则处理。没有声明的通道不视为已检查。</p>
  </section>;
}
