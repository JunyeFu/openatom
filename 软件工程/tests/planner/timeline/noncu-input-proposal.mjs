// Materialize an actual timeline-produced command for the real Host consumer.
import {readFile,writeFile} from 'node:fs/promises';import assert from 'node:assert/strict';import {source,rootUrl} from './load-source.mjs';
const {inputProposal}=await source('input');
const fixture=JSON.parse(await readFile(new URL('packages/commands/tests/fixtures/production/p202-ix01-baseline.json',rootUrl),'utf8'));
const fields=[{kind:'arrival',id:fixture.scenario.arrivals.at(-1).id},{kind:'task_release',id:fixture.scenario.production_tasks[0].id}];
const before=JSON.stringify(fixture),proposal=inputProposal(fixture.project,fixture.scenario,fields,1000);
assert.equal(JSON.stringify(fixture),before);assert.equal(proposal.base_project_revision,1);assert.equal(proposal.changes.length,2);
assert.deepEqual(proposal.changes.map(c=>c.after_ms-c.before_ms),[1000,1000]);
await writeFile(new URL('.work/delivery/input-proposal.json',rootUrl),JSON.stringify({producer:'actual timeline/input.ts inputProposal',expected:{changes:2,delta_ms:1000,new_project_revision:2,old_Run_unchanged:true,stale_revision:'RevisionConflict'},proposal},null,2));
console.log('Saved actual timeline command for frozen Project/Scenario r1; arrival and release +1000ms, due/inventory unchanged');
