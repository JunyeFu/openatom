import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { buildModulePresentation } from './modulePresentation';
import type { ModulePresentationScene, PresentationDiagnostic } from './modulePresentation';
import type { Presentation } from '../../../../packages/commands/generated/typescript/Presentation';
import './ModulePresentationPreview.css';

export interface ModulePresentationPreviewProps { presentation: Presentation | null; title?: string }
interface PreviewRuntime { replace(value: Presentation | null): void; fit(view?: 'iso' | 'top' | 'front'): void }
export function ModulePresentationPreview({ presentation, title = '定义草稿 · 原生部件预览' }: ModulePresentationPreviewProps) {
  const host = useRef<HTMLDivElement>(null), runtime = useRef<PreviewRuntime | null>(null);
  const current = useRef(presentation); current.current = presentation;
  const [error, setError] = useState(''), [diagnostics, setDiagnostics] = useState<PresentationDiagnostic[]>([]), [count, setCount] = useState(0);
  useEffect(() => {
    const element = host.current!; let renderer: THREE.WebGLRenderer;
    try { renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false }); }
    catch { setError('无法初始化 WebGL。当前没有三维绘制；可继续编辑数值，定义与运行状态未改变。'); return; }
    setError(''); renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2)); renderer.setClearColor(0xe8edf2); element.appendChild(renderer.domElement);
    renderer.domElement.setAttribute('aria-label', '声明原生部件的静态三维预览');
    const scene = new THREE.Scene(), camera = new THREE.PerspectiveCamera(42, 1, .001, 1000);
    const controls = new OrbitControls(camera, renderer.domElement); controls.enableDamping = false;
    controls.mouseButtons = { LEFT: THREE.MOUSE.ROTATE, MIDDLE: THREE.MOUSE.PAN, RIGHT: THREE.MOUSE.PAN };
    const physical = new THREE.Group(); physical.rotation.x = -Math.PI / 2; scene.add(physical);
    scene.add(new THREE.HemisphereLight(0xffffff, 0x8a9db1, 2.4));
    const light = new THREE.DirectionalLight(0xffffff, 2.4); light.position.set(4, 7, 5); scene.add(light);
    const grid = new THREE.GridHelper(10, 20, 0x9fb3c6, 0xd1dde6); scene.add(grid);
    let model: ModulePresentationScene | null = null, fitted = false, width = 0, height = 0;
    const render = () => { if (width > 0 && height > 0) renderer.render(scene, camera); };
    function fit(view: 'iso' | 'top' | 'front' = 'iso') {
      physical.updateMatrixWorld(true);
      const bounds = model ? new THREE.Box3().setFromObject(model.root) : new THREE.Box3();
      const center = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
      const extent = bounds.isEmpty() ? 1 : Math.max(bounds.getSize(new THREE.Vector3()).length(), .001);
      const distance = extent / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2)) * .7 / Math.min(1, camera.aspect);
      const direction = view === 'top' ? new THREE.Vector3(.0001, 1, 0) : view === 'front' ? new THREE.Vector3(0, 0, 1) : new THREE.Vector3(1, .8, 1);
      camera.position.copy(center).add(direction.normalize().multiplyScalar(distance)); camera.near = extent / 10000; camera.far = Math.max(extent * 100, 10); camera.updateProjectionMatrix();
      controls.target.copy(center); controls.minDistance = extent / 1000; controls.maxDistance = extent * 100; controls.update(); render();
    }
    runtime.current = { fit, replace(value) {
      if (model) { physical.remove(model.root); model.dispose(); model = null; }
      if (value) { model = buildModulePresentation(value); physical.add(model.root); }
      setDiagnostics(model?.diagnostics ?? []); setCount(model?.nativeMeshCount ?? 0);
      if (!fitted && model && model.nativeMeshCount > 0) { fit(); fitted = width > 0 && height > 0; } else render();
    } };
    controls.addEventListener('change', render);
    const resize = new ResizeObserver(entries => { const rect = entries[0].contentRect; width = rect.width; height = rect.height; if (width <= 0 || height <= 0) return; renderer.setSize(width, height); camera.aspect = width / height; camera.updateProjectionMatrix(); if (!fitted) { fit(); fitted = !!model && model.nativeMeshCount > 0; } else render(); });
    resize.observe(element);
    const lost = (event: Event) => { event.preventDefault(); setError('WebGL 上下文已丢失，当前画面不能作为最新预览。请重新打开预览；输入仍保留。'); };
    renderer.domElement.addEventListener('webglcontextlost', lost);
    runtime.current.replace(current.current);
    return () => { runtime.current = null; resize.disconnect(); controls.removeEventListener('change', render); controls.dispose(); renderer.domElement.removeEventListener('webglcontextlost', lost); model?.dispose(); grid.geometry.dispose(); (grid.material as THREE.Material).dispose(); renderer.dispose(); renderer.domElement.remove(); };
  }, []);
  useEffect(() => { runtime.current?.replace(presentation); }, [presentation]);
  return <section className="module-presentation-preview" aria-label={title}><header><div><h3>{title}</h3><p>静态未提交预览 · {count} 个声明原生网格 · 不产生库存、产量或生产时钟</p></div><div className="module-preview-controls"><button onClick={() => runtime.current?.fit('iso')} disabled={!!error}>适配全部</button><button onClick={() => runtime.current?.fit('top')} disabled={!!error}>俯视</button><button onClick={() => runtime.current?.fit('front')} disabled={!!error}>正视</button></div></header><div className="module-preview-viewport" ref={host} />{!presentation && <p className="module-preview-empty">从定义表单明确开启预览；编辑将更新同一份 Presentation。</p>}{error && <p role="alert" className="module-preview-error">{error}</p>}<p className="module-preview-help">网格单位：米{presentation && ` · 定义几何：${presentation.unit === 'millimeter' ? '毫米' : '米'} · ${presentation.coordinate_frame === 'right_handed_y_up' ? 'Y向上' : 'Z向上'} · 根缩放 ${presentation.scale.x} / ${presentation.scale.y} / ${presentation.scale.z}`}</p><p className="module-preview-help">拖动旋转，滚轮缩放；数值编辑在部件表单中完成。原生尺寸使用表现单位，位置使用各姿态单位。</p>{diagnostics.length > 0 && <ul className="module-preview-diagnostics">{diagnostics.map((issue, index) => <li key={index}>{issue.node_id !== undefined ? `部件 ${issue.node_id}：` : ''}{issue.message}</li>)}</ul>}</section>;
}
