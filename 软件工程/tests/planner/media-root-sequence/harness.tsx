/// <reference types="vite/client" />
import { StrictMode, useMemo, useRef, useState } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MediaPresentationView, type MediaPresentationCommonProps } from '../../../apps/planner/src/features/media';
import type { ActualMediaFrame } from '../../../packages/presentation/src';
import type { Presentation } from '../../../packages/commands/generated/typescript/Presentation';
import type { ImageTransition } from '../../../packages/commands/generated/typescript/ImageTransition';
import type { MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import imageJson from '../../../modules/media/p302-controlled/image.module.json';
import framesJson from '../../../scenarios/媒体表现/runtime/actual-frames.json';
import redUrl from './red.png?url';
import blueUrl from './blue.png?url';

const red = { id: 'root-sequence.red', path: 'red.png', kind: 'png' } as const;
const blue = { id: 'root-sequence.blue', path: 'blue.png', kind: 'png' } as const;
const actualFrames = (framesJson as unknown as { label: string; frame: ActualMediaFrame }[])
  .filter(value => ['calendar@0', 'calendar@20000', 'calendar@35000', 'calendar@40000', 'calendar@45000'].includes(value.label));
const originalRuns = JSON.stringify(actualFrames.map(value => value.frame.run));
type State = { mode: 'declared_preview' | 'actual_run'; phase: string; u: number; transition: ImageTransition; sequenceOnly: boolean; frameLabel: string };
type ReadRecord = { request: MediaAssetReadRequest; state: 'started' | 'held' | 'returned'; bytes?: number };
const requests: ReadRecord[] = [], held: (() => void)[] = [];
let holdBlue = false;
const readAsset: MediaPresentationCommonProps['readAsset'] = async request => {
  const exact = (asset: typeof red | typeof blue) => request.asset.id === asset.id && request.asset.path === asset.path && request.asset.kind === asset.kind;
  const url = exact(red) ? redUrl : exact(blue) ? blueUrl : null;
  if (!url) throw new Error('Exact fixture AssetRef is not declared');
  const record: ReadRecord = { request: structuredClone(request), state: 'started' }; requests.push(record);
  const response = await fetch(url); if (!response.ok) throw new Error(`Fixture bytes ${response.status}`);
  const bytes = await response.arrayBuffer(); record.bytes = bytes.byteLength;
  if (exact(blue) && holdBlue) { record.state = 'held'; await new Promise<void>(resolve => held.push(resolve)); }
  record.state = 'returned'; return bytes;
};
declare global { interface Window { __sequence: {
  read(): { state: State; requests: ReadRecord[]; held: number; runsUnchanged: boolean; frame: ActualMediaFrame };
  holdBlue(): void; release(): void;
} } }
function Harness() {
  const [state, setState] = useState<State>({ mode: 'declared_preview', phase: 'auto', u: .25, transition: 'hold', sequenceOnly: false, frameLabel: 'calendar@0' });
  const latest = useRef(state); latest.current = state;
  const frame = actualFrames.find(value => value.label === state.frameLabel)!.frame;
  const presentation = useMemo(() => {
    const value = structuredClone(imageJson.definition.payload.data) as unknown as Presentation;
    value.kind = 'image_sequence'; value.source = { kind: 'image_sequence', data: [red, blue] };
    value.lod.detailed = value.source; value.lod.simplified = value.source;
    value.timeline = state.sequenceOnly ? [] : ['load', 'auto', 'unload'].flatMap(phase_ref => [
      { id: `${phase_ref}-A`, phase_ref, start_progress: 0, end_progress: .5, source: { kind: 'image_asset' as const, data: red }, transition: 'hold' as const },
      { id: `${phase_ref}-B`, phase_ref, start_progress: .5, end_progress: 1, source: { kind: 'image_asset' as const, data: blue }, transition: state.transition },
    ]);
    value.tags = []; value.anchor_roles = {}; value.port_anchors = {};
    return value;
  }, [state.transition, state.sequenceOnly]);
  window.__sequence = {
    read: () => ({ state: latest.current, requests, held: held.length, frame,
      runsUnchanged: JSON.stringify(actualFrames.map(value => value.frame.run)) === originalRuns }),
    holdBlue() { holdBlue = true; },
    release() { holdBlue = false; for (const release of held.splice(0)) release(); },
  };
  const common: MediaPresentationCommonProps = { presentation, readAsset, lod: 'detailed', rights: [red, blue].map(asset => ({
    asset_ref: asset.id, source: 'CC0 original solid pixel probes copied from tests/planner/media/fixtures', license: 'CC0-1.0', redistribution_confirmed: true,
  })), binding: { session_id: 'isolated-root-sequence-browser', request: { project_id: frame.run.input_ref.project_id,
    scope: { kind: 'project' }, scope_revision: 1,
    source: state.mode === 'actual_run' ? { kind: 'run_module_version', data: { run_id: frame.run.id, at_ms: frame.snapshot.at_ms, module_ref: { id: 'root.sequence.probe', version: '1.0.0' } } }
      : { kind: 'module_version', data: { module_ref: { id: 'root.sequence.probe', version: '1.0.0' } } },
  } } };
  return <><h1>Sequence + 显式图片片段组合</h1><p className="probe-note">独占测试页；同一真实共享组件、CC0 PNG实字节。实际模式用原Engine帧，非Host/Native。本页没有生产或库存命令。</p>
    <div className="probe-controls">
      <label>模式<select aria-label="模式" value={state.mode} onChange={event => setState(value => ({ ...value, mode: event.target.value as State['mode'] }))}><option value="declared_preview">声明预览</option><option value="actual_run">原Engine实际帧</option></select></label>
      <label>阶段<select aria-label="阶段" value={state.phase} onChange={event => setState(value => ({ ...value, phase: event.target.value }))}>{['load', 'auto', 'unload'].map(phase => <option key={phase}>{phase}</option>)}</select></label>
      <label>声明u<input aria-label="声明u" type="range" min="0" max="1" step=".001" value={state.u} onChange={event => setState(value => ({ ...value, u: Number(event.target.value) }))} /></label><output>{state.u}</output>
      <label>过渡<select aria-label="过渡" value={state.transition} onChange={event => setState(value => ({ ...value, transition: event.target.value as ImageTransition }))}>{['hold', 'crossfade'].map(value => <option key={value}>{value}</option>)}</select></label>
      <label><input aria-label="纯sequence无显式片段" type="checkbox" checked={state.sequenceOnly} onChange={event => setState(value => ({ ...value, sequenceOnly: event.target.checked }))} />纯sequence无显式片段</label>
      <label>实际帧<select aria-label="实际帧" value={state.frameLabel} onChange={event => setState(value => ({ ...value, frameLabel: event.target.value }))}>{actualFrames.map(value => <option key={value.label}>{value.label}</option>)}</select></label>
    </div>
    {state.mode === 'actual_run' ? <MediaPresentationView {...common} mode="actual_run" frame={frame} />
      : <MediaPresentationView {...common} mode="declared_preview" preview={{ phase_ref: state.phase, view_u: state.u }} onPreviewSeek={preview => setState(value => ({ ...value, u: preview.view_u, phase: preview.phase_ref }))} />}
  </>;
}
// The existing Vite server may execute both canonical and timestamped entries.
// Reuse this harness's React root; keep the real renderer and its effects intact.
const container = document.getElementById('root') as HTMLElement & { sequenceRoot?: Root };
const root = container.sequenceRoot ?? (container.sequenceRoot = createRoot(container));
root.render(<StrictMode><Harness /></StrictMode>);
if (import.meta.hot) import.meta.hot.dispose(() => { root.unmount(); if (container.sequenceRoot === root) delete container.sequenceRoot; });
