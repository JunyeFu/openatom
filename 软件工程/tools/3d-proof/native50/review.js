import * as T from 'three';
import {OrbitControls} from 'three/addons/controls/OrbitControls.js';
import {RoomEnvironment} from 'three/addons/environments/RoomEnvironment.js';
import {decodeGlb} from '../../../packages/presentation/src/glb.ts';
import {bindScene} from '../../../packages/presentation/src/scene.ts';
import {assetKey} from '../../../packages/presentation/src/types.ts';
const $=id=>document.getElementById(id),base='/modules/media/native-equipment-50/';
const inventory=await fetch(base+'inventory.json').then(r=>r.json());
const detailed=inventory.entries.filter(e=>e.suffix==='detailed');
for(const e of detailed)$('model').add(new Option(`${String(e.index).padStart(2,'0')} ${e.title}`,e.id));
const renderer=new T.WebGLRenderer({antialias:true,preserveDrawingBuffer:true,powerPreference:'high-performance'});renderer.setPixelRatio(1);renderer.setSize(1600,1000,false);renderer.outputColorSpace=T.SRGBColorSpace;renderer.toneMapping=T.ACESFilmicToneMapping;renderer.toneMappingExposure=.85;renderer.shadowMap.enabled=true;renderer.shadowMap.type=T.PCFShadowMap;$('viewport').append(renderer.domElement);
const scene=new T.Scene();scene.background=new T.Color('#e4e9ee');const environment=new T.PMREMGenerator(renderer).fromScene(new RoomEnvironment(),.04);scene.environment=environment.texture;scene.environmentIntensity=.75;scene.add(new T.HemisphereLight(0xf2f7ff,0x77848d,1.15));
const light=new T.DirectionalLight('#ffffff',2.3);light.position.set(5,9,5);light.castShadow=true;light.shadow.mapSize.set(2048,2048);light.shadow.normalBias=.001;Object.assign(light.shadow.camera,{left:-5,right:5,top:5,bottom:-5,near:.1,far:40});light.shadow.camera.updateProjectionMatrix();scene.add(light,light.target);
const floor=new T.Mesh(new T.PlaneGeometry(100,100),new T.MeshStandardMaterial({color:'#cfd8df',roughness:.8}));floor.rotation.x=-Math.PI/2;floor.position.y=-.002;floor.receiveShadow=true;scene.add(floor);
const grid=new T.GridHelper(40,80,0xb0bdc6,0xc0ccd4);grid.position.y=-.001;scene.add(grid);
const camera=new T.PerspectiveCamera(36,1.6,.001,150),controls=new OrbitControls(camera,renderer.domElement);controls.maxPolarAngle=Math.PI;
let rig,decoded,current,playing=false,busy=false,zero=0,bounds=new T.Box3(),center=new T.Vector3(),size=new T.Vector3();
const status=s=>$('status').textContent=s;
function pose(phase,u){const clip=current.presentation.timeline.find(c=>c.phase_ref===phase);if(!clip)throw Error(current.id+' missing '+phase);rig.apply({known:true,display_phase_id:phase,effective_ms:u*1000,duration_ms:1000,progress:u,terminal_pose:false},{clip,progress:u});}
function view(which='iso'){
 $('view').value=which;const dirs={iso:[1,.7,1],front:[0,.03,1],back:[0,.03,-1],left:[-1,.03,0],right:[1,.03,0],top:[.001,1,.001],bottom:[.001,-1,.001],closeup:[1,.65,1]};
 if(which!=='closeup'){bounds.setFromObject(rig.root);bounds.getCenter(center);bounds.getSize(size);}
 let target=center.clone(),extent=Math.max(...size.toArray())/2;
 if(which==='closeup'){const focus=rig.root.getObjectByName(current.focus_node);if(focus){target=focus.getWorldPosition(new T.Vector3());extent=Math.max(.2,extent*.40);}}
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
 current=inventory.entries.find(e=>e.id===id&&e.suffix===lod);const response=await fetch(base+current.directory+'/'+current.asset.path);if(!response.ok)throw Error('Missing '+current.asset.path);const bytes=await response.arrayBuffer();
 const selected=$('phase').value;$('phase').replaceChildren(...current.phase_ids.map(id=>new Option(id,id)));$('phase').value=current.phase_ids.includes(selected)?selected:current.primary_phase;
 decoded=await decodeGlb(bytes);rig=await bindScene(current.presentation,async()=>bytes,{assetKey:assetKey(current.asset),decoded});scene.add(rig.root);
 rig.root.traverse(o=>{if(o.isMesh){o.castShadow=true;o.receiveShadow=true;}});bounds.setFromObject(rig.root);bounds.getCenter(center);bounds.getSize(size);
 if(size.toArray().some(x=>!Number.isFinite(x)||x<=0))throw Error('Invalid geometry bounds '+id);
 pose($('phase').value,Number($('progress').value)/100);view($('view').value);
 $('details').textContent=`${current.index} / 50 · ${current.title} · ${lod}\n${current.flow}\n定义 ${current.definition_id}@${current.version} · ${current.rule} · 声明周期${current.cycle_seconds}s\n${current.nodes}节点 / ${current.triangles.toLocaleString()}三角面 / ${(current.bytes/1048576).toFixed(2)}MiB · ${current.basis}`;
}
const controlsChanged=async()=>{if(busy)return;try{await load($('model').value,$('lod').value);status('已读取实际GLB；可旋转、查看背底和动作。');}catch(e){status(String(e));console.error(e);}};
$('model').onchange=controlsChanged;$('lod').onchange=controlsChanged;$('view').onchange=()=>view($('view').value);
function seek(){if(busy)return;playing=false;pose($('phase').value,Number($('progress').value)/100);if($('view').value==='closeup')view('closeup');else draw();}
$('phase').onchange=seek;$('progress').oninput=seek;$('play').onclick=()=>{playing=!playing;zero=performance.now()-Number($('progress').value)*40;$('play').textContent=playing?'暂停动作':'播放声明动作';};
function animate(t){requestAnimationFrame(animate);if(busy)return;if(playing){const u=((t-zero)%4000)/4000;$('progress').value=String(Math.round(u*100));pose($('phase').value,u);}controls.update();if(rig)draw();}requestAnimationFrame(animate);
const save=async(name,data)=>{const r=await fetch('/native50-save/'+name,{method:'POST',body:data});if(!r.ok)throw Error('Capture write failed '+name);};
function signature(){return [...rig.nodes].map(([id,o])=>[id,...o.position.toArray(),...o.quaternion.toArray(),...o.scale.toArray()]);}
function close(a,b){return a.every((row,i)=>row.every((v,j)=>Math.abs(v-b[i][j])<1e-7));}
async function picture(name,title){
 draw();const canvas=document.createElement('canvas');canvas.width=1600;canvas.height=1000;const ctx=canvas.getContext('2d');ctx.drawImage(renderer.domElement,0,0);ctx.fillStyle='rgba(19,42,57,.90)';ctx.fillRect(20,20,1560,86);ctx.fillRect(20,936,1560,44);ctx.fillStyle='#fff';ctx.font='bold 26px "Microsoft YaHei",sans-serif';ctx.fillText(`${String(current.index).padStart(2,'0')}  ${current.title}  ·  ${title}`,40,57);ctx.font='18px "Microsoft YaHei",sans-serif';ctx.fillStyle='#bfd1df';ctx.fillText(`${current.definition_id}@${current.version}  |  ${current.suffix}`,40,88);ctx.fillText('实际GLB重开 · 当前产品解码与场景绑定 · 原创示例尺寸/工时 · 声明预览，不产生业务结果',40,964);await save(name,await new Promise(resolve=>canvas.toBlob(resolve,'image/png')));
}
$('capture').onclick=async()=>{
 if(busy)return;busy=true;playing=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=true);const checks=[];
 try{
  for(const entry of inventory.entries){
   $('model').value=entry.id;$('lod').value=entry.suffix;$('phase').value='auto';$('progress').value='0';
   status(`实际读取 ${checks.length+1}/100 · ${entry.title} / ${entry.suffix}`);await load(entry.id,entry.suffix);
   const row={id:entry.id,lod:entry.suffix,module_ref:{id:entry.module_id,version:entry.media_version??entry.version},bytes:entry.bytes,nodes:entry.nodes,phase_ids:entry.phase_ids,decode:true,phase_seek:true,reverse_seek:true,moving_nodes:[],captures:[]};
   const motions=new Set();for(const phase of entry.phase_ids){
    pose(phase,0);const a=signature();pose(phase,.5);const b=signature();pose(phase,1);pose(phase,.5);if(!close(b,signature()))throw Error(entry.id+' reverse seek mismatch');pose(phase,0);if(!close(a,signature()))throw Error(entry.id+' return-to-start mismatch');
    for(let i=0;i<a.length;i++)if(!close([a[i]],[b[i]]))motions.add(a[i][0]);
   }
   if(!motions.size)throw Error(entry.id+' has no actual part motion');row.moving_nodes=[...motions];
   for(const direction of ['iso','back','bottom']){pose(entry.primary_phase,.5);view(direction);const name=entry.id+'/'+entry.suffix+'-'+direction+'.png';await picture(name,direction);row.captures.push(name);}
   if(entry.suffix==='detailed')for(const u of [0,.5,1]){pose(entry.primary_phase,u);view('closeup');const name=entry.id+'/action-'+Math.round(u*100)+'.png';await picture(name,'机构近景 · '+entry.primary_phase+' '+Math.round(u*100)+'%');row.captures.push(name);}
   checks.push(row);await save('progress.json',JSON.stringify({completed:checks.length,total:100,current:entry.id}));
  }
  await save('render-checks.json',JSON.stringify({scope:'actual_export_GLB_decoded_and_bound_by_product_presentation_declared_preview',device_count:50,lod_models:checks.length,checks,native_actual_run_wait_save_reopen:'not_executed_by_asset_reviewer'},null,2));status('完成：50种设备 / 100个GLB实际读取、各声明阶段与反向seek通过；450张实际模型图已保存。');
 }catch(e){console.error(e);await save('render-error.json',JSON.stringify({completed:checks.length,error:String(e)}));status('检查失败：'+String(e));}
 finally{busy=false;document.querySelectorAll('header select,header input,header button').forEach(e=>e.disabled=false);view('iso');}
};
await controlsChanged();
