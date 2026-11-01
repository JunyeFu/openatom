import{readFile,writeFile}from'node:fs/promises';
const root=new URL('../../../',import.meta.url),owner='scenarios/Agent共建/runtime-public-render-consumer/';
const cut=JSON.parse(await readFile(new URL(owner+'source-cut.json',root),'utf8'));
const rows=[];
for(const record of cut.records){
 const actual=await readFile(new URL(record.path,root)),frozen=await readFile(new URL(owner+'source-snapshot/'+record.path,root));
 rows.push({path:record.path,group:record.group,actual_bytes:actual.length,frozen_bytes:frozen.length,exact_bytes_equal:actual.equals(frozen)});
}
const result={evidence:'independent direct byte comparison to Root-authorized final consumer snapshots; own Native/bridge not independently reviewed',consumer_frozen_at:cut.frozen_at,expected_records:cut.records.length,checked:rows.length,mismatch:rows.filter(v=>!v.exact_bytes_equal),rows};
await writeFile(new URL('source-check.json',import.meta.url),JSON.stringify(result,null,2));
console.log(JSON.stringify({checked:result.checked,mismatch:result.mismatch.length,consumer_frozen_at:cut.frozen_at}));
if(result.mismatch.length)process.exitCode=1;
