import { TemplateCompositionSaveFailure } from '../../services/moduleAuthoring';
import type { AuthoringTemplates, AuthoringDraft, TemplateCompositionInput } from './types';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';

/** Creation can commit before the separate composition command fails. */
export async function saveTemplateComposition(templates: Extract<AuthoringTemplates, { status: 'available' }>, input: TemplateCompositionInput, onCreated: (draft: AuthoringDraft, commit: EditCommit) => void | Promise<void>) {
  try { return await templates.save(input); }
  catch (failure) {
    if (failure instanceof TemplateCompositionSaveFailure) {
      try { await onCreated(failure.createdDraft, failure.creationCommit); }
      catch (refreshFailure) { throw new AggregateError([failure, refreshFailure], `${failure.message}；共同上下文刷新也失败：${refreshFailure instanceof Error ? refreshFailure.message : String(refreshFailure)}`); }
    }
    throw failure;
  }
}
