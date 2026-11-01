import { useEffect, useRef, useState } from 'react';
import type { ModuleCandidate } from '../../../../../packages/commands/generated/typescript/ModuleCandidate';
import type { ModuleTemplateCandidates } from '../../../../../packages/commands/generated/typescript/ModuleTemplateCandidates';
import type { ApiCategory } from '../../../../../packages/commands/generated/typescript/ApiCategory';
import type { LibrarySourceScope } from '../../../../../packages/commands/generated/typescript/LibrarySourceScope';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ProjectState } from '../../services/projectHost';
import { moduleKey, readModuleLibrary } from '../../services/projectHost';
import type { TemplatePreview } from '../../../../../packages/commands/generated/typescript/TemplatePreview';
import { TemplatePreparation } from './TemplatePreparation';

interface Props {
  project: ProjectState | null;
  busy: boolean;
  initialModule?: ModuleRef;
  initialCategory?: ApiCategory;
  onDirtyChange?(dirty: boolean): void;
  onClose: () => void;
  onAdopt: (base: ProjectState, ref: ModuleRef) => Promise<void>;
  onTemplate: (base: ProjectState, preview: TemplatePreview) => Promise<void>;
}
const categories: [ApiCategory, string][] = [['items', '物品'], ['devices', '设备'], ['resources', '资源'], ['tasks', '任务'], ['operations', '工序'], ['transport', '运输'], ['templates', '模板'], ['professional', '专业'], ['media', '表现']];
const sourceLabel: Record<LibrarySourceScope, string> = { bundled: '软件内置库', project_packages: '当前工程包', user_granted_libraries: '已授权目录' };
const purpose: Record<ApiCategory, string> = {
  items: '定义原料或成品的规格和计量单位。', devices: '添加设备定义，再在工厂画布中布置设备。',
  resources: '配置人员、运输工具等生产所需资源。', tasks: '定义生产任务的输入和完成条件。',
  operations: '定义设备加工的工序和步骤。', transport: '配置物料运输所需的规则。',
  templates: '预览厂房、设备和示例生产方案，确认后一起添加到工程。',
  professional: '为纸包装结构等专业计算提供定义。', media: '为设备和物品配置外观及加工表现。',
};

