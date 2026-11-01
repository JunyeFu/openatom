// Independent execution of actual S04 source state transitions; no source mutation.
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const finalDQ05 = process.argv.includes('--final-dq05');
const root = path.resolve(__dirname, '../../..');
const proto = 'docs/02_交互设计/S04_可操作设计原型/';
const source = fs.readFileSync(path.join(root, proto, 'workbench.js'), 'utf8');
const sourceMtime = fs.statSync(path.join(root, proto, 'workbench.js')).mtime.toISOString();
const elements = new Map(), listeners = {}, dialogs = [], notices = [], downloads = [];
const storage = new Map();
let failSave = false;
function el(id) {
  if (!elements.has(id)) elements.set(id, {
    id, value: '', checked: false, style: {setProperty(){}},
    classList: {toggle(){}, add(){}, remove(){}},
    addEventListener(type, cb) { listeners[id + ':' + type] = cb; },
    close(){}, showModal(){}, click(){}, setAttribute(){}, remove(){},
    querySelector(){return {outerHTML:''};},
    getBoundingClientRect(){return {width:1000,height:600};}
  });
  return elements.get(id);
}
const box = {
  window: {}, location:{search:'?frame=VP03'}, URLSearchParams, structuredClone,
  console, setTimeout(){return 0;}, clearTimeout(){}, setInterval(){return 0;},
  localStorage:{setItem(k,v){if(failSave)throw Error('Independent storage-denied probe');storage.set(k,v);},getItem(k){return storage.get(k);}},
  history:{replaceState(){}},
  document:{getElementById:el, querySelector(){return null;}, addEventListener(type,cb){listeners['document:'+type]=cb;},
    body:{style:{setProperty(){}},classList:{toggle(){}}}},
  SceneView: class { constructor(){return new Proxy(this,{get:(o,k)=>k in o?o[k]:(()=>{})});} },
};
const context = vm.createContext(box);
vm.runInContext(fs.readFileSync(path.join(root, proto, 'prototype.data.js'),'utf8'), context);
const start = source.indexOf('const D=window.');
const stop = source.indexOf("document.addEventListener('click'");
if(start < 0 || stop < start) throw Error('Source extraction entry missing');
vm.runInContext(source.slice(start, stop), context);
context.captureDialog=(title,html)=>dialogs.push({title,html});
context.captureNotice=text=>notices.push(text);
context.captureDownload=(name,text)=>downloads.push({name,text});
vm.runInContext(`
render=()=>{};renderTools=()=>{};renderTimeline=()=>{};renderWorkspace=()=>{};
showDialog=(title,html)=>captureDialog(title,html);
notice=text=>captureNotice(text);
download=(name,text)=>captureDownload(name,text);
`, context);
const S = vm.runInContext('S',context);
const run = text=>vm.runInContext(text,context);
const clone = structuredClone;
function reset() {
  run("S.project=seed();S.revision=2;S.savedRevision=2;S.selected='A01';S.frame='VP03';S.tool='select';S.undo=[];S.redo=[];S.form=null;S.run=null;S.compute=null;S.packResult=null;S.sampleRuns=[];S.play=false;S.pendingOpen=null;S.pendingSelection=null;bindSelectedInstance();");
  dialogs.length=0;notices.length=0;downloads.length=0;storage.clear();failSave=false;
}
function summary() {
  return {project:S.project.title,selected:S.selected,revision:S.revision,savedRevision:S.savedRevision,
    undo:S.undo.length,redo:S.redo.length,form:clone(S.form),
    run:clone(S.run),viewTime:S.viewTime,packResult:clone(S.packResult),
    paper:clone(S.project.paper)};
}
function dialogActions() {
  return [...(dialogs.at(-1)?.html||'').matchAll(/data-action=["']([^"']+)["']/g)].map(x=>x[1]);
}
const cases=[];
async function test(id,meaning,fn) {
  if(finalDQ05&&!id.startsWith('X5-final-'))return;
  reset();
  try { const evidence=await fn(); cases.push({id,meaning,matched:evidence.ok===true,...evidence}); }
  catch(error) {cases.push({id,meaning,matched:false,error:String(error),state:summary()});}
}
function setPose(x=8.6,base=S.revision) {
  S.form={kind:'pose',base,dirty:true,data:{...clone(S.project.devices[0]),x}};
}
function resultBindings() {
  return {ack:!!S.project.paper.ack,bound:!!S.project.paper.bound,
    result:clone(S.packResult),paper:clone(S.project.paper)};
}
function loadAndBind() {
  run("action('paper-check')");
  S.project.paper.ack=true;
  run("action('paper-bind')");
}
function paperEdit(change) {
  S.form={kind:'paper',base:S.revision,dirty:true,data:{...clone(S.project.paper),...change}};
  return run('apply()');
}
const openMarker="$('project-file').addEventListener";
const openStart=source.indexOf(openMarker);
const openEnd=source.indexOf("$('media-file').addEventListener",openStart);
if(openStart<0||openEnd<openStart)throw Error('Open listener missing');
vm.runInContext(source.slice(openStart,openEnd), context);
const changeStart=source.indexOf("document.addEventListener('change'");
if(changeStart<0)throw Error('Change listener missing');
const inputStart=source.indexOf("document.addEventListener('input'");
if(inputStart<0||inputStart>changeStart)throw Error('Input listener missing');
vm.runInContext(source.slice(inputStart,changeStart),context);
vm.runInContext(source.slice(changeStart).split(/\r?\n/)[0],context);
const definitionStart=source.indexOf('// Native definition action remains local');
const definitionEnd=source.indexOf('setInterval(',definitionStart);
if(definitionStart<0||definitionEnd<definitionStart)throw Error('Definition listener missing');
vm.runInContext(source.slice(definitionStart,definitionEnd),context);
async function requestOpen(dataOrText) {
  const text=typeof dataOrText==='string'?dataOrText:JSON.stringify(dataOrText);
  await listeners['project-file:change']({target:{files:[{name:'C.json',text:async()=>text}]}});
}
function incoming() {
  const snap=run('snapshot()');
  snap.project.title='C-imported-project';snap.revision=10;
  return snap;
}
async function main(){
  await test('X3-menu','Dirty object switch offers apply/discard/stay',()=>{
    setPose();run("selectDevice('ROB01')");
    const actions=dialogActions();
    return {ok:actions.some(a=>/selection.*apply|apply.*selection/.test(a))
      &&actions.includes('selection-discard')&&actions.includes('dialog-close'),
      actions,selected:S.selected,pending:S.pendingSelection};
  });
  await test('X3-apply','Apply changes old object before selecting next',()=>{
    setPose();run("selectDevice('ROB01')");
    const a=dialogActions().find(a=>/selection.*apply|apply.*selection/.test(a));
    if(a)run('action('+JSON.stringify(a)+')');
    return {ok:S.selected==='ROB01'&&S.project.devices[0].x===8.6&&!S.form,
      actual:summary(),old_x:S.project.devices[0].x};
  });
  await test('X3-conflict','Apply conflict keeps old selection and all fields',()=>{
    setPose(8.6,1);run("selectDevice('ROB01')");
    const a=dialogActions().find(a=>/selection.*apply|apply.*selection/.test(a));
    if(a)run('action('+JSON.stringify(a)+')');
    return {ok:!!a&&S.selected==='A01'&&S.form?.data.x===8.6&&S.project.devices[0].x===8,
      actual:summary(),actions:dialogActions()};
  });
  await test('X3-missing','Invalid required pose cannot leave or lose fields',()=>{
    setPose('');run("selectDevice('ROB01')");
    const a=dialogActions().find(a=>/selection.*apply|apply.*selection/.test(a));
    if(a)run('action('+JSON.stringify(a)+')');
    return {ok:!!a&&S.selected==='A01'&&S.form?.data.x===''&&S.project.devices[0].x===8,actual:summary()};
  });
  await test('X3-discard','Discard switches without applying old fields',()=>{
    setPose();run("selectDevice('ROB01')");run("action('selection-discard')");
    return {ok:S.selected==='ROB01'&&S.project.devices[0].x===8&&!S.form,actual:summary()};
  });
  await test('X3-stay','Stay leaves selection/form/current run unchanged',()=>{
    setPose();S.run={id:'OLD-RUN',computedUntil:170,source_kind:'manual_expected'};
    run("selectDevice('ROB01')");run("action('dialog-close')");
    return {ok:S.selected==='A01'&&S.form?.data.x===8.6&&S.run?.id==='OLD-RUN',actual:summary()};
  });
  await test('X4-dirty-options','Opening replacement asks save/discard/stay',async()=>{
    setPose();run('apply()');await requestOpen(incoming());
    return {ok:S.project.title!=='C-imported-project'&&dialogActions().some(a=>/save/.test(a))
      &&dialogActions().some(a=>/discard/.test(a))&&dialogActions().includes('open-stay'),
      actions:dialogActions(),actual:summary()};
  });
  await test('X4-stay','Open stay preserves project, history and unapplied fields',async()=>{
    setPose();S.undo=[clone(S.project)];const before=summary();
    await requestOpen(incoming());run("action('open-stay')");
    return {ok:JSON.stringify(summary())===JSON.stringify(before)&&S.pendingOpen===null,actual:summary()};
  });
  await test('X4-save','Save current then open candidate; snapshot belongs to old project',async()=>{
    setPose();run('apply()');const oldTitle=S.project.title,oldRevision=S.revision;
    await requestOpen(incoming());run("action('open-save')");
    const saved=JSON.parse(storage.get('superfactory.S04.prototype.snapshot'));
    return {ok:saved.project.title===oldTitle&&saved.revision===oldRevision&&saved.project.devices[0].x===8.6
      &&S.project.title==='C-imported-project'&&S.undo.length===0&&S.redo.length===0,
      snapshot_title:saved.project.title,snapshot_revision:saved.revision,actual:summary()};
  });
  await test('X4-save-denied','Failed old-project snapshot save cannot replace active project',async()=>{
    setPose();run('apply()');await requestOpen(incoming());const before=summary();failSave=true;
    let error=null;try{run("action('open-save')");}catch(e){error=String(e);}
    return {ok:!!error&&JSON.stringify(summary())===JSON.stringify(before),error,actual:summary()};
  });
  await test('X4-discard','Discard current applied changes then open clears old sample/view/async state',async()=>{
    setPose();run('apply()');S.run={id:'A-RUN',computedUntil:300};S.compute={state:'running'};
    S.agent.preview={id:'old-preview'};S.packResult={outcome:'pass'};S.play=true;S.viewTime=120;
    await requestOpen(incoming());run("action('open-discard')");
    return {ok:S.project.title==='C-imported-project'&&!S.run&&!S.compute&&!S.packResult&&!S.play
      &&!S.agent.preview&&S.viewTime===0&&S.undo.length===0&&S.redo.length===0,actual:summary()};
  });
  await test('X4-apply-conflict','Unapplied-form conflict cannot open new project or lose fields',async()=>{
    setPose(8.6,1);await requestOpen(incoming());run("action('open-apply')");
    return {ok:S.project.title!=='C-imported-project'&&S.form?.data.x===8.6&&S.form.base===1,actual:summary()};
  });
  await test('X4-two-decisions','Discarding fields still asks about previously applied unsaved project',async()=>{
    setPose();run('apply()');setPose(9);await requestOpen(incoming());run("action('open-discard-fields')");
    const actions=dialogActions();return {ok:S.project.title!=='C-imported-project'&&!S.form
      &&S.project.devices[0].x===8.6&&actions.includes('open-save')&&actions.includes('open-discard'),
      actual:summary(),actions};
  });
  await test('X4-clean-history','Clean open resets scope history and old undo cannot replace project',async()=>{
    S.undo=[{...clone(S.project),title:'A-previous-project'}];S.redo=[clone(S.project)];
    await requestOpen(incoming());run("action('undo')");run("action('redo')");
    return {ok:S.project.title==='C-imported-project'&&S.undo.length===0&&S.redo.length===0,actual:summary()};
  });
  await test('X4-failed-open','Malformed input retains active project, dirty form, run and both histories',async()=>{
    S.undo=[clone(S.project)];S.redo=[clone(S.project)];setPose();
    S.run={id:'A-OLD-RUN',computedUntil:180};const before=summary();
    await requestOpen('{invalid');
    return {ok:JSON.stringify(summary())===JSON.stringify(before),actual:summary()};
  });
  await test('X4-failed-shape','Rejected snapshot cannot partially replace project',async()=>{
    setPose();const before=summary();await requestOpen({document_kind:'S04_interaction_prototype_project_state',project:{devices:[]}});
    return {ok:JSON.stringify(summary())===JSON.stringify(before),actual:summary()};
  });
  await test('X5-width','Same product version width edit invalidates old effective result/binding',()=>{
    loadAndBind();const before=resultBindings(),applied=paperEdit({width:200}),after=resultBindings();
    run("action('paper-bind')");
    return {ok:before.bound&&applied&&!S.project.paper.bound&&!(after.ack&&after.result?.outcome==='pass'),
      before,after};
  });
  await test('X5-thickness','Same product version thickness edit invalidates effective result/binding',()=>{
    loadAndBind();const before=resultBindings();paperEdit({thickness:.8});
    return {ok:before.bound&&!S.project.paper.bound&&!S.project.paper.ack,before,after:resultBindings()};
  });
  await test('X5-mechanism','Same product version mechanism edit invalidates effective result/binding',()=>{
    loadAndBind();const before=resultBindings();paperEdit({mechanism:'fold_glue'});
    return {ok:before.bound&&!S.project.paper.bound&&!S.project.paper.ack,before,after:resultBindings()};
  });
  await test('X5-preview','Only fold preview angle keeps professional input identity',()=>{
    loadAndBind();const before=resultBindings();run("action('paper-fold')");run("action('paper-open')");
    return {ok:before.bound&&S.project.paper.bound&&S.project.paper.ack,before,after:resultBindings()};
  });
  await test('X5-ack-not-resurrected','Undo after changed geometry cannot silently resurrect old acknowledgment',()=>{
    loadAndBind();paperEdit({width:200});run("action('undo')");
    return {ok:!S.project.paper.bound&&!S.project.paper.ack,actual:resultBindings(),
      note:'Local prototype effective state only; no real acknowledgment ledger is executed.'};
  });
  await test('X5-rebind','Explicit new matching sample and acknowledgment allow rebind',()=>{
    loadAndBind();paperEdit({width:200});loadAndBind();
    return {ok:S.project.paper.bound&&S.project.paper.ack,actual:resultBindings()};
  });
  await test('X6-one-clock','Axis/LOD/camera presentation cannot change frozen run times, input quantities or results',()=>{
    run("loadSchedule()");S.viewTime=120;const beforeRun=clone(S.run),beforeProject=clone(S.project),beforeTime=S.viewTime;
    run("action('axis-line')");run("action('axis-device')");run("action('axis-back')");run("action('lod-region')");
    return {ok:JSON.stringify(S.run)===JSON.stringify(beforeRun)&&JSON.stringify(S.project)===JSON.stringify(beforeProject)
      &&S.viewTime===beforeTime,run_id:S.run.id,view_time:S.viewTime,input_quantity:S.project.order.quantity};
  });
  await test('X5-camera','LOD/camera display change does not revoke valid prototype professional binding',()=>{
    loadAndBind();run("action('lod-region')");return {ok:S.project.paper.ack&&S.project.paper.bound,actual:resultBindings()};
  });
  await test('X4-old-unit','Opening project with no Run cannot keep old Run item selection',async()=>{
    S.selectedUnit='005';await requestOpen(incoming());
    return {ok:S.project.title==='C-imported-project'&&S.selectedUnit!=='005',selected_unit:S.selectedUnit,
      project:S.project.title,run:S.run};
  });
  await test('X4-stale-definition','Definition callback after project replacement cannot commit old draft into new scope',async()=>{
    S.definitionDraft={id:'A_ONLY_DEFINITION',version:'1.0.0'};S.category=1;
    await requestOpen(incoming());const before=clone(S.project.definitions);
    listeners['document:click']({target:{closest:()=>true}});
    return {ok:JSON.stringify(S.project.definitions)===JSON.stringify(before),
      definitions:S.project.definitions,pending_definition:clone(S.definitionDraft),project:S.project.title};
  });
  await test('X5-purpose-uncheck','Unchecking current purpose acknowledgment makes effective binding inactive',()=>{
    loadAndBind();listeners['document:change']({target:{id:'paper-ack',checked:false}});
    return {ok:!S.project.paper.ack&&!S.project.paper.bound,actual:resultBindings()};
  });
  await test('X5-final-clean-preview','Cancel purpose with an existing clean form; preview+Apply cannot revive its stale flags',()=>{
    loadAndBind();run("edit('paper')");const stale=clone(S.form.data);
    listeners['document:change']({target:{id:'paper-ack',checked:false}});
    listeners['document:input']({target:{id:'fold-angle',dataset:{},value:'25'}});
    const applied=run('apply()');
    return {ok:stale.ack&&stale.bound&&applied&&!S.project.paper.ack&&!S.project.paper.bound
      &&run('paperMatches()'),stale_form_flags:{ack:stale.ack,bound:stale.bound},
      applied,actual:resultBindings()};
  });
  await test('X5-final-clean-same-value','Cancel purpose, preview and same-width input; successful Apply preserves current inactive binding',()=>{
    loadAndBind();run("edit('paper')");const width=S.project.paper.width,stale=clone(S.form.data);
    listeners['document:change']({target:{id:'paper-ack',checked:false}});
    listeners['document:input']({target:{id:'fold-angle',dataset:{},value:'25'}});
    listeners['document:input']({target:{id:'width-input',dataset:{field:'width',scope:'form'},value:String(width)}});
    const before=S.revision,applied=run('apply()');
    return {ok:stale.ack&&stale.bound&&applied&&S.form===null&&S.revision>before
      &&S.project.paper.width===width&&run('paperMatches()')&&!S.project.paper.ack&&!S.project.paper.bound,
      same_width:width,stale_form_flags:{ack:stale.ack,bound:stale.bound},
      applied,committed_revision:S.revision,actual:resultBindings()};
  });
  await test('X5-final-explicit-reack','After cancellation and same-value Apply, only explicit current acknowledgment permits rebind',()=>{
    loadAndBind();run("edit('paper')");
    listeners['document:change']({target:{id:'paper-ack',checked:false}});
    listeners['document:input']({target:{id:'width-input',dataset:{field:'width'},value:String(S.project.paper.width)}});
    run('apply()');const inactive=!S.project.paper.ack&&!S.project.paper.bound;
    run("action('paper-check')");
    listeners['document:change']({target:{id:'paper-ack',checked:true}});
    run("action('paper-bind')");
    return {ok:inactive&&S.project.paper.ack&&S.project.paper.bound&&run('paperMatches()'),
      inactive_before_explicit_ack:inactive,actual:resultBindings()};
  });
  const output={document_kind:'window6_actual_prototype_source_independent_state_sequences',
    source_ref:proto+'workbench.js',source_modified_utc:sourceMtime,
    source_changed_during_execution:fs.readFileSync(path.join(root,proto,'workbench.js'),'utf8')!==source,
    extracted_range:{start_line:source.slice(0,start).split('\n').length,end_line:source.slice(0,stop).split('\n').length-1},
    cases,failed_ids:cases.filter(c=>!c.matched).map(c=>c.id),
    execution:'Actual workbench functions in Node VM; DOM/render/SceneView/download presentation are stubs.',
    actual_browser_execution:false,actual_Host_or_Engine:null,
    execution_selection:finalDQ05?'final_DQ05_combination_delta_only':'all_defined_sequences',
    limits:'Does not assert actual professional ledger, real persistence, rendering or product run.'};
  const filename=finalDQ05?'s04-final-dq05-result.json':'s04-independent-result.json';
  fs.writeFileSync(path.join(__dirname,filename),JSON.stringify(output,null,2)+'\n','utf8');
  console.log(JSON.stringify({cases:cases.length,failed:output.failed_ids,output:'tools/design-quality-audit/window6/'+filename}));
}
main().catch(e=>{console.error(e);process.exitCode=1;});

