import { Mesh, Object3D, PerspectiveCamera, Raycaster, Scene, Vector2, Vector3 } from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { TransformControls } from 'three/addons/controls/TransformControls.js';
import type { Pose } from '../../../../packages/commands/generated/typescript/Pose';

export interface LayoutControlOptions {
  mode: 'translate' | 'rotate';
  snap: boolean;
  step_m: number;
}

/** A layout handle edits the selected declared object, never the scene camera. */
export function attachLayoutControls(scene: Scene, camera: PerspectiveCamera, canvas: HTMLCanvasElement,
  orbit: OrbitControls, object: Object3D, original: Pose, options: LayoutControlOptions,
  draw: () => void, commit: (pose: Pose) => Promise<void>, failure: (error: unknown) => void) {
  object.matrix.decompose(object.position, object.quaternion, object.scale);
  object.matrixAutoUpdate = true;
  const controls = new TransformControls(camera, canvas);
  controls.setMode(options.mode); controls.setSpace('local');
  controls.setTranslationSnap(options.snap ? options.step_m : null);
  controls.setRotationSnap(options.snap ? Math.PI / 12 : null);
  const helper = controls.getHelper(); scene.add(helper); controls.attach(object);
  let live = true, changed = false, pointer: number | null = null;
  const pointerDown = (event: PointerEvent) => {
    if (event.button === 0 && controls.enabled && !controls.dragging) pointer = event.pointerId;
  };
  const pointerUp = (event: PointerEvent) => {
    if (controls.dragging && event.pointerId !== pointer) { event.stopImmediatePropagation(); return; }
    if (event.pointerId === pointer) pointer = null;
  };
  const cancel = () => {
    const captured = pointer; pointer = null;
    if (controls.dragging) controls.reset();
    controls.dragging = false; controls.axis = null; changed = false; orbit.enabled = true;
    if (captured !== null && canvas.hasPointerCapture(captured)) canvas.releasePointerCapture(captured);
    draw();
  };
  const pointerCancel = (event: PointerEvent) => { if (event.pointerId === pointer) cancel(); };
  const keyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && controls.dragging) { event.preventDefault(); event.stopPropagation(); cancel(); }
  };
  canvas.addEventListener('pointerdown', pointerDown, true);
  canvas.addEventListener('pointerup', pointerUp, true);
  canvas.addEventListener('pointercancel', pointerCancel);
  canvas.addEventListener('lostpointercapture', pointerCancel);
  canvas.ownerDocument.addEventListener('keydown', keyDown, true);
  controls.addEventListener('mouseDown', () => { changed = false; orbit.enabled = false; });
  controls.addEventListener('objectChange', () => { changed = true; draw(); });
  controls.addEventListener('change', draw);
  controls.addEventListener('mouseUp', () => {
    orbit.enabled = true;
    if (!changed || !live) return;
    changed = false;
    const pose: Pose = { ...original, translation: { x: object.position.x, y: object.position.y, z: object.position.z },
      rotation: { x: object.quaternion.x, y: object.quaternion.y, z: object.quaternion.z, w: object.quaternion.w },
      scale: { x: object.scale.x, y: object.scale.y, z: object.scale.z } };
    controls.enabled = false;
    commit(pose).catch(error => { if (live) failure(error); }).finally(() => { if (live) { controls.enabled = true; draw(); } });
  });
  return { active: () => controls.axis !== null || controls.dragging,
    // On-demand receiver for actual Native pointer acceptance; no gesture or render is scheduled.
    diagnostics() {
      scene.updateMatrixWorld(true);
      const gizmo = helper.children.find(child => child.type === 'TransformControlsGizmo') as Object3D & { picker: Record<string, Object3D> };
      const picker = gizmo.picker[options.mode], rect = canvas.getBoundingClientRect(), ray = new Raycaster();
      const points: { x: number; y: number; axis: string; triangle: number }[] = [];
      for (const part of picker.children) {
        if (!(part instanceof Mesh) || !part.visible || !['X', 'Y', 'Z'].includes(part.name)) continue;
        const positions = part.geometry.getAttribute('position'), indices = part.geometry.index;
        for (let i = 0; i < (indices?.count ?? positions.count) && points.length < 12; i += 3) {
          const center = new Vector3();
          for (let j = 0; j < 3; j++) center.add(new Vector3().fromBufferAttribute(positions, indices ? indices.getX(i + j) : i + j));
          center.divideScalar(3).applyMatrix4(part.matrixWorld).project(camera);
          if (Math.abs(center.x) >= 1 || Math.abs(center.y) >= 1 || center.z < -1 || center.z > 1) continue;
          ray.setFromCamera(new Vector2(center.x, center.y), camera);
          const hit = ray.intersectObject(picker, true).find(value => value.object.visible);
          if (hit?.object.name === part.name) points.push({ x: rect.left + (center.x + 1) * rect.width / 2, y: rect.top + (1 - center.y) * rect.height / 2, axis: part.name, triangle: i / 3 });
        }
      }
      return { mode: options.mode, dragging: controls.dragging, axis: controls.axis, position: object.position.toArray(), quaternion: object.quaternion.toArray(), scale: object.scale.toArray(), points };
    },
    dispose() {
      live = false; cancel();
      canvas.removeEventListener('pointerdown', pointerDown, true);
      canvas.removeEventListener('pointerup', pointerUp, true);
      canvas.removeEventListener('pointercancel', pointerCancel);
      canvas.removeEventListener('lostpointercapture', pointerCancel);
      canvas.ownerDocument.removeEventListener('keydown', keyDown, true);
      controls.detach(); scene.remove(helper); controls.dispose();
    } };
}
