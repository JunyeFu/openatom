export const paper={length:30,width:.50,thickness:.0001,core:.04,layers:32,sheetLength:.50};
export const radius=(fraction=1)=>Math.sqrt(paper.core**2+paper.length*paper.thickness*fraction/Math.PI);
export const unwindAngle=u=>2*Math.PI*(radius()-radius(1-u))/paper.thickness;
export const rewindAngle=u=>2*Math.PI*(radius(u)-paper.core)/paper.thickness;
export const specs={
 'suction-separator':{mode:'sheet',inputShape:'sheet',outputShape:'sheet',quantity:1,datum:[-.525,.905,0],input:[-1.55,.905,0],output:[1.55,.9025,0],inputSurface:.9035,outputSurface:.901,initialState:'adhered',finalState:'separated',sheet:[.48,.0015,.40]},
 'paper-unwinder':{mode:'unwind',inputShape:'roll',outputShape:'web',quantity:1,datum:[-.525,1.27,0],input:[-1.55,1.27,0],output:[1.55,.901,0],fold:[.525,.901,0],inputAxis:-.525,initialState:'rolled',finalState:'unwound'},
 'paper-rewinder':{mode:'rewind',inputShape:'web',outputShape:'roll',quantity:1,datum:[-.525,.901,0],input:[-1.55,.901,0],output:[1.55,1.27,0],fold:[-.525,.901,0],outputAxis:.525,initialState:'unwound',finalState:'rolled'},
 'slitter-rewinder':{mode:'slit',inputShape:'roll',outputShape:'roll',quantity:2,datum:[-1.05,1.27,0],input:[-2.08,1.27,0],output:[2.08,1.27,0],inputAxis:-1.05,outputAxis:1.05,outputWidth:.25,initialState:'wide_roll'},
 'rotary-sheet-cutter':{mode:'crosscut',inputShape:'roll',outputShape:'sheet',quantity:60,datum:[-1.05,1.27,0],input:[-2.08,1.27,0],output:[2.08,.901,0],inputAxis:-1.05,fold:[1.05,.901,0],outputWidth:.50,initialState:'rolled',sheet:[.50,.0001,.50]},
};
export const suctionTimes=[0,.15,.35,.70,.85,1];
export const suctionPath=[[-.525,1.0725,0],[-.525,1.2225,0],[-.525,1.2225,0],[.525,1.2225,0],[.525,1.2225,0],[.525,1.0700,0]];
export function paperPaths(spec){const s=specs[spec.id],r=radius();
 if(s.mode==='unwind')return [{width:.5,points:[[s.inputAxis,1.27+r,0],[.25,1,0],[.525,1,0]]}];
 if(s.mode==='rewind')return [{width:.5,points:[[-.525,.907,0],[-.25,1,0],[s.outputAxis,1.27+r,0]]}];
 if(s.mode==='slit')return [{width:.5,points:[[s.inputAxis,1.27+r,0],[-.25,1,0],[0,1,0]]},...[-1,1].map(sign=>({width:.25,points:[[0,1,sign*.125],[.25,1,sign*.125],[s.outputAxis,1.27+r,sign*.135]]}))];
 if(s.mode==='crosscut')return [{width:.5,points:[[s.inputAxis,1.27+r,0],[-.25,1,0],[0,1,0],[.35,1,0],[1.05,.907,0]]}];
 return [];
}

// A continuous 30m ribbon: 60 straight spans joined by 59 paper-thickness semicircular folds.
export const foldCount=60,foldRadius=paper.thickness/2,foldStraight=(paper.length-(foldCount-1)*Math.PI*foldRadius)/foldCount;
export const foldEntryY=.960,foldTimes=Array.from({length:1201},(_,i)=>i/1200);
export const foldFraction=(s,u)=>s.mode==='rewind'?1-u:u;
export function foldHeadX(s,u){const amount=foldFraction(s,u)*foldCount,index=Math.min(foldCount-1,Math.max(0,Math.ceil(amount)-1)),f=amount-index;return s.fold[0]+(index%2?-1:1)*(f-.5)*foldStraight;}
export const foldLayer=(s,u)=>Math.min(foldCount-1,Math.max(0,Math.ceil(foldFraction(s,u)*foldCount)-1));
