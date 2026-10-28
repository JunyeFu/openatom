export const batch='B01-Q19',profile='simulation-efficient-v1',clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const specs={
 strapper:{title:'纸箱捆扎机（双道预切PP带热合）',cycle_seconds:14,rule:'preserve',initial:'unstrapped_pack',final:'two_band_pack',auxiliary:'two_metered_PP_straps',mechanisms:['two-guided-band-arches','lower-clamp-tension-and-heat-joint'],flow:{input:'1件包件+2根各980mm预切PP带',output:'同一包件及6个真实内装物，两道捆扎'}},
 'shrink-wrapper':{title:'热收缩包装机（预制膜罩与升降热风罩）',cycle_seconds:25,rule:'preserve',initial:'unwrapped_pack',final:'shrink_hood_pack',auxiliary:'preformed_PE_hood',mechanisms:['lifted-thermal-chamber','hot-air-circulation-and-preformed-hood'],flow:{input:'1件包件+1件预制PE膜罩',output:'同一包件及6个真实内装物的收缩包装'}},
 'case-packer':{title:'龙门装箱机（六件阵列真空取放）',cycle_seconds:24,rule:'pack',initial:'unpacked_product',final:'packed_open_case',auxiliary:null,mechanisms:['XYZ-vacuum-array','six-real-products-into-one-open-case'],flow:{input:'6件独立产品+1只已成型开口纸箱',output:'1件新包件，纸箱作为材料来源、6件原产品作为contents'}}
};
export const box={L:.32,W:.24,H:.24,t:.003,flap:.119},unit={L:.08,W:.06,H:.12},workY=.9,stagingX=.76,outputX=1.32;
export const inputPort=()=>[-1,workY,0],outputPort=()=>[outputX,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.9,workY,-.52];
export const contentSeats=()=>[-.07,.07].flatMap(x=>[-.07,0,.07].map(z=>[x,box.t,z]));
export const samples=[...new Set([0,1,...Array.from({length:17},(_,i)=>i/16),.08,.15,.18,.25,.32,.35,.45,.5,.55,.65,.70,.72,.78,.80,.85,.9])].sort((a,b)=>a-b);
export function seat(id,ph,u){if(id==='case-packer')return[ph==='unload'?stagingX*u:0,workY,0];if(ph==='load')return[-1*(1-ramp(u,0,.5)),workY+.18*(ramp(u,0,.075)-ramp(u,.425,.5)),0];return[ph==='unload'?outputX*u:0,workY,0];}
export function caseSeat(ph,u){if(ph==='load'){const v=ramp(u,0,.5);return[-1+v,workY+.25*(ramp(v,0,.15)-ramp(v,.85,1)),-.52*(1-v)];}return seat('case-packer',ph,u);}
export function productSeat(ph,u){if(ph==='load')return[-1,workY,0];if(ph==='unload')return[stagingX*u,workY+box.t,0];if(u<.18)return[-1,workY,0];if(u<.32)return[-1,workY+.45*ramp(u,.18,.32),0];if(u<.55)return[-1+ramp(u,.32,.55),1.35,0];return[0,1.35-(1.35-workY-box.t)*ramp(u,.55,.72),0];}
export function pickupHead(ph,u){if(ph==='load')return[0,1.5,0];if(ph==='unload')return[0,1.5,0];if(u<.08)return[-ramp(u,0,.08),1.5,0];if(u<.18)return[-1,1.5-(1.5-workY-unit.H-.01)*ramp(u,.08,.18),0];const p=productSeat(ph,u);return[p[0],u>.78?p[1]+unit.H+.01+(1.5-p[1]-unit.H-.01)*ramp(u,.78,.9):p[1]+unit.H+.01,0];}
export function auxiliaryPose(id,ph,u){const base=auxSeat();if(ph==='load'){const v=ramp(u,.5,1),to=id==='shrink-wrapper'?[0,workY,0]:base;return[-1+(to[0]+1)*v,workY+(id==='shrink-wrapper'?.45:.15)*(ramp(v,0,.15)-ramp(v,.85,1)),-.52+(to[2]+.52)*v];}return id==='shrink-wrapper'?seat(id,ph,u):base;}
