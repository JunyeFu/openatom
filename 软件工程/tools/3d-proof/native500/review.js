import * as T from 'three';
import {OrbitControls} from 'three/addons/controls/OrbitControls.js';
import {RoomEnvironment} from 'three/addons/environments/RoomEnvironment.js';
import {decodeGlb} from '../../../packages/presentation/src/glb.ts';
import {bindScene} from '../../../packages/presentation/src/scene.ts';
import {assetKey} from '../../../packages/presentation/src/types.ts';
import {capturePlan} from './capture-policy.mjs';
const review=new URLSearchParams(location.search).get('review');
const $=id=>document.getElementById(id),base='/modules/media/native-equipment-500/'+(review?'reviews/'+review+'/':'');
const inventory=await fetch(base+'inventory.json').then(r=>r.json());
if(!review){
 const accepted=await fetch(base+'accepted-registration.json').then(r=>r.json());
 const acceptedMediaRefs=new Set(accepted.register_order.map(e=>e.id+'@'+e.version));
 for(const file of accepted.inventory_paths??[]){const batch=await fetch(base+file).then(r=>r.json()),prefix=file.slice(0,file.lastIndexOf('/')+1);for(const entry of batch.entries){if(!acceptedMediaRefs.has(entry.module_id+'@'+entry.version))continue;const index=inventory.entries.findIndex(e=>e.id===entry.id&&e.suffix===entry.suffix);inventory.entries[index]={...entry,directory:prefix+entry.directory};}}
}
inventory.batches??=[{id:'B02',title:review+' 纸包装逐机复审'}];
const detailed=inventory.entries.filter(e=>e.suffix==='detailed');
for(const b of inventory.batches)$('batch').add(new Option(b.id+' '+b.title,b.id));
$('batch').value=review?.startsWith('B01')?'B01':'B02';
for(const e of detailed)$('model').add(new Option(`${String(e.index).padStart(2,'0')} ${e.title}`,e.id));
const renderer=new T.WebGLRenderer({antialias:true,preserveDrawingBuffer:true,powerPreference:'high-performance'});renderer.setPixelRatio(1);renderer.setSize(1600,1000,false);renderer.outputColorSpace=T.SRGBColorSpace;renderer.toneMapping=T.ACESFilmicToneMapping;renderer.toneMappingExposure=.85;renderer.shadowMap.enabled=true;renderer.shadowMap.type=T.PCFShadowMap;$('viewport').append(renderer.domElement);
const scene=new T.Scene();scene.background=new T.Color('#e4e9ee');const environment=new T.PMREMGenerator(renderer).fromScene(new RoomEnvironment(),.04);scene.environment=environment.texture;scene.environmentIntensity=.75;scene.add(new T.HemisphereLight(0xf2f7ff,0x77848d,1.15));
const light=new T.DirectionalLight('#ffffff',2.3);light.position.set(5,9,5);light.castShadow=true;light.shadow.mapSize.set(2048,2048);light.shadow.normalBias=.001;Object.assign(light.shadow.camera,{left:-5,right:5,top:5,bottom:-5,near:.1,far:40});light.shadow.camera.updateProjectionMatrix();scene.add(light,light.target);
const floor=new T.Mesh(new T.PlaneGeometry(100,100),new T.MeshStandardMaterial({color:'#cfd8df',roughness:.8}));floor.rotation.x=-Math.PI/2;floor.position.y=-.002;floor.receiveShadow=true;scene.add(floor);
const grid=new T.GridHelper(40,80,0xb0bdc6,0xc0ccd4);grid.position.y=-.001;scene.add(grid);
const camera=new T.PerspectiveCamera(36,1.6,.02,150),controls=new OrbitControls(camera,renderer.domElement);controls.maxPolarAngle=Math.PI;
let rig,decoded,current,playing=false,busy=false,zero=0,bounds=new T.Box3(),center=new T.Vector3(),size=new T.Vector3();
const status=s=>$('status').textContent=s;
function pose(phase,u){const clip=current.presentation.timeline.find(c=>c.phase_ref===phase);if(!clip)throw Error(current.id+' missing '+phase);rig.apply({known:true,display_phase_id:phase,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});}
function view(which='iso'){
 $('view').value=which;const dirs={iso:[1,.7,1],front:[0,.03,1],back:[0,.03,-1],left:[-1,.03,0],right:[1,.03,0],top:[.001,1,.001],bottom:[.001,-1,.001],closeup:[1,.65,1]};
 if(which!=='closeup'){bounds.setFromObject(rig.root);bounds.getCenter(center);bounds.getSize(size);}
 let target=center.clone(),extent=Math.max(...size.toArray())/2;
 if(which==='closeup'){
  const support=new Set(['unwind','rewind','feed','guide','clamp','tubeclamp','bookclamp','vacuumtable','collet','vblock','granitetable','magnettable','suction','hopper','foil','dualunwind','wirefeed','traymag','feederbank']);
  const pick=current.mechanisms?.findLastIndex(t=>!support.has(t));
  const station=pick>=0?'Station_'+String(pick).padStart(2,'0')+'_'+current.mechanisms[pick]:current.focus_node;
  const focus=rig.root.getObjectByName(station)??rig.root.getObjectByName(current.focus_node);
  if(focus){const fb=new T.Box3().setFromObject(focus);fb.getCenter(target);extent=Math.max(.28,...fb.getSize(new T.Vector3()).toArray().map(v=>v*.52));}
 }
 const direction=new T.Vector3(...dirs[which]).normalize();let distance=extent/Math.sin(T.MathUtils.degToRad(camera.fov/2))*1.25;
 if(which!=='closeup'){
  const right=new T.Vector3().crossVectors(camera.up,direction).normalize(),up=new T.Vector3().crossVectors(direction,right).normalize();
  const tanV=Math.tan(T.MathUtils.degToRad(camera.fov/2))*.78,tanH=Math.tan(T.MathUtils.degToRad(camera.fov/2))*camera.aspect*.93;distance=0;
  for(const x of [bounds.min.x,bounds.max.x])for(const y of [bounds.min.y,bounds.max.y])for(const z of [bounds.min.z,bounds.max.z]){const d=new T.Vector3(x,y,z).sub(target);distance=Math.max(distance,d.dot(direction)+Math.max(Math.abs(d.dot(up))/tanV,Math.abs(d.dot(right))/tanH));}
 }
 controls.target.copy(target);camera.position.copy(target).addScaledVector(direction,distance);camera.lookAt(target);floor.visible=which!=='bottom';grid.visible=which!=='bottom';draw();
}
function draw(){rig?.root.updateMatrixWorld(true);renderer.render(scene,camera);}
async function load(id,lod){
 playing=false;$('play').textContent='播放声明动作';if(rig){scene.remove(rig.root);rig.dispose();decoded.dispose();}
 current={...inventory.entries.find(e=>e.id===id&&e.suffix===lod)};current.presentation=(await fetch(base+current.directory+'/module.json').then(r=>r.json())).data.definition.payload.data;const response=await fetch(base+current.directory+'/'+current.asset.path);if(!response.ok)throw Error('Missing '+current.asset.path);const bytes=await response.arrayBuffer();
 const selected=$('phase').value;$('phase').replaceChildren(...current.phase_ids.map(id=>new Option(id,id)));$('phase').value=current.phase_ids.includes(selected)?selected:current.primary_phase;
 decoded=await decodeGlb(bytes);rig=await bindScene(current.presentation,async()=>bytes,{assetKey:assetKey(current.asset),decoded});scene.add(rig.root);
 rig.root.traverse(o=>{if(o.isMesh){o.castShadow=true;o.receiveShadow=true;}});bounds.setFromObject(rig.root);bounds.getCenter(center);bounds.getSize(size);
 if(size.toArray().some(x=>!Number.isFinite(x)||x<=0))throw Error('Invalid geometry bounds '+id);
 pose($('phase').value,Number($('progress').value)/100);view($('view').value);
 $('details').textContent=`${current.index} / 500 · ${current.title} · ${lod}\n${typeof current.flow==='object'?current.flow.input+' → '+current.flow.output:current.flow}\n定义 ${current.definition_id}@${current.version} · ${current.rule} · 声明周期${current.cycle_seconds}s\n${current.nodes}节点 / ${current.triangles.toLocaleString()}三角面 / ${(current.bytes/1048576).toFixed(2)}MiB · ${current.basis}`;
}
const controlsChanged=async()=>{if(busy)return;busy=true;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=true);try{await load($('model').value,$('lod').value);status('已读取实际GLB；可旋转、查看背底和动作。');}catch(e){status(String(e));console.error(e);}finally{busy=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=false);}};
$('model').onchange=controlsChanged;$('lod').onchange=controlsChanged;$('view').onchange=()=>view($('view').value);
function seek(){if(busy)return;playing=false;pose($('phase').value,Number($('progress').value)/100);if($('view').value==='closeup')view('closeup');else draw();}
$('phase').onchange=seek;$('progress').oninput=seek;$('play').onclick=()=>{playing=!playing;zero=performance.now()-Number($('progress').value)*40;$('play').textContent=playing?'暂停动作':'播放声明动作';};
function animate(t){requestAnimationFrame(animate);if(busy)return;if(playing){const u=((t-zero)%4000)/4000;$('progress').value=String(Math.round(u*100));pose($('phase').value,u);}controls.update();if(rig)draw();}requestAnimationFrame(animate);
const save=async(name,data)=>{const r=await fetch('/native500-save/'+(review?review.toLowerCase()+'-':'')+name,{method:'POST',body:data});if(!r.ok)throw Error('Capture write failed '+name);};
function signature(){return [...rig.nodes].map(([id,o])=>[id,...o.position.toArray(),...o.quaternion.toArray(),...o.scale.toArray()]);}
function close(a,b){return a.every((row,i)=>row.every((v,j)=>Math.abs(v-b[i][j])<1e-7));}
async function picture(name,title){
 draw();const canvas=document.createElement('canvas');canvas.width=1600;canvas.height=1000;const ctx=canvas.getContext('2d');ctx.drawImage(renderer.domElement,0,0);ctx.fillStyle='rgba(19,42,57,.90)';ctx.fillRect(20,20,1560,86);ctx.fillRect(20,936,1560,44);ctx.fillStyle='#fff';ctx.font='bold 26px "Microsoft YaHei",sans-serif';ctx.fillText(`${String(current.index).padStart(2,'0')}  ${current.title}  ·  ${title}`,40,57);ctx.font='18px "Microsoft YaHei",sans-serif';ctx.fillStyle='#bfd1df';ctx.fillText(`${current.definition_id}@${current.version}  |  ${current.suffix}`,40,88);ctx.fillText('实际GLB重开 · 当前产品解码与场景绑定 · 原创示例尺寸/工时 · 声明预览，不产生业务结果',40,964);await save(name,await new Promise(resolve=>canvas.toBlob(resolve,'image/png')));
}
$('capture').onclick=async()=>{
 if(busy)return;busy=true;playing=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=true);const checks=[];
 try{
  const query=new URLSearchParams(location.search),devices=query.get('devices')?.split(',').filter(Boolean)??[];
  const {entries:selected,closeupProgress}=capturePlan(inventory.entries,$('batch').value,devices,query.get('capture')!=='lean');
  for(const entry of selected){
   $('model').value=entry.id;$('lod').value=entry.suffix;$('phase').value='auto';$('progress').value='0';
   status(`实际读取 ${checks.length+1}/${selected.length} · ${entry.title} / ${entry.suffix}`);await load(entry.id,entry.suffix);
   const row={id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.media_version??entry.version},bytes:entry.bytes,nodes:entry.nodes,phase_ids:entry.phase_ids,decode:true,phase_seek:true,reverse_seek:true,moving_nodes:[],captures:[]};
   const motions=new Set();for(const phase of entry.phase_ids){
    pose(phase,0);const a=signature();pose(phase,.53125);const b=signature();pose(phase,1);pose(phase,.53125);if(!close(b,signature()))throw Error(entry.id+' reverse seek mismatch');pose(phase,0);if(!close(a,signature()))throw Error(entry.id+' return-to-start mismatch');
    for(let i=0;i<a.length;i++)if(!close([a[i]],[b[i]]))motions.add(a[i][0]);
   }
   if(!motions.size)throw Error(entry.id+' has no actual part motion');row.moving_nodes=[...motions];
   for(const direction of ['iso','front','back','left','right','top','bottom']){pose(entry.primary_phase,.5);view(direction);const name=entry.id+'/'+entry.suffix+'-'+direction+'.png';await picture(name,direction);row.captures.push(name);}
   if(entry.suffix==='detailed')for(const u of closeupProgress){pose(entry.primary_phase,u);view('closeup');const name=entry.id+'/action-'+Math.round(u*100)+'.png';await picture(name,'机构近景 · '+entry.primary_phase+' '+Math.round(u*100)+'%');row.captures.push(name);}
   checks.push(row);await save('progress.json',JSON.stringify({completed:checks.length,total:selected.length,current:entry.id}));
  }
  const selection=devices.length?'-selected-'+devices.join('-'):'';
  await save('render-'+$('batch').value.toLowerCase()+selection+'.json',JSON.stringify({scope:'actual_export_GLB_decoded_and_bound_by_product_presentation_declared_preview',batch:$('batch').value,device_count:new Set(selected.map(e=>e.id)).size,lod_models:checks.length,checks,native_actual_run_wait_save_reopen:'not_executed_by_asset_reviewer'},null,2));status(`完成：${$('batch').value} / ${selected.length}个GLB / ${checks.reduce((n,r)=>n+r.captures.length,0)}张实拍，各阶段与反向seek通过。`);
 }catch(e){console.error(e);await save('render-error.json',JSON.stringify({completed:checks.length,error:String(e)}));status('检查失败：'+String(e));}
 finally{busy=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=false);view('iso');}
};
$('batch').onchange=()=>{const e=detailed.find(e=>e.batch===$('batch').value);$('model').value=e.id;controlsChanged();};
$('model').value=detailed.find(e=>e.batch===$('batch').value).id;
const allButton=document.createElement('button');allButton.textContent='逐批检查全部1000个GLB';document.querySelector('header').append(allButton);
allButton.onclick=async()=>{for(const b of inventory.batches){$('batch').value=b.id;await $('capture').onclick();if($('status').textContent.startsWith('检查失败'))break;}};
const repairedButton=document.createElement('button');repairedButton.textContent='复核修正的四批模型';document.querySelector('header').append(repairedButton);
repairedButton.onclick=async()=>{for(const id of ['B02','B03','B04','B10']){$('batch').value=id;await $('capture').onclick();if($('status').textContent.startsWith('检查失败'))break;}};
const closeButton=document.createElement('button');closeButton.textContent='更新关键机构近景';document.querySelector('header').append(closeButton);
closeButton.onclick=async()=>{if(busy)return;busy=true;playing=false;try{for(const entry of inventory.entries.filter(e=>e.suffix==='detailed')){status('关键机构近景 '+entry.index+'/500 · '+entry.title);await load(entry.id,entry.suffix);for(const u of [0,.5,1]){pose(entry.primary_phase,u);view('closeup');await picture(entry.id+'/action-'+Math.round(u*100)+'.png','关键机构 · '+entry.primary_phase+' '+Math.round(u*100)+'%');}}status('500种设备关键机构近景已更新。');}catch(e){status('检查失败：'+e);}finally{busy=false;view('iso');}};
if(review){
 const footButton=document.createElement('button');footButton.textContent='逐机复核支腿接地';document.querySelector('header').append(footButton);
 footButton.onclick=async()=>{
  if(busy)return;busy=true;playing=false;const checks=[];document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=true);
  try{
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);pose('load',0);const contacts=[];
    const part=name=>{const declared=current.presentation.nodes.find(node=>node.name===name);return declared?rig.nodes.get(declared.id):null;};
    const assemblies=[];
    if(review==='B02-Q02'){
     for(const node of current.presentation.nodes.filter(n=>n.name.endsWith('Top')&&part(n.name.slice(0,-3)+'Leg-0.3_-0.28'))){
      const name=node.name.slice(0,-3);for(const dx of [-.30,.30])for(const z of [-.28,.28])assemblies.push({name:name+'/'+dx+'/'+z,foot:part(name+'Foot'+dx+'_'+z),leg:part(name+'Leg'+dx+'_'+z),table:part(node.name)});
     }
    }else if(review==='B02-Q03'||review==='B02-Q03-R1'||review==='B02-Q04'||review?.startsWith('B01-')){
     for(const node of current.presentation.nodes.filter(n=>/^(InputTable|OutputTable|InkSupplyTable|AuxSupplyTable|SpentBackingTable|RearWasteSupport|FrontWasteSupport|ScrapBinTable|ClearingBase)Leg/.test(n.name))){const prefix=node.name.split('Leg')[0];assemblies.push({name:node.name,foot:part(node.name.replace('Leg','Foot')),leg:part(node.name),table:part(prefix+'Top')});}
    }else for(const sign of [-1,1])for(const z of [-.28,.28])assemblies.push({name:sign+'/'+z,foot:part('TransferRubberFoot_'+sign+'_'+z),leg:part('TransferLeg_'+sign+'_'+z),table:part('PaperTransferTable_'+sign)});
    if(!assemblies.length)throw Error(entry.id+' has no declared table-foot assemblies');
    for(const {name,foot,leg,table}of assemblies){
     if(!foot||!leg||!table)throw Error(entry.id+' missing declared contact assembly '+name);
     const a=new T.Box3().setFromObject(leg),b=new T.Box3().setFromObject(foot),c=new T.Box3().setFromObject(table),bottom=a.min.y-b.max.y,top=c.min.y-a.max.y;
     if(Math.abs(bottom)>1e-6||Math.abs(top)>1e-6)throw Error(entry.id+' '+entry.suffix+' '+name+' contact gap: bottom='+bottom+'m; top='+top+'m');
     contacts.push({name,foot_top:b.max.y,leg_bottom:a.min.y,leg_top:a.max.y,table_bottom:c.min.y,bottom_gap_m:bottom,top_gap_m:top});
    }
    const target=assemblies[0].foot.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,.04,0));
    controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.18,.10,.22));camera.lookAt(target);floor.visible=grid.visible=true;
    const file=entry.id+'/'+entry.suffix+'-foot-contact.png';await picture(file,'实际GLB支腿／脚垫接触');checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},contacts,capture:file});
   }
   await save('foot-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status(checks.length+'个实际GLB：支腿下端／脚垫和上端／台底接触通过。');
  }catch(e){await save('foot-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)}));status('接地检查失败：'+e);}
  finally{busy=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=false);}
 };
}
if(review==='B02-Q02'){
 const button=document.createElement('button');button.textContent='连续纸幅接触复核';document.querySelector('header').append(button);
 button.onclick=async()=>{
  if(busy)return;busy=true;playing=false;const checks=[];document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=true);
  let paperRig;
  try{
   for(const id of ['paper-unwinder','paper-rewinder'])for(const lod of ['detailed','simple']){
    await load(id,lod);const dir=base+'items/'+id+'-input/',presentation=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());
    const paper=await decodeGlb(bytes);paperRig=await bindScene(presentation,async()=>bytes,{assetKey:assetKey(presentation.source.data),decoded:paper});
    const cargo=rig.nodes.get(current.presentation.anchor_roles.cargo.data);cargo.add(paperRig.root);
    const part=(p,r,name)=>{const n=p.nodes.find(n=>n.name===name);if(!n||!r.nodes.has(n.id))throw Error('Missing actual GLB node '+name);return r.nodes.get(n.id);};
    const near=(a,b,what)=>{if(Math.abs(a-b)>2e-6)throw Error(id+' '+lod+' '+what+': '+a+' != '+b);};
    const phase=id==='paper-unwinder'?'rolled:auto':'unwound:auto',clip=presentation.timeline.find(c=>c.phase_ref===phase),samples=[];
    for(const u of [.001,1/60,.1175,.25,.501,29/60,.75,.999,.25,.001]){
     pose('auto',u);paperRig.apply({known:true,display_phase_id:phase,progress:u,terminal_pose:false,effective_ms:u*1000,duration_ms:1000},{clip,progress:u});rig.root.updateMatrixWorld(true);
     const bounds=o=>new T.Box3().setFromObject(o),web=bounds(part(presentation,paperRig,id==='paper-unwinder'?'HeldPaperWeb3':'HeldPaperWeb0')),left=bounds(part(current.presentation,rig,'FoldGuideLip-1')),right=bounds(part(current.presentation,rig,'FoldGuideLip1'));
     near(web.min.x,left.max.x,'left paper/slot face');near(web.max.x,right.min.x,'right paper/slot face');near(web.max.y,left.max.y,'paper/slot entry');
     const amount=(id==='paper-unwinder'?u:1-u)*60,i=Math.max(0,Math.ceil(amount)-1),layer=bounds(part(presentation,paperRig,'HeldBatchLayer'+i));
     near(web.min.y,(layer.min.y+layer.max.y)/2,'web/live layer centerline');near((web.min.x+web.max.x)/2,i%2?layer.min.x:layer.max.x,'paper/live folding endpoint');
     samples.push({progress:u,left_gap_m:web.min.x-left.max.x,right_gap_m:right.min.x-web.max.x,layer_gap_m:web.min.y-(layer.min.y+layer.max.y)/2});
     if([.1175,.501,.75].includes(u)){
      const target=new T.Vector3((web.min.x+web.max.x)/2,1.04,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.85,.50,.85));camera.lookAt(target);await picture(id+'/'+lod+'-fold-'+Math.round(u*1000)+'.png','连续纸幅/导槽 '+Math.round(u*100)+'%');
     }
    }
    checks.push({id,lod,module_ref:{id:current.module_id,version:current.version},samples,captures:3,passed:true});cargo.remove(paperRig.root);paperRig.dispose();paperRig=null;
   }
   await save('fold-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status('4个实际GLB：连续纸幅、导槽和活动折边40个正反向接触时点通过。');
  }catch(e){await save('fold-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)}));status('连续纸幅检查失败：'+e);}
  finally{paperRig?.dispose();busy=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=false);}
 };
}
await controlsChanged();
if(review==='B02-Q03'||review==='B02-Q03-R1'){
 const button=document.createElement('button');button.textContent='纸板与边料接触复核';document.querySelector('header').append(button);
 button.onclick=async()=>{
  if(busy)return;busy=true;playing=false;const checks=[];let paperRig,inkRig,paperDecoded,inkDecoded;
  document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=true);
  const {specs,scrapPieces,scrapStage,scrapOffset}=await import(base+'source/batch-spec.mjs');
  const near=(a,b,label)=>{if(Math.abs(a-b)>3e-6)throw Error(label+': '+a+' != '+b);};
  const part=(p,r,name)=>{const n=p.nodes.find(n=>n.name===name);if(!n||!r.nodes.has(n.id))throw Error('Missing actual GLB node '+name);return r.nodes.get(n.id);};
  try{
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());
    paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    let inkP;if(s.printed){const d=base+'items/'+entry.id+'-ink/';inkP=(await fetch(d+'module.json').then(r=>r.json())).data.definition.payload.data;const data=await fetch(d+'paper.glb').then(r=>r.arrayBuffer());inkDecoded=await decodeGlb(data);inkRig=await bindScene(inkP,async()=>data,{assetKey:assetKey(inkP.source.data),decoded:inkDecoded});rig.nodes.get(current.presentation.anchor_roles.ink_cup.data).add(inkRig.root);}
    const phase=s.initial+':auto',clip=p.timeline.find(c=>c.phase_ref===phase),samples=[];
    if(review==='B02-Q03-R1'){
     const u=.30/1.4;pose('auto',u);rig.root.updateMatrixWorld(true);const probes=[['PrintCylinder',.101,-1],['PrintImpressionRoll',.04,1]].map(([name,r,sign])=>{const o=part(current.presentation,rig,name),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0));return{name,o,contact,local:o.worldToLocal(contact.clone())};});const relief=new T.Box3().setFromObject(part(current.presentation,rig,'RaisedPrintPlate'),true);near(relief.min.y,s.surface+s.size[1]+s.ink_volume_m3/(.12*.10),'actual relief / actual full ink layer datum');pose('auto',u+.00001);rig.root.updateMatrixWorld(true);for(const a of probes){const velocity=(a.o.localToWorld(a.local).x-a.contact.x)/.00001;if(Math.abs(velocity-1.4)>.001)throw Error('actual print/belt nip opposes paper: '+a.name+' '+velocity);}
    }
    for(const u of [0,.17,.25,.4,.5,.6,.75,.90,.95,.98,1,.25,0]){
     pose('auto',u);paperRig.apply({known:true,display_phase_id:phase,progress:u,terminal_pose:false,effective_ms:u*1000,duration_ms:1000},{clip,progress:u});
     if(inkRig){const ic=inkP.timeline.find(c=>c.phase_ref==='ink_dose:auto');inkRig.apply({known:true,display_phase_id:'ink_dose:auto',progress:u,terminal_pose:false,effective_ms:u*1000,duration_ms:1000},{clip:ic,progress:u});}
     rig.root.updateMatrixWorld(true);const bounds=o=>new T.Box3().setFromObject(o);
     if(s.mode!=='crease')near(bounds(part(p,paperRig,'ActualCutSheet')).min.y,s.surface,entry.id+' sheet support datum');
     if(['slot','perforate'].includes(s.mode)){
      near(bounds(part(current.presentation,rig,'ClosedFeedBelt0')).max.y,s.surface,'input belt / paper');near(bounds(part(current.presentation,rig,'ClosedFeedBelt1')).max.y,s.surface,'output belt / paper');
      const axis= bounds(part(current.presentation,rig,'RelievedCounterAxis')),output=bounds(part(current.presentation,rig,'ClosedFeedBelt1'));
      for(const [i,piece]of scrapPieces(s).entries()){
       const box=bounds(part(p,paperRig,'AttachedCutPiece'+i));
       if(box.min.y<s.surface-1e-5&&u<1){if(box.min.x<axis.max.x-3e-6||box.max.x>output.min.x+3e-6)throw Error(entry.id+' scrap crosses shaft or output belt at '+u);}
       if(u===1){const off=scrapOffset(s,piece.role,i),stage=scrapStage(s);near(box.min.y,stage[1]+off[1],'actual scrap / collector');near((box.min.x+box.max.x)/2,stage[0]+off[0],'actual scrap X');near((box.min.z+box.max.z)/2,stage[2]+off[2],'actual scrap Z');}
      }
     }
     if(s.mode==='trim'&&u===1)for(const [i,piece]of scrapPieces(s).entries()){const box=bounds(part(p,paperRig,'AttachedCutPiece'+i));near(box.min.y,.90,'trim scrap / worktop');}
     if(s.mode==='crease'&&u===.5){
      const paperObject=part(p,paperRig,'CreasedUncutSheet'),points=[];paperObject.traverse(o=>{if(o.isMesh){const a=o.geometry.attributes.position;for(let i=0;i<a.count;i++){const v=new T.Vector3().fromBufferAttribute(a,i).applyMatrix4(o.matrixWorld);if(Math.abs(v.x+.125)<1e-6)points.push(v.y);}}});
      near(Math.max(...points),bounds(part(current.presentation,rig,'CreaseMaleDie-0.125')).min.y,'actual crease hinge / male die');
     }
     if(s.printed){const mark=bounds(part(p,paperRig,'ActualPrintedPanelMark')).getSize(new T.Vector3()),ink=bounds(part(inkP,inkRig,'ActualTwoMillilitreInkDose')).getSize(new T.Vector3()),volume=mark.x*mark.y*mark.z+Math.PI*(ink.x/2)**2*ink.y;if(Math.abs(volume-s.ink_volume_m3)>1e-10)throw Error('ink dose / mark and remainder: '+volume+' != '+s.ink_volume_m3);}
     samples.push({u,actual_glb_support_and_staging:true});
     if([0,.5,1].includes(u)&&samples.length<=11){view('iso');await picture(entry.id+'/'+entry.suffix+'-paper-contact-'+Math.round(u*100)+'.png','真实纸材与机构 · auto '+Math.round(u*100)+'%');}
    }
    checks.push({id:entry.id,lod:entry.suffix,samples,actual_nip_rolling_direction:review==='B02-Q03-R1'?'co_moving_verified':'not_covered_by_this_receipt'});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;if(inkRig){inkRig.root.removeFromParent();inkRig.dispose();inkDecoded.dispose();inkRig=inkDecoded=null;}
   }
   await save('paper-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks,Native:'separate acceptance'},null,2));status('10个实际GLB：纸材承托、边料落料间隙及暂存、压痕接触复核通过。');
  }catch(e){console.error(e);await save('paper-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)}));status('纸材接触检查失败：'+e);}
  finally{for(const r of [paperRig,inkRig])if(r){r.root.removeFromParent();r.dispose();}paperDecoded?.dispose();inkDecoded?.dispose();busy=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=false);}
 };
}
if(review==='B02-Q03'){
 const button=document.createElement('button');button.textContent='复核Native暂停输入与机构姿态';document.querySelector('header').append(button);
 button.onclick=async()=>{const rows=await fetch('/.work/native500-quality-20261025/q03/native-pause-inputs.json').then(r=>r.json()),{projectPhase}=await import('/packages/presentation/src/phase.ts'),checks=[];
 const visual=p=>Object.fromEntries(['known','display_phase_id','effective_ms','duration_ms','progress','terminal_pose'].map(k=>[k,p[k]]));
 for(const id of ['flexo-slotter','board-edge-trimmer']){
  await load(id,'detailed');const samples=rows.filter(r=>r.devices.some(d=>d.id===id+'--interrupted')),points=[],poses=[],items=[];
  for(const sample of samples){const frame=sample.devices.find(d=>d.id===id+'--interrupted').frame,point=projectPhase(frame);points.push(visual(point));const clip=current.presentation.timeline.find(c=>c.phase_ref===point.display_phase_id);rig.apply(point,{clip,progress:point.progress});rig.root.updateMatrixWorld(true);poses.push([...rig.nodes].map(([i,o])=>[i,...o.matrixWorld.elements]));items.push(sample.items.filter(i=>i.holder.kind==='job'&&i.frame?.job_id===frame.job_id).map(i=>({id:i.id,attributes:i.attributes,holder:i.holder,attachment:i.attachment,point:visual(projectPhase(i.frame))})));}
  if(points.some(p=>JSON.stringify(p)!==JSON.stringify(points[0])))throw Error(id+' actual Native consumer phase differs');
  if(poses.some(p=>JSON.stringify(p)!==JSON.stringify(poses[0])))throw Error(id+' actual GLB world matrices differ');
  if(items.some(p=>JSON.stringify(p)!==JSON.stringify(items[0])))throw Error(id+' actual Native held-item pose inputs differ');
  checks.push({id,samples:samples.length,point:points[0],actual_Native_source:true,exact_device_matrices:true,exact_held_item_phase_and_attachment_inputs:true});
 }
 await save('native-pause-pose-checks.json',JSON.stringify({passed:true,checks,scope:'Readonly actual Native React render inputs; current product phase reader and GLB decoder. Pixel equality is a separate raster diagnostic.'},null,2));status('实际Native暂停输入和设备/物品机构姿态一致。');};
}
if(['B01-Q02','B01-Q02-R1','B01-Q03'].includes(review)){
 const button=document.createElement('button');button.textContent=review==='B01-Q03'?'胶印纸材与油墨复核':'双色独立油墨与实际纸材复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;let paperRig,paperDecoded;const inks=[],checks=[];
  try{
   const{specs,seatX,fraction}=await import('/modules/media/native-equipment-500/reviews/'+review+'/source/batch-spec.mjs');
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    for(const c of s.colors){const dir=base+'items/'+entry.id+'-'+c.role+'/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,data=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer()),decoded=await decodeGlb(data),r=await bindScene(p,async()=>data,{assetKey:assetKey(p.source.data),decoded});rig.nodes.get(current.presentation.anchor_roles[c.role==='auxiliary'?'aux_seat':'aux_red_seat'].data).add(r.root);inks.push({c,p,r,decoded});}
    const object=(p,r,name)=>{const n=p.nodes.find(n=>n.name===name);if(!n)throw Error('missing '+name);return r.nodes.get(n.id);},bounds=o=>new T.Box3().setFromObject(o,true),near=(a,b,name)=>{if(Math.abs(a-b)>3e-6)throw Error(name+': '+a+' vs '+b);},samples=[];
    if(review==='B01-Q03'){
     pose('auto',.4);rig.root.updateMatrixWorld(true);const probes=[['BlanketCylinder',.08,-1],['ImpressionCylinder',.06,1]].map(([name,r,sign])=>{const n=current.presentation.nodes.find(n=>n.name===name),o=rig.nodes.get(n.id),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0));return{name,o,contact,local:o.worldToLocal(contact.clone())};});pose('auto',.40001);rig.root.updateMatrixWorld(true);for(const a of probes){const velocity=(a.o.localToWorld(a.local).x-a.contact.x)/.00001;if(Math.abs(velocity-2)>.001)throw Error('actual GLB nip surface opposes paper: '+a.name+' '+velocity);}
    }
    if(review==='B01-Q02-R1'){
     for(const c of s.colors){const u=(c.x+1)/2;pose('auto',u);rig.root.updateMatrixWorld(true);const named=name=>{const n=current.presentation.nodes.find(n=>n.name===name);return rig.nodes.get(n.id);},probes=[['PlateCylinder_'+c.name,.08,-1],['Impression_'+c.name,.06,1]].map(([name,r,sign])=>{const o=named(name),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0));return{name,o,contact,local:o.worldToLocal(contact.clone())};});near(bounds(named('ReliefPlate_'+c.name)).min.y,s.surface+s.size[1]+.000020,'actual relief / ink film');pose('auto',u+.00001);rig.root.updateMatrixWorld(true);for(const a of probes){const velocity=(a.o.localToWorld(a.local).x-a.contact.x)/.00001;if(Math.abs(velocity-2)>.001)throw Error('actual GLB print nip opposes paper: '+a.name+' '+velocity);}}
    }
    for(const u of [0,.1,.25,.275,.30,.325,.4,.5,.6,.675,.70,.725,.8,1,.5,0]){
     pose('auto',u);for(const[media,bound,phase]of [[p,paperRig,'flat:auto'],...inks.map(i=>[i.p,i.r,'dose_ready:auto'])]){const clip=media.timeline.find(c=>c.phase_ref===phase);bound.apply({known:true,display_phase_id:phase,progress:u,terminal_pose:false,effective_ms:u*1000,duration_ms:1000},{clip,progress:u});}
     rig.root.updateMatrixWorld(true);const paper=bounds(object(p,paperRig,'ActualPaperSubstrate'));near(paper.min.y,s.surface,'actual paper feed height');near((paper.min.x+paper.max.x)/2,seatX(s,'auto',u),'actual paper feed X');
     for(const{c,p:ip,r}of inks){const mark=bounds(object(p,paperRig,'ActualPrint_'+c.name)),liquid=bounds(object(ip,r,'ActualInkDose_'+c.name)),markVolume=(mark.max.x-mark.min.x)*(mark.max.y-mark.min.y)*(mark.max.z-mark.min.z),inkVolume=Math.PI*.020*.020*(liquid.max.y-liquid.min.y);if(Math.abs(markVolume+inkVolume-c.volume)>c.volume*1e-4)throw Error('independent '+c.name+' ink volume not conserved');near(mark.max.x-mark.min.x,.1*fraction(c,u),'ink mark progress '+c.name);}
     samples.push({u,actual_sheet_and_ink_volumes:true,independent_ink_count:s.colors.length});if([0,.30,.50,.70,1].includes(u)&&samples.length<=14){view('iso');await picture(entry.id+'/'+entry.suffix+'-two-ink-'+Math.round(u*100)+'.png',(review==='B01-Q03'?'实际胶印纸材与油墨 · ':'实际纸材与两种独立油墨 · ')+Math.round(u*100)+'%');}
    }
    checks.push({id:entry.id,lod:entry.suffix,samples,actual_nip_rolling_direction:['B01-Q03','B01-Q02-R1'].includes(review)?'co_moving_verified':'not_covered_by_this_receipt'});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;for(const i of inks){i.r.root.removeFromParent();i.r.dispose();i.decoded.dispose();}inks.length=0;
   }
   await save('two-ink-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status(review==='B01-Q03'?'2个实际GLB：胶印纸材位移与独立油墨转移通过。':'2个实际GLB：纸材位移与蓝/红两种独立油墨转移通过。');
  }catch(e){console.error(e);await save('two-ink-contact-error.json',JSON.stringify({passed:false,error:String(e),checks},null,2));status('双色复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();for(const i of inks){i.r.root.removeFromParent();i.r.dispose();i.decoded.dispose();}busy=false;}
 };
}
if(review==='B01-Q01'){
 const button=document.createElement('button');button.textContent='送纸与举升实际纸材接触复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let paperRig,paperDecoded;
  try{
   const{specs,paperPosition}=await import('/modules/media/native-equipment-500/reviews/B01-Q01/source/batch-spec.mjs');
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    const part=name=>{const n=current.presentation.nodes.find(n=>n.name===name);if(!n)throw Error('missing '+name);return rig.nodes.get(n.id);},box=o=>new T.Box3().setFromObject(o,true),near=(a,b,name)=>{if(Math.abs(a-b)>3e-6)throw Error(entry.id+' '+name+': '+a+' vs '+b);},samples=[];
    for(const phase of ['load','auto','unload'])for(const u of [0,.12,.30,.45,.50,.55,.60,.65,.75,.90,1,.50,0]){
     pose(phase,u);rig.root.updateMatrixWorld(true);const paper=box(paperRig.root),expected=paperPosition(s,phase,u);near(paper.min.y,expected[1],'paper support datum');near((paper.min.x+paper.max.x)/2,expected[0],'paper X');near(paper.max.y-paper.min.y,s.size[1],'paper thickness');
     if(s.mode==='feed'){
      if(phase==='auto'&&u<=.50)for(const x of [-.075,.075])for(const z of [-.12,.12])near(box(part('VacuumCup'+x+'_'+z)).min.y,paper.max.y,'cup / actual sheet');
      for(const x of [.14,.43,.72,1,1.28]){const a=box(part('FeedRoller'+x+'_-1')),b=box(part('FeedRoller'+x+'_1'));const facetTolerance=.02*(1-Math.cos(Math.PI/(entry.suffix==='simple'?16:40)))+3e-6;if(Math.abs(a.max.y-.90)>facetTolerance||Math.abs(b.min.y-.9008)>facetTolerance)throw Error('roller / sheet gap beyond exported cylinder facet error');if(a.max.z-a.min.z<.45||a.max.y-a.min.y>.04001)throw Error('roller axis was overwritten by motion');}
     }else{
      if(phase==='auto')near(box(part('LiftPlatform')).max.y,paper.min.y,'pile / lift deck');
      if(phase==='load'||phase==='unload'){const supports=[box(part(phase==='load'?'InputTableTop':'OutputTableTop')),box(part('LiftPlatform'))];if(!supports.some(b=>Math.abs(b.max.y-paper.min.y)<3e-6&&Math.min(b.max.x,paper.max.x)-Math.max(b.min.x,paper.min.x)>0))throw Error('pile without table/deck support');}
     }
     samples.push({phase,u,actual_sheet_or_pile_contact:true});if([0,.5,1].includes(u)&&samples.length%13<=11){view('iso');await picture(entry.id+'/'+entry.suffix+'-material-'+phase+'-'+Math.round(u*100)+'.png','真实纸材 · '+phase+' '+Math.round(u*100)+'%');}
    }
    checks.push({id:entry.id,lod:entry.suffix,samples});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;
   }
   await save('material-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status('4个实际GLB：送纸吸附/辊组方向与整垛举升承托通过。');
  }catch(e){console.error(e);await save('material-contact-error.json',JSON.stringify({passed:false,error:String(e),checks},null,2));status('纸材复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();busy=false;}
 };
}
if(review==='B02-Q04'){
 const button=document.createElement('button');button.textContent='整饰工件与耗材接触复核';document.querySelector('header').append(button);
 button.onclick=async()=>{
  if(busy)return;busy=true;playing=false;const checks=[];let paperRig,auxRig,paperDecoded,auxDecoded;
  const {specs,surface,filmHeight}=await import(base+'source/batch-spec.mjs');
  const near=(a,b,label)=>{if(Math.abs(a-b)>3e-6)throw Error(label+': '+a+' != '+b);};
  const part=(p,r,name)=>{const n=p.nodes.find(n=>n.name===name);if(!n||!r.nodes.has(n.id))throw Error('Missing actual node '+name);return r.nodes.get(n.id);};
  const bounds=o=>new T.Box3().setFromObject(o),volume=o=>{const v=bounds(o).getSize(new T.Vector3());return v.x*v.y*v.z;};
  try{
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());
    paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    let ap;if(s.auxiliary){const d=base+'items/'+entry.id+'-auxiliary/';ap=(await fetch(d+'module.json').then(r=>r.json())).data.definition.payload.data;const data=await fetch(d+'paper.glb').then(r=>r.arrayBuffer());auxDecoded=await decodeGlb(data);auxRig=await bindScene(ap,async()=>data,{assetKey:assetKey(ap.source.data),decoded:auxDecoded});rig.nodes.get(current.presentation.anchor_roles.aux_seat.data).add(auxRig.root);}
    const phase=s.initial+':auto',clip=p.timeline.find(c=>c.phase_ref===phase),samples=[];
    for(const u of [0,.12,.20,.25,.28,.35,.40,.50,.65,.70,.74,.75,.78,.82,.90,1,.50,0]){
     pose('auto',u);paperRig.apply({known:true,display_phase_id:phase,progress:u,terminal_pose:false,effective_ms:u*1000,duration_ms:1000},{clip,progress:u});
     if(auxRig){const ac=ap.timeline.find(c=>c.phase_ref.endsWith(':auto'));auxRig.apply({known:true,display_phase_id:ac.phase_ref,progress:u,terminal_pose:false,effective_ms:u*1000,duration_ms:1000},{clip:ac,progress:u});}
     rig.root.updateMatrixWorld(true);
     const main=part(p,paperRig,s.mode==='emboss'?(u>.400001?'ActualEmbossedSheet':'ActualFlatSheet'):'ActualPaperSubstrate');near(bounds(main).min.y,surface,'actual substrate / platen height');
     if(s.mode==='foil'){
      const metal=volume(part(ap,auxRig,'ActualTransferMetal')),coat=volume(part(p,paperRig,'ActualTransferredLayer')),total=s.mark[0]*s.mark[1]*s.metal_thickness;
      if(Math.abs(metal+coat-total)>total*1e-4)throw Error('foil layer volume not conserved at '+u+': '+(metal+coat));
      if(u===.50){near(bounds(part(current.presentation,rig,'HeatedDie')).min.y,bounds(part(ap,auxRig,'ActualPETCarrier')).max.y,'actual PET / hot die');for(const sign of [-1,1])for(const z of [-.04,.04])near(bounds(part(current.presentation,rig,'CarrierVacuumPad'+sign+'_'+z)).min.y,bounds(part(ap,auxRig,'ActualPETCarrier')).max.y,'actual PET / vacuum pads');}
      if(u===1){const stage=rig.nodes.get(current.presentation.anchor_roles.backing_staging.data).getWorldPosition(new T.Vector3());near(bounds(part(ap,auxRig,'ActualPETCarrier')).min.y,stage.y,'spent carrier pending anchor');}
     }else if(s.mode==='emboss'){
      const flat=part(p,paperRig,'ActualFlatSheet').scale.x,raised=part(p,paperRig,'ActualEmbossedSheet').scale.x;if(![0,1].includes(flat)||![0,1].includes(raised)||flat+raised!==1)throw Error('Embossed paper shrinks or duplicates');
      if(u===.50){const heights=[];part(p,paperRig,'ActualEmbossedSheet').traverse(o=>{if(o.isMesh){const a=o.geometry.attributes.position;for(let i=0;i<a.count;i++){const v=new T.Vector3().fromBufferAttribute(a,i).applyMatrix4(o.matrixWorld);if(Math.abs(Math.abs(v.x)-.05)<1e-6&&Math.abs(Math.abs(v.z)-.05)<1e-6)heights.push(v.y);}}});near(Math.min(...heights),bounds(part(current.presentation,rig,'PositiveEmbossTool')).max.y,'actual emboss underside / lower male');near(Math.max(...heights),bounds(part(current.presentation,rig,'FemaleEmbossRoof')).min.y,'actual emboss top / upper female');}
     }else if(s.mode==='pad'){
      const cliche=part(ap,auxRig,'ActualClicheInk'),pad=part(ap,auxRig,'ActualInkOnTransferPad'),coat=part(p,paperRig,'ActualTransferredLayer'),sum=volume(cliche)+volume(pad)+volume(coat);
      if(Math.abs(sum-s.volume_m3)>s.volume_m3*1e-4)throw Error('pad ink volume not conserved at '+u+': '+sum);
      if(u===.25)near(bounds(pad).min.y,bounds(cliche).max.y,'pad pickup ink stack');if(u===.50)near(bounds(pad).max.y,bounds(part(current.presentation,rig,'SiliconeTransferPad')).min.y,'actual ink / pad face');if(u===.74)near(bounds(pad).min.y,bounds(coat).max.y,'printed layer / remaining pad layer');
     }else{
      const left=volume(part(ap,auxRig,'ActualMeteredWellLiquid')),layer=s.mode==='uv'?volume(part(p,paperRig,'ActualWetVarnish'))+volume(part(p,paperRig,'ActualCuredVarnish')):volume(part(p,paperRig,'ActualTransferredLayer'));
      if(Math.abs(left+layer-s.volume_m3)>s.volume_m3*1e-4)throw Error('metered fluid not conserved at '+u+': '+(left+layer));
      if(u>=.12&&u<=.50)near(bounds(part(current.presentation,rig,'ScreenMeshZ0')).min.y,surface+s.size[1],'actual paper / contact screen');
      near(bounds(part(current.presentation,rig,'SqueegeeRubber')).min.y,bounds(part(current.presentation,rig,'ScreenStencilMask')).max.y,'blade / stencil mask');
     }
     samples.push({u,actual_GLB_material_and_contact:true});
     if([0,.50,1].includes(u)&&samples.length<=16){view('iso');await picture(entry.id+'/'+entry.suffix+'-finish-contact-'+Math.round(u*100)+'.png','真实工件与耗材 · auto '+Math.round(u*100)+'%');}
     if(u===1&&entry.suffix==='detailed'){const x=s.mode==='uv'?.8:s.mode==='pad'?.4:0,target=new T.Vector3(x,surface+.001,0);controls.target.copy(target);camera.position.set(s.mode==='uv'?x+.03:x,s.mode==='uv'?2.0:surface+.12,s.mode==='uv'?.03:.65);if(s.mode==='emboss'){target.set(.05,surface+.0012,.05);controls.target.copy(target);camera.position.set(.16,surface+.03,.22);}camera.lookAt(target);await picture(entry.id+'/detailed-finished-closeup.png','实际成品近景 · 完成姿态');}
    }
    checks.push({id:entry.id,lod:entry.suffix,samples,finished_closeup:entry.suffix==='detailed'?entry.id+'/detailed-finished-closeup.png':null});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;if(auxRig){auxRig.root.removeFromParent();auxRig.dispose();auxDecoded.dispose();auxRig=auxDecoded=null;}
   }
   await save('finish-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks,Native:'separate acceptance'},null,2));status('10个实际GLB：整饰工件、耗材转移、PET载体及模具/网版/胶垫接触复核通过。');
  }catch(e){console.error(e);await save('finish-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)}));status('整饰接触检查失败：'+e);}
  finally{for(const r of [paperRig,auxRig])if(r){r.root.removeFromParent();r.dispose();}paperDecoded?.dispose();auxDecoded?.dispose();busy=false;}
 };
}
if(review==='B01-Q04'){
 const button=document.createElement('button');button.textContent='喷码上光固化实际工件复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let attached=[];
  try{
   const{specs,seatX,pixels,pixelSize,printThickness,printedPixelCount,coatingFraction}=await import(base+'source/batch-spec.mjs');
   const object=(p,r,name)=>r.nodes.get(p.nodes.find(n=>n.name===name)?.id),bounds=o=>new T.Box3().setFromObject(o,true),near=(a,b,label,tolerance=3e-6)=>{if(Math.abs(a-b)>tolerance)throw Error(label+': '+a+' vs '+b);};
   const attach=async(id,anchor)=>{const dir=base+'items/'+id+'/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,data=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer()),d=await decodeGlb(data),r=await bindScene(p,async()=>data,{assetKey:assetKey(p.source.data),decoded:d});rig.nodes.get(current.presentation.anchor_roles[anchor].data).add(r.root);const value={p,r,d};attached.push(value);return value;};
   const apply=(item,phase,u)=>{const clip=item.p.timeline.find(c=>c.phase_ref===phase);if(!clip)throw Error('missing '+phase);item.r.apply({known:true,display_phase_id:phase,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],paper=await attach(entry.id+'-input','cargo'),aux=s.auxiliary?await attach(entry.id+'-auxiliary','aux_seat'):null,samples=[];
    if(s.mode==='coat'){
     pose('auto',.4);rig.root.updateMatrixWorld(true);const probes=[['ApplicationRoll',.08,-1],['ImpressionRoll',.06,1]].map(([name,r,sign])=>{const o=object(current.presentation,rig,name),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0));return{name,o,contact,local:o.worldToLocal(contact.clone())};});pose('auto',.40001);rig.root.updateMatrixWorld(true);for(const a of probes)near((a.o.localToWorld(a.local).x-a.contact.x)/.00001,2,a.name+' co-moving nip',.001);
    }
    for(const u of[0,.1,.375,.49,.5,.51,.625,.9,1,.5,0]){
     pose('auto',u);apply(paper,s.initial+':auto',u);if(aux)apply(aux,'dose_ready:auto',u);rig.root.updateMatrixWorld(true);
     const substrate=bounds(object(paper.p,paper.r,'ActualPaperSubstrate'));near(substrate.min.y,s.surface,'actual paper support');near((substrate.min.x+substrate.max.x)/2,seatX(s,'auto',u),'actual paper X');
     let applied;
     if(s.mode==='code'){
      let count=0;for(let i=0;i<pixels.length;i++){const pixel=object(paper.p,paper.r,'CodePixel'+i);if(pixel.scale.x>.5){count++;const v=bounds(pixel);near(v.min.y,s.surface+s.size[1],'ink on paper');near(v.max.y-v.min.y,printThickness,'actual ink thickness');}}
      if(count!==printedPixelCount(u))throw Error('actual code bitmap differs at '+u);applied=count*pixelSize*pixelSize*printThickness;
      const nozzle=bounds(object(current.presentation,rig,'Nozzle3'));near(nozzle.min.y-(s.surface+s.size[1]),.003,'actual nozzle clearance');
     }else{
      const film=object(paper.p,paper.r,'ActualWetFilm'),box=bounds(film);applied=film.scale.x*s.volume;near(film.scale.x,s.mode==='coat'?coatingFraction(u):1,'actual wet film coverage');if(film.scale.x>0)near(box.max.y-box.min.y,.000020,'actual film thickness');
      if(s.mode==='coat')near(object(current.presentation,rig,'ApplicationRoll').getWorldPosition(new T.Vector3()).y-.08,s.surface+s.size[1]+.000020,'application roll film datum');
      if(s.mode==='cure')near(object(paper.p,paper.r,'ActualCuredFilm').scale.x,0,'no premature cured state');
     }
     if(aux){const liquid=object(aux.p,aux.r,'ActualDose'),height=bounds(liquid).getSize(new T.Vector3()).y,remaining=Math.PI*.012*.012*height;near(applied+remaining,s.volume,'actual dose conservation',s.volume*1e-4);}
     samples.push({u,actual_paper_support:true,actual_applied_volume_m3:applied,actual_dose_conserved:!!aux});
     if([0,.5,1].includes(u)&&samples.length<=9){view('iso');await picture(entry.id+'/'+entry.suffix+'-finish-contact-'+Math.round(u*100)+'.png','真实工件与机构 · '+Math.round(u*100)+'%');}
    }
    if(entry.suffix==='detailed'){
     const u=s.mode==='code'?.60:s.mode==='coat'?.5:.8;pose('auto',u);apply(paper,s.initial+':auto',u);if(aux)apply(aux,'dose_ready:auto',u);rig.root.updateMatrixWorld(true);const target=bounds(object(paper.p,paper.r,'ActualPaperSubstrate')).getCenter(new T.Vector3());controls.target.copy(target);camera.position.copy(target).add(s.mode==='coat'?new T.Vector3(.75,.20,.60):new T.Vector3(.15,.65,.30));camera.lookAt(target);await picture(entry.id+'/actual-material-closeup.png','实际纸材与 '+(s.mode==='code'?'A01 喷码':s.mode==='coat'?'20µm 湿膜':'固化通道湿膜'));
    }
    if(s.mode==='cure'){apply(paper,'varnished_cured',0);near(object(paper.p,paper.r,'ActualWetFilm').scale.x,0,'wet film hidden after actual final state');near(object(paper.p,paper.r,'ActualCuredFilm').scale.x,1,'cured film full final state');}
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,co_moving_application_nip:s.mode==='coat',noncontact_gap_m:s.mode==='code'?.003:null,cure_state_not_premature:s.mode==='cure'});for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}attached=[];
   }
   await save('finish-contact-checks.json',JSON.stringify({passed:true,models:checks.length,actual_product_decoder:true,checks,Native:'separate acceptance'},null,2));status('6个实际GLB：喷码间隙与字点、上光剂量与辊面方向、UV湿膜状态复核通过。');
  }catch(e){console.error(e);await save('finish-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('工件复核失败：'+e);}
  finally{for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}busy=false;}
 };
}
if(review==='B01-Q05'){
 const button=document.createElement('button');button.textContent='旋转压痕纸材与轮面复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;let paperRig,paperDecoded;const checks=[];
  try{
   const{specs,seatX,scoredFraction}=await import(base+'source/batch-spec.mjs');
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    const object=(presentation,r,name)=>r.nodes.get(presentation.nodes.find(n=>n.name===name)?.id),near=(a,b,label,tolerance=3e-6)=>{if(Math.abs(a-b)>tolerance)throw Error(label+': '+a+' vs '+b);},apply=(phase,u)=>{const clip=p.timeline.find(c=>c.phase_ref===phase);paperRig.apply({known:true,display_phase_id:phase,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});};
    pose('auto',.4);rig.root.updateMatrixWorld(true);const probes=[['ScoringArbor',.08,-1],['GroovedCounterArbor',.06,1]].map(([name,r,sign])=>{const o=object(current.presentation,rig,name),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0));return{name,o,contact,local:o.worldToLocal(contact.clone())};});pose('auto',.40001);rig.root.updateMatrixWorld(true);for(const v of probes)near((v.o.localToWorld(v.local).x-v.contact.x)/.00001,2,v.name+' forward contact',.001);
    const samples=[];for(const u of[0,.2,.375,.4,.5,.6,.625,.8,1,.5,0]){
     pose('auto',u);apply('flat:auto',u);rig.root.updateMatrixWorld(true);const raw=object(p,paperRig,'ActualFlatBoard'),scored=object(p,paperRig,'ActualScoredBoard'),f=scoredFraction(u);near(raw.scale.x+scored.scale.x,1,'full paper length partition');near(scored.scale.x,f,'formed fraction');near(rig.nodes.get(current.presentation.anchor_roles.cargo.data).position.x,seatX(s,'auto',u),'paper X');
     const position=scored.geometry.attributes.position;let highest=-Infinity,valley=Infinity;for(let i=0;i<position.count;i++){highest=Math.max(highest,position.getY(i));if(Math.abs(Math.abs(position.getZ(i))-.1)<1e-6&&position.getY(i)>0)valley=Math.min(valley,position.getY(i));}near(highest,s.size[1],'paper upper datum');near(highest-valley,s.score_depth,'actual groove depth');near(object(current.presentation,rig,'ScoringArbor').position.y-.08,s.surface+valley,'crease wheel nip height');near(object(current.presentation,rig,'GroovedCounterArbor').position.y+.06,s.surface,'counter support');
     samples.push({u,actual_score_depth_m:highest-valley,formed_fraction:f,full_substrate_length:true});if([0,.5,1].includes(u)&&samples.length<=9){view('iso');await picture(entry.id+'/'+entry.suffix+'-score-contact-'+Math.round(u*100)+'.png','实际纸板与压痕轮 · '+Math.round(u*100)+'%');}
    }
    if(entry.suffix==='detailed'){pose('auto',.8);apply('longitudinally_scored:auto',.8);rig.root.updateMatrixWorld(true);const target=new T.Vector3(seatX(s,'auto',.8),s.surface,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.2,.6,.3));camera.lookAt(target);await picture(entry.id+'/actual-score-closeup.png','两条真实纵向压痕 · 0.6mm');}
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,co_moving_nip:true});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;
   }
   await save('score-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks,scope:'kinematic score profile and nip datum; not contact-force or corrugation-material solving'},null,2));status('2个实际GLB：压痕轮同向、承托面、纸长及0.6mm纵向压痕通过。');
  }catch(e){console.error(e);await save('score-contact-error.json',JSON.stringify({passed:false,error:String(e),checks},null,2));status('压痕复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();busy=false;}
 };
}
if(review==='B01-Q06'){
 const button=document.createElement('button');button.textContent='分切双输出与剪刀盘复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;let paperRig,paperDecoded;const checks=[];
  try{
   const{specs,seatX,separation}=await import(base+'source/batch-spec.mjs');
   const object=(p,r,name)=>r.nodes.get(p.nodes.find(n=>n.name===name)?.id),box=o=>new T.Box3().setFromObject(o,true),near=(a,b,label,tolerance=3e-6)=>{if(Math.abs(a-b)>tolerance)throw Error(label+': '+a+' vs '+b);};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    const apply=u=>{const phase='wide_sheet:auto',clip=p.timeline.find(c=>c.phase_ref===phase);paperRig.apply({known:true,display_phase_id:phase,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});},samples=[];
    for(const u of[0,.1,.375,.5,.625,.70,.8,.9,1,.5,0]){
     pose('auto',u);apply(u);rig.root.updateMatrixWorld(true);const left=box(object(p,paperRig,'InputHalfRegion-1')),right=box(object(p,paperRig,'InputHalfRegion1')),upper=box(object(current.presentation,rig,'UpperShearKnife')),lower=box(object(current.presentation,rig,'LowerShearKnife'));
     near(left.min.y,s.surface,'paper support');near(right.min.y,s.surface,'second half support');near(left.getSize(new T.Vector3()).x,.50,'actual half length');near(left.getSize(new T.Vector3()).z,.20,'actual half width');near(right.min.z-left.max.z,separation(u),'actual separation gap');near((left.min.x+left.max.x)/2,seatX(s,'auto',u),'paper X');near(upper.max.z,0,'upper knife face');near(lower.min.z,0,'lower knife face');if(lower.max.y-upper.min.y<.0015)throw Error('actual knife overlap lost');
     const leftArea=left.getSize(new T.Vector3()).x*left.getSize(new T.Vector3()).z,rightArea=right.getSize(new T.Vector3()).x*right.getSize(new T.Vector3()).z;near(leftArea+rightArea,.20,'total actual board area');
     if(u<.625)near(right.min.z-left.max.z,0,'no premature spreading');samples.push({u,halves:2,area_m2:leftArea+rightArea,separation_m:right.min.z-left.max.z,actual_knife_overlap_m:lower.max.y-upper.min.y});
     if([0,.5,1].includes(u)&&samples.length<=9){view('iso');await picture(entry.id+'/'+entry.suffix+'-slit-contact-'+Math.round(u*100)+'.png','实际原板两半与剪刀盘 · '+Math.round(u*100)+'%');}
    }
    if(entry.suffix==='detailed'){pose('auto',.90);apply(.90);rig.root.updateMatrixWorld(true);const target=new T.Vector3(.80,s.surface,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.20,.65,.35));camera.lookAt(target);await picture(entry.id+'/actual-two-halves-closeup.png','实际两半板与10mm出料间距');}
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,actual_product_decoder:true});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;
   }
   await save('slit-contact-checks.json',JSON.stringify({passed:true,models:checks.length,checks,scope:'actual sheet half geometry, separation timing, adjacent knife faces and overlap; Native output identity checked separately'},null,2));status('2个实际GLB：双半板面积、剪刀盘重叠/侧面及实际分离通过。');
  }catch(e){console.error(e);await save('slit-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('分切复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();busy=false;}
 };
}
if(['B01-Q07','B01-Q07-R1','B01-Q07-R2'].includes(review)){
 const button=document.createElement('button');button.textContent='平压模切与四角清废复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;let paperRig,paperDecoded;const checks=[];
  try{
   const{specs,cornerPositions,trayTop,trayZ}=await import(base+'source/batch-spec.mjs');const object=(p,r,name)=>r.nodes.get(p.nodes.find(n=>n.name===name)?.id),box=o=>new T.Box3().setFromObject(o,true),near=(a,b,label)=>{if(Math.abs(a-b)>3e-6)throw Error(label+': '+a+' vs '+b);};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    const apply=(phase,u)=>{pose(phase,u);const name='uncut_sheet:'+phase,clip=p.timeline.find(c=>c.phase_ref===name);paperRig.apply({known:true,display_phase_id:name,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});rig.root.updateMatrixWorld(true);},samples=[];
    for(const u of[0,.15,.35,.55,.72,.75,.79,.83,.90,.98,1,.55,0]){
     apply('auto',u);const bed=box(object(current.presentation,rig,'CrossBlankAnvilBed')),blank=box(object(p,paperRig,'ActualCrossBlank')),headY=object(current.presentation,rig,'DiePlaten').position.y;near(blank.min.y,bed.max.y,'actual blank on anvil');
     for(let i=0;i<12;i++){const edge=box(object(current.presentation,rig,'SteelRuleSegment'+i));if(u===.35||u===.55)near(edge.min.y,s.surface,'steel rule bottom datum');if(u>=.75&&edge.min.y<s.surface+.10)throw Error('waste tray starts before die retracts');}
     for(let i=0;i<cornerPositions.length;i++){const corner=box(object(p,paperRig,'ActualAttachedCorner'+i)),pad=box(object(current.presentation,rig,'CornerSupportPad'+i)),center=corner.getCenter(new T.Vector3());near(corner.min.y,pad.max.y,'actual corner carried by pad '+i);near(center.x,cornerPositions[i][0],'corner X');near(center.z,trayZ(Math.sign(cornerPositions[i][2]),'auto',u),'corner clear path');near(corner.min.y,trayTop(s,'auto',u),'corner lift height');}
     const piston=box(object(current.presentation,rig,'ExposedPistonRod'));near(piston.max.y,1.40,'exposed rod meets cylinder');near(piston.min.y,headY+.028,'exposed rod meets platen');
     samples.push({u,actual_die_anvil_contact:true,actual_four_corner_support:true,rod_seated:true});if([0,.35,.90,1].includes(u)&&samples.length<=11){view('iso');await picture(entry.id+'/'+entry.suffix+'-die-contact-'+Math.round(u*100)+'.png','实际刀模与清废托盘 · '+Math.round(u*100)+'%');}
    }
    for(const u of[0,.5,1]){apply('unload',u);for(let i=0;i<4;i++){const c=box(object(p,paperRig,'ActualAttachedCorner'+i)),pad=box(object(current.presentation,rig,'CornerSupportPad'+i));near(c.min.y,pad.max.y,'corner retained during operator unload');near(c.getCenter(new T.Vector3()).x,cornerPositions[i][0],'unload does not drag corner with blank');}}
    if(entry.suffix==='detailed'){apply('auto',.98);const target=new T.Vector3(0,1.08,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.6,.8,.9));camera.lookAt(target);await picture(entry.id+'/actual-blank-and-four-corners.png','实际十字坯与四角清废');}
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,actual_product_decoder:true});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;
   }
   await save('die-contact-checks.json',JSON.stringify({passed:true,models:checks.length,checks,scope:'actual decoded die/anvil/rod and four guided waste trays; Native output identities checked separately'},null,2));status('2个实际GLB：刀模/砧床、活塞杆及四角废料托盘接触通过。');
  }catch(e){console.error(e);await save('die-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('模切复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();busy=false;}
 };
}
if(review==='B01-Q09'){
 const button=document.createElement('button');button.textContent='预模切箱坯清废机构复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;let paperRig,paperDecoded;const checks=[];
  try{
   const{specs,cornerPositions,trayTop,trayZ}=await import(base+'source/batch-spec.mjs');const object=(p,r,name)=>r.nodes.get(p.nodes.find(n=>n.name===name)?.id),box=o=>new T.Box3().setFromObject(o,true),near=(a,b,label)=>{if(Math.abs(a-b)>3e-6)throw Error(label+': '+a+' vs '+b);};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    const apply=(phase,u)=>{pose(phase,u);const name='die_cut_sheet_with_tabs:'+phase,clip=p.timeline.find(c=>c.phase_ref===name);paperRig.apply({known:true,display_phase_id:name,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});rig.root.updateMatrixWorld(true);},samples=[];
    for(const u of[0,.15,.35,.371,.499,.55,.72,.75,.79,.83,.90,.98,1,.55,0]){
     apply('auto',u);const bed=box(object(current.presentation,rig,'CrossBlankAnvilBed')),blank=box(object(p,paperRig,'ActualCrossBlank')),headY=object(current.presentation,rig,'StrippingPlate').position.y;near(blank.min.y,bed.max.y,'actual blank on anvil');
     if(current.presentation.nodes.some(n=>n.name.startsWith('SteelRule')))throw Error('stripper must not cut a fresh contour');
     for(let i=0;i<4;i++){
      const punch=box(object(current.presentation,rig,'StrippingPunch'+i)),corner=box(object(p,paperRig,'ActualAttachedCorner'+i));
      if(u>=.35&&u<=.55)near(punch.min.y,corner.max.y,'punch follows corner during tab break');
      if(u>=.75&&punch.min.y<s.surface+.10)throw Error('tray moves before punch retracts');
     }
     for(const x of[-.10,.10])for(const sign of[-1,1])near(box(object(current.presentation,rig,'RetainLip'+x+'_'+sign)).min.y,1.0835,'blank retainer clearance');
     for(const x of[-.10,.10])for(const sign of[-1,1]){const post=box(object(current.presentation,rig,'RetainPost'+x+'_'+sign)),arm=box(object(current.presentation,rig,'RetainArm'+x+'_'+sign));near(post.min.y,.80,'retainer column seated on base');near(post.max.y,arm.min.y,'retainer post meets arm underside');}
     if(p.nodes.filter(n=>n.name.startsWith('RetainedXTab')||n.name.startsWith('RetainedZTab')).length!==8)throw Error('eight real retention tab meshes required');
     for(let i=0;i<cornerPositions.length;i++){const corner=box(object(p,paperRig,'ActualAttachedCorner'+i)),pad=box(object(current.presentation,rig,'CornerSupportPad'+i)),center=corner.getCenter(new T.Vector3());near(corner.min.y,pad.max.y,'actual corner carried by pad '+i);near(center.x,cornerPositions[i][0]+Math.sign(cornerPositions[i][0])*.0001,'corner X');near(center.z,trayZ(Math.sign(cornerPositions[i][2]),'auto',u)+Math.sign(cornerPositions[i][2])*.0001,'corner clear path');near(corner.min.y,trayTop(s,'auto',u),'corner lift height');}
     const piston=box(object(current.presentation,rig,'ExposedPistonRod'));near(piston.max.y,1.60,'exposed rod meets cylinder');near(piston.min.y,headY+.028,'exposed rod meets platen');
     samples.push({u,actual_corner_punch_contact:true,actual_four_corner_support:true,rod_seated:true});if([0,.35,.55,.90].includes(u)&&samples.length<=15){view('iso');await picture(entry.id+'/'+entry.suffix+'-strip-contact-'+Math.round(u*100)+'.png','实际压料头与清废托盘 · '+Math.round(u*100)+'%');}
    }
    apply('auto',0);const openY=object(current.presentation,rig,'StrippingPlate').position.y,barrel=box(object(current.presentation,rig,'HydraulicBarrel'));apply('auto',.55);const lowY=object(current.presentation,rig,'StrippingPlate').position.y;if(openY-lowY>barrel.max.y-barrel.min.y-.01)throw Error('stripper stroke exceeds cylinder body allowance');
    for(const u of[0,.5,1]){apply('unload',u);for(let i=0;i<4;i++){const c=box(object(p,paperRig,'ActualAttachedCorner'+i)),pad=box(object(current.presentation,rig,'CornerSupportPad'+i));near(c.min.y,pad.max.y,'corner retained during operator unload');near(c.getCenter(new T.Vector3()).x,cornerPositions[i][0]+Math.sign(cornerPositions[i][0])*.0001,'unload does not drag corner with blank');}}
    if(entry.suffix==='detailed'){apply('auto',.98);const target=new T.Vector3(0,1.08,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.6,.8,.9));camera.lookAt(target);await picture(entry.id+'/actual-blank-and-four-corners.png','实际清废后箱坯与四角废料');}
    if(entry.suffix==='detailed'){apply('auto',0);const target=new T.Vector3(.15,1.083,.15);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.12,.18,.16));camera.lookAt(target);await picture(entry.id+'/actual-precut-tabs.png','实际0.2mm切缝与2mm连接点');}
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,actual_product_decoder:true});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;
   }
   await save('strip-contact-checks.json',JSON.stringify({passed:true,models:checks.length,checks,scope:'actual pre-cut tabs, four stripping punches, retention clearance, rods and guided waste trays; Native output identities checked separately'},null,2));status('2个实际GLB：压料头/预切角料、活塞杆及清废托盘接触通过。');
  }catch(e){console.error(e);await save('strip-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('箱坯清废复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();busy=false;}
 };
}
if(review==='B01-Q08'){
 const button=document.createElement('button');button.textContent='数字切割压痕双工具复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;let paperRig,paperDecoded;const checks=[];
  try{
   const{specs,program,toolPose,cornerPositions,trayTop,trayZ}=await import(base+'source/batch-spec.mjs');const object=(p,r,name)=>r.nodes.get(p.nodes.find(n=>n.name===name)?.id),box=o=>new T.Box3().setFromObject(o,true),near=(a,b,label,tolerance=4e-6)=>{if(Math.abs(a-b)>tolerance)throw Error(label+': '+a+' vs '+b);};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,bytes=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer());paperDecoded=await decodeGlb(bytes);paperRig=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:paperDecoded});rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(paperRig.root);
    const apply=u=>{pose('auto',u);const phase=s.initial+':auto',clip=p.timeline.find(c=>c.phase_ref===phase);paperRig.apply({known:true,display_phase_id:phase,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});rig.root.updateMatrixWorld(true);};
    const strokes=program.filter(v=>v.kind==='stroke'),times=[0,...strokes.flatMap(v=>[v.start+(v.end-v.start)*.371,v.start+(v.end-v.start)*.683]),.78,.80,.82,.85,.88,.94,.98,1,.5,0],samples=[];
    for(const u of times){
     apply(u);const expected=toolPose(u),knife=object(current.presentation,rig,'ActualTangentialKnife').localToWorld(new T.Vector3()),wheel=object(current.presentation,rig,'CreasingWheel').getWorldPosition(new T.Vector3()),wheelBounds=box(object(current.presentation,rig,'ActualCreasingWheel'));
     near(knife.x,expected.x,'actual knife X');near(knife.z,expected.z-.035,'actual knife Z with tool offset');near(knife.y,expected.knifeY,'actual knife tip');near(wheel.x,expected.x,'actual wheel X');near(wheel.z,expected.z+.035,'actual wheel Z with tool offset');near(wheel.y-.020,expected.creaseY,'actual wheel bottom nominal');near(wheelBounds.min.y,expected.creaseY,'actual polygon wheel contact',entry.suffix==='simple'?.000025:.000007);
     const segment=program.find(v=>u>=v.start&&u<v.end);if(segment?.kind==='stroke'){
      const tool=segment.tool;if(tool==='knife'&&expected.creaseDone!==4)throw Error('cut before four creases');const direction=new T.Vector3(segment.to.x-segment.from.x,0,segment.to.z-segment.from.z).normalize(),forward=new T.Vector3(1,0,0).applyQuaternion(object(current.presentation,rig,tool+'WorkingHead').getWorldQuaternion(new T.Quaternion()));near(forward.distanceTo(direction),0,'actual tool tangent direction',.00001);
      if(tool==='knife'&&wheelBounds.min.y<s.surface+s.size[1]+.03)throw Error('inactive wheel is not lifted');if(tool==='crease'&&knife.y<s.surface+s.size[1]+.03)throw Error('inactive knife is not lifted');
     }
     for(const[tool,offset]of[['knife',.064],['crease',.09]]){const rod=box(object(current.presentation,rig,tool+'ExposedLiftRod'));near(rod.max.y,1.24,'lift rod upper contact');near(rod.min.y,expected[tool+'Y']+offset,'lift rod tool contact');}
     for(const[name,axis,distance]of[['XLeadScrew','x',expected.x],['ZLeadScrew','z',expected.z]]){const q=object(current.presentation,rig,name).quaternion,angle=2*Math.atan2(q[axis],q.w);near(Math.sin(angle+distance*Math.PI*2/.020),0,'lead screw handed travel '+name,.002);}
     let active=0;for(let i=0;i<=4;i++){const part=object(p,paperRig,'ActualCrossBlankStage'+i);if(part.scale.x>.5){active++;if(i!==expected.creaseDone)throw Error('actual crease stage mismatch');}}if(active!==1)throw Error('overlapping blank stages');
     for(let i=0;i<cornerPositions.length;i++){const corner=box(object(p,paperRig,'ActualAttachedCorner'+i)),pad=box(object(current.presentation,rig,'CornerSupportPad'+i));near(corner.min.y,pad.max.y,'actual corner support');near(corner.getCenter(new T.Vector3()).z,trayZ(Math.sign(cornerPositions[i][2]),'auto',u),'actual corner path');}if(u>=.82&&(knife.y<1.119||wheelBounds.min.y<1.119))throw Error('clearance starts before tools lifted');
     samples.push({u,actual_dual_tool_offsets_and_tangent:true,actual_lift_rods_seated:true,lead_screw_hand_verified:true,crease_stage:expected.creaseDone,actual_corner_support:true});
    }
    for(const u of[0,.30,.60,.98]){apply(u);view('iso');await picture(entry.id+'/'+entry.suffix+'-digital-contact-'+Math.round(u*100)+'.png','实际双工具与清废 · '+Math.round(u*100)+'%');}
    if(entry.suffix==='detailed'){
     for(const tool of['crease','knife']){const v=strokes.find(v=>v.tool===tool),u=(v.start+v.end)/2;apply(u);const target=object(current.presentation,rig,tool+'WorkingHead').getWorldPosition(new T.Vector3()).add(new T.Vector3(0,.04,0));controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.20,.16,.24));camera.lookAt(target);await picture(entry.id+'/actual-'+tool+'-tool.png','实际 '+tool+' 工具工作位');}
     apply(.99);controls.target.set(0,1.08,0);camera.position.set(.22,1.85,.28);camera.lookAt(controls.target);await picture(entry.id+'/actual-four-fold-lines.png','实际四条压痕与十字坯');
    }
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,actual_product_decoder:true,crease_strokes:4,cut_strokes:12});paperRig.root.removeFromParent();paperRig.dispose();paperDecoded.dispose();paperRig=paperDecoded=null;
   }
   await save('digital-contact-checks.json',JSON.stringify({passed:true,models:checks.length,checks,scope:'actual decoded dual tool paths, tangent directions, lifted inactive tool, lead screw handedness, crease stages and corner carriers; Native checked separately'},null,2));status('2个实际GLB：4压痕/12切割、双工具偏置/转向、升降杆、丝杠与清废通过。');
  }catch(e){console.error(e);await save('digital-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('数字切割复核失败：'+e);}
  finally{if(paperRig){paperRig.root.removeFromParent();paperRig.dispose();}paperDecoded?.dispose();busy=false;}
 };
}
if(['B01-Q07','B01-Q07-R1','B01-Q07-R2','B01-Q08','B01-Q09'].includes(review)){
 const button=document.createElement('button');button.textContent='清废升降缸盖接触审计';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;const checks=[];try{
  for(const entry of inventory.entries){await load(entry.id,entry.suffix);pose('auto',1);rig.root.updateMatrixWorld(true);const part=name=>rig.nodes.get(current.presentation.nodes.find(n=>n.name===name)?.id),corners=[];
   for(let i=0;i<4;i++){const barrel=new T.Box3().setFromObject(part('WasteLiftBarrel'+i),true),crosshead=new T.Box3().setFromObject(part('LiftCrosshead'+i),true),coplanar=Math.abs(barrel.max.y-crosshead.max.y)<1e-6&&Math.min(barrel.max.x,crosshead.max.x)>Math.max(barrel.min.x,crosshead.min.x)&&Math.min(barrel.max.z,crosshead.max.z)>Math.max(barrel.min.z,crosshead.min.z);corners.push({i,barrel_top_m:barrel.max.y,crosshead_top_m:crosshead.max.y,coplanar_exposed_caps:coplanar});}
   checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},corners,passed:corners.every(c=>!c.coplanar_exposed_caps)});
  }
  const passed=checks.every(c=>c.passed);await save('lift-joint-audit.json',JSON.stringify({passed,actual_product_decoder:true,checks},null,2));status(passed?'升降缸盖接触审计通过':'升降缸盖接触审计失败：存在重合外露顶面');
 }finally{busy=false;}};
}

