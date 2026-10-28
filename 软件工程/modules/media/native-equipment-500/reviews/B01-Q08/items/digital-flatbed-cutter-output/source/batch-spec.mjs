export const specs={'digital-flatbed-cutter':{mode:'digital_cut_crease',size:[.50,.003,.40],surface:1.08,inputX:-1.60,outputX:1.60,stagingX:1.05,initial:'uncut_uncreased_sheet',final:'cut_and_creased_cross_blank',auxiliary:null,area:.20,product_area:.16,crease_depth:.0003,extra_outputs:[{role:'scrap',count:4,size:[.10,.003,.10],area:.01}]}};
export const shapePoints=[[-.15,-.20],[.15,-.20],[.15,-.10],[.25,-.10],[.25,.10],[.15,.10],[.15,.20],[-.15,.20],[-.15,.10],[-.25,.10],[-.25,-.10],[-.15,-.10]];
export const creasePoints=[[-.15,-.10],[.15,-.10],[.15,.10],[-.15,.10]],cornerPositions=[[-.20,0,-.15],[.20,0,-.15],[-.20,0,.15],[.20,0,.15]];
export const clamp=v=>Math.max(0,Math.min(1,v)),inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0],scrapPort=s=>[0,.86,-.98];
export const trayTop=(s,phase,u)=>phase==='load'?s.surface-.06+.06*clamp((u-.60)/.40):phase==='auto'?s.surface-.06*clamp((u-.82)/.06):s.surface-.06;
export const trayZ=(sign,phase,u)=>sign*(phase==='load'?.65-.50*clamp(u/.60):phase==='auto'?.15+.50*clamp((u-.88)/.10):.65);
export const seatX=(s,phase,u)=>phase==='load'?s.inputX*(1-u):phase==='auto'?0:s.stagingX*u;
const home=[-.35,-.25],offset={crease:.035,knife:-.035},up=1.12,down={crease:1.0827,knife:1.08},segments=[];
let current={x:home[0],z:home[1],knifeY:up,creaseY:up,knifeYaw:0,creaseYaw:0,creaseDistance:0,creaseDone:0},clock=0;
function append(kind,tool,to,duration){const from={...current};segments.push({kind,tool,from,to:{...from,...to},start:clock,end:clock+duration});clock+=duration;current={...from,...to};}
function turn(from,to){const d=Math.atan2(Math.sin(to-from),Math.cos(to-from));return from+d;}
for(const [tool,points]of[['crease',creasePoints],['knife',shapePoints]])for(let i=0;i<points.length;i++){
 const a=points[i],b=points[(i+1)%points.length],yaw=-Math.atan2(b[1]-a[1],b[0]-a[0]),x=a[0],z=a[1]-offset[tool],length=Math.hypot(b[0]-a[0],b[1]-a[1]);
 append('travel',tool,{x,z,[tool+'Yaw']:turn(current[tool+'Yaw'],yaw)},.02+Math.hypot(x-current.x,z-current.z)/.40);
 append('lower',tool,{[tool+'Y']:down[tool]},.10);
 append('stroke',tool,{x:b[0],z:b[1]-offset[tool],creaseDistance:current.creaseDistance+(tool==='crease'?length:0),creaseDone:current.creaseDone+(tool==='crease'?1:0)},length/(tool==='crease'?.12:.15));
 append('raise',tool,{[tool+'Y']:up},.10);
}
export const program=segments.map(s=>({...s,start:s.start/clock*.78,end:s.end/clock*.78}));
export function toolPose(u){
 if(u>=.78){const f=clamp((u-.78)/.02);return{...current,x:current.x+(home[0]-current.x)*f,z:current.z+(home[1]-current.z)*f,knifeY:up,creaseY:up,creaseDone:4};}
 const s=program.find(s=>u<=s.end)??program[program.length-1],f=clamp((u-s.start)/(s.end-s.start)),p={};for(const k of Object.keys(current))p[k]=s.from[k]+(s.to[k]-s.from[k])*f;p.creaseDone=s.kind==='stroke'&&s.tool==='crease'&&f<1?s.from.creaseDone:s.to.creaseDone;return p;
}
// Lead screw key spacing keeps each quaternion step below a quarter turn.
export const samples=[...new Set([0,1,.78,.80,.82,.88,.98,...Array.from({length:81},(_,i)=>.78+.02*i/80),...Array.from({length:513},(_,i)=>i/512),...program.flatMap(s=>{const n=Math.max(2,Math.ceil(Math.max(Math.abs(s.to.x-s.from.x),Math.abs(s.to.z-s.from.z))/.0025));return Array.from({length:n+1},(_,i)=>s.start+(s.end-s.start)*i/n);})])].sort((a,b)=>a-b);
