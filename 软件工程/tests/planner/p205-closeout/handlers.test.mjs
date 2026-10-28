import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {root,out,runtime,event} from './component.mjs';
const fixture=JSON.parse(await readFile(new URL('packages/commands/tests/fixtures/production/p104-A-first.json',root),'utf8'));
const observations=[];
const clone=v=>structuredClone(v);
async function editor(extra={}){
 const rt=runtime(),{BusinessEditor}=rt.source('apps/planner/src/features/business/BusinessEditor.tsx');
 const calls=[],props={project:clone(fixture.project),scenarios:[clone(fixture.scenario)],scenarioId:fixture.scenario.id,
  modules:new Map(),history:{can_undo:false,can_redo:false,boundary_reason:null},canEditSetupPolicies:true,
  onCommit:async proposal=>{calls.push(proposal);return {ok:false,error:{code:'RevisionConflict',message:'actual submission attempt',field_errors:[],object_refs:[]}};},onRefresh:async()=>{},onRun:async()=>{},onScenarioChange(){},onLocate(){},...extra};
 const view=rt.mount(BusinessEditor,props);await view.flush();return {rt,view,props,calls};
}
function tab(view,label){return view.act(()=>view.button(label).props.onClick());}
function change(view,label,value){return view.act(()=>{const input=view.find(label);assert.ok(input,`field ${label}`);input.props.onChange(event(input,{target:{value}}));});}

test('clean refresh preserves the selected Scenario identity when another Scenario is inserted before it',async()=>{
 const {view,props}=await editor(),other=clone(props.scenarios[0]);other.id='AAA-P205';other.title='Another scenario';other.orders[0].due_at_ms=999000;
 const newer=clone(props.scenarios[0]);newer.title='Still selected';newer.orders[0].due_at_ms=181000;
 await view.act(()=>view.render({project:{...props.project,revision:2},scenarios:[other,newer]}));
 const current=view.find('业务当前方案'),due=view.find('交期（ms，可未知）');
 observations.push({case:'selected_scenario_after_insert',selected_index:current.props.value,visible_due:due?.props.value});
 assert.equal(current.props.value,1);assert.equal(due.props.value,'181000');view.unmount();
});
test('simultaneous Scenario switch and edit intent selects a record in the requested Scenario',async()=>{
 const first=clone(fixture.scenario),second=clone(fixture.scenario);second.id='P205-SECOND';second.orders=[{...clone(second.orders[0]),id:'SECOND-ORDER',due_at_ms:222000}];
 const {view}=await editor({scenarios:[first,second]});
 await view.act(()=>view.render({scenarioId:second.id,editIntent:{id:'intent-second',section:'orders',object_id:'SECOND-ORDER'}}));
 observations.push({case:'scenario_and_intent_same_update',missing:view.text().includes('未找到指定记录'),due:view.find('交期（ms，可未知）')?.props.value});
 assert.ok(!view.text().includes('未找到指定记录'));assert.equal(view.find('交期（ms，可未知）').props.value,'222000');view.unmount();
});
test('Ctrl+Enter while IME composition is active cannot apply an already-previewed draft',async()=>{
 const {view,calls}=await editor();await change(view,'交期（ms，可未知）','180001');
 await view.act(()=>view.button('检查变更').props.onClick());assert.ok(view.find('业务变更预览'));
 const region=view.find('业务输入'),key=event(view.find('交期（ms，可未知）'),{key:'Enter',ctrlKey:true,isComposing:true});
 await view.act(()=>region.props.onKeyDown(key));observations.push({case:'composing_shortcut',submission_attempts:calls.length,preview_retained:!!view.find('业务变更预览')});
 assert.equal(calls.length,0);assert.ok(view.find('业务变更预览'));assert.equal(key.defaultPrevented,false);view.unmount();
});
test('new unsaved Scenario stays selected and retains its fields when sections change',async()=>{
 const {view,props}=await editor();await view.act(()=>view.button('复制为新方案').props.onClick());
 await change(view,'名称','Explicit unsaved copy');await tab(view,'任务与订单');
 assert.equal(view.find('业务当前方案').props.value,1);await tab(view,'方案与观察窗');assert.equal(view.find('名称').props.value,'Explicit unsaved copy');
 assert.equal(props.scenarios.length,1);view.unmount();
});
test('a first Scenario loaded into an initially empty panel retains the existing automatic first selection',async()=>{
 const {view}=await editor({scenarios:[],scenarioId:null});await view.act(()=>view.render({scenarios:[clone(fixture.scenario)]}));
 assert.equal(view.find('业务当前方案').props.value,0);assert.equal(view.find('交期（ms，可未知）').props.value,'160000');view.unmount();
});
test('new snapshot, explicit Scenario selection and intent arriving together resolve the new record',async()=>{
 const second=clone(fixture.scenario);second.id='P205-ARRIVED';second.orders=[{...second.orders[0],id:'ARRIVED-ORDER',due_at_ms:223000}];
 const {view,props}=await editor();await view.act(()=>view.render({project:{...props.project,revision:2},scenarios:[clone(fixture.scenario),second],scenarioId:second.id,editIntent:{id:'new-snapshot-intent',section:'orders',object_id:'ARRIVED-ORDER'}}));
 assert.equal(view.find('业务当前方案').props.value,1);assert.ok(!view.text().includes('未找到指定记录'));assert.equal(view.find('交期（ms，可未知）').props.value,'223000');view.unmount();
});
test.after(async()=>writeFile(new URL(process.env.P205_SOURCE_BEFORE==='1'?'handler-before-observations.json':'handler-observations.json',out),JSON.stringify(observations,null,2)));
