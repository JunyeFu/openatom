import { AnimationClip, LoadingManager, Object3D } from 'three';
import { GLTFLoader, type GLTF } from 'three/addons/loaders/GLTFLoader.js';
import { UnsupportedMedia } from './types';

export interface GlbDocument { asset:{version:string}; buffers?:{uri?:string}[]; images?:{uri?:string;bufferView?:number}[]; extensionsUsed?:string[]; extensionsRequired?:string[]; nodes?:{skin?:number;weights?:number[]}[]; meshes?:{primitives:{mode?:number;targets?:unknown[];extensions?:Record<string,unknown>}[]}[]; animations?:{samplers:{interpolation?:string}[];channels:{target:{path:string}}[]}[] }
export function inspectGlb(bytes:ArrayBuffer):GlbDocument {
  const v = new DataView(bytes);
  if (bytes.byteLength < 20 || v.getUint32(0,true)!==0x46546c67 || v.getUint32(4,true)!==2 || v.getUint32(8,true)!==bytes.byteLength) throw new UnsupportedMedia('仅支持长度完整的 GLB 2。');
  let p=12, json:GlbDocument|null=null, bin=false;
  while (p<bytes.byteLength) {
    if (p+8>bytes.byteLength) throw new UnsupportedMedia('GLB 块头截断。');
    const length=v.getUint32(p,true), type=v.getUint32(p+4,true);p+=8;
    if (length%4 || p+length>bytes.byteLength) throw new UnsupportedMedia('GLB 块长度无效。');
    if (type===0x4e4f534a && p===20 && !json) json=JSON.parse(new TextDecoder().decode(new Uint8Array(bytes,p,length)));
    else if (type===0x004e4942 && json && !bin) bin=true;
    else throw new UnsupportedMedia('GLB 含重复、顺序错误或未支持的块。');
    p+=length;
  }
  if (!json || json.asset?.version!=='2.0') throw new UnsupportedMedia('GLB 缺少 glTF 2.0 JSON。');
  if (json.buffers?.some(b=>b.uri!=null) || json.images?.some(i=>i.uri!=null || i.bufferView==null)) throw new UnsupportedMedia('仅支持自包含 GLB：buffer 和 image 不得含 URI。');
  const extensions=[...(json.extensionsUsed??[]),...(json.extensionsRequired??[])];
  const unsupported=extensions.filter(x=>!['KHR_materials_unlit','KHR_texture_transform','KHR_materials_emissive_strength'].includes(x));
  if (unsupported.length) throw new UnsupportedMedia(`GLB 扩展不适用：${[...new Set(unsupported)].join('、')}。`);
  if (json.nodes?.some(n=>n.skin!=null || n.weights!=null) || json.meshes?.some(m=>m.primitives.some(p=>(p.mode!=null&&p.mode!==4) || p.targets?.length))) throw new UnsupportedMedia('本批不支持骨骼、形变或非三角面 GLB。');
  for (const animation of json.animations??[]) {
    if (animation.samplers.some(s=>s.interpolation!=null&&!['LINEAR','STEP','CUBICSPLINE'].includes(s.interpolation))) throw new UnsupportedMedia('GLB 动画插值不受支持；不得静默改为线性插值。');
    if (animation.channels.some(c=>!['translation','rotation','scale'].includes(c.target.path))) throw new UnsupportedMedia('GLB 仅支持刚性位移、旋转和尺度动画，不支持形变权重。');
  }
  return json;
}
export interface DecodedModel { gltf:GLTF; nodes:Map<number,Object3D>; dispose:()=>void }
async function decodeTemplate(bytes:ArrayBuffer):Promise<DecodedModel> {
  const doc=inspectGlb(bytes);
  const manager=new LoadingManager();
  manager.setURLModifier(url=>{ if (!url.startsWith('blob:')) throw new UnsupportedMedia('GLB 尝试加载外部资源。'); return url; });
  const gltf=await new GLTFLoader(manager).parseAsync(bytes,'');
  const nodes=new Map<number,Object3D>();
  for (let i=0;i<(doc.nodes?.length??0);i++) nodes.set(i,await gltf.parser.getDependency('node',i));
  return {gltf,nodes,dispose(){
    const geometries=new Set<{dispose():void}>(), materials=new Set<{dispose():void}>(), textures=new Set<{dispose():void}>();
    for (const root of gltf.scenes) root.traverse(object=>{ const mesh=object as unknown as {geometry?:{dispose():void};material?:any};if(mesh.geometry) geometries.add(mesh.geometry);if(mesh.material) for(const mat of Array.isArray(mesh.material)?mesh.material:[mesh.material]){ materials.add(mat);for(const item of Object.values(mat)) if((item as any)?.isTexture) textures.add(item as any); } });
    const images=new Set<{close():void}>();
    for(const texture of textures) {
      const image=(texture as import('three').Texture).source.data as ImageBitmap|null;
      if(image && typeof image.close==='function') images.add(image);
    }
    for(const value of [...textures,...materials,...geometries]) value.dispose();
    for(const image of images) image.close();
  }};
}

// Bytes are shared only by the exact authorized reader. No global path/Run cache:
// the last instance releases its template, textures and decoded ImageBitmaps.
const decodedModels=new WeakMap<ArrayBuffer,{users:number;template:Promise<DecodedModel>}>();
export async function decodeGlb(bytes:ArrayBuffer):Promise<DecodedModel> {
  let entry=decodedModels.get(bytes);
  if(!entry) {entry={users:0,template:decodeTemplate(bytes)};decodedModels.set(bytes,entry);}
  const shared=entry;shared.users++;
  let template:DecodedModel;
  try {template=await shared.template;}
  catch(error) {if(--shared.users===0)decodedModels.delete(bytes);throw error;}
  const copies=new Map<Object3D,Object3D>();
  function copy(object:Object3D):Object3D {
    const existing=copies.get(object);if(existing)return existing;
    // Rigid GLB nodes have independent transforms/mixers, immutable shared resources.
    const result=object.clone(false);copies.set(object,result);
    for(const child of object.children)result.add(copy(child));
    return result;
  }
  const scenes=template.gltf.scenes.map(scene=>copy(scene) as typeof scene);
  const scene=copy(template.gltf.scene) as typeof template.gltf.scene;
  const nodes=new Map([...template.nodes].map(([index,node])=>[index,copy(node)]));
  const uuids=new Map([...copies].map(([original,instance])=>[original.uuid,instance.uuid]));
  const animations=template.gltf.animations.map(clip=>{
    let retargeted=false;
    const tracks=clip.tracks.map(track=>{
      const split=track.name.indexOf('.'), uuid=uuids.get(track.name.slice(0,split));
      if(!uuid)return track;
      retargeted=true;
      // GLTFLoader targets unnamed nodes by UUID. Retarget the binding while
      // sharing immutable key samples and its exact (including cubic) interpolant.
      return Object.assign(Object.create(Object.getPrototypeOf(track)) as typeof track,track,{name:uuid+track.name.slice(split)});
    });
    return retargeted?new AnimationClip(clip.name,clip.duration,tracks,clip.blendMode):clip;
  });
  let live=true;
  return {gltf:{...template.gltf,scene,scenes,animations},nodes,dispose(){
    if(!live)return;live=false;
    if(--shared.users===0){decodedModels.delete(bytes);template.dispose();}
  }};
}
