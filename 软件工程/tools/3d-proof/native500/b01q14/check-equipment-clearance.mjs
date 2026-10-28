import * as T from 'three';import fs from 'node:fs/promises';
import{build}from '../../../../modules/media/native-equipment-500/reviews/B01-Q14/source/crash-reviewed.mjs';
import{specs,processU,paperShiftX,paperDatumY,seatX}from '../../../../modules/media/native-equipment-500/reviews/B01-Q14/source/batch-spec.mjs';
import{piece,buildPaperSolids,intersection}from './collision-solids.mjs';
const home='modules/media/native-equipment-500/reviews/B01-Q14',spec=JSON.parse(await fs.readFile(home+'/catalog.json','utf8')).equipment[0],s=specs[spec.id],simple=!process.argv.includes('--detailed'),built=build(spec,simple),mixer=new T.AnimationMixer(built.root),meshes=[];
built.root.traverse(o=>{if(o.isMesh)meshes.push(o);});
import{localSolids}from './equipment-solids.mjs';
const vec=i=>new T.Vector3(...i),prismFaces=[[0,2,1],[3,4,5],[0,1,4,3],[1,2,5,4],[2,0,3,5]];
const overlap=(a,b)=>[0,1,2].every(i=>a.max[i]>b.min[i]+1e-9&&b.max[i]>a.min[i]+1e-9),rows=[];let total=0,contacts=0;
const contactOwner=(h,p)=>{let m;if((m=/^BottomHook([0-3])_ContactSpoon$/.exec(h)))return p===`Bottom${m[1]}_main`;if((m=/^Diagonal([02])_ThinSpatula$/.exec(h)))return p===`Bottom${m[1]}_ear`;if(/^Folder-1_.+ContactFinger$/.test(h))return p==='BodyPanel0';if(/^Folder1_.+ContactFinger$/.test(h))return p==='BodyPanel3';return h==='LeftRetainerShoe'&&p==='BodyPanel0';};
for(const phase of ['load','auto','unload']){
 const action=mixer.clipAction(built.animations.find(c=>c.name===phase));action.play();action.setLoop(T.LoopOnce);action.clampWhenFinished=true;
 const times=phase==='auto'?[...new Set([0,1,.02,.03,.05,.06,.20,.201,.209,.21,.22,.24,.25,.26,.265,.27,.275,.28,.42,.439,.44,.455,.47,.49,.50,.51,.52,.68,.685,.695,.70,.72,.74,.76,.78,.88,.89,.91,.92,.925,.935,.94,.97,.98,.985,...Array.from({length:101},(_,i)=>i/100)])].sort((a,b)=>a-b):Array.from({length:17},(_,i)=>i/16);
 for(const u of times){action.time=u;mixer.update(0);built.root.updateMatrixWorld(true);const shift=vec([paperShiftX+seatX(s,phase,u),paperDatumY(s),0]),paper=buildPaperSolids(processU(phase,u)).map(p=>piece(p.name,p.owner,p.verts.map(v=>v.clone().add(shift)),p.verts.length===6?prismFaces:[[0,1,2,3],[4,7,6,5],[0,4,5,1],[3,2,6,7],[0,3,7,4],[1,5,6,2]])),paperBox=new T.Box3().setFromPoints(paper.flatMap(p=>p.verts)),pb={min:paperBox.min.toArray(),max:paperBox.max.toArray()},issues=[],byHardware={};let count=0,candidates=0;
  for(const mesh of meshes){if(Math.abs(mesh.matrixWorld.determinant())<1e-20)continue;const bb=new T.Box3().setFromObject(mesh),hb={min:bb.min.toArray(),max:bb.max.toArray()};if(!overlap(pb,hb))continue;
   for(const [index,solid]of localSolids(mesh.geometry).entries()){const h=piece(mesh.name+'#'+index,mesh.name,solid.verts.map(v=>v.clone().applyMatrix4(mesh.matrixWorld)),solid.faces);if(!overlap(pb,h))continue;for(const p of paper){if(!overlap(p,h))continue;candidates++;const depth=intersection(p,h);if(depth!==null){if(depth<=3e-6&&contactOwner(mesh.name,p.owner)){contacts++;continue;}count++;byHardware[mesh.name]=Math.max(byHardware[mesh.name]??0,depth);if(issues.length<30)issues.push({paper:p.name,hardware:mesh.name,minimum_projection_overlap_m:depth});}}
   }
  }
  total+=count;rows.push({phase,u,intersections:count,candidates,byHardware,examples:issues});if(count)console.log(JSON.stringify({phase,u,count,first:issues[0]}));
 }action.stop();
}
const report={passed:total===0,model_lod:simple?'simple':'detailed',scope:'source finite paper solids versus animated equipment solids; exact exported/product and hardware-hardware integration still separate',samples:rows.length,total_intersections:total,designed_contacts_within_existing_3um_tolerance:contacts,penetration_epsilon_m:1e-9,rows,device_admitted:false};await fs.writeFile(home+(simple?'/equipment-stock-clearance-checks.json':'/equipment-stock-clearance-detailed-checks.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({passed:report.passed,samples:rows.length,total_intersections:total}));
