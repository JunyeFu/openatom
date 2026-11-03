import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { nfr } from '../services/nfrMeasure';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { Pose } from '../../../../packages/commands/generated/typescript/Pose';
import type { Vec2 } from '../../../../packages/commands/generated/typescript/Vec2';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { RunObjectsPage } from '../../../../packages/commands/generated/typescript/RunObjectsPage';
import type { ModulePackage } from '../../../../packages/commands/generated/typescript/ModulePackage';
import { poseMatrix } from './layout';
import { outsideHallFloor } from './layoutEditing';
import { attachLayoutControls, type LayoutControlOptions } from './layoutControls';
import { regionSummary } from './regionSummary';
import { defaultObliqueCamera } from './defaultCamera';
import { ViewportViewCube, type ViewCubeFace, type ViewportViewCubeHandle } from './ViewportViewCube';
import { createAdaptiveViewportGrid, viewportGroundColor, type ViewportScaleValues } from './AdaptiveViewportGrid';
import { ViewportScale, type ViewportScaleHandle } from './ViewportScale';
import { decimalText, deviceActivity } from './runtime';
import { createFactoryMedia, factoryMediaLabels, type FactoryMediaInstance, type FactoryMediaLabel } from './factoryMedia';
import { declaredFactoryWorkers, factoryItemHolder, factoryWorkerLabel, type FactoryPresentationSource, type FactoryDeviceMedia } from '../services/factoryPresentation';
import { createFactoryItemLayer } from './factoryItems';
import { createFactoryHallShell, createFactoryStockSupport, factoryRegionColors } from './factoryEnvironment';
import { factoryDeviceTitle, factoryItemTitle, factoryItemPlace, factoryLifecycleLabel, factoryQualityLabel, factoryUnitLabel } from './factoryLabels';
import { createFactoryResourceLayer } from './factoryResources';
import { createFactoryTransportLayer } from './factoryTransport';
import { createFactoryConnectionLayer, factoryConnectionAt, type FactoryConnectionRow } from './factoryConnections';
import { captureCamera, visibleFactoryObjects, type FactorySceneCapture, type FactorySceneFrame } from '../features/render-surface/factoryCapture';

export type LodMode = 'detailed' | 'simplified' | 'region';
export interface FactorySceneProps {
  project: ProjectState | null;
  canvas: 'line_canvas' | 'multi_hall_canvas';
  hallId: string | null;
  selection: ObjectRef | null;
  onSelect: (ref: ObjectRef | null, additive?: boolean) => void;
  onSelectMany?: (refs: ObjectRef[], additive: boolean) => void;
  selections?: readonly ObjectRef[];
  layoutControl?: LayoutControlOptions;
  onTransform?: (ref: ObjectRef, pose: Pose, base: ProjectState) => Promise<void>;
  lod: LodMode;
  autoLod: boolean;
  resetKey: number;
  run?: RunResult | null;
  observation?: RunObjectsPage | null;
  /** Clear on context changes; actual sources contain their own frozen physical Project. */
  mediaSource?: FactoryPresentationSource | null;
  modules?: ReadonlyMap<string, ModulePackage>;
  /** Dedicated readonly task surface only; ordinary Workbench behavior is unchanged. */
  capture?: FactorySceneCapture;
}

function place(group: THREE.Object3D, pose: Pose) {
  group.matrix.copy(poseMatrix(pose));
  group.matrixAutoUpdate = false;
  group.matrixWorldNeedsUpdate = true;
}

function polygon(points: Vec2[], height: number | null | undefined, color: number) {
  if (points.length < 3) return null;
  const shape = new THREE.Shape(points.map(point => new THREE.Vector2(point.x, point.y)));
  const geometry = height !== null && height !== undefined && height > 0
    ? new THREE.ExtrudeGeometry(shape, { depth: height, bevelEnabled: false })
    : new THREE.ShapeGeometry(shape);
  return new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color, side: THREE.DoubleSide, roughness: 0.85 }));
}

