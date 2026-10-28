import {test} from 'node:test';import assert from 'node:assert/strict';import {readFile,writeFile} from 'node:fs/promises';import {stripTypeScriptTypes} from 'node:module';import {root,out} from './load-source.mjs';
const component=await readFile(new URL('apps/planner/src/features/factory/DeviceProcessWorkspace.tsx',root),'utf8');
function sourceFunction(text,start,end){const a=text.indexOf(start),b=text.indexOf(end,a);assert.ok(a>=0&&b>a);return text.slice(a,b);}
const navigation=sourceFunction(component,'  function navigate(','  function selectNode(');await writeFile(new URL('reviewed-navigate-function.ts',out),navigation);
function navigate(dirty){const state={inline:{input:'retained'},selection:{node:1},leave:null,generation:{current:10},currentNode:{current:{node:1}}},deps={nodeDirty:dirty,nodeGeneration:state.generation,currentNode:state.currentNode,setLeaveNode:value=>{state.leave=value;},setInlineEdit:value=>{state.inline=value;},setNodeSelection:value=>{state.selection=value;}};
  const code=stripTypeScriptTypes(navigation,{mode:'transform'}),names=Object.keys(deps),action=new Function(...names,`${code};return navigate;`)(...names.map(n=>deps[n]));return {state,action};}
test('actual consumer navigation holds dirty inline inputs and defers all requested navigation until explicit decision',()=>{
  const {state,action}=navigate(true);let ran=false;const original=state.inline;action(()=>{ran=true;});assert.equal(ran,false);assert.equal(state.inline,original);assert.deepEqual(state.selection,{node:1});assert.equal(typeof state.leave.run,'function');
  // The actual JSX keep action only dismisses the question; it does not run it.
  assert.match(component,/保留节点输入/);assert.match(component,/放弃节点输入并继续/);assert.match(component,/leaveNode\.run/);
});
test('actual clean navigation clears old inspection and invalidates old async source generation before callback',()=>{
  const {state,action}=navigate(false);let observed;action(()=>{observed={inline:state.inline,selection:state.selection,generation:state.generation.current};});assert.deepEqual(observed,{inline:null,selection:null,generation:11});
});
test('Root normal menus cannot unmount dirty process and automatic project refresh keeps its old component mounted with current Host context',async()=>{
  const text=await readFile(new URL('apps/planner/src/features/factory/Workbench.tsx',root),'utf8');
  assert.match(text,/const editing =[^;]*deviceProcess !== null/);assert.match(text,/if \(!nodeDirtyRef\.current\) setDeviceProcess\(null\)/);assert.match(text,/onNodeDirtyChange=\{receiveNodeDirty\}/);assert.match(text,/authoringIO \?\? \(nodeDirty \? nodeRetainedInput\.current\?\.io/);
  assert.match(text,/deviceProcess && snapshot && processAuthoringIO && processProjectRevision !== undefined/);assert.match(text,/DeviceProcessWorkspace key=\{deviceProcess\.id\} workspace=\{snapshot\}/);
  // These are source integration assertions, not a React/Native execution claim.
  await writeFile(new URL('dirty-root-source-review.json',out),JSON.stringify({normal_menu_process_guard:true,changed_project_does_not_clear_dirty_component:true,retained_io_but_actual_workspace:true,source_only:true,Native:false},null,2));
});
