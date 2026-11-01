import type { Presentation } from '../../commands/generated/typescript/Presentation';
import type { MediaLocator } from '../../commands/generated/typescript/MediaLocator';
import type { LodLevel } from '../../commands/generated/typescript/LodLevel';
import type { TimelineClip } from '../../commands/generated/typescript/TimelineClip';
import type { VisualProgress, DeclaredMediaPreview } from './types';
import { UnsupportedMedia } from './types';
export function lodSource(value: Presentation, lod: LodLevel): MediaLocator | null {
  if (lod === 'region') return null;
  if (lod === 'simplified') {
    if (!value.lod.simplified) throw new UnsupportedMedia('未声明简化档素材；不能暗降模型质量并报告简化档完成。');
    return value.lod.simplified;
  }
  return value.lod.detailed ?? value.source;
}
export function declaredProgress(preview: DeclaredMediaPreview): VisualProgress {
  if (!preview.phase_ref || !Number.isFinite(preview.view_u) || preview.view_u < 0 || preview.view_u > 1) throw new UnsupportedMedia('声明预览须提供阶段及0到1的view_u。');
  return {known:true, display_phase_id:preview.phase_ref,progress:preview.view_u,terminal_pose:preview.view_u===1,effective_ms:null,duration_ms:null};
}
export interface ClipPoint { clip: TimelineClip; progress: number; effective_ms: number | null }
export function activeClip(value: Presentation, point: VisualProgress): ClipPoint | null {
  if (value.timeline.length === 0) return null;
  if (!point.known || point.progress == null) throw new UnsupportedMedia('阶段进度未知，无法选择声明的表现片段。');
  const clips = value.timeline.filter(clip => clip.phase_ref === point.display_phase_id || (clip.phase_ref == null && point.display_phase_id === 'cycle'));
  const matches = clips.filter(clip => clip.start_progress <= point.progress! && (point.progress! < clip.end_progress || point.progress === 1 && clip.end_progress === 1));
  if (matches.length !== 1) throw new UnsupportedMedia(`阶段 ${point.display_phase_id} 在此进度没有唯一主画面片段；未回退到空闲或另一阶段。`);
  const clip = matches[0], span = clip.end_progress - clip.start_progress;
  if (!(span > 0)) throw new UnsupportedMedia('片段进度区间无效。');
  return { clip, progress: (point.progress - clip.start_progress) / span, effective_ms: point.effective_ms == null || point.duration_ms == null ? null : Math.max(0, point.effective_ms - clip.start_progress * point.duration_ms) };
}
