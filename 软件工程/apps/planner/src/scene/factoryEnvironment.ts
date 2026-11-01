import * as THREE from 'three';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import { poseMatrix } from './layout';

type Hall = ProjectState['halls'][number];
type Location = ProjectState['locations'][number];

export const factoryRegionColors = [0xdeecf7, 0xdcefe9, 0xf7e8cb, 0xeadff3, 0xe4e7f6, 0xf5dfd7, 0xe4efd6, 0xf5edcf, 0xd8eef0, 0xdcebd9, 0xe0e6eb, 0xf0e1d8];

/** A cutaway presentation of the declared envelope, with no simulation objects. */
export function createFactoryHallShell(hall: Hall) {
  const root = new THREE.Group(); root.name = 'hall-shell:' + hall.id;
  const w = hall.width_m, d = hall.depth_m, h = hall.height_m;
  if (h == null || h <= 0) return root;
  const thickness = Math.min(.18, w * .02, d * .02, h * .02);
  const steel = new THREE.MeshStandardMaterial({ color: 0x71818e, metalness: .35, roughness: .65 });
  const wall = new THREE.MeshStandardMaterial({ color: 0xd4dce1, roughness: .9 });
  function box(name: string, x: number, y: number, z: number, sx: number, sy: number, sz: number, material: THREE.Material) {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(sx, sy, sz), material);
    mesh.name = name; mesh.position.set(x, y, z); root.add(mesh);
  }
  const nx = Math.ceil(w / 18), ny = Math.ceil(d / 18);
  for (let i = 0; i <= nx; i++) for (const y of [thickness / 2, d - thickness / 2])
    box('PerimeterColumn', thickness / 2 + (w - thickness) * i / nx, y, h / 2, thickness, thickness, h, steel);
  for (let i = 1; i < ny; i++) for (const x of [thickness / 2, w - thickness / 2])
    box('PerimeterColumn', x, thickness / 2 + (d - thickness) * i / ny, h / 2, thickness, thickness, h, steel);
  for (const y of [thickness / 2, d - thickness / 2]) box('RoofEdge', w / 2, y, h - thickness / 2, w, thickness, thickness, steel);
  for (const x of [thickness / 2, w - thickness / 2]) box('RoofEdge', x, d / 2, h - thickness / 2, thickness, d, thickness, steel);
  const wallHeight = Math.min(h, 2.8);
  box('BackWallCutaway', w / 2, d - thickness / 2, wallHeight / 2, w, thickness, wallHeight, wall);
  for (const x of [thickness / 2, w - thickness / 2]) box('SideWallCutaway', x, d / 2, wallHeight / 2, thickness, d, wallHeight, wall);
  root.userData.presentation_only = true;
  return root;
}

function contains(point: { x: number; y: number }, polygon: THREE.Vector3[]) {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i], b = polygon[j];
    if ((a.y > point.y) !== (b.y > point.y) && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x) inside = !inside;
  }
  return inside;
}

/** Free storage only; a machine's declared footprint already provides its support. */
export function factoryStockSupportLayout(project: ProjectState, location: Location) {
  const pose = location.position, slots = location.item_layout?.slots;
  if (pose.translation.z <= 0 || pose.rotation.x !== 0 || pose.rotation.y !== 0 || !slots?.length) return null;
  for (const device of project.devices.filter(d => d.hall_id === location.hall_id)) {
    const line = device.line_id ? project.lines.find(l => l.id === device.line_id) : null;
    if (device.line_id && !line) continue;
    const matrix = line ? poseMatrix(line.transform).multiply(poseMatrix(device.pose)) : poseMatrix(device.pose);
    const footprint = device.footprint_m.map(p => new THREE.Vector3(p.x, p.y, 0).applyMatrix4(matrix));
    if (contains(pose.translation, footprint)) return null;
  }
  const points = slots.map(s => s.offset.translation);
  return { minX: Math.min(...points.map(p => p.x)) - .3, maxX: Math.max(...points.map(p => p.x)) + .3,
    minY: Math.min(...points.map(p => p.y)) - .3, maxY: Math.max(...points.map(p => p.y)) + .3,
    height: pose.translation.z / pose.scale.z };
}

/** Local Z=0 is the location's existing work surface; item poses stay unchanged. */
export function createFactoryStockSupport(project: ProjectState, location: Location) {
  const layout = factoryStockSupportLayout(project, location); if (!layout) return null;
  const root = new THREE.Group(); root.name = 'stock-support:' + location.id;
  const top = Math.min(.05, layout.height / 3), width = layout.maxX - layout.minX, depth = layout.maxY - layout.minY;
  const deck = new THREE.MeshStandardMaterial({ color: location.purpose === 'output' ? 0x9daeb1 : 0xb4bec3, metalness: .25, roughness: .7 });
  const frame = new THREE.MeshStandardMaterial({ color: 0x657784, metalness: .4, roughness: .6 });
  function box(name: string, sx: number, sy: number, sz: number, x: number, y: number, z: number, material: THREE.Material) {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(sx, sy, sz), material); mesh.name = name; mesh.position.set(x, y, z); root.add(mesh);
  }
  box('StorageDeck', width, depth, top, (layout.minX + layout.maxX) / 2, (layout.minY + layout.maxY) / 2, -top / 2, deck);
  for (const x of [layout.minX + .07, layout.maxX - .07]) for (const y of [layout.minY + .07, layout.maxY - .07])
    box('StorageLeg', .05, .05, layout.height - top, x, y, -(layout.height + top) / 2, frame);
  root.userData.presentation_only = true;
  return root;
}
