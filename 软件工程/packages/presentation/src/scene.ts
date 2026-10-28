import { AnimationMixer, BoxGeometry, CylinderGeometry, PlaneGeometry, Color, Group, Mesh, MeshStandardMaterial, Object3D, Quaternion, Vector3, CanvasTexture, SRGBColorSpace, LoopOnce } from 'three';
import type { Presentation } from '../../commands/generated/typescript/Presentation';
import type { Pose } from '../../commands/generated/typescript/Pose';
import type { AnchorRef } from '../../commands/generated/typescript/AnchorRef';
import type { ClipPoint } from './clips';
import type { AssetByteReader, VisualProgress } from './types';
import { assetKey, UnsupportedMedia } from './types';
import { decodeStatic } from './bytes';
import type { DecodedModel } from './glb';

const basis = new Quaternion().setFromAxisAngle(new Vector3(1,0,0),-Math.PI/2);
export function applyPose(object:Object3D, pose:Pose) {
  const factor=pose.unit==='millimeter'?0.001:1;
  object.position.set(pose.translation.x,pose.translation.y,pose.translation.z).multiplyScalar(factor);
  object.quaternion.set(pose.rotation.x,pose.rotation.y,pose.rotation.z,pose.rotation.w).normalize();
  object.scale.set(pose.scale.x,pose.scale.y,pose.scale.z);
  if (pose.coordinate_frame==='right_handed_z_up') {
    object.position.applyQuaternion(basis);
    object.quaternion.premultiply(basis).multiply(basis.clone().invert());
    object.scale.set(pose.scale.x,pose.scale.z,pose.scale.y);
  }
}
export function sampledPose(keys:{at_progress:number;pose:Pose}[], progress:number, linear:boolean):Pose|null {
  if (!keys.length) return null;
  if (keys.some((key,i)=>i>0&&key.at_progress<=keys[i-1].at_progress)) throw new UnsupportedMedia('动作关键帧必须严格按阶段进度递增。');
  let left=keys[0],right=keys[keys.length-1];
  for(const key of keys) { if(key.at_progress<=progress) left=key; if(key.at_progress>progress) {right=key;break;} }
  if(progress<=keys[0].at_progress)return keys[0].pose;
  if(!linear||progress>=right.at_progress)return left.pose;
  const a=left.pose,b=right.pose;
  if(a.unit!==b.unit||a.coordinate_frame!==b.coordinate_frame) throw new UnsupportedMedia('同一动作轨道不能混用单位或坐标基。');
  const t=(progress-left.at_progress)/(right.at_progress-left.at_progress);
  const v=(x:{x:number;y:number;z:number},y:typeof x)=>({x:x.x+(y.x-x.x)*t,y:x.y+(y.y-x.y)*t,z:x.z+(y.z-x.z)*t});
  const q=new Quaternion(a.rotation.x,a.rotation.y,a.rotation.z,a.rotation.w).slerp(new Quaternion(b.rotation.x,b.rotation.y,b.rotation.z,b.rotation.w),t);
  return {...a,translation:v(a.translation,b.translation),scale:v(a.scale,b.scale),rotation:{x:q.x,y:q.y,z:q.z,w:q.w}};
}
export interface PresentationScene {
  root:Group; nodes:Map<number,Object3D>;
  apply:(point:VisualProgress|null,clip:ClipPoint|null)=>void;
  applyNamedPose:(name:string)=>void;
  anchor:(ref:AnchorRef)=>Vector3|null;
  dispose:()=>void;
}
/** GLB stays intrinsically metre/Y-up. Canonical per-node poses are converted explicitly. */
export async function bindScene(p:Presentation, read:AssetByteReader, model?:{assetKey:string;decoded:DecodedModel}):Promise<PresentationScene> {
  const root=new Group(), nodes=new Map<number,Object3D>();
  const owned:{dispose():void}[]=[];
  root.scale.set(p.scale.x,p.coordinate_frame==='right_handed_z_up'?p.scale.z:p.scale.y,p.coordinate_frame==='right_handed_z_up'?p.scale.y:p.scale.z);
  if(model) root.add(model.decoded.gltf.scene);
  try {
    for(let i=0;i<p.nodes.length;i++) {
      const n=p.nodes[i];let object:Object3D;
      if(n.geometry.kind==='imported_node') {
        if(!model||assetKey(n.geometry.data.asset_ref)!==model.assetKey||n.geometry.data.unsupported_extensions.length) throw new UnsupportedMedia(`节点 ${n.name} 的导入来源或扩展未获支持。`);
        const found=model.decoded.nodes.get(n.geometry.data.node_index);
        if(!found)throw new UnsupportedMedia(`GLB 中不存在声明节点 ${n.geometry.data.node_index}。`);
        if([...nodes.values()].includes(found))throw new UnsupportedMedia('两个 canonical 节点绑定了同一 GLB 节点。');
        object=found;
      } else {
        const size=n.geometry.data, factor=p.unit==='millimeter'?0.001:1;
        const geometry=n.geometry.kind==='box'?new BoxGeometry((size as any).x*factor,(p.coordinate_frame==='right_handed_z_up'?(size as any).z:(size as any).y)*factor,(p.coordinate_frame==='right_handed_z_up'?(size as any).y:(size as any).z)*factor):n.geometry.kind==='cylinder'?new CylinderGeometry((size as any).radius*factor,(size as any).radius*factor,(size as any).height*factor,24):new PlaneGeometry((size as any).x*factor,(size as any).y*factor);
        const [r,g,b,a]=n.material.color_rgba;
        const material=new MeshStandardMaterial({color:new Color(r,g,b),opacity:a,transparent:a<1,metalness:n.material.metallic,roughness:n.material.roughness});
        owned.push(geometry,material);
        if(n.material.texture) {
          const decoded=await decodeStatic(n.material.texture,await read(n.material.texture));
          let bitmap:ImageBitmap;try {bitmap=await createImageBitmap(decoded,{imageOrientation:'flipY'});} finally {decoded.close();}
          // WebGL ignores UNPACK_FLIP_Y for ImageBitmap; apply the UV orientation at decode.
          const texture=new CanvasTexture(bitmap);texture.flipY=false;texture.colorSpace=SRGBColorSpace;material.map=texture;owned.push(texture,{dispose:()=>bitmap.close()});
        }
        object=new Mesh(geometry,material);object.name=n.name;
      }
      nodes.set(i,object);
    }
    for(let i=0;i<p.nodes.length;i++) {
      const n=p.nodes[i],object=nodes.get(i)!;
      if(n.parent_index!=null && (n.parent_index===i || !nodes.has(n.parent_index)))throw new UnsupportedMedia(`节点 ${n.name} 的父节点无效。`);
      const parent=n.parent_index==null?root:nodes.get(n.parent_index)!;
      if(n.geometry.kind==='imported_node') {
        if(n.parent_index!=null&&object.parent!==parent) throw new UnsupportedMedia('导入节点父子关系与 GLB 不一致；不能重接完整模型。');
      } else { let cursor: number|null|undefined=n.parent_index;const seen=new Set([i]);while(cursor!=null){if(seen.has(cursor))throw new UnsupportedMedia('节点树存在环。');seen.add(cursor);cursor=p.nodes[cursor]?.parent_index;} parent.add(object); }
      applyPose(object,n.transform);
    }
    const base=new Map<Object3D,{p:Vector3;q:Quaternion;s:Vector3}>();root.traverse(o=>base.set(o,{p:o.position.clone(),q:o.quaternion.clone(),s:o.scale.clone()}));
    const mixer=model?new AnimationMixer(model.decoded.gltf.scene):null;
    function restoreBase() {mixer?.stopAllAction();for(const [o,b]of base){o.position.copy(b.p);o.quaternion.copy(b.q);o.scale.copy(b.s);}}
    function applyNamedPose(name:string) {
      restoreBase();
      const pose=p.poses.find(x=>x.id===name);
      if(!pose)throw new UnsupportedMedia('所选命名姿态不存在。');
      for(const t of pose.node_transforms){const o=nodes.get(t.node_index);if(!o)throw new UnsupportedMedia('命名姿态引用了不支持的节点。');applyPose(o,t.pose);}
      root.updateMatrixWorld(true);
    }
    function apply(point:VisualProgress|null,clip:ClipPoint|null) {
      if(clip&&clip.clip.transition!=='hold')throw new UnsupportedMedia('crossfade/pan/zoom是图片视口过渡；canonical未声明3D跨画面合成，请以model_animation或native_pose的动作表达3D运动。');
      restoreBase();
      if(clip?.clip.source.kind==='model_animation') {
        const source=clip.clip.source.data;
        if(!model||assetKey(source.asset_ref)!==model.assetKey)throw new UnsupportedMedia('动画片段和当前 GLB 不是同一精确素材。');
        const animation=model.decoded.gltf.animations[source.animation_index];
        if(!animation)throw new UnsupportedMedia(`GLB 没有声明动画 ${source.animation_index}。`);
        const action=mixer!.clipAction(animation);action.setLoop(LoopOnce,1);action.clampWhenFinished=true;action.play();mixer!.setTime(animation.duration*clip.progress);
      }
      if(clip?.clip.source.kind==='native_pose') {
        applyNamedPose(clip.clip.source.data);
      }
      // Track keys are stage-local. Multi-stage motion must be bound by an explicit clip.
      if(point?.progress!=null && (clip?.clip.source.kind==='native_pose'||(!p.timeline.length&&point.display_phase_id==='cycle'))) {
        for(const track of p.actions){const o=nodes.get(track.node_index);if(!o)throw new UnsupportedMedia('动作轨道引用了不支持的节点。');const pose=sampledPose(track.keys,clip?.progress??point.progress,track.transition==='linear');if(pose)applyPose(o,pose);}
      }
      root.updateMatrixWorld(true);
    }
    return {root,nodes,apply,applyNamedPose,anchor(ref){if(ref.kind!=='scene_node')return null;const node=nodes.get(ref.data);return node?node.getWorldPosition(new Vector3()):null;},dispose(){mixer?.stopAllAction();if(mixer)mixer.uncacheRoot(model!.decoded.gltf.scene);for(const value of owned)value.dispose();model?.decoded.dispose();}};
  } catch(error){for(const value of owned)value.dispose();model?.decoded.dispose();throw error;}
}
