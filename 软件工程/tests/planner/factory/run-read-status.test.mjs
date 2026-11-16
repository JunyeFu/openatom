import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {stripTypeScriptTypes} from 'node:module';
import vm from 'node:vm';

const source=readFileSync(new URL('../../../apps/planner/src/features/factory/Workbench.tsx',import.meta.url),'utf8');
const choosing=source.slice(source.indexOf('  async function chooseRun('),source.indexOf('  async function chooseComparison('));
const tick=()=>new Promise(resolve=>setImmediate(resolve));

function consumer(cached=false,deferred=false){
  const project={id:'P',revision:7,halls:[{id:'H'}]};
  const run={id:'R',computed_until_ms:5000,computation_status:'completed'};
  const frozen={project:{...project},modules:new Map(),scenario:{id:'S',revision:2}};
  const writes=[],requests=[];
  const env={
    workspaceRef:{current:{project}},projectIdentity:{current:'P'},
    observation:{current:0},comparisonEpoch:{current:0},pendingRun:{current:null},
    primaryRun:{current:cached?'R':null},run:cached?run:null,runProject:cached?project:null,
    runInputs:cached?frozen:null,atMs:cached?5000:null,comparison:null,
    message:cached?'正在读取所选运行记录及当次输入…':'',
    host:{
      getRun(_project,id){
        const result={kind:'full',data:{...run,id}};
        return deferred?new Promise(resolve=>requests.push(()=>resolve(result))):Promise.resolve(result);
      },
      async readRunInputs(){return frozen;},
    },
    async readFactoryObservation(value){return {run:value,project:frozen.project,observation:{run_id:value.id,at_ms:5000}};},
    setMessage(value){env.message=value;writes.push(['Message',value]);},
    switchSource(value){writes.push(['Source',value]);},
    rememberRunSelection(projectId,id){writes.push(['Remember',projectId,id]);},
  };
  env.ownsObservation=(epoch,projectId,id)=>epoch===env.observation.current&&env.projectIdentity.current===projectId&&env.pendingRun.current===id;
  for(const name of ['FactorySource','Comparison','Run','RunProject','RunInputs','AtMs','ObservationPage','TimelineSelection','Source','HallId'])env['set'+name]=value=>writes.push([name,value]);
  vm.runInNewContext(stripTypeScriptTypes(choosing)+'\nglobalThis.choose=chooseRun;',env);
  return {env,writes,requests,run};
}

test('a completed historical read clears its progress and retains the accepted Run',async()=>{
  const t=consumer(false,true),pending=t.env.choose('R');
  assert.match(t.env.message,/正在读取/);
  t.requests[0]();await pending;
  assert.equal(t.env.message,'');
  assert.equal(t.writes.find(([name])=>name==='Run')[1].id,'R');
  assert.equal(t.writes.find(([name])=>name==='RunProject')[1].revision,7);
});

test('returning to an already loaded historical Run clears a prior progress message',async()=>{
  const t=consumer(true);
  assert.equal(await t.env.choose('R'),t.run);
  assert.equal(t.env.message,'');
  assert.equal(t.writes.some(([name])=>name==='Run'),false);
});

test('an older read finishing cannot clear the newer historical read progress',async()=>{
  const t=consumer(false,true),old=t.env.choose('R'),latest=t.env.choose('NEW');
  t.requests[0]();assert.equal(await old,null);
  assert.match(t.env.message,/正在读取/);
  assert.equal(t.writes.some(([name])=>name==='Run'),false);
  t.requests[1]();await latest;
  assert.equal(t.env.message,'');
  assert.equal(t.writes.find(([name])=>name==='Run')[1].id,'NEW');
});
