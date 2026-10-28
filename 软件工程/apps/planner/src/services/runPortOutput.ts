import {invoke} from '@tauri-apps/api/core';
import {HostFailure} from './projectHost';
import type {ApiError} from '../../../../packages/commands/generated/typescript/ApiError';
import type {ApiScope} from '../../../../packages/commands/generated/typescript/ApiScope';
import type {PortRef} from '../../../../packages/commands/generated/typescript/PortRef';
import type {Quantity} from '../../../../packages/commands/generated/typescript/Quantity';
import type {SubjectRevisionRef} from '../../../../packages/commands/generated/typescript/SubjectRevisionRef';
import type {ProjectId} from '../../../../packages/commands/generated/typescript/ProjectId';
import type {Revision} from '../../../../packages/commands/generated/typescript/Revision';
import type {RunId} from '../../../../packages/commands/generated/typescript/RunId';
import type {SimTimeMs} from '../../../../packages/commands/generated/typescript/SimTimeMs';

export interface RunPortOutputReadRequest {
  project_id: ProjectId;
  scope: Exclude<ApiScope,{kind:'workspace'}>;
  scope_revision: Revision;
  run_id: RunId;
  at_ms: SimTimeMs;
  port: PortRef;
}
/** Private UI projection; canonical RunResult is unchanged. */
export interface RunPortOutput {
  run_id: RunId;
  at_ms: SimTimeMs;
  port: PortRef;
  item_type_ref: SubjectRevisionRef|null;
  completed_output: Quantity;
  accepted_output: Quantity;
}
type PortOutputResult = {Ok: RunPortOutput} | {Err: ApiError};
type PendingRead = {request: RunPortOutputReadRequest; resolve(value: RunPortOutput): void; reject(error: unknown): void};
let pending: PendingRead[] = [];
const hostFailure = (error: unknown) => typeof error === 'object' && error !== null && 'code' in error && 'message' in error
  ? new HostFailure(error as ApiError) : error;

async function flushPortOutputs() {
  const batch = pending; pending = [];
  try {
    const results = await invoke<PortOutputResult[]>('project_read_run_port_outputs', {requests: batch.map(read => read.request)});
    results.forEach((result, index) => {
      if ('Ok' in result) batch[index].resolve(result.Ok);
      else batch[index].reject(hostFailure(result.Err));
    });
  } catch (error) {
    for (const read of batch) read.reject(hostFailure(error));
  }
}

/** Concurrent scene reads share transport; every result still comes from its original admitted getter. */
export function readRunPortOutput(request:RunPortOutputReadRequest):Promise<RunPortOutput> {
  return new Promise((resolve, reject) => {
    pending.push({request, resolve, reject});
    if (pending.length === 1) queueMicrotask(flushPortOutputs);
  });
}
