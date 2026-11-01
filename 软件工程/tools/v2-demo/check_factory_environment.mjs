// Real frozen factory geometry and labels; no Host or inventory replacement.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {pathToFileURL} from 'node:url';
const [proof,output]=process.argv.slice(2),root=process.cwd(),require=createRequire(path.join(root,'package.json'));
await fs.mkdir(output,{recursive:false});
const three=pathToFileURL(require.resolve('three').replace(/three\.cjs$/,'three.module.js')).href;
for(const name of ['layout','factoryEnvironment','factoryLabels']){
 const source=stripTypeScriptTypes(await fs.readFile(path.join(root,'apps/planner/src/scene',name+'.ts'),'utf8'),{mode:'transform'})
  .replaceAll("from 'three'",`from '${three}'`).replaceAll("from './layout'","from './layout.mjs'");
 await fs.writeFile(path.join(output,name+'.mjs'),source);
}
const THREE=await import(three),{poseMatrix}=await import(pathToFileURL(path.join(output,'layout.mjs')).href);
const {createFactoryHallShell,createFactoryStockSupport,factoryStockSupportLayout}=await import(pathToFileURL(path.join(output,'factoryEnvironment.mjs')).href);
const {factoryItemTitle,factoryItemPlace,factoryUnitLabel}=await import(pathToFileURL(path.join(output,'factoryLabels.mjs')).href);
const project=JSON.parse(await fs.readFile(path.join(proof,'actual-frozen-project.json'),'utf8'));
const frames=JSON.parse(await fs.readFile(path.join(proof,'actual-observation-frames.json'),'utf8'));
const before=JSON.stringify(project),hall=project.halls.find(h=>h.id==='PF-H1'),shipping=project.locations.find(l=>l.id==='PF-SHIPPING');
const checks=[];function check(name,body){body();checks.push(name);}
check('The cutaway envelope uses the declared104x90x9m hall',()=>{
 const shell=createFactoryHallShell(hall),bounds=new THREE.Box3().setFromObject(shell);
 assert.ok(bounds.min.toArray().every(v=>Math.abs(v)<1e-6));assert.ok(bounds.max.toArray().every((v,i)=>Math.abs(v-[104,90,9][i])<1e-6));
 assert.equal(shell.children.filter(o=>o.name==='BackWallCutaway').length,1);
 assert.equal(shell.children.filter(o=>o.name==='FrontWall').length,0);
});
check('Unknown hall height adds no invented walls or columns',()=>assert.equal(createFactoryHallShell({...hall,height_m:null}).children.length,0));
check('Shipping deck supports all33 declared slots at the actual0.9m work height',()=>{
 assert.equal(shipping.item_layout.slots.length,33);
 const support=createFactoryStockSupport(project,shipping);assert.ok(support);
 const holder=new THREE.Group();holder.matrix.copy(poseMatrix(shipping.position));holder.matrixAutoUpdate=false;holder.add(support);holder.updateMatrixWorld(true);
 const deck=support.children.find(o=>o.name==='StorageDeck'),bounds=new THREE.Box3().setFromObject(deck);
 assert.ok(Math.abs(bounds.max.z-.9)<1e-7);
 for(const slot of shipping.item_layout.slots){const p=new THREE.Vector3().copy(slot.offset.translation).applyMatrix4(poseMatrix(shipping.position));assert.ok(p.x>bounds.min.x&&p.x<bounds.max.x&&p.y>bounds.min.y&&p.y<bounds.max.y);}
 const all=new THREE.Box3().setFromObject(holder);assert.ok(Math.abs(all.min.z)<1e-7);
});
check('Line-local machine footprints suppress duplicate free-stock furniture',()=>{
 const device=project.devices.find(d=>d.line_id&&d.footprint_m.length>=3),line=project.lines.find(l=>l.id===device.line_id);
 const center=device.footprint_m.reduce((a,p)=>({x:a.x+p.x/device.footprint_m.length,y:a.y+p.y/device.footprint_m.length}),{x:0,y:0});
 const p=new THREE.Vector3(center.x,center.y,0).applyMatrix4(poseMatrix(line.transform).multiply(poseMatrix(device.pose)));
 const location={...shipping,hall_id:device.hall_id,position:{...shipping.position,translation:{x:p.x,y:p.y,z:.9}}};
 assert.equal(factoryStockSupportLayout(project,location),null);
 const shifted=structuredClone(project),moved=shifted.lines.find(l=>l.id===line.id);moved.transform.translation.x+=200;
 location.position.translation.x+=200;assert.equal(factoryStockSupportLayout(shifted,location),null);
});
check('Names and places describe the actual five finished product families',()=>{
 const items=frames.at(-1).objects.filter(o=>o.kind==='item').map(o=>o.data),finished=items.filter(i=>i.holder.kind==='location'&&i.holder.id==='PF-SHIPPING');
 assert.equal(finished.length,33);
 const names=[...new Set(finished.map(i=>factoryItemTitle(project,i)))];assert.equal(names.length,5);
 assert.ok(names.every(n=>/[\u4e00-\u9fff]/.test(n)&&!n.includes('output')));
 assert.ok(finished.every(i=>factoryItemPlace(project,i)===shipping.title));assert.equal(factoryUnitLabel('piece'),'件');
});
assert.equal(JSON.stringify(project),before);
await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify({source:proof,checks,immutable_project:true,ordinary_Native_verified:false},null,2));
console.log('Factory geometry and naming checks passed:',checks.length);
