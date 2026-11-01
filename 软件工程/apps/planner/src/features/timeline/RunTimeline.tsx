import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties, PointerEvent as Pointer } from 'react';
import type { Props, Primitive, TimeWindow, TimelineInputCommit, TimelineSelection, InputField, TimelineMode } from './types';
import { clipInterval, densityBuckets, intervalDensity, gridTicks, pointVisible, snapTime, timeText, timeX, xTime, zoomWindow } from './time';
import { fieldKey, fieldTime, inputProposal } from './input';
import { actualProjection, deviceDetailProjection, calendarInputProjection, comparisonConditions, comparisonInputs, displayGroups, frozenProjectMatches, inputProjection, itemEvents, observedItems, quantityText, quantityTotals, summarizeGroup, uniqueEvents, executionDevice, resourceUsage, holderObject, waitContext, eventNames } from './projection';
import type { Track } from './projection';
import { jobPredecessors } from './projection';
import { preferenceKey, useViewPreferences } from './preferences';
import {domPainted} from '../render-surface/visualCapture';
import './timeline.css';

export function RunTimeline(props: Props) {
  return <Timeline key={`${props.project?.id ?? props.draftProject?.id ?? 'none'}:${props.run?.id ?? 'draft'}:${props.scenario?.id ?? ''}:${props.comparison?.run.id ?? ''}`} {...props} />;
}
interface Detail { id: string; title: string; devices: string[]; window: TimeWindow; filter: string; scroll: number }
interface Preview { proposal: TimelineInputCommit; snap: { label: string; at_ms: number } | null; error: string }
type InputDrag = { pointer: number; x: number; at: number; fields: InputField[]; project: NonNullable<Props['project']>; scenario: NonNullable<Props['scenario']>; target: HTMLElement; preview: Preview | null };
function Timeline(props: Props) {
  const { project, draftProject, run, scenario, runScenario, atMs, comparison, onSelect } = props;
  const { value: prefs, update: save, storageError } = useViewPreferences(props, preferenceKey(project?.id ?? draftProject?.id ?? 'none', run?.id ?? null));
  const [mode, setMode] = useState<TimelineMode>(run ? 'actual' : 'input'), [filter, setFilter] = useState('');
  const [expanded, setExpanded] = useState(() => new Set((project ? displayGroups(project, prefs) : []).filter(g => !prefs.collapsed_group_ids.includes(g.id)).map(g => g.id)));
  const [resourcesOpen, setResourcesOpen] = useState(false), [groupsOpen, setGroupsOpen] = useState(false), [itemsOpen, setItemsOpen] = useState(false);
  const [checked, setChecked] = useState<string[]>([]), [groupName, setGroupName] = useState(''), [groupTarget, setGroupTarget] = useState('new');
  const [details, setDetails] = useState<Detail[]>([]), [selectedRow, setSelectedRow] = useState(''), [range, setRange] = useState<TimeWindow | null>(null);
  const [localSelection, setLocalSelection] = useState<TimelineSelection | null>(null), [notice, setNotice] = useState(''), [ghost, setGhost] = useState<number | null>(null);
  const [reference, setReference] = useState<TimelineSelection | null>(null);
  const [seekInput, setSeekInput] = useState(String(atMs ?? run?.computed_until_ms ?? 0)), [playing, setPlaying] = useState(false), [speed, setSpeed] = useState(1);
  const [fields, setFields] = useState<InputField[]>([]), [numeric, setNumeric] = useState(''), [preview, setPreview] = useState<Preview | null>(null), [committing, setCommitting] = useState(false);
  const [itemFilter, setItemFilter] = useState(''), [itemId, setItemId] = useState(''), [dense, setDense] = useState<Primitive[]>([]);
  const [scroll, setScroll] = useState(prefs.row_scroll), [width, setWidth] = useState(700), [height, setHeight] = useState(100);
  const viewport = useRef<HTMLDivElement>(null), ruler = useRef<HTMLDivElement>(null), inputDrag = useRef<InputDrag | null>(null);
  const itemsToggle = useRef<HTMLButtonElement>(null);
  function closeItems() { setItemsOpen(false); itemsToggle.current?.focus(); }
  const rangeDrag = useRef<{ pointer: number; at: number; x: number; target: HTMLElement } | null>(null);
  const seekDrag = useRef<number | null>(null), pending = useRef<{ run_id: string; at_ms: number } | null>(null);
  const playbackSeek = useRef(seek); playbackSeek.current = seek;
  const selected = props.selected === undefined ? localSelection : props.selected;
  const viewProject = mode === 'input' ? draftProject ?? project : project, frontier = run?.computed_until_ms ?? null;
  const domain = useMemo(() => {
    const w = mode === 'input' ? scenario?.window : run?.window;
    return { start_ms: w?.start_ms ?? 0, end_ms: Math.max((w?.start_ms ?? 0) + 1, w?.end_ms ?? 1) };
  }, [mode, scenario?.window, run?.window]);
  const axisId = details.at(-1)?.id ?? (mode === 'input' ? 'input:main' : 'main'), rememberedWindow = prefs.axis_windows[axisId] ?? domain;
  const window = zoomWindow(rememberedWindow, rememberedWindow.start_ms, 1, domain);
  const x = (t: number) => timeX(t, window, width);
  const groups = useMemo(() => viewProject ? displayGroups(viewProject, prefs) : [], [viewProject, prefs.group_overrides, prefs.independent_device_ids, prefs.row_order]);
  const missingViewIds = useMemo(() => {
    if (!viewProject) return [];
    const devices = new Set(viewProject.devices.map(d => d.id)), resources = new Set(viewProject.resources.map(r => r.id));
    return [...new Set([...prefs.group_overrides.flatMap(g => g.members), ...prefs.independent_device_ids].filter(id => !devices.has(id)).concat(prefs.pinned_resource_ids.filter(id => !resources.has(id))))];
  }, [viewProject, prefs.group_overrides, prefs.independent_device_ids, prefs.pinned_resource_ids]);
  const actual = useMemo(() => project && run ? actualProjection(project, run) : [], [project, run]);
  const inputs = useMemo(() => scenario ? inputProjection(scenario) : [], [scenario]);
  const calendarInputs = useMemo(() => scenario && viewProject ? calendarInputProjection(viewProject, scenario) : [], [viewProject, scenario]);
  const oldInputs = useMemo(() => runScenario ? inputProjection(runScenario) : [], [runScenario]);
  const other = useMemo(() => comparison ? actualProjection(comparison.project, comparison.run) : [], [comparison]);
  const events = useMemo(() => run ? uniqueEvents(run) : [], [run]);
  const items = useMemo(() => observedItems(props.observation, run, atMs), [props.observation, run, atMs]);
  const item = items?.find(i => i.id === itemId), history = useMemo(() => itemId ? itemEvents(events, itemId) : [], [events, itemId]);
  const before = [...history].reverse().find(e => e.at_ms <= (atMs ?? 0)), after = history.find(e => e.sequence > (before?.sequence ?? -1));
  const comparisonState = useMemo(() => comparisonConditions(runScenario, comparison?.scenario, project, comparison?.project), [runScenario, comparison?.scenario, project, comparison?.project]);
  const inputDifferences = useMemo(() => comparisonInputs(runScenario, comparison?.scenario), [runScenario, comparison?.scenario]);
  const inspectedDevice = details.at(-1)?.devices.length === 1 ? details.at(-1)!.devices[0] : null;
  const inspected = useMemo(() => project && run && inspectedDevice ? deviceDetailProjection(project, run, actual, inspectedDevice, window) : null, [project, run, actual, inspectedDevice, window]);
  const inspectedOther = useMemo(() => comparison && inspectedDevice ? deviceDetailProjection(comparison.project, comparison.run, other, inspectedDevice, window) : null, [comparison, other, inspectedDevice, window]);
  const primitives = mode === 'input' ? [...inputs, ...calendarInputs] : inspected?.primitives ?? actual;
  const otherPrimitives = inspectedOther?.primitives ?? other;
  const byRow = useMemo(() => {
    const map = new Map<string, Primitive[]>(); primitives.forEach(p => { const values = map.get(p.row) ?? []; values.push(p); map.set(p.row, values); }); return map;
  }, [primitives]);
  const otherByRow = useMemo(() => { const map = new Map<string, Primitive[]>(); otherPrimitives.forEach(p => { const values = map.get(p.row) ?? []; values.push(p); map.set(p.row, values); }); return map; }, [otherPrimitives]);
  const summaries = useMemo(() => new Map(groups.map(g => [g.id, summarizeGroup(actual, g.devices)])), [groups, actual]);
  const otherSummaries = useMemo(() => new Map(groups.map(g => [g.id, summarizeGroup(other, g.devices)])), [groups, other]);
  const baseRows = useMemo<Track[]>(() => {
    if (mode === 'input') return [...['arrival', 'task', 'order'].map((id, i) => ({ id: `input:${id}`, title: ['到料输入', '任务释放', '订单释放 / 交期'][i], kind: 'input' as const, devices: [], depth: 0 })),
      ...(viewProject?.calendars ?? []).map(c => ({ id: `input:calendar:${c.id}`, title: `班次开放 · ${c.title}`, kind: 'input' as const, devices: [], depth: 0, object: { kind: 'calendar' as const, id: c.id } }))];
    const rows: Track[] = [], focused = details.at(-1)?.devices;
    for (const hall of viewProject?.halls ?? []) {
      const children: Track[] = [];
      for (const group of groups.filter(g => g.hall_id === hall.id)) {
        const members = focused ? group.devices.filter(id => focused.includes(id)) : group.devices;
        if (!members.length) continue;
        const matches = members.filter(id => `${viewProject?.devices.find(d => d.id === id)?.title} ${id}`.toLowerCase().includes(filter.toLowerCase()));
        if (filter && !matches.length && !group.title.toLowerCase().includes(filter.toLowerCase())) continue;
        children.push({ ...group, title: `${group.title} (${group.devices.length})${filter ? ` · 命中 ${matches.length}` : ''}` });
        if (expanded.has(group.id) || focused) members.filter(id => !filter || matches.includes(id)).forEach(id => children.push({ id: `device:${id}`, title: viewProject!.devices.find(d => d.id === id)!.title, kind: 'device', devices: [id], depth: 2, object: { kind: 'device', id } }));
      }
      viewProject!.devices.filter(d => d.hall_id === hall.id && prefs.independent_device_ids.includes(d.id) && (!focused || focused.includes(d.id))).forEach(d => {
        if (`${d.title} ${d.id}`.toLowerCase().includes(filter.toLowerCase())) children.push({ id: `device:${d.id}`, title: d.title, kind: 'device', devices: [d.id], depth: 1, object: { kind: 'device', id: d.id } });
      });
      if (children.length) rows.push({ id: `hall:${hall.id}`, title: hall.title, kind: 'hall', devices: [], depth: 0, object: { kind: 'hall', id: hall.id } }, ...children);
    }
    if (mode === 'comparison' && comparison) {
      const missing = comparison.project.devices.filter(d => !project?.devices.some(a => a.id === d.id));
      if (missing.length) rows.push({ id: 'hall:comparison-only', title: 'B 独有设备 · 并列展示', kind: 'hall', devices: [], depth: 0 });
      missing.forEach(d => rows.push({ id: `device:${d.id}`, title: `B · ${d.title}`, kind: 'device', devices: [d.id], depth: 1, object: { kind: 'device', id: d.id } }));
    }
    const resourceIds = new Set([...(project?.resources.map(r => r.id) ?? []), ...(run?.resource_claims.map(c => c.resource_id) ?? []), ...(mode === 'comparison' ? comparison?.run.resource_claims.map(c => c.resource_id) ?? [] : [])]);
    for (const id of resourceIds) if (resourcesOpen || prefs.pinned_resource_ids.includes(id)) {
      const r = project?.resources.find(r => r.id === id) ?? comparison?.project.resources.find(r => r.id === id);
      rows.push({ id: `resource:${id}`, title: r ? `${r.title} · ${r.role} · ${quantityText(r.capacity)}` : `${id} · 冻结资源未给出`, kind: 'resource', devices: [], depth: 0, object: { kind: 'resource', id } });
    }
    for (const [id, title] of [['arrivals', '实际到料'], ['acceptance', '实际接受'], ['unbound', '其他作业 / 未绑定等待'], ['all', '全部实际事件']]) rows.push({ id: `events:${id}`, title, kind: 'events', devices: [], depth: 0 });
    if (inspectedDevice) {
      const index = rows.findIndex(r => r.id === `device:${inspectedDevice}`);
      if (index >= 0) rows.splice(index + 1, 0, ...(inspected?.rows ?? []), ...(mode === 'comparison' ? inspectedOther?.rows ?? [] : []));
    }
    return rows;
  }, [inspectedDevice, inspected, inspectedOther, mode, details, viewProject, groups, filter, expanded, prefs.independent_device_ids, prefs.pinned_resource_ids, project, run, comparison, resourcesOpen]);
  const rows = useMemo(() => { let top = 0; return baseRows.map(r => { const h = r.kind === 'group' ? (mode === 'comparison' ? 80 : 40) : r.kind === 'hall' ? 28 : mode === 'comparison' ? 64 : 32, row = { ...r, top, height: h }; top += h; return row; }); }, [baseRows, mode]);
  const selectedTrack = baseRows.find(r => r.id === selectedRow), selectedDevices = selectedTrack?.devices ?? [];
  const visibleItems = useMemo(() => (items ?? []).filter(i => `${i.id} ${i.item_type_ref.id} ${i.holder.id}`.toLowerCase().includes(itemFilter.toLowerCase())), [items, itemFilter]);
  const totals = useMemo(() => items ? quantityTotals(items) : [], [items]);
  const waiting = useMemo(() => mode === 'input' ? null : waitContext(selected, run, atMs, items), [mode, selected, run, atMs, items]);
  const usage = useMemo(() => project && run ? resourceUsage(project, run, prefs.pinned_resource_ids, { start_ms: window.start_ms, end_ms: Math.min(window.end_ms, frontier ?? window.start_ms) }) : [], [project, run, prefs.pinned_resource_ids, window, frontier]);
  const links = useMemo(() => {
    if (!selected) return [];
    const links = selected.related_objects.map(object => ({ label: `${object.kind} ${object.id}`, object }));
    const job = selected.job;
    const owner = selected.run_id === run?.id ? { project, run, scenario: runScenario } : selected.run_id === comparison?.run.id ? comparison : null;
    if (job && owner?.project && owner.run && frozenProjectMatches(owner.project, owner.run)) {
      const sourceProject = owner.project, sourceRun = owner.run;
      sourceRun.resource_claims.filter(c => c.job_id === job.id).forEach(c => links.push({ label: `实际资源 ${c.resource_id}`, object: { kind: 'resource', id: c.resource_id } }));
      if (job.execution_ref.kind === 'project_device') {
        const predecessors = jobPredecessors(sourceProject, owner.scenario, sourceRun, job);
        predecessors.jobs.forEach(p => links.push({ label: `实际前置 ${p.id}`, object: { kind: 'job', id: p.id } }));
        predecessors.declared_steps.forEach(id => links.push({ label: `声明前置 ${id} · 路线 ${predecessors.route!.id}`, object: { kind: 'route', id: predecessors.route!.id } }));
        sourceProject.connections.filter(c => c.kind === 'material'
          ? [c.data.source, c.data.destination].some(e => e.kind === 'port' && e.data.instance_id === executionDevice(job))
          : [c.data.source, c.data.destination].some(e => e.instance_id === executionDevice(job)))
          .forEach(c => links.push({ label: `${c.kind === 'material' ? '物料' : c.kind === 'signal' ? '逻辑' : '产物'}连接 ${c.data.id}`, object: { kind: 'connection', id: c.data.id } }));
      }
    }
    return [...new Map(links.map(l => [`${l.object.kind}:${l.object.id}`, l])).values()];
  }, [selected, project, run, runScenario, comparison]);
  useLayoutEffect(() => {
    const measure = () => { if (ruler.current) setWidth(Math.max(1, ruler.current.clientWidth)); if (viewport.current) setHeight(viewport.current.clientHeight); };
    const observer = new ResizeObserver(measure); if (ruler.current) observer.observe(ruler.current); if (viewport.current) observer.observe(viewport.current); measure();
    if (viewport.current) {
      viewport.current.scrollTop = prefs.row_scroll;
      setScroll(viewport.current.scrollTop);
    }
    return () => observer.disconnect();
  }, []);
  useEffect(() => { setSeekInput(String(atMs ?? frontier ?? 0));
    if (pending.current && pending.current.run_id === run?.id && pending.current.at_ms === atMs && (!props.observation || props.observation.run_id === run!.id && props.observation.at_ms === atMs && props.observation.next_index === null)) { pending.current = null; setGhost(null); }
  }, [atMs, frontier, run?.id, props.observation]);
  useEffect(() => {
    if (props.readonly || !playing || !run || frontier == null) return;
    // Result-view time shares one wall-clock origin; IPC delay must not slow the selected rate.
    const started = performance.now(), origin = atMs ?? run.window.start_ms;
    const timer = globalThis.setInterval(() => {
      if (pending.current) return;
      const t = Math.min(frontier, Math.round(origin + (performance.now() - started) * speed));
      if (t >= frontier) setPlaying(false);
      playbackSeek.current(t, run.id, true);
    }, 100);
    return () => globalThis.clearInterval(timer);
  }, [playing, run?.id, frontier, speed, props.readonly]);
  useEffect(() => {
    if(props.readonly)return;
    const element = viewport.current; if (!element) return;
    const listener = (event: WheelEvent) => wheel(event);
    element.addEventListener('wheel', listener, { passive: false });
    return () => element.removeEventListener('wheel', listener);
  }, [window, domain, width, prefs]);
  useEffect(() => { if (scroll === prefs.row_scroll) return; const timer = globalThis.setTimeout(() => save({ ...prefs, row_scroll: scroll }), 120); return () => globalThis.clearTimeout(timer); }, [scroll, prefs]);
  function togglePlayback() {
    if (!playing) { setPlaying(true); return; }
    setPlaying(false);
    // Supersede an in-flight view read with the currently displayed time.
    if (pending.current && atMs !== null) seek(atMs);
  }
  function setWindow(value: TimeWindow) { save({ ...prefs, axis_windows: { ...prefs.axis_windows, [axisId]: value } }); }
  function pointerTime(clientX: number) { const rect = ruler.current!.getBoundingClientRect(); return xTime(clientX, window, rect.width, rect.left); }
  function seek(at: number, runId = run?.id, playback = false) {
    if(props.readonly)return;
    const r = runId === run?.id ? run : comparison?.run;
    if (!r || !runId || !Number.isSafeInteger(at)) return;
    if (r.computed_until_ms == null || at < r.window.start_ms || at > r.computed_until_ms) { setNotice('该时点尚未计算；库存与生产状态保持未知。'); setGhost(null); return; }
    if (runId !== run?.id && !props.onSeek) { setNotice('另一个结果的时点查询尚未接入。'); return; }
    if (!playback) setPlaying(false); setGhost(at); pending.current = { run_id: runId, at_ms: at };
    if (props.onSeek) props.onSeek({ run_id: runId, at_ms: at }); else props.onTime(at);
    if (at === atMs && runId === run?.id) { pending.current = null; setGhost(null); }
  }
  function select(value: TimelineSelection) {
    setLocalSelection(value); props.onInspect?.(value); onSelect(value.object);
    if (value.source === 'actual') setReference(value);
    if (value.wait) { const ids = value.wait.reasons.flatMap(r => r.blocking_object?.kind === 'resource' ? [r.blocking_object.id] : []); if (ids.length) save({ ...prefs, pinned_resource_ids: [...new Set([...prefs.pinned_resource_ids, ...ids])] }); }
  }
  function inspect(p: Primitive) { select(p.selection); if (p.source === 'actual') seek(p.point_ms ?? p.interval?.start_ms ?? atMs ?? 0, p.run_id ?? undefined); }
  function followRelation(object: TimelineSelection['object']) {
    const owner = selected?.run_id === run?.id ? actual : selected?.run_id === comparison?.run.id ? other : [];
    const job = object.kind === 'job' ? owner.find(p => p.kind === 'job' && p.object.id === object.id) : undefined;
    if (job) { inspect(job); return; }
    setLocalSelection(null); props.onInspect?.(null); onSelect(object);
  }
  function eventSelect(sequence: number) { const p = actual.find(p => p.selection.event?.sequence === sequence); if (p) inspect(p); }
  function toggleGroup(id: string) { const next = new Set(expanded); next.has(id) ? next.delete(id) : next.add(id); setExpanded(next); save({ ...prefs, collapsed_group_ids: groups.filter(g => !next.has(g.id)).map(g => g.id) }); }
  function cancel() {
    const drags = [inputDrag.current, rangeDrag.current], pointer = seekDrag.current;
    inputDrag.current = null; rangeDrag.current = null; seekDrag.current = null;
    for (const d of drags) if (d?.target.hasPointerCapture(d.pointer)) d.target.releasePointerCapture(d.pointer);
    if (pointer !== null && ruler.current?.hasPointerCapture(pointer)) ruler.current.releasePointerCapture(pointer);
    setPreview(null); setRange(null); setGhost(null); setNotice('');
  }
  function changeMode(value: TimelineMode) {
    cancel(); setPlaying(false); setItemsOpen(false); setDense([]);
    setMode(value); setRange(null); setDetails([]);
  }
  function beginInput(e: Pointer<HTMLButtonElement>, p: Primitive) {
    if (e.button !== 0 || mode !== 'input' || !scenario || !viewProject || !p.input_field || committing) return;
    e.preventDefault(); e.stopPropagation(); e.currentTarget.focus({ preventScroll: true });
    if (e.ctrlKey) return; // Ctrl-click selection is handled once by onClick.
    const f = p.input_field, next = fields.some(v => fieldKey(v) === fieldKey(f)) ? [f, ...fields.filter(v => fieldKey(v) !== fieldKey(f))] : [f];
    setFields(next); setNumeric(String(p.point_ms)); select(p.selection); e.currentTarget.setPointerCapture(e.pointerId);
    inputDrag.current = { pointer: e.pointerId, x: e.clientX, at: p.point_ms!, fields: next, project: viewProject, scenario, target: e.currentTarget, preview: null };
  }
  function moveInput(e: Pointer<HTMLButtonElement>) {
    const d = inputDrag.current; if (!d || d.pointer !== e.pointerId) return;
    const visibleRowIds = new Set(rows.filter(row => row.top + row.height > scroll && row.top < scroll + height).map(row => row.id));
    const targets = [...gridTicks(window, width).map(at_ms => ({ id: `grid:${at_ms}`, at_ms, label: timeText(at_ms), kind: 'grid' as const })),
      ...inputs.filter(p => visibleRowIds.has(p.row) && p.input_field && !d.fields.some(f => fieldKey(f) === fieldKey(p.input_field!))).map(p => ({ id: p.key, at_ms: p.point_ms!, label: p.label, kind: 'business' as const })),
      ...calendarInputs.filter(p => visibleRowIds.has(p.row)).flatMap(p => [p.interval!.start_ms, p.interval!.end_ms].map((at_ms, i) => ({ id: `${p.key}:${i}`, at_ms, label: `${p.label} · ${i ? '班次结束' : '班次开始'}`, kind: 'business' as const }))),
      ...(reference?.at_ms != null ? [{ id: reference.key, at_ms: reference.at_ms, label: reference.label, kind: 'reference' as const }] : []),
      ...(reference?.true_interval ? [reference.true_interval.start_ms, reference.true_interval.end_ms].map(at_ms => ({ id: `${reference.key}:${at_ms}`, at_ms, label: reference.label, kind: 'reference' as const })) : [])];
    const rect = ruler.current!.getBoundingClientRect();
    const snapped = snapTime(d.at + (e.clientX - d.x) * (window.end_ms - window.start_ms) / rect.width, window, rect.width, targets, prefs.snap_enabled && !e.altKey);
    try { d.preview = { proposal: inputProposal(d.project, d.scenario, d.fields, snapped.at_ms - d.at), snap: snapped.target, error: '' }; setPreview(d.preview); setNumeric(String(snapped.at_ms)); }
    catch (err) { d.preview = null; setPreview(null); setNotice((err as Error).message); }
  }
  function endInput(e: Pointer<HTMLButtonElement>) {
    const d = inputDrag.current; if (d?.pointer !== e.pointerId) return;
    if (d.preview || e.clientX !== d.x) moveInput(e);
    inputDrag.current = null;
    if (d.target.hasPointerCapture(e.pointerId)) d.target.releasePointerCapture(e.pointerId);
    if (d.preview?.proposal.changes.some(c => c.after_ms !== c.before_ms)) void commit(d.preview);
  }
  function lostInput(e: Pointer<HTMLButtonElement>) {
    if (inputDrag.current?.pointer === e.pointerId && inputDrag.current.target === e.currentTarget) cancel();
  }
  function numericPreview() {
    if (!scenario || !viewProject || !fields.length) return;
    const first = fieldTime(scenario, fields[0]), target = Number(numeric);
    if (first == null || !Number.isSafeInteger(target)) { setNotice('请输入整数毫秒并选择输入。'); return; }
    try { setPreview({ proposal: inputProposal(viewProject, scenario, fields, target - first), snap: null, error: '' }); setNotice(''); } catch (e) { setNotice((e as Error).message); }
  }
  async function commit(candidate = preview) {
    if (!candidate || !props.onCommitInput || committing) return; setCommitting(true);
    try { const r = await props.onCommitInput(candidate.proposal); if (r.ok) { setPreview(null); setNotice(`已应用方案 r${r.scenario.revision}；历史结果保持冻结。`); } else setPreview({ ...candidate, error: `${r.error.code}：${r.error.message}` }); }
    catch (e) { setPreview({ ...candidate, error: (e as Error).message }); } finally { setCommitting(false); }
  }
  function startRange(e: Pointer<HTMLDivElement>) {
    if (e.target !== e.currentTarget || e.button !== 0 || mode === 'input') return;
    const at = Math.max(domain.start_ms, Math.min(domain.end_ms, pointerTime(e.clientX))); e.currentTarget.setPointerCapture(e.pointerId); rangeDrag.current = { pointer: e.pointerId, at, x: e.clientX, target: e.currentTarget }; setRange(null);
  }
  function moveRange(e: Pointer<HTMLDivElement>) { const d = rangeDrag.current; if (d?.pointer === e.pointerId) { const t = Math.max(domain.start_ms, Math.min(domain.end_ms, pointerTime(e.clientX))); setRange({ start_ms: Math.min(t, d.at), end_ms: Math.max(t, d.at) }); } }
  function endRange(e: Pointer<HTMLDivElement>) { const d = rangeDrag.current; if (d?.pointer !== e.pointerId) return; rangeDrag.current = null; if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId); if (Math.abs(d.x - e.clientX) < 3) { setRange(null); seek(d.at); } }
  function detail() { if (!range || range.end_ms <= range.start_ms || !selectedDevices.length) return; const id = `detail:${selectedRow}:${range.start_ms}:${range.end_ms}`; setDetails([...details, { id, title: selectedTrack!.title, devices: selectedDevices, window, filter, scroll }]); save({ ...prefs, axis_windows: { ...prefs.axis_windows, [id]: range } }); setFilter(''); setRange(null); if (viewport.current) viewport.current.scrollTop = 0; }
  function back() { const d = details.at(-1); if (!d) return; setDetails(details.slice(0, -1)); setFilter(d.filter); setRange(null); save({ ...prefs, axis_windows: { ...prefs.axis_windows, [details.at(-2)?.id ?? 'main']: d.window }, row_scroll: d.scroll }); if (viewport.current) viewport.current.scrollTop = d.scroll; }
  function wheel(e: WheelEvent) { if (!e.ctrlKey && !e.shiftKey) return; e.preventDefault(); if (e.ctrlKey) setWindow(zoomWindow(window, pointerTime(e.clientX), Math.exp(e.deltaY * .002), domain)); else { const span = window.end_ms - window.start_ms, start_ms = Math.max(domain.start_ms, Math.min(domain.end_ms - span, window.start_ms + Math.round((e.deltaY || e.deltaX) * span / width))); setWindow({ start_ms, end_ms: start_ms + span }); } }
  function groupAction(action: 'save' | 'remove' | 'independent' | 'auto' | 'dissolve') {
    if (!viewProject) return; const ids = new Set(checked); let overrides = prefs.group_overrides.map(g => ({ ...g, members: [...g.members] }));
    if (action === 'save' && !checked.length && groupTarget !== 'new' && groupName.trim()) { save({ ...prefs, group_overrides: overrides.map(g => g.id === groupTarget ? { ...g, title: groupName.trim() } : g) }); return; }
    if (action === 'dissolve') overrides = overrides.filter(g => g.id !== groupTarget);
    else { overrides = overrides.map(g => ({ ...g, members: g.members.filter(id => !ids.has(id)) }));
      if (action === 'save') { if (!groupName.trim() || !checked.length) { setNotice('请填写显示组名并选择设备。'); return; }
        for (const hall of viewProject.halls) { const members = checked.filter(id => viewProject.devices.some(d => d.id === id && d.hall_id === hall.id)); if (!members.length) continue; const existing = overrides.find(g => g.id === groupTarget && g.hall_id === hall.id); if (existing) { existing.title = groupName.trim(); existing.members.push(...members); } else overrides.push({ id: `manual:${hall.id}:${Date.now()}`, hall_id: hall.id, title: groupName.trim(), members }); }
      }
      overrides = overrides.filter(g => g.members.length);
    }
    let independent = prefs.independent_device_ids;
    if (action === 'independent') independent = [...new Set([...independent, ...checked])];
    if (action === 'auto' || action === 'save') independent = independent.filter(id => !ids.has(id));
    save({ ...prefs, group_overrides: overrides, independent_device_ids: independent });
  }
  function reorder(id: string, delta: number) { const order = groups.map(g => g.id), i = order.indexOf(id), j = i + delta; if (j >= 0 && j < order.length) [order[i], order[j]] = [order[j], order[i]]; save({ ...prefs, row_order: order }); }
  const totalHeight = rows.at(-1) ? rows.at(-1)!.top + rows.at(-1)!.height : 0;
  const visibleRows = rows.filter(r => r.top + r.height >= scroll - 64 && r.top <= scroll + height + 64);
  useEffect(()=>{
    if(!props.readonly||!props.onFrameReady)return;let live=true;
    domPainted(()=>live).then(done=>{
      if(!done||!viewport.current)return;const outer=viewport.current.getBoundingClientRect();
      const objects=new Map<string,NonNullable<Track['object']>>();
      for(const node of viewport.current.querySelectorAll<HTMLElement>('[data-render-object]')){const r=node.getBoundingClientRect();if(r.width<=0||r.height<=0||r.bottom<=Math.max(outer.top,0)||r.top>=Math.min(outer.bottom,innerHeight)||r.right<=outer.left||r.left>=outer.right)continue;const ref=JSON.parse(node.dataset.renderObject!) as NonNullable<Track['object']>;objects.set(`${ref.kind}:${ref.id}`,ref);}
      props.onFrameReady!({visible_objects:[...objects.values()],limitations:['二维时间轴不适用三维相机；可见对象只取当前虚拟窗口内实际DOM轨道/标记。']});
    });return()=>{live=false;};
  },[props.readonly,props.onFrameReady,rows,width,height,scroll,run,atMs]);
  // JSX lives below; all interaction state above is private UI state.
  return <section className="timeline sf-timeline" hidden={props.hidden} style={props.hidden ? { display: 'none' } : undefined} aria-label="全厂时间轴" tabIndex={0} data-run-id={run?.id ?? ''} data-view-time={atMs ?? ''} data-window-start={window.start_ms} data-window-end={window.end_ms}
      onKeyDown={e => { if(props.readonly)return;if (e.key === 'Escape') { if (preview || inputDrag.current || rangeDrag.current || seekDrag.current !== null) cancel(); else if (dense.length) setDense([]); else if (itemsOpen) { closeItems(); e.preventDefault(); e.stopPropagation(); } else if (groupsOpen) setGroupsOpen(false); else back(); }
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement || e.target instanceof HTMLSelectElement) return;
      const i = baseRows.findIndex(r => r.id === selectedRow); if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); const row = rows[Math.max(0, Math.min(rows.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))]; if (row) { setSelectedRow(row.id); viewport.current?.scrollTo({ top: row.top }); } }
      if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && selectedTrack?.kind === 'group' && expanded.has(selectedRow) !== (e.key === 'ArrowRight')) toggleGroup(selectedRow);
    }}>
    <fieldset disabled={props.readonly} style={{display:'contents'}}><div className="sf-tl-toolbar">{details.length ? <button aria-label="返回主轴" onClick={back}>← {details.at(-1)!.title}</button> : <strong>全厂主轴</strong>}
      <div role="group" aria-label="时间轴模式">{(['input', 'actual', 'comparison'] as const).map((v, i) => <button key={v} aria-pressed={mode === v} disabled={v !== 'input' && !run || v === 'comparison' && !comparison} onClick={() => changeMode(v)}>{['输入', '实际', '对比'][i]}</button>)}</div>
      <input aria-label="筛选轨道" placeholder="筛选设备 / 组" value={filter} onChange={e => setFilter(e.target.value)} /><button aria-pressed={resourcesOpen} onClick={() => setResourcesOpen(!resourcesOpen)}>共享资源</button><button aria-expanded={groupsOpen} onClick={() => setGroupsOpen(!groupsOpen)}>显示编组</button><button ref={itemsToggle} aria-expanded={itemsOpen} disabled={!run || mode === 'input'} onClick={() => setItemsOpen(!itemsOpen)}>物品跟随</button><button disabled={!range || !selectedDevices.length || range.end_ms <= range.start_ms} onClick={detail}>细化选段</button><span className="sf-tl-spacer" /><details className="sf-tl-source"><summary>{mode === 'input' ? '方案输入' : run ? '运行记录来源' : '尚未运行'}</summary><p>方案版本：{scenario?.revision ?? '—'}</p><code>{run?.id}</code></details>
    </div>
    {mode === 'comparison' && <div className="sf-tl-banner" data-comparable={comparisonState.comparable}>基准与对比记录 · {comparisonState.comparable ? `共同条件一致；${inputDifferences.join('；') || '时间输入无差异'}` : `仅并列展示：${comparisonState.differences.join('、')}`}<details><summary>比较来源详情</summary><p>基准：{run?.id}</p><p>对比：{comparison?.run.id}</p></details><button onClick={() => comparison && seek(comparison.run.computed_until_ms ?? comparison.run.window.start_ms, comparison.run.id)}>读取对比时点</button></div>}
    {!frozenProjectMatches(project, run) && <div role="alert" className="sf-tl-banner">冻结工程与结果修订不同，实际轨道暂不解释。</div>}
    {missingViewIds.length > 0 && <div className="sf-tl-banner">视图有 {missingViewIds.length} 个失效引用：{missingViewIds.join("、")}<button onClick={() => save({ ...prefs, group_overrides: prefs.group_overrides.map(g => ({ ...g, members: g.members.filter(id => !missingViewIds.includes(id)) })).filter(g => g.members.length), independent_device_ids: prefs.independent_device_ids.filter(id => !missingViewIds.includes(id)), pinned_resource_ids: prefs.pinned_resource_ids.filter(id => !missingViewIds.includes(id)) })}>清除失效引用</button></div>}
    {groupsOpen && <div className="sf-tl-flyout" aria-label="显示编组编辑"><strong>显示编组 · 本地视图</strong><button onClick={() => setGroupsOpen(false)}>关闭</button><div className="sf-tl-device-options">{viewProject?.devices.map(d => <label key={d.id}><input type="checkbox" checked={checked.includes(d.id)} onChange={e => setChecked(e.target.checked ? [...checked, d.id] : checked.filter(id => id !== d.id))} />{d.title}</label>)}</div><input aria-label="显示组名称" value={groupName} onChange={e => setGroupName(e.target.value)} /><select aria-label="目标显示组" value={groupTarget} onChange={e => { setGroupTarget(e.target.value); setGroupName(prefs.group_overrides.find(g => g.id === e.target.value)?.title ?? ''); }}><option value="new">新建显示组</option>{prefs.group_overrides.map(g => <option key={g.id} value={g.id}>{g.title} / {g.hall_id}</option>)}</select><button onClick={() => groupAction('save')}>加入 / 改名</button><button onClick={() => groupAction('remove')}>移除所选成员</button><button onClick={() => groupAction('independent')}>独立行</button><button onClick={() => groupAction('auto')}>所选恢复自动</button><button disabled={groupTarget === 'new'} onClick={() => groupAction('dissolve')}>解散所选组</button></div>}
    {itemsOpen && <div className="sf-tl-flyout" aria-label="物品跟随面板"><strong>真实物品 · {items === null ? '等待完整同结果时点查询' : `${items.length} 条身份记录 · ${totals.join(' / ')}`}</strong><button onClick={closeItems}>关闭</button><input aria-label="查找物品" placeholder="物品ID / 类型 / 持有者" value={itemFilter} onChange={e => setItemFilter(e.target.value)} /><select aria-label="跟随物品" value={itemId} onChange={e => { setItemId(e.target.value); const i = items?.find(i => i.id === e.target.value); if (i && run) select({ key: `${run.id}:item:${i.id}`, source: 'actual', run_id: run.id, input_ref: run.input_ref, object: { kind: 'item', id: i.id }, label: i.id, at_ms: atMs ?? undefined, fields: [{ label: '持有者', value: `${i.holder.kind} ${i.holder.id}` }, { label: '生命周期', value: i.lifecycle }, { label: '质量', value: i.quality }, { label: '数量', value: quantityText(i.quantity) }], related_objects: [holderObject(i.holder)] }); }}><option value="">选择身份（{visibleItems.length}）</option>{visibleItems.slice(0, 100).map(i => <option key={i.id} value={i.id}>{i.id} · {i.holder.id}</option>)}</select>{visibleItems.length > 100 && <span>前100项；按完整ID筛选可定位任一身份。</span>}{itemId && <><span>{item ? `${item.id} · ${item.holder.kind} ${item.holder.id} · ${item.lifecycle} · ${quantityText(item.quantity)}` : '此时点无该身份；不沿用末态库存'}</span><button disabled={!before} onClick={() => before && eventSelect(before.sequence)}>此前事件</button><button disabled={!after} onClick={() => after && eventSelect(after.sequence)}>后续事件</button><button disabled={!item} onClick={() => item && onSelect(holderObject(item.holder))}>定位持有者</button><span>关联实际事件 {history.length} 条</span></>}</div>}
    {(mode === 'input' || preview || notice) && <div className="sf-tl-input-form"><div style={{ zoom: props.inputTextScale }}>
    {mode === 'input' && <div className="sf-tl-input-tools"><label><input type="checkbox" checked={prefs.snap_enabled} onChange={e => save({ ...prefs, snap_enabled: e.target.checked })} />吸附 6px · Alt 关闭</label><select aria-label="选择输入标记" value="" onChange={e => { const p = inputs.find(p => p.key === e.target.value); if (p?.input_field) { setFields(v => v.some(f => fieldKey(f) === fieldKey(p.input_field!)) ? v : [...v, p.input_field!]); if (!fields.length) setNumeric(String(p.point_ms)); } }}><option value="">添加联动输入（{fields.length}）</option>{inputs.map(p => <option key={p.key} value={p.key}>{p.label} · {timeText(p.point_ms!)}</option>)}</select><button onClick={() => { cancel(); setFields([]); }}>清除联动</button>{fields[0] && <span>锚点：{fields[0].id} · 其余同 Δt</span>}{reference && <><span>参考：{reference.label} · {reference.at_ms != null ? `${reference.at_ms} ms` : reference.true_interval ? `${reference.true_interval.start_ms}—${reference.true_interval.end_ms} ms` : '无时点'}</span><button onClick={() => setReference(null)}>取消参考</button></>}{([{section:'tasks',title:'编辑优先序'},{section:'calendars',title:'编辑班次'},{section:'stages',title:'编辑阶段工时'}] as const).map(entry=><button key={entry.section} disabled={!props.onEditBusiness} title="在同工程表单编辑；保存后时间轴读取同一输入" onClick={()=>props.onEditBusiness?.({section:entry.section})}>{entry.title}</button>)}<label>目标时点 / ms<input aria-label="输入精确毫秒" type="number" step="1" value={numeric} onChange={e => setNumeric(e.target.value)} /></label><button disabled={!fields.length} onClick={numericPreview}>预览精确时点</button></div>}
    {preview && <div className="sf-tl-preview" aria-label="输入编辑预览"><strong>尚未应用 · 工程 r{preview.proposal.base_project_revision} / 方案 r{preview.proposal.base_scenario_revision}</strong>{preview.proposal.changes.map(c => <span key={fieldKey(c)}>{c.id}：{timeText(c.before_ms)} → {timeText(c.after_ms)}</span>)}<span>锚点 {preview.proposal.changes[0]?.after_ms} ms · Δt {preview.proposal.changes[0] ? preview.proposal.changes[0].after_ms - preview.proposal.changes[0].before_ms : 0} ms</span>{preview.snap && <span>吸附 {preview.snap.label} · {preview.snap.at_ms} ms</span>}{preview.error && <span role="alert">{preview.error}；预览已保留</span>}<button disabled={committing || !props.onCommitInput} onClick={() => void commit()}>{committing ? '正在应用' : '确认应用一次'}</button><button disabled={committing} onClick={cancel}>取消预览</button>{preview.error && <><button disabled={!props.onRefreshInput || committing} onClick={() => { if (props.onRefreshInput) void props.onRefreshInput().then(() => setNotice("输入已刷新；检查保留的预览后重新预览并确认。"), error => setNotice((error as Error).message)); }}>刷新当前输入</button><button onClick={numericPreview}>按已刷新输入重新预览</button></>}{!props.onCommitInput && <span>方案提交尚未接入</span>}</div>}
    {notice && <div className="sf-tl-notice" role="status">{notice}</div>}
    </div></div>}
    <div className="sf-tl-ruler"><span>对象 / 轨道 · 绝对秒</span><div ref={ruler} className="sf-tl-ruler-axis" onPointerDown={props.readonly?undefined:e => { if (mode === 'input') return; e.currentTarget.setPointerCapture(e.pointerId); seekDrag.current = e.pointerId; setGhost(pointerTime(e.clientX)); }} onPointerMove={props.readonly?undefined:e => { if (seekDrag.current === e.pointerId) setGhost(pointerTime(e.clientX)); }} onPointerUp={props.readonly?undefined:e => { if (seekDrag.current === e.pointerId) { seekDrag.current = null; if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId); seek(pointerTime(e.clientX)); } }} onPointerCancel={props.readonly?undefined:cancel} onLostPointerCapture={props.readonly?undefined:e => { if (seekDrag.current === e.pointerId && ruler.current === e.currentTarget) cancel(); }}>{gridTicks(window, width).map(t => <span key={t} style={{ left: x(t) }}>{timeText(t)}</span>)}{atMs != null && pointVisible(atMs, window) && <i className="sf-tl-head" style={{ left: Math.min(width - 2, x(atMs)) }} />}{ghost != null && pointVisible(ghost, window) && <i className="sf-tl-head sf-tl-ghost" style={{ left: Math.min(width - 2, x(ghost)) }} />}</div></div>
    <div className="sf-tl-viewport" ref={viewport} onScroll={e => setScroll(e.currentTarget.scrollTop)} onBlur={() => { if (scroll !== prefs.row_scroll) save({ ...prefs, row_scroll: scroll }); }} onPointerUp={() => { if (!inputDrag.current && !rangeDrag.current && scroll !== prefs.row_scroll) save({ ...prefs, row_scroll: scroll }); }}>
      <div className="sf-tl-virtual" style={{ height: totalHeight }}>{visibleRows.map(row => <div className={`sf-tl-row sf-tl-${row.kind} ${selectedRow === row.id ? 'sf-tl-selected' : ''}`} key={row.id} style={{ top: row.top, height: row.height }}><div className="sf-tl-name" style={{ paddingLeft: 6 + row.depth * 12 }} title={row.warning ?? row.title}>{row.kind === 'group' && <button aria-label={`${expanded.has(row.id) ? '折叠' : '展开'} ${row.title}`} onClick={() => toggleGroup(row.id)}>{expanded.has(row.id) ? '▾' : '▸'}</button>}<button className="sf-tl-row-label" onClick={() => { setSelectedRow(row.id); if (row.object) onSelect(row.object); }}>{row.title}{row.warning ? ' ⚠' : ''}</button>{row.kind === 'group' && <><button aria-label={`上移 ${row.title}`} onClick={() => reorder(row.id, -1)}>↑</button><button aria-label={`下移 ${row.title}`} onClick={() => reorder(row.id, 1)}>↓</button></>}{row.kind === 'resource' && <button aria-label={`固定 ${row.title}`} aria-pressed={prefs.pinned_resource_ids.includes(row.object!.id)} onClick={() => save({ ...prefs, pinned_resource_ids: prefs.pinned_resource_ids.includes(row.object!.id) ? prefs.pinned_resource_ids.filter(id => id !== row.object!.id) : [...prefs.pinned_resource_ids, row.object!.id] })}>⌖</button>}</div>
      <div className="sf-tl-chart" data-row={row.id} data-render-object={row.object?JSON.stringify(row.object):undefined} onPointerDown={props.readonly?undefined:e => { setSelectedRow(row.id); startRange(e); }} onPointerMove={props.readonly?undefined:moveRange} onPointerUp={props.readonly?undefined:endRange} onPointerCancel={props.readonly?undefined:cancel} onLostPointerCapture={props.readonly?undefined:e => { if (rangeDrag.current?.pointer === e.pointerId && rangeDrag.current.target === e.currentTarget) cancel(); }}>
        {mode !== 'input' && frontier != null && frontier < window.end_ms && <div className="sf-tl-unknown" style={{ bottom:mode === "comparison" ? "50%" : 0, left: Math.max(0, x(frontier)), width: Math.max(0, width - Math.max(0, x(frontier))) }} title="未计算区域">{row.kind === 'hall' ? '未计算 · 未知' : ''}</div>}
        {mode === "comparison" && comparison?.run.computed_until_ms != null && comparison.run.computed_until_ms < window.end_ms && <div className="sf-tl-unknown" style={{top:"50%",left:Math.max(0,x(comparison.run.computed_until_ms)),width:Math.max(0,width-Math.max(0,x(comparison.run.computed_until_ms)))}} title="B 未计算区域" />}
        {gridTicks(window, width).map(t => <i className="sf-tl-gridline" key={t} style={{ left: x(t) }} />)}
        {row.kind === 'group' ? [...(summaries.get(row.id) ?? []).map(band => ({...band, secondary:false})), ...(mode === 'comparison' ? (otherSummaries.get(row.id) ?? []).map(band => ({...band, secondary:true})) : [])].map((band, i) => { const clip = clipInterval(band.interval, { start_ms: window.start_ms, end_ms: Math.min(window.end_ms, (band.secondary ? comparison?.run.computed_until_ms : frontier) ?? window.start_ms) }); return clip && <button key={i} className={`sf-tl-summary ${band.secondary ? "sf-tl-summary-b" : ""}`} style={{ top:band.secondary ? 41 : 1, left: x(clip.visible.start_ms), width: Math.max(1, x(clip.visible.end_ms) - x(clip.visible.start_ms)) }} aria-label={`${band.secondary ? "B · " : "A · "}${row.title} B ${band.busy}/${band.total} W ${band.waiting}/${band.total} O ${band.occupied}/${band.total}`} title={`有效加工 B ${band.busy}/${band.total} · 等待 W ${band.waiting}/${band.total} · 容量占用 O ${band.occupied}/${band.total}；允许重叠`} onClick={() => { setSelectedRow(row.id); setRange(band.interval); }}><span>B{band.busy}/{band.total}</span><span>W{band.waiting}/{band.total}</span><span>O{band.occupied}/{band.total}</span></button>; }) : bars(byRow.get(row.id) ?? [], false)}
        {mode === 'comparison' && row.kind !== 'group' && row.kind !== 'hall' && bars(otherByRow.get(row.id) ?? [], true)}
        {mode !== 'input' && row.id === 'events:arrivals' && oldInputs.filter(p => p.input_field?.kind === 'arrival').map(p => pointVisible(p.point_ms!, window) && <i key={p.key} className="sf-tl-input-reference" title={`冻结输入 · ${p.label}`} style={{ left: Math.min(width - 8, x(p.point_ms!)) }} />)}
        {props.expected?.spans.filter(p => p.object.id === row.object?.id).map(p => { const clip = clipInterval(p.interval, window); return clip && <button className="sf-tl-expected" key={p.id} title={`手算来源 ${props.expected!.source_ref}`} style={{ left: x(clip.visible.start_ms), width: x(clip.visible.end_ms) - x(clip.visible.start_ms) }} onClick={() => select({ key: `expected:${p.id}`, source: 'manual_expected', run_id: null, object: p.object, label: `手算 · ${p.label}`, true_interval: p.interval, fields: [{ label: '来源', value: props.expected!.source_ref }], related_objects: [] })}>{p.label}</button>; })}
        {mode === 'input' && preview?.snap && pointVisible(preview.snap.at_ms, window) && <i className="sf-tl-snap-guide" aria-label={`吸附对齐 ${preview.snap.label} ${preview.snap.at_ms} ms`} style={{ left: Math.min(width - 1, x(preview.snap.at_ms)) }} />}{atMs != null && mode !== 'input' && pointVisible(atMs, window) && <i className="sf-tl-playhead" style={{ left: Math.min(width - 2, x(atMs)) }} />}{ghost != null && mode !== 'input' && pointVisible(ghost, window) && <i className="sf-tl-playhead sf-tl-ghost" style={{ left: Math.min(width - 2, x(ghost)) }} />}{range && clipInterval(range, window) && <div className="sf-tl-range" style={{ left: x(Math.max(window.start_ms, range.start_ms)), width: x(Math.min(window.end_ms, range.end_ms)) - x(Math.max(window.start_ms, range.start_ms)) }} />}
      </div></div>)}</div>
    </div>
    {dense.length > 0 && <div className="sf-tl-flyout" aria-label="密集事件列表"><strong>保留全部来源 · {dense.length} 项</strong><button onClick={() => setDense([])}>关闭</button>{dense.map(p => <button key={p.key} onClick={() => { inspect(p); setDense([]); }}>{timeText(p.point_ms ?? p.interval!.start_ms)} · {p.label} · {p.selection.event?.sequence ?? p.object.id}</button>)}</div>}
    {waiting && <div className="sf-tl-wait-details" aria-label="等待持有与事件详情">
      <strong>等待关联 · {atMs} ms</strong>
      <span>保留资源：{waiting.held_resources.map(c => `${c.resource_id} → ${c.job_id} (${c.units})`).join('；') || '此时点无持有区间'}</span>
      <span>阻断资源持有者：{waiting.blocker_holders.map(c => `${c.resource_id} → ${c.job_id} (${c.units})`).join('；') || '此时点无对应资源占用记录'}</span>
      <span>输入/待移交物品/货物持有者：{waiting.cargo === null ? '等待读取完整同Run时点物品' : waiting.cargo.map(i => `${i.id} → ${i.holder.kind} / ${i.holder.id}`).join('；') || '此时点无关联物品身份'}</span>
      {waiting.cargo?.map(i => <button key={i.id} onClick={() => onSelect(holderObject(i.holder))}>定位持有者 {i.holder.id}</button>)}
      <button disabled={!waiting.previous} onClick={() => waiting.previous && eventSelect(waiting.previous.sequence)}>此前关联事件{waiting.previous ? ` · #${waiting.previous.sequence} · ${waiting.previous.at_ms} ms · ${eventNames[waiting.previous.payload.kind]}` : ' · 无记录'}</button>
      <button disabled={!waiting.next} onClick={() => waiting.next && eventSelect(waiting.next.sequence)}>后续关联事件{waiting.next ? ` · #${waiting.next.sequence} · ${waiting.next.at_ms} ms · ${eventNames[waiting.next.payload.kind]}` : ' · 无记录'}</button>
    </div>}
    {selected && <div className="sf-tl-links" aria-label="业务关系"><span>关系 · {selected.label}</span>{links.map(link => <button key={`${link.object.kind}:${link.object.id}`} onClick={() => followRelation(link.object)}>{link.label}</button>)}{selected.event && <><button disabled={!events.some(e => e.sequence < selected.event!.sequence)} onClick={() => { const e = [...events].reverse().find(e => e.sequence < selected.event!.sequence); if (e) eventSelect(e.sequence); }}>上一事件</button><button disabled={!events.some(e => e.sequence > selected.event!.sequence)} onClick={() => { const e = events.find(e => e.sequence > selected.event!.sequence); if (e) eventSelect(e.sequence); }}>下一事件</button></>}</div>}
    <div className="sf-tl-status"><button disabled={!run || mode === 'input'} aria-pressed={playing} onClick={togglePlayback}>{playing ? '暂停回看' : '播放回看'}</button><select aria-label="回看速度" value={speed} onChange={e => setSpeed(Number(e.target.value))}>{[.5, 1, 2, 4, 10, 16].map(v => <option key={v} value={v}>{v}×</option>)}</select><input aria-label="观察毫秒" type="number" step="1" value={seekInput} onChange={e => setSeekInput(e.target.value)} /><button disabled={!run || mode === 'input'} onClick={() => seek(Number(seekInput))}>读取时点</button><button disabled={!run || mode === 'input' || frontier == null} onClick={() => frontier != null && seek(frontier)}>返回当前</button><button onClick={() => setWindow(domain)}>全窗口</button><span>{mode === 'input' ? '输入手柄可编辑 · 实际结果保持冻结' : `实际 ${atMs == null ? '未知' : timeText(atMs)} · 已计算至 ${frontier == null ? '未知' : timeText(frontier)}`} · 事件 {events.length} / 作业 {run?.jobs.length ?? 0} / 接受记录 {run?.acceptance_events.length ?? 0}{ghost == null ? '' : ` · 候选 ${timeText(ghost)}`}</span>{run && <span title={run.scope.map(o=>`${o.kind} ${o.id}`).join("、")}>范围：{run.scope.length ? `${run.scope.length} 个所选对象` : "冻结方案全范围"}</span>}{run?.limitations.map((issue,i)=><button key={`issue:${i}`} disabled={!issue.object_ref} onClick={()=>issue.object_ref && onSelect(issue.object_ref)}>{issue.message}</button>)}{usage.map(v => <span key={v.unit}>资源占用率 {v.unit}：{v.ratio == null ? '未知' : `${(v.ratio * 100).toFixed(1)}%`}</span>)}{range && <span>选段 {timeText(range.start_ms)}—{timeText(range.end_ms)}</span>}{storageError && <span role="status">{storageError}</span>}</div>
    </fieldset></section>;
  function bars(source: Primitive[], secondary: boolean) {
    const visible = source.filter(p => p.point_ms != null ? pointVisible(p.point_ms, window) : p.interval && clipInterval(p.interval, window));
    const denseIntervals = intervalDensity(visible.filter(p => p.point_ms == null), window, width);
    const bucketed = new Set(denseIntervals.flatMap(b => b.sources.map(p => p.key)));
    return <>{visible.filter(p => p.point_ms == null && !bucketed.has(p.key)).map(p => { const clip = clipInterval(p.interval!, window)!, w = Math.max(1, x(clip.visible.end_ms) - x(clip.visible.start_ms)); return <button key={`${secondary ? 'B:' : ''}${p.key}`} className={`sf-tl-bar sf-tl-bar-${p.kind} ${p.source === 'input' && p.kind === 'calendar' ? 'sf-tl-input-calendar' : ''} ${secondary ? 'sf-tl-secondary' : ''} ${!p.known_end ? 'sf-tl-open-end' : ''}`} style={{ left: x(clip.visible.start_ms), width: w, '--lane-offset': secondary ? '32px' : '0px' } as CSSProperties} data-source-key={p.key} data-render-object={JSON.stringify(p.object)} data-true-start={p.interval!.start_ms} data-true-end={p.interval!.end_ms} title={`${secondary ? 'B · ' : ''}${p.label} · ${timeText(p.interval!.start_ms)}—${timeText(p.interval!.end_ms)}${!p.known_end ? '（边界，尚未结束）' : ''}`} aria-label={`${p.label} ${p.object.id} ${timeText(p.interval!.start_ms)}—${timeText(p.interval!.end_ms)}`} onClick={event => { const at = pointerTime(event.clientX); const hits = visible.filter(v => v.kind === p.kind && v.interval && v.interval.start_ms <= at && at < v.interval.end_ms); if (hits.length > 1) setDense(hits); else inspect(p); }}>{clip.continues_left ? '‹' : ''}{w > 38 ? p.label : ''}{clip.continues_right ? '›' : ''}</button>; })}
      {denseIntervals.map(bucket => <button key={`${secondary ? "B:" : ""}interval-density:${bucket.kind}:${bucket.x}`} className={`sf-tl-bar sf-tl-bar-${bucket.kind} sf-tl-density-segments`} style={{left:bucket.x,width:6,"--lane-offset":secondary ? "32px" : "0px"} as CSSProperties} aria-label={`展开密集区间 ${bucket.sources.length} 段`} title={`${bucket.sources.length} 段真实${bucket.kind}；点击逐项查看`} data-source-keys={bucket.sources.map(p=>p.key).join("|")} onClick={()=>setDense(bucket.sources)} />)}
      {densityBuckets(visible.filter(p => p.point_ms != null), window, width).map(bucket => {
        // Editable input handles retain individual targets even at identical times.
        if (bucket.sources.length > 1 && mode !== 'input') return <button key={`${secondary ? 'B:' : ''}density:${bucket.x}`} className="sf-tl-point sf-tl-density" style={{ left: Math.min(width - 16, Math.max(0, bucket.x)), top: secondary ? 35 : 5 }} aria-label={`展开密集事件 ${bucket.sources.length} 项`} onClick={() => setDense(bucket.sources)}>{bucket.sources.length}</button>;
        return bucket.sources.map((p, i) => { const f = p.input_field, change = f ? preview?.proposal.changes.find(c => fieldKey(c) === fieldKey(f)) : undefined;
          return <button key={`${secondary ? 'B:' : ''}${p.key}`} data-render-object={JSON.stringify(p.object)} className={`sf-tl-point sf-tl-point-${p.kind} ${secondary ? 'sf-tl-secondary' : ''} ${f && fields.some(v => fieldKey(v) === fieldKey(f)) ? 'sf-tl-input-selected' : ''}`} style={{ left: Math.min(width - 10, Math.max(0, x(change?.after_ms ?? p.point_ms!))), top: secondary ? 35 : 3 + i * 7 }} aria-label={`${p.label} ${timeText(p.point_ms!)}`} title={`${p.label} · ${timeText(change?.after_ms ?? p.point_ms!)}${f ? ' · 拖动释放应用；冲突时保留预览' : ''}`} onPointerDown={f ? e => beginInput(e, p) : undefined} onPointerMove={f ? moveInput : undefined} onPointerUp={f ? endInput : undefined} onPointerCancel={f ? cancel : undefined} onLostPointerCapture={f ? lostInput : undefined} onClick={e => { e.stopPropagation(); if (f) { if (e.ctrlKey) setFields(v => v.some(a => a.kind !== f.kind) ? v : v.some(a => fieldKey(a) === fieldKey(f)) ? v.filter(a => fieldKey(a) !== fieldKey(f)) : [...v, f]); select(p.selection); } else inspect(p); }}>◆</button>;
        });
      })}</>;
  }
}
