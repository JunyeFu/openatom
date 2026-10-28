import { Matrix4, Quaternion, Vector3 } from 'three';
import type { ProjectState, EditCommand } from '../services/projectHost';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { DeviceInstance } from '../../../../packages/commands/generated/typescript/DeviceInstance';
import type { LineGroup } from '../../../../packages/commands/generated/typescript/LineGroup';
import type { Pose } from '../../../../packages/commands/generated/typescript/Pose';
import { deviceHallPosition, devicePlacementCommands, identityPose, poseMatrix } from './layout';

export function layoutSelection(project: ProjectState, selected: readonly ObjectRef[]) {
  const lines = project.lines.filter(line => selected.some(ref => ref.kind === 'line' && ref.id === line.id));
  const devices = project.devices.filter(device => selected.some(ref => ref.kind === 'device' && ref.id === device.id)
    || lines.some(line => line.members.includes(device.id) || device.line_id === line.id));
  return { lines, devices };
}

/** All matrices are the declared project metre/Z-up transforms, not camera transforms. */
export function deviceHallMatrix(project: ProjectState, device: DeviceInstance) {
  const line = device.line_id ? project.lines.find(value => value.id === device.line_id) : null;
  if (device.line_id && !line) throw new Error('设备所属线组缺失，不能猜测布局。');
  return line ? poseMatrix(line.transform).multiply(poseMatrix(device.pose)) : poseMatrix(device.pose);
}

function matrixPose(matrix: Matrix4, original: Pose): Pose {
  const position = new Vector3(), rotation = new Quaternion(), scale = new Vector3();
  matrix.decompose(position, rotation, scale);
  const restored = new Matrix4().compose(position, rotation, scale);
  if (matrix.elements.some((value, i) => Math.abs(value - restored.elements[i]) > 1e-8 * Math.max(1, Math.abs(value)))) {
    throw new Error('该布局包含不能保存为当前姿态的剪切；请先调整线组比例或朝向。');
  }
  return { ...original, translation: { x: position.x, y: position.y, z: position.z },
    rotation: { x: rotation.x, y: rotation.y, z: rotation.z, w: rotation.w }, scale: { x: scale.x, y: scale.y, z: scale.z } };
}

function reparent(project: ProjectState, devices: DeviceInstance[], destination: LineGroup | null, create = false) {
  let current = { ...project, lines: create ? [...project.lines, destination!] : [...project.lines], devices: [...project.devices] };
  const commands = new Map<string, EditCommand>();
  if (create) commands.set('line:' + destination!.id, { kind: 'put_object', data: { kind: 'line', data: destination! } });
  for (const source of devices) {
    const device = current.devices.find(value => value.id === source.id)!;
    if (destination && destination.hall_id !== device.hall_id) throw new Error('只能将同一厂房的设备编入线组。');
    const position = deviceHallPosition(current, device);
    if (!position) throw new Error('原设备缺少实际线组父级。');
    for (const command of devicePlacementCommands(current, device, destination?.id ?? null, position)) {
      if (command.kind !== 'put_object') continue;
      const object = command.data;
      if (object.kind !== 'line' && object.kind !== 'device') continue;
      commands.set(object.kind + ':' + object.data.id, command);
      if (object.kind === 'line') current = { ...current, lines: current.lines.map(line => line.id === object.data.id ? object.data : line) };
      if (object.kind === 'device') current = { ...current, devices: current.devices.map(value => value.id === object.data.id ? object.data : value) };
    }
  }
  return { current, commands };
}

