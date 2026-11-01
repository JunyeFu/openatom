import * as THREE from 'three';
import type { PackageStructureJob } from '../../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { PanelDefinition } from '../../../../../packages/commands/generated/typescript/PanelDefinition';
import type { PackagingProcessSample } from '../../../../../packages/commands/generated/typescript/PackagingProcessSample';
import type { Pose } from '../../../../../packages/commands/generated/typescript/Pose';

function poseMatrix(pose: Pose) {
  if (pose.unit !== 'millimeter' || pose.coordinate_frame !== 'right_handed_z_up') return null;
  return new THREE.Matrix4().compose(new THREE.Vector3(pose.translation.x, pose.translation.y, pose.translation.z),
    new THREE.Quaternion(pose.rotation.x, pose.rotation.y, pose.rotation.z, pose.rotation.w), new THREE.Vector3(pose.scale.x, pose.scale.y, pose.scale.z));
}
export function panelTransform(job: PackageStructureJob, panel: PanelDefinition, sample: PackagingProcessSample | null) {
  if (job.unit !== 'millimeter' || job.coordinate_frame !== 'right_handed_z_up') return null;
  if (sample) {
    const actual = sample.panel_poses.find(item => item.panel_id === panel.id)?.pose;
    return actual ? poseMatrix(actual) : null;
  }
  const flat = poseMatrix(panel.flat_pose); const placement = job.parts.find(part => part.id === panel.part_id)?.placement;
  const parent = placement ? poseMatrix(placement) : null;
  return flat && parent ? parent.multiply(flat) : null;
}
export function panelGeometry(panel: PanelDefinition) {
  if (panel.polygon_mm.length < 3) return null;
  const shape = new THREE.Shape(panel.polygon_mm.map(point => new THREE.Vector2(point.x, point.y)));
  const geometry = new THREE.ExtrudeGeometry(shape, { depth: panel.thickness_mm, bevelEnabled: false });
  geometry.translate(0, 0, -panel.thickness_mm / 2);
  return geometry;
}
/** Connection reference anchors only; activation/contact success stays with the actual result checks. */
export function connectionAnchors(job: PackageStructureJob, sample: PackagingProcessSample | null) {
  return job.connections.flatMap(connection => {
    const endpoints = [[connection.panel_a, connection.frame_a], [connection.panel_b, connection.frame_b]] as const;
    const points = endpoints.map(([id, frame]) => {
      const panel = job.panels.find(item => item.id === id); const matrix = panel ? panelTransform(job, panel, sample) : null;
      return matrix && frame.unit === 'millimeter' && frame.coordinate_frame === 'right_handed_z_up'
        ? new THREE.Vector3(frame.translation.x, frame.translation.y, frame.translation.z).applyMatrix4(matrix) : null;
    });
    return points[0] && points[1] ? [{ id: connection.id, points: [points[0], points[1]] }] : [];
  });
}
