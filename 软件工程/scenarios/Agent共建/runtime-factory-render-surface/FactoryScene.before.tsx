import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { Pose } from '../../../../packages/commands/generated/typescript/Pose';
import type { Vec2 } from '../../../../packages/commands/generated/typescript/Vec2';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { RunObjectsPage } from '../../../../packages/commands/generated/typescript/RunObjectsPage';
import { poseMatrix } from './layout';
import { decimalText, deviceActivity } from './runtime';
import { createFactoryMedia, factoryMediaLabels, type FactoryMediaInstance, type FactoryMediaLabel } from './factoryMedia';
import { factoryItemHolder, type FactoryPresentationSource, type FactoryDeviceMedia } from '../services/factoryPresentation';
import { createFactoryItemLayer } from './factoryItems';

export type LodMode = 'detailed' | 'simplified' | 'region';
export interface FactorySceneProps {
  project: ProjectState | null;
  canvas: 'line_canvas' | 'multi_hall_canvas';
  hallId: string | null;
  selection: ObjectRef | null;
  onSelect: (ref: ObjectRef | null) => void;
  lod: LodMode;
  autoLod: boolean;
  resetKey: number;
  run?: RunResult | null;
  observation?: RunObjectsPage | null;
  /** Clear on context changes; actual sources contain their own frozen physical Project. */
  mediaSource?: FactoryPresentationSource | null;
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

export function FactoryScene({ project: inputProject, canvas, hallId, selection, onSelect, lod, autoLod, resetKey, run: inputRun = null, observation: inputObservation = null, mediaSource = null }: FactorySceneProps) {
  const project = mediaSource?.project ?? inputProject;
  const run = mediaSource?.run ?? inputRun;
  const observation = mediaSource?.observation ?? inputObservation;
  const host = useRef<HTMLDivElement>(null);
  const select = useRef(onSelect);
  select.current = onSelect;
  const [error, setError] = useState<string | null>(null);
  const [visibleLod, setVisibleLod] = useState(lod);
  const [itemMessages, setItemMessages] = useState<{ key: string | null; values: Record<string, string> }>({ key: null, values: {} });
  const [dataLabels, setDataLabels] = useState<{ device_id: string; label: FactoryMediaLabel }[]>([]);
  const [labels, setLabels] = useState<{ key: string; title: string; x: number; y: number; ref: ObjectRef; kind: 'device' | 'item' }[]>([]);
  const [hiddenLabelKeys, setHiddenLabelKeys] = useState<Set<string>>(() => new Set());
  const [sidePanelOpen, setSidePanelOpen] = useState(false);
  const hiddenLabels = labels.filter(label => hiddenLabelKeys.has(label.key));
  const hasSidePanel = !!mediaSource?.run || hiddenLabels.length > 0;
  const halls = project?.halls.filter(hall => canvas === 'multi_hall_canvas' || hall.id === hallId) ?? [];
  const cameras = useRef(new Map<string, { eye: THREE.Vector3; target: THREE.Vector3; reset: number }>());

  useLayoutEffect(() => {
    const node = host.current!;
    const bounds = node.getBoundingClientRect();
    const intersects = (a: DOMRect, b: DOMRect) => a.left < b.right + 3 && a.right + 3 > b.left && a.top < b.bottom + 3 && a.bottom + 3 > b.top;
    const occupied = [...node.querySelectorAll<HTMLElement>('.scene-caption, .scene-help, details[aria-label="实际物品与共享标签"]')]
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
    setHiddenLabelKeys(previous => previous.size === hidden.size && [...hidden].every(key => previous.has(key)) ? previous : hidden);
  }, [labels, selection?.kind, selection?.id, sidePanelOpen, hasSidePanel]);

  useEffect(() => {
    const node = host.current!;
    let renderer: THREE.WebGLRenderer;
    try { renderer = new THREE.WebGLRenderer({ antialias: true }); }
    catch { setError('三维画布无法初始化，请检查图形环境。工程数据未改变。'); return; }
    setError(null);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0xe8edf2);
    node.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(42, 1, 0.01, 100000);
    const controls = new OrbitControls(camera, renderer.domElement);
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
    const regionObjects: THREE.Object3D[] = [];
    const groupedObjects: THREE.Object3D[] = [];
    let live = true;
    let itemLayer: ReturnType<typeof createFactoryItemLayer> | null = null;
    const media = new Map(mediaSource?.devices.map(value => [value.device_id, value]) ?? []);
    const mediaSlots: { group: THREE.Group; device: NonNullable<typeof project>['devices'][number]; input: FactoryDeviceMedia | undefined;
      marker: THREE.Object3D; instance: FactoryMediaInstance | null; ticket: number; baseTitle: string; annotation: typeof annotations[number]; grouped: boolean }[] = [];
    const anchors = new Map<string, THREE.Object3D>();
    const annotations: { key: string; title: string; anchor: THREE.Object3D; ref: ObjectRef; kind: 'device' | 'item' }[] = [];
    const actual = run && observation?.run_id === run.id && observation.next_index === null &&
      observation.at_ms >= run.window.start_ms && run.computed_until_ms != null && observation.at_ms <= run.computed_until_ms ? observation : null;
    for (const hall of halls) {
      const root = new THREE.Group();
      if (canvas === 'multi_hall_canvas') place(root, hall.origin);
      physical.add(root);
      const floor = polygon([{ x: 0, y: 0 }, { x: hall.width_m, y: 0 }, { x: hall.width_m, y: hall.depth_m }, { x: 0, y: hall.depth_m }], null, 0xf8fafc);
      if (floor) {
        floor.position.z = -0.01; floor.userData.ref = { kind: 'hall', id: hall.id };
        root.add(floor); selectable.push(floor);
        const edge = new THREE.LineSegments(new THREE.EdgesGeometry(floor.geometry), new THREE.LineBasicMaterial({ color: 0x8a9eb3 }));
        root.add(edge);
      }
      const lines = new Map<string, THREE.Group>();
      for (const line of project!.lines.filter(value => value.hall_id === hall.id)) {
        const group = new THREE.Group(); place(group, line.transform); root.add(group); lines.set(line.id, group);
        const area = polygon(line.region_polygon_m, null, 0xabc2d7);
        if (area) {
          area.position.z = 0.005; area.userData.ref = { kind: 'line', id: line.id };
          (area.material as THREE.MeshStandardMaterial).transparent = true;
          (area.material as THREE.MeshStandardMaterial).opacity = 0.28;
          group.add(area); regionObjects.push(area); selectable.push(area);
        }
      }
      for (const device of project!.devices.filter(value => value.hall_id === hall.id)) {
        const group = new THREE.Group(); place(group, device.pose);
        const parent = device.line_id ? lines.get(device.line_id) : root;
        // Missing layout parent is unknown; never render it at a fabricated position.
        if (!parent) continue;
        parent.add(group);
        anchors.set(`device:${device.id}`, group);
        const selected = selection?.kind === 'device' && selection.id === device.id;
        const activity = actual && run ? deviceActivity(run, actual.at_ms, device.id) : null;
        // Footprints are flat layout markers, never fabricated detailed bodies.
        const footprint = polygon(device.footprint_m, null, selected ? 0x287bcd : activity?.work ? 0x468c80 : activity?.waits ? 0xba9464 : 0x839bad);
        const marker = footprint ?? new THREE.Points(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3()]), new THREE.PointsMaterial({ color: selected ? 0x287bcd : 0x46647b, size: 7, sizeAttenuation: false }));
        marker.position.z = .01; marker.userData.ref = { kind: 'device', id: device.id };
        if (footprint) { const material = footprint.material as THREE.MeshStandardMaterial; material.transparent = true; material.opacity = .3; }
        group.add(marker); selectable.push(group);
        group.userData.ref = { kind: 'device', id: device.id };
        const grouped = !!device.line_id && project!.lines.some(line => line.id === device.line_id && line.region_polygon_m.length >= 3);
        deviceObjects.push(group); if (grouped) groupedObjects.push(group);
        const baseTitle = `${device.title}${footprint ? '' : ' · 占地未声明（定位标记）'}${activity?.work ? ` · 执行记录 ${activity.work}` : activity?.waits ? ` · 等待记录 ${activity.waits}` : ''}`;
        const annotation = { key: `device:${device.id}`, title: baseTitle, anchor: group, ref: { kind: 'device' as const, id: device.id }, kind: 'device' as const };
        annotations.push(annotation);
        mediaSlots.push({ group, device, input: media.get(device.id), marker, instance: null, ticket: 0, baseTitle, annotation, grouped });
      }
      for (const location of project!.locations.filter(value => value.hall_id === hall.id)) {
        const marker = new THREE.Mesh(new THREE.RingGeometry(0.19, 0.24, 24), new THREE.MeshBasicMaterial({ color: 0x447b98, side: THREE.DoubleSide }));
        place(marker, location.position); marker.userData.ref = { kind: 'location', id: location.id };
        anchors.set(`location:${location.id}`, marker);
        root.add(marker); selectable.push(marker);
      }
    }
    if (actual && run) {
      const items = actual.objects.flatMap(object => object.kind === 'item' ? [object.data] : []);
      for (const item of items) {
        // Consumed composition records and contained children remain selectable
        // in the shared object list; they are not painted as extra loose stock.
        if (item.lifecycle === 'transformed' || item.holder.kind === 'container') continue;
        const jobs = actual.objects.flatMap(object => object.kind === 'job' ? [object.data] : []);
        const holder = factoryItemHolder(item, items, jobs);
        const anchor = holder ? anchors.get(`${holder.kind}:${holder.id}`) : null;
        if (!anchor) continue;
        annotations.push({ key: `item:${item.id}`, title: `${item.id} · ${decimalText(item.quantity.value)} ${item.quantity.unit} · ${item.quality}`, anchor, ref: { kind: 'item', id: item.id }, kind: 'item' });
      }
    }
    const bounds = new THREE.Box3().setFromObject(physical);
    const size = bounds.isEmpty() ? 20 : bounds.getSize(new THREE.Vector3()).length();
    const center = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
    camera.position.copy(center).add(new THREE.Vector3(0.8, 1, 1).multiplyScalar(Math.max(size, 1)));
    controls.target.copy(center); controls.update();
    const cameraKey = `${project?.id ?? 'empty'}:${canvas}:${hallId ?? ''}`;
    const savedCamera = cameras.current.get(cameraKey);
    if (savedCamera?.reset === resetKey) { camera.position.copy(savedCamera.eye); controls.target.copy(savedCamera.target); controls.update(); }
    const axes = new THREE.AxesHelper(Math.max(size / 12, 0.5)); physical.add(axes);
    let lastLod: LodMode | null = null;
    const loadLod = (current: LodMode) => {
      itemLayer?.dispose(); itemLayer = null;
      for (const slot of mediaSlots) {
        const ticket = ++slot.ticket;
        if (slot.instance) { slot.group.remove(slot.instance.root); slot.instance.dispose(); slot.instance = null; }
        slot.marker.visible = true;
        slot.annotation.title = `${slot.baseTitle} · ${current === 'region' ? '区域定位' : slot.input?.unavailable ?? (slot.input?.media ? '读取素材中' : '表现素材未连接')}`;
        if (current === 'region' || !slot.input?.media) continue;
        createFactoryMedia(slot.device, slot.input, current).then(instance => {
          if (!live || ticket !== slot.ticket) { instance.dispose(); return; }
          slot.instance = instance;
          instance.root.traverse(object => { object.userData.ref = { kind: 'device', id: slot.device.id }; });
          slot.group.add(instance.root); slot.marker.visible = false;
          slot.annotation.title = `${slot.baseTitle} · ${slot.input?.projection === 'terminal' ? '实际结束姿态' : slot.input?.frame ? '实际阶段' : mediaSource?.run ? '静态素材（无活动执行）' : '静态素材'}`;
          itemLayer?.reconcile();
          render();
        }, reason => {
          if (!live || ticket !== slot.ticket) return;
          slot.annotation.title = `${slot.baseTitle} · ${reason instanceof Error ? reason.message : String(reason)}`;
          render();
        });
      }
      itemLayer = createFactoryItemLayer(mediaSource?.items ?? [], current,
        id => mediaSlots.find(slot => slot.device.id === id)?.instance ?? null,
        id => anchors.get(`location:${id}`) ?? null, render);
      itemLayer.reconcile();
    };
    const render = () => {
      if (!live) return;
      const ratio = camera.position.distanceTo(controls.target) / Math.max(size, 1);
      const current = autoLod ? (ratio < 0.8 ? 'detailed' : ratio < 2.5 ? 'simplified' : 'region') : lod;
      if (current !== lastLod) {
        lastLod = current; setVisibleLod(current);
        // Regions require a real declared polygon; ungrouped devices stay visible.
        regionObjects.forEach(value => { value.visible = current === 'region'; });
        deviceObjects.forEach(value => { value.visible = true; });
        if (current === 'region') groupedObjects.forEach(value => { value.visible = false; });
        loadLod(current);
      }
      renderer.render(scene, camera);
      setItemMessages({ key: mediaSource?.key ?? null, values: itemLayer?.statuses() ?? {} });
      const sharedLabels = mediaSlots.flatMap(slot => slot.input && (slot.instance?.presentation ?? slot.input.media?.presentation)
        ? factoryMediaLabels((slot.instance?.presentation ?? slot.input.media!.presentation), slot.input, mediaSource ?? undefined).map(label => ({ device_id: slot.device.id, label })) : []);
      setDataLabels(sharedLabels);
      const tagged = sharedLabels.flatMap(({ device_id, label }) => {
        const slot = mediaSlots.find(value => value.device.id === device_id), anchor = label.anchor.kind === 'scene_node' ? slot?.instance?.nodes.get(label.anchor.data) : null;
        return anchor && slot?.group.visible ? [{ key: `tag:${device_id}:${label.id}`, title: `${label.label} · ${label.value}`, anchor, ref: { kind: 'device' as const, id: device_id }, kind: 'device' as const }] : [];
      });
      const offsets = new Map<THREE.Object3D, number>();
      setLabels([...annotations, ...tagged].flatMap(annotation => {
        if (current === 'region' && annotation.kind === 'device' && mediaSlots.some(slot => slot.group === annotation.anchor && slot.grouped)) return [];
        const point = annotation.anchor.getWorldPosition(new THREE.Vector3()).project(camera);
        if (point.z < -1 || point.z > 1 || Math.abs(point.x) > 1 || Math.abs(point.y) > 1) return [];
        const offset = offsets.get(annotation.anchor) ?? 0; offsets.set(annotation.anchor, offset + 1);
        return [{ ...annotation, x: (point.x + 1) / 2 * node.clientWidth, y: (1 - point.y) / 2 * node.clientHeight - 22 - offset * 22 }];
      }));
    };
    const resize = new ResizeObserver(() => {
      const { width, height } = node.getBoundingClientRect();
      renderer.setSize(width, height); camera.aspect = width / Math.max(height, 1); camera.updateProjectionMatrix(); render();
    });
    resize.observe(node); controls.addEventListener('change', render);
    let down: { x: number; y: number } | null = null;
    const pointerDown = (event: PointerEvent) => { if (event.button === 0) down = { x: event.clientX, y: event.clientY }; };
    const pointerUp = (event: PointerEvent) => {
      if (event.button !== 0 || !down) return;
      const distance = Math.hypot(event.clientX - down.x, event.clientY - down.y); down = null;
      if (distance > 4) return;
      const rect = renderer.domElement.getBoundingClientRect();
      const ray = new THREE.Raycaster();
      ray.setFromCamera(new THREE.Vector2((event.clientX - rect.left) / rect.width * 2 - 1, -(event.clientY - rect.top) / rect.height * 2 + 1), camera);
      const hit = ray.intersectObjects(selectable, true).find(value => {
        let object: THREE.Object3D | null = value.object;
        while (object) { if (!object.visible) return false; object = object.parent; }
        return true;
      });
      select.current(hit ? hit.object.userData.ref as ObjectRef : null);
    };
    renderer.domElement.addEventListener('pointerdown', pointerDown);
    renderer.domElement.addEventListener('pointerup', pointerUp);
    render();
    return () => {
      live = false;
      itemLayer?.dispose();
      for (const slot of mediaSlots) {
        slot.ticket++;
        if (slot.instance) { slot.group.remove(slot.instance.root); slot.instance.dispose(); }
      }
      cameras.current.set(cameraKey, { eye: camera.position.clone(), target: controls.target.clone(), reset: resetKey });
      resize.disconnect(); controls.dispose();
      renderer.domElement.removeEventListener('pointerdown', pointerDown);
      renderer.domElement.removeEventListener('pointerup', pointerUp);
      scene.traverse(object => {
        if (object instanceof THREE.Mesh || object instanceof THREE.Line || object instanceof THREE.Points) {
          object.geometry.dispose();
          const materials = Array.isArray(object.material) ? object.material : [object.material];
          materials.forEach(material => material.dispose());
        }
      });
      renderer.dispose(); renderer.domElement.remove();
    };
  }, [project, canvas, hallId, selection?.id, selection?.kind, lod, autoLod, resetKey, run, observation, mediaSource]);

  return <div className="factory-scene" ref={host} aria-label="厂房三维画布">
    {error && <p className="scene-error" role="alert">{error}</p>}
    <div className="scene-caption">{canvas === 'line_canvas' ? '流水线画布' : '多厂房画布'} · {({ detailed: '详细', simplified: '简化', region: '区域' })[visibleLod]}{autoLod ? ' · 自动' : ''}<br />米 · Z 向上</div>
    {!project && <div className="canvas-empty"><strong>从一个工程开始</strong><span>新建工程，或打开已有工程目录</span></div>}
    {project && halls.length === 0 && <div className="canvas-empty"><strong>尚无厂房</strong><span>先创建厂房，再布置设备和产线</span></div>}
    <div className="scene-annotations">{labels.map(label => <button className={`scene-label ${label.kind}`} data-scene-label-key={label.key} key={label.key} title={label.title} style={{ left: label.x, top: label.y, visibility: hiddenLabelKeys.has(label.key) ? 'hidden' : 'visible' }} onClick={() => onSelect(label.ref)}>{label.title}</button>)}</div>
    {hasSidePanel && <details aria-label="实际物品与共享标签" onToggle={event => setSidePanelOpen(event.currentTarget.open)} style={{ position: 'absolute', top: 8, right: 8, zIndex: 4, maxWidth: 340, maxHeight: '70%', overflow: 'auto', background: '#ffffffed', color: '#18364b', padding: 8 }}>
      <summary>{mediaSource?.run ? <>实际物品 {mediaSource.items.length} · 数据标签 {dataLabels.length}</> : '画布标签'}{hiddenLabels.length > 0 && ` · 收起 ${hiddenLabels.length}`}</summary>
      {hiddenLabels.length > 0 && <div aria-label="收起的画布标签"><p>重叠或超出画布的标签移至此处，点击定位。</p>{hiddenLabels.map(label => <button key={label.key} data-collapsed-label-key={label.key} onClick={() => onSelect(label.ref)} style={{ display: 'block', maxWidth: '100%', textAlign: 'left', overflowWrap: 'anywhere' }}>{label.title}</button>)}</div>}
      {(mediaSource?.items ?? []).map(({ item, unavailable, media }) => <div key={item.id} style={{ marginTop: 8 }}>
        <button onClick={() => onSelect({ kind: 'item', id: item.id })}>{item.id}</button>
        <small style={{ display: 'block' }}>{item.item_type_ref.id}@{item.item_type_ref.revision} · {decimalText(item.quantity.value)} {item.quantity.unit} · {item.lifecycle} / {item.quality}<br />holder {item.holder.kind}:{item.holder.id}<br />媒体 {media ? `${media.binding.request.source.data.module_ref.id}@${media.binding.request.source.data.module_ref.version}` : 'unavailable'}<br />contents {item.contents.join('、') || '无'} · composition {item.composition_refs.join('、') || '无'}</small>
        <span data-item-geometry-status={item.id}>{itemMessages.key === mediaSource?.key ? itemMessages.values[item.id] ?? unavailable : unavailable}</span>
      </div>)}
      {dataLabels.map(({ device_id, label }) => <div key={`${device_id}:${label.id}`} style={{ marginTop: 8 }}><button onClick={() => onSelect({ kind: 'device', id: device_id })}>{device_id} · {label.label}</button><span data-field-status={label.status} style={{ display: 'block' }}>{label.value}</span></div>)}
    </details>}
    <div className="scene-help">右键环绕 · 中键平移 · 滚轮缩放 · 单击查看</div>
  </div>;
}
