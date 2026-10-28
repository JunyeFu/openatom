import type { ApiError } from '../../../../../packages/commands/generated/typescript/ApiError';
import type { EditCommand } from '../../../../../packages/commands/generated/typescript/EditCommand';
import type { JobState } from '../../../../../packages/commands/generated/typescript/JobState';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { RunInputRef } from '../../../../../packages/commands/generated/typescript/RunInputRef';
import type { RunResult } from '../../../../../packages/commands/generated/typescript/RunResult';
import type { RunObjectsPage } from '../../../../../packages/commands/generated/typescript/RunObjectsPage';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { SimulationEvent } from '../../../../../packages/commands/generated/typescript/SimulationEvent';
import type { WaitInterval } from '../../../../../packages/commands/generated/typescript/WaitInterval';

export type TimeWindow = { start_ms: number; end_ms: number };
export type TimelineMode = 'input' | 'actual' | 'comparison';
export type InputField = { kind: 'arrival' | 'order_release' | 'task_release' | 'order_due'; id: string };
export type InputChange = InputField & { before_ms: number; after_ms: number };
export interface TimelineInputCommit {
  project_id: string;
  base_project_revision: number;
  base_scenario_revision: number;
  scenario_id: string;
  command: Extract<EditCommand, { kind: 'put_object' }> & { data: { kind: 'scenario'; data: ScenarioInput } };
  changes: InputChange[];
}
export type TimelineCommitResult = { ok: true; scenario: ScenarioInput } | { ok: false; error: ApiError };
export interface TimelineSelection {
  key: string;
  source: 'actual' | 'input' | 'manual_expected';
  run_id: string | null;
  input_ref?: RunInputRef;
  object: ObjectRef;
  label: string;
  true_interval?: TimeWindow;
  at_ms?: number;
  event?: SimulationEvent;
  job?: JobState;
  wait?: WaitInterval;
  fields: { label: string; value: string }[];
  related_objects: ObjectRef[];
}
export interface DisplayGroup { id: string; hall_id: string; title: string; members: string[] }
export interface TimelineViewPreferencesV1 {
  version: 1;
  group_overrides: DisplayGroup[];
  independent_device_ids: string[];
  row_order: string[];
  collapsed_group_ids: string[];
  pinned_resource_ids: string[];
  axis_windows: Record<string, TimeWindow>;
  row_scroll: number;
  snap_enabled: boolean;
}
export interface TimelineComparison { project: ProjectState; run: RunResult; scenario?: ScenarioInput | null }
export interface ManualExpectedSpan { id: string; object: ObjectRef; label: string; interval: TimeWindow }
export interface TimelineExpected { title: string; source_ref: string; spans: ManualExpectedSpan[] }
export interface Props {
  inputTextScale?: number;
  /** Hide the dock surface without interrupting the result-view clock. */
  hidden?: boolean;
  /** Immutable render task: same projection/virtual rows, no playback, edits or stored preferences. */
  readonly?:boolean;
  onFrameReady?:(value:{visible_objects:ObjectRef[];limitations:string[]})=>void;
  project: ProjectState | null;
  run: RunResult | null;
  atMs: number | null;
  onTime: (value: number) => void;
  onSelect: (value: ObjectRef) => void;
  scenario?: ScenarioInput | null;
  runScenario?: ScenarioInput | null;
  draftProject?: ProjectState | null;
  observation?: RunObjectsPage | null;
  comparison?: TimelineComparison | null;
  expected?: TimelineExpected | null;
  selected?: TimelineSelection | null;
  onInspect?: (value: TimelineSelection | null) => void;
  onSeek?: (request: { run_id: string; at_ms: number }) => void;
  onCommitInput?: (proposal: TimelineInputCommit) => Promise<TimelineCommitResult>;
  onRefreshInput?: () => Promise<void>;
  onEditBusiness?: (request: { section: 'orders' | 'tasks' | 'calendars' | 'stages'; object_id?: string }) => void;
  preferences?: TimelineViewPreferencesV1;
  onPreferencesChange?: (value: TimelineViewPreferencesV1) => void;
}
export interface Primitive {
  key: string;
  row: string;
  label: string;
  kind: 'job' | 'work' | 'wait' | 'claim' | 'calendar' | 'arrival' | 'transport' | 'acceptance' | 'event' | 'input' | 'expected';
  source: 'actual' | 'input' | 'manual_expected';
  run_id: string | null;
  object: ObjectRef;
  interval?: TimeWindow;
  point_ms?: number;
  known_end: boolean;
  phase?: string;
  selection: TimelineSelection;
  input_field?: InputField;
}
