import * as THREE from 'three';
import { activeClip, assetKey, bindScene, decodeGlb, projectPhase, resolveMediaData, UnsupportedMedia, type VisualProgress, type ClipPoint } from '../../../../packages/presentation/src';
import { resolveMedia, type ResolvedMedia } from '../features/media/load';
import { createImageSurface, mediaAsset } from '../features/media/imageSurface';
import type { DeviceInstance } from '../../../../packages/commands/generated/typescript/DeviceInstance';
import type { LodLevel } from '../../../../packages/commands/generated/typescript/LodLevel';
import type { FactoryDeviceMedia, FactoryActualItem, FactoryPresentationSource, FactoryResourceMedia } from '../services/factoryPresentation';
import type { AssetRef } from '../../../../packages/commands/generated/typescript/AssetRef';
import type { AnchorRef } from '../../../../packages/commands/generated/typescript/AnchorRef';
import type { MediaAssetReadRequest } from '../services/mediaAssets';

export interface FactoryMediaInstance {
  root: THREE.Object3D;
  nodes: Map<number, THREE.Object3D>;
  presentation: ResolvedMedia['presentation'];
  asset: AssetRef | null;
  source: MediaAssetReadRequest['source'];
  update(input: FactoryDeviceMedia | FactoryActualItem | FactoryResourceMedia, lod: LodLevel): Promise<boolean>;
  dispose(): void;
}
type FactoryMediaInput = FactoryDeviceMedia | FactoryActualItem | FactoryResourceMedia;
/** Declared finite item state; qualified clips opt into the actual owning Job's clock. */
export function factoryItemPoint(input: Pick<FactoryActualItem, 'item' | 'holder_frame' | 'holder_observed_phase'>,
  presentation: Pick<ResolvedMedia['presentation'], 'timeline'>): VisualProgress | null {
  const name = input.item.attributes?.presentation_state ? 'presentation_state' : 'closure';
  const state = input.item.attributes?.[name];
  if (!state || !presentation.timeline.length) return null;
  if (state.kind !== 'enum') throw new UnsupportedMedia(`物品${name}不是已声明有限状态。`);
  const frame = input.holder_frame;
  const holders = frame?.snapshot.objects.flatMap(object => object.kind === 'job' && object.data.id === frame.job_id ? [object.data] : []) ?? [];
  const reference = input.holder_observed_phase?.operation_ref ?? (holders.length === 1 ? holders[0].operation_ref : null);
  const scopedPrefix = reference ? `${state.value}:${reference.id}:${reference.version}:` : null;
  const usesScopedPhase = scopedPrefix != null && presentation.timeline.some(clip => clip.phase_ref?.startsWith(scopedPrefix));
  const observedPoint = input.holder_observed_phase?.point;
  const phaseIds = observedPoint?.display_phase_id ? [observedPoint.display_phase_id] : frame?.operation.phases.length ? frame.operation.phases.map(value => value.id) : frame?.operation.duration ? ['cycle'] : [];
  const usesLegacyPhase = phaseIds.some(id => presentation.timeline.some(clip => clip.phase_ref === `${state.value}:${id}`));
  const actualPhase = (usesScopedPhase || usesLegacyPhase) && frame ? projectPhase(frame) : null;
  if (actualPhase && !actualPhase.known) throw new UnsupportedMedia(`物品实际持有Job阶段未知：${actualPhase.issues.join('；')}`);
  const phase = usesScopedPhase || usesLegacyPhase ? observedPoint ?? actualPhase : null;
  if (phase) {
    const scoped = scopedPrefix && `${scopedPrefix}${phase.display_phase_id}`;
    const legacy = `${state.value}:${phase.display_phase_id}`;
    const qualified = scoped && presentation.timeline.some(clip => clip.phase_ref === scoped) ? scoped
      : usesLegacyPhase && presentation.timeline.some(clip => clip.phase_ref === legacy) ? legacy : null;
    if (qualified) return { ...phase, display_phase_id: qualified };
  }
  return { known: true, display_phase_id: state.value, progress: 1, terminal_pose: true, effective_ms: null, duration_ms: null };
}
async function resolveInput(input: NonNullable<FactoryDeviceMedia['media']>, lod: LodLevel) {
  let source = input.binding.request.source;
  const resolved = await resolveMedia({ ...input, lod, mode: 'asset_inspection', resolveModule: input.resolveModule ? async next => {
    const value = await input.resolveModule!(next); source = next; return value;
  } : undefined });
  return { resolved, source };
}
const ownerKey = (input: FactoryMediaInput, source: MediaAssetReadRequest['source']) => JSON.stringify([
  input.media!.binding.session_id, input.media!.binding.request.project_id, input.media!.binding.request.scope,
  input.media!.binding.request.scope_revision, source.kind, source.data.module_ref,
  source.kind === 'run_module_version' ? source.data.run_id : null,
]);
async function prepare(input: FactoryMediaInput, lod: LodLevel) {
  if (!input.media) throw new UnsupportedMedia(input.unavailable ?? '没有精确媒体绑定。');
  const { resolved, source } = await resolveInput(input.media, lod);
  let point: VisualProgress | null = null;
  if ('device_id' in input) {
    const actualPhase = input.frame ? projectPhase(input.frame) : null;
    if (actualPhase && !actualPhase.known) throw new UnsupportedMedia(`实际阶段未知：${actualPhase.issues.join('；')}`);
    point = input.observed_phase ? input.observed_phase.point : actualPhase;
  } else if ('item' in input) {
    point = factoryItemPoint(input, resolved.presentation);
  }
  const clip = point ? activeClip(resolved.presentation, point) : null;
  const asset = mediaAsset(resolved, clip);
  if (!('device_id' in input) && resolved.locator?.kind !== 'native_scene' && asset?.kind !== 'glb') throw new UnsupportedMedia('物品或资源附件只支持已声明GLB或原生场景，不补物理尺寸。');
  return { resolved, source, asset, point, clip, owner: ownerKey(input, source) };
}
const reusable = (before: Awaited<ReturnType<typeof prepare>>, next: Awaited<ReturnType<typeof prepare>>) =>
  before.owner === next.owner && before.resolved.presentation === next.resolved.presentation
  && (before.asset ? !!next.asset && assetKey(before.asset) === assetKey(next.asset) : next.asset === null);
