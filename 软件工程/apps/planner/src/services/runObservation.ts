import {invoke} from '@tauri-apps/api/core';
import {HostFailure} from './projectHost';
import type {ApiError} from '../../../../packages/commands/generated/typescript/ApiError';
import type {RunObjectsPage} from '../../../../packages/commands/generated/typescript/RunObjectsPage';
import type {ProcessScope} from './presentationHost';
import {decodeObservationFrame, type ObservationFrame, type ObservationFrameReply} from './runObservationDelta';

export interface RunObservationReadRequest extends ProcessScope { run_id: string; at_ms: number }
let previous: {key: string; value: ObservationFrame} | null = null;
let readSerial = 0;
export async function readRunObservation({session_id, ...request}: RunObservationReadRequest): Promise<RunObjectsPage> {
  const key = JSON.stringify([session_id, request.project_id, request.scope, request.scope_revision, request.run_id]);
  const base = previous?.key === key ? previous.value : null;
  const serial = ++readSerial;
  try {
    const reply = await invoke<ObservationFrameReply>('project_read_run_observation_delta', {request, baseFrame: base?.frame ?? null});
    const value = decodeObservationFrame(base, reply);
    if (value.page.run_id !== request.run_id || value.page.at_ms !== request.at_ms || value.page.next_index !== null) throw new Error('实际观察的Run、时点或完整性不匹配。');
    if (serial === readSerial) previous = {key, value};
    return value.page;
  }
  catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) throw new HostFailure(error as ApiError);
    throw error;
  }
}
