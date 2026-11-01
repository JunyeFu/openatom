// Compile the real component for finite SSR checks; no browser or server.
import { build } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
const root = new URL('../../../', import.meta.url);
await build({ configFile: false, plugins: [react()], root: fileURLToPath(root), logLevel: 'warn', build: {
  ssr: fileURLToPath(new URL('tests/planner/packaging/view-fixture.tsx', root)),
  outDir: fileURLToPath(new URL('.work/noncu-p003-view/', root)), emptyOutDir: false,
  minify: false, rollupOptions: { output: { entryFileNames: 'view-fixture.mjs' } },
} });
