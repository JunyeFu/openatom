import { invoke } from '@tauri-apps/api/core';
import type { ApiError } from '../../../../packages/commands/generated/typescript/ApiError';
import type { ArtifactId } from '../../../../packages/commands/generated/typescript/ArtifactId';
import type { ArtifactRecord } from '../../../../packages/commands/generated/typescript/ArtifactRecord';
import type { ApiScope } from '../../../../packages/commands/generated/typescript/ApiScope';
import type { PackageStructureJob } from '../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { PackagingResult } from '../../../../packages/commands/generated/typescript/PackagingResult';
import type { ProjectId } from '../../../../packages/commands/generated/typescript/ProjectId';
import type { Revision } from '../../../../packages/commands/generated/typescript/Revision';
import type { FileRef } from '../../../../packages/commands/generated/typescript/FileRef';
import { HostFailure } from './projectHost';

export interface StoredPackagingReadRequest {
  project_id: ProjectId;
  scope: Exclude<ApiScope, { kind: 'workspace' }>;
  scope_revision: Revision;
  artifact_id: ArtifactId;
}

export interface StoredPackagingEvidence {
  artifact: ArtifactRecord;
  result: PackagingResult;
  input: PackageStructureJob;
}

export async function listStoredPackaging(request: Omit<StoredPackagingReadRequest, 'artifact_id'>): Promise<ArtifactId[]> {
  try { return await invoke<ArtifactId[]>('project_list_stored_packaging', { request }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}

export async function readStoredPackaging(request: StoredPackagingReadRequest): Promise<StoredPackagingEvidence> {
  try { return await invoke<StoredPackagingEvidence>('project_read_stored_packaging', { request }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}

export async function readStoredManufacturingFile(request: StoredPackagingReadRequest & { file: FileRef }): Promise<ArrayBuffer> {
  try { return await invoke<ArrayBuffer>('project_read_stored_manufacturing_file', { request }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}
