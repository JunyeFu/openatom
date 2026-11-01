import { useImperativeHandle, useRef, type Ref } from 'react';
import { viewportScaleFor, type ViewportScaleValues } from './AdaptiveViewportGrid';
import './ViewportScale.css';

export interface ViewportScaleHandle {
  update: (metersPerPixel: number) => ViewportScaleValues;
}

export function ViewportScale({ ref }: { ref: Ref<ViewportScaleHandle> }) {
  const host = useRef<HTMLDivElement>(null), track = useRef<HTMLDivElement>(null);
  const middle = useRef<HTMLSpanElement>(null), end = useRef<HTMLSpanElement>(null);
  useImperativeHandle(ref, () => {
    let previous: ViewportScaleValues | null = null;
    return { update(metersPerPixel) {
      const value = viewportScaleFor(metersPerPixel);
      if (value.widthPx !== previous?.widthPx) {
        track.current!.style.width = `${value.widthPx}px`;
        host.current!.style.width = `${value.widthPx + 12}px`;
      }
      if (value.midLabel !== previous?.midLabel) middle.current!.textContent = value.midLabel;
      if (value.endLabel !== previous?.endLabel) {
        end.current!.textContent = value.endLabel;
        host.current!.setAttribute('aria-label', `视点参考分度尺：${value.endLabel}`);
      }
      host.current!.style.visibility = 'visible';
      previous = value;
      return value;
    } };
  }, []);
  return <div ref={host} className="viewport-scale" role="img" aria-label="视点参考分度尺">
    <div ref={track} className="viewport-scale__track">
      <span className="viewport-scale__zero">0</span>
      <span ref={middle} className="viewport-scale__middle" />
      <span ref={end} className="viewport-scale__end" />
      <div className="viewport-scale__line">{[0, 25, 50, 75, 100].map(percent =>
        <i key={percent} className="viewport-scale__tick" style={{ left: `${percent}%` }} />)}</div>
    </div>
  </div>;
}
