import * as THREE from 'three';
import { applyPose } from '../../../../packages/presentation/src';
import type { LodLevel } from '../../../../packages/commands/generated/typescript/LodLevel';
import type { FactoryActualItem } from '../services/factoryPresentation';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import { createFactoryItemMedia, type FactoryMediaInstance } from './factoryMedia';

/** Owned item attachments only; no stock count, guessed slots or movement. */
export function createFactoryItemLayer(items: FactoryActualItem[], lod: LodLevel,
  device: (id: string) => Pick<FactoryMediaInstance, 'root' | 'nodes' | 'presentation'> | null,
  location: (id: string) => THREE.Object3D | null,
  changed: () => void,
  transport: (id: string) => THREE.Object3D | null = () => null) {
  let live = true, updating = false, epoch = 0;
  const initial = (input: FactoryActualItem) => ({ input, pending: false, attempted: false, failed: false, ticket: 0, instance: null as FactoryMediaInstance | null,
    attachment: null as THREE.Group | null, offset: null as THREE.Group | null,
    message: input.unavailable ?? (lod === 'region' ? '区域档不绘制物品几何；实际ID保留。' : '等待明确附件节点。') });
  let states = items.map(initial);
  let byId = new Map(states.map(state => [state.input.item.id, state]));
  function target(input: FactoryActualItem): { parent: THREE.Object3D; zUp: boolean } | string | null {
    const { owner_ref, slot } = input.attachment!;
    if (Object.values(slot.offset.rotation).every(value => value === 0) || Object.values(slot.offset.scale).some(value => value === 0)) return 'unavailable · 展示槽旋转四元数或尺度退化。';
    if (owner_ref.kind === 'location' || owner_ref.kind === 'transport') {
      const parent = owner_ref.kind === 'transport' ? transport(owner_ref.id) : location(owner_ref.id);
      return parent ? { parent, zUp: true } : 'unavailable · 实际Location没有可用的物理坐标。';
    }
    const parentItem = owner_ref.kind === 'item' ? byId.get(owner_ref.id) : null;
    if (parentItem && (parentItem.input.unavailable || !parentItem.input.media || !parentItem.input.attachment)) return `unavailable · 父容器 ${owner_ref.id} 没有合法空间或精确媒体。`;
    if (parentItem?.instance && !parentItem.attachment?.parent) return null;
    const owner = owner_ref.kind === 'device' ? device(owner_ref.id) : parentItem?.instance;
    if (!owner) {
      if (parentItem) {
        const missing = parentItem.input.unavailable ?? (!parentItem.input.media || !parentItem.input.attachment ? '没有合法空间或3D媒体' : null);
        if (missing) return `unavailable · 父容器 ${owner_ref.id}：${missing}`;
        if (parentItem.message.startsWith('unavailable')) return `unavailable · 父容器 ${owner_ref.id}：${parentItem.message}`;
        const ancestor = target(parentItem.input);
        if (typeof ancestor === 'string') return `unavailable · 父容器 ${owner_ref.id}：${ancestor}`;
      }
      return null;
    }
    const anchor = slot.anchor_role ? owner.presentation.anchor_roles[slot.anchor_role] : null;
    if (!anchor) return `unavailable · ${lod}档最终精确媒体缺少命名角色 ${slot.anchor_role ?? '未声明'}。`;
    if (anchor.kind !== 'scene_node') return `unavailable · 角色 ${slot.anchor_role} 不是scene_node，不能作为3D支承点。`;
    const parent = owner.nodes.get(anchor.data);
    return parent ? { parent, zUp: false } : `unavailable · ${lod}档命名角色没有对应canonical节点。`;
  }
  function reconcile() {
    if (!live || updating || lod === 'region') return;
    let attached: boolean;
    do {
      attached = false;
      for (const state of states) {
        const { input } = state;
        if (input.unavailable || !input.attachment || !input.media) { state.attachment?.removeFromParent(); state.message = input.unavailable ?? 'unavailable · 没有合法空间或精确媒体。'; continue; }
        const selected = target(input);
        if (!selected) { state.message = 'unavailable · 实际holder模型/节点尚未提供，未绘制物品几何。'; continue; }
        if (typeof selected === 'string') { state.message = selected; continue; }
        if (state.instance && state.attachment && state.offset) {
          state.attachment.rotation.x = selected.zUp ? Math.PI / 2 : 0;
          applyPose(state.offset, input.attachment.slot.offset);
          if (state.attachment.parent !== selected.parent) { selected.parent.add(state.attachment); attached = true; }
          state.message = '已按冻结展示槽、实际holder及当前LOD命名节点附着（时点代表布局）。'; continue;
        }
        if (state.attempted || state.pending) continue;
        const ticket = ++state.ticket;
        state.pending = true; state.attempted = true; state.message = '读取精确物品素材中。';
        createFactoryItemMedia(input, lod).then(instance => {
          if (!live || ticket !== state.ticket) { instance.dispose(); return; }
          state.pending = false;
          const current = target(input);
          if (!current || typeof current === 'string' || current.parent !== selected.parent) { instance.dispose(); state.message = 'unavailable · 物品的实际附着节点已变化。'; changed(); return; }
          const attachment = new THREE.Group();
          // Shared node parents are Y-up; physical Locations are Z-up.
          if (selected.zUp) attachment.rotation.x = Math.PI / 2;
          const offset = new THREE.Group(); applyPose(offset, input.attachment!.slot.offset); attachment.add(offset);
          const basis = new THREE.Group(); basis.rotation.x = -Math.PI / 2; offset.add(basis); basis.add(instance.root);
          attachment.traverse(object => { object.userData.ref = { kind: 'item', id: input.item.id }; });
          selected.parent.add(attachment); state.instance = instance; state.attachment = attachment; state.offset = offset; state.message = '已按冻结展示槽、实际holder及当前LOD命名节点附着（时点代表布局）。';
          reconcile(); changed();
        }, reason => { if (live && ticket === state.ticket) { state.pending = false; state.failed = true; state.message = `unavailable · ${reason instanceof Error ? reason.message : String(reason)}`; reconcile(); changed(); } });
      }
    } while (attached && states.some(state => state.input.attachment?.owner_ref.kind === 'item'));
  }
  const detach = () => { updating = true; for (const state of states) state.attachment?.removeFromParent(); };
  return { reconcile, detach,
    async update(next: FactoryActualItem[]) {
      const current = ++epoch;
      detach();
      const previous = byId;
      states = next.map(input => {
        const state = previous.get(input.item.id) ?? initial(input);
        state.ticket++; state.input = input; state.pending = false; state.failed = false; state.attempted = false;
        return state;
      });
      byId = new Map(states.map(state => [state.input.item.id, state]));
      for (const [id, state] of previous) if (!byId.has(id)) { state.ticket++; state.instance?.dispose(); }
      await Promise.all(states.map(async state => {
        const input = state.input, instance = state.instance;
        if (!instance || input.unavailable || !input.media || !input.attachment) return;
        state.pending = true;
        try {
          const kept = await instance.update(input, lod);
          if (!live || current !== epoch) return;
          state.pending = false;
          if (!kept) { instance.dispose(); state.instance = null; state.attachment = null; state.offset = null; }
        } catch (reason) {
          if (!live || current !== epoch) return;
          instance.dispose(); state.instance = null; state.attachment = null; state.offset = null;
          state.pending = false; state.failed = true; state.attempted = true;
          state.message = `unavailable · ${reason instanceof Error ? reason.message : String(reason)}`;
        }
      }));
      if (!live || current !== epoch) return;
      updating = false; reconcile(); changed();
    },
    bounds(id: string) {
      const state = byId.get(id);
      if (!state?.instance || !state.attachment?.parent || ['transformed', 'scrapped'].includes(state.input.item.lifecycle)) return null;
      state.attachment.updateWorldMatrix(true, true);
      const bounds = new THREE.Box3().setFromObject(state.instance.root);
      return bounds.isEmpty() ? null : bounds;
    },
    statuses: () => Object.fromEntries(states.map(state => [state.input.item.id, state.message])),
    renderState(ownersSettled = false): { pending: number; objects: Array<{ ref: ObjectRef; root: THREE.Object3D }>; failures: Array<{ ref: ObjectRef; message: string }>; unavailable: Array<{ ref: ObjectRef; message: string }> } {
      const result = { pending: 0, objects: [] as Array<{ ref: ObjectRef; root: THREE.Object3D }>, failures: [] as Array<{ ref: ObjectRef; message: string }>, unavailable: [] as Array<{ ref: ObjectRef; message: string }> };
      if (!live) return result;
      for (const state of states) {
        const ref: ObjectRef = { kind: 'item', id: state.input.item.id };
        if (state.instance && state.attachment?.parent) result.objects.push({ ref, root: state.instance.root });
        else if (state.failed) result.failures.push({ ref, message: state.message });
        else if (state.pending) result.pending++;
        else if (!ownersSettled && lod !== 'region' && !state.input.unavailable && state.input.attachment && state.input.media && target(state.input) === null) result.pending++;
        else result.unavailable.push({ ref, message: state.message });
      }
      return result;
    },
    dispose() {
      if (!live) return;
      live = false; epoch++;
      // Detach every descendant first: an imported parent's disposer traverses
      // its own tree and must never dispose another item owner's resources.
      for (const state of states) state.attachment?.removeFromParent();
      for (const state of states) { state.ticket++; state.instance?.dispose(); }
    } };
}
