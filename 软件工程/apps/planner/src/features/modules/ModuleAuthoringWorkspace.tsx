import { useEffect, useRef, useState } from 'react';
import type { CSSProperties, FormEvent, KeyboardEvent } from 'react';
import { SchemaField } from './SchemaField';
import { formSchemas } from './formSchema';
import { DeviceDefinitionEditor } from '../device-authoring/DeviceDefinitionEditor';
import { PresentationEditor } from '../device-authoring/PresentationEditor';
import { AdoptionPreparation, VersionChanges } from './AdoptionPreparation';
import { authoringRequest, categories, copy, definitionIssues, definitionSourceText, differences, newDefinition, payloadSchema, presentationOf, readDraft, refKey } from './model';
import { optionLabel } from './labels';
import { fieldLabel, authoringIssueMessage, authoringHistoryReason } from './labels';
import { fieldExamples } from './fieldExamples';
import { TemplateCompositionEditor } from './TemplateCompositionEditor';
import { compositionFromPreview, compositionInput, emptyComposition } from './templateComposition';
import { importAssetChoices } from './importAssets';
import { shapeIssues } from './model';
import { saveTemplateComposition } from './templateSave';
import { readAuthoringTemplatePreview } from '../../services/moduleAuthoring';
import { authoringPresentationSource } from '../../services/authoringPresentation';
import { readDefinitionSampleRun, sameSampleContext, sampleHasRun } from './definitionSample';
import type { DefinitionSampleContext } from './definitionSample';
import { AuthoringFailure, DefinitionSampleResult } from './DefinitionSampleResult';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { Category } from './model';
import type { AuthoringDraft, AuthoringNavigationGuard, ModuleAuthoringWorkspaceProps } from './types';
import type { ApiScope } from '../../../../../packages/commands/generated/typescript/ApiScope';
import type { ModuleDefinition } from '../../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { ModulePayload } from '../../../../../packages/commands/generated/typescript/ModulePayload';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleTemplateCandidates } from '../../../../../packages/commands/generated/typescript/ModuleTemplateCandidates';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { ImportDraft } from '../../../../../packages/commands/generated/typescript/ImportDraft';
import type { LibrarySourceScope } from '../../../../../packages/commands/generated/typescript/LibrarySourceScope';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { ModuleAdoption } from '../../../../../packages/commands/generated/typescript/ModuleAdoption';
import type { SiteEnvelope } from '../../../../../packages/commands/generated/typescript/SiteEnvelope';
import type { TemplateCompositionState } from './templateComposition';
import './authoring.css';

