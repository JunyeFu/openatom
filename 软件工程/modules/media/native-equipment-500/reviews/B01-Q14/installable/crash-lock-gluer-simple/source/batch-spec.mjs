import {carton,manufactureState,glueArea,gluePolygon,V}from './lock-bottom-kinematics.mjs';
import {finitePaperPoint,finiteCoreAnchor,developedArea}from './finite-scored-bands.mjs';
export const specs={'crash-lock-gluer':{mode:'indexed_crash_lock_bottom',surface:.90,inputX:-1.60,outputX:1.60,stagingX:1.60,initial:'precreased_auto_bottom_blank',final:'pre_glued_flat_auto_bottom_carton',auxiliary:'metered_water_based_glue',volume:glueArea()*carton.glueThickness,extra_outputs:[],cycle_seconds:34}};
export const clamp=v=>Math.max(0,Math.min(1,v)),paperShiftX=-carton.height/2;
export const paperDatumY=s=>s.surface+carton.paperThickness/2;
export const seatX=(s,phase,u)=>phase==='load'?s.inputX*(1-u):phase==='unload'?s.stagingX*u:0;
export const clockKnots=[[0,0],[.20,.20],[.28,.22],[.42,.34],[.44,.35],[.49,.39],[.52,.40],[.68,.58],[.78,.64],[.92,.80],[1,1]];
export const processU=(phase,u)=>{if(phase==='load')return 0;if(phase==='unload')return 1;for(let i=1;i<clockKnots.length;i++)if(u<=clockKnots[i][0]){const[a,x]=clockKnots[i-1],[b,y]=clockKnots[i];return x+(y-x)*(u-a)/(b-a);}return 1;};
export const sourceArea=()=>developedArea();
export const inputPort=s=>[s.inputX,paperDatumY(s),0],outputPort=s=>[s.outputX,paperDatumY(s),0],auxPort=s=>[-1.60,.908,-1.05],auxSeat=s=>[.50,1.10,-.75];
export const glueFraction=(phase,u)=>phase==='load'?0:phase==='unload'?1:clamp((u-.44)/.05);
export const glueParts=[{role:'bottom_0',part:{kind:'bottom',index:0,piece:'ear'},points:gluePolygon(0),surfaceSide:1},{role:'bottom_2',part:{kind:'bottom',index:2,piece:'ear'},points:gluePolygon(2),surfaceSide:1},{role:'side',part:{kind:'body',index:4},points:[[0,0,-.22],[carton.height,0,-.22],[carton.height,0,-.206],[0,0,-.206]].map(V),surfaceSide:-1}];
export function coatedPoint(patch,p,phase,u,layerFraction=1){const q=p.clone().add(V([0,patch.surfaceSide*(carton.paperThickness/2+carton.glueThickness*layerFraction/2),0]));return finitePaperPoint(patch.part,q,processU(phase,u)).add(V([paperShiftX,0,0]));}
export const doseRadius=.008,doseSegments=32,doseHeight=specs['crash-lock-gluer'].volume/(doseSegments/2*doseRadius**2*Math.sin(2*Math.PI/doseSegments));
export function upperGlueCenters(s){return glueParts.filter(p=>p.role.startsWith('bottom')).map(patch=>{const c=patch.points.reduce((a,p)=>a.add(p),V([0,0,0])).multiplyScalar(1/patch.points.length),q=coatedPoint(patch,c,'auto',.44);return{role:patch.role,point:[q.x,paperDatumY(s)+q.y,q.z]};});}
export {carton,manufactureState,finitePaperPoint,finiteCoreAnchor};

export const thickness=carton.paperThickness,neutral=thickness/2,leftRadius=.0021,rightRadius=.0024;
export const foldAngle=(side,phase,u)=>{if(phase==='unload')return Math.PI*(1-clamp((u-.70)/.30));const m=manufactureState(processU(phase,u));return side<0?m.left:m.right;};
export const foldRetract=(side,phase,u)=>phase==='load'?.30*(1-clamp(u/.5)):phase==='unload'?.30:.30*clamp((u-(side<0?.70:.925))/.01);
export const folderOffset=(phase,u)=>phase==='auto'?.30-.20*clamp(u/.04)+.20*clamp((u-.94)/.03):.30;
export const leftClampBottom=(phase,u)=>phase==='auto'?1.08-(1.08-.90470)*clamp((u-.685)/.01)+(1.08-.90470)*clamp((u-.98)/.02):1.08;
export const pressBottom=(phase,u)=>phase==='auto'?1.08-(1.08-.90530)*clamp((u-.95)/.02)+(1.08-.90530)*clamp((u-.98)/.02):1.08;
export const sideGlueZ=-.213-Math.PI*.0021;
export const glueHeadX=(phase,u)=>phase!=='auto'?.20:u<.40?.20:u<.42?.20-.32*clamp((u-.40)/.02):u<.44?-.12:u<.49?-.12+.24*clamp((u-.44)/.05):.12+.08*clamp((u-.49)/.04);
export const glueHeadY=(phase,u)=>phase==='auto'?.892+.0079*clamp((u-.42)/.02)-.0079*clamp((u-.49)/.03):.892;
export const gluePoint=(phase,u)=>{const patch=glueParts[2],p=coatedPoint(patch,V([carton.height/2,0,-.213]),phase,u);return{x:p.x,y:p.y,z:p.z};};
export const samples=[...new Set([0,1,.40,.53,.74,.76,.78,.925,.935,.95,.985,.02,.03,.04,.05,.06,.20,.201,.209,.21,.22,.24,.25,.26,.265,.27,.275,.28,.42,.439,.44,.455,.47,.49,.50,.51,.52,.68,.685,.695,.70,.72,.88,.89,.91,.92,.94,.97,.98,...Array.from({length:513},(_,i)=>i/512)])].sort((a,b)=>a-b);

export const folderUnitOffset=(side,phase,u)=>phase==='auto'?1.30-1.20*clamp((u-(side<0?.46:.74))/(side<0?.06:.04))+1.20*clamp((u-(side<0?.71:.94))/(side<0?.05:.06)):1.30;
