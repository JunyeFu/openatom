import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { AuthoringIO } from '../modules/types';
import type { ApiScope } from '../../../../../packages/commands/generated/typescript/ApiScope';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { PackageStructureJob } from '../../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { PackagingResult } from '../../../../../packages/commands/generated/typescript/PackagingResult';
import type { ArtifactRecord } from '../../../../../packages/commands/generated/typescript/ArtifactRecord';
import type { ArtifactId } from '../../../../../packages/commands/generated/typescript/ArtifactId';
import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { ManufacturingPackageExportRequest } from '../../services/manufacturingExport';
import type {FileRef} from '../../../../../packages/commands/generated/typescript/FileRef';

export type PackagingScope = Exclude<ApiScope, { kind: 'workspace' }>;
export interface PackagingEvidence {
  result: PackagingResult;
  artifact: ArtifactRecord;
  /** Only a matching actual get_object response; null means precise geometry is unavailable. */
  input: PackageStructureJob | null;
  inputReadIssue: string | null;
}
export type ManufacturingExportPort =
  | { status: 'unavailable'; reason: string }
  | { status: 'available'; start(request: ManufacturingPackageExportRequest): Promise<TaskHandle> };
export interface PackagingWorkspaceProps {
  workspace: WorkspaceSnapshot;
  scope: PackagingScope;
  scopeRevision: number;
  io: AuthoringIO;
  /** A real previously read Host input. Declaration templates are selected separately. */
  initialJob?: PackageStructureJob;
  /** Optional actual evidence already read by Root; fixtures must not be supplied to the product. */
  initialEvidence?: PackagingEvidence;
  storedArtifacts?:
    | { status: 'unavailable'; reason: string }
    | { status: 'available'; list(): Promise<ArtifactId[]>; read(artifactId: string): Promise<PackagingEvidence> };
  manufacturingExport?: ManufacturingExportPort;
  readManufacturingFile?:(evidence:PackagingEvidence,file:FileRef)=>Promise<ArrayBuffer>;
  onCommitted(commit: EditCommit): void | Promise<void>;
  /** Root reloads the same actual workspace after Task completion or binding. */
  onRefresh(): void | Promise<void>;
  onClose(): void;
  onEditorStateChanged?(state: { identity: string; dirty: boolean; busy: boolean }): void;
}
