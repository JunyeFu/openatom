import { runStatusLabel, scenarioName } from '../../services/workbenchLabels';
import { useState } from 'react';
import type { MetricScope } from '../../../../../packages/commands/generated/typescript/MetricScope';
import type { EvaluationFormat } from '../../../../../packages/commands/generated/typescript/EvaluationFormat';
import type { ReportMode } from '../../../../../packages/commands/generated/typescript/ReportMode';
import type { EvaluationReport } from '../../../../../packages/commands/generated/typescript/EvaluationReport';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { DeliveryServices, DeliveryWorkspaceProps, RunEvidence } from '../comparison/types';
import { quantityText, scenarioReferences, scopeText } from '../comparison/model';
import { IssueList, JsonDetails } from '../comparison/Shared';
import type { DeliveryTask } from './useDeliveryTask';

export function ReportPanel({ workspace, services, evidence, scope, ready, task, onLocate }: {
  workspace: WorkspaceSnapshot; services: DeliveryServices; evidence: readonly RunEvidence[]; scope: MetricScope; ready: boolean; task: DeliveryTask; onLocate: DeliveryWorkspaceProps['onLocate'];
}) {
  const [mode, setMode] = useState<ReportMode>('simulation_evaluation');
  const [formats, setFormats] = useState<EvaluationFormat[]>(['pdf', 'html', 'csv', 'json']);
  const [scenarios, setScenarios] = useState<string[]>(workspace.scenarios.map(s => s.id));
  const project = workspace.project!;
  const scenarioRefs = mode === 'design_description' ? workspace.scenarios.filter(s => scenarios.includes(s.id)).map(s => ({ scenario_id: s.id, revision: s.revision })) : scenarioReferences(evidence);
  const disabledReason = !formats.length ? '至少选择一种输出格式。' : mode === 'simulation_evaluation' && (!ready || !evidence.length) ? '请先在左侧选择一条运行记录。尚未运行时，可选择导出设计说明。' : '';
  const output = task.result?.kind === 'evaluation' ? task.result.data : null;
  return <section className="delivery-panel" aria-label="评估报告准备">
    <header><div><h2>导出评估报告</h2><p>一次生成所选格式的同一份报告。可先导出一次运行；调整后再比较两次运行。</p></div></header>
    <fieldset><legend>报告依据</legend><label><input type="radio" name="report-mode" checked={mode === 'simulation_evaluation'} onChange={() => setMode('simulation_evaluation')} />实际运行评估</label><label><input type="radio" name="report-mode" checked={mode === 'design_description'} onChange={() => setMode('design_description')} />尚未运行的设计说明</label></fieldset>
    {mode === 'design_description' ? <><p className="delivery-hint">设计说明介绍当前工厂和方案；实际产量、完工时间和改善量将在运行后评估。</p><fieldset><legend>当前草稿方案</legend>{workspace.scenarios.map(s => <label key={s.id}><input type="checkbox" checked={scenarios.includes(s.id)} onChange={e => setScenarios(e.target.checked ? [...scenarios, s.id] : scenarios.filter(id => id !== s.id))} />{scenarioName(s)}</label>)}{!workspace.scenarios.length && <p>尚无方案；保留当前工程的设计说明。</p>}</fieldset></> : <div className="delivery-evidence">{evidence.map(e => <p key={e.run.id}>{scenarioName(e.scenario)} · {e.run.whole_window_computed ? '完整窗口' : '仅前缀，尾项未知'} · {runStatusLabel[e.run.computation_status]}</p>)}</div>}
    <fieldset><legend>输出格式</legend>{(['pdf', 'html', 'csv', 'json'] as const).map(f => <label key={f}><input type="checkbox" checked={formats.includes(f)} onChange={e => setFormats(e.target.checked ? [...formats, f] : formats.filter(v => v !== f))} />{f.toUpperCase()}{f === 'html' && '（离线）'}{f === 'csv' && '（15分表）'}</label>)}</fieldset>
    <details><summary>工程与运行来源</summary><dl><dt>当前工程</dt><dd>{project.title} · {project.id} r{project.revision}</dd><dt>实际Run引用</dt><dd>{mode === 'design_description' ? '不适用' : evidence.map(e => e.run.id).join('、') || '未选择'}</dd><dt>方案及版本</dt><dd>{scenarioRefs.map(s => `${s.scenario_id} r${s.revision}`).join('、') || '无'}</dd><dt>接受范围</dt><dd>{scope.acceptance.length ? scope.acceptance.map(scopeText).join('；') : '按宿主分别列出已声明范围；不暗分单位成本'}</dd></dl></details>
    <p>价格、币种、日历或计算边界缺失时保留未知；明确的零费率与“没有数据”分开。专业图、用途认可和绑定只取真实已有结果。</p>
    {disabledReason && <p className="delivery-hint">{disabledReason}</p>}<button className="primary" disabled={task.active || !!disabledReason} onClick={() => void task.start(() => services.startEvaluation({ revision: project.revision, report_mode: mode, scenario_refs: scenarioRefs, run_refs: mode === 'design_description' ? [] : evidence.map(e => e.run.id), metric_scope: scope, formats }), 'evaluation')}>选择目录并生成报告</button>
    {output && <section><h3>上次任务返回的实际文件</h3><p>报告已生成，文件保存在刚才选择的目录中。</p><OutputFiles key={output.context.id} files={output.files} /><IssueList issues={output.limitations} /><EvaluationView report={output.context} onLocate={onLocate} /></section>}
  </section>;
}

