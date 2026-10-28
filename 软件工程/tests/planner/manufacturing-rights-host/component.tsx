// Test transport only. All business operations use the actual shared SDK/ProjectHost.
import React, { useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { PackagingWorkspace } from '../../../apps/planner/src/features/packaging/PackagingWorkspace';
import type { PackagingScope } from '../../../apps/planner/src/features/packaging/types';
import * as host from '../../../apps/planner/src/services/projectHost';
import { createModuleAuthoringIO } from '../../../apps/planner/src/services/moduleAuthoring';
import { startManufacturingPackageExport } from '../../../apps/planner/src/services/manufacturingExport';
import { listStoredPackaging, readStoredPackaging } from '../../../apps/planner/src/services/storedPackaging';
import type { ApiError } from '../../../packages/commands/generated/typescript/ApiError';
import type { ProjectContext } from '../../../packages/commands/generated/typescript/ProjectContext';
import type { PackageStructureJob } from '../../../packages/commands/generated/typescript/PackageStructureJob';
import '../../../apps/planner/src/styles/workbench.css';

async function transport<T>(request: object): Promise<T> {
  const response = await fetch('/__manufacturing_core', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(request) });
  const result = await response.json() as { ok: boolean; value: T; error: ApiError };
  if (!result.ok) throw result.error;
  return result.value;
}
const records: {command: string; args: object | undefined}[] = [];
Object.assign(window, { __TAURI_INTERNALS__: {
  async invoke<T>(command: string, args?: object): Promise<T> { records.push({ command, args }); return transport<T>({ kind: 'invoke', command, args }); },
} });
const ready = await fetch('/__manufacturing_core').then(response => response.json()) as {job: PackageStructureJob};
const initial = await host.readWorkspace();
let inspect = () => ({ workspace: initial, scope: {kind:'project'} as PackagingScope, open: true });
let refreshCurrent: () => Promise<void>;
let switchScope: (scope: PackagingScope) => Promise<void>;
let reopen: () => void;
Object.assign(window, { __manufacturing: {
  read: () => inspect(), records,
  refresh: () => refreshCurrent(),
  remoteEdit: () => transport({ kind:'remote_edit' }),
  changeScope: (scope: PackagingScope) => switchScope(scope),
  createDraft: () => transport<PackagingScope>({kind:'create_draft'}),
  reopen: () => reopen(),
} });
function App() {
  const [workspace, setWorkspace] = useState(initial);
  const current = useRef(workspace); current.current = workspace;
  const [scope, setScope] = useState<PackagingScope>({ kind: 'project' });
  const [scopedContext, setScopedContext] = useState(workspace.context);
  const [open, setOpen] = useState(true);
  const io = createModuleAuthoringIO(() => current.current);
  async function readContext(s: PackagingScope) {
    return host.dispatch<ProjectContext>({api_version:'0.1',request_id:crypto.randomUUID(),project_id:current.current.project!.id,scope:s,operation:'get_context',params:{selection:[]}});
  }
  async function refresh() {
    const value = await host.readWorkspace(); current.current = value; setWorkspace(value);
    setScopedContext(await readContext(scope));
  }
  refreshCurrent = refresh;
  switchScope = async next => { const value = await host.readWorkspace(); current.current=value; setWorkspace(value); setScopedContext(await readContext(next)); setScope(next); setOpen(true); };
  reopen = () => setOpen(true);
  inspect = () => ({workspace,scope,open});
  const project = workspace.project!;
  const revision = scope.kind==='project' ? project.revision : scopedContext.definition!.revision;
  async function binding() {
    const actual=await readContext(scope);
    return {project_id:project.id,scope,scope_revision:scope.kind==='project'?actual.project!.revision:actual.definition!.revision};
  }
  return <main><p>实际组件＋真实 Host · 私有测试传输 · 非 Native／真人验收</p>{open ? <PackagingWorkspace key={JSON.stringify([workspace.context.host.session_id,project.id,scope])}
    workspace={{...workspace,context:scopedContext}} scope={scope} scopeRevision={revision} io={io} initialJob={ready.job}
    manufacturingExport={{status:'available',start:startManufacturingPackageExport}}
    storedArtifacts={{status:'available',list:async()=>listStoredPackaging(await binding()),read:async artifact_id=>({...await readStoredPackaging({...await binding(),artifact_id}),inputReadIssue:null})}}
    onCommitted={refresh} onRefresh={refresh} onClose={()=>setOpen(false)} /> : <p>专业工作区已关闭；源工程仍保留。</p>}</main>;
}
createRoot(document.getElementById('root')!).render(<App />);
