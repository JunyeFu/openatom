import {build} from 'vite';
import react from '@vitejs/plugin-react';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
await build({configFile:false,root:fileURLToPath(new URL('apps/planner/src/features/media/',root)),plugins:[react()],build:{outDir:fileURLToPath(new URL('scenarios/媒体表现/runtime/shared-preview/web/',root)),emptyOutDir:true,target:'es2022',rollupOptions:{input:['harness.html','shared-harness.html'].map(file=>fileURLToPath(new URL('apps/planner/src/features/media/'+file,root)))}}});