export function FactoryScene({ project: inputProject, canvas, hallId, selection, onSelect, onSelectMany, selections, layoutControl, onTransform, lod, autoLod, resetKey, run: inputRun = null, observation: inputObservation = null, mediaSource = null, modules: inputModules, capture }: FactorySceneProps) {
  const project = mediaSource?.project ?? inputProject;
  const run = mediaSource?.run ?? inputRun;
  const observation = mediaSource?.observation ?? inputObservation;
  const regionActualPending = !!run && observation?.run_id === run.id && !mediaSource;
  const resourceSources = mediaSource?.resources ?? declaredFactoryWorkers(project);
  const modules = mediaSource?.modules ?? inputModules;
  const frameInput = useRef({ mediaSource, observation, regionActualPending, resourceSources });
  frameInput.current = { mediaSource, observation, regionActualPending, resourceSources };
  const updateFrame = useRef<((frame: typeof frameInput.current) => Promise<void>) | null>(null);
  const captureSource = capture ? mediaSource : null;
  const host = useRef<HTMLDivElement>(null);
  const viewCube = useRef<ViewportViewCubeHandle>(null);
  const viewportScale = useRef<ViewportScaleHandle>(null);
  const navigateView = useRef<((face: ViewCubeFace) => void) | null>(null);
  const viewActions = useRef<{ fit: () => void; focus: (ref: ObjectRef) => void; follow: (id: string) => void; stop: () => void } | null>(null);
  const following = useRef<string | null>(null);
  const [followingId, setFollowingId] = useState<string | null>(null);
  const [followMessage, setFollowMessage] = useState('');
  const [showConnections, setShowConnections] = useState(false);
  const select = useRef(onSelect);
  select.current = onSelect;
  const selectMany = useRef(onSelectMany); selectMany.current = onSelectMany;
  const selectionContext = useRef(''); selectionContext.current = JSON.stringify([project?.id, project?.revision, canvas, hallId, !!onSelectMany]);
  const [selectionBox, setSelectionBox] = useState<{ left: number; top: number; width: number; height: number } | null>(null);
  const interaction = useRef({ selection, selections, layoutControl, onTransform, showConnections });
  interaction.current = { selection, selections, layoutControl, onTransform, showConnections };
  const updateInteraction = useRef<(() => void) | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [visibleLod, setVisibleLod] = useState(lod);
  const [focusedDetail, setFocusedDetail] = useState(false);
  const [itemMessages, setItemMessages] = useState<{ key: string | null; values: Record<string, string> }>({ key: null, values: {} });
  const [resourceMessages, setResourceMessages] = useState<{ key: string | null; values: Record<string, string> }>({ key: null, values: {} });
  const [dataLabels, setDataLabels] = useState<{ device_id: string; label: FactoryMediaLabel }[]>([]);
  const [connectionRows, setConnectionRows] = useState<FactoryConnectionRow[]>([]);
  const [labels, setLabels] = useState<{ key: string; title: string; text: string; x: number; y: number; ref: ObjectRef; kind: 'device' | 'item' | 'region' | 'resource' | 'location' }[]>([]);
  const [hiddenLabelKeys, setHiddenLabelKeys] = useState<Set<string>>(() => new Set());
  const [sidePanelOpen, setSidePanelOpen] = useState(false);
  const hiddenLabels = labels.filter(label => hiddenLabelKeys.has(label.key));
  const hasSidePanel = !!mediaSource?.run || resourceSources.length > 0 || hiddenLabels.length > 0 || connectionRows.length > 0;
  const halls = project?.halls.filter(hall => canvas === 'multi_hall_canvas' || hall.id === hallId) ?? [];
  const cameras = useRef(new Map<string, { eye: THREE.Vector3; target: THREE.Vector3; reset: number; changed: boolean }>());
  const [captureFrame, setCaptureFrame] = useState(0);
  const captureLayout = useRef<((frame: number) => void) | null>(null);
  const expectedHiddenLabels = useRef<Set<string>>(new Set());

  useLayoutEffect(() => {
    const node = host.current!;
    const bounds = node.getBoundingClientRect();
    const intersects = (a: DOMRect, b: DOMRect) => a.left < b.right + 3 && a.right + 3 > b.left && a.top < b.bottom + 3 && a.bottom + 3 > b.top;
    const occupied = [...node.querySelectorAll<HTMLElement>('.scene-caption, .scene-view-actions, .scene-help, .viewport-view-cube, .viewport-scale, details[aria-label="实际物品与共享标签"], details[aria-label="区域实际汇总"]')]
      .map(element => element.getBoundingClientRect());
    const elements = new Map([...node.querySelectorAll<HTMLButtonElement>('[data-scene-label-key]')]
      .map(element => [element.dataset.sceneLabelKey!, element.getBoundingClientRect()]));
    const selected = (label: typeof labels[number]) => selection?.kind === label.ref.kind && selection.id === label.ref.id;
    const ordered = [...labels].sort((a, b) => Number(selected(b)) - Number(selected(a)) || Number(a.kind === 'item') - Number(b.kind === 'item'));
    const hidden = new Set<string>();
    for (const label of ordered) {
      const box = elements.get(label.key)!;
      if (box.left < bounds.left || box.right > bounds.right || box.top < bounds.top || box.bottom > bounds.bottom || occupied.some(other => intersects(box, other))) hidden.add(label.key);
      else occupied.push(box);
    }
    expectedHiddenLabels.current = hidden;
    setHiddenLabelKeys(previous => previous.size === hidden.size && [...hidden].every(key => previous.has(key)) ? previous : hidden);
  }, [labels, selection?.kind, selection?.id, sidePanelOpen, hasSidePanel]);

  useLayoutEffect(() => {
    if (!capture) return;
    const expected = expectedHiddenLabels.current;
    if (expected.size === hiddenLabelKeys.size && [...expected].every(key => hiddenLabelKeys.has(key))) captureLayout.current?.(captureFrame);
  }, [capture, captureFrame, labels, hiddenLabelKeys]);

  const viewportRenderer = useRef<THREE.WebGLRenderer | null>(null);
  // A source change can invalidate dozens of device loads before their GLBs finish decoding.
  const mediaWorkers = useRef(Array.from({length: 4}, () => Promise.resolve()));

  useEffect(() => () => {
    const renderer = viewportRenderer.current;
    renderer?.dispose();
    renderer?.forceContextLoss();
    renderer?.domElement.remove();
    viewportRenderer.current = null;
  }, []);

  useEffect(() => {
    let { mediaSource, observation, regionActualPending, resourceSources } = frameInput.current;
    const node = host.current!;
    let renderer: THREE.WebGLRenderer;
    try { renderer = viewportRenderer.current ??= new THREE.WebGLRenderer({ antialias: true }); }
    catch (reason) {
      setError('三维画布无法初始化，请检查图形环境。工程数据未改变。');
      capture?.onFailure({ key: capture.key, stage: 'initialize', message: reason instanceof Error ? reason.message : String(reason) });
      return;
    }
    setError(null);
    renderer.setPixelRatio(capture ? window.devicePixelRatio : Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(viewportGroundColor);
    node.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(42, 1, 0.01, 100000);
    Object.assign(renderer.domElement, {
      readFactoryDiagnostics: () => {
        const pixels = renderer.getDrawingBufferSize(new THREE.Vector2());
        return {
          project_id: project?.id ?? null, project_revision: project?.revision ?? null,
          run_id: run?.id ?? null, observation_ms: observation?.at_ms ?? null,
          execution_source: mediaSource?.execution_source ?? null,
          pixels: { width: pixels.x, height: pixels.y, css_width: node.clientWidth, css_height: node.clientHeight, pixel_ratio: renderer.getPixelRatio() },
          lod: lastLod,
          render: { ...renderer.info.render }, memory: { ...renderer.info.memory },
          asset_loading: {
            pending_devices: pendingMedia,
            devices: mediaSlots.map(slot => ({ id: slot.device.id, lod: slot.lod, loaded: !!slot.instance, visible: slot.group.visible, title: slot.annotation.title,
              phase: slot.instance?.root.userData.mqSource?.phase ?? null, progress: slot.instance?.root.userData.mqSource?.progress ?? null,
              effective_ms: slot.instance?.root.userData.mqSource?.effective_ms ?? null, duration_ms: slot.instance?.root.userData.mqSource?.duration_ms ?? null })),
            items: (() => { const state = itemLayer?.renderState(pendingMedia === 0); return state && { pending: state.pending, failures: state.failures, unavailable: state.unavailable }; })(),
            waiting_items: Object.entries(itemLayer?.statuses() ?? {}).filter(([, status]) => /读取|尚未/.test(status)),
          },
          capture_readiness: capture ? { fonts_ready: fontsReady, size_applied: sizeApplied, submitted, gpu_ready: gpuReady, dom_ready: domReady, delivered, pending_render_frame: mediaRenderFrame } : null,
          camera: { eye: camera.position.toArray(), target: controls.target.toArray(), zoom: camera.zoom },
          following_item: following.current,
          item_bounds: Object.fromEntries((mediaSource?.items ?? []).flatMap(({ item }) => { const bounds = itemLayer?.bounds(item.id); return bounds ? [[item.id, { min: bounds.min.toArray(), max: bounds.max.toArray() }]] : []; })),
          viewport: grid.state, scale: scaleState,
          render_scope: 'last_renderer_submission', visibility_scope: 'geometry_frustum_without_occlusion',
          visible_objects: visibleFactoryObjects(scene, camera),
        };
      },
    });
    const controls = new OrbitControls(camera, renderer.domElement);
    if (capture) controls.enabled = false;
    controls.enableDamping = false;
    controls.mouseButtons = { MIDDLE: THREE.MOUSE.PAN, RIGHT: THREE.MOUSE.ROTATE };
    controls.minDistance = 0.1;
    scene.add(new THREE.HemisphereLight(0xffffff, 0x8a9db1, 2.8));
    const light = new THREE.DirectionalLight(0xffffff, 2.1);
    light.position.set(12, 20, 8); scene.add(light);
    // The complete physical hierarchy is Z-up. Convert once at its root.
    const physical = new THREE.Group(); physical.rotation.x = -Math.PI / 2; scene.add(physical);
    const selectable: THREE.Object3D[] = [];
    const deviceObjects: THREE.Object3D[] = [];
    const groupedObjects: THREE.Object3D[] = [];
    let live = true;
    let captureFailed = false, fontsReady = false, sizeApplied = false, pendingMedia = 0;
    let submitted = 0, gpuReady = 0, domReady = 0, delivered = 0, gpuWait = 0;
    let gpuFence: WebGLSync | null = null;
    const gl = renderer.getContext();
    const gpu = gl instanceof WebGL2RenderingContext ? gl : null;
    let submittedFrame: FactorySceneFrame | null = null;
    const fail = (stage: string, message: string, object_ref?: ObjectRef) => {
      if (!capture || !live || captureFailed) return;
      captureFailed = true; setError(message); capture.onFailure({ key: capture.key, stage, message, object_ref });
    };
    let itemLayer: ReturnType<typeof createFactoryItemLayer> | null = null;
    let resourceLayer: ReturnType<typeof createFactoryResourceLayer> | null = null;
    let layoutHandle: ReturnType<typeof attachLayoutControls> | null = null;
    (renderer.domElement as HTMLCanvasElement & { readLayoutDiagnostics?: () => unknown }).readLayoutDiagnostics =
      () => ({ object_ref: interaction.current.selection, handle: layoutHandle?.diagnostics() ?? null });
    const media = new Map(mediaSource?.devices.map(value => [value.device_id, value]) ?? []);
    const mediaSlots: { group: THREE.Group; device: NonNullable<typeof project>['devices'][number]; input: FactoryDeviceMedia | undefined;
      marker: THREE.Object3D; instance: FactoryMediaInstance | null; lod: LodMode | null; detailDistance: number; ticket: number; baseTitle: string; annotation: typeof annotations[number]; grouped: boolean }[] = [];
    const anchors = new Map<string, THREE.Object3D>();
    const selectionMarkers: { ref: ObjectRef; material: { color: THREE.Color }; baseColor: number; selectedColor: number; additive: boolean }[] = [];
    const annotations: { key: string; title: string; anchor: THREE.Object3D; ref: ObjectRef; kind: 'device' | 'item' | 'region' | 'resource' | 'location' }[] = [];
    let actual = run && observation?.run_id === run.id && observation.next_index === null &&
      observation.at_ms >= run.window.start_ms && run.computed_until_ms != null && observation.at_ms <= run.computed_until_ms ? observation : null;
    for (const hall of halls) {
      const root = new THREE.Group();
      if (canvas === 'multi_hall_canvas') place(root, hall.origin);
      physical.add(root);
      anchors.set('hall:' + hall.id, root);
      root.add(createFactoryHallShell(hall));
      const floor: THREE.Mesh<THREE.BufferGeometry, THREE.Material> | null = polygon([{ x: 0, y: 0 }, { x: hall.width_m, y: 0 }, { x: hall.width_m, y: hall.depth_m }, { x: 0, y: hall.depth_m }], null, viewportGroundColor);
      if (floor) {
        floor.material.dispose(); floor.material = new THREE.MeshBasicMaterial({ color: 0xe8edef, side: THREE.DoubleSide });
        floor.position.z = -0.01; floor.userData.ref = { kind: 'hall', id: hall.id };
        root.add(floor); selectable.push(floor);
        const edge = new THREE.LineSegments(new THREE.EdgesGeometry(floor.geometry), new THREE.LineBasicMaterial({ color: 0x8a9eb3 }));
        root.add(edge);
      }
      const lines = new Map<string, THREE.Group>();
      for (const [lineIndex, line] of project!.lines.filter(value => value.hall_id === hall.id).entries()) {
        const group = new THREE.Group(); place(group, line.transform); root.add(group); lines.set(line.id, group);
        anchors.set('line:' + line.id, group);
        const area: THREE.Mesh<THREE.BufferGeometry, THREE.Material> | null = polygon(line.region_polygon_m, null, factoryRegionColors[lineIndex % factoryRegionColors.length]);
        if (area) {
          area.material.dispose(); area.material = new THREE.MeshBasicMaterial({ color: factoryRegionColors[lineIndex % factoryRegionColors.length], side: THREE.DoubleSide });
          area.position.z = 0.005; area.userData.ref = { kind: 'line', id: line.id };
          group.add(area); selectable.push(area);
          const state = regionSummary(project!, line, mediaSource, regionActualPending);
          const labelAnchor = new THREE.Object3D(); area.geometry.computeBoundingBox();
          area.geometry.boundingBox!.getCenter(labelAnchor.position);
          labelAnchor.position.y = area.geometry.boundingBox!.max.y - .6; group.add(labelAnchor);
          annotations.push({ key: 'region:' + line.id, title: line.title + ' · ' + state.members.length + '台 · ' + state.state,
            anchor: labelAnchor, ref: { kind: 'line', id: line.id }, kind: 'region' });
        }
      }
      for (const device of project!.devices.filter(value => value.hall_id === hall.id)) {
        const group = new THREE.Group(); place(group, device.pose);
        const parent = device.line_id ? lines.get(device.line_id) : root;
        // Missing layout parent is unknown; never render it at a fabricated position.
        if (!parent) continue;
        parent.add(group);
        anchors.set(`device:${device.id}`, group);
        const selected = selection?.kind === 'device' && selection.id === device.id || selections?.some(ref => ref.kind === 'device' && ref.id === device.id);
        const outside = outsideHallFloor(project!, device) === true;
        const activity = actual && run ? deviceActivity(run, actual.at_ms, device.id) : null;
        // Footprints are flat layout markers, never fabricated detailed bodies.
        const footprint = polygon(device.footprint_m, null, outside ? 0xc84b4b : selected ? 0x287bcd : activity?.work ? 0x468c80 : activity?.waits ? 0xba9464 : 0x839bad);
        const marker = footprint ?? new THREE.Points(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3()]), new THREE.PointsMaterial({ color: selected ? 0x287bcd : 0x46647b, size: 7, sizeAttenuation: false }));
        marker.position.z = .01; marker.userData.ref = { kind: 'device', id: device.id };
        if (footprint) { const material = footprint.material as THREE.MeshStandardMaterial; material.transparent = true; material.opacity = .3; }
        selectionMarkers.push({ ref: { kind: 'device', id: device.id }, material: marker.material as THREE.MeshStandardMaterial | THREE.PointsMaterial,
          baseColor: footprint ? outside ? 0xc84b4b : activity?.work ? 0x468c80 : activity?.waits ? 0xba9464 : 0x839bad : 0x46647b,
          selectedColor: footprint && outside ? 0xc84b4b : 0x287bcd, additive: true });
        group.add(marker); selectable.push(group);
        group.userData.ref = { kind: 'device', id: device.id };
        const grouped = !!device.line_id && project!.lines.some(line => line.id === device.line_id && line.region_polygon_m.length >= 3);
        deviceObjects.push(group); if (grouped) groupedObjects.push(group);
        const baseTitle = `${device.title}${outside ? ' · 占地超出厂房范围' : ''}${footprint ? '' : ' · 占地未声明（定位标记）'}${activity?.work ? ` · 执行记录 ${activity.work}` : activity?.waits ? ` · 等待记录 ${activity.waits}` : ''}`;
        const annotation = { key: `device:${device.id}`, title: baseTitle, anchor: group, ref: { kind: 'device' as const, id: device.id }, kind: 'device' as const };
        annotations.push(annotation);
        mediaSlots.push({ group, device, input: media.get(device.id), marker, instance: null, lod: null, detailDistance: device.footprint_m.length ? Math.max(6, 3 * Math.hypot(
          Math.max(...device.footprint_m.map(p => p.x)) - Math.min(...device.footprint_m.map(p => p.x)),
          Math.max(...device.footprint_m.map(p => p.y)) - Math.min(...device.footprint_m.map(p => p.y)))) : 6, ticket: 0, baseTitle, annotation, grouped });
      }
      for (const location of project!.locations.filter(value => value.hall_id === hall.id)) {
        const selected = selection?.kind === 'location' && selection.id === location.id;
        const marker = new THREE.Mesh(new THREE.RingGeometry(0.19, 0.24, 24), new THREE.MeshBasicMaterial({ color: selected ? 0x287bcd : 0x447b98, side: THREE.DoubleSide }));
        place(marker, location.position); marker.userData.ref = { kind: 'location', id: location.id };
        const support = createFactoryStockSupport(project!, location);
        if (support) marker.add(support);
        selectionMarkers.push({ ref: marker.userData.ref, material: marker.material, baseColor: 0x447b98, selectedColor: 0x287bcd, additive: false });
        anchors.set(`location:${location.id}`, marker);
        root.add(marker); selectable.push(marker);
        annotations.push({ key: `location:${location.id}`, title: `库位 ${location.title} · ${location.id}`, anchor: marker, ref: { kind: 'location', id: location.id }, kind: 'location' });
      }
    }
    const transportLayer = createFactoryTransportLayer(mediaSource?.transports ?? [], halls, canvas === 'multi_hall_canvas');
    physical.add(transportLayer.root); selectable.push(transportLayer.root);
    const refreshObservedAnnotations = () => {
      for (let index = annotations.length - 1; index >= 0; index--) if (annotations[index].kind === 'item' || annotations[index].kind === 'resource') annotations.splice(index, 1);
      for (const slot of mediaSlots) {
        const activity = actual && run ? deviceActivity(run, actual.at_ms, slot.device.id) : null;
        const outside = outsideHallFloor(project!, slot.device) === true, footprint = slot.device.footprint_m.length >= 3;
        slot.baseTitle = `${slot.device.title}${outside ? ' · 占地超出厂房范围' : ''}${footprint ? '' : ' · 占地未声明（定位标记）'}${activity?.work ? ` · 执行记录 ${activity.work}` : activity?.waits ? ` · 等待记录 ${activity.waits}` : ''}`;
        if (slot.instance) slot.annotation.title = `${slot.baseTitle} · ${slot.input?.observed_phase?.label ?? (slot.input?.projection === 'terminal' ? '实际结束姿态' : slot.input?.frame ? '实际阶段' : mediaSource?.run ? '静态素材（无活动执行）' : '静态素材')}`;
        const marker = selectionMarkers.find(marker => marker.ref.kind === 'device' && marker.ref.id === slot.device.id);
        if (marker) {
          marker.baseColor = footprint ? outside ? 0xc84b4b : activity?.work ? 0x468c80 : activity?.waits ? 0xba9464 : 0x839bad : 0x46647b;
          const input = interaction.current;
          const selected = input.selection?.kind === 'device' && input.selection.id === slot.device.id
            || input.selections?.some(ref => ref.kind === 'device' && ref.id === slot.device.id);
          marker.material.color.setHex(selected ? marker.selectedColor : marker.baseColor);
        }
      }
      for (const line of project?.lines ?? []) {
        const annotation = annotations.find(value => value.kind === 'region' && value.ref.id === line.id);
        if (annotation) { const state = regionSummary(project!, line, mediaSource, regionActualPending); annotation.title = line.title + ' · ' + state.members.length + '台 · ' + state.state; }
      }
      for (const input of resourceSources) {
        const anchor = input.resource.home_location ? anchors.get(`location:${input.resource.home_location}`) : null;
        if (input.worker_status && anchor) annotations.push({ key: `resource:${input.resource.id}`, title: factoryWorkerLabel(input),
          anchor, ref: { kind: 'resource', id: input.resource.id }, kind: 'resource' });
      }
      if (actual && run || mediaSource?.execution_source) {
        const items = actual ? actual.objects.flatMap(object => object.kind === 'item' ? [object.data] : []) : mediaSource!.items.map(input => input.item);
        const jobs = actual?.objects.flatMap(object => object.kind === 'job' ? [object.data] : []) ?? [];
        for (const item of items) {
          // Consumed composition records and contained children remain selectable
          // in the shared object list; they are not painted as extra loose stock.
          if (['transformed', 'scrapped'].includes(item.lifecycle) || item.holder.kind === 'container') continue;
          const holder = mediaSource?.items.find(input => input.item.id === item.id)?.attachment?.owner_ref ?? factoryItemHolder(item, items, jobs);
          const anchor = holder ? anchors.get(`${holder.kind}:${holder.id}`) : null;
          if (!anchor) continue;
          annotations.push({ key: `item:${item.id}`, title: `${factoryItemTitle(project, item)} · ${decimalText(item.quantity.value)}${factoryUnitLabel(item.quantity.unit)} · ${factoryLifecycleLabel[item.lifecycle]} · ${factoryQualityLabel[item.quality]}`, anchor, ref: { kind: 'item', id: item.id }, kind: 'item' });
        }
      }
    };
    refreshObservedAnnotations();
    const connections = project ? createFactoryConnectionLayer(project, modules, physical, anchors, selection) : null;
    setConnectionRows(connections?.rows ?? []);
    if (connections) { physical.add(connections.root); if (connections.drawn) selectable.push(connections.root); }
    const bounds = new THREE.Box3().setFromObject(physical);
    const declaredBounds = bounds.clone();
    for (const device of project?.devices ?? []) {
      const object = anchors.get(`device:${device.id}`);
      if (!object || device.height_m == null) continue;
      for (const point of device.footprint_m) declaredBounds.expandByPoint(new THREE.Vector3(point.x, point.y, device.height_m).applyMatrix4(object.matrixWorld));
    }
    const size = bounds.isEmpty() ? 20 : bounds.getSize(new THREE.Vector3()).length();
    const center = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
    camera.position.copy(center).add(new THREE.Vector3(0.8, 1, 1).multiplyScalar(Math.max(size, 1)));
    controls.target.copy(center); controls.update();
    const cameraKey = `${project?.id ?? 'empty'}:${canvas}:${hallId ?? ''}`;
    const savedCamera = cameras.current.get(cameraKey);
    let userCameraChanged = !!(savedCamera?.reset === resetKey && savedCamera.changed);
    let followOffset: THREE.Vector3 | null = null;
    following.current = null; setFollowingId(null); setFollowMessage('');
    const stopFollowing = () => { following.current = null; followOffset = null; setFollowingId(null); };
    const followCurrent = () => {
      const id = following.current; if (!id) return;
      const input = mediaSource?.items.find(value => value.item.id === id);
      if (!input || ['transformed', 'scrapped'].includes(input.item.lifecycle)) {
        stopFollowing(); setFollowMessage(input ? '此物品已结束使用，跟随已停止。可选择后续产出继续查看。' : '此时点尚无该物品，跟随已停止。'); return;
      }
      const bounds = itemLayer?.bounds(id);
      if (!bounds) { setFollowMessage('正在读取此物品的空间位置'); return; }
      const target = bounds.getCenter(new THREE.Vector3());
      if (!followOffset) followOffset = new THREE.Vector3(.8, .7, 1).normalize().multiplyScalar(Math.max(1, bounds.getSize(new THREE.Vector3()).length() * 2.3));
      userCameraChanged = true;
      camera.position.copy(target).add(followOffset); controls.target.copy(target); controls.update();
      setFollowMessage('跟随 ' + factoryItemTitle(project, input.item));
    };
    const refitDefault = () => {
      if (capture || userCameraChanged || declaredBounds.isEmpty()) return;
      const fit = defaultObliqueCamera(new THREE.Box3().setFromObject(physical).union(declaredBounds), camera.fov, camera.aspect);
      camera.position.copy(fit.eye); controls.target.copy(fit.target); controls.update();
    };
    // Asset completions arrive in bursts; fit bounds and redraw once per frame.
    let mediaRenderFrame = 0;
    const mediaChanged = () => {
      if (!live || mediaRenderFrame) return;
      mediaRenderFrame = requestAnimationFrame(() => {
        mediaRenderFrame = 0;
        if (!live) return;
        refitDefault(); followCurrent(); render();
      });
    };
    controls.addEventListener('start', () => { userCameraChanged = true; });
    if (capture) {
      const requested = captureCamera(capture.camera);
      camera.position.copy(requested.eye); controls.target.copy(requested.target); camera.zoom = requested.zoom;
      camera.lookAt(requested.target); camera.updateProjectionMatrix();
    } else if (savedCamera?.reset === resetKey && savedCamera.changed) { camera.position.copy(savedCamera.eye); controls.target.copy(savedCamera.target); controls.update(); }
    else refitDefault();
    // Keep the engineering ground outside physical bounds, picking and layout transforms.
    const grid = createAdaptiveViewportGrid(); grid.mesh.position.y = -.02; scene.add(grid.mesh);
    let scaleState: ViewportScaleValues | null = null;
    let lastLod: LodMode | null = null;
    function loadDevice(device: NonNullable<typeof project>['devices'][number], input: FactoryDeviceMedia, level: LodMode, current: () => boolean) {
      const worker = mediaWorkers.current.shift()!;
      const result = worker.then(() => current() ? createFactoryMedia(device, input, level) : null);
      mediaWorkers.current.push(result.then(() => undefined, () => undefined));
      return result;
    }
    const loadLod = (current: LodMode, layersChanged: boolean) => {
      if (layersChanged) {
        itemLayer?.dispose(); itemLayer = null;
        resourceLayer?.dispose(); resourceLayer = null;
      }
      for (const slot of mediaSlots) {
        const selected = interaction.current.selection;
        const deviceLod: LodMode = autoLod && !capture && current !== 'region' && selected?.kind === 'device' && selected.id === slot.device.id
          && camera.position.distanceTo(slot.group.getWorldPosition(new THREE.Vector3())) < slot.detailDistance ? 'detailed' : current;
        if (slot.lod === deviceLod) continue;
        slot.lod = deviceLod;
        const ticket = ++slot.ticket;
        if (slot.instance) { slot.group.remove(slot.instance.root); slot.instance.dispose(); slot.instance = null; }
        slot.marker.visible = true;
        slot.annotation.title = `${slot.baseTitle} · ${current === 'region' ? '区域定位' : regionActualPending && !slot.input ? '正在读取此时点表现来源…' : slot.input?.unavailable ?? (slot.input?.media ? '读取素材中' : '表现素材未连接')}`;
        if (current === 'region' || !slot.input?.media) continue;
        pendingMedia++;
        loadDevice(slot.device, slot.input, deviceLod, () => live && ticket === slot.ticket).then(instance => {
          pendingMedia--;
          if (!instance) return;
          if (!live || ticket !== slot.ticket) { instance.dispose(); return; }
          slot.instance = instance;
          instance.root.traverse(object => { object.userData.ref = { kind: 'device', id: slot.device.id }; });
          slot.group.add(instance.root); slot.marker.visible = outsideHallFloor(project!, slot.device) === true;
          slot.annotation.title = `${slot.baseTitle} · ${slot.input?.observed_phase?.label ?? (slot.input?.projection === 'terminal' ? '实际结束姿态' : slot.input?.frame ? '实际阶段' : mediaSource?.run ? '静态素材（无活动执行）' : '静态素材')}`;
          itemLayer?.reconcile();
          mediaChanged();
        }, reason => {
          pendingMedia--;
          if (!live || ticket !== slot.ticket) return;
          slot.annotation.title = `${slot.baseTitle} · ${reason instanceof Error ? reason.message : String(reason)}`;
          fail('device_resource', reason instanceof Error ? reason.message : String(reason), { kind: 'device', id: slot.device.id });
          mediaChanged();
        });
      }
      setFocusedDetail(autoLod && !capture && mediaSlots.some(slot => slot.lod === 'detailed'));
      if (!layersChanged) return;
      itemLayer = createFactoryItemLayer(mediaSource?.items ?? [], current,
        id => mediaSlots.find(slot => slot.device.id === id)?.instance ?? null,
        id => anchors.get(`location:${id}`) ?? null, mediaChanged, transportLayer.anchor);
      itemLayer.reconcile();
      resourceLayer = createFactoryResourceLayer(resourceSources, current,
        id => anchors.get(`location:${id}`) ?? null, mediaChanged);
      resourceLayer.reconcile();
    };
    const notifyReady = () => {
      if (!capture || !live || captureFailed || !fontsReady || !sizeApplied || pendingMedia || mediaRenderFrame || !submittedFrame || gpuReady !== submitted || domReady !== submitted || delivered === submitted) return;
      const items = itemLayer?.renderState(true);
      const resources = resourceLayer?.renderState();
      if (items?.failures.length) { const issue = items.failures[0]; fail('item_resource', issue.message, issue.ref); return; }
      if (resources?.failures.length) { const issue = resources.failures[0]; fail('resource_media', issue.message, issue.ref); return; }
      if (items?.pending || resources?.pending) return;
      delivered = submitted; capture.onFrameReady(submittedFrame);
    };
    if (capture) {
      captureLayout.current = frame => { domReady = frame; notifyReady(); };
      document.fonts.ready.then(() => { if (live) { fontsReady = true; render(); } }, reason => fail('fonts', String(reason)));
      renderer.debug.onShaderError = (_gl, program) => fail('shader', gl.getProgramInfoLog(program) ?? 'GPU shader link failed');
    }
    const render = () => {
      if (!live) return;
      // Keep millimetre-scale depth useful for close views and full-sized halls.
      camera.near = Math.max(.01, Math.min(.5, camera.position.distanceTo(controls.target) / 100));
      camera.updateProjectionMatrix();
      const ratio = camera.position.distanceTo(controls.target) / Math.max(size, 1);
      const current = autoLod && !capture ? (ratio < 2.5 || following.current ? 'simplified' : 'region') : lod;
      const layersChanged = current !== lastLod;
      if (layersChanged) {
        lastLod = current; setVisibleLod(current);
        // Regions require a real declared polygon; ungrouped devices stay visible.
        deviceObjects.forEach(value => { value.visible = true; });
        if (current === 'region') groupedObjects.forEach(value => { value.visible = false; });
      }
      loadLod(current, layersChanged);
      // Report captures only consume the complete frame. Avoid submitting the
      // full factory repeatedly while its device and item assets are arriving.
      if (capture && (!sizeApplied || !fontsReady || pendingMedia || itemLayer?.renderState(true).pending || resourceLayer?.renderState().pending)) return;
      viewCube.current?.update(camera.quaternion);
      if (node.clientHeight > 0) {
        const viewport = grid.update(camera, controls.target, node.clientHeight);
        scaleState = viewportScale.current?.update(viewport.metersPerPixel) ?? null;
      }
      try { renderer.render(scene, camera); }
      catch (reason) { if (!capture) throw reason; fail('gpu_submit', reason instanceof Error ? reason.message : String(reason)); return; }
      if (nfr.enabled) nfr.semantic('scene_submit', { project_id: project?.id ?? null, revision: project?.revision ?? null, run_id: run?.id ?? null, pending_media: pendingMedia, lod: current, canvas, device_count: project?.devices.length ?? 0 });
      setItemMessages({ key: mediaSource?.key ?? null, values: itemLayer?.statuses() ?? {} });
      setResourceMessages({ key: mediaSource?.key ?? null, values: resourceLayer?.statuses() ?? {} });
      const sharedLabels = mediaSlots.flatMap(slot => slot.input && (slot.instance?.presentation ?? slot.input.media?.presentation)
        ? factoryMediaLabels((slot.instance?.presentation ?? slot.input.media!.presentation), slot.input, mediaSource ?? undefined).map(label => ({ device_id: slot.device.id, label })) : []);
      setDataLabels(sharedLabels);
      const tagged = sharedLabels.flatMap(({ device_id, label }) => {
        const slot = mediaSlots.find(value => value.device.id === device_id), anchor = label.anchor.kind === 'scene_node' ? slot?.instance?.nodes.get(label.anchor.data) : null;
        return anchor && slot?.group.visible ? [{ key: `tag:${device_id}:${label.id}`, title: `${label.label} · ${label.value}`, anchor, ref: { kind: 'device' as const, id: device_id }, kind: 'device' as const }] : [];
      });
      const offsets = new Map<THREE.Object3D, number>();
      setLabels([...annotations, ...tagged].flatMap(annotation => {
        const selected = interaction.current.selection?.kind === annotation.ref.kind && interaction.current.selection.id === annotation.ref.id;
        const wideView = camera.position.distanceTo(controls.target) > 35;
        if (annotation.kind === 'location' && !selected) return [];
        if (annotation.kind === 'region' && !wideView && current !== 'region') return [];
        if ((annotation.kind === 'item' || annotation.kind === 'device') && wideView && !selected) return [];
        if (annotation.kind === 'item' && following.current && !selected && annotation.ref.id !== following.current) return [];
        if (annotation.kind === 'resource' && current === 'region') return [];
        if (current === 'region' && annotation.kind === 'device' && mediaSlots.some(slot => slot.group === annotation.anchor && slot.grouped)) return [];
        const itemBounds = annotation.kind === 'item' ? itemLayer?.bounds(annotation.ref.id) : null;
        const worldPoint = itemBounds ? itemBounds.getCenter(new THREE.Vector3()) : annotation.anchor.getWorldPosition(new THREE.Vector3());
        if (itemBounds) worldPoint.y = itemBounds.max.y;
        if (!wideView && !selected && annotation.kind !== 'region' && worldPoint.distanceTo(controls.target) > Math.max(3, camera.position.distanceTo(controls.target))) return [];
        const point = worldPoint.project(camera);
        if (![point.x, point.y, point.z].every(Number.isFinite) || point.z < -1 || point.z > 1 || Math.abs(point.x) > 1 || Math.abs(point.y) > 1) return [];
        const offset = offsets.get(annotation.anchor) ?? 0;
        // Co-located item labels share one position; collision handling keeps the selected identity visible.
        if (annotation.kind !== 'item') offsets.set(annotation.anchor, offset + 1);
        const text = annotation.kind === 'device' ? factoryDeviceTitle(project!.devices.find(device => device.id === annotation.ref.id)!.title)
          : annotation.kind === 'region' ? annotation.title.split(' · ').slice(0, 3).join(' · ') : annotation.title;
        return [{ ...annotation, text, x: (point.x + 1) / 2 * node.clientWidth, y: (1 - point.y) / 2 * node.clientHeight - 22 - offset * 22 }];
      }));
      if (capture && !captureFailed && sizeApplied) {
        if (!gpu) { fail('gpu_sync', '当前图形环境不支持实际帧完成栅栏。'); return; }
        const items = itemLayer?.renderState(pendingMedia === 0);
        const resources = resourceLayer?.renderState();
        if (items?.failures.length) { const issue = items.failures[0]; fail('item_resource', issue.message, issue.ref); return; }
        if (resources?.failures.length) { const issue = resources.failures[0]; fail('resource_media', issue.message, issue.ref); return; }
        const pixels = renderer.getDrawingBufferSize(new THREE.Vector2());
        submittedFrame = { key: capture.key, frame_id: ++submitted, camera: capture.camera, lod: current,
          pixels: { width: pixels.x, height: pixels.y, css_width: node.clientWidth, css_height: node.clientHeight, pixel_ratio: renderer.getPixelRatio() },
          visible_objects: visibleFactoryObjects(scene, camera), limitations: [
            ...(halls.length === 0 ? [{ object_ref: null, message: '冻结工程未声明当前画布的厂房，物理画布为空。' }] : []),
            ...(connections?.limitations ?? []),
            ...(project?.devices.filter(device => halls.some(hall => hall.id === device.hall_id) && !anchors.has(`device:${device.id}`)) ?? []).map(device => ({ object_ref: { kind: 'device' as const, id: device.id }, message: '实际布局父级缺失，未在猜测的位置绘制设备。' })),
            ...mediaSlots.filter(slot => slot.group.visible && !slot.instance && current !== 'region').map(slot => ({ object_ref: { kind: 'device' as const, id: slot.device.id }, message: `仅实际占地/定位标记：${slot.input?.unavailable ?? (slot.input?.media ? '精确素材装载中' : '表现素材未连接')}` })),
            ...(items?.unavailable ?? []).map(issue => ({ object_ref: issue.ref, message: issue.message })),
            ...(resources?.unavailable ?? []).map(issue => ({ object_ref: issue.ref, message: issue.message })),
          ] };
        setCaptureFrame(submitted);
        if (gpuFence) gpu.deleteSync(gpuFence);
        cancelAnimationFrame(gpuWait);
        gpuFence = gpu.fenceSync(gpu.SYNC_GPU_COMMANDS_COMPLETE, 0);
        if (!gpuFence) { fail('gpu_sync', 'GPU未建立实际帧完成栅栏。'); return; }
        gpu.flush();
        const frame = submitted, fence = gpuFence;
        const probe = () => {
          if (!live || captureFailed || frame !== submitted) return;
          const status = gpu.clientWaitSync(fence, 0, 0);
          if (status === gpu.WAIT_FAILED) {
            const contextLost = gpu.isContextLost();
            fail('gpu_sync', `GPU帧完成查询失败（上下文${contextLost ? '已丢失' : '有效'}，错误码 0x${gpu.getError().toString(16)}，帧 ${frame}）。`);
            return;
          }
          if (status === gpu.ALREADY_SIGNALED || status === gpu.CONDITION_SATISFIED) {
            gpu.deleteSync(fence); gpuFence = null; gpuReady = frame; notifyReady();
          } else gpuWait = requestAnimationFrame(probe);
        };
        gpuWait = requestAnimationFrame(probe);
      }
    };
    navigateView.current = face => {
      const direction = {
        right: new THREE.Vector3(1, 0, 0), left: new THREE.Vector3(-1, 0, 0),
        back: new THREE.Vector3(0, 0, -1), front: new THREE.Vector3(0, 0, 1),
        top: new THREE.Vector3(0, 1, 0.00001), bottom: new THREE.Vector3(0, -1, 0.00001),
      }[face].normalize();
      const radius = camera.position.distanceTo(controls.target);
      userCameraChanged = true;
      camera.position.copy(controls.target).addScaledVector(direction, radius);
      controls.update(); render();
    };
    const resize = new ResizeObserver(() => {
      const { width, height } = node.getBoundingClientRect();
      if (capture && !(width > 0 && height > 0)) { sizeApplied = false; return; }
      sizeApplied = true;
      renderer.setSize(width, height); camera.aspect = width / Math.max(height, 1); camera.updateProjectionMatrix(); refitDefault(); render();
    });
    const cameraChanged = () => {
      if (following.current && followOffset) followOffset.copy(camera.position).sub(controls.target);
      render();
    };
    resize.observe(node); if (!capture) controls.addEventListener('change', cameraChanged);
    viewActions.current = {
      fit: () => { stopFollowing(); setFollowMessage(''); userCameraChanged = false; refitDefault(); render(); },
      focus: ref => {
        const object = anchors.get(ref.kind + ':' + ref.id); if (!object) return;
        object.updateWorldMatrix(true, true);
        const bounds = new THREE.Box3().setFromObject(object); if (bounds.isEmpty()) return;
        stopFollowing(); setFollowMessage(''); userCameraChanged = true;
        const fit = defaultObliqueCamera(bounds, camera.fov, camera.aspect);
        camera.position.copy(fit.eye); controls.target.copy(fit.target); controls.update(); render();
      },
      follow: id => { following.current = id; followOffset = null; setFollowingId(id); followCurrent(); render(); },
      stop: () => { stopFollowing(); setFollowMessage(''); render(); },
    };
    const contextLost = () => fail('gpu_context', '实际WebGL上下文已丢失。');
    if (capture) renderer.domElement.addEventListener('webglcontextlost', contextLost);
    let down: { x: number; y: number; pointer: number } | null = null;
    const pointerDown = (event: PointerEvent) => { if (event.button === 0) down = layoutHandle?.active() ? null : { x: event.clientX, y: event.clientY, pointer: event.pointerId }; };
    const pointerUp = (event: PointerEvent) => {
      if (event.button !== 0 || !down || down.pointer !== event.pointerId) return;
      const distance = Math.hypot(event.clientX - down.x, event.clientY - down.y); down = null;
      if (distance > 4) return;
      const rect = renderer.domElement.getBoundingClientRect();
      const ray = new THREE.Raycaster();
      ray.params.Line.threshold = Math.max(size / 600, .02);
      ray.setFromCamera(new THREE.Vector2((event.clientX - rect.left) / rect.width * 2 - 1, -(event.clientY - rect.top) / rect.height * 2 + 1), camera);
      const hit = ray.intersectObjects(selectable, true).find(value => {
        let object: THREE.Object3D | null = value.object;
        while (object) { if (!object.visible) return false; object = object.parent; }
        return true;
      });
      const connection = hit ? factoryConnectionAt(hit.object, hit.index) : null;
      let object: THREE.Object3D | null = hit?.object ?? null;
      while (object && !object.userData.ref) object = object.parent;
      select.current(connection ?? object?.userData.ref as ObjectRef ?? null, event.shiftKey || event.ctrlKey || event.metaKey);
    };
    const pointerCancel = (event: PointerEvent) => { if (down?.pointer === event.pointerId) down = null; };
    let box: { pointer: number; x: number; y: number; endX: number; endY: number; orbitEnabled: boolean; context: string } | null = null;
    const projectedDevices = () => {
      scene.updateMatrixWorld(true); const rect = renderer.domElement.getBoundingClientRect();
      return deviceObjects.flatMap(object => {
        for(let parent: THREE.Object3D | null = object; parent; parent = parent.parent) if(!parent.visible) return [];
        const center = new THREE.Box3().setFromObject(object).getCenter(new THREE.Vector3()).project(camera);
        return center.z < -1 || center.z > 1 || Math.abs(center.x) > 1 || Math.abs(center.y) > 1 ? []
          : [{ ref: object.userData.ref as ObjectRef, x: rect.left + (center.x + 1) * rect.width / 2, y: rect.top + (1 - center.y) * rect.height / 2 }];
      });
    };
    const boxedDevices = (value: NonNullable<typeof box>) => projectedDevices().filter(point => point.x >= Math.min(value.x, value.endX) && point.x <= Math.max(value.x, value.endX) && point.y >= Math.min(value.y, value.endY) && point.y <= Math.max(value.y, value.endY)).map(point => point.ref);
    const endBox = () => {
      const value = box; if(!value) return;
      box = null; down = null; controls.enabled = value.orbitEnabled; setSelectionBox(null);
      if(renderer.domElement.hasPointerCapture(value.pointer)) renderer.domElement.releasePointerCapture(value.pointer);
    };
    const boxDown = (event: PointerEvent) => {
      if(box || event.button !== 0 || !event.shiftKey || !selectMany.current || layoutHandle?.active()) return;
      event.preventDefault(); event.stopImmediatePropagation();
      box = { pointer: event.pointerId, x: event.clientX, y: event.clientY, endX: event.clientX, endY: event.clientY, orbitEnabled: controls.enabled, context: selectionContext.current };
      controls.enabled = false; renderer.domElement.setPointerCapture(event.pointerId);
    };
    const boxMove = (event: PointerEvent) => {
      if(!box || box.pointer !== event.pointerId) return;
      event.preventDefault(); event.stopImmediatePropagation(); box.endX = event.clientX; box.endY = event.clientY;
      const rect = renderer.domElement.getBoundingClientRect();
      if(Math.hypot(box.endX - box.x, box.endY - box.y) > 4) setSelectionBox({ left: Math.min(box.x, box.endX) - rect.left, top: Math.min(box.y, box.endY) - rect.top, width: Math.abs(box.endX - box.x), height: Math.abs(box.endY - box.y) });
    };
    const boxUp = (event: PointerEvent) => {
      if(!box || box.pointer !== event.pointerId) return;
      event.preventDefault(); event.stopImmediatePropagation(); const value = box; value.endX = event.clientX; value.endY = event.clientY;
      const dragged = Math.hypot(value.endX - value.x, value.endY - value.y) > 4, refs = dragged ? boxedDevices(value) : [];
      endBox(); if(value.context !== selectionContext.current || !selectMany.current) return;
      if(dragged) selectMany.current(refs, true);
      else { down = { x: value.x, y: value.y, pointer: value.pointer }; pointerUp(event); }
    };
    const boxCancel = (event: PointerEvent) => { if(box?.pointer === event.pointerId) endBox(); };
    const boxEscape = (event: KeyboardEvent) => { if(box && event.key === 'Escape') { event.preventDefault(); event.stopImmediatePropagation(); endBox(); } };
    (renderer.domElement as HTMLCanvasElement & { readSelectionDiagnostics?: () => unknown }).readSelectionDiagnostics = () => ({ selected: interaction.current.selections ?? [], box: box ? { pointer: box.pointer, start: [box.x, box.y], end: [box.endX, box.endY], candidates: boxedDevices(box) } : null, projected_devices: projectedDevices() });
    if (!capture) {
      renderer.domElement.addEventListener('pointerdown', boxDown, true);
      renderer.domElement.addEventListener('pointermove', boxMove, true);
      renderer.domElement.addEventListener('pointerup', boxUp, true);
      renderer.domElement.addEventListener('pointercancel', boxCancel);
      renderer.domElement.addEventListener('lostpointercapture', boxCancel);
      window.addEventListener('keydown', boxEscape, true);
      renderer.domElement.addEventListener('pointercancel', pointerCancel);
      renderer.domElement.addEventListener('lostpointercapture', pointerCancel);
      renderer.domElement.addEventListener('pointerdown', pointerDown);
      renderer.domElement.addEventListener('pointerup', pointerUp);
    }
    let layoutTarget: { object: THREE.Object3D; pose: Pose } | null = null;
    updateInteraction.current = () => {
      const input = interaction.current, selected = input.selection;
      for (const marker of selectionMarkers) {
        const active = selected?.kind === marker.ref.kind && selected.id === marker.ref.id
          || marker.additive && input.selections?.some(ref => ref.kind === marker.ref.kind && ref.id === marker.ref.id);
        marker.material.color.setHex(active ? marker.selectedColor : marker.baseColor);
      }
      connections?.setSelection(selected);
      if (connections) connections.root.visible = input.showConnections || selected?.kind === 'connection';
      layoutHandle?.dispose(); layoutHandle = null;
      if (layoutTarget) { place(layoutTarget.object, layoutTarget.pose); layoutTarget = null; }
      if (!capture && input.layoutControl && input.onTransform && project && selected) {
        const object = anchors.get(selected.kind + ':' + selected.id);
        const pose = selected.kind === 'device' ? project.devices.find(value => value.id === selected.id)?.pose
          : selected.kind === 'line' ? project.lines.find(value => value.id === selected.id)?.transform
          : selected.kind === 'hall' ? project.halls.find(value => value.id === selected.id)?.origin : null;
        if (object && pose) {
          layoutTarget = { object, pose };
          layoutHandle = attachLayoutControls(scene, camera, renderer.domElement, controls, object, pose, input.layoutControl,
            render, next => input.onTransform!(selected, next, project), reason => setError(reason instanceof Error ? reason.message : String(reason)));
        }
      }
      render();
    };
    updateInteraction.current();
    let frameEpoch = 0;
    updateFrame.current = async next => {
      if (next.mediaSource === mediaSource && next.observation === observation) return;
      const epoch = ++frameEpoch, current = lastLod ?? lod;
      const inputs = new Map(next.mediaSource?.devices.map(input => [input.device_id, input]) ?? []);
      itemLayer?.detach();
      await Promise.all(mediaSlots.map(async slot => {
        const ticket = ++slot.ticket, input = inputs.get(slot.device.id), deviceLod = slot.lod ?? current;
        slot.input = input;
        const owns = () => live && epoch === frameEpoch && ticket === slot.ticket;
        try {
          if (current !== 'region' && input?.media && slot.instance) {
            const kept = await slot.instance.update(input, deviceLod);
            if (!owns()) return;
            if (kept) return;
          }
          if (!owns()) return;
          if (slot.instance) { slot.group.remove(slot.instance.root); slot.instance.dispose(); slot.instance = null; }
          slot.marker.visible = true;
          slot.annotation.title = `${slot.baseTitle} · ${current === 'region' ? '区域定位' : next.regionActualPending && !input ? '正在读取此时点表现来源…' : input?.unavailable ?? (input?.media ? '读取素材中' : '表现素材未连接')}`;
          if (current === 'region' || !input?.media) return;
          pendingMedia++;
          let instance: FactoryMediaInstance | null;
          try { instance = await loadDevice(slot.device, input, deviceLod, owns); } finally { pendingMedia--; }
          if (!instance) return;
          if (!owns()) { instance.dispose(); return; }
          slot.instance = instance;
          instance.root.traverse(object => { object.userData.ref = { kind: 'device', id: slot.device.id }; });
          slot.group.add(instance.root); slot.marker.visible = outsideHallFloor(project!, slot.device) === true;
        } catch (reason) {
          if (!owns()) return;
          if (slot.instance) { slot.group.remove(slot.instance.root); slot.instance.dispose(); slot.instance = null; }
          slot.marker.visible = true;
          slot.annotation.title = `${slot.baseTitle} · ${reason instanceof Error ? reason.message : String(reason)}`;
          fail('device_resource', reason instanceof Error ? reason.message : String(reason), { kind: 'device', id: slot.device.id });
        }
      }));
      if (!live || epoch !== frameEpoch) return;
      mediaSource = next.mediaSource; observation = next.observation; resourceSources = next.resourceSources; regionActualPending = next.regionActualPending;
      transportLayer.update(mediaSource?.transports ?? []);
      actual = run && observation?.run_id === run.id && observation.next_index === null &&
        observation.at_ms >= run.window.start_ms && run.computed_until_ms != null && observation.at_ms <= run.computed_until_ms ? observation : null;
      refreshObservedAnnotations();
      await Promise.all([itemLayer?.update(mediaSource?.items ?? []), resourceLayer?.update(resourceSources)]);
      if (!live || epoch !== frameEpoch) return;
      mediaChanged();
    };
    return () => {
      live = false;
      frameEpoch++;
      cancelAnimationFrame(mediaRenderFrame);
      updateFrame.current = null;
      viewActions.current = null;
      updateInteraction.current = null;
      layoutHandle?.dispose();
      if (capture) { captureLayout.current = null; cancelAnimationFrame(gpuWait); if (gpuFence) gpu?.deleteSync(gpuFence); }
      itemLayer?.dispose();
      resourceLayer?.dispose();
      transportLayer.dispose();
      for (const slot of mediaSlots) {
        slot.ticket++;
        if (slot.instance) { slot.group.remove(slot.instance.root); slot.instance.dispose(); }
      }
      if (!capture) cameras.current.set(cameraKey, { eye: camera.position.clone(), target: controls.target.clone(), reset: resetKey, changed: userCameraChanged });
      navigateView.current = null;
      resize.disconnect(); controls.dispose();
      endBox();
      renderer.domElement.removeEventListener('pointerdown', boxDown, true);
      renderer.domElement.removeEventListener('pointermove', boxMove, true);
      renderer.domElement.removeEventListener('pointerup', boxUp, true);
      renderer.domElement.removeEventListener('pointercancel', boxCancel);
      renderer.domElement.removeEventListener('lostpointercapture', boxCancel);
      window.removeEventListener('keydown', boxEscape, true);
      renderer.domElement.removeEventListener('pointercancel', pointerCancel);
      renderer.domElement.removeEventListener('lostpointercapture', pointerCancel);
      renderer.domElement.removeEventListener('pointerdown', pointerDown);
      renderer.domElement.removeEventListener('pointerup', pointerUp);
      renderer.domElement.removeEventListener('webglcontextlost', contextLost);
      scene.traverse(object => {
        if (object instanceof THREE.Mesh || object instanceof THREE.Line || object instanceof THREE.Points) {
          object.geometry.dispose();
          const materials = Array.isArray(object.material) ? object.material : [object.material];
          materials.forEach(material => material.dispose());
        }
      });
      renderer.renderLists.dispose();
    };
  }, [project, canvas, hallId, lod, autoLod, resetKey, run, inputModules, !!mediaSource, capture, captureSource]);

  useEffect(() => {
    void updateFrame.current?.(frameInput.current);
  }, [mediaSource, observation]);

  useEffect(() => {
    updateInteraction.current?.();
  }, [selection?.id, selection?.kind, selections, layoutControl?.mode, layoutControl?.snap, layoutControl?.step_m, showConnections]);

  return <div className="factory-scene" ref={host} aria-label="厂房三维画布">
    {!capture && <ViewportViewCube ref={viewCube} onFace={face => navigateView.current?.(face)} />}
    {!capture && <ViewportScale ref={viewportScale} />}
    {selectionBox && <div aria-label="布局框选范围" style={{ position: 'absolute', ...selectionBox, border: '1px solid #287bcd', background: '#287bcd20', pointerEvents: 'none', zIndex: 6 }} />}
    {error && <p className="scene-error" role="alert">{error}</p>}
    <div className="scene-caption">{canvas === 'line_canvas' ? halls[0]?.title ?? '厂房' : '多厂房总览'}{project && ` · ${project.devices.filter(device => halls.some(hall => hall.id === device.hall_id)).length}台设备`}<br />{({ detailed: '详细模型', simplified: '简化模型', region: '区域总览' })[visibleLod]}{focusedDetail ? ' · 选中设备详细' : ''}{autoLod ? ' · 自动切换' : ''}</div>
    {!capture && project && <div className="scene-view-actions" role="group" aria-label="画面视角" style={{ position: 'absolute', top: 68, left: 15, right: 170, display: 'flex', flexWrap: 'wrap', gap: 6, alignItems: 'center', zIndex: 4 }}>
      <button onClick={() => viewActions.current?.fit()}>全厂适配</button>
      {connectionRows.length > 0 && <label title="工程声明的交接方向，非实际物流轨迹" style={{display:'flex',alignItems:'center',gap:4,fontSize:12,background:'#f5f8faed',padding:'3px 6px'}}><input type="checkbox" checked={showConnections || selection?.kind === 'connection'} disabled={selection?.kind === 'connection'} onChange={event => setShowConnections(event.target.checked)} />物料连线</label>}
      {selection && ['device', 'location', 'line', 'hall'].includes(selection.kind) && <button onClick={() => viewActions.current?.focus(selection)}>聚焦选中对象</button>}
      {selection?.kind === 'item' && <button onClick={() => viewActions.current?.follow(selection.id)}>跟随选中物品</button>}
      {followingId && <button onClick={() => viewActions.current?.stop()}>停止跟随</button>}
      {followMessage && <span role="status" style={{ background: '#f5f8faed', padding: '3px 6px', fontSize: 12 }}>{followMessage}</span>}
    </div>}
    {visibleLod === 'region' && project && project.lines.some(line => halls.some(hall => hall.id === line.hall_id)) && <details aria-label="区域实际汇总" style={{ position: 'absolute', top: 8, left: 8, zIndex: 4, maxWidth: 340, maxHeight: '70%', overflow: 'auto', background: '#ffffffed', color: '#18364b', padding: 8 }}>
      <summary>区域与跨区物流</summary>{project.lines.filter(line => halls.some(hall => hall.id === line.hall_id)).map(line => {
        const value = regionSummary(project, line, mediaSource, regionActualPending);
        return <section key={line.id}><button disabled={!!capture} onClick={() => onSelect({ kind: 'line', id: line.id })}>{line.title} · {value.members.length}台设备</button><p>{value.state}</p>
          <p>本时点已读人员／资源占用：{value.resources.map(resource => resource.title).join(' / ') || '未读到占用记录'}</p>
          {value.ports.map(({ port, label }) => <p key={port.instance_id + ':' + port.port_id}>边界口 {port.instance_id} / {port.port_id}：{label}</p>)}
          {!value.ports.length && <p>未声明区域边界口。</p>}
          {value.boundary.map(connection => <p key={connection.id}>{connection.label} · {connection.id}</p>)}
          <div>{value.members.map(device => <button key={device.id} disabled={!!capture} onClick={() => onSelect({ kind: 'device', id: device.id })}>{device.title}</button>)}</div>
        </section>;
      })}</details>}
    {!project && <div className="canvas-empty"><strong>从一个工程开始</strong><span>新建工程，或打开已有工程目录</span></div>}
    {project && halls.length === 0 && <div className="canvas-empty"><strong>尚无厂房</strong><span>先创建厂房，再布置设备和产线</span></div>}
    <div className="scene-annotations">{labels.map(label => <button disabled={!!capture} className={`scene-label ${label.kind}`} data-scene-label-key={label.key} key={label.key} title={label.title} style={{ left: label.x, top: label.y, visibility: hiddenLabelKeys.has(label.key) ? 'hidden' : 'visible', opacity: capture ? 1 : undefined }} onClick={event => onSelect(label.ref, event.shiftKey || event.ctrlKey || event.metaKey)}>{label.text}</button>)}</div>
    {!capture && hasSidePanel && <details aria-label="实际物品与共享标签" onToggle={event => setSidePanelOpen(event.currentTarget.open)} style={{ position: 'absolute', top: 128, right: 16, zIndex: 4, maxWidth: 340, maxHeight: 'calc(100% - 164px)', overflow: 'auto', background: '#ffffffed', color: '#18364b', padding: 8 }}>
      <summary>画面信息</summary>
      {sidePanelOpen && <>
      <p>{mediaSource?.run ? <>已读取物品记录 {mediaSource.items.length} 条 · 数据标注 {dataLabels.length} 条</> : '画布标签'}{connectionRows.length > 0 && ` · 可显示连线 ${connectionRows.filter(row => !row.problem).length}/${connectionRows.length}`}{hiddenLabels.length > 0 && ` · 已收起重叠标签 ${hiddenLabels.length} 个`}</p>
      {connectionRows.length > 0 && <section aria-label="物料逻辑连接"><p>箭头表达工程声明的交接方向，不表示生产流动或运输时长。连线定位于对象，端口标识见详情。</p>{connectionRows.map(row => <div key={row.id} style={{ marginTop: 8 }}><button disabled={!!capture} style={{ display: 'block', width: '100%', textAlign: 'left', overflowWrap: 'anywhere' }} onClick={() => onSelect({ kind: 'connection', id: row.id })}>{row.title}</button>{row.problem && <p>未绘制：{row.problem}</p>}</div>)}</section>}
      {hiddenLabels.length > 0 && <div aria-label="收起的画布标签"><p>重叠或超出画布的标签移至此处，点击定位。</p>{hiddenLabels.map(label => <button key={label.key} data-collapsed-label-key={label.key} onClick={event => onSelect(label.ref, event.shiftKey || event.ctrlKey || event.metaKey)} style={{ display: 'block', maxWidth: '100%', textAlign: 'left', overflowWrap: 'anywhere' }}>{label.title}</button>)}</div>}
      {resourceSources.map(input => <div key={input.resource.id}><button onClick={() => onSelect({ kind: 'resource', id: input.resource.id })}>{input.resource.title} · {input.resource.id}</button>{input.worker_status && <p>{factoryWorkerLabel(input)}</p>}<p>{resourceMessages.key === mediaSource?.key ? resourceMessages.values[input.resource.id] ?? '正在读取资源表现' : input.unavailable ?? '正在读取资源表现'}</p></div>)}
      {(mediaSource?.transports ?? []).map(input => <div key={input.trip.id}><button onClick={() => onSelect({ kind: 'transport', id: input.trip.id })}>厂内转运 · {input.label}</button><p>{input.unavailable ?? '按运行记录与厂内路线显示'} · {input.trip.phase === 'returning' ? '空车' : `${input.trip.cargo.length} 件装载物`}</p></div>)}
      {(mediaSource?.items ?? []).map(({ item, unavailable, media }) => <div key={item.id} style={{ marginTop: 8 }}>
        <button onClick={() => onSelect({ kind: 'item', id: item.id })}>{factoryItemTitle(project, item)} · {decimalText(item.quantity.value)}{factoryUnitLabel(item.quantity.unit)}</button>
        <p>{factoryLifecycleLabel[item.lifecycle]} · {factoryQualityLabel[item.quality]} · {factoryItemPlace(project, item, run, mediaSource?.items.map(value => value.item))}</p>
        <details><summary>物品与显示详情</summary><small style={{ display: 'block', overflowWrap: 'anywhere' }}>编号 {item.id}<br />类型 {item.item_type_ref.id}@{item.item_type_ref.revision}<br />位置编号 {item.holder.kind}:{item.holder.id}<br />媒体 {media ? `${media.binding.request.source.data.module_ref.id}@${media.binding.request.source.data.module_ref.version}` : '未提供'}<br />内装物 {item.contents.join('、') || '无'} · 组成来源 {item.composition_refs.join('、') || '无'}</small>
        <span data-item-geometry-status={item.id}>{itemMessages.key === mediaSource?.key ? itemMessages.values[item.id] ?? unavailable : unavailable}</span></details>
      </div>)}
      {dataLabels.map(({ device_id, label }) => <div key={`${device_id}:${label.id}`} style={{ marginTop: 8 }}><button onClick={() => onSelect({ kind: 'device', id: device_id })}>{device_id} · {label.label}</button><span data-field-status={label.status} style={{ display: 'block' }}>{label.value}</span></div>)}
      </>}
    </details>}
    <div className="scene-help">{capture ? '只读捕获 · 明确相机与固定LOD' : '右键环绕 · 中键平移 · 滚轮缩放 · 单击查看'}</div>
  </div>;
}
