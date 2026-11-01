import { Matrix4, Quaternion, Vector3 } from 'three';
import { presentationBasis, presentationPoseMatrix } from '../../scene/modulePresentation';
import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { Pose } from '../../../../../packages/commands/generated/typescript/Pose';
export function convertPoseUnit(pose: Pose, unit: Pose['unit']): Pose {
  const factor = pose.unit === unit ? 1 : unit === 'meter' ? .001 : 1000;
  return { ...pose, unit, translation: { x: pose.translation.x * factor, y: pose.translation.y * factor, z: pose.translation.z * factor } };
}
import type { SceneNode } from '../../../../../packages/commands/generated/typescript/SceneNode';

export const identityPose = (unit: Pose['unit'] = 'millimeter', frame: Pose['coordinate_frame'] = 'right_handed_z_up'): Pose => ({ translation: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0, w: 1 }, scale: { x: 1, y: 1, z: 1 }, unit, coordinate_frame: frame });
export function newPart(presentation: Presentation, kind: 'box' | 'cylinder' | 'plane'): SceneNode {
  const size = presentation.unit === 'meter' ? 0.5 : 500;
  return { id: Math.max(0, ...presentation.nodes.map(node => node.id)) + 1, name: `部件 ${presentation.nodes.length + 1}`, parent_index: null, transform: identityPose(presentation.unit, presentation.coordinate_frame), geometry: kind === 'box' ? { kind, data: { x: size, y: size, z: size } } : kind === 'cylinder' ? { kind, data: { radius: size / 2, height: size } } : { kind, data: { x: size, y: size } }, material: { color_rgba: [0.42, 0.58, 0.72, 1], metallic: 0, roughness: 0.65, texture: null } };
}
function matrix(pose: Pose): Matrix4 {
  return presentationPoseMatrix(pose, 'right_handed_z_up');
}
function transformed(value: Matrix4, original: Pose): Pose {
  const basis = presentationBasis(original.coordinate_frame);
  const local = basis.clone().invert().multiply(value).multiply(basis);
  const position = new Vector3(), rotation = new Quaternion(), scale = new Vector3();
  local.decompose(position, rotation, scale);
  const reconstructed = new Matrix4().compose(position, rotation, scale);
  if (local.elements.some((element, index) => Math.abs(element - reconstructed.elements[index]) > 1e-9)) throw new Error('该组合产生剪切，无法用现有位置、旋转和缩放保持原姿态；请先调整部件变换。');
  const factor = original.unit === 'meter' ? 1 : 1000;
  return { ...original, translation: { x: position.x * factor, y: position.y * factor, z: position.z * factor }, rotation: { x: rotation.x, y: rotation.y, z: rotation.z, w: rotation.w }, scale: { x: scale.x, y: scale.y, z: scale.z } };
}
function compatible(nodes: SceneNode[]): void {
  if (nodes.some(node => [...Object.values(node.transform.translation), ...Object.values(node.transform.rotation), ...Object.values(node.transform.scale)].some(value => !Number.isFinite(value)))) throw new Error('先补齐有限的位置、角度和缩放数值。');
  if (nodes.some(node => node.transform.unit !== nodes[0].transform.unit || node.transform.coordinate_frame !== nodes[0].transform.coordinate_frame)) throw new Error('先明确统一所选部件的单位与坐标系，再编组。');
  if (nodes.some(node => Object.values(node.transform.scale).some(value => value === 0))) throw new Error('缩放为零的部件不能作为编组变换。');
}
/** Parent the remaining selection to its first member, preserving world pose. */
export function groupParts(nodes: SceneNode[], selection: number[]): SceneNode[] {
  if (selection.length < 2) throw new Error('至少选择两个部件。');
  const selected = selection.map(index => nodes[index]); compatible(selected);
  if (selected.some(node => (node.parent_index ?? null) !== (selected[0].parent_index ?? null))) throw new Error('先选择同一父级下的部件；不会自动跨层重绑。');
  const inverse = matrix(selected[0].transform).invert();
  return nodes.map((node, index) => selection.includes(index) && index !== selection[0] ? { ...node, parent_index: selection[0], transform: transformed(inverse.clone().multiply(matrix(node.transform)), node.transform) } : node);
}
export function ungroupParts(nodes: SceneNode[], parent: number): SceneNode[] {
  const children = nodes.filter(node => node.parent_index === parent); compatible([nodes[parent], ...children]);
  const parentMatrix = matrix(nodes[parent].transform);
  return nodes.map(node => node.parent_index === parent ? { ...node, parent_index: nodes[parent].parent_index ?? null, transform: transformed(parentMatrix.clone().multiply(matrix(node.transform)), node.transform) } : node);
}
export function copyParts(nodes: SceneNode[], selection: number[]): SceneNode[] {
  let nextId = Math.max(0, ...nodes.map(node => node.id));
  const remap = new Map(selection.map((index, offset) => [index, nodes.length + offset]));
  const copies = selection.map(index => ({ ...structuredClone(nodes[index]), id: ++nextId, name: `${nodes[index].name} · 副本`, parent_index: remap.get(nodes[index].parent_index ?? -1) ?? nodes[index].parent_index ?? null }));
  return [...nodes, ...copies];
}
export function moveParts(nodes: SceneNode[], selection: number[], delta: { x: number; y: number; z: number }): SceneNode[] {
  compatible(selection.map(index => nodes[index]));
  const rootSelection = selection.filter(index => {
    let parent = nodes[index].parent_index; const visited = new Set<number>();
    while (parent != null) { if (selection.includes(parent)) return false; if (visited.has(parent)) throw new Error('部件父级存在循环，请先修正。'); visited.add(parent); parent = nodes[parent]?.parent_index; }
    return true;
  });
  return nodes.map((node, index) => {
    if (!rootSelection.includes(index)) return node;
    const path: number[] = []; let parent = node.parent_index;
    while (parent != null) {
      if (path.includes(parent)) throw new Error('部件父级存在循环，请先修正。');
      path.push(parent); parent = nodes[parent].parent_index;
    }
    const parentWorld = new Matrix4();
    for (const ancestor of path.reverse()) parentWorld.multiply(matrix(nodes[ancestor].transform));
    if (!Number.isFinite(parentWorld.determinant()) || parentWorld.determinant() === 0) throw new Error('父级缩放不可逆，不能计算共同坐标中的整体移动。');
    const unit = node.transform.unit === 'meter' ? 1 : .001;
    const inverse = parentWorld.invert();
    const localDelta = new Vector3(delta.x * unit, delta.y * unit, delta.z * unit).applyMatrix4(inverse).sub(new Vector3().applyMatrix4(inverse)).applyMatrix4(presentationBasis(node.transform.coordinate_frame).invert());
    return { ...node, transform: { ...node.transform, translation: { x: node.transform.translation.x + localDelta.x / unit, y: node.transform.translation.y + localDelta.y / unit, z: node.transform.translation.z + localDelta.z / unit } } };
  });
}
export function removalDependencies(presentation: Presentation, index: number): string[] {
  const uses: string[] = [];
  presentation.nodes.forEach((node, child) => { if (child !== index && node.parent_index != null && node.parent_index >= index) uses.push(`部件 ${child} 的父级`); });
  presentation.actions.forEach(action => { if (action.node_index >= index) uses.push(`动作 ${action.id}`); });
  presentation.poses.forEach(pose => { if (pose.node_transforms.some(transform => transform.node_index >= index)) uses.push(`姿态 ${pose.id}`); });
  for (const [role, anchor] of Object.entries({ ...presentation.anchor_roles, ...presentation.port_anchors })) if (anchor.kind === 'scene_node' && anchor.data >= index) uses.push(`锚点 ${role}`);
  presentation.tags.forEach(tag => { if (tag.anchor.kind === 'scene_node' && tag.anchor.data >= index) uses.push(`标签 ${tag.label}`); });
  return uses;
}
