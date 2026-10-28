import React from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { REVISION } from 'three';
import type { EngineeringStatus } from './generated/EngineeringStatus';
import { Workbench } from './features/factory/Workbench';
import { NativeCaptureProbe, type NativeCapturePlan } from './features/native-capture-probe/NativeCaptureProbe';
import { RenderTaskSurface } from './features/render-surface/RenderTaskSurface';
import { readNativeRenderInput } from './services/renderTask';
import './styles/workbench.css';

const native = '__TAURI_INTERNALS__' in window;
const renderTask = native ? await readNativeRenderInput() : null;
const capturePlan = native && renderTask === null ? await invoke<NativeCapturePlan | null>('native_capture_probe_plan') : null;
createRoot(document.getElementById('root')!).render(renderTask ? <RenderTaskSurface task={renderTask} /> : capturePlan ? <NativeCaptureProbe plan={capturePlan} /> : <Workbench />);

// Private engineering handshake, not one of the frozen 18 business operations.
if (native && renderTask === null) {
  const status = await invoke<EngineeringStatus>('engineering_status');
  await invoke('engineering_frontend_ready', { phase: status.phase, reactVersion: React.version, threeRevision: REVISION });
}
