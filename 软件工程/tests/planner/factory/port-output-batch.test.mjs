import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {stripTypeScriptTypes} from 'node:module';
import vm from 'node:vm';

const source=readFileSync(new URL('../../../apps/planner/src/services/runPortOutput.ts',import.meta.url),'utf8');
const code=stripTypeScriptTypes(source.replace(/^import .*;\r?\n/gm,'').replace(/export /g,''))+'\nglobalThis.read=readRunPortOutput;';
const tick=()=>new Promise(resolve=>setImmediate(resolve));
class HostFailure extends Error {constructor(detail){super(detail.message);this.detail=detail;}}
function consumer(){
  const calls=[];
  const context={Promise,queueMicrotask,HostFailure,invoke:(command,args)=>new Promise((resolve,reject)=>calls.push({command,args,resolve,reject}))};
  vm.runInNewContext(code,context);
  return {read:context.read,calls};
}

test('60 concurrent actual port reads use one transport without losing order or per-request errors',async()=>{
  const c=consumer();
  const requests=Array.from({length:60},(_,i)=>({project_id:'P1',scope:{kind:'project'},scope_revision:7,run_id:'R1',at_ms:600000,port:{instance_id:'D'+i,port_id:'output',module_ref:{id:'station',version:'1.0.5'}}}));
  const reads=requests.map(request=>c.read(request).then(value=>({value}),error=>({error})));
  await tick();assert.equal(c.calls.length,1);assert.equal(c.calls[0].command,'project_read_run_port_outputs');
  assert.deepEqual(Array.from(c.calls[0].args.requests),requests);
  const denied={code:'UnsupportedCapability',message:'Exact port unavailable'};
  c.calls[0].resolve(requests.map((request,i)=>i===17?{Err:denied}:{Ok:{...request,completed_output:{value:String(i),unit:'piece'}}}));
  const results=await Promise.all(reads);
  assert.equal(results[17].error.detail,denied);
  for(let i=0;i<results.length;i++)if(i!==17)assert.equal(results[i].value.port,requests[i].port);
});

test('a later timepoint has independent ownership while an earlier batch is still pending',async()=>{
  const c=consumer();const old=c.read({run_id:'R1',at_ms:600000});await tick();
  const next=c.read({run_id:'R2',at_ms:0});await tick();assert.equal(c.calls.length,2);
  c.calls[1].resolve([{Ok:{run_id:'R2',at_ms:0}}]);assert.equal((await next).run_id,'R2');
  c.calls[0].resolve([{Ok:{run_id:'R1',at_ms:600000}}]);assert.equal((await old).run_id,'R1');
});

test('transport failure rejects every queued read and is not replaced by zero counters',async()=>{
  const c=consumer(), failure=new Error('Actual IPC disconnected');
  const a=c.read({run_id:'R1'}).catch(error=>error), b=c.read({run_id:'R2'}).catch(error=>error);
  await tick();c.calls[0].reject(failure);assert.equal(await a,failure);assert.equal(await b,failure);
});
