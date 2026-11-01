import { invoke } from '@tauri-apps/api/core';
import type { FilePurpose } from '../../../../packages/commands/generated/typescript/FilePurpose';
import type { ObjectKind } from '../../../../packages/commands/generated/typescript/ObjectKind';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { ModuleRef } from '../../../../packages/commands/generated/typescript/ModuleRef';
import type { OperationName } from '../../../../packages/commands/generated/typescript/OperationName';
import type { SelectedFileRef } from '../../../../packages/commands/generated/typescript/SelectedFileRef';
import type { TaskScope } from '../../../../packages/commands/generated/typescript/TaskScope';

/** Private main-window adapter DTOs, matching agent_pipe.rs; not public API grants. */
export interface AgentFieldChoice { pointer: string; element_id: string | null }
export type AgentBranchChoice = 'adopt_artifact' | 'create_project' | 'put_object' | 'remove_object'
  | 'set_project_settings' | 'create_definition_draft' | 'put_definition' | 'set_declared_parameter'
  | 'adopt_module' | 'instantiate_template' | 'put_packaging_job' | 'record_artifact_acknowledgment'
  | 'adopt_imported_draft' | 'commit_definition_version' | 'undo' | 'redo';
export interface AgentGrantChoice {
  scope: TaskScope;
  branches: AgentBranchChoice[];
  objects: { target: ObjectRef; fields: AgentFieldChoice[]; can_remove: boolean }[];
  creates: { kind: ObjectKind; max_new_objects: number; fields: AgentFieldChoice[] }[];
  source_versions: ModuleRef[];
  reusable_resources: string[];
}
export interface AgentAuthorityChoice {
  project_id: string | null;
  operations: OperationName[];
  definition_drafts: string[];
  allow_view_control: boolean;
  edit: AgentGrantChoice[];
}
export interface AgentConnectionInfo {
  endpoint: string;
  host_instance_id: string;
  connections: { session_id: string; pid: number; os_session: number; authorized: boolean; revoked: boolean }[];
}

export const readAgentConnections = () => invoke<AgentConnectionInfo>('agent_connection_info');
export const authorizeAgent = (sessionId: string, choice: AgentAuthorityChoice) =>
  invoke<void>('agent_authorize_connection', { sessionId, choice });
export const updateAgentAuthority = (sessionId: string, choice: AgentAuthorityChoice) =>
  invoke<void>('agent_update_connection_authority', { sessionId, choice });
export const revokeAgent = (sessionId: string) => invoke<void>('agent_revoke_connection', { sessionId });
export const grantAgentTask = (sessionId: string, taskId: string, allowControls: boolean) =>
  invoke<void>('agent_grant_task_access', { sessionId, taskId, allowControls });
export const selectAgentDirectory = (sessionId: string, purpose: FilePurpose) =>
  invoke<SelectedFileRef>('agent_select_directory', { sessionId, purpose });

/** Copyable configuration only. Never writes a client/global config or launches a shell. */
export function agentCodexConfiguration(bridgePath: string, info: AgentConnectionInfo): string {
  return ['[mcp_servers.superfactory]', `command = ${JSON.stringify(bridgePath)}`,
    `args = ${JSON.stringify(['--host-pipe', info.endpoint, '--host-instance-id', info.host_instance_id])}`,
    'startup_timeout_sec = 120'].join('\n');
}
