from pathlib import Path
import json,copy,sys
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/(sys.argv[1] if len(sys.argv)>1 else 'modules/media/native-equipment-500/reviews/B01-Q18');OUT=ROOT/(sys.argv[2] if len(sys.argv)>2 else '.work/native500-quality-20261025/b01q18')
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
base=read(ROOT/'packages/commands/tests/fixtures/production/local-assembly-pack.json');project=copy.deepcopy(base['project']);scenario=copy.deepcopy(base['scenario'])
for k in ['item_types','locations','halls','lines','devices','resources','calendars','products','boms','routes','connections','module_refs','artifact_refs','acknowledgments','bindings','sources','issues','template_adoptions','setup_policies']:project[k]=[]
project['title']=HOME.name+' 纸箱检验封箱贴标及真实内装物验收';project['scenario_refs']=['PAPER-QA'];project['generation_context']=None
scenario.update(id='PAPER-QA',title=project['title'],window={'start_ms':0,'end_ms':90000},orders=[],production_tasks=[],arrivals=[],supplies=[],scope=[],parameter_overrides=[],quality_presets=[])
scenario['initial_conditions']={'items':[],'initial_setup_families':{},'declared_quality_decisions':[],'material_allocations':[]}
definitions=[];cases=['normal','missing','phase-wait','interrupted','output-wait'];equipment=read(HOME/'catalog.json')['equipment']
q=lambda unit='piece':{'value':{'coefficient':1,'decimal_places':0},'unit':unit}
ref=lambda id,version='0.1.0':{'id':id,'version':version}
pose=lambda x=0,y=0,z=0:{'translation':dict(x=x,y=y,z=z),'rotation':dict(x=0,y=0,z=0,w=1),'scale':dict(x=1,y=1,z=1),'unit':'meter','coordinate_frame':'right_handed_z_up'}
slot=lambda item,anchor=None:{'assignment':'item_id_order','slots':[{'item_type_ref':{'id':item,'revision':'r1'},'anchor_role':anchor,'offset':pose()}]}
def calendar(id,intervals):project['calendars'].append({'id':id,'title':id,'intervals':[dict(start_ms=a,end_ms=b) for a,b in intervals],'source':'user_provided'})
calendar('ALL',[(0,90000)])
def definition(id,category,data):
 d=copy.deepcopy(base['definitions'][0]);d.update(id=id,title=id,version='0.1.0',parameters=[],attribute_schema=[]);d['contribution']={'author':'Super Factory QA','license':'CC0-1.0','assets':[]};d['payload']={'category':category,'data':data};definitions.append(d);return d
for name,kind,unit in [('machine','fixture','slot'),('operator','worker','person'),('auto_fixture','fixture','slot')]:definition('qa.'+name,'resources',{'kind':kind,'role':'execution_capacity' if name=='machine' else name,'capacity':q(unit),'qualifications':[],'default_calendar_ms':[dict(start_ms=0,end_ms=90000)],'presentation':None})
def resource(id,kind,role,unit,cal='ALL'):
 project['resources'].append({'id':id,'definition_ref':ref('qa.'+role),'title':id,'kind':kind,'role':'execution_capacity' if role=='machine' else role,'capacity':q(unit),'qualifications':[],'calendar_id':cal,'home_location':None,'presentation':None})
