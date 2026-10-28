import {build} from 'vite';import react from '@vitejs/plugin-react';import {writeFile} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('../../../',import.meta.url)),runtime=new URL('../../../scenarios/图片时间轴/runtime-port-output/',import.meta.url),entry=new URL('build-preview.html',runtime);
await writeFile(entry,'<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"><title>P303 same-Host port output build</title></head><body><div id="root"></div><script type="module" src="./preview.tsx"></script></body></html>');
await build({root,configFile:false,plugins:[react()],build:{target:'esnext',outDir:fileURLToPath(new URL('build/',runtime)),emptyOutDir:false,rollupOptions:{input:fileURLToPath(entry)}}});
