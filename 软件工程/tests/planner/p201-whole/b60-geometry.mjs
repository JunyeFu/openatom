import { build } from 'vite';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { Box3, Vector3, Mesh } from 'three';
const root = new URL('../../../', import.meta.url), out = new URL('.work/p201-whole-20261017/b60-geometry/', root);
await build({ configFile: false, root: fileURLToPath(root), build: { target: 'es2022', outDir: fileURLToPath(out), emptyOutDir: false,
  lib: { entry: fileURLToPath(new URL('packages/presentation/src/index.ts', root)), formats: ['es'], fileName: () => 'presentation.mjs' }, rollupOptions: { external: ['three', /^three\//] } } });
const { bindScene, declaredProgress, activeClip } = await import(new URL('presentation.mjs', out));
const report = { kind: 'current_shared_Native_scene_parser_geometry_and_declared_motion_only', Native_app: false, Engine: false, quality_acceptance: null, lods: [] };
for (const [lod, file] of [['detailed', 'p201.b60.station'], ['simplified', 'p201.b60.simple']]) {
  const value = JSON.parse(await readFile(new URL('tests/planner/p201-whole/fixtures/b60/' + file + '.module.json', root), 'utf8')).data.definition;
  const presentation = lod === 'detailed' ? value.payload.data.presentation : value.payload.data;
  const scene = await bindScene(presentation, async () => { throw new Error('This benchmark declares zero external assets'); });
  scene.root.updateMatrixWorld(true);
  const box = new Box3().setFromObject(scene.root), size = box.getSize(new Vector3());
  assert.ok(size.x <= 2.000001 && size.y <= 3.000001 && size.z <= 2.000001, JSON.stringify({ lod, size }));
  assert.ok(box.min.y >= -1e-6, 'No geometry below its declared floor anchor');
  let meshes = 0, triangles = 0, materials = new Set(); scene.root.traverse(object => { if (object instanceof Mesh) { meshes++; triangles += (object.geometry.index?.count ?? object.geometry.attributes.position.count) / 3; materials.add(object.material); } });
  assert.equal(meshes, presentation.nodes.length, 'Every declared structural component must reach the shared scene parser');
  const track = presentation.actions[0], target = scene.nodes.get(track.node_index), positions = [];
  for (const u of [0, .5, 1]) { const point = declaredProgress({ phase_ref: 'cycle', view_u: u }); scene.apply(point, activeClip(presentation, point)); scene.root.updateMatrixWorld(true); positions.push(target.getWorldPosition(new Vector3()).toArray()); }
  assert.ok(Math.abs(positions[0][1] - 2.05) < 1e-6 && Math.abs(positions[1][1] - 1.805) < 1e-6 && Math.abs(positions[2][1] - 2.05) < 1e-6);
  const structural = presentation.nodes.map(node => node.name);
  const inputAnchor = scene.nodes.get(presentation.port_anchors.input.data), outputAnchor = scene.nodes.get(presentation.port_anchors.output.data); assert.ok(inputAnchor && outputAnchor); assert.ok(inputAnchor.getWorldPosition(new Vector3()).z > outputAnchor.getWorldPosition(new Vector3()).z);
  assert.ok(structural.includes('welded_base_bottom') && structural.includes('workpiece_fixture_anchor') && structural.includes('gantry_crossbeam'));
  if (lod === 'detailed') for (const name of ['rear_lower_service_cover', 'removable_bottom_service_tray', 'drive_motor_body', 'transmission_guard', 'control_console', 'rear_vent_louver_0', 'clear_safety_door_-0.43', 'roller_bearing_0_-0.73']) assert.ok(structural.includes(name), name);
  report.lods.push({ lod, meshes, triangles, distinct_material_objects: materials.size, bounds_renderer_y_up: { min: box.min.toArray(), max: box.max.toArray() }, envelope_factory_m: [size.x, size.z, size.y], declared_cycle_ram_positions_renderer: positions, external_asset_count: 0 }); scene.dispose();
}
const data = JSON.parse(await readFile(new URL('tests/planner/p201-whole/fixtures/b60/balanced.input.json', root), 'utf8'));
assert.equal(new Set(data.scenario.initial_conditions.items.map(item => item.id)).size, 3000);
assert.deepEqual(data.project.halls.map(hall => data.project.devices.filter(device => device.hall_id === hall.id).length), [20, 20, 20]);
report.input_counts = { halls: data.project.halls.length, devices: data.project.devices.length, actual_initial_identities: data.scenario.initial_conditions.items.length, routes: data.project.routes.length };
await writeFile(new URL('geometry.json', out), JSON.stringify(report, null, 2));
console.log(JSON.stringify({ lods: report.lods.map(({ lod, meshes, triangles, envelope_factory_m }) => ({ lod, meshes, triangles, envelope_factory_m })), input_counts: report.input_counts, Engine: false, Native_app: false }));
