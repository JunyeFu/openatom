import {spawnSync} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-factory-canvas/',root);await mkdir(out,{recursive:true});const results={};
async function run(name,args){const result=spawnSync(process.execPath,args,{cwd:fileURLToPath(root),encoding:'utf8'});if(result.error)throw result.error;await writeFile(new URL(`${name}.stdout.txt`,out),result.stdout??'');await writeFile(new URL(`${name}.stderr.txt`,out),result.stderr??'');results[name]={exit_code:result.status};console.log(`${name}: ${result.status}`);}
await run('protocol',['--test','tests/planner/factory-media/source.test.mjs']);
await run('planner-typecheck',['node_modules/typescript/bin/tsc','--noEmit','--project','apps/planner/tsconfig.json']);
await run('harness-typecheck',['node_modules/typescript/bin/tsc','--noEmit','--strict','--skipLibCheck','--resolveJsonModule','--allowSyntheticDefaultImports','--target','ES2022','--module','ESNext','--moduleResolution','Bundler','--jsx','react-jsx','tests/planner/factory-media/harness.tsx']);
await run('independent-build',['tests/planner/factory-media/build.mjs']);
await run('browser-green',['tests/planner/factory-media/browser.mjs','green']);
await writeFile(new URL('verification.json',out),JSON.stringify({results},null,2)+'\n');if(Object.values(results).some(v=>v.exit_code!==0))process.exitCode=1;
