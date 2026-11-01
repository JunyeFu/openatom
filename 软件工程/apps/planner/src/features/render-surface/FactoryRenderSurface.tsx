import { useEffect, useMemo, useRef } from 'react';
import type { ApiScope } from '../../../../../packages/commands/generated/typescript/ApiScope';
import type { RenderViewSpec } from '../../../../../packages/commands/generated/typescript/RenderViewSpec';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import { FactoryScene } from '../../scene/FactoryScene';
import { factoryContextKey, type FactoryPresentationContext, type FactoryPresentationSource } from '../../services/factoryPresentation';
import type { FactorySceneCapture, FactorySceneFailure, FactorySceneFrame } from './factoryCapture';

export interface FactoryRenderFrame extends FactorySceneFrame {
  source_key: string;
  project_id: string;
  scope: ApiScope;
  scene_revision: number;
  project_revision: number;
  view: RenderViewSpec;
}
export interface FactoryRenderSurfaceProps {
  context: FactoryPresentationContext;
  source: FactoryPresentationSource;
  view: RenderViewSpec;
  target?: ObjectRef | null;
  onFrameReady(frame: FactoryRenderFrame): void;
  onFailure(failure: FactorySceneFailure): void;
}

export function factoryRenderInputIssue(context: FactoryPresentationContext, source: FactoryPresentationSource, view: RenderViewSpec, target: ObjectRef | null = view.object_ref ?? null): string | null {
  if (context.scope.kind !== 'project') return '厂房捕获需要真实Project Scope。';
  if (source.key !== factoryContextKey(context) || context.project_id !== source.project.id) return '捕获来源不属于请求的工程/Scope修订。';
  if (view.kind !== 'line_canvas' && view.kind !== 'multi_hall_canvas' || view.canvas !== view.kind) return '厂房RenderView的kind与canvas必须精确匹配。';
  if (view.project_revision !== source.project.revision) return 'RenderView工程修订与冻结厂房来源不一致。';
  if (target && (view.kind === 'line_canvas' ? target.kind !== 'line' || !source.project.lines.some(line => line.id === target.id && line.hall_id === view.hall_id) : target.kind !== 'project' || target.id !== source.project.id)) return '画布目标必须为同一冻结厂房的明确产线，或同一冻结工程。';
  if (view.object_ref && (!target || view.object_ref.kind !== target.kind || view.object_ref.id !== target.id)) return '画布的两处精确目标不一致。';
  if (view.canvas === 'line_canvas' ? source.project.halls.length ? !view.hall_id || !source.project.halls.some(hall => hall.id === view.hall_id) : view.hall_id != null || target != null : view.hall_id != null) return '流水线画布需明确已有厂房；空工程允许空厂房；多厂房画布不筛选单厂房。';
  if (context.view) {
    if (source.run?.id !== context.view.run_id || view.run_id !== source.run.id || view.view_time_ms !== context.view.at_ms || source.observation?.at_ms !== context.view.at_ms || source.observation.run_id !== source.run.id || source.observation.next_index !== null || source.run.input_ref.project_revision !== source.project.revision || source.run.computed_until_ms == null || context.view.at_ms < source.run.window.start_ms || context.view.at_ms > source.run.computed_until_ms || context.view.at_ms > source.run.window.end_ms) return '捕获Run/观看时点与完整实际冻结来源或已计算前沿不一致。';
  } else if (source.run || source.observation || view.run_id != null || view.view_time_ms != null) return '草稿捕获不得混入另一Run或观看时点。';
  const { camera } = view;
  if (!['meter', 'millimeter'].includes(camera.unit) || !['right_handed_z_up', 'right_handed_y_up'].includes(camera.coordinate_frame) || ![...Object.values(camera.eye), ...Object.values(camera.target), camera.zoom].every(Number.isFinite) || !(camera.zoom > 0) || Object.keys(camera.eye).every(axis => camera.eye[axis as keyof typeof camera.eye] === camera.target[axis as keyof typeof camera.target])) return '捕获相机需要明确单位/坐标系、非重合eye/target及正zoom。';
  return null;
}

