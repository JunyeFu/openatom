import { useEffect, useMemo, useRef, useState } from 'react';
import { MediaPresentationView } from '../../../apps/planner/src/features/media';
import { ImageTimelineEditor, createImageTimelineMedia } from '../../../apps/planner/src/features/presentation-editor';
import type { ImageTimelineEditorProps } from '../../../apps/planner/src/features/presentation-editor/types';
import type { WorkspaceSnapshot } from '../../../apps/planner/src/services/projectHost';
import type { MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import { createPresentationHost, processContextKey, ProcessSourceChanged,
  type ActualProcessObservation, type DeviceProcessSelection, type ProcessDefinition,
  type ProcessReadContext, type ProcessScope, type ProcessActions, type ProcessAssetSource } from './presentationHost.before';
import type { DeviceAction } from '../../../packages/commands/generated/typescript/DeviceAction';
import type { LodLevel } from '../../../packages/commands/generated/typescript/LodLevel';
import type { ModuleRef } from '../../../packages/commands/generated/typescript/ModuleRef';
import type { AuthoringDraft } from '../../../apps/planner/src/features/modules/types';
import '../../../apps/planner/src/features/factory/DeviceProcessWorkspace.css';

export type { ProcessActions, ProcessActionRequest } from './presentationHost.before';
export interface DeviceProcessWorkspaceProps {
  workspace: WorkspaceSnapshot;
  /** The actual authorized reading scope, including DefinitionCycle DraftR. */
  scope: ProcessScope['scope'];
  scopeRevision: number;
  selection: DeviceProcessSelection;
  actions: ProcessActions;
  /** Original Root AuthoringIO/Samples/Commands and the already-read Draft. */
  editor?: Omit<ImageTimelineEditorProps, 'workspace' | 'onClose' | 'media'>;
  onSelectionChanged?(selection: DeviceProcessSelection): void;
  onInspectItem?(request: { context: ProcessReadContext; item_id: string }): void | Promise<void>;
  onSelectNode?(request: { context: ProcessReadContext; module_ref: ModuleRef; node_index: number }): void;
  onReturnToAuthoring?(draft: AuthoringDraft | null): void;
  onClose(): void;
}
type LoadedProcess = ProcessDefinition | ActualProcessObservation;
const isActual = (value: LoadedProcess): value is ActualProcessObservation => 'frame' in value;
const refText = (ref: ModuleRef) => `${ref.id}@${ref.version}`;
const actionNames: Record<DeviceAction, string> = { configure: '配置参数', inspect: '定位检查', preview_cycle: '实际周期样例' };
const editorKey = (workspace: WorkspaceSnapshot, editor: DeviceProcessWorkspaceProps['editor']) => JSON.stringify([
  workspace.context.host.session_id, workspace.project?.id, editor?.draft?.summary.draft_id,
  editor?.source?.definition.id, editor?.source?.definition.version,
]);

/** Read-only process consumer. Root retains Project, AuthoringIO and actual action ports. */
export function DeviceProcessWorkspace(props: DeviceProcessWorkspaceProps) {
  const [seek, setSeek] = useState<{ input: string; value: number } | null>(null);
  const [loaded, setLoaded] = useState<{ key: string; value: LoadedProcess } | null>(null);
  const [error, setError] = useState(''), [message, setMessage] = useState(''), [refresh, setRefresh] = useState(0);
  const [lod, setLod] = useState<LodLevel>('detailed');
  const [inspection, setInspection] = useState(false), [phase, setPhase] = useState(''), [u, setU] = useState(0);
  const [playback, setPlayback] = useState({ playing: false, rate: 1, loop: false });
  const [acting, setActing] = useState(false);
  const [editing, setEditing] = useState<{ key: string; workspace: WorkspaceSnapshot; editor: NonNullable<DeviceProcessWorkspaceProps['editor']> } | null>(null);
  const createdDraft = useRef<AuthoringDraft | null>(null);
  const incoming = JSON.stringify(props.selection);
  const emittedSelection = useRef<string | null>(null);
  const selection: DeviceProcessSelection = props.selection.kind === 'actual_run' && seek?.input === incoming
    ? { ...props.selection, at_ms: seek.value } : props.selection;
  const context: ProcessReadContext | null = props.workspace.project ? {
    session_id: props.workspace.context.host.session_id, project_id: props.workspace.project.id,
    scope: props.scope, scope_revision: props.scopeRevision, selection,
  } : null;
  const key = context ? processContextKey(context) : '';
  const latest = useRef(context); latest.current = context;
  const active = useRef(key); active.current = key;
  const host = useMemo(() => createPresentationHost(() => latest.current), []);
  const value = loaded?.key === key ? loaded.value : null;
  const actual = value && isActual(value) ? value : null;
  const reader = useMemo(() => value && context ? host.media(context, value.binding) : null, [host, key, value]);
  const declared = value?.definition.payload.category === 'devices' ? value.definition.payload.data.operations : [];
  const phases = value ? [...new Set(value.presentation.timeline.map(clip => clip.phase_ref ?? 'cycle'))] : [];
  const displayPhase = phase && phases.includes(phase) ? phase : phases[0] ?? 'cycle';
  const currentEditorKey = editorKey(props.workspace, props.editor);
  const createdHere = !!editing && !editing.editor.draft && !!props.editor?.draft
    && createdDraft.current?.summary.draft_id === props.editor.draft.summary.draft_id
    && editorKey(props.workspace, { ...props.editor, draft: null }) === editing.key;
  const editorInput = editing && props.editor && editing.key === currentEditorKey
    ? { workspace: props.workspace, editor: props.editor }
    : createdHere && props.editor ? { workspace: props.workspace, editor: { ...props.editor, draft: null } } : editing;
  // First save may create the actual Draft. Keep the mounted editor identity and
  // its local conflict input, while using the real returned Draft for read scope.
  const effectiveEditorDraft = createdHere ? props.editor!.draft : editorInput?.editor.draft;
  const editorDefinition = effectiveEditorDraft?.definition ?? editorInput?.editor.source?.definition;
  const editorActual = editorInput && actual && context && editorDefinition?.id === actual.module_ref.id
    && editorInput.workspace.context.host.session_id === context.session_id && editorInput.workspace.project?.id === context.project_id
    && (context.scope.kind === 'project' || effectiveEditorDraft?.summary.draft_id === context.scope.data) ? actual : null;
  const declarationContext: ProcessReadContext | null = editorInput?.workspace.project && editorDefinition ? {
    session_id: editorInput.workspace.context.host.session_id, project_id: editorInput.workspace.project.id,
    scope: effectiveEditorDraft ? { kind: 'definition_draft', data: effectiveEditorDraft.summary.draft_id } : { kind: 'project' },
    scope_revision: effectiveEditorDraft?.summary.revision ?? editorInput.workspace.project.revision,
    selection: { kind: 'definition', module_ref: { id: editorDefinition.id, version: editorDefinition.version } },
  } : null;
  const declarationKey = declarationContext ? processContextKey(declarationContext) : '';
  const declarationCurrent = useRef<ProcessReadContext | null>(null);
  declarationCurrent.current = editing && (editing.key === currentEditorKey || createdHere) ? declarationContext : null;
  const declarationHost = useMemo(() => createPresentationHost(() => declarationCurrent.current), []);
  const declarationReader = useMemo(() => {
    if (!declarationContext || declarationContext.selection.kind !== 'definition') return null;
    const binding = { session_id: declarationContext.session_id, request: {
      project_id: declarationContext.project_id, scope: declarationContext.scope, scope_revision: declarationContext.scope_revision,
      source: { kind: 'module_version' as const, data: { module_ref: declarationContext.selection.module_ref } },
    } };
    return { binding,
      async readAsset(request: MediaAssetReadRequest) {
        return declarationHost.media(declarationContext, binding).readAsset(request);
      },
      async resolveModule(source: ProcessAssetSource) {
        return declarationHost.media(declarationContext, binding).resolveModule(source);
      },
    };
  }, [declarationHost, declarationKey]);
  const editorMedia = useMemo(() => declarationReader ? createImageTimelineMedia({
    declaration: { ...declarationReader, rights: effectiveEditorDraft?.definition.contribution.assets ?? editorInput?.editor.source?.rights ?? [] },
    actual: editorActual && reader ? { binding: editorActual.binding, readAsset: reader.readAsset, resolveModule: reader.resolveModule,
      rights: editorActual.rights, presentation: editorActual.presentation, frame: editorActual.frame } : null,
    unavailable_reason: '尚未取得此Run同源的实际Job历史、完整物品页与冻结表现；不借最终Job或当前草稿补画。',
    onActualSeek: at_ms => { setPlayback(value => ({ ...value, playing: false })); seekTo(at_ms); },
    onInspectItem: props.onInspectItem ? item_id => { void inspectItem(item_id); } : undefined,
  }) : undefined, [declarationReader, effectiveEditorDraft, editorInput?.editor.source, editorActual, reader, key, props.onInspectItem]);

  useEffect(() => {
    latest.current = context; active.current = key;
    return () => { latest.current = null; active.current = ''; declarationCurrent.current = null; };
  }, []);
  useEffect(() => {
    let live = true; setLoaded(null); setError(''); setMessage('');
    if (!context) return;
    host.read(context).then(value => { if (live && active.current === key) setLoaded({ key, value }); }, failure => {
      if (live && active.current === key && !(failure instanceof ProcessSourceChanged)) { setError(failure instanceof Error ? failure.message : String(failure)); setPlayback(value => ({ ...value, playing: false })); }
    });
    return () => { live = false; };
  }, [host, key, refresh]);
  useEffect(() => { setActing(false); }, [key]);
  useEffect(() => {
    if (emittedSelection.current === incoming) { emittedSelection.current = null; return; }
    setPlayback(value => ({ ...value, playing: false })); setInspection(false);
  }, [incoming]);
  useEffect(() => { setPlayback(value => ({ ...value, playing: false })); setInspection(false); }, [props.workspace.context.host.session_id, props.workspace.project?.id, props.scope.kind, props.scope.kind === 'definition_draft' ? props.scope.data : '', props.scopeRevision]);

  function seekTo(at_ms: number) {
    if (selection.kind !== 'actual_run') return;
    const next: DeviceProcessSelection = { ...selection, at_ms };
    emittedSelection.current = JSON.stringify(next);
    setSeek({ input: incoming, value: at_ms }); props.onSelectionChanged?.(next);
  }
  // This clock only requests recorded views. It never runs/resumes a Scenario or
  // writes Commands. Declaration u has its own explicit 4-second viewing cycle.
  useEffect(() => {
    if (!playback.playing || editing || !value) return;
    let frame = 0, start = performance.now(), last = start, cursor = u;
    const tick = (now: number) => {
      if (actual && !inspection) {
        const end = Math.min(actual.run.computed_until_ms!, actual.run.window.end_ms);
        if (now - start < 100) { frame = requestAnimationFrame(tick); return; }
        const at = actual.view_snapshot.at_ms + Math.floor((now - start) * playback.rate);
        if (at >= end) {
          if (playback.loop && end > actual.run.window.start_ms) seekTo(actual.run.window.start_ms);
          else { seekTo(end); setPlayback(value => ({ ...value, playing: false })); }
        } else seekTo(at);
      } else {
        cursor += (now - last) / 4000 * playback.rate; last = now;
        if (cursor >= 1) {
          if (playback.loop) cursor %= 1;
          else { cursor = 1; setPlayback(value => ({ ...value, playing: false })); }
        }
        setU(cursor); if (cursor < 1 || playback.loop) frame = requestAnimationFrame(tick);
      }
    };
    frame = requestAnimationFrame(tick); return () => cancelAnimationFrame(frame);
  }, [playback.playing, playback.rate, playback.loop, actual, value, inspection, !!editing]);

  async function invoke(action: DeviceAction) {
    if (!context || !value || acting) return;
    const port = props.actions[action], captured = key;
    if (!declared.includes(action) || !port) { setError(`${actionNames[action]} unavailable：当前精确声明与Root实际操作入口未同时具备。`); return; }
    setActing(true); setError(''); setMessage(''); setPlayback(value => ({ ...value, playing: false }));
    try {
      const result = await host.action(context, value, action, props.actions);
      if (active.current === captured) setMessage(result.message);
    } catch (failure) { if (active.current === captured) setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { if (active.current === captured) setActing(false); }
  }
  function openEditor() {
    if (!props.editor) return;
    setPlayback(value => ({ ...value, playing: false }));
    createdDraft.current = null;
    setEditing({ key: currentEditorKey, workspace: props.workspace, editor: props.editor });
  }
  async function inspectItem(item_id: string) {
    if (!context || !props.onInspectItem) return;
    const captured = key;
    try { await props.onInspectItem({ context, item_id }); }
    catch (failure) { if (active.current === captured) setError(failure instanceof Error ? failure.message : String(failure)); }
  }
  if (editing && editorInput) return <section className="sf-device-process sf-device-process-editor" aria-label="设备过程原草稿编辑">
    {editing.key !== currentEditorKey && !createdHere && <p role="alert">当前工程或创作来源已切换；原编辑输入保留。请通过编辑器的未保存确认关闭，旧Scope的保存由原AuthoringIO检查。</p>}
    <ImageTimelineEditor {...editorInput.editor} workspace={editorInput.workspace} run={editorActual?.run ?? null}
      observation={editorActual?.view_snapshot ?? null}
      media={editorMedia}
      instance={editorActual?.device ? { id: editorActual.device.id, module_ref: editorActual.device.module_ref } : editorInput.editor.instance}
      onDraftChanged={draft => { if (!editing.editor.draft) createdDraft.current = draft; editorInput.editor.onDraftChanged(draft); }}
      onClose={() => setEditing(null)} />
  </section>;

  return <section className="sf-device-process" aria-label="设备过程工作区">
    <header className="sf-device-process-header"><div><h2>设备过程</h2><p>{actual?.device?.title ?? value?.definition.title ?? '读取设备过程来源'}</p></div>
      <div className="sf-device-process-header-actions"><button disabled={!props.editor || acting} onClick={openEditor}>图片过程与标签编辑</button>
        {props.onReturnToAuthoring && <button onClick={() => props.onReturnToAuthoring?.(props.editor?.draft ?? null)}>返回原定义工作区</button>}
        <button onClick={props.onClose}>返回工作台</button></div></header>
    <div className="sf-device-process-source" aria-label="过程来源"><span>Host会话 {context?.session_id ?? '未连接'}</span><span>工程 {context?.project_id ?? '未打开'}</span>
      <span>{props.scope.kind === 'project' ? 'Project' : `Draft ${props.scope.data}`} / 授权修订 r{props.scopeRevision}</span>
      <span>{value ? `精确模块 ${refText(value.module_ref)}` : selection.kind === 'definition' ? refText(selection.module_ref) : `Run ${selection.run_id} / Job ${selection.job_id}`}</span>
      {actual && <><span>实例 {actual.device ? `${actual.device.id} → ${refText(actual.device.module_ref)}` : 'DefinitionCycle，无采用实例'}</span>
        <span>冻结输入 r{actual.run.input_ref.project_revision} / 工艺 {refText(actual.job.operation_ref)}</span></>}
    </div>
    <div className="sf-device-process-tools"><label>显示档位 <select aria-label="过程显示档位" value={lod} onChange={event => setLod(event.target.value as LodLevel)}><option value="detailed">详细</option><option value="simplified">简化</option><option value="region">区域</option></select></label>
      {actual && <label><input aria-label="仅检查冻结声明" type="checkbox" checked={inspection} onChange={event => { setInspection(event.target.checked); setPlayback(value => ({ ...value, playing: false })); }} />仅检查此Run冻结声明</label>}
      {(!actual || inspection) && <label>声明阶段 <select aria-label="过程声明阶段" value={displayPhase} onChange={event => { setPhase(event.target.value); setU(0); setPlayback(value => ({ ...value, playing: false })); }}>{(phases.length ? phases : ['cycle']).map(id => <option key={id}>{id}</option>)}</select></label>}
      {selection.kind === 'actual_run' && <><label>实际观看时点 <input aria-label="过程时点ms" type="number" value={selection.at_ms} onChange={event => { if (event.target.value !== '') { setPlayback(value => ({ ...value, playing: false })); seekTo(Number(event.target.value)); } }} /> ms</label><button onClick={() => setRefresh(value => value + 1)}>重读实际来源</button></>}
    </div>
    {actual && <p className="sf-device-process-frontier" aria-label="实际计算边界">观看 {actual.view_snapshot.at_ms} ms · 已计算至 {actual.run.computed_until_ms} ms · {actual.run.computation_status} · 原窗口 [{actual.run.window.start_ms}, {actual.run.window.end_ms}] ms</p>}
    {!context && <p role="status">先打开实际工程，才能读取设备过程。</p>}
    {error && <p role="alert" className="sf-device-process-error">{error}</p>}{message && <p role="status">{message}</p>}
    {context && !value && !error && <p role="status">正在读取所选Scope的精确过程来源…</p>}
    {value && context && reader && <MediaPresentationView presentation={value.presentation} binding={value.binding} readAsset={reader.readAsset} resolveModule={reader.resolveModule}
      lod={lod} rights={value.rights} playback={playback} onPlaybackChange={setPlayback}
      onSelectNode={props.onSelectNode ? node_index => props.onSelectNode?.({ context, module_ref: value.module_ref, node_index }) : undefined}
      onInspectItem={props.onInspectItem ? item_id => { void inspectItem(item_id); } : undefined}
      onAction={declared.some(action => props.actions[action]) && !acting ? action => { void invoke(action); } : undefined}
      {...actual && !inspection ? { mode: 'actual_run', frame: actual.frame, onSeek: at_ms => { setPlayback(value => ({ ...value, playing: false })); seekTo(at_ms); } }
        : { mode: 'declared_preview', preview: { phase_ref: displayPhase, view_u: u }, onPreviewSeek: preview => { setPlayback(value => ({ ...value, playing: false })); setPhase(preview.phase_ref); setU(preview.view_u); } }} />}
    <div className="sf-device-process-foot"><aside><h3>声明操作与实际入口</h3>{(Object.keys(actionNames) as DeviceAction[]).map(action => <p key={action}><strong>{actionNames[action]}</strong><span>{declared.includes(action) && props.actions[action] ? '当前精确声明 + Root实际入口' : 'unavailable · 缺少声明或实际入口'}</span></p>)}
      <p>配置、定位和样例运行由Root实际表单/对象/DefinitionCycle入口处理。观看不提交生产命令。</p></aside>
      {actual && <details><summary>原始读取与只读视图来源</summary><dl><dt>原始Run</dt><dd>{actual.raw.run.id} · 原完整结果保留</dd><dt>显式Job ObjectsAt</dt><dd>{actual.raw.job_pages.length} 页 · {actual.job.id} @ {actual.view_snapshot.at_ms} ms</dd><dt>原空对象查询</dt><dd>{actual.raw.item_pages.length} 页 · 只消费实际Item</dd><dt>冻结Project</dt><dd>{actual.raw.frozen_project.id} r{actual.raw.frozen_project.revision} · {actual.raw.frozen_project.resources.length} 条原资源声明</dd><dt>呈现快照</dt><dd>本服务只读组合，非Host原始单页；资源占用只取Run.resource_claims和此刻Job持有。</dd></dl>
        <details><summary>原始Job页</summary><pre>{JSON.stringify(actual.raw.job_pages, null, 2)}</pre></details>
        <details><summary>原始Item页</summary><pre>{JSON.stringify(actual.raw.item_pages, null, 2)}</pre></details>
      </details>}
    </div>
  </section>;
}