item_metadata=read(HOME/'items/inventory.json')['metadata']
for e in item_metadata:
 project['item_types'].append({'id':e['id'],'revision':'r1','title':e['id'],'unit':'piece','tracking':e['tracking'],'spec_ref':None,'attributes':e['attributes'],'attribute_schema':e['attribute_schema'],'contents_layout':{'assignment':'item_id_order','slots':[{'item_type_ref':{'id':'content-unit','revision':'r1'},'anchor_role':'ContentSeat_'+str(i),'offset':pose()} for i in range(6)]} if e['role']=='input' else None})
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
# Independent cases use one actual package plus six container-held RuntimeItems.
for number,spec in enumerate(equipment):
 key=spec['id'];canonical=read(HOME/'devices'/key/'module.json')['data']['definition'];stub=copy.deepcopy(canonical);stub.update(id='qa.seed.'+key,version='0.1.0');stub['payload']['data']['presentation']=None;definitions.append(stub)
 op=canonical['payload']['data']['behavior']['inline'];load=op['phases'][0]['duration']['value'];auto=op['phases'][1]['duration']['value'];g=geometry[key];bindings={r['role']:key+'-input' for r in op['inputs']+op['outputs']}
 if spec['auxiliary']:bindings['auxiliary']=key+'-auxiliary'
 for ci,case in enumerate(cases+(['missing-auxiliary'] if spec['auxiliary'] else [])+spec.get('acceptance_conditions',[])):
  device=key+'--'+case;hall='hall-'+device;route='route-'+device;module=ref(stub['id']);power='power-'+device;machine='machine-'+device;operator='operator-'+device
  h=copy.deepcopy(base['project']['halls'][0]);h.update(id=hall,title=spec['title']+' / '+case,width_m=8,depth_m=8,height_m=4,origin=pose((number*10+ci)*10),input_locations=[device+'-in'],output_locations=[device+'-out']);project['halls'].append(h)
  cal='ALL'
  if case=='phase-wait':cal='cal-'+device;calendar(cal,[(load+5000,90000)])
  if case=='interrupted':cal='cal-'+device;cut=load+int(auto*.4);calendar(cal,[(0,cut),(cut+5000,90000)])
  resource(machine,'fixture','machine','slot');resource(operator,'worker','operator','person');resource(power,'fixture','auto_fixture','slot',cal)
  d=copy.deepcopy(base['project']['devices'][0]);d.update(id=device,title=spec['title']+' / '+case,hall_id=hall,module_ref=module,pose=pose(3,3),footprint_m=[{'x':x,'y':y}for x,y in[(-1.8,-1.25),(1.8,-1.25),(1.8,1.25),(-1.8,1.25)]],height_m=2.15,configuration={},role_bindings={'item_types':bindings.copy(),'resources':{'execution_capacity':machine,'operator':operator,'auto_fixture':power},'targets':{'isolation':{'kind':'location','id':device+'-isolation'}}if spec['rule']=='inspect'else{},'ports':{}},operation_binding=None,item_layout={'assignment':'item_id_order','slots':[layout_slot(bindings['input'],'cargo')]+([layout_slot(bindings['auxiliary'],'aux_seat')]if spec['auxiliary']else[])})
  for role in bindings:d['role_bindings']['ports'][role]=port(device,module,role)['data']
  project['setup_policies'].append({'device_id':device,'initial_family':op['family'],'changeovers':[],'first_article':None});line='line-'+device;d['line_id']=line;project['lines'].append({'id':line,'title':d['title'],'hall_id':hall,'transform':pose(),'members':[device],'boundary_ports':[],'region_polygon_m':[{'x':.7,'y':1.15},{'x':5.3,'y':1.15},{'x':5.3,'y':4.85},{'x':.7,'y':4.85}]});project['devices'].append(d);scenario['scope'].append({'kind':'device','id':device})
  location(device+'-in',hall,at_port(g['input']),[bindings['input']]);location(device+'-out',hall,at_port(g['output']),[bindings['output'],'blocker'])
  if spec['auxiliary']:location(device+'-auxiliary',hall,at_port(g['auxiliary']),[bindings['auxiliary']])
  if spec['rule']=='inspect':
   location(device+'-rejected',hall,at_port(g['rejected']),[bindings['input'],'blocker']);location(device+'-isolation',hall,at_port(g['isolation']),[bindings['input'],'blocker']);project['locations'][-1].update(purpose='isolation',ordinary_issue_enabled=False)
  for role in bindings:
   loc=device+({'input':'-in','auxiliary':'-auxiliary','output':'-out'}.get(role,'-'+role))
   if role in ['input','auxiliary']:connect(device+'-feed-'+role,{'kind':'location','data':loc},port(device,module,role),bindings[role],loc)
   else:connect(device+'-deliver-'+role,port(device,module,role),{'kind':'location','data':loc},bindings[role],loc)
  steps=[{'id':'process','operation_ref':module,'device_id':device,'predecessors':[],'item_bindings':bindings.copy(),'resource_bindings':{'operator':operator,'auto_fixture':power}}]
  if case in ['rework','rejected','reject-wait']:
   rid=device+'-rework';rm=ref('qa.rework');capacity='machine-'+rid;resource(capacity,'fixture','machine','slot');rd=copy.deepcopy(d);rd.update(id=rid,title='明确返工夹具（非新增模型）',pose=pose(3,5.5),module_ref=rm,line_id=None,presentation_override=None,item_layout=None,role_bindings={'item_types':{'product':bindings['input']},'resources':{'execution_capacity':capacity},'targets':{},'ports':{name:port(rid,rm,name)['data']for name in ['input','output']}});project['devices'].append(rd);scenario['scope'].append({'kind':'device','id':rid});project['setup_policies'].append({'device_id':rid,'initial_family':'qa-rework','changeovers':[],'first_article':None});connect(rid+'-feed',{'kind':'location','data':device+'-rejected'},port(rid,rm,'input'),bindings['input'],device+'-rejected');connect(rid+'-deliver',port(rid,rm,'output'),{'kind':'location','data':device+'-in'},bindings['input'],device+'-in');steps.append({'id':'rework','operation_ref':rm,'device_id':rid,'predecessors':['process'],'item_bindings':{'product':bindings['input']},'resource_bindings':{}})
  project['routes'].append({'id':route,'revision':'r1','steps':steps});scenario['production_tasks'].append({'id':'task-'+device,'routing_ref':{'id':route,'revision':'r1'},'order_allocations':[],'planned_quantity':q(),'release_at_ms':0,'priority':0})
  for role in ['input']+(['auxiliary']if spec['auxiliary']else[]):
   loc=device+('-in'if role=='input'else'-auxiliary');physical=item(('paper-'if role=='input'else'auxiliary-')+device,bindings[role],loc);children=[]
   if role=='input':
    physical['quality']='unchecked'if spec['rule']=='inspect'else'released';physical['contents']=['unit-'+str(i)+'-'+device for i in range(6)]
    for uid in physical['contents']:
     child=item(uid,'content-unit',loc);child['holder']={'kind':'container','id':physical['id']};children.append(child)
   if case==('missing'if role=='input'else'missing-auxiliary'):scenario['arrivals'].append({'id':'arrival-'+role+'-'+device,'at_ms':8000,'destination':loc,'items':[physical,*children],'release_policy':'unchecked_allowed'})
   else:scenario['initial_conditions']['items'].extend([physical,*children])
  if spec['rule']=='inspect':
   results=[]if case=='pending-recovery'else['FAIL','PASS']if case in ['rework','reject-wait']else['FAIL','FAIL']if case=='rejected'else['FAIL']if case=='reject-wait'else['PASS'];inspection=ref('qa.seed.'+key);scenario['quality_presets'].append({'id':'preset-'+device,'item_or_ancestor':'paper-'+device,'match_kind':'ancestor','inspection_module':inspection,'results':[{'kind':'string','value':r}for r in results]})
   if case=='pending-recovery':scenario['initial_conditions']['declared_quality_decisions'].append({'id':'explicit-release-'+device,'item_ref':'paper-'+device,'inspection_ref':inspection,'source_kind':'manual','observed_result':{'kind':'string','value':'PASS'},'disposition':'release','effective_at_ms':25000,'decision_source':{'kind':'human_statement','data':'QA scenario explicitly declares a manual PASS release at 25000ms after pending inspection; not a human acceptance of the model.'}})
  if case in ['output-wait','reject-wait']:
   loc=device+('-out'if case=='output-wait'else'-rejected');scenario['initial_conditions']['items'].append(item('blocker-'+device,'blocker',loc));drainers.append((device,hall,loc))
