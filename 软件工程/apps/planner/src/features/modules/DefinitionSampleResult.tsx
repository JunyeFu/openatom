import { HostFailure } from '../../services/projectHost';
import { decimalText } from '../../scene/runtime';
import { sampleHasRun } from './definitionSample';
import { optionLabel } from './labels';
import type { AuthoringDraft } from './types';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { Quantity } from '../../../../../packages/commands/generated/typescript/Quantity';
import type { ApiError } from '../../../../../packages/commands/generated/typescript/ApiError';
import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';

const statuses: Record<string, string> = { queued: '排队中', running: '计算中', paused: '已暂停', cancel_requested: '已申请取消', completed: '已完成', failed: '失败', cancelled: '已取消', ready: '就绪', working: '工作中', waiting_resource: '等待资源', waiting_output: '等待输出' };
const reasons: Record<string, string> = { missing_material: '缺少物料', resource_busy: '资源占用', calendar: '日历', preparation: '准备条件', quality: '质量条件', output_capacity: '输出容量', receiver_capacity: '接收容量' };
const ends: Record<string, string> = { resolved: '已解决', condition_changed: '条件变化', released: '已释放', observation_end: '观察窗口结束', computed_boundary: '计算边界' };
const claimKinds: Record<string, string> = { job: '作业占用', phase: '阶段占用', output_hold: '等待输出保留', transport: '运输占用' };
const stages: Record<string, string> = { factory_released: '工厂放行', received: '已接收', unloaded_and_released: '卸货并放行' };
const origins: Record<string, string> = { actual_software: '实际软件执行', design_artifact: '设计材料', manual_expected: '人工预期', type_fixture: '类型夹具', imported_claim: '导入声明' };
const ms = (value: number | null | undefined) => value == null ? '未知（未提供）' : `${value} ms`;
const quantity = (value: Quantity | null | undefined) => value == null ? '未知（未提供）' : `${decimalText(value.value)} ${value.unit}`;
const object = (value: { kind: string; id: string } | null | undefined) => value ? `${value.kind} · ${value.id}` : '未指明';
function IssueRows({ issues }: { issues: Issue[] }) { return <>{issues.map((issue, index) => <p className="authoring-warning" key={index}><strong>{issue.code}{issue.field_path ? ` · ${issue.field_path}` : ''}</strong><span>{issue.message}</span>{issue.object_ref && <span>{object(issue.object_ref)}</span>}{issue.blocks.length > 0 && <span>限制用途：{issue.blocks.map(optionLabel).join('、')}</span>}</p>)}</>; }
function ApiFailure({ value }: { value: ApiError }) { return <div className="authoring-error"><strong>{value.code} · {value.message}</strong><dl><dt>预期修订</dt><dd>{value.expected_revision ?? '未提供'}</dd><dt>实际修订</dt><dd>{value.actual_revision ?? '未提供'}</dd></dl>{value.object_refs.map((ref, index) => <p key={index}>{object(ref)}</p>)}{value.field_errors.map((field, index) => <p key={index}>{field.pointer}：{field.reason}</p>)}</div>; }

export function AuthoringFailure({ value }: { value: unknown }) {
  return value instanceof HostFailure ? <ApiFailure value={value.detail} /> : <p className="authoring-error">{value instanceof Error ? value.message : String(value)}</p>;
}

