import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { RunComparison } from '../../../../../packages/commands/generated/typescript/RunComparison';
import type { CompareRunsParams } from '../../../../../packages/commands/generated/typescript/CompareRunsParams';
import type { ExportEvaluationParams } from '../../../../../packages/commands/generated/typescript/ExportEvaluationParams';
import type { ExportPackageParams } from '../../../../../packages/commands/generated/typescript/ExportPackageParams';
import type { TaskHandle } from '../../../../../packages/commands/generated/typescript/TaskHandle';
import type { TaskState } from '../../../../../packages/commands/generated/typescript/TaskState';
import type { CancellationState } from '../../../../../packages/commands/generated/typescript/CancellationState';
import type { ScenarioReference } from '../../../../../packages/commands/generated/typescript/ScenarioReference';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';

export interface RunEvidence { project: ProjectState; scenario: ScenarioInput; run: RunResult }
export type EvaluationSelection = Omit<ExportEvaluationParams, 'selected_destination_ref'>;
export type PackageExportSelection = Omit<ExportPackageParams, 'selected_destination_ref'>;
export interface DeliveryServices {
  readEvidence(runId: string): Promise<RunEvidence>;
  compare(params: CompareRunsParams): Promise<RunComparison>;
  /** Root selects a trusted native destination. null means the picker was cancelled. */
  startEvaluation(params: EvaluationSelection): Promise<TaskHandle | null>;
  startPackage(params: PackageExportSelection): Promise<TaskHandle | null>;
  readTask(handle: TaskHandle): Promise<TaskState>;
  cancelTask(handle: TaskHandle): Promise<CancellationState>;
  startSave?(project: ProjectState): Promise<TaskHandle | null>;
  cloneScenario?(request: { source_run_id: string; new_scenario_id: string; title: string }): Promise<ScenarioReference>;
}
export interface DeliveryWorkspaceProps {
  workspace: WorkspaceSnapshot;
  services: DeliveryServices;
  initialTab?: 'comparison' | 'reporting' | 'handoff';
  onClose(): void;
  onInspectRun(runId: string): void;
  onLocate(runId: string, object: ObjectRef): void;
  onOpenPackage(): void;
  onEditModule?(ref: ModuleRef): void;
}
