import {build} from 'vite';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {spawnSync} from 'node:child_process';

const source=fileURLToPath(new URL('.',import.meta.url));
if(!process.argv[2])throw Error('Pass an explicit output directory for checks.');
const directory=resolve(process.argv[2]);
await build({configFile:false,root:source,logLevel:'warn',ssr:{noExternal:true},
 build:{ssr:resolve(source,'check-surface.ts'),outDir:directory,emptyOutDir:false,minify:false,
 rollupOptions:{output:{entryFileNames:'check.mjs'}}}});
const adapter=pathToFileURL(resolve(source,'../../../../tests/planner/presentation/pixel-decoder.mjs')).href;
const result=spawnSync(process.execPath,[resolve(directory,'check.mjs'),resolve(source,'..'),
 resolve(directory,'result.json'),adapter],{stdio:'inherit'});
if(result.error)throw result.error;
process.exitCode=result.status;
