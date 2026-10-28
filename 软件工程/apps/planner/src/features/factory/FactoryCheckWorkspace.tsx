import { useEffect, useState } from 'react';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { FactoryDraftCheckResult } from '../../../../../packages/commands/generated/typescript/FactoryDraftCheckResult';
import type { AcceptanceStage } from '../../../../../packages/commands/generated/typescript/AcceptanceStage';
import type { FactoryGenerationGoal } from '../../../../../packages/commands/generated/typescript/FactoryGenerationGoal';
import { dispatch, type ProjectState } from '../../services/projectHost';
import { SiteConstraintsEditor } from './SiteConstraintsEditor';
import { readSiteConstraints, siteConstraintsDraft, type SiteConstraintsDraft } from './siteConstraints';

export function FactoryCheckWorkspace({ project, supported, selection, busy, onClose, onLocate, onDirtyChange }: {
  project: ProjectState; supported: boolean; selection: ObjectRef[]; busy: boolean;
  onClose(): void; onLocate(object: ObjectRef): void; onDirtyChange(dirty: boolean): void;
}) {
  const [ownerId] = useState(project.id);
  const saved = project.generation_context?.goal;
  const [industry, setIndustry] = useState(saved?.industry ?? ''), [family, setFamily] = useState(saved?.product_family ?? '');
  const [product, setProduct] = useState(saved ? `${saved.acceptance_scope.product_ref.id}@${saved.acceptance_scope.product_ref.revision}` : '');
  const [destination, setDestination] = useState(saved?.acceptance_scope.destination ?? '');
  const [stage, setStage] = useState<AcceptanceStage>(saved?.acceptance_scope.stage ?? 'factory_released');
  const [start, setStart] = useState(saved ? String(saved.observation_window.start_ms) : ''), [end, setEnd] = useState(saved ? String(saved.observation_window.end_ms) : '');
  const [site, setSite] = useState(saved ? 'saved' : '');
  const [siteDrafts, setSiteDrafts] = useState<Record<string, SiteConstraintsDraft>>((): Record<string, SiteConstraintsDraft> => saved ? { saved: siteConstraintsDraft(saved.site_envelope) } : {});
  const constraints = siteDrafts[site] ?? siteConstraintsDraft();
  const [calendar, setCalendar] = useState(saved?.production_calendar_ref ?? '');
  const [references, setReferences] = useState(true), [business, setBusiness] = useState(true), [layout, setLayout] = useState(true), [onlySelected, setOnlySelected] = useState(false);
  const [result, setResult] = useState<FactoryDraftCheckResult | null>(null), [error, setError] = useState(''), [checking, setChecking] = useState(false), [leaving, setLeaving] = useState(false);
  const value = JSON.stringify({ industry, family, product, destination, stage, start, end, site, siteDrafts, calendar, references, business, layout, onlySelected, selection: onlySelected ? selection : null });
  const [resultInput, setResultInput] = useState('');
  const resultCurrent = !!result && resultInput === value && project.id === ownerId && result.project_revision === project.revision;
  const [initial] = useState(value);
  const dirty = value !== initial;
  useEffect(() => { onDirtyChange(dirty); }, [dirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange(false), [onDirtyChange]);
  async function check(event: React.FormEvent) {
    event.preventDefault(); setChecking(true); setError(''); setResult(null); setResultInput('');
    try {
      if (project.id !== ownerId) throw new Error('检查输入所属工程已切换，请返回原工程核对；本地输入保留。');
      const actualProduct = project.products.find(value => `${value.id}@${value.revision}` === product);
      const item = actualProduct && project.item_types.find(value => value.id === actualProduct.item_type_ref.id && value.revision === actualProduct.item_type_ref.revision);
      const hall = project.halls.find(value => value.id === site);
      if (!actualProduct || !item) throw new Error('请明确选择已有产品及其实际物品修订。');
      if (!industry.trim() || !family.trim()) throw new Error('请填写行业与产品族。');
      if (!start.trim() || !end.trim() || !Number.isSafeInteger(Number(start)) || !Number.isSafeInteger(Number(end)) || Number(start) < 0 || Number(end) <= Number(start)) throw new Error('请填写明确的观察窗口，结束毫秒须大于开始毫秒。');
      if (!project.locations.some(value => value.id === destination)) throw new Error('请明确选择验收目的库位。');
      if (!(site === 'saved' && saved) && !hall) throw new Error('请明确选择已有场地。');
      const sameProduct = saved?.acceptance_scope.product_ref.id === actualProduct.id && saved.acceptance_scope.product_ref.revision === actualProduct.revision;
      const goal: FactoryGenerationGoal = { industry: industry.trim(), product_family: family.trim(), product_specification_ref: sameProduct ? saved?.product_specification_ref ?? null : null,
        accepted_quantity_per_day: sameProduct ? saved?.accepted_quantity_per_day ?? null : null,
        acceptance_scope: { product_ref: { id: actualProduct.id, revision: actualProduct.revision }, unit: item.unit, destination, stage },
        ...(saved?.additional_acceptance_scopes?.length ? { additional_acceptance_scopes: saved.additional_acceptance_scopes } : {}),
        production_calendar_ref: calendar || null, observation_window: { start_ms: Number(start), end_ms: Number(end) },
        site_envelope: { ...(site === 'saved' && saved ? saved.site_envelope : { length_m: hall!.width_m, width_m: hall!.depth_m, height_m: hall!.height_m, usable_polygon_m: [{ x: 0, y: 0 }, { x: hall!.width_m, y: 0 }, { x: hall!.width_m, y: hall!.depth_m }, { x: 0, y: hall!.depth_m }] }), ...readSiteConstraints(constraints) },
        existing_equipment: saved?.existing_equipment ?? project.devices.map(value => ({ kind: 'device', id: value.id })), constraints: saved?.constraints ?? [] };
      const actual = await dispatch<FactoryDraftCheckResult>({ api_version: '0.1', request_id: 'factory-check-' + crypto.randomUUID(), project_id: project.id, scope: { kind: 'project' }, operation: 'check_factory_draft',
        params: { revision: project.revision, generation_goal: goal, check_scope: { objects: onlySelected ? selection : [{ kind: 'project', id: project.id }], check_references: references, check_business: business, check_static_layout: layout } } });
      setResult(actual); setResultInput(value);
    } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setChecking(false); }
  }
  return <form className="object-editor" aria-label="工程草稿检查" onSubmit={check}>
    <header><h2>工程草稿检查</h2><button type="button" disabled={busy || checking} onClick={() => dirty ? setLeaving(true) : onClose()}>返回画布</button></header>
    <p>检查基础 r{project.revision}。静态结果不替代生产运行；未提供的净距、工时与日产目标保持未知。</p>
    {!supported && <p role="alert">当前 Host 未提供工程草稿检查能力。</p>}
    <fieldset disabled={busy || checking}>
      <div className="field-grid"><label>行业<input value={industry} onChange={event => setIndustry(event.target.value)} required /></label><label>产品族<input value={family} onChange={event => setFamily(event.target.value)} required /></label></div>
      <label>产品修订<select value={product} onChange={event => setProduct(event.target.value)} required><option value="">明确选择产品</option>{project.products.map(value => <option key={`${value.id}@${value.revision}`} value={`${value.id}@${value.revision}`}>{value.id}@{value.revision}</option>)}</select></label>
      {!project.products.length && <p>工程尚未声明产品修订，请在业务输入中补齐后检查。</p>}
      <div className="field-grid"><label>验收目的库位<select value={destination} onChange={event => setDestination(event.target.value)} required><option value="">明确选择库位</option>{project.locations.map(value => <option key={value.id} value={value.id}>{value.title}</option>)}</select></label><label>验收阶段<select value={stage} onChange={event => setStage(event.target.value as AcceptanceStage)}><option value="factory_released">工厂放行</option><option value="received">目的地接收</option><option value="unloaded_and_released">卸货并放行</option></select></label></div>
      {!!saved?.additional_acceptance_scopes?.length && <section aria-label="已保存的附加接受范围"><h3>附加接受范围 · 已保存，只读</h3>{saved.additional_acceptance_scopes.map((scope, index) => <p key={index}><strong>{scope.product_ref.id}@{scope.product_ref.revision}</strong> · {scope.unit} · {scope.destination} · {({ factory_released: '工厂放行', received: '目的地接收', unloaded_and_released: '卸货并放行' })[scope.stage]}</p>)}</section>}
      <div className="field-grid"><label>观察开始 / ms<input type="number" min="0" step="1" value={start} onChange={event => setStart(event.target.value)} required /></label><label>观察结束 / ms<input type="number" min="1" step="1" value={end} onChange={event => setEnd(event.target.value)} required /></label></div>
      <label>场地参照<select value={site} onChange={event => setSite(event.target.value)} required><option value="">明确选择场地</option>{saved && <option value="saved">原生成目标的场地边界</option>}{project.halls.map(value => <option key={value.id} value={value.id}>{value.title} · {value.width_m} × {value.depth_m} m</option>)}</select></label>
      {site && <SiteConstraintsEditor value={constraints} onChange={next => setSiteDrafts(previous => ({ ...previous, [site]: next }))} />}
      <label>生产日历<select value={calendar} onChange={event => setCalendar(event.target.value)}><option value="">未提供</option>{project.calendars.map(value => <option key={value.id} value={value.id}>{value.id}</option>)}</select></label>
      <p>日产目标：{saved?.accepted_quantity_per_day && product === `${saved.acceptance_scope.product_ref.id}@${saved.acceptance_scope.product_ref.revision}` ? '沿用此产品的已保存目标' : '未提供'}。本表单的场地声明仅用于本次只读检查，不改写工程或原目标。</p>
      <label className="check-label"><input type="checkbox" checked={references} onChange={event => setReferences(event.target.checked)} />引用与单位</label><label className="check-label"><input type="checkbox" checked={business} onChange={event => setBusiness(event.target.checked)} />物料与过程连接</label><label className="check-label"><input type="checkbox" checked={layout} onChange={event => setLayout(event.target.checked)} />场地、占地、高度和净距</label><label className="check-label"><input type="checkbox" checked={onlySelected} onChange={event => setOnlySelected(event.target.checked)} />只检查当前所选 {selection.length} 项</label>
      <button type="submit" disabled={!supported || checking}>{checking ? '正在读取检查结果' : '执行只读检查'}</button>
    </fieldset>
    {error && <p role="alert">{error} 检查输入保留。</p>}
    {result && !resultCurrent && <p role="status">检查输入或工程已变化；上次结果基于 r{result.project_revision}，请重新检查当前条件。</p>}
    {result && resultCurrent && <section aria-label="实际草稿检查结果"><h3>实际检查 · r{result.project_revision}{result.project_revision !== project.revision ? ' · 工程已变化，请重新检查' : ''}</h3>{result.checks.map((value, index) => <article key={index} data-check-outcome={value.outcome}><strong>{({ pass: '通过', fail: '失败', undetermined: '无法确定' })[value.outcome]} · {({ references_and_units: '引用与单位', material_process_connectivity: '物料与过程连接', site_bounds: '场地边界', footprint_overlap: '占地重叠', height: '设备高度', clearance_and_static_lanes: '净距与静态通道' })[value.kind]}</strong><p>{value.message}</p>{value.objects.map(object => <button type="button" key={`${object.kind}:${object.id}`} onClick={() => onLocate(object)}>{object.kind} / {object.id}</button>)}</article>)}{result.issues.map((value, index) => <p key={index}>{value.code} · {value.message}{value.object_ref && <button type="button" onClick={() => onLocate(value.object_ref!)}>定位 {value.object_ref.id}</button>}</p>)}</section>}
    {leaving && <div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-label="放弃检查输入"><h2>放弃检查输入</h2><p>检查目标输入尚未保存到工程。</p><footer><button type="button" autoFocus onClick={() => setLeaving(false)}>返回保留输入</button><button type="button" onClick={onClose}>明确放弃检查输入</button></footer></section></div>}
  </form>;
}
