import { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import * as THREE from 'three';
import { FactoryRenderSurface, type FactoryRenderFrame } from '../../../apps/planner/src/features/render-surface/FactoryRenderSurface';
import type { FactorySceneFailure } from '../../../apps/planner/src/features/render-surface/factoryCapture';
import { createFactoryPresentation, type FactoryPresentationContext, type FactoryPresentationSource } from '../../../apps/planner/src/services/factoryPresentation';
import type { MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import type { RenderViewSpec } from '../../../packages/commands/generated/typescript/RenderViewSpec';
import { fixture, fixturePorts, frames, originalRuns, pose } from '../factory-media/fixture';
import '../../../apps/planner/src/styles/workbench.css';
import modelUrl from '../../../modules/media/p302-controlled/station.glb?url';
import simpleUrl from '../../../modules/media/p302-controlled/station-simple.glb?url';
import redUrl from '../media-root-sequence/red.png?url';

const ready: FactoryRenderFrame[] = [], failures: FactorySceneFailure[] = [], reads: { request: MediaAssetReadRequest; state: string }[] = [];
const releases: (() => void)[] = [];
let holding = false, rejectBytes = false;
let latest: { scene: THREE.Scene; camera: THREE.Camera; renderer: THREE.WebGLRenderer } | null = null;
const oldBeforeRender = THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender = function (renderer, scene, camera, geometry, material, group) {
  latest = { scene: scene as THREE.Scene, camera, renderer };
  return oldBeforeRender.call(this, renderer, scene, camera, geometry, material, group);
};
async function readBytes(request: MediaAssetReadRequest) {
  const assets = [...fixture().model.assets, { id: 'root.pixel.red', path: 'red.png', kind: 'png' }];
  if (!assets.some(asset => asset.id === request.asset.id && asset.kind === request.asset.kind && asset.path === request.asset.path)) throw new Error('Exact test AssetRef is not declared');
  const record = { request: structuredClone(request), state: 'started' }; reads.push(record);
  if (holding) { record.state = 'held'; await new Promise<void>(resolve => releases.push(resolve)); }
  if (rejectBytes) { record.state = 'failed'; throw new Error('observed-test-exact-asset-read-failure'); }
  const urls: Record<string, string> = { 'station.glb': modelUrl, 'station-simple.glb': simpleUrl, 'red.png': redUrl };
  const url = urls[request.asset.path]; if (!url) throw new Error('No exact test asset provider');
  const response = await fetch(url); if (!response.ok) throw new Error('Actual fixture resource failed');
  const bytes = await response.arrayBuffer(); record.state = 'returned'; return bytes;
}
type State = { generation: number; camera: 'A' | 'B' | 'mm' | 'yup' | 'zoom' | 'away'; canvas: 'line_canvas' | 'multi_hall_canvas'; lod: 'detailed' | 'simplified' | 'region'; mounted: boolean; width: number; height: number; actual: boolean; missingMedia: boolean; emptyHalls: boolean; invalid: string | null };
declare global { interface Window { __renderSurface: { set(patch: Partial<State>): void; hold(): void; release(): void; reject(value: boolean): void; clear(): void; read(): unknown; capture(): unknown } } }
function Harness() {
  const [state, setState] = useState<State>({ generation: 1, camera: 'A', canvas: 'multi_hall_canvas', lod: 'detailed', mounted: true, width: 1040, height: 680, actual: false, missingMedia: false, emptyHalls: false, invalid: null });
  const [prepared, setPrepared] = useState<{ source: FactoryPresentationSource; context: FactoryPresentationContext } | null>(null), [error, setError] = useState('');
  const current = useRef<FactoryPresentationContext | null>(null);
  useEffect(() => {
    let live = true; setPrepared(null); setError('');
    const data = fixture();
    if (!state.actual) {
      data.project.id = `surface-${state.generation}`;
      data.project.halls.push({ ...structuredClone(data.project.halls[0]), id: 'H-REMOTE', origin: pose(35, 5) });
      data.project.devices.push({ ...structuredClone(data.project.devices[0]), id: 'REMOTE', hall_id: 'H-REMOTE', line_id: null, pose: pose(2, 2) });
      if (state.missingMedia) {
        const declared = structuredClone(data.pkg); declared.definition.id = 'declared-device-without-presentation';
        if (declared.definition.payload.category !== 'devices') throw new Error('Fixture device category');
        declared.definition.payload.data.presentation = null;
        data.project.devices[0].module_ref = { id: declared.definition.id, version: declared.definition.version };
        data.project.module_refs.push(data.project.devices[0].module_ref); data.packages.push(declared);
      }
      if (state.emptyHalls) data.project.halls = [];
    }
    const actual = frames.find(frame => frame.label === 'calendar@35000')!;
    const context: FactoryPresentationContext = { session_id: 'surface-readonly-browser', project_id: data.project.id,
      scope: { kind: 'project' }, scope_revision: 9, project: data.project, view: state.actual ? { run_id: actual.frame.run.id, at_ms: actual.frame.snapshot.at_ms } : null };
    current.current = context;
    const { ports } = fixturePorts(data, context, readBytes);
    createFactoryPresentation(() => current.current, ports).read(context).then(source => { if (live) setPrepared({ source, context }); }, reason => { if (live) setError(String(reason)); });
    return () => { live = false; current.current = null; };
  }, [state.generation, state.actual, state.missingMedia, state.emptyHalls]);
  const view = useMemo<RenderViewSpec | null>(() => {
    if (!prepared) return null;
    const line = state.canvas === 'line_canvas';
    const target = line ? { x: 2, y: 3, z: .5 } : { x: 5, y: 23, z: .5 };
    const eye = line ? { x: 15, y: 16, z: 18 } : { x: 18, y: 36, z: 18 };
    if (state.camera === 'B') { Object.assign(target, line ? { x: 5, y: 5, z: .6 } : { x: 37, y: 7, z: .6 }); Object.assign(eye, line ? { x: 6, y: -5, z: 6 } : { x: 38, y: -3, z: 6 }); }
    if (state.camera === 'away') { Object.assign(target, { x: -200, y: -200, z: 1 }); Object.assign(eye, { x: -190, y: -190, z: 10 }); }
    let camera: RenderViewSpec['camera'] = { eye, target, coordinate_frame: 'right_handed_z_up', unit: 'meter', zoom: state.camera === 'zoom' ? 2 : 1 };
    if (state.camera === 'mm') camera = { ...camera, unit: 'millimeter', eye: { x: eye.x * 1000, y: eye.y * 1000, z: eye.z * 1000 }, target: { x: target.x * 1000, y: target.y * 1000, z: target.z * 1000 } };
    if (state.camera === 'yup') camera = { ...camera, coordinate_frame: 'right_handed_y_up', eye: { x: eye.x, y: eye.z, z: -eye.y }, target: { x: target.x, y: target.z, z: -target.y } };
    const result: RenderViewSpec = { kind: state.canvas, canvas: state.canvas, hall_id: line ? prepared.source.project.halls[0].id : null,
      project_revision: prepared.source.project.revision, run_id: prepared.source.run?.id ?? null, view_time_ms: prepared.source.observation?.at_ms ?? null, lod: state.lod, camera };
    if (state.invalid === 'revision') result.project_revision++;
    if (state.invalid === 'canvas') result.kind = line ? 'multi_hall_canvas' : 'line_canvas';
    if (state.invalid === 'hall') result.hall_id = line ? 'unknown-hall' : prepared.source.project.halls[0].id;
    if (state.invalid === 'time') result.view_time_ms = (result.view_time_ms ?? 0) + 1;
    return result;
  }, [prepared, state.camera, state.canvas, state.lod, state.invalid]);
  window.__renderSurface = { set: patch => setState(old => ({ ...old, ...patch })), hold: () => { holding = true; }, release: () => { holding = false; for (const release of releases.splice(0)) release(); }, reject: value => { rejectBytes = value; }, clear: () => { ready.length = 0; failures.length = 0; reads.length = 0; },
    read: () => ({ state, ready, failures, reads, held: releases.length, error, source_key: prepared?.source.key, project: prepared?.source.project, view,
      originalRunsUnchanged: originalRuns === JSON.stringify(frames.map(frame => frame.frame.run)) }),
    capture: () => {
      if (!latest) return null;
      const { scene, camera, renderer } = latest;
      const meshes: { ref: unknown; name: string; position: number[] }[] = [];
      scene.traverse(object => { if (object instanceof THREE.Mesh) meshes.push({ ref: object.userData.ref, name: object.name, position: object.getWorldPosition(new THREE.Vector3()).toArray() }); });
      return { eye: camera.position.toArray(), zoom: (camera as THREE.PerspectiveCamera).zoom, matrix: camera.matrixWorld.toArray(), meshes,
        canvases: document.querySelectorAll('.factory-scene canvas').length, labels: [...document.querySelectorAll('[data-scene-label-key]')].map(label => ({ key: (label as HTMLElement).dataset.sceneLabelKey, disabled: (label as HTMLButtonElement).disabled })) };
    } };
  return <><header><h1>同源厂房只读捕获验证</h1><button onClick={() => setState(old => ({ ...old, camera: 'A' }))}>相机A</button><button onClick={() => setState(old => ({ ...old, camera: 'B' }))}>相机B</button><span>{prepared ? '来源已固定' : '读取精确来源'}</span></header>
    {error && <p role="alert">{error}</p>}
    <div style={{ position: 'relative', width: state.width, height: state.height }}>
      {state.mounted && prepared && view && <FactoryRenderSurface context={state.invalid === 'scope' ? { ...prepared.context, scope_revision: prepared.context.scope_revision + 1 } : state.invalid === 'session' ? { ...prepared.context, session_id: 'another-session' } : prepared.context} source={prepared.source} view={view} onFrameReady={frame => ready.push(frame)} onFailure={failure => failures.push(failure)} />}
    </div></>;
}
const container = document.getElementById('root') as HTMLElement & { surfaceRoot?: ReturnType<typeof createRoot> };
const root = container.surfaceRoot ?? (container.surfaceRoot = createRoot(container)); root.render(<Harness />);
if (import.meta.hot) import.meta.hot.dispose(() => { root.unmount(); if (container.surfaceRoot === root) delete container.surfaceRoot; });
