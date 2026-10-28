import { Box3, Vector3 } from 'three';

/** Product default oblique view, with five percent of each viewport edge free. */
export function defaultObliqueCamera(bounds: Box3, verticalFovDegrees: number, aspect: number) {
  const target = bounds.getCenter(new Vector3());
  const towardEye = new Vector3(.8, 1, 1).normalize();
  const right = new Vector3(0, 1, 0).cross(towardEye).normalize();
  const up = towardEye.clone().cross(right).normalize();
  const vertical = Math.tan(verticalFovDegrees * Math.PI / 360) * .9;
  let distance = 0;
  for (const x of [bounds.min.x, bounds.max.x]) for (const y of [bounds.min.y, bounds.max.y]) for (const z of [bounds.min.z, bounds.max.z]) {
    const corner = new Vector3(x, y, z).sub(target), depth = corner.dot(towardEye);
    distance = Math.max(distance, depth + Math.abs(corner.dot(up)) / vertical,
      depth + Math.abs(corner.dot(right)) / (vertical * aspect));
  }
  return { target, eye: target.clone().addScaledVector(towardEye, distance) };
}
