import {build} from 'vite';
import react from '@vitejs/plugin-react';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
await build({configFile:false,root:fileURLToPath(root),plugins:[react()],build:{
  outDir:fileURLToPath(new URL('scenarios/Agent共建/runtime-factory-render-surface/web/',root)),emptyOutDir:false,target:'es2022',
  rollupOptions:{input:[fileURLToPath(new URL('./harness.html',import.meta.url)),fileURLToPath(new URL('./actual.html',import.meta.url))]},
}});
