import type { WebGLRenderer } from 'three';
import type { CameraState } from '../../../../../packages/commands/generated/typescript/CameraState';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';

export interface ReadonlyVisualCapture {
  key: string;
  camera: CameraState;
  object: ObjectRef;
  poseName?: string;
  onFrame(value: { visible_objects: ObjectRef[]; limitations: string[] }): void;
  onFailure(message: string): void;
}
export const nextPaint = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
/** Resource owners call this only after their actual draw. No timer or business clock. */
export async function domPainted(current: () => boolean) {
  await document.fonts.ready;
  await nextPaint(); await nextPaint();
  return current();
}
export async function gpuPainted(renderer: WebGLRenderer, current: () => boolean) {
  const gl = renderer.getContext();
  if (!(gl instanceof WebGL2RenderingContext)) throw new Error('实际渲染完成需要WebGL2帧栅栏。');
  const fence = gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE, 0);
  if (!fence) throw new Error('GPU未建立实际帧完成栅栏。');
  gl.flush();
  try {
    while (current()) {
      const state = gl.clientWaitSync(fence, 0, 0);
      if (state === gl.WAIT_FAILED) throw new Error('GPU帧完成查询失败。');
      if (state === gl.ALREADY_SIGNALED || state === gl.CONDITION_SATISFIED) return domPainted(current);
      await nextPaint();
    }
    return false;
  } finally { gl.deleteSync(fence); }
}
