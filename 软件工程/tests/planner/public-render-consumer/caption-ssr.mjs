import {build} from 'vite';import react from '@vitejs/plugin-react';import {fileURLToPath} from 'node:url';import {readFile,writeFile} from 'node:fs/promises';import assert from 'node:assert/strict';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/',root);
await build({configFile:false,plugins:[react()],root:fileURLToPath(root),logLevel:'warn',build:{ssr:fileURLToPath(new URL('tests/planner/public-render-consumer/caption-ssr.tsx',root)),outDir:fileURLToPath(new URL('caption-compiled/',out)),emptyOutDir:false,minify:false,rollupOptions:{output:{entryFileNames:'caption.mjs'}}}});
const {renderCaption}=await import(new URL('caption-compiled/caption.mjs',out));
const input=JSON.parse(await readFile(new URL('scenarios/Agent共建/runtime-public-render-host/actual-inputs/session-82352/capture-15/input.json',root),'utf8'));
const actual=renderCaption(input.packaging.job,true),ordinary=renderCaption(input.packaging.job,false);
assert.match(actual,/只读捕获，使用请求相机与固定时点/);assert.ok(!actual.includes('右键环绕'));assert.match(ordinary,/右键环绕、中键平移、滚轮缩放/);assert.ok(!ordinary.includes('只读捕获'));
await writeFile(new URL('caption-ssr.json',out),JSON.stringify({checks:2,status:'passed',actual,ordinary,evidence:'real ProcessViewport finite server render over original actual Host job; wording only; browser overlay attempted but original8922 refused connection',Native:false},null,2));
console.log('PASS actual ProcessViewport caption2, capture truthful readonly / ordinary original interaction text; SSR only');
