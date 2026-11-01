import { build } from 'vite';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const directory = fileURLToPath(new URL('.', import.meta.url));
if (!process.argv[2]) throw Error('Pass an explicit output directory for the check bundle.');
await build({ configFile: false, root: directory, logLevel: 'warn',
  ssr: { noExternal: true }, build: { ssr: resolve(directory, 'check.ts'),
    outDir: resolve(process.argv[2]), emptyOutDir: false, minify: false,
    rollupOptions: { output: { entryFileNames: 'check.mjs' } } } });
