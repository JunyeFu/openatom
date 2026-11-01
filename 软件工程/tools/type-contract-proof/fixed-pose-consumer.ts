// Isolated DTO consumer for professional forms / Agent authors; no frontend.
import type { FixedPoseMotion } from './output/typescript/FixedPoseMotion';
import type { PackagingMotion } from './output/typescript/PackagingMotion';
import type { ComputePackagingParams } from './output/typescript/ComputePackagingParams';
import type { ApplyCommandsParams } from './output/typescript/ApplyCommandsParams';

export function holdTarget(motion: PackagingMotion): string | undefined {
  return motion.kind === 'fixed_pose' ? motion.data.part_id : undefined;
}
export function computeHoldTargets(params: ComputePackagingParams) {
  return params.packaging_job.steps.map(step => holdTarget(step.motion));
}
export function editedHoldTargets(params: ApplyCommandsParams) {
  return params.commands.flatMap(command => command.kind === 'put_packaging_job'
    ? command.data.steps.map(step => holdTarget(step.motion)) : []);
}
export const authoredHold: FixedPoseMotion = {
  part_id: 'PART-B',
  pose: {
    translation: { x: 120, y: 0, z: 0 },
    rotation: { x: 0, y: 0, z: 0, w: 1 },
    scale: { x: 1, y: 1, z: 1 },
    unit: 'millimeter', coordinate_frame: 'right_handed_z_up',
  },
};
// @ts-expect-error The old bare Pose is not a FixedPoseMotion.
const oldBarePose: FixedPoseMotion = authoredHold.pose;
void oldBarePose;
