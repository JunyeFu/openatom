import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url);
const {renderProcessContract,createImageTimelineMedia}=await import(new URL('scenarios/媒体表现/runtime-root-consumer/static/root-contract.mjs',root));
const input=JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime/calendar.input.json',root),'utf8'));
const workspace={context:{host:{session_id:'static-contract-session'}},project:input.project};
const base={workspace,scope:{kind:'project'},scopeRevision:9,actions:{},onClose(){throw new Error('Static render cannot close UI');}};
test('static React: real component preserves explicit Run/Job/time and separate current Scope revision',()=>{
  const html=renderProcessContract({...base,selection:{kind:'actual_run',run_id:'old-frozen-run',job_id:'actual-job',at_ms:35000}});
  for(const text of ['设备过程工作区','old-frozen-run','actual-job','35000','授权修订 r9','unavailable','正在读取'])assert.ok(html.includes(text),text);
  assert.equal(html.includes('已计算至'),false,'static render has not received an actual Run');
});
test('static React: definition declaration explicitly names Draft Scope and exact version',()=>{
  const html=renderProcessContract({...base,scope:{kind:'definition_draft',data:'original-draft'},scopeRevision:4,selection:{kind:'definition',module_ref:{id:'frozen-device',version:'0.1.0'}}});
  for(const text of ['Draft original-draft','授权修订 r4','frozen-device@0.1.0'])assert.ok(html.includes(text),text);
  assert.equal(html.includes('实际计算边界'),false);
});
test('static React: disconnected workspace offers return and no synthetic process result',()=>{
  const html=renderProcessContract({...base,workspace:{...workspace,project:null},selection:{kind:'definition',module_ref:{id:'device',version:'1'}}});
  assert.ok(html.includes('先打开实际工程'));assert.ok(html.includes('返回工作台'));assert.equal(html.includes('原始Run'),false);
});
const image=JSON.parse(await readFile(new URL('modules/media/p302-controlled/image.module.json',root),'utf8'));
const frames=JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime/actual-frames.json',root),'utf8'));
const frame=frames.find(value=>value.label==='calendar@35000').frame;
const module_ref={id:'w5.station.CNC',version:'0.1.0'};
const declaration={binding:{session_id:'contract-session',request:{project_id:'P1',scope:{kind:'definition_draft',data:'original-draft'},scope_revision:4,source:{kind:'module_version',data:{module_ref}}}},rights:image.rights,
  readAsset:async()=>{throw new Error('Element contract must not load bytes');}};
const binding={session_id:'contract-session',request:{project_id:'P1',scope:{kind:'project'},scope_revision:9,source:{kind:'run_module_version',data:{module_ref,run_id:frame.run.id,at_ms:35000}}}};
const local=structuredClone(image.definition.payload.data),frozen=structuredClone(local);
local.tags=[];
const view={presentation:local,mode:'actual_run',view_u:.5,phase_ref:'load',canAction:()=>false,onAction(){throw new Error('No action port');},onSeek(){throw new Error('No declaration seek in actual mode');}};
const mediaElement=tree=>tree.props.children[1];
test('actual W02 adapter contract: frozen actual presentation/binding/frame wins over edited local declaration',()=>{
  const sought=[],port=createImageTimelineMedia({declaration,actual:{...declaration,binding,presentation:frozen,frame},onActualSeek:at=>sought.push(at)});
  const element=mediaElement(port.render(view));
  assert.equal(element.type.name,'MediaPresentationView');assert.equal(element.props.presentation,frozen);assert.equal(element.props.binding,binding);
  assert.equal(element.props.mode,'actual_run');assert.equal(element.props.frame,frame);assert.equal(element.props.preview,undefined);
  element.props.onSeek(40000);assert.deepEqual(sought,[40000]);
  const value=port.resolveData({source_field:'job.phase_ref',value_type:'enum',object_ref:null,port_ref:null,unit:null},'actual_run');
  assert.equal(value.status,'available');assert.equal(value.value,'unload','Job field remains distinct from held auto pose');
});
test('actual W02 adapter contract: declaration phase/u and missing actual source remain separate',()=>{
  const sought=[],port=createImageTimelineMedia({declaration,actual:null,unavailable_reason:'explicit actual history unavailable'});
  const element=mediaElement(port.render({...view,mode:'isolated_view',onSeek:u=>sought.push(u)}));
  assert.equal(element.props.presentation,local);assert.equal(element.props.binding,declaration.binding);assert.equal(element.props.mode,'declared_preview');
  assert.equal(element.props.frame,undefined);assert.deepEqual(element.props.preview,{phase_ref:'load',view_u:.5});
  element.props.onPreviewSeek({phase_ref:'load',view_u:.8});assert.deepEqual(sought,[.8]);
  assert.equal(port.resolveData({source_field:'job.state',value_type:'enum',object_ref:null,port_ref:null,unit:null},'isolated_view').status,'unavailable');
  const unavailable=port.render(view);assert.equal(unavailable.type,'p');assert.equal(unavailable.props.children,'explicit actual history unavailable');
});
