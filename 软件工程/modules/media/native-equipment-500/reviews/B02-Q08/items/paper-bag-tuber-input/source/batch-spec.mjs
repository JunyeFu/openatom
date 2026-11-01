export const batch='B02-Q08',profile='simulation-efficient-v1';
export const clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const workY=.9,stagingX=.76,samples=[0,.12,.22,.34,.46,.58,.70,.80,.86,.94,1];
const spec=(title,initial,final,glue,kind,dimensions,flow)=>({title,initial,final,auxiliary:glue?'metered_glue_'+String(glue).replace('.','p')+'ml':null,glue_ml:glue,rule:'transform',cycle_seconds:20,input_quantity:1,kind,...dimensions,flow,extra_outputs:[],acceptance_conditions:[],mechanisms:[kind,'supported-folding-fixture-and-controlled-press']});
export const specs={
 'paper-bag-tuber':spec('纸袋筒成型机（定长纸幅放卷、围模糊缝）','metered_rolled_paper_web','open_rectangular_bag_tube',1,'wrap',{L:.30,W:.20,H:.10,t:.0006,lap:.010},{input:'1段610×300mm计量卷纸幅+1mL胶',output:'1个300×200×100mm两端开口袋筒'}),
 'paper-bag-bottomer':spec('纸袋糊底机（折平袋筒折底压合）','open_flat_bag_tube','pasted_fold_bottom_bag',1,'bottom',{L:.30,W:.20,t:.0006,flap:.035},{input:'1个300×200mm折平袋筒+1mL胶',output:'1个265×200mm封底纸袋，另一端留口'}),
 'envelope-folder':spec('信封折糊机（三边糊合，封舌留开）','precut_envelope_blank','three_edge_envelope_open_flap',.5,'envelope',{L:.22,W:.14,t:.0003,side:.030,lid:.045},{input:'1张预切信封坯+0.5mL胶',output:'1个220×140mm三边糊合信封，45mm封舌留开'}),
 'mailing-bag-sealer':spec('快递纸袋封边机（覆膜纸坯三边热封）','pre_folded_heat_sealable_paper_blank','three_edge_heat_sealed_mailer',0,'heatseal',{L:.32,W:.22,t:.0005},{input:'1份预折合、已覆热封层的纸袋坯',output:'1个320×220mm三边热封袋，收件口留开'}),
 'sleeve-former':spec('纸滑套折糊机（预切坯围模搭接）','precut_sleeve_blank','open_rectangular_sleeve',.7,'wrap',{L:.16,W:.22,H:.12,t:.0006,lap:.010},{input:'1张690×160mm预切滑套坯+0.7mL胶',output:'1个160×220×120mm两端开口滑套'}),
 'box-lid-former':spec('纸盒天地盖成型机（单盖坯折合压角）','precut_scored_lid_blank','open_formed_box_lid',.8,'lid',{L:.20,W:.15,H:.035,t:.0007},{input:'1张预开槽/压痕、带角耳的盖坯+0.8mL胶',output:'1个200×150×35mm单件天地盖，盒口留开'})
};
export const inputPort=()=>[-1,workY,0],outputPort=()=>[1.32,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.90,workY,-.52],componentPort=()=>[-1,workY,.52],wastePort=()=>[.20,.55,-.94];
export const seat=(id,ph,u)=>[ph==='load'?-1+(id==='paper-bag-tuber'?ramp(u,.40,1):u):ph==='unload'?stagingX*u:0,workY,0];
export const forming=(ph,u)=>ph==='load'?0:ph==='unload'?1:ramp(u,.22,.70);
export const tooling=(ph,u)=>ph==='auto'?ramp(u,.22,.70)*(1-ramp(u,.86,1)):0;
export const mandrelX=(ph,u)=>ph==='auto'?-.55*(1-ramp(u,0,.12)+ramp(u,.80,.94)):-.55;
