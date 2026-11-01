import {build} from 'vite';import react from '@vitejs/plugin-react';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-inspector/',root);
await build({configFile:false,root:fileURLToPath(root),plugins:[react()],build:{outDir:fileURLToPath(new URL('web/',out)),emptyOutDir:false,target:'es2022',rollupOptions:{input:fileURLToPath(new URL('tests/planner/node-inspector/harness.html',root))}}});
await build({configFile:fileURLToPath(new URL('apps/planner/vite.config.ts',root)),root:fileURLToPath(new URL('apps/planner/',root)),build:{outDir:fileURLToPath(new URL('planner-web/',out)),emptyOutDir:false}});