export function ModuleAuthoringWorkspace(props: ModuleAuthoringWorkspaceProps) {
  const { workspace, io } = props;
  const [projectId] = useState(workspace.project?.id ?? null);
  const [draft, setDraft] = useState<AuthoringDraft | null>(props.initialDraft ?? null);
  const [refreshDraftId, setRefreshDraftId] = useState<string | null>(props.initialDraft?.summary.draft_id ?? null);
  const [definition, setDefinition] = useState<ModuleDefinition>(() => copy(props.initialDraft?.definition ?? newDefinition('items')));
  const [baseline, setBaseline] = useState<ModuleDefinition>(() => copy(props.initialDraft?.definition ?? newDefinition('items')));
  const [source, setSource] = useState<ModulePackage | null>(null);
  const [committedRef, setCommittedRef] = useState<ModuleRef | null>(props.initialDraft ? null : props.initialModule ?? null);
  const [commitVersion, setCommitVersion] = useState(props.initialDraft?.definition.version ?? (props.initialModule ? '' : '0.1.0'));
  const [tab, setTab] = useState<'payload' | 'parameters' | 'rights' | 'sample'>('payload');
  const [pending, setPending] = useState(props.initialModule && !props.initialDraft && projectId ? '读取初始精确版本' : ''); const [error, setError] = useState<unknown>(null); const [message, setMessage] = useState('');
  const [catalog, setCatalog] = useState<ModuleTemplateCandidates | null>(null);
  const [sources, setSources] = useState<LibrarySourceScope[]>(['project_packages', 'bundled']);
  const [industry, setIndustry] = useState(''); const [family, setFamily] = useState('');
  const [site, setSite] = useState<SiteEnvelope | null>(null);
  const [sampleId, setSampleId] = useState(''); const [sampleResult, setSampleResult] = useState<TaskState | null>(null);
  const [sampleRun, setSampleRun] = useState<RunResult | null>(null);
  const [sampleReadError, setSampleReadError] = useState<unknown | null>(null);
  const [sampleReading, setSampleReading] = useState(false);
  const [sampleContext, setSampleContext] = useState<DefinitionSampleContext | null>(null);
  const sampleGeneration = useRef(0), sampleMounted = useRef(true);
  const currentSampleContext: DefinitionSampleContext = { project_id: workspace.project?.id ?? null, session_id: workspace.context.host.session_id, draft_id: draft?.summary.draft_id ?? null };
  const sampleContextRef = useRef(currentSampleContext); sampleContextRef.current = currentSampleContext;
  useEffect(() => { sampleMounted.current = true; return () => { sampleMounted.current = false; sampleGeneration.current++; }; }, []);
  useEffect(() => { clearSample(); }, [workspace.project?.id, workspace.context.host.session_id]);
  const [adopting, setAdopting] = useState<ModulePackage | null>(null);
  const [importing, setImporting] = useState<{ task: TaskHandle; value: ImportDraft; base: ProjectState } | null>(null);
  const [importRefs, setImportRefs] = useState<ModuleRef[]>([]); const [acceptMissing, setAcceptMissing] = useState(false);
  const [redistribute, setRedistribute] = useState(false);
  const [importAssets, setImportAssets] = useState<string[]>([]);
  const [composition, setComposition] = useState<TemplateCompositionState | null>(null);
  const [compositionDirty, setCompositionDirty] = useState(false);
  const [transition, setTransition] = useState<{ title: string; action(): void } | null>(null);
  const [textScale, setTextScale] = useState(1);
  const [previewEnabled, setPreviewEnabled] = useState(false);
  const previewCallback = useRef(props.onPreviewChange); previewCallback.current = props.onPreviewChange;
  useEffect(() => { if (previewEnabled) previewCallback.current(presentationOf(definition),
    (workspace.project?.id ?? null) === projectId ? authoringPresentationSource(workspace, draft, source, definition.contribution.assets) : null);
  }, [previewEnabled, definition, draft, source, workspace.project?.id, workspace.project?.revision, workspace.context.host.session_id, projectId]);
  useEffect(() => () => previewCallback.current(null), []);
  const form = useRef<HTMLFormElement>(null);
  const unreadModule = !!committedRef && !source && !draft;
  const disabled = props.busy || !!pending;
  const wrongProject = (workspace.project?.id ?? null) !== projectId;
  const dirty = differences(baseline, definition).length > 0 || compositionDirty;
  useEffect(() => {
    props.onDirtyChange?.(dirty);
    return () => props.onDirtyChange?.(false);
  }, [dirty, props.onDirtyChange]);
  const category = definition.payload.category;
  const categoryInfo = categories.find(item => item.id === category)!;
  const localIssues = unreadModule ? [] : definitionIssues(definition);
  const issues = [...localIssues, ...(draft?.summary.issues ?? [])];
  const availableModules = [...new Map([...workspace.modules.values(), ...(source ? [source] : [])].map(module => [refKey({ id: module.definition.id, version: module.definition.version }), module])).values()];
  const canWrite = !!projectId && !wrongProject && !unreadModule && workspace.capabilities.implemented_operations.some(operation => operation.operation === 'apply_commands');
  function report(failure: unknown) { setError(failure); }
  async function action(label: string, operation: () => Promise<void>) { setPending(label); setError(''); setMessage(''); try { await operation(); } catch (failure) { report(failure); } finally { setPending(''); } }
  function navigate(title: string, next: () => void) { if (dirty) setTransition({ title, action: next }); else next(); }
  const rootNavigation = useRef<AuthoringNavigationGuard>(() => {});
  rootNavigation.current = (title, proceed) => {
    if (disabled || wrongProject) { setError('当前定义操作尚未完成或工程已切换，请保留输入并在完成后重新选择入口。'); return; }
    navigate(title, proceed);
  };
  useEffect(() => {
    const register = props.onNavigationGuard;
    if (!register) return;
    register((title, proceed) => rootNavigation.current(title, proceed));
    return () => register(null);
  }, [props.onNavigationGuard]);
  function setSnapshot(snapshot: AuthoringDraft) { setDraft(snapshot); setRefreshDraftId(snapshot.summary.draft_id); setDefinition(copy(snapshot.definition)); setBaseline(copy(snapshot.definition)); setCommittedRef(null); props.onDraftChanged(snapshot); }
  function clearSample() { sampleGeneration.current++; setSampleResult(null); setSampleRun(null); setSampleReadError(null); setSampleReading(false); setSampleContext(null); }
  function currentSample(context: DefinitionSampleContext, generation: number) { return sampleMounted.current && generation === sampleGeneration.current && sameSampleContext(context, sampleContextRef.current); }
  async function loadSampleRun(task: TaskState, context: DefinitionSampleContext, generation: number) {
    if (!context.project_id || !currentSample(context, generation)) return;
    setSampleReading(true);
    try { const run = await readDefinitionSampleRun(io, context.project_id, task, () => currentSample(context, generation)); if (currentSample(context, generation)) setSampleRun(run); }
    catch (failure) { if (currentSample(context, generation)) setSampleReadError(failure); }
    finally { if (currentSample(context, generation)) setSampleReading(false); }
  }
  async function retrySampleRead() {
    if (!sampleContext || !sampleResult || !sampleHasRun(sampleResult) || !sameSampleContext(sampleContext, sampleContextRef.current)) return;
    const generation = ++sampleGeneration.current; setSampleRun(null); setSampleReadError(null);
    await loadSampleRun(sampleResult, sampleContext, generation);
  }
  function start(category: Category) { const next = newDefinition(category); setDefinition(next); setBaseline(copy(next)); setDraft(null); setRefreshDraftId(null); setSource(null); setCommittedRef(null); setCommitVersion(next.version); setTab('payload'); clearSample(); setError(''); setMessage(''); setComposition(null); setCompositionDirty(false); props.onDraftChanged(null); setPreviewEnabled(false); props.onPreviewChange(null); }
  function valid(): boolean { if (!form.current?.reportValidity()) return false; if (localIssues.length) { setError(localIssues.map(issue => issue.message).join(' ')); return false; } return true; }
  async function save(): Promise<AuthoringDraft> {
    if (!projectId || wrongProject) throw new Error('请先在当前工程中保存定义；不会把旧工程输入提交到新工程。');
    if (draft && !dirty) { setMessage(`当前草稿 r${draft.summary.revision} 没有待提交修改。`); return draft; }
    if (!valid()) throw new Error('请先补齐定义标识、版本和表单必填项。');
    if (composition && dirty) {
      if (props.templates.status === 'unavailable') throw new Error(props.templates.reason);
      const issues = shapeIssues(formSchemas.TemplateSubset, composition.subset);
      if (issues.length) throw new Error(issues.map(issue => issue.message).join(' '));
      const result = await saveTemplateComposition(props.templates, compositionInput(projectId, composition, definition, draft), async (createdDraft, creationCommit) => {
        // Deliberately do not replace candidate definition/subset/baseline here.
        setDraft(createdDraft); setRefreshDraftId(createdDraft.summary.draft_id);
        props.onDraftChanged(createdDraft); await props.onCommitted(creationCommit);
      });
      setSnapshot(result.draft); setCompositionDirty(false); await props.onCommitted(result.commit);
      setMessage('组合伴随文件与定义已由实际写入消费者保存；采用前仍须核对新 ID 与角色映射。');
      return result.draft;
    }
    const scope: ApiScope = draft ? { kind: 'definition_draft', data: draft.summary.draft_id } : { kind: 'project' };
    const commands: EditCommand[] = [draft ? { kind: 'put_definition', data: definition } : { kind: 'create_definition_draft', data: definition }];
    const commit = await io.dispatch<EditCommit>(authoringRequest(projectId, scope, 'apply_commands', { base_revision: draft?.summary.revision ?? workspace.project!.revision, commands }));
    if (commit.scope.kind !== 'definition_draft') throw new Error('Host 未返回定义草稿 Scope。');
    setRefreshDraftId(commit.scope.data.id);
    const snapshot = await readDraft(io, projectId, commit.scope.data.id, commit.history, commit.new_revision);
    setSnapshot(snapshot); await props.onCommitted(commit); setMessage(`保存命令形成 r${commit.new_revision}；当前已读草稿 r${snapshot.summary.revision}，检查项按 Host 实际回包显示。`);
    return snapshot;
  }
  async function commitDefinition() {
    const current = dirty || !draft ? await save() : draft;
    const commit = await io.dispatch<EditCommit>(authoringRequest(projectId, { kind: 'definition_draft', data: current.summary.draft_id }, 'apply_commands', { base_revision: current.summary.revision, commands: [{ kind: 'commit_definition_version', data: { version: commitVersion } }] }));
    const snapshot = await readDraft(io, projectId!, current.summary.draft_id, commit.history, commit.new_revision);
    setSnapshot(snapshot); setCommittedRef({ id: current.definition.id, version: commitVersion }); await props.onCommitted(commit);
    setMessage(`精确版本 ${commitVersion} 已提交到本地库；当前已读草稿 r${snapshot.summary.revision}，尚未改变任何实例。`);
  }
  async function history(kind: 'undo' | 'redo') {
    if (!draft || !projectId || wrongProject) return;
    const commit = await io.dispatch<EditCommit>(authoringRequest(projectId, { kind: 'definition_draft', data: draft.summary.draft_id }, 'apply_commands', { base_revision: draft.summary.revision, commands: [{ kind }] }));
    const actual = await readDraft(io, projectId, draft.summary.draft_id, commit.history, commit.new_revision);
    setSnapshot(actual); setComposition(null); setCompositionDirty(false); await props.onCommitted(commit);
    if (actual.definition.payload.category === 'templates') setComposition(compositionFromPreview(await readAuthoringTemplatePreview(() => workspace, actual, null, null, io)));
    setMessage(`${kind === 'undo' ? '撤销' : '重做'}形成新草稿修订 r${commit.new_revision}；旧 Run、库存、认可和文件没有倒退。`);
  }
  async function openModule(reference: ModuleRef) {
    if (!projectId) throw new Error('请先打开工程，再读取精确库版本。');
    const object = await io.dispatch<DomainObject>(authoringRequest(projectId, { kind: projectId ? 'project' : 'workspace' }, 'get_object', { object_id: { kind: 'module', id: reference.id }, source: { kind: 'module_version', data: { module_ref: reference } }, node_index: null }));
    if (object.kind !== 'module') throw new Error('Host 未返回所选精确模块包。');
    setSource(object.data); setDefinition(copy(object.data.definition)); setBaseline(copy(object.data.definition)); setDraft(null); setRefreshDraftId(null); setCommittedRef(reference); setCommitVersion(''); clearSample(); setComposition(null); setCompositionDirty(false); props.onDraftChanged(null);
    setMessage('正在查看精确库版本。编辑后先保存为定义草稿，再明确提交新版本；旧实例与旧方案保持原版本。');
  }
  async function search() { setCatalog(await io.dispatch<ModuleTemplateCandidates>(authoringRequest(projectId, { kind: projectId ? 'project' : 'workspace' }, 'query_module_library', { category_filters: [category], industry: industry.trim() || null, product_family: family.trim() || null, site_envelope: site, source_scope: sources }))); }
  async function readTemplateComposition() {
    if (!workspace.project || wrongProject || (!draft && !committedRef)) throw new Error('先选择实际定义草稿或工程中的精确模板版本。');
    if (draft) {
      const actual = await readDraft(io, workspace.project.id, draft.summary.draft_id);
      const preview = await readAuthoringTemplatePreview(() => workspace, actual, null, null, io);
      setSnapshot(actual); setComposition(compositionFromPreview(preview)); setCompositionDirty(false);
      setMessage(preview.issues.length ? preview.issues.map(issue => issue.message).join('；') : '已从实际 DefinitionDraft Scope 读回组合伴随文件；尚未采用到工程。');
      return;
    }
    const base = workspace.project;
    const object = await io.dispatch<DomainObject>(authoringRequest(base.id, { kind: 'project' }, 'get_object', { object_id: { kind: 'module', id: committedRef!.id }, source: { kind: 'template_preview', data: { module_ref: committedRef!, base_revision: base.revision, choices: null, seed: null } }, node_index: null }));
    if (object.kind !== 'template_preview') throw new Error('Host 未返回模板实际组合与准备预览。');
    setComposition(compositionFromPreview(object.data)); setCompositionDirty(false);
    setDefinition(copy(baseline));
    setMessage(object.data.issues.length ? object.data.issues.map(issue => issue.message).join('；') : '已读取模板伴随文件中的实际组合；尚未采用到工程。');
  }
  async function importPackage() {
    if (!workspace.project || wrongProject) throw new Error('请先打开当前工程。');
    const base = workspace.project; const selected = await io.pickDirectory('import_package'); if (!selected) return;
    const task = await io.dispatch<TaskHandle>(authoringRequest(base.id, { kind: 'project' }, 'import_package', { selected_file_ref: selected, import_policy: 'into_project', base_revision: base.revision }));
    const result = await io.awaitTask(task); if (result.kind !== 'import') throw new Error('Host 未返回实际待导入包。');
    setImporting({ task, value: result.data, base }); setImportRefs([]); setImportAssets([]); setAcceptMissing(false);
  }
  async function acceptImport() {
    if (!importing) return;
    if (wrongProject || workspace.project?.id !== importing.base.id) throw new Error('当前工程已切换，请取消此导入准备后在当前工程重新选择。');
    const available = new Set(importAssetChoices(importing.value, importRefs).map(choice => choice.asset.id));
    const commit = await io.dispatch<EditCommit>(authoringRequest(importing.base.id, { kind: 'project' }, 'apply_commands', { base_revision: importing.base.revision, commands: [{ kind: 'adopt_imported_draft', data: { task_id: importing.task.id, choices: { chosen_policy: 'into_project', objects_to_create: [], module_versions: importRefs, reused_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} }, selected_assets: importAssets.filter(id => available.has(id)), accept_missing_as_draft: acceptMissing, departure: 'stay' } } }] }));
    setImporting(null); await props.onCommitted(commit); setMessage('所选精确定义已明确导入；没有复制正式库存、运行结果或人工认可。');
  }
  async function openAdoption() {
    if (!committedRef || !workspace.project || wrongProject || dirty) return;
    const object = await io.dispatch<DomainObject>(authoringRequest(projectId, { kind: 'project' }, 'get_object', { object_id: { kind: 'module', id: committedRef.id }, source: { kind: 'module_version', data: { module_ref: committedRef } }, node_index: null }));
    if (object.kind !== 'module') throw new Error('Host 未返回已提交的精确版本。'); setAdopting(object.data);
  }
  async function adopt(adoption: ModuleAdoption, base: ProjectState) {
    if (wrongProject || workspace.project?.id !== base.id) throw new Error('当前工程已切换，请取消此采用准备后在当前工程重新选择。');
    setPending('采用与实例升级'); setError('');
    try { const commit = await io.dispatch<EditCommit>(authoringRequest(base.id, { kind: 'project' }, 'apply_commands', { base_revision: base.revision, commands: [{ kind: 'adopt_module', data: adoption }] })); await props.onCommitted(commit); setAdopting(null); setMessage(`采用已形成工程 r${commit.new_revision}；可进入对应消费者继续准备实例或运行。`); }
    catch (failure) { report(failure); throw failure; }
    finally { setPending(''); }
  }
  async function exportContribution() {
    if (!committedRef || !projectId || !workspace.project || dirty || wrongProject || unreadModule) throw new Error('先读取或保存并提交当前工程中的精确本地版本。');
    if (!definition.contribution.license?.trim()) throw new Error('贡献包须填写定义许可；缺项不能当作允许公开。');
    const scope: ApiScope = draft ? { kind: 'definition_draft', data: draft.summary.draft_id } : { kind: 'project' };
    const revision = draft?.summary.revision ?? workspace.project.revision;
    const selected = await io.pickDirectory('export_package'); if (!selected) return;
    const task = await io.dispatch<TaskHandle>(authoringRequest(projectId, scope, 'export_package', { revision, selected_destination_ref: selected, package_scope: { purpose: 'community_module', module_refs: [committedRef], scenarios: [], run_ids: [], artifact_ids: [], include_redistributable_assets: redistribute } }));
    const result = await io.awaitTask(task); if (result.kind !== 'package') throw new Error('Host 未返回实际贡献包导出结果。');
    setMessage(result.data.missing.length ? `包已写出，但仍有缺项：${result.data.missing.map(issue => issue.message).join('；')}` : `贡献包已写入实际选择的位置，共 ${result.data.included_files.length} 个文件；尚未发布。`);
  }
  async function runSample() {
    if (!draft || dirty || wrongProject || !projectId || props.samples.status !== 'available') return;
    const scenario = workspace.scenarios.find(item => item.id === sampleId); if (!scenario) throw new Error('请选择实际已读入的样例方案输入。');
    const context = { ...currentSampleContext }; const generation = ++sampleGeneration.current;
    setSampleResult(null); setSampleRun(null); setSampleReadError(null); setSampleReading(false); setSampleContext(context);
    let task: TaskState;
    try { task = await props.samples.run(draft, scenario); }
    catch (failure) { if (currentSample(context, generation)) throw failure; return; }
    if (!currentSample(context, generation)) return;
    setSampleResult(task);
    if (sampleHasRun(task)) await loadSampleRun(task, context, generation);
  }
  function keyboard(event: KeyboardEvent<HTMLElement>) {
    if (!(event.ctrlKey || event.metaKey)) return;
    if (event.key.toLowerCase() === 's') { event.preventDefault(); if (canWrite && !disabled) void action('保存定义草稿', async () => { await save(); }); return; }
    if ((event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
    const kind = event.key.toLowerCase() === 'y' || (event.key.toLowerCase() === 'z' && event.shiftKey) ? 'redo' : event.key.toLowerCase() === 'z' ? 'undo' : null;
    if (kind && draft?.history?.[kind === 'undo' ? 'can_undo' : 'can_redo'] && !disabled && !dirty && !wrongProject) { event.preventDefault(); void action(kind === 'undo' ? '撤销草稿编辑' : '重做草稿编辑', () => history(kind)); }
  }
  const rights = definition.contribution.assets;
  const restricted = rights.filter(right => !right.redistribution_confirmed || !right.license || !right.source || right.missing_reason);
  useEffect(() => {
    if (!props.initialModule || props.initialDraft) return;
    if (!projectId) { setError('请先打开工程，再读取初始精确模块版本。'); return; }
    let active = true; const reference = props.initialModule;
    setPending('读取初始精确版本');
    void io.dispatch<DomainObject>(authoringRequest(projectId, { kind: projectId ? 'project' : 'workspace' }, 'get_object', { object_id: { kind: 'module', id: reference.id }, source: { kind: 'module_version', data: { module_ref: reference } }, node_index: null })).then(object => {
      if (!active) return; if (object.kind !== 'module') throw new Error('Host 未返回初始精确模块包。');
      setSource(object.data); setDefinition(copy(object.data.definition)); setBaseline(copy(object.data.definition)); setCommittedRef(reference); setCommitVersion('');
    }).catch(failure => { if (active) report(failure); }).finally(() => { if (active) setPending(''); });
    return () => { active = false; };
  }, []);
  return <section className="authoring-workspace" style={{ '--authoring-text-scale': textScale } as CSSProperties} aria-label="九类定义创作" onKeyDown={keyboard}>
    <header className="authoring-topbar"><div><h2>模块定义创作</h2><span>{workspace.project?.title ?? '尚未打开工程'} · {draft ? `定义草稿 r${draft.summary.revision}` : committedRef ? `${unreadModule ? '待读取精确库版本' : '库版本'} ${committedRef.version}` : '本地未保存输入'}{dirty ? ' · 有未保存输入' : ''}</span></div><div className="authoring-actions"><label>表单文字<select aria-label="表单文字大小" value={textScale} onChange={event => setTextScale(Number(event.target.value))}><option value={1}>100%</option><option value={1.25}>125%</option><option value={1.5}>150%</option><option value={2}>200%</option></select></label><button disabled={disabled} onClick={() => navigate('返回共同画布', () => { setPreviewEnabled(false); props.onPreviewChange(null); props.onClose(); })}>返回画布</button><button disabled={disabled || !canWrite} onClick={() => void action('导入类别包', importPackage)}>导入类别包</button><button className="authoring-primary" disabled={disabled || !canWrite} onClick={() => void action('保存定义草稿', async () => { await save(); })}>保存并检查 · Ctrl+S</button></div></header>
    {unreadModule && <p className="authoring-warning">已明确选择精确版本，内容尚未实际读到。读取完成前不会保存占位定义或进入本类消费者。<button disabled={disabled || wrongProject || !projectId} onClick={() => void action('实际读取所选精确版本', () => openModule(committedRef!))}>实际读取所选精确版本</button></p>}
    {wrongProject && <p role="alert" className="authoring-error">工程已经切换。当前输入不会提交到新工程，也不会沿用旧工程撤销历史。</p>}
    {!!error && <section role="alert"><AuthoringFailure value={error} /><button disabled={disabled || !refreshDraftId || wrongProject} onClick={() => navigate('读取共同定义草稿', () => void action('读取共同草稿', async () => { const actual = await readDraft(io, projectId!, refreshDraftId!); setSnapshot(actual); setComposition(null); setCompositionDirty(false); if (actual.definition.payload.category === 'templates') setComposition(compositionFromPreview(await readAuthoringTemplatePreview(() => workspace, actual, null, null, io))); }))}>明确读取共同草稿</button></section>}
    {(pending || message) && <p role="status" className="authoring-status">{pending ? `${pending}…` : message}</p>}
    <div className="authoring-layout"><nav className="authoring-category-nav" aria-label="九类定义类别">{categories.map(item => <button key={item.id} className={!unreadModule && category === item.id ? 'selected' : ''} aria-current={!unreadModule && category === item.id ? 'page' : undefined} disabled={disabled} onClick={() => navigate(`新建${item.title}定义`, () => start(item.id))}>{item.title}</button>)}<hr /><button disabled={disabled} onClick={() => navigate(`新建${categoryInfo.title}定义`, () => start(category))}>新建本类定义</button><button disabled={disabled || !draft?.history?.can_undo || dirty || wrongProject} onClick={() => void action('撤销草稿编辑', () => history('undo'))}>撤销草稿编辑</button><button disabled={disabled || !draft?.history?.can_redo || dirty || wrongProject} onClick={() => void action('重做草稿编辑', () => history('redo'))}>重做草稿编辑</button>{draft?.history?.boundary_reason && <p>{authoringHistoryReason(draft.history.boundary_reason)}</p>}</nav>
      <main className="authoring-editor">{unreadModule ? <p className="authoring-help">精确库内容尚未读取，不将本地占位表单当作该版本内容。</p> : <><div className="authoring-section-heading"><h3>{categoryInfo.heading}</h3><p>{categoryInfo.boundary}</p></div><div className="authoring-tabs" role="tablist" aria-label="定义编辑分区">{([['payload','类别表单'],['parameters','参数与属性'],['rights','来源与许可'],['sample','适用样例']] as const).map(([id,title]) => <button role="tab" aria-selected={tab === id} key={id} onClick={() => setTab(id)}>{title}</button>)}</div><form ref={form} onSubmit={(event: FormEvent) => { event.preventDefault(); if (!disabled && canWrite) void action('保存定义草稿', async () => { await save(); }); }}>
        <fieldset className="authoring-group"><legend>共同身份与版本</legend><div className="authoring-field-grid"><label>中文名称<input required value={definition.title} disabled={disabled} onChange={event => setDefinition({ ...definition, title: event.target.value })} /></label><label>稳定模块标识<input required value={definition.id} disabled={disabled || !!draft} onChange={event => setDefinition({ ...definition, id: event.target.value })} /></label><label>定义中的精确版本<input required value={definition.version} disabled={disabled || !!draft} onChange={event => setDefinition({ ...definition, version: event.target.value })} /></label><SchemaField schema={formSchemas.DefinitionState} value={definition.definition_state} name="definition_state" disabled={disabled} onChange={state => setDefinition({ ...definition, definition_state: state as ModuleDefinition['definition_state'] })} /></div><p className="authoring-help">定义声明、工程实例与当前运行是三层。这里不写正式库存、资源占用或实际结果；标记版本不等于完成公开发布。</p></fieldset>
        {tab === 'payload' && category === 'operations' && <p className="authoring-help">投入配比的“要求质量”是允许领用的状态列表；空列表不会允许所有质量，实际作业会等待质量条件。需要未检验或已放行物料时，请明确添加对应状态。</p>}
        {tab === 'payload' && (category === 'devices' ? <DeviceDefinitionEditor key={draft?.summary.draft_id ?? definition.id} value={definition.payload.category === 'devices' ? definition.payload.data : neverPayload()} disabled={disabled} modules={availableModules} onChange={data => setDefinition({ ...definition, payload: { category: 'devices', data } })} /> : category === 'media' ? <PresentationEditor key={draft?.summary.draft_id ?? definition.id} value={definition.payload.category === 'media' ? definition.payload.data : neverPayload()} disabled={disabled} onChange={data => setDefinition({ ...definition, payload: { category: 'media', data } })} /> : <SchemaField schema={payloadSchema(category)} value={definition.payload.data} name={category} disabled={disabled} onChange={data => setDefinition({ ...definition, payload: { category, data } as ModulePayload })} />)}
        {tab === 'payload' && category === 'templates' && <><button type="button" disabled={disabled || !workspace.project || !!composition} onClick={() => { setComposition({ subset: emptyComposition(), base_revision: workspace.project!.revision, source_scenario: null }); setCompositionDirty(true); if (definition.payload.category === 'templates') setDefinition({ ...definition, payload: { category: 'templates', data: { ...definition.payload.data, project: 'template/project.subset.json' } } }); }}>明确从当前工程创作组合</button><button type="button" disabled={disabled || (!draft && !committedRef) || wrongProject} onClick={() => navigate('读取实际模板组合', () => void action('读取实际模板组合', readTemplateComposition))}>读取精确模板的实际组合（不采用）</button>{props.templates.status === 'unavailable' && <p className="authoring-warning">{props.templates.reason}</p>}{composition && <TemplateCompositionEditor workspace={workspace} value={composition.subset} sourceScenario={composition.source_scenario} issues={composition.issues} baseRevision={composition.base_revision} disabled={disabled || wrongProject} onChange={(subset, source_scenario = composition.source_scenario) => { setComposition({ ...composition, subset, source_scenario }); setCompositionDirty(true); }} />}</>}
        {tab === 'payload' && definition.payload.category === 'resources' && definition.payload.data.kind === 'service_declaration' && <p className="authoring-warning">服务声明可保存；首版执行后置，不能当作已提供的资源执行能力。</p>}
        {tab === 'payload' && definition.payload.category === 'tasks' && ['deferred_maintenance','deferred_construction'].includes(definition.payload.data.kind) && <p className="authoring-warning">维护/施工任务只保存声明；当前没有此类实际执行能力。</p>}
        {tab === 'parameters' && <><SchemaField schema={formSchemas.ParameterDeclaration ? { type: 'array', items: formSchemas.ParameterDeclaration } : neverPayload()} value={definition.parameters} name="parameters" disabled={disabled} onChange={parameters => setDefinition({ ...definition, parameters: parameters as ModuleDefinition['parameters'] })} /><SchemaField schema={{ type: 'array', items: formSchemas.AttributeDeclaration }} value={definition.attribute_schema} name="attribute_schema" disabled={disabled} onChange={attribute_schema => setDefinition({ ...definition, attribute_schema: attribute_schema as ModuleDefinition['attribute_schema'] })} /><p className="authoring-help">每个参数/属性须给出名称、类型和适用单位；来源未知保持“来源未知”，不默认为测量值。修改实例参数不会改写本定义。</p></>}
        {tab === 'rights' && <><SchemaField schema={formSchemas.Contribution} value={definition.contribution} name="contribution" disabled={disabled} onChange={contribution => setDefinition({ ...definition, contribution: contribution as ModuleDefinition['contribution'] })} />{restricted.map(right => <p className="authoring-warning" key={right.asset_ref}>{right.asset_ref}：{right.missing_reason ?? '再分发许可/来源尚未齐备，不作为公开贡献素材。'}</p>)}<p>素材许可与人工专业认可不同。导入声明不会成为实际计算或人工认可。</p></>}
        {tab === 'sample' && <section className="authoring-sample"><h3>定义检查与实际样例</h3><button type="button" disabled={disabled} onClick={() => { const example = fieldExamples[category].definition; setDefinition({ ...definition, payload: copy(example.payload), parameters: copy(example.parameters), attribute_schema: copy(example.attribute_schema) }); setMessage('已填入项目类型示例字段；引用与素材仍须实际导入，未执行样例。'); }}>填入本类字段示例（不导入素材、不执行）</button><p>先保存并读取 Host 检查项。字段示例和预览不是运行结果；只观察实际返回的 Task。</p><label>实际方案输入<select disabled={disabled} value={sampleId} onChange={event => setSampleId(event.target.value)}><option value="">请选择</option>{workspace.scenarios.map(scenario => <option key={scenario.id} value={scenario.id}>{scenario.title} · {scenario.id} · r{scenario.revision}</option>)}</select></label>{props.samples.status === 'unavailable' ? <p className="authoring-warning">{props.samples.reason}</p> : <button type="button" disabled={disabled || !draft || dirty || !sampleId} onClick={() => void action('实际样例检查', runSample)}>运行实际样例</button>}{sampleResult && sampleContext && sameSampleContext(sampleContext, currentSampleContext) && <DefinitionSampleResult task={sampleResult} run={sampleRun} readError={sampleReadError} reading={disabled || sampleReading} currentDraft={draft} onRead={() => void action('读取此冻结样例结果', retrySampleRead)} />}<p className="authoring-help">维护/施工声明保持执行后置；专业模式只按实际提供者能力显示。</p>{category === 'professional' && <ul>{workspace.capabilities.providers.filter(provider => provider.provider_id === definition.id).map(provider => <li key={provider.provider_id}>{provider.provider_id}：{optionLabel(provider.producer_status)} · {provider.available_modes.map(optionLabel).join('、') || '无实际模式'}</li>)}{!workspace.capabilities.providers.some(provider => provider.provider_id === definition.id) && <li>当前 Host 未列出本提供者的实际可用模式。</li>}</ul>}</section>}
      </form></>}</main>
      <aside className="authoring-summary"><section><h3>版本与采用</h3><dl><dt>库中来源</dt><dd>{definitionSourceText(draft, source?.definition ?? null, committedRef)}</dd><dt>当前精确库引用</dt><dd>{committedRef ? `${committedRef.id} @ ${committedRef.version}` : '尚未明确提交或选择'}</dd><dt>定义草稿</dt><dd>{draft ? `${draft.summary.draft_id} · r${draft.summary.revision}` : '尚未保存到 Host'}</dd><dt>工程实例</dt><dd>仅在明确确认采用/升级后改变</dd></dl><label>另存为精确本地版本<input value={commitVersion} disabled={disabled} onChange={event => setCommitVersion(event.target.value)} /></label><button disabled={disabled || !canWrite || !commitVersion.trim()} onClick={() => void action('提交本地版本', commitDefinition)}>提交精确版本到本地库</button><button disabled={disabled || !committedRef || dirty || !canWrite} onClick={() => void action('准备采用与升级', openAdoption)}>选择实例并核对采用</button><button disabled={disabled || !committedRef || dirty || wrongProject || unreadModule} onClick={() => props.onConsumerRequested(category, committedRef!)}>打开使用页面</button>{source && <VersionChanges before={source.definition} after={definition} />}</section>
        <section><h3>共同画布预览</h3><button disabled={disabled || !presentationOf(definition)} onClick={() => { if (form.current?.reportValidity()) setPreviewEnabled(true); }}>在共同画布核对表现</button>{previewEnabled && <><p>已开启同一表单预览；实际来源的素材经共同媒体渲染器读取，表单编辑会更新画布。</p><button onClick={() => { setPreviewEnabled(false); props.onPreviewChange(null); }}>停止草稿预览</button></>}<p>预览不改变行为、实际库存和当前 Run。缺失字节和未保存的新素材引用按实际读取结果显示，不补画替代。</p></section>
        <section><h3>检查项 · {issues.length}</h3>{issues.map((issue, index) => <p className="authoring-warning" key={index}><strong>{issue.field_path ? issue.field_path.split('/').filter(Boolean).map(fieldLabel).join(' / ') : '定义'}</strong><span>{authoringIssueMessage(issue)}</span></p>)}{issues.length === 0 && <p>{draft ? '当前已读检查项为空；不等于九类运行/渲染全部通过。' : '尚未读取 Host 检查结果。'}</p>}</section>
        <section><h3>贡献包</h3><label className="authoring-check"><input type="checkbox" disabled={disabled} checked={redistribute} onChange={event => setRedistribute(event.target.checked)} />仅附带已确认允许再分发的素材（受限条目不加入）</label><button disabled={disabled || !committedRef || dirty || wrongProject || unreadModule} onClick={() => void action('导出贡献包', exportContribution)}>选择 ZIP 文件并导出贡献包</button><p>只导出精确版本和许可允许的资产，不自动上传或发布。已落盘文件不由草稿撤销回滚。</p></section>
      </aside>
    </div>
    <section className="authoring-library"><div className="authoring-field-heading"><h3>实际模块与模板目录</h3><button disabled={disabled || !sources.length || unreadModule} onClick={() => void action('查询实际目录', search)}>查询当前类别</button></div><div className="authoring-field-grid"><label>行业<input value={industry} onChange={event => setIndustry(event.target.value)} /></label><label>产品族<input value={family} onChange={event => setFamily(event.target.value)} /></label><fieldset><legend>实际来源</legend>{(['bundled','project_packages','user_granted_libraries'] as const).map(scope => <label className="authoring-check" key={scope}><input type="checkbox" checked={sources.includes(scope)} onChange={event => setSources(previous => event.target.checked ? [...previous, scope] : previous.filter(item => item !== scope))} />{scope === 'bundled' ? '内置' : scope === 'project_packages' ? '工程包' : '已授权目录'}</label>)}</fieldset></div>{formSchemas.SiteEnvelope && <details><summary>明确场地适用条件</summary><SchemaField schema={{ anyOf: [formSchemas.SiteEnvelope, { type: 'null' }] }} value={site} name="场地边界" onChange={value => setSite(value as SiteEnvelope | null)} disabled={disabled} /></details>}{catalog?.missing.map((issue, index) => <p className="authoring-warning" key={index}>{issue.message}</p>)}<div className="authoring-candidate-list">{catalog?.candidates.map(candidate => <article key={refKey(candidate.module_ref)}><div><strong>{candidate.title}</strong><span>{candidate.module_ref.id} @ {candidate.module_ref.version}</span><small>{candidate.applicable ? '目录未报告适用阻断；仍需 Host 采用检查' : '存在待核条件'}</small>{candidate.sources.map(source => <small key={source.id}>{source.title} · {source.locator ?? '未提供定位'}</small>)}{candidate.missing.map((issue, index) => <p key={index}>{issue.message}</p>)}</div><button disabled={disabled} onClick={() => navigate('打开精确模块版本', () => void action('读取精确模块', () => openModule(candidate.module_ref)))}>查看与编辑副本</button></article>)}{catalog?.candidates.length === 0 && <p>当前所选来源和条件没有返回候选；未适配不当作成功。</p>}</div></section>
    {adopting && workspace.project && <AdoptionPreparation workspace={workspace} module={adopting} busy={disabled} onCancel={() => setAdopting(null)} onApply={adopt} />}
    {importing && <div className="authoring-dialog-backdrop"><section role="dialog" aria-modal="true" aria-label="明确导入类别包" className="authoring-dialog"><h2>实际待导入定义 · 尚未采用</h2><p>只选择精确模块版本，不复制包中的正式库存、当前占用、运行结果或认可。</p>{importing.value.proposed_definitions.map(ref => <label className="authoring-check" key={refKey(ref)}><input type="checkbox" disabled={disabled} checked={importRefs.some(item => refKey(item) === refKey(ref))} onChange={event => setImportRefs(previous => event.target.checked ? [...previous, ref] : previous.filter(item => refKey(item) !== refKey(ref)))} />{ref.id} @ {ref.version}</label>)}<fieldset className="authoring-group"><legend>所选模块的实际素材</legend>{importAssetChoices(importing.value, importRefs).map(({ asset, module, rights }) => <label className="authoring-check" key={`${module.id}@${module.version}/${asset.id}`}><input type="checkbox" disabled={disabled} checked={importAssets.includes(asset.id)} onChange={event => setImportAssets(previous => event.target.checked ? [...previous, asset.id] : previous.filter(id => id !== asset.id))} /><span>{asset.id} · {asset.path}<small>来源：{rights?.source ?? '未声明'}；许可：{rights?.license ?? '未声明'}；{rights?.redistribution_confirmed ? '已声明可再分发' : '未获公开再分发确认'}</small></span></label>)}{importAssetChoices(importing.value, importRefs).length === 0 && <p>当前实际回包未列出所选定义的素材；不会补造素材路径。</p>}</fieldset>{[...importing.value.missing,...importing.value.conflicts].map((issue,index) => <p className="authoring-warning" key={index}>{issue.message}</p>)}{importing.value.required_human_choices.map(choice => <p key={choice}>{choice}</p>)}<label className="authoring-check"><input type="checkbox" checked={acceptMissing} disabled={disabled} onChange={event => setAcceptMissing(event.target.checked)} />明确接受缺项，仅作草稿</label><div className="authoring-actions"><button disabled={disabled} onClick={() => setImporting(null)}>取消，不采用</button><button disabled={disabled || !importRefs.length} onClick={() => void action('明确采用导入定义', acceptImport)}>确认所选精确版本</button></div></section></div>}
    {transition && <div className="authoring-dialog-backdrop"><section role="dialog" aria-modal="true" aria-label="未保存输入" className="authoring-dialog"><h2>当前输入尚未保存</h2><p>准备{transition.title}。保存经共同 Commands 形成草稿修订；丢弃只影响当前未提交输入。</p><div className="authoring-actions"><button disabled={disabled} onClick={() => setTransition(null)}>继续编辑</button><button disabled={disabled} onClick={() => { const next = transition.action; setTransition(null); next(); }}>放弃未保存输入后继续</button><button disabled={disabled || !canWrite} onClick={() => void action('保存后继续', async () => { await save(); const next = transition.action; setTransition(null); next(); })}>保存后继续</button></div></section></div>}
  </section>;
}
function neverPayload(): never { throw new Error('已解析的定义类别与载荷不一致。'); }
