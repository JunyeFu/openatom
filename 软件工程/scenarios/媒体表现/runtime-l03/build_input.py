"""The original L03 declarations adapted to current canonical commands.
The separate Engine fixture is test input, never a saved Host project archive.
"""
from copy import deepcopy
import importlib.util
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
BASE=HERE.parent/'runtime-line-grade/native-l03-input'
spec=importlib.util.spec_from_file_location('original_l03_builder',BASE/'build_plan.py')
old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)
read=lambda p:json.loads(Path(p).read_text(encoding='utf-8-sig'))
write=lambda n,v:(HERE/n).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
plan=old.build()
def update(v):
    if isinstance(v,list):
        for x in v:update(x)
    elif isinstance(v,dict):
        if str(v.get('id','')).startswith(('p302.l03.','p302.line-grade.')) and 'version' in v:v['version']='0.2.0' if str(v['id']).startswith('p302.l03.operation.') else '0.3.0'
        for x in v.values():update(x)
update(plan)
original=old.read('scenarios/媒体表现/runtime-line-grade/root-registration.json')['register_order']
new=read(ROOT/'modules/media/p302-l03-v03/registration.json')['register_order']
plan['register_order']=original+new
plan['formal_media_commands']=[old.adopt(dict(id=e['id'],version=e['version'])) for e in plan['register_order']]
plan['document_kind']='canonical_L03_Host_command_plan_and_separate_Engine_test_input'
plan['translation_complete']=True
plan['blocking_gaps']=[]
defs={d['id']:d for d in plan['definitions']}
for device,flow in {
    'R01':dict(primary_role='assembly',auxiliary_roles=[],acquire_after_phase='grip',consume_after_phase=None,
           reserve_before_phase='release',deliver_after_phase='release',required_primary_attributes={},attributes_after_consume={}),
    'CASE01':dict(primary_role='package',auxiliary_roles=['sealing_tape'],acquire_after_phase=None,consume_after_phase='seal',
           reserve_before_phase=None,deliver_after_phase=None,required_primary_attributes=dict(closure=dict(kind='enum',value='open')),
           attributes_after_consume=dict(closure=dict(kind='enum',value='sealed'))),
}.items():defs['p302.l03.operation.'+device]['payload']['data']['material_flow']=flow
for device in ['R01','CASE01']:defs['p302.l03.operation.'+device]['payload']['data']['duration']=None
package=defs['p302.l03.item.package']['payload']['data']
package['attributes']=dict(closure=dict(kind='enum',value='open'))
package['attribute_schema']=[dict(name='closure',value_type='enum',unit=None,shape=None,options=['open','sealed'],basis='user_provided')]

# Adopted exact Media owns all nodes, anchors, bytes and phase clips.
def media(family):
    p=deepcopy(read(ROOT/f'modules/media/p302-l03-v03/installable/{family}/module.json')['data']['definition']['payload']['data'])
    p['source']=dict(kind='media_ref',data=dict(id='p302.line-grade.'+family,version='0.3.0'))
    p['lod']['detailed']=p['source'];p['lod']['simplified']=p['source']
    for k in ['nodes','poses','actions','timeline','tags']:p[k]=[]
    p['anchor_roles']={};p['port_anchors']={}
    return p
for alias in ['shell','pcb','fastener','sealing_tape','assembly','carton','package']:
    defs['p302.l03.item.'+alias]['payload']['data']['presentation_or_media_ref']=media('control-box' if alias=='assembly' else 'carton' if alias in ['carton','package'] else alias.replace('_','-'))
defs['p302.l03.pallet12']['payload']['data']['presentation']=media('pallet')

def slot(item,offset=None,anchor='cargo'):
    return dict(item_type_ref=dict(id=item,revision='r1'),anchor_role=anchor,offset=offset or old.pose())
