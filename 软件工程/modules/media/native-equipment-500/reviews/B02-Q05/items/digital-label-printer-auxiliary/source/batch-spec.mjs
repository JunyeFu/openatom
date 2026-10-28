export const batch='B02-Q05',profile='simulation-efficient-v1',clamp=x=>Math.max(0,Math.min(1,x)),ramp=(u,a,b)=>clamp((u-a)/(b-a));
export const specs={
 'digital-label-printer':{title:'卷筒标签数码印刷机（六标定长段）',cycle_seconds:24,rule:'transform',initial:'plain_web',final:'printed_web',auxiliary:'metered_black_ink_003ml',extra_outputs:[],mechanisms:['nip-feed-and-inkjet-bar','six-pattern-printing'],flow:{input:'600×80mm未印标签复合段+0.03mL计量墨',output:'1段有六个SF-DEMO-01图案的新印刷复合段'}},
 'label-die-cutter':{title:'标签圆刀模切机（六标半切、保留矩阵）',cycle_seconds:24,rule:'transform',initial:'printed_web',final:'kiss_cut_web',auxiliary:null,extra_outputs:[],mechanisms:['registered-rotary-kiss-cut-cylinder','five-repeat-die-and-anvil'],flow:{input:'1段六标印刷复合段',output:'1段六标半切复合段，底纸/矩阵保留'}},
 'label-peeler':{title:'标签剥离机（六标与底纸矩阵分离）',cycle_seconds:20,rule:'transform',initial:'kiss_cut_web',final:'peeled_label',auxiliary:null,extra_outputs:[{role:'backing'}],acceptance_conditions:['backing-wait'],mechanisms:['supported-peel-nose-and-return-path','six-original-patterns-to-six-new-labels'],flow:{input:'1段含六标的半切复合段',output:'6件新标签+1段带废矩阵的底纸，保留来源'}},
 'label-web-inspector':{title:'标签复卷检验机（六标段检测与分流）',cycle_seconds:24,rule:'inspect',initial:'kiss_cut_web',final:'kiss_cut_web',auxiliary:null,extra_outputs:[{role:'isolation'}],acceptance_conditions:['rejected','pending-recovery','rework','reject-wait'],mechanisms:['line-camera-and-supported-light-bar','pass-recheck-isolation-destinations'],flow:{input:'1段六标半切复合段',output:'同一复合段按实际质量判定通过、复核或隔离'}}
};
export const web={L:.6,W:.08,pitch:.1,labelL:.08,labelW:.06,linerT:.00008,faceT:.00010,noseR:.025,code:'SF-DEMO-01'},workY=.9,stagingX=.76;
export const inputPort=()=>[-1,workY,0],outputPort=()=>[1.32,workY,0],outputY=()=>workY,auxPort=()=>[-1,workY,-.52],auxSeat=()=>[.9,workY,-.52],backingPort=()=>[-.35,workY-2*web.noseR-web.linerT-web.faceT,-.52],backingSeat=()=>[-.35,workY-2*web.noseR-web.linerT-web.faceT,0],rejectedPort=()=>[0,workY,1.0],isolationPort=()=>[0,workY,-1.0];
export const labelXs=Array.from({length:6},(_,i)=>-.25+i*.1),labelSeats=()=>[-.055,.055].flatMap(x=>[-.07,0,.07].map(z=>[x,0,z]));
const marks=labelXs.flatMap(x=>[(.3-x-.04)/.6,(.3-x+.04)/.6]);
export const samples=[...new Set([0,1,...Array.from({length:17},(_,i)=>i/16),.08,.92,...marks].map(Math.fround))].sort((a,b)=>a-b);
export const printedFraction=(i,u)=>ramp(u,(.3-labelXs[i]-.04)/.6,(.3-labelXs[i]+.04)/.6);
export function seat(id,phase,u){if(phase==='load')return[-1+.7*u,workY,0];if(phase==='auto')return[id==='label-peeler'?0:-.3+.6*u,workY,0];return[id==='label-peeler'?0:.3+(stagingX-.3)*u,workY,0];}
export function peelPath(s,u){const q=web.L*u-s,R=web.noseR;if(q<=0)return{p:[q,0,0],angle:0};if(q<Math.PI*R){const a=q/R;return{p:[R*Math.sin(a),-R*(1-Math.cos(a)),0],angle:-a};}return{p:[-(q-Math.PI*R),-2*R,0],angle:-Math.PI};}
export function labelPose(i,phase,u){if(phase==='load')return[labelXs[i],0,0];if(phase==='auto')return[-.3+web.L*u+labelXs[i],0,0];const a=.3+labelXs[i],b=labelSeats()[i];return[a+(stagingX+b[0]-a)*u,0,b[2]*u];}
