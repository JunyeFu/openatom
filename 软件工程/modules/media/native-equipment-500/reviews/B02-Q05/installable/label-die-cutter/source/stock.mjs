import * as T from 'three';import{geometry}from './geometry.mjs';import{web,labelXs}from './batch-spec.mjs';
export function artwork(api,root,prefix){const{box,m}=api;for(let j=0;j<9;j++)box(root,prefix+'Bar_'+j,[[.001,.002,.0015][j%3],.000012,.028],[-.028+j*.006,.000006,-.004],m.dark,false);box(root,prefix+'TextA',[.042,.000012,.002],[0,.000006,.020],m.dark,false);box(root,prefix+'TextB',[.028,.000012,.0015],[-.007,.000006,.025],m.dark,false);}
export function stock(id,role){const api=geometry(false),{box,group,m}=api,root=new T.Group();root.name='StockRoot';const parts=[],labels=[],cuts=[],peeler=id==='label-peeler',labelMat=new T.MeshStandardMaterial({name:'White label facestock',color:'#f0efe7',roughness:.78}),linerMat=new T.MeshStandardMaterial({name:'Release liner',color:'#baaa88',roughness:.55});let dose;
 if(role==='blocker')box(root,'OutputOccupant',[.10,.04,.10],[0,.02,0],m.amber,false);
 else if(role==='auxiliary'){dose=group(root,'MeteredInkDose');api.cyl(dose,'InkDoseCapsule',.012,.025,[0,.0125,0],m.dark);box(dose,'InkDoseMark',[.010,.0001,.008],[0,.02505,0],m.blue,false);}
 else if(peeler&&role==='output'){box(root,'PeeledLabel',[web.labelL,web.faceT,web.labelW],[0,web.faceT/2,0],labelMat,false);const g=group(root,'PrintedLabelArtwork',[0,web.faceT,0]);artwork(api,g,'Output_');}
 else{
  const segmented=peeler&&role==='input',step=segmented?.0125:web.L;
  function piece(name,a,b,z,width,y,height,mat){for(let x=a;x<b-1e-8;){const end=Math.min(b,x+step),centre=(x+end)/2,o=box(root,name+'_'+parts.length,[end-x,height,width],[centre,y,z],mat,false);parts.push({o,c:centre,y,z});x=end;}}
  piece('Liner',-web.L/2,web.L/2,0,web.W,web.linerT/2,web.linerT,linerMat);
  for(const sign of[-1,1])piece('SideMatrix_'+sign,-web.L/2,web.L/2,sign*.035,.010,web.linerT+web.faceT/2,web.faceT,labelMat);
  for(const[a,b]of[[-.30,-.29],[-.21,-.19],[-.11,-.09],[-.01,.01],[.09,.11],[.19,.21],[.29,.30]])piece('CrossMatrix',a,b,0,web.labelW,web.linerT+web.faceT/2,web.faceT,labelMat);
  if(role!=='backing')for(let i=0;i<6;i++){const g=group(root,'LabelFace_'+i,[labelXs[i],0,0]);box(g,'Facestock_'+i,[web.labelL,web.faceT,web.labelW],[0,web.linerT+web.faceT/2,0],labelMat,false);const ink=group(g,'PrintedPattern_'+i,[0,web.linerT+web.faceT,0]);artwork(api,ink,'Label_'+i+'_');ink.scale.x=0;
   const cut=group(g,'KissCut_'+i,[0,web.linerT+web.faceT+.00001,0]);for(const sign of[-1,1]){box(cut,'CutLong_'+i+'_'+sign,[web.labelL,.000015,.00025],[0,0,sign*web.labelW/2],m.edge,false);box(cut,'CutEnd_'+i+'_'+sign,[.00025,.000015,web.labelW],[sign*web.labelL/2,0,0],m.edge,false);}cut.scale.setScalar(0);labels.push({g,ink,cut,i});cuts.push(cut);}
  if(role==='backing'){root.rotation.z=Math.PI;root.position.y=web.linerT+web.faceT;}
 }
 return{root,parts,labels,cuts,dose};}
