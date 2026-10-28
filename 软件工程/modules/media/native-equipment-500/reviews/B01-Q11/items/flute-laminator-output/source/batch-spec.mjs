export const specs={'flute-laminator':{mode:'flute_laminate',size:[.50,.004,.40],surface:.92,inputX:-1.60,outputX:1.60,stagingX:1.25,initial:'single_face_corrugated',final:'wet_laminated_board',auxiliary:'water_based_adhesive_dose',volume:.000004,extra_outputs:[]}};
export const clamp=v=>Math.max(0,Math.min(1,v)),pitch=.010,mediumThickness=.0002,bottomLiner=.0005,crestTop=.004,glueHeight=.0002,faceThickness=.0005,bandWidth=.001,faceCells=256,nipX=.20,glueX=-.40;
export const crestCenters=Array.from({length:50},(_,i)=>-.245+i*pitch),theta=12*Math.PI/180,nipRadius=.080,facePathRadius=nipRadius+faceThickness/2,faceFlatY=.92+crestTop+glueHeight+faceThickness/2;
export const seatX=(s,phase,u)=>phase==='load'?s.inputX+(-1-s.inputX)*u:phase==='auto'?-1+2*u:1+(s.stagingX-1)*u;
export function facePath(q){
 if(q>=0)return{x:nipX+q,y:faceFlatY,angle:0};
 if(q>=-facePathRadius*theta)return{x:nipX+facePathRadius*Math.sin(q/facePathRadius),y:faceFlatY+facePathRadius*(1-Math.cos(q/facePathRadius)),angle:q/facePathRadius};
 const d=q+facePathRadius*theta;return{x:nipX-facePathRadius*Math.sin(theta)+d*Math.cos(theta),y:faceFlatY+facePathRadius*(1-Math.cos(theta))-d*Math.sin(theta),angle:-theta};
}
export const faceDatum=(s,phase,u)=>{const p=facePath(seatX(s,phase,u)-nipX);return[p.x,p.y,0];};
export const inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0],componentPort=s=>faceDatum(s,'load',0),auxPort=s=>[-1.60,.908,-.75],auxSeat=s=>[-.475,1.12,-.42];
export const coveredBand=(i,s,phase,u)=>bandWidth*clamp((seatX(s,phase,u)+crestCenters[i]+bandWidth/2-glueX)/bandWidth);
export const appliedVolume=(s,phase,u)=>crestCenters.reduce((a,_,i)=>a+coveredBand(i,s,phase,u)*glueHeight*.4,0);
export const doseRadius=.012,doseSegments=32,doseHeight=.000004/(doseSegments/2*doseRadius**2*Math.sin(2*Math.PI/doseSegments));
// Flat crest/trough lands join smooth cosine flanks; normal offsets give a real0.2mm medium wall.
export function waveAt(x){
 let phase=((x+.245)%pitch+pitch)%pitch;const sign=phase<=pitch/2?1:-1,d=Math.min(phase,pitch-phase),land=.0006,span=pitch/2-2*land;
 if(d<=land)return{y:.0039,dy:0};if(d>=pitch/2-land)return{y:.0006,dy:0};const a=Math.PI*(d-land)/span;return{y:.00225+.00165*Math.cos(a),dy:-.00165*Math.PI/span*Math.sin(a)*sign};
}
export const waveSamples=[...new Set([-.25,.25,...Array.from({length:3201},(_,i)=>-.25+.5*i/3200),...crestCenters.flatMap(c=>[c-.0006,c-.0005,c,c+.0005,c+.0006])])].sort((a,b)=>a-b);
export const samples=[...new Set([0,1,.175,.425,.475,.725,...Array.from({length:1025},(_,i)=>i/1024),...crestCenters.flatMap(c=>[-1,1].map(sign=>(glueX-c+sign*bandWidth/2+1)/2)),...Array.from({length:faceCells},(_,i)=>[0,-facePathRadius*theta].map(q=>((i+.5)*.5/faceCells-.25+nipX+q+1)/2)).flat()])].filter(u=>u>=0&&u<=1).sort((a,b)=>a-b);
export const feedTravel=2;
