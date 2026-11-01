export const batch='B01-Q17',profile='simulation-efficient-v1',clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const specs={
 'rigid-box-wrapper':{title:'硬盒包边机（四角包覆与折入）',cycle_seconds:35,rule:'transform',initial:'unwrapped_rigid_box_core',final:'wrapped_turned_in_rigid_box',auxiliary:'precoated_notched_wrap_paper',mechanisms:['core-location-mandrel','four-side-wrap','corner-tuck-and-turn-in'],flow:{input:'1件硬盒盒芯 + 1张预涂胶切角面纸',output:'1件四面包覆、四角搭接及口沿折入的硬盒'}},
 'window-patcher':{title:'开窗贴膜机（平张预切膜贴合）',cycle_seconds:28,rule:'transform',initial:'die_cut_window_blank',final:'film_patched_window_blank',auxiliary:'perimeter_precoated_PET_patch',mechanisms:['open-window-vacuum-frame','film-vacuum-pick','perimeter-press'],flow:{input:'1件已开窗箱坯 + 1片周边预涂胶PET膜',output:'1件保留透明窗口的贴膜箱坯'}},
 'carton-stitcher':{title:'钉箱机（三钉侧缝接合）',cycle_seconds:20,rule:'transform',initial:'folded_unstitched_case',final:'three_stitch_joined_case',auxiliary:'three_metered_wire_segments',mechanisms:['indexed-wire-former','three-driver-strokes','lower-clinch-die'],flow:{input:'1件侧缝待钉合平折纸箱 + 1组总长171mm钉线',output:'1件含三枚成形铆合钉的纸箱'}}
};
export const workY=.90,inputX=-1,outputX=1.32,stagingX=.76,inputPort=()=>[inputX,workY,0],outputPort=()=>[outputX,workY,0],auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.90,workY,-.52],outputY=()=>workY;
export const wrap={L:.2234,W:.1634,H:.055,turn:.020,rim:.00179,ear:.012,paper:.00018,board:.0015};
export const windowBlank={L:.48,W:.32,t:.0012,holeL:.16,holeW:.08,filmL:.18,filmW:.10,filmT:.0001,glueT:.00015};
export const stitched={L:.44,W:.32,ply:.003,seamZ:.13,seamT:.0092,wireR:.00035,crown:.020,leg:.0096,tail:.0089,wireLength:.171};
export const stitchStarts=[.08,.36,.64],stitchXs=[-.10,0,.10];
export const stitchState=(index,u)=>{const a=stitchStarts[index];return{feed:ramp(u,a-.06,a),form:ramp(u,a,a+.06),drive:ramp(u,a+.06,a+.16),clinch:ramp(u,a+.16,a+.22)};};
export const headX=u=>-.10+.10*ramp(u,.30,.36)+.10*ramp(u,.58,.64);
export const headStroke=u=>stitchStarts.reduce((v,a)=>v+ramp(u,a+.06,a+.16)-ramp(u,a+.22,a+.28),0);
export const sideAngle=u=>Math.PI/2*ramp(u,.10,.30),earAngle=u=>Math.PI/2*ramp(u,.32,.44),endAngle=u=>Math.PI/2*ramp(u,.46,.62),turnAngle=u=>Math.PI/2*ramp(u,.66,.74),insideAngle=u=>Math.PI/2*ramp(u,.74,.84);
const boundaries=[.10,.30,.32,.44,.46,.62,.66,.74,.80,.84,.85,.92,.5,.7,.75,.9,.18,.20,.65,...stitchStarts.flatMap(a=>[a-.06,a,a+.06,a+.16,a+.22,a+.28])];
export const samples=[...new Set([0,1,...Array.from({length:33},(_,i)=>i/32),...boundaries])].sort((a,b)=>a-b);
const transfer=(u,a,b,start,end,lift=.14)=>{const v=ramp(u,a,b);return[start[0]+(end[0]-start[0])*v,start[1]+(end[1]-start[1])*v+lift*(ramp(v,0,.15)-ramp(v,.85,1)),start[2]+(end[2]-start[2])*v];};
export function seat(id,phase,u){const raised=id==='rigid-box-wrapper'?wrap.paper:0;if(phase==='load')return transfer(u,id==='rigid-box-wrapper'?.5:0,id==='rigid-box-wrapper'?1:.5,inputPort(),[0,workY+raised,0]);return[phase==='unload'?stagingX*u:0,workY+raised,0];}
export function filmPose(phase,u){if(phase==='load'){if(u<.7)return transfer(u,.5,.7,auxPort(),auxSeat());if(u<.8)return auxSeat();if(u<.9)return[.9,workY+.22*ramp(u,.8,.9),-.52];const v=ramp(u,.9,1);return[.9*(1-v),1.12,-.52*(1-v)];}if(phase==='auto')return[0,1.12-(1.12-(workY+windowBlank.t+windowBlank.glueT))*ramp(u,0,.20),0];return[stagingX*u,workY+windowBlank.t+windowBlank.glueT,0];}
export function filmHead(phase,u){const p=filmPose(phase,u);if(phase==='load'&&u<.7)return[.9,1.12,-.52];if(phase==='load'&&u<.8)return[.9,1.12-(1.12-workY)*ramp(u,.7,.8),-.52];if(phase==='auto'&&u>.65)return[0,p[1]+.22*ramp(u,.65,.8),0];if(phase==='unload')return[0,1.12,0];return p;}
export const wrapPaperPose=(phase,u)=>phase==='load'?transfer(u,0,.45,auxPort(),[0,workY,0]):[phase==='unload'?stagingX*u:0,workY,0];
