import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {root,source} from './load-source.mjs';
const draft=await source('draft'),schema=await source('schema');
const w5=JSON.parse(await readFile(new URL('.work/whole-packages/window5-timeline-evaluation/native-structured-79676/structured-committed.json',root),'utf8'));
const catalog=JSON.parse(await readFile(new URL('packages/commands/generated/schemas/get_object.response.schema.json',root),'utf8'));
// Before the fix the actual postcondition uses equal; preserve that red path.
const matches=(request,actual)=>(draft.setupPoliciesEqual??draft.equal)(request,actual);
const actual=w5.project.setup_policies;
function formRequest(){
 const policy=schema.emptyValue(schema.fieldSchema('DeviceSetupPolicy',catalog),catalog),rule=schema.emptyValue(schema.fieldSchema('ChangeoverRule',catalog),catalog);
 Object.assign(rule,{from_family:'A',to_family:'B',preparation_mode:'internal'});
 Object.assign(rule.duration,{value:23000,unit:'millisecond',basis:'user_provided'});
 Object.assign(policy,{device_id:'w5-structured-final:Assembly01',initial_family:'A',changeovers:[rule]});return [policy];
}
const requested=formRequest();
const output=new URL('scenarios/真实业务设计/runtime-ui/readback-repair/',root);await mkdir(output,{recursive:true});
await writeFile(new URL('w5-input-reconstruction.json',output),JSON.stringify({source:'Original W5 Native canonical readback, request reconstructed by unchanged actual Schema new-row initializer; not an original captured request',Native_project_revision:w5.project.revision,Native_scenario_revision:w5.scenario.revision,requested,actual,strict_equal:draft.equal(requested,actual),postcondition_match:matches(requested,actual)},null,2));
test('W5 exact23000ms new-row request accepts the actual Native canonical readback',()=>{
 assert.equal(w5.project.revision,3);assert.equal(w5.scenario.revision,2);assert.equal(Object.hasOwn(requested[0],'first_article'),false);assert.equal(Object.hasOwn(requested[0].changeovers[0].duration,'source_ref'),false);assert.equal(actual[0].first_article,null);assert.equal(actual[0].changeovers[0].duration.source_ref,null);assert.equal(matches(requested,actual),true);const sameFields=structuredClone(requested);sameFields[0].first_article=null;sameFields[0].changeovers[0].duration.source_ref=null;assert.equal(JSON.stringify(sameFields)===JSON.stringify(actual),false);assert.equal(draft.equal(sameFields,actual),true);
});
test('JSON object key order is irrelevant at every depth; scalar types, null/absence and array order stay distinct',()=>{
 assert.equal(draft.equal({a:1,b:{x:2,y:3}},{b:{y:3,x:2},a:1}),true);
 for(const[a,b]of [[{}, {value:null}],[{value:null},{value:0}],[{value:undefined},{}],[{value:23000},{value:'23000'}],[[],null],[[1,2],[2,1]]])assert.equal(draft.equal(a,b),false);
});
test('only four established setup Option fields normalize omission; required data/default arrays are not invented',()=>{
 const cold=structuredClone(actual);cold[0].initial_family=null;cold[0].changeovers[0].from_family=null;const omitted=structuredClone(cold);
 delete omitted[0].initial_family;delete omitted[0].first_article;delete omitted[0].changeovers[0].from_family;delete omitted[0].changeovers[0].duration.source_ref;
 assert.equal(matches(omitted,cold),true);assert.equal(draft.equal(omitted,cold),false);
 const missingRequired=structuredClone(actual);delete missingRequired[0].changeovers[0].resources;assert.equal(matches(missingRequired,actual),false);
 const foreignNull=structuredClone(actual);foreignNull[0].unrecognized_nullable=null;assert.equal(matches(foreignNull,actual),false);
});
test('real readback changes to matrix/value/unit/basis/source/family/first-article remain mismatches',()=>{
 const cases=[['missing matrix',v=>v.splice(0)],['value',v=>v[0].changeovers[0].duration.value=23001],['zero',v=>v[0].changeovers[0].duration.value=0],['unit',v=>v[0].changeovers[0].duration.unit='second'],['basis',v=>v[0].changeovers[0].duration.basis='unknown'],['source',v=>v[0].changeovers[0].duration.source_ref='explicit-other-source'],['family',v=>v[0].initial_family=null],['first article',v=>v[0].first_article={route_ref:{id:'route',revision:'r1'},family:'B',inspection_module:{id:'inspection',version:'1.0.0'},gate_id:'gate'}]];
 for(const missing of [[],null,undefined])assert.equal(matches(requested,missing),false);
 for(const[name,change]of cases){const next=structuredClone(actual);change(next);assert.equal(matches(requested,next),false,name);}
});
test('matrix/resource array order and exact resource quantities/units/qualifications are preserved',()=>{
 const request=structuredClone(actual),rule=request[0].changeovers[0];rule.resources=[{role:'worker',quantity:{value:{coefficient:1,decimal_places:0},unit:'person'},qualifications:['q1','q2']},{role:'fixture',quantity:{value:{coefficient:1,decimal_places:0},unit:'piece'},qualifications:[]}];request[0].changeovers.push({...structuredClone(rule),from_family:'B',to_family:'A'});
 for(const change of[v=>v[0].changeovers.reverse(),v=>v[0].changeovers[0].resources.reverse(),v=>v[0].changeovers[0].resources[0].quantity.value.coefficient=2,v=>v[0].changeovers[0].resources[0].quantity.unit='piece',v=>v[0].changeovers[0].resources[0].qualifications.reverse()]){const next=structuredClone(request);change(next);assert.equal(matches(request,next),false);}
});