for command in plan['create_commands']:
    for obj in command['data']['new_instances']:
        d=obj['data']
        if obj['kind']=='item_type' and d['id']=='package':
            d['contents_layout']=dict(assignment='item_id_order',slots=[slot('assembly',old.pose(z=.004))])
        if obj['kind']=='device':
            sid=d['id'];items=['package'] if sid=='CASE01' else ['assembly']
            if sid=='PK01':items=['assembly','carton','package']
            if sid=='ASM01':items=['shell','pcb','fastener','assembly']
            if sid=='CASE01':items=['package','sealing_tape']
            offsets={}
            if sid=='R01':
                offsets['assembly']=dict(translation=dict(x=0,y=.0325,z=0),rotation=dict(x=1,y=0,z=0,w=0),
                       scale=dict(x=1,y=1,z=1),unit='meter',coordinate_frame='right_handed_y_up')
            if sid=='V01':offsets['assembly']=old.pose(z=.003)
            if sid=='PK01':offsets['assembly']=old.pose(x=-.32)
            d['item_layout']=dict(assignment='item_id_order',slots=[slot(i,offsets.get(i),i if sid=='ASM01' and i!='assembly' else 'tape' if i=='sealing_tape' else 'cargo') for i in items])
        if obj['kind']=='device' and d['id']=='ASM01':
            d['item_layout']['slots']=[slot('shell'),slot('pcb',old.pose(z=.026)),slot('assembly')]+[slot('fastener',old.pose(x=x,y=y,z=.025)) for x in [-.103,.103] for y in [-.073,.073]]
        elif obj['kind']=='resource' and d['id']=='PAL01':d['presentation']=media('pallet')
        elif obj['kind']=='resource' and d['kind']=='worker':
            station={'assembler':'ASM01','inspector':'V01','repairer':'RW01','packer':'PK01'}[d['role']]
            source=next(s for s in plan['business_source']['stations'] if s['id']==station)
            loc='worker-home-'+station;d['home_location']=loc
            plan['pre_commands'].append(old.put('location',dict(id=loc,title=loc,hall_id='H01',position=old.pose(source['placement'][0],source['placement'][1]-1.02),
                 capacity=old.quantity(1,'person'),accepted_item_types=[],purpose='buffer',ordinary_issue_enabled=False)))
