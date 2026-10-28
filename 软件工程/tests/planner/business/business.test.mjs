import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { root, source } from './load-source.mjs';
const schema=await source('schema'),draft=await source('draft');
const recorded=JSON.parse(await readFile(new URL('scenarios/真实业务设计/runtime-ui/core-initial.actual.json',root)));
const baseline=recorded.state.snapshot,catalog=JSON.parse(await readFile(new URL('packages/commands/generated/schemas/get_object.response.schema.json',root)));
const clone=()=>structuredClone(baseline);
test('explicit reset removes only the selected override through the real Device put proposal',()=>{
  const base=clone(),device=base.project.devices[0];device.configuration={cycle_seconds:{kind:'duration',value:{value:40000,unit:'millisecond',basis:'module_declared'}},retained:{kind:'number',value:3}};
  const original=structuredClone(base),next=draft.removeInstanceOverride(base,device.id,'cycle_seconds'),proposal=draft.commitProposal(base,next,false);
  assert.deepEqual(base,original);assert.equal(proposal.commands.length,1);assert.equal(proposal.commands[0].kind,'put_object');assert.equal(proposal.commands[0].data.kind,'device');
  const expected=structuredClone(device);delete expected.configuration.cycle_seconds;assert.deepEqual(proposal.commands[0].data.data,expected);
  const current=structuredClone(base);current.project.devices[0].configuration.cycle_seconds.value.value=50000;
  assert.ok(draft.mergeDraft(base,next,current).conflicts.length,'concurrent replacement needs a visible choice');
});
test('all actual canonical business source values satisfy their real public schemas',()=>{
  for(const scenario of baseline.scenarios)assert.deepEqual(schema.validateForm(schema.fieldSchema('ScenarioInput',catalog),scenario,catalog),[]);
  for(const [collection,type]of Object.entries({products:'ProductRevision',boms:'BomDefinition',routes:'RouteDefinition',resources:'ResourceInstance',calendars:'Calendar',setup_policies:'DeviceSetupPolicy',devices:'DeviceInstance'}))for(const value of baseline.project[collection])assert.deepEqual(schema.validateForm(schema.fieldSchema(type,catalog),value,catalog),[]);
});
test('edit order and arrival in one current-revision Scenario command, preserve exact frozen input',()=>{
  const next=clone(),before=JSON.stringify(baseline);next.scenarios[0].orders[0].due_at_ms=180000;next.scenarios[0].arrivals[0].at_ms=110000;
  const p=draft.commitProposal(baseline,next,false);assert.equal(p.commands.length,1);assert.equal(p.commands[0].kind,'put_object');assert.equal(p.commands[0].data.kind,'scenario');assert.equal(p.base_project_revision,1);assert.equal(p.scenario_revisions[next.scenarios[0].id],1);assert.equal(p.commands[0].data.data.revision,1);assert.equal(p.commands[0].data.data.project_revision,1);assert.equal(JSON.stringify(baseline),before);
});
test('calendar/product/BOM/route/source changes use their canonical Commands and no run/stock object',()=>{
  const next=clone();next.project.calendars[0].title='新的日历声明';next.project.products[0].artifact_refs=[];next.project.boms[0].rows[0].quantity.value.coefficient+=1;next.project.routes[0].revision='explicit-r2';next.project.sources.push({id:'manual-source',title:'人工估计来源',basis:'user_provided'});
  const p=draft.commitProposal(baseline,next,false);assert.ok(p.commands.some(c=>c.kind==='put_object'&&c.data.kind==='calendar'));assert.ok(p.commands.some(c=>c.kind==='put_object'&&c.data.kind==='bom'));assert.ok(p.commands.some(c=>c.kind==='put_object'&&c.data.kind==='route'));assert.ok(p.commands.some(c=>c.kind==='set_project_settings'));assert.ok(p.commands.every(c=>!['run_scenario','item','undo','redo'].includes(c.kind)));
});
test('setup field gap is explicit; cannot silently drop policy while claiming saved',()=>{
  const next=clone();next.project.setup_policies[0].initial_family='explicit-next';assert.throws(()=>draft.commitProposal(baseline,next,false),/换单矩阵/);const p=draft.commitProposal(baseline,next,true);assert.deepEqual(p.commands.find(c=>c.kind==='set_project_settings').data.setup_policies,next.project.setup_policies);
});
test('new Scenario is revision1 and preserves old Scenario and original Run',()=>{
  const next=clone(),old=JSON.stringify(recorded.run);next.scenarios.push({...structuredClone(next.scenarios[0]),id:'next-scenario',title:'明确新方案',revision:100,project_revision:100});const p=draft.commitProposal(baseline,next,false);assert.equal(p.commands[0].data.data.id,'next-scenario');assert.equal(p.commands[0].data.data.revision,1);assert.equal(p.commands[0].data.data.project_revision,1);assert.deepEqual(p.scenario_revisions,{});assert.equal(JSON.stringify(recorded.run),old);
});
test('exact decimal input retains base coefficient/places and rejects overflow/nonfinite/fraction aliases',()=>{
  assert.deepEqual(schema.parseDecimal('0.1250'),{coefficient:1250,decimal_places:4});assert.equal(schema.decimalText(schema.parseDecimal('12345.678900')),'12345.678900');for(const text of ['','-1','1e2','Infinity','135000/7','9007199254740992'])assert.throws(()=>schema.parseDecimal(text));
});
test('unknown required number stays blank, optional price remains null, schema rejects NaN/unsafe integer',()=>{
  assert.equal(schema.emptyValue({type:'integer'},catalog),null);assert.equal(schema.emptyValue({anyOf:[{$ref:'#/$defs/DecimalAmount'},{type:'null'}]},catalog),null);assert.ok(schema.validateForm({type:'number'},NaN,catalog).length);assert.ok(schema.validateForm({type:'integer'},Number.MAX_SAFE_INTEGER+1,catalog).length);
});
test('copy a batch assigns fresh owned item IDs and remaps inner holder/composition only',()=>{
  const arrival={id:'batch',at_ms:1000,items:[{id:'box',components:['part'],contents:['part'],holder:{kind:'location',id:'raw'}},{id:'part',components:[],contents:[],holder:{kind:'container',id:'box'}}],destination:'raw'};const copied=draft.copyRecord(arrival,'ArrivalPlan');assert.notEqual(copied.id,arrival.id);assert.notEqual(copied.items[0].id,'box');assert.equal(copied.items[0].contents[0],copied.items[1].id);assert.equal(copied.items[1].holder.id,copied.items[0].id);assert.equal(arrival.items[0].id,'box');
});
test('route copy remaps own predecessor step IDs without changing exact operation/device binding',()=>{
  const route={id:'route',revision:'r1',steps:[{id:'s1',predecessors:[],operation_ref:{id:'op',version:'1'},device_id:'D1'},{id:'s2',predecessors:['s1'],operation_ref:{id:'op2',version:'1'},device_id:'D2'}]};const copied=draft.copyRecord(route,'RouteDefinition');assert.equal(copied.steps[1].predecessors[0],copied.steps[0].id);assert.notEqual(copied.steps[0].id,'s1');assert.deepEqual(copied.steps[0].operation_ref,route.steps[0].operation_ref);assert.equal(copied.steps[0].device_id,'D1');
});
test('copy nullable quality attempt and plain role strings without crashing or inventing values',()=>{assert.equal(draft.copyRecord(null,'TypedValue'),null);assert.equal(draft.copyRecord('worker','RoleId'),'worker');});
test('three-way refresh preserves local due edit and concurrent title, no silent object overwrite',()=>{
  const mine=clone(),current=clone();mine.scenarios[0].orders[0].due_at_ms=180000;current.scenarios[0].title='真实并发标题';current.project.revision=2;current.scenarios[0].revision=2;current.scenarios[0].project_revision=2;const m=draft.mergeDraft(baseline,mine,current);assert.deepEqual(m.conflicts,[]);assert.equal(m.value.scenarios[0].title,current.scenarios[0].title);assert.equal(m.value.scenarios[0].orders[0].due_at_ms,180000);assert.equal(m.value.project.revision,2);
});
test('same-field conflict requires explicit choice while unrelated new data remains',()=>{
  const base={rows:[{id:'A',due:10,title:'original'}]},mine={rows:[{id:'A',due:20,title:'original'}]},current={rows:[{id:'A',due:30,title:'remote'},{id:'B',due:40}]};const m=draft.mergeDraft(base,mine,current);assert.equal(m.conflicts.length,1);assert.equal(m.value.rows[0].due,30);const accepted=draft.mergeDraft(base,mine,current,{[m.conflicts[0].path]:'mine'});assert.deepEqual(accepted.conflicts,[]);assert.equal(accepted.value.rows[0].due,20);assert.equal(accepted.value.rows[0].title,'remote');assert.equal(accepted.value.rows.length,2);
});
test('schema discriminator and decimal metadata survive canonical refs and nullable wrappers',()=>{
  const s=schema.resolve(schema.fieldSchema('CostInput',catalog),catalog).properties.rate,real=schema.branches(s,catalog).find(v=>v.type!=='null');assert.equal(real.title,'DecimalAmount');const typed=schema.fieldSchema('TypedValue',catalog),branch=schema.branchFor(typed,{kind:'duration',value:{value:1,unit:'millisecond',basis:'user_provided'}},catalog);assert.equal(branch.properties.kind.const,'duration');
});
