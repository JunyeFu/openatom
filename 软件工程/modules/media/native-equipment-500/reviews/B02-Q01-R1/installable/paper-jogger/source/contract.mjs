import * as T from 'three';
export const version='1.1.1';
const ref=id=>({id,version});
const vector=a=>({x:a[0],y:a[1],z:a[2]});
const decode=b=>JSON.parse(b.subarray(20,20+b.readUInt32LE(12)).toString());
const pose=n=>({translation:vector(n.translation??[0,0,0]),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:vector(n.scale??[1,1,1]),unit:'meter',coordinate_frame:'right_handed_y_up'});
const duration=value=>({value,unit:'millisecond',basis:'module_declared',source_ref:null});
const quantity={value:{coefficient:1,decimal_places:0},unit:'piece'};
const auxiliary=spec=>!!spec.auxiliary;
const rights=assets=>assets.map(a=>({asset_ref:a.id,source:'Original SF Native500 parametric machine geometry and original micro-surfaces; editable source supplied. Declared illustrative dimensions, no manufacturer CAD.',license:'CC0-1.0',redistribution_confirmed:true,missing_reason:null}));
export const definition=(id,title,category,data,assets=[])=>({id,version,title,definition_state:'runtime_candidate',contribution:{author:'Super Factory original equipment modeling',license:'CC0-1.0',assets:rights(assets)},parameters:[],attribute_schema:[],payload:{category,data}});
export const pack=(definition,files=[],dependencies=[],assets=[])=>({schema_version:'1.0',data:{definition,files:files.map(path=>({path,kind:'asset'})),dependencies,assets,rights:rights(assets),issues:[]}});
export async function embedLabel(binary,png){
 const b=Buffer.from(binary),j=decode(b),offset=20+b.readUInt32LE(12),original=b.subarray(offset+8,offset+8+b.readUInt32LE(offset));
 const bin=Buffer.concat([original,png,Buffer.alloc((4-png.length%4)%4)]);j.bufferViews??=[];const bv=j.bufferViews.push({buffer:0,byteOffset:original.length,byteLength:png.length})-1;
 j.images??=[];j.textures??=[];const im=j.images.push({bufferView:bv,mimeType:'image/png',name:'Original equipment identification'})-1;
 const tex=j.textures.push({source:im})-1;for(const m of j.materials??[])if(m.name==='LABEL_ATLAS')m.pbrMetallicRoughness.baseColorTexture={index:tex};
 j.buffers[0].byteLength=bin.length;const raw=Buffer.from(JSON.stringify(j)),chunk=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),out=Buffer.alloc(28+chunk.length+bin.length);
 out.writeUInt32LE(0x46546c67);out.writeUInt32LE(2,4);out.writeUInt32LE(out.length,8);out.writeUInt32LE(chunk.length,12);out.writeUInt32LE(0x4e4f534a,16);chunk.copy(out,20);out.writeUInt32LE(bin.length,20+chunk.length);out.writeUInt32LE(0x004e4942,24+chunk.length);bin.copy(out,28+chunk.length);return out;
}
export function presentation(spec,j,asset,built,simple){
 const indices=new Map(j.nodes.map((n,i)=>[n.name,i])),anchor=name=>{if(!indices.has(name))throw Error(spec.id+' missing anchor '+name);return{kind:'scene_node',data:indices.get(name)};};
 const ports=Object.fromEntries(['input','output',...(spec.rule==='inspect'?['rejected']:[]),...(spec.rule==='pack'?['container']:[]),...(spec.rule==='assemble'?['component']:[]),...(auxiliary(spec)?['auxiliary']:[])].map(id=>[id,anchor('Port_'+id)]));
 return {kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},
  nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.5,texture:null}})),
  poses:[],actions:[],timeline:j.animations.map((a,i)=>({id:spec.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),
  tags:[{id:'identity',label:spec.title,anchor:{kind:'viewport',data:{x:.18,y:.1}},progress_range:null,payload:{kind:'annotation',data:spec.basis}}],
  anchor_roles:{...ports,operator:anchor('OperatorSeat'),cargo:anchor(built.cargo)},port_anchors:ports,
  lod:{detailed:null,simplified:simple?{kind:'asset',data:asset}:{kind:'media_ref',data:ref(spec.media_id+'.simple')},region:'device_icons'},production_effect:'none'};
}
export function device(spec){
 const role=(name)=>({role:name,port:name,quantity,required_quality:[],declared_output_quality:null});
 const policy={transform:'transform_with_origin',preserve:'preserve',inspect:'preserve',assemble:'assembly_with_composition',rework:'preserve',pack:'container_with_contents'}[spec.rule];
 let inputs=[role('input')],outputs=[role('output')];
 if(spec.rule==='assemble')inputs.push(role('component'));
 if(spec.rule==='pack')inputs.push(role('container'));
 if(auxiliary(spec))inputs.push(role('auxiliary'));
 if(spec.rule==='inspect')outputs.push(role('rejected'));
 const ports=[...inputs.map(v=>({id:v.port,direction:'input',payload:{kind:'material',data:{item_role:v.role,unit:'piece',quantity_rule:null}}})),...outputs.map(v=>({id:v.port,direction:'output',payload:{kind:'material',data:{item_role:v.role,unit:'piece',quantity_rule:null}}}))];
 const total=spec.cycle_seconds*1000, times=[Math.round(total*.2),Math.round(total*.6),Math.round(total*.2)];
 const operation={builtin:{preserve:'transform',inspect:'inspect_and_route',assemble:'assemble',rework:'rework',pack:'pack',transform:'transform'}[spec.rule],item_roles:[...inputs,...outputs].map(v=>v.role),inputs,outputs,resource_roles:[],duration:null,
  phases:['load','auto','unload'].map((id,i)=>({id,duration:duration(times[i]),phase_resources:[],interruption_policy:'pause_resume',work_kind:i===0?'loading':i===2?'unloading':spec.rule==='inspect'?'inspection':spec.rule==='pack'?'packing':'processing'})),
  hold_on_output_wait:[],quality_and_routes:spec.rule==='inspect'?{source_kind:'preset',result_ref:null,max_completed_reworks:1,pass_port:'output',fail_port:'rejected',pending_policy:'hold_station',pending_location_role:null,isolation_location_role:null,release_required:true}:null,identity_policy:policy,family:spec.id};
 if(spec.rule==='preserve')operation.material_flow={primary_role:'input',auxiliary_roles:auxiliary(spec)?['auxiliary']:[],acquire_after_phase:null,consume_after_phase:auxiliary(spec)?'auto':null,reserve_before_phase:'auto',deliver_after_phase:'unload',required_primary_attributes:{},attributes_after_consume:{}};
 if(spec.rule==='pack')operation.pack_identity={output_role:'output',container_material_roles:['container'],content_roles:['input']};
 const media={kind:'model',source:{kind:'media_ref',data:{id:spec.media_id,version:spec.media_version??version}},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:[],poses:[],actions:[],timeline:[],tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'media_ref',data:{id:spec.media_id+'.simple',version:spec.media_version??version}},region:'device_icons'},production_effect:'none'};
 return definition(spec.definition_id,spec.title,'devices',{placement:'fixed_during_simulation',ports,behavior:{selected:'inline',inline:operation,operation_ref:null},configuration:{},presentation:media,operations:['configure','inspect','preview_cycle']});
}
