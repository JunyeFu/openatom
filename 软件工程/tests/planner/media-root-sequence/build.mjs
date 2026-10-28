import {build} from 'vite';
import react from '@vitejs/plugin-react';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
await build({configFile:false,root:fileURLToPath(root),plugins:[react()],build:{
  outDir:fileURLToPath(new URL('scenarios/媒体表现/runtime-root-sequence/web/',root)),emptyOutDir:true,target:'es2022',
  rollupOptions:{input:fileURLToPath(new URL('tests/planner/media-root-sequence/harness.html',root))},
}});
