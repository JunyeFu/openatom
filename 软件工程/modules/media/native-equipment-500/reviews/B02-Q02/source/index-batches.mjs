import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),old=path.resolve(home,'../native-equipment-50');
const baseline=JSON.parse(await fs.readFile(path.join(old,'inventory.json'),'utf8'));
const previous=JSON.parse(await fs.readFile(path.join(old,'registration.json'),'utf8'));
const clean=e=>{const {presentation,...rest}=e;return rest;};
const entries=baseline.entries.map(e=>({...clean(e),batch:'B01',directory:'../native-equipment-50/'+e.directory}));
const register=previous.register_order.map(r=>({...r,directory:'../native-equipment-50/'+path.relative(old,path.resolve(home,'../../../..',r.directory)).replaceAll('\\','/')}));
// Registration paths are rooted at this index; original exact versions are reused in place.
for(let i=0;i<register.length;i++)register[i].directory=previous.register_order[i].directory.replace('modules/media/native-equipment-50','../native-equipment-50');
const batches=[{id:'B01',title:'既有纸包装与通用设备',devices:50,models:100,reused_exact_version:true}];
const revisions=await fs.access(path.join(home,'batches/B01-revisions.json')).then(()=>true,()=>false);
if(revisions){const b=JSON.parse(await fs.readFile(path.join(home,'batches/B01-revisions.json'),'utf8'));for(const e of b.entries)entries[entries.findIndex(old=>old.id===e.id&&old.suffix===e.suffix)]=e;for(const r of b.register)register[register.findIndex(old=>old.id===r.id)]=r;batches[0].revised_devices=b.ids.length;}
const equipment=entries.filter(e=>e.suffix==='detailed');
for(const f of (await fs.readdir(path.join(home,'batches'))).filter(f=>f.endsWith('.inventory.json')).sort()){
 const b=JSON.parse(await fs.readFile(path.join(home,'batches',f),'utf8'));entries.push(...b.entries);equipment.push(...b.entries.filter(e=>e.suffix==='detailed'));register.push(...b.register);batches.push({id:b.batch,title:b.title,devices:b.devices,models:b.models,reused_exact_version:false});
}
if(new Set(equipment.map(e=>e.id)).size!==equipment.length)throw Error('Duplicate device identity across batches');
register.sort((a,b)=>{const tier=r=>r.id.includes('.device.')||r.id.startsWith('sf.native50.device.')?2:r.id.endsWith('.simple')?0:1;return tier(a)-tier(b);});
const json=(n,v)=>fs.writeFile(path.join(home,n),JSON.stringify(v,null,2)+'\n');
await json('inventory.json',{version:'1.0.0',target_devices:500,devices:equipment.length,models:entries.length,scope:'installed_model_candidates_not_full_MQ_acceptance',batches,entries});
await json('catalog.json',{target_devices:500,device_count:equipment.length,batches,equipment});
await json('registration.json',{device_count:equipment.length,media_count:entries.length,register_order:register});
await fs.writeFile(path.join(home,'register-directories.txt'),register.map(r=>r.directory||'.').join('\n')+'\n');
console.log(JSON.stringify({devices:equipment.length,models:entries.length,packages:register.length,batches:batches.length}));