if(['B01-Q07-R1','B01-Q07-R2','B01-Q09'].includes(review)){
 const button=document.createElement('button');button.textContent='液压缸实际行程审计';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;const checks=[];try{
  for(const entry of inventory.entries){await load(entry.id,entry.suffix);const get=n=>rig.nodes.get(current.presentation.nodes.find(x=>x.name===n)?.id),head=get(review==='B01-Q09'?'StrippingPlate':'DiePlaten'),body=get('HydraulicBarrel');const ys=[];for(const u of[0,.15,.35,.55,.72,1]){pose('auto',u);ys.push(head.position.y);}const bb=new T.Box3().setFromObject(body,true),stroke=Math.max(...ys)-Math.min(...ys),bodyLength=bb.max.y-bb.min.y;checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},actual_stroke_m:stroke,actual_cylinder_body_m:bodyLength,passed:stroke<=bodyLength+1e-6});}
  const passed=checks.every(c=>c.passed);await save('hydraulic-stroke-audit.json',JSON.stringify({passed,actual_product_decoder:true,checks},null,2));status(passed?'液压缸行程审计通过':'液压缸行程审计失败：行程超出缸体总长');
 }finally{busy=false;}};
}
if(review==='B01-Q10'){
 const button=document.createElement('button');button.textContent='热覆膜卷尾与实际膜量复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let attached=[];
  try{
   const{specs,seatX,film,hotY,hotRadius,pathRadius,spoolX,spoolY,fedLength,pathState,filmPoint}=await import(base+'source/batch-spec.mjs');
   const object=(p,r,n)=>r.nodes.get(p.nodes.find(x=>x.name===n)?.id),bounds=o=>new T.Box3().setFromObject(o,true),near=(a,b,label,tol=3e-6)=>{if(Math.abs(a-b)>tol)throw Error(label+': '+a+' vs '+b);};
   const cache=new WeakMap();function geometryVolume(g){if(cache.has(g))return cache.get(g);const p=g.attributes.position,idx=g.index,n=idx?.count??p.count;let sum=0;for(let i=0;i<n;i+=3){const v=[0,1,2].map(j=>new T.Vector3().fromBufferAttribute(p,idx?idx.getX(i+j):i+j));sum+=v[0].dot(v[1].clone().cross(v[2]))/6;}const value=Math.abs(sum);cache.set(g,value);return value;}
   const volume=o=>{let v=0;o.traverse(n=>{if(n.isMesh)v+=geometryVolume(n.geometry)*Math.abs(n.matrixWorld.determinant());});return v;};
   const attach=async(id,anchor)=>{const dir=base+'items/'+id+'/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,data=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer()),d=await decodeGlb(data),r=await bindScene(p,async()=>data,{assetKey:assetKey(p.source.data),decoded:d});rig.nodes.get(current.presentation.anchor_roles[anchor].data).add(r.root);const val={p,r,d};attached.push(val);return val;};
   const apply=(v,name,u)=>{const clip=v.p.timeline.find(c=>c.phase_ref===name);if(!clip)throw Error('missing '+name);v.r.apply({known:true,display_phase_id:name,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],paper=await attach(entry.id+'-input','cargo'),aux=await attach(entry.id+'-auxiliary','aux_seat'),samples=[];
    pose('auto',0);rig.root.updateMatrixWorld(true);
    for(const z of[-.285,.285]){const stem=bounds(object(current.presentation,rig,'NipAdjustScrew'+z)),wheel=bounds(object(current.presentation,rig,'NipAdjustHandwheel'+z));near(stem.min.y,hotY+.045,'adjust stem on bearing');if(stem.max.y<wheel.min.y||stem.max.y>wheel.max.y)throw Error('adjust stem not seated in handwheel');}
    const box=bounds(object(current.presentation,rig,'HeaterTerminalBox')),stand=bounds(object(current.presentation,rig,'HeaterJunctionStandColumn'));near(stand.min.y,.80,'heater stand on base');near(stand.max.y,box.min.y,'heater junction box on stand');
    const probe=(name,r,sign)=>{const o=object(current.presentation,rig,name),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0));return{o,contact,local:o.worldToLocal(contact.clone())};};
    pose('auto',.49);rig.root.updateMatrixWorld(true);const probes=[probe('HeatedLaminationRoll',hotRadius,-1),probe('ImpressionRoll',.060,1)];pose('auto',.49001);rig.root.updateMatrixWorld(true);for(const a of probes)near((a.o.localToWorld(a.local).x-a.contact.x)/.00001,2,'nip tangent co-moving',.001);
    for(const u of[0,.375,.4013,.49,.5,.55,.624,.625,.9,1,.49,0]){
     pose('auto',u);apply(paper,s.initial+':auto',u);apply(aux,'threaded_BOPP_tail:auto',u);rig.root.updateMatrixWorld(true);
     const substrate=bounds(object(paper.p,paper.r,'ActualBoard')),cover=object(paper.p,paper.r,'ActualBondedBOPP'),st=pathState(u),fed=fedLength(u);near(substrate.min.y,s.surface,'board on feed');near(substrate.getCenter(new T.Vector3()).x,seatX(s,'auto',u),'board feed');
     near(cover.scale.x,fed,'actual covered length');if(fed>0){const cb=bounds(cover);near(cb.min.y,s.surface+s.size[1],'film bonded on substrate');near(cb.max.y-cb.min.y,film.thickness,'actual25um film');if(fed<film.length)near(cb.min.x,0,'coverage begins at nip');}
     const remaining=volume(aux.r.root),bonded=volume(cover);near(remaining+bonded,s.volume,'actual film mesh volume balance',s.volume*1e-4);near(remaining,(film.length-fed)*film.width*film.thickness,'actual remaining film',s.volume*1e-4);
     for(let i=0;i<film.cells;i++){const o=object(aux.p,aux.r,'ActualFilmCell'+i);if(o.scale.x<=0)continue;const len=film.length/film.cells,start=i*len,end=(i+1)*len,d=(Math.max(start,st.fed)+end)/2-st.fed,p=filmPoint(d,st),center=o.getWorldPosition(new T.Vector3());near(center.x,p.x,'actual film cell X',.000012);near(center.y,p.y,'actual film cell Y',.000012);}
     near(object(current.presentation,rig,'HeatedLaminationRoll').getWorldPosition(new T.Vector3()).y-hotRadius,s.surface+s.size[1]+film.thickness,'heated nip gap');
     samples.push({u,fed_m:fed,remaining_actual_mesh_m3:remaining,bonded_actual_mesh_m3:bonded,remaining_coil_m:st.coil,all_actual_cells_on_declared_path:true});
     if([0,.5,1].includes(u)&&samples.length<=10){view('iso');await picture(entry.id+'/'+entry.suffix+'-laminate-contact-'+Math.round(u*100)+'.png','实际BOPP卷尾转移 · '+Math.round(u*100)+'%');}
    }
    if(entry.suffix==='detailed')for(const[u,label]of[[0,'threaded-tail'],[.5,'half-covered'],[.625,'fully-laminated']]){pose('auto',u);apply(paper,s.initial+':auto',u);apply(aux,'threaded_BOPP_tail:auto',u);rig.root.updateMatrixWorld(true);const target=new T.Vector3(-.06,1.06,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(-.45,.08,.48));camera.lookAt(target);await picture(entry.id+'/actual-'+label+'.png','实际放卷/热辊/覆膜层 · '+label);}
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,actual_product_decoder:true,co_moving_nip:true});for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}attached=[];
   }
   await save('laminate-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status('2个实际GLB：卷尾/导膜/已覆膜实际体积、辊面方向和支撑通过。');
  }catch(e){console.error(e);await save('laminate-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('覆膜复核失败：'+e);}
  finally{for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}busy=false;}
 };
}
if(review==='B01-Q11'){
 const button=document.createElement('button');button.textContent='裱纸三物料与楞峰裱合复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let attached=[];
  try{
   const{specs,seatX,crestCenters,crestTop,bottomLiner,faceThickness,faceCells,nipX,nipRadius,glueHeight,glueX,facePath,theta,coveredBand}=await import(base+'source/batch-spec.mjs');
   const object=(p,r,n)=>r.nodes.get(p.nodes.find(x=>x.name===n)?.id),bounds=o=>new T.Box3().setFromObject(o,true),near=(a,b,label,tol=3e-6)=>{if(Math.abs(a-b)>tol)throw Error(label+': '+a+' vs '+b);};
   const cache=new WeakMap();function gv(g){if(cache.has(g))return cache.get(g);const p=g.attributes.position,idx=g.index,n=idx?.count??p.count;let sum=0;for(let i=0;i<n;i+=3){const v=[0,1,2].map(j=>new T.Vector3().fromBufferAttribute(p,idx?idx.getX(i+j):i+j));sum+=v[0].dot(v[1].clone().cross(v[2]))/6;}const val=Math.abs(sum);cache.set(g,val);return val;}
   const volume=o=>{let v=0;o.traverse(n=>{if(n.isMesh)v+=gv(n.geometry)*Math.abs(n.matrixWorld.determinant());});return v;};
   const attach=async(id,anchor)=>{const dir=base+'items/'+id+'/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,data=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer()),d=await decodeGlb(data),r=await bindScene(p,async()=>data,{assetKey:assetKey(p.source.data),decoded:d});rig.nodes.get(current.presentation.anchor_roles[anchor].data).add(r.root);const v={p,r,d};attached.push(v);return v;};
   const apply=(v,name,u)=>{const clip=v.p.timeline.find(c=>c.phase_ref===name);if(!clip)throw Error('missing '+name);v.r.apply({known:true,display_phase_id:name,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});};
   for(const entry of inventory.entries){
    await load(entry.id,entry.suffix);const s=specs[entry.id],paper=await attach(entry.id+'-input','cargo'),face=await attach(entry.id+'-component','component_seat'),aux=await attach(entry.id+'-auxiliary','aux_seat'),out=await attach(entry.id+'-output','product_staging');rig.root.updateMatrixWorld(true);const outputVolume=volume(out.r.root);out.r.root.removeFromParent();const samples=[];
    pose('auto',.2);rig.root.updateMatrixWorld(true);
    for(const sign of[-1,1]){const foot=bounds(object(current.presentation,rig,'UpperRearFoot'+sign)),col=bounds(object(current.presentation,rig,'UpperRearColumn'+sign)),bar=bounds(object(current.presentation,rig,'UpperCrossbar0'));near(foot.max.y,col.min.y,'upper rear foot-column');near(col.max.y,bar.min.y,'upper rear column-crossbar');const mount=bounds(object(current.presentation,rig,'UpperFrontMount'+sign)),front=bounds(object(current.presentation,rig,'UpperCrossbar1'));near(mount.min.y,.80,'upper front on machine base');near(mount.max.y,front.min.y,'upper front crossbar seated');}
    const pedestal=bounds(object(current.presentation,rig,'UpperFeedMotorPedestal')),motorFoot=bounds(object(current.presentation,rig,'UpperFeedMotorFoot')),motorMount=bounds(object(current.presentation,rig,'UpperFaceFeedDriveMount'));near(pedestal.min.y,motorFoot.max.y,'upper motor foot');near(pedestal.max.y,motorMount.min.y,'upper motor stand');
    for(const z of[-.27,.27])near(bounds(object(current.presentation,rig,'DoctorStand'+z+'Column')).max.y,bounds(object(current.presentation,rig,'DoctorClamp'+z)).min.y,'doctor clamp support');
    const probes=[['LaminationNip',new T.Vector3(0,-nipRadius,0),new T.Vector3(2,0,0)],['LaminationLower',new T.Vector3(0,.060,0),new T.Vector3(2,0,0)],['UpperFaceRoll0',new T.Vector3(Math.sin(theta)*.019,Math.cos(theta)*.019,0),new T.Vector3(2*Math.cos(theta),-2*Math.sin(theta),0)]];
    const saved=probes.map(([name,r,v])=>{const o=object(current.presentation,rig,name),p=o.getWorldPosition(new T.Vector3()).add(r);return{o,p,local:o.worldToLocal(p.clone()),v};});pose('auto',.20001);rig.root.updateMatrixWorld(true);for(const a of saved){const d=a.o.localToWorld(a.local).sub(a.p).divideScalar(.00001);near(d.x,a.v.x,'actual feed roller X direction',.002);near(d.y,a.v.y,'actual feed roller Y direction',.002);}
    for(const u of[0,.175,.2513,.40,.49,.60,.725,.8,1,.60,0]){
     pose('auto',u);apply(paper,s.initial+':auto',u);apply(face,'loose_face_liner:auto',u);apply(aux,'adhesive_dose_ready:auto',u);rig.root.updateMatrixWorld(true);
     const substrate=bounds(object(paper.p,paper.r,'ActualBottomLiner')),medium=bounds(object(paper.p,paper.r,'ActualCorrugatedMedium'));near(substrate.min.y,s.surface,'lower substrate on belt');near(medium.min.y,substrate.max.y,'corrugated trough on bottom liner');near(medium.max.y,s.surface+crestTop,'actual crest height');near(substrate.getCenter(new T.Vector3()).x,seatX(s,'auto',u),'lower feed');
     for(let i=0;i<faceCells;i++){const p=facePath(seatX(s,'auto',u)+.25-nipX-(i+.5)*.5/faceCells),v=object(face.p,face.r,'ActualFaceCell'+i).getWorldPosition(new T.Vector3());near(v.x,p.x,'actual face material X',.00003);near(v.y,p.y,'actual face material Y',.00003);}
     if(u===0&&samples.length===0){const lands=Array.from({length:50},()=>({min:Infinity,max:-Infinity})),m=object(paper.p,paper.r,'ActualCorrugatedMedium');m.traverse(o=>{if(!o.isMesh)return;const pos=o.geometry.attributes.position;for(let n=0;n<pos.count;n++){const v=new T.Vector3().fromBufferAttribute(pos,n).applyMatrix4(o.matrixWorld);if(Math.abs(v.y-s.surface-crestTop)>2e-7)continue;const x=v.x-seatX(s,'auto',u),i=Math.round((x+.245)/.01);if(i>=0&&i<50){lands[i].min=Math.min(lands[i].min,x);lands[i].max=Math.max(lands[i].max,x);}}});for(const[i,c]of crestCenters.entries())if(lands[i].min>c-.0005+3e-6||lands[i].max<c+.0005-3e-6)throw Error('actual glue band not seated on flat crest '+i);}
     let applied=0;for(const[i,c]of crestCenters.entries()){const band=object(aux.p,aux.r,'ActualAppliedGlue'+i),w=coveredBand(i,s,'auto',u);near(band.scale.x,w,'actual crest glue coverage');if(w>0){const bb=bounds(band);near(bb.min.y,s.surface+crestTop,'glue on crest');near(bb.max.y-bb.min.y,glueHeight,'actual glue thickness');}applied+=volume(band);}
     const auxiliaryVolume=volume(aux.r.root);near(auxiliaryVolume,s.volume,'actual glue dose plus applied bands',s.volume*1e-4);near(volume(paper.r.root)+volume(face.r.root)+auxiliaryVolume,outputVolume,'actual three-source material volume equals output',outputVolume*1e-5);
     near(object(current.presentation,rig,'LaminationNip').getWorldPosition(new T.Vector3()).y-nipRadius,s.surface+crestTop+glueHeight+faceThickness,'final nip gap');
     samples.push({u,actual_three_source_volume_m3:volume(paper.r.root)+volume(face.r.root)+auxiliaryVolume,output_material_volume_m3:outputVolume,actual_glue_volume_m3:auxiliaryVolume,applied_glue_m3:applied,face_cells_on_bending_path:true});
     if([0,.60,1].includes(u)&&samples.length<=9){view('iso');await picture(entry.id+'/'+entry.suffix+'-flute-contact-'+Math.round(u*100)+'.png','实际瓦楞/面纸/胶条裱合 · '+Math.round(u*100)+'%');}
    }
    const transferSamples=[];for(const phase of['load','unload'])for(const u of[0,.5,1]){pose(phase,u);apply(paper,s.initial+':'+phase,u);apply(face,'loose_face_liner:'+phase,u);apply(aux,'adhesive_dose_ready:'+phase,u);rig.root.updateMatrixWorld(true);near(bounds(object(paper.p,paper.r,'ActualBottomLiner')).getCenter(new T.Vector3()).x,seatX(s,phase,u),'load/unload lower material feed');for(let i=0;i<faceCells;i++){const v=object(face.p,face.r,'ActualFaceCell'+i).getWorldPosition(new T.Vector3()),p=facePath(seatX(s,phase,u)+.25-nipX-(i+.5)*.5/faceCells);near(v.x,p.x,'load/unload face X',.00003);near(v.y,p.y,'load/unload face Y',.00003);}near(volume(paper.r.root)+volume(face.r.root)+volume(aux.r.root),outputVolume,'load/unload three material volumes',outputVolume*1e-5);transferSamples.push({phase,u,three_material_transfer_and_volume:true});}
    if(entry.suffix==='detailed'){
     pose('auto',.60);apply(paper,s.initial+':auto',.60);apply(face,'loose_face_liner:auto',.60);apply(aux,'adhesive_dose_ready:auto',.60);rig.root.updateMatrixWorld(true);const target=new T.Vector3(.24,.922,.20);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.10,.007,.16));camera.lookAt(target);await picture(entry.id+'/actual-layer-section.png','实际4.7mm湿胶裱合截面');
     view('iso');const pt=facePath(-1.1),t=new T.Vector3(pt.x,pt.y,0);controls.target.copy(t);camera.position.copy(t).add(new T.Vector3(.45,.25,.65));camera.lookAt(t);await picture(entry.id+'/actual-inclined-face-feed.png','实际倾斜面纸供纸与支撑');
     for(const[name,file]of[['UpperRearFoot1','actual-upper-rear-foot.png'],['UpperFeedMotorFoot','actual-upper-motor-foot.png']]){const t=bounds(object(current.presentation,rig,name)).getCenter(new T.Vector3());controls.target.copy(t);camera.position.copy(t).add(new T.Vector3(.18,.10,.22));camera.lookAt(t);await picture(entry.id+'/'+file,'实际上层送料支腿/电机落地');}
     for(const v of[paper,face,aux])v.r.root.removeFromParent();rig.nodes.get(current.presentation.anchor_roles.product_staging.data).add(out.r.root);rig.root.updateMatrixWorld(true);const ob=bounds(out.r.root);near(ob.max.y-ob.min.y,.0047,'actual final composite total thickness');const targetOut=ob.getCenter(new T.Vector3());controls.target.copy(targetOut);camera.position.copy(targetOut).add(new T.Vector3(.25,.10,.30));camera.lookAt(targetOut);await picture(entry.id+'/actual-new-wet-composite.png','实际新组合纸板：三来源几何一致');const edgeTarget=new T.Vector3(s.stagingX,s.surface+.0022,.20);controls.target.copy(edgeTarget);camera.position.copy(edgeTarget).add(new T.Vector3(.10,.006,.16));camera.lookAt(edgeTarget);await picture(entry.id+'/actual-layer-section.png','实际新组合纸板4.7mm截面');
    }
    checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},samples,transferSamples,actual_flat_crest_glue_contact:true,actual_product_decoder:true,actual_three_source_volume_conserved:true,co_moving_upper_feed_and_nip:true,upper_feed_floor_and_machine_supports:true});for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}attached=[];
   }
   await save('flute-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status('2个实际GLB：双纸路/楞峰胶量/新组合体积、辊向与上层送料支撑通过。');
  }catch(e){console.error(e);await save('flute-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('裱纸复核失败：'+e);}
  finally{for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}busy=false;}
 };
}
if(review==='B01-Q12'){
 const button=document.createElement('button');button.textContent='圆周刀刃与后推清废实物复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let paper;
 try{
 const {specs,shapePoints,cornerPositions,dieX,ruleRadius,shellRadius,anvilRadius,seatX,pusherX,pusherBottom,trayTop,trayZ}=await import(base+'source/batch-spec.mjs');
 const obj=(p,r,n)=>r.nodes.get(p.nodes.find(x=>x.name===n)?.id),bb=o=>new T.Box3().setFromObject(o,true),near=(a,b,n,t=3e-6)=>{if(Math.abs(a-b)>t)throw Error(n+': '+a+' vs '+b);},eq=(a,b,n,t=3e-6)=>{if(a.distanceTo(b)>t)throw Error(n+': '+a.toArray()+' vs '+b.toArray());};
 for(const entry of inventory.entries){
  await load(entry.id,entry.suffix);const s=specs[entry.id],part=n=>{const o=obj(current.presentation,rig,n);if(!o)throw Error('missing '+n);return o;};
  const dir=base+'items/'+entry.id+'-input/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,data=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer()),d=await decodeGlb(data),r=await bindScene(p,async()=>data,{assetKey:assetKey(p.source.data),decoded:d});paper={p,r,d};rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(r.root);
  const apply=(phase,u)=>{pose(phase,u);const clip=p.timeline.find(c=>c.phase_ref==='uncut_sheet:'+phase);r.apply({known:true,display_phase_id:clip.phase_ref,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});rig.root.updateMatrixWorld(true);};
  const bladeSamples=[];
  for(let i=0;i<12;i++){
   const rib=part('RotaryRule'+i),vs=[];rib.traverse(m=>{if(!m.isMesh)return;const v=m.geometry.attributes.position;for(let j=0;j<v.count;j++){const a=new T.Vector3().fromBufferAttribute(v,j);if(Math.abs(Math.hypot(a.x,a.y)-ruleRadius)<2e-7)vs.push({m,a});}});if(vs.length<4)throw Error('no beveled blade tips '+i);
   for(const k of[0,Math.floor(vs.length/2),vs.length-1]){const {m,a}=vs[k],angle=Math.atan2(a.x,-a.y),materialX=angle*ruleRadius,u=.4*((dieX-materialX)+1)/1.1;apply('auto',u);const w=m.localToWorld(a.clone());near(w.x,dieX,'actual tip at nip',.00003);near(w.y,s.surface,'actual tip touches board underside',.00003);const polygonA=shapePoints[i],polygonB=shapePoints[(i+1)%12];if(materialX<Math.min(polygonA[0],polygonB[0])-.00006||materialX>Math.max(polygonA[0],polygonB[0])+.00006||a.z<Math.min(polygonA[1],polygonB[1])-.00006||a.z>Math.max(polygonA[1],polygonB[1])+.00006)throw Error('blade off cut contour '+i);apply('auto',u+.00001);const velocity=m.localToWorld(a.clone()).sub(w).divideScalar(.00001);near(velocity.x,2.75,'upper actual nip rolling direction',.007);bladeSamples.push({rule:i,u,materialX,tip_world:w.toArray(),co_moving_velocity_x:velocity.x});}
  }
  apply('auto',.22);const anvil=part('AnvilCylinder'),point=anvil.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,anvilRadius,0)),local=anvil.worldToLocal(point.clone());apply('auto',.22001);near(anvil.localToWorld(local).sub(point).divideScalar(.00001).x,2.75,'lower actual nip rolling direction',.007);
  const samples=[];
  for(const phase of['load','auto','unload'])for(const u of(phase==='auto'?[0,.25,.40,.425,.45,.50,.60,.70,.74,.76,.80,.84,.90,.98,1,.60,0]:[0,.5,1])){
   apply(phase,u);const stock=bb(obj(p,r,'ActualCrossBlank')),head=bb(part('ReceivingPusherFace')),coupling=bb(part('PusherHeadCoupling')),rod=bb(part('PusherLiftRod')),barrel=bb(part('PusherLiftBarrel'));near(stock.getCenter(new T.Vector3()).x,seatX(s,phase,u),'actual stock X');near(head.min.y,pusherBottom(phase,u),'pusher bottom');near(rod.min.y,coupling.max.y,'rod head coupling');near(rod.max.y,barrel.min.y,'rod barrel connection');if(rod.max.y-rod.min.y>.160+1e-6)throw Error('exposed rod exceeds barrel');
   if(phase==='auto'&&u>=.45&&u<=.70){near(head.max.x,stock.min.x,'pusher engages actual trailing edge');near(head.min.y,stock.min.y,'pusher seated to board');}
   if(phase==='auto'&&u>=.40&&stock.min.x<=dieX+.103)throw Error('pusher engaged before blank clears nip');
   const endpoints=[];for(let i=0;i<32;i++){const hose=part('MovingPusherHose'+i),a=hose.localToWorld(new T.Vector3(0,-.5,0)),b=hose.localToWorld(new T.Vector3(0,.5,0));if(i===0)eq(a,new T.Vector3(.34,1.335,-.37),'moving hose manifold');else eq(a,endpoints.at(-1),'continuous actual moving hose');endpoints.push(b);}eq(endpoints.at(-1),new T.Vector3(pusherX(s,phase,u),1.325,-.027),'moving hose cylinder');
   for(const[i,c]of cornerPositions.entries()){const waste=bb(obj(p,r,'ActualAttachedCorner'+i)),pad=bb(part('CornerSupportPad'+i));if(phase==='auto'&&u>=.70||phase==='unload'){near(waste.min.y,pad.max.y,'actual corner supported');near(waste.getCenter(new T.Vector3()).x,pad.getCenter(new T.Vector3()).x,'corner pad X');near(waste.getCenter(new T.Vector3()).z,pad.getCenter(new T.Vector3()).z,'corner pad Z');}if(phase==='auto'&&u>=.84||phase==='unload'){if(waste.max.y>=s.surface-.03)throw Error('waste not below side guide');}const rail=bb(part('CarrierRailBlock'+i)),lift=bb(part('LiftCrosshead'+i)),cap=bb(part('WasteLiftBarrel'+i));if(Math.abs(trayZ(Math.sign(c[2]),phase,u)-c[2])<1e-6)near(rail.min.y,lift.max.y,'raised carrier on crosshead');for(const dx of[-.025,.025]){const post=bb(part('WasteRailInnerStand'+i+'_'+dx));if(post.intersectsBox(lift))throw Error('fixed post enters moving crosshead');}near(cap.min.y,bb(part('WasteLiftPedestal'+i)).max.y,'lift barrel on pedestal');if(cap.max.y>=lift.max.y-.001)throw Error('lift cap exposed through crosshead');}
   samples.push({phase,u,actual_pusher_rod_and_hose:true,corner_pads_and_clearance:true});
   if(phase==='auto'&&[.25,.50,.80,.98].includes(u)&&samples.length<20){view('iso');await picture(entry.id+'/'+entry.suffix+'-rotary-contact-'+Math.round(u*100)+'.png','实际圆压切/推料/角废承托 '+Math.round(u*100)+'%');}
  }
  if(entry.suffix==='detailed')for(const[n,u,file]of[['DieCylinder',.25,'actual-rotary-steel-rule.png'],['ReceivingPusherHead',.60,'actual-pusher-rod-hose.png'],['CornerCarrier0',.98,'actual-corner-tray-docked.png']]){apply('auto',u);const t=bb(part(n)).getCenter(new T.Vector3());controls.target.copy(t);camera.position.copy(t).add(n==='CornerCarrier0'?new T.Vector3(-.20,.13,-.25):new T.Vector3(.33,.18,.44));camera.lookAt(t);await picture(entry.id+'/'+file,'实际圆周刀刃/推料执行器/清废托盘');}
  checks.push({id:entry.id,lod:entry.suffix,actual_product_decoder:true,bladeSamples,samples,beveled_rules:12,co_moving_nip:true,actual_rod_and_moving_hose:true,actual_corner_support_and_lowered_clearance:true});r.root.removeFromParent();r.dispose();d.dispose();paper=null;
 }
 await save('rotary-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status('2个实际GLB：12段圆周刀刃、同向切点、后推料气缸/随动管及清废承托通过。');
 }catch(e){console.error(e);await save('rotary-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('圆压切复核失败：'+e);}
 finally{if(paper){paper.mount?.removeFromParent();paper.r.root.removeFromParent();paper.r.dispose();paper.d.dispose();}busy=false;}
 };
}
if(review==='B01-Q13'){
 const button=document.createElement('button');button.textContent='糊盒纸路与折指粘合实物复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let attached=[];
 try{
 const {specs,seatX,cellPose,panelPoints,foldAngle,foldRetract,folderOffset,leftRadius,rightRadius,neutral,thickness,glueHeight,glueWidth,glueQ,glueFraction,gluePoint,leftClampBottom,pressBottom,glueHeadX,glueHeadY,samples:times}=await import(base+'source/batch-spec.mjs');
 const ob=(p,r,n)=>r.nodes.get(p.nodes.find(x=>x.name===n)?.id),bb=o=>new T.Box3().setFromObject(o,true),near=(a,b,n,t=3e-6)=>{if(Math.abs(a-b)>t)throw Error(n+': '+a+' vs '+b);},eq=(a,b,n,t=3e-6)=>{if(a.distanceTo(b)>t)throw Error(n+': '+a.toArray()+' vs '+b.toArray());};
 const cache=new WeakMap();function gv(g){if(cache.has(g))return cache.get(g);const pos=g.attributes.position,idx=g.index,n=idx?.count??pos.count;let v=0;for(let i=0;i<n;i+=3){const a=[0,1,2].map(j=>new T.Vector3().fromBufferAttribute(pos,idx?idx.getX(i+j):i+j));v+=a[0].dot(a[1].clone().cross(a[2]))/6;}const r=Math.abs(v);cache.set(g,r);return r;}const volume=o=>{let v=0;o.traverse(m=>{if(m.isMesh)v+=gv(m.geometry)*Math.abs(m.matrixWorld.determinant());});return v;};
 const attach=async(id,anchor)=>{const dir=base+'items/'+id+'/',p=(await fetch(dir+'module.json').then(r=>r.json())).data.definition.payload.data,data=await fetch(dir+'paper.glb').then(r=>r.arrayBuffer()),d=await decodeGlb(data),r=await bindScene(p,async()=>data,{assetKey:assetKey(p.source.data),decoded:d});rig.nodes.get(current.presentation.anchor_roles[anchor].data).add(r.root);const v={p,r,d};attached.push(v);return v;},apply=(v,state,phase,u)=>{const clip=v.p.timeline.find(c=>c.phase_ref===state+':'+phase);v.r.apply({known:true,display_phase_id:clip.phase_ref,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});};
 for(const entry of inventory.entries){
  await load(entry.id,entry.suffix);const s=specs[entry.id],part=n=>{const o=ob(current.presentation,rig,n);if(!o)throw Error('missing '+n);return o;},paper=await attach(entry.id+'-input','cargo'),glue=await attach(entry.id+'-auxiliary','aux_seat'),out=await attach(entry.id+'-output','product_staging');rig.root.updateMatrixWorld(true);const finishedVolume=volume(out.r.root);out.r.root.removeFromParent();const phases=[];
  for(const x of[-1.25,1.25])for(const sign of[-1,1]){const bearing=bb(part('EndBearing'+x+'_'+sign)),mount=bb(part('EndBearingMount'+x+'_'+sign)),beam=bb(part('EndRollCrossbeam'+x));if(sign<0?bearing.max.z>=-.22:bearing.min.z<=.20)throw Error('feed bearing obstructs flat blank');near(bearing.min.y,mount.max.y,'feed bearing on mount');near(mount.min.y,beam.max.y,'feed mount on end crossmember');}near(bb(part('BeltDriveMotorFoot')).min.y,bb(part('EndRollCrossbeam1.25')).max.y,'belt drive motor mounted');if(bb(part('InputTableTop')).max.x> -1.275+1e-6||bb(part('OutputTableTop')).min.x<1.275-1e-6)throw Error('table overlaps exposed belt');
  for(const[name,top]of[['InputGlueCupFloor',bb(part('AuxSupplyTableTop')).max.y],['WorkingGlueCupFloor',bb(part('WorkingGlueStandColumn')).max.y]])near(bb(part(name)).min.y,top,'actual cup seated');
  for(const phase of['load','auto','unload'])for(const u of(phase==='auto'?[0,.04,.09,.10,.175,.25,.30,.39,.48,.53,.58,.62,.70,.78,.83,.88,.90,.94,1,.39,0]:[0,.25,.5,.75,1])){
   pose(phase,u);apply(paper,s.initial,phase,u);apply(glue,'adhesive_dose_ready',phase,u);rig.root.updateMatrixWorld(true);
   near(volume(paper.r.root)+volume(glue.r.root),finishedVolume,'actual board and glue material volume',finishedVolume*2e-5);near(volume(glue.r.root),s.volume,'dose plus applied actual glue',s.volume*2e-5);
   const stock=bb(ob(paper.p,paper.r,'CentralPanel-1'));near(stock.min.y,s.surface,'central board on belt');near(stock.getCenter(new T.Vector3()).x,seatX(s,phase,u),'actual paper indexed X');
   const application=bb(part('GlueNozzle')),lift=bb(part('GlueHeadLift')),body=bb(part('GlueHeadBody'));near(lift.max.y,body.min.y,'actual applicator lift to head');near(lift.min.y,bb(part('GlueHeadCarriage')).max.y,'actual lift seated on rail carriage');
   if(phase==='auto'&&u>=.10&&u<=.25){const band=bb(ob(glue.p,glue.r,'ActualSeamGlue'));near(application.max.y,s.surface-glueHeight,'nozzle under paper');near(band.min.y,application.max.y,'nozzle at applied glue lower face');near(application.getCenter(new T.Vector3()).x,seatX(s,phase,u)-.20+.40*glueFraction(phase,u),'actual glue front follows nozzle');}
   let previous;for(let i=0;i<24;i++){const a=part('MovingGlueHose'+i),p=a.localToWorld(new T.Vector3(0,-.5,0)),q=a.localToWorld(new T.Vector3(0,.5,0));if(i===0)eq(p,new T.Vector3(.24,.84,-.30),'glue hose fixed manifold');else eq(p,previous,'glue hose continuous');previous=q;}eq(previous,new T.Vector3(glueHeadX(phase,u),glueHeadY(phase,u)-.021,-.2305),'glue hose head port');
   for(const side of[-1,1])for(const sign of[-1,1]){
    const prefix='Folder'+side+'_'+sign,unit=part(prefix+'SideUnit'),bearing=bb(part(prefix+'Bearing'));near(unit.getWorldPosition(new T.Vector3()).z,side*folderOffset(phase,u),'whole bearing unit side clearance');near(bb(part(prefix+'UnitShoe')).max.y,bearing.min.y,'unit bearing on slide shoe');near(bb(part(prefix+'UnitMotorFoot')).min.y,bb(part(prefix+'UnitMotorStandColumn')).max.y,'unit feed motor foot seated');const nut=part(prefix+'FingerDriveNut'),cs=part(prefix+'FingerDriveCase'),np=nut.getWorldPosition(new T.Vector3());const face=cs.localToWorld(new T.Vector3(0,.010,0)),normal=new T.Vector3(0,1,0).transformDirection(cs.matrixWorld);near(np.clone().sub(face).dot(normal),.0075,'finger nut seated on enclosed stage',.000008);for(const zz of[.10,.30])near(bb(part(prefix+'GuideStand'+zz+'Column')).max.y,bb(part(prefix+'UnitGuideRail')).min.y,'unit rail on guide stands');
    if(phase!=='auto'){if(side<0?bearing.max.z>=-.22:bearing.min.z<=.20)throw Error('bearing obstructs blank transport');}
    const theta=foldAngle(side,phase,u),finger=part(prefix+'ContactFinger');
    if(phase==='auto'&&u>=.04&&u<=.90&&foldRetract(side,phase,u)<1e-8){
     const r=side<0?leftRadius:rightRadius,q=r*theta+.055,qs=panelPoints(side),i=qs.findIndex((a,i)=>i<qs.length-1&&a<=q&&q<=qs[i+1]),cell=ob(paper.p,paper.r,'ActualFoldCell'+side+'_'+i),a=cellPose(side,qs[i],qs[i+1],theta),point=finger.localToWorld(new T.Vector3(0,.004,0)),local=cell.worldToLocal(point.clone());near(local.y,-a.thickness/2,'fold finger touches actual paper underside',.000008);if(Math.abs(local.z)>a.depth/2+.000008||Math.abs(local.x)>.20)throw Error('finger outside actual contact cell');
    }
   }
   for(const[name,fn]of[['LeftRetainer',leftClampBottom],['SeamPress',pressBottom]]){const shoe=bb(part(name+'Shoe')),coupling=bb(part(name+'Coupling')),rod=bb(part(name+'Rod')),barrel=bb(part(name+'Barrel'));near(shoe.min.y,fn(phase,u),'actual press shoe');near(rod.min.y,coupling.max.y,'press rod lower coupling');near(rod.max.y,barrel.min.y,'press rod barrel');if(rod.max.y-rod.min.y>barrel.max.y-barrel.min.y+1e-6)throw Error('press stroke exceeds barrel');}
   if(phase==='auto'&&u>=.88&&u<=.95){const band=bb(ob(glue.p,glue.r,'ActualSeamGlue')),right=bb(ob(paper.p,paper.r,'ActualFoldCell1_'+(panelPoints(1).length-2))),left=bb(ob(paper.p,paper.r,'ActualFoldCell-1_'+(panelPoints(-1).length-2)));near(band.min.y,left.max.y,'actual glue on folded tab');near(band.max.y,right.min.y,'actual right flap on glue');near(bb(part('SeamPressShoe')).min.y,right.max.y,'actual seam press on upper paper');near(bb(part('LeftRetainerShoe')).min.y,left.max.y,'actual left retainer on paper');}
   phases.push({phase,u,actual_material_volume_m3:volume(paper.r.root)+volume(glue.r.root),actual_glue_volume_m3:volume(glue.r.root),fold_finger_contact_and_unit_transport_clearance:true});
   if(phase==='auto'&&[.175,.39,.70,.88].includes(u)&&phases.length<30){view('iso');await picture(entry.id+'/'+entry.suffix+'-folder-contact-'+Math.round(u*100)+'.png','实际涂胶/左右折翼/接缝压合 '+Math.round(u*100)+'%');}
  }
  if(entry.suffix==='detailed'){
   for(const[u,sign,file]of[[.39,-1,'actual-left-flap-fold.png'],[.70,1,'actual-right-flap-fold.png']]){pose('auto',u);apply(paper,s.initial,'auto',u);apply(glue,'adhesive_dose_ready','auto',u);rig.root.updateMatrixWorld(true);const target=new T.Vector3(0,s.surface+.045,0);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.55,.32,sign*.60));camera.lookAt(target);await picture(entry.id+'/'+file,'实际预压痕纸坯与折指接触');}
   pose('auto',.88);apply(paper,s.initial,'auto',.88);apply(glue,'adhesive_dose_ready','auto',.88);rig.root.updateMatrixWorld(true);let target=new T.Vector3(.20,s.surface+.0015,gluePoint('unload',1).z);controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.025,.003,.018));camera.lookAt(target);await picture(entry.id+'/actual-seam-layer-section.png','实际0.1mm胶层与两折翼接缝');
   for(const v of[paper,glue])v.r.root.removeFromParent();rig.nodes.get(current.presentation.anchor_roles.product_staging.data).add(out.r.root);rig.root.updateMatrixWorld(true);view('iso');target=bb(out.r.root).getCenter(new T.Vector3());controls.target.copy(target);camera.position.copy(target).add(new T.Vector3(.30,.15,.35));camera.lookAt(target);await picture(entry.id+'/actual-finished-flat-sleeve.png','实际新物品：有弯曲折痕的折平盒筒');
  }
  checks.push({id:entry.id,lod:entry.suffix,actual_product_decoder:true,phases,actual_two_source_volume_conserved:true,actual_glue_contact:true,actual_fold_finger_contact:true,whole_unit_transport_clearance:true});for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}attached=[];
 }
 await save('folder-contact-checks.json',JSON.stringify({passed:true,actual_product_decoder:true,models:checks.length,checks},null,2));status('2个实际GLB：纸板/胶量、折指接触、整组退让、压合/气管和新盒筒通过。');
 }catch(e){console.error(e);await save('folder-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('糊盒复核失败：'+e);}
 finally{for(const v of attached){v.r.root.removeFromParent();v.r.dispose();v.d.dispose();}busy=false;}
 };
}
if(review==='B01-Q14'){
 const button=document.createElement('button');button.textContent='锁底工具与有限纸材实接触复核';document.querySelector('header').append(button);
 button.onclick=async()=>{if(busy)return;busy=true;playing=false;const checks=[];let paper;
 try{
 const S=await import(base+'source/batch-spec.mjs'),M=await import(base+'source/lock-tool-motion.mjs'),{finiteCoreAnchor}=await import(base+'source/finite-scored-bands.mjs'),dir='/.work/native500-quality-20261025/b01q14/finite-prototype-02/',p=(await fetch(dir+'presentation.json').then(r=>r.json())),bytes=await fetch(dir+'finite-lock-stock.glb').then(r=>r.arrayBuffer()),near=(a,b,n,t=3e-6)=>{if(Math.abs(a-b)>t)throw Error(n+': '+a+' vs '+b);},eq=(a,b,n,t=3e-6)=>{if(a.distanceTo(b)>t)throw Error(n+': '+a.distanceTo(b));},bb=o=>new T.Box3().setFromObject(o,true);
 for(const entry of inventory.entries){
  await load(entry.id,entry.suffix);const ids=new Map(current.presentation.nodes.map(n=>[n.name,n.id])),part=n=>{const o=rig.nodes.get(ids.get(n));if(!o)throw Error('missing '+n);return o;};
  const d=await decodeGlb(bytes),r=await bindScene(p,async()=>bytes,{assetKey:assetKey(p.source.data),decoded:d}),pids=new Map(p.nodes.map(n=>[n.name,n.id]));const mount=new T.Group();mount.position.x=S.paperShiftX;rig.nodes.get(current.presentation.anchor_roles.cargo.data).add(mount);mount.add(r.root);paper={r,d,mount};
  const paperPoint=(kind,index,piece,q)=>r.nodes.get(pids.get(kind==='body'?'BodyPanel'+index:'Bottom'+index+'_'+piece)).localToWorld(q.clone().sub(finiteCoreAnchor({kind,index,piece}))),apply=(ph,u)=>{pose(ph,u);const clip=p.timeline[0],v=S.processU(ph,u);r.apply({known:true,display_phase_id:'manufacture',effective_ms:v*1000,duration_ms:1000,progress:v,terminal_pose:false},{clip,progress:v});rig.root.updateMatrixWorld(true);};
  const rows=[];
  for(const phase of['load','auto','unload'])for(const u of(phase==='auto'?[0,.02,.05,.06,.075,.12,.17,.20,.209,.21,.22,.24,.26,.27,.28,.31,.35,.39,.42,.439,.44,.465,.49,.51,.52,.60,.68,.695,.71,.72,.84,.88,.92,.94,.96,1,.12,0]:[0,.5,1])){
   apply(phase,u);let bottomContacts=0,diagonalContacts=0;
   for(const index of[0,1,2,3])if(M.bottomHookState(index,phase,u).contact){const c=M.bottomContact(index),expected=paperPoint('bottom',index,'main',c.point.clone().add(new T.Vector3(0,-S.carton.paperThickness/2,0))),got=part('BottomHook'+index+'_ContactSpoon').localToWorld(new T.Vector3(0,.001,0));eq(got,expected,'actual bottom hook contact '+index);bottomContacts++;}
   for(const index of[0,2])if(M.diagonalToolState(index,phase,u).contact){const c=M.diagonalContact(index),expected=paperPoint('bottom',index,'ear',c.point.clone().add(new T.Vector3(0,-S.carton.paperThickness/2,0))),got=part('Diagonal'+index+'_ThinSpatula').localToWorld(new T.Vector3(-.075,.0002,0));eq(got,expected,'actual thin spatula contact '+index);diagonalContacts++;}
   if(phase==='auto'&&u>=.209&&u<=.49)for(const index of[0,1,2,3]){const expected=paperPoint('bottom',index,'main',new T.Vector3(-.030,-S.carton.paperThickness/2,(index-2)*S.carton.side+(index===2?.050:.025)));near(bb(part('BottomRetainer'+index+'_RetainingShoe')).min.y,expected.y,'actual keeper on flipped paper upper face');}
   for(const index of[0,2]){const prefix='UpperGlue_bottom_'+index;near(bb(part(prefix+'_PadStem')).min.y,bb(part(prefix+'_ApplicatorPad')).max.y,'actual pad-stem contact');near(bb(part(prefix+'_PadStem')).max.y,bb(part(prefix+'_Coupling')).min.y,'actual pad stem-coupling contact');const fitting=part(prefix+'_FeedConnector');eq(fitting.localToWorld(new T.Vector3(0,0,0)),part(prefix+'_PadStem').localToWorld(new T.Vector3(.008,-.002,0)),'actual upper feed fitting/stem');eq(part(prefix+'_SupplyHose23').localToWorld(new T.Vector3(0,.5,0)),fitting.localToWorld(new T.Vector3(.006,0,0)),'actual upper hose/fitting');eq(part(prefix+'_SupplyHose0').localToWorld(new T.Vector3(0,-.5,0)),part(prefix+'_HeaderElbow').localToWorld(new T.Vector3(.004,0,0)),'actual upper hose/header elbow');for(let i=0;i<23;i++)eq(part(prefix+'_SupplyHose'+i).localToWorld(new T.Vector3(0,.5,0)),part(prefix+'_SupplyHose'+(i+1)).localToWorld(new T.Vector3(0,-.5,0)),'actual upper hose segment continuity');}
   if(phase==='auto'&&u>=.439&&u<=.49)for(const index of[0,2]){const patch=S.glueParts.find(p=>p.role==='bottom_'+index),c=patch.points.reduce((a,p)=>a.add(p),new T.Vector3()).multiplyScalar(1/patch.points.length),top=paperPoint('bottom',index,'ear',c.add(new T.Vector3(0,S.carton.paperThickness/2,0)));near(bb(part('UpperGlue_bottom_'+index+'_ApplicatorPad')).min.y-top.y,S.carton.glueThickness,'actual planned bottom adhesive gap');}
   for(const name of['BottomHook0','BottomHook1','BottomHook2','BottomHook3','Diagonal0','Diagonal2']){
    const rod=bb(part(name+'_YRod')),flange=bb(part(name+'_BedFlange')),barrel=bb(part(name+'_YBarrel')),plate=bb(part(name+'_YCarriagePlate'));near(rod.min.y,barrel.max.y,'actual Y rod-barrel');near(rod.max.y,plate.min.y,'actual Y rod-carriage');near(flange.min.y,barrel.max.y,'barrel mounted under bed');near(flange.max.y,name.startsWith('BottomHook')?.74:.51,'flange at declared support face');if(rod.max.y-rod.min.y>barrel.max.y-barrel.min.y+1e-6)throw Error('Y stroke exceeds barrel');
    if(phase!=='auto'){for(const node of['_RotatingTool','_RotaryMotor']){const b=bb(part(name+node)),y=S.specs[entry.id].surface;if(b.max.y>y-.001&&b.min.y<y+.0015)throw Error('parked tool intersects transport clearance '+name+node);}}
   }
   eq(part('MovingGlueHose0').localToWorld(new T.Vector3(0,-.5,0)),new T.Vector3(.40,.83,-.19),'actual lower hose fixed riser');eq(part('MovingGlueHose19').localToWorld(new T.Vector3(0,.5,0)),part('LowerGlueFeedConnector').localToWorld(new T.Vector3(.006,0,0)),'actual lower hose/fitting');for(let i=0;i<19;i++)eq(part('MovingGlueHose'+i).localToWorld(new T.Vector3(0,.5,0)),part('MovingGlueHose'+(i+1)).localToWorld(new T.Vector3(0,-.5,0)),'actual lower hose segment continuity');for(const name of['BottomHook0','BottomHook1','BottomHook2','BottomHook3','Diagonal0','Diagonal2']){const index=Number(name.at(-1)),bottom=name.startsWith('BottomHook'),minor=bottom&&index%2===0,out=index<2?-1:1,cap=(n,top)=>part(n).localToWorld(new T.Vector3(0,top?.5:-.5,0)),end=(suffix)=>part(name+suffix).localToWorld(new T.Vector3(0,0,out*.006));
    eq(cap(name+'_LiftPower0',false),end('_LiftPowerOutlet'),'actual XYZ lift fixed supply');eq(cap(name+'_LiftPower26',true),end('_XBodyPowerIn'),'actual XYZ lift/body feed');eq(cap(name+'_TraversePower0',false),end('_XBodyPowerOut'),'actual XYZ traverse start');eq(cap(name+'_TraversePower26',true),end('_CarriagePowerPort'),'actual XYZ traverse/carriage feed');for(const prefix of['_LiftPower','_TraversePower'])for(let i=0;i<26;i++)eq(cap(name+prefix+i,true),cap(name+prefix+(i+1),false),'actual XYZ cable continuity '+name+prefix+i+' '+phase+':'+u);
    const motor=bb(part(name+'_RotaryMotor')),mount=bb(part(name+'_RotaryMount')),port=part(name+'_RotaryPowerPort').localToWorld(new T.Vector3());near(port.y,bottom&&!minor?motor.max.y:motor.min.y,'actual protected rotary power inlet');if(!minor)near(bottom?mount.min.y:mount.max.y,bottom?motor.max.y:motor.min.y,'actual rotary mounting case/motor');else{near(bb(part(name+'_PowerPickup')).max.x,mount.min.x,'actual vertical carrier pickup/mount');near(bb(part(name+'_EnergyChannelHead')).max.x,bb(part(name+'_ToolCarriage')).min.x,'actual energy channel/carriage');eq(cap(name+'_ToolYPower0',false),end('_CarriagePowerPort'),'actual tool Y feed start');eq(cap(name+'_ToolYPower32',true),part(name+'_ToolYPowerPort').localToWorld(new T.Vector3(0,-.006,0)),'actual tool Y motor feed');for(let i=0;i<32;i++)eq(cap(name+'_ToolYPower'+i,true),cap(name+'_ToolYPower'+(i+1),false),'actual tool Y cable continuity');}
   }
   for(const side of[-1,1])for(const sign of[-1,1]){const prefix='Folder'+side+'_'+sign;eq(part(prefix+'_MovingPower0').localToWorld(new T.Vector3(0,-.5,0)),new T.Vector3(sign*.52,1.21,side*1.43),'actual folder fixed feed');eq(part(prefix+'_MovingPower41').localToWorld(new T.Vector3(0,.5,0)),part(prefix+'_MovingMotorPowerPort').localToWorld(new T.Vector3(sign*.006,0,0)),'actual folder moving motor feed');for(let i=0;i<41;i++)eq(part(prefix+'_MovingPower'+i).localToWorld(new T.Vector3(0,.5,0)),part(prefix+'_MovingPower'+(i+1)).localToWorld(new T.Vector3(0,-.5,0)),'actual folder moving cable continuity');eq(part(prefix+'_RotarySlipRing').localToWorld(new T.Vector3(.006,0,0)),part(prefix+'Motor').localToWorld(new T.Vector3()),'actual slip ring coaxial with motor');eq(part(prefix+'_FingerMotorPowerPort').localToWorld(new T.Vector3()),part(prefix+'FingerFeedMotor').localToWorld(new T.Vector3(0,-sign*.025,0)),'actual rotating finger motor port');}
   for(const prefix of[...Array.from({length:4},(_,i)=>'BottomRetainer'+i),'UpperGlue_bottom_0','UpperGlue_bottom_2']){near(part(prefix+'_AirPort').localToWorld(new T.Vector3()).x,bb(part(prefix+'_Barrel')).max.x,'actual cylinder air inlet/barrel');near(part(prefix+'_ValvePort').localToWorld(new T.Vector3()).y,bb(part('AirValveManifold')).max.y,'actual valve outlet on manifold');}
   eq(part('LowerLiftAirHose0').localToWorld(new T.Vector3(0,-.5,0)),new T.Vector3(.84,.68,-.45),'actual lower lift fixed air supply');eq(part('LowerLiftAirHose19').localToWorld(new T.Vector3(0,.5,0)),part('LowerLiftAirPort').localToWorld(new T.Vector3(0,0,-.006)),'actual moving lift air fitting');for(let i=0;i<19;i++)eq(part('LowerLiftAirHose'+i).localToWorld(new T.Vector3(0,.5,0)),part('LowerLiftAirHose'+(i+1)).localToWorld(new T.Vector3(0,-.5,0)),'actual lift air hose continuity');near(part('GlueAxisPowerPort').localToWorld(new T.Vector3()).x,bb(part('GlueAxisMotor')).max.x,'actual glue-axis power inlet/motor');
   rows.push({phase,u,bottomContacts,diagonalContacts,actual_y_stage_endpoints:true});
   if(phase==='auto'&&[.12,.35,.465,.60,.84,.96].includes(u)&&rows.length<43){view('iso');await picture(entry.id+'/'+entry.suffix+'-lock-contact-'+Math.round(u*1000)+'.png','实际底钩/斜折/上胶/侧折 '+Math.round(u*100)+'%');}
  }
  if(entry.suffix==='detailed')for(const[name,u,file]of[['BottomHook0_ContactSpoon',.12,'actual-bottom-hook.png'],['Diagonal0_ThinSpatula',.35,'actual-thin-spatula.png'],['UpperGlue_bottom_0_ApplicatorPad',.465,'actual-upper-glue-pad.png']]){apply('auto',u);const t=bb(part(name)).getCenter(new T.Vector3());controls.target.copy(t);camera.position.copy(t).add(name.startsWith('BottomHook')?new T.Vector3(-.25,.12,-.20):name.startsWith('Diagonal')?new T.Vector3(-.25,.13,-.25):new T.Vector3(-.13,.035,-.16));camera.lookAt(t);await picture(entry.id+'/'+file,'实际锁底工具与有限纸材');}
  if(entry.suffix==='detailed')for(const[name,u,file,offset]of[['Folder-1_1_MovingMotorPowerPort',.60,'actual-folder-following-power.png',[.50,.15,-.60]],['BottomHook0_PowerPickup',.12,'actual-minor-carrier-and-saddle.png',[-.40,.20,-.30]],['BottomHook1_RotaryMount',.12,'actual-major-closed-feed.png',[-.20,.13,.20]],['Diagonal0_RotaryMount',.35,'actual-diagonal-closed-feed.png',[.20,.17,.23]]]){apply('auto',u);const t=bb(part(name)).getCenter(new T.Vector3());controls.target.copy(t);camera.position.copy(t).add(new T.Vector3(...offset));camera.lookAt(t);await picture(entry.id+'/'+file,'实际执行器供给与随动连接');}
  checks.push({id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.version},bytes:entry.bytes,nodes:entry.nodes,actual_product_decoder:true,rows,paper_reference:'finite-prototype-02',actual_bottom_hook_contact:true,actual_thin_spatula_contact:true,actual_retaining_and_planned_glue_gap:true,actual_six_cylinder_air_ports:true,actual_lower_lift_air_continuity:true,actual_glue_axis_motor_power_port:true,actual_four_folder_following_feeds:true,actual_six_XYZ_feed_chains:true,Native:'not_run'});paper.mount.removeFromParent();r.root.removeFromParent();r.dispose();d.dispose();paper=null;
 }
 await save('lock-tool-contact-checks.json',JSON.stringify({passed:true,scope:'actual equipment GLB and finite paper component, cylinder ports and moving adhesive/air endpoints; final Host/Native pending',models:checks.length,checks},null,2));status('双档实际锁底工具/有限纸材接触、保持夹/胶隙及升降机构通过；Native待验。');
 }catch(e){console.error(e);await save('lock-tool-contact-error.json',JSON.stringify({passed:false,checks,error:String(e)},null,2));status('锁底工具复核失败：'+e);}
 finally{if(paper){paper.mount?.removeFromParent();paper.r.root.removeFromParent();paper.r.dispose();paper.d.dispose();}busy=false;}
 };
}
