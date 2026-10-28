// Reconstruct only this leased file's pre-integration render body from the
// exact opening read, run the identical real-browser probe, then restore it.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),path=new URL('apps/planner/src/scene/FactoryScene.tsx',root),out=new URL('scenarios/媒体表现/runtime-factory-canvas/',root);
await mkdir(out,{recursive:true});
const fixed=await readFile(path,'utf8');let old=fixed;
old=old.replace(/import \{ createFactoryMedia[^\n]*\n/,'').replace(/import type \{ FactoryPresentationSource[^\n]*\n/,'');
old=old.replace('export interface FactorySceneProps {','interface Props {').replace(/  \/\*\* Clear on context changes[^\n]*\n  mediaSource\?: FactoryPresentationSource \| null;\n/,'');
old=old.replace('  group.matrixWorldNeedsUpdate = true;\n','');
old=old.replace(/export function FactoryScene\(\{ project: inputProject,[\s\S]*?  const host =/,'export function FactoryScene({ project, canvas, hallId, selection, onSelect, lod, autoLod, resetKey, run = null, observation = null }: Props) {\n  const host =');
old=old.replace(/    let live = true;[\s\S]*?    const anchors =/,'    const anchors =');
const originalDevice=`        const mesh = polygon(device.footprint_m, device.height_m, selected ? 0x287bcd : activity?.work ? 0x468c80 : activity?.waits ? 0xba9464 : 0x839bad);
        annotations.push({ key: \`device:\${device.id}\`, title: \`\${device.title}\${mesh ? '' : ' · 占地未声明（定位标记）'}\${activity?.work ? \` · 执行 \${activity.work}\` : activity?.waits ? \` · 等待记录 \${activity.waits}\` : ''}\`, anchor: group, ref: { kind: 'device', id: device.id }, kind: 'device' });
        if (!mesh) {
          // A screen-size point marks the declared pose; it is no physical body
          // or invented footprint. The label remains the accessible selector.
          const marker = new THREE.Points(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3()]), new THREE.PointsMaterial({ color: selected ? 0x287bcd : 0x46647b, size: 7, sizeAttenuation: false }));
          group.add(marker); deviceObjects.push(marker);
          if (device.line_id && project!.lines.some(line => line.id === device.line_id && line.region_polygon_m.length >= 3)) groupedObjects.push(marker);
          continue;
        }
        mesh.userData.ref = { kind: 'device', id: device.id };
        group.add(mesh); selectable.push(mesh); deviceObjects.push(mesh);
        if (device.line_id && project!.lines.some(line => line.id === device.line_id && line.region_polygon_m.length >= 3)) groupedObjects.push(mesh);
        const edges = new THREE.LineSegments(new THREE.EdgesGeometry(mesh.geometry), new THREE.LineBasicMaterial({ color: selected ? 0x0e4e8b : 0x46647b }));
        group.add(edges); deviceObjects.push(edges);
        if (device.line_id && project!.lines.some(line => line.id === device.line_id && line.region_polygon_m.length >= 3)) groupedObjects.push(edges);`;
old=old.replace(/        \/\/ Footprints are flat layout markers[\s\S]*?        mediaSlots.push\([^\n]*\);/,originalDevice);
old=old.replace(/    const loadLod = \(current: LodMode\) => \{[\s\S]*?    const render =/,'    const render =');
old=old.replace('      if (!live) return;\n','').replace('deviceObjects.forEach(value => { value.visible = true; });',"deviceObjects.forEach(value => { value.visible = current === 'detailed' || !(value instanceof THREE.LineSegments); });").replace('        loadLod(current);\n','');
old=old.replace('mediaSlots.some(slot => slot.group === annotation.anchor && slot.grouped)','groupedObjects.some(object => object.parent === annotation.anchor)');
old=old.replace(/      const hit = ray.intersectObjects\(selectable, true\).find\(value => \{[\s\S]*?      \}\);/,'      const hit = ray.intersectObjects(selectable).find(value => value.object.visible);');
old=old.replace(/      live = false;[\s\S]*?      cameras.current.set\(/,'      cameras.current.set(').replace('object instanceof THREE.Line || object instanceof THREE.Points','object instanceof THREE.LineSegments');
old=old.replace('resetKey, run, observation, mediaSource]);','resetKey, run, observation]);');
if(old.includes('factoryMedia')||old.includes('mediaSlots')||old.includes('mediaSource'))throw new Error('Original reconstruction incomplete');
await writeFile(new URL('FactoryScene.before.tsx.txt',out),old);await writeFile(new URL('FactoryScene.after.tsx.txt',out),fixed);
try{await writeFile(path,old);const result=spawnSync(process.execPath,['tests/planner/factory-media/browser.mjs','red'],{cwd:fileURLToPath(root),encoding:'utf8'});await writeFile(new URL('browser-red.stdout.txt',out),result.stdout??'');await writeFile(new URL('browser-red.stderr.txt',out),result.stderr??'');console.log(result.stdout);if(result.error)throw result.error;if(result.status===0)throw new Error('Original detailed renderer unexpectedly passed');}
finally{await writeFile(path,fixed);}
