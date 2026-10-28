import { Matrix4, Quaternion, Vector3 } from 'three';
import type { Pose } from '../../../../packages/commands/generated/typescript/Pose';
import type { DeviceInstance } from '../../../../packages/commands/generated/typescript/DeviceInstance';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { EditCommand } from '../../../../packages/commands/generated/typescript/EditCommand';

export const identityPose = (): Pose => ({
  translation: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0, w: 1 },
  scale: { x: 1, y: 1, z: 1 }, unit: 'meter', coordinate_frame: 'right_handed_z_up',
});

export function poseMatrix(pose: Pose) {
  const { translation: p, rotation: q, scale: s } = pose;
  return new Matrix4().compose(new Vector3(p.x, p.y, p.z), new Quaternion(q.x, q.y, q.z, q.w), new Vector3(s.x, s.y, s.z));
}

// Stored member poses are line-local; an ungrouped device is hall-local.
// Inspector position uses hall coordinates, matching the frozen industrial spec.
export function deviceHallPosition(project: ProjectState, device: DeviceInstance) {
  const line = device.line_id ? project.lines.find(value => value.id === device.line_id) : undefined;
  if (device.line_id && !line) return null;
  const value = new Vector3(device.pose.translation.x, device.pose.translation.y, device.pose.translation.z);
  return line ? value.applyMatrix4(poseMatrix(line.transform)) : value;
}

export function withHallPosition(project: ProjectState, device: DeviceInstance, position: Vector3): DeviceInstance {
  const line = device.line_id ? project.lines.find(value => value.id === device.line_id) : undefined;
  if (device.line_id && !line) throw new Error('设备所属线组缺失，请先选择实际线组。');
  const local = line ? position.clone().applyMatrix4(poseMatrix(line.transform).invert()) : position;
  return { ...device, pose: { ...device.pose, translation: { x: local.x, y: local.y, z: local.z } } };
}

export function devicePlacementCommands(project: ProjectState, device: DeviceInstance, lineId: string | null, position: Vector3): EditCommand[] {
  const old = project.devices.find(value => value.id === device.id);
  const destination = lineId ? project.lines.find(value => value.id === lineId) : undefined;
  if (lineId && (!destination || destination.hall_id !== device.hall_id)) throw new Error('线组必须位于设备所属厂房。');
  const commands: EditCommand[] = [];
  let next = device;
  if ((old?.line_id ?? null) !== lineId) {
    const previous = old?.line_id ? project.lines.find(value => value.id === old.line_id) : undefined;
    if (old?.line_id && !previous) throw new Error('原线组缺失，无法提交成员重组。');
    const hall = previous ? poseMatrix(previous.transform).multiply(poseMatrix(device.pose)) : poseMatrix(device.pose);
    const local = destination ? poseMatrix(destination.transform).invert().multiply(hall) : hall;
    const p = new Vector3(), q = new Quaternion(), s = new Vector3();
    local.decompose(p, q, s);
    const recomposed = new Matrix4().compose(p, q, s);
    if (local.elements.some((value, i) => Math.abs(value - recomposed.elements[i]) > 1e-8 * Math.max(1, Math.abs(value)))) {
      throw new Error('该重组产生无法用当前姿态保存的剪切，请先调整线组姿态。');
    }
    next = { ...device, pose: { ...device.pose, rotation: { x: q.x, y: q.y, z: q.z, w: q.w }, scale: { x: s.x, y: s.y, z: s.z } } };
  }
  if (old?.line_id && old.line_id !== lineId) {
    const previous = project.lines.find(value => value.id === old.line_id);
    if (!previous) throw new Error('原线组缺失，无法提交成员重组。');
    commands.push({ kind: 'put_object', data: { kind: 'line', data: { ...previous, members: previous.members.filter(id => id !== device.id) } } });
  }
  if (destination && !destination.members.includes(device.id)) {
    commands.push({ kind: 'put_object', data: { kind: 'line', data: { ...destination, members: [...destination.members, device.id] } } });
  }
  commands.push({ kind: 'put_object', data: { kind: 'device', data: withHallPosition(project, { ...next, line_id: lineId }, position) } });
  return commands;
}

export function rotatePoseZ(pose: Pose, deltaDegrees: number): Pose {
  const q = new Quaternion(pose.rotation.x, pose.rotation.y, pose.rotation.z, pose.rotation.w);
  q.premultiply(new Quaternion().setFromAxisAngle(new Vector3(0, 0, 1), deltaDegrees * Math.PI / 180));
  return { ...pose, rotation: { x: q.x, y: q.y, z: q.z, w: q.w } };
}
