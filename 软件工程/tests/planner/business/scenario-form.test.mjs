// Run production BusinessSession handlers without claiming browser/Native input.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import vm from 'node:vm';
import {transformWithOxc} from 'vite';
import {root,source} from './closeout-source.mjs';
const draft=await source('draft'),schema=await source('schema'),{sections}=await source('sections');
const catalog=JSON.parse(await readFile(new URL('packages/commands/generated/schemas/get_object.response.schema.json',root),'utf8'));
const baseline=JSON.parse(await readFile(new URL('scenarios/真实业务设计/runtime-ui/core-initial.actual.json',root),'utf8')).state.snapshot;
const raw=await readFile(new URL('apps/planner/src/features/business/BusinessEditor.tsx',root),'utf8');
const compiled=await transformWithOxc(raw,'BusinessEditor.tsx',{jsx:{runtime:'classic',pragma:'h',pragmaFrag:'Fragment'}});
const code=compiled.code.replace(/^import .*;\r?\n/gm,'').replace('export function BusinessEditor','function BusinessEditor')+'\nmodule.exports=BusinessSession;';
function mount(snapshot){
  let cursor=0,first=true,tree;const states=[],effects=[],refs=[],module={exports:{}};
  const props={...snapshot,scenarioId:snapshot.scenarios[0].id,busy:false,modules:[],onDirtyChange(){},onCommit(){assert.fail('checking a proposal cannot commit');}};
  vm.runInNewContext(code,{...draft,...schema,sections,module,SchemaField:'SchemaField',BusinessRuntime:'BusinessRuntime',DeclaredParameters:'DeclaredParameters',labels:{},Fragment:'Fragment',
    h:(type,attributes,...children)=>({type,props:{...attributes,children}}),
    useState(initial){const i=cursor++;if(!(i in states))states[i]=typeof initial==='function'?initial():initial;return[states[i],next=>{states[i]=typeof next==='function'?next(states[i]):next;}];},
    useRef(initial){const i=cursor++;return refs[i]??={current:initial};},useMemo:fn=>fn(),useEffect(fn){if(first)effects.push(fn);},
    loadBusinessSchema:async()=>catalog});
  function nodes(node){if(Array.isArray(node))return node.flatMap(nodes);return node?.props?[node,...nodes(node.props.children)]:[];}
  function text(node){if(Array.isArray(node))return node.map(text).join('');return node?.props?text(node.props.children):node==null||typeof node==='boolean'?'':String(node);}
  const view={render(){cursor=0;tree=module.exports(props);first=false;return view;},button(label){const n=nodes(tree).find(n=>n.type==='button'&&text(n)===label);assert.ok(n,label);return n;},
    input(){return nodes(tree).find(n=>n.type==='SchemaField'&&n.props.label==='方案输入');},text:()=>text(tree),nodes:()=>nodes(tree)};
  view.render();for(const effect of effects)effect();return view;
}
async function scenarioForm(explicitNull){
  const snapshot=structuredClone(baseline);snapshot.scenarios=snapshot.scenarios.slice(0,1);
  if(explicitNull)snapshot.scenarios[0].acceptance_goal=null;else delete snapshot.scenarios[0].acceptance_goal;
  const view=mount(snapshot);await Promise.resolve();view.render();view.button('方案与观察窗').props.onClick();view.render();
  assert.ok(view.input(),'real Scenario SchemaField mounted');return{view,snapshot};
}
for(const explicitNull of [false,true])test(`edit and restore Scenario title makes no command; acceptance goal ${explicitNull?'explicit null':'omitted'}`,async()=>{
  const{view,snapshot}=await scenarioForm(explicitNull),original=structuredClone(snapshot),title=snapshot.scenarios[0].title;
  view.input().props.onChange({...view.input().props.value,title:title+' · 原生验证F'});view.render();
  assert.match(view.text(),/有待应用变更/);
  view.input().props.onChange({...view.input().props.value,title});view.render();view.button('检查变更').props.onClick();view.render();
  assert.match(view.text(),/没有需要提交的变更/);assert.match(view.text(),/已同步/);
  assert.ok(view.button('检查变更').props.disabled);assert.ok(view.button('撤销草稿').props.disabled);
  assert.equal(view.nodes().some(n=>n.props['aria-label']==='业务变更预览'),false);assert.deepEqual(snapshot,original);
});
test('a real title edit still proposes one current-revision Scenario and preserves omitted goal and all other inputs',async()=>{
  const{view,snapshot}=await scenarioForm(false),scenario=snapshot.scenarios[0];
  view.input().props.onChange({...view.input().props.value,title:'明确修改的名称'});view.render();view.button('检查变更').props.onClick();view.render();
  assert.ok(view.nodes().some(n=>n.props['aria-label']==='业务变更预览'));assert.match(view.text(),/1条共同命令/);
  // The actual form output goes through the production proposal builder.
  const next={...snapshot,scenarios:[{...scenario,...view.input().props.value}]},p=draft.commitProposal(snapshot,next,false);
  assert.equal(p.commands.length,1);assert.equal(p.scenario_revisions[scenario.id],scenario.revision);
  assert.deepEqual(p.commands[0].data.data,{...scenario,title:'明确修改的名称',project_revision:snapshot.project.revision});
  assert.equal(Object.hasOwn(p.commands[0].data.data,'acceptance_goal'),false);
});
