import {createServer} from 'vite';import fs from 'node:fs/promises';import assert from 'node:assert/strict';
const root=process.cwd(),read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const server=await createServer({root,configFile:false,server:{middlewareMode:true,watch:null},appType:'custom',optimizeDeps:{noDiscovery:true}});
try{
 const {factoryItemPoint}=await server.ssrLoadModule('/apps/planner/src/scene/factoryMedia.ts');
 const proof=await read('.work/native500-quality-20261025/ordinary-10-proof.json'),run=await read('.work/native500-quality-20261025/attempt-09/PAPER-QA.run.json');
 const rows=proof.samples.filter(s=>s.device==='pile-turner'&&s.condition==='interrupted').slice(0,2);
 const operation=(await read('modules/media/native-equipment-500/reviews/B02-Q01/devices/pile-turner/module.json')).data.definition.payload.data.behavior.inline;
 const item={attributes:{presentation_state:{kind:'enum',value:'rolled'},closure:{kind:'enum',value:'closed'}}};
 const presentation={timeline:[{phase_ref:'rolled'},{phase_ref:'rolled:auto'},{phase_ref:'unwound'},{phase_ref:'closed'}]};
 const points=rows.map(s=>factoryItemPoint({item,holder_frame:{run,snapshot:{run_id:run.id,at_ms:s.at_ms,objects:s.job,next_index:null},job_id:s.job[0].data.id,operation}},presentation));
 assert.equal(points[0].display_phase_id,'rolled:auto');assert.equal(points[0].progress,.4);assert.equal(points[1].progress,.4);
 assert.equal(factoryItemPoint({item,holder_frame:null},presentation).display_phase_id,'rolled');
 assert.equal(factoryItemPoint({item:{attributes:{closure:{kind:'enum',value:'closed'}}},holder_frame:null},presentation).display_phase_id,'closed');
 assert.throws(()=>factoryItemPoint({item:{attributes:{presentation_state:{kind:'number',value:1}}}},presentation),/presentation_state/);
 assert.equal(factoryItemPoint({item},{timeline:[]}),null);
 const out='.work/native500-quality-20261025/q02';await fs.mkdir(out,{recursive:true});await fs.writeFile(out+'/state-binding-check.json',JSON.stringify({passed:true,checks:6,actual_clock_source:run.id,paused_progress:points.map(p=>p.progress),location_has_no_job_clock:true,legacy_carton_closure_preserved:true},null,2)+'\n');
 console.log('Six item-state checks PASS: actual paused Job, no Location clock, finite form and existing carton closure.');
}finally{await server.close();}
