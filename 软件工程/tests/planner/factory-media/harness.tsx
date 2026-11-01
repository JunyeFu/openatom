/// <reference types="vite/client" />
import { useEffect, useRef, useState } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import * as THREE from 'three';
import { FactoryScene } from '../../../apps/planner/src/scene/FactoryScene';
import { createFactoryPresentation, type FactoryPresentationSource, type FactoryPresentationContext } from '../../../apps/planner/src/services/factoryPresentation';
import type { ObjectRef } from '../../../packages/commands/generated/typescript/ObjectRef';
import type { MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import { fixture, fixturePorts, frames, originalRuns, sequential, sequentialOriginal, type ProbeRead } from './fixture';
import modelUrl from '../../../modules/media/p302-controlled/station.glb?url';
import simpleUrl from '../../../modules/media/p302-controlled/station-simple.glb?url';
import imageUrl from '../../../modules/media/p302-controlled/station.png?url';
import endUrl from '../../../modules/media/p302-controlled/station-end.png?url';
import gifUrl from '../../../modules/media/p302-controlled/station.gif?url';
import svgUrl from '../../../modules/media/p302-controlled/anchor.svg?url';
import jpegUrl from '../../../modules/media/p302-controlled/station.jpg?url';
import redUrl from '../media-root-sequence/red.png?url';

let latestRender: { renderer: THREE.WebGLRenderer; scene: THREE.Scene; camera: THREE.Camera } | null = null;
const nativeBeforeRender = THREE.Scene.prototype.onBeforeRender;
THREE.Scene.prototype.onBeforeRender = function (renderer, scene, camera, geometry, material, group) {
  latestRender = { renderer, scene: scene as THREE.Scene, camera };
  return nativeBeforeRender.call(this, renderer, scene, camera, geometry, material, group);
};
const disposed = { geometry: 0, material: 0, texture: 0, bitmap: 0 };
for (const [prototype, kind] of [[THREE.BufferGeometry.prototype, 'geometry'], [THREE.Material.prototype, 'material'], [THREE.Texture.prototype, 'texture'], [ImageBitmap.prototype, 'bitmap']] as const) {
  const method = kind === 'bitmap' ? 'close' : 'dispose';
  const p = prototype as unknown as Record<string, (...args: unknown[]) => unknown>, original = p[method];
  p[method] = function (...args) { disposed[kind]++; return original.apply(this, args); };
}
const reads: ProbeRead[] = [], releases: (() => void)[] = [];
let hold = false;
const declaredAssets = [...fixture().model.assets, { id: 'root.pixel.red', path: 'red.png', kind: 'png' }];
const urls: Record<string, string> = { 'station.glb': modelUrl, 'station-simple.glb': simpleUrl, 'station.png': imageUrl, 'station-end.png': endUrl, 'station.gif': gifUrl, 'anchor.svg': svgUrl, 'station.jpg': jpegUrl, 'red.png': redUrl };
async function readBytes(request: MediaAssetReadRequest) {
  if (!declaredAssets.some(asset => asset.id === request.asset.id && asset.path === request.asset.path && asset.kind === request.asset.kind)) throw new Error('Exact fixture AssetRef not declared');
  const record: ProbeRead = { request: structuredClone(request), state: 'started' }; reads.push(record);
  const url = urls[request.asset.path]; if (!url) throw new Error('Undeclared fixture AssetRef');
  const response = await fetch(url); if (!response.ok) throw new Error('Fixture bytes unavailable');
  const bytes = await response.arrayBuffer(); record.bytes = bytes.byteLength;
  if (hold) { record.state = 'held'; await new Promise<void>(resolve => releases.push(resolve)); }
  record.state = 'returned'; return bytes;
}
type State = { lod: 'detailed' | 'simplified' | 'region'; actual: boolean; at: number; project: string; mounted: boolean; canvas: 'line_canvas' | 'multi_hall_canvas'; auto: boolean; reset: number; imageKind: 'image' | 'gif'; family: 'calendar' | 'busy' | 'sequential' };
declare global { interface Window { __factory: { set(patch: Partial<State>): void; hold(): void; release(): void; read(): unknown; capture(): unknown } } }
function Harness() {
  const [state, setState] = useState<State>({ lod: 'detailed', actual: false, at: 35000, project: 'P1', mounted: true, canvas: 'multi_hall_canvas', auto: false, reset: 0, imageKind: 'image', family: 'calendar' });
  const [source, setSource] = useState<FactoryPresentationSource | null>(null), [failure, setFailure] = useState(''), [selection, setSelection] = useState<ObjectRef | null>(null);
  const current = useRef<FactoryPresentationContext | null>(null), activeSource = useRef(source); activeSource.current = source;
  const data = useRef(fixture());
  useEffect(() => {
    const dataNow = fixture(state.family); data.current = dataNow;
    const image = dataNow.project.devices.find(device => device.id === 'CNC')!;
    if (image && state.imageKind === 'gif' && dataNow.gif.definition.payload.category === 'media') image.presentation_override = dataNow.gif.definition.payload.data;
    if (state.family !== 'sequential') dataNow.project.id = state.project;
    // Actual frames keep their original frozen project identity and revision.
    const context: FactoryPresentationContext = { session_id: 'isolated-factory-browser', project_id: dataNow.project.id, scope: { kind: 'project' }, scope_revision: 9,
      project: { ...dataNow.project, revision: state.actual ? 99 : 1 }, view: state.actual ? { run_id: state.family === 'sequential' ? sequential.run.id : frames.find(value => value.label === `${state.family}@${state.at}`)!.frame.run.id, at_ms: state.at } : null };
    current.current = context; setSource(null); setFailure('');
    const { ports } = fixturePorts(dataNow, context, readBytes, state.family);
    const host = createFactoryPresentation(() => current.current, ports);
    let live = true;
    host.read(context).then(value => { if (live) setSource(value); }, error => { if (live) setFailure(String(error)); });
    return () => { live = false; current.current = null; };
  }, [state.actual, state.at, state.project, state.imageKind, state.family]);
  window.__factory = {
    set: patch => setState(value => ({ ...value, ...patch })), hold: () => { hold = true; }, release: () => { hold = false; for (const release of releases.splice(0)) release(); },
    read: () => ({ state, selection, reads, held: releases.length, disposed, sourceKey: activeSource.current?.key, failure, runUnchanged: originalRuns === JSON.stringify(frames.map(value => value.frame.run)) && sequentialOriginal === JSON.stringify(sequential),
      devices: activeSource.current?.devices.map(device => ({ device_id: device.device_id, frame: device.frame && { job_id: device.frame.job_id, at_ms: device.frame.snapshot.at_ms }, unavailable: device.unavailable, projection: device.projection })),
      offscreen: [...document.querySelectorAll('body > div[aria-hidden="true"]')].length }),
    capture() {
      if (!latestRender) return null;
      const { scene, camera, renderer } = latestRender;
      scene.updateMatrixWorld(true); const meshes: unknown[] = [], devices: unknown[] = [];
      scene.traverse(object => {
        if (object instanceof THREE.Group && object.userData.ref?.kind === 'device' && object.matrixAutoUpdate === false) {
          const box = new THREE.Box3().setFromObject(object), origin = object.getWorldPosition(new THREE.Vector3()), point = box.getCenter(new THREE.Vector3()).project(camera);
          devices.push({ id: object.userData.ref.id, origin: origin.toArray(), size: box.getSize(new THREE.Vector3()).toArray(), screen: [(point.x+1)/2*renderer.domElement.clientWidth,(1-point.y)/2*renderer.domElement.clientHeight], visible: object.visible });
        }
        if (object instanceof THREE.Mesh && object.name) meshes.push({ name: object.name, ref: object.userData.ref, world: object.getWorldPosition(new THREE.Vector3()).toArray(), local: object.position.toArray(), size: new THREE.Box3().setFromObject(object).getSize(new THREE.Vector3()).toArray() });
      });
      renderer.render(scene, camera);
      const gl = renderer.getContext(), pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
      gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      let colorful = 0; for (let i=0;i<pixels.length;i+=4) if (pixels[i] > pixels[i+1]*1.25 || pixels[i+2] > pixels[i]*1.25) colorful++;
      const textures: { id: string; rgba: number[]; glRgba: number[]; probes: number[]; frameIndex: string | undefined }[] = [];
      scene.traverse(object => { if (!(object instanceof THREE.Mesh)) return; const materials = Array.isArray(object.material) ? object.material : [object.material]; for (const material of materials) {
        const texture = (material as THREE.MeshBasicMaterial).map;
        if (texture?.image instanceof HTMLCanvasElement) { const canvas = texture.image, point = object.getWorldPosition(new THREE.Vector3()).project(camera), pixel = new Uint8Array(4);
          gl.readPixels(Math.floor((point.x+1)/2*gl.drawingBufferWidth), Math.floor((point.y+1)/2*gl.drawingBufferHeight), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
          const ctx = canvas.getContext('2d')!, probes: number[] = [];
          for (let y=.2;y<.8;y+=.05) for (let x=.2;x<.8;x+=.05) probes.push(...ctx.getImageData(canvas.width*x,canvas.height*y,1,1).data);
          textures.push({ id: object.userData.ref?.id, rgba: [...ctx.getImageData(canvas.width/2,canvas.height/2,1,1).data], glRgba: [...pixel], probes, frameIndex: canvas.dataset.frameIndex }); }
      } });
      return { meshes, devices, textures, colorful, camera: camera.position.toArray(), canvases: document.querySelectorAll('.factory-scene canvas').length,
        render: renderer.info.render, memory: renderer.info.memory, labels: [...document.querySelectorAll('.scene-label')].map(label => label.textContent) };
    },
  };
  const props = { project: data.current.project, canvas: state.canvas, hallId: 'H-1', selection, onSelect: setSelection, lod: state.lod,
    autoLod: state.auto, resetKey: state.reset, mediaSource: source };
  return <><p>独立协议端口 + 原FactoryScene/共享解码/单WebGL；原Engine帧不重跑、不改业务。{failure}</p>
    {state.mounted && source ? <FactoryScene {...props} /> : <p data-pending>读取来源</p>}</>;
}
const container = document.getElementById('root') as HTMLElement & { factoryRoot?: Root };
const root = container.factoryRoot ?? (container.factoryRoot = createRoot(container)); root.render(<Harness />);
if (import.meta.hot) import.meta.hot.dispose(() => { root.unmount(); if (container.factoryRoot === root) delete container.factoryRoot; });
