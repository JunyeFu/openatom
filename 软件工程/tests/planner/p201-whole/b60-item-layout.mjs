import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir, copyFile } from 'node:fs/promises';
import path from 'node:path';

// Frozen snapshot display positions; no queue capacity, holder or simulation change.
export function b60LocationLayout(location) {
  const count = location.purpose === 'buffer' ? 1 : 500;
  return { assignment: 'item_id_order', slots: Array.from({ length: count }, (_, index) => ({
    item_type_ref: { id: 'benchmark-item-v1', revision: 'r1' }, anchor_role: null,
    offset: { translation: count === 1 ? { x: 0, y: 0, z: .05 } : {
      x: ((index % 10) - 4.5) * .22,
      y: ((Math.floor(index / 10) % 10) - 4.5) * .22,
      z: .05 + Math.floor(index / 100) * .12,
    }, rotation: { x: 0, y: 0, z: 0, w: 1 }, scale: { x: 1, y: 1, z: 1 },
    unit: 'meter', coordinate_frame: 'right_handed_z_up' },
  })) };
}

const root = process.cwd(), output = path.resolve(process.argv[2]);
const sources = {
  balanced: '.work/cu-closeout-20261024/root-C17-balanced-readiness/saved-run-r6',
  dense: '.work/cu-closeout-20261023/b60-105-host/saved/dense',
};
const read = async p => JSON.parse(await readFile(p, 'utf8'));
const write = (p, value) => writeFile(p, JSON.stringify(value, null, 2) + '\n', { flag: 'wx' });
await mkdir(output, { recursive: true });
const receipt = [];
for (const [name, relative] of Object.entries(sources)) {
  const source = path.join(root, relative), target = path.join(output, 'inputs', name);
  await mkdir(target, { recursive: true });
  const manifest = await read(path.join(source, 'project.json'));
  const original = structuredClone(manifest);
  manifest.data.documents = manifest.data.documents.filter(d => ['active_project', 'scenario_input', 'module_definition', 'asset'].includes(d.role));
  const projectDoc = manifest.data.documents.find(d => d.role === 'active_project');
  const scenarioDoc = manifest.data.documents.find(d => d.role === 'scenario_input');
  const project = await read(path.join(source, projectDoc.file_ref.path));
  const scenario = await read(path.join(source, scenarioDoc.file_ref.path));
  const oldProject = structuredClone(project), oldScenario = structuredClone(scenario);
  const id = `NFR-B60-${name}-layout-v1`, scenarioId = `B60-${name}-layout-v1`;
  Object.assign(project.data, { id, revision: 7, title: `B60 1.0.5 · ${name} · 明确展示槽`, scenario_refs: [scenarioId] });
  for (const location of project.data.locations) location.item_layout = b60LocationLayout(location);
  Object.assign(scenario.data, { id: scenarioId, revision: 7, project_revision: 7, title: project.data.title });
  Object.assign(manifest.data, { project_id: id, project_revision: 7, title: project.data.title });
  Object.assign(projectDoc, { object_ref: { kind: 'project', id }, revision: 7 });
  Object.assign(scenarioDoc, { object_ref: { kind: 'scenario', id: scenarioId }, revision: 7 });
  const productionOnly = p => {
    const copy = structuredClone(p.data);
    for (const key of ['id', 'revision', 'title', 'scenario_refs']) delete copy[key];
    copy.locations.forEach(l => { delete l.item_layout; });
    return copy;
  };
  assert.deepEqual(productionOnly(project), productionOnly(oldProject));
  for (const key of Object.keys(oldScenario.data).filter(k => !['id', 'revision', 'project_revision', 'title'].includes(k)))
    assert.deepEqual(scenario.data[key], oldScenario.data[key], key);
  assert.equal(scenario.data.initial_conditions.items.length, 3000);
  for (const location of project.data.locations) {
    const slots = location.item_layout.slots;
    assert.equal(new Set(slots.map(s => JSON.stringify(s.offset.translation))).size, slots.length);
  }
  for (const document of manifest.data.documents) {
    const relativeFile = document.file_ref.path;
    const dest = path.join(target, relativeFile);
    await mkdir(path.dirname(dest), { recursive: true });
    if (document.role === 'active_project') await write(dest, project);
    else if (document.role === 'scenario_input') await write(dest, scenario);
    else {
      await copyFile(path.join(source, relativeFile), dest, 1);
      assert.deepEqual(await readFile(dest), await readFile(path.join(source, relativeFile)));
    }
  }
  await write(path.join(target, 'project.json'), manifest);
  assert.deepEqual(await read(path.join(source, 'project.json')), original);
  receipt.push({ variant: name, source: relative, input: path.relative(root, target), project_id: id,
    revision: 7, source_revision: 6, production_semantics_unchanged: true, initial_identities: 3000,
    locations_with_layout: project.data.locations.length, endpoint_slots: 500, buffer_slots: 1,
    device_layouts_unchanged: true, fixed_modules_and_assets_byte_equal: true,
    old_saved_Run_untouched: true, actual_Host_Run: false, Native: false, formal_NFR: false });
}
await write(path.join(output, 'preparation.json'), receipt);
console.log(JSON.stringify(receipt));
