// Explicit acceptance-build telemetry. Marks are semantics/submission, never presented frames.
type Data = Record<string, unknown>;
type PathSelection = { kind: 'directory' | 'file'; purpose: string; path: string; attempt_id: string };
export function createNfrRecorder(enabled: boolean, emit: (name: string, detail: Data) => void) {
  let active: Data | null = null;
  let serial = 0;
  const paths: PathSelection[] = [];
  const observed = new Set<string>();
  const mark = (stage: string, detail: Data = {}) => {
    if (enabled && active) emit(`sf.nfr.${String(active.attempt_id)}.${stage}.${++serial}`, { ...active, ...detail, stage, presented_frame: false });
  };
  return {
    enabled,
    begin(detail: Data) {
      if (!enabled) throw new Error('NFR instrumentation is not in this build');
      if (active) throw new Error('Finish the current attempt before starting another');
      if (!detail.attempt_id) throw new Error('Actual attempt identity required');
      active = { ...detail }; observed.clear(); mark('armed');
    },
    finish() { mark('attempt_end'); active = null; paths.length = 0; observed.clear(); },
    mark,
    reply(request_id: string, operation: string, raw: unknown) {
      if (!enabled || !active) return;
      const outcome = raw as { status: string; data?: Data };
      const task = outcome.data?.task as Data | undefined;
      mark('host_reply', { request_id, operation, outcome: outcome.status, new_revision: outcome.data?.new_revision, saved_revision: outcome.data?.saved_revision, task_id: task?.id ?? outcome.data?.id, task_state: task?.state });
    },
    queuePath(value: PathSelection) {
      if (!enabled || !active || value.attempt_id !== active.attempt_id) throw new Error('Path must belong to the active explicit NFR attempt');
      if (paths.length) throw new Error('A path is already queued');
      paths.push({ ...value });
    },
    takePath(kind: PathSelection['kind'], purpose: string) {
      if (!enabled || !paths.length) return null;
      const value = paths[0];
      if (value.kind !== kind || value.purpose !== purpose) throw new Error('Queued path purpose/kind mismatch');
      paths.shift(); mark('path_request', { purpose, kind });
      return value;
    },
    semantic(stage: string, detail: Data) {
      if (!active || !enabled) return;
      const key = stage + JSON.stringify(detail);
      if (!observed.has(key)) { observed.add(key); mark(stage, detail); }
    },
    snapshot() { return { enabled, active: active ? { ...active } : null, queued_paths: paths.length }; },
  };
}
const enabled = (import.meta as ImportMeta & { env?: Record<string, string> }).env?.VITE_SF_NFR_MEASURE === '1';
export const nfr = createNfrRecorder(enabled, (name, detail) => performance.mark(name, { detail }));
if (enabled && typeof window !== 'undefined') {
  Object.defineProperty(window, '__SF_NFR__', { value: nfr, configurable: false });
  for (const type of ['click', 'pointerdown', 'keydown', 'change']) {
    window.addEventListener(type, event => {
      if (event instanceof KeyboardEvent && !['Enter', 'Escape', ' '].includes(event.key)) return;
      nfr.mark('input', { event_type: event.type, trusted: event.isTrusted });
    }, { capture: true });
  }
}
