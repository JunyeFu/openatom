import { Color, Mesh, PerspectiveCamera, PlaneGeometry, ShaderMaterial, DoubleSide, Vector2, Vector3 } from 'three';

export const viewportGroundColor = 0xe8edf2;

export function viewportMetersPerPixel(camera: PerspectiveCamera, target: Vector3, cssHeight: number, viewSize: Vector2) {
  return camera.getViewSize(camera.position.distanceTo(target), viewSize).y / cssHeight;
}

export function chooseGridStep(metersPerPixel: number, previous: number | null) {
  if (previous !== null) {
    const pixels = previous / metersPerPixel;
    if (pixels >= 40 && pixels <= 96) return previous;
  }
  return 2 ** Math.round(Math.log2(64 * metersPerPixel));
}

export interface ViewportScaleValues {
  widthPx: number;
  totalM: number;
  midLabel: string;
  endLabel: string;
}

export function viewportScaleFor(metersPerPixel: number): ViewportScaleValues {
  const exponent = Math.floor(Math.log10(128 * metersPerPixel));
  let referenceLength = 0, score = Infinity;
  for (let power = exponent - 1; power <= exponent + 1; power++) {
    for (const multiple of [1, 2, 5]) {
      const length = multiple * 10 ** power, pixels = length / metersPerPixel;
      // The inclusive endpoints can differ by floating-point rounding at exact decades.
      if (pixels < 80 - 1e-9 || pixels > 200 + 1e-9) continue;
      const distance = Math.abs(pixels - 128);
      if (distance < score) { referenceLength = length; score = distance; }
    }
  }
  const totalM = referenceLength / 2;
  const [unit, divisor]: [string, number] = totalM >= 1000 ? ['km', 1000] : totalM >= 0.1 ? ['m', 1] : totalM >= 0.01 ? ['cm', 0.01] : ['mm', 0.001];
  const number = (value: number) => String(Number((value / divisor).toPrecision(6)));
  return { widthPx: totalM / metersPerPixel, totalM, midLabel: number(totalM / 2), endLabel: `${number(totalM)} ${unit}` };
}

const frustumEdges = [[0, 1], [0, 2], [1, 3], [2, 3], [4, 5], [4, 6], [5, 7], [6, 7], [0, 4], [1, 5], [2, 6], [3, 7]];
export interface ViewportGridState {
  metersPerPixel: number;
  stepM: number;
  cellPx: number;
  bounds: { minX: number; maxX: number; minZ: number; maxZ: number } | null;
}

export function createAdaptiveViewportGrid() {
  const material = new ShaderMaterial({
    uniforms: {
      stepM: { value: 1 },
      minorColor: { value: new Color(0xcbd2d9) },
      majorColor: { value: new Color(0xa8b2bc) },
    },
    vertexShader: `
      varying vec2 ground;
      void main() {
        vec4 world = modelMatrix * vec4(position, 1.0);
        ground = world.xz;
        gl_Position = projectionMatrix * viewMatrix * world;
      }
    `,
    fragmentShader: `
      uniform float stepM;
      uniform vec3 minorColor;
      uniform vec3 majorColor;
      varying vec2 ground;
      float gridLine(vec2 cell, float width) {
        vec2 derivative = max(fwidth(cell), vec2(0.000001));
        vec2 distancePx = abs(fract(cell + 0.5) - 0.5) / derivative;
        vec2 coverage = 1.0 - smoothstep(vec2(0.0), vec2(width), distancePx);
        coverage *= 1.0 - smoothstep(vec2(0.35), vec2(0.75), derivative);
        return max(coverage.x, coverage.y);
      }
      void main() {
        float minor = gridLine(ground / stepM, 0.65);
        float major = gridLine(ground / (stepM * 4.0), 0.85);
        gl_FragColor = vec4(mix(minorColor, majorColor, major), max(minor * 0.75, major * 0.9));
        #include <tonemapping_fragment>
        #include <colorspace_fragment>
      }
    `,
    transparent: true, depthWrite: false, side: DoubleSide,
  });
  const mesh = new Mesh(new PlaneGeometry(1, 1).rotateX(-Math.PI / 2), material);
  mesh.name = 'viewport-ground-grid';
  mesh.position.y = 0.001;
  // Draw beneath translucent region fills, while retaining opaque geometry's depth test.
  mesh.renderOrder = -1;
  const corners = Array.from({ length: 8 }, () => new Vector3());
  const point = new Vector3(), viewSize = new Vector2();
  let state: ViewportGridState | null = null;
  return {
    mesh,
    get state() { return state; },
    update(camera: PerspectiveCamera, target: Vector3, cssHeight: number) {
      const metersPerPixel = viewportMetersPerPixel(camera, target, cssHeight, viewSize);
      const stepM = chooseGridStep(metersPerPixel, state?.stepM ?? null);
      material.uniforms.stepM.value = stepM;
      camera.updateMatrixWorld();
      for (let i = 0; i < 8; i++) corners[i].set(i & 1 ? 1 : -1, i & 2 ? 1 : -1, i & 4 ? 1 : -1).unproject(camera);
      let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
      const include = (p: Vector3) => { minX = Math.min(minX, p.x); maxX = Math.max(maxX, p.x); minZ = Math.min(minZ, p.z); maxZ = Math.max(maxZ, p.z); };
      for (const [a, b] of frustumEdges) {
        const start = corners[a], end = corners[b], from = start.y - mesh.position.y, to = end.y - mesh.position.y;
        if (from === 0) include(start);
        if (to === 0) include(end);
        if (from * to < 0) include(point.lerpVectors(start, end, from / (from - to)));
      }
      mesh.visible = minX < maxX && minZ < maxZ;
      if (mesh.visible) {
        minX = Math.floor(minX / stepM) * stepM - stepM; maxX = Math.ceil(maxX / stepM) * stepM + stepM;
        minZ = Math.floor(minZ / stepM) * stepM - stepM; maxZ = Math.ceil(maxZ / stepM) * stepM + stepM;
        mesh.position.x = (minX + maxX) / 2; mesh.position.z = (minZ + maxZ) / 2;
        mesh.scale.set(maxX - minX, 1, maxZ - minZ);
      }
      state = { metersPerPixel, stepM, cellPx: stepM / metersPerPixel, bounds: mesh.visible ? { minX, maxX, minZ, maxZ } : null };
      return state;
    },
  };
}
