import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { ProjectContext } from '../../../../../packages/commands/generated/typescript/ProjectContext';
import type { RenderViewSpec } from '../../../../../packages/commands/generated/typescript/RenderViewSpec';
import { createFactoryPresentation, type FactoryPresentationContext, type FactoryPresentationSource } from '../../services/factoryPresentation';
import { FactoryRenderSurface, type FactoryRenderFrame } from '../render-surface/FactoryRenderSurface';
import type { FactorySceneFailure } from '../render-surface/factoryCapture';

export interface NativeCapturePlan { plan_id: string; project: ProjectState; context: ProjectContext; views: RenderViewSpec[] }
interface StepReply { next_step: number | null; ignored: boolean }

/** Private CLI smoke surface. Normal startup never constructs this component. */
export function NativeCaptureProbe({ plan }: { plan: NativeCapturePlan }) {
  const [step, setStep] = useState(0), [source, setSource] = useState<FactoryPresentationSource | null>(null);
  const current = useRef<FactoryPresentationContext | null>(null), active = useRef(step), submitting = useRef(false);
  active.current = step;
  const view = plan.views[step];
  const context = useMemo<FactoryPresentationContext>(() => ({ session_id: plan.context.host.session_id,
    project_id: plan.project.id, scope: { kind: 'project' }, scope_revision: plan.project.revision, project: plan.project,
    view: view.run_id != null && view.view_time_ms != null ? { run_id: view.run_id, at_ms: view.view_time_ms } : null,
  }), [plan, view]);
  const fail = (failure: FactorySceneFailure) => {
    if (active.current !== step || submitting.current) return;
    submitting.current = true;
    void invoke('native_capture_probe_failure', { request: { plan_id: plan.plan_id, step, ...failure } });
  };
  useEffect(() => {
    let live = true; current.current = context; submitting.current = false; setSource(null);
    createFactoryPresentation(() => current.current).read(context).then(value => { if (live) setSource(value); }, error => {
      if (live) fail({ key: '', stage: 'source', message: error instanceof Error ? error.message : String(error) });
    });
    return () => { live = false; current.current = null; };
  }, [context]);
  const ready = (frame: FactoryRenderFrame) => {
    if (active.current !== step || submitting.current) return;
    submitting.current = true;
    void invoke<StepReply>('native_capture_probe_frame', { request: { plan_id: plan.plan_id, step, frame } }).then(reply => {
      if (active.current !== step || reply.ignored) return;
      if (reply.next_step != null) { setSource(null); setStep(reply.next_step); }
    }).catch(error => {
      // Native writes and terminates API/capture failures itself. This displays
      // no replacement image and never resubmits the same frame.
      console.error('Native capture probe failed', error);
    });
  };
  return <main aria-label="同源原生捕获表面" style={{ position: 'fixed', inset: 0, overflow: 'hidden' }}>
    {source && <FactoryRenderSurface key={`${plan.plan_id}:${step}`} context={context} source={source} view={view} onFrameReady={ready} onFailure={fail} />}
  </main>;
}
