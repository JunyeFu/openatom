import {build} from 'vite';import {fileURLToPath} from 'node:url';
const root=new URL('../../../../',import.meta.url);
await build({configFile:fileURLToPath(new URL('apps/planner/vite.config.ts',root)),root:fileURLToPath(new URL('apps/planner/',root)),build:{outDir:fileURLToPath(new URL('scenarios/Agent共建/runtime-public-render-consumer/repairs-01/planner-web/',root)),emptyOutDir:false}});