/** Keep only the selected line's declared devices and material dependencies. */
export function projectFactoryCaptureSource(source: FactoryPresentationSource, target: ObjectRef | null): FactoryPresentationSource {
    if(target?.kind!=='line')return source;
    const line=source.project.lines.find(line=>line.id===target.id);
    if(!line)return source;
    const ids=new Set([...line.members,...source.project.devices.filter(device=>device.line_id===line.id).map(device=>device.id)]);
    const devices=source.project.devices.filter(device=>ids.has(device.id));
    const locationIds=new Set<string>(),resourceIds=new Set<string>(),connections: typeof source.project.connections=[];
    for(const connection of source.project.connections) {
      if(connection.kind!=='material')continue;
      const {source:from,destination:to,destination_store}=connection.data;
      if(![from,to].some(endpoint=>endpoint.kind==='port'&&ids.has(endpoint.data.instance_id)))continue;
      connections.push(connection);
      for(const endpoint of [from,to])if(endpoint.kind==='location')locationIds.add(endpoint.data);
      locationIds.add(destination_store);
    }
    for(const device of devices) {
      Object.values(device.role_bindings.resources).forEach(id=>resourceIds.add(id));
      Object.values(device.role_bindings.targets).forEach(ref=>{if(ref.kind==='location')locationIds.add(ref.id);});
    }
    for(const route of source.project.routes)for(const step of route.steps)if(ids.has(step.device_id))
      Object.values(step.resource_bindings).forEach(id=>resourceIds.add(id));
    const resources=source.project.resources.filter(resource=>resourceIds.has(resource.id)||!!resource.home_location&&locationIds.has(resource.home_location));
    for(const resource of resources)if(resource.home_location)locationIds.add(resource.home_location);
    const locations=source.project.locations.filter(location=>location.hall_id===line.hall_id&&locationIds.has(location.id));
    const retainedLocations=new Set(locations.map(location=>location.id)),retainedResources=new Set(resources.map(resource=>resource.id));
    return {...source,project:{...source.project,lines:[line],devices,locations,resources,connections},devices:source.devices.filter(device=>ids.has(device.device_id)),
      resources:source.resources?.filter(value=>retainedResources.has(value.resource.id)),
      items:source.items.filter(item=>item.anchor_ref?.kind==='device'&&ids.has(item.anchor_ref.id)||item.anchor_ref?.kind==='location'&&retainedLocations.has(item.anchor_ref.id))};
}

/** The existing product scene with explicit frozen input and capture-only hooks.
 * It never reads the active Workbench, writes preferences, fetches another source
 * or submits image bytes. Native owns task authorization and PNG capture. */
export function FactoryRenderSurface(props: FactoryRenderSurfaceProps) {
  const { context, source, view } = props;
  const target=props.target??view.object_ref??null;
  const projected=useMemo(()=>projectFactoryCaptureSource(source,target),[source,target?.kind,target?.id]);
  const callbacks = useRef(props); callbacks.current = props;
  const key = JSON.stringify([source.key, context.scope_revision, view]);
  const issue = factoryRenderInputIssue(context, source, view,target);
  const active = useRef(''); active.current = issue ? '' : key;
  const capture = useMemo<FactorySceneCapture>(() => ({ key, camera: view.camera,
    onFailure(failure) { if (active.current === key) callbacks.current.onFailure(failure); },
    onFrameReady(frame) {
      if (active.current !== key) return;
      callbacks.current.onFrameReady({ ...frame, source_key: source.key, project_id: source.project.id,
        scope: context.scope, scene_revision: context.scope_revision, project_revision: source.project.revision, view });
    },
  }), [key]);
  useEffect(() => {
    active.current = issue ? '' : key;
    if (issue) callbacks.current.onFailure({ key, stage: 'input', message: issue });
    return () => { if (active.current === key) active.current = ''; };
  }, [key, issue]);
  return <section aria-label="厂房只读捕获表面" data-render-source-key={source.key} style={{ position: 'absolute', inset: 0, overflow: 'hidden' }}>
    {issue ? <p role="alert">{issue}</p> : <FactoryScene project={projected.project} canvas={view.canvas} hallId={view.hall_id ?? null}
      selection={null} onSelect={() => {}} lod={view.lod} autoLod={false} resetKey={0} mediaSource={projected} capture={capture} />}
  </section>;
}