async function modelMedia(initial: Awaited<ReturnType<typeof prepare>>, lod: LodLevel): Promise<FactoryMediaInstance> {
  const { resolved, source, asset, point, clip } = initial;
  const textureEdge = lod === 'simplified' ? Math.ceil(512 * devicePixelRatio) : undefined;
  const decoded = asset ? await decodeGlb(await resolved.read(asset), textureEdge) : undefined;
  const rig = await bindScene(resolved.presentation, resolved.read, decoded && asset ? { assetKey: assetKey(asset), decoded } : undefined, textureEdge);
  try { rig.apply(point, clip); } catch (error) { rig.dispose(); throw error; }
  const root = new THREE.Group(); root.rotation.x = Math.PI / 2; root.add(rig.root);
  root.userData.mqSource={asset,source,phase:point?.display_phase_id??null,progress:point?.progress??null,effective_ms:point?.effective_ms??null,duration_ms:point?.duration_ms??null};
  let live = true, epoch = 0;
  const instance: FactoryMediaInstance = { root, nodes: rig.nodes, presentation: resolved.presentation, asset, source,
    async update(input, lod) {
      const ticket = ++epoch;
      const next = await prepare(input, lod);
      if (!live || ticket !== epoch || !reusable(initial, next)) return false;
      root.userData.mqSource={asset:next.asset,source:next.source,phase:next.point?.display_phase_id??null,progress:next.point?.progress??null,effective_ms:next.point?.effective_ms??null,duration_ms:next.point?.duration_ms??null};
      rig.apply(next.point, next.clip); instance.source = next.source; return true;
    }, dispose() { if (!live) return; live = false; epoch++; rig.dispose(); } };
  return instance;
}
export interface FactoryMediaLabel { id: string; label: string; value: string; status: 'available' | 'unavailable'; anchor: AnchorRef }
/** The same canonical field reader used by the detail view/editor. */
export function factoryMediaLabels(presentation: ResolvedMedia['presentation'], input: FactoryDeviceMedia, source?: FactoryPresentationSource): FactoryMediaLabel[] {
  if (!presentation.tags.some(tag => tag.payload.kind === 'data')) return [];
  const actualPoint = input.frame ? projectPhase(input.frame) : null;
  const point = input.observed_phase ? input.observed_phase.point : actualPoint;
  return presentation.tags.flatMap(tag => {
    if (tag.payload.kind !== 'data') return [];
    if (tag.progress_range && !(point?.progress != null && point.progress >= tag.progress_range[0] && (point.progress < tag.progress_range[1] || point.progress === 1 && tag.progress_range[1] === 1))) return [];
    const device = source?.project.devices.find(device => device.id === input.device_id);
    const context = input.frame && actualPoint ? { mode: 'actual_run' as const, frame: input.frame, point: actualPoint }
      : source?.run && source.observation && device ? { mode: 'actual_device' as const, frame: { run: source.run, snapshot: source.observation, device, port_outputs: input.port_outputs } }
      : { mode: 'asset_inspection' as const };
    const value = resolveMediaData(tag.payload.data, context);
    return [{ id: tag.id, label: tag.label, value: value.status === 'available' ? value.formatted : `unavailable · ${value.message}`, status: value.status, anchor: tag.anchor }];
  });
}
export async function createFactoryItemMedia(input: FactoryActualItem, lod: LodLevel): Promise<FactoryMediaInstance> {
  // Finite instance state is read from the actual RuntimeItem at this seek time.
  // Qualified item clips read only the actual holding Job; no synthetic Job or clock.
  return modelMedia(await prepare(input, lod), lod);
}

