import * as THREE from 'three';
import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { ObjectRef } from '../../../../packages/commands/generated/typescript/ObjectRef';
import type { MaterialEndpoint } from '../../../../packages/commands/generated/typescript/MaterialEndpoint';
import type { ModulePackage } from '../../../../packages/commands/generated/typescript/ModulePackage';
import type { ModuleRef } from '../../../../packages/commands/generated/typescript/ModuleRef';

export interface FactoryConnectionRow {
  id: string;
  title: string;
  problem: string | null;
}

/** Logical material arrows at declared object anchors, never physical pipes or transport paths. */
export function createFactoryConnectionLayer(project: ProjectState, modules: ReadonlyMap<string, ModulePackage> | undefined,
  parent: THREE.Object3D, anchors: ReadonlyMap<string, THREE.Object3D>, selection: ObjectRef | null) {
  const rows: FactoryConnectionRow[] = [], refs: ObjectRef[] = [];
  const positions: number[] = [], colors: number[] = [];
  parent.updateWorldMatrix(true, true);
  for (const connection of project.connections) {
    if (connection.kind !== 'material') continue;
    const value = connection.data, item = project.item_types.find(type => type.id === value.item_type);
    const endpoint = (input: MaterialEndpoint, direction: 'input' | 'output') => {
      if (input.kind === 'location') {
        const location = project.locations.find(place => place.id === input.data);
        return { ref: { kind: 'location' as const, id: input.data }, title: location?.title ?? input.data,
          problem: !location ? '库位不存在。' : !location.accepted_item_types.includes(value.item_type) ? '库位不接受该物品。'
            : location.capacity && location.capacity.unit !== value.unit ? '库位声明单位与连接不相容。' : null };
      }
      const reference = input.data, device = project.devices.find(instance => instance.id === reference.instance_id);
      const title = `${device?.title ?? reference.instance_id} / ${reference.port_id}`;
      const ref = { kind: 'device' as const, id: reference.instance_id };
      if (!device || device.module_ref.id !== reference.module_ref.id || device.module_ref.version !== reference.module_ref.version) {
        return { ref, title, problem: '设备或精确模块版本与端点不一致。' };
      }
      const definition = modules?.get(JSON.stringify([device.module_ref.id, device.module_ref.version]))?.definition;
      const payload = definition?.id === device.module_ref.id && definition.version === device.module_ref.version ? definition.payload : null;
      if (payload?.category !== 'devices') return { ref, title, problem: '精确设备端口定义尚未读取。' };
      const port = payload.data.ports.find(port => port.id === reference.port_id);
      const routeBindsItem = () => {
        if (!port || port.payload.kind !== 'material') return false;
        const role = port.payload.data.item_role;
        const sameModule = (left: ModuleRef, right: ModuleRef) => left.id === right.id && left.version === right.version;
        // Like compile::material, a step overrides this device's defaults only through its exact permitted operation.
        const currentBinding = project.bindings.filter(binding => binding.consumer_ref.kind === 'device' && binding.consumer_ref.id === device.id).at(-1);
        return project.routes.some(route => route.steps.some(step => {
          if (step.device_id !== device.id) return false;
          const operationDefinition = modules?.get(JSON.stringify([step.operation_ref.id, step.operation_ref.version]))?.definition;
          if (!operationDefinition || !sameModule(operationDefinition, step.operation_ref)) return false;
          const source = operationDefinition.payload, behavior = payload.data.behavior;
          const allowed = device.route_operation_refs?.some(reference => sameModule(reference, step.operation_ref))
            || (device.operation_binding ? sameModule(device.operation_binding, step.operation_ref)
              : behavior.selected === 'operation_ref' && behavior.operation_ref && sameModule(behavior.operation_ref, step.operation_ref));
          const operation = source.category === 'operations' && allowed ? source.data
            : source.category === 'devices' && sameModule(device.module_ref, step.operation_ref) && behavior.selected === 'inline'
              ? behavior.inline : null;
          if (!operation || currentBinding?.mode === 'calculated_nominal' && !sameModule(currentBinding.operation_ref, step.operation_ref)) return false;
          return (direction === 'input' ? operation.inputs : operation.outputs).some(material => material.port === port.id
            && material.role === role && material.quantity.unit === value.unit
            && (step.item_bindings[role] ?? device.role_bindings.item_types[role]) === value.item_type);
        }));
      };
      const problem = !port || port.payload.kind !== 'material' ? '该精确版本没有此物料端口。'
        : port.direction !== direction && port.direction !== 'bidirectional' ? '物料端口方向不相容。'
        : port.payload.data.unit !== value.unit || (device.role_bindings.item_types[port.payload.data.item_role] !== value.item_type && !routeBindsItem())
          ? '物料角色绑定或单位不相容。' : null;
      return { ref, title, problem };
    };
    const from = endpoint(value.source, 'output'), to = endpoint(value.destination, 'input');
    const store = project.locations.find(place => place.id === value.destination_store);
    const row: FactoryConnectionRow = { id: value.id,
      title: `${from.title} → ${to.title} · ${item?.title ?? value.item_type} / ${value.unit}`,
      problem: !item || item.unit !== value.unit ? '物品定义或共同单位不相容。' : from.problem ?? to.problem
        ?? (!store || !store.accepted_item_types.includes(value.item_type) || store.capacity && store.capacity.unit !== value.unit
          ? '接收库位或声明单位不相容。' : value.destination.kind === 'location' && store.id !== value.destination.data
            ? '目的库位与接收库位不一致。' : null) };
    rows.push(row);
    if (row.problem) continue;
    const fromAnchor = anchors.get(`${from.ref.kind}:${from.ref.id}`), toAnchor = anchors.get(`${to.ref.kind}:${to.ref.id}`);
    if (!fromAnchor || !toAnchor) { row.problem = '端点不在当前画布，或布局父级缺失。'; continue; }
    const start = parent.worldToLocal(fromAnchor.getWorldPosition(new THREE.Vector3()));
    const end = parent.worldToLocal(toAnchor.getWorldPosition(new THREE.Vector3()));
    start.z += .08; end.z += .08;
    const direction = end.clone().sub(start), length = direction.length();
    if (length < 1e-8) { row.problem = '端点定位重合，无法显示方向。'; continue; }
    direction.normalize();
    const side = new THREE.Vector3().crossVectors(direction, new THREE.Vector3(0, 0, 1));
    if (side.lengthSq() < 1e-8) side.crossVectors(direction, new THREE.Vector3(1, 0, 0));
    side.normalize();
    const head = Math.min(.5, length * .2), tip = start.clone().lerp(end, .7);
    const back = tip.clone().addScaledVector(direction, -head);
    const color = new THREE.Color(selection?.kind === 'connection' && selection.id === value.id ? 0x176ca3 : 0x53788c);
    for (const point of [start, end, tip, back.clone().addScaledVector(side, head * .5), tip, back.addScaledVector(side, -head * .5)]) {
      positions.push(point.x, point.y, point.z); colors.push(color.r, color.g, color.b);
    }
    refs.push({ kind: 'connection', id: value.id });
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
  const root = new THREE.LineSegments(geometry, new THREE.LineBasicMaterial({ vertexColors: true, depthTest: false }));
  root.renderOrder = 3;
  root.userData.connectionRefs = refs;
  const setSelection = (selected: ObjectRef | null) => {
    const attribute = geometry.getAttribute('color');
    refs.forEach((ref, index) => {
      const color = new THREE.Color(selected?.kind === 'connection' && selected.id === ref.id ? 0x176ca3 : 0x53788c);
      for (let vertex = index * 6; vertex < (index + 1) * 6; vertex++) attribute.setXYZ(vertex, color.r, color.g, color.b);
    });
    attribute.needsUpdate = true;
  };
  return { root, rows, drawn: refs.length, setSelection, limitations: rows.flatMap(row => row.problem
    ? [{ object_ref: { kind: 'connection' as const, id: row.id }, message: `物料逻辑连接未绘制：${row.problem}` }]
    : []) };
}

/** Three LineSegments reports the first vertex index of the intersected segment. */
export function factoryConnectionAt(root: THREE.Object3D, vertexIndex: number | undefined): ObjectRef | null {
  return vertexIndex === undefined ? null : root.userData.connectionRefs?.[Math.floor(vertexIndex / 6)] ?? null;
}
