import fs from 'node:fs/promises';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const root=process.cwd(),home=path.join(root,'modules/media/native-equipment-500'),scratch=path.join(root,'.work/native500-20261024/source-rebuild');
const inv=JSON.parse(await fs.readFile(path.join(home,'inventory.json'),'utf8')),selected=process.argv.slice(2),out=path.join(root,'tools/3d-proof/native500/source-rebuild-checks.json');
const prior=selected.length?JSON.parse(await fs.readFile(out,'utf8')).results:[],results=prior.filter(e=>!selected.includes(e.batch));
await fs.mkdir(scratch,{recursive:true});
for(const e of inv.entries.filter(e=>(e.batch!=='B01'||e.revised_from)&&(!selected.length||selected.includes(e.batch)))){
 const original=path.join(home,e.directory),target=path.join(scratch,e.batch);await fs.mkdir(target,{recursive:true});
 await fs.cp(path.join(original,'source'),path.join(target,'source'),{recursive:true});
 for(const f of ['LICENSE.txt','nameplate.png'])await fs.copyFile(path.join(original,f),path.join(target,f));
 const run=spawnSync(process.execPath,[e.revised_from?'source/generate.mjs':'source/export.mjs'],{cwd:target,windowsHide:true,encoding:'utf8'});if(run.status!==0)throw Error(e.id+' source rebuild: '+run.stderr);
 const actual=await fs.readFile(path.join(target,e.asset.path)),expected=await fs.readFile(path.join(original,e.asset.path));if(!actual.equals(expected))throw Error(e.id+' '+e.suffix+' export differs');
 results.push({id:e.id,lod:e.suffix,batch:e.batch,same_export_bytes:true});
 // Only task-created output is removed, retaining the compact final source workspace per batch.
 await fs.unlink(path.join(target,e.asset.path));
 if(results.length%100===0)console.log(e.batch+' independent source re-export 100/100 PASS');
}
await fs.writeFile(out,JSON.stringify({models:results.length,unmodified_baseline_models:90,baseline_evidence:'../native50/source-rebuild-checks.json',scope:'each_new_or_revised_package_source_and_own_spec_copied_then_independently_reexported',results},null,2)+'\n');
