import { useEffect, useRef, useState } from 'react';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { TemplatePreview } from '../../../../../packages/commands/generated/typescript/TemplatePreview';
import type { TemplatePreparationChoices } from '../../../../../packages/commands/generated/typescript/TemplatePreparationChoices';
import type { TemplateSeedSelection } from '../../../../../packages/commands/generated/typescript/TemplateSeedSelection';
import type { QualityState } from '../../../../../packages/commands/generated/typescript/QualityState';
import type { SiteEnvelope } from '../../../../../packages/commands/generated/typescript/SiteEnvelope';
import type { ProjectState } from '../../services/projectHost';
import { HostFailure, previewTemplate } from '../../services/projectHost';
import { HostFailureDetails } from './HostFailureDetails';
import { decimalText } from '../../scene/runtime';
import { factoryUnitLabel } from '../../scene/factoryLabels';

interface Props { project: ProjectState; moduleRef: ModuleRef; busy: boolean; onApply: (base: ProjectState, preview: TemplatePreview) => Promise<void>; onDirtyChange?(dirty: boolean): void }
const kindLabel: Record<string, string> = { hall: '厂房', line: '线组', device: '设备', location: '库位', item_type: '物品类型', resource: '资源', calendar: '日历', product: '产品', bom: '物料清单', route: '工艺路线', connection: '连接' };
const qualities: [QualityState, string][] = [['unchecked', '未检'], ['pending', '待判'], ['released', '合格放行'], ['rejected', '不合格']];
class TemplateInputError extends Error {
  constructor(message: string, readonly field: string) { super(message); }
}

