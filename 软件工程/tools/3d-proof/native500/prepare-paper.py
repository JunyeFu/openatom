from pathlib import Path
import json,copy,sys
ROOT=Path(__file__).resolve().parents[3];HOME=ROOT/(sys.argv[1] if len(sys.argv)>1 else 'modules/media/native-equipment-500/reviews/B02-Q01');OUT=ROOT/(sys.argv[2] if len(sys.argv)>2 else '.work/native500-quality-20261025')
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
base=read(ROOT/'packages/commands/tests/fixtures/production/local-assembly-pack.json');project=copy.deepcopy(base['project']);scenario=copy.deepcopy(base['scenario'])
for k in ['item_types','locations','halls','lines','devices','resources','calendars','products','boms','routes','connections','module_refs','artifact_refs','acknowledgments','bindings','sources','issues','template_adoptions','setup_policies']:project[k]=[]
project['title']=HOME.name+' 纸包装五设备 · 五类真实运行验收';project['scenario_refs']=['PAPER-QA'];project['generation_context']=None
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
for e in read(HOME/'items/registration.json')['register_order']:
 if e['category']=='items':project['item_types'].append({'id':e['item_type'],'revision':'r1','title':e['item_type'],'unit':'piece','tracking':'individual','spec_ref':None,'attributes':{},'attribute_schema':[]})
def location(id,hall,pos,types,cap=1):
 l=copy.deepcopy(base['project']['locations'][0]);l.update(id=id,title=id,hall_id=hall,position=pos,capacity={'value':{'coefficient':cap,'decimal_places':0},'unit':'piece'},accepted_item_types=types,purpose='buffer',item_layout={'assignment':'item_id_order','slots':[slot(t)['slots'][0] for t in types]});project['locations'].append(l)
def connect(id,source,dest,item,store):project['connections'].append({'kind':'material','data':{'id':id,'source':source,'destination':dest,'item_type':item,'unit':'piece','quantity_rule':None,'destination_store':store,'capacity_rule':None}})
def port(device,module,name):return {'kind':'port','data':{'module_ref':module,'instance_id':device,'port_id':name}}
def item(id,type,loc):return {'id':id,'item_type_ref':{'id':type,'revision':'r1'},'quantity':q(),'holder':{'kind':'location','id':loc},'quality':'released','components':[],'contents':[],'source':'user_provided'}
for number,spec in enumerate(equipment):
 key=spec['id'];canonical=read(HOME/'devices'/key/'module.json')['data']['definition'];stub=copy.deepcopy(canonical);stub.update(id='qa.seed.'+key,version='0.1.0');stub['payload']['data']['presentation']=None;definitions.append(stub)
 raw=key+'-input';product=raw if spec['rule']=='preserve' else key+'-output';operation=canonical['payload']['data']['behavior']['inline'];load=operation['phases'][0]['duration']['value'];auto=operation['phases'][1]['duration']['value']
 surface=1.0475 if key=='pile-turner' else .8775 if key=='paper-jogger' else .8425 if key=='air-table-loader' else .845
 for ci,case in enumerate(cases):
  device=key+'--'+case;hall='hall-'+device;route='route-'+device;module=ref(stub['id']);power='power-'+device;machine='machine-'+device;operator='operator-'+device
  h=copy.deepcopy(base['project']['halls'][0]);h.update(id=hall,title=spec['title']+' / '+case,width_m=8,depth_m=8,height_m=4,origin=pose((number*5+ci)*10),input_locations=[device+'-in'],output_locations=[device+'-out']);project['halls'].append(h)
  cal='ALL'
  if case=='phase-wait':cal='cal-'+device;calendar(cal,[(load+5000,60000)])
  if case=='interrupted':cal='cal-'+device;cut=load+int(auto*.4);calendar(cal,[(0,cut),(cut+5000,60000)])
  resource(machine,'fixture','machine','slot');resource(operator,'worker','operator','person');resource(power,'fixture','auto_fixture','slot',cal)
  d=copy.deepcopy(base['project']['devices'][0]);d.update(id=device,title=spec['title']+' · '+case,hall_id=hall,module_ref=module,pose=pose(3,3),footprint_m=[{'x':-1.55,'y':-.66},{'x':1.55,'y':-.66},{'x':1.55,'y':.66},{'x':-1.55,'y':.66}],height_m=2,configuration={},role_bindings={'item_types':{'input':raw,'output':product},'resources':{'execution_capacity':machine,'operator':operator,'auto_fixture':power},'targets':{},'ports':{}},operation_binding=None,item_layout={'assignment':'item_id_order','slots':[slot(t,'cargo')['slots'][0] for t in dict.fromkeys([raw,product])]})
  project['setup_policies'].append({'device_id':device,'initial_family':operation['family'],'changeovers':[],'first_article':None})
  for name in ['input','output']:d['role_bindings']['ports'][name]=port(device,module,name)['data']
  project['devices'].append(d);scenario['scope'].append({'kind':'device','id':device})
  location(device+'-in',hall,pose(1.95,3,surface),[raw]);location(device+'-out',hall,pose(4.05,3,surface),[product,'blocker'])
  connect(device+'-feed',{'kind':'location','data':device+'-in'},port(device,module,'input'),raw,device+'-in');connect(device+'-deliver',port(device,module,'output'),{'kind':'location','data':device+'-out'},product,device+'-out')
  project['routes'].append({'id':route,'revision':'r1','steps':[{'id':'process','operation_ref':module,'device_id':device,'predecessors':[],'item_bindings':{'input':raw,'output':product},'resource_bindings':{'operator':operator,'auto_fixture':power}}]})
  scenario['production_tasks'].append({'id':'task-'+device,'routing_ref':{'id':route,'revision':'r1'},'order_allocations':[],'planned_quantity':q(),'release_at_ms':0,'priority':0})
  rawitem=item('paper-'+device,raw,device+'-in')
  if case=='missing':scenario['arrivals'].append({'id':'arrival-'+device,'at_ms':8000,'destination':device+'-in','items':[rawitem],'release_policy':'unchecked_allowed'})
  else:scenario['initial_conditions']['items'].append(rawitem)
  if case=='output-wait':scenario['initial_conditions']['items'].append(item('blocker-'+device,'blocker',device+'-out'))
