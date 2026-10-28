import fs from 'node:fs/promises';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const root=process.cwd(),home=path.join(root,'modules/media/native-equipment-500'),old=path.join(root,'modules/media/native-equipment-50');
const inv=JSON.parse(await fs.readFile(path.join(old,'inventory.json'),'utf8')),ids=['digital-flatbed-cutter','case-packer','gantry-pick-place','vacuum-palletizer','cnc-mill'],entries=[],register=[];
const version='1.0.1';
const original="function cycle(tracks){clips.push(new T.AnimationClip('load',1,tracks.load??[]),new T.AnimationClip('auto',1,tracks.auto),new T.AnimationClip('unload',1,tracks.unload??[]));}";
const fixed=`function cycle(tracks){
 for(const phase of Object.keys(tracks)){
  const groups=new Map();for(const track of tracks[phase]){if(!groups.has(track.name))groups.set(track.name,[]);groups.get(track.name).push(track);}
  tracks[phase]=[...groups].map(([name,rows])=>{
   if(rows.length===1)return rows[0];if(!name.endsWith('.position'))throw Error('Unmerged duplicate '+name);
   const rest=root.getObjectByName(name.slice(0,-9)).position.toArray();
   for(const tr of rows)if(tr.times.length!==rows[0].times.length||tr.times.some((v,i)=>v!==rows[0].times[i]))throw Error('Mismatched times '+name);
   return new T.VectorKeyframeTrack(name,rows[0].times,Array.from(rows[0].values,(_,i)=>rest[i%3]+rows.reduce((s,tr)=>s+tr.values[i]-rest[i%3],0)));
  });
 }
 clips.push(new T.AnimationClip('load',1,tracks.load??[]),new T.AnimationClip('auto',1,tracks.auto),new T.AnimationClip('unload',1,tracks.unload??[]));
}`;
for(const id of ids){
 const spec=inv.entries.find(e=>e.id===id&&e.suffix==='detailed');
 for(const e of inv.entries.filter(e=>e.id===id)){
  const dir=path.join(home,'installable','b01-'+id+(e.suffix==='simple'?'-simple':''));await fs.cp(path.join(old,e.directory),dir,{recursive:true});
  const geom=path.join(dir,'source/geometry.mjs'),source=await fs.readFile(geom,'utf8');if(!source.includes(original))throw Error('Baseline source changed');await fs.writeFile(geom,source.replace(original,fixed));
  const catalog=path.join(dir,'source/catalog.mjs');await fs.writeFile(catalog,(await fs.readFile(catalog,'utf8')).replace("version = '1.0.0'","version = '1.0.1'"));
  const run=spawnSync(process.execPath,['source/generate.mjs'],{cwd:dir,windowsHide:true,encoding:'utf8'});if(run.status!==0)throw Error(run.stderr);
  const pkg=JSON.parse(await fs.readFile(path.join(dir,'module.json'),'utf8')).data,b=await fs.readFile(path.join(dir,e.asset.path)),j=JSON.parse(b.subarray(20,20+b.readUInt32LE(12)));
  const {presentation,...rest}=e;entries.push({...rest,batch:'B01',version,media_version:version,revised_from:'1.0.0',directory:path.relative(home,dir).replaceAll('\\','/'),bytes:b.length,nodes:j.nodes.length});register.push({id:e.module_id,version,directory:path.relative(home,dir).replaceAll('\\','/')});
 }
 const dir=path.join(home,'devices','b01-'+id);await fs.mkdir(dir,{recursive:true});const pkg=JSON.parse(await fs.readFile(path.join(old,'devices',id,'module.json'),'utf8'));pkg.data.definition.version=version;
 function update(o){if(!o||typeof o!=='object')return;if(o.id===spec.media_id||o.id===spec.media_id+'.simple')o.version=version;for(const v of Object.values(o))update(v);}update(pkg);
 await fs.writeFile(path.join(dir,'module.json'),JSON.stringify(pkg,null,2)+'\n');register.push({id:spec.definition_id,version,directory:path.relative(home,dir).replaceAll('\\','/')});
 console.log(id+' 1.0.0 → 1.0.1, two LODs and device revised');
}
await fs.writeFile(path.join(home,'batches/B01-revisions.json'),JSON.stringify({ids,entries,register,reason:'merge absolute XYZ translation tracks for same node; retain original source/packages untouched'},null,2)+'\n');
