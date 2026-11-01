import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { ApiCategory } from '../../../../../packages/commands/generated/typescript/ApiCategory';
import type { ApiRequest } from '../../../../../packages/commands/generated/typescript/ApiRequest';
import type { DefinitionSummary } from '../../../../../packages/commands/generated/typescript/DefinitionSummary';
import type { ModuleDefinition } from '../../../../../packages/commands/generated/typescript/ModuleDefinition';
import type { HistoryAvailability } from '../../../../../packages/commands/generated/typescript/HistoryAvailability';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { FilePurpose } from '../../../../../packages/commands/generated/typescript/FilePurpose';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { AuthoringPresentationSource } from '../../services/authoringPresentation';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { SelectedFileRef } from '../../../../../packages/commands/generated/typescript/SelectedFileRef';
import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskOutput } from '../../../../../packages/commands/generated/typescript/TaskOutput';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { TemplateSubset } from '../../../../../packages/commands/generated/typescript/TemplateSubset';
import type { ScenarioReference } from '../../../../../packages/commands/generated/typescript/ScenarioReference';

export interface TemplateCompositionInput {
  project_id: string;
  base_revision: number;
  definition: ModuleDefinition;
  draft: AuthoringDraft | null;
  subset: TemplateSubset;
  source_scenario?: ScenarioReference | null;
}
export type AuthoringTemplates =
  | { status: 'unavailable'; reason: string }
  | { status: 'available'; save(input: TemplateCompositionInput): Promise<{ draft: AuthoringDraft; commit: EditCommit }> };

/** Root supplies the existing SDK and native pickers; this feature owns no Host. */
export interface AuthoringIO {
  dispatch<T>(request: ApiRequest): Promise<T>;
  awaitTask(task: TaskHandle, onState?: (state: TaskState) => void): Promise<TaskOutput>;
  pickDirectory(purpose: FilePurpose): Promise<SelectedFileRef | null>;
}

/** Only fields actually returned by context/get_object/EditCommit are retained. */
export interface AuthoringDraft {
  summary: DefinitionSummary;
  definition: ModuleDefinition;
  history: HistoryAvailability | null;
}

export type AuthoringSamples =
  | { status: 'unavailable'; reason: string }
  | { status: 'available'; run(draft: AuthoringDraft, scenario: ScenarioInput): Promise<TaskState> };

export type AuthoringNavigationGuard = (title: string, proceed: () => void) => void;
export interface ModuleAuthoringWorkspaceProps {
  workspace: WorkspaceSnapshot;
  busy: boolean;
  io: AuthoringIO;
  samples: AuthoringSamples;
  templates: AuthoringTemplates;
  initialModule?: ModuleRef;
  initialDraft?: AuthoringDraft;
  onDraftChanged(draft: AuthoringDraft | null): void;
  onCommitted(commit: EditCommit): void | Promise<void>;
  onPreviewChange(presentation: Presentation | null, source?: AuthoringPresentationSource | null): void;
  onConsumerRequested(category: ApiCategory, module: ModuleRef): void;
  onClose(): void;
  /** Root menus must use the same unsaved-input decision as this workspace. */
  onNavigationGuard?(guard: AuthoringNavigationGuard | null): void;
  onDirtyChange?(dirty: boolean): void;
}
