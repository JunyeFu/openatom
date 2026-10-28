import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import type { RunObjectsPage } from '../../../../../packages/commands/generated/typescript/RunObjectsPage';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { HistoryAvailability } from '../../../../../packages/commands/generated/typescript/HistoryAvailability';
import type { WorkspaceSnapshot, DepartureDecision, EditCommand, TaskHandle, ImportDraft, ProjectState } from '../../services/projectHost';
import * as host from '../../services/projectHost';
import { FactoryScene, type LodMode } from '../../scene/FactoryScene';
import { deviceHallPosition } from '../../scene/layout';
import { ObjectEditor, type EditorTarget } from './ObjectEditor';
import { ConnectionEditor } from './ConnectionEditor';
import { ModuleLibrary } from './ModuleLibrary';
import { BusinessEditor } from '../business/BusinessEditor';
import type { BusinessEditIntent } from '../business/types';
import type { BusinessCommitProposal, BusinessCommitResult, BusinessRunRequest, BusinessLocateRequest } from '../business/types';
import { DeliveryWorkspace } from '../reporting/DeliveryWorkspace';
import { PackagingWorkspace } from '../packaging/PackagingWorkspace';
import type { PackagingScope } from '../packaging/types';
import type { ProjectContext } from '../../../../../packages/commands/generated/typescript/ProjectContext';
import { readStoredPackaging, listStoredPackaging } from '../../services/storedPackaging';
import { startManufacturingPackageExport } from '../../services/manufacturingExport';
import { DeviceProcessWorkspace, type ProcessActions } from './DeviceProcessWorkspace';
import type { DeviceProcessSelection, ProcessReadContext, ProcessScope } from '../../services/presentationHost';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { RunView } from '../../../../../packages/commands/generated/typescript/RunView';
import { ModuleAuthoringWorkspace } from '../modules';
import { ItemLayoutWorkspace, type ItemLayoutOwner } from '../item-layout';
import type { AuthoringDraft, AuthoringNavigationGuard } from '../modules/types';
import { readDraft } from '../modules/model';
import { createModuleAuthoringIO, createAuthoringSamples, createAuthoringTemplates } from '../../services/moduleAuthoring';
import { ModulePresentationPreview } from '../../scene/ModulePresentationPreview';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ApiCategory } from '../../../../../packages/commands/generated/typescript/ApiCategory';
import { createFactoryPresentation, factoryContextKey, type FactoryPresentationContext, type FactoryPresentationSource } from '../../services/factoryPresentation';
import type { DeliveryServices } from '../comparison/types';
import { prepareScenarioFork } from '../comparison/model';
import type { MaterialConnection } from '../../../../../packages/commands/generated/typescript/MaterialConnection';
import { RunTimeline } from '../timeline/RunTimeline';
import type { Props as TimelineProps, TimelineInputCommit, TimelineCommitResult, TimelineSelection, TimelineComparison } from '../timeline/types';
import { decimalText, itemAnchor } from '../../scene/runtime';

const kindLabel: Record<string, string> = { hall: '厂房', line: '流水线', device: '设备', location: '库位', resource: '资源', scenario: '方案', connection: '连接' };
const same = (a: ObjectRef | null, b: ObjectRef) => a?.id === b.id && a.kind === b.kind;

