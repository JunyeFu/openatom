const pointerUp=(e:PointerEvent)=>{
    if(e.button!==0||!this.down)return;const start=this.down;this.down=null;if(Math.hypot(e.clientX-start.x,e.clientY-start.y)>4)return;
    const rect=this.renderer.domElement.getBoundingClientRect(),ray=new Raycaster();ray.setFromCamera(new Vector2((e.clientX-rect.left)/rect.width*2-1,-(e.clientY-rect.top)/rect.height*2+1),this.camera);
    const hit=ray.intersectObject(this.rig.root,true)[0];if(!hit)return;
    let object:Object3D|null=hit.object;
    while(object){for(const [index,node]of this.rig.nodes)if(node===object){this.onSelect?.(index);return;}object=object.parent;}
  };