import type { Primitive, TimeWindow } from './types';

export function timeX(t: number, window: TimeWindow, width: number, left = 0) {
  return left + (t - window.start_ms) * width / (window.end_ms - window.start_ms);
}
export function xTime(x: number, window: TimeWindow, width: number, left = 0) {
  return Math.round(window.start_ms + (x - left) * (window.end_ms - window.start_ms) / width);
}
export function clipInterval(interval: TimeWindow, window: TimeWindow) {
  const start_ms = Math.max(interval.start_ms, window.start_ms), end_ms = Math.min(interval.end_ms, window.end_ms);
  return end_ms > start_ms ? { visible: { start_ms, end_ms }, true_interval: interval,
    continues_left: interval.start_ms < window.start_ms, continues_right: interval.end_ms > window.end_ms } : null;
}
export function pointVisible(at: number, window: TimeWindow) { return at >= window.start_ms && at <= window.end_ms; }
export function timeText(ms: number) {
  const sign = ms < 0 ? '−' : '', absolute = Math.abs(Math.round(ms));
  return `${sign}${Math.floor(absolute / 1000)}${absolute % 1000 ? `.${String(absolute % 1000).padStart(3, '0').replace(/0+$/, '')}` : ''}秒`;
}
export function gridStep(window: TimeWindow, width: number) {
  const desired = (window.end_ms - window.start_ms) * 90 / Math.max(width, 1);
  const power = 10 ** Math.floor(Math.log10(Math.max(desired, 1)));
  return [1, 2, 5, 10].map(n => n * power).find(n => n >= desired) ?? power * 10;
}
export function gridTicks(window: TimeWindow, width: number) {
  const step = gridStep(window, width), values: number[] = [];
  for (let t = Math.ceil(window.start_ms / step) * step; t <= window.end_ms; t += step) values.push(t);
  return values;
}
export function zoomWindow(window: TimeWindow, anchor: number, factor: number, domain: TimeWindow): TimeWindow {
  const span = Math.max(1, Math.min(domain.end_ms - domain.start_ms, Math.round((window.end_ms - window.start_ms) * factor)));
  const ratio = (anchor - window.start_ms) / (window.end_ms - window.start_ms);
  const start_ms = Math.max(domain.start_ms, Math.min(domain.end_ms - span, Math.round(anchor - span * ratio)));
  return { start_ms, end_ms: start_ms + span };
}
export interface SnapTarget { id: string; at_ms: number; label: string; kind: 'reference' | 'business' | 'grid' }
export function snapTime(raw: number, window: TimeWindow, width: number, targets: SnapTarget[], enabled: boolean) {
  if (!enabled) return { at_ms: Math.round(raw), target: null };
  const rank = { reference: 0, business: 1, grid: 2 };
  const matches = targets.filter(t => pointVisible(t.at_ms, window)).map(t => ({ target: t, distance: Math.abs(timeX(t.at_ms, window, width) - timeX(raw, window, width)) }))
    .filter(t => t.distance <= 6).sort((a, b) => a.distance - b.distance || rank[a.target.kind] - rank[b.target.kind] || a.target.id.localeCompare(b.target.id));
  return matches[0] ? { at_ms: matches[0].target.at_ms, target: matches[0].target } : { at_ms: Math.round(raw), target: null };
}
// Density is a rendering projection, retaining every actual source reference.
export function densityBuckets(primitives: Primitive[], window: TimeWindow, width: number) {
  const buckets = new Map<number, Primitive[]>();
  for (const p of primitives) {
    const at = p.point_ms ?? p.interval?.start_ms;
    if (at === undefined || !pointVisible(at, window)) continue;
    const key = Math.floor(timeX(at, window, width) / 6);
    const group = buckets.get(key) ?? []; group.push(p); buckets.set(key, group);
  }
  return [...buckets].map(([bucket, sources]) => ({ x: bucket * 6, sources }));
}
export function intervalDensity(primitives: Primitive[], window: TimeWindow, width: number) {
  const groups = new Map<string, { x: number; kind: Primitive['kind']; sources: Primitive[] }>();
  for (const p of primitives) {
    if (!p.interval) continue;
    const clip = clipInterval(p.interval, window); if (!clip) continue;
    const left = timeX(clip.visible.start_ms, window, width), right = timeX(clip.visible.end_ms, window, width);
    if (right - left >= 6) continue;
    const x = Math.floor(left / 6) * 6, key = `${p.kind}:${x}`;
    const group = groups.get(key) ?? { x, kind: p.kind, sources: [] }; group.sources.push(p); groups.set(key, group);
  }
  return [...groups.values()].filter(g => g.sources.length > 1);
}
