import { runStatusLabel } from '../../services/workbenchLabels';
import { useEffect, useMemo, useRef, useState } from 'react';
import { MediaPresentationView } from '../media';
import { ImageTimelineEditor, createImageTimelineMedia } from '../presentation-editor';
import type { ImageTimelineEditorProps } from '../presentation-editor/types';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { MediaAssetReadRequest } from '../../services/mediaAssets';
import { createPresentationHost, processContextKey, ProcessSourceChanged,
  type ActualProcessObservation, type DeviceProcessSelection, type ProcessDefinition,
  type ProcessReadContext, type ProcessScope, type ProcessActions, type ProcessAssetSource, type FrozenProcessObservation } from '../../services/presentationHost';
import type { DeviceAction } from '../../../../../packages/commands/generated/typescript/DeviceAction';
import type { LodLevel } from '../../../../../packages/commands/generated/typescript/LodLevel';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { AuthoringDraft } from '../modules/types';
import { lodSource, UnsupportedMedia } from '../../../../../packages/presentation/src';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import './DeviceProcessWorkspace.css';
import { NodeInspector, InlineNodeEditor, createNodeInspectorReader } from '../node-inspector';
import { nodeSelectionKey } from '../node-inspector/model';
import type { MediaNodeSelection, ProcessNodeSelection, NodeInspection, NodeEditRequest, NodeSourceEditRequest } from '../node-inspector';
import type { ProjectState, EditCommand } from '../../services/projectHost';

export type { ProcessActions, ProcessActionRequest } from '../../services/presentationHost';
export type { NodeEditRequest, NodeSourceEditRequest } from '../node-inspector';
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
  onApplyNode?(commands: EditCommand[], base: ProjectState): Promise<ProjectState>;
  onEditNodeSource?(request: NodeSourceEditRequest): void | Promise<void>;
  onNodeDirtyChange?(dirty: boolean): void;
  onReturnToAuthoring?(draft: AuthoringDraft | null): void;
  onClose(): void;
}
type LoadedProcess = ProcessDefinition | ActualProcessObservation | FrozenProcessObservation;
const isActual = (value: LoadedProcess): value is ActualProcessObservation => 'frame' in value;
const isFrozen = (value: LoadedProcess): value is FrozenProcessObservation => 'raw_static' in value;
const refText = (ref: ModuleRef) => `${ref.id}@${ref.version}`;
const actionNames: Record<DeviceAction, string> = { configure: '配置参数', inspect: '定位检查', preview_cycle: '实际周期样例' };
// Time changes the recorded frame, not the immutable declaration/bytes in a Run.
// This key is only for retaining that source; every Host frame read keeps its
// exact ProcessReadContext, including at_ms.
const mediaSourceKey = (context: ProcessReadContext) => processContextKey(context.selection.kind === 'actual_run'
  ? { ...context, selection: { ...context.selection, at_ms: 0 } } : context);
const editorKey = (workspace: WorkspaceSnapshot, editor: DeviceProcessWorkspaceProps['editor']) => JSON.stringify([
  workspace.context.host.session_id, workspace.project?.id, editor?.draft?.summary.draft_id,
  editor?.source?.definition.id, editor?.source?.definition.version,
]);

