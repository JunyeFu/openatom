export const specs={'film-laminator':{mode:'laminate',size:[.50,.0008,.40],surface:.92,inputX:-1.60,outputX:1.60,initial:'unlaminated_board',final:'dry_BOPP_laminated',auxiliary:'precoated_BOPP_tail',volume:.000005,extra_outputs:[]}};
export const clamp=v=>Math.max(0,Math.min(1,v));
export const film={length:.50,width:.40,thickness:.000025,core:.038,cells:512};
export const hotRadius=.080,hotY=.92+.0008+film.thickness+hotRadius,pathRadius=hotRadius+film.thickness/2,spoolY=1.15;
export const arcLength=pathRadius*Math.PI/2,initialPath=spoolY-hotY+arcLength,initialCoil=film.length-initialPath;
export const coilRadius=c=>Math.sqrt((film.core+film.thickness/2)**2+film.thickness*Math.max(0,c)/Math.PI);
export const spoolX=-pathRadius-coilRadius(initialCoil),auxPort=s=>[spoolX,spoolY,0],auxSeat=auxPort;
export const seatX=(s,phase,u)=>phase==='load'?s.inputX+(-1-s.inputX)*u:phase==='auto'?-1+2*u:1+(s.outputX-1)*u;
export const fedLength=u=>film.length*clamp(((-1+2*u)+.25)/.50);
export function pathState(u){
 const fed=fedLength(u),remaining=film.length-fed,dy=spoolY-hotY;
 let lo=0,hi=Math.max(0,remaining-arcLength-dy);
 for(let n=0;n<40;n++){const c=(lo+hi)/2,dx=spoolX+coilRadius(c)+pathRadius,l=Math.hypot(dx,dy);if(c+arcLength+l>remaining)hi=c;else lo=c;}
 const coil=remaining>=initialPath?(lo+hi)/2:0,r=coilRadius(coil),dx=spoolX+r+pathRadius,lineLength=Math.hypot(dx,dy);
 return{fed,remaining,coil,r,lineLength,pathLength:arcLength+lineLength,dx,dy};
}
// Material coordinate d counts upstream from the nip. Centerline boxes approximate bending at sub-mm spacing.
export function filmPoint(d,st){
 if(d<=arcLength){const a=-Math.PI/2-d/pathRadius;return{x:pathRadius*Math.cos(a),y:hotY+pathRadius*Math.sin(a),yaw:Math.atan2(Math.cos(a),-Math.sin(a)),section:'heated_wrap'};}
 if(d<=st.pathLength){const f=(d-arcLength)/st.lineLength;return{x:-pathRadius+st.dx*f,y:hotY+st.dy*f,yaw:Math.atan2(-st.dy,-st.dx),section:'free_web'};}
 const v=Math.max(0,Math.min(st.coil,d-st.pathLength)),r=Math.sqrt(Math.max((film.core+film.thickness/2)**2,st.r**2-film.thickness*v/Math.PI)),a=2*Math.PI*(st.r-r)/film.thickness,dr=-film.thickness/(2*Math.PI*r),da=1/r;
 const dx=dr*Math.cos(a)-r*Math.sin(a)*da,dy=dr*Math.sin(a)+r*Math.cos(a)*da;
 return{x:spoolX+r*Math.cos(a),y:spoolY+r*Math.sin(a),yaw:Math.atan2(-dy,-dx),section:'remaining_coil'};
}
export const unwindAngle=u=>-2*Math.PI*(coilRadius(initialCoil)-pathState(u).r)/film.thickness;
export const feedTravel=2,inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0];
export const samples=[...new Set([0,1,.375,.625,...Array.from({length:257},(_,i)=>i/256),...Array.from({length:513},(_,i)=>.375+.25*i/512)])].sort((a,b)=>a-b);
