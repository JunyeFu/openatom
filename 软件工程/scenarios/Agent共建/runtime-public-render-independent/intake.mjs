import {readFile,writeFile} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url);
const indexFiles=['scenarios/Agent共建/runtime-public-render-host/actual-inputs-index.json','scenarios/Agent共建/runtime-public-render-host/actual-visual-supplement/complete-index.json'];
const rows=[];
for(let group=0;group<indexFiles.length;group++){
 const index=JSON.parse(await readFile(new URL(indexFiles[group],root),'utf8'));
 for(const capture of index.captures){
  const directory=capture.directory,read=async name=>JSON.parse(await readFile(new URL(directory+'/'+name,root),'utf8'));
  const frame=await read('frame.json'),input=await read('input.json'),bytes=await read('bytes.json');
  rows.push({group:group?'supplement':'original',directory,task_id:frame.task_id,kind:frame.request.view.kind,scope:frame.scope,scope_revision:frame.scope_revision,project_revision:frame.request.view.project_revision,run_id:frame.request.view.run_id,at_ms:frame.request.view.view_time_ms,target:frame.request.object_id,hall_id:frame.request.view.hall_id,module_ref:frame.request.module_ref??null,preview:frame.request.preview,asset_count:input.assets.length,file_count:input.files.length,byte_asset_count:bytes.assets.length,byte_file_count:bytes.files.length,definition_category:input.definition?.payload.category??null,packaging_outcome:input.packaging?.result.outcome??null,active_job_id:input.active_job_id,issues:input.issues.map(value=>value.code)});
 }
}
const result={stage:'intake_only_not_consumer_verification',indices:indexFiles,original:rows.filter(row=>row.group==='original').length,supplement:rows.filter(row=>row.group==='supplement').length,kinds:[...new Set(rows.map(row=>row.kind))],rows};
await writeFile(new URL(process.argv[2]??'source-matrix.json',import.meta.url),JSON.stringify(result,null,2));
for(const row of rows)console.log([row.group,row.directory.split('/').at(-1),row.kind,row.scope.kind,'ScopeR'+row.scope_revision,'ProjectR'+row.project_revision,row.run_id??'',row.at_ms??'',row.definition_category??'',row.packaging_outcome??''].join(' | '));
