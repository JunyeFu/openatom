import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';

const root = new URL('../../../', import.meta.url);
const output = new URL('.work/single-window-20261018/p201-public-native23/static-region-entry/compiled/', root);
await mkdir(output, { recursive: true });
await writeFile(new URL('site-constraints.mjs', output), stripTypeScriptTypes(await readFile(new URL('apps/planner/src/features/factory/siteConstraints.ts', root), 'utf8')));
const { readSiteConstraints, siteConstraintsDraft } = await import(new URL('site-constraints.mjs', output));
const region = { id: 'lane-H1', kind: 'static_lane', clearance_m: 0.25,
  polygon_m: [{ x: 1.25, y: -2 }, { x: 7.5, y: -2 }, { x: 7.5, y: 4 }, { x: 5, y: 3 }, { x: 1.25, y: 4 }] };

test('missing declarations stay missing and explicit zero stays zero', () => {
  assert.deepEqual(readSiteConstraints(siteConstraintsDraft()), { clearance_m: null, regions: [] });
  assert.equal(readSiteConstraints({ clearance: '0', regions: [] }).clearance_m, 0);
  assert.equal(readSiteConstraints({ clearance: '  ', regions: [] }).clearance_m, null);
});

test('saved irregular regions and exact margins round-trip without rectangle substitution or source mutation', () => {
  const original = { regions: [region, { ...region, id: 'keep-out', kind: 'keep_out', clearance_m: null }], clearance_m: 1.125 };
  const before = structuredClone(original), draft = siteConstraintsDraft(original);
  assert.deepEqual(readSiteConstraints(draft), original);
  draft.regions[0].points[0].x = '99';
  assert.deepEqual(original, before);
});

test('incomplete points cannot be coerced to a zero coordinate', () => {
  const draft = siteConstraintsDraft({ regions: [region] });
  draft.regions[0].points[2].y = '';
  assert.throws(() => readSiteConstraints(draft), /顶点 3 Y/);
  draft.regions[0].points[2].y = 'Infinity';
  assert.throws(() => readSiteConstraints(draft), /有限数值/);
});

test('negative margins, duplicate region identities and incomplete polygons are rejected before dispatch', () => {
  assert.throws(() => readSiteConstraints({ clearance: '-1', regions: [] }), /不得小于零/);
  const draft = siteConstraintsDraft({ regions: [region, region] });
  assert.throws(() => readSiteConstraints(draft), /不重复/);
  draft.regions.pop(); draft.regions[0].clearance = '-0.25';
  assert.throws(() => readSiteConstraints(draft), /区域 lane-H1 净距/);
  draft.regions[0].clearance = ''; draft.regions[0].points.length = 2;
  assert.throws(() => readSiteConstraints(draft), /至少需要三个顶点/);
});
