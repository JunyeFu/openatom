import type {RunComparison} from '../../../../../packages/commands/generated/typescript/RunComparison';
import type {RunEvidence,DeliveryWorkspaceProps} from './types';
import {differenceText,metricLabels,metricText,scopeText} from './model';
import {IssueList,JsonDetails} from './Shared';
/** The actual comparison result, also used by the ordinary comparison panel. */
export function ComparisonResult({value,evidence,onLocate,embedded=false}:{value:RunComparison;evidence:readonly RunEvidence[];onLocate?:DeliveryWorkspaceProps['onLocate'];embedded?:boolean}){
 const runIds=value.run_ids;
 return <section className={embedded?'comparison-result':'delivery-panel'} aria-label="实际方案比较结果">
 {evidence.length>0&&<div className="delivery-run-pair">{evidence.map(e=><article key={e.run.id}><strong>{e.run.id}</strong><dl><dt>冻结工程</dt><dd>{e.project.id} r{e.project.revision}</dd><dt>冻结方案</dt><dd>{e.scenario.id} r{e.scenario.revision}</dd><dt>计算边界</dt><dd>{e.run.computed_until_ms ?? '未知'} ms · {e.run.whole_window_computed?'完整窗口':'仅已计算前缀'}</dd></dl><IssueList issues={e.run.limitations}/></article>)}</div>}

      {value.pairs.map(pair => <section key={`${pair.baseline_run_id}:${pair.candidate_run_id}`}><h3>可比性：{{ strict: '共同条件一致', controlled_changes: '有明确输入变化', side_by_side_only: '仅并列，不作改善结论' }[pair.status]}</h3>
        <ul>{pair.conditions.map(c => <li key={c.key}>{c.matches ? '一致' : '不一致'} · {c.key}<IssueList issues={c.issues} /></li>)}</ul>
        <h3>输入变化与来源</h3>{!pair.input_changes.length && <p>宿主没有列出输入变化。</p>}
        <div className="delivery-table-scroll"><table><thead><tr><th>对象／字段</th><th>基准输入</th><th>候选输入</th><th>来源与理由</th><th>定位</th></tr></thead><tbody>{pair.input_changes.map((c, n) => <tr key={`${c.target.id}:${c.path}:${n}`}><td>{c.target.kind}:{c.target.id}<br /><code>{c.path}</code><br />{c.kind}</td><td><JsonDetails label="展开原值" value={c.before ?? null} /></td><td><JsonDetails label="展开新值" value={c.after ?? null} /></td><td>{c.basis ?? '来源未知'}<br />{c.reason ?? '未声明理由'}</td><td>{onLocate ? <><button onClick={() => onLocate(pair.baseline_run_id, c.target)}>基准定位</button><button onClick={() => onLocate(pair.candidate_run_id, c.target)}>候选定位</button></> : '只读来源'}</td></tr>)}</tbody></table></div>
      </section>)}
      <h3>宿主计算的同口径指标</h3><div className="delivery-table-scroll"><table><thead><tr><th>指标与范围</th>{runIds.map(id => <th key={id}>{id}</th>)}<th>差值与限制</th></tr></thead><tbody>{value.metrics.map((m, n) => <tr key={`${m.metric}:${n}`}><td>{metricLabels[m.metric]}{m.object_ref && (onLocate ? <button onClick={() => onLocate(runIds[0], m.object_ref!)}>{m.object_ref.kind}:{m.object_ref.id}</button> : <span>{m.object_ref.kind}:{m.object_ref.id}</span>)}{m.acceptance_scope && <small>{scopeText(m.acceptance_scope)}</small>}</td>{runIds.map(id => <td key={id}>{m.values.find(v => v.run_id === id) ? metricText(m.values.find(v => v.run_id === id)!.value) : '未返回该Run指标'}</td>)}<td>{m.pair_differences.map((d, i) => <div key={i}>{d.comparable ? differenceText(d.difference) : '不可比，不显示改善量'}{d.comparable && d.change_percent != null && <span> · 相对变化 {d.change_percent}%</span>}<IssueList issues={d.conditions} /></div>)}</td></tr>)}</tbody></table></div>
      {value.conclusions.map((s, n) => <p key={n}>{s}</p>)}<IssueList issues={value.limitations} />


 </section>;
}
