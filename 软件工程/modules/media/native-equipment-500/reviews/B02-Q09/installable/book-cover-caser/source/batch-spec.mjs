export const batch='B02-Q09',profile='simulation-efficient-v1',workY=.9,stagingX=.76;
export const clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const samples=[0,.10,.20,.30,.40,.55,.68,.80,.86,1];
export const book={L:.21,W:.15,H:.015,caseL:.225,boardW:.158,boardT:.002,spine:.019,caseBlockZ:.078};
export const saddle={L:.21,W:.1485,t:.0012,apex:.1485/Math.SQRT2,staples:[-.055,.055]};
export const wire={L:.21,W:.148,H:.012,r:.010,cy:.0106,cz:-.074,pageLift:.0046,pitch:.015,holes:Array.from({length:13},(_,i)=>-.09+i*.015),holeR:.003,rodR:.0004,segments:12};
export const stack={L:.21,W:.148,H:.020,holeR:.003,holeZ:-.045,holes:[-.06,0,.06],cornerR:.010,sheets:200};
const spec=(title,kind,initial,final,seconds,rule,auxiliary,flow)=>({title,kind,initial,final,cycle_seconds:seconds,rule,auxiliary,flow,input_quantity:1,component_quantity:1,aux_quantity:1,extra_outputs:[],acceptance_conditions:[],mechanisms:[kind,'supported-clamp-or-forming-head']});
export const specs={
 'book-cover-caser':{...spec('书芯上壳机（书芯与书壳贴合）','case','uncased_book_block','cased_hardback_book',24,'assemble','metered_endpaper_glue_3ml',{input:'1件书芯+1件书壳+3mL胶',output:'1本225×158×19mm上壳图书'}),glue_ml:3,acceptance_conditions:['missing-component']},
 'spine-gluer':{...spec('书脊涂胶机（夹持书芯滚涂）','glue','bare_spine_book_block','spine_glued_book_block',18,'transform','metered_spine_glue_0p8ml',{input:'1件书芯+0.8mL胶',output:'1件涂胶书芯，胶面朝外卸出'}),glue_ml:.8},
 'saddle-stitcher':{...spec('骑马订书机（双钉订合折帖）','stitch','gathered_open_signature','two_staple_saddle_booklet',14,'transform','staple_wire_42mm_segments',{input:'1份12张嵌套折帖+2段42mm订丝',output:'1本双钉骑马订书本'}),aux_quantity:2},
 'wire-o-binder':{...spec('双线圈装订机（预冲孔页叠装环闭合）','wire','prepunched_page_block','closed_twin_wire_notebook',20,'assemble',null,{input:'1份13孔页叠+1件13组双环线圈',output:'1本闭合双线圈本'}),acceptance_conditions:['missing-component']},
 'paper-drilling-machine':{...spec('多头打孔机（三头空心钻）','drill','square_paper_stack','three_hole_paper_stack',18,'transform',null,{input:'1份200张纸叠',output:'1份三孔纸叠+1批纸芯及纸屑'}),extra_outputs:[{role:'waste'}],acceptance_conditions:['waste-wait']},
 'corner-rounding-machine':{...spec('纸品圆角机（四角同时修切）','round','square_paper_stack','four_corner_rounded_stack',16,'transform',null,{input:'1份200张纸叠',output:'1份四角R10纸叠+1批四角废料'}),extra_outputs:[{role:'waste'}],acceptance_conditions:['waste-wait']}
};
export const inputPort=()=>[-1,workY,0],outputPort=()=>[1.32,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.90,workY,-.52],componentPort=()=>[-1,workY,.52],componentSeat=()=>[0,workY,0],wasteSeat=()=>[.20,.55,-.50],wastePort=()=>[.20,.55,-.94];
export const seat=(id,ph,u)=>[ph==='load'?-1+(id==='book-cover-caser'?ramp(u,.60,.90):id==='wire-o-binder'?ramp(u,0,.50):u):ph==='unload'?stagingX*u:0,workY,0];
export const caseFold=(ph,u)=>ph==='load'?0:ph==='unload'?1:ramp(u,.30,.68);
export const cutDone=(ph,u)=>ph==='unload'||ph==='auto'&&u>=.55;
export const wasteState=id=>id==='paper-drilling-machine'?'collected_paper_cores_and_dust':'collected_four_corner_cuttings';
export const clampAmount=(ph,u)=>ph==='auto'?ramp(u,.02,.18)*(1-ramp(u,.85,1)):0;
