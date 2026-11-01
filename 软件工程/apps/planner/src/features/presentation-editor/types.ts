import type {MediaDataValue} from '../media';
import type { ReactNode } from 'react';
import type { AuthoringDraft, AuthoringIO, AuthoringSamples } from '../modules/types';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { DeviceTag } from '../../../../../packages/commands/generated/typescript/DeviceTag';
import type { DeviceAction } from '../../../../../packages/commands/generated/typescript/DeviceAction';
import type { DataTagBinding } from '../../../../../packages/commands/generated/typescript/DataTagBinding';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { RunObjectsPage } from '../../../../../packages/commands/generated/typescript/RunObjectsPage';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';

/** Private UI ports; Domain/Run/Assets remain canonical, Host/loader remain external. */
export interface PresentationViewInput {
  presentation: Presentation;
  mode: 'isolated_view' | 'actual_run';
  view_u: number;
  phase_ref: string | null;
  canAction(tag: DeviceTag): boolean;
  onAction(tag: DeviceTag): void;
  onSeek(view_u: number): void;
}
export interface PresentationMediaPort {
  fields: readonly { label: string; binding: DataTagBinding; unavailable_reason?: string }[];
  render(input: PresentationViewInput): ReactNode;
  resolveData?(binding:DataTagBinding,mode:PresentationViewInput['mode']):MediaDataValue;
}
export interface PresentationActionRequest {
  project_id: string;
  draft_id: string;
  draft_revision: number;
  tag: DeviceTag;
}
export interface PresentationEditorState {
  identity: string;
  dirty: boolean;
  busy: boolean;
}
export interface ImageTimelineEditorProps {
  workspace: WorkspaceSnapshot;
  draft: AuthoringDraft | null;
  source: ModulePackage | null;
  io: AuthoringIO;
  samples: AuthoringSamples;
  scenario: ScenarioInput | null;
  media?: PresentationMediaPort;
  actions: Partial<Record<DeviceAction, (request: PresentationActionRequest) => Promise<{ message: string }>>>;
  run?: RunResult | null;
  observation?: RunObjectsPage | null;
  instance?: { id: string; module_ref: ModuleRef } | null;
  onDraftChanged(draft: AuthoringDraft): void;
  onCommitted(commit: EditCommit): void | Promise<void>;
  onSampleRun?(run: RunResult): void | Promise<void>;
  /** Actual local inputs/pending work for the existing Native close guard. */
  onEditorStateChanged?(state: PresentationEditorState): void;
  onAdoptionRequested?(reference: ModuleRef): void | Promise<void>;
  onImportPackageRequested?(): void | Promise<void>;
  onClose(): void | Promise<void>;
}
