import {build} from 'vite';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {spawnSync} from 'node:child_process';

const source=fileURLToPath(new URL('.',import.meta.url));
if(!process.argv[2])throw Error('Pass an explicit directory for the generator bundle.');
const directory=resolve(process.argv[2]);
await build({configFile:false,root:source,logLevel:'warn',ssr:{noExternal:true},
 build:{ssr:resolve(source,'generate-surface.mjs'),outDir:directory,emptyOutDir:false,minify:false,
 rollupOptions:{output:{entryFileNames:'generate-surface.mjs'}}}});
const result=spawnSync(process.execPath,[resolve(directory,'generate-surface.mjs'),
 resolve(process.argv[3]??resolve(source,'../candidate-1.0.5')),source],{stdio:'inherit'});
if(result.error)throw result.error;
process.exitCode=result.status;
