import type { RunResult } from '../../commands/generated/typescript/RunResult';
import type { RunObjectsPage } from '../../commands/generated/typescript/RunObjectsPage';
import type { OperationDefinition } from '../../commands/generated/typescript/OperationDefinition';
import type { JobState } from '../../commands/generated/typescript/JobState';
import type { ResourceClaim } from '../../commands/generated/typescript/ResourceClaim';
import type { RuntimeItem } from '../../commands/generated/typescript/RuntimeItem';
import type { WaitInterval } from '../../commands/generated/typescript/WaitInterval';
import type { AssetRef } from '../../commands/generated/typescript/AssetRef';
import type { PortRef } from '../../commands/generated/typescript/PortRef';
import type { ModuleRef } from '../../commands/generated/typescript/ModuleRef';
import type { Quantity } from '../../commands/generated/typescript/Quantity';
import type { SubjectRevisionRef } from '../../commands/generated/typescript/SubjectRevisionRef';
export type ActualPortOutputObservation = {port:PortRef;unit:string} & (
  {status:'available';run_id:string;at_ms:number;item_type_ref:SubjectRevisionRef|null;completed_output:Quantity;accepted_output:Quantity} |
  {status:'unavailable';reason:'not_wired'|'unsupported_capability';message:string}
);
export interface ActualPortOutputFrame {
  run_id:string;
  at_ms:number;
  device:{id:string;module_ref:ModuleRef}|null;
  ports:ActualPortOutputObservation[];
  /** DefinitionCycle has no formal Project instance port. */
  not_applicable?:string;
}
export interface ActualMediaFrame {
  run: RunResult;
  snapshot: RunObjectsPage;
  job_id: string;
  /** Exact operation frozen for this job, not subsequently edited rules. */
  operation: OperationDefinition;
  /** Optional private getter results; older consumers may omit this source. */
  port_outputs?: ActualPortOutputFrame;
}
/** Unitless view projection shared by declarations and actual execution. */
export interface VisualProgress {
  known: boolean;
  display_phase_id: string | null;
  effective_ms: number | null;
  duration_ms: number | null;
  progress: number | null;
  terminal_pose: boolean;
}
export interface DeclaredMediaPreview {
  phase_ref: string;
  view_u: number;
}
export interface PhasePoint extends VisualProgress {
  at_ms: number;
  job: JobState | null;
  phase_id: string | null;
  paused: boolean;
  waits: WaitInterval[];
  claims: ResourceClaim[];
  items: RuntimeItem[];
  issues: string[];
}
export class UnsupportedMedia extends Error {
  constructor(message: string, readonly asset?: AssetRef) { super(message); this.name = 'UnsupportedMedia'; }
}
export type AssetByteReader = (asset: AssetRef) => Promise<ArrayBuffer>;
export const assetKey = (asset: AssetRef) => JSON.stringify([asset.id, asset.path, asset.kind]);
