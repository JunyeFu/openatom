// Independent DQ-05 state sequences against actual prototype source.
// Only DOM/SceneView/paint/timers are stubs; source business functions and listeners execute.
// No browser, service, Figma, Host, Engine, or public-source writes.
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const assert=require('node:assert/strict');
const root=path.resolve(__dirname,'../../..');
const sourceRef='docs/02_交互设计/S04_可操作设计原型/workbench.js';
const source=fs.readFileSync(path.join(root,sourceRef),'utf8');
const stat=fs.statSync(path.join(root,sourceRef));
const inputKeys=['mechanism','width','depth','height','thickness','productRevision'];
const copy=value=>structuredClone(value);

function environment(){
  const listeners=new Map(),elements=new Map(),notes=[],viewCalls=[],downloads=[],storage=new Map();
  function element(id){
    if(!elements.has(id))elements.set(id,{id,dataset:{},style:{setProperty(){}},classList:{contains(){return false;},toggle(){}},
      textContent:'',innerHTML:'',outerHTML:'',checked:false,value:'',
      addEventListener(kind,fn){listeners.set(id+':'+kind,[fn]);},close(){this.open=false;},showModal(){this.open=true;},remove(){},
      querySelector(){return element(id+':child');},querySelectorAll(){return[];}});
    return elements.get(id);
  }
  const view=new Proxy({}, {get(_target,key){return (...args)=>{viewCalls.push({method:String(key),args:copy(args)});return key==='load'?Promise.resolve():undefined;};}});
  const document={getElementById:element,querySelector(){return null;},querySelectorAll(){return[];},body:element('body'),
    addEventListener(kind,fn){const key='document:'+kind;listeners.set(key,[...(listeners.get(key)||[]),fn]);},removeEventListener(){}};
  const context=vm.createContext({structuredClone,URLSearchParams,location:{search:'?frame=VP20'},
    window:{S04_DATA:{frames:Array.from({length:26},(_,i)=>({id:'VP'+String(i+1).padStart(2,'0'),title:'UI fixture'})),flows:[],module_categories:[]}},document,
    SceneView:function(){return view;},setInterval(){return 0;},setTimeout(){return 0;},clearTimeout(){},
    requestAnimationFrame(){return 0;},history:{replaceState(){}},console:{log(){},warn(){},error(){}},
    localStorage:{setItem(key,value){storage.set(key,value);},getItem(key){return storage.get(key)||null;}},
    fetch(){throw Error('Network/service use forbidden in this probe');}});
  const bootstrap=/^go\(S\.frame,true\);view\.load\(\).*$/m;
  assert(bootstrap.test(source),'Prototype bootstrap changed; inspect harness before using results');
  const actual=source.replace(/^import\s+.*?from\s+['"]\.\/scene-view\.js['"];\s*$/m,'')
    .replace(bootstrap,'');
  vm.runInContext(actual,context,{filename:sourceRef});
  context.__notes=notes;
  context.__downloads=downloads;
  context.__statusPaintLine=source.split(/\r?\n/).find(line=>line.includes("$('save-indicator').textContent"));
  assert(context.__statusPaintLine,'Actual save-indicator statement required for this sequence');
  vm.runInContext(`
    render=()=>{eval(__statusPaintLine);};renderWorkspace=()=>{};renderTimeline=()=>{};renderProperties=()=>{};renderTools=()=>{};
    download=(name,content,type)=>__downloads.push({name,content,type});
    notice=text=>__notes.push(text);
    globalThis.__api={state:()=>S,action,apply,edit,save,snapshot,packagingMarkup:()=>packagingWorkspace()};
  `,context);
  const api=context.__api;
  function change(id,value){
    const target={id,value,checked:!!value,dataset:{}};
    for(const fn of listeners.get('document:change')||[])fn({target});
    return target;
  }
  function input(key,value){
    api.edit('paper');
    const target={id:'',dataset:{field:key,scope:'form'},value:String(value)};
    for(const fn of listeners.get('document:input')||[])fn({target});
  }
  function foldRange(value){
    api.edit('paper');
    for(const fn of listeners.get('document:input')||[])fn({target:{id:'fold-angle',dataset:{},value:String(value)}});
  }
  async function openSnapshot(doc){
    const target={files:[{name:'OTHER_C.json',async text(){return JSON.stringify(doc);}}],value:'OTHER_C.json'};
    for(const fn of listeners.get('project-file:change')||[])await fn({target});
  }
  function snapshot(label){const s=api.state();return {label,revision:s.revision,savedRevision:s.savedRevision,
    dirty:s.revision!==s.savedRevision,save_indicator:element('save-indicator').textContent,project_title:s.project.title,
    paper:copy(s.project.paper),form:copy(s.form),result:copy(s.packResult),undo:s.undo.length,redo:s.redo.length,latest_notice:notes.at(-1)||null};}
  return {api,change,input,foldRange,openSnapshot,snapshot,notes,viewCalls,downloads,element,state:api.state};
}

function matchingResult(env){const s=env.state();assert(s.packResult);for(const key of inputKeys)assert.equal(s.packResult.input?.[key],s.project.paper[key],key+' in frozen result');}
function loadPass(env){env.change('paper-outcome','pass');env.api.action('paper-check');matchingResult(env);assert.equal(env.state().packResult.source_kind,'manual_expected');assert.equal(env.state().packResult.actual_provider,false);}
function approveBind(env){env.change('paper-ack',true);env.api.action('paper-bind');assert.equal(env.state().project.paper.ack,true);assert.equal(env.state().project.paper.bound,true);}
function ready(){const env=environment();loadPass(env);approveBind(env);return env;}
function applied(env,key,value){env.input(key,value);assert.equal(env.api.apply(),true);if(typeof value==='number')assert.equal(Number(env.state().project.paper[key]),value);else assert.equal(env.state().project.paper[key],value);}
function noActiveApproval(env){assert.equal(env.state().project.paper.ack,false,'current use must remain unacknowledged after invalidation/cancellation');assert.equal(env.state().project.paper.bound,false,'current binding must remain inactive after invalidation/cancellation');}

const cases=[],plans=[];
function check(id,meaning,fn){
  plans.push({id,meaning,fn});
}
for(const [key,value] of [['width',200],['thickness',0.8],['mechanism','tray_sleeve'],['depth',125],['height',95]]){
  check('DQ05-'+key,'same product version: '+key+' change invalidates old effective approval/binding',trace=>{
    const e=ready();trace.push(e.snapshot('old160/Pass/approved/bound'));const old=e.state().packResult;
    applied(e,key,value);trace.push(e.snapshot('semantic change applied'));assert.equal(e.state().project.paper.productRevision,'1.0.0');noActiveApproval(e);
    if(e.state().packResult)assert.notEqual(e.state().packResult.input[key],value,'retained result must remain old input, not silently re-labelled');
    e.api.action('paper-bind');trace.push(e.snapshot('stale bind attempted'));noActiveApproval(e);
    assert.equal(old.input[key],key==='mechanism'?'tuck_carton':key==='width'?160:key==='thickness'?0.5:key==='depth'?100:80,'old result remains frozen');
  });
}
for(const [key,value] of [['width',200],['thickness',0.8],['mechanism','tray_sleeve']]){
  check('DQ05-reload-'+key,'changed input -> explicit matching sample -> new purpose acknowledgment -> bind',trace=>{
    const e=ready();const old=e.state().packResult;applied(e,key,value);noActiveApproval(e);
    e.api.action('paper-check');trace.push(e.snapshot('matching manual sample loaded'));matchingResult(e);assert.notEqual(e.state().packResult,old);noActiveApproval(e);
    e.api.action('paper-bind');noActiveApproval(e);e.change('paper-ack',true);e.api.action('paper-bind');trace.push(e.snapshot('explicit acknowledgment then binding'));
    assert.equal(e.state().project.paper.ack,true);assert.equal(e.state().project.paper.bound,true);assert.equal(e.state().packResult.actual_provider,false);
  });
}
check('DQ05-new-same-input-result','reloading a new Pass sample for identical input does not borrow old result acknowledgment',trace=>{
  const e=ready(),old=e.state().packResult;e.api.action('paper-check');trace.push(e.snapshot('new sample for same input'));
  matchingResult(e);assert.notEqual(e.state().packResult,old);noActiveApproval(e);e.api.action('paper-bind');noActiveApproval(e);
});
check('DQ05-undo-after-change','undo geometry must not recreate acknowledgment/binding from draft snapshot',trace=>{
  const e=ready();applied(e,'width',200);e.api.action('undo');trace.push(e.snapshot('undo to160'));
  assert.equal(e.state().project.paper.width,160);noActiveApproval(e);e.api.action('paper-bind');noActiveApproval(e);
});
check('DQ05-undo-new-result','undo160 after approved200 must not attach200 result/ack to160; redo creates no approval',trace=>{
  const e=ready();applied(e,'width',200);e.api.action('paper-check');approveBind(e);const result200=e.state().packResult;
  e.api.action('undo');trace.push(e.snapshot('undo to160 with200 result retained'));
  assert.equal(e.state().project.paper.width,160);assert.equal(e.state().packResult,result200);noActiveApproval(e);e.api.action('paper-bind');noActiveApproval(e);
  e.api.action('redo');trace.push(e.snapshot('redo200 with no new acknowledgment'));assert.equal(e.state().project.paper.width,200);noActiveApproval(e);
  e.change('paper-ack',true);e.api.action('paper-bind');assert.equal(e.state().project.paper.bound,true);
});
check('DQ05-display-angle','fold/open/range are declared viewing changes and keep same valid result/ack/binding',trace=>{
  const e=ready(),old=e.state().packResult;e.api.action('paper-fold');e.api.action('paper-open');
  // Execute the real fold-angle range listener, not an invented data-field control.
  const before=e.snapshot('before visual range');
  e.foldRange(35);trace.push(before,e.snapshot('view angle35 displayed'));
  assert.equal(e.state().project.paper.angle,35);assert.equal(e.state().packResult,old);assert.equal(e.state().project.paper.ack,true);assert.equal(e.state().project.paper.bound,true);matchingResult(e);
});
check('DQ05-camera','camera reset/back are pure display actions and do not clear valid professional state',trace=>{
  const e=ready(),old=e.state().packResult;e.api.action('camera-reset');e.api.action('camera-back');trace.push(e.snapshot('camera actions'));
  assert(e.viewCalls.some(x=>x.method==='resetCamera'));assert(e.viewCalls.some(x=>x.method==='inspect'));
  assert.equal(e.state().packResult,old);assert.equal(e.state().project.paper.ack,true);assert.equal(e.state().project.paper.bound,true);
});
check('DQ05-undo-unrelated-draft','undo unrelated A01 layout rotation preserves current valid paper approval',trace=>{
  const e=ready(),old=e.state().packResult;const before=e.state().project.devices.find(d=>d.id==='A01').yaw;
  e.api.action('rotate');assert.notEqual(e.state().project.devices.find(d=>d.id==='A01').yaw,before);
  e.api.action('undo');trace.push(e.snapshot('undo unrelated device rotation'));assert.equal(e.state().project.devices.find(d=>d.id==='A01').yaw,before);
  assert.equal(e.state().packResult,old);assert.equal(e.state().project.paper.ack,true);assert.equal(e.state().project.paper.bound,true);
});
for(const outcome of ['fail','undetermined'])check('DQ05-'+outcome,'non-Pass sample cannot acknowledge or bind',trace=>{
  const e=ready();e.change('paper-outcome',outcome);noActiveApproval(e);e.api.action('paper-check');e.change('paper-ack',true);e.api.action('paper-bind');trace.push(e.snapshot('non-Pass acknowledgment/binding attempt'));
  assert.equal(e.state().packResult.outcome,outcome);noActiveApproval(e);assert.equal(e.state().packResult.actual_provider,false);
});
check('DQ05-revoke-current-use','unchecking current purpose acknowledgment makes effective binding inactive',trace=>{
  const e=ready(),old=e.state().packResult;e.change('paper-ack',false);trace.push(e.snapshot('purpose acknowledgment unchecked'));
  assert.equal(e.state().project.paper.ack,false);assert.equal(e.state().project.paper.bound,false,'prototype must not still display active bound after acknowledgment is unchecked');
  const markup=e.api.packagingMarkup();
  trace.push({label:'actual current prototype markup',current_warning:/class="message warning"/.test(markup),
    current_binding_success:/class="message success"/.test(markup),same_manual_result_retained:e.state().packResult===old});
  assert.match(markup,/class="message warning"/);assert.doesNotMatch(markup,/class="message success"/);
});
check('DQ05-invalid-apply','invalid pending width is not committed or treated as a newly computed sample',trace=>{
  const e=ready(),old=e.state().packResult;e.input('width','');e.api.action('paper-check');trace.push(e.snapshot('invalid calculation/sample request'));
  assert.equal(e.state().project.paper.width,160);assert.equal(e.state().packResult,old);assert.equal(e.state().form.dirty,true);
  const rev=e.state().revision;e.api.action('paper-bind');assert.equal(e.state().revision,rev,'pending invalid form cannot create a new binding');
});
check('DQ05-saved-current-use-dirty','saved prototype -> cancel current use -> dirty and unsaved-open decision; no-op does not add revision',async trace=>{
  const e=ready();e.api.save();trace.push(e.snapshot('saved current approved/bound snapshot'));
  const saved=e.state().revision;assert.equal(e.state().savedRevision,saved);assert(e.downloads.length);
  assert.equal(JSON.parse(e.downloads.at(-1).content).project.paper.bound,true);
  e.change('paper-ack',false);trace.push(e.snapshot('current use cancelled after save'));
  const changed=e.state().revision;
  const candidate=e.api.snapshot();candidate.project.title='OTHER_C';
  await e.openSnapshot(candidate);trace.push(e.snapshot('attempt open another snapshot'));
  assert(changed>saved,'persisted acknowledgment/binding change must create a draft revision');
  assert.equal(e.state().savedRevision,saved);assert.match(e.element('save-indicator').textContent,/未保存/);
  assert.notEqual(e.state().project.title,'OTHER_C','unsaved use change cannot silently replace current project');
  assert.equal(e.element('dialog').open,true);assert.match(e.element('dialog-content').innerHTML,/未保存工程/);
  assert.match(e.element('dialog-content').innerHTML,/open-save/);assert.match(e.element('dialog-content').innerHTML,/open-discard/);assert.match(e.element('dialog-content').innerHTML,/open-stay/);
  e.change('paper-ack',false);assert.equal(e.state().revision,changed,'no-op checkbox event must not manufacture a revision');
});
check('DQ05-clean-form-preview-after-ack-off','actual clean form -> cancel current use -> viewing slider + Apply cannot revive cancelled use',trace=>{
  const e=ready();e.api.edit('paper');trace.push(e.snapshot('clean form contains old active flags'));
  assert.equal(e.state().form.data.ack,true);assert.equal(e.state().form.data.bound,true);
  e.change('paper-ack',false);e.foldRange(35);trace.push(e.snapshot('actual preview range after use cancelled'));
  e.api.apply();trace.push(e.snapshot('Apply after actual viewing-only range'));noActiveApproval(e);
});
check('DQ05-clean-form-same-width-after-ack-off','clean stale form -> cancel use -> view angle + same numeric width input -> Apply must not resurrect flags',trace=>{
  const e=ready();e.api.edit('paper');e.change('paper-ack',false);noActiveApproval(e);
  e.foldRange(35);e.input('width',160);trace.push(e.snapshot('only angle changed, professional width re-entered unchanged'));
  assert.equal(Number(e.state().form.data.width),Number(e.state().project.paper.width));
  assert.equal(e.api.apply(),true);trace.push(e.snapshot('applied unchanged professional input with preview angle'));
  assert.equal(e.state().project.paper.angle,35);assert.equal(Number(e.state().project.paper.width),160);noActiveApproval(e);
});

async function main(){
for(const {id,meaning,fn} of plans){
  const trace=[];
  try{await fn(trace);cases.push({id,meaning,passed:true,trace});}
  catch(error){cases.push({id,meaning,passed:false,error:error.message,trace});}
}
const sourceLines=source.split(/\r?\n/);
const sourceEvidence=[];
for(let i=0;i<sourceLines.length;i++)if(/function (paperInput|paperMatches|invalidatePaper|commit|restoreDraft)|case 'paper-(check|bind|change)'|el\.id==='paper-ack'/.test(sourceLines[i]))sourceEvidence.push({line:i+1,text:sourceLines[i]});
const result={document_kind:'DQ05_window3_independent_actual_source_function_state_sequences',
  source_ref:sourceRef,source_observation:{mtime_ms:stat.mtimeMs,bytes:stat.size,lines:sourceLines.length},
  actual_source_functions_executed:true,source_evidence:sourceEvidence,
  harness:{only_import_and_browser_bootstrap_omitted:true,UI_DOM_SceneView_timers_stubbed:true,renderer_functions_stubbed:true,
    professional_functions_or_action_handlers_replaced:false,actual_save_indicator_statement_executed:true,
    localStorage_and_download_terminal_stubbed:true,service_started:false,Figma_used:false,owner_tests_or_PASS_read:false},
  oracle:{source:'independently stated lifecycle obligations, not prototype own paperMatches or owner test expected values',
    input_keys:inputKeys,independent_result_snapshot_assertions:true},
  harness_corrections:['Numeric geometric comparison uses numeric meaning, not UI storage string/number; full canonical DTO typing is outside this prototype probe.',
    'Angle uses the actual fold-angle range listener; no nonexistent angle data-field control.',
    'Undo of a nonprofessional edit uses the actual A01 rotate action; preview angle itself need not create undo history.'],
  case_count:cases.length,passed:cases.filter(x=>x.passed).length,failed:cases.filter(x=>!x.passed).length,cases,
  status:cases.every(x=>x.passed)?'all_independent_sequences_passed':'blocker_independent_sequence_failed',
  actual_browser:false,actual_product_Host_or_Engine:false,actual_professional_solver:false,actual_human_or_industrial_acceptance:false};
const out=path.join(__dirname,'dq05-result.json');fs.writeFileSync(out,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({status:result.status,cases:result.case_count,passed:result.passed,failed:result.failed,
  failures:cases.filter(x=>!x.passed).map(({id,error})=>({id,error})),source_observation:result.source_observation,output:path.relative(root,out)}));
if(result.failed)process.exitCode=1;
}
main().catch(error=>{console.error(error.stack);process.exitCode=2;});
