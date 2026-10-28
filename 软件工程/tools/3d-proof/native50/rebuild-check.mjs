import fs from 'node:fs/promises';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const root=process.cwd(),base=path.join(root,'modules/media/native-equipment-50');
const inventory=JSON.parse(await fs.readFile(path.join(base,'inventory.json'),'utf8')),scratch=path.join(root,'.work/native50-20261021/rebuild');await fs.mkdir(scratch,{recursive:true});
const results=[];
for(const e of inventory.entries){
 const target=path.join(scratch,e.id+'-'+e.suffix);await fs.cp(path.join(base,e.directory),target,{recursive:true});
 const run=spawnSync(process.execPath,['source/generate.mjs'],{cwd:target,windowsHide:true,encoding:'utf8'});
 if(run.status!==0)throw Error(e.id+' rebuild failed: '+run.stderr);
 const original=await fs.readFile(path.join(base,e.directory,e.asset.path)),rebuilt=await fs.readFile(path.join(target,e.asset.path));
 const same=original.equals(rebuilt);results.push({id:e.id,lod:e.suffix,same_export_bytes:same});if(!same)throw Error(e.id+' rebuilt export differs');
 console.log(`${results.length}/100 ${e.id}/${e.suffix}: rebuilt`);
}
await fs.writeFile(path.join(root,'tools/3d-proof/native50/source-rebuild-checks.json'),JSON.stringify({scope:'standalone_copied_source_reexport_compared_to_selected_GLB',models:results.length,results},null,2));
