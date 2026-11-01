import { useImperativeHandle, useRef, type Ref } from 'react';
import { Matrix4, type Quaternion } from 'three';
import './ViewportViewCube.css';

export type ViewCubeFace = 'right' | 'left' | 'back' | 'front' | 'top' | 'bottom';
export interface ViewportViewCubeHandle {
  update: (cameraRotation: Quaternion) => void;
}

const faces: { face: ViewCubeFace; letter: string; label: string }[] = [
  { face: 'right', letter: 'X', label: '+X' },
  { face: 'left', letter: 'X', label: '-X' },
  { face: 'back', letter: 'Y', label: '+Y' },
  { face: 'front', letter: 'Y', label: '-Y' },
  { face: 'top', letter: 'Z', label: '+Z' },
  { face: 'bottom', letter: 'Z', label: '-Z' },
];

export function ViewportViewCube({ ref, onFace }: {
  ref: Ref<ViewportViewCubeHandle>;
  onFace: (face: ViewCubeFace) => void;
}) {
  const rotation = useRef<HTMLDivElement>(null);
  useImperativeHandle(ref, () => {
    const flipY = new Matrix4().makeScale(1, -1, 1);
    const view = new Matrix4(), css = new Matrix4();
    return {
      update(cameraRotation) {
        // CSS has downward Y; the camera and the physical scene use Three's upward Y.
        view.makeRotationFromQuaternion(cameraRotation).transpose();
        css.copy(flipY).multiply(view).multiply(flipY);
        rotation.current!.style.transform = `matrix3d(${css.elements.join(',')})`;
      },
    };
  }, []);

  return <div className="viewport-view-cube" role="group" aria-label="视角方向" onPointerDown={event => event.stopPropagation()} onDoubleClick={event => event.stopPropagation()}>
    <div className="viewport-view-cube__rotation" ref={rotation}>
      {faces.map(({ face, letter, label }) => <button key={face} type="button"
        className={`viewport-view-cube__face viewport-view-cube__face--${face}`}
        aria-label={`沿${label}查看`} onClick={event => { event.stopPropagation(); onFace(face); }}>
        {letter}
      </button>)}
    </div>
  </div>;
}
