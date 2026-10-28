import {test} from 'node:test';import assert from 'node:assert/strict';import {readFile,writeFile,mkdir} from 'node:fs/promises';import {stripTypeScriptTypes} from 'node:module';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-static/old-item-layout/compiled/',root);await mkdir(out,{recursive:true});
for(const name of ['presentationHost','factoryPresentation']){let source=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/'+name+'.ts',root),'utf8'),{mode:'transform'}).replaceAll("from './frozenProcess'","from './frozenProcess.mjs'").replace(/import \{ dispatch \} from '.\/projectHost';/,"const dispatch=()=>{throw new Error('Protocol fixture never invokes Native')};").replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('Protocol fixture never invokes Native bytes')};").replace("from './presentationHost'","from './presentationHost.mjs'");await writeFile(new URL(name+'.mjs',out),source);}
let fixtureSource=stripTypeScriptTypes(await readFile(new URL('tests/planner/factory-media/fixture.ts',root),'utf8'),{mode:'transform'});for(const match of [...fixtureSource.matchAll(/import (\w+) from '([^']+\.json)';/g)])fixtureSource=fixtureSource.replace(match[0],`const ${match[1]}=${await readFile(new URL(match[2].replace('../../../',''),root),'utf8')};`);await writeFile(new URL('fixture.mjs',out),fixtureSource);
await writeFile(new URL('frozenProcess.mjs',out),stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/frozenProcess.ts',root),'utf8'),{mode:'transform'}));
const service=await import(new URL('factoryPresentation.mjs',out)),{fixture,fixturePorts,frames,pose}=await import(new URL('fixture.mjs',out));
test('persisted generated device slot is consumed from the same frozen Project and original ItemsAt without an attachment endpoint',async()=>{
 const data=fixture(),frame=frames.find(v=>v.label==='calendar@35000').frame,item=frame.snapshot.objects.find(v=>v.kind==='item').data,device=data.project.devices[0];
 const type=data.project.item_types.find(t=>t.id===item.item_type_ref.id);type.revision=item.item_type_ref.revision;type.module_ref={id:'owned.item',version:'1.0.0'};
 data.packages.push({definition:{...structuredClone(data.model.definition),id:'owned.item',version:'1.0.0',payload:{category:'items',data:{presentation_or_media_ref:data.model.definition.payload.data}}},assets:data.model.assets,files:data.model.files,dependencies:[],rights:data.model.rights,issues:[]});
 device.item_layout={assignment:'item_id_order',slots:[{item_type_ref:item.item_type_ref,anchor_role:'fixture-workpiece',offset:pose(.25,0)}]};
 const context={session_id:'pure-layout',project_id:data.project.id,scope:{kind:'project'},scope_revision:9,project:data.project,view:{run_id:frame.run.id,at_ms:35000}},raw=fixturePorts(data,context,async()=>new ArrayBuffer(0));
 const result=await service.createFactoryPresentation(()=>context,raw.ports).read(context),actual=result.items.find(v=>v.item.id===item.id);assert.ok(actual.attachment,actual.unavailable);assert.equal(actual.attachment.slot.anchor_role,'fixture-workpiece');assert.equal(actual.attachment.owner_ref.id,device.id);assert.deepEqual(actual.item,item);
});
const slot=(ref,x=0,anchor_role='workpiece')=>({item_type_ref:structuredClone(ref),anchor_role,offset:pose(x,0)});
function population(){
 const data=fixture(),f=frames.find(v=>v.label==='calendar@35000').frame,prototype=f.snapshot.objects.find(v=>v.kind==='item').data;
 const job=f.snapshot.objects.find(v=>v.kind==='job').data;
 const device=data.project.devices.find(v=>v.id===job.execution_ref.data.device_id);data.project.halls[0].id=device.hall_id;
 const make=(id,holder={kind:'job',id:job.id},extra={})=>({...structuredClone(prototype),id,holder,lifecycle:'available',contents:[],composition_refs:[],...extra});
 return{data,device,job,make,ref:prototype.item_type_ref};
}
test('lexical identity and slot order use a single physical device pool across multiple Jobs; batch quantity does not clone identity',()=>{
 const {data,device,job,make,ref}=population();const second={...job,id:'later-job'};
 device.item_layout={assignment:'item_id_order',slots:[slot(ref,9),slot(ref,3),slot(ref,6)]};
 const items=[make('part-2'),make('part-10',{kind:'job',id:second.id}),make('part-1',undefined,{quantity:{value:400,unit:'piece'}})];
 const result=service.projectFactoryItemSlots(data.project,items,[job,second]);
 assert.deepEqual(['part-1','part-10','part-2'].map(id=>result.get(id).attachment.slot.offset.translation.x),[9,3,6]);assert.equal(result.size,3);assert.equal(items[2].quantity.value,400);
});
test('overflow rejects the whole exact type pool while keeping all identities; missing and explicit empty remain distinct',()=>{
 const {data,device,job,make,ref}=population();const items=[make('a'),make('b')];device.item_layout={assignment:'item_id_order',slots:[slot(ref)]};
 let r=service.projectFactoryItemSlots(data.project,items,[job]);for(const id of ['a','b']){assert.equal(r.get(id).attachment,null);assert.match(r.get(id).unavailable,/2 identities \/ 1 declared/);}
 device.item_layout=null;r=service.projectFactoryItemSlots(data.project,[items[0]],[job]);assert.match(r.get('a').unavailable,/未声明/);
 device.item_layout={assignment:'item_id_order',slots:[]};r=service.projectFactoryItemSlots(data.project,[items[0]],[job]);assert.match(r.get('a').unavailable,/1 identities \/ 0 declared/);
 device.item_layout.slots=[slot({...ref,revision:ref.revision+'-different'})];assert.match(service.projectFactoryItemSlots(data.project,[items[0]],[job]).get('a').unavailable,/0 declared/);
});
test('location requires real hall and null anchor, device requires explicit scene role, transport and inactive IDs do not receive geometry',()=>{
 const {data,device,job,make,ref}=population();data.project.locations=[{id:'loc',hall_id:device.hall_id,item_layout:{assignment:'item_id_order',slots:[slot(ref,1,null)]}}];
 const items=[make('loc-item',{kind:'location',id:'loc'}),make('moving',{kind:'transport',id:'transport'}),make('past',undefined,{lifecycle:'transformed'}),make('scrap',undefined,{lifecycle:'scrapped'}),make('live')];
 device.item_layout={assignment:'item_id_order',slots:[slot(ref,0,null)]};let r=service.projectFactoryItemSlots(data.project,items,[job]);assert.equal(r.get('loc-item').attachment.owner_ref.id,'loc');assert.match(r.get('moving').unavailable,/transport/);assert.match(r.get('live').unavailable,/anchor_role/);assert.equal(r.get('past').attachment,null);assert.equal(r.get('scrap').attachment,null);
 data.project.locations[0].item_layout.slots[0].anchor_role='wrong';assert.match(service.projectFactoryItemSlots(data.project,items,[job]).get('loc-item').unavailable,/必须为null/);
 data.project.locations[0].hall_id='missing';assert.match(service.projectFactoryItemSlots(data.project,items,[job]).get('loc-item').unavailable,/厂房/);
});
test('container uses reciprocal contents and exact parent type layout; composition cannot authorize containment',()=>{
 const {data,device,job,make,ref}=population();const parentType=data.project.item_types[0];parentType.revision='precise-parent';parentType.contents_layout={assignment:'item_id_order',slots:[slot(ref,4,'cavity')]};
 const parent=make('box',undefined,{item_type_ref:{id:parentType.id,revision:parentType.revision},contents:['child']});const child=make('child',{kind:'container',id:parent.id});
 const r=service.projectFactoryItemSlots(data.project,[child,parent],[job]);assert.equal(r.get(child.id).attachment.owner_ref.kind,'item');assert.equal(r.get(child.id).attachment.slot.anchor_role,'cavity');
 parent.contents=[];parent.composition_refs=['child'];assert.match(service.projectFactoryItemSlots(data.project,[child,parent],[job]).get(child.id).unavailable,/双向/);
 parent.contents=['child'];parent.item_type_ref.revision='stale';assert.match(service.projectFactoryItemSlots(data.project,[child,parent],[job]).get(child.id).unavailable,/精确type/);
});
test('container ancestry cycles, inactive or transport parents and nonreciprocal ancestors are explicit unavailable',()=>{
 const {data,job,make,ref}=population();data.project.item_types.find(t=>t.id===ref.id).revision=ref.revision;
 const a=make('a',{kind:'container',id:'b'},{contents:['b']}),b=make('b',{kind:'container',id:'a'},{contents:['a']});
 let r=service.projectFactoryItemSlots(data.project,[a,b],[job]);assert.match(r.get('a').unavailable,/循环/);
 b.holder={kind:'job',id:job.id};b.lifecycle='scrapped';assert.match(service.projectFactoryItemSlots(data.project,[a,b],[job]).get('a').unavailable,/父容器已/);
 b.lifecycle='available';b.holder={kind:'transport',id:'t'};assert.match(service.projectFactoryItemSlots(data.project,[a,b],[job]).get('a').unavailable,/transport/);
 b.holder={kind:'container',id:'c'};const c=make('c',undefined,{contents:[]});assert.match(service.projectFactoryItemSlots(data.project,[a,b,c],[job]).get('a').unavailable,/不互认/);
});
test('input model distinguishes undeclared/empty, requires explicit new pose, and preserves canonical full owner',async()=>{
 let source=stripTypeScriptTypes(await readFile(new URL('apps/planner/src/features/item-layout/model.ts',root),'utf8'),{mode:'transform'});await writeFile(new URL('layoutModel.mjs',out),source);const m=await import(new URL('layoutModel.mjs',out));
 const owner={kind:'device',id:'CNC'},input=m.layoutInput(null);assert.equal(m.parseLayout(input,owner),null);input.declared=true;assert.throws(()=>m.parseLayout(input,owner),/明确选择/);input.assignment='item_id_order';assert.deepEqual(m.parseLayout(input,owner),{assignment:'item_id_order',slots:[]});input.slots.push(m.blankSlot());assert.throws(()=>m.parseLayout(input,owner),/精确子类型/);input.slots[0].item=m.typeKey({id:'type',revision:'r1'});assert.throws(()=>m.parseLayout(input,owner),/单位与坐标系/);
 input.slots=[m.slotInput(slot({id:'type',revision:'r1'},2,null))];assert.equal(m.parseLayout(input,owner).slots[0].anchor_role,null);const obj=m.layoutObject(fixture().project,owner),withLayout=m.withLayout(obj,m.parseLayout(input,owner));const {item_layout,...unchanged}=withLayout.data;assert.deepEqual(unchanged,obj.data);assert.equal(item_layout.slots[0].offset.translation.x,2);
});