function OutputFiles({ files }: { files: readonly { path: string; kind: string }[] }) {
  const [open, setOpen] = useState(false), [page, setPage] = useState(0);
  return <details onToggle={event => setOpen(event.currentTarget.open)}><summary>查看生成文件 · 共 {files.length} 项，含报告资源与工程数据</summary>{open && <><button disabled={page === 0} onClick={() => setPage(page - 1)}>上一页文件</button><span> 第 {page + 1} / {Math.ceil(files.length / 100)} 页 </span><button disabled={(page + 1) * 100 >= files.length} onClick={() => setPage(page + 1)}>下一页文件</button><ul>{files.slice(page * 100, page * 100 + 100).map(file => <li key={file.path}><code>{file.path}</code> · {file.kind}</li>)}</ul></>}</details>;
}

export function EvaluationView({ report, onLocate }: { report: EvaluationReport; onLocate?: DeliveryWorkspaceProps['onLocate'] }) {
  return <section aria-label="实际评估结果"><h3>{report.mode === 'design_description' ? '设计说明' : '实际运行评估'} · {report.id}</h3><p>{report.question_and_scope}</p><p>工程 {report.project_id} · 导出修订 r{report.project_revision}；Run：{report.run_refs.join('、') || '不适用'}</p>
    {report.conclusions.map((v, n) => <p key={n}>{v}</p>)}<IssueList issues={report.missing_conditions} /><IssueList issues={report.limitations} />
    <h4>接受范围与支持边界</h4><ul>{report.acceptance_scope.map((s, i) => <li key={i}>{scopeText(s)}</li>)}{report.supported_scope.map((s, i) => <li key={`supported-${i}`}>{s}</li>)}</ul>
    {report.generation_context && <section><h4>目标建厂依据</h4><dl><dt>行业／产品</dt><dd>{report.generation_context.goal.industry}／{report.generation_context.goal.product_family}</dd><dt>每日接受目标</dt><dd>{report.generation_context.goal.accepted_quantity_per_day ? quantityText(report.generation_context.goal.accepted_quantity_per_day) : '未声明'}</dd><dt>模板精确版本</dt><dd>{report.generation_context.template_ref ? `${report.generation_context.template_ref.id}@${report.generation_context.template_ref.version}` : '未提供'}</dd><dt>初案Run</dt><dd>{report.generation_context.initial_run_id ?? '未提供'}</dd><dt>修正Run</dt><dd>{report.generation_context.revised_run_ids.join('、') || '无；单Run可直接交付'}</dd><dt>停止原因</dt><dd><JsonDetails label="查看宿主停止原因" value={report.generation_context.stop_reason ?? null} /></dd></dl><JsonDetails label="场地、班次、采用参数与布局检查" value={{ goal: report.generation_context.goal, parameter_sources: report.generation_context.parameter_sources, draft_check: report.generation_context.draft_check, changes: report.generation_context.changes }} /></section>}
    {report.run_evaluations.map(r => <section key={r.run_id}><h4>{r.run_id} · 冻结输入 r{r.input_ref.project_revision} · {r.whole_window_computed ? '完整窗口' : '仅已计算前缀'}</h4><p>窗口 {r.requested_window.start_ms}—{r.requested_window.end_ms} ms；计算到 {r.computed_until_ms ?? '未知'}；目标{r.goal_met == null ? '未知' : r.goal_met ? '达标' : '未达标'}</p>
      <div className="delivery-table-scroll"><table><caption>订单接受与完整时刻（基础单位）</caption><thead><tr><th>订单</th><th>需求</th><th>实际接受</th><th>完整接受C</th><th>迟延</th></tr></thead><tbody>{r.order_metrics.map(o => <tr key={o.order_id}><td>{o.order_id}</td><td>{quantityText(o.demand)}</td><td>{quantityText(o.accepted)}</td><td>{o.completion_ms == null ? '未知' : `${o.completion_ms} ms`}</td><td>{o.tardiness_ms == null ? '未知' : `${o.tardiness_ms} ms`}</td></tr>)}</tbody></table></div>
      <div className="delivery-table-scroll"><table><caption>条件成本（同币种分别列出，不将未知补零）</caption><thead><tr><th>币种</th><th>已知部分</th><th>完整所选总额</th><th>接受口径／数量</th><th>单位成本</th></tr></thead><tbody>{r.costs.currencies.map(c => <tr key={c.currency}><td>{c.currency}</td><td>{c.known_subtotal}</td><td>{c.complete_total ?? '未知'}</td><td>{c.accepted_scope ? scopeText(c.accepted_scope) : '未选单一口径'}{c.accepted_quantity && ` · ${quantityText(c.accepted_quantity)}`}</td><td>{c.unit_cost ?? '未知'}<IssueList issues={c.missing_conditions} /></td></tr>)}</tbody></table></div>
      {!r.costs.inputs.length && <p>没有提供所选成本输入，不能据此声称完整成本为零。</p>}<IssueList issues={r.costs.missing_conditions} /><JsonDetails label="冻结费率、资源占用与来源" value={{ inputs: r.costs.inputs, lines: r.costs.lines, resources: r.resource_metrics }} />
      <div className="delivery-table-scroll"><table><caption>实际资源占用与等待（不相加为总工时）</caption><thead><tr><th>资源</th><th>占用／容量时间</th><th>可用容量时间</th><th>有效工作</th><th>等待</th><th>利用率</th><th>缺项</th></tr></thead><tbody>{r.resource_metrics.map(m => <tr key={m.resource_id}><td>{m.resource_id}</td><td>{m.any_busy_ms} ms／{m.occupied_capacity_ms} {m.capacity_unit}·ms</td><td>{m.available_capacity_ms == null ? '未知' : `${m.available_capacity_ms} ${m.capacity_unit}·ms`}</td><td>{m.effective_work_ms == null ? '未知' : `${m.effective_work_ms} ms`}</td><td>{m.waiting_ms == null ? '未知' : `${m.waiting_ms} ms`}</td><td>{m.utilization ?? '未知'}</td><td><IssueList issues={m.missing_conditions} /></td></tr>)}</tbody></table></div>
      <IssueList issues={r.limitations} /><JsonDetails label="该Run的等待原因、接受事件与参数来源" value={{ waits: r.waits, acceptance_events: r.acceptance_events, input_assumptions: r.input_assumptions }} />
      {!!r.waits.length && <WaitingRecords key={`${report.id}:${r.run_id}`} run={r} onLocate={onLocate} />}
    </section>)}<JsonDetails label="报告模块精确版本、素材权利与专业附件" value={{ modules: report.module_versions, rights: report.asset_rights, images: report.layout_and_schedule, editable_project: report.editable_project_ref }} /><p>报告JSON是评估数据；另行交付可编辑工程，才能重开修改。没有完整运行快照时从初始条件重跑。</p>
  </section>;
}

