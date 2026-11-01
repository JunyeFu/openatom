import * as T from 'three';
import {OrbitControls} from 'three/addons/controls/OrbitControls.js';
import {RoomEnvironment} from 'three/addons/environments/RoomEnvironment.js';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {decodeGlb} from '../../../packages/presentation/src/glb.ts';
import {bindScene} from '../../../packages/presentation/src/scene.ts';
import {assetKey} from '../../../packages/presentation/src/types.ts';
const $=id=>document.getElementById(id),base='/modules/media/equipment-quality-v042/';
const inventory=await fetch(base+'inventory.json',{cache:'no-store'}).then(r=>r.json()),models=inventory.entries.filter(v=>v.suffix==='detailed');
const qualityLedger=await fetch('/modules/media/model-quality-ledger.json',{cache:'no-store'}).then(r=>r.json());
const qualityByPath=new Map(qualityLedger.models.map(row=>[row.model_path,row]));
const names={assembly:'装配台',rework:'返工台',packing:'装箱台',robot:'六轴机器人',conveyor:'输送机',inspection:'检验台',sealer:'封箱机',pallet:'托盘','control-box':'控制盒',carton:'纸箱',shell:'壳体',pcb:'PCB',fastener:'螺钉','sealing-tape':'胶带卷'};
for(const entry of models){const o=new Option(names[entry.family],entry.family);$('model').add(o);}
const renderer=new T.WebGLRenderer({antialias:true,preserveDrawingBuffer:true,powerPreference:'high-performance'});
renderer.setSize(1920,1080,false);renderer.setPixelRatio(1);renderer.outputColorSpace=T.SRGBColorSpace;renderer.toneMapping=T.ACESFilmicToneMapping;renderer.toneMappingExposure=.9;renderer.shadowMap.enabled=true;renderer.shadowMap.type=T.VSMShadowMap;$('viewport').append(renderer.domElement);
const scene=new T.Scene();scene.background=new T.Color('#e4e9ee');
const pmrem=new T.PMREMGenerator(renderer);scene.environment=pmrem.fromScene(new RoomEnvironment(),.04).texture;scene.environmentIntensity=.75;
scene.add(new T.HemisphereLight(0xf1f7ff,0x7c8690,1.15));
const sun=new T.DirectionalLight(0xffffff,2.2);sun.position.set(8,16,5);sun.castShadow=true;sun.shadow.mapSize.set(2048,2048);sun.shadow.normalBias=.001;sun.shadow.radius=2;sun.shadow.blurSamples=8;scene.add(sun,sun.target);
const floor=new T.Mesh(new T.PlaneGeometry(200,200),new T.MeshStandardMaterial({color:'#dce3e9',roughness:.9,metalness:0}));floor.rotation.x=-Math.PI/2;floor.receiveShadow=true;floor.position.y=-.001;scene.add(floor);
const camera=new T.PerspectiveCamera(36,16/9,.0001,500);const controls=new OrbitControls(camera,renderer.domElement);controls.mouseButtons={LEFT:T.MOUSE.ROTATE,MIDDLE:T.MOUSE.PAN,RIGHT:T.MOUSE.ROTATE};controls.maxPolarAngle=Math.PI;
let rig=null,decoded=null,current=null,center=new T.Vector3(),size=new T.Vector3(),radius=1,busy=false,playing=false,zero=0;
const results=[];
const phaseSelect=document.createElement('select');phaseSelect.id='phase';phaseSelect.setAttribute('aria-label','声明阶段');
const phaseProgress=document.createElement('input');phaseProgress.id='phase-progress';phaseProgress.type='range';phaseProgress.min='0';phaseProgress.max='100';phaseProgress.value='0';phaseProgress.setAttribute('aria-label','声明阶段进度');
document.querySelector('header').append(phaseSelect,phaseProgress);
function drawPreview(){if($('view').value==='closeup')view('closeup');else draw();}
function seekPreview(){if(busy)return;pose(phaseSelect.value,Number(phaseProgress.value)/100);drawPreview();status(`声明预览 ${phaseSelect.value} · ${phaseProgress.value}%（非生产结果）`);}
phaseSelect.onchange=seekPreview;phaseProgress.oninput=seekPreview;
const tick=()=>new Promise(resolve=>requestAnimationFrame(resolve));
function status(text){$('status').textContent=text;}
async function post(route,data,query=''){const r=await fetch('/quality-api/'+route+query,{method:'POST',body:data});const value=await r.json();if(!r.ok)throw Error(value.error);return value;}
function pose(phase,u){
 const clip=current.presentation.timeline.find(c=>c.phase_ref===phase);
 rig.apply(clip?{known:true,display_phase_id:phase,progress:u}:null,clip?{clip,progress:u}:null);
}
function view(name,paint=true){
 $('view').value=name;
 const directions={isometric:current.family==='robot'?[-1,.7,1]:[1,.65,1],front:[0,0,1],back:[0,0,-1],left:[-1,0,0],right:[1,0,0],top:[0,1,0],bottom:[0,-1,0],closeup:current.family==='robot'?[-1,.7,-1.2]:[1,.65,1],actuator:[1,.55,1]};
 let target=center.clone(),r=radius;
 if(name==='closeup'){
  const nodeNames=current.suffix==='reference'?{assembly:'electric-driver',robot:'axis6-tool-mount',conveyor:'belt-top',inspection:'industrial-camera',sealer:'tape-reel',pallet:'pallet-deck'}:{assembly:'ToolTip',rework:'ToolTip',packing:'PackingGate',robot:'GraspCenter',conveyor:'BeltDrive',inspection:'OpticalHead',sealer:'TapeHead',pallet:'PalletDeck'};
  const targetNode=rig.root.getObjectByName(nodeNames[current.family]);if(targetNode)target=targetNode.getWorldPosition(new T.Vector3());
  if(current.suffix!=='reference'){
   if(['assembly','rework'].includes(current.family))target.y=.96;
   if(current.family==='inspection')target.set(0,1.25,0);
   if(current.family==='sealer')target.set(0,1.34,0);
  }
  r*=.34;if(current.family==='robot')r=.26;if(!targetNode&&!['assembly','rework','inspection','sealer'].includes(current.family))r=radius*.67;
 }
 if(name==='actuator'){
  const targets={assembly:[.103,.92,.073,.22],rework:[-.103,.92,-.073,.22],packing:[.34,.84,0,.12],robot:[0,.2,0,.25],conveyor:[-.70,.73,.35,.22],inspection:[0,1.25,.016,.23],sealer:[0,1.25,0,.27],pallet:[.45,.10,.28,.18],'control-box':[.05,.03,.10,.036],shell:[-.07,.007,.104,.028],pcb:[-.103,.002,-.073,.018],fastener:[0,.018,0,.009],'sealing-tape':[0,.08,.02,.075],carton:[0,.355,0,.29]};
  const a=targets[current.family];target.set(a[0],a[1],a[2]);r=a[3];if(current.family==='robot')target=rig.root.getObjectByName('GraspCenter').getWorldPosition(new T.Vector3());
 }
 floor.visible=name!=='bottom';camera.up.set(0,1,0);if(name==='top')camera.up.set(0,0,-1);if(name==='bottom')camera.up.set(0,0,1);
 const direction=new T.Vector3(...(name==='actuator'&&current.family==='robot'?[-1,.7,-1.2]:name==='actuator'&&['assembly','rework'].includes(current.family)?[-1,.55,1]:directions[name])).normalize();
 let distance=r/Math.sin(camera.fov*Math.PI/360)*1.12;
 if(name!=='closeup'&&name!=='actuator'){
  const right=new T.Vector3().crossVectors(camera.up,direction).normalize(),up=new T.Vector3().crossVectors(direction,right).normalize(),tan=Math.tan(camera.fov*Math.PI/360);
  distance=0;for(const x of [-1,1])for(const y of [-1,1])for(const z of [-1,1]){
   const q=new T.Vector3(x*size.x/2,y*size.y/2,z*size.z/2);
   distance=Math.max(distance,Math.max(Math.abs(q.dot(right))/(tan*camera.aspect),Math.abs(q.dot(up))/tan)+q.dot(direction));
  }distance*=1.1;
 }
 // Tight depth range preserves millimetre-separated faces; the previous tiny
 // near/huge far range caused z fighting even on the protected reference GLBs.
 camera.position.copy(target).addScaledVector(direction,distance);camera.near=Math.max(distance/20,.0001);camera.far=distance+radius*4;camera.updateProjectionMatrix();controls.target.copy(target);camera.lookAt(target);controls.update();if(paint)draw();
}
function draw(){rig?.root.updateMatrixWorld(true);renderer.render(scene,camera);}
async function load(family,suffix='detailed'){
 playing=false;if(rig){scene.remove(rig.root);rig.dispose();rig=null;decoded=null;}
 const reference=suffix==='reference';
 const entry=reference?{family,suffix,asset:{id:'supervised-reference.'+family,path:family+'.glb',kind:'glb'},version:'supervised-reference'}:inventory.entries.find(v=>v.family===family&&v.suffix===suffix),directory=reference?'/tools/3d-proof/output/':base+entry.directory+'/';
 const packageDocument=reference?null:await fetch(directory+'module.json').then(r=>r.json()),p=reference?{kind:'model',source:{kind:'asset',data:entry.asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:[],poses:[],actions:[],timeline:[],tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:null,region:'device_icons'},production_effect:'none'}:packageDocument.data.definition.payload.data;
 const bytes=await fetch(directory+entry.asset.path,{cache:'no-store'}).then(r=>r.arrayBuffer());
 if(!reference&&entry.bytes!==bytes.byteLength)throw Error('GLB文件与载入清单已变更，请重新打开审阅器后捕获。');
 if(reference){
  // The protected historical baseline uses emissive_strength. Load it unchanged
  // for visual comparison only; candidate assets still use the product decoder.
  const gltf=await new GLTFLoader().parseAsync(bytes,'');decoded={gltf,nodes:new Map(),dispose(){gltf.scene.traverse(o=>{if(o.isMesh){o.geometry.dispose();for(const m of Array.isArray(o.material)?o.material:[o.material])m.dispose();}});}};
 }else decoded=await decodeGlb(bytes);
 rig=await bindScene(p,async asset=>fetch(directory+asset.path).then(r=>r.arrayBuffer()),{assetKey:assetKey(entry.asset),decoded});
 scene.add(rig.root);rig.root.traverse(o=>{if(o.isMesh){o.castShadow=true;o.receiveShadow=true;}});current={...entry,bytes:bytes.byteLength,presentation:p};
 $('model').value=family;if(!reference)$('variant').value=suffix;
 phaseSelect.replaceChildren(...[...new Set(p.timeline.filter(c=>!['cycle','working'].includes(c.phase_ref)).map(c=>c.phase_ref))].map(name=>{const option=document.createElement('option');option.value=option.textContent=name;return option;}));phaseProgress.value='0';
 const bound=new T.Box3().setFromObject(rig.root);
 for(const phase of new Set(p.timeline.map(c=>c.phase_ref))){for(const u of [0,.5,1]){pose(phase,u);bound.union(new T.Box3().setFromObject(rig.root));}}
 pose(null,0);bound.getCenter(center);bound.getSize(size);radius=Math.max(size.length()/2,.005);
 sun.target.position.copy(center);sun.position.copy(center).add(new T.Vector3(2,4,3).normalize().multiplyScalar(radius*4));const sc=sun.shadow.camera;sc.left=sc.bottom=-radius*1.7;sc.right=sc.top=radius*1.7;sc.near=radius*1.5;sc.far=radius*6;sc.updateProjectionMatrix();sun.shadow.bias=-.0003;sun.shadow.normalBias=radius*.001;
 view('isometric');$('asset-source').textContent=directory+entry.asset.path;
 const quality=qualityByPath.get((directory+entry.asset.path).slice(1));
 $('acceptance-status').textContent=reference?'原监督基准；不替代当前资产接收。':quality
  ?`${quality.asset_quality==='accepted'?'技术已接收':'技术待接收'} · ${suffix==='simple'?'仅简化档身份；':''}真人认可：${quality.review.human_acceptance==='accepted'?'已记录':'待确认'} · ${quality.review.reviewed_at}`
  :'质量清单未登记此文件。';
 $('play').textContent=p.timeline.length?'播放声明动作':'播放观察转台';phaseSelect.disabled=phaseProgress.disabled=!p.timeline.length;
 status(`${names[family]} · ${suffix==='detailed'?'详细档':suffix==='simple'?'简化档':'原监督参照'} · 实际GLB ${bytes.byteLength.toLocaleString()}字节`);return current;
}
function pngBytes(canvas){return Uint8Array.from(atob(canvas.toDataURL('image/png').split(',')[1]),c=>c.charCodeAt(0));}
async function saveImage(name){draw();await post('save',pngBytes(renderer.domElement),'?name='+encodeURIComponent(name));}
async function capture(family,suffix='detailed'){
 await load(family,suffix);$('model').value=family;
 const folder=family+(suffix==='simple'?'-simple':''),variantLabel=suffix==='simple'?'简化档':'详细档';
 const model={family,suffix,source:current.asset,source_directory:current.directory,version:current.version,glb_bytes:current.bytes,renderer_profile:'depth-fitted-v1-vsm',preview_only:true,views:[],reverse_seek:[],motion_kind:current.presentation.timeline.length?'declared_phase_animation':'camera_observation_only'};
 const envelope={center:center.clone(),size:size.clone(),radius};
 const stillBound=new T.Box3().setFromObject(rig.root);stillBound.getCenter(center);stillBound.getSize(size);radius=Math.max(size.length()/2,.005);
 for(const name of ['isometric','front','back','left','right','top','bottom','closeup']){
  status(`正在导出 ${names[family]} ${variantLabel} / ${name}`);view(name);await saveImage(folder+'/'+name+'.png');model.views.push(name);
 }
 center.copy(envelope.center);size.copy(envelope.size);radius=envelope.radius;view('isometric');
 const phases=family==='carton'?['open','fold_preview','sealed']:[...new Set(current.presentation.timeline.filter(c=>!['cycle','working'].includes(c.phase_ref)).map(c=>c.phase_ref))];
 for(const phase of phases)for(const [label,u]of [['start',0],['middle',.5],['end',1]]){pose(phase,u);await saveImage(`${folder}/${phase}-${label}.png`);view('closeup');await saveImage(`${folder}/${phase}-${label}-detail.png`);view('isometric');}
 for(const phase of phases){pose(phase,1);for(const [label,u]of [['middle',.5],['start',0]]){pose(phase,u);view('closeup');const file=`${folder}/${phase}-reverse-${label}-detail.png`;await saveImage(file);model.reverse_seek.push(file);}view('isometric');}
 const still=phases.length===0;
 const film=document.createElement('canvas');film.width=1280;film.height=720;const ctx=film.getContext('2d');
 renderer.setSize(1280,720,false);renderer.autoClear=false;renderer.setScissorTest(true);
 await post('video-start',null,'?family='+folder);
 for(let frame=0;frame<288;frame++){
  const u=frame/287,i=still?0:Math.min(phases.length-1,Math.floor(u*phases.length)),v=still?u:Math.min(1,u*phases.length-i);
  if(!still)pose(phases[i],v);
  ctx.fillStyle='#edf2f6';ctx.fillRect(0,0,1280,720);ctx.fillStyle='#233a4c';ctx.font='bold 25px Microsoft YaHei';ctx.fillText(`${names[family]} · ${variantLabel} · 实际GLB ${current.version}`,24,40);
  ctx.font='18px Microsoft YaHei';ctx.fillText(still?'观察转台：静态物品不虚构动作':`声明阶段 ${phases[i]} · ${Math.round(v*100)}% · 非生产结果`,24,76);
  renderer.setViewport(0,0,1280,720);renderer.setScissor(0,0,1280,720);renderer.clear();
  camera.aspect=16/9;view('isometric',false);
  if(still){const angle=u*Math.PI*2,dist=radius/Math.sin(camera.fov*Math.PI/360)*1.18;camera.position.set(center.x+Math.sin(angle)*dist,center.y+dist*.36,center.z+Math.cos(angle)*dist);camera.lookAt(center);}
  renderer.shadowMap.autoUpdate=true;renderer.setViewport(0,132,832,468);renderer.setScissor(0,132,832,468);draw();
  camera.aspect=1;view('closeup',false);renderer.shadowMap.autoUpdate=false;renderer.setViewport(840,150,432,432);renderer.setScissor(840,150,432,432);draw();
  ctx.drawImage(renderer.domElement,0,120,832,468,0,120,832,468);ctx.drawImage(renderer.domElement,840,138,432,432,840,138,432,432);
  ctx.font='18px Microsoft YaHei';ctx.fillStyle='#233a4c';ctx.fillText('整机 / 整体观察',24,620);ctx.fillText('机构 / 材质近景',854,620);
  ctx.fillStyle='#cad7e1';ctx.fillRect(24,660,1232,8);ctx.fillStyle='#347b9e';ctx.fillRect(24,660,1232*u,8);
  await post('video-frame',pngBytes(film));
  if(frame%48===0)status(`正在录制 ${names[family]} · ${frame}/288帧 · ${still?'观察转台':'声明机构动作'}`);
 }
 model.video=await post('video-finish');renderer.autoClear=true;renderer.shadowMap.autoUpdate=true;renderer.setScissorTest(false);camera.aspect=16/9;renderer.setSize(1920,1080,false);renderer.setViewport(0,0,1920,1080);pose(null,0);view('isometric');
 if(suffix==='detailed'){await load(family,'simple');await saveImage(family+'/simplified.png');await load(family,suffix);}
 await post('save',JSON.stringify(model,null,2),'?name='+folder+'/capture.json');results.push(model);
 await post('save',JSON.stringify({renderer:renderer.getContext().getParameter(renderer.getContext().VERSION),models:results},null,2),'?name=batch/progress.json');
 return model;
}
 async function selectModel(){if(busy)return;busy=true;try{await load($('model').value,$('variant').value);}catch(e){fail(e);}finally{busy=false;}}
 $('model').onchange=selectModel;$('view').onchange=()=>{if(!busy)view($('view').value);};
 $('variant').onchange=selectModel;
