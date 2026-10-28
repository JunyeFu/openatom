import {test,after} from 'node:test';import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';import {stripTypeScriptTypes} from 'node:module';
import {Raycaster,Vector2,Vector3,PerspectiveCamera,Box3} from 'three';
import {openCore} from '../../node-inspector/core.mjs';import {root,out,createNodeInspectorReader,model} from './load-source.mjs';
const compiled=new URL('compiled/presentation/',out);await mkdir(compiled,{recursive:true});
for(const name of await readdir(new URL('packages/presentation/src/',root))){if(!name.endsWith('.ts'))continue;const code=stripTypeScriptTypes(await readFile(new URL('packages/presentation/src/'+name,root),'utf8'),{mode:'transform'}).replace(/from '(\.\/[^']+)'/g,(_,path)=>`from '${path}.mjs'`);await writeFile(new URL(name.replace(/\.ts$/,'.mjs'),compiled),code);}
const presentation=await import(new URL('index.mjs',compiled));
const load=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/features/media/load.ts',root),'utf8'),{mode:'transform'}).replace("from '../../../../../packages/presentation/src'","from './presentation/index.mjs'");await writeFile(new URL('compiled/media-load.mjs',out),load);const {resolveMedia}=await import(new URL('compiled/media-load.mjs',out));
const view=await readFile(new URL('apps/planner/src/features/media/viewport.ts',root),'utf8'),a=view.indexOf('  private pointerUp='),b=view.indexOf('\n  private fitSize=',a);assert.ok(a>=0&&b>a);const arrow=view.slice(a,b).trim().replace(/^private pointerUp=/,'const pointerUp='),handler=stripTypeScriptTypes(arrow,{mode:'transform'}),pointerUp=new Function('Raycaster','Vector2',`return function(){${handler}\nreturn pointerUp;}`)(Raycaster,Vector2);await writeFile(new URL('reviewed-pointer-handler.ts',out),arrow);
const core=await openCore('scenarios/图片时间轴/runtime/phase.input.json'),pid=core.ready.snapshot.project.id,scope={kind:'project'},evidence=[];
for(const directory of JSON.parse(await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/registry.json',root),'utf8')))await core.rpc({kind:'register',directory});
const ports={async dispatch(request){const reply=await core.rpc({kind:'dispatch',request});if(reply.outcome.status!=='ok')throw Object.assign(new Error(reply.outcome.data.message),{detail:reply.outcome.data});return reply.outcome.data;}},request=(operation,params)=>ports.dispatch({api_version:'0.1',request_id:'independent-render-'+crypto.randomUUID(),project_id:pid,scope,operation,params});
after(async()=>{await writeFile(new URL('actual-render-source.json',out),JSON.stringify({evidence,records:core.records,Native:false,browser:false,WebGL:false,actual_three_raycaster:true,synthetic_rect_and_pointer:true},null,2));await core.close();});
const ref=(id,version='1.0.0')=>({id,version}),source=ref=>({kind:'module_version',data:{module_ref:ref}});
async function pkg(reference){const result=await request('get_object',{object_id:{kind:'module',id:reference.id},source:source(reference),node_index:null});assert.equal(result.kind,'module');return result.data;}
async function bytes(source,asset){const result=await core.rpc({kind:'media',project_id:pid,scope,scope_revision:core.ready.snapshot.project.revision,source,asset});assert.equal(result.ok,true,JSON.stringify(result.error));return Uint8Array.from(result.bytes).buffer;}
function hit(rig,index){rig.root.updateMatrixWorld(true);const target=new Box3().setFromObject(rig.nodes.get(index)).getCenter(new Vector3()),camera=new PerspectiveCamera(38,1,.01,1000);camera.position.copy(target).add(new Vector3(0,0,3));camera.lookAt(target);camera.updateMatrixWorld(true);const picked=[],context={down:{x:100,y:100},renderer:{domElement:{getBoundingClientRect:()=>({left:0,top:0,width:200,height:200})}},camera,rig,onSelect:i=>picked.push(i)};pointerUp.call(context)({button:0,clientX:100,clientY:100});return {picked,target:target.toArray()};}
test('actual shared MediaRef and both LOD paths retain final metadata owner independently of original byte basis',async()=>{
  for(const version of ['1.0.0','2.0.0']){const wrapper=await pkg(ref('inspector.adopted',version));
  for(const lod of ['detailed','simplified']){const props={presentation:wrapper.definition.payload.data,lod,rights:wrapper.rights,
    nodeOwner:{object_id:{kind:'module',id:wrapper.definition.id},source:source(ref('inspector.adopted',version))},binding:{session_id:core.ready.context.host.session_id,request:{project_id:pid,scope,scope_revision:core.ready.snapshot.project.revision,source:source(ref('inspector.adopted',version))}},
    resolveModule:s=>pkg(s.data.module_ref),readAsset:q=>bytes(q.source,q.asset)};
    const resolved=await resolveMedia(props);assert.equal(resolved.owner.object_id.id,lod==='detailed'?'inspector.native':'inspector.native.simple');assert.equal(resolved.owner.source.data.module_ref.version,version);
    const actual=await request('get_object',{object_id:resolved.owner.object_id,source:resolved.owner.source,node_index:0});assert.deepEqual(actual.data,resolved.presentation.nodes[0]);
    assert.equal(resolved.presentation.nodes[0].id,lod==='detailed'?91:702);evidence.push({name:'actual nested MediaRef LOD exact owner',version,lod,owner:resolved.owner,byte_source:resolved.source,canonical0_id:resolved.presentation.nodes[0].id});
  }
  }
});
test('actual Three native geometry and production pointer handler hit canonical index instead of duplicated name or SceneNode id',async()=>{
  const p=(await pkg(ref('inspector.native'))).definition.payload.data,rig=await presentation.bindScene(p,()=>{throw new Error('native null asset must not request bytes')});try{
    const picked=hit(rig,1);assert.deepEqual(picked.picked,[1]);assert.equal(p.nodes[1].id,702);assert.equal(p.nodes[0].name,p.nodes[1].name);assert.equal(rig.nodes.size,2);evidence.push({name:'actual native canonical pick math',...picked,node:p.nodes[1],asset:null});
  }finally{rig.dispose();}
});
test('actual SDK GLB decode binding and nearest visible pick keep canonical position node id and GLB index distinct',async()=>{
  const module=await pkg(ref('inspector.glb')),p=module.definition.payload.data,asset=p.nodes[0].geometry.data.asset_ref,raw=await bytes(source(ref('inspector.glb')),asset),decoded=await presentation.decodeGlb(raw),rig=await presentation.bindScene(p,()=>{throw new Error('fixture declares no texture byte reads')},{assetKey:presentation.assetKey(asset),decoded});try{
    // Looking at node53's centre in the complete real fixture first hits its
    // visible occluder (canonical48/GLB47). Do not manufacture a pick through it.
    const picked=hit(rig,0);assert.deepEqual(picked.picked,[48]);const index=picked.picked[0],hostNode=await request('get_object',{object_id:{kind:'module',id:'inspector.glb'},source:source(ref('inspector.glb')),node_index:index});
    assert.equal(hostNode.data.id,1047);assert.equal(hostNode.data.geometry.data.node_index,47);assert.deepEqual(hostNode.data,p.nodes[index]);assert.equal(p.nodes[0].geometry.data.node_index,53);assert.equal(p.nodes[0].id,1053);assert.equal(rig.nodes.get(0),decoded.nodes.get(53));evidence.push({name:'actual GLB decode bind and nearest visible pick math',...picked,node:hostNode.data,target_node0_is_occluded:true,canonical0_still_binds_GLB53:true,bytes:raw.byteLength});
  }finally{rig.dispose();}
});
test('real metadata replies are discarded on every selection identity change including close LOD source asset seek and ScopeR',async()=>{
  const p=await pkg(ref('inspector.native')),base={context:{session_id:core.ready.context.host.session_id,project_id:pid,scope,scope_revision:core.ready.snapshot.project.revision,selection:{kind:'definition',module_ref:ref('inspector.native')}},owner:{object_id:{kind:'module',id:'inspector.native'},source:source(ref('inspector.native'))},node_index:1,lod:'detailed',asset:null,byte_source:source(ref('inspector.native'))};
  const cases=[['close',()=>null],['session',s=>({...s,context:{...s.context,session_id:'other'}})],['project',s=>({...s,context:{...s.context,project_id:'other'}})],['ScopeR',s=>({...s,context:{...s.context,scope_revision:99}})],['Draft',s=>({...s,context:{...s.context,scope:{kind:'definition_draft',data:'other'}}})],['LOD',s=>({...s,lod:'simplified'})],['index',s=>({...s,node_index:0})],['owner',s=>({...s,owner:{...s.owner,object_id:{kind:'module',id:'other'}}})],['asset',s=>({...s,asset:{id:'other',path:'other.glb',kind:'glb'}})],['bytes',s=>({...s,byte_source:source(ref('inspector.native','2.0.0'))})],['seek',s=>({...s,context:{...s.context,selection:{kind:'actual_run',run_id:'other',job_id:'other',at_ms:1}}})]];
  for(const [name,change] of cases){let current=structuredClone(base),changed=false;const reader=createNodeInspectorReader(()=>current,{async dispatch(q){const result=await ports.dispatch(q);if(!changed&&q.operation==='get_object'&&q.params.node_index!==null){changed=true;current=change(current);}return result;}});await assert.rejects(reader.read(base),e=>e.name==='ProcessSourceChanged',name);evidence.push({name:'actual late metadata discarded',change:name,actual_Host_reply_received:changed});}
  assert.equal(p.definition.payload.data.nodes[1].id,702);
});
