export const specs={'rotary-scorer':{mode:'score',size:[.50,.003,.40],surface:.92,inputX:-1.60,outputX:1.60,initial:'flat',final:'longitudinally_scored',auxiliary:null,extra_outputs:[],score_centers:[-.10,.10],score_width:.006,score_depth:.0006}};
export const samples=[...new Set([...Array.from({length:513},(_,i)=>i/512),.375,.5,.625])].sort((a,b)=>a-b),feedTravel=2;
export const seatX=(s,phase,u)=>phase==='load'?s.inputX+(-1-s.inputX)*u:phase==='auto'?-1+2*u:1+(s.outputX-1)*u;
export const inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0];
export const scoredFraction=u=>Math.max(0,Math.min(1,((-1+2*u)+.25)/.50));
