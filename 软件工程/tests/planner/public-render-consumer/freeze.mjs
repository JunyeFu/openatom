import {readFile,writeFile,mkdir,copyFile,readdir,stat} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/',root);
const rootOwned=['apps/planner/src/main.tsx','apps/planner/src/features/factory/Workbench.tsx','apps/planner/src/services/storedPackaging.ts'];
const original=JSON.parse(await readFile(new URL('before/files.json',out),'utf8'));
const own=[...original.filter(path=>!rootOwned.includes(path)),
 'apps/planner/src/services/renderTask.ts','apps/planner/src/services/renderTaskSource.ts',
 'apps/planner/src/features/render-surface/RenderTaskSurface.tsx','apps/planner/src/features/render-surface/visualCapture.ts','apps/planner/src/features/render-surface/render-task.css',
 'apps/planner/src/features/packaging/ManufacturingPreview.tsx','apps/planner/src/features/packaging/manufacturing-preview.css',
 'apps/planner/src/features/comparison/ComparisonResult.tsx','packages/presentation/src/scene.ts'];
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const records=[];
async function snapshot(path,owner,group){
 const source=new URL(path,root),bytes=await readFile(source),target=new URL('source-snapshot/'+path,out);await mkdir(new URL('./',target),{recursive:true});await copyFile(source,target);
 let before=null;try{const old=await readFile(new URL('before/'+path.replaceAll('/','__'),out));before={sha256:sha(old),bytes:old.length,changed:!old.equals(bytes)};}catch(error){if(error.code!=='ENOENT')throw error;}
 records.push({path,owner,group,bytes:bytes.length,sha256:sha(bytes),before});
}
for(const path of own)await snapshot(path,'actual_device_process','production');
for(const path of rootOwned)await snapshot(path,'Root','root_mount');
for(const name of await readdir(new URL('tests/planner/public-render-consumer/',root))){const path='tests/planner/public-render-consumer/'+name;if((await stat(new URL(path,root))).isFile())await snapshot(path,'actual_device_process','test');}
const frozenAt=new Date().toISOString();
await writeFile(new URL('source-cut.json',out),JSON.stringify({frozen_at:frozenAt,production_files:own.length,root_mount_files:rootOwned.length,records,note:'Copies of exact authorized sources and Root mount. No Host/Native/domain/config mutation. Original before/raw retained; scene.ts later explicit lease has no claimed original before copy.'},null,2));
const artifacts=[];for(const directory of ['', 'assets/'])for(const name of await readdir(new URL('planner-web/'+directory,out))){const path='planner-web/'+directory+name;if((await stat(new URL(path,out))).isFile()){const bytes=await readFile(new URL(path,out));artifacts.push({path,bytes:bytes.length,sha256:sha(bytes)});}}
const html=await readFile(new URL('planner-web/index.html',out),'utf8'),entry=html.match(/src="([^"]+\.js)"/)?.[1];
await writeFile(new URL('build-receipt.json',out),JSON.stringify({frozen_at:frozenAt,command:'node tests/planner/public-render-consumer/build.mjs',exit_code:0,raw:'freeze-build.stdout.log',production_source_cut:'source-cut.json',output:'planner-web',active_html:'planner-web/index.html',active_entry:entry,artifacts,retained_previous_artifacts:artifacts.filter(value=>value.path.endsWith('.js')&&value.path.includes('/index-')&&!value.path.endsWith(entry?.split('/').at(-1))).map(value=>value.path),typescript:[{project:'apps/planner',exit_code:0,raw:'freeze-planner-ts.log'},{project:'packages/presentation',exit_code:0,raw:'final-presentation-ts.log'},{project:'consumer harness',exit_code:0,raw:'freeze-harness-ts.log'}],build_warning:'Vite large chunk advisory retained, no warning suppressed. No Native build/capture performed.'},null,2));
console.log(JSON.stringify({production:own.length,root:rootOwned.length,tests:records.filter(value=>value.group==='test').length,entry,sourceCut:fileURLToPath(new URL('source-cut.json',out))}));
