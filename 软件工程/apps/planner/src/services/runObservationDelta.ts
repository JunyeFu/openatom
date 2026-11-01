import type { DomainObject } from '../../../../packages/commands/generated/typescript/DomainObject';
import type { RunObjectsPage } from '../../../../packages/commands/generated/typescript/RunObjectsPage';

export interface ObservationFrame { frame: number; page: RunObjectsPage }
export type ObservationFrameReply =
  | { kind: 'full'; data: ObservationFrame }
  | { kind: 'delta'; data: { frame: number; base: number; run_id: string; at_ms: number; objects_len: number; updates: [number, DomainObject][] } };

/** Transport reconstruction only; every updated object comes from the actual Host. */
export function decodeObservationFrame(base: ObservationFrame | null, reply: ObservationFrameReply): ObservationFrame {
  if (reply.kind === 'full') return reply.data;
  if (!base || reply.data.base !== base.frame) throw new Error('实际观察增量未匹配此请求的原帧，停止展示。');
  const objects = base.page.objects.slice(0, reply.data.objects_len);
  for (const [index, object] of reply.data.updates) objects[index] = object;
  return { frame: reply.data.frame, page: { run_id: reply.data.run_id, at_ms: reply.data.at_ms, objects, next_index: null } };
}
