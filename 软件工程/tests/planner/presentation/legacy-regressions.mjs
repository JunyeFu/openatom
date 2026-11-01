// Keep all original assertions. Redirect compilation output to this task's .work.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
const root=new URL('../../../',import.meta.url),out=new URL('.work/delivery/baseline/',root);
await mkdir(out,{recursive:true});
const files=[];
for(const name of ['media.test.mjs','shared-preview.test.mjs']) {
 let source=await readFile(new URL('tests/planner/media/'+name,root),'utf8');
 source=source.replace("new URL('../../../',import.meta.url)",`new URL(${JSON.stringify(root.href)})`)
  .replaceAll('scenarios/媒体表现/runtime/compiled/','.work/delivery/baseline/compiled/')
  .replaceAll('scenarios/媒体表现/runtime/shared-preview/compiled/','.work/delivery/baseline/shared-compiled/')
  .replaceAll('scenarios/媒体表现/runtime/actual-frames.json','.work/delivery/legacy-engine/actual-frames.json')
  .replaceAll('scenarios/媒体表现/runtime/shared-preview/actual-prefix.json','.work/delivery/legacy-engine/shared-preview/actual-prefix.json');
 const file=new URL(name,out);await writeFile(file,source);files.push(fileURLToPath(file));
}
const run=spawnSync(process.execPath,['--test',...files],{cwd:fileURLToPath(root),encoding:'utf8',windowsHide:true});
process.stdout.write(run.stdout??'');process.stderr.write(run.stderr??'');process.exitCode=run.status??1;
