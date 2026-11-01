import * as THREE from 'three';
import { poseMatrix } from './layout';
import type { Presentation } from '../../../../packages/commands/generated/typescript/Presentation';
import type { Pose } from '../../../../packages/commands/generated/typescript/Pose';
import type { CoordinateFrame } from '../../../../packages/commands/generated/typescript/CoordinateFrame';
import type { AssetRef } from '../../../../packages/commands/generated/typescript/AssetRef';

export interface PresentationDiagnostic {
  code: 'invalid_geometry' | 'invalid_transform' | 'invalid_material' | 'invalid_parent' | 'duplicate_id' | 'missing_asset' | 'static_preview';
  message: string;
  node_id?: number;
  asset?: AssetRef;
}
export interface ModulePresentationScene {
  root: THREE.Group;
  nodes: Map<number, THREE.Group>;
  diagnostics: PresentationDiagnostic[];
  nativeMeshCount: number;
  dispose(): void;
}
const meters = (unit: Pose['unit']) => unit === 'millimeter' ? .001 : 1;
/** Source coordinates to the workbench's physical right-handed Z-up basis. */
export function presentationBasis(frame: CoordinateFrame): THREE.Matrix4 {
  return frame === 'right_handed_y_up' ? new THREE.Matrix4().makeRotationX(Math.PI / 2) : new THREE.Matrix4();
}
/** A Pose declares its own translation unit and coordinate basis; geometry uses Presentation.unit. */
export function presentationPoseMatrix(pose: Pose, frame: CoordinateFrame): THREE.Matrix4 {
  const factor = meters(pose.unit);
  const q = new THREE.Quaternion(pose.rotation.x, pose.rotation.y, pose.rotation.z, pose.rotation.w);
  const numbers = [...Object.values(pose.translation), ...Object.values(pose.rotation), ...Object.values(pose.scale)];
  if (!numbers.every(Number.isFinite) || q.lengthSq() === 0) throw new Error('姿态须使用有限数值和非零旋转四元数。');
  q.normalize();
  const local = poseMatrix({ ...pose, translation: { x: pose.translation.x * factor, y: pose.translation.y * factor, z: pose.translation.z * factor }, rotation: { x: q.x, y: q.y, z: q.z, w: q.w }, unit: 'meter' });
  const basis = presentationBasis(frame).invert().multiply(presentationBasis(pose.coordinate_frame));
  return basis.clone().multiply(local).multiply(basis.clone().invert());
}

