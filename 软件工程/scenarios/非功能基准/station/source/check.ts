import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
import { Box3, Vector3, type Mesh } from 'three';
import { bindScene } from '../../../../packages/presentation/src/scene';

const root = resolve(process.argv[2]);
const output = resolve(process.argv[3]);
const json = async (path: string) => JSON.parse(await readFile(path, 'utf8'));
const before = await json(resolve(root, 'source/station-1.0.1.seed.json'));
const after = await json(resolve(root, 'candidate-1.0.5-native/station.module-document.json'));
const oldData = before.data.definition.payload.data;
const newData = after.data.definition.payload.data;
const { presentation: oldPresentation, ...oldBusiness } = oldData;
const { presentation: p, ...newBusiness } = newData;
assert.deepEqual(newBusiness, oldBusiness, 'Repair must preserve every behavior/port/configuration field');
assert.equal(after.data.definition.version, '1.0.5');
assert.equal(after.data.dependencies[0].version, '1.0.5');
const simpleBefore = await json(resolve(root, 'source/simple-1.0.1.seed.json'));
const simpleAfter = await json(resolve(root, 'candidate-1.0.5-native/simple.module-document.json'));
assert.deepEqual(simpleAfter.data.definition.payload, simpleBefore.data.definition.payload);
const checks: string[] = ['Engine behavior, ports, configuration and simplified geometry retained'];
const rig = await bindScene(p, async () => { throw Error('Unexpected external asset request'); });
const index = (name: string) => {
  const value = p.nodes.findIndex((n: { name: string }) => n.name === name);
  assert(value >= 0, name);
  return value;
};
const domain = (value: Vector3) => new Vector3(value.x, -value.z, value.y);
const center = (name: string) => domain(rig.nodes.get(index(name))!.getWorldPosition(new Vector3()));
const box = (name: string) => {
  const mesh = rig.nodes.get(index(name)) as Mesh;
  mesh.geometry.computeBoundingBox();
  return mesh.geometry.boundingBox!.clone().applyMatrix4(mesh.matrixWorld);
};
const close = (a: number, b: number, message: string) => assert(Math.abs(a - b) < 1e-8, `${message}: ${a} vs ${b}`);
const touching = (a: string, b: string) => {
  const left = box(a).expandByScalar(1e-8);
  assert(left.intersectsBox(box(b)), `${a} must connect to ${b}`);
};
const home = center('cycle_ram_carriage');
const samples: Array<Record<string, unknown>> = [];
for (const progress of [0, .3, .65, 1, .3, 0]) {
  rig.apply({ progress, display_phase_id: 'cycle' } as any,
    { clip: p.timeline[0], progress } as any);
  const carriage = center('cycle_ram_carriage');
  const head = center('ram_tool_head');
  const fixture = center('workpiece_fixture_anchor');
  close(head.x, fixture.x, 'tool/workpiece X alignment throughout cycle');
  close(head.y, fixture.y, 'tool/workpiece Y alignment throughout cycle');
  for (const x of [-.36, .36]) {
    const bearing = center(`guide_bearing_${x}`);
    close(bearing.x - carriage.x, x, 'bearing X offset');
    close(bearing.y, carriage.y, 'bearing remains on carriage/guide axis');
    close(bearing.z - carriage.z, .01, 'bearing follows full carriage displacement');
  }
  const rod = box('ram_actuator_rod');
  const carriageBody = box('cycle_ram_carriage');
  close(rod.min.y, carriageBody.max.y, 'actuator rod lower end contacts carriage top');
  close(rod.max.y, 2.19, 'actuator rod upper end remains in fixed housing');
  if (progress === .3 || progress === .65) {
    const workpieceTop = fixture.z + .075 + .1 / 2;
    close(box('ram_tool_head').min.y, workpieceTop, 'ram contacts declared benchmark workpiece at full stroke');
  }
  const bounds = new Box3().setFromObject(rig.root);
  const size = domain(bounds.getSize(new Vector3()));
  assert(size.x <= 2 + 1e-8 && Math.abs(size.y) <= 2 + 1e-8 && size.z <= 3 + 1e-8, 'original 2×2×3m envelope');
  close(bounds.min.y, 0, 'feet contact ground');
  samples.push({ progress, carriage: carriage.toArray(), head: head.toArray(), bounds: { min: bounds.min.toArray(), max: bounds.max.toArray() } });
}
close(center('cycle_ram_carriage').z, home.z, 'reverse seek restores original pose');
checks.push('actual production bindScene: aligned ram, moving bearings and continuous extending actuator at six forward/reverse samples');
checks.push('full-stroke tool/workpiece contact and unchanged support offset; original envelope and ground contact at every sample');
touching('control_console', 'control_console_bracket');
touching('control_console_bracket', 'front_guard_post_-0.86');
touching('drive_motor_mount', 'motor_mount_pedestal');
touching('motor_mount_pedestal', 'welded_base_bottom');
for (const x of [-.1032, .1032]) {
  const handle = `safety_door_handle_${x < 0 ? '-0.43' : '0.43'}`;
  const door = `clear_safety_door_${x < 0 ? '-0.43' : '0.43'}`;
  for (const z of [1.77, 1.93]) {
    touching(handle, `door_handle_foot_${x}_${z}`);
    touching(door, `door_handle_foot_${x}_${z}`);
  }
}
for (let n = 0; n < 5; n++) for (const x of [-.395, .395])
  touching(`rear_vent_louver_${n}`, `rear_louver_frame_${x}`);
checks.push('actual built geometry: console, motor pedestal, handle feet and every rear louver connect to supports');
let meshes = 0, triangles = 0;
rig.root.traverse((node: any) => {
  if (!node.isMesh) return;
  meshes++;
  triangles += (node.geometry.index?.count ?? node.geometry.attributes.position.count) / 3;
});
rig.dispose();
await writeFile(output, JSON.stringify({ observed_at: new Date().toISOString(), result: 'passed',
  evidence_kind: 'Current production bindScene rigid geometry and motion checks in Node; no renderer or Native acceptance',
  exact_module: 'p201.b60.station@1.0.5', checks, samples,
  nodes: p.nodes.length, constructed_meshes: meshes, constructed_triangles: triangles,
  old_nodes: oldPresentation.nodes.length, formal_MQ_acceptance: false, Native_acceptance: false,
  benchmark_ready: false, remaining: ['same-version seven views and closeups against L03 grade', 'actual Native holder/wait/seek/save/reopen', 'materials/surface equivalence review', 'all original NFR sampling'] }, null, 2) + '\n');
console.log(JSON.stringify({ result: 'passed', checks: checks.length, samples: samples.length, nodes: p.nodes.length, benchmark_ready: false }));
