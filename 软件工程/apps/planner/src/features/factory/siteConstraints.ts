import type { LayoutRegion } from '../../../../../packages/commands/generated/typescript/LayoutRegion';
import type { LayoutRegionKind } from '../../../../../packages/commands/generated/typescript/LayoutRegionKind';
import type { SiteEnvelope } from '../../../../../packages/commands/generated/typescript/SiteEnvelope';

export interface SiteRegionDraft {
  id: string;
  kind: LayoutRegionKind;
  clearance: string;
  points: { x: string; y: string }[];
}
export interface SiteConstraintsDraft { clearance: string; regions: SiteRegionDraft[] }

export function siteConstraintsDraft(site?: SiteEnvelope): SiteConstraintsDraft {
  return {
    clearance: site?.clearance_m == null ? '' : String(site.clearance_m),
    regions: (site?.regions ?? []).map(region => ({
      id: region.id, kind: region.kind,
      clearance: region.clearance_m == null ? '' : String(region.clearance_m),
      points: region.polygon_m.map(point => ({ x: String(point.x), y: String(point.y) })),
    })),
  };
}

export function readSiteConstraints(draft: SiteConstraintsDraft): Pick<SiteEnvelope, 'clearance_m' | 'regions'> {
  const number = (text: string, label: string) => {
    if (!text.trim() || !Number.isFinite(Number(text))) throw new Error(`${label}须填写有限数值。`);
    return Number(text);
  };
  const clearance = (text: string, label: string) => {
    if (!text.trim()) return null;
    const value = number(text, label);
    if (value < 0) throw new Error(`${label}不得小于零。`);
    return value;
  };
  const ids = new Set<string>();
  const regions: LayoutRegion[] = draft.regions.map((region, index) => {
    const id = region.id.trim();
    if (!id || ids.has(id)) throw new Error(`区域 ${index + 1} 须填写不重复的标识。`);
    ids.add(id);
    if (region.points.length < 3) throw new Error(`区域 ${id} 至少需要三个顶点。`);
    return { id, kind: region.kind, clearance_m: clearance(region.clearance, `区域 ${id} 净距`),
      polygon_m: region.points.map((point, i) => ({
        x: number(point.x, `区域 ${id} 顶点 ${i + 1} X`),
        y: number(point.y, `区域 ${id} 顶点 ${i + 1} Y`),
      })) };
  });
  return { clearance_m: clearance(draft.clearance, '设备间最小净距'), regions };
}
