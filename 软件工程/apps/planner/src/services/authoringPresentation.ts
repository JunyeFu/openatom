import type { WorkspaceSnapshot } from './projectHost';
import type { ProcessReadContext, ProcessMediaBinding } from './presentationHost';
import type { AuthoringDraft } from '../features/modules/types';
import type { ModulePackage } from '../../../../packages/commands/generated/typescript/ModulePackage';
import type { AssetRights } from '../../../../packages/commands/generated/typescript/AssetRights';
import type { Presentation } from '../../../../packages/commands/generated/typescript/Presentation';

export interface AuthoringPresentationSource {
  context: ProcessReadContext;
  binding: ProcessMediaBinding;
  rights: AssetRights[];
}
export interface AuthoringPresentationPreviewValue {
  presentation: Presentation;
  source: AuthoringPresentationSource | null;
}

/** Local edits keep the actual saved source identity; new input is not a Host draft. */
export function authoringPresentationSource(workspace: WorkspaceSnapshot, draft: AuthoringDraft | null,
  module: ModulePackage | null, rights: AssetRights[]): AuthoringPresentationSource | null {
  if (!workspace.project || (!draft && !module)) return null;
  const module_ref = draft ? draft.summary.definition_ref : { id: module!.definition.id, version: module!.definition.version };
  const context: ProcessReadContext = {
    session_id: workspace.context.host.session_id, project_id: workspace.project.id,
    scope: draft ? { kind: 'definition_draft', data: draft.summary.draft_id } : { kind: 'project' },
    scope_revision: draft ? draft.summary.revision : workspace.project.revision,
    selection: { kind: 'definition', module_ref },
  };
  return { context, rights, binding: { session_id: context.session_id, request: {
    project_id: context.project_id, scope: context.scope, scope_revision: context.scope_revision,
    source: { kind: 'module_version', data: { module_ref } },
  } } };
}
