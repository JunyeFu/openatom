// Original illustrative process recipes. Geometry never owns stock or a clock.
export const batch='B01-Q16',profile='simulation-efficient-v1';
export const clamp=x=>Math.max(0,Math.min(1,x));
export const ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const specs={
 'carton-erector':{title:'纸箱开箱机（RSC折底封底）',cycle_seconds:22,rule:'transform',initial:'flat_glued_RSC_tube',final:'erected_bottom_taped_RSC',auxiliary:'pre_cut_bottom_tape',mechanisms:['opposed-vacuum-opening','minor-major-bottom-fold','bottom-tape-transfer'],flow:{input:'1件预粘RSC平折箱坯 + 1段预切封底胶带',output:'1件四壁撑正、四底翼折合并封底的开口箱'},tape_length_m:.48},
 'adhesive-dispenser':{title:'热熔胶涂布机（箱坯侧缝胶线）',cycle_seconds:15,rule:'transform',initial:'released_side_seam_blank',final:'hot_melt_beaded_blank',auxiliary:'ready_metered_hot_melt',mechanisms:['blank-vacuum-deck','indexed-XYZ-glue-head','heated-service-bundle'],flow:{input:'1件纸板箱坯 + 1份0.300mL已可施用热熔胶',output:'1件带320mm侧缝胶线的箱坯'},volume:.0000003},
 'compression-stack':{title:'纸箱压合机（折叠箱坯侧缝压合）',cycle_seconds:30,rule:'preserve',initial:'fresh_glued_flat_case',final:'pressed_flat_case',auxiliary:null,mechanisms:['flat-case-conveyor','guided-compression-platen','regulated-pressure-hold'],flow:{input:'1件侧缝待压合的平折纸箱',output:'保持原身份的1件已压合平折纸箱'}}
};
export const carton={length:.36,width:.24,height:.30,thickness:.003,base:.119,flapDepth:.119,theta0:.04};
export const samples=[...new Set([0,1,...Array.from({length:33},(_,i)=>i/32),.12,.32,.40,.52,.60,.72,.76,.78,.80,.90,.92])].sort((a,b)=>a-b);
export const inputX=-1.00,stagingX=.76,outputX=1.32,workY=.90;
export const outputY=id=>id==='carton-erector'?workY+carton.base-carton.thickness/2:workY;
export const inputPort=()=>[inputX,workY,0],outputPort=id=>[outputX,outputY(id),0],auxPort=()=>[-1.00,workY,-.52],auxSeat=()=>[.90,workY,-.52];
export function seat(phase,u){
 if(phase==='load')return[inputX*(1-ramp(u,0,1)),workY+.18*(ramp(u,0,.15)-ramp(u,.85,1)),0];
 if(phase==='unload')return[stagingX*ramp(u,0,1),workY,0];
 return[0,workY,0];
}
export const opening=u=>carton.theta0+(Math.PI/2-carton.theta0)*ramp(u,.12,.32);
export const minorFold=u=>Math.PI/2*ramp(u,.40,.52),majorFold=u=>Math.PI/2*ramp(u,.60,.72);
export const tapeFraction=u=>ramp(u,.78,.92);
export const glueFraction=u=>ramp(u,.20,.76);
export const pressFraction=u=>ramp(u,.12,.25)-ramp(u,.80,.92);
export const blank={length:.48,width:.32,thickness:.0012,beadLength:.32,beadWidth:.003,beadHeight:.0003125,beadZ:.135};
export const flatCase={length:.44,width:.32,thickness:.0062};