/** Read-only process consumer. Root retains Project, AuthoringIO and actual action ports. */
export function DeviceProcessWorkspace(props: DeviceProcessWorkspaceProps) {
  const [seek, setSeek] = useState<{ input: string; value: number } | null>(null);
  const [loaded, setLoaded] = useState<{ key: string; context: ProcessReadContext; refresh: number; value: LoadedProcess } | null>(null);
  const [error, setError] = useState(''), [message, setMessage] = useState(''), [refresh, setRefresh] = useState(0);
  const [lod, setLod] = useState<LodLevel>('detailed');
  const [resolvedDeclaration, setResolvedDeclaration] = useState<{ key: string; owner: string; presentation: Presentation } | null>(null);
  const [inspection, setInspection] = useState(false), [phase, setPhase] = useState(''), [u, setU] = useState(0);
  const [playback, setPlayback] = useState({ playing: false, rate: 1, loop: false });
  const [acting, setActing] = useState(false);
  const [editing, setEditing] = useState<{ key: string; workspace: WorkspaceSnapshot; editor: NonNullable<DeviceProcessWorkspaceProps['editor']> } | null>(null);
  const [nodeSelection, setNodeSelection] = useState<{ value: ProcessNodeSelection; lane: 'main' | 'editor'; generation: number } | null>(null);
  const nodeGeneration = useRef(0);
  const [inlineEdit, setInlineEdit] = useState<{ inspection: NodeInspection; base: ProjectState } | null>(null);
  const [nodeDirty, setNodeDirty] = useState(false), [leaveNode, setLeaveNode] = useState<{ run(): void } | null>(null);
  const createdDraft = useRef<AuthoringDraft | null>(null);
  const incoming = JSON.stringify(props.selection);
  const emittedSelection = useRef<string | null>(null);
  const selection: DeviceProcessSelection = props.selection.kind !== 'definition' && seek?.input === incoming
    ? { ...props.selection, at_ms: seek.value } : props.selection;
  const context: ProcessReadContext | null = props.workspace.project ? {
    session_id: props.workspace.context.host.session_id, project_id: props.workspace.project.id,
    scope: props.scope, scope_revision: props.scopeRevision, selection,
  } : null;
  const key = context ? processContextKey(context) : '';
  const sourceKey = context ? mediaSourceKey(context) : '';
  const nodeBasis = useRef(key); if (nodeBasis.current !== key) { nodeBasis.current = key; nodeGeneration.current++; }
  const latest = useRef(context); latest.current = context;
  const active = useRef(key); active.current = key;
  const host = useMemo(() => createPresentationHost(() => latest.current), []);
  const value = !error && loaded?.refresh === refresh && mediaSourceKey(loaded.context) === sourceKey ? loaded.value : null;
  const actual = value && isActual(value) ? value : null;
  const frozen = value && isFrozen(value) ? value : null;
  const staticSelection = selection.kind === 'frozen_device' || selection.kind === 'frozen_definition';
  // Hold a legal frozen byte-reading context while successive actual frames are
  // read. Scope/revision/Run/Job/source changes and explicit refresh replace it.
  const mediaInput = useMemo(() => value && loaded ? { context: loaded.context, value } : null, [sourceKey, refresh, !!value]);
  const mediaCurrent = useRef<ProcessReadContext | null>(null); mediaCurrent.current = mediaInput?.context ?? null;
  const mediaHost = useMemo(() => createPresentationHost(() => mediaCurrent.current), []);
  const reader = useMemo(() => mediaInput ? mediaHost.media(mediaInput.context, mediaInput.value.binding) : null, [mediaHost, mediaInput]);
  const activeSource = useRef(sourceKey); activeSource.current = sourceKey;
  const phaseOwner = JSON.stringify([sourceKey, refresh]);
  const phaseKey = JSON.stringify([phaseOwner, lod]);
  const phaseCurrent = useRef(phaseKey); phaseCurrent.current = phaseKey;
  // Observe the same metadata traversal used by the shared view. A linked shell
  // owns no timeline; the final precise Media package owns declaration phases.
  const viewReader = useMemo(() => reader ? { ...reader, async resolveModule(source: ProcessAssetSource) {
    const pkg = await reader.resolveModule(source);
    if (pkg.definition.payload.category === 'media' && lodSource(pkg.definition.payload.data, lod)?.kind !== 'media_ref'
      && activeSource.current === sourceKey && phaseCurrent.current === phaseKey) {
      setResolvedDeclaration({ key: phaseKey, owner: phaseOwner, presentation: pkg.definition.payload.data });
    }
    return pkg;
  } } : null, [reader, sourceKey, lod, phaseKey, phaseOwner]);
  const declared = !staticSelection && value?.definition.payload.category === 'devices' ? value.definition.payload.data.operations : [];
  const declarationPresentation = useMemo(() => {
    if (!value) return null;
    if (resolvedDeclaration && (resolvedDeclaration.key === phaseKey || lod === 'region' && resolvedDeclaration.owner === phaseOwner)) return resolvedDeclaration.presentation;
    try { return lodSource(value.presentation, lod)?.kind === 'media_ref' || lod === 'region' && value.presentation.source.kind === 'media_ref' ? null : value.presentation; }
    catch (failure) { if (failure instanceof UnsupportedMedia) return null; throw failure; }
  }, [value, resolvedDeclaration, phaseKey, phaseOwner, lod]);
  const phases = declarationPresentation ? [...new Set(declarationPresentation.timeline.map(clip => clip.phase_ref ?? 'cycle'))] : [];
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
  const currentNode = useRef<ProcessNodeSelection | null>(null);
  const nodeContext = nodeSelection?.lane === 'editor' ? (nodeSelection.value.context.selection.kind !== 'definition' ? context : declarationCurrent.current) : context;
  currentNode.current = nodeSelection && nodeSelection.generation === nodeGeneration.current && nodeContext && processContextKey(nodeContext) === processContextKey(nodeSelection.value.context)
    && (nodeSelection.lane === 'editor' ? !!editing : nodeSelection.value.lod === lod) ? nodeSelection.value : null;
  const nodeReader = useMemo(() => createNodeInspectorReader(() => currentNode.current), []);
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
    declaration: { ...declarationReader, nodeOwner: effectiveEditorDraft ? { object_id: { kind: 'definition_draft', id: effectiveEditorDraft.summary.draft_id }, source: { kind: 'draft' } }
      : editorDefinition ? { object_id: { kind: 'module', id: editorDefinition.id }, source: declarationReader.binding.request.source } : undefined,
      rights: effectiveEditorDraft?.definition.contribution.assets ?? editorInput?.editor.source?.rights ?? [] },
    actual: editorActual && reader && mediaInput ? { binding: mediaInput.value.binding, readAsset: reader.readAsset, resolveModule: reader.resolveModule,
      rights: mediaInput.value.rights, presentation: mediaInput.value.presentation, frame: editorActual.frame, nodeOwner: mediaInput.value.node_owner } : null,
    unavailable_reason: '尚未取得此Run同源的实际Job历史、完整物品页与冻结表现；不借最终Job或当前草稿补画。',
    onActualSeek: at_ms => { setPlayback(value => ({ ...value, playing: false })); seekTo(at_ms); },
    onInspectItem: props.onInspectItem ? item_id => { void inspectItem(item_id); } : undefined,
    onSelectNode: selected => selectNode(selected, 'editor'),
  }) : undefined, [declarationReader, effectiveEditorDraft, editorInput?.editor.source, editorActual, reader, mediaInput, key, props.onInspectItem]);

  useEffect(() => {
    latest.current = context; active.current = key;
    return () => { latest.current = null; active.current = ''; mediaCurrent.current = null; activeSource.current = ''; declarationCurrent.current = null; currentNode.current = null; };
  }, []);
  useEffect(() => {
    let live = true; setError(''); setMessage('');
    if (!context) return;
    host.read(context).then(value => { if (live && active.current === key) setLoaded({ key, context, refresh, value }); }, failure => {
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
    if (selection.kind === 'definition') return;
    const next: DeviceProcessSelection = { ...selection, at_ms };
    emittedSelection.current = JSON.stringify(next);
    setSeek({ input: incoming, value: at_ms }); props.onSelectionChanged?.(next);
  }
  function dirtyChanged(dirty: boolean) { setNodeDirty(dirty); props.onNodeDirtyChange?.(dirty); }
  function navigate(run: () => void) {
    if (nodeDirty) { setLeaveNode({ run }); return; }
    currentNode.current = null; nodeGeneration.current++;
    setInlineEdit(null); setNodeSelection(null); run();
  }
  function selectNode(selected: MediaNodeSelection, lane: 'main' | 'editor') {
    const sourceContext = lane === 'editor' && selected.byte_source.kind === 'module_version' ? declarationCurrent.current : latest.current;
    if (!sourceContext) return;
    if (selected.byte_source.kind === 'run_module_version' && sourceContext.selection.kind === 'actual_run') {
      if (loaded?.key !== key) return;
      // Node inspection reads the displayed frame's exact time. Rebase only
      // this same Run's immutable node provenance; no Host boundary is relaxed.
      const { run_id, at_ms } = sourceContext.selection, source = selected.owner.source, bytes = selected.byte_source;
      if (bytes.data.run_id === run_id && (source.kind === 'run_snapshot' || source.kind === 'run_module_version') && source.data.run_id === run_id) {
        const ownerSource = source.kind === 'run_snapshot'
          ? { kind: 'run_snapshot' as const, data: { ...source.data, at_ms } }
          : { kind: 'run_module_version' as const, data: { ...source.data, at_ms } };
        selected = { ...selected, owner: { ...selected.owner, source: ownerSource },
          byte_source: { ...bytes, data: { ...bytes.data, at_ms } } };
      }
    }
    navigate(() => { setPlayback(value => ({ ...value, playing: false })); setNodeSelection({ value: { ...selected, context: sourceContext }, lane, generation: ++nodeGeneration.current }); });
  }
  async function editNode(request: NodeEditRequest) {
    if (editing) throw new Error('原图片过程编辑器仍打开；请先通过其未保存确认关闭，当前输入与节点检查保留。');
    if (!currentNode.current || nodeSelectionKey(currentNode.current) !== nodeSelectionKey(request.selection)) throw new ProcessSourceChanged();
    if (request.target.kind === 'device_override' && !request.target.frozen) {
      if (!props.onApplyNode || !props.workspace.project) throw new Error('原Project节点PutObject入口尚未接通。');
      setInlineEdit({ inspection: request, base: props.workspace.project }); return;
    }
    if (!props.onEditNodeSource) throw new Error('原定义创作入口尚未接通。');
    const captured = nodeSelectionKey(request.selection), generation = nodeGeneration.current;
    const assertCurrent = () => { if (nodeGeneration.current !== generation || !currentNode.current || nodeSelectionKey(currentNode.current) !== captured) throw new ProcessSourceChanged(); };
    assertCurrent();
    await props.onEditNodeSource({ ...request, assertCurrent });
  }
  const nodePanel = <>{nodeSelection && !inlineEdit && <NodeInspector key={nodeSelection.generation} selection={nodeSelection.value} read={nodeReader.read}
    onEditSource={editNode} onClose={() => navigate(() => setNodeSelection(null))}/>}
    {nodeSelection && !inlineEdit && !currentNode.current && <p role="alert">节点检查属于原来源；当前Scope、档位或时点已变化，请重新选择。原结果不作为当前来源编辑。</p>}
    {inlineEdit && <InlineNodeEditor inspection={inlineEdit.inspection} base={inlineEdit.base} current={!!currentNode.current}
      currentContext={context} onApply={props.onApplyNode!} onDirtyChange={dirtyChanged}
      onClose={() => { dirtyChanged(false); setInlineEdit(null); setNodeSelection(null); }}/ >}
    {leaveNode && <div role="alertdialog" aria-label="离开节点编辑"><p>节点有未应用输入，当前输入保留。</p><button onClick={() => setLeaveNode(null)}>保留节点输入</button><button onClick={() => { const run = leaveNode.run; dirtyChanged(false); setInlineEdit(null); setNodeSelection(null); setLeaveNode(null); run(); }}>放弃节点输入并继续</button></div>}</>;
  // This clock only requests recorded views. It never runs/resumes a Scenario or
  // writes Commands. Declaration u has its own explicit 4-second viewing cycle.
  useEffect(() => {
    if (!playback.playing || editing || !value || staticSelection) return;
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
  }, [playback.playing, playback.rate, playback.loop, actual, value, inspection, !!editing, staticSelection]);

  async function invoke(action: DeviceAction) {
    if (!context || !value || loaded?.key !== key || acting) return;
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
    if (!context || loaded?.key !== key || !props.onInspectItem) return;
    const captured = key;
    try { await props.onInspectItem({ context, item_id }); }
    catch (failure) { if (active.current === captured) setError(failure instanceof Error ? failure.message : String(failure)); }
  }
  if (editing && editorInput) return <section className="sf-device-process sf-device-process-editor" aria-label="设备过程原草稿编辑">
    {editing.key !== currentEditorKey && !createdHere && <p role="alert">当前工程或定义已切换，原编辑输入仍保留。请在编辑器中选择保留或放弃；保存时仍检查原编辑范围。</p>}
    <ImageTimelineEditor {...editorInput.editor} workspace={editorInput.workspace} run={editorActual?.run ?? null}
      observation={editorActual?.view_snapshot ?? null}
      media={editorMedia}
      instance={editorActual?.device ? { id: editorActual.device.id, module_ref: editorActual.device.module_ref } : editorInput.editor.instance}
      onDraftChanged={draft => { if (!editing.editor.draft) createdDraft.current = draft; editorInput.editor.onDraftChanged(draft); }}
      onClose={() => setEditing(null)} />
    {nodePanel}
  </section>;

  return <section className="sf-device-process" aria-label="设备过程工作区">
    <header className="sf-device-process-header"><div><h2>{staticSelection ? '静态外观检查' : '设备过程'}</h2><p>{value?.device?.title ?? value?.definition.title ?? '读取设备过程来源'}</p></div>
      <div className="sf-device-process-header-actions">{playback.playing && <button aria-label="暂停设备观看" onClick={() => setPlayback(value => ({ ...value, playing: false }))}>暂停观看</button>}
        {!staticSelection && <button disabled={!props.editor || acting} onClick={() => navigate(openEditor)}>图片过程与标签编辑</button>}
        {!staticSelection && props.onReturnToAuthoring && <button onClick={() => navigate(() => props.onReturnToAuthoring?.(props.editor?.draft ?? null))}>返回原定义工作区</button>}
        <button onClick={() => navigate(props.onClose)}>返回工作台</button></div></header>
    <div className="sf-device-process-source" aria-label="过程来源"><strong>{staticSelection ? '已保存运行的静态外观' : actual ? '已保存运行的设备过程' : '当前设备定义与声明动作'}</strong><details><summary>来源与版本详情</summary><span>桌面会话 {context?.session_id ?? '未连接'}</span><span>工程 {context?.project_id ?? '未打开'}</span>
      <span>{props.scope.kind === 'project' ? 'Project' : `Draft ${props.scope.data}`} / 授权修订 r{props.scopeRevision}</span>
      <span>{value ? `精确模块 ${refText(value.module_ref)}` : selection.kind === 'definition' ? refText(selection.module_ref) : `Run ${selection.run_id}${selection.kind === 'actual_run' ? ` / Job ${selection.job_id}` : selection.kind === 'frozen_device' ? ` / 冻结设备 ${selection.device_id}` : ' / 冻结定义'}`}</span>
      {value?.device && !actual && !frozen && <span>当前实例 {value.device.id} → {refText(value.device.module_ref)} · {value.device.presentation_override ? '已采用表现覆盖' : '库定义表现'}</span>}
      {frozen && <><span>冻结静态输入 r{frozen.run.input_ref.project_revision} · {frozen.device ? `设备 ${frozen.device.id} → ${refText(frozen.device.module_ref)}` : '独立工艺样例的原定义，未采用到工厂设备'}</span><span>声明局部变换，不是运动后世界姿态</span></>}
      {actual && <><span>实例 {actual.device ? `${actual.device.id} → ${refText(actual.device.module_ref)}` : '独立工艺样例，未采用到工厂设备'}</span>
        <span>冻结输入 r{actual.run.input_ref.project_revision} / 工艺 {refText(actual.job.operation_ref)}</span></>}
    </details></div>
    <div className="sf-device-process-tools"><label>显示档位 <select aria-label="过程显示档位" value={lod} onChange={event => { const next = event.target.value as LodLevel; navigate(() => setLod(next)); }}><option value="detailed">详细</option><option value="simplified">简化</option><option value="region">区域</option></select></label>
      {actual && <label><input aria-label="仅检查冻结声明" type="checkbox" checked={inspection} onChange={event => { const checked = event.target.checked; navigate(() => { setInspection(checked); setPlayback(value => ({ ...value, playing: false })); }); }} />仅检查本次运行保存的静态定义</label>}
      {actual && props.onSelectionChanged && context && <button onClick={() => navigate(() => { setPlayback(value => ({ ...value, playing: false })); if (context.scope.kind === 'definition_draft') props.onSelectionChanged?.({ kind: 'frozen_definition', run_id: actual.run.id, at_ms: actual.view_snapshot.at_ms }); else if (actual.device) props.onSelectionChanged?.({ kind: 'frozen_device', run_id: actual.run.id, at_ms: actual.view_snapshot.at_ms, device_id: actual.device.id }); })}>检查冻结静态输入</button>}
      {!staticSelection && (!actual || inspection) && <label>声明阶段 <select aria-label="过程声明阶段" disabled={!declarationPresentation} value={displayPhase} onChange={event => { const phase = event.target.value; navigate(() => { setPhase(phase); setU(0); setPlayback(value => ({ ...value, playing: false })); }); }}>{declarationPresentation ? (phases.length ? phases : ['cycle']).map(id => <option key={id} value={id}>{{cycle:'完整周期',idle:'待机',working:'加工',output_wait:'等待出料'}[id] ?? id}</option>) : <option value="cycle">读取精确媒体阶段…</option>}</select></label>}
      {selection.kind !== 'definition' && <><label>{staticSelection ? '静态检查时点' : '实际观看时点'} <input aria-label="过程时点（毫秒）" type="number" value={selection.at_ms} onChange={event => { if (event.target.value !== '') { const at = Number(event.target.value); navigate(() => { setPlayback(value => ({ ...value, playing: false })); seekTo(at); }); } }} /> 毫秒</label><button onClick={() => navigate(() => setRefresh(value => value + 1))}>重读实际来源</button></>}
    </div>
    {actual && <p className="sf-device-process-frontier" aria-label="实际计算边界">观看 {actual.view_snapshot.at_ms/1000} 秒 · 已计算至 {actual.run.computed_until_ms == null ? '未知' : actual.run.computed_until_ms/1000} 秒 · {runStatusLabel[actual.run.computation_status]}</p>}
    {actual && selection.kind === 'actual_run' && loaded?.key !== key && <p role="status" aria-label="过程帧更新">正在读取 {selection.at_ms} 毫秒；画面保留 {actual.view_snapshot.at_ms} 毫秒的完整记录。</p>}
    {frozen && selection.kind !== 'definition' && <p className="sf-device-process-frontier" aria-label="静态计算边界">静态检查 {selection.at_ms/1000} 秒 · 已计算至 {frozen.run.computed_until_ms == null ? '未知' : frozen.run.computed_until_ms/1000} 秒。静态外观不提供作业阶段、加工进度或出料数量。</p>}
    {!context && <p role="status">先打开实际工程，才能读取设备过程。</p>}
    {error && <p role="alert" className="sf-device-process-error">{error}</p>}{message && <p role="status">{message}</p>}
    {context && !value && !error && <p role="status">正在读取当前设备过程…</p>}
    {value && context && viewReader && mediaInput && <MediaPresentationView presentation={mediaInput.value.presentation} binding={mediaInput.value.binding} readAsset={viewReader.readAsset} resolveModule={viewReader.resolveModule} sourceProject={actual?.raw.frozen_project ?? frozen?.raw_static.frozen_project ?? null}
      nodeOwner={mediaInput.value.node_owner}
      lod={lod} rights={mediaInput.value.rights} playback={playback} onPlaybackChange={setPlayback}
      onSelectNode={loaded?.key === key ? selected => selectNode(selected, 'main') : undefined}
      onInspectItem={props.onInspectItem && loaded?.key === key ? item_id => { void inspectItem(item_id); } : undefined}
      onAction={loaded?.key === key && declared.some(action => props.actions[action]) && !acting ? action => { void invoke(action); } : undefined}
      {...frozen ? { mode: 'asset_inspection', inspectionLabel: `已保存的静态外观 · ${selection.kind !== 'definition' ? selection.at_ms/1000 : ''} 秒 · 显示定义姿态` }
        : actual && !inspection ? { mode: 'actual_run', frame: actual.frame, onSeek: at_ms => { setPlayback(value => ({ ...value, playing: false })); seekTo(at_ms); } }
        : { mode: 'declared_preview', preview: { phase_ref: displayPhase, view_u: u }, onPreviewSeek: preview => { setPlayback(value => ({ ...value, playing: false })); setPhase(preview.phase_ref); setU(preview.view_u); } }} />}
    {nodePanel}
    <div className="sf-device-process-foot"><aside><h3>声明操作与实际入口</h3>{(Object.keys(actionNames) as DeviceAction[]).map(action => <p key={action}><strong>{actionNames[action]}</strong><span>{declared.includes(action) && props.actions[action] ? '可使用' : '暂不可用：缺少操作定义或可用入口'}</span></p>)}
      <p>{staticSelection ? '静态外观用于查看节点和创建新定义，此处不提供加工进度或产量。' : '配置参数、定位设备或运行独立工艺样例请使用上方操作；观看过程不会派工。'}</p></aside>
      {frozen && <details><summary>冻结静态输入来源</summary><p>原Run {frozen.run.id}；冻结Project r{frozen.raw_static.frozen_project.revision}；元数据拥有者 {frozen.node_owner.object_id.kind}/{frozen.node_owner.object_id.id}。未读取或创建Job帧。</p><pre>{JSON.stringify(frozen.raw_static.frozen_owner, null, 2)}</pre></details>}
      {actual && <details><summary>原始读取与只读视图来源</summary><dl><dt>原始Run</dt><dd>{actual.raw.run.id} · 原完整结果保留</dd><dt>显式Job ObjectsAt</dt><dd>{actual.raw.job_pages.length} 页 · {actual.job.id} @ {actual.view_snapshot.at_ms} ms</dd><dt>原空对象查询</dt><dd>{actual.raw.item_pages.length} 页 · 只消费实际Item</dd><dt>冻结Project</dt><dd>{actual.raw.frozen_project.id} r{actual.raw.frozen_project.revision} · {actual.raw.frozen_project.resources.length} 条原资源声明</dd><dt>呈现快照</dt><dd>本服务只读组合，非Host原始单页；资源占用只取Run.resource_claims和此刻Job持有。</dd></dl>
        <details><summary>原始Job页</summary><pre>{JSON.stringify(actual.raw.job_pages, null, 2)}</pre></details>
        <details><summary>原始Item页</summary><pre>{JSON.stringify(actual.raw.item_pages, null, 2)}</pre></details>
      </details>}
    </div>
  </section>;
}
