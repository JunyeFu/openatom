import * as THREE from 'three';
import type { CameraState } from '../../../../../packages/commands/generated/typescript/CameraState';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { LodLevel } from '../../../../../packages/commands/generated/typescript/LodLevel';

export interface FactoryRenderLimitation { object_ref: ObjectRef | null; message: string }
export interface FactorySceneFailure { key: string; stage: string; message: string; object_ref?: ObjectRef }
export interface FactorySceneFrame {
  key: string;
  frame_id: number;
  camera: CameraState;
  lod: LodLevel;
  pixels: { width: number; height: number; css_width: number; css_height: number; pixel_ratio: number };
  visible_objects: ObjectRef[];
  limitations: FactoryRenderLimitation[];
}
export interface FactorySceneCapture {
  key: string;
  camera: CameraState;
  onFrameReady(frame: FactorySceneFrame): void;
  onFailure(error: FactorySceneFailure): void;
}

/** The physical scene is converted once from meters/Z-up to Three's Y-up. */
export function captureCamera(state: CameraState) {
  const scale = state.unit === 'millimeter' ? .001 : 1;
  const vector = (value: CameraState['eye']) => state.coordinate_frame === 'right_handed_z_up'
    ? new THREE.Vector3(value.x, value.z, -value.y).multiplyScalar(scale)
    : new THREE.Vector3(value.x, value.y, value.z).multiplyScalar(scale);
  return { eye: vector(state.eye), target: vector(state.target), zoom: state.zoom };
}

/** Drawable geometry in the actual final hierarchy/layers and camera frustum.
 * This is geometric visibility, not an occlusion or raster-ID pass. DOM lists
 * and annotations never add an ObjectRef to this set. Batched logical arrows
 * report their existing Connection identities using their own drawn segments. */
export function visibleFactoryObjects(scene: THREE.Scene, camera: THREE.Camera): ObjectRef[] {
  scene.updateMatrixWorld(true); camera.updateMatrixWorld(true);
  const frustum = new THREE.Frustum().setFromProjectionMatrix(new THREE.Matrix4().multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse));
  const found = new Map<string, ObjectRef>();
  scene.traverse(object => {
    if (!(object instanceof THREE.Mesh || object instanceof THREE.Line || object instanceof THREE.Points || object instanceof THREE.Sprite)) return;
    if (!camera.layers.test(object.layers)) return;
    if (!(object instanceof THREE.Sprite) && (object.geometry.drawRange.count === 0 || !object.geometry.getAttribute('position')?.count)) return;
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    if (!materials.some(material => material.visible && material.opacity > 0)) return;
    let parent: THREE.Object3D | null = object, reference: ObjectRef | undefined;
    while (parent) {
      if (!parent.visible) return;
      reference ??= parent.userData.ref as ObjectRef | undefined;
      parent = parent.parent;
    }
    const connections = object.userData.connectionRefs as ObjectRef[] | undefined;
    if (object instanceof THREE.LineSegments && connections) {
      const positions = object.geometry.getAttribute('position');
      const { start, count } = object.geometry.drawRange;
      const end = Math.min(positions.count, start + count);
      const a = new THREE.Vector3(), b = new THREE.Vector3();
      connections.forEach((ref, row) => {
        for (let vertex = row * 6; vertex < (row + 1) * 6; vertex += 2) {
          if (vertex < start || vertex + 1 >= end) continue;
          a.fromBufferAttribute(positions, vertex).applyMatrix4(object.matrixWorld);
          b.fromBufferAttribute(positions, vertex + 1).applyMatrix4(object.matrixWorld);
          let enter = 0, leave = 1;
          for (const plane of frustum.planes) {
            const from = plane.distanceToPoint(a), to = plane.distanceToPoint(b);
            if (from < 0 && to < 0) { enter = 1; leave = 0; break; }
            if (from < 0) enter = Math.max(enter, from / (from - to));
            else if (to < 0) leave = Math.min(leave, from / (from - to));
          }
          if (enter <= leave) { found.set(`${ref.kind}:${ref.id}`, ref); break; }
        }
      });
      return;
    }
    if (!reference || !(object instanceof THREE.Sprite ? frustum.intersectsSprite(object) : frustum.intersectsObject(object))) return;
    found.set(`${reference.kind}:${reference.id}`, reference);
  });
  return [...found.values()].sort((a, b) => `${a.kind}:${a.id}`.localeCompare(`${b.kind}:${b.id}`));
}
