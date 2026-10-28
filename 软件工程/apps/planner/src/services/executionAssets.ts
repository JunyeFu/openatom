import { invoke } from '@tauri-apps/api/core';
import type { ApiError } from '../../../../packages/commands/generated/typescript/ApiError';
import type { ExecutionMediaAssetReadRequest } from '../../../../packages/commands/generated/typescript/ExecutionMediaAssetReadRequest';
import { HostFailure } from './projectHost';
import { CloudProductionError, type CloudContext } from './cloudProduction';
import type { ExecutionAssetPort } from './executionPresentation';

/** The port captures the actual current project revision; Native establishes UI authority. */
export function createExecutionAssetPort(context: CloudContext): ExecutionAssetPort {
  return async selected => {
    if (selected.identity.project_id !== context.projectId) {
      throw new CloudProductionError('source_mismatch', '执行素材不属于当前工程。');
    }
    const request: ExecutionMediaAssetReadRequest = {
      project_id: context.projectId,
      scope_revision: context.projectRevision,
      ...selected,
    };
    try { return await invoke<ArrayBuffer>('project_read_execution_media_asset', { request }); }
    catch (error) {
      if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
      throw error;
    }
  };
}
