export const specs={
 'sheet-feeder':{mode:'feed',size:[.50,.0008,.40],sheets:1,initial:'flat',final:'flat',preserve:true,extra_outputs:[]},
 'pile-lift':{mode:'lift',size:[.50,.016,.40],sheets:20,initial:'stacked',final:'stacked',preserve:true,extra_outputs:[]},
};
export const clamp=v=>Math.max(0,Math.min(1,v));
export function lerpKey(u,points){for(let i=1;i<points.length;i++)if(u<=points[i][0]){const[a,x]=points[i-1],[b,y]=points[i];return x+(y-x)*clamp((u-a)/(b-a));}return points.at(-1)[1];}
export const samples=[...new Set([...Array.from({length:513},(_,i)=>i/512),.10,.12,.20,.30,.45,.50,.55,.60,.65,.75,.85,.90])].sort((a,b)=>a-b);
export const inputPort=s=>s.mode==='feed'?[-.55,.90,0]:[-1.0,.20,0];
export const outputPort=s=>[1.0,.90,0];
export const liftY=(phase,u)=>phase==='load'?.20:phase==='auto'?.20+.70*u:lerpKey(u,[[0,.90],[.65,.90],[1,.20]]);
export function paperPosition(s,phase,u){
 if(s.mode==='lift')return phase==='load'?[-1+u,.20,0]:phase==='auto'?[0,liftY(phase,u),0]:[u,.90,0];
 if(phase==='load')return[-.55,.90,0];
 if(phase==='unload')return[.75+.25*u,.90,0];
 return[lerpKey(u,[[0,-.55],[.12,-.55],[.45,.10],[.55,.10],[1,.75]]),lerpKey(u,[[0,.90],[.12,.94],[.45,.94],[.50,.90],[1,.90]]),0];
}
export function suctionPosition(phase,u){if(phase!=='auto')return[-.55,phase==='load'?lerpKey(u,[[0,1.05],[1,.9008]]):1.05,0];const p=paperPosition(specs['sheet-feeder'],phase,u);return u<=.50?[p[0],p[1]+.0008,0]:[lerpKey(u,[[.50,.10],[.60,.10],[.85,-.55],[1,-.55]]),lerpKey(u,[[.50,.9008],[.60,1.05],[1,1.05]]),0];}
