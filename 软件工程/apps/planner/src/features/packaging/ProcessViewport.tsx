import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { connectionAnchors, panelGeometry, panelTransform } from './process';
import type { PackageStructureJob } from '../../../../../packages/commands/generated/typescript/PackageStructureJob';
import type { PackagingProcessSample } from '../../../../../packages/commands/generated/typescript/PackagingProcessSample';
import {visibleFactoryObjects} from '../render-surface/factoryCapture';
import {gpuPainted,type ReadonlyVisualCapture} from '../render-surface/visualCapture';

/** Geometry comes only from the exact result Job and the selected actual sample. No solver or interpolation. */
export function ProcessViewport({ job, sample,capture }: { job: PackageStructureJob; sample: PackagingProcessSample | null;capture?:ReadonlyVisualCapture }) {
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const node = container.current;
    if (!node) return;
    if (job.unit !== 'millimeter' || job.coordinate_frame !== 'right_handed_z_up') {capture?.onFailure('此实际包装输入的单位/坐标尚不适用三维消费。');return;}
    let live=true,serial=0;
    let renderer:THREE.WebGLRenderer;
    try{renderer=new THREE.WebGLRenderer({antialias:true,alpha:true});}catch(reason){if(!capture)throw reason;capture.onFailure(reason instanceof Error?reason.message:String(reason));return;}
    const contextLost=()=>capture?.onFailure('实际包装WebGL上下文已丢失。');
    if(capture){renderer.domElement.addEventListener('webglcontextlost',contextLost);renderer.debug.onShaderError=(gl,program)=>capture.onFailure(gl.getProgramInfoLog(program)??'实际包装GPU着色程序失败。');}
    renderer.setPixelRatio(window.devicePixelRatio);
    node.appendChild(renderer.domElement);
    const scene = new THREE.Scene(); scene.background = new THREE.Color('#e8edf1');
    const camera = new THREE.PerspectiveCamera(42, 1, .01, 100000); camera.up.set(0, 0, 1);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.mouseButtons = { LEFT: undefined, MIDDLE: THREE.MOUSE.PAN, RIGHT: THREE.MOUSE.ROTATE };
    if(capture)controls.enabled=false;
    scene.add(new THREE.HemisphereLight('#ffffff', '#85909a', 2));
    const light = new THREE.DirectionalLight('#ffffff', 2); light.position.set(120, -90, 190); scene.add(light);
    const group = new THREE.Group(); scene.add(group);
    if(capture)group.userData.ref=capture.object;
    const geometries: THREE.BufferGeometry[] = []; const materials: THREE.Material[] = [];
    job.panels.forEach((panel, index) => {
      const transform = panelTransform(job, panel, sample); if (!transform) return;
      const geometry = panelGeometry(panel); if (!geometry) return;
      const material = new THREE.MeshStandardMaterial({ color: new THREE.Color().setHSL(index * .137 % 1, .25, .6), roughness: .85, side: THREE.DoubleSide });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.applyMatrix4(transform);
      mesh.name = panel.id; group.add(mesh); geometries.push(geometry); materials.push(material);
      const edges = new THREE.EdgesGeometry(geometry); const edgeMaterial = new THREE.LineBasicMaterial({ color: '#294452' });
      const lines = new THREE.LineSegments(edges, edgeMaterial); mesh.add(lines); geometries.push(edges); materials.push(edgeMaterial);
    });
    connectionAnchors(job, sample).forEach(connection => {
      const geometry = new THREE.BufferGeometry().setFromPoints(connection.points);
      const material = new THREE.LineBasicMaterial({ color: '#b36c25' });
      const line = new THREE.Line(geometry, material); line.name = connection.id; group.add(line); geometries.push(geometry); materials.push(material);
    });
    const bounds = new THREE.Box3().setFromObject(group); const center = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
    const span = bounds.isEmpty() ? 100 : Math.max(10, bounds.getSize(new THREE.Vector3()).length());
    controls.target.copy(center); camera.position.copy(center).add(new THREE.Vector3(span, -span, span * .8)); controls.update();
    if(capture){const state=capture.camera,factor=state.unit==='meter'?1000:1;const vector=(v:typeof state.eye)=>state.coordinate_frame==='right_handed_y_up'?new THREE.Vector3(v.x,-v.z,v.y).multiplyScalar(factor):new THREE.Vector3(v.x,v.y,v.z).multiplyScalar(factor);camera.position.copy(vector(state.eye));controls.target.copy(vector(state.target));camera.zoom=state.zoom;camera.lookAt(controls.target);}
    const draw = () => { const width = node.clientWidth; const height = Math.max(300, node.clientHeight); renderer.setSize(width, height, false); camera.aspect = width / height; camera.updateProjectionMatrix(); renderer.render(scene, camera);if(capture){const ticket=++serial;gpuPainted(renderer,()=>live&&ticket===serial).then(done=>{if(done)capture.onFrame({visible_objects:visibleFactoryObjects(scene,camera),limitations:[]});},reason=>{if(live&&ticket===serial)capture.onFailure(String(reason));});} };
    controls.addEventListener('change', draw); const resize = new ResizeObserver(draw); resize.observe(node); draw();
    return () => {live=false;serial++;resize.disconnect(); controls.dispose(); geometries.forEach(item => item.dispose()); materials.forEach(item => item.dispose());renderer.domElement.removeEventListener('webglcontextlost',contextLost);renderer.dispose();renderer.forceContextLoss(); renderer.domElement.remove(); };
  }, [job, sample,capture]);
  const supported = job.unit === 'millimeter' && job.coordinate_frame === 'right_handed_z_up';
  return <div><p>{sample ? `实际样本 ${sample.step_id} · u=${sample.progress}` : '输入平面结构（不是计算过程样本）'}；毫米 / 右手 Z 向上。{capture?'只读捕获，使用请求相机与固定时点。':'右键环绕、中键平移、滚轮缩放。'}橙线为声明连接锚点参考，连接激活与干涉以实际检查为准。</p>{supported ? <div ref={container} className="packaging-viewport" aria-label="真实板片与实际样本三维回看" /> : <p role="status">此近景仅消费当前核心的 millimeter / right_handed_z_up；其他单位坐标仍保留原输入，不作猜测换算。</p>}</div>;
}
