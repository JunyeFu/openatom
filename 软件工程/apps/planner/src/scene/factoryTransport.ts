import * as THREE from 'three';
import type { FactoryTransport } from '../services/factoryPresentation';
import type { HallDesign } from '../../../../packages/commands/generated/typescript/HallDesign';
import { poseMatrix } from './layout';

/** A transport marker, carrying actual item models. It is not an additional
 * manufacturing machine or a second owner of cargo/vehicle resources. */
export function createFactoryTransportLayer(inputs: FactoryTransport[], halls: HallDesign[], includeOrigins: boolean) {
  const root = new THREE.Group(), hallRoots = new Map<string, THREE.Group>(), anchors = new Map<string, THREE.Group>();
  for (const hall of halls) {
    const group = new THREE.Group();
    if (includeOrigins) { group.matrix.copy(poseMatrix(hall.origin)); group.matrixAutoUpdate = false; }
    root.add(group); hallRoots.set(hall.id, group);
  }
  const geometry = new THREE.BoxGeometry(1.1, .8, .12), material = new THREE.MeshStandardMaterial({ color: 0x087f8c, roughness: .65 });
  const wheelGeometry = new THREE.CylinderGeometry(.12, .12, .1, 12), wheelMaterial = new THREE.MeshStandardMaterial({ color: 0x344251 });
  const legGeometry = new THREE.BoxGeometry(.06, .06, 1);
  function update(next: FactoryTransport[]) {
    const retained = new Set<string>();
    for (const input of next) {
      const parent = input.hall_id ? hallRoots.get(input.hall_id) : null;
      if (!parent || !input.pose) continue;
      retained.add(input.trip.id);
      let group = anchors.get(input.trip.id);
      if (!group) {
        group = new THREE.Group(); group.userData.ref = { kind: 'transport', id: input.trip.id };
        const deck = new THREE.Mesh(geometry, material); deck.position.z = -.06; group.add(deck);
        for (const x of [-.4, .4]) for (const y of [-.38, .38]) {
          const wheel = new THREE.Mesh(wheelGeometry, wheelMaterial); wheel.position.set(x, y, 0); wheel.userData.part='wheel'; group.add(wheel);
          const leg = new THREE.Mesh(legGeometry, wheelMaterial); leg.position.set(x, y, 0); leg.userData.part='leg'; group.add(leg);
        }
        parent.add(group); anchors.set(input.trip.id, group);
      }
      group.matrix.copy(poseMatrix(input.pose)); group.matrixAutoUpdate = false; group.matrixWorldNeedsUpdate = true;
      const height=input.pose.translation.z;
      for(const part of group.children) {
        if(part.userData.part==='wheel')part.position.z=.12-height;
        if(part.userData.part==='leg'){part.scale.z=Math.max(0,height-.32);part.position.z=(.08-height)/2;}
      }
      group.userData.transport_phase = input.trip.phase;
    }
    for (const [id, group] of anchors) if (!retained.has(id)) { group.removeFromParent(); anchors.delete(id); }
  }
  update(inputs);
  return { root, update, anchor: (id: string) => anchors.get(id) ?? null,
    dispose() { root.removeFromParent(); anchors.clear(); geometry.dispose(); material.dispose(); wheelGeometry.dispose(); wheelMaterial.dispose(); legGeometry.dispose(); } };
}
