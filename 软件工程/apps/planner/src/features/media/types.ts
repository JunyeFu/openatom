import type { Presentation } from '../../../../../packages/commands/generated/typescript/Presentation';
import type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';
import type { AssetRights } from '../../../../../packages/commands/generated/typescript/AssetRights';
import type { LodLevel } from '../../../../../packages/commands/generated/typescript/LodLevel';
import type { DeviceAction } from '../../../../../packages/commands/generated/typescript/DeviceAction';
import type { ActualMediaFrame,DeclaredMediaPreview } from '../../../../../packages/presentation/src';
import type { MediaAssetReadRequest } from '../../services/mediaAssets';
import type { MediaNodeOwner, MediaNodeSelection } from '../node-inspector/types';
import type { ReadonlyVisualCapture } from '../render-surface/visualCapture';
import type { ProcessItems } from './processItems';
import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
export interface MediaPresentationCommonProps {
  presentation: Presentation;
  binding: {session_id:string;request:Omit<MediaAssetReadRequest,'asset'>};
  readAsset:(request:MediaAssetReadRequest)=>Promise<ArrayBuffer>;
  resolveModule?:(source:MediaAssetReadRequest['source'])=>Promise<ModulePackage>;
  /** Optional explicit complete-observation reader, like readAsset. The desktop
   * default uses the canonical Host; captured consumers supply real Host data. */
  readItems?:(frame:ActualMediaFrame,binding:MediaPresentationCommonProps['binding'],current:()=>boolean)=>Promise<ProcessItems>;
  lod:LodLevel;
  rights:AssetRights[];
  nodeOwner?:MediaNodeOwner;
  /** The source project frozen for this view; used only for declared display names. */
  sourceProject?:ProjectState|null;
  /** Explicit frozen declaration provenance; does not supply an actual frame. */
  inspectionLabel?:string;
  inspectionTitle?:string;
  /** Same renderer, explicit camera and actual completion; no authoring or clock. */
  capture?:ReadonlyVisualCapture;
  onSelectNode?:(selection:MediaNodeSelection)=>void;
  onInspectItem?:(id:string)=>void;
  onAction?:(action:DeviceAction)=>void;
  playback?:{playing:boolean;rate:number;loop:boolean};
  onPlaybackChange?:(value:{playing:boolean;rate:number;loop:boolean})=>void;
}
export type MediaPresentationProps=MediaPresentationCommonProps & (
  {mode:'actual_run';frame:ActualMediaFrame;preview?:never;onSeek?:(at_ms:number)=>void;onPreviewSeek?:never} |
  {mode:'declared_preview';preview:DeclaredMediaPreview;frame?:never;onPreviewSeek?:(value:DeclaredMediaPreview)=>void;onSeek?:never} |
  {mode:'asset_inspection';frame?:never;preview?:never;onSeek?:never;onPreviewSeek?:never}
);
