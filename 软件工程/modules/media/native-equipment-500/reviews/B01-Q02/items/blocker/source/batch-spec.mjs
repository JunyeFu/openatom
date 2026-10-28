export const specs={'flexo-printer':{mode:'flexo',size:[.50,.0008,.40],surface:.92,inputX:-1.60,outputX:1.60,initial:'flat',final:'two_color_printed',preserve:true,extra_outputs:[],colors:[{role:'auxiliary',name:'blue',color:'#214f83',x:-.40,z:-.08,volume:.0000002},{role:'auxiliary_red',name:'red',color:'#ad3940',x:.40,z:.08,volume:.0000002}]}};
export const feedStart=-1,feedEnd=1,feedTravel=2,clamp=v=>Math.max(0,Math.min(1,v));
export const samples=[...new Set([...Array.from({length:257},(_,i)=>i/256),.275,.30,.325,.675,.70,.725])].sort((a,b)=>a-b);
export function seatX(s,phase,u){return phase==='load'?s.inputX+(feedStart-s.inputX)*u:phase==='auto'?feedStart+feedTravel*u:feedEnd+(s.outputX-feedEnd)*u;}
export const inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0];
export const inkPort=(color)=>[color.name==='blue'?-1.60:-1.30,.908,-.75],inkSeat=color=>[color.x,1.18,-.48];
export const fraction=(color,u)=>clamp((feedStart+feedTravel*u+.05-color.x)/.10);
