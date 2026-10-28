export const batch='B01-Q21',profile='simulation-efficient-v1',workY=.9,stagingX=.75;
export const clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a)),samples=[0,.15,.30,.45,.60,.80,1];
const spec=(title,kind,seconds)=>({title,kind,initial:'carton_ready',final:'carton_ready',cycle_seconds:seconds,rule:'preserve',auxiliary:null,input_quantity:1,component_quantity:1,aux_quantity:1,extra_outputs:[],acceptance_conditions:[],mechanisms:[kind,'supported-drive-and-guide','identity-preserving-transport'],flow:{input:'1件240×180×120mm箱件',output:'同一箱件，身份/数量/质量不变'}});
export const specs={
 'belt-conveyor':spec('皮带输送机（箱件直行）','belt',12),
 'roller-conveyor':spec('辊筒输送机（箱件直行）','roller',12),
 'curved-conveyor':spec('曲线输送机（90度随行转向）','curve',16),
 'right-angle-transfer':spec('直角移载机（顶升横移，朝向保持）','transfer',18),
 'lift-conveyor':spec('升降输送机（低位进料，高位出料）','lift',20)
};
export const feet=id=>id==='curved-conveyor'?[[-.9,-.27,.9],[-.9,.27,.9],[-.27,1.30,.9],[.27,1.30,.9],[-.27,.60,.9],[.27,.60,.9]]:id==='right-angle-transfer'?[[-.90,-.30,.9],[-.90,.30,.9],[.30,-.30,.9],[-.30,1.30,.94],[.30,1.30,.94]]:[[-.90,-.30,id==='lift-conveyor'?.45:.9],[-.90,.30,id==='lift-conveyor'?.45:.9],[1.25,-.30,.9],[1.25,.30,.9]];
export const inputPort=id=>[-.9,id==='lift-conveyor'?.45:.9,0],outputPort=id=>['curved-conveyor','right-angle-transfer'].includes(id)?[0,id==='right-angle-transfer'?.94:.9,1.20]:[1.20,.9,0];
export const pathEnd=id=>['curved-conveyor','right-angle-transfer'].includes(id)?[0,id==='right-angle-transfer'?.94:.9,.75]:[.75,.9,0];
export function seat(id,ph,u){const kind=specs[id].kind;
 if(kind==='curve'){if(ph==='load')return[-.9+.3*u,.9,0];if(ph==='unload')return[0,.9,.60+.15*u];const t=-Math.PI/2+Math.PI/2*u;return[-.6+.6*Math.cos(t),.9,.6+.6*Math.sin(t)];}
 if(kind==='transfer'){if(ph==='load')return[-.9+.9*u,.9,0];if(ph==='unload')return[0,.94,.60+.15*u];return[0,.9+.04*ramp(u,.10,.30),.60*ramp(u,.35,1)];}
 if(kind==='lift'){if(ph==='load')return[-.9+.9*u,.45,0];if(ph==='auto')return[0,.45+.45*u,0];return[.75*u,.9,0];}
 return[ph==='load'?-.9+.45*u:ph==='auto'?-.45+.80*u:.35+.40*u,.9,0];
}
export const heading=(id,ph,u)=>specs[id].kind==='curve'?(ph==='load'?0:ph==='auto'?-Math.PI/2*u:-Math.PI/2):0;
export const auxPort=()=>[-1,.9,-.52],auxSeat=()=>[.9,.9,-.52],componentPort=()=>[-1,.9,.52],componentSeat=()=>[0,.9,0],componentState=()=>null,wasteState=()=>null,wastePort=()=>[.2,.55,-.94];
