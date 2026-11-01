import {spawnSync} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const stage=process.argv[2];if(!['red','green'].includes(stage))throw new Error('Pass red or green explicitly');
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-root-sequence/',root);await mkdir(out,{recursive:true});
const results={};
async function run(name,args){const result=spawnSync(process.execPath,args,{cwd:fileURLToPath(root),encoding:'utf8'});if(result.error)throw result.error;
 await writeFile(new URL(`${name}-${stage}.stdout.txt`,out),result.stdout??'');await writeFile(new URL(`${name}-${stage}.stderr.txt`,out),result.stderr??'');
 results[name]={exit_code:result.status};console.log(`${name}: ${result.status}`);return result.status===0;}
if(stage==='green'){
 await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','--project','apps/planner/tsconfig.json']);
 await run('harness-typecheck',['node_modules/typescript/bin/tsc','--noEmit','--strict','--skipLibCheck','--resolveJsonModule','--allowSyntheticDefaultImports','--target','ES2022','--module','ESNext','--moduleResolution','Bundler','--jsx','react-jsx','tests/planner/media-root-sequence/harness.tsx']);
 await run('independent-build',['tests/planner/media-root-sequence/build.mjs']);
}
await run('browser',['tests/planner/media-root-sequence/browser.mjs',stage]);
await writeFile(new URL(`verification-${stage}.json`,out),JSON.stringify({stage,results},null,2)+'\n');
if(Object.values(results).some(value=>value.exit_code!==0))process.exitCode=1;