function fail(e){busy=false;playing=false;status('实际导出失败：'+e);post('save',JSON.stringify({error:String(e),family:current?.family}), '?name=batch/error.json');}
 $('capture').onclick=async()=>{if(busy)return;busy=true;try{await capture($('model').value,$('variant').value);status('当前模型多视角与动作导出完成');}catch(e){fail(e);}busy=false;};
$('all').onclick=async()=>{if(busy)return;busy=true;try{for(const e of models){await capture(e.family);await capture(e.family,'simple');}status('28个实际GLB多视角与动作已导出；待逐图审查');}catch(e){fail(e);}busy=false;};
 $('simple-all').onclick=async()=>{if(busy)return;busy=true;try{for(const e of models)await capture(e.family,'simple');status('14类简化档实际GLB多视角与动作已补齐；待逐图审查');}catch(e){fail(e);}busy=false;};
const referenceButton=document.createElement('button');referenceButton.textContent='导出原监督参照';referenceButton.id='references';document.querySelector('header').append(referenceButton);
const diagnosticButton=document.createElement('button');diagnosticButton.textContent='几何诊断：切换阴影';diagnosticButton.id='shadow-diagnostic';document.querySelector('header').append(diagnosticButton);
diagnosticButton.onclick=()=>{if(busy)return;renderer.shadowMap.enabled=!renderer.shadowMap.enabled;draw();status(renderer.shadowMap.enabled?'阴影已打开':'仅几何诊断：暂时关闭阴影，不作为最终验收画面');};
const repairedButton=document.createElement('button');repairedButton.textContent='重拍四类细节返修';repairedButton.id='repaired';document.querySelector('header').append(repairedButton);
repairedButton.onclick=async()=>{if(busy)return;busy=true;try{for(const family of ['conveyor','sealer','carton','sealing-tape'])await capture(family);status('四类运动细节已重拍；其余10类GLB字节未变，原画面继续适用');}catch(e){fail(e);}busy=false;};
const patchButton=document.createElement('button');patchButton.textContent='重拍驱动与接头修订（两档）';patchButton.id='patch-capture';document.querySelector('header').append(patchButton);
patchButton.onclick=async()=>{if(busy)return;busy=true;try{for(const family of ['assembly','rework','packing','inspection'])for(const suffix of ['detailed','simple'])await capture(family,suffix);status('0.4.1八个实际GLB多视角、正反向关键帧与动作视频已重拍；待逐图审查');}catch(e){fail(e);}busy=false;};
referenceButton.onclick=async()=>{if(busy)return;busy=true;try{
 for(const family of ['assembly','robot','conveyor','inspection','sealer','pallet']){
  await load(family,'reference');for(const name of ['isometric','front','back','left','right','top','bottom','closeup']){view(name);status('正在重开原监督参照 '+family+' / '+name);await saveImage('reference-'+family+'/'+name+'.png');}
 }
 await load('assembly');status('原监督六类实际GLB已在同一显示条件下导出比较视角');
}catch(e){fail(e);}busy=false;};
$('play').onclick=()=>{if(busy)return;playing=!playing;zero=performance.now();$('play').textContent=playing?'暂停预览':current.presentation.timeline.length?'播放声明动作':'播放观察转台';};
function animate(now){if(playing&&rig&&!busy){const phases=current.family==='carton'?['open','fold_preview','sealed']:[...new Set(current.presentation.timeline.filter(c=>!['cycle','working'].includes(c.phase_ref)).map(c=>c.phase_ref))];if(phases.length){const t=((now-zero)/1800)%phases.length;phaseSelect.value=phases[Math.floor(t)];phaseProgress.value=String(Math.round(t%1*100));pose(phases[Math.floor(t)],t%1);drawPreview();}else{const angle=(now-zero)/4000*Math.PI,dist=radius/Math.sin(camera.fov*Math.PI/360)*1.18;camera.up.set(0,1,0);camera.position.set(center.x+Math.sin(angle)*dist,center.y+dist*.36,center.z+Math.cos(angle)*dist);camera.lookAt(center);draw();}}requestAnimationFrame(animate);}requestAnimationFrame(animate);
controls.addEventListener('change',()=>{if(!busy)draw();});
busy=true;await load('robot');$('model').value='robot';busy=false;