export async function createFactoryResourceMedia(input: FactoryResourceMedia, lod: LodLevel): Promise<FactoryMediaInstance> {
  return modelMedia(await prepare(input, lod), lod);
}

/** One device's owned scene objects; no renderer, clock or business state. */
export async function createFactoryMedia(device: DeviceInstance, input: FactoryDeviceMedia, lod: LodLevel): Promise<FactoryMediaInstance> {
  const prepared = await prepare(input, lod);
  const { resolved, source, point, clip, asset } = prepared;
  // An idle/static asset has no fabricated cycle or phase progress.
  const root = new THREE.Group(); root.userData.ref = { kind: 'device', id: device.id };
  if (resolved.locator?.kind === 'native_scene' || asset?.kind === 'glb') {
    // Shared rigs are meters/Y-up; the parent physical hierarchy is meters/Z-up.
    return modelMedia(prepared, lod);
  }
  const xs = device.footprint_m.map(p => p.x), ys = device.footprint_m.map(p => p.y);
  const width = Math.max(...xs) - Math.min(...xs), depth = Math.max(...ys) - Math.min(...ys);
  if (!(width > 0 && depth > 0)) throw new UnsupportedMedia('图片画面缺少实际占地尺寸，不能虚构物理贴图大小。');
  // Reuse the unique controlled image/GIF/sequence/transition renderer as an
  // offscreen 2D texture, never a second WebGL viewport or autonomous GIF.
  const host = document.createElement('div');
  Object.assign(host.style, { position: 'fixed', left: '-10000px', top: '0', width: '512px', height: '512px', pointerEvents: 'none' });
  host.setAttribute('aria-hidden', 'true');
  let surface: Awaited<ReturnType<typeof createImageSurface>> | null = null;
  try {
    const textureEdge = lod === 'simplified' ? Math.ceil(512 * devicePixelRatio) : undefined;
    surface = await createImageSurface(host, resolved, asset, clip, () => {}, () => {}, undefined, textureEdge);
    document.body.append(host);
    await surface.draw(point, clip);
    const canvas = host.querySelector('canvas')!;
    const texture = new THREE.CanvasTexture(canvas); texture.colorSpace = THREE.SRGBColorSpace;
    const geometry = new THREE.PlaneGeometry(width, depth);
    const material = new THREE.MeshBasicMaterial({ map: texture, side: THREE.DoubleSide });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set((Math.max(...xs) + Math.min(...xs)) / 2, (Math.max(...ys) + Math.min(...ys)) / 2, device.height_m ?? 0);
    mesh.scale.set(resolved.presentation.scale.x, resolved.presentation.scale.y, resolved.presentation.scale.z);
    root.add(mesh);
    let live = true, epoch = 0;
    const instance: FactoryMediaInstance = { root, nodes: new Map(), presentation: resolved.presentation, asset, source,
      async update(nextInput, nextLod) {
        const ticket = ++epoch;
        const next = await prepare(nextInput, nextLod);
        if (!live || ticket !== epoch || !reusable(prepared, next) || prepared.clip?.clip.id !== next.clip?.clip.id) return false;
        await surface!.draw(next.point, next.clip);
        if (!live || ticket !== epoch) return false;
        texture.needsUpdate = true; instance.source = next.source; return true;
      }, dispose() { if (!live) return; live = false; epoch++; surface!.dispose(); host.remove(); texture.dispose(); material.dispose(); geometry.dispose(); } };
    return instance;
  } catch (error) { surface?.dispose(); host.remove(); throw error; }
}
