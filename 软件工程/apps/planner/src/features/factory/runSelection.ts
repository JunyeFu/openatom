import type { RunSummary } from '../../../../../packages/commands/generated/typescript/RunSummary';

type RunSelectionStorage = Pick<Storage, 'getItem' | 'setItem'>;
const selectionKey = (projectId: string) => `sf.workbench.selectedRun.${encodeURIComponent(projectId)}`;
function localStorageForRun(storage?: RunSelectionStorage | null): RunSelectionStorage | null {
  try { return storage === undefined ? (typeof window === 'undefined' ? null : window.localStorage) : storage; }
  catch { return null; }
}

export function readRememberedRun(projectId: string, storage?: RunSelectionStorage | null): string | null {
  if (!projectId) return null;
  try {
    const raw = localStorageForRun(storage)?.getItem(selectionKey(projectId));
    if (!raw) return null;
    const id: unknown = JSON.parse(raw);
    return typeof id === 'string' && id.length > 0 ? id : null;
  } catch { return null; }
}

export function rememberRunSelection(projectId: string, runId: string, storage?: RunSelectionStorage | null): void {
  if (!projectId || !runId) return;
  try { localStorageForRun(storage)?.setItem(selectionKey(projectId), JSON.stringify(runId)); }
  catch { /* UI preference storage must not block a Run. */ }
}

export type RunSelectionRequest = {
  projectId: string | null;
  scenarioId: string | null;
  runs: readonly RunSummary[];
  selectedRunId: string | null;
  selectedProjectId: string | null;
  completedRunId: string | null;
  projectOpened: boolean;
};

/** Choose an available result without replacing a user's selection on ordinary refresh. */
export function selectRunForWorkspace(request: RunSelectionRequest): string | null {
  const { projectId, scenarioId, runs, selectedRunId, selectedProjectId, completedRunId, projectOpened } = request;
  if (!projectId) return null;
  const available = runs.filter(summary => summary.input.project_id === projectId && summary.computed_until_ms !== null);
  if (completedRunId) return available.find(summary => summary.run_id === completedRunId)?.run_id ?? null;
  if (selectedProjectId === projectId && selectedRunId) {
    const selected = available.find(summary => summary.run_id === selectedRunId);
    if (selected) return selected.run_id;
  }
  if (!projectOpened) return null;
  const matchingScenario = scenarioId ? available.filter(summary => summary.input.scenario_id === scenarioId) : [];
  return (matchingScenario.at(-1) ?? available.at(-1))?.run_id ?? null;
}
