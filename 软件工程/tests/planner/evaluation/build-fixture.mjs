import {build} from 'vite';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
const out=new URL('scenarios/方案比较/runtime-delivery-ui/fixture-build/',root);
await mkdir(out,{recursive:true});
await build({configFile:false,root:fileURLToPath(root),logLevel:'warn',define:{'process.env.NODE_ENV':JSON.stringify('development')},build:{outDir:fileURLToPath(out),emptyOutDir:false,
 lib:{entry:fileURLToPath(new URL('tests/planner/evaluation/fixture.tsx',root)),name:'FiniteDeliveryFixture',formats:['iife'],fileName:()=> 'fixture.js'},
 minify:false,sourcemap:false}});
const runs=JSON.parse(await readFile(new URL('packages/commands/tests/fixtures/comparison/actual-frozen-inputs.json',root),'utf8'));
const report=JSON.parse(await readFile(new URL('scenarios/方案比较/runtime-evaluation/output/comparison/report.json',root),'utf8')).data;
await writeFile(new URL('data.json',out),JSON.stringify({evidence:runs.map(([project,scenario,,run])=>({project,scenario,run})),report,mode:'success',calls:[]}));
console.log('Compiled actual components as an offline finite consumer; no HTTP listener or Host started.');
