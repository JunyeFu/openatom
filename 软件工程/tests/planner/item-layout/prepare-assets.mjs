import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-item-layout-consumer/',root),clone=structuredClone;
const original=JSON.parse(await readFile(new URL('modules/media/p302-controlled/model.module.json',root),'utf8'));
const pose=(x=0,y=0,z=0,scale=1)=>({unit:'meter',coordinate_frame:'right_handed_z_up',translation:{x,y,z},rotation:{x:0,y:0,z:0,w:1},scale:{x:scale,y:scale,z:scale}});
const ref=(id,version='1.0.0')=>({id,version});const registry=[];
function native(simple=false,version='1.0.0'){
 const p=clone(original.definition.payload.data);p.kind='model';p.source={kind:'native_scene',data:'layout-owned-scene'};p.lod={...p.lod,detailed:{...p.source},simplified:{...p.source}};p.timeline=[];p.actions=[];p.poses=[];p.tags=[];p.port_anchors={};
 const mk=(id,name,x,color)=>({id,name,parent_index:null,transform:pose(x,0,0,1.25),geometry:{kind:'box',data:{x:.24,y:.18,z:.12}},material:{color_rgba:color,metallic:0,roughness:.8,texture:null}});
 p.nodes=[mk(80,'cavity',0,version==='1.0.0'?[1,.1,.1,1]:[.1,1,.1,1]),mk(81,'support',.4,[.2,.2,.2,1])];if(simple){p.nodes.reverse();p.nodes.unshift(mk(82,'simple-extra',-.4,[.1,.1,.8,1]));}
 p.anchor_roles={cavity:{kind:'scene_node',data:simple?2:0},workpiece:{kind:'scene_node',data:simple?2:0}};return p;
}
async function writePackage(id,version,payload,assets=[],dependencies=[],glb=false){
 const dir=new URL('packages/'+id+'/'+version+'/',out);await mkdir(dir,{recursive:true});const rights=assets.map(a=>({asset_ref:a.id,source:'p302-controlled original CC0; owned consumer logical-role fixture',license:null,redistribution_confirmed:false,missing_reason:'controlled fixture only'}));
 const definition={id,version,title:id,definition_state:'design',contribution:{author:null,license:null,assets:rights},parameters:[],attribute_schema:[],payload};
 await writeFile(new URL('module.json',dir),JSON.stringify({schema_version:'1.0',data:{definition,assets,files:assets.map(a=>({path:a.path,kind:'asset'})),dependencies,rights,issues:[]}},null,2));if(glb)for(const a of assets)await copyFile(new URL('modules/media/p302-controlled/'+a.path,root),new URL(a.path,dir));registry.push(fileURLToPath(dir));
}
for(const version of ['1.0.0','2.0.0']){
 await writePackage('layout.native.simple',version,{category:'media',data:native(true,version)});
 let item=native(false,version);item.lod.simplified={kind:'media_ref',data:ref('layout.native.simple',version)};await writePackage('layout.native.item',version,{category:'media',data:item},[],[ref('layout.native.simple',version)]);
 let model=clone(original.definition.payload.data);model.timeline=[];model.anchor_roles={workpiece:{kind:'scene_node',data:1}};model.lod.simplified={kind:'media_ref',data:ref('layout.native.simple',version)};
 await writePackage('layout.device.model',version,{category:'media',data:model},original.assets,[ref('layout.native.simple',version)],true);
 for(const title of ['shell','pcb','fastener','assembly','carton','package'])await writePackage('layout.items.'+title,version,{category:'items',data:{presentation_or_media_ref:{...native(false,version),nodes:[],anchor_roles:{},source:{kind:'media_ref',data:ref('layout.native.item',version)},lod:{...item.lod,detailed:null,simplified:{kind:'media_ref',data:ref('layout.native.item',version)}}},unit:'piece',tracking:'individual',spec_ref:null,attributes:{},attribute_schema:[]}},[],[ref('layout.native.item',version)]);
}
await writeFile(new URL('registry.json',out),JSON.stringify(registry,null,2));
