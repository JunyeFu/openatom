import {build} from 'vite';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/planner-web/',root);
await build({configFile:fileURLToPath(new URL('apps/planner/vite.config.ts',root)),root:fileURLToPath(new URL('apps/planner/',root)),build:{outDir:fileURLToPath(out),emptyOutDir:false}});
