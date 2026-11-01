import * as T from 'three';
const vec=i=>new T.Vector3(...i),prismFaces=[[0,2,1],[3,4,5],[0,1,4,3],[1,2,5,4],[2,0,3,5]],cache=new WeakMap();
export function localSolids(g){
 if(cache.has(g))return cache.get(g);const p=g.attributes.position,idx=g.index,verts=Array.from({length:p.count},(_,i)=>new T.Vector3().fromBufferAttribute(p,i)),n=idx?.count??p.count,faces=[];for(let i=0;i<n;i+=3)faces.push([0,1,2].map(j=>idx?idx.getX(i+j):i+j));let out;
 if(g.type==='ExtrudeGeometry'){
  if(g.parameters.options.bevelEnabled!==false)throw Error('unreviewed bevel extrusion');const cap=g.groups[0],f=faces[cap.start/3],normal=verts[f[1]].clone().sub(verts[f[0]]).cross(verts[f[2]].clone().sub(verts[f[0]])).normalize(),depth=g.parameters.options.depth,outward=normal.clone().multiplyScalar(-depth);out=[];
  for(let i=cap.start/3;i<(cap.start+cap.count)/3;i++){const f=faces[i],a=f.map(j=>verts[j]),fn=a[1].clone().sub(a[0]).cross(a[2].clone().sub(a[0])).normalize();if(fn.dot(normal)<.999)continue;out.push({verts:[...a,...a.map(v=>v.clone().add(outward))],faces:prismFaces});}
 }else if(g.type==='TubeGeometry'){
  const{tubularSegments:ts,radialSegments:rs}=g.parameters;out=[];for(let i=0;i<ts;i++){const a=Array.from({length:rs},(_,j)=>verts[i*(rs+1)+j]),b=Array.from({length:rs},(_,j)=>verts[(i+1)*(rs+1)+j]);out.push({verts:[...a,...b],faces:[a.map((_,j)=>j).reverse(),b.map((_,j)=>j+rs),...a.map((_,j)=>[j,(j+1)%rs,(j+1)%rs+rs,j+rs])]});}
 }else out=[{verts,faces}];cache.set(g,out);return out;
}
