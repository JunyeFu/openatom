export const batch='B01-Q18',profile='simulation-efficient-v1',clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const specs={
 'vision-carton-inspector':{title:'纸箱视觉检验机（合格、返工与隔离）',cycle_seconds:12,rule:'inspect',initial:'sealed_pack',final:'sealed_pack',auxiliary:null,extra_outputs:[{role:'isolation'}],acceptance_conditions:['rejected','pending-recovery','rework','reject-wait'],mechanisms:['top-and-side-optical-inspection','manual-positioning-and-sliding-access-doors','three-result-destinations'],flow:{input:'1件待检包件，含6个真实内装物',output:'同一包件按合格、返工、隔离分流，待判定保持占用'}},
 'tape-case-sealer':{title:'胶带封箱机（折盖与预切胶带绕贴）',cycle_seconds:20,rule:'preserve',initial:'open_top_pack',final:'taped_top_pack',auxiliary:'metered_top_tape_440mm',extra_outputs:[],mechanisms:['minor-major-top-flaps','guided-folder-lifts','continuous-U-path-tape-roller'],flow:{input:'1件开口包件 + 1段440mm预切封口胶带',output:'保持包件ID及6个内装物的封口包件'}},
 'label-applicator':{title:'纸箱贴标机（真空移贴单枚标签）',cycle_seconds:10,rule:'preserve',initial:'sealed_unlabelled_pack',final:'sealed_labelled_pack',auxiliary:'ready_peeled_shipping_label',extra_outputs:[],mechanisms:['XYZ-label-pick','quarter-turn-vacuum-wrist','carton-face-label-press'],flow:{input:'1件已封包件 + 1枚已剥离自粘标签',output:'保持包件ID及6个内装物的已贴标包件'}}
};
export const box={L:.32,W:.24,H:.24,t:.003,flap:.119},label={L:.10,H:.06,t:.0001,code:'SF-DEMO-01'},unit={L:.08,W:.06,H:.12};
export const workY=.90,inputX=-1,outputX=1.32,stagingX=.76,inputPort=()=>[inputX,workY,0],outputPort=()=>[outputX,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.90,workY,-.52],rejectedPort=()=>[0,workY,1.00],isolationPort=()=>[0,workY,-1.00];
const boundaries=[.06,.08,.10,.12,.15,.20,.22,.24,.25,.30,.35,.40,.42,.44,.45,.5,.52,.55,.57,.60,.62,.65,.70,.72,.74,.80,.85,.875,.90,.92,.94,.95,.98];
export const samples=[...new Set([0,1,...Array.from({length:33},(_,i)=>i/32),...boundaries])].sort((a,b)=>a-b);
export const minorFold=u=>Math.PI/2*ramp(u,.08,.22),majorFold=u=>Math.PI/2*ramp(u,.24,.42);
export function tapeMotion(u){const r=.0221,Y=workY+box.H+box.t/2,leg=.06,arc=Math.PI*r/2,total=2*leg+box.L+2*arc,v=ramp(u,.62,.94)*total;let p,angle,fraction;
 if(v<leg){p=[-box.L/2-r,Y-leg+v,0];angle=Math.PI/2;fraction=v/.44;}
 else if(v<leg+arc){const a=(v-leg)/r;p=[-box.L/2-r*Math.cos(a),Y+r*Math.sin(a),0];angle=Math.PI/2-a;fraction=leg/.44;}
 else if(v<leg+arc+box.L){const x=v-leg-arc;p=[-box.L/2+x,Y+r,0];angle=0;fraction=(leg+x)/.44;}
 else if(v<leg+2*arc+box.L){const a=(v-leg-arc-box.L)/r;p=[box.L/2+r*Math.sin(a),Y+r*Math.cos(a),0];angle=-a;fraction=(leg+box.L)/.44;}
 else{const x=v-leg-2*arc-box.L;p=[box.L/2+r,Y-x,0];angle=-Math.PI/2;fraction=(leg+box.L+x)/.44;}
 if(u<.62){const a=ramp(u,.52,.57),b=ramp(u,.57,.62);p=[.50+(-box.L/2-r-.50)*a,1.42+(Y-leg-1.42)*b,0];angle=Math.PI/2*a;}
 if(u>.94){const a=ramp(u,.94,1);p=[p[0]+.20*a,p[1]+.44*a,0];}
 return{p,angle,fraction};}
export const tapeFraction=u=>tapeMotion(u).fraction;
export function seat(id,phase,u){if(phase==='load')return[inputX*(1-ramp(u,0,.5)),workY+.18*(ramp(u,0,.075)-ramp(u,.425,.5)),0];return[id==='vision-carton-inspector'?0:phase==='unload'?outputX*u:0,workY,0];}
export function labelPose(phase,u){if(phase==='load'){const v=ramp(u,.5,1),a=auxPort(),b=auxSeat();return{p:[a[0]+(b[0]-a[0])*v,workY+.15*(ramp(v,0,.15)-ramp(v,.85,1)),a[2]],angle:0};}if(phase==='unload')return{p:[outputX*u,workY+.14,box.W/2],angle:Math.PI/2};
 const angle=Math.PI/2*ramp(u,.30,.45);let p;if(u<.10)p=[.9,workY+.40*ramp(u,0,.1),-.52];else if(u<.30){const v=ramp(u,.1,.3);p=[.9*(1-v),1.30,-.52+.77*v];}else if(u<.45)p=[0,1.30,.25];else if(u<.60)p=[0,1.30-.26*ramp(u,.45,.60),.25];else p=[0,1.04,.25-(.25-box.W/2)*ramp(u,.60,.72)];return{p,angle};}
export function labelHead(phase,u){const a=labelPose(phase,u);if(phase==='load'){const v=ramp(u,0,.5),p=auxSeat();return{p:[p[0]*v,1.30+(p[1]-1.30)*v,.30+(p[2]-.30)*v],angle:Math.PI/2*(1-v)};}if(phase==='unload')return{p:[0,1.30,.30],angle:Math.PI/2};if(u>.80){const v=ramp(u,.8,.9),w=ramp(u,.9,1);return{p:[0,1.04+.26*w,box.W/2+.18*v],angle:Math.PI/2};}return a;}
export const contentSeats=()=>[-.07,.07].flatMap(x=>[-.07,0,.07].map(z=>[x,box.t,z]));
