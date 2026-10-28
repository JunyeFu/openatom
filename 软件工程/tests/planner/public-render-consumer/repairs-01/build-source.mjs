import{build}from'vite';import{fileURLToPath}from'node:url';
const root=new URL('../../../../',import.meta.url);
await build({configFile:false,root:fileURLToPath(root),logLevel:'warn',build:{ssr:fileURLToPath(new URL('tests/planner/public-render-consumer/repairs-01/source-exports.ts',root)),outDir:fileURLToPath(new URL('scenarios/Agent共建/runtime-public-render-consumer/repairs-01/compiled/',root)),emptyOutDir:false,minify:false,rollupOptions:{output:{entryFileNames:'source.mjs'}}}});
