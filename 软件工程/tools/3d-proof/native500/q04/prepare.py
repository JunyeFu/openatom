from pathlib import Path
import json,copy,sys
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/(sys.argv[1] if len(sys.argv)>1 else 'modules/media/native-equipment-500/reviews/B02-Q04');OUT=ROOT/(sys.argv[2] if len(sys.argv)>2 else '.work/native500-quality-20261025/q04')
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
base=read(ROOT/'packages/commands/tests/fixtures/production/local-assembly-pack.json');project=copy.deepcopy(base['project']);scenario=copy.deepcopy(base['scenario'])
for k in ['item_types','locations','halls','lines','devices','resources','calendars','products','boms','routes','connections','module_refs','artifact_refs','acknowledgments','bindings','sources','issues','template_adoptions','setup_policies']:project[k]=[]
project['title']=HOME.name+' 五设备 · 整饰、耗材与阶段等待真实运行验收';project['scenario_refs']=['PAPER-QA'];project['generation_context']=None
scenario.update(id='PAPER-QA',title=project['title'],window={'start_ms':0,'end_ms':60000},orders=[],production_tasks=[],arrivals=[],supplies=[],scope=[],parameter_overrides=[],quality_presets=[])
scenario['initial_conditions']={'items':[],'initial_setup_families':{},'declared_quality_decisions':[],'material_allocations':[]}
definitions=[];cases=['normal','missing','phase-wait','interrupted','output-wait'];equipment=read(HOME/'catalog.json')['equipment']
q=lambda unit='piece':{'value':{'coefficient':1,'decimal_places':0},'unit':unit}
ref=lambda id,version='0.1.0':{'id':id,'version':version}
pose=lambda x=0,y=0,z=0:{'translation':dict(x=x,y=y,z=z),'rotation':dict(x=0,y=0,z=0,w=1),'scale':dict(x=1,y=1,z=1),'unit':'meter','coordinate_frame':'right_handed_z_up'}
slot=lambda item,anchor=None:{'assignment':'item_id_order','slots':[{'item_type_ref':{'id':item,'revision':'r1'},'anchor_role':anchor,'offset':pose()}]}
def calendar(id,intervals):project['calendars'].append({'id':id,'title':id,'intervals':[dict(start_ms=a,end_ms=b) for a,b in intervals],'source':'user_provided'})
calendar('ALL',[(0,60000)])
def definition(id,category,data):
 d=copy.deepcopy(base['definitions'][0]);d.update(id=id,title=id,version='0.1.0',parameters=[],attribute_schema=[]);d['contribution']={'author':'Super Factory QA','license':'CC0-1.0','assets':[]};d['payload']={'category':category,'data':data};definitions.append(d);return d
for name,kind,unit in [('machine','fixture','slot'),('operator','worker','person'),('auto_fixture','fixture','slot')]:definition('qa.'+name,'resources',{'kind':kind,'role':'execution_capacity' if name=='machine' else name,'capacity':q(unit),'qualifications':[],'default_calendar_ms':[dict(start_ms=0,end_ms=60000)],'presentation':None})
def resource(id,kind,role,unit,cal='ALL'):
 project['resources'].append({'id':id,'definition_ref':ref('qa.'+role),'title':id,'kind':kind,'role':'execution_capacity' if role=='machine' else role,'capacity':q(unit),'qualifications':[],'calendar_id':cal,'home_location':None,'presentation':None})
item_metadata=read(HOME/'items/inventory.json')['metadata']
for e in item_metadata:
 project['item_types'].append({'id':e['id'],'revision':'r1','title':e['id'],'unit':'piece','tracking':e['tracking'],'spec_ref':None,'attributes':e['attributes'],'attribute_schema':e['attribute_schema']})
item_by_id={e['id']:e for e in item_metadata}
media_spec={e['id']:e for e in read(HOME/'catalog.json')['equipment']}
# Coordinates below match declared editable machine/item sources, in meters.
geometry=read(HOME/'geometry.json')

def yup_offset(v):return pose(v[0],-v[2],v[1])
def at_port(point):return pose(3+point[0],3-point[2],point[1])

def location(id,hall,pos,types,cap=1):
 l=copy.deepcopy(base['project']['locations'][0]);l.update(id=id,title=id,hall_id=hall,position=pos,capacity={'value':{'coefficient':cap,'decimal_places':0},'unit':'piece'},accepted_item_types=types,purpose='buffer',item_layout={'assignment':'item_id_order','slots':[slot(t)['slots'][0] for t in types]});project['locations'].append(l)