export function TemplatePreparation({ project, moduleRef, busy, onApply, onDirtyChange }: Props) {
  const [source, setSource] = useState<TemplatePreview | null>(null);
  const [namespace, setNamespace] = useState('');
  const [resources, setResources] = useState<Record<string, string>>({});
  const [reuseItems, setReuseItems] = useState<Record<string, string>>({});
  const [includeProduct, setIncludeProduct] = useState(false);
  const [fillSite, setFillSite] = useState(false);
  const [siteX, setSiteX] = useState(''), [siteY, setSiteY] = useState(''), [siteZ, setSiteZ] = useState('');
  const [seedEnabled, setSeedEnabled] = useState(false), [scenarioId, setScenarioId] = useState('');
  const [supply, setSupply] = useState(false), [demand, setDemand] = useState(false), [inspection, setInspection] = useState(false);
  const [initialQuality, setInitialQuality] = useState<Record<string, QualityState>>({});
  const [qualityForAll, setQualityForAll] = useState<QualityState | ''>('');
  const [preparing, setPreparing] = useState(false), [error, setError] = useState<unknown>(null);
  const [prepared, setPrepared] = useState<{ fingerprint: string; value: TemplatePreview } | null>(null);
  const epoch = useRef(0), formRef = useRef<HTMLFormElement>(null), errorRef = useRef<HTMLParagraphElement>(null);
  function locateError(failure: unknown) {
    const pointer = failure instanceof HostFailure ? failure.detail.field_errors[0]?.pointer ?? '' : '';
    const field = failure instanceof TemplateInputError ? failure.field
      : pointer.includes('resources') ? 'resources'
      : pointer.includes('site_envelope') ? 'site-x'
      : pointer.includes('seed') || pointer.includes('scenario') ? 'scenario'
      : '';
    requestAnimationFrame(() => {
      const target = field ? formRef.current?.querySelector<HTMLElement>(`[data-template-field="${field}"]`) : null;
      const focus = target ?? errorRef.current;
      const details = focus?.closest('details');
      if (details) details.open = true;
      focus?.scrollIntoView({ block: 'center' });
      focus?.focus();
    });
  }
  const fingerprint = JSON.stringify({ namespace, resources, reuseItems, includeProduct, fillSite, siteX, siteY, siteZ, seedEnabled, scenarioId, supply, demand, inspection, initialQuality });
  const dirty = !!(namespace || Object.values(resources).some(Boolean) || Object.values(reuseItems).some(Boolean) || includeProduct || fillSite || siteX || siteY || siteZ || seedEnabled || scenarioId || supply || demand || inspection || qualityForAll || Object.keys(initialQuality).length);
  useEffect(() => { onDirtyChange?.(dirty); }, [dirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  useEffect(() => {
    const request = ++epoch.current;
    setPreparing(true);
    void previewTemplate(project, moduleRef).then(value => { if (request === epoch.current) setSource(value); })
      .catch(failure => { if (request === epoch.current) { setError(failure); locateError(failure); } })
      .finally(() => { if (request === epoch.current) setPreparing(false); });
    return () => { epoch.current++; };
  }, [project.id, project.revision, moduleRef.id, moduleRef.version]);

  async function prepare(event: React.FormEvent) {
    event.preventDefault(); if (!source) return;
    setError(null);
    const request = ++epoch.current;
    try {
      if (!namespace.trim()) throw new TemplateInputError('请填写工厂前缀，用于区分本次新建设备。', 'namespace');
      const positive = (text: string, label: string, field: string) => { const value = Number(text); if (!text.trim() || !Number.isFinite(value) || value <= 0) throw new TemplateInputError(`${label}须为正数（m）。`, field); return value; };
      const site: SiteEnvelope | null = fillSite ? { length_m: positive(siteX, '厂房宽度', 'site-x'), width_m: positive(siteY, '厂房进深', 'site-y'), height_m: siteZ.trim() ? positive(siteZ, '厂房净高', 'site-z') : null, usable_polygon_m: [], clearance_m: null, regions: [] } : null;
      const choices: TemplatePreparationChoices = { source_project: source.source_project, namespace: namespace.trim(), resources: {}, reuse_items: {}, include_example_product: includeProduct, site_envelope: site };
      for (const [id, raw] of Object.entries(resources)) {
        if (!raw) continue;
        const choice: { kind: 'create' } | { kind: 'reuse'; id: string } = JSON.parse(raw);
        choices.resources[id] = choice.kind === 'create' ? null : choice.id;
      }
      for (const [id, target] of Object.entries(reuseItems)) if (target) choices.reuse_items[id] = target;
      if (seedEnabled && !scenarioId.trim()) throw new TemplateInputError('创建示例输入时，请填写方案名称。', 'scenario');
      const seed: TemplateSeedSelection | null = seedEnabled ? { scenario_id: scenarioId.trim(), supply, demand, initial_quality: initialQuality, adopt_declared_inspection_results: inspection } : null;
      setPreparing(true);
      const value = await previewTemplate(project, moduleRef, choices, seed);
      if (request === epoch.current) setPrepared({ value, fingerprint });
    } catch (failure) { if (request === epoch.current) { setError(failure); locateError(failure); } }
    finally { if (request === epoch.current) setPreparing(false); }
  }
  async function apply() {
    if (!prepared || prepared.fingerprint !== fingerprint) return;
    setError(null);
    try { await onApply(project, prepared.value); }
    catch (failure) { setError(failure); locateError(failure); }
  }
  const stock = source?.subset.adoption?.deferred_seed?.initial_stock ?? [];
  const stockLocations = [...new Set(stock.map(row => row.location_id))];
  const qualityInputs = source?.subset.seed ? [
    ...source.subset.seed.initial_conditions.items.map(item => ({ id: item.id, title: `初始物品 ${item.id} · ${item.holder.id}` })),
    ...source.subset.seed.arrivals.flatMap(arrival => arrival.items.map(item => ({ id: item.id, title: `到货物品 ${item.id} · ${arrival.id} · ${arrival.at_ms / 1000}s` }))),
  ] : stockLocations.map(id => ({ id, title: id }));
  function useDeclaredDemo() {
    if (!source?.subset.seed) return;
    setResources(Object.fromEntries(source.resources.map(resource => [resource.id, JSON.stringify({ kind: 'create' })])));
    setReuseItems({}); setIncludeProduct(true); setFillSite(false);
    setSeedEnabled(true); setSupply(true); setDemand(true);
    setInspection(!!source.subset.adoption?.inspection_presets.length);
    const items = [...source.subset.seed.initial_conditions.items, ...source.subset.seed.arrivals.flatMap(arrival => arrival.items)];
    setInitialQuality(Object.fromEntries(items.map(item => [item.id, item.quality]))); setQualityForAll('');
  }
  const preparedSeed = prepared?.value.command?.explicit_new_scenario_seed;
  return <form ref={formRef} className="template-preparation" onSubmit={prepare} onInvalidCapture={event => {
    const target = event.target;
    if (target instanceof HTMLElement) {
      const details = target.closest('details');
      if (details) details.open = true;
      requestAnimationFrame(() => { target.scrollIntoView({ block: 'center' }); target.focus(); });
    }
    setError(new Error('请填写或修正当前必填项，再预览创建对象。'));
  }}>
    <h3>配置模板工厂</h3><p>先填工厂前缀；若启用场地尺寸，须填正数宽度与进深；若创建演示方案，须填方案名称，并明确选择供料的初始质量。预览核对后再创建。</p>
    {preparing && !source && <p>读取模板…</p>}
    {source && <><p className="form-hint">模板包含 {source.subset.halls.length} 个厂房、{source.subset.devices.length} 台设备；配置将添加到当前工程。</p>
      <fieldset disabled={busy || preparing}>
        {source.subset.seed && <><button type="button" onClick={useDeclaredDemo}>使用模板演示配置</button><p className="form-hint">填入示例资源、原料质量、订单与检验设置。填写下方工厂前缀和方案名称后，先预览再创建。</p></>}
        <label>工厂前缀（必填）<input data-template-field="namespace" placeholder="例如 演示工厂" value={namespace} onChange={event => setNamespace(event.target.value)} required /></label>
        <details open={source.resources.length <= 12}><summary>资源配置 · 已选择 {Object.values(resources).filter(Boolean).length}/{source.resources.length} 项</summary><p id="template-resource-note" className="form-hint">逐项选择新建或复用；未选择的资源会在预览中列为缺项，不会自动创建。</p><button type="button" onClick={() => setResources(Object.fromEntries(source.resources.map(resource => [resource.id, JSON.stringify({ kind: 'create' })])))}>全部新建为模板资源</button>{source.resources.map(resource => <label key={resource.id}>{resource.title} · {decimalText(resource.capacity.value)} {factoryUnitLabel(resource.capacity.unit)}<select data-template-field="resources" aria-describedby="template-resource-note" value={resources[resource.id] ?? ''} onChange={event => setResources(previous => ({ ...previous, [resource.id]: event.target.value }))}><option value="">尚未选择（保留缺项）</option><option value={JSON.stringify({ kind: 'create' })}>新建该声明示例资源</option>{project.resources.map(target => <option key={target.id} value={JSON.stringify({ kind: 'reuse', id: target.id })}>复用 {target.title} · {target.id}</option>)}</select></label>)}</details>
        <details open={source.subset.item_types.length <= 12}><summary>物品类型 · {source.subset.item_types.length} 种，默认新建</summary>{source.subset.item_types.map(item => <label key={item.id}>{item.title} · {factoryUnitLabel(item.unit)}<select value={reuseItems[item.id] ?? ''} onChange={event => setReuseItems(previous => ({ ...previous, [item.id]: event.target.value }))}><option value="">新建模板声明的物品类型</option>{project.item_types.map(target => <option key={target.id} value={target.id}>复用 {target.title} · {factoryUnitLabel(target.unit)}</option>)}</select></label>)}</details>
        <label className="check-label"><input type="checkbox" checked={includeProduct} onChange={event => setIncludeProduct(event.target.checked)} />导入模板产品、物料清单与工艺示例</label>
        <label className="check-label"><input type="checkbox" checked={fillSite} onChange={event => setFillSite(event.target.checked)} />填写实际场地尺寸</label>{fillSite && <><div className="field-grid"><label>厂房宽度 X（m，必填）<input data-template-field="site-x" type="number" step="any" value={siteX} onChange={event => setSiteX(event.target.value)} required /></label><label>厂房进深 Y（m，必填）<input data-template-field="site-y" type="number" step="any" value={siteY} onChange={event => setSiteY(event.target.value)} required /></label><label>厂房净高（m，可选）<input data-template-field="site-z" type="number" step="any" value={siteZ} onChange={event => setSiteZ(event.target.value)} placeholder="未填写" /></label></div><p className="form-hint">场地尺寸用于本次模板实例。净距与静态区域请在工程草稿检查中明确填写；模板尺寸采纳不代表已通过布局检查。</p></>}
        <label className="check-label"><input type="checkbox" checked={seedEnabled} onChange={event => setSeedEnabled(event.target.checked)} />创建演示生产方案</label>{seedEnabled && <><label>方案名称（必填）<input data-template-field="scenario" placeholder="例如 纸包装演示" value={scenarioId} onChange={event => setScenarioId(event.target.value)} required /></label><label className="check-label"><input type="checkbox" checked={supply} onChange={event => setSupply(event.target.checked)} />采用声明的示例供料</label><label className="check-label"><input type="checkbox" checked={demand} onChange={event => setDemand(event.target.checked)} />采用声明的示例订单</label>{supply && qualityInputs.length > 1 && <section aria-label="本次声明初始质量批量填写"><label>批量初始质量（明确选择后应用）<select value={qualityForAll} onChange={event => setQualityForAll(event.target.value as QualityState | '')}><option value="">明确选择</option>{qualities.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label><button type="button" disabled={!qualityForAll} onClick={() => { if (!qualityForAll) return; const quality = qualityForAll; setInitialQuality(Object.fromEntries(qualityInputs.map(input => [input.id, quality]))); }}>应用到本次 {qualityInputs.length} 项质量输入</button><p className="form-hint">仅填写下方列出的初始质量输入；应用后仍可逐项修改，不自动采用检验结果。</p></section>}{supply && <details open={qualityInputs.length <= 12}><summary>原料初始质量（必填） · 已填写 {Object.keys(initialQuality).length}/{qualityInputs.length} 项</summary>{qualityInputs.map(input => <label key={input.id}>初始质量 · {input.title}<select value={initialQuality[input.id] ?? ''} onChange={event => setInitialQuality(previous => { const next = { ...previous }; if (event.target.value) next[input.id] = event.target.value as QualityState; else delete next[input.id]; return next; })} required><option value="">明确选择</option>{qualities.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>)}</details>}<label className="check-label"><input type="checkbox" checked={inspection} onChange={event => setInspection(event.target.checked)} />采用模板声明的检验预设（示例输入）</label>{inspection && !supply && <p className="issue-row">检验预设需要相应种子物品；请明确选择示例供料，不会自动创建物品。</p>}<p className="form-hint">示例输入不是生产结果或人工认可；供料与订单未选时不会自动补入。</p></>}
      </fieldset>
      <button type="submit" disabled={busy || preparing}>预览创建对象</button>
      {source.issues.map((issue, index) => <p className="issue-row" key={index}>{issue.message}</p>)}
    </>}
    {error != null && <><p ref={errorRef} tabIndex={-1} role="alert" className="error-message">{error instanceof Error ? error.message : String(error)}</p>{error instanceof HostFailure && <HostFailureDetails detail={error.detail} />}</>}
    {prepared && <section className="template-preview"><h4>创建预览</h4>{prepared.fingerprint !== fingerprint && <p className="issue-row">配置已改变，请重新预览后采纳。</p>}{prepared.value.issues.map((issue, index) => <p className="issue-row" key={index}>{issue.message}</p>)}{prepared.value.command && <><p>将创建 {Object.entries(prepared.value.command.proposed_objects.reduce<Record<string, number>>((counts, object) => { counts[object.kind] = (counts[object.kind] ?? 0) + 1; return counts; }, {})).map(([kind, count]) => `${kindLabel[kind] ?? kind} ${count}`).join(' · ')}</p><details><summary>查看创建对象明细</summary><ul>{prepared.value.command.proposed_objects.map((object, index) => <li key={index}>{kindLabel[object.kind] ?? object.kind} · {'title' in object.data ? object.data.title : object.kind === 'connection' ? object.data.data.id : object.data.id}</li>)}</ul></details>{prepared.value.command.project_setup_policies != null && <details><summary>设备换型设置 · {prepared.value.command.project_setup_policies.length} 台</summary><section aria-label="实际采用后的换型规则"><h4>同次提交的换型规则 · {prepared.value.command.project_setup_policies.length} 台设备</h4><p className="form-hint">以下为 Host 实际重放后的完整提议，包含保留的工程规则；取消不会写入。</p>{prepared.value.command.project_setup_policies.map(policy => <article key={policy.device_id}><strong>{policy.device_id}</strong><p>{project.setup_policies?.some(current => current.device_id === policy.device_id) ? '工程已有设备规则' : '本次模板设备规则'} · 初始家族：{policy.initial_family ?? '未声明'}</p>{policy.changeovers.map((rule, index) => <p key={index}>{rule.from_family ?? '冷启动'} → {rule.to_family} · {rule.duration.value} {rule.duration.unit} · {rule.preparation_mode} · {rule.duration.basis} · 来源 {rule.duration.source_ref ?? '未提供'} · 资源 {rule.resources.map(resource => resource.role).join('、') || '无'}</p>)}{policy.first_article && <p>首件：{policy.first_article.family} · 路线 {policy.first_article.route_ref.id} r{policy.first_article.route_ref.revision} · 检验 {policy.first_article.inspection_module.id} @ {policy.first_article.inspection_module.version} · 门禁 {policy.first_article.gate_id}</p>}</article>)}</section></details>}<p>{prepared.value.command.explicit_new_scenario_seed ? `同时创建已选择的示例方案：${prepared.value.command.explicit_new_scenario_seed.id}` : '不创建方案、初始库存或订单输入。'}</p>{preparedSeed && <section aria-label="本次方案声明输入摘要"><h4>生产方案</h4><p>初始物品 {preparedSeed.initial_conditions.items.length} 项 · 到货 {preparedSeed.arrivals.length} 批 · 供应与运输 {preparedSeed.supplies.length} 条 · 订单 {preparedSeed.orders.length} 条 · 生产任务 {preparedSeed.production_tasks.length} 条 · 检验预设 {preparedSeed.quality_presets.length} 条</p><p className="form-hint">运行后可查看实际产量与检验结果。</p></section>}<button type="button" className="primary" disabled={busy || preparing || prepared.fingerprint !== fingerprint} onClick={() => void apply()}>创建模板工厂</button></>}</section>}
  </form>;
}
