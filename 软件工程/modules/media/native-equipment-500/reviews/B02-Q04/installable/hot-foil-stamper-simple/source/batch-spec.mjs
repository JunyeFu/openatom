export const specs={
 'hot-foil-stamper':{mode:'foil',preserve:false,size:[.50,.0008,.40],mark:[.12,.12],auxiliary:'foil_patch',carrier:[.16,.000012,.16],metal_thickness:.00000015,initial:'flat',final:'foil_stamped',extra_outputs:[{role:'spent_backing',count:1}]},
 'blind-embosser':{mode:'emboss',preserve:true,size:[.50,.0008,.40],mark:[.10,.10],raise:.0012,initial:'flat',final:'embossed',extra_outputs:[]},
 'spot-uv-printer':{mode:'uv',preserve:true,size:[.50,.0008,.40],mark:[.10,.10],auxiliary:'uv_varnish',volume_m3:.0000002,initial:'flat',final:'uv_cured',extra_outputs:[]},
 'screen-print-press':{mode:'screen',preserve:true,size:[.50,.0008,.40],mark:[.10,.10],auxiliary:'screen_ink',volume_m3:.0000002,initial:'flat',final:'screen_printed',extra_outputs:[]},
 'pad-print-press':{mode:'pad',preserve:true,size:[.20,.0008,.14],mark:[.04,.04],auxiliary:'pad_ink',volume_m3:.00000008,initial:'flat',final:'pad_printed',extra_outputs:[]},
};
export const surface=.90,inputX=-1.55,outputX=1.55,wellSize=.04;
export const clamp=(v,a=0,b=1)=>Math.max(a,Math.min(b,v));
export function lerpKey(u,points){for(let i=1;i<points.length;i++)if(u<=points[i][0]){const [a,x]=points[i-1],[b,y]=points[i];return x+(y-x)*clamp((u-a)/(b-a));}return points.at(-1)[1];}
export const workX=s=>s.mode==='pad'?.40:0;
export function paperX(s,phase,u){if(phase==='load')return inputX+(workX(s)-inputX)*u;if(s.mode==='uv'&&phase==='auto')return lerpKey(u,[[0,0],[.55,0],[.65,.80],[1,.80]]);return s.mode==='uv'&&phase==='unload'?.80:workX(s);}
export const paperPort=s=>[inputX,surface,0],outputPort=s=>[outputX,surface,0],auxPort=s=>[inputX,s.mode==='foil'?.95:.908,-.85],backingPort=[outputX,.95,-.85];
export const filmHeight=s=>s.mode==='foil'?s.metal_thickness:s.volume_m3/(s.mark[0]*s.mark[1]);
export function printedFraction(s,u){if(s.mode==='foil')return clamp((u-.65)/.10);if(s.mode==='pad')return clamp((u-.70)/.08);if(s.mode==='screen'||s.mode==='uv'){const x=lerpKey(u,[[0,-.16],[.15,-.16],[.50,.16],[1,.16]]);return clamp((x+s.mark[0]/2)/s.mark[0]);}return 0;}
export const screenY=(phase,u)=>phase==='auto'?lerpKey(u,[[0,1.10],[.12,.9008],[.50,.9008],[.55,1.10],[1,1.10]]):1.10;
export const screenX=u=>lerpKey(u,[[0,-.16],[.15,-.16],[.50,.16],[1,.16]]);
export const foilX=u=>lerpKey(u,[[0,0],[.82,0],[1,-.35]]),foilY=s=>u=>lerpKey(u,[[0,.95],[.15,surface+s.size[1]],[.75,surface+s.size[1]],[.82,.95],[1,.95]]);
export const pressTop=(s,u)=>s.mode==='foil'?lerpKey(u,[[0,1.10],[.20,surface+s.size[1]+s.metal_thickness+s.carrier[1]],[.65,surface+s.size[1]+s.metal_thickness+s.carrier[1]],[.82,1.10],[1,1.10]]):lerpKey(u,[[0,1.10],[.20,surface+s.size[1]],[.65,surface+s.size[1]],[.82,1.10],[1,1.10]]);
export const padX=u=>lerpKey(u,[[0,-.40],[.40,-.40],[.60,.40],[1,.40]]);
export const padY=u=>lerpKey(u,[[0,1.10],[.20,.9804],[.30,.9804],[.40,1.10],[.60,1.10],[.70,.98085],[.78,.98085],[.90,1.10],[1,1.10]]);
export function auxSeat(s,phase,u){if(s.mode==='foil')return [foilX(phase==='auto'?u:phase==='unload'?1:0),foilY(s)(phase==='auto'?u:phase==='unload'?1:0),0];if(s.mode==='pad')return [-.40,.90035,0];return [-.36,screenY(phase,u)+.08,-.33];}
export const samples=[...new Set([...Array.from({length:513},(_,i)=>i/512),.10,.12,.14,.15,.20,.25,.2703125,.28,.30,.3796875,.40,.50,.55,.60,.65,.70,.75,.78,.82,.90])].sort((a,b)=>a-b);