function WaitingRecords({ run, onLocate }: { run: EvaluationReport['run_evaluations'][number]; onLocate?: DeliveryWorkspaceProps['onLocate'] }) {
  const [page, setPage] = useState(0);
  const start = page * 100;
  return <section aria-label="等待记录明细"><p>共 {run.waits.length} 条等待记录，每页100条。</p><button disabled={page === 0} onClick={() => setPage(page - 1)}>上一页等待</button><span> 第 {page + 1} / {Math.ceil(run.waits.length / 100)} 页 </span><button disabled={start + 100 >= run.waits.length} onClick={() => setPage(page + 1)}>下一页等待</button><div className="delivery-table-scroll"><table><caption>等待与对象定位</caption><thead><tr><th>对象</th><th>实际区间（ms）</th><th>原因与结束</th></tr></thead><tbody>{run.waits.slice(start, start + 100).map((w, index) => <tr key={start + index}><td>{onLocate ? <button onClick={() => onLocate(run.run_id, w.object_ref)}>{w.object_ref.kind}:{w.object_ref.id}</button> : `${w.object_ref.kind}:${w.object_ref.id}`}</td><td>{w.interval.start_ms}—{w.interval.end_ms}</td><td><JsonDetails label="查看等待条件" value={{ reasons: w.reasons, end_cause: w.end_cause, execution_ref: w.execution_ref, phase_ref: w.phase_ref }} /></td></tr>)}</tbody></table></div></section>;
}
