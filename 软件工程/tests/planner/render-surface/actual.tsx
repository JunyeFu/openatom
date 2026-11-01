import { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { FactoryRenderSurface, type FactoryRenderFrame } from '../../../apps/planner/src/features/render-surface/FactoryRenderSurface';
import type { FactorySceneFailure } from '../../../apps/planner/src/features/render-surface/factoryCapture';
import { createFactoryPresentation, type FactoryPresentationContext, type FactoryPresentationSource } from '../../../apps/planner/src/services/factoryPresentation';
import type { PresentationReadPorts } from '../../../apps/planner/src/services/presentationHost';
import type { ProjectState } from '../../../packages/commands/generated/typescript/ProjectState';
import type { RenderViewSpec } from '../../../packages/commands/generated/typescript/RenderViewSpec';
import type { ApiRequest } from '../../../packages/commands/generated/typescript/ApiRequest';
import '../../../apps/planner/src/styles/workbench.css';
const boot = await fetch('/__render_surface_core').then(response => response.json());
const rpc = (query: unknown) => fetch('/__render_surface_core', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(query) }).then(response => response.json());
const ready: FactoryRenderFrame[] = [], failures: FactorySceneFailure[] = [], calls: unknown[] = [], releases: (() => void)[] = [];
let holdItems = false, rejectItems = false;
const ports: PresentationReadPorts = {
  async dispatch<T>(request: ApiRequest): Promise<T> {
    calls.push(request); const reply = await rpc({ kind: 'dispatch', request });
    if (reply.outcome.status !== 'ok') throw new Error(JSON.stringify(reply.outcome.data));
    if (request.operation === 'get_object' && request.params.object_id.id === 'layout.native.item') {
      if (holdItems) await new Promise<void>(resolve => releases.push(resolve));
      if (rejectItems) throw new Error('actual-test-item-module-provider-failed');
    }
    return reply.outcome.data as T;
  },
  async readAsset(request) {
    calls.push(request); const reply = await rpc({ kind: 'media', ...request });
    if (!reply.ok) throw new Error(JSON.stringify(reply.error)); return Uint8Array.from(reply.bytes).buffer;
  },
  async readPortOutput(request) {
    const reply = await rpc({ kind: 'port_output', ...request });
    if (!reply.ok) throw new Error(JSON.stringify(reply.error)); return reply.data;
  },
};
type State = { at: number; lod: 'detailed' | 'simplified' | 'region'; mounted: boolean; camera: 'wide' | 'package'; run: string; epoch: number };
declare global { interface Window { __actualSurface: { set(patch: Partial<State>): void; clear(): void; hold(): void; release(): void; reject(value: boolean): void; read(): unknown } } }
function App() {
  const [state, setState] = useState<State>({ at: 110000, lod: 'detailed', mounted: true, camera: 'package', run: boot.layout_run.id, epoch: 0 });
  const [prepared, setPrepared] = useState<{ source: FactoryPresentationSource; context: FactoryPresentationContext } | null>(null), [error, setError] = useState('');
  const current = useRef<FactoryPresentationContext | null>(null);
  useEffect(() => {
    let live = true; setPrepared(null); setError('');
    (async () => {
      const snapshot = await rpc({ kind: 'snapshot' }), project = snapshot.project as ProjectState;
      const context: FactoryPresentationContext = { session_id: boot.context.host.session_id, project_id: project.id, scope: { kind: 'project' }, scope_revision: project.revision,
        project, view: { run_id: state.run, at_ms: state.at } };
      current.current = context; const source = await createFactoryPresentation(() => current.current, ports).read(context);
      if (live) setPrepared({ source, context });
    })().catch(reason => { if (live) setError(String(reason)); });
    return () => { live = false; current.current = null; };
  }, [state.at, state.run, state.epoch]);
  const view = useMemo<RenderViewSpec | null>(() => prepared ? { kind: 'multi_hall_canvas', canvas: 'multi_hall_canvas', hall_id: null,
    project_revision: prepared.source.project.revision, run_id: prepared.source.run!.id, view_time_ms: prepared.source.observation!.at_ms, lod: state.lod,
    camera: { coordinate_frame: 'right_handed_z_up', unit: 'meter', eye: state.camera === 'package' ? { x: 20, y: -2, z: 4 } : { x: 32, y: 28, z: 28 }, target: state.camera === 'package' ? { x: 17.3, y: 2, z: .3 } : { x: 12, y: 8, z: 0 }, zoom: 1 } } : null,
  [prepared, state.camera, state.run, state.at, state.lod]);
  window.__actualSurface = { set: patch => setState(old => ({ ...old, ...patch })), clear: () => { ready.length = 0; failures.length = 0; calls.length = 0; },
    hold: () => { holdItems = true; }, release: () => { holdItems = false; for (const release of releases.splice(0)) release(); }, reject: value => { rejectItems = value; },
    read: () => ({ state, ready, failures, held: releases.length, error, calls, source: prepared?.source }) };
  return <><h1>实际Host厂房只读捕获表面</h1>{error && <p role="alert">{error}</p>}<div style={{ position: 'relative', width: 1040, height: 680 }}>
    {state.mounted && prepared && view && prepared.context.view?.run_id === state.run && prepared.context.view.at_ms === state.at && <FactoryRenderSurface context={prepared.context} source={prepared.source} view={view} onFrameReady={frame => ready.push(frame)} onFailure={failure => failures.push(failure)} />}
  </div></>;
}
const container = document.getElementById('root') as HTMLElement & { actualSurfaceRoot?: ReturnType<typeof createRoot> };
const root = container.actualSurfaceRoot ?? (container.actualSurfaceRoot = createRoot(container)); root.render(<App />);
if (import.meta.hot) import.meta.hot.dispose(() => { root.unmount(); if (container.actualSurfaceRoot === root) delete container.actualSurfaceRoot; });