for command in plan['pre_commands']:
    obj=command.get('data',{})
    if obj.get('kind')=='calendar':
        # The original local proof explicitly overrides the definition calendar
        # with always-available resources; the observation end is not a shift end.
        obj['data']['intervals']=[dict(start_ms=0,end_ms=9007199254740991)]
    if obj.get('kind')=='location':
        d=obj['data'];name=d['id']
        if name.startswith('stock-'):
            alias=name.removeprefix('stock-');n={'shell':6,'pcb':6,'fastener':24,'carton':6,'sealing_tape':3.6}[alias]
            d['capacity']=old.quantity(n,'meter' if alias=='sealing_tape' else 'piece')
        if d['accepted_item_types'] and name!='PAL01-store':
            alias=d['accepted_item_types'][0]
            if name.startswith('stock-'):
                positions={'shell':(-1,-3),'pcb':(-1,-4),'fastener':(0,-4),'carton':(8,-3),'sealing_tape':(12,-3)}
                d['position']=old.pose(*positions[alias])
                offsets=[old.pose(x=(i%3)*(.55 if alias=='carton' else .27),y=(i//3)*(.4 if alias=='carton' else .21)) for i in range(6)] if alias!='sealing_tape' else [old.pose()]
                if alias=='fastener':offsets=[old.pose(x=(i%6)*.014,y=(i//6)*.014) for i in range(24)]
            else:
                targets={'out-ASM01':(0,0,.805),'out-R01':(2.65,0,.805),'out-CV01':(3.95,0,.805),'out-V01':(6.08,0,.805),
                   'out-RW01':(5.6,2.6,.805),'out-CV02':(8.55,0,.805),'out-PK01':(10.3,0,.798),'out-CASE01':(13.35,0,.798),
                   'rework-in':(5.6,1.3,.805),'quarantine':(6,4,0)}
                d['position']=old.pose(*targets[name])
                offsets=[old.pose(x=x,y=y) for x,y in [(0,0),(-.30,0),(.30,0),(0,.23),(-.30,.23),(.30,.23)]]
            d['item_layout']=dict(assignment='item_id_order',slots=[slot(alias,pose,None) for pose in offsets])
        if name=='PAL01-store':
            d['item_layout']=dict(assignment='item_id_order',slots=[slot('package',old.pose(x=x,y=y,z=.1448+k*.36),None)
                 for k in range(3) for y in [-.18,.18] for x in [-.255,.255]])
        if name=='quarantine':d['capacity']=old.quantity(1)
for command in plan['business_commands']:
    obj=command['data']
    if obj['kind']=='route':
        # Physical material availability drives this original continuous line.
        # A robot may deliver before its return phase completes.
        for step in obj['data']['steps']:
            step['predecessors']=['step-CV01'] if step['device_id']=='V01' else []
for d in plan['definitions']:
    if d['payload']['category']=='operations':
        op=d['payload']['data']
        if op['builtin'] not in ['inspect_and_route','rework']:
            for row in op['inputs']:row['required_quality']=['released','unchecked']
plan['scenario']['issues']=[]
plan['scenario']['id']='L03-SOURCE'
plan['scenario']['title']='Original L03 six-package line with exact0.3 media and declared phase material flow'
plan['requested_times_ms']=sorted(set(plan['requested_times_ms']+[47999,48999,82999,84000,87999,88000,89999,203000,213000]))
# Original station poses stay in source-Line coordinates; place that complete
# line inside the canonical nonnegative Hall and map Locations explicitly.
for command in plan['pre_commands']:
    if command['kind']!='put_object':continue
    obj=command['data']
    if obj['kind']=='line':obj['data']['transform']=old.pose(2,4)
    elif obj['kind']=='location':
        position=obj['data']['position']['translation'];position['x']+=2;position['y']+=4
plan['source_to_hall_mapping']=dict(translation_m=dict(x=2,y=4,z=0),original_device_line_local_poses_preserved=True)
# Referenced ItemTypes must exist before locations with exact typed display slots.
item_commands=[c for c in plan['create_commands'] if any(o['kind']=='item_type' for o in c['data']['new_instances'])]
plan['create_commands']=[c for c in plan['create_commands'] if c not in item_commands]
plan['pre_commands']=item_commands+plan['pre_commands']
write('command-plan.json',plan)

# Compile-only / Engine tests take an ordinary DTO, not an archive manifest.
source=BASE/'execution-1792171766822967000/actual-project-before.json'
project=read(source)['data']
project['id']='l03-canonical-test';project['revision']=1;project['scenario_refs']=[plan['scenario']['id']]
fields=dict(item_type='item_types',location='locations',hall='halls',line='lines',device='devices',resource='resources',calendar='calendars',product='products',bom='boms',route='routes',connection='connections')
for value in fields.values():project[value]=[]
project['module_refs']=[dict(id=e['id'],version=e['version']) for e in plan['register_order']]+[dict(id=d['id'],version=d['version']) for d in plan['definitions']]
for key in ['pre_commands','create_commands','business_commands']:
    for c in plan[key]:
        objects=[c['data']] if c['kind']=='put_object' else c['data'].get('new_instances',[])
        for o in objects:
            if o['kind'] not in fields:continue
            obj=deepcopy(o['data'])
            if o['kind']=='item_type':
                definition=defs[obj['module_ref']['id']]['payload']['data']
                for attr in ['unit','tracking','spec_ref','attributes','attribute_schema']:obj[attr]=deepcopy(definition[attr])
                # contents_layout is already an actual public create-instance command field.
            if o['kind']=='device':obj['presentation_override']=media(old.FAMILIES[obj['id']])
            project[fields[o['kind']]].append(obj)
scenario=deepcopy(plan['scenario']);scenario['project_revision']=1
definitions=deepcopy(plan['definitions'])+[read(ROOT/e['directory']/'module.json')['data']['definition'] for e in plan['register_order']]
write('engine-input.json',dict(project=project,scenario=scenario,definitions=definitions))
types='../../../packages/commands/generated/typescript/'
contract=f"import type {{EditCommand}} from '{types}EditCommand';\nimport type {{ModuleDefinition}} from '{types}ModuleDefinition';\nimport type {{ScenarioInput}} from '{types}ScenarioInput';\nimport type {{ProjectState}} from '{types}ProjectState';\n"
for key,typ in [('definitions','ModuleDefinition[]'),('pre_commands','EditCommand[]'),('create_commands','EditCommand[]'),('formal_media_commands','EditCommand[]'),('media_commands','EditCommand[]'),('business_commands','EditCommand[]'),('scenario','ScenarioInput')]:
    contract+='const '+key+' = '+json.dumps(plan[key],ensure_ascii=False)+f' satisfies {typ};\n'
contract+='const engine_project = '+json.dumps(project,ensure_ascii=False)+' satisfies ProjectState;\n'
(HERE/'contract-check.ts').write_text(contract,encoding='utf-8')
print('New exact0.3 command plan and separate Engine fixture; no saved project/Run generated.')