def connect(id,source,dest,item,store):project['connections'].append({'kind':'material','data':{'id':id,'source':source,'destination':dest,'item_type':item,'unit':'piece','quantity_rule':None,'destination_store':store,'capacity_rule':None}})
def port(device,module,name):return {'kind':'port','data':{'module_ref':module,'instance_id':device,'port_id':name}}
def item(id,type,loc):return {'id':id,'item_type_ref':{'id':type,'revision':'r1'},'quantity':q(),'holder':{'kind':'location','id':loc},'quality':'released','components':[],'contents':[],'source':'user_provided'}
drainers=[]
def layout_slot(type,anchor=None,offset=(0,0,0)):
 return {'item_type_ref':{'id':type,'revision':'r1'},'anchor_role':anchor,'offset':yup_offset(offset)}
for number,spec in enumerate(equipment):
 key=spec['id'];canonical=read(HOME/'devices'/key/'module.json')['data']['definition'];stub=copy.deepcopy(canonical);stub.update(id='qa.seed.'+key,version='0.1.0');stub['payload']['data']['presentation']=None;definitions.append(stub)
 op=canonical['payload']['data']['behavior']['inline'];load=op['phases'][0]['duration']['value'];auto=op['phases'][1]['duration']['value'];g=geometry[key]
 outputs={o['role']:key+'-'+o['role'] for o in op['outputs']};outputs['output']=key+'-input' if spec['rule']=='preserve' else key+'-output'
 bindings={'input':key+'-input',**outputs}
 if spec['auxiliary']:bindings['auxiliary']=key+'-auxiliary'
 extras=[o for o in op['outputs'] if o['role']!='output'];scrap_count=sum(o['quantity']['value']['coefficient'] for o in extras)
 for ci,case in enumerate(cases+(['backing-wait'] if extras else [])+(['missing-auxiliary'] if spec['auxiliary'] else [])):
  device=key+'--'+case;hall='hall-'+device;route='route-'+device;module=ref(stub['id']);power='power-'+device;machine='machine-'+device;operator='operator-'+device
  h=copy.deepcopy(base['project']['halls'][0]);h.update(id=hall,title=spec['title']+' / '+case,width_m=8,depth_m=8,height_m=4,origin=pose((number*8+ci)*10),input_locations=[device+'-in'],output_locations=[device+'-out']);project['halls'].append(h)
  cal='ALL'
  if case=='phase-wait':cal='cal-'+device;calendar(cal,[(load+5000,60000)])
  if case=='interrupted':cal='cal-'+device;cut=load+int(auto*.4);calendar(cal,[(0,cut),(cut+5000,60000)])
  resource(machine,'fixture','machine','slot');resource(operator,'worker','operator','person');resource(power,'fixture','auto_fixture','slot',cal)
  slots=[layout_slot(bindings['input'],'cargo')]
  if spec['rule']!='preserve':slots.append(layout_slot(bindings['output'],'product_staging'))
  if spec['auxiliary']:slots.append(layout_slot(bindings['auxiliary'],'aux_seat'))
  for out in extras:slots.append(layout_slot(bindings[out['role']],'backing_staging'))
  d=copy.deepcopy(base['project']['devices'][0]);d.update(id=device,title=spec['title']+' · '+case,hall_id=hall,module_ref=module,pose=pose(3,3),footprint_m=[{'x':x,'y':y} for x,y in [(-g['width']/2,-1.05),(g['width']/2,-1.05),(g['width']/2,1.05),(-g['width']/2,1.05)]],height_m=2,configuration={},role_bindings={'item_types':bindings,'resources':{'execution_capacity':machine,'operator':operator,'auto_fixture':power},'targets':{},'ports':{}},operation_binding=None,item_layout={'assignment':'item_id_order','slots':slots})
  for role in bindings:d['role_bindings']['ports'][role]=port(device,module,role)['data']
  project['setup_policies'].append({'device_id':device,'initial_family':op['family'],'changeovers':[],'first_article':None})
  line='line-'+device;d['line_id']=line;project['lines'].append({'id':line,'title':spec['title']+' / '+case,'hall_id':hall,'transform':pose(),'members':[device],'boundary_ports':[],'region_polygon_m':[{'x':.7,'y':1.8},{'x':5.3,'y':1.8},{'x':5.3,'y':4.2},{'x':.7,'y':4.2}]});project['devices'].append(d);scenario['scope'].append({'kind':'device','id':device})
  location(device+'-in',hall,at_port(g['input']),[bindings['input']]);location(device+'-out',hall,at_port(g['output']),[bindings['output'],'blocker'])
  for role in bindings:
   if role=='input':connect(device+'-feed-input',{'kind':'location','data':device+'-in'},port(device,module,role),bindings[role],device+'-in')
   elif role=='auxiliary':
    loc=device+'-auxiliary';location(loc,hall,at_port(g['auxiliary']),[bindings[role]]);connect(device+'-feed-auxiliary',{'kind':'location','data':loc},port(device,module,role),bindings[role],loc)
   else:connect(device+'-deliver-'+role,port(device,module,role),{'kind':'location','data':device+('-out' if role=='output' else '-backing')},bindings[role],device+('-out' if role=='output' else '-backing'))
  if extras:
   location(device+'-backing',hall,at_port(g['backing']),[bindings[e['role']] for e in extras]+['blocker'],scrap_count)
   project['locations'][-1]['item_layout']['slots']=[layout_slot(bindings[out['role']]) for out in extras]+[layout_slot('blocker')]
  project['routes'].append({'id':route,'revision':'r1','steps':[{'id':'process','operation_ref':module,'device_id':device,'predecessors':[],'item_bindings':bindings,'resource_bindings':{'operator':operator,'auto_fixture':power}}]})
  scenario['production_tasks'].append({'id':'task-'+device,'routing_ref':{'id':route,'revision':'r1'},'order_allocations':[],'planned_quantity':q(),'release_at_ms':0,'priority':0})
  for role in ['input']+(['auxiliary'] if spec['auxiliary'] else []):
   loc=device+('-in' if role=='input' else '-auxiliary');physical=item(('paper-' if role=='input' else 'auxiliary-')+device,bindings[role],loc)
   if case==('missing' if role=='input' else 'missing-auxiliary'):scenario['arrivals'].append({'id':'arrival-'+role+'-'+device,'at_ms':8000,'destination':loc,'items':[physical],'release_policy':'unchecked_allowed'})
   else:scenario['initial_conditions']['items'].append(physical)
  if case in ['output-wait','backing-wait']:
   loc=device+('-out' if case=='output-wait' else '-backing');scenario['initial_conditions']['items'].append(item('blocker-'+device,'blocker',loc));drainers.append((device,hall,loc))
