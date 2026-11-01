import { readFile, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { out } from './current-core.mjs';

const variant = process.argv[2], attempts = process.argv.slice(3, 6);
const outputName = process.argv[6] ?? `b60-${variant}.repeat-comparison.json`;
assert.ok(['balanced', 'dense'].includes(variant)); assert.equal(attempts.length, 3, 'Pass the three original frozen-input attempt names');
const rows = [];
for (const attempt of attempts) {
  const reference = `b60-${variant}.current.${attempt}`;
  const result = JSON.parse(await readFile(new URL(reference + '.result.json', out), 'utf8'));
  assert.equal(result.actual_result, 'passed_current_Host_Engine_counts_and_identity_not_Native_performance');
  const run = JSON.parse(await readFile(new URL(reference + '.run.json', out), 'utf8'));
  const { id, ...semantic } = run;
  rows.push({ attempt, result, id, semantic });
}
for (const row of rows.slice(1)) {
  assert.equal(row.result.frontier, rows[0].result.frontier);
  assert.equal(row.result.executable, rows[0].result.executable);
  assert.deepEqual(row.semantic, rows[0].semantic, 'Same frozen input must preserve every Run field except its fresh RunId');
}
const result = { kind: 'three_actual_same_frozen_input_Run_semantic_comparisons', variant, attempts,
  actual_result: 'passed', excluded_field: 'RunResult.id', frontier: rows[0].result.frontier,
  runs: rows.map(row => ({ attempt: row.attempt, run_id: row.id, counts: row.result.counts })),
  all_other_Run_fields_equal: true, Native_performance: false, human_acceptance: null };
await writeFile(new URL(outputName, out), JSON.stringify(result, null, 2));
console.log(JSON.stringify(result));
