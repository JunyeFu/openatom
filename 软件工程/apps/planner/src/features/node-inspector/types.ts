import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';
import type { ObjectSource } from '../../../../../packages/commands/generated/typescript/ObjectSource';
import type { AssetRef } from '../../../../../packages/commands/generated/typescript/AssetRef';
import type { LodLevel } from '../../../../../packages/commands/generated/typescript/LodLevel';
import type { SceneNode } from '../../../../../packages/commands/generated/typescript/SceneNode';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import type { ProcessReadContext, ProcessAssetSource } from '../../services/presentationHost';
import type { DeviceInstance } from '../../../../../packages/commands/generated/typescript/DeviceInstance';
import type { SceneGeometry } from '../../../../../packages/commands/generated/typescript/SceneGeometry';
import type { SceneMaterial } from '../../../../../packages/commands/generated/typescript/SceneMaterial';

/** The parent that directly owns Presentation.nodes; independent of asset permissions. */
export interface MediaNodeOwner { object_id: ObjectRef; source: ObjectSource }
export interface MediaNodeSelection {
  owner: MediaNodeOwner;
  node_index: number;
  /** Identity observed at that canonical position; never used to look up an index. */
  rendered_node_id?: number;
  lod: LodLevel;
  asset: AssetRef | null;
  byte_source: ProcessAssetSource;
}
export interface ProcessNodeSelection extends MediaNodeSelection { context: ProcessReadContext }
export interface NodeInspection { selection: ProcessNodeSelection; node: SceneNode; parent_device?: DeviceInstance }
export interface InlineNodeFields {
  name: string;
  transform: SceneNode['transform'];
  geometry?: Exclude<SceneGeometry, { kind: 'imported_node' }>;
  material: Pick<SceneMaterial, 'color_rgba' | 'metallic' | 'roughness'>;
}
export type NodeEditTarget =
  | { kind: 'module_version'; module_ref: ModuleRef; frozen: boolean }
  | { kind: 'definition_draft'; draft_id: string; frozen: boolean }
  | { kind: 'device_override'; device_id: string; frozen: boolean };
export interface NodeEditRequest extends NodeInspection { target: NodeEditTarget }
/** Private UI navigation lease; never serialized into SDK requests. */
export interface NodeSourceEditRequest extends NodeEditRequest { assertCurrent(): void }
