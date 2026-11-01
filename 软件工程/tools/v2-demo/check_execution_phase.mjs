// Explicit phase-report fixtures exercising the real projection, not a simulated Host.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {stripTypeScriptTypes} from 'node:module';import {pathToFileURL} from 'node:url';
const [output]=process.argv.slice(2);await fs.mkdir(output,{recursive:false});
await fs.writeFile(path.join(output,'executionPhase.mjs'),stripTypeScriptTypes(await fs.readFile('apps/planner/src/services/executionPhase.ts','utf8'),{mode:'transform'}));
const {executionDevicePhases}=await import(pathToFileURL(path.resolve(output,'executionPhase.mjs')).href);
const base={at_host_ms:2000,objects:[{kind:'job',data:{job_id:'job',logical_job_id:'logical',execution_ref:{kind:'project_device',data:{device_id:'device'}},phase_id:'cycle',state:'working',started_at_host_ms:1000,finished_at_host_ms:null}},{kind:'attempt',data:{attempt:{logical_job_id:'logical',command_id:'command',phase_id:'cycle'},device_id:'device',state:'device_started',target_device_boot_id:'boot',started_at_host_ms:1000,completed_at_host_ms:null}},{kind:'device',data:{binding:{device_id:'device',operation_ref:{id:'operation',version:'1.0.0'}},freshness:'fresh',latest_observation:{host_received_elapsed_ms:1500,observation:{device_state:'busy',device_boot_id:'boot',active_command_id:'command',phase_progress:{phase_id:'cycle',effective_work_ms:500,duration_ms:1800}}}}}]};
const before=JSON.stringify(base),checks=[];function check(name,body){body();checks.push(name);}
const view=o=>executionDevicePhases(o).get('device');
check('Measured report is held at its sampled value; Host time does not animate it',()=>{
 assert.equal(view(base).point.progress,500/1800);const later=structuredClone(base);later.at_host_ms=50000;assert.deepEqual(view(later).point,view(base).point);
});
check('Old observations without progress stay explicitly unmeasured',()=>{const o=structuredClone(base);delete o.objects[2].data.latest_observation.observation.phase_progress;assert.equal(view(o).point,null);assert.match(view(o).label,/未提供/);});
check('Other command, device boot or phase cannot drive this model',()=>{for(const [key,value] of [['active_command_id','other'],['device_boot_id','other']]){const o=structuredClone(base);o.objects[2].data.latest_observation.observation[key]=value;assert.equal(view(o).point,null);}const o=structuredClone(base);o.objects[2].data.latest_observation.observation.phase_progress.phase_id='other';assert.equal(view(o).point,null);});
check('Future and pre-Started samples cannot drive the current observation',()=>{for(const time of [999,2001]){const o=structuredClone(base);o.objects[2].data.latest_observation.host_received_elapsed_ms=time;assert.equal(view(o).point,null);}});
check('A real completion receipt supplies an endpoint even for an old archive',()=>{const o=structuredClone(base);o.objects[0].data.state='waiting_output';o.objects[1].data.state='output_wait';o.objects[1].data.completed_at_host_ms=1800;delete o.objects[2].data.latest_observation.observation.phase_progress;assert.equal(view(o).point.progress,1);assert.equal(view(o).point.terminal_pose,true);});
check('An ambiguous physical device owner is never selected arbitrarily',()=>{const o=structuredClone(base);o.objects.push({...o.objects[0],data:{...o.objects[0].data,job_id:'other'}});assert.equal(view(o),undefined);});
assert.equal(JSON.stringify(base),before);
const presentation=await fs.readFile('apps/planner/src/services/executionPresentation.ts','utf8');
const factory=await fs.readFile('apps/planner/src/services/factoryPresentation.ts','utf8');
const transportCode=presentation.slice(presentation.indexOf('export function executionTransports('),presentation.indexOf('export async function createExecutionPresentation('));
const pathCode=factory.slice(factory.indexOf('export function pointOnPath('),factory.indexOf('export function projectFactoryTransport('));
await fs.writeFile(path.join(output,'executionTransport.mjs'),"import {executionDevicePhases} from './executionPhase.mjs';\n"+stripTypeScriptTypes(pathCode+'\n'+transportCode,{mode:'transform'}));
const {executionTransports}=await import(pathToFileURL(path.resolve(output,'executionTransport.mjs')).href);
check('Actual carrier reports move the original cargo along its frozen supply path',()=>{
 const o=structuredClone(base);o.objects[0].data.execution_ref={kind:'definition_cycle',data:{module_ref:{id:'transport',version:'1.0.0'}}};
 o.objects[0].data.phase_id='outbound';o.objects[1].data.attempt.phase_id='outbound';o.objects[2].data.latest_observation.observation.phase_progress={phase_id:'outbound',effective_work_ms:50,duration_ms:100};
 o.objects.push({kind:'transport',data:{id:'job',supply_ref:'supply',carrier_id:'device',phase:'outbound',cargo:['original-item']}});
 const inputs={scenario:{supplies:[{id:'supply',presentation_path:{hall_id:'hall',cargo_layout:{slots:[]},waypoints_m:[{x:0,y:0,z:1},{x:10,y:0,z:1}]}}]}};
 const t=executionTransports(inputs,o)[0];assert.equal(t.pose.translation.x,5);assert.deepEqual(t.trip.cargo,['original-item']);
 const later=structuredClone(o);later.at_host_ms=10000;assert.deepEqual(executionTransports(inputs,later)[0].pose,t.pose);
 delete o.objects[2].data.latest_observation.observation.phase_progress;assert.equal(executionTransports(inputs,o)[0].pose,null);
});
await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify({scope:'Projection unit fixtures only; actual cloud/Native not verified',checks,clock_interpolation:false,nominal_duration_used:false,live_cloud_verified:false},null,2));console.log('Execution phase projection checks passed:',checks.length);