# One explicit rework fixture definition, using the existing production rework contract.
qa=copy.deepcopy(read(HOME/'devices/vision-carton-inspector/module.json')['data']['definition']);qa.update(id='qa.rework',version='0.1.0');qa['payload']['data']['presentation']=None
rw=next(d['payload']['data']for d in read(ROOT/'packages/commands/tests/fixtures/production/p1-parallel-A.json')['definitions']if d['payload']['data'].get('builtin')=='rework');rw=copy.deepcopy(rw);rw.update(resource_roles=[],duration={'value':4000,'unit':'millisecond','basis':'module_declared','source_ref':None},family='qa-rework');qa['payload']['data']['behavior']['inline']=rw;qa['payload']['data']['ports']=[{'id':name,'direction':direction,'payload':{'kind':'material','data':{'item_role':'product','unit':'piece','quantity_rule':None}}}for name,direction in [('input','input'),('output','output')]];definitions.append(qa)
sink=copy.deepcopy(read(HOME/'devices/vision-carton-inspector/module.json')['data']['definition']);sink.update(id='qa.output-drainer',version='0.1.0');sink['payload']['data']['presentation']=None
op=sink['payload']['data']['behavior']['inline'];op.update(builtin='transform',identity_policy='transform_with_origin',family='qa-output-clear');op.pop('material_flow',None);op['quality_and_routes']=None;op['resource_roles']=[];op['inputs']=op['inputs'][:1];op['inputs'][0]['required_quality']=['released'];op['outputs']=op['outputs'][:1];op['outputs'][0]['declared_output_quality']='released';op['item_roles']=['input','output'];sink['payload']['data']['ports']=[p for p in sink['payload']['data']['ports'] if p['id'] in ['input','output']]
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
 scenario['production_tasks'].append({'id':'task-'+device,'routing_ref':{'id':route,'revision':'r1'},'order_allocations':[],'planned_quantity':q(),'release_at_ms':40000,'priority':0})
project['module_refs']=[ref(d['id']) for d in definitions]
OUT.mkdir(exist_ok=True,parents=True);write(OUT/'paper-input.json',{'project':project,'scenario':scenario,'definitions':definitions})
print(f'Prepared {len(scenario["production_tasks"])} tasks, including {len(drainers)} explicit buffer drainers; not counted.')
