import { build } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
const root = new URL('../../../', import.meta.url);
await build({ configFile: false, plugins: [react()], root: fileURLToPath(root), logLevel: 'warn', build: {
  ssr: fileURLToPath(new URL('tests/planner/packaging/view-fixture.tsx', root)),
  outDir: fileURLToPath(new URL('scenarios/Agent共建/runtime-public-render-consumer/regression/old-packaging-ssr/', root)), emptyOutDir: false,
  minify: false, rollupOptions: { output: { entryFileNames: 'view-fixture.mjs' } },
} });
console.log('Built actual PackagingWorkspace as finite SSR consumer; no server, Native or Host started.');
