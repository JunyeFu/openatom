import{readFile,writeFile,mkdir,access}from'node:fs/promises';import{spawnSync}from'node:child_process';import{fileURLToPath}from'node:url';
const root=new URL('../../../',import.meta.url),owner=new URL('scenarios/Agent共建/runtime-public-render-consumer/',root),out=new URL('diffs/',import.meta.url);await mkdir(out,{recursive:true});
const cut=JSON.parse(await readFile(new URL('source-cut.json',owner),'utf8')),rows=[];
for(const record of cut.records.filter(r=>r.group==='production'||r.group==='root_mount')){
 const flat=record.path.replaceAll('/','__'),before=new URL('before/'+flat,owner);let exists=true;try{await access(before);}catch(e){if(e.code==='ENOENT')exists=false;else throw e;}
 if(!exists){rows.push({path:record.path,mode:'new file reviewed directly'});continue;}
 const result=spawnSync('git',['diff','--no-index','--no-color','--no-ext-diff','--no-textconv','--unified=3',fileURLToPath(before),fileURLToPath(new URL(record.path,root))],{encoding:'utf8',windowsHide:true});
 if(![0,1].includes(result.status))throw Error(result.stderr);await writeFile(new URL(flat+'.patch',out),result.stdout);rows.push({path:record.path,changed:result.status===1,patch:'diffs/'+flat+'.patch'});
}
await writeFile(new URL('diff-intake.json',import.meta.url),JSON.stringify({consumer_frozen_at:cut.frozen_at,rows},null,2));console.log(JSON.stringify({files:rows.length,changed:rows.filter(r=>r.changed).length,new_files:rows.filter(r=>r.mode).length}));
