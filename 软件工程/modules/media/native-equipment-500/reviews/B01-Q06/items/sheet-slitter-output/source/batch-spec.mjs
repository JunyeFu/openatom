export const specs={'sheet-slitter':{mode:'slit',size:[.50,.003,.40],output_size:[.50,.003,.20],surface:.92,inputX:-1.60,outputX:1.60,stagingX:1.05,initial:'wide_sheet',final:'slit_half',auxiliary:null,extra_outputs:[],output_quantity:2,area:.20,output_area:.10}};
export const samples=[...new Set([...Array.from({length:513},(_,i)=>i/512),.375,.5,.625,.8])].sort((a,b)=>a-b),feedTravel=2;
export const seatX=(s,phase,u)=>phase==='load'?s.inputX+(-1-s.inputX)*u:phase==='auto'?-1+2*u:1+(s.stagingX-1)*u;
export const inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0];
export const separation=u=>Math.max(0,Math.min(1,(u-.625)/.175))*.010;
export const outputOffset=index=>[0,0,index===0?-.105:.105];
