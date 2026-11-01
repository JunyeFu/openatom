import {readFile,writeFile} from 'node:fs/promises';
for(const name of ['core_consumer.rs','core.mjs','build-core.py']){
 let text=await readFile(new URL('../item-layout/'+name,import.meta.url),'utf8');text=text.replaceAll('runtime-item-layout-consumer','runtime-node-inspector').replaceAll('tests/planner/item-layout/','tests/planner/node-inspector/');
 if(name==='build-core.py')text=text.replace('Root approved item-layout + CapturePreview frozen current cut; exact libsf_commands-e579663d2bd6c8b8.rlib','Root approved Node8 + frozen-instance4 cut; exact libsf_commands-e579663d2bd6c8b8.rlib');
 await writeFile(new URL(name,import.meta.url),text);
}
