const glyphs={A:['01110','10001','10001','11111','10001','10001','10001'],'0':['01110','10001','10011','10101','11001','10001','01110'],'1':['00100','01100','00100','00100','00100','00100','01110']};
export const pixelPitch=.002,pixelSize=.0015,printThickness=.000005;
export const pixels=[...'A01'].flatMap((char,letter)=>glyphs[char].flatMap((row,r)=>[...row].flatMap((v,col)=>v==='1'?[{x:(letter*6+col-8)*pixelPitch,z:(r-3)*pixelPitch}]:[])));
export const inkVolume=pixels.length*pixelSize*pixelSize*printThickness;
export const specs={
 'inkjet-marker':{mode:'code',size:[.50,.0008,.40],surface:.92,inputX:-1.60,outputX:1.60,initial:'flat',final:'coded_A01',auxiliary:'ink',volume:inkVolume,extra_outputs:[]},
 'varnish-coater':{mode:'coat',size:[.50,.0008,.40],surface:.92,inputX:-1.60,outputX:1.60,initial:'flat',final:'varnished_wet',auxiliary:'varnish',volume:.000004,extra_outputs:[]},
 'uv-curer':{mode:'cure',size:[.50,.0008,.40],surface:.92,inputX:-1.60,outputX:1.60,initial:'varnished_wet',final:'varnished_cured',auxiliary:null,volume:.000004,extra_outputs:[]},
};
export const feedTravel=2,samples=[...new Set([...Array.from({length:513},(_,i)=>i/512),...pixels.map(p=>(1-p.x)/2),.2,.375,.5,.625,.8])].sort((a,b)=>a-b);
export const clamp=v=>Math.max(0,Math.min(1,v));
export const seatX=(s,phase,u)=>phase==='load'?s.inputX+(-1-s.inputX)*u:phase==='auto'?-1+2*u:1+(s.outputX-1)*u;
export const inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0];
export const auxPort=s=>[-1.60,.908,-.75],auxSeat=s=>s.mode==='code'?[.45,1.02,-.38]:[-.25,1.12,-.42];
export const coatingFraction=u=>clamp(((-1+2*u)+.25)/.50);
export const printedPixelCount=u=>pixels.filter(p=>-1+2*u+p.x>=-1e-9).length;
