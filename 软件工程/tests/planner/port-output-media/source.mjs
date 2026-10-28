import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
export const root=new URL('../../../',import.meta.url),out=new URL('scenarios/图片时间轴/runtime-port-output/',root),compiled=new URL('compiled/',out);
await mkdir(compiled,{recursive:true});
export const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
for(const [name,path]of [['dataFields','packages/presentation/src/dataFields.ts'],['phase','packages/presentation/src/phase.ts'],['presentationHost','apps/planner/src/services/presentationHost.ts']]){
 let source=stripTypeScriptTypes(await readFile(new URL(path,root),'utf8'),{mode:'transform'});
 if(name==='presentationHost')source=source.replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('Native not invoked by protocol tests')};").replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('Native bytes not invoked by protocol tests')};");
 await writeFile(new URL(name+'.mjs',compiled),source);
}
export const fields=await import(new URL('dataFields.mjs',compiled)),phase=await import(new URL('phase.mjs',compiled)),host=await import(new URL('presentationHost.mjs',compiled));
const saved=await json('scenarios/图片时间轴/runtime/phase-observations.actual.json'),bootstrap=await json('scenarios/图片时间轴/runtime/phase.bootstrap.actual.json'),historical=await json('scenarios/图片时间轴/runtime/phase-historical-job-current.actual.json'),inputs=await json('scenarios/图片时间轴/runtime/phase.input.json');
export function setup(options={}){
 const run=structuredClone(saved.run),project=structuredClone(saved.project),pkg=structuredClone(bootstrap.package),job=structuredClone(historical.outcome.data.data),at=options.at??75000;
 if(at>=85000)Object.assign(job,structuredClone(run.jobs.find(j=>j.id===job.id)));
 const snapshot={run_id:run.id,at_ms:at,next_index:null,objects:[{kind:'job',data:job},...(saved.observations.find(s=>s.at_ms===at)?.objects??[]).filter(o=>o.kind==='item')]};
 const context={session_id:'port-protocol',project_id:project.id,scope:{kind:'project'},scope_revision:9,selection:{kind:'actual_run',run_id:run.id,job_id:job.id,at_ms:at}};
 let current=context;const calls=[],portCalls=[];
 const packages=inputs.definitions.map(definition=>definition.id===pkg.definition.id?pkg:{definition,assets:[],files:[],dependencies:[],rights:[],issues:[]});
 const ports={async dispatch(request){calls.push(structuredClone(request));if(request.operation==='get_run'){if(request.params.query.kind==='full_result')return {kind:'full',data:run};const q=request.params.query.data;return {kind:'objects',data:{...snapshot,objects:q.objects.length?[{kind:'job',data:job}]:snapshot.objects.filter(o=>o.kind==='item')}};}if(request.params.object_id.kind==='project')return {kind:'project',data:project};const wanted=request.params.source.data.module_ref;return {kind:'module',data:packages.find(p=>p.definition.id===wanted.id&&p.definition.version===wanted.version)};},async readAsset(){return new ArrayBuffer(0);}};
 if(!options.omitPort)ports.readPortOutput=async request=>{portCalls.push(structuredClone(request));if(options.readPort)return options.readPort(request);return {run_id:run.id,at_ms:at,port:request.port,item_type_ref:{id:'part',revision:'r1'},completed_output:{value:{coefficient:at>=75000?1:0,decimal_places:0},unit:'piece'},accepted_output:{value:{coefficient:at>=85000?1:0,decimal_places:0},unit:'piece'}};};
 return {run,project,pkg,job,snapshot,context,calls,portCalls,ports,reader:host.createPresentationHost(()=>current,ports),switchTo:value=>current=value};
}
