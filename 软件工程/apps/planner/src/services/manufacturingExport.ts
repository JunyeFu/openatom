import { invoke } from '@tauri-apps/api/core';
import { HostFailure } from './projectHost';
import type { ApiError } from '../../../../packages/commands/generated/typescript/ApiError';
import type { ApiScope } from '../../../../packages/commands/generated/typescript/ApiScope';
import type { ArtifactId } from '../../../../packages/commands/generated/typescript/ArtifactId';
import type { FileRef } from '../../../../packages/commands/generated/typescript/FileRef';
import type { ProjectId } from '../../../../packages/commands/generated/typescript/ProjectId';
import type { Revision } from '../../../../packages/commands/generated/typescript/Revision';
import type { ExportPackageParams } from '../../../../packages/commands/generated/typescript/ExportPackageParams';
import type { TaskHandle } from '../../../../packages/commands/generated/typescript/TaskHandle';

export interface ManufacturingFileDeclaration {
  artifact_id: ArtifactId;
  file: FileRef;
  source: string;
  license: string;
}
export interface ManufacturingPackageExportRequest {
  project_id: ProjectId;
  scope: Exclude<ApiScope, { kind: 'workspace' }>;
  scope_revision: Revision;
  params: ExportPackageParams;
  declarations: ManufacturingFileDeclaration[];
  confirm_redistribution: true;
}

/** Root mounts this private Main-window port only after the actual Native producer is connected. */
export async function startManufacturingPackageExport(request: ManufacturingPackageExportRequest): Promise<TaskHandle> {
  try { return await invoke<TaskHandle>('project_export_manufacturing_package', { request }); }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}
