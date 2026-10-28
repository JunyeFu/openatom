import { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import * as THREE from 'three';
import type { FactorySceneFrame, FactorySceneFailure } from '../../../apps/planner/src/features/render-surface/factoryCapture';
import { FactoryRenderSurface } from '../../../apps/planner/src/features/render-surface/FactoryRenderSurface';
import { MediaPresentationView } from '../../../apps/planner/src/features/media/MediaPresentationView';
import type { ReadonlyVisualCapture } from '../../../apps/planner/src/features/render-surface/visualCapture';
import { FactoryScene } from '../../../apps/planner/src/scene/FactoryScene';
import { projectFactoryPresentation, type FactoryPresentationContext, type FactoryPresentationSource } from '../../../apps/planner/src/services/factoryPresentation';
import type { ProjectState } from '../../../packages/commands/generated/typescript/ProjectState';
import type { RunResult } from '../../../packages/commands/generated/typescript/RunResult';
import type { RunObjectsPage } from '../../../packages/commands/generated/typescript/RunObjectsPage';
import type { ModulePackage } from '../../../packages/commands/generated/typescript/ModulePackage';
import actual from '../../../scenarios/媒体表现/runtime-l03/browser-input.json';
import '../../../apps/planner/src/styles/workbench.css';

const caseName = new URL(location.href).searchParams.get('case') ?? 'baseline';
const loaded = caseName === 'baseline' ? actual : await (await fetch(new URL('../../../scenarios/媒体表现/runtime-l03/'+(['image','gif'].includes(caseName)?'styles':'variants')+'/'+caseName+'/browser-input.json',import.meta.url))).json();
const data = loaded as unknown as { project: ProjectState; run: RunResult; scope_revision: number; snapshots: RunObjectsPage[]; modules: ModulePackage[]; archive_absolute_path: string };
const original = JSON.stringify(data.run);
let rendered: { scene: THREE.Scene; camera: THREE.Camera; renderer: THREE.WebGLRenderer } | null = null;
const before = THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender = function(renderer, scene, camera, geometry, material, group) {
  rendered = { scene: scene as THREE.Scene, camera, renderer };
  return before.call(this, renderer, scene, camera, geometry, material, group);
};
function Harness() {
  const [state, set] = useState({ at: 43000, lod: 'detailed' as 'detailed' | 'simplified' | 'region', focus: 'line', angle: 'iso', mode: 'scene' });
  const [source, setSource] = useState<FactoryPresentationSource | null>(null), [error, setError] = useState('');
  const publicReady = useRef<{key:string;frame:FactorySceneFrame}|null>(null);
  const detailReady = useRef<{key:string;visible_objects:unknown[]}|null>(null);
  const generation = useRef(0), ready = useRef<FactorySceneFrame|null>(null), failure = useRef<FactorySceneFailure|null>(null);
  const capture = useMemo(() => {
    const xy: Record<string,number[]>={ASM01:[0,0],R01:[1.7,-.9],CV01:[3.3,0],V01:[5.6,0],RW01:[5.6,2.6],CV02:[7.9,0],PK01:[10.3,0],CASE01:[12.7,0],PAL01:[14.9,0]};
    const [sourceX,sourceY]=xy[state.focus]??[7.5,0];const x=sourceX+2,y=sourceY+4;
    const offsets:Record<string,number[]>={iso:[2.5,-3.5,2.5],front:[0,-3.3,1.3],back:[0,3.3,1.3],bottom:[1.8,-2,-.4],tool:[.6,-.8,1.5]};
    const offset=offsets[state.angle];
    return {key:JSON.stringify(state),camera:{unit:'meter' as const,coordinate_frame:'right_handed_z_up' as const,zoom:1,
      eye:state.focus==='line'?{x:10,y:-13,z:13}:{x:x+offset[0],y:y+offset[1],z:offset[2]},target:{x,y,z:.8}},
      onFrameReady(frame:FactorySceneFrame){ready.current=frame;},onFailure(value:FactorySceneFailure){failure.current=value;}};
  },[state]);
  useEffect(() => {
    const ticket = ++generation.current;
    setSource(null); setError('');
    const snapshot = data.snapshots.find(snapshot => snapshot.at_ms === state.at);
    if (!snapshot) { setError('Requested time has no actual Host snapshot'); return; }
    const observation: RunObjectsPage = { ...snapshot, next_index: null, objects: [...snapshot.objects, ...data.project.resources.map(resource => ({ kind: 'resource' as const, data: resource }))] };
    const context: FactoryPresentationContext = { session_id: 'recorded-Host-browser-projection', project_id: data.project.id,
      scope: { kind: 'project' }, scope_revision: data.scope_revision, project: data.project, view: { run_id: data.run.id, at_ms: state.at } };
    const assertCurrent = () => { if (generation.current !== ticket) throw Error('source changed'); };
    projectFactoryPresentation(context, { project: data.project, run: data.run, observation,
      jobs: observation.objects.flatMap(object => object.kind === 'job' ? [object.data] : []),
      items: observation.objects.flatMap(object => object.kind === 'item' ? [object.data] : []), raw: { job_pages: [snapshot], item_pages: [snapshot] } }, {
      async module(ref) {
        const pkg = data.modules.find(pkg => pkg.definition.id === ref.id && pkg.definition.version === ref.version);
        if (!pkg) throw Error('Exact frozen module missing: ' + JSON.stringify(ref));
        return pkg;
      },
      async readAsset(request) {
        assertCurrent();
        if (request.source.kind !== 'run_module_version' || request.source.data.run_id !== data.run.id || request.source.data.at_ms !== state.at) throw Error('Wrong actual frozen source');
        const ref = request.source.data.module_ref;
        const pkg = data.modules.find(pkg => pkg.definition.id === ref.id && pkg.definition.version === ref.version);
        if (!pkg?.assets.some(asset => JSON.stringify(asset) === JSON.stringify(request.asset))) throw Error('AssetRef differs from actual Host exact package');
        const response = await fetch('/@fs/' + encodeURI(data.archive_absolute_path.replaceAll('\\', '/') + '/' + request.asset.path));
        if (!response.ok) throw Error('Archived exact bytes failed: ' + response.status);
        const bytes = await response.arrayBuffer(); assertCurrent(); return bytes;
      },
    }, assertCurrent).then(value => { assertCurrent(); setSource(value); }, reason => { if (generation.current === ticket) setError(String(reason)); });
    return () => { generation.current++; };
  }, [state.at]);
  const detailInput = source?.devices.find(device=>device.device_id===state.focus);
  const detailCapture: ReadonlyVisualCapture = useMemo(()=>{
    const origins:Record<string,number[]>={ASM01:[0,0],R01:[1.7,-.9],CV01:[3.3,0],V01:[5.6,0],RW01:[5.6,2.6],CV02:[7.9,0],PK01:[10.3,0],CASE01:[12.7,0]};
    const [x,y]=origins[state.focus]??[0,0];
    return {key:capture.key,object:{kind:'device',id:state.focus},camera:{...capture.camera,eye:{x:capture.camera.eye.x-x-2,y:capture.camera.eye.y-y-4,z:capture.camera.eye.z},target:{x:0,y:0,z:.8}},
      onFrame(frame){detailReady.current={key:capture.key,...frame};},onFailure(message){failure.current={key:capture.key,stage:'detail',message};}};
  },[capture,state.focus]);
  (window as any).__l03 = { set: (patch: Partial<typeof state>) => set(value => {const next={...value,...patch};return JSON.stringify(next)===JSON.stringify(value)?value:next;}), read() {
    const meshes: unknown[] = [];
    rendered?.scene.updateMatrixWorld(true);
    rendered?.scene.traverse(object => {
      if (object instanceof THREE.Mesh && object.name) meshes.push({ name: object.name, ref: object.userData.ref,
        position: object.getWorldPosition(new THREE.Vector3()).toArray(), size: new THREE.Box3().setFromObject(object).getSize(new THREE.Vector3()).toArray(), scale: object.scale.toArray() });
    });
    return { state, error, ready:state.mode==='public'?(publicReady.current?.key===capture.key?publicReady.current.frame:null):state.mode==='detail'?(detailReady.current?.key===capture.key?detailReady.current:null):(ready.current?.key===capture.key?ready.current:null), failure:failure.current?.key===capture.key?failure.current:null, source: !!source, unchanged: original === JSON.stringify(data.run), canvases: document.querySelectorAll('.factory-scene canvas').length,
      meshes, resources: source?.resources?.map(value=>({id:value.resource.id,status:value.worker_status,unavailable:value.unavailable})), devices: source?.devices.map(device => ({ id: device.device_id, projection: device.projection, unavailable: device.unavailable })),
      items: source?.items.map(item => ({ id: item.item.id, type: item.item.item_type_ref.id, holder: item.item.holder, attributes: item.item.attributes, unavailable: item.unavailable })),
      render: rendered?.renderer.info.render, text: document.body.innerText };
  } };
  const publicContext:FactoryPresentationContext={session_id:'recorded-Host-browser-projection',project_id:data.project.id,scope:{kind:'project'},scope_revision:data.scope_revision,project:data.project,view:{run_id:data.run.id,at_ms:state.at}};
  return <><p>实际保存并由新Host重读的L03 Run；浏览器只读投影检查。{error}</p><div style={{ width: 1500, height: 900 }}>
    {source && source.observation?.at_ms===state.at && state.mode==='public' ? <FactoryRenderSurface context={publicContext} source={source} view={{kind:'line_canvas',canvas:'line_canvas',hall_id:'H01',object_ref:{kind:'line',id:'L03'},project_revision:data.project.revision,run_id:data.run.id,view_time_ms:state.at,lod:state.lod,camera:capture.camera}} onFrameReady={frame=>{publicReady.current={key:capture.key,frame};}} onFailure={value=>{failure.current={...value,key:capture.key};}} /> : source && source.observation?.at_ms===state.at && state.mode==='detail' && detailInput?.media ? <MediaPresentationView {...detailInput.media} lod={state.lod} capture={detailCapture} {...(detailInput.frame?{mode:'actual_run' as const,frame:detailInput.frame}:{mode:'asset_inspection' as const,inspectionLabel:'同一Run精确版本静态结构检查，无活动Job'})} /> : source && source.observation?.at_ms===state.at ? <FactoryScene project={data.project} canvas="line_canvas" hallId="H01" selection={null} onSelect={() => {}} lod={state.lod} autoLod={false}
      resetKey={0} mediaSource={source} capture={capture} run={data.run} observation={source.observation} /> : <p>读取实际来源</p>}
  </div></>;
}
const container = document.getElementById('root') as HTMLElement & { l03Root?: ReturnType<typeof createRoot> };
const root = container.l03Root ?? (container.l03Root=createRoot(container)); root.render(<Harness />);
if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();delete container.l03Root;});
