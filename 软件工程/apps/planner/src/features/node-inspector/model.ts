import type { GetObjectParams } from '../../../../../packages/commands/generated/typescript/GetObjectParams';
import type { ProcessAssetSource } from '../../services/presentationHost';
import type { ProcessReadContext } from '../../services/presentationHost';
import { processContextKey } from '../../services/presentationHost';
import type { DeviceInstance } from '../../../../../packages/commands/generated/typescript/DeviceInstance';
import type { SceneNode } from '../../../../../packages/commands/generated/typescript/SceneNode';
import type { InlineNodeFields, MediaNodeOwner, NodeEditTarget, ProcessNodeSelection } from './types';

/** A MediaRef traversal changes metadata owner; it never borrows the wrapper ID. */
export const moduleNodeOwner = (source: ProcessAssetSource): MediaNodeOwner => ({ object_id: { kind: 'module', id: source.data.module_ref.id }, source });
export const nodeRequestParams = (value: ProcessNodeSelection): GetObjectParams => ({ object_id: value.owner.object_id, source: value.owner.source, node_index: value.node_index });
export const nodeSelectionKey = (value: ProcessNodeSelection) => JSON.stringify([processContextKey(value.context), value.owner, value.node_index, value.rendered_node_id, value.lod, value.asset, value.byte_source]);
/** A successful own PutObject advances R; it must not close another session/selection. */
export const nodeEditContextIdentity = (value: ProcessReadContext | null) => value ? JSON.stringify([value.session_id, value.project_id, value.scope, value.selection]) : null;
export function nodeEditTarget(value: ProcessNodeSelection): NodeEditTarget | null {
  const { object_id, source } = value.owner;
  const frozen = source.kind === 'run_snapshot' || source.kind === 'run_module_version';
  if (object_id.kind === 'module' && (source.kind === 'module_version' || source.kind === 'run_module_version')) return { kind: 'module_version', module_ref: source.data.module_ref, frozen };
  if (object_id.kind === 'definition_draft') return { kind: 'definition_draft', draft_id: object_id.id, frozen };
  if (object_id.kind === 'device') return { kind: 'device_override', device_id: object_id.id, frozen };
  return null;
}
export function nodeEditLabel(target: NodeEditTarget): string {
  if (target.frozen) return '从此冻结来源创作新版本';
  if (target.kind === 'module_version') return '从此精确版本创作草稿';
  return target.kind === 'definition_draft' ? '编辑此定义草稿来源' : '编辑此实例表现来源';
}
export const inlineNodeFields = (node: SceneNode): InlineNodeFields => ({ name: node.name, transform: node.transform,
  ...(node.geometry.kind === 'imported_node' ? {} : { geometry: node.geometry }),
  material: { color_rgba: node.material.color_rgba, metallic: node.material.metallic, roughness: node.material.roughness } });
export function canEditInlineNode(value: ProcessNodeSelection, parent: DeviceInstance | undefined): boolean {
  const selected = value.context.selection;
  return value.context.scope.kind === 'project' && value.owner.object_id.kind === 'device' && value.owner.source.kind === 'draft'
    && selected.kind === 'definition' && selected.device_id === value.owner.object_id.id && parent?.id === value.owner.object_id.id
    && parent.module_ref.id === selected.module_ref.id && parent.module_ref.version === selected.module_ref.version;
}
/** Only this canonical entry changes; published dependencies and file references cannot be edited here. */
export function withInlineNode(device: DeviceInstance, index: number, fields: InlineNodeFields): DeviceInstance {
  const presentation = device.presentation_override, node = presentation?.nodes[index];
  if (!presentation || !node) throw new Error('此实际设备没有所选自有override节点；不能改写定义或依赖模块。');
  if (fields.geometry && node.geometry.kind === 'imported_node') throw new Error('导入节点保留原AssetRef和GLB索引；此入口不替换素材。');
  const replacement: SceneNode = { ...node, name: fields.name, transform: fields.transform,
    geometry: fields.geometry ?? node.geometry, material: { ...node.material, ...fields.material } };
  return { ...device, presentation_override: { ...presentation, nodes: presentation.nodes.map((value, position) => position === index ? replacement : value) } };
}
