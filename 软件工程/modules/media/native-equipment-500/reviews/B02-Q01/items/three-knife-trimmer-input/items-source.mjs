import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
import * as T from 'three';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import './export.mjs';import {definition,pack,version} from './contract.mjs';import {stock} from './paper-reviewed.mjs';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),root=path.resolve(home,'../../../../..');
const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),regs=[];
const specs=Object.entries(stock).flatMap(([id,values])=>['input','output'].map(role=>({id:id+'-'+role,size:values[role]})));
specs.push({id:'blocker',size:[.10,.02,.10]});
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:{x:1,y:1,z:1},unit:'meter',coordinate_frame:'right_handed_y_up'});
for(const spec of specs){
 const id='qa.paper.media.'+spec.id,itemId='qa.paper.items.'+spec.id,dir=path.join(home,'items',spec.id);await fs.mkdir(dir,{recursive:true});
 const scene=new T.Group();scene.name='PaperStack';const [w,h,d]=spec.size;
 const paper=new T.MeshStandardMaterial({name:'Uncoated paper',color:'#dbd0ac',roughness:.84}),edge=new T.MeshStandardMaterial({name:'Cut paper edges',color:'#afa482',roughness:.90});
 const sheets=8;for(let i=0;i<sheets;i++){const m=new T.Mesh(new T.BoxGeometry(w,h/sheets*.96,d),i%2?paper:edge);m.name='PaperLayer_'+i;m.position.y=(i+.5)*h/sheets;scene.add(m);}
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(scene,{binary:true}));await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.84,texture:null}})),poses:[],actions:[],timeline:[],tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 await fs.copyFile(path.join(home,'LICENSE.txt'),path.join(dir,'LICENSE.txt'));await fs.copyFile(fileURLToPath(import.meta.url),path.join(dir,'items-source.mjs'));
 await json(path.join(dir,'module.json'),pack(definition(id,'显式尺寸纸叠验收物品 '+spec.id,'media',media,[asset]),['paper.glb','LICENSE.txt','items-source.mjs'],[],[asset]));
 regs.push({id,version,category:'media',directory:path.relative(root,dir).replaceAll('\\','/')});
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 await json(path.join(itemDir,'module.json'),pack(definition(itemId,'纸叠验收类型 '+spec.id,'items',{unit:'piece',tracking:'individual',spec_ref:null,attributes:{},attribute_schema:[],presentation_or_media_ref:wrapper}),[],[{id,version}]));
 regs.push({id:itemId,version,category:'items',item_type:spec.id,directory:path.relative(root,itemDir).replaceAll('\\','/')});
}
regs.sort((a,b)=>Number(a.category==='items')-Number(b.category==='items'));await json(path.join(home,'items/registration.json'),{register_order:regs});console.log('Explicit paper item modules:',specs.length);
