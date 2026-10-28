import React from 'react';
import { createRoot } from 'react-dom/client';
import { mockIPC } from '@tauri-apps/api/mocks';
import { AgentWorkspace, type AgentEditorState } from '../../../../apps/planner/src/features/agent';
import type { WorkspaceSnapshot } from '../../../../apps/planner/src/services/projectHost';

const operationNames = ['describe_capabilities','get_context','get_object','query_module_library','get_run','get_task','apply_commands','check_factory_draft','render_view','run_scenario','save_project','export_evaluation'];
const project = { id: 'UI-P1', title: '受控UI工程', revision: 3 };
const workspace = { project, context: { host: { session_id: 'UI-SESSION' }, tasks: [{ id: 'UI-TASK', state: 'completed' }] },
  capabilities: { implemented_operations: operationNames.map(operation => ({ operation, allowed_scopes: [{ kind: 'project' }] })) } } as WorkspaceSnapshot;
const state = {
  calls: [] as { command: string; args: unknown }[], states: [] as AgentEditorState[], closed: 0, refreshes: 0,
  failure: '', delayedRefresh: false, resolveRefresh: null as null | (() => void),
  revision: 3,
  info: { endpoint: '\\\\.\\pipe\\UI-TEST', host_instance_id: 'UI-HOST', connections: [{ session_id: 'PEER-1', pid: 1234, os_session: 2, authorized: false, revoked: false }] },
};
mockIPC((command, args) => {
  state.calls.push({ command, args });
  if (command === 'agent_connection_info') return structuredClone(state.info);
  if (command === 'project_api') return { api_version: '0.1', request_id: (args as any).request.request_id,
    context: null, outcome: { status: 'ok', data: { ...workspace.context, project: { ...project, revision: state.revision }, run_summaries: [] } } };
  if (state.failure) throw new Error(state.failure);
  if (command === 'agent_authorize_connection' || command === 'agent_update_connection_authority') { state.info.connections[0].authorized = true; return; }
  if (command === 'agent_revoke_connection') { state.info.connections[0].revoked = true; return; }
  if (command === 'agent_select_directory') return { id: 'TOKEN-1', purpose: (args as { purpose: string }).purpose };
  if (command === 'agent_grant_task_access') return;
  throw new Error('Unexpected controlled boundary invocation: ' + command);
});
const root = createRoot(document.getElementById('root')!);
const render = (session = 'UI-SESSION') => root.render(<AgentWorkspace key={session} workspace={{ ...workspace, context: { ...workspace.context, host: { ...workspace.context.host, session_id: session } } }}
  onRefresh={async () => { state.refreshes++; if (state.delayedRefresh) await new Promise<void>(resolve => { state.resolveRefresh = resolve; }); }}
  onClose={() => { state.closed++; }} onEditorStateChanged={value => state.states.push(value)} />);
render();
Object.assign(window, { __agentUi: { state, render, unmount: () => root.unmount() } });
