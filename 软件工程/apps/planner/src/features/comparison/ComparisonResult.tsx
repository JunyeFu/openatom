import { scenarioName } from '../../services/workbenchLabels';
import type { ComparisonConditionKey } from '../../../../../packages/commands/generated/typescript/ComparisonConditionKey';
import type { MetricValue } from '../../../../../packages/commands/generated/typescript/MetricValue';
import type {RunComparison} from '../../../../../packages/commands/generated/typescript/RunComparison';
import type {RunEvidence,DeliveryWorkspaceProps} from './types';
import {differenceText,metricLabels,metricText,scopeText} from './model';
import {IssueList,JsonDetails} from './Shared';
import type { SourceBasis } from '../../../../../packages/commands/generated/typescript/SourceBasis';
const basisNames: Record<SourceBasis, string> = { user_provided: '用户提供', module_declared: '模块声明', template_declared_example: '模板示例', unknown: '来源未知' };
const conditionNames: Record<ComparisonConditionKey, string> = { time_reference: '时间基准', observation_window: '观察窗口', acceptance_scope: '产品、单位与接受位置', demand: '需求、释放时点与交期', selected_scope: '执行对象范围', initial_conditions: '初始库存与资源条件', supported_capabilities: '完整计算与模块能力', explained_changes: '输入变化说明' };
const changeKinds: Record<string, string> = { added: '新增', removed: '移除', changed: '修改' };
const changeReason = (reason: string | null | undefined) => reason === 'JSON text representation of complete frozen field; no value inferred' ? '保留完整冻结字段的原始JSON，不据此推断未声明值。' : !reason ? '未声明理由' : /[\u3400-\u9fff]/u.test(reason) ? reason : '变更理由需进一步核对，请查看差异技术来源。';
function MetricReading({ value }: { value: MetricValue | undefined }) {
 return <>{value ? metricText(value) : '未返回该Run指标'}{value?.kind === 'unknown' && <JsonDetails label="查看未知原因来源" value={value.data} />}</>;
}
/** The actual comparison result, also used by the ordinary comparison panel. */
export function ComparisonResult({value,evidence,onLocate,embedded=false}:{value:RunComparison;evidence:readonly RunEvidence[];onLocate?:DeliveryWorkspaceProps['onLocate'];embedded?:boolean}){
 const runIds=value.run_ids;
 const runName = (id: string) => { const source = evidence.find(e => e.run.id === id); return `${runIds.indexOf(id) === 0 ? '基准' : '候选'} · ${source ? scenarioName(source.scenario) : id}`; };
 return <section className={embedded?'comparison-result':'delivery-panel'} aria-label="实际方案比较结果">
 {!embedded&&evidence.length>0&&<div className="delivery-run-pair">{evidence.map(e=><article key={e.run.id}><strong>{runName(e.run.id)}</strong><details><summary>运行记录与冻结来源</summary><dl><dt>运行记录</dt><dd>{e.run.id}</dd><dt>冻结工程</dt><dd>{e.project.id} r{e.project.revision}</dd><dt>冻结方案</dt><dd>{e.scenario.id} r{e.scenario.revision}</dd><dt>计算边界</dt><dd>{e.run.computed_until_ms ?? '未知'} ms · {e.run.whole_window_computed?'完整窗口':'仅已计算前缀'}</dd></dl></details><IssueList issues={e.run.limitations}/></article>)}</div>}

      {value.pairs.map(pair => <section key={`${pair.baseline_run_id}:${pair.candidate_run_id}`}><h3>可比性：{{ strict: '共同条件一致', controlled_changes: '有明确输入变化', side_by_side_only: '仅并列，不作改善结论' }[pair.status]}</h3>
        <ul>{pair.conditions.map(c => <li key={c.key}>{c.matches ? '一致' : '不一致'} · {conditionNames[c.key]}<IssueList issues={c.issues} /></li>)}</ul>
        <h3>输入变化与来源</h3>{!pair.input_changes.length && <p>宿主没有列出输入变化。</p>}
        <details><summary>查看全部输入变化与技术来源 · {pair.input_changes.length}处</summary><div className="delivery-table-scroll"><table><thead><tr><th>对象／字段</th><th>基准输入</th><th>候选输入</th><th>来源与理由</th><th>定位</th></tr></thead><tbody>{pair.input_changes.map((c, n) => <tr key={`${c.target.id}:${c.path}:${n}`}><td>{c.target.kind}:{c.target.id}<br /><code>{c.path}</code><br />{changeKinds[c.kind] ?? c.kind}</td><td><JsonDetails label="展开原值" value={c.before ?? null} /></td><td><JsonDetails label="展开新值" value={c.after ?? null} /></td><td>{c.basis ? basisNames[c.basis] : '来源未知'}<br />{changeReason(c.reason)}<JsonDetails label="查看差异技术来源" value={{ kind: c.kind, basis: c.basis, reason: c.reason }} /></td><td>{onLocate ? <><button onClick={() => onLocate(pair.baseline_run_id, c.target)}>基准定位</button><button onClick={() => onLocate(pair.candidate_run_id, c.target)}>候选定位</button></> : '只读来源'}</td></tr>)}</tbody></table></div></details>
      </section>)}
      <h3>宿主计算的同口径指标</h3><div className="delivery-table-scroll"><table><thead><tr><th>指标与范围</th>{runIds.map(id => <th key={id}>{runName(id)}<JsonDetails label="查看运行记录来源" value={{ run_id: id }} /></th>)}<th>差值与限制</th></tr></thead><tbody>{value.metrics.map((m, n) => <tr key={`${m.metric}:${n}`}><td>{metricLabels[m.metric]}{m.object_ref && (onLocate ? <button onClick={() => onLocate(runIds[0], m.object_ref!)}>{m.object_ref.kind}:{m.object_ref.id}</button> : <span>{m.object_ref.kind}:{m.object_ref.id}</span>)}{m.acceptance_scope && <small>{scopeText(m.acceptance_scope)}</small>}</td>{runIds.map(id => <td key={id}><MetricReading value={m.values.find(v => v.run_id === id)?.value} /></td>)}<td>{m.pair_differences.map((d, i) => <div key={i}>{d.comparable ? differenceText(d.difference) : '不可比，不显示改善量'}{d.comparable && d.change_percent != null && <span> · 相对变化 {d.change_percent}%</span>}<IssueList issues={d.conditions} /></div>)}</td></tr>)}</tbody></table></div>
      {value.conclusions.map((s, n) => <p key={n}>{s}</p>)}<IssueList issues={value.limitations} />


 </section>;
}