export function ModuleLibrary({ project, busy, initialModule, initialCategory, onDirtyChange, onClose, onAdopt, onTemplate }: Props) {
  const [base] = useState(project);
  const [category, setCategory] = useState<ApiCategory | ''>(initialCategory ?? '');
  const [sources, setSources] = useState<LibrarySourceScope[]>(['bundled', 'project_packages']);
  const [result, setResult] = useState<ModuleTemplateCandidates | null>(null);
  const [selected, setSelected] = useState<ModuleCandidate | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [templateDirty, setTemplateDirty] = useState(false), [leaving, setLeaving] = useState(false);
  useEffect(() => { onDirtyChange?.(templateDirty); }, [templateDirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  const epoch = useRef(0);
  useEffect(() => {
    const request = ++epoch.current;
    setLoading(true); setError(''); setSelected(null);
    void readModuleLibrary(base?.id ?? null, category || null, sources).then(value => {
      if (request === epoch.current) {
        setResult(value);
        if (initialModule) setSelected(value.candidates.find(candidate => moduleKey(candidate.module_ref) === moduleKey(initialModule)) ?? null);
      }
    }).catch(failure => {
      if (request === epoch.current) { setResult(null); setError(failure instanceof Error ? failure.message : String(failure)); }
    }).finally(() => { if (request === epoch.current) setLoading(false); });
    return () => { epoch.current++; };
  }, [base?.id, category, sources]);
  const adopted = selected && base?.module_refs.some(ref => moduleKey(ref) === moduleKey(selected.module_ref));
  async function adopt() {
    if (!base || !selected) return;
    setError('');
    try { await onAdopt(base, selected.module_ref); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }
  return <section className="module-library" aria-label="模块与模板库">
    <header><div><h2>模块与模板库</h2><p>{base ? `为“${base.title}”选择模板或模块。创建模板工厂后，下一步配置订单并运行方案。` : '先浏览模板；创建工程后即可添加。'}</p></div><button disabled={busy} onClick={() => templateDirty ? setLeaving(true) : onClose()}>返回画布</button></header>
    <div className="library-filters"><label>类别<select disabled={templateDirty} value={category} onChange={event => setCategory(event.target.value as ApiCategory | '')}><option value="">全部九类</option>{categories.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label><fieldset><legend>来源</legend>{(Object.keys(sourceLabel) as LibrarySourceScope[]).map(source => <label key={source}><input type="checkbox" disabled={templateDirty} checked={sources.includes(source)} onChange={event => setSources(previous => event.target.checked ? [...previous, source] : previous.filter(value => value !== source))} />{sourceLabel[source]}</label>)}</fieldset></div>
    {error && <p role="alert" className="error-message">{error}</p>}
    {result?.missing.map((issue, index) => <p className="issue-row" key={index}>{issue.message}</p>)}
    <div className="library-body"><div className="library-list" aria-label="模块与模板清单">{loading ? <p>正在读取模板与模块…</p> : result?.candidates.map(candidate => <button key={moduleKey(candidate.module_ref)} disabled={templateDirty && selected !== candidate} className={selected === candidate ? 'selected' : ''} onClick={() => setSelected(candidate)}><strong>{candidate.title}</strong><span>{categories.find(([key]) => key === candidate.category)?.[1]} · {candidate.applicable ? '可预览' : '有待处理事项'}</span></button>)}{!loading && result?.candidates.length === 0 && <p className="muted">没有找到模板或模块。可更换类别，或勾选“软件内置库”。</p>}</div>
      <div className="library-details">{selected ? <>
        <h3>{selected.title}</h3><p>{purpose[selected.category]}</p>
        <p>{adopted ? '这个版本已添加到当前工程。' : '尚未添加到当前工程。'}{!selected.applicable && '请先处理下方缺项，再继续。'}</p>
        {selected.missing.map((issue,index) => <p className="issue-row" key={index}>需要处理：{issue.message}</p>)}
        <details className="library-source-details"><summary>来源与版本</summary><dl><dt>模块编号</dt><dd>{selected.module_ref.id}</dd><dt>版本</dt><dd>{selected.module_ref.version}</dd><dt>目录检查</dt><dd>{selected.applicable ? '当前目录未发现阻断；添加时仍检查工程条件。' : '存在待处理事项。'}</dd></dl>{selected.sources.map(source => <p key={source.id}>{source.title} · {source.locator ?? '未提供目录位置'}</p>)}</details>
        {selected.category !== 'templates' && <p>添加定义后，设备实例、物料库存和生产方案仍需分别配置。</p>}
        <button className={selected.category === 'templates' ? '' : 'primary'} disabled={!base || busy || loading || adopted === true || !selected.applicable} onClick={() => void adopt()}>{selected.category === 'templates' ? '仅添加模板定义' : '添加定义到工程'}</button>
        {!base && <p className="muted">先创建工程，再添加模板或模块。</p>}
        {base && selected.category === 'templates' && <TemplatePreparation onDirtyChange={setTemplateDirty} key={moduleKey(selected.module_ref)} project={base} moduleRef={selected.module_ref} busy={busy} onApply={onTemplate} />}
      </> : <p className="muted">从左侧选择模板或模块，查看用途和需要填写的内容。</p>}</div></div>
    {leaving && <section role="dialog" aria-label="放弃模板准备输入"><p>模板配置尚未采纳。请返回保留输入，或明确放弃后离开。</p><button onClick={() => setLeaving(false)}>返回保留模板输入</button><button onClick={onClose}>明确放弃模板输入</button></section>}
  </section>;
}
