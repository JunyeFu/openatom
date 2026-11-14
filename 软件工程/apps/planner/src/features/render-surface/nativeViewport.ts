import type { CSSProperties } from 'react';
import type { NativeViewport } from '../../services/renderTask';

/** Size the drawn surface to the actual native framebuffer. Fractional-DPI
 * browser viewport rounding can otherwise make fixed inset:0 one pixel wider. */
export function nativeViewportStyle(viewport: NativeViewport | null | undefined): CSSProperties | undefined {
  if (!viewport || !Number.isSafeInteger(viewport.width) || viewport.width <= 0
    || !Number.isSafeInteger(viewport.height) || viewport.height <= 0
    || !Number.isFinite(viewport.pixel_ratio) || viewport.pixel_ratio <= 0) return undefined;
  return { width: viewport.width / viewport.pixel_ratio, height: viewport.height / viewport.pixel_ratio,
    right: 'auto', bottom: 'auto', boxSizing: 'border-box' };
}
