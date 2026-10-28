import { AmbientLight,Box3,Color,DirectionalLight,GridHelper,MOUSE,Mesh,PerspectiveCamera,Raycaster,Scene,Vector2,Vector3,WebGLRenderer,type Object3D } from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { PresentationScene } from '../../../../../packages/presentation/src';
import type { AnchorRef } from '../../../../../packages/commands/generated/typescript/AnchorRef';
import {captureCamera,visibleFactoryObjects} from '../render-surface/factoryCapture';
import {gpuPainted,type ReadonlyVisualCapture} from '../render-surface/visualCapture';
export class ModelViewport {
  private renderer:WebGLRenderer;
  private scene=new Scene();
  private camera=new PerspectiveCamera(38,1,0.01,1000);
  private controls:OrbitControls;
  private resize:ResizeObserver;
  private center=new Vector3();private radius=1;
  private down:{x:number;y:number}|null=null;
  private live=true;private serial=0;
  constructor(private host:HTMLElement,private rig:PresentationScene,private onSelect:((index:number)=>void)|undefined,private onProject:()=>void,private capture?:ReadonlyVisualCapture,private onInspectItem?:(id:string)=>void,private itemRoots?:()=>Map<Object3D,string>) {
    this.renderer=new WebGLRenderer({antialias:true,alpha:false});this.renderer.setClearColor(new Color('#eaf1f7'));this.renderer.setPixelRatio(window.devicePixelRatio);
    if(capture){this.renderer.domElement.addEventListener('webglcontextlost',this.contextLost);this.renderer.debug.onShaderError=(gl,program)=>capture.onFailure(gl.getProgramInfoLog(program)??'实际模型GPU着色程序失败。');}
    // Test-only private receiver. On demand; does not schedule rendering or change production facts.
    (this.renderer.domElement as HTMLCanvasElement & {readMediaDiagnostics?:()=>unknown}).readMediaDiagnostics=()=>{
      this.rig.root.updateMatrixWorld(true);
      const bounds=(object:Object3D)=>{
        let b:Box3;
        if(object instanceof Mesh){
          if(!object.geometry.boundingBox)object.geometry.computeBoundingBox();
          b=object.geometry.boundingBox!.clone().applyMatrix4(object.matrixWorld);
        }else b=new Box3().setFromObject(object);
        return b.isEmpty()?null:{min:b.min.toArray(),max:b.max.toArray()};
      };
      const nodes=[...this.rig.nodes].map(([node_index,node])=>({node_index,name:node.name,geometry_kind:node.type==='Mesh'?'mesh':'group',matrix_world:node.matrixWorld.toArray(),bounds:bounds(node)}));
      const roots=this.itemRoots?.()??new Map<Object3D,string>();
      const items=[...roots].map(([object,id])=>{
        let parent: Object3D|null=object.parent,parent_node_index:number|null=null,parent_item_id:string|null=null;
        while(parent){const item=roots.get(parent);if(item){parent_item_id=item;break;}const match=[...this.rig.nodes].find(([,n])=>n===parent);if(match){parent_node_index=match[0];break;}parent=parent.parent;}
        const surfaces:{name:string;bounds:ReturnType<typeof bounds>}[]=[];
        object.traverse(child=>{if(child.type==='Mesh')surfaces.push({name:child.name,bounds:bounds(child)});});
        return {id,parent_node_index,parent_item_id,matrix_world:object.matrixWorld.toArray(),bounds:bounds(object),surfaces,media_source:object.userData.mqSource??null};
      });
      return {schema_version:1,scope:'actual viewport world transforms and axis-aligned support witnesses; not general collision or formal presented FPS',nodes,items,media_source:this.rig.root.userData.mqSource??null};
    };
    host.append(this.renderer.domElement);this.scene.add(rig.root,new AmbientLight(0xffffff,2.4));
    const light=new DirectionalLight(0xffffff,3);light.position.set(3,6,5);this.scene.add(light);
    this.controls=new OrbitControls(this.camera,this.renderer.domElement);this.controls.mouseButtons={LEFT:null,MIDDLE:MOUSE.PAN,RIGHT:MOUSE.ROTATE};this.controls.enableDamping=false;this.controls.minDistance=0.08;this.controls.maxDistance=100;
    if(capture){this.controls.enabled=false;rig.root.userData.ref=capture.object;const requested=captureCamera(capture.camera);this.camera.position.copy(requested.eye);this.controls.target.copy(requested.target);this.camera.zoom=requested.zoom;this.camera.lookAt(requested.target);this.camera.updateProjectionMatrix();}
    this.controls.addEventListener('change',this.draw);
    this.renderer.domElement.addEventListener('contextmenu',this.context);
    if(!capture){this.renderer.domElement.addEventListener('pointerdown',this.pointerDown);this.renderer.domElement.addEventListener('pointerup',this.pointerUp);}
    this.resize=new ResizeObserver(this.fitSize);this.resize.observe(host);this.fitSize();if(!capture)this.view('iso');
  }
  private context=(e:Event)=>e.preventDefault();
  private contextLost=()=>this.capture?.onFailure('实际模型WebGL上下文已丢失。');
  private pointerDown=(e:PointerEvent)=>{if(e.button===0)this.down={x:e.clientX,y:e.clientY};};
  private pointerUp=(e:PointerEvent)=>{
    if(e.button!==0||!this.down)return;const start=this.down;this.down=null;if(Math.hypot(e.clientX-start.x,e.clientY-start.y)>4)return;
    const rect=this.renderer.domElement.getBoundingClientRect(),ray=new Raycaster();ray.setFromCamera(new Vector2((e.clientX-rect.left)/rect.width*2-1,-(e.clientY-rect.top)/rect.height*2+1),this.camera);
    const hit=ray.intersectObject(this.rig.root,true)[0];if(!hit)return;
    let object:Object3D|null=hit.object;
    const items=this.itemRoots?.();
    while(object){const item=items?.get(object);if(item){this.onInspectItem?.(item);return;}for(const [index,node]of this.rig.nodes)if(node===object){this.onSelect?.(index);return;}object=object.parent;}
  };
  private fitSize=()=>{const w=this.host.clientWidth,h=this.host.clientHeight;if(!w||!h)return;this.renderer.setSize(w,h);this.camera.aspect=w/h;this.camera.updateProjectionMatrix();this.draw();};
  view(kind:'iso'|'front'|'back'|'left'|'right'|'top'|'bottom'|'fit') {
    if(this.capture)return;
    this.rig.root.updateMatrixWorld(true);const box=new Box3().setFromObject(this.rig.root);box.getCenter(this.center);this.radius=Math.max(box.getSize(new Vector3()).length()/2,0.05);
    // Keep the Y-up orbit axis; the tiny pole offset gives top/bottom a defined screen orientation.
    const directions={iso:[1,0.65,1],front:[0,0,1],back:[0,0,-1],left:[-1,0,0],right:[1,0,0],top:[0,1,0.000001],bottom:[0,-1,0.000001],fit:[1,0.65,1]} as const;
    const direction=new Vector3(...directions[kind]);
    const distance=this.radius/Math.sin(this.camera.fov*Math.PI/360)*1.2;
    this.camera.position.copy(this.center).add(direction.normalize().multiplyScalar(distance));this.camera.near=Math.max(distance/1000,0.001);this.camera.far=distance*100;this.camera.updateProjectionMatrix();this.controls.target.copy(this.center);this.controls.update();this.draw();
  }
  project(ref:AnchorRef):{x:number;y:number}|null {
    const world=this.rig.anchor(ref);if(!world)return null;
    const p=world.project(this.camera);return p.z>=-1&&p.z<=1&&Math.abs(p.x)<=1&&Math.abs(p.y)<=1?{x:(p.x+1)*50,y:(1-p.y)*50}:null;
  }
  draw=()=>{this.rig.root.updateMatrixWorld(true);this.renderer.render(this.scene,this.camera);this.onProject();if(this.capture){const serial=++this.serial,capture=this.capture;gpuPainted(this.renderer,()=>this.live&&serial===this.serial).then(done=>{if(done)capture.onFrame({visible_objects:visibleFactoryObjects(this.scene,this.camera),limitations:[]});},e=>{if(this.live&&serial===this.serial)capture.onFailure(String(e));});}};
  dispose(){this.live=false;this.serial++;this.resize.disconnect();this.controls.removeEventListener('change',this.draw);this.controls.dispose();this.renderer.domElement.removeEventListener('contextmenu',this.context);this.renderer.domElement.removeEventListener('pointerdown',this.pointerDown);this.renderer.domElement.removeEventListener('pointerup',this.pointerUp);this.renderer.domElement.removeEventListener('webglcontextlost',this.contextLost);this.renderer.dispose();this.renderer.forceContextLoss();this.renderer.domElement.remove();this.rig.dispose();}
}
