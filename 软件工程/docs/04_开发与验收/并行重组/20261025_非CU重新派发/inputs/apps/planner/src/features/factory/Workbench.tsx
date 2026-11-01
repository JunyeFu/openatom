import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import type { RunObjectsPage } from '../../../../../packages/commands/generated/typescript/RunObjectsPage';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { HistoryAvailability } from '../../../../../packages/commands/generated/typescript/HistoryAvailability';
import type { WorkspaceSnapshot, DepartureDecision, EditCommand, TaskHandle, ImportDraft, ProjectState } from '../../services/projectHost';
import * as host from '../../services/projectHost';
import { useWindowClose, type WindowCloseDecision } from '../../services/windowClose';
import type { PresentationEditorState } from '../presentation-editor';
import { FactoryScene, type LodMode } from '../../scene/FactoryScene';
import { deviceHallPosition } from '../../scene/layout';
import { ObjectEditor, type EditorTarget } from './ObjectEditor';
import { ObjectTreeBranch } from './ObjectTreeBranch';
import { HostFailureDetails } from './HostFailureDetails';
import { FactoryCheckWorkspace } from './FactoryCheckWorkspace';
import { LayoutSelectionEditor } from './LayoutSelectionEditor';
import { LayoutTransformProposal, type TransformProposal } from './LayoutTransformProposal';
import { ungroupLayoutSelection, layoutSelection } from '../../scene/layoutEditing';
import type { LayoutControlOptions } from '../../scene/layoutControls';
import type { Pose } from '../../../../../packages/commands/generated/typescript/Pose';
import { ConnectionEditor } from './ConnectionEditor';
import { WorkbenchModal } from './WorkbenchModal';
import { ProcessSampleChooser, type ProcessSampleChoice } from './ProcessSampleChooser';
import { timelineProcessJob } from './timelineProcessJob';
import { ModuleLibrary } from './ModuleLibrary';
import { BusinessEditor } from '../business/BusinessEditor';
import type { BusinessEditIntent } from '../business/types';
import type { BusinessCommitProposal, BusinessCommitResult, BusinessRunRequest, BusinessLocateRequest } from '../business/types';
import { DeliveryWorkspace } from '../reporting/DeliveryWorkspace';
import { PackagingWorkspace } from '../packaging/PackagingWorkspace';
import type { PackagingScope, PackagingEvidence } from '../packaging/types';
import type { FileRef } from '../../../../../packages/commands/generated/typescript/FileRef';
import type { ProjectContext } from '../../../../../packages/commands/generated/typescript/ProjectContext';
import { readStoredPackaging, listStoredPackaging, readStoredManufacturingFile } from '../../services/storedPackaging';
import { startManufacturingPackageExport } from '../../services/manufacturingExport';
import { DeviceProcessWorkspace, type ProcessActions } from './DeviceProcessWorkspace';
import type { DeviceProcessSelection, ProcessReadContext, ProcessScope } from '../../services/presentationHost';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleDefinition } from '../../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { RunView } from '../../../../../packages/commands/generated/typescript/RunView';
import { ModuleAuthoringWorkspace } from '../modules';
import { ItemLayoutWorkspace, type ItemLayoutOwner } from '../item-layout';
import { AgentWorkspace, agentEditorIdentity, type AgentEditorState } from '../agent';
import type { NodeSourceEditRequest } from '../node-inspector';
import type { AuthoringDraft, AuthoringIO, AuthoringNavigationGuard } from '../modules/types';
import { authoringRequest, readDraft, assertPackagingDefinition } from '../modules/model';
import { createModuleAuthoringIO, createAuthoringSamples, createAuthoringTemplates } from '../../services/moduleAuthoring';
import { AuthoringPresentationPreview } from '../../scene/AuthoringPresentationPreview';
import { authoringPresentationSource, type AuthoringPresentationPreviewValue } from '../../services/authoringPresentation';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ApiCategory } from '../../../../../packages/commands/generated/typescript/ApiCategory';
import { createFactoryPresentation, factoryContextKey, type FactoryPresentationContext, type FactoryPresentationSource } from '../../services/factoryPresentation';
import type { DeliveryServices } from '../comparison/types';
import { prepareScenarioFork } from '../comparison/model';
import type { MaterialConnection } from '../../../../../packages/commands/generated/typescript/MaterialConnection';
import { RunTimeline } from '../timeline/RunTimeline';
import { observedItems, resolveFollowedItemSelection } from '../timeline/projection';
import type { Props as TimelineProps, TimelineInputCommit, TimelineCommitResult, TimelineSelection, TimelineComparison } from '../timeline/types';
import { decimalText, itemAnchor, objectHall, transportPhaseAt } from '../../scene/runtime';

const kindLabel: Record<string, string> = { hall: '厂房', line: '流水线', device: '设备', location: '库位', resource: '资源', scenario: '方案', connection: '连接', transport: '运输批次' };
const same = (a: ObjectRef | null, b: ObjectRef) => a?.id === b.id && a.kind === b.kind;