export interface DefinitionSampleResultProps {
  task: TaskState;
  run: RunResult | null;
  readError: unknown | null;
  reading: boolean;
  currentDraft: AuthoringDraft | null;
  onRead(): void;
}
/** Independent sample view. It owns no Project, production clock or main Run selection. */
export function DefinitionSampleResult({ task, run, readError, reading, currentDraft, onRead }: DefinitionSampleResultProps) {
  const scope = task.task.scope.kind === 'edit' && task.task.scope.data.kind === 'definition_draft' ? task.task.scope.data.data : null;
  return <section className="authoring-sample-result" aria-label="此定义样例结果"><h3>此定义样例结果</h3><p>任务完成只表示计算任务终止；业务目标以此冻结 Run 的实际判定和限制为准。</p><dl><dt>实际任务</dt><dd>{task.task.id}</dd><dt>任务状态</dt><dd>{statuses[task.task.state]}</dd><dt>冻结定义输入</dt><dd>{scope ? `${scope.id} · r${scope.revision}` : '未返回定义草稿 Scope'}</dd><dt>Task 输入修订</dt><dd>{task.task.input_revision}</dd><dt>原始操作</dt><dd>{task.originating_operation}</dd><dt>结果类型</dt><dd>{task.result?.kind ?? '无结果'}</dd></dl>
    {scope && currentDraft?.summary.draft_id === scope.id && currentDraft.summary.revision !== scope.revision && <p className="authoring-status">当前草稿已为 r{currentDraft.summary.revision}；此结果仍属于冻结的 r{scope.revision}，后续编辑未进入这个样例。</p>}
    {task.failure && <ApiFailure value={task.failure} />}{task.task.failure && <IssueRows issues={[task.task.failure]} />}
    {sampleHasRun(task) ? <div className="authoring-actions"><button type="button" disabled={reading} onClick={onRead}>{reading ? '正在实际读取冻结结果…' : '重新实际读取此冻结样例'}</button></div> : <p>此任务没有可读取的已完成 Run，不生成业务结果。</p>}
    {readError != null && <section role="alert"><p>业务 Run 读取失败，原任务状态仍保留；尚未取得完整业务结果。</p><AuthoringFailure value={readError} /></section>}
    {sampleHasRun(task) && !run && readError == null && <p>{reading ? '正在通过原 DefinitionScope 读取完整业务结果。' : '尚未读取完整业务 Run。'}</p>}
    {sampleHasRun(task) && run && <><dl><dt>实际样例 Run</dt><dd>{run.id}</dd><dt>Run 冻结工程</dt><dd>{run.input_ref.project_id} · r{run.input_ref.project_revision}</dd><dt>方案输入版本</dt><dd>{run.input_ref.scenario_id} · r{run.input_ref.scenario_revision}</dd><dt>观察窗口</dt><dd>{ms(run.window.start_ms)} → {ms(run.window.end_ms)}</dd><dt>实际计算边界</dt><dd>{ms(run.computed_until_ms)}</dd><dt>计算状态</dt><dd>{statuses[run.computation_status]}</dd><dt>完整窗口标志</dt><dd>{run.whole_window_computed ? '是' : '否'}</dd><dt>实际目标判定</dt><dd>{run.goal_met == null ? '未知（实际回包未给出）' : run.goal_met ? '已达成（此样例实际回包）' : '未达成（此样例实际回包）'}</dd><dt>证据来源</dt><dd>{origins[run.evidence_origin]} · {run.evidence_origin}</dd></dl>
      {!run.whole_window_computed && <p className="authoring-warning">观察窗口尚未完整计算；计算边界之后未知，不补全尾段。</p>}
      <details><summary>实际冻结输入文件引用</summary><p>{run.input_ref.frozen_project_file.kind} · {run.input_ref.frozen_project_file.path}</p><p>{run.input_ref.frozen_scenario_file.kind} · {run.input_ref.frozen_scenario_file.path}</p></details>
      <h4>接受事件 · {run.acceptance_events.length}</h4><div className="authoring-result-table"><table><thead><tr><th>实际时刻</th><th>实际数量</th><th>订单与物品</th><th>接收位置 / 阶段</th></tr></thead><tbody>{run.acceptance_events.map((event, index) => <tr key={index}><td>{ms(event.at_ms)}</td><td>{quantity(event.quantity)}</td><td>订单：{event.order_id ?? '未指明'}<br />物品：{event.item_refs.join('、') || '未指明'}</td><td>{event.destination}<br />{stages[event.acceptance_stage]}</td></tr>)}</tbody></table></div>
      <h4>实际订单结果 · {run.orders.length}</h4><div className="authoring-result-table"><table><thead><tr><th>订单</th><th>需求 / 接受 / 未满足</th><th>完成 / 交期 / 延迟</th></tr></thead><tbody>{run.orders.map(order => <tr key={order.order_id}><td>{order.order_id}</td><td>{quantity(order.demand)} / {quantity(order.accepted)} / {quantity(order.outstanding)}</td><td>{ms(order.completion_ms)} / {ms(order.due_at_ms)} / {ms(order.tardiness_ms)}</td></tr>)}</tbody></table></div>
      <p>实际作业 {run.jobs.length} · 资源占用记录 {run.resource_claims.length} · 阶段工作记录 {run.phase_work_intervals.length} · 等待记录 {run.waits.length} · 运输记录 {run.transport.length}</p>
      <details><summary>查看实际作业与阶段工作记录</summary><div className="authoring-result-table"><table><thead><tr><th>作业 / 状态</th><th>开始 / 完成</th><th>有效进度 / 阶段</th></tr></thead><tbody>{run.jobs.map(job => <tr key={job.id}><td>{job.id}<br />{statuses[job.state]}</td><td>{ms(job.started_at_ms)} / {ms(job.finished_at_ms)}</td><td>{ms(job.effective_progress_ms)} / {job.phase_ref ?? '未指明'}</td></tr>)}</tbody></table></div>{run.phase_work_intervals.map((work, index) => <p key={index}>{work.job_id} · {work.phase_id} · {ms(work.interval.start_ms)} → {ms(work.interval.end_ms)}</p>)}</details>
      <details><summary>查看实际等待原因与区间 · {run.waits.length}</summary>{run.waits.map((wait, index) => <section className="authoring-result-record" key={index}><p>{object(wait.object_ref)} · 阶段 {wait.phase_ref ?? '未指明'}</p><p>{ms(wait.interval.start_ms)} → {ms(wait.interval.end_ms)} · 结束原因：{ends[wait.end_cause]}</p>{wait.reasons.map((reason, position) => <p key={position}>{reasons[reason.kind]} · 阻挡对象 {object(reason.blocking_object)} · 实际要求 {reason.required} · 下个已知边界 {ms(reason.next_known_boundary_ms)}</p>)}</section>)}</details>
      <details><summary>查看实际资源占用记录 · {run.resource_claims.length}</summary>{run.resource_claims.map(claim => <section className="authoring-result-record" key={claim.id}><p>{claim.resource_id} · {claimKinds[claim.kind]} · 实际占用 {claim.units} 单位</p><p>记录 {claim.id} · 作业 {claim.job_id} · 阶段 {claim.phase_ref ?? '未指明'}</p>{claim.intervals.map((interval, index) => <p key={index}>{ms(interval.start_ms)} → {ms(interval.end_ms)} · {ends[claim.interval_end_causes[index]] ?? '结束原因未知（未提供）'}</p>)}</section>)}</details>
      <h4>实际 Run 限制 · {run.limitations.length}</h4><IssueRows issues={run.limitations} />{run.limitations.length === 0 && <p>此 Run 未返回限制项。</p>}
    </>}
    {task.result?.kind === 'run' && <details><summary>原任务 Run 摘要的检查项 · {task.result.data.issues.length}</summary><IssueRows issues={task.result.data.issues} /></details>}
  </section>;
}
