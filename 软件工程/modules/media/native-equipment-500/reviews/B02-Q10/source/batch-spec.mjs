export const batch='B02-Q10',profile='simulation-efficient-v1',workY=.9,stagingX=.76;
export const clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const samples=[0,.15,.30,.4499,.45,.65,.80,1];
const spec=(title,kind,initial,final,seconds,rb,rt,H)=>({title,kind,initial,final,cycle_seconds:seconds,rule:kind==='plate'?'transform':'assemble',auxiliary:null,input_quantity:1,component_quantity:1,aux_quantity:1,extra_outputs:[],acceptance_conditions:kind==='plate'?[]:['missing-component'],mechanisms:['supported-forming-mandrel','heated-closing-jaw','transfer-nest'],rb,rt,H,t:kind==='plate'?.0005:.0004,flow:{input:kind==='plate'?'1张圆纸坯':'1张预涂热封层侧壁坯+1片预涂层底片',output:kind==='plate'?'1只压制纸盘':'1只开口热封纸容器'}});
export const specs={
 'paper-cup-former':spec('纸杯成型机（侧壁与底片热封）','cup','coated_cup_sidewall_blank','heat_sealed_paper_cup',16,.030,.045,.100),
 'paper-bowl-former':spec('纸碗成型机（侧壁与底片热封）','bowl','coated_bowl_sidewall_blank','heat_sealed_paper_bowl',18,.050,.070,.060),
 'paper-plate-former':spec('纸盘热压成型机（圆坯压制）','plate','round_paper_plate_blank','pressed_paper_plate',12,.080,.110,.018)
};
export const inputPort=()=>[-1,workY,0],outputPort=()=>[1.32,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.90,workY,-.52],componentPort=()=>[-1,workY,.52],componentSeat=()=>[0,workY,0],wasteSeat=()=>[.2,.55,-.5],wastePort=()=>[.2,.55,-.94];
export const seat=(id,ph,u)=>[ph==='load'?-1+u:ph==='unload'?stagingX*u:0,workY,0];
export const formed=(ph,u)=>ph==='unload'||ph==='auto'&&u>=.45;
export const componentState='coated_bottom_disc',wasteState=()=>null;