export function Workbench() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [failure, setFailure] = useState<Error | null>(null);
  const [selection, setSelection] = useState<ObjectRef | null>(null);
  const [selections, setSelections] = useState<ObjectRef[]>([]);
  const [layoutMode, setLayoutMode] = useState<'select' | 'translate' | 'rotate'>('select');
  const [snapLayout, setSnapLayout] = useState(false);
  const [snapStep, setSnapStep] = useState('0.5');
  const [layoutEditor, setLayoutEditor] = useState<{ mode: 'group' | 'copy'; project: ProjectState; selection: ObjectRef[] } | null>(null);
  const [transformProposal, setTransformProposal] = useState<TransformProposal | null>(null);
  const [layoutDirty, setLayoutDirty] = useState(false);
  const [checkOpen, setCheckOpen] = useState(false);
  const [checkDirty, setCheckDirty] = useState(false);
  const [canvas, setCanvas] = useState<'line_canvas' | 'multi_hall_canvas'>('line_canvas');
  const [hallId, setHallId] = useState<string | null>(null);
  const [lod, setLod] = useState<LodMode>('detailed');
  const [autoLod, setAutoLod] = useState(false);
  const [resetKey, setResetKey] = useState(0);
  const [leftWidth, setLeftWidth] = useState(256);
  const [rightWidth, setRightWidth] = useState(400);
  const [leftVisible, setLeftVisible] = useState(() => window.innerWidth >= 1050);
  const [rightVisible, setRightVisible] = useState(() => window.innerWidth >= 1050);
  const [timelineVisible, setTimelineVisible] = useState(true);
  const [descriptionVisible, setDescriptionVisible] = useState(true);
  const [dockHeight, setDockHeight] = useState(160);
  const [editor, setEditor] = useState<{ target: EditorTarget; base: WorkspaceSnapshot } | null>(null);
  const [connectionEditor, setConnectionEditor] = useState<{ value?: MaterialConnection; base: WorkspaceSnapshot } | null>(null);
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [libraryDirty, setLibraryDirty] = useState(false);
  const [agentOpen, setAgentOpen] = useState(false);
  const [agentEditorState, setAgentEditorState] = useState<AgentEditorState | null>(null);
  const [libraryModule, setLibraryModule] = useState<ModuleRef>();
  const [authoring, setAuthoring] = useState<{ id: string; module?: ModuleRef; draft?: AuthoringDraft } | null>(null);
  const [authoringDraft, setAuthoringDraft] = useState<AuthoringDraft | null>(null);
  const [authoringPresentation, setAuthoringPresentation] = useState<AuthoringPresentationPreviewValue | null>(null);
  const authoringNavigation = useRef<AuthoringNavigationGuard | null>(null);
  const registerAuthoringNavigation = useCallback((guard: AuthoringNavigationGuard | null) => { authoringNavigation.current = guard; }, []);
  const [businessOpen, setBusinessOpen] = useState(false);
  const [businessEditIntent, setBusinessEditIntent] = useState<BusinessEditIntent>();
  const [deliveryTab, setDeliveryTab] = useState<'comparison' | 'reporting' | 'handoff' | null>(null);
  const [packagingScope, setPackagingScope] = useState<PackagingScope | null>(null);
  const [packagingContext, setPackagingContext] = useState<ProjectContext | null>(null);
  const [deviceProcess, setDeviceProcess] = useState<{ id: string; scope: ProcessScope['scope']; selection: DeviceProcessSelection; draft: AuthoringDraft | null; module: ModulePackage | null } | null>(null);
  const [processSampleChoice, setProcessSampleChoice] = useState<ProcessSampleChoice | null>(null);
  const [itemLayout, setItemLayout] = useState<{ initialOwner?: ItemLayoutOwner } | null>(null);
  const [nodeDirty, setNodeDirty] = useState(false);
  const [objectDirty, setObjectDirty] = useState(false);
  const [connectionDirty, setConnectionDirty] = useState(false);
  const [definitionDirty, setDefinitionDirty] = useState(false);
  const [itemLayoutDirty, setItemLayoutDirty] = useState(false);
  const [businessDirty, setBusinessDirty] = useState<{ project_id: string; dirty: boolean } | null>(null);
  const [processEditorState, setProcessEditorState] = useState<PresentationEditorState | null>(null);
  const [packagingEditorState, setPackagingEditorState] = useState<PresentationEditorState | null>(null);
  const nodeDirtyRef = useRef(false);
  const nodeRetainedInput = useRef<{ io: AuthoringIO; projectRevision: number } | null>(null);
  const receiveNodeDirty = useCallback((dirty: boolean) => { nodeDirtyRef.current = dirty; setNodeDirty(dirty); }, []);
  const deviceProcessRef = useRef(deviceProcess); deviceProcessRef.current = deviceProcess;
  const [committedHistory, setCommittedHistory] = useState<{ projectId: string; revision: number; value: HistoryAvailability } | null>(null);
  const editing = agentOpen || checkOpen || layoutEditor !== null || transformProposal !== null || editor !== null || connectionEditor !== null || libraryOpen || businessOpen || deliveryTab !== null || authoring !== null || packagingScope !== null || deviceProcess !== null || itemLayout !== null;
  const [newDialog, setNewDialog] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [startIntent, setStartIntent] = useState<'empty' | 'template' | 'demonstration'>('empty');
  const [libraryCategory, setLibraryCategory] = useState<ApiCategory>();
  const [propertyTextScale, setPropertyTextScale] = useState(1);
  const returnFocus = useRef<HTMLElement | null>(null);
  const overlayWasOpen = useRef(false);
  const [departure, setDeparture] = useState<DepartureDecision>('stay');
  const [windowClosePrompt, setWindowClosePrompt] = useState<{ resolve: (decision: WindowCloseDecision) => void } | null>(null);
  const [opening, setOpening] = useState<{ task: TaskHandle; draft: ImportDraft } | null>(null);
  const [allowMissing, setAllowMissing] = useState(false);
  useLayoutEffect(() => {
    const open = editing || newDialog || opening !== null;
    if (overlayWasOpen.current && !open) {
      const previous = returnFocus.current;
      const trigger = previous?.isConnected ? previous : previous?.id ? document.getElementById(previous.id) : null;
      if (trigger instanceof HTMLElement && !trigger.matches(':disabled')) trigger.focus({ preventScroll: true });
      returnFocus.current = null;
    }
    overlayWasOpen.current = open;
  }, [editing, newDialog, opening]);
  const [taskState, setTaskState] = useState<TaskState | null>(null);
  const [scenarioId, setScenarioId] = useState('');
  const [run, setRun] = useState<RunResult | null>(null);
  const [runProject, setRunProject] = useState<ProjectState | null>(null);
  const [runInputs, setRunInputs] = useState<Awaited<ReturnType<typeof host.readRunInputs>> | null>(null);
  const [comparison, setComparison] = useState<TimelineComparison | null>(null);
  const [observationPage, setObservationPage] = useState<RunObjectsPage | null>(null);
  const [storedTimelineSelection, setTimelineSelection] = useState<TimelineSelection | null>(null);
  const [atMs, setAtMs] = useState<number | null>(null);
  const [source, setSource] = useState<'draft' | 'run'>('draft');
  const observation = useRef(0);
  const projectIdentity = useRef<string | null>(null);
  const pendingRun = useRef<string | null>(null);
  const primaryRun = useRef<string | null>(null);
  const comparisonEpoch = useRef(0);
  const actionCount = useRef(0);
  const refreshEpoch = useRef(0);
  const workspaceRef = useRef(snapshot);
  workspaceRef.current = snapshot;
  const project = snapshot?.project ?? null;
  const agentIdentity = agentOpen && snapshot ? agentEditorIdentity(snapshot) : null;
  const agentIdentityRef = useRef(agentIdentity); agentIdentityRef.current = agentIdentity;
  const receiveAgentEditorState = useCallback((state: AgentEditorState) => { if (state.identity === agentIdentityRef.current) setAgentEditorState(state); }, []);
  const displayedProject = source === 'run' ? runProject : project;
  const displayedModules = source === 'run' ? runInputs?.modules : snapshot?.modules;
  const runObjects = observationPage?.objects ?? [];
  const items = runObjects.filter(value => value.kind === 'item').map(value => value.data);
  const connected = host.nativeHostAvailable();
  const processEditorIdentity = deviceProcess && snapshot ? `${snapshot.context.host.session_id}/${project?.id ?? ''}/${deviceProcess.draft?.summary.draft_id ?? `${deviceProcess.module?.definition.id}@${deviceProcess.module?.definition.version}`}` : null;
  const processEditorIdentityRef = useRef(processEditorIdentity); processEditorIdentityRef.current = processEditorIdentity;
  const receiveProcessEditorState = useCallback((value: PresentationEditorState) => {
    if (value.identity === processEditorIdentityRef.current) setProcessEditorState(value);
  }, []);
  const packagingEditorIdentity = packagingScope && snapshot ? JSON.stringify([snapshot.context.host.session_id, project?.id, packagingScope]) : null;
  const packagingEditorIdentityRef = useRef(packagingEditorIdentity); packagingEditorIdentityRef.current = packagingEditorIdentity;
  const receivePackagingEditorState = useCallback((value: PresentationEditorState) => {
    if (value.identity === packagingEditorIdentityRef.current) setPackagingEditorState(value);
  }, []);
  const commands = snapshot?.capabilities.available_command_kinds ?? [];
  const runs = snapshot?.context.run_summaries ?? [];
  const definitionDraftIds = [...new Set(snapshot?.capabilities.implemented_operations
    .filter(operation => operation.operation === 'get_context')
    .flatMap(operation => operation.allowed_scopes)
    .filter(scope => scope.kind === 'definition_draft')
    .map(scope => scope.data) ?? [])];

  const factoryReadCurrent = useRef<FactoryPresentationContext | null>(null);
  factoryReadCurrent.current = connected && snapshot && project && displayedProject ? {
    session_id: snapshot.context.host.session_id, project_id: project.id, scope: { kind: 'project' },
    scope_revision: project.revision, project: displayedProject,
    view: source === 'run' && run && atMs !== null ? { run_id: run.id, at_ms: atMs } : null,
    preparedRun: source === 'run' && run && runInputs ? { run, modules: runInputs.modules } : undefined,
  } : null;
  const factoryCurrent = useRef<FactoryPresentationContext | null>(null);
  factoryCurrent.current = editing ? null : factoryReadCurrent.current;
  const factoryKey = factoryCurrent.current ? factoryContextKey(factoryCurrent.current) : null;
  const factoryReader = useMemo(() => createFactoryPresentation(() => factoryCurrent.current), []);
  const [factorySource, setFactorySource] = useState<FactoryPresentationSource | null>(null);
  const visibleFactorySource = factorySource?.key === factoryKey ? factorySource : null;
  useEffect(() => {
    const captured = factoryCurrent.current;
    if (!captured) { setFactorySource(null); return; }
    // A seek publishes the actual observation and its presentation together.
    if (factorySource?.key === factoryKey) return;
    let live = true;
    factoryReader.read(captured).then(value => {
      if (live && factoryCurrent.current && value.key === factoryContextKey(factoryCurrent.current)) setFactorySource(value);
    }, error => {
      if (live && factoryCurrent.current && factoryContextKey(factoryCurrent.current) === factoryContextKey(captured)) setFailure(error instanceof Error ? error : new Error(String(error)));
    });
    return () => { live = false; };
  }, [factoryReader, factoryKey]);

  async function refresh(resetResults = false) {
    const epoch = ++refreshEpoch.current;
    if (resetResults) { observation.current++; pendingRun.current = null; }
    const next = await host.readWorkspace();
    if (epoch !== refreshEpoch.current) throw new Error('旧的工程读取已被较新的读取取代，请在当前工程继续。');
    const changedProject = projectIdentity.current !== (next.project?.id ?? null);
    if (changedProject) {
      setSelections([]); setLayoutMode('select');
      setBusinessOpen(false); setBusinessEditIntent(undefined);
      setAuthoring(null); setAuthoringDraft(null); setAuthoringPresentation(null);
      setPackagingScope(null); setPackagingContext(null);
      if (!nodeDirtyRef.current) setDeviceProcess(null);
    }
    const reset = resetResults || changedProject || (run !== null && !next.context.run_summaries?.some(summary => summary.run_id === run.id));
    if (reset) {
        observation.current++; comparisonEpoch.current++; pendingRun.current = null; primaryRun.current = null; setComparison(null); setSelection(null); setTimelineSelection(null); setRun(null); setRunProject(null); setRunInputs(null); setObservationPage(null); setAtMs(null); setSource('draft'); setHallId(null);
    } else if (comparison && !next.context.run_summaries?.some(summary => summary.run_id === comparison.run.id)) {
        comparisonEpoch.current++; setComparison(null);
    }
    projectIdentity.current = next.project?.id ?? null;
    setCommittedHistory(current => current?.projectId === next.project?.id && current?.revision === next.project?.revision ? current : null);
    workspaceRef.current = next;
    setSnapshot(next);
    const graph = !reset && source === 'run' ? runProject : next.project;
    setHallId(current => graph?.halls.some(hall => hall.id === current) ? current : graph?.halls[0]?.id ?? null);
    setScenarioId(current => next.scenarios.some(scenario => scenario.id === current) ? current : next.scenarios[0]?.id ?? '');
    return next;
  }

  const refreshLatest = useRef(refresh);
  refreshLatest.current = refresh;

  const authoringIO = useMemo(() => snapshot?.project
    ? createModuleAuthoringIO(() => workspaceRef.current) : null,
  [snapshot?.context.host.session_id, project?.id]);
  const authoringTemplates = useMemo(() => authoringIO
    ? createAuthoringTemplates(() => workspaceRef.current, authoringIO) : null,
  [authoringIO, snapshot?.capabilities]);
  const authoringSamples = createAuthoringSamples(() => workspaceRef.current, authoringDraft, authoringIO ?? undefined);
  if (deviceProcess && authoringIO && project && !nodeDirty) nodeRetainedInput.current = { io: authoringIO, projectRevision: project.revision };
  // Keep dirty local inputs mounted even when the actual Host has no active
  // Project. The workspace remains current; the retained IO still rejects it.
  const processAuthoringIO = authoringIO ?? (nodeDirty ? nodeRetainedInput.current?.io : null);
  const processProjectRevision = project?.revision ?? (nodeDirty ? nodeRetainedInput.current?.projectRevision : undefined);

  async function readPackagingContext(current: WorkspaceSnapshot, scope: PackagingScope) {
    if (!current.project || !authoringIO) throw new Error('请先打开实际工程。');
    if (scope.kind === 'project') return current.context;
    return authoringIO.dispatch<ProjectContext>({ api_version: '0.1', request_id: `packaging-context-${crypto.randomUUID()}`, project_id: current.project.id, scope, operation: 'get_context', params: { selection: [] } });
  }
  async function openPackaging(scope: PackagingScope = { kind: 'project' }) {
    const current = await refreshLatest.current();
    if (scope.kind === 'definition_draft') {
      const draft = await readDraft(authoringIO!, current.project!.id, scope.data);
      assertPackagingDefinition(draft.definition);
    }
    const context = await readPackagingContext(current, scope);
    closeAuthoring(); setDeliveryTab(null); setLibraryOpen(false);
    setPackagingContext(context); setPackagingScope(scope); setSource('draft');
  }
  async function refreshPackaging() {
    const current = await refreshLatest.current();
    if (packagingScope && current.project?.id === project?.id) setPackagingContext(await readPackagingContext(current, packagingScope));
  }
  const packagingWorkspace = snapshot && packagingContext ? { ...snapshot, context: packagingContext } : snapshot;
  const packagingRevision = packagingScope?.kind === 'definition_draft' ? packagingContext?.definition?.revision : project?.revision;
  const packagingReadKey = snapshot && project && packagingScope && packagingRevision !== undefined
    ? JSON.stringify([snapshot.context.host.session_id, project.id, project.revision, packagingScope, packagingRevision]) : null;
  const packagingReadKeyRef = useRef(packagingReadKey); packagingReadKeyRef.current = packagingReadKey;
  const readManufacturingFile = useCallback(async (evidence: PackagingEvidence, file: FileRef) => {
    const current = workspaceRef.current;
    if (!current?.project || !authoringIO || !packagingScope || packagingRevision === undefined || !packagingReadKey) throw new Error('请在当前实际专业Scope重新读取制造成果。');
    const assertCurrent = () => {
      if (packagingReadKeyRef.current !== packagingReadKey) throw new Error('制造图所属工程、Scope或修订已变化；丢弃旧响应。');
    };
    const checkHost = async () => {
      const actual = await authoringIO.dispatch<ProjectContext>(authoringRequest(current.project!.id, packagingScope, 'get_context', { selection: [] }));
      assertCurrent();
      const revision = packagingScope.kind === 'definition_draft' ? actual.definition?.revision : actual.project?.revision;
      if (actual.host.session_id !== current.context.host.session_id || actual.project?.id !== current.project!.id || actual.project.revision !== current.project!.revision || revision !== packagingRevision) throw new Error('实际Host的制造图来源已变化，请重读当前成果。');
    };
    await checkHost();
    const bytes = await readStoredManufacturingFile({ project_id: current.project.id, scope: packagingScope, scope_revision: packagingRevision, artifact_id: evidence.artifact.id, file });
    assertCurrent();
    await checkHost();
    return bytes;
  }, [authoringIO, packagingScope, packagingRevision, packagingReadKey]);
  async function packagingReadIdentity() {
    const current = workspaceRef.current;
    if (!current?.project || current.project.id !== project?.id || !packagingScope) throw new Error('工程或专业Scope已切换，请重新读取成果。');
    const context = await readPackagingContext(current, packagingScope);
    const revision = packagingScope.kind === 'definition_draft' ? context.definition?.revision : current.project.revision;
    if (revision === undefined) throw new Error('未读取到此实际专业Scope修订。');
    return { project_id: current.project.id, scope: packagingScope, scope_revision: revision };
  }

  function returnToAuthoring(draft: AuthoringDraft | null) {
    setDeviceProcess(null);
    if (draft) {
      setAuthoringDraft(draft); setAuthoringPresentation(null);
      setAuthoring(current => ({ id: current?.id ?? crypto.randomUUID(), draft }));
    }
  }
  async function openDraftProcess() {
    if (!authoringDraft || !project || !authoringIO) throw new Error('请先保存并读取实际定义草稿。');
    const actual = await readDraft(authoringIO, project.id, authoringDraft.summary.draft_id);
    const original = authoring?.module;
    let module: ModulePackage | null = null;
    if (original) {
      const object = await authoringIO.dispatch<DomainObject>({ api_version: '0.1', request_id: `process-original-${crypto.randomUUID()}`, project_id: project.id, scope: { kind: 'project' }, operation: 'get_object', params: { object_id: { kind: 'module', id: original.id }, source: { kind: 'module_version', data: { module_ref: original } }, node_index: null } });
      if (object.kind !== 'module') throw new Error('未读到原精确库来源。');
      module = object.data;
    }
    setAuthoringDraft(actual); setAuthoring(current => current ? { ...current, draft: actual } : current);
    setDeviceProcess({ id: crypto.randomUUID(), scope: { kind: 'definition_draft', data: actual.summary.draft_id }, selection: { kind: 'definition', module_ref: { id: actual.definition.id, version: actual.definition.version } }, draft: actual, module });
  }
  function openDeviceProcess() {
    if (!selectedDevice || !exactModule) throw new Error('尚未读取该设备的精确版本。');
    const actual = visibleFactorySource?.devices.find(device => device.device_id === selectedDevice.id);
    let selection: DeviceProcessSelection = { kind: 'definition', module_ref: selectedDevice.module_ref, device_id: selectedDevice.id };
    if (source === 'run') {
      if (!run || atMs === null || !actual || actual.projection === 'unavailable' || actual.unavailable) throw new Error(actual?.unavailable ?? '尚未读取所选设备此时点的真实冻结来源。');
      if (actual.projection === 'active') {
        if (!actual.frame?.job_id) throw new Error('此时点活动作业缺少精确Job身份，不能打开实际过程。');
        selection = { kind: 'actual_run', run_id: run.id, job_id: actual.frame.job_id, at_ms: atMs };
      } else selection = { kind: 'frozen_device', run_id: run.id, device_id: selectedDevice.id, at_ms: atMs };
    }
    setDeviceProcess({ id: crypto.randomUUID(), scope: { kind: 'project' }, selection, draft: null, module: exactModule });
  }
  function openTimelineJobProcess() {
    if (!selectedTimelineJob || !run || atMs === null || !runProject || selectedTimelineJob.execution_ref.kind !== 'project_device') throw new Error('请在当前实际 Run 中明确选择设备作业。');
    const deviceId = selectedTimelineJob.execution_ref.data.device_id;
    const device = runProject.devices.find(value => value.id === deviceId);
    const module = device && displayedModules?.get(host.moduleKey(device.module_ref));
    if (!device || !module) throw new Error('尚未读取该作业的冻结设备和精确版本。');
    setSelection({ kind: 'device', id: device.id });
    setDeviceProcess({ id: crypto.randomUUID(), scope: { kind: 'project' }, selection: { kind: 'actual_run', run_id: run.id, job_id: selectedTimelineJob.id, at_ms: atMs }, draft: null, module });
  }
  const processSamples = createAuthoringSamples(() => workspaceRef.current, deviceProcess?.draft ?? null, authoringIO ?? undefined);
  function receiveProcessDraft(draft: AuthoringDraft) {
    setDeviceProcess(current => current ? { ...current, draft, selection: current.scope.kind === 'definition_draft' && current.selection.kind === 'definition' ? { kind: 'definition', module_ref: { id: draft.definition.id, version: draft.definition.version } } : current.selection } : current);
    if (authoring?.draft?.summary.draft_id === draft.summary.draft_id) {
      setAuthoringDraft(draft); setAuthoring(current => current ? { ...current, draft } : current);
    }
  }
  async function receiveProcessSample(value: RunResult) {
    const current = deviceProcessRef.current, draft = current?.draft;
    if (!current || !draft || value.input_ref.project_id !== workspaceRef.current?.project?.id) return;
    if (!authoringIO || value.computed_until_ms == null) throw new Error('实际样例未提供原Host读取入口或计算边界。');
    const sessionId = workspaceRef.current.context.host.session_id;
    const scope: ProcessScope['scope'] = { kind: 'definition_draft', data: draft.summary.draft_id };
    function assertSampleOrigin() {
      const active = deviceProcessRef.current, workspace = workspaceRef.current;
      if (!active || active.id !== current!.id || workspace?.context.host.session_id !== sessionId || workspace.project?.id !== value.input_ref.project_id
        || active.draft?.summary.draft_id !== draft!.summary.draft_id || active.draft.summary.revision !== draft!.summary.revision
        || JSON.stringify(active.scope) !== JSON.stringify(current!.scope) || JSON.stringify(active.selection) !== JSON.stringify(current!.selection)) {
        throw new Error(`实际样例 ${value.id} 已产生，但过程来源已切换；请在原草稿读取该Run，不打开旧响应。`);
      }
    }
    async function checkSampleDraft() {
      const actual = await authoringIO!.dispatch<ProjectContext>(authoringRequest(value.input_ref.project_id, scope, 'get_context', { selection: [] }));
      assertSampleOrigin();
      if (actual.host.session_id !== sessionId || actual.project?.id !== value.input_ref.project_id
        || actual.definition?.draft_id !== draft!.summary.draft_id || actual.definition.revision !== draft!.summary.revision) {
        throw new Error(`实际样例 ${value.id} 已产生，但Host中的原定义草稿修订已变化；请重读来源，原输入保留。`);
      }
    }
    await checkSampleDraft();
    const frozen = await authoringIO.dispatch<DomainObject>(authoringRequest(value.input_ref.project_id, scope, 'get_object', {
      object_id: { kind: 'project', id: value.input_ref.project_id },
      source: { kind: 'run_snapshot', data: { run_id: value.id, at_ms: value.computed_until_ms } }, node_index: null,
    }));
    assertSampleOrigin();
    await checkSampleDraft();
    if (frozen.kind !== 'project' || frozen.data.id !== value.input_ref.project_id || frozen.data.revision !== value.input_ref.project_revision) {
      throw new Error('Host未返回此实际样例的冻结工程，不能按当前实例猜测作业归属。');
    }
    const matches = (reference: ModuleRef) => reference.id === draft.definition.id && reference.version === draft.definition.version;
    const jobs = value.jobs.filter(item => item.execution_ref.kind === 'definition_cycle' ? matches(item.execution_ref.data.module_ref)
      : frozen.data.devices.some(device => item.execution_ref.kind === 'project_device' && device.id === item.execution_ref.data.device_id && matches(device.module_ref)));
    if (!jobs.length) throw new Error(`实际样例 ${value.id} 中匹配此精确草稿的作业有0项；不能按其他版本或当前实例猜测。`);
    function openJob(job: typeof jobs[number]) {
      assertSampleOrigin();
      setDeviceProcess({ ...current!, scope: { kind: 'definition_draft', data: draft!.summary.draft_id }, selection: { kind: 'actual_run', run_id: value.id, job_id: job.id, at_ms: value.computed_until_ms! } });
      setProcessSampleChoice(null);
    }
    if (jobs.length > 1) {
      setProcessSampleChoice({ run_id: value.id, definition_ref: { id: draft.definition.id, version: draft.definition.version }, draft_revision: draft.summary.revision, jobs,
        async select(jobId) {
          const job = jobs.find(job => job.id === jobId);
          if (!job) throw new Error('所选作业不属于此实际样例的精确草稿来源。');
          // Choosing is a later user action; recheck the real Host Draft revision
          // and original process entry before opening its immutable Run.
          await checkSampleDraft();
          openJob(job);
        } });
      return;
    }
    openJob(jobs[0]);
  }
  async function runProcessCycle() {
      const captured = deviceProcess?.id, draft = deviceProcess?.draft, scenario = snapshot?.scenarios.find(value => value.id === scenarioId);
      if (!draft || !scenario || processSamples.status !== 'available' || !authoringIO || !project) throw new Error(processSamples.status === 'unavailable' ? processSamples.reason : '请先保存真实草稿并选择适用方案。');
      const state = await processSamples.run(draft, scenario);
      if (state.result?.kind !== 'run') throw new Error('样例未返回实际 Run，不能以观看动画代替。');
      const value = await authoringIO.dispatch<RunView>({ api_version: '0.1', request_id: `process-sample-${crypto.randomUUID()}`, project_id: project.id, scope: { kind: 'definition_draft', data: draft.summary.draft_id }, operation: 'get_run', params: { run_id: state.result.data.run_id, query: { kind: 'full_result' } } });
      if (value.kind !== 'full') throw new Error('未读到此实际样例的完整结果。');
      if (deviceProcessRef.current?.id === captured) await receiveProcessSample(value.data);
      await refreshLatest.current();
      return { message: '实际隔离周期已完成，读取原 Task 和冻结 Run。' };
  }
  async function inspectSavedProcessDraft(request: { project_id: string; draft_id: string; draft_revision: number }) {
    const captured = deviceProcess?.id;
    if (!authoringIO || request.project_id !== project?.id) throw new Error('工程已切换，请重新选择草稿。');
    const actual = await readDraft(authoringIO, request.project_id, request.draft_id);
    if (actual.summary.revision !== request.draft_revision) throw new Error('草稿修订已变化；原输入保留，请重读并核对冲突。');
    if (deviceProcessRef.current?.id !== captured) throw new Error('过程入口已关闭，不打开旧响应。');
    returnToAuthoring(actual);
    return { message: '已打开同一实际草稿的参数与来源详情。' };
  }
  async function inspectProcessItem(request: { context: ProcessReadContext; item_id: string }) {
    const captured = deviceProcessRef.current?.id, context = request.context;
    if (!authoringIO || context.selection.kind !== 'actual_run') throw new Error('物品定位需要当前实际运行时点。');
    const value = await authoringIO.dispatch<DomainObject>({ api_version: '0.1', request_id: `process-item-${crypto.randomUUID()}`, project_id: context.project_id, scope: context.scope, operation: 'get_object', params: { object_id: { kind: 'item', id: request.item_id }, source: { kind: 'run_snapshot', data: { run_id: context.selection.run_id, at_ms: context.selection.at_ms } }, node_index: null } });
    const active = deviceProcessRef.current, current = workspaceRef.current;
    const revision = active?.scope.kind === 'definition_draft' ? active.draft?.summary.revision : current?.project?.revision;
    if (!active || active.id !== captured || current?.context.host.session_id !== context.session_id || current.project?.id !== context.project_id || revision !== context.scope_revision || JSON.stringify(active.scope) !== JSON.stringify(context.scope) || JSON.stringify(active.selection) !== JSON.stringify(context.selection)) return;
    if (value.kind !== 'item') throw new Error('未读到此实际时点的物品。');
    if (context.scope.kind === 'project') selectObject({ kind: 'item', id: value.data.id });
    setMessage(`实际物品 ${value.data.id} · holder ${value.data.holder.kind}/${value.data.holder.id} · ${context.selection.at_ms}ms`);
  }
  async function editProcessNodeSource(request: NodeSourceEditRequest) {
    const origin = deviceProcessRef.current, context = request.selection.context;
    const io = authoringIO, base = workspaceRef.current?.project;
    if (!origin || !io || !base) throw new Error('请在实际设备过程来源中重新选择节点。');
    function assertOrigin() {
      request.assertCurrent();
      const active = deviceProcessRef.current, current = workspaceRef.current;
      const revision = active?.scope.kind === 'definition_draft' ? active.draft?.summary.revision : current?.project?.revision;
      if (!active || active.id !== origin!.id || current?.context.host.session_id !== context.session_id
        || current.project?.id !== context.project_id || revision !== context.scope_revision
        || JSON.stringify(active.scope) !== JSON.stringify(context.scope)
        || JSON.stringify(active.selection) !== JSON.stringify(context.selection)) {
        throw new Error('节点所属工程、修订或观察来源已变化，请在当前来源重新选择；不打开旧响应。');
      }
    }
    async function checkHost() {
      const actual = await io!.dispatch<ProjectContext>(authoringRequest(context.project_id, context.scope, 'get_context', { selection: [] }));
      assertOrigin();
      const revision = context.scope.kind === 'definition_draft' ? actual.definition?.revision : actual.project?.revision;
      if (actual.host.session_id !== context.session_id || actual.project?.id !== context.project_id || revision !== context.scope_revision) {
        throw new Error('实际 Host 的节点来源修订已变化，请重读后再创作。');
      }
    }
    assertOrigin();
    actionCount.current++; setBusy(true); setFailure(null); setMessage('');
    let creationCommit: EditCommit | null = null;
    try {
      await checkHost();
      let draft: AuthoringDraft;
      if (request.target.kind === 'definition_draft' && !request.target.frozen) {
        draft = await readDraft(io, context.project_id, request.target.draft_id);
        assertOrigin();
        if (context.scope.kind !== 'definition_draft' || context.scope.data !== draft.summary.draft_id || draft.summary.revision !== context.scope_revision) {
          throw new Error('节点不属于当前定义草稿修订，不能跳到其他草稿编辑。');
        }
      } else {
        const owner = request.selection.owner;
        const object = await io.dispatch<DomainObject>(authoringRequest(context.project_id, context.scope, 'get_object', {
          object_id: owner.object_id, source: owner.source, node_index: null,
        }));
        assertOrigin();
        let definition: ModuleDefinition;
        if (request.target.kind === 'module_version' && object.kind === 'module') {
          if (object.data.definition.id !== request.target.module_ref.id || object.data.definition.version !== request.target.module_ref.version) {
            throw new Error('Host 未返回所选节点的精确模块版本。');
          }
          definition = object.data.definition;
        } else if (request.target.kind === 'definition_draft' && request.target.frozen && object.kind === 'definition'
          && owner.object_id.kind === 'definition_draft' && owner.object_id.id === request.target.draft_id
          && owner.source.kind === 'run_snapshot' && context.scope.kind === 'definition_draft' && context.scope.data === request.target.draft_id) {
          definition = object.data;
        } else if (request.target.kind === 'device_override' && request.target.frozen && object.kind === 'device'
          && object.data.id === request.target.device_id && owner.source.kind === 'run_snapshot' && object.data.presentation_override) {
          const module = await io.dispatch<DomainObject>(authoringRequest(context.project_id, context.scope, 'get_object', {
            object_id: { kind: 'module', id: object.data.module_ref.id },
            source: { kind: 'run_module_version', data: { ...owner.source.data, module_ref: object.data.module_ref } }, node_index: null,
          }));
          assertOrigin();
          if (module.kind !== 'module' || module.data.definition.payload.category !== 'devices'
            || module.data.definition.id !== object.data.module_ref.id || module.data.definition.version !== object.data.module_ref.version) {
            throw new Error('未读到同一冻结实例的精确设备定义；不借当前定义补齐。');
          }
          definition = { ...module.data.definition, payload: { category: 'devices', data: {
            ...module.data.definition.payload.data, presentation: object.data.presentation_override,
          } } };
        } else throw new Error('此节点来源没有可用的定义创作路径；当前实例自有节点请使用所选节点编辑。');
        await checkHost();
        const commit = await io.dispatch<EditCommit>(authoringRequest(context.project_id, { kind: 'project' }, 'apply_commands', {
          base_revision: base.revision, commands: [{ kind: 'create_definition_draft', data: definition }],
        }));
        creationCommit = commit;
        assertOrigin();
        if (commit.scope.kind !== 'definition_draft') throw new Error('Host 未返回新定义草稿 Scope。');
        draft = await readDraft(io, context.project_id, commit.scope.data.id, commit.history, commit.new_revision);
        assertOrigin();
        rememberCommit(commit);
      }
      await refreshLatest.current();
      assertOrigin();
      setDeviceProcess(null); setDeliveryTab(null); setLibraryOpen(false); setSource('draft');
      setAuthoringDraft(draft); setAuthoringPresentation(null);
      setAuthoring({ id: crypto.randomUUID(), draft });
      setMessage(request.target.frozen ? '已从所选冻结来源创建可编辑草稿；采用须明确操作，旧运行保持原输入。' : '已打开所选节点的实际定义草稿。');
    } catch (error) {
      if (creationCommit?.scope.kind === 'definition_draft') {
        const failure = new Error(`已在工程 ${creationCommit.project_id} 创建定义草稿 ${creationCommit.scope.data.id} / r${creationCommit.new_revision}；后续读取或导航未完成。请在原工程读取此草稿，不要重复创建。原因为：${error instanceof Error ? error.message : String(error)}`, { cause: error });
        setFailure(failure);
        throw failure;
      }
      throw error;
    } finally { actionCount.current--; setBusy(actionCount.current > 0); }
  }
  const processActions: ProcessActions = {
    configure: async request => {
      if (request.device_id) {
        const current = await refreshLatest.current();
        const device = current.project?.devices.find(value => value.id === request.device_id && value.module_ref.id === request.module_ref.id && value.module_ref.version === request.module_ref.version);
        if (!device) throw new Error('当前工程已不是该精确实例版本；历史结果保留，请明确选择当前实例后编辑。');
        setDeviceProcess(null); closeAuthoring(); setSource('draft'); setEditor({ base: current, target: { kind: 'device', value: device } });
      } else if (deviceProcess?.draft) returnToAuthoring(deviceProcess.draft);
      else throw new Error('没有实际实例或定义草稿可配置。');
      return { message: '已打开实际配置入口。' };
    },
    inspect: async request => {
      if (request.device_id) selectObject({ kind: 'device', id: request.device_id });
      else if (deviceProcess?.draft) returnToAuthoring(deviceProcess.draft);
      else throw new Error('当前没有实际实例或草稿可定位。');
      return { message: '已定位实际对象或原定义草稿。' };
    },
    preview_cycle: runProcessCycle,
  };

  function closeAuthoring() {
    setAuthoring(null); setAuthoringDraft(null); setAuthoringPresentation(null);
  }
  function navigateFromAuthoring(title: string, proceed: () => void) {
    if (!authoring) { proceed(); return; }
    const guard = authoringNavigation.current;
    if (!guard) { setFailure(new Error('定义工作区尚未准备好离开核对，请保留当前输入。')); return; }
    guard(title, proceed);
  }
  async function openAuthoring(module?: ModuleRef, draftId?: string) {
    const next = await refreshLatest.current();
    if (!next.project) throw new Error('请先创建或打开工程，再创作定义。');
    const draft = draftId ? await readDraft(createModuleAuthoringIO(() => workspaceRef.current), next.project.id, draftId) : undefined;
    setDeliveryTab(null); setLibraryOpen(false); setLibraryModule(undefined);
    setSource('draft');
    setAuthoringDraft(draft ?? null); setAuthoringPresentation(null);
    setAuthoring({ id: crypto.randomUUID(), module, draft });
  }
  async function openAuthoringConsumer(category: ApiCategory, reference: ModuleRef) {
    const next = await refreshLatest.current();
    if (!next.project || next.project.id !== project?.id) throw new Error('工程已切换，请在当前工程重新选择精确版本。');
    if (category === 'templates') {
      closeAuthoring(); setLibraryModule(reference); setLibraryOpen(true); return;
    }
    if (category === 'devices') {
      if (!next.modules.has(host.moduleKey(reference))) throw new Error('此精确设备版本尚未采用到工程，请先在创作面板选择实例并核对采用。');
      if (!next.project.halls.length) throw new Error('请先创建实际厂房，再明确创建设备实例。');
      closeAuthoring(); setEditor({ base: next, target: { kind: 'device', moduleRef: reference } }); return;
    }
    const businessSections: Partial<Record<ApiCategory, BusinessEditIntent['section']>> = {
      items: 'items', resources: 'resources', transport: 'supplies', tasks: 'routes', operations: 'routes',
    };
    const section = businessSections[category];
    if (section) {
      if (!next.modules.has(host.moduleKey(reference))) throw new Error('此精确版本尚未采用到工程，请先在创作面板明确核对采用。');
      closeAuthoring(); setBusinessEditIntent({ id: crypto.randomUUID(), section }); setBusinessOpen(true);
      setMessage(`已进入业务分区；来源定义 ${reference.id} @ ${reference.version}，实例与方案仍须明确选择。`); return;
    }
    if (category === 'media') {
      const object = await authoringIO!.dispatch<DomainObject>({ api_version: '0.1', request_id: `authoring-consumer-${crypto.randomUUID()}`, project_id: next.project.id, scope: { kind: 'project' }, operation: 'get_object', params: { object_id: { kind: 'module', id: reference.id }, source: { kind: 'module_version', data: { module_ref: reference } }, node_index: null } });
      if (object.kind !== 'module' || object.data.definition.payload.category !== 'media') throw new Error('Host 未返回此精确表现版本。');
      setAuthoringPresentation({ presentation: object.data.definition.payload.data, source: authoringPresentationSource(next, null, object.data, object.data.rights) }); return;
    }
    if (category === 'professional') {
      if (!next.modules.has(host.moduleKey(reference))) throw new Error('此精确专业版本尚未采用到工程，请先在创作面板明确核对采用。');
      const object = await authoringIO!.dispatch<DomainObject>({ api_version: '0.1', request_id: `authoring-consumer-${crypto.randomUUID()}`, project_id: next.project.id, scope: { kind: 'project' }, operation: 'get_object', params: { object_id: { kind: 'module', id: reference.id }, source: { kind: 'module_version', data: { module_ref: reference } }, node_index: null } });
      if (object.kind !== 'module') throw new Error('Host 未返回此精确专业定义。');
      assertPackagingDefinition(object.data.definition);
      await openPackaging(); return;
    }
    throw new Error('本类定义可在当前面板明确核对采用；独立业务消费者入口尚未接入，当前输入仍保留。');
  }

  async function action(work: () => Promise<unknown>) {
    actionCount.current++;
    setBusy(true); setFailure(null); setMessage('');
    try { await work(); }
    catch (error) { setFailure(error instanceof Error ? error : new Error(String(error))); }
    finally { actionCount.current--; setBusy(actionCount.current > 0); }
  }
  useEffect(() => { if (connected) void action(refresh); }, []);

  function rememberCommit(commit: EditCommit) {
    setFailure(null);
    if (commit.scope.kind === 'project') setCommittedHistory({ projectId: commit.project_id, revision: commit.new_revision, value: commit.history });
  }
  const history = committedHistory && project && committedHistory.projectId === project.id && committedHistory.revision === project.revision ? committedHistory.value : null;
  async function edit(commandsToApply: EditCommand[], base = project) {
    if (!base) return;
    rememberCommit(await host.applyCommands(base, commandsToApply));
    await refresh(); setMessage('已应用到工程草稿。');
  }
  async function saveCurrentProject() {
    if (!project) return;
    const result = await host.saveProject(project, await host.selectDirectory('save_project'), setTaskState);
    if (result.kind !== 'save') throw new Error('Host 未返回保存结果。');
    await refresh(); setMessage(`已保存 r${result.data.saved_revision}${result.data.still_dirty ? '；当前仍有新修订未保存' : ''}。`);
  }
  function beginProject(intent: 'empty' | 'template' | 'demonstration', reuseEmpty: boolean) {
    if (intent === 'demonstration') { void action(() => openPackage('directory')); return; }
    if (reuseEmpty && project && !project.halls.length && !project.devices.length) {
      if (intent === 'empty') openEditor({ kind: 'hall' });
      else { setLibraryModule(undefined); setLibraryCategory('templates'); setLibraryOpen(true); }
      return;
    }
    setStartIntent(intent); setNewTitle('');
    setDeparture(project ? 'stay' : 'already_saved'); setNewDialog(true);
  }
  async function commitItemLayout(commandsToApply: EditCommand[], base: ProjectState): Promise<ProjectState> {
    actionCount.current++; setBusy(true); setFailure(null); setMessage('');
    try {
      rememberCommit(await host.applyCommands(base, commandsToApply));
      const next = await refresh();
      if (!next.project || next.project.id !== base.id) throw new Error('展示槽已提交，但活动工程已切换，请在原工程重读实际修订。');
      setMessage('展示槽已应用到工程草稿；新运行读取新布局，旧运行保留冻结来源。');
      return next.project;
    } finally { actionCount.current--; setBusy(actionCount.current > 0); }
  }
  async function commitLayout(commandsToApply: EditCommand[], base: ProjectState) {
    actionCount.current++; setBusy(true); setFailure(null); setMessage('');
    try {
      if (workspaceRef.current?.project?.id !== base.id) throw new Error('布局输入所属工程已切换，请返回原工程核对。');
      rememberCommit(await host.applyCommands(base, commandsToApply));
      const next = await refreshLatest.current();
      if (next.project?.id !== base.id) throw new Error('布局已提交，活动工程已切换，请在原工程重读实际修订。');
      setMessage('布局已应用到工程草稿，可在本会话撤销。');
    } finally { actionCount.current--; setBusy(actionCount.current > 0); }
  }
  async function applyTransform(object: ObjectRef, pose: Pose, base: ProjectState) {
    const actual = workspaceRef.current?.project;
    const original = object.kind === 'device' ? base.devices.find(value => value.id === object.id) : object.kind === 'line' ? base.lines.find(value => value.id === object.id) : base.halls.find(value => value.id === object.id);
    const current = object.kind === 'device' ? actual?.devices.find(value => value.id === object.id) : object.kind === 'line' ? actual?.lines.find(value => value.id === object.id) : actual?.halls.find(value => value.id === object.id);
    if (!original || !current) throw new Error('变换对象已不在当前工程中，输入保留。');
    if ('line_id' in original && 'line_id' in current && (original.line_id !== current.line_id || original.hall_id !== current.hall_id) || 'transform' in original && 'transform' in current && original.hall_id !== current.hall_id) throw new Error('对象父坐标已变化，请在当前画布重新定位，原变换保留供核对。');
    if (object.kind === 'device' && 'pose' in original) await commitLayout([{ kind: 'put_object', data: { kind: 'device', data: { ...original, pose } } }], base);
    else if (object.kind === 'line' && 'transform' in original) await commitLayout([{ kind: 'put_object', data: { kind: 'line', data: { ...original, transform: pose } } }], base);
    else if (object.kind === 'hall' && 'origin' in original) await commitLayout([{ kind: 'put_object', data: { kind: 'hall', data: { ...original, origin: pose } } }], base);
    else throw new Error('此对象没有可编辑的布局姿态。');
  }
  async function transformLayout(object: ObjectRef, pose: Pose, base: ProjectState) {
    try { await applyTransform(object, pose, base); }
    catch (error) { setTransformProposal({ object, pose, base, error: error instanceof Error ? error.message : String(error) }); }
  }
  async function reloadLayout() {
    const next = await refreshLatest.current();
    if (!next.project) throw new Error('当前未打开工程，本地布局输入保留。');
    return next.project;
  }
  async function commitProcessNode(commandsToApply: EditCommand[], base: ProjectState): Promise<ProjectState> {
    actionCount.current++; setBusy(true); setFailure(null); setMessage('');
    try {
      rememberCommit(await host.applyCommands(base, commandsToApply));
      const next = await refresh();
      if (!next.project || next.project.id !== base.id) throw new Error('节点已提交，但活动工程已切换，请在原工程重读实际修订。');
      setMessage('所选节点已应用到工程草稿；旧运行保持原冻结表现。');
      return next.project;
    } finally { actionCount.current--; setBusy(actionCount.current > 0); }
  }
  function openEditor(target: EditorTarget) {
    if (!snapshot || !project || source !== 'draft') return;
    setEditor({ target, base: snapshot });
  }
  async function reloadObjectEditor() {
    if (!editor?.base.project) return;
    const previous = editor;
    setBusy(true);
    try {
      const next = await refreshLatest.current();
      if (!next.project || next.project.id !== previous.base.project!.id || next.context.host.session_id !== previous.base.context.host.session_id) {
        throw new Error('工程或 Host 已切换，原表单输入保留；请返回原工程重新选择对象。');
      }
      let target = previous.target;
      if (target.value) {
        const id = target.value.id;
        if (target.kind === 'hall') {
          const value = next.project.halls.find(object => object.id === id);
          if (!value) throw new Error('当前厂房已不存在，原表单输入保留。');
          target = { kind: 'hall', value };
        } else if (target.kind === 'line') {
          const value = next.project.lines.find(object => object.id === id);
          if (!value) throw new Error('当前线组已不存在，原表单输入保留。');
          target = { kind: 'line', value };
        } else {
          const value = next.project.devices.find(object => object.id === id);
          if (!value) throw new Error('当前设备已不存在，原表单输入保留。');
          target = { kind: 'device', value };
        }
      }
      setEditor({ target, base: next });
    } finally { setBusy(false); }
  }
  const currentHall = displayedProject?.halls.find(hall => hall.id === hallId);
  const selectedDevice = selection?.kind === 'device' ? displayedProject?.devices.find(device => device.id === selection.id) : undefined;
  const selectedLine = selection?.kind === 'line' ? displayedProject?.lines.find(line => line.id === selection.id) : undefined;
  const selectedHall = selection?.kind === 'hall' ? displayedProject?.halls.find(hall => hall.id === selection.id) : undefined;
  const selectedLocation = selection?.kind === 'location' ? displayedProject?.locations.find(location => location.id === selection.id) : undefined;
  const selectedResource = selection?.kind === 'resource' ? displayedProject?.resources.find(value => value.id === selection.id) : undefined;
  const selectedConnection = selection?.kind === 'connection' ? displayedProject?.connections.find(connection => connection.data.id === selection.id) : undefined;
  const selectedItem = source === 'run' && selection?.kind === 'item' ? items.find(item => item.id === selection.id) : undefined;
  const selectedTransport = source === 'run' && (selection?.kind === 'transport' || selection?.kind === 'job') ? run?.transport.find(trip => trip.id === selection.id) : undefined;
  const selected = selectedDevice ?? selectedLine ?? selectedHall ?? selectedLocation ?? selectedResource ?? (selectedConnection ? { id: selectedConnection.data.id, title: ({ material: '物料连接', signal: '信号连接', artifact: '专业成果连接' })[selectedConnection.kind] } : selectedItem ? { id: selectedItem.id, title: selectedItem.item_type_ref.id } : selectedTransport ? { id: selectedTransport.id, title: '运输批次' } : undefined);
  const exactModule = selectedDevice ? displayedModules?.get(host.moduleKey(selectedDevice.module_ref)) : undefined;
  const position = selectedDevice && displayedProject ? deviceHallPosition(displayedProject, selectedDevice) : null;
  const activeTask = taskState && ['queued', 'running', 'paused', 'cancel_requested'].includes(taskState.task.state) ? taskState : null;
  const snapValid = Number.isFinite(Number(snapStep)) && Number(snapStep) > 0;
  const layoutControl: LayoutControlOptions | undefined = source === 'draft' && !busy && !editing && layoutMode !== 'select' && (!snapLayout || snapValid)
    ? { mode: layoutMode, snap: snapLayout, step_m: Number(snapStep) } : undefined;
  const selectedLayout = project ? layoutSelection(project, selections) : null;
  const keyboardAction = useRef<(event: KeyboardEvent) => void>(() => {});
  keyboardAction.current = event => {
    const target = event.target;
    if (event.isComposing || target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) || event.altKey || busy || editing || newDialog || opening) return;
    const modifier = event.ctrlKey || event.metaKey, key = event.key.toLowerCase();
    if (modifier && key === 's' && project) { event.preventDefault(); void action(saveCurrentProject); }
    else if (modifier && key === 'z' && project) { const redo = event.shiftKey; if ((redo ? history?.can_redo : history?.can_undo) === false || !commands.includes(redo ? 'redo' : 'undo')) return; event.preventDefault(); void action(() => edit([{ kind: redo ? 'redo' : 'undo' }])); }
    else if (modifier && key === 'y' && project && history?.can_redo !== false && commands.includes('redo')) { event.preventDefault(); void action(() => edit([{ kind: 'redo' }])); }
    else if (modifier && key === 'd' && source === 'draft' && project && selectedLayout?.devices.length) { event.preventDefault(); setLayoutEditor({ mode: 'copy', project, selection: selections }); }
    else if (!modifier && key === 'home') { event.preventDefault(); setResetKey(value => value + 1); }
    else if (!modifier && key === 'v') { event.preventDefault(); setLayoutMode('select'); }
    else if (!modifier && (key === 'm' || key === 'r') && source === 'draft' && (selectedDevice || selectedLine || selectedHall && canvas === 'multi_hall_canvas')) { event.preventDefault(); setLayoutMode(key === 'm' ? 'translate' : 'rotate'); }
    else if (!modifier && key === 'g' && source === 'draft' && project && selectedLayout?.devices.length) { event.preventDefault(); setLayoutEditor({ mode: 'group', project, selection: selections }); }
    else if (!modifier && key === 'enter' && source === 'draft' && target === document.body) { if (selectedDevice) openEditor({ kind: 'device', value: selectedDevice }); else if (selectedLine) openEditor({ kind: 'line', value: selectedLine }); else if (selectedHall) openEditor({ kind: 'hall', value: selectedHall }); }
  };
  useEffect(() => { const handle = (event: KeyboardEvent) => keyboardAction.current(event); window.addEventListener('keydown', handle); return () => window.removeEventListener('keydown', handle); }, []);
  const hasUnappliedInput = !!(agentOpen && agentEditorState?.identity === agentIdentity && agentEditorState.dirty || libraryOpen && libraryDirty || checkOpen && checkDirty || (layoutEditor || transformProposal) && layoutDirty || editor && objectDirty || connectionEditor && connectionDirty
      || authoring && definitionDirty || itemLayout && itemLayoutDirty
      || businessOpen && businessDirty?.project_id === project?.id && businessDirty?.dirty
      || deviceProcess && (nodeDirty || processEditorState?.identity === processEditorIdentity && processEditorState.dirty)
      || packagingScope && packagingEditorState?.identity === packagingEditorIdentity && packagingEditorState.dirty
      || newDialog && newTitle.trim());
  useWindowClose({
    hasUnappliedInput,
    workInProgress: busy || activeTask !== null || !!(deviceProcess && processEditorState?.identity === processEditorIdentity && processEditorState.busy)
      || !!(packagingScope && packagingEditorState?.identity === packagingEditorIdentity && packagingEditorState.busy)
      || !!(agentOpen && agentEditorState?.identity === agentIdentity && agentEditorState.busy),
    contextKey: JSON.stringify([project?.id, project?.revision, authoringDraft?.summary.draft_id, authoringDraft?.summary.revision, deviceProcess?.id, processEditorIdentity, packagingEditorIdentity, agentIdentity]),
    confirmClose: () => new Promise(resolve => setWindowClosePrompt({ resolve })),
    save: async () => {
      actionCount.current++; setBusy(true); setFailure(null);
      try { await saveCurrentProject(); }
      finally { actionCount.current--; setBusy(actionCount.current > 0); }
    },
    onError: setFailure,
  });
  function decideWindowClose(decision: WindowCloseDecision) {
    windowClosePrompt?.resolve(decision);
    setWindowClosePrompt(null);
  }

  async function chooseRun(id: string, requestedTime?: number) {
    if (!project || !id) return;
    const projectId = project.id, epoch = ++observation.current;
    comparisonEpoch.current++;
    pendingRun.current = id;
    const view = await host.getRun(projectId, id, { kind: 'full_result' });
    if (!ownsObservation(epoch, projectId, id)) return;
    if (view.kind !== 'full') throw new Error('Host 未返回完整 Run。');
    const frontier = view.data.computed_until_ms;
    if (frontier == null) throw new Error('该 Run 尚无已计算边界，不能显示实际状态。');
    const time = requestedTime ?? frontier;
    const frozen = await host.readRunInputs(projectId, view.data, frontier);
    if (!ownsObservation(epoch, projectId, id)) return;
    const presentation = await readFactoryObservation(view.data, frozen, time, projectId, epoch);
    if (presentation === null) return;
    if (comparison?.run.id === id && run && runProject) setComparison({ project: runProject, run, scenario: runInputs?.scenario });
    primaryRun.current = id;
    setFactorySource(presentation);
    setRun(view.data); setRunProject(frozen.project); setRunInputs(frozen); setAtMs(time); setObservationPage(presentation.observation); setTimelineSelection(current => current?.run_id === id ? current : null); setSource('run');
    setHallId(current => frozen.project.halls.some(hall => hall.id === current) ? current : frozen.project.halls[0]?.id ?? null);
  }
  async function chooseComparison(id: string) {
    const epoch = ++comparisonEpoch.current;
    if (!id) { setComparison(null); return; }
    if (!project || !run || id === run.id) return;
    const projectId = project.id, primary = run.id;
    const owns = () => epoch === comparisonEpoch.current && projectIdentity.current === projectId && primaryRun.current === primary;
    const view = await host.getRun(projectId, id, { kind: 'full_result' });
    if (!owns()) return;
    if (view.kind !== 'full') throw new Error('Host 未返回比较 Run 的完整结果。');
    if (view.data.computed_until_ms == null) throw new Error('比较 Run 尚无已计算边界，不能解释实际轨道。');
    const frozen = await host.readRunInputs(projectId, view.data, view.data.computed_until_ms);
    if (owns()) setComparison({ project: frozen.project, run: view.data, scenario: frozen.scenario });
  }
  function ownsObservation(epoch: number, projectId: string, runId: string) {
    return epoch === observation.current && projectIdentity.current === projectId && pendingRun.current === runId;
  }
  async function readFactoryObservation(value: RunResult, frozen: NonNullable<typeof runInputs>, time: number,
    projectId: string, epoch: number): Promise<FactoryPresentationSource | null> {
    const active = factoryReadCurrent.current;
    if (!active || !ownsObservation(epoch, projectId, value.id)) return null;
    const captured: FactoryPresentationContext = { ...active, project: frozen.project,
      view: { run_id: value.id, at_ms: time }, preparedRun: { run: value, modules: frozen.modules } };
    const key = factoryContextKey(captured);
    const current = () => {
      const next = factoryReadCurrent.current;
      if (!next || next.session_id !== captured.session_id || next.project_id !== captured.project_id
        || next.scope_revision !== captured.scope_revision) return null;
      // Keep the displayed frame's pending media valid until its replacement
      // commits, but never publish a superseded seek or a different workspace.
      return ownsObservation(epoch, projectId, value.id) || factoryContextKey(next) === key ? captured : null;
    };
    try {
      const presentation = await createFactoryPresentation(current).read(captured);
      return ownsObservation(epoch, projectId, value.id) && current() ? presentation : null;
    } catch (error) {
      if (!ownsObservation(epoch, projectId, value.id) || !current()) return null;
      throw error;
    }
  }
  async function seekRun(runId: string, time: number) {
    if (!project || !run || !runInputs) return;
    if (runId !== run.id) {
      if (comparison?.run.id === runId) await chooseRun(runId, time);
      return;
    }
    const epoch = ++observation.current;
    pendingRun.current = runId;
    const presentation = await readFactoryObservation(run, runInputs, time, project.id, epoch);
    if (presentation !== null) { setFactorySource(presentation); setAtMs(time); setObservationPage(presentation.observation); setSource('run'); }
  }
  async function commitTimeline(proposal: TimelineInputCommit): Promise<TimelineCommitResult> {
    if (!project || projectIdentity.current !== proposal.project_id || project.id !== proposal.project_id) throw new Error('时间轴草稿所属工程已切换，请重新读取该工程。');
    try {
      // Both captured revisions reach the single Host transaction: Project in
      // base_revision and Scenario in the unchanged PutObject metadata.
      rememberCommit(await host.applyCommands({ ...project, revision: proposal.base_project_revision }, [proposal.command]));
      const next = await refresh();
      const committed = next.scenarios.find(value => value.id === proposal.scenario_id);
      if (!committed) throw new Error('提交后未读到该方案，不能将预览标为已提交。');
      setTimelineSelection(null);
      setMessage('时间轴输入已提交；原 Run 继续保留原冻结输入。');
      return { ok: true, scenario: committed };
    } catch (error) {
      if (error instanceof host.HostFailure) return { ok: false, error: { ...error.detail, message: error.message } };
      throw error;
    }
  }
  async function commitBusiness(proposal: BusinessCommitProposal): Promise<BusinessCommitResult> {
    if (!project || projectIdentity.current !== proposal.project_id || project.id !== proposal.project_id) throw new Error('业务表单所属工程已切换，请重新读取该工程。');
    setBusy(true); setMessage(''); setFailure(null);
    try {
      const commit = await host.applyCommands({ ...project, revision: proposal.base_project_revision }, proposal.commands);
      rememberCommit(commit);
      const next = await refresh();
      if (!next.project || next.project.id !== proposal.project_id) throw new Error('提交后活动工程已切换，请读取原工程核对结果。');
      return { ok: true, commit, snapshot: { project: next.project, scenarios: next.scenarios } };
    } catch (error) {
      if (error instanceof host.HostFailure) return { ok: false, error: { ...error.detail, message: error.message } };
      throw error;
    } finally { setBusy(false); }
  }
  async function runBusiness(request: BusinessRunRequest) {
    if (!project || projectIdentity.current !== request.project_id || project.id !== request.project_id || project.revision !== request.base_project_revision) throw new Error('工程修订已变化，请刷新业务输入后再运行。');
    const scenario = snapshot?.scenarios.find(value => value.id === request.scenario_id);
    if (!scenario || scenario.revision !== request.base_scenario_revision) throw new Error('方案修订已变化，请刷新并核对后再运行。');
    setBusy(true); setMessage(''); setFailure(null);
    try {
      const result = await host.runScenario(project, scenario, setTaskState);
      if (result.kind !== 'run') throw new Error('Host 未返回实际 Run。');
      await refresh(); await chooseRun(result.data.run_id);
    } finally { setBusy(false); }
  }
  function locateBusiness(request: BusinessLocateRequest) {
    setSelection(request.object); setTimelineSelection(null);
    if (request.source === 'draft') switchSource('draft');
    else if (request.run_id) {
      const runId = request.run_id;
      void action(async () => {
        if (runId === run?.id && request.at_ms === undefined) switchSource('run');
        else if (runId === run?.id) await seekRun(runId, request.at_ms!);
        else await chooseRun(runId, request.at_ms);
      });
    }
  }
  async function pickExport(purpose: 'export_evaluation' | 'export_package' | 'save_project', kind: 'directory' | 'file' = 'directory') {
    try { return await (purpose === 'export_package' && kind === 'file' ? host.selectFile('export_package') : host.selectDirectory(purpose)); }
    catch (error) {
      if (error instanceof host.HostFailure && error.detail.code === 'Cancelled') return null;
      throw error;
    }
  }
  function deliveryProjectId() {
    if (!project || project.id !== projectIdentity.current) throw new Error('交付面板所属工程已切换，请重新读取该工程。');
    return project.id;
  }
  async function readDeliveryEvidence(runId: string) {
    const projectId = deliveryProjectId();
    const value = await host.getRun(projectId, runId, { kind: 'full_result' });
    if (value.kind !== 'full') throw new Error('Host 未返回完整 Run。');
    if (value.data.computed_until_ms == null) throw new Error('该 Run 缺少已计算边界，冻结输入尚不能由此读取入口解释。');
    const frozen = await host.readRunInputs(projectId, value.data, value.data.computed_until_ms);
    if (projectIdentity.current !== projectId) throw new Error('读取期间工程已切换，请重新打开交付面板。');
    return { project: frozen.project, scenario: frozen.scenario, run: value.data };
  }
  async function openPackage(kind: 'directory' | 'file') {
    let selected;
    try { selected = kind === 'file' ? await host.selectFile('import_package') : await host.selectDirectory('import_package'); }
    catch (error) {
      if (error instanceof host.HostFailure && error.detail.code === 'Cancelled') return;
      throw error;
    }
    setOpening(await host.stageOpen(selected, setTaskState));
    setDeparture(project ? 'stay' : 'already_saved'); setAllowMissing(false);
  }
  const deliveryServices = useMemo<DeliveryServices>(() => ({
    readEvidence: readDeliveryEvidence,
    compare: params => host.compareActualRuns(deliveryProjectId(), params.run_ids, params.metric_scope),
    startEvaluation: async params => {
      const projectId = deliveryProjectId();
      const destination = await pickExport('export_evaluation');
      return destination ? host.startEvaluationExport(projectId, { ...params, selected_destination_ref: destination }) : null;
    },
    startPackage: async params => {
      const projectId = deliveryProjectId();
      const destination = await pickExport('export_package', 'file');
      return destination ? host.startPackageExport(projectId, { ...params, selected_destination_ref: destination }) : null;
    },
    readTask: host.getTaskState,
    cancelTask: host.cancelTask,
    startSave: async captured => {
      if (captured.id !== deliveryProjectId()) throw new Error('保存输入所属工程已切换。');
      const destination = await pickExport('save_project');
      return destination ? host.startSaveProject(captured, destination) : null;
    },
    cloneScenario: async request => {
      const projectId = deliveryProjectId();
      const current = await host.readWorkspace();
      if (!current.project || current.project.id !== projectId) throw new Error('创建方案时工程已切换，请重新读取。');
      const evidence = await readDeliveryEvidence(request.source_run_id);
      const proposal = prepareScenarioFork(current.project, evidence, request);
      rememberCommit(await host.applyCommands(current.project, proposal.commands));
      const next = await refreshLatest.current();
      const created = next.scenarios.find(value => value.id === request.new_scenario_id.trim());
      if (next.project?.id !== projectId || !created) throw new Error('提交后未读取到实际新方案，请核对工程。');
      return { scenario_id: created.id, revision: created.revision };
    },
  }), [project?.id]);
  const timelineSelection = resolveFollowedItemSelection(storedTimelineSelection, observationPage, run, atMs);
  const selectedTimelineJob = timelineProcessJob(timelineSelection, run);
  const timelineProps: TimelineProps = {
    project: run ? runProject : project, draftProject: project,
    run, runScenario: runInputs?.scenario ?? null,
    scenario: snapshot?.scenarios.find(value => value.id === scenarioId) ?? null,
    atMs, observation: observationPage, selected: timelineSelection, comparison,
    onSelect: value => { setSelection(value); revealObjectHall(value); }, onInspect: value => { setTimelineSelection(value); if (value) setSelection(value.object); },
    onTime: time => { if (run) void action(() => seekRun(run.id, time)); },
    onSeek: request => { void action(() => seekRun(request.run_id, request.at_ms)); },
    onCommitInput: commitTimeline,
    onRefreshInput: async () => { await refresh(); },
    onEditBusiness: project ? request => {
      if (busy || editor || connectionEditor || libraryOpen || authoring || deliveryTab !== null || itemLayout !== null || !project) return;
      void action(async () => {
        const next = await refresh();
        if (next.project?.id !== project.id) throw new Error('业务输入所属工程已切换，请重新选择。');
        setBusinessEditIntent({ ...request, id: crypto.randomUUID() });
        setBusinessOpen(true);
      });
    } : undefined,
  };
  function switchSource(next: 'draft' | 'run') {
    const graph = next === 'run' ? runProject : project;
    observation.current++; pendingRun.current = null;
    setSource(next); setTimelineSelection(null); setSelections([]); setSelection(null); setLayoutMode('select');
    setHallId(current => graph?.halls.some(hall => hall.id === current) ? current : graph?.halls[0]?.id ?? null);
  }
  function revealObjectHall(value: ObjectRef | null) {
    if (!value || !displayedProject) return;
    const hall = objectHall(displayedProject, value, source === 'run' ? observedItems(observationPage, run, atMs) ?? [] : [], source === 'run' ? run : null);
    if (hall) setHallId(hall);
  }
  function selectObject(value: ObjectRef | null, additive = false) {
    const next = value ? additive && source === 'draft' ? selections.some(ref => same(ref, value)) ? selections.filter(ref => !same(ref, value)) : [...selections, value] : [value] : [];
    setSelections(next); setSelection(next.at(-1) ?? null); setTimelineSelection(null);
    revealObjectHall(next.at(-1) ?? null);
  }
  function resize(event: React.PointerEvent, axis: 'left' | 'right' | 'bottom') {
    event.preventDefault();
    const initialX = event.clientX, initialY = event.clientY;
    const move = (value: PointerEvent) => {
      if (axis === 'left') setLeftWidth(Math.max(200, Math.min(420, leftWidth + value.clientX - initialX)));
      else if (axis === 'right') setRightWidth(Math.max(280, Math.min(480, rightWidth - value.clientX + initialX)));
      else setDockHeight(Math.max(120, Math.min(320, dockHeight - value.clientY + initialY)));
    };
    const stop = () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', stop); };
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', stop, { once: true });
  }

  return <main className="workbench" aria-busy={busy} onClickCapture={event => {
    if (editing || newDialog || opening) return;
    const trigger = event.target instanceof Element ? event.target.closest('button') : null;
    if (trigger instanceof HTMLButtonElement) {
      returnFocus.current = trigger.closest('.menu-popup')?.parentElement?.querySelector('summary') ?? trigger;
    }
  }} style={{ '--left-width': `${leftVisible ? leftWidth : 32}px`, '--right-width': `${rightVisible ? rightWidth : 32}px`, '--dock-height': `${dockHeight}px` } as React.CSSProperties}>
    <div className="titlebar"><strong><img className="product-mark" src="/superfactory-icon.png" alt="" width="28" height="28" />超级工厂</strong><span>{project?.title ?? '未打开工程'}{snapshot?.context.project?.dirty || hasUnappliedInput ? ' *' : ''}</span><span className="title-status">{project ? `r${project.revision}` : '本地工程'}</span></div>
    <nav className="menubar" aria-label="主菜单" onClick={event => {
      const trigger = event.target instanceof HTMLElement ? event.target : null;
      const activeMenu = trigger?.closest('summary')?.parentElement;
      if (!activeMenu) trigger?.closest('details')?.querySelector('summary')?.focus();
      event.currentTarget.querySelectorAll('details[open]').forEach(menu => {
        if (menu !== activeMenu) menu.removeAttribute('open');
      });
    }}>
      <details><summary>文件</summary><div className="menu-popup">
        <button disabled={!connected || busy || editing} onClick={() => beginProject('empty', false)}>新建工程…</button>
        <button disabled={!connected || busy || editing} onClick={() => void action(() => openPackage('directory'))}>打开工程目录…</button><button disabled={!connected || busy || editing} onClick={() => void action(() => openPackage('file'))}>打开工程ZIP…</button>
        <button disabled={!project || busy} onClick={() => void action(saveCurrentProject)}>保存工程…</button>
        <button disabled={!connected || busy} onClick={() => void action(refresh)}>刷新工程</button>
      </div></details>
      <details><summary>编辑</summary><div className="menu-popup"><button disabled={!project || busy || editing || !commands.includes('undo') || history?.can_undo === false} onClick={() => void action(() => edit([{ kind: 'undo' }]))}>撤销</button><button disabled={!project || busy || editing || !commands.includes('redo') || history?.can_redo === false} onClick={() => void action(() => edit([{ kind: 'redo' }]))}>重做</button><button disabled={!project || busy || editing || source !== 'draft' || !commands.includes('put_object')} onClick={() => setItemLayout({ initialOwner: selection && (selection.kind === 'device' || selection.kind === 'location' || selection.kind === 'item_type') ? { kind: selection.kind, id: selection.id } : undefined })}>物品展示槽…</button></div></details>
      <details><summary>视图</summary><div className="menu-popup">{(['line_canvas', 'multi_hall_canvas'] as const).map(value => <button key={value} onClick={() => setCanvas(value)}>{canvas === value ? '✓ ' : ''}{value === 'line_canvas' ? '流水线画布' : '多厂房画布'}</button>)}{[1, 1.5, 2].map(scale => <button key={scale} onClick={() => setPropertyTextScale(scale)}>{propertyTextScale === scale ? '✓ ' : ''}属性文字 {scale * 100}%</button>)}<button onClick={() => setResetKey(value => value + 1)}>复位为斜视</button>{(['detailed', 'simplified', 'region'] as LodMode[]).map(value => <button key={value} onClick={() => { setAutoLod(false); setLod(value); }}>{lod === value && !autoLod ? '✓ ' : ''}{({ detailed: '详细', simplified: '简化', region: '区域' })[value]}</button>)}<button onClick={() => setAutoLod(value => !value)}>{autoLod ? '✓ ' : ''}自动呈现档位</button></div></details>
      <details><summary>Agent</summary><div className="menu-popup"><button disabled={!snapshot || busy || editing} onClick={() => { switchSource('draft'); setAgentEditorState(null); setAgentOpen(true); }}>连接、权限与任务…</button></div></details><details><summary>模块</summary><div className="menu-popup"><button disabled={!project || busy || editing} onClick={() => void action(() => openAuthoring())}>新建定义…</button>{definitionDraftIds.map(id => <button key={id} disabled={!project || busy || editing} onClick={() => void action(() => openAuthoring(undefined, id))}>继续定义草稿 · {id}</button>)}{!definitionDraftIds.length && <span className="form-hint">当前 Host 没有可读取的定义草稿</span>}</div></details><details><summary>设备过程</summary><div className="menu-popup"><button disabled={!authoringDraft || busy || deviceProcess !== null} onClick={() => navigateFromAuthoring("打开当前定义的图片过程", () => void action(openDraftProcess))}>当前定义的图片过程与标签…</button><button disabled={!selectedDevice || !exactModule || busy || editing} onClick={() => void action(async () => openDeviceProcess())}>所选设备的过程…</button></div></details><details><summary>专业工程</summary><div className="menu-popup"><button disabled={!project || busy || editing} onClick={() => void action(() => openPackaging())}>纸包装 · 结构与过程…</button>{authoringDraft?.definition.payload.category === 'professional' && <button disabled={busy} onClick={() => navigateFromAuthoring('打开当前专业定义的计算页', () => void action(() => openPackaging({ kind: 'definition_draft', data: authoringDraft.summary.draft_id })))}>在当前专业定义草稿中计算…</button>}</div></details><details><summary>评审与交付</summary><div className="menu-popup">{(['comparison', 'reporting', 'handoff'] as const).map(tab => <button key={tab} disabled={!project || busy || editing} onClick={() => void action(async () => { await refresh(); setDeliveryTab(tab); })}>{({ comparison: '方案比较', reporting: '评估报告', handoff: '工程交接' })[tab]}</button>)}</div></details><details><summary>窗口</summary><div className="menu-popup">{[['对象树', leftVisible, setLeftVisible], ['属性', rightVisible, setRightVisible], ['时间轴', timelineVisible, setTimelineVisible], ['事件描述', descriptionVisible, setDescriptionVisible]].map(([label, visible, update]) => <button key={String(label)} onClick={() => (update as (value: boolean) => void)(!visible)}>{visible ? '✓ ' : ''}{String(label)}</button>)}<button onClick={() => { setLeftVisible(true); setRightVisible(true); setTimelineVisible(true); setDescriptionVisible(true); setLeftWidth(256); setRightWidth(400); setDockHeight(160); }}>恢复默认布局</button></div></details>
      <span className="menu-divider" /><button className="menu-action" disabled={!connected || busy || editing || source !== 'draft'} onClick={() => void action(async () => { await refresh(); setLibraryModule(undefined); setLibraryCategory(undefined); setLibraryOpen(true); })}>模块与模板库</button><button className="menu-action" disabled={!project || busy || source !== 'draft' || editing} onClick={() => openEditor({ kind: 'hall' })}>新建厂房</button><button className="menu-action" disabled={!project?.halls.length || busy || source !== 'draft' || editing} onClick={() => openEditor({ kind: 'device' })}>添加设备</button><button className="menu-action" disabled={!project?.halls.length || busy || source !== 'draft' || editing} onClick={() => openEditor({ kind: 'line' })}>新建线组</button><button className="menu-action" disabled={!project || busy || source !== 'draft' || editing} onClick={() => { if (snapshot) setConnectionEditor({ base: snapshot }); }}>连接物料</button>
      <button className="menu-action" disabled={!project || busy || editing} onClick={() => void action(() => openAuthoring())}>定义创作</button><button className="menu-action" disabled={!project || busy || editing} onClick={() => void action(async () => { await refresh(); setBusinessEditIntent(undefined); setBusinessOpen(true); })}>业务输入</button><span className="menu-spacer" /><select aria-label="当前方案" value={scenarioId} disabled={busy || editing} onChange={event => setScenarioId(event.target.value)}><option value="">未选择方案</option>{snapshot?.scenarios.map(scenario => <option key={scenario.id} value={scenario.id}>{scenario.title} · {scenario.id} · r{scenario.revision}</option>)}</select><button className="primary" disabled={!project || !scenarioId || busy || editing} onClick={() => void action(async () => { const scenario = snapshot?.scenarios.find(value => value.id === scenarioId); if (!project || !scenario) return; const result = await host.runScenario(project, scenario, setTaskState); if (result.kind !== 'run') throw new Error('Host 未返回 Run。'); await refresh(); await chooseRun(result.data.run_id); })}>运行方案</button>
    </nav>
    {(!connected || failure || message || activeTask) && <div className={`notice ${failure ? 'notice-error' : ''}`} role={failure ? 'alert' : 'status'}>
      {!connected ? '浏览器预览 · 工程操作请使用桌面应用' : failure ? `${failure.name}: ${failure.message}` : activeTask ? `任务 ${activeTask.task.id} · ${activeTask.task.state}` : message}
      {failure instanceof host.HostFailure && failure.detail.actual_revision !== null && <span>当前 r{failure.detail.actual_revision}；表单仍保留原值，请返回核对。</span>}
      {failure instanceof host.HostFailure && <HostFailureDetails detail={failure.detail} onLocate={busy||editing?undefined:object=>{
        locateBusiness({object,source:'draft'});
        const section=({route:'routes',device:'stages',resource:'resources',item_type:'items',calendar:'calendars'} as Partial<Record<ObjectRef['kind'],BusinessEditIntent['section']>>)[object.kind];
        if(section){setBusinessEditIntent({id:crypto.randomUUID(),section,object_id:object.id});setBusinessOpen(true);}
      }}/>}
      {activeTask && <button onClick={() => void action(async () => { const result = await host.cancelTask(activeTask.task); setTaskState({ ...activeTask, task: result.task }); setMessage(result.action_accepted ? '已请求取消，等待任务实际终态。' : '任务已越过取消边界，请读取实际结果。'); })}>取消计算</button>}
    </div>}
    <div className="workspace-body">
      <aside className={`object-tree ${leftVisible ? '' : 'collapsed'}`} aria-label="工程对象树"><header><strong>{leftVisible ? '工程对象' : ''}</strong><button aria-label={leftVisible ? '收起对象树' : '展开对象树'} onClick={() => setLeftVisible(value => !value)}>{leftVisible ? '‹' : '›'}</button></header>{leftVisible && <>
        <div className="tree-caption">{displayedProject?.title ?? '未打开工程'}<small>{source === 'run' ? 'Run 冻结输入' : '工程草稿'}</small></div>
        <div className="tree-scroll">{displayedProject?.halls.map(hall => <ObjectTreeBranch key={hall.id} kind="hall" title={hall.title} selected={same(selection, { kind: 'hall', id: hall.id })} onSelect={() => { selectObject({ kind: 'hall', id: hall.id }); setHallId(hall.id); }}>{displayedProject.lines.filter(line => line.hall_id === hall.id).map(line => <ObjectTreeBranch key={line.id} kind="line" title={line.title} selected={same(selection, { kind: 'line', id: line.id })} onSelect={event => selectObject({ kind: 'line', id: line.id }, event.shiftKey || event.ctrlKey || event.metaKey)}>{displayedProject.devices.filter(device => device.line_id === line.id).map(device => treeDevice(device.id, device.title))}</ObjectTreeBranch>)}{displayedProject.devices.filter(device => device.hall_id === hall.id && !device.line_id).map(device => treeDevice(device.id, device.title))}{displayedProject.locations.filter(value => value.hall_id === hall.id).map(location => <button className="tree-leaf" key={location.id} onClick={() => selectObject({ kind: 'location', id: location.id })}>▧ {location.title}</button>)}</ObjectTreeBranch>)}{displayedProject?.connections.length ? <details open><summary>连接</summary>{displayedProject.connections.map(connection => <button key={connection.data.id} className={`tree-leaf ${same(selection, { kind: 'connection', id: connection.data.id }) ? 'selected' : ''}`} onClick={() => selectObject({ kind: 'connection', id: connection.data.id })}>{connection.kind === 'material' ? '物料' : connection.kind === 'signal' ? '信号' : '成果'} · {connection.data.id}</button>)}</details> : null}{displayedProject?.resources.length ? <details><summary>资源</summary>{displayedProject.resources.map(value => <button className="tree-leaf" key={value.id} onClick={() => selectObject({ kind: 'resource', id: value.id })}>{value.title}</button>)}</details> : null}{project && !project.halls.length && <p className="muted">工程为空，先创建厂房。</p>}</div>
        <footer>设备 {displayedProject?.devices.length ?? 0} · 线组 {displayedProject?.lines.length ?? 0}</footer>
      </>}</aside>
      <div className="resize-handle" role="separator" aria-label="对象树宽度" onPointerDown={event => resize(event, 'left')} />
      <section className="central-workspace"><div className="canvas-tabs"><button className={canvas === 'line_canvas' ? 'active' : ''} onClick={() => setCanvas('line_canvas')}>流水线画布</button><button className={canvas === 'multi_hall_canvas' ? 'active' : ''} onClick={() => setCanvas('multi_hall_canvas')}>多厂房画布</button><span /><select aria-label="画布厂房" value={hallId ?? ''} onChange={event => setHallId(event.target.value)}>{displayedProject?.halls.map(hall => <option key={hall.id} value={hall.id}>{hall.title} · {hall.id}</option>)}</select></div>
        <div className="sourcebar"><button className={source === 'draft' ? 'active' : ''} disabled={busy || editing} onClick={() => switchSource('draft')}>输入草稿</button><button disabled={!runProject || busy || editing} className={source === 'run' ? 'active' : ''} onClick={() => switchSource('run')}>实际回看</button><span aria-label={source === 'run' ? '实际结果冻结输入' : '当前工程草稿'}>{source === 'run' ? `冻结 r${runProject?.revision} · ${atMs == null ? '时点未知' : `${atMs / 1000}s`}` : `草稿 r${project?.revision ?? '—'}`}{source === 'run' && run && <> · 冻结方案 {run.input_ref.scenario_id} · r{run.input_ref.scenario_revision}</>}</span><select aria-label="历史结果" value={run?.id ?? ''} disabled={!project || busy || editing} onChange={event => void action(() => chooseRun(event.target.value))}><option value="">未选择 Run</option>{runs.map(summary => <option key={summary.run_id} value={summary.run_id}>{summary.run_id} · {summary.computation_status}</option>)}</select><select aria-label="时间轴比较结果" value={comparison?.run.id ?? ''} disabled={!run || busy || editing} onChange={event => void action(() => chooseComparison(event.target.value))}><option value="">不选择比较 Run</option>{runs.filter(summary => summary.run_id !== run?.id).map(summary => <option key={summary.run_id} value={summary.run_id}>{summary.run_id} · 对比参考</option>)}</select></div>
        <div className="canvas-content" style={{ zoom: editing ? propertyTextScale : undefined }}>{agentOpen && snapshot ? <AgentWorkspace key={agentEditorIdentity(snapshot)} workspace={snapshot} onRefresh={() => refreshLatest.current()} onClose={() => setAgentOpen(false)} onEditorStateChanged={receiveAgentEditorState} /> : checkOpen && project && snapshot ? <FactoryCheckWorkspace project={project} supported={snapshot.capabilities.implemented_operations.some(value => value.operation === 'check_factory_draft')} selection={selections} busy={busy} onDirtyChange={setCheckDirty} onClose={() => setCheckOpen(false)} onLocate={selectObject} /> : transformProposal ? <LayoutTransformProposal proposal={transformProposal} busy={busy} onApply={applyTransform} onReload={reloadLayout} onClose={() => setTransformProposal(null)} onDirtyChange={setLayoutDirty} /> : layoutEditor ? <LayoutSelectionEditor project={layoutEditor.project} selection={layoutEditor.selection} mode={layoutEditor.mode} busy={busy} onDirtyChange={setLayoutDirty} onApply={async (commands, base) => { await commitLayout(commands, base); setLayoutEditor(null); }} onReload={reloadLayout} onClose={() => setLayoutEditor(null)} /> : itemLayout && snapshot ? <ItemLayoutWorkspace onDirtyChange={setItemLayoutDirty} workspace={snapshot} initialOwner={itemLayout.initialOwner} busy={busy} onApply={commitItemLayout} onClose={() => setItemLayout(null)} /> : deviceProcess && snapshot && processAuthoringIO && processProjectRevision !== undefined ? <DeviceProcessWorkspace key={deviceProcess.id} workspace={snapshot} scope={deviceProcess.scope} scopeRevision={deviceProcess.scope.kind === 'definition_draft' ? deviceProcess.draft!.summary.revision : processProjectRevision} selection={deviceProcess.selection} actions={processActions} editor={{ onEditorStateChanged: receiveProcessEditorState, draft: deviceProcess.draft, source: deviceProcess.module, io: processAuthoringIO, samples: processSamples, scenario: snapshot.scenarios.find(value => value.id === scenarioId) ?? null, actions: { configure: inspectSavedProcessDraft, inspect: inspectSavedProcessDraft, preview_cycle: runProcessCycle }, run: deviceProcess.selection.kind === 'actual_run' && deviceProcess.scope.kind === 'project' && run?.id === deviceProcess.selection.run_id ? run : null, observation: deviceProcess.selection.kind === 'actual_run' && observationPage?.run_id === deviceProcess.selection.run_id ? observationPage : null, instance: selectedDevice ? { id: selectedDevice.id, module_ref: selectedDevice.module_ref } : null, onDraftChanged: receiveProcessDraft, onCommitted: async commit => { rememberCommit(commit); await refreshLatest.current(); }, onSampleRun: receiveProcessSample, onAdoptionRequested: reference => { setDeviceProcess(null); void action(() => openAuthoring(reference)); }, onImportPackageRequested: () => returnToAuthoring(deviceProcess.draft) }} onSelectionChanged={value => { setDeviceProcess(current => current ? { ...current, selection: value } : current); if (deviceProcess.scope.kind === 'project' && (value.kind === 'actual_run' || value.kind === 'frozen_device') && run?.id === value.run_id) void action(() => seekRun(value.run_id, value.at_ms)); }} onReturnToAuthoring={returnToAuthoring} onInspectItem={inspectProcessItem} onApplyNode={commitProcessNode} onEditNodeSource={editProcessNodeSource} onNodeDirtyChange={receiveNodeDirty} onClose={() => setDeviceProcess(null)} /> : packagingScope && packagingWorkspace && project && authoringIO && packagingRevision !== undefined ? <PackagingWorkspace onEditorStateChanged={receivePackagingEditorState} key={JSON.stringify([snapshot?.context.host.session_id, project.id, packagingScope])} workspace={packagingWorkspace} scope={packagingScope} scopeRevision={packagingRevision} io={authoringIO} manufacturingExport={{ status: 'available', start: startManufacturingPackageExport }} readManufacturingFile={readManufacturingFile} storedArtifacts={{ status: 'available', list: async () => listStoredPackaging(await packagingReadIdentity()), read: async artifactId => { const current = workspaceRef.current; if (!current?.project || current.project.id !== project.id) throw new Error('工程已切换，请在当前工程重新读取成果。'); const context = await readPackagingContext(current, packagingScope); const revision = packagingScope.kind === 'definition_draft' ? context.definition?.revision : current.project.revision; if (revision === undefined) throw new Error('未读取到此实际专业 Scope 修订。'); const value = await readStoredPackaging({ project_id: current.project.id, scope: packagingScope, scope_revision: revision, artifact_id: artifactId }); return { ...value, inputReadIssue: null }; } }} onCommitted={async commit => { rememberCommit(commit); await refreshPackaging(); }} onRefresh={refreshPackaging} onClose={() => { setPackagingScope(null); setPackagingContext(null); }} /> : authoring && snapshot && authoringIO && authoringTemplates ? <div className={`module-authoring-integrated ${authoringPresentation ? 'has-preview' : ''}`}><ModuleAuthoringWorkspace onDirtyChange={setDefinitionDirty} key={`${snapshot.context.host.session_id}:${project?.id}:${authoring.id}`} workspace={snapshot} busy={busy} io={authoringIO} samples={authoringSamples} templates={authoringTemplates} initialModule={authoring.module} initialDraft={authoring.draft} onNavigationGuard={registerAuthoringNavigation} onDraftChanged={setAuthoringDraft} onCommitted={async commit => { rememberCommit(commit); await refreshLatest.current(); }} onPreviewChange={(presentation, source) => setAuthoringPresentation(presentation ? { presentation, source: source ?? null } : null)} onConsumerRequested={(category, reference) => void action(() => openAuthoringConsumer(category, reference))} onClose={closeAuthoring} />{authoringPresentation && <AuthoringPresentationPreview value={authoringPresentation} />}</div> : deliveryTab && snapshot ? <DeliveryWorkspace key={`${project?.id}:${deliveryTab}`} workspace={snapshot} services={deliveryServices} initialTab={deliveryTab} onClose={() => setDeliveryTab(null)} onInspectRun={id => { setDeliveryTab(null); void action(() => chooseRun(id)); }} onLocate={(id, object) => { setDeliveryTab(null); setSelection(object); void action(() => chooseRun(id)); }} onOpenPackage={() => { setDeliveryTab(null); void action(() => openPackage('file')); }} onEditModule={reference => void action(() => openAuthoring(reference))} /> : businessOpen && snapshot ? <BusinessEditor onDirtyChange={setBusinessDirty} editIntent={businessEditIntent} project={project} scenarios={snapshot.scenarios} modules={snapshot.modules} scenarioId={scenarioId || null} history={history} run={run} frozenProject={runProject} frozenScenario={runInputs?.scenario ?? null} observation={observationPage} atMs={atMs} busy={busy} canEditSetupPolicies onCommit={commitBusiness} onRefresh={async () => { await refresh(); }} onRun={runBusiness} onScenarioChange={setScenarioId} onLocate={locateBusiness} onClose={() => setBusinessOpen(false)} /> : libraryOpen ? <ModuleLibrary onDirtyChange={setLibraryDirty} key={libraryModule ? host.moduleKey(libraryModule) : 'library'} initialModule={libraryModule} initialCategory={libraryCategory} onTemplate={async (base, preview) => { setBusy(true); setMessage(''); setFailure(null); try { rememberCommit(await host.applyTemplate(base, preview)); await refresh(); setLibraryOpen(false); setMessage('模板已按确认预览采纳；示例输入仍与实际运行结果分开。'); } finally { setBusy(false); } }} project={project} busy={busy} onClose={() => setLibraryOpen(false)} onAdopt={async (base, reference) => { setBusy(true); setMessage(''); setFailure(null); try { rememberCommit(await host.adoptDefinition(base, reference)); await refresh(); setLibraryOpen(false); setMessage('定义已采纳到工程；设备与输入仍须明确创建。'); } finally { setBusy(false); } }} /> : connectionEditor && connectionEditor.base.project ? <ConnectionEditor onDirtyChange={setConnectionDirty} project={connectionEditor.base.project} modules={connectionEditor.base.modules} value={connectionEditor.value} busy={busy} onClose={() => setConnectionEditor(null)} onApply={async values => { setBusy(true); setMessage(''); setFailure(null); try { await edit(values, connectionEditor.base.project); setConnectionEditor(null); } finally { setBusy(false); } }} /> : editor && editor.base.project ? <ObjectEditor onDirtyChange={setObjectDirty} key={`${editor.target.kind}-${editor.target.value?.id ?? 'new'}-${editor.base.project.revision}`} onReload={reloadObjectEditor} project={editor.base.project} modules={editor.base.modules} target={editor.target} busy={busy} onClose={() => setEditor(null)} onApply={async values => { setBusy(true); setMessage(''); setFailure(null); try { await edit(values, editor.base.project); setEditor(null); } finally { setBusy(false); } }} /> : <FactoryScene project={displayedProject} modules={displayedModules} canvas={canvas} hallId={hallId} selection={selection} selections={selections} layoutControl={layoutControl} onTransform={transformLayout} onSelect={selectObject} lod={lod} autoLod={autoLod} resetKey={resetKey} run={source === 'run' ? run : null} observation={source === 'run' ? observationPage : null} mediaSource={visibleFactorySource} />}{(!project || !project.halls.length && !project.devices.length) && !editing && <section className="project-start" aria-label="工程开始方式"><header><img className="project-start-mark" src="/superfactory-icon.png" alt="超级工厂" width="64" height="64" /><h2>选择开始方式</h2></header><div>{([['demonstration', '展示任务', '打开展示任务工程目录，核对其精确模块与实际输入。'], ['template', '从模板开始', '浏览实际模板，明确采用的对象与精确版本。'], ['empty', '空工程', '从厂房与设备开始，逐步补齐必要参数。']] as const).map(([intent, title, detail]) => <button id={`start-${intent}`} key={intent} disabled={!connected || busy} onClick={() => beginProject(intent, true)}><strong>{title}</strong><span>{detail}</span></button>)}</div></section>}</div>
        <div className="canvas-status"><span>{currentHall?.title ?? '未选择厂房'} · {source === 'run' ? '只读结果' : '布局编辑'}</span><span>{source === 'run' ? `实际对象 ${runObjects.length}` : '占地与高度取工程声明'}</span></div>
      </section>
      <div className="resize-handle" role="separator" aria-label="属性栏宽度" onPointerDown={event => resize(event, 'right')} />
      <aside className={`inspector ${rightVisible ? '' : 'collapsed'}`} aria-label="对象属性"><header><strong>{rightVisible ? '对象属性' : ''}</strong><button aria-label={rightVisible ? '收起属性栏' : '展开属性栏'} onClick={() => setRightVisible(value => !value)}>{rightVisible ? '›' : '‹'}</button></header>{rightVisible && <div className="inspector-scroll" style={{ zoom: propertyTextScale }}>{selected ? <>
        <div className="object-heading"><small>{selection ? kindLabel[selection.kind] ?? selection.kind : ''}</small><h2>{selected.title}</h2><code>{selected.id}</code></div>
        <section><h3>身份与来源</h3><dl><dt>当前来源</dt><dd>{source === 'run' ? `Run ${run?.id}` : '工程草稿'}</dd><dt>修订</dt><dd>r{displayedProject?.revision}</dd>{selectedDevice && <><dt>定义</dt><dd>{selectedDevice.module_ref.id}</dd><dt>精确版本</dt><dd>{selectedDevice.module_ref.version}</dd><dt>行为来源</dt><dd>{exactModule?.definition.payload.category === 'devices' ? '精确设备定义' : '尚未读取'}</dd></>}</dl></section>
        {position && <section><h3>布局 · 厂房坐标</h3><dl>{(['x', 'y', 'z'] as const).map(key => <span className="dl-row" key={key}><dt>{key.toUpperCase()}</dt><dd>{position[key]} m</dd></span>)}<dt>设备高度</dt><dd>{selectedDevice?.height_m == null ? '未填写' : `${selectedDevice.height_m} m`}</dd><dt>占地轮廓</dt><dd>{selectedDevice?.footprint_m.length} 个顶点</dd></dl></section>}
        {selectedHall && <section><h3>场地</h3><dl><dt>宽 × 深</dt><dd>{selectedHall.width_m} × {selectedHall.depth_m} m</dd><dt>净高</dt><dd>{selectedHall.height_m == null ? '未填写' : `${selectedHall.height_m} m`}</dd></dl></section>}
        {selectedLine && <section><h3>编组</h3><p>{selectedLine.members.length} 个成员 · 移动组根保留成员局部布局</p>{displayedProject?.devices.filter(value => value.line_id === selectedLine.id || selectedLine.members.includes(value.id)).map(value => <button key={value.id} onClick={event => selectObject({ kind: 'device', id: value.id }, event.shiftKey || event.ctrlKey || event.metaKey)}>{value.title}</button>)}</section>}
        {exactModule?.definition.payload.category === 'devices' && <section><h3>端口 · 精确版本</h3>{exactModule.definition.payload.data.ports.length ? exactModule.definition.payload.data.ports.map(port => <div className="port-row" key={port.id}><strong>{port.id}</strong><span>{port.direction} · {port.payload.kind}</span></div>) : <p className="muted">定义未声明端口。</p>}</section>}
        {selectedResource && <section><h3>资源 · 真实声明</h3><dl><dt>定义</dt><dd>{selectedResource.definition_ref.id} @ {selectedResource.definition_ref.version}</dd><dt>种类 / 角色</dt><dd>{selectedResource.kind} / {selectedResource.role}</dd><dt>容量</dt><dd>{decimalText(selectedResource.capacity.value)} {selectedResource.capacity.unit}</dd><dt>日历</dt><dd>{selectedResource.calendar_id}</dd><dt>声明位置</dt><dd>{selectedResource.home_location ?? '未提供'}</dd></dl>{selectedResource.home_location && <button onClick={() => selectObject({ kind: 'location', id: selectedResource.home_location! })}>查看声明库位</button>}</section>}
        {selectedTransport && <section aria-label="实际运输批次"><h3>实际运输 · {atMs == null ? '未知时点' : `${atMs / 1000}s`}</h3><dl><dt>当前阶段</dt><dd>{transportPhaseAt(selectedTransport, atMs)}</dd><dt>供货输入</dt><dd>{selectedTransport.supply_ref}</dd><dt>承运资源</dt><dd>{selectedTransport.carrier_id}</dd>{([['发出记录', selectedTransport.departed_ms], ['到达记录', selectedTransport.arrived_ms], ['接收记录', selectedTransport.received_ms], ['返库记录', selectedTransport.returned_ms]] as const).map(([label, time]) => <span className="dl-row" key={label}><dt>{label}</dt><dd>{time == null ? '未形成记录' : `${time / 1000}s`}</dd></span>)}<dt>空间位置</dt><dd>未声明运输空间轨迹；不按时间推算位置</dd></dl><button onClick={() => selectObject({ kind: 'resource', id: selectedTransport.carrier_id })}>查看承运资源 {selectedTransport.carrier_id}</button><p>本次记录载货身份：{selectedTransport.cargo.length}</p>{selectedTransport.cargo.map(id => <button key={id} disabled={!items.some(item => item.id === id)} onClick={() => selectObject({ kind: 'item', id })}>查看货物 {id}</button>)}</section>}
        {selectedConnection?.kind === 'material' && <section><h3>物料交接</h3><dl><dt>物品</dt><dd>{selectedConnection.data.item_type}</dd><dt>单位</dt><dd>{selectedConnection.data.unit}</dd><dt>源端点</dt><dd>{selectedConnection.data.source.kind === 'location' ? selectedConnection.data.source.data : `${selectedConnection.data.source.data.instance_id} / ${selectedConnection.data.source.data.port_id}`}</dd><dt>目的端点</dt><dd>{selectedConnection.data.destination.kind === 'location' ? selectedConnection.data.destination.data : `${selectedConnection.data.destination.data.instance_id} / ${selectedConnection.data.destination.data.port_id}`}</dd><dt>接收库位</dt><dd>{selectedConnection.data.destination_store}</dd></dl><button disabled={source !== 'draft' || busy || editing} onClick={() => { if (snapshot) setConnectionEditor({ base: snapshot, value: selectedConnection.data }); }}>编辑物料连接</button></section>}
        {selectedItem && <section><h3>实际物品 · {atMs == null ? '未知时点' : `${atMs / 1000}s`}</h3><dl><dt>物品类型</dt><dd>{selectedItem.item_type_ref.id} r{selectedItem.item_type_ref.revision}</dd><dt>数量</dt><dd>{decimalText(selectedItem.quantity.value)} {selectedItem.quantity.unit}</dd><dt>实际持有者</dt><dd>{selectedItem.holder.kind} / {selectedItem.holder.id}</dd><dt>生命周期</dt><dd>{selectedItem.lifecycle}</dd><dt>质量</dt><dd>{selectedItem.quality}</dd><dt>容纳记录</dt><dd>{selectedItem.contents.length} 项</dd><dt>空间位置</dt><dd>{run && itemAnchor(selectedItem, items, run) ? '依实际持有者定位；未声明物品独立姿态' : '未定位；无空间位置记录'}</dd></dl>{selectedItem.contents.map(id => <button key={id} onClick={() => selectObject({ kind: 'item', id })}>查看容纳物品 {id}</button>)}</section>}
        <section><h3>问题与缺项</h3>{displayedProject?.issues.map((issue, i) => <p className="issue-row" key={i}>{issue.message}</p>)}{(source === 'run' ? runInputs?.moduleIssues : snapshot?.moduleIssues)?.map(issue => <p className="issue-row" key={host.moduleKey(issue.ref)}>{issue.ref.id} @ {issue.ref.version}：{issue.error.message}</p>)}{source === 'draft' && snapshot?.scenarioIssues.map(issue => <p className="issue-row" key={issue.id}>方案 {issue.id}：{issue.error.message}</p>)}<button disabled={source !== 'draft' || busy || editing} onClick={() => setCheckOpen(true)}>检查工程草稿…</button></section>
        {selectedDevice && <button disabled={!exactModule || busy || editing} onClick={() => void action(async () => openDeviceProcess())}>查看设备过程 / 图片编辑</button>}
        <button disabled={source !== 'draft' || busy || editing || !['device', 'hall', 'line'].includes(selection?.kind ?? '')} onClick={() => { if (selectedDevice) openEditor({ kind: 'device', value: selectedDevice }); else if (selectedLine) openEditor({ kind: 'line', value: selectedLine }); else if (selectedHall) openEditor({ kind: 'hall', value: selectedHall }); }}>打开详情编辑</button>
      </> : <div className="inspector-empty"><h2>选择一个对象</h2><p>在画布或对象树中单击，查看设备、厂房和流水线属性。</p><p className="muted">缩放只改变画布呈现。</p></div>}{source === 'run' && observationPage && <section className="runtime-items"><h3>实际物品记录 · {atMs == null ? '未知时点' : `${atMs / 1000}s`}</h3>{items.map(item => <button key={item.id} className={same(selection, { kind: 'item', id: item.id }) ? 'selected' : ''} onClick={() => selectObject({ kind: 'item', id: item.id })}><strong>{item.id}</strong><span>{decimalText(item.quantity.value)} {item.quantity.unit} · {item.lifecycle}</span><small>{item.holder.kind} / {item.holder.id}</small></button>)}{!items.length && <p className="muted">该时点无物品记录。</p>}</section>}</div>}</aside>
    </div>
    <div className="toolstrip"><button className={layoutMode === 'select' ? 'active' : ''} title="V：选择；Shift/Ctrl 单击增减多选；M：移动；R：旋转；G：编组；Ctrl+D：复制" onClick={() => setLayoutMode('select')}>↖ 选择</button>{(['translate', 'rotate'] as const).map(mode => <button key={mode} className={layoutMode === mode ? 'active' : ''} disabled={(!selectedDevice && !selectedLine && !(selectedHall && canvas === 'multi_hall_canvas')) || source !== 'draft' || busy || editing} onClick={() => setLayoutMode(mode)}>{mode === 'translate' ? '↔ 移动手柄' : '⟳ 旋转手柄'}</button>)}
      <label className="check-label"><input type="checkbox" checked={snapLayout} onChange={event => setSnapLayout(event.target.checked)} />网格吸附</label><label>步长 / m <input aria-label="网格步长" type="number" min="0.01" step="any" value={snapStep} onChange={event => setSnapStep(event.target.value)} style={{ width: 65 }} /></label>{snapLayout && !snapValid && <small role="alert">步长须大于0</small>}
      <button disabled={!selectedLayout?.devices.length || source !== 'draft' || busy || editing} onClick={() => { if (project) setLayoutEditor({ mode: 'group', project, selection: selections }); }}>编组所选</button><button disabled={!selectedLayout?.lines.length || source !== 'draft' || busy || editing} onClick={() => { if (project) void action(() => commitLayout(ungroupLayoutSelection(project, selections), project)); }}>解组</button><button disabled={!selectedLayout?.devices.length || source !== 'draft' || busy || editing} onClick={() => { if (project) setLayoutEditor({ mode: 'copy', project, selection: selections }); }}>复制为新ID</button>
      <button disabled={!project || source !== 'draft' || busy || editing} onClick={() => setCheckOpen(true)}>草稿检查</button><button onClick={() => setResetKey(value => value + 1)}>⌂ 复位视图</button><span /><small>{selections.length > 1 ? `已选 ${selections.length} 项 · ` : ''}布局 m · 时间 s</small></div>
    {(timelineVisible || descriptionVisible) && <div className="dock-resizer" role="separator" aria-label="底部停靠高度" onPointerDown={event => resize(event, 'bottom')} />}
    <div className="bottom-dock" style={!timelineVisible && !descriptionVisible ? { display: 'none' } : undefined}>
      {descriptionVisible && <section className={`description ${timelineVisible ? '' : 'fill'}`}><header><strong>事件描述</strong><button aria-label="隐藏事件描述" onClick={() => setDescriptionVisible(false)}>×</button></header><div>{timelineSelection ? <><strong>{timelineSelection.label}</strong>{selectedTimelineJob && <button disabled={source !== 'run' || atMs === null || busy || editing} onClick={() => void action(async () => openTimelineJobProcess())}>查看所选作业过程</button>}<p>{timelineSelection.source === 'actual' ? `实际 · Run ${timelineSelection.run_id}` : timelineSelection.source === 'input' ? '可编辑输入' : '人工期望参考'}</p><p>{timelineSelection.object.kind} · {timelineSelection.object.id}</p>{timelineSelection.true_interval && <p>原始区间 {timelineSelection.true_interval.start_ms / 1000}–{timelineSelection.true_interval.end_ms / 1000}s</p>}<dl>{timelineSelection.fields.map((field, index) => <span className="dl-row" key={index}><dt>{field.label}</dt><dd>{field.value}</dd></span>)}</dl>{timelineSelection.related_objects.map(value => <button key={`${value.kind}-${value.id}`} onClick={() => selectObject(value)}>{kindLabel[value.kind] ?? value.kind} · {value.id}</button>)}</> : selection ? <><strong>{selected?.title ?? selection.id}</strong><p>{kindLabel[selection.kind] ?? selection.kind} · {selection.id}</p><p>{source === 'run' ? `Run ${run?.id} · ${atMs == null ? '未知时点' : `${atMs / 1000}s`}` : '工程草稿 · 尚非生产结果'}</p></> : <p className="muted">选择对象或实际阶段，查看其来源与时点。</p>}</div></section>}
      <RunTimeline {...timelineProps} hidden={!timelineVisible} inputTextScale={propertyTextScale} />
    </div>
    <footer className="app-status"><span>{connected ? '桌面 Host' : '浏览器预览'} · {busy ? '正在处理' : '就绪'}</span><span>{hasUnappliedInput ? `本地输入未应用${snapshot?.context.project?.dirty ? ' · 有未保存修订' : ''}` : source === 'run' ? '回看不改变工程库存' : snapshot?.context.project?.dirty ? '有未保存修订' : '工程状态'}</span></footer>
    {newDialog && <WorkbenchModal labelId="new-project-title" busy={busy} onDismiss={() => setNewDialog(false)}><form className="modal" onSubmit={event => { event.preventDefault(); void action(async () => { rememberCommit(await host.createProject(newTitle, departure)); await refresh(); setNewDialog(false); if (startIntent !== 'empty') { setLibraryModule(undefined); setLibraryCategory('templates'); setLibraryOpen(true); } }); }}><h2 id="new-project-title">{({ empty: '新建空工程', template: '从模板开始', demonstration: '从展示任务开始' })[startIntent]}</h2><label>工程名称<input value={newTitle} onChange={event => setNewTitle(event.target.value)} required autoFocus /></label>{departureChoice()}<footer><button type="button" disabled={busy} onClick={() => setNewDialog(false)}>取消</button><button className="primary" disabled={busy || !newTitle.trim() || !!project && departure === 'stay'}>创建</button></footer></form></WorkbenchModal>}
    {opening && <WorkbenchModal labelId="open-project-title" busy={busy} onDismiss={() => setOpening(null)}><section className="modal"><h2 id="open-project-title">打开工程 · 采纳前核对</h2><p>{opening.draft.proposed_objects.length} 个对象 · {opening.draft.proposed_definitions.length} 个精确模块版本</p>{[...opening.draft.missing, ...opening.draft.conflicts].map((issue, i) => <p className="issue-row" key={i}>{issue.message}</p>)}{opening.draft.required_human_choices.map((choice, i) => <p key={i}>{choice}</p>)}{opening.draft.missing.length > 0 && <label className="check-label"><input type="checkbox" checked={allowMissing} onChange={event => setAllowMissing(event.target.checked)} />保留缺项，以可编辑草稿打开</label>}{departureChoice()}<footer><button disabled={busy} onClick={() => setOpening(null)}>取消</button><button className="primary" disabled={busy || !!project && departure === 'stay' || opening.draft.missing.length > 0 && !allowMissing} onClick={() => void action(async () => { rememberCommit(await host.adoptOpen(opening.task, opening.draft, departure, allowMissing)); await refresh(true); setOpening(null); })}>确认打开</button></footer></section></WorkbenchModal>}
    {processSampleChoice && <ProcessSampleChooser key={`${processSampleChoice.run_id}:${processSampleChoice.draft_revision}`} choice={processSampleChoice} onDismiss={() => setProcessSampleChoice(null)} />}
    {windowClosePrompt && <WorkbenchModal labelId="window-close-title" busy={false} onDismiss={() => decideWindowClose('stay')}><section className="modal">
      <h2 id="window-close-title">关闭工程 · 尚有未保存内容</h2>
      <p>保存工程成功后再关闭，或明确放弃本次会话的未保存内容。</p>
      {hasUnappliedInput && <p>仍有未应用的编辑输入。请先留在当前窗口，应用或丢弃这些输入，再保存工程。</p>}
      <p>未提交的定义草稿须先在对应工作区提交；保存工程不会替你提交定义。</p>
      <footer><button autoFocus onClick={() => decideWindowClose('stay')}>留在当前窗口</button><button disabled={!project || hasUnappliedInput} onClick={() => decideWindowClose('save')}>保存工程后关闭</button><button onClick={() => decideWindowClose('discard')}>放弃未保存内容并关闭</button></footer>
    </section></WorkbenchModal>}
  </main>;

  function treeDevice(id: string, title: string) {
    return <button key={id} className={`tree-leaf ${selections.some(ref => same(ref, { kind: 'device', id })) ? 'selected' : ''}`} onClick={event => selectObject({ kind: 'device', id }, event.shiftKey || event.ctrlKey || event.metaKey)}>▣ {title}</button>;
  }
  function departureChoice() {
    return project && <><p>保存工程不会提交定义草稿。未提交的定义草稿须先在对应工作区提交，再确认离开。</p><label>离开当前工程<select value={departure} onChange={event => setDeparture(event.target.value as DepartureDecision)}><option value="stay">留在当前工程</option>{!snapshot?.context.project?.dirty && <option value="already_saved">工程已保存、定义已提交，继续</option>}<option value="discard_after_tasks_terminal">任务结束后放弃当前未保存更改</option></select></label></>;
  }
}

