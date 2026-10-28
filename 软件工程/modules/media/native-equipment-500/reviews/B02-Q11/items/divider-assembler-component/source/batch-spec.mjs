export const batch='B02-Q11',profile='simulation-efficient-v1',workY=.9,stagingX=.76;
export const clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a)),samples=[0,.10,.15,.25,.35,.45,.55,.65,.80,.90,1];
const spec=(title,kind,initial,final,seconds,rule='transform',auxiliary=null)=>({title,kind,initial,final,cycle_seconds:seconds,rule,auxiliary,input_quantity:1,component_quantity:1,aux_quantity:1,extra_outputs:[],acceptance_conditions:[],mechanisms:[kind,'supported-main-actuator','continuous-output-belt']});
export const specs={
 'bag-handle-attacher':{...spec('纸袋提手粘接机（双提手贴合）','handle','open_paper_bag','two_handle_paper_bag',18,'assemble','metered_handle_glue_2ml'),glue_ml:2,component_quantity:2,acceptance_conditions:['missing-component']},
 'rope-handle-twister':{...spec('纸绳捻制机（三纸条成绳）','rope','three_paper_strips','twisted_three_ply_paper_cord',16),input_quantity:3},
 'partition-slotter':{...spec('纸隔板开槽机（双槽清废）','slot','uncut_partition_strip','top_slotted_partition_strip',14),extra_outputs:[{role:'waste'}],acceptance_conditions:['waste-wait']},
 'divider-assembler':{...spec('纸隔板装配机（两纵两横插合）','divider','two_upper_slotted_dividers','interlocked_four_board_divider',20,'assemble'),input_quantity:2,component_quantity:2,acceptance_conditions:['missing-component']},
 'paper-waste-baler':{...spec('废纸打包压缩机（定量压缩与双带束紧）','baler','four_loose_paper_charges','strapped_paper_bale',24,'transform','two_bale_straps_700mm'),input_quantity:4,aux_quantity:2},
 'board-pile-palletizer':{...spec('纸板垛码垛机（整垛夹抱落托）','pallet','aligned_ten_board_stack','palletized_ten_board_stack',22,'assemble'),input_quantity:10,acceptance_conditions:['missing-component']}
};
export const componentState=id=>({'bag-handle-attacher':'two_prefabricated_rope_handles','divider-assembler':'two_lower_slotted_cross_dividers','board-pile-palletizer':'empty_board_pallet'})[id];
export const wasteState=()=> 'two_partition_slot_offcuts';
export const inputPort=()=>[-1,workY,0],outputPort=()=>[1.32,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.90,workY,-.52],componentPort=()=>[-1,workY,.52],componentSeat=()=>[0,workY,0],wasteSeat=()=>[.2,.55,-.5],wastePort=()=>[.2,.55,-.94];
export const seat=(id,ph,u)=>[ph==='load'?-1+(id==='board-pile-palletizer'?.55:1)*u:ph==='unload'?stagingX*u:id==='board-pile-palletizer'?-.45+.45*ramp(u,.25,.60):0,workY,0];
export const pileLift=(ph,u)=>ph==='load'?0:ph==='unload'?.07:.15*(ramp(u,.10,.25)-ramp(u,.60,.80))+.07*ramp(u,.60,.80);
export const done=(ph,u,at=.55)=>ph==='unload'||ph==='auto'&&u>=at;

const flows={handle:["1只纸袋+2件预制纸绳提手+2mL胶","1只双提手纸袋"],rope:["3条360mm原纸条","1条300mm三股纸绳"],slot:["1条300×80×2mm隔板坯","1条双槽隔板+1批两片槽料"],divider:["2条上开槽纵板+2条下开槽横板","1套四条井字隔板"],baler:["4份0.25kg松散废纸+2条700mm打包带","1件1kg双带压缩纸包"],pallet:["10张300×220×4mm纸板+1只托盘","1件含10张纸板的托盘化整垛"]};
for(const s of Object.values(specs))s.flow={input:flows[s.kind][0],output:flows[s.kind][1]};