sink=copy.deepcopy(read(HOME/'devices/blind-embosser/module.json')['data']['definition']);sink.update(id='qa.output-drainer',version='0.1.0');sink['payload']['data']['presentation']=None
op=sink['payload']['data']['behavior']['inline'];op.update(builtin='transform',identity_policy='transform_with_origin',family='qa-output-clear');op.pop('material_flow',None)
for ph,ms in zip(op['phases'],[500,1000,500]):ph['duration']['value']=ms;ph['phase_resources']=[]
definitions.append(sink)
for origin,hall,source in drainers:
 device=origin+'-drainer';module=ref('qa.output-drainer');machine='machine-'+device;resource(machine,'fixture','machine','slot')
 d=copy.deepcopy(project['devices'][0]);d.update(id=device,title='验收缓冲清空器（不计设备类型）',hall_id=hall,pose=pose(3,6),module_ref=module,role_bindings={'item_types':{'input':'blocker','output':'blocker'},'resources':{'execution_capacity':machine},'targets':{},'ports':{}},item_layout=None,presentation_override=None,line_id=None)
 for name in ['input','output']:d['role_bindings']['ports'][name]=port(device,module,name)['data']
 project['devices'].append(d);scenario['scope'].append({'kind':'device','id':device});destination=device+'-out';location(destination,hall,pose(4.05,6,0),['blocker'],10)
 project['setup_policies'].append({'device_id':device,'initial_family':'qa-output-clear','changeovers':[],'first_article':None})
 connect(device+'-feed',{'kind':'location','data':source},port(device,module,'input'),'blocker',source);connect(device+'-deliver',port(device,module,'output'),{'kind':'location','data':destination},'blocker',destination)
 route='route-'+device;project['routes'].append({'id':route,'revision':'r1','steps':[{'id':'clear','operation_ref':module,'device_id':device,'predecessors':[],'item_bindings':{'input':'blocker','output':'blocker'},'resource_bindings':{}}]})
 scenario['production_tasks'].append({'id':'task-'+device,'routing_ref':{'id':route,'revision':'r1'},'order_allocations':[],'planned_quantity':q(),'release_at_ms':30000,'priority':0})
project['module_refs']=[ref(d['id']) for d in definitions]
OUT.mkdir(exist_ok=True,parents=True);write(OUT/'paper-input.json',{'project':project,'scenario':scenario,'definitions':definitions})
print(f'Prepared {len(scenario["production_tasks"])} tasks, including {len(drainers)} explicit buffer drainers; not counted.')
