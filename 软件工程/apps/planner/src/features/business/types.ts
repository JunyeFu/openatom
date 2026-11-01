import type { ApiError } from '../../../../../packages/commands/generated/typescript/ApiError';
import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { EditCommit } from '../../../../../packages/commands/generated/typescript/EditCommit';
import type { HistoryAvailability } from '../../../../../packages/commands/generated/typescript/HistoryAvailability';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { RunObjectsPage } from '../../../../../packages/commands/generated/typescript/RunObjectsPage';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';

export interface BusinessSnapshot { project: ProjectState; scenarios: ScenarioInput[] }
export interface BusinessCommitProposal {
  project_id: string;
  base_project_revision: number;
  scenario_revisions: Record<string, number>;
  label: string;
  commands: EditCommand[];
}
export type BusinessCommitResult = { ok: true; commit: EditCommit; snapshot: BusinessSnapshot } | { ok: false; error: ApiError };
export interface BusinessRunRequest { project_id: string; base_project_revision: number; scenario_id: string; base_scenario_revision: number }
export interface BusinessLocateRequest { object: ObjectRef; source: 'draft' | 'run'; run_id?: string; at_ms?: number }
export interface BusinessEditIntent { id: string; section: 'orders' | 'tasks' | 'routes' | 'calendars' | 'stages' | 'items' | 'resources' | 'supplies' | 'actual'; object_id?: string }
export interface BusinessEditorProps {
  project: ProjectState | null;
  scenarios: readonly ScenarioInput[];
  modules: ReadonlyMap<string, ModulePackage>;
  scenarioId: string | null;
  history: HistoryAvailability | null;
  run?: RunResult | null;
  frozenProject?: ProjectState | null;
  frozenScenario?: ScenarioInput | null;
  observation?: RunObjectsPage | null;
  atMs?: number | null;
  busy?: boolean;
  editIntent?: BusinessEditIntent;
  // Set true only after the real public SetProjectSettings consumer supports
  // ProjectState.setup_policies; never label an ignored field as committed.
  canEditSetupPolicies?: boolean;
  onCommit: (proposal: BusinessCommitProposal) => Promise<BusinessCommitResult>;
  onRefresh: () => Promise<void>;
  onRun: (request: BusinessRunRequest) => Promise<void>;
  onScenarioChange: (id: string) => void;
  onLocate: (request: BusinessLocateRequest) => void;
  onClose?: () => void;
  onDirtyChange?: (value: { project_id: string; dirty: boolean }) => void;
}
