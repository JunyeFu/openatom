import { useMemo, useState } from 'react';
import type { ScenarioInput } from '../../../../../packages/commands/generated/typescript/ScenarioInput';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import { cloudContextKey, type CloudTaskRecord } from '../../services/cloudProduction';
import { createExecutionAssetPort } from '../../services/executionAssets';
import { createExecutionExportDirectories } from '../../services/executionDirectories';
import { CloudProductionPanel, type ExecutionViewSelection } from './CloudProductionPanel';
import { ExecutionObservationView } from './ExecutionObservationView';
import { RetainedCloudTasks } from './RetainedCloudTasks';

interface Props {
  workspace: WorkspaceSnapshot;
  scenario: ScenarioInput | null;
  retainedTasks: readonly CloudTaskRecord[];
  onTaskRegistered: (record: CloudTaskRecord) => void;
  onDirtyChange: (dirty: boolean) => void;
  onClose: () => void;
  onRefresh: () => Promise<void>;
  onLocateDevice: (deviceId: string) => void;
  onLocateExecutionObject: (kind: string, id: string) => void;
}

export function CloudProductionWorkspace(props: Props) {
  const { workspace } = props;
  const [selection, setSelection] = useState<ExecutionViewSelection | null>(null);
  const directories = useMemo(createExecutionExportDirectories, []);
  const context = useMemo(() => workspace.project && workspace.hostContext ? {
    projectId: workspace.project.id,
    projectRevision: workspace.project.revision,
    hostContextId: workspace.hostContext.host_instance_id,
  } : null, [workspace.project?.id, workspace.project?.revision, workspace.hostContext?.host_instance_id]);
  const readAsset = useMemo(() => context ? createExecutionAssetPort(context) : undefined, [context]);
  if (!context || !workspace.hostContext) return <section role="alert">尚未取得当前 Native 工程会话，请刷新工程后再打开云端生产。</section>;
  const currentTasks = props.retainedTasks.filter(record => record.operation !== 'read_cloud_production' && cloudContextKey(record.context) === cloudContextKey(context));
  const retained = [...currentTasks].reverse().find(record => !['completed', 'failed', 'cancelled', 'rejected'].includes(record.status)) ?? currentTasks.at(-1);
  const predictions = (workspace.context.run_summaries ?? [])
    .filter(run => ['completed', 'failed', 'cancelled'].includes(run.computation_status))
    .map(run => ({ run_id: run.run_id, input_ref: {
      project_id: run.input.project_id, project_revision: run.input.project_revision,
      scenario_id: run.input.scenario_id, scenario_revision: run.input.scenario_revision,
    } }));
  return <div style={{ height: '100%', minHeight: 0, display: 'flex', flexDirection: 'column' }}>
    <div style={{ display: selection ? 'none' : undefined, flex: 1, minHeight: 0 }}>
      <CloudProductionPanel project={workspace.project} scenario={props.scenario}
        capabilities={workspace.capabilities} hostContextId={context.hostContextId}
        retainedTask={retained} onTaskRegistered={props.onTaskRegistered}
        onUnappliedInputChanged={props.onDirtyChange} onRefreshProject={props.onRefresh}
        predictionRuns={predictions} directories={directories} onClose={props.onClose}
        onLocateDevice={props.onLocateDevice} onInspectExecution={setSelection}/>
    </div>
    {!selection && <RetainedCloudTasks context={context} records={props.retainedTasks} onTaskRegistered={props.onTaskRegistered}/>}
    {selection && <ExecutionObservationView context={context}
      nativeSessionId={workspace.hostContext.session_id} selection={selection}
      readAsset={readAsset} onClose={() => setSelection(null)}
      onSelectionChange={setSelection}
      onLocateObject={reference => props.onLocateExecutionObject(reference.kind, reference.id)}/>} 
  </div>;
}
