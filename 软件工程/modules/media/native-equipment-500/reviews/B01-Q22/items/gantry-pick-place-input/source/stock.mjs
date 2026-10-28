import * as T from 'three';
import{geometry}from './geometry.mjs';
export function stock(id,role){const a=geometry(false),root=new T.Group();root.name='StockRoot';if(role==='blocker')a.box(root,'OutputOccupant',[.10,.04,.10],[0,.02,0],a.m.amber);else{a.box(root,'TransportCarton',[.24,.12,.18],[0,.06,0],a.m.paper);a.box(root,'CartonTopSeam',[.24,.0005,.018],[0,.1203,0],a.m.tape,false);for(const z of[-.0903,.0903])a.box(root,'CartonFold_'+z,[.001,.10,.0005],[0,.06,z],a.m.edge,false);}return{root,parts:[]};}
