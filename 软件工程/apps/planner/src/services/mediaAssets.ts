import { invoke } from '@tauri-apps/api/core';
import type { ApiError } from '../../../../packages/commands/generated/typescript/ApiError';
import type { ApiScope } from '../../../../packages/commands/generated/typescript/ApiScope';
import type { AssetRef } from '../../../../packages/commands/generated/typescript/AssetRef';
import type { ObjectSource } from '../../../../packages/commands/generated/typescript/ObjectSource';
import type { ProjectId } from '../../../../packages/commands/generated/typescript/ProjectId';
import type { Revision } from '../../../../packages/commands/generated/typescript/Revision';
import { HostFailure } from './projectHost';

export interface MediaAssetReadRequest {
  project_id: ProjectId;
  scope: Exclude<ApiScope, { kind: 'workspace' }>;
  scope_revision: Revision;
  source: Extract<ObjectSource, { kind: 'module_version' | 'run_module_version' }>;
  asset: AssetRef;
}

export async function readMediaAsset(request: MediaAssetReadRequest): Promise<ArrayBuffer> {
  try { return await invoke<ArrayBuffer>('project_read_media_asset', { request }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}
