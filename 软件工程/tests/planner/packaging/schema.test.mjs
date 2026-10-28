import test from 'node:test';
import assert from 'node:assert/strict';
import { packagingSchemas } from '../../../apps/planner/src/features/packaging/schemas.ts';

test('part and material titles remain editable required domain fields in the projected Job schema', () => {
  for (const name of ['parts', 'materials']) {
    const item = packagingSchemas.PackageStructureJob.properties[name].items;
    assert.ok(item.required.includes('title'));
    assert.deepEqual(item.properties.title, { type: 'string' });
  }
});
