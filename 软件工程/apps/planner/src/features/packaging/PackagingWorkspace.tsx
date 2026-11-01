import { factoryItemTypeTitle } from '../../scene/factoryLabels';
import { useEffect, useRef, useState } from 'react';
import { SchemaField } from '../modules/SchemaField';
import { shapeIssues } from '../modules/model';
import { ProcessViewport } from './ProcessViewport';
import { ManufacturingPreview } from './ManufacturingPreview';
import { ManufacturingExportForm } from './ManufacturingExportForm';
import { packagingSchemas } from './schemas';
import { packagingExamples } from './examples';
import { acknowledgmentCommand, artifactAcknowledgments, adoptArtifact, assertPackagingContext, cancelPackagingTask, commitProfessionalDefinitionVersion, computeJob, exportEvidence, exportDeclaredManufacturingEvidence, manufacturingExportIdentity, packagingStations, readScopeContext, readTaskEvidence, recordAcknowledgment, sameInput, saveJob, saveProduct, scopeAvailable, selectStationJob } from './model';
import { HostFailure } from '../../services/projectHost';
import type { PackagingEvidence, PackagingWorkspaceProps } from './types';
import type { PackageStructureJob } from '../../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { ProductRevision } from '../../../../../packages/commands/generated/typescript/ProductRevision';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { ArtifactAcknowledgment } from '../../../../../packages/commands/generated/typescript/ArtifactAcknowledgment';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { ArtifactId } from '../../../../../packages/commands/generated/typescript/ArtifactId';
import { taskStateLabel } from '../../services/workbenchLabels';
import './packaging.css';

const outcome = { pass: '通过', fail: '不满足', undetermined: '未判定' };
const purposeNames: Record<string,string> = {forming:'成型过程',opening:'开合过程',linear_extraction:'直线取物',net_space:'内部净空间',manufacturing:'制造展开'};
const sections: [keyof PackageStructureJob, string][] = [
  ['subject_ref', '精确产品版本'], ['mechanism', '结构机制'], ['changes', '三类变化'], ['unit', '长度单位'], ['coordinate_frame', '坐标系'],
  ['panels', '板片 / 多边形 / 厚度 / 材料'], ['fold_bands', '折带 / 轴 / 角度区间'], ['connections', '固定连接 / 接触许可 / 激活时刻'],
  ['parts', '多件 / 构件关系 / 布置'], ['materials', '材料声明 / 厚度 / 认可来源'], ['conditions', '净空间 / 允许姿态 / 取物 / 局部接触'],
  ['variables', '结构变量 / 有限候选'], ['dimension_constraints', '尺寸与变量条件'], ['steps', '成型 / 开合 / 运动过程'], ['tolerance', '显式容差与依据'], ['budget', '显式计算预算与依据'],
];
const terminal = (state: TaskState | null) => !state || ['completed', 'failed', 'cancelled'].includes(state.task.state);
const message = (error: unknown) => error instanceof HostFailure ? `${error.message}\n${JSON.stringify(error.detail, null, 2)}` : error instanceof Error ? error.message : String(error);
type StoredArtifactListState =
  | { status: 'loading'; ids: ArtifactId[] }
  | { status: 'ready'; ids: ArtifactId[] }
  | { status: 'failed'; ids: ArtifactId[]; error: string };
/** One read, owned by the current mounted Scope/revision; never a Task observer. */
export async function loadStoredArtifactList(
  list: () => Promise<ArtifactId[]>,
  isCurrent: () => boolean,
  publish: (state: StoredArtifactListState) => void,
): Promise<void> {
  if (!isCurrent()) return;
  publish({ status: 'loading', ids: [] });
  try {
    const ids = await list();
    if (isCurrent()) publish({ status: 'ready', ids });
  } catch (failure) {
    if (isCurrent()) publish({ status: 'failed', ids: [], error: message(failure) });
  }
}
function TaskDetail({ state }: { state: TaskState | null }) {
  if (!state) return null;
  return <div className="packaging-task" role="status"><strong>计算任务 · {taskStateLabel[state.task.state]}</strong>{state.task.progress && <p>{state.task.progress.phase}：{state.task.progress.processed} / {state.task.progress.total ?? '总量未知'} · {state.task.progress.message}</p>}<details><summary>任务来源与版本</summary><p>任务编号：{state.task.id} · 输入版本：{state.task.input_revision}</p><pre>{JSON.stringify(state.task.scope,null,2)}</pre></details>{state.failure && <details><summary>任务错误详情</summary><pre>{JSON.stringify(state.failure, null, 2)}</pre></details>}{state.task.failure && <p>{state.task.failure.code}：{state.task.failure.message}</p>}</div>;
}

