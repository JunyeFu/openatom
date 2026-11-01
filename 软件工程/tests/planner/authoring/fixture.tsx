import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ModulePresentationPreview } from '../../../apps/planner/src/scene/ModulePresentationPreview';
import type { Presentation } from '../../../packages/commands/generated/typescript/Presentation';
import { ModuleAuthoringWorkspace } from '../../../apps/planner/src/features/modules';
import { AdoptionPreparation } from '../../../apps/planner/src/features/modules/AdoptionPreparation';
import { fieldExamples } from '../../../apps/planner/src/features/modules/fieldExamples';
import { categories, copy, refKey } from '../../../apps/planner/src/features/modules/model';
import projectFixture from '../../../tools/type-contract-proof/fixtures/project.json';
import scenarioFixture from '../../../tools/type-contract-proof/fixtures/scenario.json';
import type { Category } from '../../../apps/planner/src/features/modules/model';
import type { WorkspaceSnapshot } from '../../../apps/planner/src/services/projectHost';
import type { AuthoringDraft } from '../../../apps/planner/src/features/modules';

// Offline finite UI consumer only. All native/Host IO fails explicitly.
const project = copy(projectFixture.data);
project.title = '九类创作 · 有限页面检查';
const workspace: WorkspaceSnapshot = {
  project, modules: new Map(Object.values(fieldExamples).map(module => [refKey(module.definition), module])), moduleIssues: [], scenarios: [scenarioFixture.data], scenarioIssues: [],
  context: { host: { session_id: 'offline-ui-no-host', selected_scope: null }, project: null, selected_objects: [], view_state: null, definition: null, tasks: [], authorized_operations: [], issues: [] },
  capabilities: { api_version: '0.1', implemented_operations: [{ operation: 'apply_commands', input_schema_ref: '', output_schema_ref: '', allowed_scopes: [], available_controls: [] }], available_command_kinds: [], providers: [], category_ids: categories.map(item => item.id), media_kinds: [], native_primitives: [], max_ipc_frame_bytes: 0, limitations: [] },
};
function Fixture() {
  const [category, setCategory] = useState<Category>('devices'); const [mode, setMode] = useState('authoring');
  const [events, setEvents] = useState<string[]>([]); const [key, setKey] = useState(0);
  const [switchProject, setSwitchProject] = useState(false);
  const [preview, setPreview] = useState<Presentation | null>(null);
  const current = { ...workspace, project: switchProject ? { ...project, id: 'other-project' } : project };
  const initial: AuthoringDraft = { definition: copy(fieldExamples[category].definition), summary: { draft_id: `offline-${category}`, revision: 7, definition_ref: { id: fieldExamples[category].definition.id, version: fieldExamples[category].definition.version }, issues: [] }, history: { can_undo: true, can_redo: true, boundary_reason: null } };
  function log(message: string) { setEvents(previous => [...previous, message]); }
  function unavailable(label: string): never { log(label); throw new Error('离线页面检查没有 Native/Host；未执行业务写入。'); }
  return <><div className="fixture-controls"><strong>有限页面检查 · 无 Native/Host</strong><label>字段例<select aria-label="有限字段例" value={category} onChange={event => { setCategory(event.target.value as Category); setKey(key + 1); }}>{categories.map(item => <option key={item.id} value={item.id}>{item.title}</option>)}</select></label><label>检查入口<select aria-label="有限检查入口" value={mode} onChange={event => { setMode(event.target.value); setKey(key + 1); }}><option value="authoring">定义编辑</option><option value="adoption">采用确认</option></select></label><button onClick={() => setSwitchProject(!switchProject)}>切换有限工程</button><button onClick={() => { setEvents([]); setKey(key + 1); }}>重置页面检查</button></div><output aria-label="有限测试事件">{events.join('\n') || '尚无事件'}</output><div className={`fixture-content${preview ? ' with-preview' : ''}`}>
    {mode === 'authoring' ? <ModuleAuthoringWorkspace key={key} workspace={current} busy={false} initialDraft={initial} io={{ dispatch: async request => unavailable(JSON.stringify(request)), awaitTask: async () => unavailable('awaitTask'), pickDirectory: async purpose => unavailable(`picker:${purpose}`) }} samples={{ status: 'unavailable', reason: '此页仅做有限浏览器检查，没有真实样例执行者。' }} templates={{ status: 'unavailable', reason: '此页没有实际组合文件写入消费者；可以编辑输入，不能声称已保存。' }} onDraftChanged={value => log(`draft:${value?.summary.draft_id ?? 'none'}`)} onCommitted={() => unavailable('onCommitted')} onPreviewChange={value => { setPreview(value); log(`preview:${value?.kind ?? 'none'}`); }} onConsumerRequested={value => log(`consumer:${value}`)} onClose={() => log('close')} /> : <div className="authoring-workspace"><AdoptionPreparation key={key} workspace={current} module={fieldExamples[category]} busy={false} onCancel={() => log('cancel:zero-command')} onApply={async (adoption, base) => { log(JSON.stringify({ proposal_only: adoption, base_revision: base.revision })); throw new Error('已记录有限 UI 提案；没有 Host，未执行采用。'); }} /></div>}
    {preview && <ModulePresentationPreview presentation={preview} title="同一草稿 · 有限原生预览" />}
  </div></>;
}
createRoot(document.getElementById('root')!).render(<Fixture />);