/** Creates only declared native meshes. No asset URL/path is fetched here. */
export function buildModulePresentation(value: Presentation): ModulePresentationScene {
  const root = new THREE.Group(); root.name = 'definition-presentation-preview'; root.matrixAutoUpdate = false;
  const diagnostics: PresentationDiagnostic[] = [], nodes = new Map<number, THREE.Group>();
  const meshes: THREE.Mesh<THREE.BufferGeometry, THREE.MeshStandardMaterial>[] = [];
  const scale = Object.values(value.scale);
  if (!scale.every(Number.isFinite)) diagnostics.push({ code: 'invalid_transform', message: '表现根缩放存在未填写或非有限数值，当前几何不可绘制。' });
  else root.matrix.copy(presentationBasis(value.coordinate_frame).multiply(new THREE.Matrix4().makeScale(value.scale.x, value.scale.y, value.scale.z)));
  const missing = (asset: AssetRef, message: string, node_id?: number) => diagnostics.push({ code: 'missing_asset', asset, node_id, message: `${message}：${asset.id} · ${asset.path}` });
  if (value.source.kind === 'asset') missing(value.source.data, '尚未接入此源素材的实际授权字节');
  if (value.source.kind === 'image_sequence') value.source.data.forEach(asset => missing(asset, '尚未接入序列图片的实际授权字节'));
  if (value.source.kind === 'media_ref') diagnostics.push({ code: 'missing_asset', message: `尚未解析精确媒体版本 ${value.source.data.id}@${value.source.data.version}，不以原生几何代替其素材。` });
  if (value.actions.length || value.timeline.length) diagnostics.push({ code: 'static_preview', message: '当前显示声明的初始静态姿态；命名动作与片段尚未执行，不代表生产周期。' });
  const duplicateIds = new Set(value.nodes.filter((node, index) => value.nodes.some((other, position) => position !== index && other.id === node.id)).map(node => node.id));
  const groups: (THREE.Group | null)[] = value.nodes.map(node => {
    if (duplicateIds.has(node.id)) { diagnostics.push({ code: 'duplicate_id', node_id: node.id, message: `部件 ID ${node.id} 重复，须先明确修正。` }); return null; }
    try {
      const group = new THREE.Group(); group.name = node.name; group.userData.node_id = node.id; group.matrixAutoUpdate = false;
      group.matrix.copy(presentationPoseMatrix(node.transform, value.coordinate_frame)); return group;
    } catch (failure) { diagnostics.push({ code: 'invalid_transform', node_id: node.id, message: `${node.name}：${failure instanceof Error ? failure.message : String(failure)}` }); return null; }
  });
  const state = new Map<number, boolean>();
  function link(index: number, path: number[]): boolean {
    const known = state.get(index); if (known !== undefined) return known;
    const group = groups[index], node = value.nodes[index]; if (!group) return false;
    const parent = node.parent_index;
    if (parent != null && (!Number.isInteger(parent) || parent < 0 || parent >= groups.length || path.includes(parent) || !link(parent, [...path, index]))) {
      state.set(index, false); diagnostics.push({ code: 'invalid_parent', node_id: node.id, message: `${node.name} 的父级缺失、无效或成环，未把它改挂到根节点。` }); return false;
    }
    (parent == null ? root : groups[parent]!).add(group); nodes.set(node.id, group); state.set(index, true); return true;
  }
  const unit = meters(value.unit), finitePositive = (...numbers: number[]) => numbers.every(number => Number.isFinite(number) && number > 0);
  value.nodes.forEach((node, index) => {
    if (!scale.every(Number.isFinite) || !link(index, [index])) return;
    if (node.geometry.kind === 'imported_node') {
      const imported = node.geometry.data;
      missing(imported.asset_ref, imported.unsupported_extensions.length ? `导入节点 ${imported.node_index} 有未支持扩展 ${imported.unsupported_extensions.join('、')}，未绘制模型` : `导入节点 ${imported.node_index} 未取得实际模型字节，未绘制模型`, node.id);
      return;
    }
    const material = node.material;
    if (!material.color_rgba.every(number => Number.isFinite(number) && number >= 0 && number <= 1) || ![material.metallic, material.roughness].every(number => Number.isFinite(number) && number >= 0 && number <= 1)) {
      diagnostics.push({ code: 'invalid_material', node_id: node.id, message: `${node.name} 的颜色、透明度、金属度与粗糙度须在0至1之间，未绘制此部件。` }); return;
    }
    if (material.texture) missing(material.texture, '原生部件仅显示声明底色，纹理字节尚未接入', node.id);
    const geometry = node.geometry; let shape: THREE.BufferGeometry | null = null;
    switch (geometry.kind) {
      case 'box': if (finitePositive(geometry.data.x, geometry.data.y, geometry.data.z)) shape = new THREE.BoxGeometry(geometry.data.x * unit, geometry.data.y * unit, geometry.data.z * unit); break;
      case 'cylinder': if (finitePositive(geometry.data.radius, geometry.data.height)) {
        shape = new THREE.CylinderGeometry(geometry.data.radius * unit, geometry.data.radius * unit, geometry.data.height * unit, 48);
        if (value.coordinate_frame === 'right_handed_z_up') shape.rotateX(Math.PI / 2);
      } break;
      case 'plane': if (finitePositive(geometry.data.x, geometry.data.y)) {
        shape = new THREE.PlaneGeometry(geometry.data.x * unit, geometry.data.y * unit);
        if (value.coordinate_frame === 'right_handed_y_up') shape.rotateX(-Math.PI / 2);
      } break;
    }
    if (!shape) { diagnostics.push({ code: 'invalid_geometry', node_id: node.id, message: `${node.name} 的原生尺寸须为有限正数，未绘制此部件。` }); return; }
    const [r, g, b, alpha] = material.color_rgba;
    const surface = new THREE.MeshStandardMaterial({ color: new THREE.Color().setRGB(r, g, b), opacity: alpha, transparent: alpha < 1, depthWrite: alpha === 1, metalness: material.metallic, roughness: material.roughness, side: THREE.DoubleSide });
    const mesh = new THREE.Mesh(shape, surface); mesh.name = node.name; mesh.userData.node_id = node.id;
    groups[index]!.add(mesh); meshes.push(mesh);
  });
  root.updateMatrixWorld(true);
  return { root, nodes, diagnostics, nativeMeshCount: meshes.length, dispose() { meshes.forEach(mesh => { mesh.geometry.dispose(); mesh.material.dispose(); }); root.clear(); } };
}