export function PackagingWorkspace(props: PackagingWorkspaceProps) {
  const { workspace, scope, io } = props;
  const project = workspace.project;
  const identity = JSON.stringify([workspace.context.host.session_id, project?.id, scope]);
  const current = useRef(identity); current.current = identity;
  const live = useRef(true);
  useEffect(() => { live.current = true; return () => { live.current = false; }; }, []);
  const [job, setJob] = useState<PackageStructureJob>(() => structuredClone(props.initialJob ?? packagingExamples[3].job));
  const [saved, setSaved] = useState<PackageStructureJob | null>(() => props.initialJob ? structuredClone(props.initialJob) : null);
  const initialDisplayedJob = useRef({ identity, job: structuredClone(props.initialJob ?? packagingExamples[3].job) });
  if (initialDisplayedJob.current.identity !== identity) initialDisplayedJob.current = { identity, job: structuredClone(props.initialJob ?? packagingExamples[3].job) };
  const [revision, setRevision] = useState(props.scopeRevision);
  const [product, setProduct] = useState<ProductRevision | null>(project?.products[0] ?? null);
  const [pending, setPending] = useState(0); const busy = pending > 0; const [error, setError] = useState('');
  const [scopedTasks, setScopedTasks] = useState(workspace.context.tasks);
  const [task, setTask] = useState<TaskState | null>(null); const [exportTask, setExportTask] = useState<TaskState | null>(null);
  const [evidence, setEvidence] = useState<PackagingEvidence | null>(props.initialEvidence ?? null); const [sampleIndex, setSampleIndex] = useState(0);
  const [ackScopes, setAckScopes] = useState<string[]>([]); const [statement, setStatement] = useState('');
  const [ack, setAck] = useState<ArtifactAcknowledgment | null>(null); const [station, setStation] = useState(''); const [requested, setRequested] = useState<string[]>([]);
  const [exported, setExported] = useState(''); const [notice, setNotice] = useState('');
  const [definitionVersion, setDefinitionVersion] = useState('');
  const [closing, setClosing] = useState(false);
  const [listRefresh, setListRefresh] = useState(0);
  const scopeTasks = scopedTasks.filter(value => value.kind === 'packaging' && value.scope.kind === 'edit' && (scope.kind === 'project' ? value.scope.data.kind === 'project' && value.scope.data.data === project?.id : value.scope.data.kind === 'definition_draft' && value.scope.data.data.id === scope.data));
  const completedTaskIds = scopeTasks.filter(value => value.state === 'completed').map(value => value.id).join('\n');
  const listKey = JSON.stringify([identity, revision]);
  const currentListKey = useRef(listKey); currentListKey.current = listKey;
  const [storedList, setStoredList] = useState<{ key: string; value: StoredArtifactListState }>({ key: '', value: { status: 'loading', ids: [] } });
  const listed: StoredArtifactListState = storedList.key === listKey ? storedList.value : { status: 'loading', ids: [] };
  useEffect(() => {
    if (!project || props.storedArtifacts?.status !== 'available') return;
    let active = true;
    const producer = props.storedArtifacts;
    void loadStoredArtifactList(() => producer.list(), () => active && live.current && currentListKey.current === listKey,
      value => setStoredList({ key: listKey, value }));
    return () => { active = false; };
  }, [identity, revision, props.storedArtifacts, listRefresh, completedTaskIds]);
  useEffect(() => { setRevision(props.scopeRevision); }, [props.scopeRevision]);
  useEffect(() => { if (scope.kind === 'project' || workspace.context.definition?.draft_id === scope.data) setScopedTasks(workspace.context.tasks); }, [workspace.context.tasks, identity]);
  useEffect(() => {
    setJob(structuredClone(props.initialJob ?? packagingExamples[3].job)); setSaved(props.initialJob ? structuredClone(props.initialJob) : null);
    setRevision(props.scopeRevision); setProduct(project?.products[0] ?? null); setScopedTasks(workspace.context.tasks); setTask(null); setExportTask(null); setEvidence(props.initialEvidence ?? null); setAck(null); setAckScopes([]); setStatement(''); setStation(''); setRequested([]); setError(''); setNotice(''); setExported(''); setPending(0); setDefinitionVersion('');
  }, [identity]);
  const target = { projectId: project?.id ?? '', scope, revision };
  const manufacturingKey = manufacturingExportIdentity(identity, target, evidence);
  const currentManufacturingKey = useRef(manufacturingKey); currentManufacturingKey.current = manufacturingKey;
  const assertCurrent = (captured: string) => assertPackagingContext(captured, current.current, live.current);
  async function act(action: (guard: () => void) => Promise<void>) {
    const captured = identity; setPending(count => count + 1); setError(''); setNotice('');
    try { await action(() => assertCurrent(captured)); }
    catch (failure) { if (live.current && current.current === captured) setError(message(failure)); }
    finally { if (live.current && current.current === captured) setPending(count => count - 1); }
  }
  async function committed(commit: EditCommit, guard: () => void) {
    guard(); setRevision(commit.new_revision); await props.onCommitted(commit); guard();
  }
  const issues = shapeIssues(packagingSchemas.PackageStructureJob, job);
  const unsaved = saved === null || !sameInput(job, saved);
  const working = busy;
  const localInputChanged = !sameInput(job, saved ?? initialDisplayedJob.current.job)
    || !!product && !project?.products.some(value => sameInput(value, product));
  const editorStateListener = useRef(props.onEditorStateChanged); editorStateListener.current = props.onEditorStateChanged;
  useEffect(() => { editorStateListener.current?.({ identity, dirty: localInputChanged, busy: working }); }, [identity, localInputChanged, working]);
  useEffect(() => () => editorStateListener.current?.({ identity, dirty: false, busy: false }), [identity]);
  const result = evidence?.result ?? (task?.result?.kind === 'packaging' ? task.result.data : null);
  const samples = result?.process_poses ?? [];
  const sample = samples[Math.min(sampleIndex, samples.length - 1)] ?? null;
  const stations = project ? packagingStations(project, workspace.modules) : [];
  const selectedStation = stations.find((_, index) => String(index) === station);
  const stationJob = selectedStation?.device.configuration;
  const selectedJobMatches = !!result && stationJob?.packaging_job_id?.kind === 'string' && stationJob.packaging_job_id.value === result.job_ref.id
    && stationJob?.packaging_job_revision?.kind === 'integer' && stationJob.packaging_job_revision.value === result.job_ref.revision;
  const acknowledgments = project && evidence ? artifactAcknowledgments(project, evidence) : [];
  const actualAck = acknowledgments.find(value => value.id === ack?.id) ?? null;
  const computationAvailable = scopeAvailable(workspace.capabilities, 'compute_packaging', scope);
  const inputDiffers = !!evidence && (!evidence.input || !sameInput(job, evidence.input));
  function toggle(values: string[], value: string) { return values.includes(value) ? values.filter(item => item !== value) : [...values, value]; }
  if (!project) return <section className="packaging-workspace"><h2>纸包装专业桌面</h2><p>请先创建或打开工程，再配置结构计算输入。</p><button onClick={props.onClose}>关闭</button></section>;
  return <section className="packaging-workspace" aria-label="纸包装专业桌面">
    <header><div><h2>纸包装 · 结构与过程</h2><p>{project.title} · {scope.kind === 'project' ? '工程内的专业计算' : '专业定义草稿'}</p></div><button onClick={() => localInputChanged ? setClosing(true) : props.onClose()}>关闭</button></header>
    {closing && <div role="dialog" aria-label="未保存的专业输入"><p>当前专业输入尚未全部保存。关闭会放弃本地修改，已提交输入和实际计算任务继续保留。</p><button onClick={() => setClosing(false)}>返回继续编辑</button><button onClick={props.onClose}>明确放弃本地输入并关闭</button></div>}
    <button disabled={working} onClick={() => act(async guard => { const context = await readScopeContext(io, target); guard(); const actual = scope.kind === 'project' ? context.project?.revision : context.definition?.draft_id === scope.data ? context.definition.revision : undefined; if (actual === undefined) throw new Error('未读取到当前编辑范围的数据版本。'); setRevision(actual); setScopedTasks(context.tasks); setListRefresh(value => value + 1); await props.onRefresh(); guard(); setNotice('已刷新当前数据，本地结构输入仍保留。请核对后保存。'); })}>刷新当前数据（保留本地输入）</button><p className="packaging-boundary">编辑结构 → 保存输入 → 计算 → 人工确认适用用途 → 绑定包装工位。专业计算不产生工厂库存，几何结果不代替防护、强度或现场工时。</p>
    <div className="packaging-columns"><div className="packaging-input">
      <h3>产品与输入来源</h3><button disabled={working || scope.kind !== 'project'} onClick={() => setProduct({ id: '', revision: '', item_type_ref: { id: '', revision: '' }, bom_ref: { id: '', revision: '' }, routing_ref: { id: '', revision: '' }, artifact_refs: [] })}>新建版本化产品输入</button><details><summary>可引用的物料、物料清单与工艺版本</summary><p>工程中可引用的实际对象：产品类型 {project.item_types.map(item => item.id).join(', ') || '无'}；BOM {project.boms.map(item => `${item.id}@${item.revision}`).join(', ') || '无'}；工艺路线 {project.routes.map(item => `${item.id}@${item.revision}`).join(', ') || '无'}。引用版本在下方表单填写，保存时检查缺项。</p></details><label>已有工程产品<select value={product?.id ?? ''} disabled={working} onChange={event => setProduct(project.products.find(item => item.id === event.target.value) ?? null)}><option value="">未选择</option>{project.products.map(item => <option key={item.id} value={item.id}>{factoryItemTypeTitle(project,item.item_type_ref)} · 版本 {item.revision}</option>)}</select></label>
      {product && <details><summary>编辑版本化产品与精确依赖</summary><SchemaField schema={packagingSchemas.ProductRevision} value={product} name="版本化产品" onChange={value => setProduct(value as ProductRevision)} disabled={working || scope.kind !== 'project'} /><button disabled={working || scope.kind !== 'project' || shapeIssues(packagingSchemas.ProductRevision, product).length > 0} onClick={() => act(async guard => { const commit = await saveProduct(io, target, product); await committed(commit, guard); setNotice('产品已保存；受影响绑定请核对实际 Host 诊断。'); })}>保存产品修改</button><button disabled={working} onClick={() => setJob({ ...job, subject_ref: { id: product.id, revision: product.revision }, conditions: { ...job.conditions, product_envelope: job.conditions.product_envelope ? { ...job.conditions.product_envelope, subject: { id: product.id, revision: product.revision } } : null } })}>将此产品用于当前结构输入</button></details>}
      <p>包络、允许姿态、净空间与取物条件在下方编辑。未声明的材料、容差和姿态保持缺项。</p>
      <label>新声明输入<select defaultValue="" disabled={working} onChange={event => { if (event.target.value === '') return; setJob(structuredClone(packagingExamples[Number(event.target.value)].job)); setSaved(null); event.target.value = ''; }}><option value="">选择三机制或空白入口</option>{packagingExamples.map((example, index) => <option key={example.title} value={index}>{example.title}</option>)}</select></label><small>自建声明示例；材料与预算依据原样保留，选择后尚未保存和计算。</small>
      <div className="packaging-row"><label>专业输入编号<input value={job.id} disabled={working} onChange={event => setJob({ ...job, id: event.target.value })} /></label><label>输入版本<input type="number" min="0" step="1" value={job.revision} disabled={working} onChange={event => setJob({ ...job, revision: event.target.valueAsNumber })} /></label><button disabled={working || !Number.isSafeInteger(job.revision)} onClick={() => setJob({ ...job, revision: job.revision + 1 })}>新输入修订 +1</button></div>
      <p>板片尺寸、厚度和折轴坐标使用毫米，转动角度使用弧度。展开下方分组可编辑结构、材料、工序和计算预算。</p><details><summary>结构字段与引用格式</summary><p>polygon_mm 为板片顶点，thickness_mm 为厚度，axis_*_mm 为折轴坐标。part_id/panel_*、material_ref 和 step/prerequisites 均引用明确标识。</p></details>
      {sections.map(([key, title]) => <details key={key}><summary>{title}</summary><SchemaField schema={packagingSchemas.PackageStructureJob.properties![key]} value={job[key]} name={title} disabled={working} onChange={value => setJob({ ...job, [key]: value })} /></details>)}
      {issues.length > 0 && <ul className="packaging-error">{issues.map((issue, index) => <li key={index}>{issue.path}：{issue.message}</li>)}</ul>}
      <div className="packaging-actions"><button disabled={working || !!issues.length} onClick={() => act(async guard => { const frozen = structuredClone(job); const commit = await saveJob(io, target, frozen); await committed(commit, guard); setSaved(frozen); setNotice(`已保存 ${frozen.id}@r${frozen.revision}，工程数据版本 ${commit.new_revision}。`); })}>保存专业输入</button><button disabled={working || unsaved || !job.budget || !computationAvailable || !!issues.length} onClick={() => act(async guard => {
        if (!saved || !job.budget) throw new Error('先保存精确输入及明确预算。'); setTask(null); setEvidence(null);
        const displayEvidence = (value: PackagingEvidence) => { guard(); setEvidence(value); setSampleIndex(0); setAck(null); setAckScopes([]); setStatement(''); setRequested([]); };
        const value = await computeJob(io, target, structuredClone(saved), structuredClone(job.budget), state => { guard(); setTask(state); }, async state => {
          guard();
          if (state.task.state === 'cancelled' && state.result?.kind === 'packaging') displayEvidence(await readTaskEvidence(io, target, state.result.data));
          await props.onRefresh(); guard();
        });
        displayEvidence(value);
      })}>计算已保存输入</button>{task && !terminal(task) && task.allowed_controls.includes('cancel') && <button onClick={() => act(async guard => { const value = await cancelPackagingTask(io, target, task.task); guard(); setNotice(`取消回执：state=${value.task.state}，action_accepted=${value.action_accepted}，terminal_won=${value.terminal_won}；${value.issues.map(issue => issue.message).join('；')}。以实际 Task 终态为准。`); })}>请求取消计算</button>}</div>
      <p>{unsaved ? '本地输入尚未保存；运行按钮不会把它当成已保存输入。' : `已保存 ${saved?.id}@r${saved?.revision}。`}{!computationAvailable && ' 当前编辑范围暂未提供结构计算功能。'}</p>
      <TaskDetail state={task} />
      <details><summary>实际提供者与能力边界</summary>{workspace.capabilities.providers.length ? workspace.capabilities.providers.map(provider => <p key={provider.provider_id}>{provider.provider_id} · {provider.producer_status} · {provider.available_modes.join(', ')} · {provider.supported_scope.join(', ')}</p>) : <p>Host 未列出提供者目录；计算按钮只按该精确 Scope 的真实 compute_packaging 能力开放。其它专业定义不据名称获得执行能力。</p>}</details>
    </div><div className="packaging-results">
      <h3>实际结果与冻结输入</h3>
      <label>本次打开后的计算任务<select defaultValue="" disabled={working} onChange={event => { const id = event.target.value; event.target.value = ''; if (!id) return; void act(async guard => {
        const handle = scopedTasks.find(value => value.id === id); if (!handle) throw new Error('实际上下文未列出此 Task。');
        const output = await io.awaitTask(handle, state => { guard(); setTask(state); }); guard();
        if (output.kind !== 'packaging') throw new Error('实际 Task 不是专业结果。'); const value = await readTaskEvidence(io, target, output.data); guard(); setEvidence(value); setSampleIndex(0); setAck(null); setAckScopes([]); setStatement(''); setRequested([]);
      }); }}><option value="">选择计算任务，查看结果或进度</option>{scopeTasks.map((value,index) => <option key={value.id} value={value.id}>计算任务 {index+1} · {taskStateLabel[value.state]}</option>)}</select></label>
      <label>当前范围已保存的计算成果<select defaultValue="" disabled={working || props.storedArtifacts?.status !== 'available' || listed.status !== 'ready' || !listed.ids.length} onChange={event => { const id = event.target.value; event.target.value = ''; if (!id) return; void act(async guard => { if (props.storedArtifacts?.status !== 'available' || listed.status !== 'ready' || !listed.ids.includes(id)) throw new Error('选定成果不在当前范围的已保存列表中，请刷新。'); const value = await props.storedArtifacts.read(id); guard(); if (value.artifact.id !== id || value.result.artifact_id !== id) throw new Error('读取返回的成果与所选成果不一致。'); setEvidence(value); setSampleIndex(0); setAck(null); setAckScopes([]); setStatement(''); setRequested([]); }); }}><option value="">读取保存并重开的实际成果</option>{props.storedArtifacts?.status === 'available' && listed.status === 'ready' && listed.ids.map((id,index) => <option key={id} value={id}>已保存成果 {index+1}</option>)}</select></label>
      {props.storedArtifacts?.status === 'available' ? <><button disabled={working || listed.status === 'loading'} onClick={() => setListRefresh(value => value + 1)}>刷新已保存成果</button>{listed.status === 'loading' ? <p role="status">正在读取已保存的计算成果。</p> : listed.status === 'failed' ? <pre role="alert">{listed.error}</pre> : !listed.ids.length && <p>当前范围尚无已保存的计算成果。</p>}</> : <p>{props.storedArtifacts?.reason ?? '已保存成果的重开读取尚未接通；本会话可继续查看真实计算任务，当前不以其它输入重绘旧成果。'}</p>}
      {!result ? <p>尚无可查看的计算结果。可先选择结构示例，保存输入后开始计算。</p> : <><h4 className={`packaging-${result.outcome}`}>{outcome[result.outcome]}</h4><details><summary>计算成果来源与版本</summary><p>成果编号：{result.artifact_id}；输入编号：{result.job_ref.id}，版本 {result.job_ref.revision}；产品编号：{result.subject_ref.id}，版本 {result.subject_ref.revision}；结果来源：{result.evidence_origin}</p>{evidence && <p>本次计算输入：{evidence.artifact.input_ref.path}<br />计算结果：{evidence.artifact.result_ref.path}</p>}</details>{inputDiffers && <p className="packaging-warning">当前本地输入与此成果所用输入不同。此处保留原成果，认可和绑定仍只指向上述计算成果。</p>}{evidence?.inputReadIssue && <p className="packaging-warning">{evidence.inputReadIssue}</p>}
        {evidence?.input && <button disabled={working} onClick={() => { setJob(structuredClone(evidence.input!)); setSaved(null); setNotice('已将此成果的冻结输入载入本地编辑；保存和计算仍须分别提交。'); }}>将此成果的冻结输入载入本地编辑</button>}
        <table className="packaging-checks"><thead><tr><th>检查</th><th>三态</th><th>查询次数</th><th>干涉 / 误差 / 未判定条件</th></tr></thead><tbody>{result.checks.map((check, index) => <tr key={index}><td>{purposeNames[check.purpose]}</td><td>{outcome[check.outcome]}</td><td>{check.queries_used}</td><td>{check.witnesses.map((witness, i) => <p key={i}>step={witness.step_id}，u=[{witness.interval.join(', ')}]，对象={witness.object_pair?.join(' / ') ?? '未指定'}，接触点={witness.contact_point_mm ? JSON.stringify(witness.contact_point_mm) : '未给出'}；{witness.message}</p>)}{check.unresolved.map((issue, i) => <p key={i}>{issue.code} {issue.field_path}：{issue.message}（{issue.blocks.join(', ')}）</p>)}</td></tr>)}</tbody></table>
        <p>真实预算消耗：{JSON.stringify(result.budget_used)}</p>{result.limitations.map((issue, index) => <p key={index}>{issue.code} {issue.field_path}：{issue.message}（限制：{issue.blocks.join(', ')}）</p>)}
        <h3>步骤与实际三维样本</h3><p>此结果的三维样本只包含板片姿态；未提供固定姿态直线取物的产品运动样本，取物结果见“直线取物”检查。</p>{!samples.length && <p>无实际过程样本；不从终态图生成中间过程。</p>}{evidence?.input && <><ol>{evidence.input.steps.map(step => <li key={step.id}>{step.id} · {step.purpose} · {step.motion.kind}；前置 {step.prerequisites.join(', ') || '无'}</li>)}</ol>{samples.length > 0 && <label>选择实际采样段<select value={Math.min(sampleIndex, samples.length - 1)} onChange={event => setSampleIndex(Number(event.target.value))}>{samples.map((value, index) => <option key={index} value={index}>{value.step_id} · u={value.progress}</option>)}</select></label>}<ProcessViewport job={evidence.input} sample={sample} /><p>构件：{evidence.input.parts.map(part => `${part.id} [${part.panel_ids.join(',')}]`).join('；')}<br />连接：{evidence.input.connections.map(connection => `${connection.id}: ${connection.panel_a}↔${connection.panel_b} (${connection.kind})`).join('；') || '无固定连接'}</p></>}
        {evidence && <ManufacturingExportForm key={manufacturingKey} evidence={evidence} producer={props.manufacturingExport} working={working} onExport={async (declarations, confirmed) => act(async guard => {
          const ensureCurrent = () => { guard(); assertPackagingContext(manufacturingKey, currentManufacturingKey.current, live.current); };
          ensureCurrent(); setExportTask(null); setExported('');
          const value = await exportDeclaredManufacturingEvidence(io, target, evidence, declarations, confirmed, props.manufacturingExport, state => { ensureCurrent(); setExportTask(state); }, ensureCurrent);
          ensureCurrent(); setExported(value ? `${value.package.path}；实际包含 ${value.included_files.map(file => file.path).join('；')}；缺项 ${value.missing.map(issue => issue.message).join('；') || '无'}` : '已取消文件选择，没有导出；再次尝试须重新确认本次携带。');
        })} />}{evidence && <ManufacturingPreview evidence={evidence} readFile={props.readManufacturingFile} />}<h3>制造层与材料数量</h3><table><thead><tr><th>构件</th><th>实际文件</th><th>图层</th><th>单位</th></tr></thead><tbody>{result.manufacturing_files.map((file, index) => <tr key={index}><td>{file.part_id}</td><td>{file.file.path}</td><td>{file.layers.join(' / ')}</td><td>{file.unit}</td></tr>)}</tbody></table><table><thead><tr><th>材料</th><th>构件</th><th>数量</th><th>面积 mm²</th></tr></thead><tbody>{result.material_counts.map((count, index) => <tr key={index}><td>{count.material_id}</td><td>{count.part_id}</td><td>{count.quantity}</td><td>{count.area_mm2 ?? '未给出'}</td></tr>)}</tbody></table><p>SVG 字节由 Host 保存。原导出未确认制造文件的交付权利；实际包含文件和缺项以 Host 回执为准。</p>{scope.kind === 'definition_draft' && <details><summary>定义草稿导出前的真实版本提交</summary><p>Host 要求先提交实际定义版本；计算成果仍在此草稿的专业计算范围，不能把未提交草稿当作已保存模块导出。</p><button disabled={working} onClick={() => act(async guard => { const context = await readScopeContext(io, target); guard(); if (context.definition?.draft_id !== scope.data) throw new Error('Host 未返回此实际定义草稿。'); setDefinitionVersion(context.definition.definition_ref.version); setScopedTasks(context.tasks); setRevision(context.definition.revision); setNotice(context.definition.issues.map(issue => `${issue.code}: ${issue.message}`).join('；') || '已读取实际定义版本。'); })}>读取实际定义版本与修订</button><label>明确提交的版本<input value={definitionVersion} disabled={working} onChange={event => setDefinitionVersion(event.target.value)} /></label><button disabled={working || !definitionVersion.trim()} onClick={() => act(async guard => { const commit = await commitProfessionalDefinitionVersion(io, target, definitionVersion); await committed(commit, guard); setNotice('定义版本已由 Host 提交；可另行选择 ZIP 导出专业成果包。'); })}>明确提交此定义版本</button></details>}<button disabled={working || !evidence} onClick={() => act(async guard => { if (!evidence) return; const value = await exportEvidence(io, target, evidence, state => { guard(); setExportTask(state); }); guard(); setExported(value ? `${value.package.path}；实际包含 ${value.included_files.map(file => file.path).join('；')}；缺项 ${value.missing.map(issue => issue.message).join('；') || '无'}` : '已取消文件选择，没有导出。'); })}>选择真实 ZIP 并导出专业工程包（未声明文件列缺项）</button><TaskDetail state={exportTask} />{exportTask && !terminal(exportTask) && exportTask.allowed_controls.includes('cancel') && <button onClick={() => act(async guard => { const value = await cancelPackagingTask(io, target, exportTask.task); guard(); setNotice(`导出取消回执：${value.task.state}，accepted=${value.action_accepted}，terminal_won=${value.terminal_won}。`); })}>请求取消导出</button>}{exported && <p>{exported}</p>}
        <h3>明确人类认可</h3>{scope.kind !== 'project' && <p>当前是定义草稿；可计算和导出，人工用途确认和设备绑定须在工程中完成。</p>}<div>{result.supported_scope.map(purpose => <label key={purpose}><input type="checkbox" disabled={working || scope.kind !== 'project'} checked={ackScopes.includes(purpose)} onChange={() => setAckScopes(toggle(ackScopes, purpose))} />{purposeNames[purpose] ?? purpose}</label>)}</div><label>我的认可说明<textarea disabled={working || scope.kind !== 'project'} value={statement} onChange={event => setStatement(event.target.value)} /></label><button disabled={working || scope.kind !== 'project' || !evidence || !ackScopes.length || !statement.trim()} onClick={() => act(async guard => { if (!evidence) return; const acknowledgment = acknowledgmentCommand(evidence, ackScopes, statement); const commit = await recordAcknowledgment(io, target, acknowledgment); await committed(commit, guard); setAck(acknowledgment); setRequested([]); setNotice('用途认可已实际保存；包装工位尚未绑定。'); })}>确认并记录用途认可</button>
        <h3>单独绑定真实包装工位</h3><label>实际设备 / 精确工艺规则<select value={station} disabled={working || scope.kind !== 'project'} onChange={event => setStation(event.target.value)}><option value="">明确选择工位</option>{stations.map((value, index) => <option key={index} value={index}>{value.device.title} ({value.device.id}) · {value.operation.id}@{value.operation.version}</option>)}</select></label>{!stations.length && <p>当前工程还没有可用的包装工位。请先添加具备包装工艺的设备。</p>}{selectedStation && <><p>工位输入选择：{selectedJobMatches ? '已明确选择此成果的输入及版本' : '当前配置尚未选择此成果的输入及版本'}。{!selectedStation.jobSelectionAvailable && ' 此设备版本缺少真实 packaging_job_id / packaging_job_revision 参数声明，须先在定义中补齐并显式采用。'}</p><button disabled={working || scope.kind !== 'project' || !result || !selectedStation.jobSelectionAvailable || selectedJobMatches} onClick={() => act(async guard => { if (!result) return; const commit = await selectStationJob(io, target, selectedStation.device.id, result.job_ref); await committed(commit, guard); setNotice('工位输入与版本选择已提交；成果绑定仍需下一次明确动作。'); })}>明确为工位选择此执行 Job</button></>}<label>选择已保存的用途认可<select value={actualAck?.id ?? ''} disabled={working || scope.kind !== 'project'} onChange={event => { setAck(acknowledgments.find(value => value.id === event.target.value) ?? null); setRequested([]); }}><option value="">明确选择认可</option>{acknowledgments.map(value => <option key={value.id} value={value.id}>{value.id} · {value.accepted_scope.join(', ')}</option>)}</select></label><p>实际认可：{actualAck ? `${actualAck.id} · ${actualAck.accepted_scope.join(', ')}` : '尚未明确选择此成果的用途认可'}</p>{actualAck?.accepted_scope.map(purpose => <label key={purpose}><input type="checkbox" checked={requested.includes(purpose)} disabled={working || scope.kind !== 'project'} onChange={() => setRequested(toggle(requested, purpose))} />绑定用途：{purpose}</label>)}<p>预览影响：只为选中包装设备与精确 Operation 采用上述 Artifact；base_revision={revision}。最终内容、认可用途及权限由 Host 复核。</p><button disabled={working || scope.kind !== 'project' || !evidence || !actualAck || !selectedStation || !selectedJobMatches || !requested.length} onClick={() => act(async guard => { if (!evidence || !actualAck || !selectedStation) return; const commit = await adoptArtifact(io, target, { artifact_id: evidence.artifact.id, acceptance_ref: actualAck.id, consumer_ref: { kind: 'device', id: selectedStation.device.id }, operation_ref: selectedStation.operation, requested_scope: requested }); guard(); setRevision(commit.new_revision); await props.onRefresh(); guard(); setNotice('包装工位绑定已由 Host 提交；工厂执行仍读取实际绑定与输入。'); })}>单独提交此工位的采用绑定</button>
      </>}
      <h3>工程中实际保存的绑定与复核诊断</h3>{project.bindings.map(binding => <p key={binding.id}>{binding.id}：{binding.consumer_ref.id} / {binding.operation_ref.id}@{binding.operation_ref.version} → {binding.artifact_ref}，预期产品版本 {binding.expected_subject_revision}，用途 {binding.requested_scope.join(', ')}</p>)}{project.issues.filter(issue => /binding|artifact|subject|professional|packaging/.test(issue.code)).map((issue, index) => <p className="packaging-warning" key={index}>{issue.code} {issue.field_path}：{issue.message}</p>)}
    </div></div>{error && <div className="packaging-error" role="alert"><p>{error.split('\n')[0]} 本地输入已保留。</p><details><summary>错误技术详情</summary><pre>{error}</pre></details></div>}{notice && <p role="status">{notice}</p>}
  </section>;
}
