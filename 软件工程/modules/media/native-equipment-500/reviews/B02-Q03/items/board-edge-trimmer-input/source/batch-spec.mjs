export const specs={
 'flexo-slotter':{mode:'slot',printed:true,size:[.50,.003,.60],surface:.90,toolX:.35,inputX:-1.80,outputX:1.80,ink_volume_m3:.000002,extra_outputs:[{role:'scrap',count:6,size:[.10,.003,.006],area:.0006}],area:.30,product_area:.2964,initial:'raw',final:'printed_slotted'},
 'corrugated-slotter':{mode:'slot',printed:false,size:[.50,.003,.60],surface:.90,toolX:.35,inputX:-1.80,outputX:1.80,extra_outputs:[{role:'scrap',count:6,size:[.10,.003,.006],area:.0006}],area:.30,product_area:.2964,initial:'raw',final:'slotted'},
 'board-edge-trimmer':{mode:'trim',size:[.54,.003,.44],product:[.50,.003,.40],surface:.90,toolX:0,inputX:-1.80,outputX:1.80,extra_outputs:[{role:'side_scrap',count:2,size:[.54,.003,.02],area:.0108},{role:'end_scrap',count:2,size:[.02,.003,.40],area:.008}],area:.2376,product_area:.20,initial:'raw',final:'trimmed'},
 'paper-perforator':{mode:'perforate',size:[.50,.0005,.40],surface:.90,toolX:.25,inputX:-1.80,outputX:1.80,extra_outputs:[{role:'scrap',count:25,size:[.010,.0005,.0006],area:.000006}],area:.20,product_area:.19985,initial:'raw',final:'perforated'},
 'platen-creaser':{mode:'crease',size:[.50,.0005,.40],surface:.82,toolX:0,inputX:-1.30,outputX:1.30,extra_outputs:[],area:.20,product_area:.20,initial:'flat',final:'creased'},
};
export const feedStart=-.65,feedEnd=.75,feedTravel=1.40;
export function seatX(s,phase,u){const feed=['slot','perforate'].includes(s.mode),start=feed?feedStart:0;if(phase==='load')return s.inputX+(start-s.inputX)*u;if(phase==='auto'&&feed)return feedStart+feedTravel*u;return feed?feedEnd:0;}
export const scrapStage=s=>s.mode==='trim'?[0,.90,0]:[s.toolX+.11,.810,0],scrapPort=s=>[s.toolX,.285,-.90];
export const inputPort=s=>[s.inputX,s.surface,0],outputPort=s=>[s.outputX,s.surface,0];
export const inkPort=[-1.30,.908,-.80],inkSeat=[-.53,1.105,-.42];
export const slotCenters=[-.15,0,.15],slotRadius=.125,perforationRadius=.5/(2*Math.PI);
export const pressTimes=[0,.15,.40,.60,.85,1],pressY=[.8405,.8395,.83835,.83835,.94,1.05];
export function scrapPieces(s){
 if(s.mode==='slot')return [-1,1].flatMap(sign=>slotCenters.map(z=>({role:'scrap',size:[.10,.003,.006],position:[sign*.20,.0015,z],cut_u:(s.toolX-(sign>0?.15:-.25)-feedStart)/feedTravel})));
 if(s.mode==='trim')return [{role:'side_scrap',size:[.54,.003,.02],position:[0,.0015,-.21],cut_u:.64},{role:'side_scrap',size:[.54,.003,.02],position:[0,.0015,.21],cut_u:.64},{role:'end_scrap',size:[.02,.003,.40],position:[-.26,.0015,0],cut_u:.82},{role:'end_scrap',size:[.02,.003,.40],position:[.26,.0015,0],cut_u:.82}];
 if(s.mode==='perforate')return Array.from({length:25},(_,i)=>({role:'scrap',size:[.010,.0005,.0006],position:[-.24+i*.02,.00025,0],cut_u:(s.toolX-(-.245+i*.02)-feedStart)/feedTravel}));
 return [];
}
export function scrapBinOffset(s,role,index){if(s.mode==='trim')return [role==='end_scrap'?index*.045-.0225:0,.003*(role==='end_scrap'?1:0),role==='side_scrap'?index*.045-.0225:0];return [(index%5-2)*.025,Math.floor(index/5)*s.size[1],0];}

export function scrapOffset(s,role,index){
 if(s.mode==='trim')return role==='side_scrap'?[0,0,index===0?-.2125:.2125]:[index===0?-.2625:.2625,0,0];
 if(s.mode==='slot')return [0,index<3?.003:0,slotCenters[index%3]];
 return scrapBinOffset(s,role,index);
}