export function Workbench() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [failure, setFailure] = useState<Error | null>(null);
  const [selection, setSelection] = useState<ObjectRef | null>(null);
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
  const [libraryModule, setLibraryModule] = useState<ModuleRef>();
  const [authoring, setAuthoring] = useState<{ id: string; module?: ModuleRef; draft?: AuthoringDraft } | null>(null);
  const [authoringDraft, setAuthoringDraft] = useState<AuthoringDraft | null>(null);
  const [authoringPresentation, setAuthoringPresentation] = useState<Presentation | null>(null);
  const authoringNavigation = useRef<AuthoringNavigationGuard | null>(null);
  const registerAuthoringNavigation = useCallback((guard: AuthoringNavigationGuard | null) => { authoringNavigation.current = guard; }, []);
  const [businessOpen, setBusinessOpen] = useState(false);
  const [businessEditIntent, setBusinessEditIntent] = useState<BusinessEditIntent>();
  const [deliveryTab, setDeliveryTab] = useState<'comparison' | 'reporting' | 'handoff' | null>(null);
  const [packagingScope, setPackagingScope] = useState<PackagingScope | null>(null);
  const [packagingContext, setPackagingContext] = useState<ProjectContext | null>(null);
  const [deviceProcess, setDeviceProcess] = useState<{ id: string; scope: ProcessScope['scope']; selection: DeviceProcessSelection; draft: AuthoringDraft | null; module: ModulePackage | null } | null>(null);
  const [itemLayout, setItemLayout] = useState<{ initialOwner?: ItemLayoutOwner } | null>(null);
  const deviceProcessRef = useRef(deviceProcess); deviceProcessRef.current = deviceProcess;
  const [committedHistory, setCommittedHistory] = useState<{ projectId: string; revision: number; value: HistoryAvailability } | null>(null);
  const editing = editor !== null || connectionEditor !== null || libraryOpen || businessOpen || deliveryTab !== null || authoring !== null || packagingScope !== null || deviceProcess !== null || itemLayout !== null;
  const [newDialog, setNewDialog] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [departure, setDeparture] = useState<DepartureDecision>('stay');
  const [opening, setOpening] = useState<{ task: TaskHandle; draft: ImportDraft } | null>(null);
  const [allowMissing, setAllowMissing] = useState(false);
  const [taskState, setTaskState] = useState<TaskState | null>(null);
  const [scenarioId, setScenarioId] = useState('');
  const [run, setRun] = useState<RunResult | null>(null);
  const [runProject, setRunProject] = useState<ProjectState | null>(null);
  const [runInputs, setRunInputs] = useState<Awaited<ReturnType<typeof host.readRunInputs>> | null>(null);
  const [comparison, setComparison] = useState<TimelineComparison | null>(null);
  const [observationPage, setObservationPage] = useState<RunObjectsPage | null>(null);
  const [timelineSelection, setTimelineSelection] = useState<TimelineSelection | null>(null);
  const [atMs, setAtMs] = useState<number | null>(null);
  const [source, setSource] = useState<'draft' | 'run'>('draft');
  const observation = useRef(0);
  const projectIdentity = useRef<string | null>(null);
  const pendingRun = useRef<string | null>(null);
  const primaryRun = useRef<string | null>(null);
  const comparisonEpoch = useRef(0);
  const actionCount = useRef(0);
  const workspaceRef = useRef(snapshot);
  workspaceRef.current = snapshot;
  const project = snapshot?.project ?? null;
  const displayedProject = source === 'run' ? runProject : project;
  const displayedModules = source === 'run' ? runInputs?.modules : snapshot?.modules;
  const runObjects = observationPage?.objects ?? [];
  const items = runObjects.filter(value => value.kind === 'item').map(value => value.data);
  const connected = host.nativeHostAvailable();
  const commands = snapshot?.capabilities.available_command_kinds ?? [];
  const runs = snapshot?.context.run_summaries ?? [];
  const definitionDraftIds = [...new Set(snapshot?.capabilities.implemented_operations
    .filter(operation => operation.operation === 'get_context')
    .flatMap(operation => operation.allowed_scopes)
    .filter(scope => scope.kind === 'definition_draft')
    .map(scope => scope.data) ?? [])];

  const factoryCurrent = useRef<FactoryPresentationContext | null>(null);
  factoryCurrent.current = connected && snapshot && project && displayedProject && !editing ? {
    session_id: snapshot.context.host.session_id, project_id: project.id, scope: { kind: 'project' },
    scope_revision: project.revision, project: displayedProject,
    view: source === 'run' && run && atMs !== null ? { run_id: run.id, at_ms: atMs } : null,
  } : null;
  const factoryKey = factoryCurrent.current ? factoryContextKey(factoryCurrent.current) : null;
  const factoryReader = useMemo(() => createFactoryPresentation(() => factoryCurrent.current), []);
  const [factorySource, setFactorySource] = useState<FactoryPresentationSource | null>(null);
  const visibleFactorySource = factorySource?.key === factoryKey ? factorySource : null;
  useEffect(() => {
    const captured = factoryCurrent.current;
    setFactorySource(null);
    if (!captured) return;
    let live = true;
    factoryReader.read(captured).then(value => {
      if (live && factoryCurrent.current && value.key === factoryContextKey(factoryCurrent.current)) setFactorySource(value);
    }, error => {
      if (live && factoryCurrent.current && factoryContextKey(factoryCurrent.current) === factoryContextKey(captured)) setFailure(error instanceof Error ? error : new Error(String(error)));
    });
    return () => { live = false; };
  }, [factoryReader, factoryKey]);

  async function refresh(resetResults = false) {
    if (resetResults) { observation.current++; pendingRun.current = null; }
    const next = await host.readWorkspace();
    const changedProject = projectIdentity.current !== (next.project?.id ?? null);
    if (changedProject) {
      setBusinessOpen(false); setBusinessEditIntent(undefined);
      setAuthoring(null); setAuthoringDraft(null); setAuthoringPresentation(null);
      setPackagingScope(null); setPackagingContext(null);
      setDeviceProcess(null);
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

  async function readPackagingContext(current: WorkspaceSnapshot, scope: PackagingScope) {
    if (!current.project || !authoringIO) throw new Error('请先打开实际工程。');
    if (scope.kind === 'project') return current.context;
    return authoringIO.dispatch<ProjectContext>({ api_version: '0.1', request_id: `packaging-context-${crypto.randomUUID()}`, project_id: current.project.id, scope, operation: 'get_context', params: { selection: [] } });
  }
  async function openPackaging(scope: PackagingScope = { kind: 'project' }) {
    const current = await refreshLatest.current();
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
    const jobId = actual?.frame?.job_id;
    if (source === 'run' && (!run || atMs === null || !jobId)) throw new Error(actual?.unavailable ?? '此时点尚未读到唯一活动作业；主画布可查看静态设备，也可在草稿中检查声明。');
    setDeviceProcess({ id: crypto.randomUUID(), scope: { kind: 'project' }, selection: source === 'run' && run && jobId && atMs !== null ? { kind: 'actual_run', run_id: run.id, job_id: jobId, at_ms: atMs } : { kind: 'definition', module_ref: selectedDevice.module_ref, device_id: selectedDevice.id }, draft: null, module: exactModule });
  }
  const processSamples = createAuthoringSamples(() => workspaceRef.current, deviceProcess?.draft ?? null, authoringIO ?? undefined);
  function receiveProcessDraft(draft: AuthoringDraft) {
    setDeviceProcess(current => current ? { ...current, draft, selection: current.scope.kind === 'definition_draft' && current.selection.kind === 'definition' ? { kind: 'definition', module_ref: { id: draft.definition.id, version: draft.definition.version } } : current.selection } : current);
    if (authoring?.draft?.summary.draft_id === draft.summary.draft_id) {
      setAuthoringDraft(draft); setAuthoring(current => current ? { ...current, draft } : current);
    }
  }
  function receiveProcessSample(value: RunResult) {
    const current = deviceProcessRef.current, draft = current?.draft;
    if (!current || !draft || value.input_ref.project_id !== workspaceRef.current?.project?.id) return;
    const job = value.jobs.find(item => item.execution_ref.kind === 'definition_cycle');
    if (!job || value.computed_until_ms == null) throw new Error('实际样例未提供可读取作业或计算边界。');
    setDeviceProcess({ ...current, scope: { kind: 'definition_draft', data: draft.summary.draft_id }, selection: { kind: 'actual_run', run_id: value.id, job_id: job.id, at_ms: value.computed_until_ms } });
  }
  async function runProcessCycle() {
      const captured = deviceProcess?.id, draft = deviceProcess?.draft, scenario = snapshot?.scenarios.find(value => value.id === scenarioId);
      if (!draft || !scenario || processSamples.status !== 'available' || !authoringIO || !project) throw new Error(processSamples.status === 'unavailable' ? processSamples.reason : '请先保存真实草稿并选择适用方案。');
      const state = await processSamples.run(draft, scenario);
      if (state.result?.kind !== 'run') throw new Error('样例未返回实际 Run，不能以观看动画代替。');
      const value = await authoringIO.dispatch<RunView>({ api_version: '0.1', request_id: `process-sample-${crypto.randomUUID()}`, project_id: project.id, scope: { kind: 'definition_draft', data: draft.summary.draft_id }, operation: 'get_run', params: { run_id: state.result.data.run_id, query: { kind: 'full_result' } } });
      if (value.kind !== 'full') throw new Error('未读到此实际样例的完整结果。');
      if (deviceProcessRef.current?.id === captured) receiveProcessSample(value.data);
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
      items: 'items', resources: 'resources', transport: 'supplies', tasks: 'tasks', operations: 'stages',
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
      setAuthoringPresentation(object.data.definition.payload.data); return;
    }
    if (category === 'professional') {
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
    if (commit.scope.kind === 'project') setCommittedHistory({ projectId: commit.project_id, revision: commit.new_revision, value: commit.history });
  }
  const history = committedHistory && project && committedHistory.projectId === project.id && committedHistory.revision === project.revision ? committedHistory.value : null;
  async function edit(commandsToApply: EditCommand[], base = project) {
    if (!base) return;
    rememberCommit(await host.applyCommands(base, commandsToApply));
    await refresh(); setMessage('已应用到工程草稿。');
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
  function openEditor(target: EditorTarget) {
    if (!snapshot || !project || source !== 'draft') return;
    setEditor({ target, base: snapshot });
  }
  const currentHall = displayedProject?.halls.find(hall => hall.id === hallId);
  const selectedDevice = selection?.kind === 'device' ? displayedProject?.devices.find(device => device.id === selection.id) : undefined;
  const selectedLine = selection?.kind === 'line' ? displayedProject?.lines.find(line => line.id === selection.id) : undefined;
  const selectedHall = selection?.kind === 'hall' ? displayedProject?.halls.find(hall => hall.id === selection.id) : undefined;
  const selectedLocation = selection?.kind === 'location' ? displayedProject?.locations.find(location => location.id === selection.id) : undefined;
  const selectedConnection = selection?.kind === 'connection' ? displayedProject?.connections.find(connection => connection.data.id === selection.id) : undefined;
  const selectedItem = source === 'run' && selection?.kind === 'item' ? items.find(item => item.id === selection.id) : undefined;
  const selected = selectedDevice ?? selectedLine ?? selectedHall ?? selectedLocation ?? (selectedConnection ? { id: selectedConnection.data.id, title: ({ material: '物料连接', signal: '信号连接', artifact: '专业成果连接' })[selectedConnection.kind] } : selectedItem ? { id: selectedItem.id, title: selectedItem.item_type_ref.id } : undefined);
  const exactModule = selectedDevice ? displayedModules?.get(host.moduleKey(selectedDevice.module_ref)) : undefined;
  const position = selectedDevice && displayedProject ? deviceHallPosition(displayedProject, selectedDevice) : null;
  const activeTask = taskState && ['queued', 'running', 'paused', 'cancel_requested'].includes(taskState.task.state) ? taskState : null;

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
    const objects = await readObservation(id, time, projectId, epoch);
    if (objects === null) return;
    if (comparison?.run.id === id && run && runProject) setComparison({ project: runProject, run, scenario: runInputs?.scenario });
    primaryRun.current = id;
    setRun(view.data); setRunProject(frozen.project); setRunInputs(frozen); setAtMs(time); setObservationPage(objects); setTimelineSelection(current => current?.run_id === id ? current : null); setSource('run');
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
  async function readObservation(runId: string, time: number, projectId: string, epoch: number): Promise<RunObjectsPage | null> {
    const objects: DomainObject[] = [];
    let after: number | null = null;
    do {
      const result = await host.getRun(projectId, runId, { kind: 'objects_at', data: { at_ms: time, objects: [], page: { after_index: after, limit: 500 } } });
      if (result.kind !== 'objects') throw new Error('Host 未返回该观察时点的对象。');
      if (!ownsObservation(epoch, projectId, runId)) return null;
      if (result.data.run_id !== runId || result.data.at_ms !== time) throw new Error('Host 返回的观察时点或 Run 与请求不一致。');
      objects.push(...result.data.objects); after = result.data.next_index;
    } while (after !== null);
    return ownsObservation(epoch, projectId, runId) ? { run_id: runId, at_ms: time, objects, next_index: null } : null;
  }
  async function seekRun(runId: string, time: number) {
    if (!project || !run) return;
    if (runId !== run.id) {
      if (comparison?.run.id === runId) await chooseRun(runId, time);
      return;
    }
    const epoch = ++observation.current;
    pendingRun.current = runId;
    const objects = await readObservation(runId, time, project.id, epoch);
    if (objects !== null) { setAtMs(time); setObservationPage(objects); setSource('run'); }
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
  const timelineProps: TimelineProps = {
    project: run ? runProject : project, draftProject: project,
    run, runScenario: runInputs?.scenario ?? null,
    scenario: snapshot?.scenarios.find(value => value.id === scenarioId) ?? null,
    atMs, observation: observationPage, selected: timelineSelection, comparison,
    onSelect: setSelection, onInspect: value => { setTimelineSelection(value); if (value) setSelection(value.object); },
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
    setSource(next); setTimelineSelection(null);
    setHallId(current => graph?.halls.some(hall => hall.id === current) ? current : graph?.halls[0]?.id ?? null);
  }
  function selectObject(value: ObjectRef | null) {
    setSelection(value); setTimelineSelection(null);
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

  return <main className="workbench" style={{ '--left-width': `${leftVisible ? leftWidth : 32}px`, '--right-width': `${rightVisible ? rightWidth : 32}px`, '--dock-height': `${dockHeight}px` } as React.CSSProperties}>
    <div className="titlebar"><strong>超级工厂</strong><span>{project?.title ?? '未打开工程'}{snapshot?.context.project?.dirty ? ' *' : ''}</span><span className="title-status">{project ? `r${project.revision}` : '本地工程'}</span></div>
    <nav className="menubar" aria-label="主菜单" onClick={event => {
      if (event.target instanceof HTMLElement && event.target.closest('button')) event.target.closest('details')?.removeAttribute('open');
    }}>
      <details><summary>文件</summary><div className="menu-popup">
        <button disabled={!connected || busy || editing} onClick={() => { setNewTitle(''); setDeparture(project ? 'stay' : 'already_saved'); setNewDialog(true); }}>新建工程…</button>
        <button disabled={!connected || busy || editing} onClick={() => void action(() => openPackage('directory'))}>打开工程目录…</button><button disabled={!connected || busy || editing} onClick={() => void action(() => openPackage('file'))}>打开工程ZIP…</button>
        <button disabled={!project || busy} onClick={() => void action(async () => { if (!project) return; const result = await host.saveProject(project, await host.selectDirectory('save_project'), setTaskState); if (result.kind !== 'save') throw new Error('Host 未返回保存结果。'); await refresh(); setMessage(`已保存 r${result.data.saved_revision}${result.data.still_dirty ? '；当前仍有新修订未保存' : ''}。`); })}>保存工程…</button>
        <button disabled={!connected || busy} onClick={() => void action(refresh)}>刷新工程</button>
      </div></details>
      <details><summary>编辑</summary><div className="menu-popup"><button disabled={!project || busy || editing || !commands.includes('undo') || history?.can_undo === false} onClick={() => void action(() => edit([{ kind: 'undo' }]))}>撤销</button><button disabled={!project || busy || editing || !commands.includes('redo') || history?.can_redo === false} onClick={() => void action(() => edit([{ kind: 'redo' }]))}>重做</button><button disabled={!project || busy || editing || source !== 'draft' || !commands.includes('put_object')} onClick={() => setItemLayout({ initialOwner: selection && (selection.kind === 'device' || selection.kind === 'location' || selection.kind === 'item_type') ? { kind: selection.kind, id: selection.id } : undefined })}>物品展示槽…</button></div></details>
      <details><summary>视图</summary><div className="menu-popup"><button onClick={() => setResetKey(value => value + 1)}>复位为斜视</button>{(['detailed', 'simplified', 'region'] as LodMode[]).map(value => <button key={value} onClick={() => { setAutoLod(false); setLod(value); }}>{lod === value && !autoLod ? '✓ ' : ''}{({ detailed: '详细', simplified: '简化', region: '区域' })[value]}</button>)}<button onClick={() => setAutoLod(value => !value)}>{autoLod ? '✓ ' : ''}自动呈现档位</button></div></details>
      <details><summary>模块</summary><div className="menu-popup"><button disabled={!project || busy || editing} onClick={() => void action(() => openAuthoring())}>新建定义…</button>{definitionDraftIds.map(id => <button key={id} disabled={!project || busy || editing} onClick={() => void action(() => openAuthoring(undefined, id))}>继续定义草稿 · {id}</button>)}{!definitionDraftIds.length && <span className="form-hint">当前 Host 没有可读取的定义草稿</span>}</div></details><details><summary>设备过程</summary><div className="menu-popup"><button disabled={!authoringDraft || busy || deviceProcess !== null} onClick={() => navigateFromAuthoring("打开当前定义的图片过程", () => void action(openDraftProcess))}>当前定义的图片过程与标签…</button><button disabled={!selectedDevice || !exactModule || busy || editing} onClick={() => void action(async () => openDeviceProcess())}>所选设备的过程…</button></div></details><details><summary>专业工程</summary><div className="menu-popup"><button disabled={!project || busy || editing} onClick={() => void action(() => openPackaging())}>纸包装 · 结构与过程…</button>{authoringDraft?.definition.payload.category === 'professional' && <button disabled={busy} onClick={() => navigateFromAuthoring('打开当前专业定义的计算页', () => void action(() => openPackaging({ kind: 'definition_draft', data: authoringDraft.summary.draft_id })))}>在当前专业定义草稿中计算…</button>}</div></details><details><summary>评审与交付</summary><div className="menu-popup">{(['comparison', 'reporting', 'handoff'] as const).map(tab => <button key={tab} disabled={!project || busy || editing} onClick={() => void action(async () => { await refresh(); setDeliveryTab(tab); })}>{({ comparison: '方案比较', reporting: '评估报告', handoff: '工程交接' })[tab]}</button>)}</div></details><details><summary>窗口</summary><div className="menu-popup">{[['对象树', leftVisible, setLeftVisible], ['属性', rightVisible, setRightVisible], ['时间轴', timelineVisible, setTimelineVisible], ['事件描述', descriptionVisible, setDescriptionVisible]].map(([label, visible, update]) => <button key={String(label)} onClick={() => (update as (value: boolean) => void)(!visible)}>{visible ? '✓ ' : ''}{String(label)}</button>)}<button onClick={() => { setLeftVisible(true); setRightVisible(true); setTimelineVisible(true); setDescriptionVisible(true); setLeftWidth(256); setRightWidth(400); setDockHeight(160); }}>恢复默认布局</button></div></details>
      <span className="menu-divider" /><button className="menu-action" disabled={!connected || busy || editing || source !== 'draft'} onClick={() => void action(async () => { await refresh(); setLibraryModule(undefined); setLibraryOpen(true); })}>模块与模板库</button><button className="menu-action" disabled={!project || busy || source !== 'draft' || editing} onClick={() => openEditor({ kind: 'hall' })}>新建厂房</button><button className="menu-action" disabled={!project?.halls.length || busy || source !== 'draft' || editing} onClick={() => openEditor({ kind: 'device' })}>添加设备</button><button className="menu-action" disabled={!project?.halls.length || busy || source !== 'draft' || editing} onClick={() => openEditor({ kind: 'line' })}>新建线组</button><button className="menu-action" disabled={!project || busy || source !== 'draft' || editing} onClick={() => { if (snapshot) setConnectionEditor({ base: snapshot }); }}>连接物料</button>
      <button className="menu-action" disabled={!project || busy || editing} onClick={() => void action(() => openAuthoring())}>定义创作</button><button className="menu-action" disabled={!project || busy || editing} onClick={() => void action(async () => { await refresh(); setBusinessEditIntent(undefined); setBusinessOpen(true); })}>业务输入</button><span className="menu-spacer" /><select aria-label="当前方案" value={scenarioId} disabled={busy || editing} onChange={event => setScenarioId(event.target.value)}><option value="">未选择方案</option>{snapshot?.scenarios.map(scenario => <option key={scenario.id} value={scenario.id}>{scenario.title}</option>)}</select><button className="primary" disabled={!project || !scenarioId || busy || editing} onClick={() => void action(async () => { const scenario = snapshot?.scenarios.find(value => value.id === scenarioId); if (!project || !scenario) return; const result = await host.runScenario(project, scenario, setTaskState); if (result.kind !== 'run') throw new Error('Host 未返回 Run。'); await refresh(); await chooseRun(result.data.run_id); })}>运行方案</button>
    </nav>
    {(!connected || failure || message || activeTask) && <div className={`notice ${failure ? 'notice-error' : ''}`} role={failure ? 'alert' : 'status'}>
      {!connected ? '浏览器预览 · 工程操作请使用桌面应用' : failure ? `${failure.name}: ${failure.message}` : activeTask ? `任务 ${activeTask.task.id} · ${activeTask.task.state}` : message}
      {failure instanceof host.HostFailure && failure.detail.actual_revision !== null && <span>当前 r{failure.detail.actual_revision}；表单仍保留原值，请返回核对。</span>}
      {activeTask && <button onClick={() => void action(async () => { const result = await host.cancelTask(activeTask.task); setTaskState({ ...activeTask, task: result.task }); setMessage(result.action_accepted ? '已请求取消，等待任务实际终态。' : '任务已越过取消边界，请读取实际结果。'); })}>取消计算</button>}
    </div>}
    <div className="workspace-body">
      <aside className={`object-tree ${leftVisible ? '' : 'collapsed'}`} aria-label="工程对象树"><header><strong>{leftVisible ? '工程对象' : ''}</strong><button aria-label={leftVisible ? '收起对象树' : '展开对象树'} onClick={() => setLeftVisible(value => !value)}>{leftVisible ? '‹' : '›'}</button></header>{leftVisible && <>
        <div className="tree-caption">{displayedProject?.title ?? '未打开工程'}<small>{source === 'run' ? 'Run 冻结输入' : '工程草稿'}</small></div>
        <div className="tree-scroll">{displayedProject?.halls.map(hall => <details key={hall.id} open><summary><button className={same(selection, { kind: 'hall', id: hall.id }) ? 'selected' : ''} onClick={() => { selectObject({ kind: 'hall', id: hall.id }); setHallId(hall.id); }}>▱ {hall.title}</button></summary>{displayedProject.lines.filter(line => line.hall_id === hall.id).map(line => <details key={line.id} open className="tree-line"><summary><button onClick={() => selectObject({ kind: 'line', id: line.id })}>▤ {line.title}</button></summary>{displayedProject.devices.filter(device => device.line_id === line.id).map(device => treeDevice(device.id, device.title))}</details>)}{displayedProject.devices.filter(device => device.hall_id === hall.id && !device.line_id).map(device => treeDevice(device.id, device.title))}{displayedProject.locations.filter(value => value.hall_id === hall.id).map(location => <button className="tree-leaf" key={location.id} onClick={() => selectObject({ kind: 'location', id: location.id })}>▧ {location.title}</button>)}</details>)}{displayedProject?.connections.length ? <details open><summary>连接</summary>{displayedProject.connections.map(connection => <button key={connection.data.id} className={`tree-leaf ${same(selection, { kind: 'connection', id: connection.data.id }) ? 'selected' : ''}`} onClick={() => selectObject({ kind: 'connection', id: connection.data.id })}>{connection.kind === 'material' ? '物料' : connection.kind === 'signal' ? '信号' : '成果'} · {connection.data.id}</button>)}</details> : null}{project && !project.halls.length && <p className="muted">工程为空，先创建厂房。</p>}</div>
        <footer>设备 {displayedProject?.devices.length ?? 0} · 线组 {displayedProject?.lines.length ?? 0}</footer>
      </>}</aside>
      <div className="resize-handle" role="separator" aria-label="对象树宽度" onPointerDown={event => resize(event, 'left')} />
      <section className="central-workspace"><div className="canvas-tabs"><button className={canvas === 'line_canvas' ? 'active' : ''} onClick={() => setCanvas('line_canvas')}>流水线画布</button><button className={canvas === 'multi_hall_canvas' ? 'active' : ''} onClick={() => setCanvas('multi_hall_canvas')}>多厂房画布</button><span /><select aria-label="画布厂房" value={hallId ?? ''} onChange={event => setHallId(event.target.value)}>{displayedProject?.halls.map(hall => <option key={hall.id} value={hall.id}>{hall.title}</option>)}</select></div>
        <div className="sourcebar"><button className={source === 'draft' ? 'active' : ''} disabled={busy || editing} onClick={() => switchSource('draft')}>输入草稿</button><button disabled={!runProject || busy || editing} className={source === 'run' ? 'active' : ''} onClick={() => switchSource('run')}>实际回看</button><span>{source === 'run' ? `冻结 r${runProject?.revision} · ${atMs == null ? '时点未知' : `${atMs / 1000}s`}` : `草稿 r${project?.revision ?? '—'}`}</span><select aria-label="历史结果" value={run?.id ?? ''} disabled={!project || busy || editing} onChange={event => void action(() => chooseRun(event.target.value))}><option value="">未选择 Run</option>{runs.map(summary => <option key={summary.run_id} value={summary.run_id}>{summary.run_id} · {summary.computation_status}</option>)}</select><select aria-label="时间轴比较结果" value={comparison?.run.id ?? ''} disabled={!run || busy || editing} onChange={event => void action(() => chooseComparison(event.target.value))}><option value="">不选择比较 Run</option>{runs.filter(summary => summary.run_id !== run?.id).map(summary => <option key={summary.run_id} value={summary.run_id}>{summary.run_id} · 对比参考</option>)}</select></div>
        <div className="canvas-content">{itemLayout && snapshot ? <ItemLayoutWorkspace workspace={snapshot} initialOwner={itemLayout.initialOwner} busy={busy} onApply={commitItemLayout} onClose={() => setItemLayout(null)} /> : deviceProcess && snapshot && project && authoringIO ? <DeviceProcessWorkspace key={deviceProcess.id} workspace={snapshot} scope={deviceProcess.scope} scopeRevision={deviceProcess.scope.kind === 'definition_draft' ? deviceProcess.draft!.summary.revision : project.revision} selection={deviceProcess.selection} actions={processActions} editor={{ draft: deviceProcess.draft, source: deviceProcess.module, io: authoringIO, samples: processSamples, scenario: snapshot.scenarios.find(value => value.id === scenarioId) ?? null, actions: { configure: inspectSavedProcessDraft, inspect: inspectSavedProcessDraft, preview_cycle: runProcessCycle }, run: deviceProcess.selection.kind === 'actual_run' && deviceProcess.scope.kind === 'project' && run?.id === deviceProcess.selection.run_id ? run : null, observation: deviceProcess.selection.kind === 'actual_run' && observationPage?.run_id === deviceProcess.selection.run_id ? observationPage : null, instance: selectedDevice ? { id: selectedDevice.id, module_ref: selectedDevice.module_ref } : null, onDraftChanged: receiveProcessDraft, onCommitted: async commit => { rememberCommit(commit); await refreshLatest.current(); }, onSampleRun: receiveProcessSample, onAdoptionRequested: reference => { setDeviceProcess(null); void action(() => openAuthoring(reference)); }, onImportPackageRequested: () => returnToAuthoring(deviceProcess.draft) }} onSelectionChanged={value => { setDeviceProcess(current => current ? { ...current, selection: value } : current); if (deviceProcess.scope.kind === 'project' && value.kind === 'actual_run' && run?.id === value.run_id) void action(() => seekRun(value.run_id, value.at_ms)); }} onReturnToAuthoring={returnToAuthoring} onInspectItem={inspectProcessItem} onClose={() => setDeviceProcess(null)} /> : packagingScope && packagingWorkspace && project && authoringIO && packagingRevision !== undefined ? <PackagingWorkspace key={JSON.stringify([snapshot?.context.host.session_id, project.id, packagingScope])} workspace={packagingWorkspace} scope={packagingScope} scopeRevision={packagingRevision} io={authoringIO} manufacturingExport={{ status: 'available', start: startManufacturingPackageExport }} storedArtifacts={{ status: 'available', list: async () => listStoredPackaging(await packagingReadIdentity()), read: async artifactId => { const current = workspaceRef.current; if (!current?.project || current.project.id !== project.id) throw new Error('工程已切换，请在当前工程重新读取成果。'); const context = await readPackagingContext(current, packagingScope); const revision = packagingScope.kind === 'definition_draft' ? context.definition?.revision : current.project.revision; if (revision === undefined) throw new Error('未读取到此实际专业 Scope 修订。'); const value = await readStoredPackaging({ project_id: current.project.id, scope: packagingScope, scope_revision: revision, artifact_id: artifactId }); return { ...value, inputReadIssue: null }; } }} onCommitted={async commit => { rememberCommit(commit); await refreshPackaging(); }} onRefresh={refreshPackaging} onClose={() => { setPackagingScope(null); setPackagingContext(null); }} /> : authoring && snapshot && authoringIO && authoringTemplates ? <div className={`module-authoring-integrated ${authoringPresentation ? 'has-preview' : ''}`}><ModuleAuthoringWorkspace key={`${snapshot.context.host.session_id}:${project?.id}:${authoring.id}`} workspace={snapshot} busy={busy} io={authoringIO} samples={authoringSamples} templates={authoringTemplates} initialModule={authoring.module} initialDraft={authoring.draft} onNavigationGuard={registerAuthoringNavigation} onDraftChanged={setAuthoringDraft} onCommitted={async commit => { rememberCommit(commit); await refreshLatest.current(); }} onPreviewChange={setAuthoringPresentation} onConsumerRequested={(category, reference) => void action(() => openAuthoringConsumer(category, reference))} onClose={closeAuthoring} />{authoringPresentation && <ModulePresentationPreview presentation={authoringPresentation} />}</div> : deliveryTab && snapshot ? <DeliveryWorkspace key={`${project?.id}:${deliveryTab}`} workspace={snapshot} services={deliveryServices} initialTab={deliveryTab} onClose={() => setDeliveryTab(null)} onInspectRun={id => { setDeliveryTab(null); void action(() => chooseRun(id)); }} onLocate={(id, object) => { setDeliveryTab(null); setSelection(object); void action(() => chooseRun(id)); }} onOpenPackage={() => { setDeliveryTab(null); void action(() => openPackage('file')); }} onEditModule={reference => void action(() => openAuthoring(reference))} /> : businessOpen && snapshot ? <BusinessEditor editIntent={businessEditIntent} project={project} scenarios={snapshot.scenarios} modules={snapshot.modules} scenarioId={scenarioId || null} history={history} run={run} frozenProject={runProject} frozenScenario={runInputs?.scenario ?? null} observation={observationPage} atMs={atMs} busy={busy} canEditSetupPolicies onCommit={commitBusiness} onRefresh={async () => { await refresh(); }} onRun={runBusiness} onScenarioChange={setScenarioId} onLocate={locateBusiness} onClose={() => setBusinessOpen(false)} /> : libraryOpen ? <ModuleLibrary key={libraryModule ? host.moduleKey(libraryModule) : 'library'} initialModule={libraryModule} onTemplate={async (base, preview) => { setBusy(true); setMessage(''); setFailure(null); try { rememberCommit(await host.applyTemplate(base, preview)); await refresh(); setLibraryOpen(false); setMessage('模板已按确认预览采纳；示例输入仍与实际运行结果分开。'); } finally { setBusy(false); } }} project={project} busy={busy} onClose={() => setLibraryOpen(false)} onAdopt={async (base, reference) => { setBusy(true); setMessage(''); setFailure(null); try { rememberCommit(await host.adoptDefinition(base, reference)); await refresh(); setLibraryOpen(false); setMessage('定义已采纳到工程；设备与输入仍须明确创建。'); } finally { setBusy(false); } }} /> : connectionEditor && connectionEditor.base.project ? <ConnectionEditor project={connectionEditor.base.project} modules={connectionEditor.base.modules} value={connectionEditor.value} busy={busy} onClose={() => setConnectionEditor(null)} onApply={async values => { setBusy(true); setMessage(''); setFailure(null); try { await edit(values, connectionEditor.base.project); setConnectionEditor(null); } finally { setBusy(false); } }} /> : editor && editor.base.project ? <ObjectEditor key={`${editor.target.kind}-${editor.target.value?.id ?? 'new'}`} project={editor.base.project} modules={editor.base.modules} target={editor.target} busy={busy} onClose={() => setEditor(null)} onApply={async values => { setBusy(true); setMessage(''); setFailure(null); try { await edit(values, editor.base.project); setEditor(null); } finally { setBusy(false); } }} /> : <FactoryScene project={displayedProject} canvas={canvas} hallId={hallId} selection={selection} onSelect={selectObject} lod={lod} autoLod={autoLod} resetKey={resetKey} run={source === 'run' ? run : null} observation={source === 'run' ? observationPage : null} mediaSource={visibleFactorySource} />}</div>
        <div className="canvas-status"><span>{currentHall?.title ?? '未选择厂房'} · {source === 'run' ? '只读结果' : '布局编辑'}</span><span>{source === 'run' ? `实际对象 ${runObjects.length}` : '占地与高度取工程声明'}</span></div>
      </section>
      <div className="resize-handle" role="separator" aria-label="属性栏宽度" onPointerDown={event => resize(event, 'right')} />
      <aside className={`inspector ${rightVisible ? '' : 'collapsed'}`} aria-label="对象属性"><header><strong>{rightVisible ? '对象属性' : ''}</strong><button aria-label={rightVisible ? '收起属性栏' : '展开属性栏'} onClick={() => setRightVisible(value => !value)}>{rightVisible ? '›' : '‹'}</button></header>{rightVisible && <div className="inspector-scroll">{selected ? <>
        <div className="object-heading"><small>{selection ? kindLabel[selection.kind] ?? selection.kind : ''}</small><h2>{selected.title}</h2><code>{selected.id}</code></div>
        <section><h3>身份与来源</h3><dl><dt>当前来源</dt><dd>{source === 'run' ? `Run ${run?.id}` : '工程草稿'}</dd><dt>修订</dt><dd>r{displayedProject?.revision}</dd>{selectedDevice && <><dt>定义</dt><dd>{selectedDevice.module_ref.id}</dd><dt>精确版本</dt><dd>{selectedDevice.module_ref.version}</dd><dt>行为来源</dt><dd>{exactModule?.definition.payload.category === 'devices' ? '精确设备定义' : '尚未读取'}</dd></>}</dl></section>
        {position && <section><h3>布局 · 厂房坐标</h3><dl>{(['x', 'y', 'z'] as const).map(key => <span className="dl-row" key={key}><dt>{key.toUpperCase()}</dt><dd>{position[key]} m</dd></span>)}<dt>设备高度</dt><dd>{selectedDevice?.height_m == null ? '未填写' : `${selectedDevice.height_m} m`}</dd><dt>占地轮廓</dt><dd>{selectedDevice?.footprint_m.length} 个顶点</dd></dl></section>}
        {selectedHall && <section><h3>场地</h3><dl><dt>宽 × 深</dt><dd>{selectedHall.width_m} × {selectedHall.depth_m} m</dd><dt>净高</dt><dd>{selectedHall.height_m == null ? '未填写' : `${selectedHall.height_m} m`}</dd></dl></section>}
        {selectedLine && <section><h3>编组</h3><p>{selectedLine.members.length} 个成员 · 移动组根保留成员局部布局</p></section>}
        {exactModule?.definition.payload.category === 'devices' && <section><h3>端口 · 精确版本</h3>{exactModule.definition.payload.data.ports.length ? exactModule.definition.payload.data.ports.map(port => <div className="port-row" key={port.id}><strong>{port.id}</strong><span>{port.direction} · {port.payload.kind}</span></div>) : <p className="muted">定义未声明端口。</p>}</section>}
        {selectedConnection?.kind === 'material' && <section><h3>物料交接</h3><dl><dt>物品</dt><dd>{selectedConnection.data.item_type}</dd><dt>单位</dt><dd>{selectedConnection.data.unit}</dd><dt>源端点</dt><dd>{selectedConnection.data.source.kind === 'location' ? selectedConnection.data.source.data : `${selectedConnection.data.source.data.instance_id} / ${selectedConnection.data.source.data.port_id}`}</dd><dt>目的端点</dt><dd>{selectedConnection.data.destination.kind === 'location' ? selectedConnection.data.destination.data : `${selectedConnection.data.destination.data.instance_id} / ${selectedConnection.data.destination.data.port_id}`}</dd><dt>接收库位</dt><dd>{selectedConnection.data.destination_store}</dd></dl><button disabled={source !== 'draft' || busy || editing} onClick={() => { if (snapshot) setConnectionEditor({ base: snapshot, value: selectedConnection.data }); }}>编辑物料连接</button></section>}
        {selectedItem && <section><h3>实际物品 · {atMs == null ? '未知时点' : `${atMs / 1000}s`}</h3><dl><dt>物品类型</dt><dd>{selectedItem.item_type_ref.id} r{selectedItem.item_type_ref.revision}</dd><dt>数量</dt><dd>{decimalText(selectedItem.quantity.value)} {selectedItem.quantity.unit}</dd><dt>实际持有者</dt><dd>{selectedItem.holder.kind} / {selectedItem.holder.id}</dd><dt>生命周期</dt><dd>{selectedItem.lifecycle}</dd><dt>质量</dt><dd>{selectedItem.quality}</dd><dt>容纳记录</dt><dd>{selectedItem.contents.length} 项</dd><dt>空间位置</dt><dd>{run && itemAnchor(selectedItem, items, run) ? '依实际持有者定位；未声明物品独立姿态' : '未定位；无空间位置记录'}</dd></dl>{selectedItem.contents.map(id => <button key={id} onClick={() => selectObject({ kind: 'item', id })}>查看容纳物品 {id}</button>)}</section>}
        <section><h3>问题与缺项</h3>{displayedProject?.issues.map((issue, i) => <p className="issue-row" key={i}>{issue.message}</p>)}{(source === 'run' ? runInputs?.moduleIssues : snapshot?.moduleIssues)?.map(issue => <p className="issue-row" key={host.moduleKey(issue.ref)}>{issue.ref.id} @ {issue.ref.version}：{issue.error.message}</p>)}{source === 'draft' && snapshot?.scenarioIssues.map(issue => <p className="issue-row" key={issue.id}>方案 {issue.id}：{issue.error.message}</p>)}<p className="muted">静态场地检查尚未执行。</p></section>
        {selectedDevice && <button disabled={!exactModule || busy || editing} onClick={() => void action(async () => openDeviceProcess())}>查看设备过程 / 图片编辑</button>}
        <button disabled={source !== 'draft' || busy || editing || !['device', 'hall', 'line'].includes(selection?.kind ?? '')} onClick={() => { if (selectedDevice) openEditor({ kind: 'device', value: selectedDevice }); else if (selectedLine) openEditor({ kind: 'line', value: selectedLine }); else if (selectedHall) openEditor({ kind: 'hall', value: selectedHall }); }}>打开详情编辑</button>
      </> : <div className="inspector-empty"><h2>选择一个对象</h2><p>在画布或对象树中单击，查看设备、厂房和流水线属性。</p><p className="muted">缩放只改变画布呈现。</p></div>}{source === 'run' && observationPage && <section className="runtime-items"><h3>实际物品记录 · {atMs == null ? '未知时点' : `${atMs / 1000}s`}</h3>{items.map(item => <button key={item.id} className={same(selection, { kind: 'item', id: item.id }) ? 'selected' : ''} onClick={() => selectObject({ kind: 'item', id: item.id })}><strong>{item.id}</strong><span>{decimalText(item.quantity.value)} {item.quantity.unit} · {item.lifecycle}</span><small>{item.holder.kind} / {item.holder.id}</small></button>)}{!items.length && <p className="muted">该时点无物品记录。</p>}</section>}</div>}</aside>
    </div>
    <div className="toolstrip"><button className="active" title="单击选择对象">↖ 选择</button><button disabled={!selectedDevice && !selectedLine || source !== 'draft' || busy || editing} onClick={() => { if (selectedDevice) openEditor({ kind: 'device', value: selectedDevice }); else if (selectedLine) openEditor({ kind: 'line', value: selectedLine }); }}>↔ 移动 / 旋转</button><button onClick={() => setResetKey(value => value + 1)}>⌂ 复位视图</button><span /><small>布局单位 m · 时间显示 s</small></div>
    {(timelineVisible || descriptionVisible) && <><div className="dock-resizer" role="separator" aria-label="底部停靠高度" onPointerDown={event => resize(event, 'bottom')} /><div className="bottom-dock">
      {descriptionVisible && <section className={`description ${timelineVisible ? '' : 'fill'}`}><header><strong>事件描述</strong><button aria-label="隐藏事件描述" onClick={() => setDescriptionVisible(false)}>×</button></header><div>{timelineSelection ? <><strong>{timelineSelection.label}</strong><p>{timelineSelection.source === 'actual' ? `实际 · Run ${timelineSelection.run_id}` : timelineSelection.source === 'input' ? '可编辑输入' : '人工期望参考'}</p><p>{timelineSelection.object.kind} · {timelineSelection.object.id}</p>{timelineSelection.true_interval && <p>原始区间 {timelineSelection.true_interval.start_ms / 1000}–{timelineSelection.true_interval.end_ms / 1000}s</p>}<dl>{timelineSelection.fields.map((field, index) => <span className="dl-row" key={index}><dt>{field.label}</dt><dd>{field.value}</dd></span>)}</dl>{timelineSelection.related_objects.map(value => <button key={`${value.kind}-${value.id}`} onClick={() => selectObject(value)}>{kindLabel[value.kind] ?? value.kind} · {value.id}</button>)}</> : selection ? <><strong>{selected?.title ?? selection.id}</strong><p>{kindLabel[selection.kind] ?? selection.kind} · {selection.id}</p><p>{source === 'run' ? `Run ${run?.id} · ${atMs == null ? '未知时点' : `${atMs / 1000}s`}` : '工程草稿 · 尚非生产结果'}</p></> : <p className="muted">选择对象或实际阶段，查看其来源与时点。</p>}</div></section>}
      {timelineVisible && <RunTimeline {...timelineProps} />}
    </div></>}
    <footer className="app-status"><span>{connected ? '桌面 Host' : '浏览器预览'} · {busy ? '正在处理' : '就绪'}</span><span>{source === 'run' ? '回看不改变工程库存' : snapshot?.context.project?.dirty ? '有未保存修订' : '工程状态'}</span></footer>
    {newDialog && <div className="modal-backdrop"><form className="modal" onSubmit={event => { event.preventDefault(); void action(async () => { rememberCommit(await host.createProject(newTitle, departure)); await refresh(); setNewDialog(false); }); }}><h2>新建工程</h2><label>工程名称<input value={newTitle} onChange={event => setNewTitle(event.target.value)} required autoFocus /></label>{departureChoice()}<footer><button type="button" onClick={() => setNewDialog(false)}>取消</button><button className="primary" disabled={busy || !newTitle.trim() || !!project && departure === 'stay'}>创建</button></footer></form></div>}
    {opening && <div className="modal-backdrop"><section className="modal"><h2>打开工程 · 采纳前核对</h2><p>{opening.draft.proposed_objects.length} 个对象 · {opening.draft.proposed_definitions.length} 个精确模块版本</p>{[...opening.draft.missing, ...opening.draft.conflicts].map((issue, i) => <p className="issue-row" key={i}>{issue.message}</p>)}{opening.draft.required_human_choices.map((choice, i) => <p key={i}>{choice}</p>)}{opening.draft.missing.length > 0 && <label className="check-label"><input type="checkbox" checked={allowMissing} onChange={event => setAllowMissing(event.target.checked)} />保留缺项，以可编辑草稿打开</label>}{departureChoice()}<footer><button onClick={() => setOpening(null)}>取消</button><button className="primary" disabled={busy || !!project && departure === 'stay' || opening.draft.missing.length > 0 && !allowMissing} onClick={() => void action(async () => { rememberCommit(await host.adoptOpen(opening.task, opening.draft, departure, allowMissing)); await refresh(true); setOpening(null); })}>确认打开</button></footer></section></div>}
  </main>;

  function treeDevice(id: string, title: string) {
    return <button key={id} className={`tree-leaf ${same(selection, { kind: 'device', id }) ? 'selected' : ''}`} onClick={() => selectObject({ kind: 'device', id })}>▣ {title}</button>;
  }
  function departureChoice() {
    return project && <label>离开当前工程<select value={departure} onChange={event => setDeparture(event.target.value as DepartureDecision)}><option value="stay">留在当前工程</option>{!snapshot?.context.project?.dirty && <option value="already_saved">当前已保存，继续</option>}<option value="discard_after_tasks_terminal">任务结束后放弃当前未保存更改</option></select></label>;
  }
}