const mechanismsButton=document.createElement('button');mechanismsButton.textContent='补拍关键机构与孔洞';document.querySelector('header').append(mechanismsButton);
mechanismsButton.onclick=async()=>{if(busy)return;busy=true;try{
 const evidence=[];
 for(const e of models){await load(e.family);const files=[];
  const states=e.family==='carton'?[['fold_preview',0],['fold_preview',.2],['fold_preview',.4],['fold_preview',.65],['fold_preview',.85],['fold_preview',1]]:['assembly','rework'].includes(e.family)?[['auto',.125],['auto',.375],['auto',.625],['auto',.875]]:(e.animations.length?[...new Set(current.presentation.timeline.filter(c=>!['cycle','working'].includes(c.phase_ref)).map(c=>c.phase_ref))].flatMap(p=>(['conveyor','sealer'].includes(e.family)?[0,.07,.17,.29,.43,.61,.83,1]:[0,.5,1]).map(u=>[p,u])):[[null,0]]);
  for(const [phase,u] of states){pose(phase,u);view('actuator');const file=e.family+'/mechanism-'+(phase??'static')+'-'+String(u).replace('.','_')+'.png';await saveImage(file);files.push({phase,progress:u,file});if(e.family==='carton'){view('top');const top=file.replace('mechanism-','fold-top-');await saveImage(top);files.push({phase,progress:u,file:top});}}
  evidence.push({family:e.family,version:e.version,bytes:e.bytes,images:files});status('关键机构补拍完成：'+names[e.family]);
 }
 await post('save',JSON.stringify({preview_only:true,evidence},null,2),'?name=batch/mechanisms.json');status('14类关键机构/真实孔洞与接触点补拍完成');
}catch(e){fail(e);}busy=false;};

const lastRepairs=document.createElement('button');lastRepairs.textContent='重拍夹座与焊盘返修';document.querySelector('header').append(lastRepairs);
lastRepairs.onclick=async()=>{if(busy)return;busy=true;try{for(const family of ['packing','control-box','pcb'])for(const suffix of ['detailed','simple'])await capture(family,suffix);status('夹座/贴板焊盘与孔壁镀层的六个GLB已重拍');}catch(e){fail(e);}busy=false;};
