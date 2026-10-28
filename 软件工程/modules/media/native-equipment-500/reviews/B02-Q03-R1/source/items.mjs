import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
import {specs} from './batch-spec.mjs';import {exportItem} from './item-build.mjs';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),root=path.resolve(home,'../../../../..'),regs=[],metadata=[];
for(const cfg of [...Object.entries(specs).flatMap(([device,s])=>[{id:device+'-input',device,role:'input'},...(s.mode!=='crease'?[{id:device+'-output',device,role:'output'}]:[]),...s.extra_outputs.map(e=>({id:device+'-'+e.role,device,role:e.role})),...(s.printed?[{id:device+'-ink',device,role:'auxiliary'}]:[])]),{id:'blocker',device:'flexo-slotter',role:'blocker'}]){
 const dir=path.join(home,'items',cfg.id),out=await exportItem(cfg,dir);metadata.push({...cfg,...out,id:cfg.id,media_id:out.id});
 regs.push({id:out.id,version:out.version,category:'media',directory:path.relative(root,dir).replaceAll('\\','/')});
 regs.push({id:out.itemId,version:out.version,category:'items',item_type:cfg.id,directory:path.relative(root,path.join(dir,'item-definition')).replaceAll('\\','/')});
}
regs.sort((a,b)=>Number(a.category==='items')-Number(b.category==='items'));await fs.writeFile(path.join(home,'items/registration.json'),JSON.stringify({register_order:regs},null,2)+'\n');await fs.writeFile(path.join(home,'items/inventory.json'),JSON.stringify({metadata},null,2)+'\n');console.log('Q03 sixteen exact paper, scrap, ink and blocker types and media generated; not counted as equipment.');
