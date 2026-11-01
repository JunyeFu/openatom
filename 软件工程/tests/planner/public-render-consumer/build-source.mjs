import{build}from'vite';import{fileURLToPath}from'node:url';
const root=new URL('../../../',import.meta.url);
await build({configFile:false,root:fileURLToPath(root),logLevel:'warn',build:{ssr:fileURLToPath(new URL('tests/planner/public-render-consumer/source-exports.ts',root)),outDir:fileURLToPath(new URL('scenarios/Agent共建/runtime-public-render-consumer/compiled/',root)),emptyOutDir:false,minify:false,rollupOptions:{output:{entryFileNames:'source.mjs'}}}});
