import {build} from 'vite';
import react from '@vitejs/plugin-react';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
await build({configFile:false,root:fileURLToPath(root),plugins:[react()],build:{
  ssr:fileURLToPath(new URL('tests/planner/process-workspace/root-contract.tsx',root)),
  outDir:fileURLToPath(new URL('scenarios/媒体表现/runtime-root-consumer/static/',root)),emptyOutDir:true,target:'node24',
}});
