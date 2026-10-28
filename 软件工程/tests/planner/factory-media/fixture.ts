import type { ModulePackage } from '../../../packages/commands/generated/typescript/ModulePackage';
import type { ProjectState } from '../../../packages/commands/generated/typescript/ProjectState';
import type { ActualMediaFrame } from '../../../packages/presentation/src';
import type { Pose } from '../../../packages/commands/generated/typescript/Pose';
import type { PresentationReadPorts } from '../../../apps/planner/src/services/presentationHost';
import type { FactoryPresentationContext } from '../../../apps/planner/src/services/factoryPresentation';
import inputJson from '../../../scenarios/媒体表现/runtime/calendar.input.json';
import busyJson from '../../../scenarios/媒体表现/runtime/busy.input.json';
import framesJson from '../../../scenarios/媒体表现/runtime/actual-frames.json';
import modelJson from '../../../modules/media/p302-controlled/model.module.json';
import simpleJson from '../../../modules/media/p302-controlled/model-simple.module.json';
import imageJson from '../../../modules/media/p302-controlled/image.module.json';
import gifJson from '../../../modules/media/p302-controlled/gif.module.json';
import type { MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import type { ApiRequest } from '../../../packages/commands/generated/typescript/ApiRequest';
import type { JobState } from '../../../packages/commands/generated/typescript/JobState';
import type { RunResult } from '../../../packages/commands/generated/typescript/RunResult';
import sequentialJson from '../../../scenarios/真实业务设计/runtime-job-history/root_original.actual.json';
import sequentialProject from '../../../scenarios/模板与交接/runtime/source-clock/root_original.project.json';
import sequentialDefinitions from '../../../modules/templates/purchased-assembly/candidate/definitions.json';

export const frames = framesJson as unknown as { label: string; frame: ActualMediaFrame }[];
export const originalRuns = JSON.stringify(frames.map(value => value.frame.run));
export const sequential = sequentialJson as unknown as { job_history: [number, JobState][]; run: RunResult };
export const sequentialOriginal = JSON.stringify(sequential);
// Only captured closed-instant rows are used, without recreating progress or
// running an Engine. 0/40000/80000 are real recorded producer boundaries.
export function sequentialJobs(at: number) {
  if (![0,40000,80000].includes(at)) throw new Error('No captured exact sequential boundary');
  const states = new Map<string, JobState>();
  for (const [when, job] of sequential.job_history) if (when <= at) states.set(job.id, structuredClone(job));
  return [...states.values()];
}
const input = inputJson as unknown as { project: ProjectState; definitions: ModulePackage['definition'][] };
export const pose = (x: number, y: number, z = 0, rz = 0, scale = 1): Pose => ({ unit: 'meter', coordinate_frame: 'right_handed_z_up', translation: { x, y, z }, rotation: { x: 0, y: 0, z: Math.sin(rz / 2), w: Math.cos(rz / 2) }, scale: { x: scale, y: scale, z: scale } });
export const ref = (v: ModulePackage['definition']) => ({ id: v.id, version: v.version });
export function fixture(family = 'calendar') {
  const original = family === 'busy' ? busyJson as unknown as typeof input : input;
  const project = structuredClone(original.project), packages: ModulePackage[] = original.definitions.map(definition => ({ definition: structuredClone(definition), assets: [], files: [], dependencies: [], rights: [], issues: [] }));
  const model = structuredClone(modelJson) as unknown as ModulePackage, simple = structuredClone(simpleJson) as unknown as ModulePackage;
  const image = structuredClone(imageJson) as unknown as ModulePackage, gif = structuredClone(gifJson) as unknown as ModulePackage;
  const device = project.devices[0], pkg = packages.find(value => value.definition.id === device.module_ref.id)!;
  if (pkg.definition.payload.category !== 'devices' || model.definition.payload.category !== 'media') throw new Error('Fixture category');
  pkg.definition.payload.data.presentation = structuredClone(model.definition.payload.data);
  pkg.definition.payload.data.presentation.lod.simplified = { kind: 'media_ref', data: ref(simple.definition) };
  pkg.assets = model.assets; pkg.files = model.files; pkg.rights = model.rights; pkg.dependencies = [ref(simple.definition)];
  project.halls[0].width_m = 10; project.halls[0].depth_m = 9; project.halls[0].origin = pose(10, 20, 0, Math.PI / 2);
  project.lines = [{ id: 'L', title: 'Declared transformed line', hall_id: device.hall_id, transform: pose(2, 3, 0, Math.PI / 2), members: ['CNC', 'B'], boundary_ports: [], region_polygon_m: [{x:-2,y:-2},{x:6,y:-2},{x:6,y:4},{x:-2,y:4}] }];
  device.pose = pose(1, 2); device.line_id = 'L';
  project.devices.push({ ...structuredClone(device), id: 'B', title: 'Second exact instance', pose: pose(4, 2, 0, Math.PI / 2, 1.5) });
  const imagePresentation = image.definition.payload.category === 'media' ? structuredClone(image.definition.payload.data) : null;
  if (imagePresentation) { imagePresentation.source = { kind: 'asset', data: { id: 'root.pixel.red', path: 'red.png', kind: 'png' } }; imagePresentation.lod.detailed = imagePresentation.source; imagePresentation.lod.simplified = imagePresentation.source; imagePresentation.timeline = []; }
  project.devices.push({ ...structuredClone(device), id: 'IMAGE', title: 'Exact image plane', line_id: null, pose: pose(6, 6), height_m: .02,
    footprint_m: [{x:0,y:0},{x:2,y:0},{x:2,y:2},{x:0,y:2}], presentation_override: imagePresentation });
  project.locations = [];
  packages.push(model, simple, image, gif);
  if (family === 'sequential') {
    const frozen = structuredClone(sequentialProject) as unknown as ProjectState;
    const exactPackages: ModulePackage[] = (sequentialDefinitions as unknown as ModulePackage['definition'][]).map(definition => ({ definition: structuredClone(definition), assets: model.assets, files: model.files, dependencies: [ref(simple.definition)], rights: model.rights, issues: [] }));
    for (const value of exactPackages) if (value.definition.payload.category === 'devices') {
      value.definition.payload.data.presentation = structuredClone(model.definition.payload.category === 'media' ? model.definition.payload.data : null);
      // The real legacy operation has no visual phase binding; keep a static
      // model while checking actual execution selection, no invented cycle.
      if (value.definition.payload.data.presentation) value.definition.payload.data.presentation.timeline = [];
    }
    return { project: frozen, packages: [...exactPackages,model,simple,image,gif], pkg: exactPackages[0], model, simple, image, gif };
  }
  return { project, packages, pkg, model, simple, image, gif };
}
export interface ProbeRead { request: MediaAssetReadRequest; state: 'started' | 'held' | 'returned'; bytes?: number }
export function fixturePorts(state: ReturnType<typeof fixture>, context: FactoryPresentationContext,
  readBytes: (request: MediaAssetReadRequest) => Promise<ArrayBuffer>, family = 'calendar') {
  const calls: unknown[] = [];
  const ports: PresentationReadPorts = { async dispatch<T>(request: ApiRequest): Promise<T> {
    calls.push(structuredClone(request));
    const frame = context.view ? frames.find(value => value.frame.snapshot.at_ms === context.view!.at_ms && value.label.startsWith(`${family}@`))?.frame : null;
    let value: unknown;
    if (request.operation === 'get_run') {
      if (family === 'sequential') {
        const query = request.params.query;
        if (query.kind === 'full_result') value = { kind: 'full', data: sequential.run };
        else if (query.kind === 'objects_at') {
          const jobs = query.data.objects.length ? sequentialJobs(query.data.at_ms).filter(job => query.data.objects.some(ref => ref.kind === 'job' && ref.id === job.id)).map(data => ({ kind: 'job' as const, data })) : [];
          const start = query.data.page.after_index ?? 0;
          value = { kind: 'objects', data: { run_id: sequential.run.id, at_ms: query.data.at_ms, objects: jobs.slice(start,start+2), next_index: start+2 < jobs.length ? start+2 : null } };
        } else throw new Error('Unexpected sequential query');
        return value as T;
      }
      if (!frame) throw new Error('No original Engine frame');
      const query = request.params.query;
      if (query.kind === 'full_result') value = { kind: 'full', data: frame.run };
      else if (query.kind === 'objects_at') {
        const objects = frame.snapshot.objects.filter(object => query.data.objects.length ? object.kind === 'job' && query.data.objects.some(ref => ref.id === object.data.id) : object.kind === 'item');
        const start = query.data.page.after_index ?? 0;
        value = { kind: 'objects', data: { run_id: frame.run.id, at_ms: frame.snapshot.at_ms, objects: objects.slice(start, start + 1), next_index: start + 1 < objects.length ? start + 1 : null } };
      } else throw new Error('Unexpected query');
    } else if (request.operation === 'get_object') {
      if (request.params.object_id.kind === 'project') value = { kind: 'project', data: state.project };
      else if (request.params.source.kind === 'module_version' || request.params.source.kind === 'run_module_version') {
        const wanted = request.params.source.data.module_ref;
        const pkg = state.packages.find(pkg => pkg.definition.id === wanted.id && pkg.definition.version === wanted.version);
        if (!pkg) throw new Error(`No exact fixture module ${JSON.stringify(wanted)}`);
        value = { kind: 'module', data: pkg };
      } else throw new Error('Unexpected source');
    } else throw new Error('Read-only fixture forbids commands');
    return value as T;
  }, readAsset: readBytes };
  return { ports, calls };
}