sink=copy.deepcopy(read(HOME/'devices/paper-jogger/module.json')['data']['definition']);sink.update(id='qa.output-drainer',version='0.1.0');sink['payload']['data']['presentation']=None
op=sink['payload']['data']['behavior']['inline'];op.update(builtin='transform',identity_policy='transform_with_origin',family='qa-output-clear');op.pop('material_flow',None)
for ph,ms in zip(op['phases'],[500,1000,500]):ph['duration']['value']=ms;ph['phase_resources']=[]
definitions.append(sink)
for spec in equipment:
 origin=spec['id']+'--output-wait';hall='hall-'+origin;device=origin+'-drainer';module=ref('qa.output-drainer');machine='machine-'+device;resource(machine,'fixture','machine','slot')
 d=copy.deepcopy(project['devices'][0]);d.update(id=device,title='验收输出缓冲清空器（不计新增类型）',hall_id=hall,pose=pose(3,6),module_ref=module,role_bindings={'item_types':{'input':'blocker','output':'blocker'},'resources':{'execution_capacity':machine},'targets':{},'ports':{}},item_layout=slot('blocker','cargo'),presentation_override=None)
 for name in ['input','output']:d['role_bindings']['ports'][name]=port(device,module,name)['data']
 project['devices'].append(d);scenario['scope'].append({'kind':'device','id':device});destination=device+'-out';location(destination,hall,pose(4.05,6,.8775),['blocker'],10)
 project['setup_policies'].append({'device_id':device,'initial_family':'qa-output-clear','changeovers':[],'first_article':None})
 connect(device+'-feed',{'kind':'location','data':origin+'-out'},port(device,module,'input'),'blocker',origin+'-out');connect(device+'-deliver',port(device,module,'output'),{'kind':'location','data':destination},'blocker',destination)
 route='route-'+device;project['routes'].append({'id':route,'revision':'r1','steps':[{'id':'clear','operation_ref':module,'device_id':device,'predecessors':[],'item_bindings':{'input':'blocker','output':'blocker'},'resource_bindings':{}}]})
 scenario['production_tasks'].append({'id':'task-'+device,'routing_ref':{'id':route,'revision':'r1'},'order_allocations':[],'planned_quantity':q(),'release_at_ms':30000,'priority':0})
project['module_refs']=[ref(d['id']) for d in definitions]
OUT.mkdir(exist_ok=True,parents=True);write(OUT/'paper-input.json',{'project':project,'scenario':scenario,'definitions':definitions})
if '--input-only' not in sys.argv[3:]:
 write(HOME/'quality-review.json',{'review_batch':HOME.name,'status':'in_progress','device_ids':[e['id'] for e in equipment],'target_device_count':500,'candidate_device_count':500,'formal_supported_device_count':0,'visual':'pending','Native':'pending','actual_input':(OUT/'paper-input.json').relative_to(ROOT).as_posix()})
print('Prepared explicit paper QA:25 reviewed-device instances,5 explicit buffer-draining fixtures,30 tasks,five conditions with recovery.')
