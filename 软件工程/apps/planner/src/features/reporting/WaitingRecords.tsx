import { useMemo, useState } from 'react';
import type { WaitInterval } from '../../../../../packages/commands/generated/typescript/WaitInterval';
import type { DeliveryWorkspaceProps } from '../comparison/types';
import { JsonDetails } from '../comparison/Shared';

const reasonNames = { missing_material: '缺少物料', resource_busy: '资源占用', calendar: '日历限制', preparation: '准备条件', quality: '质量条件', output_capacity: '出料容量', receiver_capacity: '接收容量' };
const endNames = { resolved: '已解决', condition_changed: '条件变化', observation_end: '观察结束', computed_boundary: '已计算边界' };

export function WaitingRecords({ runId, waits, onLocate }: { runId: string; waits: readonly WaitInterval[]; onLocate?: DeliveryWorkspaceProps['onLocate'] }) {
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(0);
  const indexed = useMemo(() => waits.map((wait, index) => ({ wait, index, text: `${JSON.stringify(wait)} ${wait.reasons.map(reason => reasonNames[reason.kind]).join(' ')} ${endNames[wait.end_cause]}`.toLocaleLowerCase() })), [waits]);
  const filtered = useMemo(() => { const term = query.trim().toLocaleLowerCase(); return term ? indexed.filter(row => row.text.includes(term)) : indexed; }, [indexed, query]);
  const pages = Math.max(1, Math.ceil(filtered.length / 100));
  const currentPage = Math.min(page, pages - 1);
  return <section aria-label="等待记录明细">
    <label>筛选全部等待记录<input type="search" value={query} placeholder="对象、资源、等待原因或区间" onChange={event => { setQuery(event.target.value); setPage(0); }} /></label>
    <p role="status">共 {waits.length} 条，筛选后 {filtered.length} 条，每页100条。第 {currentPage + 1} / {pages} 页。</p>
    <button disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>上一页等待</button><button disabled={currentPage + 1 >= pages} onClick={() => setPage(currentPage + 1)}>下一页等待</button>
    {!filtered.length && <p>没有匹配的等待记录。请缩短关键词，或<button onClick={() => { setQuery(''); setPage(0); }}>清除筛选</button>查看全部记录。</p>}
    <div className="delivery-table-scroll"><table><caption>等待与对象定位 · 原始记录序号保留</caption><thead><tr><th>序号／对象</th><th>实际区间（ms）</th><th>原因与结束</th></tr></thead><tbody>{filtered.slice(currentPage * 100, currentPage * 100 + 100).map(({ wait: w, index }) => <tr key={index}><td>{index + 1} · {onLocate ? <button onClick={() => onLocate(runId, w.object_ref)}>{w.object_ref.kind}:{w.object_ref.id}</button> : `${w.object_ref.kind}:${w.object_ref.id}`}</td><td>{w.interval.start_ms}—{w.interval.end_ms}</td><td>{w.reasons.map(reason => reasonNames[reason.kind]).join('、')} · {endNames[w.end_cause]}<JsonDetails label="查看等待条件与来源" value={{ reasons: w.reasons, end_cause: w.end_cause, execution_ref: w.execution_ref, phase_ref: w.phase_ref }} /></td></tr>)}</tbody></table></div>
  </section>;
}
