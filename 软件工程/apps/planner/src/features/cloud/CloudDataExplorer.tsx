import { useEffect, useMemo, useRef, useState } from 'react';
import type { DaaSQueryResult } from '../../../../../packages/commands/generated/typescript/DaaSQueryResult';
import { dataColumns, displayCell, minuteSeries, rowCells, rowKindText, sourceModeText, type CloudDataRow, type MinuteSeries } from './cloudDataPresentation';
import './cloudDataExplorer.css';

export interface CloudDataExplorerProps { result: DaaSQueryResult; pageNumber: number }
const colors = ['#286ca5', '#cf7725', '#38806b', '#795b91'];

function MinuteTrend({ series }: { series: MinuteSeries[] }) {
  const buckets = [...new Set(series.flatMap(s => s.points.map(p => p.bucket)))].sort();
  const max = Math.max(...series.flatMap(s => s.points.map(p => p.count)));
  const first = Date.parse(buckets[0]);
  const span = Math.max(60000, Date.parse(buckets.at(-1)!) - first);
  const x = (bucket: string) => 44 + (Date.parse(bucket) - first) * 492 / span;
  const y = (count: number) => 178 - count * 138 / Math.max(1, max);
  return <div className="cloud-data-trend">
    <p>当前页设备分钟计数 · 条。按设备和来源分别显示；分页外的数据不在此图内。</p>
    <svg role="img" aria-label="当前页设备分钟计数趋势" viewBox="0 0 580 230">
      <line x1="44" y1="178" x2="540" y2="178" stroke="#8395a6"/>
      <line x1="44" y1="40" x2="44" y2="178" stroke="#8395a6"/>
      <text x="10" y="182">0</text><text x="10" y="44">{max}</text>
      {series.map((s, index) => <g key={`${s.deviceId}:${s.sourceMode}`} stroke={colors[index % colors.length]} fill={colors[index % colors.length]}>
        {s.points.map((point, n) => <g key={`${point.bucket}:${n}`}>
          {n > 0 && Date.parse(point.bucket) - Date.parse(s.points[n - 1].bucket) === 60000 && <line x1={x(s.points[n - 1].bucket)} y1={y(s.points[n - 1].count)} x2={x(point.bucket)} y2={y(point.count)} strokeWidth="2"/>}
          <circle cx={x(point.bucket)} cy={y(point.count)} r="3"><title>{s.deviceId} · {sourceModeText(s.sourceMode)} · {point.bucket} · {point.count}条</title></circle>
        </g>)}
      </g>)}
      <text x="44" y="205">{buckets[0]}</text>
      {buckets.length > 1 && <text x="540" y="222" textAnchor="end">{buckets.at(-1)}</text>}
    </svg>
    <ul className="cloud-data-legend">{series.map((s, index) => <li key={`${s.deviceId}:${s.sourceMode}`}><span style={{ background: colors[index % colors.length] }}/>{s.deviceId} · {sourceModeText(s.sourceMode)}</li>)}</ul>
  </div>;
}

export function CloudDataExplorer({ result, pageNumber }: CloudDataExplorerProps) {
  const columns = useMemo(() => dataColumns(result.rows), [result]);
  const rows = useMemo(() => result.rows.map(rowCells), [result]);
  const series = useMemo(() => minuteSeries(result.rows), [result]);
  const [hidden, setHidden] = useState<ReadonlySet<string>>(new Set());
  const [mode, setMode] = useState<'table' | 'trend'>('table');
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => { if (scrollRef.current) scrollRef.current.scrollTop = 0; }, [result, pageNumber]);
  const visible = columns.filter(column => !hidden.has(column.path));
  const trendVisible = mode === 'trend' && series.length > 0;
  const source = result.dataset === 'telemetry' ? `数据前沿 F${result.selected_visible_frontier}` : `执行修订 R${result.selected_execution_revision}`;
  const toggle = (path: string, checked: boolean) => setHidden(previous => { const next = new Set(previous); if (checked) next.delete(path); else next.add(path); return next; });
  return <section className="cloud-data-explorer" aria-label="数据结果浏览器">
    <header><h4>数据表与字段说明</h4><p>{source} · 第{pageNumber}页 · 当前页{rows.length}行 · {result.next_cursor === null ? '已到末页' : '还有后续页'}</p></header>
    <div className="cloud-data-tools"><button type="button" aria-pressed={!trendVisible} onClick={() => setMode('table')}>数据表</button><button type="button" aria-pressed={trendVisible} disabled={!series.length} onClick={() => setMode('trend')}>分钟计数趋势</button><button type="button" onClick={() => setHidden(new Set())}>显示全部字段</button></div>
    <div className="cloud-data-layout"><div className="cloud-data-body">
      {rows.length === 0 ? <p role="status">当前固定筛选没有数据。筛选条件仍然保留。</p>
        : trendVisible ? <MinuteTrend series={series}/>
        : !visible.length ? <p role="status">请选择至少一个显示字段。</p>
        : <div className="cloud-data-scroll" ref={scrollRef}><table aria-label="当前页数据表"><thead><tr>{visible.map(column => <th key={column.path} scope="col">{column.label}<small>{column.path}</small>{column.unit !== '—' && <small>{column.unit}</small>}</th>)}</tr></thead><tbody>{rows.map((row, index) => <tr key={index}>{visible.map(column => <td key={column.path} title={displayCell(row[column.path])}>{column.path === 'kind' ? rowKindText(row.kind as CloudDataRow['kind']) : column.path.endsWith('source_mode') && typeof row[column.path] === 'string' ? sourceModeText(row[column.path] as string) : displayCell(row[column.path])}</td>)}</tr>)}</tbody></table></div>}
      <p className="cloud-data-scope">显示字段只改变本页呈现。查询条件、固定来源和全部筛选导出保持原选择。</p>
    </div><aside aria-label="字段含义与单位"><h4>字段含义与单位</h4><div className="cloud-data-fields">{columns.map(column => <label key={column.path}><input type="checkbox" aria-label={`显示字段 ${column.path}`} checked={!hidden.has(column.path)} onChange={event => toggle(column.path, event.target.checked)}/><span><strong>{column.label}</strong><code>{column.path}</code><small>单位：{column.unit}</small></span></label>)}</div></aside></div>
  </section>;
}
