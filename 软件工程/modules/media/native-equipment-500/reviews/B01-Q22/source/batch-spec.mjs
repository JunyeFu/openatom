export const batch='B01-Q22',profile='simulation-efficient-v1',workY=.9,stagingX=.95;
export const clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a)),samples=[0,.10,.20,.30,.40,.50,.55,.60,.70,.75,.80,.90,1];
const spec=(title,kind,seconds)=>({title,kind,initial:'carton_ready',final:'carton_ready',cycle_seconds:seconds,rule:'preserve',auxiliary:null,input_quantity:1,component_quantity:1,aux_quantity:1,extra_outputs:[],acceptance_conditions:[],mechanisms:[kind,'named-independent-joints','tool-attached-held-item'],flow:{input:'1件240×180×120mm已放行箱件',output:'同一箱件，身份/数量/质量保持'}});
export const specs={
 'six-axis-robot':spec('六轴机器人（夹持式箱件取放）','six',20),
 'scara-robot':spec('SCARA机器人（平面关节与升降取放）','scara',16),
 'gantry-pick-place':spec('龙门搬运机（三轴夹爪取放）','gantry',18),
 'vacuum-palletizer':spec('真空码垛机（单箱放入固定托盘槽位）','vacuum',22)
};
export const inputPort=()=>[-.65,.9,.25],outputPort=id=>[.95,specs[id].kind==='vacuum'?.14:.9,.25],pathEnd=outputPort,heading=()=>0;
export function boxPose(id,ph,u){const a=inputPort(id),b=outputPort(id);
 if(ph==='load')return[a[0],a[1]+.22*ramp(u,.75,1),a[2]];
 if(ph==='auto')return[a[0]+(b[0]-a[0])*u,1.12+.08*Math.sin(Math.PI*u),.25+.15*Math.sin(Math.PI*u)];
 return[b[0],1.12+(b[1]-1.12)*ramp(u,0,.55),b[2]];
}
export function toolPose(id,ph,u){const p=boxPose(id,ph,u);p[1]+=.18;if(ph==='load')p[1]+=.24*(1-ramp(u,0,.55));if(ph==='unload')p[1]+=.22*ramp(u,.75,1);return p;}
export const gripping=(ph,u)=>ph==='load'?ramp(u,.55,.75):ph==='auto'?1:1-ramp(u,.55,.75);
export const feet=id=>specs[id].kind==='six'||specs[id].kind==='scara'?[[-.05,-.35],[.35,-.35],[-.05,.05],[.35,.05]]:[[-1.35,-.65],[-1.35,.65],[1.35,-.65],[1.35,.65]];
export const auxPort=()=>[-1,.9,-.52],auxSeat=()=>[.9,.9,-.52],componentPort=()=>[-1,.9,.52],componentSeat=()=>[0,.9,0],componentState=()=>null,wasteState=()=>null,wastePort=()=>[.2,.55,-.94];