export function groupLayoutSelection(project: ProjectState, selected: readonly ObjectRef[], id: string, title: string): EditCommand[] {
  const { devices } = layoutSelection(project, selected);
  if (!devices.length) throw new Error('先选择要编组的设备。');
  if (!id.trim() || !title.trim() || project.lines.some(line => line.id === id)) throw new Error('请填写未使用的线组ID和名称。');
  const hall = devices[0].hall_id;
  if (devices.some(device => device.hall_id !== hall)) throw new Error('跨厂房设备不能编为同一线组。');
  const positions = devices.map(device => deviceHallPosition(project, device));
  if (positions.some(position => position === null)) throw new Error('原设备所属线组缺失，不能猜测编组位置。');
  const center = positions.reduce<Vector3>((sum, position) => sum.add(position!), new Vector3()).divideScalar(devices.length);
  const transform = { ...identityPose(), translation: { x: center.x, y: center.y, z: center.z } };
  const line: LineGroup = { id, title: title.trim(), hall_id: hall, transform, members: [], boundary_ports: [], region_polygon_m: [] };
  const result = reparent(project, devices, line, true);
  const members = result.current.devices.filter(device => device.line_id === id);
  if (members.every(device => device.footprint_m.length >= 3)) {
    const points = members.flatMap(device => device.footprint_m.map(point => new Vector3(point.x, point.y, 0).applyMatrix4(poseMatrix(device.pose))));
    const xs = points.map(point => point.x), ys = points.map(point => point.y);
    line.region_polygon_m = [{ x: Math.min(...xs), y: Math.min(...ys) }, { x: Math.max(...xs), y: Math.min(...ys) },
      { x: Math.max(...xs), y: Math.max(...ys) }, { x: Math.min(...xs), y: Math.max(...ys) }];
  }
  const finalLine = result.current.lines.find(value => value.id === id)!;
  result.commands.set('line:' + id, { kind: 'put_object', data: { kind: 'line', data: { ...finalLine, region_polygon_m: line.region_polygon_m } } });
  return [...result.commands.values()];
}

export function ungroupLayoutSelection(project: ProjectState, selected: readonly ObjectRef[]): EditCommand[] {
  const { lines, devices } = layoutSelection(project, selected);
  if (!devices.length && !lines.length) throw new Error('先选择设备或线组。');
  const result = reparent(project, devices, null);
  for (const line of lines) {
    result.commands.delete('line:' + line.id);
    result.commands.set('remove:' + line.id, { kind: 'remove_object', data: { kind: 'line', id: line.id } });
  }
  return [...result.commands.values()];
}

export function copyLayoutSelection(project: ProjectState, selected: readonly ObjectRef[], newIds: Record<string, string>, offset: { x: number; y: number; z: number }): EditCommand[] {
  const { lines, devices } = layoutSelection(project, selected);
  if (!devices.length && !lines.length) throw new Error('先选择要复制的设备或线组。');
  const used = new Set([...project.lines, ...project.devices, ...project.halls, ...project.locations, ...project.resources].map(object => object.id));
  for (const object of [...lines, ...devices]) {
    const id = newIds[object.id];
    if (!id?.trim() || used.has(id)) throw new Error('复制ID必须填写且与现有对象及本次其他新ID不同：' + object.id);
    used.add(id);
  }
  const commands: EditCommand[] = lines.map(line => ({ kind: 'put_object', data: { kind: 'line', data: { ...line,
    id: newIds[line.id], title: line.title + ' · 副本', transform: { ...line.transform, translation: {
      x: line.transform.translation.x + offset.x, y: line.transform.translation.y + offset.y, z: line.transform.translation.z + offset.z } },
    members: devices.filter(device => device.line_id === line.id || line.members.includes(device.id)).map(device => newIds[device.id]), boundary_ports: [] } } }));
  for (const device of devices) {
    const copiedLine = lines.find(line => line.id === device.line_id);
    const matrix = deviceHallMatrix(project, device);
    const pose = copiedLine ? structuredClone(device.pose) : matrixPose(matrix, device.pose);
    if (!copiedLine) pose.translation = { x: pose.translation.x + offset.x, y: pose.translation.y + offset.y, z: pose.translation.z + offset.z };
    commands.push({ kind: 'put_object', data: { kind: 'device', data: { ...structuredClone(device), id: newIds[device.id],
      title: device.title + ' · 副本', line_id: copiedLine ? newIds[copiedLine.id] : null, pose,
      role_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} } } } });
  }
  return commands;
}

/** A known rectangular-floor breach; an inside result is not a full site check. */
export function outsideHallFloor(project: ProjectState, device: DeviceInstance): boolean | null {
  const hall = project.halls.find(value => value.id === device.hall_id);
  if (!hall || device.footprint_m.length < 3 || device.line_id && !project.lines.some(line => line.id === device.line_id)) return null;
  const matrix = deviceHallMatrix(project, device);
  return device.footprint_m.some(point => {
    const p = new Vector3(point.x, point.y, 0).applyMatrix4(matrix);
    return p.x < -1e-8 || p.y < -1e-8 || p.x > hall.width_m + 1e-8 || p.y > hall.depth_m + 1e-8;
  });
}
