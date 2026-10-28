import * as THREE from 'three';
import type { LodLevel } from '../../../../packages/commands/generated/typescript/LodLevel';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { FactoryResourceMedia } from '../services/factoryPresentation';
import { createFactoryResourceMedia } from './factoryMedia';

/** Stationary reusable carriers at their declared home Location, in the existing scene. */
export function createFactoryResourceLayer(resources: FactoryResourceMedia[], lod: LodLevel,
  location: (id: string) => THREE.Object3D | null, changed: () => void) {
  let live = true, epoch = 0;
  const initial = (input: FactoryResourceMedia) => ({ input, pending: false, attempted: false, failed: false, ticket: 0,
    instance: null as { root: THREE.Object3D; update(input: FactoryResourceMedia, lod: LodLevel): Promise<boolean>; dispose(): void } | null, parent: null as THREE.Object3D | null,
    message: input.unavailable ?? (lod === 'region' ? '区域档不绘制资源几何。' : '等待声明的资源位置。') });
  let states = resources.map(initial);
  function drawWorker(input: FactoryResourceMedia, ctx: CanvasRenderingContext2D) {
    const status = input.worker_status!;
    ctx.clearRect(0,0,640,140);
    ctx.fillStyle = '#ffffffee'; ctx.fillRect(0,0,640,140);
    ctx.fillStyle = status.kind === 'claimed' ? '#ac5708' : status.kind === 'available' ? '#23644d' : '#516170';
    ctx.beginPath(); ctx.arc(48,40,16,0,Math.PI*2); ctx.fill();
    ctx.beginPath(); ctx.ellipse(48,91,30,29,0,Math.PI,0); ctx.fill();
    ctx.font = '30px "Microsoft YaHei", sans-serif';
    ctx.fillText('人员位置标记 · '+input.resource.role,96,51);
    ctx.fillText(status.kind === 'claimed' ? '占用 · '+status.job_ids.length+'作业' : status.kind === 'available' ? '本时点未占用' : '占用尚未读取',96,103);
  }
  function reconcile() {
    if (!live || lod === 'region') return;
    for (const state of states) {
      if (state.input.unavailable || !state.input.media && !state.input.worker_status) { state.instance?.root.removeFromParent(); state.message = state.input.unavailable ?? 'unavailable · 没有精确资源媒体。'; continue; }
      const parent = state.input.resource.home_location ? location(state.input.resource.home_location) : null;
      if (!parent) { state.message = 'unavailable · 资源home_location没有实际空间节点。'; continue; }
      if (state.instance) { parent.add(state.instance.root); state.parent = parent; continue; }
      if (state.attempted || state.pending) continue;
      if (state.input.worker_status) {
        const canvas = document.createElement('canvas'); canvas.width = 640; canvas.height = 140;
        const ctx = canvas.getContext('2d')!;
        drawWorker(state.input, ctx);
        const texture = new THREE.CanvasTexture(canvas), material = new THREE.SpriteMaterial({ map: texture, depthTest: false });
        const root = new THREE.Sprite(material); root.scale.set(1.8,.394,1); root.position.z=.35;
        root.userData.ref={kind:'resource',id:state.input.resource.id};
        parent.add(root); state.instance={root,async update(input) { if (!input.worker_status) return false; drawWorker(input,ctx); texture.needsUpdate=true; return true; },dispose(){texture.dispose();material.dispose();}}; state.attempted=true;
        state.message='人员声明home位置与实际JobsAt资源持有；不推断行走轨迹或工时。'; continue;
      }
      const ticket = ++state.ticket;
      state.attempted = true; state.pending = true;
      createFactoryResourceMedia(state.input, lod).then(instance => {
        if (!live || ticket !== state.ticket) { instance.dispose(); return; }
        instance.root.traverse(object => { object.userData.ref = { kind: 'resource', id: state.input.resource.id }; });
        parent.add(instance.root); state.instance = instance; state.parent = parent;
        state.pending = false; state.message = '已按精确Resources媒体及声明home_location附着。'; changed();
      }, error => {
        if (!live || ticket !== state.ticket) return;
        state.pending = false; state.failed = true;
        state.message = error instanceof Error ? error.message : String(error); changed();
      });
    }
  }
  return { reconcile,
    async update(next: FactoryResourceMedia[]) {
      const current = ++epoch, previous = new Map(states.map(state => [state.input.resource.id, state]));
      for (const state of states) state.instance?.root.removeFromParent();
      states = next.map(input => {
        const state = previous.get(input.resource.id) ?? initial(input);
        state.ticket++; state.input=input; state.attempted=false; state.pending=false; state.failed=false;
        return state;
      });
      const ids = new Set(next.map(input => input.resource.id));
      for (const [id,state] of previous) if (!ids.has(id)) { state.ticket++; state.instance?.dispose(); }
      await Promise.all(states.map(async state => {
        const instance = state.instance, input = state.input;
        if (!instance || input.unavailable || !input.media && !input.worker_status) return;
        state.pending = true;
        try {
          const kept = await instance.update(input,lod);
          if (!live || current !== epoch) return;
          state.pending = false;
          if (!kept) { instance.dispose(); state.instance=null; }
        } catch (reason) {
          if (!live || current !== epoch) return;
          instance.dispose(); state.instance=null; state.pending=false; state.failed=true; state.attempted=true;
          state.message = reason instanceof Error ? reason.message : String(reason);
        }
      }));
      if (!live || current !== epoch) return;
      reconcile(); changed();
    },
    statuses: () => Object.fromEntries(states.map(state => [state.input.resource.id, state.message])),
    renderState() {
      const result = { pending: 0, objects: [] as Array<{ ref: ObjectRef; root: THREE.Object3D }>,
        failures: [] as Array<{ ref: ObjectRef; message: string }>, unavailable: [] as Array<{ ref: ObjectRef; message: string }> };
      if (!live) return result;
      for (const state of states) {
        const ref: ObjectRef = { kind: 'resource', id: state.input.resource.id };
        if (state.instance?.root.parent) result.objects.push({ ref, root: state.instance.root });
        else if (state.pending) result.pending++;
        else (state.failed ? result.failures : result.unavailable).push({ ref, message: state.message });
      }
      return result;
    },
    dispose() {
      if (!live) return;
      live = false; epoch++;
      for (const state of states) state.instance?.root.removeFromParent();
      for (const state of states) { state.ticket++; state.instance?.dispose(); }
    } };
}
