import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'../../..'),base=path.join(root,'.work/full-closeout-20261021/delivery');
const read=async p=>JSON.parse(await readFile(path.resolve(root,p),'utf8'));
await mkdir(path.join(base,'consumer'),{recursive:true});
for(const file of await readdir(path.join(root,'packages/presentation/src'))){
 if(!file.endsWith('.ts'))continue;
 const source=await readFile(path.join(root,'packages/presentation/src',file),'utf8');
 const code=stripTypeScriptTypes(source,{mode:'transform'}).replace(/from '\.\/(\w+)'/g,"from './$1.mjs'");
 await writeFile(path.join(base,'consumer',file.replace(/\.ts$/,'.mjs')),code);
}
const files={load:'apps/planner/src/features/media/load.ts',imageSurface:'apps/planner/src/features/media/imageSurface.ts',
 visualCapture:'apps/planner/src/features/render-surface/visualCapture.ts',factoryMedia:'apps/planner/src/scene/factoryMedia.ts',
 factoryItems:'apps/planner/src/scene/factoryItems.ts',factoryResources:'apps/planner/src/scene/factoryResources.ts',
 factoryPresentation:'apps/planner/src/services/factoryPresentation.ts',presentationHost:'apps/planner/src/services/presentationHost.ts',
 frozenProcess:'apps/planner/src/services/frozenProcess.ts',layout:'apps/planner/src/scene/layout.ts'};
for(const [name,file]of Object.entries(files)){
 let code=stripTypeScriptTypes(await readFile(path.join(root,file),'utf8'),{mode:'transform'});
 code=code.replace(/import\s*\{\s*dispatch\s*\}\s*from\s*['"]\.\/projectHost['"];?/g,"const dispatch=async()=>{throw Error('Captured Host port only; Native dispatch not exercised');};")
 .replace(/import\s*\{\s*readMediaAsset\s*\}\s*from\s*['"]\.\/mediaAssets['"];?/g,"const readMediaAsset=async()=>{throw Error('Exact captured Host asset port required');};");
 // Vite resolves even unused dynamic imports. These Native-default thunks are
 // outside projectFactoryPresentation and never used by its injected ports.
 code=code.replace(/import\('\.\/(runObservation|runPortOutput)'\)/g,"Promise.reject(new Error('Native default is not exercised by this captured-input browser'))");
 code=code.replace(/from\s+['"]([^'"]+)['"]/g,(whole,ref)=>ref.endsWith('/packages/presentation/src')?"from './index.mjs'":files[ref.split('/').at(-1)]?`from './${ref.split('/').at(-1)}.mjs'`:whole);
 await writeFile(path.join(base,'consumer',name+'.mjs'),code);
}
const runtime=await read((await read('.work/delivery/host-runtime/delivery/latest-runtime-result.json')).result);
const priorRoot='.work/delivery/items-reception/artifacts';
let prior;
for(const dir of await readdir(path.join(root,priorRoot))){
 try{if((await read(priorRoot+'/'+dir+'/l03-complete/case-result.json')).passed){prior=priorRoot+'/'+dir;break;}}
 catch(error){if(error.code!=='ENOENT')throw error;}
}
if(!prior)throw Error('No actual successful original Items archive');
const persisted=await read((await read('.work/full-closeout-20261021/delivery/items-persistence/latest.json')).result);
const newlySaved=new Map(persisted.cases.filter(c=>c.new_case_saved).map(c=>[c.label,c]));
const selected={
 'l03-complete':[42999,43000,48999,49000,87999,88000,89999,90000],
 'l03-missing-pcb':[0,360000],'phase-resource-wait':[34999,35000,39999,40000,45000],
 'phase-interrupted':[19999,20000,30000,39999,40000,60000],
 'l03-robot-output-wait':[47999,48000,60000,360000],
 'l03-pallet-output-wait':[87999,88000,89999,90000,360000],
};
const cases=[];
for(const entry of runtime.cases){
 const current=newlySaved.get(entry.label);
 const directory=current?path.dirname(current.moved_project).replace(/[\\/]receiver$/,''):path.join(prior,entry.label);
 const result=await read(path.join(directory,'case-result.json')),runs=await read(path.join(directory,'runs.json'));
 const projects=await read(path.join(directory,'frozen-projects.json')),points=await read(path.join(directory,'timepoints.json'));
 const moved=current?.moved_project??result.moved_project,manifest=(await read(path.join(moved,'project.json'))).data;
 const modules=[];
 for(const doc of manifest.documents)if(doc.role==='module_definition')modules.push((await read(path.join(moved,doc.file_ref.path))).data);
 for(const t of selected[entry.label])if(!points.points.some(p=>p.at_ms===t))throw Error('Missing exact original point '+entry.label+'/'+t);
 cases.push({label:entry.label,run:runs.new,project:projects.new,points:points.points.filter(p=>selected[entry.label].includes(p.at_ms)),
  scope_revision:points.current_revision,modules,old_run:runs.old,old_project:projects.old});
 if(entry.label==='l03-complete')cases.push({label:'l03-complete-old-0.3',run:runs.old,project:projects.old,
  points:points.points.filter(p=>[87999,88000,90000].includes(p.at_ms)),scope_revision:points.current_revision,modules});
}
const controlled=await read('.work/delivery/controlled-host/result.json');
const gif=controlled.byte_reads.find(x=>x.variant==='gif'&&x.asset.kind==='gif');
await writeFile(path.join(base,'render/input.json'),JSON.stringify({cases,gif,source:'actual ProjectHost snapshots and portable exact module packages',Native:false}));
await writeFile(path.join(base,'render/expectations.json'),JSON.stringify({frozen_before_browser:true,selected_times:selected,
 old_exact_carton_times:[87999,88000,90000],lods:['detailed','simplified'],expected_screens:66,source:'original Host assertions, effective work, immutable inventory and canonical attachment contracts',
 native_acceptance:false,performance_acceptance:false},null,2));
console.log(JSON.stringify({cases:cases.length,screens:cases.reduce((n,c)=>n+c.points.length*2,0),output:base}));
